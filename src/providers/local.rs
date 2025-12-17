use anyhow::{Result, anyhow};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};
use std::io::{self, Write};
use tracing::{info, error};
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
    init_error: Option<String>,
}

pub struct LocalProvider {
    config: LocalModelConfig,
    state: Arc<Mutex<LocalState>>,
    // Signal to notify when background loading is complete
    loaded_notify: Arc<Notify>,
}

impl LocalProvider {
    pub fn new(config: LocalModelConfig) -> Result<Self> {
        let state = Arc::new(Mutex::new(LocalState {
            model: None,
            init_error: None,
        }));
        let loaded_notify = Arc::new(Notify::new());

        let config_clone = config.clone();
        let state_clone = state.clone();
        let notify_clone = loaded_notify.clone();

        // 🚀 SPAWN BACKGROUND LOADING TASK
        // This runs immediately without blocking the main application startup
        tokio::spawn(async move {
            info!("🚀 Starting background initialization of local model...");

            match load_model_internal(config_clone).await {
                Ok(model) => {
                    let mut guard = state_clone.lock().await;
                    guard.model = Some(model);
                    info!("✅ Background model loading complete. Ready for queries.");
                }
                Err(e) => {
                    let mut guard = state_clone.lock().await;
                    guard.init_error = Some(e.to_string());
                    error!("❌ Background model loading failed: {}", e);
                }
            }

            // Wake up anyone waiting in ensure_loaded()
            notify_clone.notify_waiters();
        });

        Ok(Self {
            config,
            state,
            loaded_notify,
        })
    }

    async fn ensure_loaded(&self) -> Result<()> {
        // Fast path: check if loaded
        {
            let guard = self.state.lock().await;
            if guard.model.is_some() {
                return Ok(());
            }
            if let Some(err) = &guard.init_error {
                return Err(anyhow!("Model failed to initialize: {}", err));
            }
        } // Drop lock

        info!("⏳ Waiting for background model loading to complete...");

        // Wait for the background task to signal completion
        self.loaded_notify.notified().await;

        // Check result again
        let guard = self.state.lock().await;
        if guard.model.is_some() {
            Ok(())
        } else if let Some(err) = &guard.init_error {
            Err(anyhow!("Model failed to initialize: {}", err))
        } else {
            Err(anyhow!("Model loading signal received but state is invalid"))
        }
    }
}

// 📦 Extracted loading logic to keep things clean
async fn load_model_internal(config: LocalModelConfig) -> Result<Arc<Model>> {
    let model_path = config.model_path.clone();
    if !std::path::Path::new(&model_path).exists() {
        return Err(anyhow!("Model file not found at: {:?}. Run 'air setup --local' first.", model_path));
    }

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

    if let Some(draft_path_str) = &config.draft_model_path {
        // Speculative decoding config (same as before)
         let draft_path = std::path::Path::new(draft_path_str);
         if draft_path.exists() {
             info!("🚀 Speculative decoding candidate found: {:?}", draft_path);
         }
    }

    builder = builder.with_paged_attn(|| {
        PagedAttentionMetaBuilder::default().build()
    })?;

    match config.device.to_lowercase().as_str() {
        "cpu" => { builder = builder.with_force_cpu(); },
        "gpu" | "cuda" => {
            let device = Device::new_cuda(0)?;
            builder = builder.with_device(device);
        },
        "metal" => {
                let device = Device::new_metal(0)?;
                builder = builder.with_device(device);
        },
        _ => {}
    }

    let model = builder.build().await?;
    Ok(model.into())
}

#[async_trait]
impl ModelProvider for LocalProvider {
    fn name(&self) -> &str {
        "mistralrs-local"
    }

    fn is_available(&self) -> bool {
        // Always return true to allow app to start; error handling happens at generation time
        std::path::Path::new(&self.config.model_path).exists()
    }

    fn estimated_latency_ms(&self) -> u64 { 200 }

    fn quality_score(&self) -> f32 { 0.8 }

    async fn generate(&self, context: &QueryContext) -> Result<ModelResponse> {
        // This will now wait politely if the background thread is still running
        self.ensure_loaded().await?;

        let model = {
            let state = self.state.lock().await;
            state.model.as_ref().unwrap().clone()
        };

        let start_time = std::time::Instant::now();

        // Create messages (Same logic as before)
        let messages = if let Some(msgs) = &context.messages {
            let mut tm = TextMessages::new();
            for msg in msgs {
                let role = match msg.role.as_str() {
                    "system" => TextMessageRole::System,
                    "user" => TextMessageRole::User,
                    "assistant" => TextMessageRole::Assistant,
                    _ => TextMessageRole::User,
                };
                tm = tm.add_message(role, msg.content.clone());
            }
            tm
        } else {
            TextMessages::new().add_message(TextMessageRole::User, context.prompt.clone())
        };

        let mut request_builder = RequestBuilder::from(messages)
            .set_sampler_max_len(context.max_tokens as usize)
            .set_sampler_temperature(context.temperature as f64)
            .set_sampler_topp(0.9)
            .set_sampler_topk(40);

        // FIX 2: Grammar Constraint for Small Models
        // If config says small model, force JSON structure
        if self.config.is_small_model {
             // Simple regex to force JSON-like structure: { "tool": "...", "args": { ... } }
             // let pattern = r#"\s*\{\s*"tool"\s*:\s*"[a-zA-Z0-9_]+"\s*,\s*"args"\s*:\s*\{[\s\S]*\}\s*\}"#;
             // request_builder = request_builder.set_grammar(mistralrs::Grammar::Regex(pattern.to_string()));
             // TODO: Re-enable strict grammar when mistralrs API is confirmed or updated
             // For now, we rely on the few-shot examples to guide the model.
             info!("🚀 Small model optimization active (simplified prompt). Grammar enforcement pending API update.");
        }

        let request = request_builder;

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
            // Handle other errors...
        }
        println!(); // Newline after stream

        Ok(ModelResponse {
            content,
            model_used: "mistralrs-gguf".to_string(),
            tokens_used,
            response_time_ms: start_time.elapsed().as_millis() as u64,
            confidence_score: None,
        })
    }
}
