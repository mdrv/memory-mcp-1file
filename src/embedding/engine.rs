use anyhow::Result;
use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config, DTYPE};
use hf_hub::api::sync::Api;
use tokenizers::Tokenizer;

use super::config::ModelType;

pub struct EmbeddingEngine {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
    dimensions: usize,
}

impl EmbeddingEngine {
    pub fn new(model_type: ModelType) -> Result<Self> {
        let device = Device::Cpu;
        let api = Api::new()?;
        let repo = api.model(model_type.repo_id().to_string());

        let config_filename = repo.get("config.json")?;
        let tokenizer_filename = repo.get("tokenizer.json")?;
        let weights_filename = repo.get("model.safetensors")?;

        let config_content = std::fs::read_to_string(&config_filename)?;
        let config: Config = serde_json::from_str(&config_content)?;

        let tokenizer =
            Tokenizer::from_file(&tokenizer_filename).map_err(anyhow::Error::msg)?;

        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[weights_filename], DTYPE, &device)?
        };
        let model = BertModel::load(vb, &config)?;

        Ok(Self {
            model,
            tokenizer,
            device,
            dimensions: model_type.dimensions(),
        })
    }

    pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let tokens = self
            .tokenizer
            .encode(text, true)
            .map_err(anyhow::Error::msg)?;

        let token_ids = tokens.get_ids();
        let token_ids_tensor = Tensor::new(token_ids, &self.device)?.unsqueeze(0)?;
        let token_type_ids = Tensor::new(tokens.get_type_ids(), &self.device)?.unsqueeze(0)?;

        let embeddings = self
            .model
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
