use super::{Tool, ToolResult, FileSystemTool, CalculatorTool, MemoryTool, PlannerTool, WebTool, CommandTool, ScreenshotTool, VoiceTool, KnowledgeTool};
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
        }
    }
    
    // Deprecated: Using LLM for tool selection instead
    pub fn detect_tool_intent(&self, _query: &str) -> Option<(String, String, serde_json::Value)> {
        None
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
            _ => return Err(anyhow::anyhow!("Unknown tool: {}", tool_name)),
        };
        
        tool.execute(function, args).await
    }
}
