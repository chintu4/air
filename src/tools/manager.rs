use super::{Tool, ToolResult, FileSystemTool, CalculatorTool, MemoryTool, PlannerTool, WebTool, CommandTool, ScreenshotTool, VoiceTool, KnowledgeTool};
use anyhow::Result;
use serde_json::json;
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
    pub fn new() -> Self {
        Self {
            filesystem: Arc::new(FileSystemTool::new(None)),
            calculator: Arc::new(CalculatorTool::new()),
            memory: Arc::new(MemoryTool::new(None)),
            planner: Arc::new(PlannerTool::new()),
            web: Arc::new(WebTool::new()),
            command: Arc::new(CommandTool::new()),
            screenshot: Arc::new(ScreenshotTool::new(None)),
            voice: Arc::new(VoiceTool::new(None)),
            knowledge: Arc::new(KnowledgeTool::new().expect("Failed to init knowledge tool")),
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
    
    fn extract_url(&self, query: &str) -> Option<String> {
        // Enhanced URL extraction with better regex support
        use regex::Regex;
        
        // Try to match full URLs first (http/https)
        if let Ok(url_regex) = Regex::new(r"https?://[^\s]+") {
            if let Some(m) = url_regex.find(query) {
                let url = m.as_str();
                // Clean up trailing punctuation that might be captured
                let cleaned = url.trim_end_matches(&['.', ',', '!', '?', ';', ')', ']'][..]);
                return Some(cleaned.to_string());
            }
        }
        
        // Fallback: look for www. patterns and add https://
        if let Ok(www_regex) = Regex::new(r"www\.[^\s]+") {
            if let Some(m) = www_regex.find(query) {
                let url = m.as_str();
                let cleaned = url.trim_end_matches(&['.', ',', '!', '?', ';', ')', ']'][..]);
                return Some(format!("https://{}", cleaned));
            }
        }
        
        // Final fallback: look for domain-like patterns, but exclude common file extensions
        if let Ok(domain_regex) = Regex::new(r"[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}[^\s]*") {
            if let Some(m) = domain_regex.find(query) {
                let domain = m.as_str();
                
                // Exclude common file extensions
                let file_extensions = [
                    ".toml", ".json", ".rs", ".txt", ".md", ".yml", ".yaml", ".xml", 
                    ".js", ".ts", ".py", ".java", ".cpp", ".c", ".h", ".css", ".html",
                    ".ini", ".cfg", ".conf", ".log", ".zip", ".tar", ".gz"
                ];
                
                let domain_lower = domain.to_lowercase();
                let is_file = file_extensions.iter().any(|&ext| domain_lower.ends_with(ext));
                
                // Only convert to URL if it looks like a domain and not a file
                if domain.contains('.') && !domain.starts_with('.') && !is_file {
                    let cleaned = domain.trim_end_matches(&['.', ',', '!', '?', ';', ')', ']'][..]);
                    // Check if it already has a protocol
                    if !cleaned.starts_with("http") {
                        return Some(format!("https://{}", cleaned));
                    }
                    return Some(cleaned.to_string());
                }
            }
        }
        
        None
    }
    
    fn looks_like_file_path(&self, query: &str) -> bool {
        // Check if the query contains patterns that suggest it's referring to a file
        let file_indicators = [
            ".toml", ".json", ".rs", ".txt", ".md", ".yml", ".yaml", ".xml", ".js", ".ts",
            ".py", ".java", ".cpp", ".c", ".h", ".css", ".html", ".ini", ".cfg", ".conf",
            "./", "../", "\\", "/", "src/", "target/", "tests/", "docs/"
        ];
        
        let query_lower = query.to_lowercase();
        file_indicators.iter().any(|&indicator| query_lower.contains(indicator))
    }
    
    fn extract_file_path(&self, query: &str) -> Option<String> {
        // Enhanced file path extraction - handle various patterns
        let words: Vec<&str> = query.split_whitespace().collect();
        
        // Look for patterns like "file [path]", "analyze [path]", etc.
        for (i, word) in words.iter().enumerate() {
            if word.to_lowercase() == "file" && i + 1 < words.len() {
                return Some(words[i + 1].to_string());
            }
        }
        
        // Look for file-like patterns in the query
        for word in &words {
            if self.looks_like_file_path(&word.to_string()) {
                // Remove leading/trailing quotes if present
                let cleaned = word.trim_matches('"').trim_matches('\'');
                return Some(cleaned.to_string());
            }
        }
        
        None
    }
    
    fn extract_write_file_intent(&self, query: &str) -> Option<(String, String)> {
        // For now, just extract basic intent - this would be enhanced with better parsing
        if let Some(path) = self.extract_file_path(query) {
            let content = "// File created by air agent\n// Add your content here";
            Some((path, content.to_string()))
        } else {
            None
        }
    }
    
    fn extract_directory_path(&self, query: &str) -> Option<String> {
        let words: Vec<&str> = query.split_whitespace().collect();
        for (i, word) in words.iter().enumerate() {
            if (word.to_lowercase() == "directory" || word.to_lowercase() == "folder") && i + 1 < words.len() {
                return Some(words[i + 1].to_string());
            }
        }
        None
    }
    
    fn is_math_query(&self, query: &str) -> bool {
        // Check for mathematical operations and keywords
        let math_patterns = [
            "+", "-", "*", "/", "=", "calculate", "math", "factorial", 
            "percentage", "percent", "% of"
        ];
        
        // Strong indicators for math
        if math_patterns.iter().any(|&pattern| query.contains(pattern)) {
            return true;
        }
        
        // More specific "what is" patterns that indicate math
        if query.contains("what is") && (
            query.contains("+") || query.contains("-") || query.contains("*") || 
            query.contains("/") || query.contains("=") || query.contains("%") ||
            query.chars().any(|c| c.is_digit(10))
        ) {
            return true;
        }
        
        // More specific "how much is" patterns that indicate math
        if query.contains("how much is") && (
            query.contains("+") || query.contains("-") || query.contains("*") || 
            query.contains("/") || query.contains("=") || query.contains("%") ||
            query.chars().any(|c| c.is_digit(10))
        ) {
            return true;
        }
        
        // Check for percentage notation like "15% of 200"
        if query.contains('%') {
            return true;
        }
        
        // Check if it's mostly numbers and operators
        let math_chars = query.chars().filter(|c| 
            c.is_digit(10) || "+-*/=().,% ".contains(*c)
        ).count();
        
        // If more than 60% of non-space characters are math-related (increased threshold)
        let non_space_chars = query.chars().filter(|c| !c.is_whitespace()).count();
        if non_space_chars > 0 && math_chars > (non_space_chars * 6) / 10 {
            return true;
        }
        
        false
    }
}
