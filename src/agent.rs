use crate::models::{ModelProvider, ModelResponse, QueryContext};
use crate::providers::{OpenAIProvider, AnthropicProvider, GeminiProvider, OpenRouterProvider, LocalProvider};
use crate::config::Config;
use crate::tools::ToolManager;
use anyhow::{Result, anyhow};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn, debug};

pub struct AIAgent {
    local_provider: Option<Arc<dyn ModelProvider>>,
    cloud_providers: Vec<Arc<dyn ModelProvider>>,
    config: Config,
    tool_manager: ToolManager,
    successful_queries: Arc<Mutex<u32>>,
    failed_queries: Arc<Mutex<u32>>,
}

impl std::fmt::Debug for AIAgent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AIAgent")
            .field("local_provider", &self.local_provider.is_some())
            .field("cloud_providers_count", &self.cloud_providers.len())
            .field("config", &self.config)
            .field("tool_manager", &"ToolManager")
            .finish()
    }
}

impl AIAgent {
    pub async fn new(config: Config) -> Result<Self> {
        info!("Initializing AI Agent...");
        
        // Initialize local provider
        let local_provider: Option<Arc<dyn ModelProvider>> = match LocalProvider::new(config.local_model.clone()) {
            Ok(provider) => {
                // Check availability (does file exist?)
                if provider.is_available() {
                    info!("✅ Local provider initialized");
                    Some(Arc::new(provider))
                } else {
                    info!("ℹ️  Local provider configured but model file not found (run 'setup --local' to install)");
                    None
                }
            },
            Err(e) => {
                warn!("❌ Failed to initialize local provider: {}", e);
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
        
        if cloud_providers.is_empty() && local_provider.is_none() {
            // It's okay if cloud providers are empty if we have local, but warn if nothing.
            warn!("⚠️  No providers available! Run 'air setup --local' or configure API keys.");
        }
        
        info!("Agent ready - Local: {}, Cloud: {}", local_provider.is_some(), cloud_providers.len());
        
        Ok(Self {
            local_provider,
            cloud_providers,
            config,
            tool_manager: ToolManager::new(),
            successful_queries: Arc::new(Mutex::new(0)),
            failed_queries: Arc::new(Mutex::new(0)),
        })
    }
    
    /// Enhanced query with tool detection and execution
    pub async fn query_with_tools(&self, prompt: &str) -> Result<ModelResponse> {
        info!("🔧 Processing query with tool detection");
        
        // First, check if this query should use tools
        if let Some((tool_name, function, args)) = self.tool_manager.detect_tool_intent(prompt) {
            info!("🎯 Detected tool usage: {} -> {}", tool_name, function);
            
            match self.tool_manager.execute_tool(&tool_name, &function, args).await {
                Ok(tool_result) => {
                    if tool_result.success {
                        info!("✅ Tool execution successful");
                        
                        // For simple tool results, return directly
                        if tool_name == "web" || tool_name == "filesystem" {
                            let mut successful_queries = self.successful_queries.lock().await;
                            *successful_queries += 1;
                            return Ok(ModelResponse {
                                content: format!("🔧 Tool Result ({}): \n\n{}", tool_name, tool_result.result),
                                model_used: format!("Tool-{}", tool_name),
                                tokens_used: 0,
                                response_time_ms: 0,
                                confidence_score: Some(1.0),
                            });
                        }
                        
                        // For other tools, combine with AI response
                        let enhanced_prompt = format!(
                            "Based on this tool result: {}\n\nOriginal query: {}\n\nPlease provide a helpful response:",
                            tool_result.result, prompt
                        );
                        
                        let mut ai_response = self.query(&enhanced_prompt).await?;
                        ai_response.content = format!(
                            "🔧 Tool Result:\n{}\n\n🤖 AI Analysis:\n{}", 
                            tool_result.result, 
                            ai_response.content
                        );
                        return Ok(ai_response);
                        
                    } else {
                        warn!("❌ Tool execution failed: {}", tool_result.result);
                        // Fall through to regular AI processing
                    }
                }
                Err(e) => {
                    warn!("❌ Tool execution error: {}", e);
                    // Fall through to regular AI processing
                }
            }
        }
        
        // No tool detected or tool failed, use regular AI processing
        self.query(prompt).await
    }
    
    /// Query the best available provider (local or cloud)
    pub async fn query(&self, prompt: &str) -> Result<ModelResponse> {
        // Try local first if configured or fallback to cloud
        // Logic:
        // 1. If local is available, use it (fast, free, private).
        // 2. If local fails or not available, use cloud.
        
        // TODO: Implement "smart routing" (use cloud for complex tasks)
        // For now, simple priority: Local > Cloud
        
        let context = QueryContext {
            prompt: prompt.to_string(),
            max_tokens: 1000,
            temperature: 0.7,
            timeout: std::time::Duration::from_secs(60), // Longer timeout for local
            pure_mode: false,
        };

        if let Some(local) = &self.local_provider {
            if local.is_available() {
                info!("🏠 Using local model");
                match local.generate(&context).await {
                    Ok(response) => return Ok(response),
                    Err(e) => {
                        warn!("❌ Local inference failed: {}. Falling back to cloud.", e);
                    }
                }
            }
        }

        if self.cloud_providers.is_empty() {
            return Err(anyhow!("No cloud providers available and local failed/missing."));
        }
        
        info!("🌤️  Using cloud models");
        self.try_best_cloud_provider(&context).await
    }
    
    async fn try_best_cloud_provider(&self, context: &QueryContext) -> Result<ModelResponse> {
        if self.cloud_providers.is_empty() {
            return Err(anyhow!("No cloud providers available"));
        }
        
        // Sort providers by quality score and availability
        let mut available_providers: Vec<_> = self.cloud_providers.iter()
            .filter(|p| p.is_available())
            .collect();
        
        if available_providers.is_empty() {
            return Err(anyhow!("No cloud providers are available (check API keys)"));
        }
        
        available_providers.sort_by(|a, b| 
            b.quality_score().partial_cmp(&a.quality_score()).unwrap_or(std::cmp::Ordering::Equal)
        );
        
        // Try providers in order of quality
        for provider in available_providers {
            debug!("Trying cloud provider: {}", provider.name());
            match provider.generate(context).await {
                Ok(mut response) => {
                    info!("✅ {} succeeded in {}ms", provider.name(), response.response_time_ms);
                    response.content = format!("☁️  {} Response:\n{}", provider.name(), response.content);
                    return Ok(response);
                }
                Err(e) => {
                    warn!("❌ {} failed: {}", provider.name(), e);
                    continue;
                }
            }
        }
        
        Err(anyhow!("All cloud providers failed"))
    }
}
