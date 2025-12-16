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

    /// Enhanced query with ReAct loop
    pub async fn query_with_tools(
        &self,
        prompt: &str,
        local_provider: &Option<Arc<dyn ModelProvider>>,
        cloud_providers: &[Arc<dyn ModelProvider>],
        tool_manager: &ToolManager,
        memory_manager: &MemoryManager,
        config: &Config,
    ) -> Result<ModelResponse> {
        info!("🔄 Starting ReAct loop");

        let mut current_prompt = prompt.to_string();
        let max_steps = 5;
        let mut steps = 0;
        let mut tool_history = String::new();

        // Add tool definitions to the context
        let tool_definitions = tool_manager.get_tool_definitions();
        let tool_context = format!("\nAvailable Tools:\n{}\n", serde_json::to_string_pretty(&tool_definitions)?);

        // We'll prepend this to the prompt internally in `query_with_fallback` via `memory_manager.build_enhanced_prompt`
        // But since we want to dynamically inject it, we might need a way to pass it down.
        // For now, let's append it to the prompt if it's the first step.
        // Actually, MemoryManager constructs the system prompt. We should probably update MemoryManager to accept tool defs,
        // but for now, let's append it to the user prompt to ensure the model sees it.
        current_prompt = format!("{}\n\n{}", tool_context, current_prompt);

        while steps < max_steps {
            steps += 1;
            info!("📍 ReAct Step {}/{}", steps, max_steps);

            // 1. Query the model
            let response = self.query_with_fallback(
                &current_prompt,
                local_provider,
                cloud_providers,
                memory_manager,
                config
            ).await?;

            // 2. Check for tool usage (JSON block)
            if let Some(tool_call) = self.extract_json_tool_call(&response.content) {
                info!("🛠️  Model requested tool: {}", tool_call.tool_name);

                // 3. Execute tool
                // Clone arguments for execution so we can still use tool_call later
                match tool_manager.execute_tool(
                    &tool_call.tool_name,
                    &tool_call.function,
                    tool_call.arguments.clone()
                ).await {
                    Ok(tool_result) => {
                        info!("✅ Tool execution successful");

                        let result_json = serde_json::to_string(&tool_result.result).unwrap_or_default();

                        // 4. Feed back to model
                        let tool_output = format!(
                            "\n\nTool '{}' (function '{}') executed.\nResult: {}\n\nBased on this result, continue.",
                            tool_call.tool_name,
                            tool_call.function,
                            result_json
                        );

                        tool_history.push_str(&format!("\nThought: {}\nAction: {}\nObservation: {}\n",
                            response.content, // Capture the model's thought process
                            serde_json::to_string(&tool_call).unwrap_or_default(),
                            result_json
                        ));

                        current_prompt.push_str(&tool_output);

                        // Loop continues to next iteration to let model process the result
                    },
                    Err(e) => {
                        warn!("❌ Tool execution failed: {}", e);
                        let error_msg = format!("\n\nTool execution failed: {}\n", e);
                        current_prompt.push_str(&error_msg);
                    }
                }
            } else {
                // No tool call detected, this is the final answer
                info!("🏁 Final response generated");
                return Ok(response);
            }
        }

        warn!("🛑 Max ReAct steps reached");
        // Return the last response
        self.query_with_fallback(&current_prompt, local_provider, cloud_providers, memory_manager, config).await
    }

    fn extract_json_tool_call(&self, content: &str) -> Option<crate::tools::ToolCall> {
        // Look for JSON block ```json ... ``` or just { ... }
        // Simple extraction logic

        let json_str = if let Some(start) = content.find("```json") {
            if let Some(end) = content[start..].find("```") {
                 // Determine end of block (start + len("```json") ... start + end)
                 // Wait, find("```") returns index relative to start.
                 // We need the *second* ``` which closes the block.
                 // content[start..] starts with ```json.
                 // We need to find the closing ```
                 let code_block = &content[start+7..]; // skip ```json
                 if let Some(end_rel) = code_block.find("```") {
                     &code_block[..end_rel]
                 } else {
                     return None;
                 }
            } else {
                return None;
            }
        } else if let Some(start) = content.find('{') {
             if let Some(end) = content.rfind('}') {
                 if end > start {
                     &content[start..=end]
                 } else {
                     return None;
                 }
             } else {
                 return None;
             }
        } else {
            return None;
        };

        // Try to parse as ToolCall
        // Expected format: {"tool": "name", "function": "func", "args": {...}}
        #[derive(serde::Deserialize)]
        struct RawToolCall {
            tool: String,
            function: String,
            args: serde_json::Value,
        }

        if let Ok(raw) = serde_json::from_str::<RawToolCall>(json_str) {
            Some(crate::tools::ToolCall {
                tool_name: raw.tool,
                function: raw.function,
                arguments: raw.args,
            })
        } else {
            None
        }
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
        let enhanced_prompt = memory_manager.build_enhanced_prompt(prompt, &Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()))).await?;
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
        let enhanced_prompt = memory_manager.build_enhanced_prompt(prompt, &Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()))).await?;
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
        let enhanced_prompt = memory_manager.build_enhanced_prompt(prompt, &Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()))).await?;
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
        let enhanced_prompt = memory_manager.build_enhanced_prompt(prompt, &Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()))).await?;
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
        if let Ok(recent_convs) = memory_manager.get_recent_conversations(10).await {
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
