use async_trait::async_trait;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelResponse {
    pub content: String,
    pub model_used: String,
    pub tokens_used: u32,
    pub response_time_ms: u64,
    pub confidence_score: Option<f32>,
}

impl fmt::Display for ModelResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.content)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct QueryContext {
    pub prompt: String,
    pub messages: Option<Vec<Message>>,
    pub max_tokens: u32,
    pub temperature: f32,
    pub timeout: Duration,
    pub pure_mode: bool,
}

#[async_trait]
pub trait ModelProvider: Send + Sync {
    async fn generate(&self, context: &QueryContext) -> Result<ModelResponse>;
    fn name(&self) -> &str;
    fn is_available(&self) -> bool;
    fn estimated_latency_ms(&self) -> u64;
    fn quality_score(&self) -> f32; // 0.0-1.0
}

#[derive(Clone, Debug)]
pub struct ModelMetrics {
    pub avg_response_time_ms: u64,
    pub success_rate: f32,
    pub last_error: Option<String>,
    pub total_requests: u64,
    pub successful_requests: u64,
}

impl Default for ModelMetrics {
    fn default() -> Self {
        Self {
            avg_response_time_ms: 0,
            success_rate: 1.0,
            last_error: None,
            total_requests: 0,
            successful_requests: 0,
        }
    }
}

impl ModelMetrics {
    pub fn record_success(&mut self, response_time_ms: u64) {
        self.total_requests += 1;
        self.successful_requests += 1;
        self.avg_response_time_ms = 
            (self.avg_response_time_ms * (self.successful_requests - 1) + response_time_ms) 
            / self.successful_requests;
        self.success_rate = self.successful_requests as f32 / self.total_requests as f32;
    }
    
    pub fn record_failure(&mut self, error: String) {
        self.total_requests += 1;
        self.last_error = Some(error);
        self.success_rate = self.successful_requests as f32 / self.total_requests as f32;
    }
}
