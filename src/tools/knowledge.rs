use super::{Tool, ToolResult};
use crate::rag::store::KnowledgeStore;
use crate::rag::langchain_embedding::CandleEmbedder;
use async_trait::async_trait;
use anyhow::{Result, anyhow};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::fs;
use tracing::warn;

pub struct KnowledgeTool {
    store: Option<Arc<KnowledgeStore<CandleEmbedder>>>,
}

impl KnowledgeTool {
    pub async fn new() -> Result<Self> {
        let app_data = crate::utils::paths::get_air_data_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().to_string());

        let store = match KnowledgeStore::new(&app_data).await {
            Ok(s) => Some(Arc::new(s)),
            Err(e) => {
                warn!("⚠️ Failed to initialize Knowledge Store (RAG): {}. Knowledge features will be disabled.", e);
                None
            }
        };

        Ok(Self { store })
    }

    pub async fn add_file(&self, path_str: &str) -> Result<String> {
        if let Some(store) = &self.store {
            let path = std::path::Path::new(path_str);
            if !path.exists() {
                return Err(anyhow!("File not found: {}", path_str));
            }

            let content = fs::read_to_string(path).await?;
            let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();

            // Naive chunking: split by paragraphs
            let chunks: Vec<&str> = content.split("\n\n").collect();
            let mut added_chunks = 0;

            for chunk in chunks {
                if chunk.trim().len() < 20 { continue; } // Skip small chunks

                store.add_text(chunk, json!({
                    "source": path_str,
                    "filename": filename,
                    "type": "file"
                })).await?;
                added_chunks += 1;
            }

            Ok(format!("Indexed {} chunks from {}", added_chunks, path_str))
        } else {
            Err(anyhow!("Knowledge store is not available."))
        }
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
        if self.store.is_none() {
            return Ok(ToolResult {
                success: false,
                result: "Knowledge system is currently unavailable (initialization failed).".to_string(),
                metadata: None,
            });
        }
        let store = self.store.as_ref().unwrap();

        match function {
            "search_knowledge" => {
                let query = args["query"].as_str()
                    .ok_or_else(|| anyhow!("Missing 'query' parameter"))?;

                let results = store.search(query, 3).await?;

                if results.is_empty() {
                    return Ok(ToolResult {
                        success: true,
                        result: "No relevant information found in knowledge base.".to_string(),
                        metadata: None,
                    });
                }

                let mut result_text = String::new();
                for (doc, score) in results {
                    let source = doc.metadata.get("filename").and_then(|v| v.as_str()).unwrap_or("unknown");
                    result_text.push_str(&format!(
                        "[Score: {:.2}] (Source: {})\n{}\n\n",
                        score, source, doc.page_content
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
                    match self.add_file(p).await {
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
                    store.add_text(c, json!({"type": "manual_entry"})).await?;
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
