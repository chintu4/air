use super::{Tool, ToolResult};
use async_trait::async_trait;
use anyhow::{Result, anyhow};
use serde_json::{json, Value};
use std::path::Path;
use std::fs;
use tracing::info;
use std::io::{self, Write};

pub struct FileSystemTool {
    base_directory: String,
}

impl FileSystemTool {
    pub fn new(base_directory: Option<String>) -> Self {
        let base_dir = base_directory.unwrap_or_else(|| {
            std::env::current_dir()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        });
        
        Self {
            base_directory: base_dir,
        }
    }
    
    fn is_safe_path(&self, path: &str) -> bool {
        !path.contains('\0') // Null bytes are always invalid
    }
    
    fn get_full_path(&self, path: &str) -> Result<std::path::PathBuf> {
        if !self.is_safe_path(path) {
            return Err(anyhow!("Invalid file path: {}", path));
        }
        
        let path_buf = Path::new(path);
        
        if path_buf.is_absolute() {
            Ok(path_buf.to_path_buf())
        } else {
            Ok(Path::new(&self.base_directory).join(path))
        }
    }

    fn ask_confirmation(&self, action: &str, path: &str) -> bool {
        // In some environments (e.g. tests), stdin might not be interactive.
        // But for this CLI tool, we assume it is.
        print!("⚠️  Confirmation required: Do you want to {} '{}'? [y/N] ", action, path);
        io::stdout().flush().unwrap_or(());

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_ok() {
            let response = input.trim().to_lowercase();
            return response == "y" || response == "yes";
        }
        false
    }
}

#[async_trait]
impl Tool for FileSystemTool {
    fn name(&self) -> &str {
        "filesystem"
    }
    
    fn description(&self) -> &str {
        "File system operations: read, write, list files and directories"
    }
    
    fn available_functions(&self) -> Vec<String> {
        vec![
            "read_file".to_string(),
            "write_file".to_string(),
            "list_directory".to_string(),
            "file_exists".to_string(),
            "get_file_info".to_string(),
            "create_directory".to_string(),
        ]
    }
    
    async fn execute(&self, function: &str, args: Value) -> Result<ToolResult> {
        match function {
            "read_file" => {
                let path = args["path"].as_str()
                    .ok_or_else(|| anyhow!("Missing 'path' parameter"))?;
                
                info!("Reading file: {}", path);
                let full_path = self.get_full_path(path)?;
                
                match fs::read_to_string(&full_path) {
                    Ok(content) => {
                        let metadata = json!({
                            "path": path,
                            "size": content.len(),
                            "lines": content.lines().count()
                        });
                        
                        Ok(ToolResult {
                            success: true,
                            result: content,
                            metadata: Some(metadata),
                        })
                    }
                    Err(e) => Ok(ToolResult {
                        success: false,
                        result: format!("Failed to read file: {}", e),
                        metadata: None,
                    })
                }
            }
            
            "write_file" => {
                let path = args["path"].as_str()
                    .ok_or_else(|| anyhow!("Missing 'path' parameter"))?;
                let content = args["content"].as_str()
                    .ok_or_else(|| anyhow!("Missing 'content' parameter"))?;
                
                if !self.ask_confirmation("WRITE to file", path) {
                     return Ok(ToolResult {
                        success: false,
                        result: "Operation cancelled by user.".to_string(),
                        metadata: None,
                    });
                }

                info!("Writing file: {}", path);
                let full_path = self.get_full_path(path)?;
                
                if let Some(parent) = full_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                
                match fs::write(&full_path, content) {
                    Ok(_) => {
                        let metadata = json!({
                            "path": path,
                            "bytes_written": content.len()
                        });
                        
                        Ok(ToolResult {
                            success: true,
                            result: format!("Successfully wrote {} bytes to {}", content.len(), path),
                            metadata: Some(metadata),
                        })
                    }
                    Err(e) => Ok(ToolResult {
                        success: false,
                        result: format!("Failed to write file: {}", e),
                        metadata: None,
                    })
                }
            }
            
            "list_directory" => {
                let path = args["path"].as_str().unwrap_or(".");
                
                info!("Listing directory: {}", path);
                let full_path = self.get_full_path(path)?;
                
                match fs::read_dir(&full_path) {
                    Ok(entries) => {
                        let mut files = Vec::new();
                        let mut dirs = Vec::new();
                        
                        for entry in entries {
                            if let Ok(entry) = entry {
                                let name = entry.file_name().to_string_lossy().to_string();
                                if entry.path().is_dir() {
                                    dirs.push(name);
                                } else {
                                    files.push(name);
                                }
                            }
                        }
                        
                        let result = format!(
                            "Directories ({}): {}\nFiles ({}): {}",
                            dirs.len(),
                            dirs.join(", "),
                            files.len(),
                            files.join(", ")
                        );
                        
                        let metadata = json!({
                            "path": path,
                            "directories": dirs,
                            "files": files,
                            "total_items": dirs.len() + files.len()
                        });
                        
                        Ok(ToolResult {
                            success: true,
                            result,
                            metadata: Some(metadata),
                        })
                    }
                    Err(e) => Ok(ToolResult {
                        success: false,
                        result: format!("Failed to list directory: {}", e),
                        metadata: None,
                    })
                }
            }
            
            "file_exists" => {
                let path = args["path"].as_str()
                    .ok_or_else(|| anyhow!("Missing 'path' parameter"))?;
                
                let full_path = self.get_full_path(path)?;
                let exists = full_path.exists();
                
                Ok(ToolResult {
                    success: true,
                    result: if exists { "File exists" } else { "File does not exist" }.to_string(),
                    metadata: Some(json!({"path": path, "exists": exists})),
                })
            }
            
            "get_file_info" => {
                let path = args["path"].as_str()
                    .ok_or_else(|| anyhow!("Missing 'path' parameter"))?;
                
                let full_path = self.get_full_path(path)?;
                
                match fs::metadata(&full_path) {
                    Ok(metadata) => {
                        let info = json!({
                            "path": path,
                            "size": metadata.len(),
                            "is_file": metadata.is_file(),
                            "is_directory": metadata.is_dir(),
                            "readonly": metadata.permissions().readonly(),
                            "modified": metadata.modified().ok()
                                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                .map(|d| d.as_secs())
                        });
                        
                        Ok(ToolResult {
                            success: true,
                            result: format!("File info for {}: {} bytes, {}", 
                                path, metadata.len(),
                                if metadata.is_file() { "file" } else { "directory" }
                            ),
                            metadata: Some(info),
                        })
                    }
                    Err(e) => Ok(ToolResult {
                        success: false,
                        result: format!("Failed to get file info: {}", e),
                        metadata: None,
                    })
                }
            }
            
            "create_directory" => {
                let path = args["path"].as_str()
                    .ok_or_else(|| anyhow!("Missing 'path' parameter"))?;
                
                if !self.ask_confirmation("CREATE directory", path) {
                     return Ok(ToolResult {
                        success: false,
                        result: "Operation cancelled by user.".to_string(),
                        metadata: None,
                    });
                }

                let full_path = self.get_full_path(path)?;
                
                match fs::create_dir_all(&full_path) {
                    Ok(_) => Ok(ToolResult {
                        success: true,
                        result: format!("Created directory: {}", path),
                        metadata: Some(json!({"path": path})),
                    }),
                    Err(e) => Ok(ToolResult {
                        success: false,
                        result: format!("Failed to create directory: {}", e),
                        metadata: None,
                    })
                }
            }
            
            _ => Err(anyhow!("Unknown function: {}", function))
        }
    }
}
