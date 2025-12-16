use anyhow::{Result, anyhow};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::io::{self, Write};
use crate::models::{ModelProvider, ModelResponse, QueryContext};
use crate::config::LocalModelConfig;
use mistralrs::{
    GgufModelBuilder, Model,
    TextMessageRole,
    TextMessages, Device, PagedAttentionMetaBuilder,
    RequestBuilder, Response, ChatCompletionChunkResponse, ChunkChoice, Delta
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

        let mut builder = GgufModelBuilder::new(
             parent.to_string_lossy(),
             vec![filename.to_string()]
        )
        .with_logging();

        builder = builder.with_paged_attn(|| {
            PagedAttentionMetaBuilder::default().build()
        })?;

        match self.config.device.to_lowercase().as_str() {
            "cpu" => {
                builder = builder.with_force_cpu();
            },
            "gpu" | "cuda" => {
                let device = Device::new_cuda(0)?;
                builder = builder.with_device(device);
            },
            _ => {
                // "auto" or anything else: Let mistralrs decide
            }
        }
 
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
        let model = state.model.as_ref().unwrap();

        let start_time = std::time::Instant::now();

        // Create the messages
        let messages = TextMessages::new()
            .add_message(TextMessageRole::User, context.prompt.clone());

        let request = RequestBuilder::from(messages)
            .set_sampler_max_len(context.max_tokens as usize)
            .set_sampler_temperature(context.temperature as f64)
            .set_sampler_topp(0.9)
            .set_sampler_topk(40);

        let mut stream = model.stream_chat_request(request).await?;
        let mut content = String::new();
        let mut tokens_used = 0;

        while let Some(chunk) = stream.next().await {
            if let Response::Chunk(ChatCompletionChunkResponse { choices, .. }) = chunk {
                if let Some(ChunkChoice { delta: Delta { content: Some(c), .. }, .. }) = choices.first() {
                    print!("{}", c);
                    io::stdout().flush().ok();
                    content.push_str(c);
                    tokens_used += 1;
                }
            } else if let Response::ModelError(msg, _) = chunk {
                return Err(anyhow!("Model error: {}", msg));
            } else if let Response::ValidationError(msg) = chunk {
                return Err(anyhow!("Validation error: {}", msg));
            } else if let Response::InternalError(e) = chunk {
                 return Err(anyhow!("Internal error: {}", e));
            }
        }

        if content.is_empty() {
            return Err(anyhow!("Local model produced no tokens"));
        }

        Ok(ModelResponse {
            content,
            model_used: "mistralrs-gguf".to_string(),
            tokens_used,
            response_time_ms: start_time.elapsed().as_millis() as u64,
            confidence_score: None,
        })
    }
}
