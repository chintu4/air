use crate::models::{ModelProvider, ModelResponse, QueryContext};
use crate::providers::{LocalLlamaProvider, OpenAIProvider, AnthropicProvider, GeminiProvider, OpenRouterProvider};
use crate::config::Config;
use crate::tools::ToolManager;
use anyhow::{Result, anyhow};
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn, debug};

pub struct AIAgent {
    local_provider: Option<Arc<dyn ModelProvider>>,
    cloud_providers: Vec<Arc<dyn ModelProvider>>,
    config: Config,
    tool_manager: ToolManager,
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
        let local_provider = match LocalLlamaProvider::new(config.local_model.clone()) {
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
                        
                        let mut ai_response = self.query_with_fallback(&enhanced_prompt).await?;
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
        self.query_with_fallback(prompt).await
    }
    
    /// Query with smart fallback: try local first, then cloud if needed
    pub async fn query_with_fallback(&self, prompt: &str) -> Result<ModelResponse> {
        info!("🔄 Processing query with smart fallback strategy");
        
        let context = QueryContext {
            prompt: prompt.to_string(),
            max_tokens: self.config.local_model.max_tokens,
            temperature: self.config.local_model.temperature,
            timeout: Duration::from_secs(self.config.performance.local_timeout_seconds),
            pure_mode: false,
        };
        
        // Strategy 1: Try local first for fast response
        if let Some(local_provider) = &self.local_provider {
            if local_provider.is_available() {
                info!("🏠 Trying local model first...");
                
                match tokio::time::timeout(
                    Duration::from_secs(self.config.performance.local_timeout_seconds),
                    local_provider.generate(&context)
                ).await {
                    Ok(Ok(mut response)) => {
                        info!("✅ Local model succeeded in {}ms", response.response_time_ms);
                        
                        // Check if we should also try cloud for comparison/quality
                        if self.should_try_cloud_for_quality(&response) {
                            info!("🌤️  Also trying cloud for potential quality improvement...");
                            if let Ok(cloud_response) = self.try_best_cloud_provider(&context).await {
                                if cloud_response.confidence_score.unwrap_or(0.0) > 
                                   response.confidence_score.unwrap_or(0.0) + 0.1 {
                                    info!("📈 Cloud provider gave significantly better response");
                                    return Ok(cloud_response);
                                }
                            }
                        }
                        
                        response.content = format!("🏠 Local Model Response:\n{}", response.content);
                        return Ok(response);
                    }
                    Ok(Err(e)) => {
                        warn!("❌ Local model failed: {}", e);
                    }
                    Err(_) => {
                        warn!("⏰ Local model timed out");
                    }
                }
            }
        }
        
        // Strategy 2: Fallback to cloud providers
        info!("🌤️  Falling back to cloud providers...");
        self.try_best_cloud_provider(&context).await
    }
    
    /// Force local model only
    pub async fn query_local_only(&self, prompt: &str) -> Result<ModelResponse> {
        let local_provider = self.local_provider.as_ref()
            .ok_or_else(|| anyhow!("Local provider not available"))?;
        
        if !local_provider.is_available() {
            return Err(anyhow!("Local model is not available"));
        }
        
        info!("🏠 Using local model only");
        
        let context = QueryContext {
            prompt: prompt.to_string(),
            max_tokens: self.config.local_model.max_tokens,
            temperature: self.config.local_model.temperature,
            timeout: Duration::from_secs(self.config.performance.local_timeout_seconds),
            pure_mode: false,
        };
        
        let mut response = local_provider.generate(&context).await?;
        response.content = format!("🏠 Local Model Response:\n{}", response.content);
        Ok(response)
    }
    
    /// Force cloud model only
    pub async fn query_cloud_only(&self, prompt: &str) -> Result<ModelResponse> {
        if self.cloud_providers.is_empty() {
            return Err(anyhow!("No cloud providers available"));
        }
        
        info!("🌤️  Using cloud models only");
        
        let context = QueryContext {
            prompt: prompt.to_string(),
            max_tokens: 1000, // Use higher limit for cloud
            temperature: 0.7,
            timeout: Duration::from_secs(30),
            pure_mode: false,
        };
        
        self.try_best_cloud_provider(&context).await
    }
    
    /// Force local model only with pure response (no templates)
    pub async fn query_pure_local(&self, prompt: &str) -> Result<ModelResponse> {
        let local_provider = self.local_provider.as_ref()
            .ok_or_else(|| anyhow!("Local provider not available"))?;
        
        if !local_provider.is_available() {
            return Err(anyhow!("Local model is not available"));
        }
        
        info!("🏠 Using local model in pure mode (no templates)");
        
        let context = QueryContext {
            prompt: prompt.to_string(),
            max_tokens: self.config.local_model.max_tokens,
            temperature: self.config.local_model.temperature,
            timeout: Duration::from_secs(self.config.performance.local_timeout_seconds),
            pure_mode: true,
        };
        
        local_provider.generate(&context).await
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
    
    fn should_try_cloud_for_quality(&self, local_response: &ModelResponse) -> bool {
        // Don't try cloud if local response was very fast and good enough
        if local_response.response_time_ms < self.config.performance.fallback_threshold_ms {
            if let Some(confidence) = local_response.confidence_score {
                if confidence >= self.config.performance.quality_threshold {
                    return false;
                }
            }
        }
        
        // For simple queries, prefer local
        if self.config.performance.prefer_local_for_simple_queries {
            let word_count = local_response.content.split_whitespace().count();
            if word_count < 50 { // Simple query/response
                return false;
            }
        }
        
        // Try cloud for complex queries or if we have fast cloud providers
        !self.cloud_providers.is_empty()
    }
}
