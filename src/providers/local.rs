use anyhow::{Result, anyhow, Error};
use async_trait::async_trait;
use std::path::PathBuf;
use tracing::{info, warn, debug};
use std::time::Instant;
use std::sync::{Arc, Mutex};

use crate::models::{ModelProvider, ModelResponse, QueryContext};
use crate::config::LocalModelConfig;

use candle_core::{Device, Tensor, DType};
use candle_transformers::models::quantized_llama::ModelWeights;
use candle_transformers::generation::LogitsProcessor;
use tokenizers::Tokenizer;
use hf_hub::api::sync::Api;
use minijinja::Environment;
use serde::Serialize;

struct LocalState {
    model: Option<ModelWeights>,
    tokenizer: Option<Tokenizer>,
    template: Option<String>, // Raw template string
}

pub struct LocalProvider {
    config: LocalModelConfig,
    state: Arc<Mutex<LocalState>>,
}

impl LocalProvider {
    pub fn new(config: LocalModelConfig) -> Result<Self> {
        let provider = Self {
            config,
            state: Arc::new(Mutex::new(LocalState {
                model: None,
                tokenizer: None,
                template: None,
            })),
        };

        Ok(provider)
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
        let tokenizer = match load_tokenizer(&model_path) {
            Ok(t) => t,
            Err(e) => {
                warn!("Could not find local tokenizer.json: {}. Attempting download...", e);
                let api = Api::new()?;
                let repo = api.model("TinyLlama/TinyLlama-1.1B-Chat-v1.0".to_string());
                let path = repo.get("tokenizer.json")?;
                Tokenizer::from_file(path).map_err(Error::msg)?
            }
        };

        // Try to get chat template from tokenizer config
        // tokenizers crate doesn't expose the raw config JSON easily in struct,
        // but we can look for tokenizer_config.json if it exists
        let parent = model_path.parent().unwrap();
        let config_path = parent.join("tokenizer_config.json");
        let template = if config_path.exists() {
             if let Ok(content) = std::fs::read_to_string(config_path) {
                 if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                     json.get("chat_template").and_then(|v| v.as_str()).map(|s| s.to_string())
                 } else { None }
             } else { None }
        } else {
             // Fallback: TinyLlama template
             Some("{% for message in messages %}<|{{message.role}}|>\n{{message.content}}</s>\n{% endfor %}{% if add_generation_prompt %}<|assistant|>\n{% endif %}".to_string())
        };

        state.model = Some(model);
        state.tokenizer = Some(tokenizer);
        state.template = template;

        info!("Model loaded in {:.2?}", start.elapsed());
        Ok(())
    }
}

fn load_tokenizer(model_path: &PathBuf) -> Result<Tokenizer> {
    let parent = model_path.parent().unwrap();
    let json_path = parent.join("tokenizer.json");
    if json_path.exists() {
        return Tokenizer::from_file(json_path).map_err(Error::msg);
    }
    Err(anyhow!("tokenizer.json not found"))
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
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
        let temperature = context.temperature as f64;
        let max_tokens = context.max_tokens as usize;

        let result = tokio::task::spawn_blocking(move || {
            let mut state = state_arc.lock().unwrap();
            // Clone simple things first
            let tokenizer = state.tokenizer.clone().unwrap();
            let template_str = state.template.clone().unwrap_or_default();
            // Then get mutable reference to model
            let model = state.model.as_mut().unwrap();

            // Render Prompt using Minijinja
            let mut env = Environment::new();
            // We need to register the template.
            // Since we are running in a tight loop, creating env every time is slightly inefficient
            // but acceptable for local LLM inference speeds.

            let formatted_prompt = if !template_str.is_empty() {
                env.add_template("chat", &template_str).map_err(|e| anyhow!(e))?;
                let tmpl = env.get_template("chat").map_err(|e| anyhow!(e))?;

                let messages = vec![
                    Message { role: "user".to_string(), content: prompt }
                ];

                tmpl.render(serde_json::json!({
                    "messages": messages,
                    "add_generation_prompt": true
                })).map_err(|e| anyhow!(e))?
            } else {
                // Fallback simple format
                format!("User: {}\nAssistant:", prompt)
            };

            debug!("Formatted Prompt:\n{}", formatted_prompt);

            let tokens = tokenizer.encode(formatted_prompt, true).map_err(Error::msg)?;
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

                // Check for EOS token
                if let Some(eos_id) = tokenizer.token_to_id("</s>") {
                     if next_token == eos_id {
                         break;
                     }
                } else if let Some(eos_id) = tokenizer.token_to_id("<|end_of_text|>") {
                    // Llama 3 style
                     if next_token == eos_id {
                         break;
                     }
                }
            }

            let response_text = tokenizer.decode(&generated_tokens, true).map_err(Error::msg)?;
            let time_ms = start_gen.elapsed().as_millis() as u64;

            Ok::<ModelResponse, anyhow::Error>(ModelResponse {
                content: response_text,
                model_used: "Local GGUF".to_string(),
                tokens_used: generated_tokens.len() as u32,
                response_time_ms: time_ms,
                confidence_score: None,
            })
        }).await??;

        Ok(result)
    }
}
