use super::{Tool, ToolResult, FileSystemTool, CalculatorTool, MemoryTool, PlannerTool, WebTool, CommandTool, ScreenshotTool, VoiceTool, KnowledgeTool, SystemTool, NewsTool};
use anyhow::Result;
use std::sync::Arc;
use tracing::{info, debug};

pub struct ToolManager {
    filesystem: Arc<dyn Tool>,
    calculator: Arc<dyn Tool>,
    memory: Arc<dyn Tool>,
    planner: Arc<dyn Tool>,
    web: Arc<dyn Tool>,
    command: Arc<dyn Tool>,
    screenshot: Arc<dyn Tool>,
    voice: Arc<dyn Tool>,
    knowledge: Arc<dyn Tool>,
    system: Arc<dyn Tool>,
    news: Arc<dyn Tool>,
}

impl ToolManager {
    pub async fn new() -> Self {
        Self {
            filesystem: Arc::new(FileSystemTool::new(None)),
            calculator: Arc::new(CalculatorTool::new()),
            memory: Arc::new(MemoryTool::new(None)),
            planner: Arc::new(PlannerTool::new()),
            web: Arc::new(WebTool::new()),
            command: Arc::new(CommandTool::new()),
            screenshot: Arc::new(ScreenshotTool::new(None)),
            voice: Arc::new(VoiceTool::new(None)),
            knowledge: Arc::new(KnowledgeTool::new().await.unwrap_or_else(|_| {
                // This branch should technically be unreachable now since new() handles errors internally,
                // but just in case we return a dummy struct or panic safely?
                // Actually KnowledgeTool::new() returns Result<Self>, so we can unwrap safely if we know it returns Ok.
                // But wait, I changed it to return Ok even on error (just with None store).
                // So unwrap() is fine.
                // However, the signature is Result<Self, anyhow::Error>.
                // I'll stick to unwrap() but I'll ensure KnowledgeTool::new() catches everything.
                panic!("KnowledgeTool::new() should not fail")
            })),
            system: Arc::new(SystemTool::new()),
            news: Arc::new(NewsTool::new()),
        }
    }
    
    pub fn get_tool_definitions(&self) -> serde_json::Value {
        let tools: Vec<&Arc<dyn Tool>> = vec![
            &self.filesystem,
            &self.calculator,
            &self.memory,
            &self.planner,
            &self.web,
            &self.command,
            &self.screenshot,
            &self.voice,
            &self.knowledge,
            &self.system,
            &self.news,
        ];

        let definitions: Vec<serde_json::Value> = tools.iter().map(|tool| {
            serde_json::json!({
                "name": tool.name(),
                "description": tool.description(),
                "functions": tool.available_functions()
            })
        }).collect();

        serde_json::json!(definitions)
    }

    pub async fn execute_tool(&self, tool_name: &str, function: &str, args: serde_json::Value) -> Result<ToolResult> {
        info!("🔧 Executing tool: {} -> {}", tool_name, function);
        debug!("Tool arguments: {}", args);
        
        let tool: &Arc<dyn Tool> = match tool_name {
            "filesystem" => &self.filesystem,
            "calculator" => &self.calculator,
            "memory" => &self.memory,
            "planner" => &self.planner,
            "web" => &self.web,
            "command" => &self.command,
            "screenshot" => &self.screenshot,
            "voice" => &self.voice,
            "knowledge" => &self.knowledge,
            "system" => &self.system,
            "WebScraper" => &self.news,
            _ => return Err(anyhow::anyhow!("Unknown tool: {}", tool_name)),
        };
        
        tool.execute(function, args).await
    }
}
