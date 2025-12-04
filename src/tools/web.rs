use super::{Tool, ToolResult};
use async_trait::async_trait;
use anyhow::{Result, anyhow};
use serde_json::{json, Value};
use reqwest::Client;
use std::time::Duration;
use tracing::info;

pub struct WebTool {
    client: Client,
}

impl WebTool {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("air-Agent/1.0")
            .build()
            .unwrap();
            
        Self { client }
    }
    
    fn is_valid_url(&self, url: &str) -> bool {
        url.starts_with("http://") || url.starts_with("https://")
    }
    
    fn extract_text_content(&self, html: &str) -> String {
        // Simple HTML text extraction (for a more robust solution, use a proper HTML parser)
        let mut text = html.to_string();
        
        // Remove script and style tags completely
        while let Some(start) = text.find("<script") {
            if let Some(end) = text[start..].find("</script>") {
                text.replace_range(start..start + end + 9, "");
            } else {
                break;
            }
        }
        
        while let Some(start) = text.find("<style") {
            if let Some(end) = text[start..].find("</style>") {
                text.replace_range(start..start + end + 8, "");
            } else {
                break;
            }
        }
        
        // Remove HTML tags
        let mut result = String::new();
        let mut in_tag = false;
        
        for ch in text.chars() {
            match ch {
                '<' => in_tag = true,
                '>' => in_tag = false,
                _ if !in_tag => result.push(ch),
                _ => {}
            }
        }
        
        // Clean up whitespace
        result
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[async_trait]
impl Tool for WebTool {
    fn name(&self) -> &str {
        "web"
    }
    
    fn description(&self) -> &str {
        "Web operations: fetch pages, extract content, check status"
    }
    
    fn available_functions(&self) -> Vec<String> {
        vec![
            "fetch".to_string(),
            "get_headers".to_string(),
            "check_status".to_string(),
            "extract_text".to_string(),
        ]
    }
    
    async fn execute(&self, function: &str, args: Value) -> Result<ToolResult> {
        match function {
            "fetch" => {
                let url = args["url"].as_str()
                    .ok_or_else(|| anyhow!("Missing 'url' parameter"))?;
                
                if !self.is_valid_url(url) {
                    return Ok(ToolResult {
                        success: false,
                        result: format!("Invalid URL format: {}. Must start with http:// or https://", url),
                        metadata: None,
                    });
                }
                
                info!("Fetching URL: {}", url);
                
                match self.client.get(url).send().await {
                    Ok(response) => {
                        let status = response.status();
                        let headers = response.headers().clone();
                        
                        if status.is_success() {
                            match response.text().await {
                                Ok(content) => {
                                    let text_content = self.extract_text_content(&content);
                                    let preview = if text_content.len() > 1000 {
                                        format!("{}...\n\n[Content truncated - {} total characters]", 
                                               &text_content[..1000], text_content.len())
                                    } else {
                                        text_content.clone()
                                    };
                                    
                                    let metadata = json!({
                                        "url": url,
                                        "status_code": status.as_u16(),
                                        "content_length": content.len(),
                                        "text_length": text_content.len(),
                                        "content_type": headers.get("content-type")
                                            .and_then(|v| v.to_str().ok()),
                                    });
                                    
                                    Ok(ToolResult {
                                        success: true,
                                        result: format!("Successfully fetched {}:\n\n{}", url, preview),
                                        metadata: Some(metadata),
                                    })
                                }
                                Err(e) => Ok(ToolResult {
                                    success: false,
                                    result: format!("Failed to read response body: {}", e),
                                    metadata: None,
                                })
                            }
                        } else {
                            Ok(ToolResult {
                                success: false,
                                result: format!("HTTP Error {}: Failed to fetch {}", status, url),
                                metadata: Some(json!({
                                    "url": url,
                                    "status_code": status.as_u16(),
                                    "status_text": status.canonical_reason()
                                })),
                            })
                        }
                    }
                    Err(e) => Ok(ToolResult {
                        success: false,
                        result: format!("Network error fetching {}: {}", url, e),
                        metadata: None,
                    })
                }
            }
            
            "check_status" => {
                let url = args["url"].as_str()
                    .ok_or_else(|| anyhow!("Missing 'url' parameter"))?;
                
                if !self.is_valid_url(url) {
                    return Ok(ToolResult {
                        success: false,
                        result: format!("Invalid URL format: {}", url),
                        metadata: None,
                    });
                }
                
                info!("Checking status for: {}", url);
                
                match self.client.head(url).send().await {
                    Ok(response) => {
                        let status = response.status();
                        let headers = response.headers();
                        
                        let result = format!(
                            "Status for {}: {} {}\nServer: {}\nContent-Type: {}",
                            url,
                            status.as_u16(),
                            status.canonical_reason().unwrap_or("Unknown"),
                            headers.get("server")
                                .and_then(|v| v.to_str().ok())
                                .unwrap_or("Unknown"),
                            headers.get("content-type")
                                .and_then(|v| v.to_str().ok())
                                .unwrap_or("Unknown")
                        );
                        
                        Ok(ToolResult {
                            success: true,
                            result,
                            metadata: Some(json!({
                                "url": url,
                                "status_code": status.as_u16(),
                                "status_text": status.canonical_reason(),
                                "is_success": status.is_success()
                            })),
                        })
                    }
                    Err(e) => Ok(ToolResult {
                        success: false,
                        result: format!("Failed to check status for {}: {}", url, e),
                        metadata: None,
                    })
                }
            }
            
            "get_headers" => {
                let url = args["url"].as_str()
                    .ok_or_else(|| anyhow!("Missing 'url' parameter"))?;
                
                if !self.is_valid_url(url) {
                    return Ok(ToolResult {
                        success: false,
                        result: format!("Invalid URL format: {}", url),
                        metadata: None,
                    });
                }
                
                match self.client.head(url).send().await {
                    Ok(response) => {
                        let headers = response.headers();
                        let mut header_list = Vec::new();
                        
                        for (name, value) in headers.iter() {
                            if let Ok(value_str) = value.to_str() {
                                header_list.push(format!("{}: {}", name, value_str));
                            }
                        }
                        
                        let result = format!("Headers for {}:\n{}", url, header_list.join("\n"));
                        
                        Ok(ToolResult {
                            success: true,
                            result,
                            metadata: Some(json!({
                                "url": url,
                                "header_count": header_list.len()
                            })),
                        })
                    }
                    Err(e) => Ok(ToolResult {
                        success: false,
                        result: format!("Failed to get headers for {}: {}", url, e),
                        metadata: None,
                    })
                }
            }
            
            _ => Err(anyhow!("Unknown function: {}", function))
        }
    }
}
