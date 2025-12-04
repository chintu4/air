use crate::models::{ModelProvider, ModelResponse};
use crate::providers::{LocalProvider, OpenAIProvider, AnthropicProvider, GeminiProvider, OpenRouterProvider};
use crate::config::Config;
use crate::tools::ToolManager;
use crate::agent::memory::MemoryManager;
use crate::agent::query::QueryProcessor;
use anyhow::{Result, anyhow};
use std::sync::Arc;
use std::sync::Mutex;
use tracing::{info, warn};

pub struct AIAgent {
    local_provider: Option<Arc<dyn ModelProvider>>,
    cloud_providers: Vec<Arc<dyn ModelProvider>>,
    config: Config,
    tool_manager: ToolManager,
    memory_manager: MemoryManager,
    query_processor: QueryProcessor,
    prompt_cache: Arc<Mutex<std::collections::HashMap<String, (String, std::time::Instant)>>>,
}

impl std::fmt::Debug for AIAgent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AIAgent")
            .field("local_provider", &self.local_provider.is_some())
            .field("cloud_providers_count", &self.cloud_providers.len())
            .field("config", &self.config)
            .field("tool_manager", &"ToolManager")
            .field("memory_manager", &"MemoryManager")
            .finish()
    }
}

impl AIAgent {
    pub async fn new(config: Config) -> Result<Self> {
        info!("Initializing AI Agent...");

        // Get app data directory for database
        let app_data = std::env::var("APPDATA")
            .or_else(|_| std::env::var("LOCALAPPDATA"))
            .unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().to_string());

        // Initialize memory manager
        let memory_manager = MemoryManager::new(&app_data)?;

        // Initialize local provider
        let local_provider = match LocalProvider::new(config.local_model.clone()) {
            Ok(provider) => {
                info!("✅ Local model initialized: {:?}", config.local_model.model_path);
                Some(Arc::new(provider) as Arc<dyn ModelProvider>)
            }
            Err(e) => {
                warn!("❌ Failed to initialize local model: {}", e);
                None
            }
        };

        // Initialize cloud providers
        let mut cloud_providers: Vec<Arc<dyn ModelProvider>> = Vec::new();

        for cloud_config in &config.cloud_providers {
            match cloud_config.name.as_str() {
                "openai" => {
                    match OpenAIProvider::new(cloud_config.clone()) {
                        Ok(provider) => {
                            if provider.is_available() {
                                info!("✅ OpenAI provider initialized");
                                cloud_providers.push(Arc::new(provider));
                            } else {
                                warn!("⚠️  OpenAI provider created but not available (missing API key)");
                            }
                        }
                        Err(e) => warn!("❌ Failed to initialize OpenAI provider: {}", e),
                    }
                }
                "anthropic" => {
                    match AnthropicProvider::new(cloud_config.clone()) {
                        Ok(provider) => {
                            if provider.is_available() {
                                info!("✅ Anthropic provider initialized");
                                cloud_providers.push(Arc::new(provider));
                            } else {
                                warn!("⚠️  Anthropic provider created but not available (missing API key)");
                            }
                        }
                        Err(e) => warn!("❌ Failed to initialize Anthropic provider: {}", e),
                    }
                }
                "gemini" => {
                    match GeminiProvider::new(cloud_config.clone()) {
                        Ok(provider) => {
                            if provider.is_available() {
                                info!("✅ Gemini provider initialized");
                                cloud_providers.push(Arc::new(provider));
                            } else {
                                warn!("⚠️  Gemini provider created but not available (missing API key)");
                            }
                        }
                        Err(e) => warn!("❌ Failed to initialize Gemini provider: {}", e),
                    }
                }
                "openrouter" => {
                    match OpenRouterProvider::new(cloud_config.clone()) {
                        Ok(provider) => {
                            if provider.is_available() {
                                info!("✅ OpenRouter provider initialized");
                                cloud_providers.push(Arc::new(provider));
                            } else {
                                warn!("⚠️  OpenRouter provider created but not available (missing API key)");
                            }
                        }
                        Err(e) => warn!("❌ Failed to initialize OpenRouter provider: {}", e),
                    }
                }
                _ => warn!("Unknown cloud provider: {}", cloud_config.name),
            }
        }

        if local_provider.is_none() && cloud_providers.is_empty() {
            return Err(anyhow!("No providers available! Check your configuration."));
        }

        info!("Agent ready - Local: {}, Cloud: {}",
              local_provider.is_some(), cloud_providers.len());

        Ok(Self {
            local_provider,
            cloud_providers,
            config,
            tool_manager: ToolManager::new(),
            memory_manager,
            query_processor: QueryProcessor::new(),
            prompt_cache: Arc::new(Mutex::new(std::collections::HashMap::new())),
        })
    }

    // Public interface methods that delegate to appropriate modules
    pub async fn query_with_tools(&self, prompt: &str) -> Result<ModelResponse> {
        self.query_processor.query_with_tools(
            prompt,
            &self.local_provider,
            &self.cloud_providers,
            &self.tool_manager,
            &self.memory_manager,
            &self.config,
        ).await
    }

    pub async fn query_with_fallback(&self, prompt: &str) -> Result<ModelResponse> {
        self.query_processor.query_with_fallback(
            prompt,
            &self.local_provider,
            &self.cloud_providers,
            &self.memory_manager,
            &self.config,
        ).await
    }

    pub async fn query_local_only(&self, prompt: &str) -> Result<ModelResponse> {
        self.query_processor.query_local_only(
            prompt,
            &self.local_provider,
            &self.memory_manager,
            &self.config,
        ).await
    }

    pub async fn query_cloud_only(&self, prompt: &str) -> Result<ModelResponse> {
        self.query_processor.query_cloud_only(
            prompt,
            &self.cloud_providers,
            &self.memory_manager,
            &self.config,
        ).await
    }

    pub async fn query_pure_local(&self, prompt: &str) -> Result<ModelResponse> {
        self.query_processor.query_pure_local(
            prompt,
            &self.local_provider,
            &self.memory_manager,
            &self.config,
        ).await
    }

    // Memory management delegation
    pub fn store_conversations_batch(&self, conversations: Vec<(String, String, Option<String>, Option<String>)>) -> Result<()> {
        self.memory_manager.store_conversations_batch(conversations)
    }

    pub fn store_ram_memory(&self, key: &str, value: &str) -> Result<()> {
        self.memory_manager.store_ram_memory(key, value)
    }

    pub fn get_ram_memory(&self, key: &str) -> Result<Option<String>> {
        self.memory_manager.get_ram_memory(key)
    }

    pub fn store_persistent_memory(&self, key: &str, value: &str) -> Result<()> {
        self.memory_manager.store_persistent_memory(key, value)
    }

    pub fn get_persistent_memory(&self, key: &str) -> Result<Option<String>> {
        self.memory_manager.get_persistent_memory(key)
    }

    pub fn store_user_preference(&self, key: &str, value: &str) -> Result<()> {
        self.memory_manager.store_user_preference(key, value)
    }

    pub fn get_user_preference(&self, key: &str) -> Result<Option<String>> {
        self.memory_manager.get_user_preference(key)
    }

    pub fn get_air_info(&self, key: &str) -> Result<Option<String>> {
        self.memory_manager.get_air_info(key)
    }

    pub fn get_recent_conversations(&self, limit: usize) -> Result<Vec<(String, String, String)>> {
        self.memory_manager.get_recent_conversations(limit)
    }

    pub fn perform_maintenance(&self) -> Result<()> {
        self.memory_manager.perform_maintenance()
    }

    pub fn store_mistake(&self, session_id: &str, user_input: &str, ai_response: Option<&str>,
                        error_type: &str, error_message: &str, context: Option<&str>) -> Result<i64> {
        self.memory_manager.store_mistake(session_id, user_input, ai_response, error_type, error_message, context)
    }

    pub fn mark_mistake_learned(&self, mistake_id: i64) -> Result<()> {
        self.memory_manager.mark_mistake_learned(mistake_id)
    }

    pub fn get_unlearned_mistakes(&self, error_type: Option<&str>, limit: usize) -> Result<Vec<(i64, String, String, String, String)>> {
        self.memory_manager.get_unlearned_mistakes(error_type, limit)
    }

    pub fn update_learning_pattern(&self, pattern: &str, was_success: bool) -> Result<()> {
        self.memory_manager.update_learning_pattern(pattern, was_success)
    }

    pub fn get_learning_insights(&self, pattern: &str) -> Result<Option<(i32, i32, f64)>> {
        self.memory_manager.get_learning_insights(pattern)
    }

    pub fn get_mistake_insights(&self, prompt: &str) -> Result<Vec<String>> {
        self.memory_manager.get_mistake_insights(prompt)
    }

    pub fn record_query_error(&self, session_id: &str, user_input: &str, error: &anyhow::Error, context: Option<&str>) -> Result<()> {
        self.memory_manager.record_query_error(session_id, user_input, error, context)
    }

    pub fn build_enhanced_prompt(&self, base_prompt: &str) -> Result<String> {
        self.memory_manager.build_enhanced_prompt(base_prompt, &self.prompt_cache)
    }
}
