use anyhow::{Result, anyhow};
use async_trait::async_trait;
use std::path::PathBuf;

use tracing::{error,info, warn, debug};
use std::time::Instant;
use std::sync::{Arc, Mutex};
use crate::models::{ModelProvider, ModelResponse, QueryContext, ModelMetrics};
use crate::providers::gguf_model::GGUFModel;
use crate::config::LocalModelConfig;

use candle_core::{Device, Tensor, DType};
use candle_transformers::models::quantized_llama::ModelWeights;
use candle_transformers::generation::LogitsProcessor;
use tokenizers::Tokenizer;
use hf_hub::api::sync::Api;

struct LocalState {
    model: Option<ModelWeights>,
    tokenizer: Option<Tokenizer>,
}

pub struct LocalProvider {
    config: LocalModelConfig,
    state: Arc<Mutex<LocalState>>,
}

fn load_tokenizer(model_path: &PathBuf) -> Result<Tokenizer> {
    let parent = model_path.parent().ok_or_else(|| anyhow!("Model path has no parent"))?;
    let json_path = parent.join("tokenizer.json");
    Ok(Tokenizer::from_file(json_path).map_err(|e| anyhow!(e))?)
}

impl LocalProvider {
    pub fn new(config: LocalModelConfig) -> Result<Self> {
        Ok(Self {
            config,
            state: Arc::new(Mutex::new(LocalState {
                model: None,
                tokenizer: None,
            })),
        })
    }

    fn ensure_loaded(&self) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        if state.model.is_some() {
            return Ok(());
        }

        let model_path = PathBuf::from(&self.config.model_path);
        if !model_path.exists() {
            return Err(anyhow!("Model file not found at: {:?}. Run 'air setup --local' first.", model_path));
        }

        info!("Loading local model from {:?}...", model_path);
        let start = Instant::now();

        // Load model
        let mut file = std::fs::File::open(&model_path)?;
        let model = candle_transformers::models::quantized_llama::ModelWeights::from_gguf(
            candle_core::quantized::gguf_file::Content::read(&mut file)?,
            &mut file,
            &Device::Cpu
        )?;

        // Load tokenizer
        // We try to find tokenizer.json in the same folder, or ~/.air/models/tokenizer.json
        // Or fetch from HF.
        let tokenizer = match load_tokenizer(&model_path) {
            Ok(t) => t,
            Err(e) => {
                warn!("Could not find local tokenizer.json: {}. Attempting download...", e);
                let api = Api::new()?;
                let repo = api.model("TinyLlama/TinyLlama-1.1B-Chat-v1.0".to_string());
                let path = repo.get("tokenizer.json")?;
                Tokenizer::from_file(path).map_err(|e| anyhow!(e))?
            }
        };

        state.model = Some(model);
        state.tokenizer = Some(tokenizer);

        info!("Model loaded in {:.2?}", start.elapsed());
        Ok(())
    }
}

#[async_trait]
impl ModelProvider for LocalProvider {
    fn name(&self) -> &str {
        "local-candle"
    }

    fn is_available(&self) -> bool {
        PathBuf::from(&self.config.model_path).exists()
    }

    fn estimated_latency_ms(&self) -> u64 {
        500
    }

    fn quality_score(&self) -> f32 {
        0.7
    }

    async fn generate(&self, context: &QueryContext) -> Result<ModelResponse> {
        self.ensure_loaded()?;

        // We clone the state wrapper to move into the blocking task
        let state_arc = self.state.clone();
        let prompt = context.prompt.clone();
        let temperature = context.temperature as f64; // candle expects f64
        let max_tokens = context.max_tokens as usize;

        // Run inference in a blocking thread to avoid blocking the async runtime
        let result = tokio::task::spawn_blocking(move || {
            let mut state = state_arc.lock().unwrap();
            // We need to split the borrow
            let tokenizer = state.tokenizer.clone().unwrap();
            let model = state.model.as_mut().unwrap();

            // Format prompt for TinyLlama Chat
            // <|user|>\n{prompt}</s>\n<|assistant|>
            let formatted_prompt = format!("<|user|>\n{}</s>\n<|assistant|>", prompt);

            let tokens = tokenizer.encode(formatted_prompt, true).map_err(|e| anyhow!(e))?;
            let tokens = tokens.get_ids();

            let mut logits_processor = LogitsProcessor::new(299792458, Some(temperature), None);
            let mut generated_tokens = Vec::new();
            let mut current_tokens = tokens.to_vec();

            let start_gen = Instant::now();

            for _ in 0..max_tokens {
                let input = Tensor::new(current_tokens.as_slice(), &Device::Cpu)?.unsqueeze(0)?;
                let logits = model.forward(&input, current_tokens.len())?;
                let logits = logits.squeeze(0)?;

                let next_token = logits_processor.sample(&logits)?;
                generated_tokens.push(next_token);
                current_tokens.push(next_token);

                if next_token == tokenizer.token_to_id("</s>").unwrap_or(2) {
                    break;
                }
            }

            let response_text = tokenizer.decode(&generated_tokens, true).map_err(|e| anyhow!(e))?;
            let time_ms = start_gen.elapsed().as_millis() as u64;

            Ok::<ModelResponse, anyhow::Error>(ModelResponse {
                content: response_text,
                model_used: "TinyLlama-1.1B-Quantized".to_string(),
                tokens_used: generated_tokens.len() as u32,
                response_time_ms: time_ms,
                confidence_score: None,
            })
        }).await??;

        Ok(result)
    }
}