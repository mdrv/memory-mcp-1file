use std::path::Path;

use anyhow::Result;
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

        let tokens = self
            .tokenizer
            .as_ref()
            .unwrap()
            .encode(text, true)
            .map_err(anyhow::Error::msg)?;

        let token_ids = tokens.get_ids();
        let token_ids_tensor = Tensor::new(token_ids, &self.device)?.unsqueeze(0)?;
        let token_type_ids = Tensor::new(tokens.get_type_ids(), &self.device)?.unsqueeze(0)?;

        let embeddings =
            self.model
                .as_ref()
                .unwrap()
                .forward(&token_ids_tensor, &token_type_ids, None)?;

        let (_n_batch, seq_len, _hidden_size) = embeddings.dims3()?;
        let pooled = (embeddings.sum(1)? / (seq_len as f64))?;
        let pooled = pooled.get(0)?;

        let norm = pooled.sqr()?.sum_all()?.sqrt()?;
        let normalized = pooled.broadcast_div(&norm)?;

        let vec: Vec<f32> = normalized.to_vec1()?;

        debug_assert_eq!(vec.len(), self.dimensions);
        Ok(vec)
    }

    pub fn dimensions(&self) -> usize {
        self.dimensions
    }
}
