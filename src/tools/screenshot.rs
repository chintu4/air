use super::{Tool, ToolResult};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde_json::Value;
use std::process::Command;
use std::path::Path;
use chrono::Utc;
use base64::{Engine as _, engine::general_purpose};

pub struct ScreenshotTool {
    output_dir: String,
}

impl ScreenshotTool {
    pub fn new(output_dir: Option<String>) -> Self {
        let output_dir = output_dir.unwrap_or_else(|| {
            std::env::current_dir()
                .unwrap_or_default()
                .join("screenshots")
                .to_string_lossy()
                .to_string()
        });
        
        // Create screenshots directory if it doesn't exist
        std::fs::create_dir_all(&output_dir).ok();
        
        Self { output_dir }
    }
    
    fn generate_filename(&self, prefix: Option<&str>) -> String {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let prefix = prefix.unwrap_or("screenshot");
        format!("{}_{}.png", prefix, timestamp)
    }
    
    async fn take_screenshot(&self, filename: Option<String>, region: Option<(i32, i32, i32, i32)>) -> Result<ToolResult> {
        let filename = filename.unwrap_or_else(|| self.generate_filename(None));
        let filepath = Path::new(&self.output_dir).join(&filename);
        
        let result = {
            #[cfg(target_os = "windows")]
            { self.take_windows_screenshot(&filepath, region).await }
            #[cfg(target_os = "macos")]
            { self.take_macos_screenshot(&filepath, region).await }
            #[cfg(target_os = "linux")]
            { self.take_linux_screenshot(&filepath, region).await }
            #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
            { Err(anyhow!("Unsupported OS for screenshots")) }
        };
        
        match result {
            Ok(_) => {
                let absolute_path = std::fs::canonicalize(&filepath)
                    .unwrap_or(filepath)
                    .to_string_lossy()
                    .to_string();
                    
                Ok(ToolResult {
                    success: true,
                    result: serde_json::json!({
                        "filepath": absolute_path,
                        "filename": filename,
                        "timestamp": Utc::now().to_rfc3339()
                    }),
                    metadata: Some(serde_json::json!({
                        "filepath": absolute_path,
                        "filename": filename,
                        "timestamp": Utc::now().to_rfc3339(),
                        "vision_analysis_available": true
                    })),
                })
            }
            Err(e) => Ok(ToolResult {
                success: false,
                result: serde_json::json!(format!("Failed to take screenshot: {}", e)),
                metadata: Some(serde_json::json!({
                    "error": e.to_string()
                })),
            })
        }
    }
    
    #[cfg(target_os = "windows")]
    async fn take_windows_screenshot(&self, filepath: &Path, region: Option<(i32, i32, i32, i32)>) -> Result<()> {
        // Use PowerShell to take screenshot
        let script = if let Some((x, y, width, height)) = region {
            format!(
                r#"
                Add-Type -AssemblyName System.Drawing
                Add-Type -AssemblyName System.Windows.Forms
                $bitmap = New-Object System.Drawing.Bitmap {}, {}
                $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
                $graphics.CopyFromScreen({}, {}, 0, 0, $bitmap.Size)
                $bitmap.Save('{}')
                $graphics.Dispose()
                $bitmap.Dispose()
                "#,
                width, height, x, y, filepath.to_string_lossy()
            )
        } else {
            format!(
                r#"
                Add-Type -AssemblyName System.Drawing
                Add-Type -AssemblyName System.Windows.Forms
                $bounds = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
                $bitmap = New-Object System.Drawing.Bitmap $bounds.Width, $bounds.Height
                $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
                $graphics.CopyFromScreen($bounds.X, $bounds.Y, 0, 0, $bounds.Size)
                $bitmap.Save('{}')
                $graphics.Dispose()
                $bitmap.Dispose()
                "#,
                filepath.to_string_lossy()
            )
        };
        
        let output = Command::new("powershell")
            .args(["-Command", &script])
            .output()?;
            
        if !output.status.success() {
            return Err(anyhow!("PowerShell screenshot failed: {}", 
                String::from_utf8_lossy(&output.stderr)));
        }
        
        Ok(())
    }
    
    #[cfg(target_os = "macos")]
    async fn take_macos_screenshot(&self, filepath: &Path, region: Option<(i32, i32, i32, i32)>) -> Result<()> {
        let mut cmd = Command::new("screencapture");
        
        if let Some((x, y, width, height)) = region {
            cmd.args(["-R", &format!("{},{},{},{}", x, y, width, height)]);
        }
        
        cmd.arg(filepath);
        let output = cmd.output()?;
        
        if !output.status.success() {
            return Err(anyhow!("screencapture failed: {}", 
                String::from_utf8_lossy(&output.stderr)));
        }
        
        Ok(())
    }
    
    #[cfg(target_os = "linux")]
    async fn take_linux_screenshot(&self, filepath: &Path, region: Option<(i32, i32, i32, i32)>) -> Result<()> {
        // Try different screenshot tools available on Linux
        let tools = vec!["gnome-screenshot", "scrot", "import"];
        
        for tool in tools {
            if Command::new("which").arg(tool).output().map(|o| o.status.success()).unwrap_or(false) {
                let mut cmd = Command::new(tool);
                
                match tool {
                    "gnome-screenshot" => {
                        cmd.args(["-f", &*filepath.to_string_lossy()]);
                        if region.is_some() {
                            cmd.arg("-a"); // Area selection
                        }
                    }
                    "scrot" => {
                        if let Some((x, y, width, height)) = region {
                            cmd.args(["-a", &format!("{},{},{},{}", x, y, width, height)]);
                        }
                        cmd.arg(&*filepath.to_string_lossy());
                    }
                    "import" => {
                        if region.is_some() {
                            cmd.arg("-frame"); // Interactive selection
                        } else {
                            cmd.arg("-window").arg("root");
                        }
                        cmd.arg(&*filepath.to_string_lossy());
                    }
                    _ => continue,
                }
                
                let output = cmd.output()?;
                if output.status.success() {
                    return Ok(());
                }
            }
        }
        
        Err(anyhow!("No screenshot tool found. Please install gnome-screenshot, scrot, or imagemagick"))
    }
    
    #[allow(dead_code)]
    async fn analyze_screenshot(&self, filepath: &str, prompt: Option<&str>) -> Result<ToolResult> {
        let path = Path::new(filepath);
        if !path.exists() {
            return Ok(ToolResult {
                success: false,
                result: serde_json::json!(format!("Screenshot not found: {}", filepath)),
                metadata: None,
            });
        }
        
        // Read and encode the image
        let image_data = std::fs::read(path)?;
        let base64_image = general_purpose::STANDARD.encode(&image_data);
        
        let analysis_prompt = prompt.unwrap_or("Describe what you see in this screenshot. Include details about UI elements, text, colors, and any notable features.");
        
        // Return the encoded image and prompt for the agent to process
        // The actual vision API call will be handled by the agent/cloud providers
        Ok(ToolResult {
            success: true,
            result: serde_json::json!({
                "filepath": filepath,
                "status": "ready_for_analysis",
                "prompt": analysis_prompt
            }),
            metadata: Some(serde_json::json!({
                "filepath": filepath,
                "base64_image": base64_image,
                "prompt": analysis_prompt,
                "image_size": image_data.len(),
                "requires_vision_api": true,
                "estimated_cost": "$0.003"
            })),
        })
    }
    
    async fn list_screenshots(&self) -> Result<ToolResult> {
        let screenshots_dir = Path::new(&self.output_dir);
        
        if !screenshots_dir.exists() {
            return Ok(ToolResult {
                success: true,
                result: serde_json::json!({
                    "directory": self.output_dir,
                    "files": []
                }),
                metadata: Some(serde_json::json!({
                    "directory": self.output_dir,
                    "files": []
                })),
            });
        }
        
        let mut files = Vec::new();
        if let Ok(entries) = std::fs::read_dir(screenshots_dir) {
            for entry in entries.flatten() {
                if let Some(filename) = entry.file_name().to_str() {
                    if filename.ends_with(".png") || filename.ends_with(".jpg") || filename.ends_with(".jpeg") {
                        if let Ok(metadata) = entry.metadata() {
                            files.push(serde_json::json!({
                                "filename": filename,
                                "size": metadata.len(),
                                "modified": metadata.modified().ok()
                                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                    .map(|d| d.as_secs())
                            }));
                        }
                    }
                }
            }
        }
        
        files.sort_by(|a, b| {
            b.get("modified").and_then(|v| v.as_u64())
                .cmp(&a.get("modified").and_then(|v| v.as_u64()))
        });
        
        Ok(ToolResult {
            success: true,
            result: serde_json::json!({
                "directory": self.output_dir,
                "files": files,
                "count": files.len()
            }),
            metadata: Some(serde_json::json!({
                "directory": self.output_dir,
                "files": files
            })),
        })
    }
}

#[async_trait]
impl Tool for ScreenshotTool {
    fn name(&self) -> &str {
        "screenshot"
    }
    
    fn description(&self) -> &str {
        "Take screenshots of the screen or specific regions. Supports full screen capture and region selection on Windows, macOS, and Linux."
    }
    
    fn available_functions(&self) -> Vec<String> {
        vec![
            "capture".to_string(),
            "capture_region".to_string(),
            "list_screenshots".to_string(),
        ]
    }
    
    async fn execute(&self, function: &str, args: Value) -> Result<ToolResult> {
        match function {
            "capture" => {
                let filename = args.get("filename")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                
                self.take_screenshot(filename, None).await
            }
            "capture_region" => {
                let filename = args.get("filename")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                    
                let region = if let (Some(x), Some(y), Some(w), Some(h)) = (
                    args.get("x").and_then(|v| v.as_i64()).map(|i| i as i32),
                    args.get("y").and_then(|v| v.as_i64()).map(|i| i as i32),
                    args.get("width").and_then(|v| v.as_i64()).map(|i| i as i32),
                    args.get("height").and_then(|v| v.as_i64()).map(|i| i as i32),
                ) {
                    Some((x, y, w, h))
                } else {
                    None
                };
                
                self.take_screenshot(filename, region).await
            }
            "list_screenshots" => {
                self.list_screenshots().await
            }
            _ => Err(anyhow!("Unknown function: {}", function))
        }
    }
}

impl Default for ScreenshotTool {
    fn default() -> Self {
        Self::new(None)
    }
}
