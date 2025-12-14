use anyhow::{Result, anyhow, Error};
use std::path::PathBuf;
use std::time::Instant;
use tracing::{info, warn};

use candle_core::{Device, Tensor};
use candle_transformers::models::quantized_llama::ModelWeights;
use candle_transformers::generation::LogitsProcessor;
use tokenizers::Tokenizer;
use hf_hub::api::sync::Api;

pub struct GGUFModel {
    model: ModelWeights,
    tokenizer: Tokenizer,
}

impl GGUFModel {
    pub fn load(model_path: &PathBuf) -> Result<Self> {
        info!("Loading local model from {:?}...", model_path);
        let start = Instant::now();

        // Load model
        let mut file = std::fs::File::open(&model_path)?;
        let model = ModelWeights::from_gguf(
            candle_core::quantized::gguf_file::Content::read(&mut file)?,
            &mut file,
            &Device::Cpu
        )?;

        // Load tokenizer
        let tokenizer = match Self::load_tokenizer(&model_path) {
            Ok(t) => t,
            Err(e) => {
                warn!("Could not find valid local tokenizer.json: {}. Attempting download...", e);
                let api = Api::new()?;
                let repo = api.model("TinyLlama/TinyLlama-1.1B-Chat-v1.0".to_string());
                let path = repo.get("tokenizer.json")?;
                Tokenizer::from_file(path).map_err(Error::msg)?
            }
        };

        info!("Model loaded in {:.2?}", start.elapsed());
        Ok(Self { model, tokenizer })
    }

    fn load_tokenizer(model_path: &PathBuf) -> Result<Tokenizer> {
        let parent = model_path.parent().unwrap();
        let json_path = parent.join("tokenizer.json");

        if json_path.exists() {
            match Tokenizer::from_file(&json_path) {
                Ok(t) => return Ok(t),
                Err(e) => {
                    warn!("Found local tokenizer.json but it failed to load: {}", e);
                    // If the error indicates a missing model structure or other corruption, delete it.
                    // The error "Model missing" is typical for empty/invalid JSON that isn't a tokenizer.
                    // We'll conservatively delete it if it fails to load at all, to force a refresh.
                    if let Err(del_err) = std::fs::remove_file(&json_path) {
                        warn!("Failed to delete corrupted tokenizer.json: {}", del_err);
                    } else {
                        info!("Deleted corrupted tokenizer.json to trigger redownload.");
                    }
                }
            }
        }
        Err(anyhow!("tokenizer.json not found or corrupted"))
    }

    pub fn generate(&mut self, prompt: &str, max_tokens: usize, temperature: f64) -> Result<(String, u32, u64)> {
        let start_gen = Instant::now();

        // Format prompt for TinyLlama Chat
        // <|user|>\n{prompt}</s>\n<|assistant|>
        let formatted_prompt = format!("<|user|>\n{}</s>\n<|assistant|>", prompt);

        let tokens = self.tokenizer.encode(formatted_prompt, true).map_err(Error::msg)?;
        let tokens = tokens.get_ids();

        let mut logits_processor = LogitsProcessor::new(299792458, Some(temperature), None);
        let mut generated_tokens = Vec::new();
        let mut current_tokens = tokens.to_vec();

        for _ in 0..max_tokens {
            let input = Tensor::new(current_tokens.as_slice(), &Device::Cpu)?.unsqueeze(0)?;
            let logits = self.model.forward(&input, current_tokens.len())?;
            let logits = logits.squeeze(0)?;

            let next_token = logits_processor.sample(&logits)?;
            generated_tokens.push(next_token);
            current_tokens.push(next_token);

            if next_token == self.tokenizer.token_to_id("</s>").unwrap_or(2) {
                break;
            }
        }

        let response_text = self.tokenizer.decode(&generated_tokens, true).map_err(Error::msg)?;
        let time_ms = start_gen.elapsed().as_millis() as u64;

        Ok((response_text, generated_tokens.len() as u32, time_ms))
    }
}
