use std::path::Path;

use anyhow::{anyhow, Result};
use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config, DTYPE};
use hf_hub::api::sync::Api;
use tokenizers::Tokenizer;

use super::config::ModelType;

pub struct EmbeddingEngine {
    model: Option<BertModel>,
    tokenizer: Option<Tokenizer>,
    device: Device,
    dimensions: usize,
}

impl EmbeddingEngine {
    pub fn new(model_type: ModelType) -> Result<Self> {
        let device = Device::Cpu;
        let dimensions = model_type.dimensions();

        if model_type == ModelType::Mock {
            return Ok(Self {
                model: None,
                tokenizer: None,
                device,
                dimensions,
            });
        }

        let api = Api::new()?;
        let repo = api.model(model_type.repo_id().to_string());

        let config_filename = repo.get("config.json")?;
        let tokenizer_filename = repo.get("tokenizer.json")?;
        let weights_filename = repo.get("model.safetensors")?;

        Self::from_files(
            model_type,
            &config_filename,
            &tokenizer_filename,
            &weights_filename,
        )
    }

    pub fn from_files(
        model_type: ModelType,
        config_path: &Path,
        tokenizer_path: &Path,
        weights_path: &Path,
    ) -> Result<Self> {
        let device = Device::Cpu;
        let dimensions = model_type.dimensions();

        if model_type == ModelType::Mock {
            return Ok(Self {
                model: None,
                tokenizer: None,
                device,
                dimensions,
            });
        }

        let config_content = std::fs::read_to_string(config_path)?;
        let config: Config = serde_json::from_str(&config_content)?;

        let tokenizer = Tokenizer::from_file(tokenizer_path).map_err(anyhow::Error::msg)?;

        let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[weights_path], DTYPE, &device)? };
        let model = BertModel::load(vb, &config)?;

        Ok(Self {
            model: Some(model),
            tokenizer: Some(tokenizer),
            device,
            dimensions,
        })
    }

    pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
        if self.model.is_none() || self.tokenizer.is_none() {
            // Mock embedding: deterministic but random-looking
            let mut vec = vec![0.0; self.dimensions];
            let hash = blake3::hash(text.as_bytes());
            let hash_bytes = hash.as_bytes();
            for i in 0..self.dimensions {
                let byte = hash_bytes[i % 32];
                vec[i] = (byte as f32 / 255.0) * 2.0 - 1.0;
            }
            return Ok(vec);
        }

        let tokenizer = self
            .tokenizer
            .as_ref()
            .ok_or_else(|| anyhow!("Tokenizer not loaded"))?;
        let model = self
            .model
            .as_ref()
            .ok_or_else(|| anyhow!("Model not loaded"))?;

        let tokens = tokenizer.encode(text, true).map_err(anyhow::Error::msg)?;

        let vocab_size = tokenizer.get_vocab_size(true) as u32;
        let unk_id = tokenizer.token_to_id("[UNK]").unwrap_or(0);
        let token_ids: Vec<u32> = tokens
            .get_ids()
            .iter()
            .map(|&id| if id >= vocab_size { unk_id } else { id })
            .collect();

        let token_ids_tensor = Tensor::new(&token_ids[..], &self.device)?.unsqueeze(0)?;
        let token_type_ids = Tensor::new(tokens.get_type_ids(), &self.device)?.unsqueeze(0)?;

        let embeddings = model.forward(&token_ids_tensor, &token_type_ids, None)?;

        let (_n_batch, seq_len, _hidden_size) = embeddings.dims3()?;
        let pooled = (embeddings.sum(1)? / (seq_len as f64))?;
        let pooled = pooled.get(0)?;

        let norm = pooled.sqr()?.sum_all()?.sqrt()?;
        let normalized = pooled.broadcast_div(&norm)?;

        let vec: Vec<f32> = normalized.to_vec1()?;

        debug_assert_eq!(vec.len(), self.dimensions);
        Ok(vec)
    }

    pub fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        if self.model.is_none() || self.tokenizer.is_none() {
            // Mock embedding: deterministic but random-looking for each text
            let mut results = Vec::with_capacity(texts.len());
            for text in texts {
                let mut vec = vec![0.0; self.dimensions];
                let hash = blake3::hash(text.as_bytes());
                let hash_bytes = hash.as_bytes();
                for i in 0..self.dimensions {
                    let byte = hash_bytes[i % 32];
                    vec[i] = (byte as f32 / 255.0) * 2.0 - 1.0;
                }
                results.push(vec);
            }
            return Ok(results);
        }

        let tokenizer = self
            .tokenizer
            .as_ref()
            .ok_or_else(|| anyhow!("Tokenizer not loaded"))?;
        let model = self
            .model
            .as_ref()
            .ok_or_else(|| anyhow!("Model not loaded"))?;

        let vocab_size = tokenizer.get_vocab_size(true) as u32;
        let unk_id = tokenizer.token_to_id("[UNK]").unwrap_or(0);

        // 1. Tokenize all texts
        let mut all_token_ids = Vec::with_capacity(texts.len());
        let mut all_token_type_ids = Vec::with_capacity(texts.len());
        let mut max_len = 0;

        for text in texts {
            let tokens = tokenizer
                .encode(text.as_str(), true)
                .map_err(anyhow::Error::msg)?;
            let token_ids: Vec<u32> = tokens
                .get_ids()
                .iter()
                .map(|&id| if id >= vocab_size { unk_id } else { id })
                .collect();
            let type_ids = tokens.get_type_ids();
            max_len = max_len.max(token_ids.len());
            all_token_ids.push(token_ids);
            all_token_type_ids.push(type_ids.to_vec());
        }

        // 2. Pad and create batch tensors
        // This is a naive implementation. For real batching with candle/huggingface,
        // we should create a single Tensor with shape (batch_size, max_seq_len).

        // Flatten for Tensor creation
        let mut flat_token_ids = Vec::with_capacity(texts.len() * max_len);
        let mut flat_type_ids = Vec::with_capacity(texts.len() * max_len);

        for (ids, types) in all_token_ids.iter().zip(all_token_type_ids.iter()) {
            // Copy data
            flat_token_ids.extend_from_slice(ids);
            flat_type_ids.extend_from_slice(types);

            // Padding (0)
            let pad_len = max_len - ids.len();
            flat_token_ids.extend(std::iter::repeat_n(0, pad_len));
            flat_type_ids.extend(std::iter::repeat_n(0, pad_len));
        }

        let batch_token_ids = Tensor::new(flat_token_ids.as_slice(), &self.device)?
            .reshape((texts.len(), max_len))?;

        let batch_token_type_ids =
            Tensor::new(flat_type_ids.as_slice(), &self.device)?.reshape((texts.len(), max_len))?;

        // 3. Forward pass
        let embeddings = model.forward(&batch_token_ids, &batch_token_type_ids, None)?;

        // 4. Mean pooling
        let (_n_batch, seq_len, _hidden_size) = embeddings.dims3()?;
        let pooled = (embeddings.sum(1)? / (seq_len as f64))?;

        // 5. Normalize
        // Normalize each vector in the batch independently
        // pooled is (batch_size, hidden_size)
        // sqr().sum(1) gives (batch_size) -> norms squared
        let norms_sq = pooled.sqr()?.sum(1)?;
        let norms = norms_sq.sqrt()?;
        // reshape norms to (batch_size, 1) to broadcast
        let norms_reshaped = norms.reshape((texts.len(), 1))?;
        let normalized = pooled.broadcast_div(&norms_reshaped)?;

        // 6. Extract vectors
        // normalized.to_vec2() returns Vec<Vec<f32>>
        let vectors: Vec<Vec<f32>> = normalized.to_vec2()?;

        Ok(vectors)
    }

    pub fn dimensions(&self) -> usize {
        self.dimensions
    }
}
