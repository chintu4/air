use crate::models::{ModelProvider, ModelResponse, QueryContext, ModelMetrics};
use crate::config::CloudProviderConfig;
use async_trait::async_trait;
use anyhow::{Result, anyhow};
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Instant;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{warn, error, debug, info};

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
    cached_models: Arc<Mutex<Option<Vec<String>>>>,
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
            cached_models: Arc::new(Mutex::new(None)),
        })
    }

    async fn fetch_and_sort_models(&self, api_key: &str) -> Result<Vec<String>> {
        // Check cache first
        {
            let cache = self.cached_models.lock().await;
            if let Some(models) = cache.as_ref() {
                debug!("Using cached Gemini models: {:?}", models);
                return Ok(models.clone());
            }
        }

        debug!("Fetching Gemini models from API...");
        let url = format!("{}/v1beta/models?key={}", self.config.base_url, api_key);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
             return Err(anyhow!("Failed to fetch models list: {}", response.status()));
        }

        let json: Value = response.json().await?;

        let mut models: Vec<String> = Vec::new();

        if let Some(items) = json["models"].as_array() {
            for item in items {
                if let Some(name) = item["name"].as_str() {
                    // Filter for generateContent supported models
                    // Usually models are named like "models/gemini-1.5-pro"
                    // We need to ensure it supports "generateContent" method
                    if let Some(methods) = item["supportedGenerationMethods"].as_array() {
                        let supports_generate = methods.iter().any(|m| m.as_str() == Some("generateContent"));
                        if supports_generate && name.contains("gemini") {
                            // Extract just the model name part if it starts with "models/"
                            let model_name = name.trim_start_matches("models/").to_string();
                            models.push(model_name);
                        }
                    }
                }
            }
        }

        // Sort models
        // Priority: Version (descending), then Capability (Ultra > Pro > Flash > others)
        models.sort_by(|a, b| {
             // Extract version numbers roughly
             let get_version = |s: &str| -> f32 {
                 if let Some(start) = s.find("gemini-") {
                     let rest = &s[start+7..];
                     let end = rest.find('-').unwrap_or(rest.len());
                     rest[..end].parse::<f32>().unwrap_or(0.0)
                 } else {
                     0.0
                 }
             };

             let ver_a = get_version(a);
             let ver_b = get_version(b);

             if (ver_a - ver_b).abs() > 0.001 {
                 return ver_b.partial_cmp(&ver_a).unwrap_or(std::cmp::Ordering::Equal);
             }

             // Same version, check capability priority
             // Ultra (hypothetically) > Pro > Flash
             let score = |s: &str| -> u8 {
                 if s.contains("ultra") { 4 }
                 else if s.contains("pro") { 3 }
                 else if s.contains("flash") { 2 }
                 else { 1 }
             };

             score(b).cmp(&score(a))
        });

        info!("Fetched and sorted Gemini models: {:?}", models);

        // Update cache
        let mut cache = self.cached_models.lock().await;
        *cache = Some(models.clone());

        Ok(models)
    }
}

#[async_trait]
impl ModelProvider for GeminiProvider {
    async fn generate(&self, context: &QueryContext) -> Result<ModelResponse> {
        let api_key = self.config.api_key.as_ref()
            .ok_or_else(|| anyhow!("Gemini API key not configured"))?;
            
        let start = Instant::now();
        let mut metrics = self.metrics.lock().await;
        
        // Fetch dynamic model list
        let available_models = match self.fetch_and_sort_models(api_key).await {
             Ok(models) => models,
             Err(e) => {
                 warn!("Failed to fetch dynamic model list: {}. Falling back to configured default.", e);
                 vec![self.config.model.clone()]
             }
        };
        
        let mut last_error = anyhow!("No models available");
        
        // Iterate through models until success
        for model_name in available_models {
            debug!("Attempting generation with Gemini model: {}", model_name);
            
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
                             self.config.base_url, model_name, api_key);

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

                        // Extract content safely
                        if let Some(candidates) = response_json["candidates"].as_array() {
                            if let Some(first) = candidates.first() {
                                if let Some(parts) = first["content"]["parts"].as_array() {
                                    if let Some(text) = parts[0]["text"].as_str() {
                                        let content = text.to_string();
                                        let tokens_used = (content.len() / 4) as u32;

                                        let response_time = start.elapsed().as_millis() as u64;
                                        metrics.record_success(response_time);

                                        return Ok(ModelResponse {
                                            content,
                                            model_used: format!("Gemini-{}", model_name),
                                            tokens_used,
                                            response_time_ms: response_time,
                                            confidence_score: Some(0.92),
                                        });
                                    }
                                }
                            }
                        }
                        // If we parsed JSON successfully but structure was unexpected (e.g. safety block)
                        warn!("Gemini model {} returned success but unexpected structure (likely safety block). Trying next model.", model_name);
                        last_error = anyhow!("Response parsing failed for {}", model_name);
                    } else {
                        let status = resp.status();
                        // If 4xx/5xx error, warn and try next
                        warn!("Gemini model {} failed with status {}. Trying next model...", model_name, status);
                        last_error = anyhow!("API error {}: {}", status, resp.text().await.unwrap_or_default());
                    }
                }
                Err(e) => {
                    warn!("Request failed for {}: {}. Trying next model...", model_name, e);
                    last_error = anyhow!(e);
                }
            }
        }

        // If we get here, all models failed
        let error_msg = format!("All Gemini models failed. Last error: {}", last_error);
        error!("{}", error_msg);
        metrics.record_failure(error_msg.clone());
        Err(anyhow!(error_msg))
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
