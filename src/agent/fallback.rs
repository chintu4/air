use crate::models::ModelResponse;
use crate::agent::memory::MemoryManager;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait FallbackStrategy: Send + Sync {
    async fn execute(&self, prompt: &str, memory_manager: &MemoryManager) -> Result<ModelResponse>;
}

pub struct CacheFallback;

#[async_trait]
impl FallbackStrategy for CacheFallback {
    async fn execute(&self, prompt: &str, memory_manager: &MemoryManager) -> Result<ModelResponse> {
        // Try to find similar past responses
        if let Ok(recent_convs) = memory_manager.get_recent_conversations(10) {
            for (user_input, ai_response, _) in recent_convs {
                if Self::is_similar_query(prompt, &user_input) {
                    tracing::info!("📋 Found similar past response, using as fallback");
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

        Err(anyhow::anyhow!("No similar cached response found"))
    }
}

impl CacheFallback {
    fn is_similar_query(query1: &str, query2: &str) -> bool {
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
}

pub struct DefaultFallback;

#[async_trait]
impl FallbackStrategy for DefaultFallback {
    async fn execute(&self, prompt: &str, _memory_manager: &MemoryManager) -> Result<ModelResponse> {
        Ok(ModelResponse {
            content: format!("⚠️  I'm currently experiencing connectivity issues. Please try again in a moment.\n\nYour query was: '{}'\n\nFor urgent matters, you can also try:\n• Using 'mode local' to force local processing\n• Checking your internet connection\n• Verifying API keys in your configuration", prompt),
            model_used: "Fallback-Default".to_string(),
            tokens_used: 0,
            response_time_ms: 0,
            confidence_score: Some(0.1),
        })
    }
}

pub struct FallbackChain {
    strategies: Vec<Box<dyn FallbackStrategy>>,
}

impl FallbackChain {
    pub fn new() -> Self {
        Self {
            strategies: vec![
                Box::new(CacheFallback),
                Box::new(DefaultFallback),
            ],
        }
    }

    pub async fn execute(&self, prompt: &str, memory_manager: &MemoryManager) -> Result<ModelResponse> {
        for strategy in &self.strategies {
            match strategy.execute(prompt, memory_manager).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    tracing::debug!("Fallback strategy failed: {}", e);
                    continue;
                }
            }
        }

        Err(anyhow::anyhow!("All fallback strategies failed"))
    }
}

impl Default for FallbackChain {
    fn default() -> Self {
        Self::new()
    }
}
