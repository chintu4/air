use super::{Tool, ToolResult};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde_json::Value;
use std::process::Command;
use std::io::{self, Write};
use std::collections::HashSet;

pub struct CommandTool {
    // Safe commands that don't require explicit permission
    safe_commands: HashSet<String>,
    // Whether to auto-approve safe commands
    auto_approve_safe: bool,
}

impl CommandTool {
    pub fn new() -> Self {
        let mut safe_commands = HashSet::new();
        
        // Add commonly safe read-only commands
        safe_commands.insert("dir".to_string());
        safe_commands.insert("ls".to_string());
        safe_commands.insert("pwd".to_string());
        safe_commands.insert("cd".to_string());
        safe_commands.insert("echo".to_string());
        safe_commands.insert("type".to_string());
        safe_commands.insert("cat".to_string());
        safe_commands.insert("head".to_string());
        safe_commands.insert("tail".to_string());
        safe_commands.insert("find".to_string());
        safe_commands.insert("grep".to_string());
        safe_commands.insert("which".to_string());
        safe_commands.insert("where".to_string());
        safe_commands.insert("whoami".to_string());
        safe_commands.insert("date".to_string());
        safe_commands.insert("time".to_string());
        safe_commands.insert("hostname".to_string());
        safe_commands.insert("ping".to_string());
        safe_commands.insert("git".to_string()); // Git commands are generally safe
        safe_commands.insert("cargo".to_string()); // Cargo commands for Rust development
        safe_commands.insert("node".to_string());
        safe_commands.insert("npm".to_string());
        safe_commands.insert("python".to_string());
        safe_commands.insert("rustc".to_string());
        
        Self {
            safe_commands,
            auto_approve_safe: true,
        }
    }
    
    fn is_safe_command(&self, command: &str) -> bool {
        let parts: Vec<&str> = command.trim().split_whitespace().collect();
        if parts.is_empty() {
            return false;
        }
        
        let base_command = parts[0].to_lowercase();
        
        // Check if it's in our safe list
        if self.safe_commands.contains(&base_command) {
            // Additional safety checks for specific commands
            match base_command.as_str() {
                "rm" | "del" | "rmdir" | "format" | "shutdown" | "reboot" => false,
                "git" => {
                    // Allow most git commands except potentially destructive ones
                    if parts.len() > 1 {
                        match parts[1] {
                            "push" | "commit" | "add" | "merge" | "rebase" => {
                                // These require permission as they modify state
                                false
                            }
                            _ => true
                        }
                    } else {
                        true
                    }
                }
                "cargo" => {
                    // Allow most cargo commands
                    if parts.len() > 1 {
                        match parts[1] {
                            "publish" | "install" => false, // These can modify system
                            _ => true
                        }
                    } else {
                        true
                    }
                }
                _ => true
            }
        } else {
            false
        }
    }
    
    fn request_permission(&self, command: &str) -> Result<bool> {
        println!("\n🔐 Command Execution Permission Required");
        println!("═══════════════════════════════════════");
        println!("📋 Command: {}", command);
        println!("⚠️  This command will be executed on your system.");
        println!("💡 Review the command carefully before proceeding.");
        print!("\n❓ Do you want to execute this command? (y/N): ");
        
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        let response = input.trim().to_lowercase();
        Ok(response == "y" || response == "yes")
    }
    
    async fn execute_command(&self, command: &str) -> Result<ToolResult> {
        // Determine if we need permission
        let needs_permission = !self.is_safe_command(command) || !self.auto_approve_safe;
        
        if needs_permission {
            println!("\n🤖 AI wants to execute: {}", command);
            if !self.request_permission(command)? {
                return Ok(ToolResult {
                    success: false,
                    result: serde_json::json!("Command execution cancelled by user."),
                    metadata: Some(serde_json::json!({
                        "cancelled": true,
                        "command": command
                    })),
                });
            }
        }
        
        // Execute the command
        println!("⚡ Executing: {}", command);
        
        let output = if cfg!(target_os = "windows") {
            Command::new("powershell")
                .args(["-Command", command])
                .output()
        } else {
            Command::new("sh")
                .args(["-c", command])
                .output()
        };
        
        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                
                let result_json = serde_json::json!({
                    "stdout": stdout,
                    "stderr": stderr,
                    "exit_code": output.status.code()
                });
                
                Ok(ToolResult {
                    success: output.status.success(),
                    result: result_json,
                    metadata: Some(serde_json::json!({
                        "command": command,
                    })),
                })
            }
            Err(e) => {
                Ok(ToolResult {
                    success: false,
                    result: serde_json::json!(format!("Failed to execute command: {}", e)),
                    metadata: Some(serde_json::json!({
                        "error": e.to_string(),
                        "command": command
                    })),
                })
            }
        }
    }
}

#[async_trait]
impl Tool for CommandTool {
    fn name(&self) -> &str {
        "command"
    }
    
    fn description(&self) -> &str {
        "Execute operating system commands with user permission. Supports both safe commands (automatically approved) and potentially dangerous commands (requires explicit user permission)."
    }
    
    fn available_functions(&self) -> Vec<String> {
        vec![
            "execute".to_string(),
            "execute_safe".to_string(),
            "list_safe_commands".to_string(),
        ]
    }
    
    async fn execute(&self, function: &str, args: Value) -> Result<ToolResult> {
        match function {
            "execute" => {
                let command = args.get("command")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'command' argument"))?;
                
                self.execute_command(command).await
            }
            "execute_safe" => {
                let command = args.get("command")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'command' argument"))?;
                
                if self.is_safe_command(command) {
                    self.execute_command(command).await
                } else {
                    Ok(ToolResult {
                        success: false,
                        result: serde_json::json!(format!("Command '{}' is not in the safe commands list. Use 'execute' function for explicit permission.", command)),
                        metadata: Some(serde_json::json!({
                            "safe": false,
                            "command": command
                        })),
                    })
                }
            }
            "list_safe_commands" => {
                let safe_list: Vec<String> = self.safe_commands.iter().cloned().collect();
                Ok(ToolResult {
                    success: true,
                    result: serde_json::json!({
                        "safe_commands": safe_list
                    }),
                    metadata: None,
                })
            }
            _ => Err(anyhow!("Unknown function: {}", function))
        }
    }
}

impl Default for CommandTool {
    fn default() -> Self {
        Self::new()
    }
}
