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
    
    /// ReAct-style query loop (Think -> Act -> Observe)
    pub async fn query_with_tools(&self, prompt: &str) -> Result<ModelResponse> {
        info!("🧠 Starting ReAct workflow for: {}", prompt);

        let system_prompt = r#"You are AIR, an intelligent AI agent that uses a ReAct (Reasoning and Acting) workflow.
You have access to the following tools:

1. filesystem
   - read_file(path: str): Read content of a file
   - write_file(path: str, content: str): Create or overwrite a file
   - list_directory(path: str): List files in a directory
2. calculator
   - calculate(expression: str): Evaluate a math expression
3. web
   - fetch(url: str): Fetch content from a URL
4. command
   - execute(command: str): Run a shell command
5. planner
   - create_task(title: str, description: str): Create a new task
6. memory
   - search_conversations(query: str): Search past conversations

To solve a problem, you must interleave Thought, Action, and Observation.
Use the following format:

Thought: [Your reasoning about what to do next]
Action: [A JSON object with "tool", "function", and "arguments"]
Observation: [The result of the tool - I will provide this]
... (repeat Thought/Action/Observation as needed)
Final Answer: [Your final response to the user]

Example Action:
Action: { "tool": "filesystem", "function": "read_file", "arguments": { "path": "test.txt" } }

If you can answer directly without tools, just provide the Final Answer.
Start your response with 'Thought:'.
"#;

        let mut full_history = format!("{}\n\nUser Query: {}\n", system_prompt, prompt);
        let mut total_tokens = 0;
        let mut total_time = 0;
        let max_steps = 10;
        
        for step in 0..max_steps {
            debug!("🔄 ReAct Step {}/{}", step + 1, max_steps);

            // 1. Ask LLM for Thought/Action
            let response = self.query(&full_history).await?;
            total_tokens += response.tokens_used;
            total_time += response.response_time_ms;

            let content = response.content.trim();
            info!("🤖 AI Step {}: {}", step + 1, content);
            
            // Append AI response to history
            full_history.push_str("\n");
            full_history.push_str(content);

            // 2. Check for Final Answer
            if content.contains("Final Answer:") {
                let answer = content.split("Final Answer:").last().unwrap_or(content).trim();
                return Ok(ModelResponse {
                    content: answer.to_string(),
                    model_used: response.model_used,
                    tokens_used: total_tokens,
                    response_time_ms: total_time,
                    confidence_score: response.confidence_score,
                });
            }

            // 3. Extract Action (JSON)
            // Look for "Action: { ... }" block
            let action_json = if let Some(start_idx) = content.find("Action: {") {
                 // Basic bracket counting to find end of JSON
                 let json_start = start_idx + 8; // skip "Action: "
                 let rest = &content[json_start..];
                 let mut depth = 0;
                 let mut end_idx = 0;
                 for (i, c) in rest.chars().enumerate() {
                     if c == '{' { depth += 1; }
                     if c == '}' { depth -= 1; }
                     if depth == 0 {
                         end_idx = i + 1;
                         break;
                     }
                 }

                 if end_idx > 0 {
                     Some(&rest[..end_idx])
                 } else {
                     None
                 }
            } else {
                // Try finding just a JSON block if "Action:" prefix is missing or malformed
                 if let Some(start) = content.find('{') {
                     if let Some(end) = content.rfind('}') {
                         if end > start {
                             Some(&content[start..=end])
                         } else { None }
                     } else { None }
                 } else { None }
            };

            if let Some(json_str) = action_json {
                // 4. Execute Tool
                match serde_json::from_str::<serde_json::Value>(json_str) {
                    Ok(tool_call) => {
                        if let (Some(tool), Some(func), Some(args)) = (
                            tool_call.get("tool").and_then(|v| v.as_str()),
                            tool_call.get("function").and_then(|v| v.as_str()),
                            tool_call.get("arguments")
                        ) {
                            info!("🛠️ Executing: {} -> {}", tool, func);
                            let result = self.tool_manager.execute_tool(tool, func, args.clone()).await;

                            let observation = match result {
                                Ok(res) => format!("Observation: {}", res.result),
                                Err(e) => format!("Observation: Error: {}", e),
                            };

                            info!("👀 {}", observation);
                            full_history.push_str("\n");
                            full_history.push_str(&observation);
                        } else {
                            let msg = "Observation: Error: Invalid JSON structure. Need tool, function, arguments.";
                            full_history.push_str("\n");
                            full_history.push_str(msg);
                        }
                    },
                    Err(e) => {
                        // JSON parse error
                        let msg = format!("Observation: Error parsing JSON: {}", e);
                        full_history.push_str("\n");
                        full_history.push_str(&msg);
                    }
                }
            } else {
                // No action found, but no final answer?
                // If the AI is just "thinking" without acting, let it continue, but warn if loop
                if !content.contains("Thought:") {
                     // Force it to conclude if it's rambling
                     full_history.push_str("\nObservation: Please provide an Action or Final Answer.");
                } else {
                    // Just a thought, assume it's building up to an action in next turn?
                    // Actually, typically Thought and Action come together.
                    // If we see Thought but no Action, prompt for Action.
                    full_history.push_str("\nObservation: You provided a Thought. Now please provide an Action or Final Answer.");
                }
            }
        }
        
        // If loop exhausted
        Ok(ModelResponse {
            content: "I pondered the problem for too long and reached the step limit.".to_string(),
            model_used: "System".to_string(),
            tokens_used: total_tokens,
            response_time_ms: total_time,
            confidence_score: Some(0.0),
        })
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
                Ok(response) => {
                    info!("✅ {} succeeded in {}ms", provider.name(), response.response_time_ms);
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
