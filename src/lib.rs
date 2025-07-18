//! # RUAI - Rust AI Agent Library
//! 
//! A high-performance AI agent system supporting both local and cloud models.
//! 
//! ## Features
//! 
//! - Local GGUF model support with intelligent response generation
//! - Cloud provider integration (OpenAI, Anthropic, etc.)
//! - Automatic fallback between local and cloud models
//! - Performance monitoring and metrics
//! - Query classification and routing
//! 
//! ## Usage
//! 
//! ```rust,no_run
//! use ruai::{agent::AIAgent, config::Config};
//! 
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = Config::from_file("config.toml")?;
//!     let agent = AIAgent::new(config).await?;
//!     
//!     let response = agent.query("What is 2+2?").await?;
//!     println!("Response: {}", response.content);
//!     
//!     Ok(())
//! }
//! ```

pub mod agent;
pub mod config;
pub mod models;
pub mod providers;
pub mod tools;

// Re-export commonly used types for convenience
pub use agent::AIAgent;
pub use config::{Config, LocalModelConfig, CloudProviderConfig, PerformanceConfig};
pub use models::{ModelProvider, ModelResponse, QueryContext, ModelMetrics};
pub use tools::{Tool, ToolCall, ToolResult};
