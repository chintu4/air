use super::{Tool, ToolResult};
use crate::rag::VectorStore;
use async_trait::async_trait;
use anyhow::{Result, anyhow};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use std::fs;

pub struct KnowledgeTool {
    store: Arc<Mutex<VectorStore>>,
}

impl KnowledgeTool {
    pub fn new() -> Result<Self> {
        Ok(Self {
            store: Arc::new(Mutex::new(VectorStore::new()?)),
        })
    }

    pub fn add_file(&self, path_str: &str) -> Result<String> {
        let path = std::path::Path::new(path_str);
        if !path.exists() {
            return Err(anyhow!("File not found: {}", path_str));
        }

        let content = fs::read_to_string(path)?;
        let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();

        let mut store = self.store.lock().unwrap();

        // Naive chunking: split by paragraphs or chunks of 512 chars
        // For now, let's do simple paragraph splitting
        let chunks: Vec<&str> = content.split("\n\n").collect();
        let mut added_chunks = 0;

        for chunk in chunks {
            if chunk.trim().len() < 20 { continue; } // Skip small chunks

            store.add_text(chunk, json!({
                "source": path_str,
                "filename": filename,
                "type": "file"
            }))?;
            added_chunks += 1;
        }

        Ok(format!("Indexed {} chunks from {}", added_chunks, path_str))
    }
}

#[async_trait]
impl Tool for KnowledgeTool {
    fn name(&self) -> &str {
        "knowledge"
    }

    fn description(&self) -> &str {
        "Long-term memory and knowledge retrieval from indexed files"
    }

    fn available_functions(&self) -> Vec<String> {
        vec![
            "search_knowledge".to_string(),
            "add_knowledge".to_string(),
        ]
    }

    async fn execute(&self, function: &str, args: Value) -> Result<ToolResult> {
        match function {
            "search_knowledge" => {
                let query = args["query"].as_str()
                    .ok_or_else(|| anyhow!("Missing 'query' parameter"))?;

                let mut store = self.store.lock().unwrap();
                let results = store.search(query, 3)?;

                if results.is_empty() {
                    return Ok(ToolResult {
                        success: true,
                        result: "No relevant information found in knowledge base.".to_string(),
                        metadata: None,
                    });
                }

                let mut result_text = String::new();
                for (doc, score) in results {
                    let source = doc.metadata["filename"].as_str().unwrap_or("unknown");
                    result_text.push_str(&format!(
                        "[Score: {:.2}] (Source: {})\n{}\n\n",
                        score, source, doc.content
                    ));
                }

                Ok(ToolResult {
                    success: true,
                    result: result_text,
                    metadata: None,
                })
            }

            "add_knowledge" => {
                let content = args["content"].as_str();
                let path = args["path"].as_str();

                if let Some(p) = path {
                    match self.add_file(p) {
                        Ok(msg) => Ok(ToolResult {
                            success: true,
                            result: msg,
                            metadata: None,
                        }),
                        Err(e) => Ok(ToolResult {
                            success: false,
                            result: format!("Failed to index file: {}", e),
                            metadata: None,
                        }),
                    }
                } else if let Some(c) = content {
                    let mut store = self.store.lock().unwrap();
                    store.add_text(c, json!({"type": "manual_entry"}))?;
                    Ok(ToolResult {
                        success: true,
                        result: "Added text content to knowledge base.".to_string(),
                        metadata: None,
                    })
                } else {
                    Err(anyhow!("Must provide 'path' or 'content'"))
                }
            }

            _ => Err(anyhow!("Unknown function: {}", function))
        }
    }
}
