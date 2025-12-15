use anyhow::{Result, anyhow};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::models::{ModelProvider, ModelResponse, QueryContext};
use crate::config::LocalModelConfig;
use mistralrs::{
    GgufModelBuilder, Model,
    ChatCompletionResponse, TextMessageRole,
    TextMessages,
};

struct LocalState {
    model: Option<Arc<Model>>,
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

    async fn ensure_loaded(&self) -> Result<()> {
        let mut state = self.state.lock().await;
        if state.model.is_some() {
            return Ok(());
        }

        let model_path = self.config.model_path.clone();
        if !std::path::Path::new(&model_path).exists() {
            return Err(anyhow!("Model file not found at: {:?}. Run 'air setup --local' first.", model_path));
        }

        // Initialize mistralrs GGUF model
        let path = std::path::Path::new(&model_path);
        let parent = path.parent().unwrap_or(std::path::Path::new("."));
        let filename = path.file_name()
            .ok_or_else(|| anyhow!("Invalid model filename"))?
            .to_str()
            .ok_or_else(|| anyhow!("Invalid filename string"))?;

        let builder = GgufModelBuilder::new(
             parent.to_string_lossy(),
             vec![filename.to_string()]
        );

        let model = builder.build().await?;

        state.model = Some(model.into());
        Ok(())
    }
}

#[async_trait]
impl ModelProvider for LocalProvider {
    fn name(&self) -> &str {
        "mistralrs-local"
    }

    fn is_available(&self) -> bool {
        std::path::Path::new(&self.config.model_path).exists()
    }

    fn estimated_latency_ms(&self) -> u64 {
        200
    }

    fn quality_score(&self) -> f32 {
        0.8
    }

    async fn generate(&self, context: &QueryContext) -> Result<ModelResponse> {
        self.ensure_loaded().await?;

        let state = self.state.lock().await;
        let model = state.model.as_ref().unwrap().clone();

        let start_time = std::time::Instant::now();

        // Create the messages
        let messages = TextMessages::new()
            .add_message(TextMessageRole::User, context.prompt.clone());

        // Send request to the model
        // According to recent error, this returns ChatCompletionResponse directly
        let response: ChatCompletionResponse = model.send_chat_request(messages).await?;

        let content = response.choices.first()
            .map(|c| c.message.content.clone().unwrap_or_default())
            .unwrap_or_default();

        let tokens_used = response.usage.total_tokens as usize;

        Ok(ModelResponse {
            content,
            model_used: "mistralrs-gguf".to_string(),
            tokens_used: tokens_used.try_into().unwrap_or(0),
            response_time_ms: start_time.elapsed().as_millis() as u64,
            confidence_score: None,
        })
    }
}
