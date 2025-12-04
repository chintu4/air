use crate::models::{ModelProvider, ModelResponse, QueryContext};
use crate::config::Config;
use crate::tools::ToolManager;
use crate::agent::memory::MemoryManager;
use anyhow::{Result, anyhow};
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn, debug};
use futures;

#[derive(Debug, Clone)]
pub enum QueryMode {
    Auto,       // Smart fallback (default)
    LocalOnly,  // Force local model
    CloudOnly,  // Force cloud model
    PureLocal,  // Pure local model without templates
}

#[derive(Debug, Clone)]
pub struct QueryRequest {
    pub prompt: String,
    pub mode: QueryMode,
    pub context: QueryContext,
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct QueryResponse {
    pub content: String,
    pub tool_results: Vec<crate::tools::ToolResult>,
    pub model_used: String,
    pub processing_time: Duration,
    pub confidence: Option<f64>,
}

pub struct QueryProcessor;

impl QueryProcessor {
    pub fn new() -> Self {
        Self
    }

    /// Enhanced query with tool detection and execution
    pub async fn query_with_tools(
        &self,
        prompt: &str,
        local_provider: &Option<Arc<dyn ModelProvider>>,
        cloud_providers: &[Arc<dyn ModelProvider>],
        tool_manager: &ToolManager,
        memory_manager: &MemoryManager,
        config: &Config,
    ) -> Result<ModelResponse> {
        info!("🔧 Processing query with tool detection");

        // First, check if this query should use tools
        if let Some((tool_name, function, args)) = tool_manager.detect_tool_intent(prompt) {
            info!("🎯 Detected tool usage: {} -> {}", tool_name, function);

            match tool_manager.execute_tool(&tool_name, &function, args).await {
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

                        let mut ai_response = self.query_with_fallback(&enhanced_prompt, local_provider, cloud_providers, memory_manager, config).await?;
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
        self.query_with_fallback(prompt, local_provider, cloud_providers, memory_manager, config).await
    }

    /// Query with smart fallback: try local first, then cloud if needed
    pub async fn query_with_fallback(
        &self,
        prompt: &str,
        local_provider: &Option<Arc<dyn ModelProvider>>,
        cloud_providers: &[Arc<dyn ModelProvider>],
        memory_manager: &MemoryManager,
        config: &Config,
    ) -> Result<ModelResponse> {
        info!("🔄 Processing query with smart fallback strategy");

        // Build enhanced prompt with context
        let enhanced_prompt = memory_manager.build_enhanced_prompt(prompt, &Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())))?;
        info!("📝 Enhanced prompt length: {} characters", enhanced_prompt.len());

        let context = QueryContext {
            prompt: enhanced_prompt,
            max_tokens: config.local_model.max_tokens,
            temperature: config.local_model.temperature,
            timeout: Duration::from_secs(config.performance.local_timeout_seconds),
            pure_mode: false,
        };

        // Strategy 1: Try local first for fast response
        if let Some(local_provider) = local_provider {
            if local_provider.is_available() {
                info!("🏠 Trying local model first...");

                match tokio::time::timeout(
                    Duration::from_secs(config.performance.local_timeout_seconds),
                    local_provider.generate(&context)
                ).await {
                    Ok(Ok(mut response)) => {
                        info!("✅ Local model succeeded in {}ms", response.response_time_ms);

                        // Check if we should also try cloud for comparison/quality
                        if self.should_try_cloud_for_quality(&response) {
                            info!("🌤️  Also trying cloud for potential quality improvement...");
                            if let Ok(cloud_response) = self.try_best_cloud_provider(&context, cloud_providers).await {
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
        match self.try_best_cloud_provider(&context, cloud_providers).await {
            Ok(response) => Ok(response),
            Err(e) => {
                warn!("❌ All providers failed: {}", e);
                // Graceful degradation: try to provide a cached/default response
                self.provide_graceful_fallback(prompt, memory_manager).await
            }
        }
    }

    /// Force local model only
    pub async fn query_local_only(
        &self,
        prompt: &str,
        local_provider: &Option<Arc<dyn ModelProvider>>,
        memory_manager: &MemoryManager,
        config: &Config,
    ) -> Result<ModelResponse> {
        let local_provider = local_provider.as_ref()
            .ok_or_else(|| anyhow!("Local provider not available"))?;

        if !local_provider.is_available() {
            return Err(anyhow!("Local model is not available"));
        }

        info!("🏠 Using local model only");

        // Build enhanced prompt with context
        let enhanced_prompt = memory_manager.build_enhanced_prompt(prompt, &Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())))?;
        info!("📝 Enhanced prompt length: {} characters", enhanced_prompt.len());

        let context = QueryContext {
            prompt: enhanced_prompt,
            max_tokens: config.local_model.max_tokens,
            temperature: config.local_model.temperature,
            timeout: Duration::from_secs(config.performance.local_timeout_seconds),
            pure_mode: false,
        };

        let mut response = local_provider.generate(&context).await?;
        response.content = format!("🏠 Local Model Response:\n{}", response.content);
        Ok(response)
    }

    /// Force cloud model only
    pub async fn query_cloud_only(
        &self,
        prompt: &str,
        cloud_providers: &[Arc<dyn ModelProvider>],
        memory_manager: &MemoryManager,
        _config: &Config,
    ) -> Result<ModelResponse> {
        if cloud_providers.is_empty() {
            return Err(anyhow!("No cloud providers available"));
        }

        info!("🌤️  Using cloud models only");

        // Build enhanced prompt with context
        let enhanced_prompt = memory_manager.build_enhanced_prompt(prompt, &Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())))?;
        info!("📝 Enhanced prompt length: {} characters", enhanced_prompt.len());

        let context = QueryContext {
            prompt: enhanced_prompt,
            max_tokens: 1000, // Use higher limit for cloud
            temperature: 0.7,
            timeout: Duration::from_secs(30),
            pure_mode: false,
        };

        self.try_best_cloud_provider(&context, cloud_providers).await
    }

    /// Force local model only with pure response (no templates)
    pub async fn query_pure_local(
        &self,
        prompt: &str,
        local_provider: &Option<Arc<dyn ModelProvider>>,
        memory_manager: &MemoryManager,
        config: &Config,
    ) -> Result<ModelResponse> {
        let local_provider = local_provider.as_ref()
            .ok_or_else(|| anyhow!("Local provider not available"))?;

        if !local_provider.is_available() {
            return Err(anyhow!("Local model is not available"));
        }

        info!("🏠 Using local model in pure mode (no templates)");

        // Build enhanced prompt with context (minimal for pure mode)
        let enhanced_prompt = memory_manager.build_enhanced_prompt(prompt, &Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())))?;
        info!("📝 Enhanced prompt length: {} characters", enhanced_prompt.len());

        let context = QueryContext {
            prompt: enhanced_prompt,
            max_tokens: config.local_model.max_tokens,
            temperature: config.local_model.temperature,
            timeout: Duration::from_secs(config.performance.local_timeout_seconds),
            pure_mode: true,
        };

        local_provider.generate(&context).await
    }

    async fn try_best_cloud_provider(&self, context: &QueryContext, cloud_providers: &[Arc<dyn ModelProvider>]) -> Result<ModelResponse> {
        if cloud_providers.is_empty() {
            return Err(anyhow!("No cloud providers available"));
        }

        // Sort providers by quality score and availability
        let mut available_providers: Vec<_> = cloud_providers.iter()
            .filter(|p| p.is_available())
            .collect();

        if available_providers.is_empty() {
            return Err(anyhow!("No cloud providers are available (check API keys)"));
        }

        available_providers.sort_by(|a, b|
            b.quality_score().partial_cmp(&a.quality_score()).unwrap_or(std::cmp::Ordering::Equal)
        );

        // Try top 2 providers in parallel for faster response
        if available_providers.len() >= 2 {
            let provider1 = available_providers[0].clone();
            let provider2 = available_providers[1].clone();
            let context1 = context.clone();
            let context2 = context.clone();

            let (result1, result2) = futures::join!(
                self.try_provider_with_retry(&provider1, &context1),
                self.try_provider_with_retry(&provider2, &context2)
            );

            // Return the first successful result
            if let Ok(mut response) = result1 {
                info!("✅ {} succeeded in {}ms (parallel)", provider1.name(), response.response_time_ms);
                response.content = format!("☁️  {} Response:\n{}", provider1.name(), response.content);
                return Ok(response);
            }

            if let Ok(mut response) = result2 {
                info!("✅ {} succeeded in {}ms (parallel)", provider2.name(), response.response_time_ms);
                response.content = format!("☁️  {} Response:\n{}", provider2.name(), response.content);
                return Ok(response);
            }
        }

        // Fallback to sequential for remaining providers
        for provider in available_providers.iter().skip(if available_providers.len() >= 2 { 2 } else { 0 }) {
            debug!("Trying cloud provider: {}", provider.name());

            match self.try_provider_with_retry(provider, context).await {
                Ok(mut response) => {
                    info!("✅ {} succeeded in {}ms", provider.name(), response.response_time_ms);
                    response.content = format!("☁️  {} Response:\n{}", provider.name(), response.content);
                    return Ok(response);
                }
                Err(e) => {
                    warn!("❌ {} failed after retries: {}", provider.name(), e);
                    continue;
                }
            }
        }

        Err(anyhow!("All cloud providers failed"))
    }

    /// Try a provider with exponential backoff retry logic
    async fn try_provider_with_retry(&self, provider: &Arc<dyn ModelProvider>, context: &QueryContext) -> Result<ModelResponse> {
        let max_retries = 3;
        let mut delay_ms = 1000; // Start with 1 second

        for attempt in 0..max_retries {
            match provider.generate(context).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    if attempt < max_retries - 1 {
                        warn!("⚠️  {} attempt {} failed: {}. Retrying in {}ms...",
                              provider.name(), attempt + 1, e, delay_ms);
                        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                        delay_ms *= 2; // Exponential backoff
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        Err(anyhow!("Max retries exceeded for {}", provider.name()))
    }

    /// Provide graceful fallback when all providers fail
    async fn provide_graceful_fallback(&self, prompt: &str, memory_manager: &MemoryManager) -> Result<ModelResponse> {
        // Try to find similar past responses
        if let Ok(recent_convs) = memory_manager.get_recent_conversations(10) {
            for (user_input, ai_response, _) in recent_convs {
                if self.is_similar_query(prompt, &user_input) {
                    info!("📋 Found similar past response, using as fallback");
                    return Ok(ModelResponse {
                        content: format!("⚠️  Service temporarily unavailable. Here's a similar response from our conversation history:\n\n{}", ai_response),
                        model_used: "Fallback-Cache".to_string(),
                        tokens_used: 0,
                        response_time_ms: 0,
                        confidence_score: Some(0.5),
                    });
                }
            }
        }

        // Default fallback response
        Ok(ModelResponse {
            content: format!("⚠️  I'm currently experiencing connectivity issues. Please try again in a moment.\n\nYour query was: '{}'\n\nFor urgent matters, you can also try:\n• Using 'mode local' to force local processing\n• Checking your internet connection\n• Verifying API keys in your configuration", prompt),
            model_used: "Fallback-Default".to_string(),
            tokens_used: 0,
            response_time_ms: 0,
            confidence_score: Some(0.1),
        })
    }

    /// Check if two queries are similar (simple implementation)
    fn is_similar_query(&self, query1: &str, query2: &str) -> bool {
        let q1_lower = query1.to_lowercase();
        let q1_words: std::collections::HashSet<&str> = q1_lower.split_whitespace().collect();
        let q2_lower = query2.to_lowercase();
        let q2_words: std::collections::HashSet<&str> = q2_lower.split_whitespace().collect();

        let intersection = q1_words.intersection(&q2_words).count();
        let union = q1_words.union(&q2_words).count();

        if union == 0 {
            return false;
        }

        let similarity = intersection as f64 / union as f64;
        similarity > 0.6 // 60% word overlap
    }

    /// Check if we should try cloud for quality improvement
    fn should_try_cloud_for_quality(&self, response: &ModelResponse) -> bool {
        // Try cloud if local confidence is low or response seems incomplete
        response.confidence_score.unwrap_or(0.0) < 0.7 ||
        response.content.len() < 50 ||
        response.content.contains("I'm not sure") ||
        response.content.contains("I don't know")
    }
}
