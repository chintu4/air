pub mod filesystem;
pub mod calculator;
pub mod memory;
pub mod planner;
pub mod web;
pub mod command;
pub mod screenshot;
pub mod voice;
pub mod knowledge;
pub mod system;
pub mod news;
pub mod manager;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub tool_name: String,
    pub function: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub result: serde_json::Value,
    pub metadata: Option<serde_json::Value>,
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn available_functions(&self) -> Vec<String>;
    async fn execute(&self, function: &str, args: serde_json::Value) -> Result<ToolResult>;
}

pub use filesystem::FileSystemTool;
pub use calculator::CalculatorTool;
pub use memory::MemoryTool;
pub use planner::PlannerTool;
pub use web::WebTool;
pub use command::CommandTool;
pub use screenshot::ScreenshotTool;
pub use voice::VoiceTool;
pub use knowledge::KnowledgeTool;
pub use system::SystemTool;
pub use news::NewsTool;
pub use manager::ToolManager;
