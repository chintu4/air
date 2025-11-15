//! # RUAI - Rust AI Agent Library
//! 
//! A high-performance AI agent system supporting cloud models.
//! 
//! ## Features
//! 
//! - Cloud provider integration (OpenAI, Anthropic, etc.)
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
//!     let config = Config::load()?;
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
pub use config::{Config, CloudProviderConfig, PerformanceConfig};
pub use models::{ModelProvider, ModelResponse, QueryContext, ModelMetrics};
pub use tools::{Tool, ToolCall, ToolResult};
