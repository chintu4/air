use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub local_model: LocalModelConfig,
    pub cloud_providers: Vec<CloudProviderConfig>,
    pub performance: PerformanceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalModelConfig {
    pub model_path: String,
    pub max_tokens: u32,
    pub temperature: f32,
    pub context_length: u32,
    pub threads: u32,
}

impl Default for LocalModelConfig {
    fn default() -> Self {
        Self {
            model_path: "C:\\models\\tinyllama-1.1b-chat-v1.0.Q2_K.gguf".to_string(),
            max_tokens: 512,
            temperature: 0.7,
            context_length: 2048,
            threads: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudProviderConfig {
    pub name: String,
    pub api_key: Option<String>,
    pub base_url: String,
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f32,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub fallback_threshold_ms: u64,
    pub quality_threshold: f32,
}

impl Config {
    pub fn load() -> Result<Self> {
        // Try to load from config file, otherwise use defaults
        let config_path = std::env::current_dir()?.join("config.toml");
        
        if config_path.exists() {
            let content = std::fs::read_to_string(config_path)?;
            let mut config: Config = toml::from_str(&content)?;
            
            // Override API keys from environment variables
            for provider in &mut config.cloud_providers {
                match provider.name.as_str() {
                    "openai" => {
                        if let Ok(key) = std::env::var("OPENAI_API_KEY") {
                            provider.api_key = Some(key);
                        }
                    }
                    "anthropic" => {
                        if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
                            provider.api_key = Some(key);
                        }
                    }
                    "gemini" => {
                        if let Ok(key) = std::env::var("GEMINI_KEY") {
                            provider.api_key = Some(key);
                        }
                    }
                    "openrouter" => {
                        if let Ok(key) = std::env::var("OPEN_ROUTER") {
                            provider.api_key = Some(key);
                        }
                    }
                    _ => {}
                }
            }
            
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            cloud_providers: vec![
                // CloudProviderConfig {
                //     name: "openai".to_string(),
                //     api_key: std::env::var("OPENAI_API_KEY").ok(),
                //     base_url: "https://api.openai.com/v1".to_string(),
                //     model: "gpt-3.5-turbo".to_string(),
                //     max_tokens: 1000,
                //     temperature: 0.7,
                //     timeout_seconds: 30,
                // },
                // CloudProviderConfig {
                //     name: "anthropic".to_string(),
                //     api_key: std::env::var("ANTHROPIC_API_KEY").ok(),
                //     base_url: "https://api.anthropic.com".to_string(),
                //     model: "claude-3-haiku-20240307".to_string(),
                //     max_tokens: 1000,
                //     temperature: 0.7,
                //     timeout_seconds: 30,
                // },
                CloudProviderConfig {
                    name: "gemini".to_string(),
                    api_key: std::env::var("GEMINI_KEY").ok(),
                    base_url: "https://generativelanguage.googleapis.com".to_string(),
                    model: "gemini-pro".to_string(),
                    max_tokens: 1000,
                    temperature: 0.7,
                    timeout_seconds: 30,
                },
                CloudProviderConfig {
                    name: "openrouter".to_string(),
                    api_key: std::env::var("OPEN_ROUTER").ok(),
                    base_url: "https://openrouter.ai/api/v1".to_string(),
                    model: "anthropic/claude-3.5-haiku".to_string(),
                    max_tokens: 1000,
                    temperature: 0.7,
                    timeout_seconds: 30,
                },
            ],
            local_model: LocalModelConfig::default(),
            performance: PerformanceConfig {
                fallback_threshold_ms: 3000,
                quality_threshold: 0.8,
            },
        }
    }
}
