use anyhow::{Result, anyhow};
use async_trait::async_trait;
use std::path::PathBuf;

use std::sync::{Arc, Mutex};
use crate::models::{ModelProvider, ModelResponse, QueryContext};
use crate::providers::gguf_model::GGUFModel;
use crate::config::LocalModelConfig;

struct LocalState {
    model: Option<GGUFModel>,
}

pub struct LocalProvider {
    config: LocalModelConfig,
    state: Arc<Mutex<LocalState>>,
}

impl LocalProvider {
    pub fn new(config: LocalModelConfig) -> Result<Self> {
        Ok(Self {
            config,
            state: Arc::new(Mutex::new(LocalState {
                model: None,
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

        // Use GGUFModel to load the model and tokenizer
        let model = GGUFModel::load(&model_path)?;

        state.model = Some(model);
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
        let temperature = context.temperature as f64;
        let max_tokens = context.max_tokens as usize;

        // Run inference in a blocking thread to avoid blocking the async runtime
        let result = tokio::task::spawn_blocking(move || {
            let mut state = state_arc.lock().unwrap();
            let model = state.model.as_mut().unwrap();

            let (response_text, tokens_count, time_ms) = model.generate(&prompt, max_tokens, temperature)?;

            Ok::<ModelResponse, anyhow::Error>(ModelResponse {
                content: response_text,
                model_used: "TinyLlama-1.1B-Quantized".to_string(),
                tokens_used: tokens_count,
                response_time_ms: time_ms,
                confidence_score: None,
            })
        }).await??;

        Ok(result)
    }
}
