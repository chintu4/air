use anyhow::{Result, anyhow};
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::models::{ModelProvider, ModelResponse, QueryContext};
use crate::config::LocalModelConfig;
use crate::providers::gguf_model::GGUFModel;

pub struct LocalProvider {
    config: LocalModelConfig,
    // Use Arc<Mutex<Option<GGUFModel>>> to allow sharing and lazy loading
    // GGUFModel is internal to providers
    model: Arc<Mutex<Option<GGUFModel>>>,
}

impl LocalProvider {
    pub fn new(config: LocalModelConfig) -> Result<Self> {
        Ok(Self {
            config,
            model: Arc::new(Mutex::new(None)),
        })
    }

    fn ensure_loaded(&self) -> Result<()> {
        let mut model_guard = self.model.lock().unwrap();
        if model_guard.is_some() {
            return Ok(());
        }

        let model_path = PathBuf::from(&self.config.model_path);
        if !model_path.exists() {
            return Err(anyhow!("Model file not found at: {:?}. Run 'air setup --local' first.", model_path));
        }

        let model = GGUFModel::load(&model_path)?;
        *model_guard = Some(model);
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

        let model_arc = self.model.clone();
        let prompt = context.prompt.clone();
        let temperature = context.temperature as f64;
        let max_tokens = context.max_tokens as usize;

        // Run inference in a blocking thread
        let result = tokio::task::spawn_blocking(move || {
            let mut model_guard = model_arc.lock().unwrap();
            let model = model_guard.as_mut().ok_or(anyhow!("Model not loaded"))?;

            let (response_text, tokens_used, time_ms) = model.generate(&prompt, max_tokens, temperature)?;

            Ok::<ModelResponse, anyhow::Error>(ModelResponse {
                content: response_text,
                model_used: "TinyLlama-1.1B-Quantized".to_string(),
                tokens_used,
                response_time_ms: time_ms,
                confidence_score: None,
            })
        }).await??;

        Ok(result)
    }
}
