use super::{Tool, ToolResult};
use async_trait::async_trait;
use anyhow::{Result, anyhow};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub user_input: String,
    pub ai_response: String,
    pub context: Option<String>,
    pub tools_used: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSummary {
    pub total_exchanges: usize,
    pub topics_discussed: Vec<String>,
    pub tools_used: HashMap<String, usize>,
    pub start_time: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
}

pub struct MemoryTool {
    conversations: Arc<Mutex<Vec<ConversationEntry>>>,
    session_data: Arc<Mutex<HashMap<String, Value>>>,
    max_history: usize,
}

impl MemoryTool {
    pub fn new(max_history: Option<usize>) -> Self {
        Self {
            conversations: Arc::new(Mutex::new(Vec::new())),
            session_data: Arc::new(Mutex::new(HashMap::new())),
            max_history: max_history.unwrap_or(100),
        }
    }
    
    pub fn add_conversation(&self, user_input: String, ai_response: String, context: Option<String>, tools_used: Vec<String>) -> Result<String> {
        let entry = ConversationEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            user_input,
            ai_response,
            context,
            tools_used,
        };
        
        let entry_id = entry.id.clone();
        
        let mut conversations = self.conversations.lock().unwrap();
        conversations.push(entry);
        
        // Keep only the last max_history entries
        let current_len = conversations.len();
        if current_len > self.max_history {
            let excess = current_len - self.max_history;
            conversations.drain(0..excess);
        }
        
        Ok(entry_id)
    }
    
    fn get_conversation_summary(&self) -> ConversationSummary {
        let conversations = self.conversations.lock().unwrap();
        
        if conversations.is_empty() {
            return ConversationSummary {
                total_exchanges: 0,
                topics_discussed: Vec::new(),
                tools_used: HashMap::new(),
                start_time: Utc::now(),
                last_activity: Utc::now(),
            };
        }
        
        let mut tools_used = HashMap::new();
        let mut topics = Vec::new();
        
        for entry in conversations.iter() {
            // Count tool usage
            for tool in &entry.tools_used {
                *tools_used.entry(tool.clone()).or_insert(0) += 1;
            }
            
            // Extract potential topics from user input (simple keyword extraction)
            let words: Vec<&str> = entry.user_input
                .split_whitespace()
                .filter(|w| w.len() > 4)
                .collect();
            topics.extend(words.iter().map(|w| w.to_lowercase()));
        }
        
        // Remove duplicates and keep only the most frequent topics
        topics.sort();
        topics.dedup();
        topics.truncate(10);
        
        ConversationSummary {
            total_exchanges: conversations.len(),
            topics_discussed: topics,
            tools_used,
            start_time: conversations.first().unwrap().timestamp,
            last_activity: conversations.last().unwrap().timestamp,
        }
    }
    
    fn search_conversations(&self, query: &str) -> Vec<ConversationEntry> {
        let conversations = self.conversations.lock().unwrap();
        let query_lower = query.to_lowercase();
        
        conversations.iter()
            .filter(|entry| {
                entry.user_input.to_lowercase().contains(&query_lower) ||
                entry.ai_response.to_lowercase().contains(&query_lower) ||
                entry.context.as_ref().map_or(false, |c| c.to_lowercase().contains(&query_lower))
            })
            .cloned()
            .collect()
    }
}

#[async_trait]
impl Tool for MemoryTool {
    fn name(&self) -> &str {
        "memory"
    }
    
    fn description(&self) -> &str {
        "Conversation memory and context management"
    }
    
    fn available_functions(&self) -> Vec<String> {
        vec![
            "get_recent_history".to_string(),
            "search_conversations".to_string(),
            "get_summary".to_string(),
            "store_data".to_string(),
            "retrieve_data".to_string(),
            "clear_history".to_string(),
        ]
    }
    
    async fn execute(&self, function: &str, args: Value) -> Result<ToolResult> {
        match function {
            "get_recent_history" => {
                let limit = args["limit"].as_u64().unwrap_or(5) as usize;
                
                let conversations = self.conversations.lock().unwrap();
                let recent: Vec<_> = conversations.iter()
                    .rev()
                    .take(limit)
                    .collect();
                
                let mut result = String::new();
                for (i, entry) in recent.iter().enumerate() {
                    result.push_str(&format!(
                        "{}. [{}] User: {}\n   AI: {}\n\n",
                        i + 1,
                        entry.timestamp.format("%H:%M:%S"),
                        entry.user_input,
                        if entry.ai_response.len() > 100 {
                            format!("{}...", &entry.ai_response[..100])
                        } else {
                            entry.ai_response.clone()
                        }
                    ));
                }
                
                if result.is_empty() {
                    result = "No conversation history found.".to_string();
                }
                
                Ok(ToolResult {
                    success: true,
                    result,
                    metadata: Some(json!({
                        "total_entries": conversations.len(),
                        "returned_entries": recent.len()
                    })),
                })
            }
            
            "search_conversations" => {
                let query = args["query"].as_str()
                    .ok_or_else(|| anyhow!("Missing 'query' parameter"))?;
                
                let conversations = self.conversations.lock().unwrap();
                let query_lower = query.to_lowercase();
                
                let matches: Vec<_> = conversations.iter()
                    .filter(|entry| {
                        entry.user_input.to_lowercase().contains(&query_lower) ||
                        entry.ai_response.to_lowercase().contains(&query_lower)
                    })
                    .collect();
                
                let mut result = String::new();
                for (i, entry) in matches.iter().enumerate() {
                    result.push_str(&format!(
                        "{}. [{}] User: {}\n   AI: {}\n\n",
                        i + 1,
                        entry.timestamp.format("%m/%d %H:%M"),
                        entry.user_input,
                        if entry.ai_response.len() > 150 {
                            format!("{}...", &entry.ai_response[..150])
                        } else {
                            entry.ai_response.clone()
                        }
                    ));
                }
                
                if result.is_empty() {
                    result = format!("No conversations found matching '{}'", query);
                }
                
                Ok(ToolResult {
                    success: true,
                    result,
                    metadata: Some(json!({
                        "query": query,
                        "matches_found": matches.len()
                    })),
                })
            }
            
            "get_summary" => {
                let summary = self.get_conversation_summary();
                
                let result = format!(
                    "Conversation Summary:\n\
                     Total exchanges: {}\n\
                     Session started: {}\n\
                     Last activity: {}\n\
                     Tools used: {:?}\n\
                     Topics discussed: {}",
                    summary.total_exchanges,
                    summary.start_time.format("%Y-%m-%d %H:%M:%S"),
                    summary.last_activity.format("%Y-%m-%d %H:%M:%S"),
                    summary.tools_used,
                    summary.topics_discussed.join(", ")
                );
                
                Ok(ToolResult {
                    success: true,
                    result,
                    metadata: Some(json!(summary)),
                })
            }
            
            "store_data" => {
                let key = args["key"].as_str()
                    .ok_or_else(|| anyhow!("Missing 'key' parameter"))?;
                let value = args["value"].clone();
                
                let mut session_data = self.session_data.lock().unwrap();
                session_data.insert(key.to_string(), value.clone());
                
                Ok(ToolResult {
                    success: true,
                    result: format!("Stored data with key: {}", key),
                    metadata: Some(json!({
                        "key": key,
                        "value": value
                    })),
                })
            }
            
            "retrieve_data" => {
                let key = args["key"].as_str()
                    .ok_or_else(|| anyhow!("Missing 'key' parameter"))?;
                
                let session_data = self.session_data.lock().unwrap();
                
                if let Some(value) = session_data.get(key) {
                    Ok(ToolResult {
                        success: true,
                        result: format!("Retrieved data for key '{}': {}", key, value),
                        metadata: Some(json!({
                            "key": key,
                            "value": value
                        })),
                    })
                } else {
                    Ok(ToolResult {
                        success: false,
                        result: format!("No data found for key: {}", key),
                        metadata: None,
                    })
                }
            }
            
            "clear_history" => {
                let confirm = args["confirm"].as_bool().unwrap_or(false);
                
                if !confirm {
                    return Ok(ToolResult {
                        success: false,
                        result: "Please confirm history clearing by setting 'confirm': true".to_string(),
                        metadata: None,
                    });
                }
                
                let mut conversations = self.conversations.lock().unwrap();
                let cleared_count = conversations.len();
                conversations.clear();
                
                let mut session_data = self.session_data.lock().unwrap();
                session_data.clear();
                
                Ok(ToolResult {
                    success: true,
                    result: format!("Cleared {} conversation entries and all session data", cleared_count),
                    metadata: Some(json!({
                        "cleared_conversations": cleared_count
                    })),
                })
            }
            
            _ => Err(anyhow!("Unknown function: {}", function))
        }
    }
}
