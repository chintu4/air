use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use chrono::Local;
use super::{Tool, ToolResult};

pub struct SystemTool;

impl SystemTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for SystemTool {
    fn name(&self) -> &str {
        "system"
    }

    fn description(&self) -> &str {
        "Provides access to system information like time and date."
    }

    fn available_functions(&self) -> Vec<String> {
        vec!["get_system_time".to_string()]
    }

    async fn execute(&self, function: &str, _args: Value) -> Result<ToolResult> {
        match function {
            "get_system_time" => {
                let time = Local::now().format("%Y-%m-%d %H:%M:%S %Z").to_string();
                Ok(ToolResult {
                    success: true,
                    result: time,
                    metadata: None,
                })
            }
            _ => Err(anyhow::anyhow!("Unknown function: {}", function)),
        }
    }
}
