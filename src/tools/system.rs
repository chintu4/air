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
                let now = Local::now();
                let time_json = serde_json::json!({
                    "iso": now.to_rfc3339(),
                    "timestamp": now.timestamp(),
                    "formatted": now.format("%Y-%m-%d %H:%M:%S %Z").to_string(),
                    "timezone": now.format("%Z").to_string()
                });

                Ok(ToolResult {
                    success: true,
                    result: time_json,
                    metadata: None,
                })
            }
            _ => Err(anyhow::anyhow!("Unknown function: {}", function)),
        }
    }
}
