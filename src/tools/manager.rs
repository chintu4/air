use super::{Tool, ToolResult, FileSystemTool, CalculatorTool, MemoryTool, PlannerTool, WebTool, CommandTool, ScreenshotTool, VoiceTool};
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
        }
    }
    
    pub fn detect_tool_intent(&self, query: &str) -> Option<(String, String, serde_json::Value)> {
        let query_lower = query.to_lowercase();
        
        // File operations - Check FIRST to prioritize file paths over URLs
        // Enhanced file pattern detection
        if query_lower.contains("read file") || query_lower.contains("read the file") ||
           query_lower.contains("analyze file") || query_lower.contains("show file") ||
           (query_lower.contains("analyze") && self.looks_like_file_path(query)) {
            if let Some(path) = self.extract_file_path(query) {
                return Some((
                    "filesystem".to_string(),
                    "read_file".to_string(),
                    json!({"path": path})
                ));
            }
        }
        
        // Web operations - Enhanced detection for various web-related patterns
        let web_patterns = [
            "fetch", "http", "https", "www.", "get", "load", "visit", "browse", 
            "summarize", "summaries", "summary", "extract", "read content",
            "scrape", "download", "url", "website", "webpage", "link"
        ];
        
        let has_web_pattern = web_patterns.iter().any(|&pattern| query_lower.contains(pattern));
        let has_url = self.extract_url(query).is_some();
        
        // Only treat as web request if it's clearly a URL (starts with http/https or www.)
        if has_web_pattern && has_url {
            if let Some(url) = self.extract_url(query) {
                // Additional check to ensure it's actually a web URL, not a file path
                if url.starts_with("http://") || url.starts_with("https://") || url.starts_with("www.") {
                    return Some((
                        "web".to_string(),
                        "fetch".to_string(),
                        json!({"url": url})
                    ));
                }
            }
        }
        
        // Command execution - Check BEFORE calculator to avoid conflicts
        if query_lower.contains("run ") || query_lower.contains("execute ") || 
           query_lower.contains("command ") || query_lower.starts_with("git ") ||
           query_lower.starts_with("cargo ") || query_lower.starts_with("npm ") ||
           query_lower.starts_with("node ") || query_lower.starts_with("python ") ||
           query_lower.starts_with("dir") || query_lower.starts_with("ls ") ||
           query_lower.starts_with("pwd") || query_lower.starts_with("cd ") {
            
            let command = if query_lower.starts_with("run ") {
                query[4..].trim()
            } else if query_lower.starts_with("execute ") {
                query[8..].trim()
            } else if query_lower.starts_with("command ") {
                query[8..].trim()
            } else {
                query.trim()
            };
            
            return Some((
                "command".to_string(),
                "execute".to_string(),
                json!({"command": command})
            ));
        }
        
        if query_lower.contains("write file") || query_lower.contains("create file") {
            if let Some((path, content)) = self.extract_write_file_intent(query) {
                return Some((
                    "filesystem".to_string(),
                    "write_file".to_string(),
                    json!({"path": path, "content": content})
                ));
            }
        }
        
        if query_lower.contains("list files") || query_lower.contains("list directory") {
            let path = self.extract_directory_path(query).unwrap_or(".".to_string());
            return Some((
                "filesystem".to_string(),
                "list_directory".to_string(),
                json!({"path": path})
            ));
        }
        
        // Calculator operations
        if self.is_math_query(&query_lower) {
            return Some((
                "calculator".to_string(),
                "calculate".to_string(),
                json!({"expression": query})
            ));
        }
        
        // Task planning
        if query_lower.contains("create task") || query_lower.contains("add task") {
            let title_str = query.replace("create task", "").replace("add task", "");
            let title = title_str.trim();
            return Some((
                "planner".to_string(),
                "create_task".to_string(),
                json!({"title": title, "description": title})
            ));
        }
        
        if query_lower.contains("break down") || query_lower.contains("breakdown") {
            return Some((
                "planner".to_string(),
                "break_down_task".to_string(),
                json!({"description": query})
            ));
        }
        
        if query_lower.contains("list tasks") || query_lower.contains("show tasks") {
            return Some((
                "planner".to_string(),
                "list_tasks".to_string(),
                json!({})
            ));
        }
        
        // Memory operations
        if query_lower.contains("what did we discuss") || query_lower.contains("remember") || query_lower.contains("earlier") {
            let search_str = query.replace("what did we discuss about", "")
                                  .replace("what did we discuss", "")
                                  .replace("remember", "")
                                  .replace("earlier", "");
            let search_term = search_str.trim();
            return Some((
                "memory".to_string(),
                "search_conversations".to_string(),
                json!({"query": search_term})
            ));
        }
        
        // Screenshot operations - Check region FIRST, then general screenshot
        if query_lower.contains("screenshot region") || query_lower.contains("capture region") {
            return Some((
                "screenshot".to_string(),
                "capture_region".to_string(),
                json!({})
            ));
        }
        
        if query_lower.contains("list screenshots") || query_lower.contains("show screenshots") {
            return Some((
                "screenshot".to_string(),
                "list_screenshots".to_string(),
                json!({})
            ));
        }
        
        if query_lower.contains("screenshot") || query_lower.contains("screen capture") || 
           query_lower.contains("take a screenshot") || query_lower.contains("capture screen") {
            return Some((
                "screenshot".to_string(),
                "capture".to_string(),
                json!({})
            ));
        }
        
        // Voice operations
        if query_lower.contains("speak ") || query_lower.contains("say ") || 
           query_lower.contains("text to speech") || query_lower.contains("tts") {
            let text = if query_lower.starts_with("speak ") {
                query[6..].trim()
            } else if query_lower.starts_with("say ") {
                query[4..].trim()
            } else {
                query.trim()
            };
            
            return Some((
                "voice".to_string(),
                "speak".to_string(),
                json!({"text": text})
            ));
        }
        
        if query_lower.contains("listen") || query_lower.contains("speech to text") || 
           query_lower.contains("voice recognition") || query_lower.contains("transcribe") {
            return Some((
                "voice".to_string(),
                "listen".to_string(),
                json!({"duration": 5})
            ));
        }
        
        if query_lower.contains("list voices") || query_lower.contains("available voices") {
            return Some((
                "voice".to_string(),
                "list_voices".to_string(),
                json!({})
            ));
        }

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
            let content = "// File created by RUAI agent\n// Add your content here";
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
