use anyhow::Result;
use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config, DTYPE};
use hf_hub::{api::sync::ApiBuilder, Repo, RepoType};
use tokenizers::{PaddingParams, Tokenizer};
use std::path::PathBuf;

pub struct EmbeddingModel {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
}

impl EmbeddingModel {
    pub fn new() -> Result<Self> {
        let device = Device::Cpu;

        // Use all-MiniLM-L6-v2 for efficient local embeddings
        let model_id = "sentence-transformers/all-MiniLM-L6-v2";
        let _revision = "refs/pr/21";

        // Set explicit cache path to avoid environment issues
        let mut cache_path = std::env::current_dir()?;
        cache_path.push(".air");
        cache_path.push("cache");

        let api = ApiBuilder::new()
            .with_cache_dir(cache_path)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to init HF API: {}", e))?;

        let repo = api.repo(Repo::new(model_id.to_string(), RepoType::Model));

        let config_filename = repo.get("config.json").map_err(|e| anyhow::anyhow!("Failed to get config: {}", e))?;
        let tokenizer_filename = repo.get("tokenizer.json").map_err(|e| anyhow::anyhow!("Failed to get tokenizer: {}", e))?;
        let weights_filename = repo.get("model.safetensors").map_err(|e| anyhow::anyhow!("Failed to get weights: {}", e))?;

        let config: Config = serde_json::from_str(&std::fs::read_to_string(config_filename)?)?;
        let tokenizer = Tokenizer::from_file(tokenizer_filename).map_err(|e| anyhow::anyhow!(e))?;

        let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[weights_filename], DTYPE, &device)? };
        let model = BertModel::load(vb, &config)?;

        Ok(Self {
            model,
            tokenizer,
            device,
        })
    }

    pub fn embed(&mut self, text: &str) -> Result<Vec<f32>> {
        let mut tokenizer = self.tokenizer.clone();

        if let Some(pp) = tokenizer.get_padding_mut() {
            pp.strategy = tokenizers::PaddingStrategy::BatchLongest;
        } else {
            let pp = PaddingParams {
                strategy: tokenizers::PaddingStrategy::BatchLongest,
                ..Default::default()
            };
            tokenizer.with_padding(Some(pp));
        }

        let tokens = tokenizer.encode(text, true).map_err(|e| anyhow::anyhow!(e))?;
        let token_ids = Tensor::new(&tokens.get_ids().to_vec()[..], &self.device)?.unsqueeze(0)?;
        let token_type_ids = token_ids.zeros_like()?;

        let embeddings = self.model.forward(&token_ids, &token_type_ids, None)?;

        // Mean pooling
        let (_n_sentence, n_tokens, _hidden_size) = embeddings.dims3()?;
        let embeddings = (embeddings.sum(1)? / (n_tokens as f64))?;
        let embeddings = embeddings.squeeze(0)?;

        // Normalize
        let norm = embeddings.sqr()?.sum_all()?.sqrt()?;
        let embeddings = embeddings.broadcast_div(&norm)?;

        Ok(embeddings.to_vec1()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::Tensor;

    #[test]
    fn test_normalization_shape_handling() -> Result<()> {
        let device = Device::Cpu;
        // Simulate a 384-dim embedding vector
        let data = vec![1.0f32; 384];
        let embeddings = Tensor::new(data, &device)?;

        // Calculate norm (scalar)
        let norm = embeddings.sqr()?.sum_all()?.sqrt()?;
        assert_eq!(norm.rank(), 0);

        // This should not panic
        let normalized = embeddings.broadcast_div(&norm)?;
        assert_eq!(normalized.shape().dims(), &[384]);

        Ok(())
    }
}
