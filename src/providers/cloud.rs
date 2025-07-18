use crate::models::{ModelProvider, ModelResponse, QueryContext, ModelMetrics};
use crate::config::CloudProviderConfig;
use async_trait::async_trait;
use anyhow::{Result, anyhow};
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Instant;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{warn, error, debug};

pub struct OpenAIProvider {
    config: CloudProviderConfig,
    client: Client,
    metrics: Arc<Mutex<ModelMetrics>>,
}

impl OpenAIProvider {
    pub fn new(config: CloudProviderConfig) -> Result<Self> {
        if config.api_key.is_none() {
            warn!("OpenAI API key not provided, provider will be unavailable");
        }
        
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()?;
            
        Ok(Self {
            config,
            client,
            metrics: Arc::new(Mutex::new(ModelMetrics::default())),
        })
    }
}

#[async_trait]
impl ModelProvider for OpenAIProvider {
    async fn generate(&self, context: &QueryContext) -> Result<ModelResponse> {
        let api_key = self.config.api_key.as_ref()
            .ok_or_else(|| anyhow!("OpenAI API key not configured"))?;
            
        let start = Instant::now();
        let mut metrics = self.metrics.lock().await;
        
        debug!("Sending request to OpenAI API");
        
        let payload = json!({
            "model": self.config.model,
            "messages": [
                {
                    "role": "user",
                    "content": context.prompt
                }
            ],
            "max_tokens": context.max_tokens,
            "temperature": context.temperature
        });
        
        let response = self.client
            .post(&format!("{}/chat/completions", self.config.base_url))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await;
            
        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    let response_json: Value = resp.json().await?;
                    let content = response_json["choices"][0]["message"]["content"]
                        .as_str()
                        .unwrap_or("No response content")
                        .to_string();
                    
                    let tokens_used = response_json["usage"]["total_tokens"]
                        .as_u64()
                        .unwrap_or(0) as u32;
                    
                    let response_time = start.elapsed().as_millis() as u64;
                    metrics.record_success(response_time);
                    
                    Ok(ModelResponse {
                        content,
                        model_used: format!("OpenAI-{}", self.config.model),
                        tokens_used,
                        response_time_ms: response_time,
                        confidence_score: Some(0.95), // OpenAI models typically high quality
                    })
                } else {
                    let error_msg = format!("OpenAI API error: {}", resp.status());
                    error!("{}", error_msg);
                    metrics.record_failure(error_msg.clone());
                    Err(anyhow!(error_msg))
                }
            }
            Err(e) => {
                let error_msg = format!("OpenAI request failed: {}", e);
                error!("{}", error_msg);
                metrics.record_failure(error_msg.clone());
                Err(anyhow!(error_msg))
            }
        }
    }
    
    fn name(&self) -> &str {
        "OpenAI"
    }
    
    fn is_available(&self) -> bool {
        self.config.api_key.is_some()
    }
    
    fn estimated_latency_ms(&self) -> u64 {
        1500 // Typical cloud API latency
    }
    
    fn quality_score(&self) -> f32 {
        0.95 // High quality responses
    }
}

pub struct AnthropicProvider {
    config: CloudProviderConfig,
    client: Client,
    metrics: Arc<Mutex<ModelMetrics>>,
}

impl AnthropicProvider {
    pub fn new(config: CloudProviderConfig) -> Result<Self> {
        if config.api_key.is_none() {
            warn!("Anthropic API key not provided, provider will be unavailable");
        }
        
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()?;
            
        Ok(Self {
            config,
            client,
            metrics: Arc::new(Mutex::new(ModelMetrics::default())),
        })
    }
}

#[async_trait]
impl ModelProvider for AnthropicProvider {
    async fn generate(&self, context: &QueryContext) -> Result<ModelResponse> {
        let api_key = self.config.api_key.as_ref()
            .ok_or_else(|| anyhow!("Anthropic API key not configured"))?;
            
        let start = Instant::now();
        let mut metrics = self.metrics.lock().await;
        
        debug!("Sending request to Anthropic API");
        
        let payload = json!({
            "model": self.config.model,
            "max_tokens": context.max_tokens,
            "temperature": context.temperature,
            "messages": [
                {
                    "role": "user",
                    "content": context.prompt
                }
            ]
        });
        
        let response = self.client
            .post(&format!("{}/v1/messages", self.config.base_url))
            .header("x-api-key", api_key)
            .header("Content-Type", "application/json")
            .header("anthropic-version", "2023-06-01")
            .json(&payload)
            .send()
            .await;
            
        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    let response_json: Value = resp.json().await?;
                    let content = response_json["content"][0]["text"]
                        .as_str()
                        .unwrap_or("No response content")
                        .to_string();
                    
                    let tokens_used = response_json["usage"]["output_tokens"]
                        .as_u64()
                        .unwrap_or(0) as u32;
                    
                    let response_time = start.elapsed().as_millis() as u64;
                    metrics.record_success(response_time);
                    
                    Ok(ModelResponse {
                        content,
                        model_used: format!("Anthropic-{}", self.config.model),
                        tokens_used,
                        response_time_ms: response_time,
                        confidence_score: Some(0.93),
                    })
                } else {
                    let error_msg = format!("Anthropic API error: {}", resp.status());
                    error!("{}", error_msg);
                    metrics.record_failure(error_msg.clone());
                    Err(anyhow!(error_msg))
                }
            }
            Err(e) => {
                let error_msg = format!("Anthropic request failed: {}", e);
                error!("{}", error_msg);
                metrics.record_failure(error_msg.clone());
                Err(anyhow!(error_msg))
            }
        }
    }
    
    fn name(&self) -> &str {
        "Anthropic"
    }
    
    fn is_available(&self) -> bool {
        self.config.api_key.is_some()
    }
    
    fn estimated_latency_ms(&self) -> u64 {
        1200 // Typically fast
    }
    
    fn quality_score(&self) -> f32 {
        0.93 // High quality responses
    }
}

pub struct GeminiProvider {
    config: CloudProviderConfig,
    client: Client,
    metrics: Arc<Mutex<ModelMetrics>>,
}

impl GeminiProvider {
    pub fn new(config: CloudProviderConfig) -> Result<Self> {
        if config.api_key.is_none() {
            warn!("Gemini API key not provided, provider will be unavailable");
        }
        
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()?;
            
        Ok(Self {
            config,
            client,
            metrics: Arc::new(Mutex::new(ModelMetrics::default())),
        })
    }
}

#[async_trait]
impl ModelProvider for GeminiProvider {
    async fn generate(&self, context: &QueryContext) -> Result<ModelResponse> {
        let api_key = self.config.api_key.as_ref()
            .ok_or_else(|| anyhow!("Gemini API key not configured"))?;
            
        let start = Instant::now();
        let mut metrics = self.metrics.lock().await;
        
        debug!("Sending request to Gemini API");
        
        let payload = json!({
            "contents": [{
                "parts": [{
                    "text": context.prompt
                }]
            }],
            "generationConfig": {
                "temperature": context.temperature,
                "maxOutputTokens": context.max_tokens,
                "candidateCount": 1
            }
        });
        
        let url = format!("{}/v1beta/models/{}:generateContent?key={}", 
                         self.config.base_url, self.config.model, api_key);
        
        let response = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await;
            
        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    let response_json: Value = resp.json().await?;
                    
                    let content = response_json["candidates"][0]["content"]["parts"][0]["text"]
                        .as_str()
                        .unwrap_or("No response content")
                        .to_string();
                    
                    // Gemini doesn't always return token usage, estimate based on content length
                    let tokens_used = (content.len() / 4) as u32; // Rough estimation
                    
                    let response_time = start.elapsed().as_millis() as u64;
                    metrics.record_success(response_time);
                    
                    Ok(ModelResponse {
                        content,
                        model_used: format!("Gemini-{}", self.config.model),
                        tokens_used,
                        response_time_ms: response_time,
                        confidence_score: Some(0.92),
                    })
                } else {
                    let status_code = resp.status();
                    let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                    let error_msg = format!("Gemini API error: {} - {}", status_code, error_text);
                    error!("{}", error_msg);
                    metrics.record_failure(error_msg.clone());
                    Err(anyhow!(error_msg))
                }
            }
            Err(e) => {
                let error_msg = format!("Gemini request failed: {}", e);
                error!("{}", error_msg);
                metrics.record_failure(error_msg.clone());
                Err(anyhow!(error_msg))
            }
        }
    }
    
    fn name(&self) -> &str {
        "Gemini"
    }
    
    fn is_available(&self) -> bool {
        self.config.api_key.is_some()
    }
    
    fn estimated_latency_ms(&self) -> u64 {
        1000 // Gemini is typically fast
    }
    
    fn quality_score(&self) -> f32 {
        0.92 // High quality responses, slightly lower than GPT-4 but very competitive
    }
}

pub struct OpenRouterProvider {
    config: CloudProviderConfig,
    client: Client,
    metrics: Arc<Mutex<ModelMetrics>>,
}

impl OpenRouterProvider {
    pub fn new(config: CloudProviderConfig) -> Result<Self> {
        if config.api_key.is_none() {
            warn!("OpenRouter API key not provided, provider will be unavailable");
        }
        
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()?;
            
        Ok(Self {
            config,
            client,
            metrics: Arc::new(Mutex::new(ModelMetrics::default())),
        })
    }
}

#[async_trait]
impl ModelProvider for OpenRouterProvider {
    async fn generate(&self, context: &QueryContext) -> Result<ModelResponse> {
        let api_key = self.config.api_key.as_ref()
            .ok_or_else(|| anyhow!("OpenRouter API key not configured"))?;
            
        let start = Instant::now();
        let mut metrics = self.metrics.lock().await;
        
        debug!("Sending request to OpenRouter API");
        
        let payload = json!({
            "model": self.config.model,
            "messages": [
                {
                    "role": "user",
                    "content": context.prompt
                }
            ],
            "max_tokens": context.max_tokens,
            "temperature": context.temperature,
            "stream": false
        });
        
        let response = self.client
            .post(&format!("{}/chat/completions", self.config.base_url))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .header("HTTP-Referer", "https://github.com/your-repo/ruai") // Required by OpenRouter
            .header("X-Title", "RUAI - Rust AI Agent") // Optional but recommended
            .json(&payload)
            .send()
            .await;
            
        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    let response_json: Value = resp.json().await?;
                    
                    let content = response_json["choices"][0]["message"]["content"]
                        .as_str()
                        .unwrap_or("No response content")
                        .to_string();
                    
                    let tokens_used = response_json["usage"]["total_tokens"]
                        .as_u64()
                        .unwrap_or(0) as u32;
                    
                    let response_time = start.elapsed().as_millis() as u64;
                    metrics.record_success(response_time);
                    
                    Ok(ModelResponse {
                        content,
                        model_used: format!("OpenRouter-{}", self.config.model),
                        tokens_used,
                        response_time_ms: response_time,
                        confidence_score: Some(0.90), // Good quality, varies by model
                    })
                } else {
                    let status_code = resp.status();
                    let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                    let error_msg = format!("OpenRouter API error: {} - {}", status_code, error_text);
                    error!("{}", error_msg);
                    metrics.record_failure(error_msg.clone());
                    Err(anyhow!(error_msg))
                }
            }
            Err(e) => {
                let error_msg = format!("OpenRouter request failed: {}", e);
                error!("{}", error_msg);
                metrics.record_failure(error_msg.clone());
                Err(anyhow!(error_msg))
            }
        }
    }
    
    fn name(&self) -> &str {
        "OpenRouter"
    }
    
    fn is_available(&self) -> bool {
        self.config.api_key.is_some()
    }
    
    fn estimated_latency_ms(&self) -> u64 {
        1200 // Varies by model, but generally fast
    }
    
    fn quality_score(&self) -> f32 {
        0.90 // Quality depends on the specific model chosen
    }
}
