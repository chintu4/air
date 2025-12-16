use super::{Tool, ToolResult};
use async_trait::async_trait;
use anyhow::{Result, anyhow};
use serde_json::{json, Value};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: String,
    pub priority: Priority,
    pub status: TaskStatus,
    pub created_at: DateTime<Utc>,
    pub due_date: Option<DateTime<Utc>>,
    pub subtasks: Vec<String>,
    pub dependencies: Vec<String>,
    pub estimated_duration: Option<u32>, // minutes
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    NotStarted,
    InProgress,
    Completed,
    Blocked,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tasks: Vec<String>, // Task IDs
    pub created_at: DateTime<Utc>,
    pub estimated_completion: Option<DateTime<Utc>>,
}

pub struct PlannerTool {
    tasks: std::sync::Arc<std::sync::Mutex<HashMap<String, Task>>>,
}

impl PlannerTool {
    pub fn new() -> Self {
        Self {
            tasks: std::sync::Arc::new(std::sync::Mutex::new(HashMap::new())),
        }
    }
    
    fn parse_priority(&self, priority_str: &str) -> Priority {
        match priority_str.to_lowercase().as_str() {
            "low" => Priority::Low,
            "medium" => Priority::Medium,
            "high" => Priority::High,
            "critical" => Priority::Critical,
            _ => Priority::Medium,
        }
    }
    
    fn parse_status(&self, status_str: &str) -> TaskStatus {
        match status_str.to_lowercase().as_str() {
            "not_started" | "todo" => TaskStatus::NotStarted,
            "in_progress" | "doing" => TaskStatus::InProgress,
            "completed" | "done" => TaskStatus::Completed,
            "blocked" => TaskStatus::Blocked,
            "cancelled" => TaskStatus::Cancelled,
            _ => TaskStatus::NotStarted,
        }
    }
    
    fn break_down_complex_task(&self, description: &str) -> Vec<String> {
        // Simple heuristic to break down complex tasks
        let keywords = ["and", "then", "after", "also", "additionally", "furthermore"];
        
        let mut subtasks = Vec::new();
        let mut current_task = String::new();
        
        for word in description.split_whitespace() {
            current_task.push_str(word);
            current_task.push(' ');
            
            if keywords.iter().any(|&kw| word.to_lowercase().contains(kw)) {
                if !current_task.trim().is_empty() {
                    subtasks.push(current_task.trim().to_string());
                    current_task.clear();
                }
            }
        }
        
        if !current_task.trim().is_empty() {
            subtasks.push(current_task.trim().to_string());
        }
        
        // If no breakdown detected, return the original as a single task
        if subtasks.len() <= 1 {
            subtasks = vec![description.to_string()];
        }
        
        subtasks
    }
}

#[async_trait]
impl Tool for PlannerTool {
    fn name(&self) -> &str {
        "planner"
    }
    
    fn description(&self) -> &str {
        "Task planning and breakdown: create, manage, and track tasks and plans"
    }
    
    fn available_functions(&self) -> Vec<String> {
        vec![
            "create_task".to_string(),
            "update_task".to_string(),
            "list_tasks".to_string(),
            "break_down_task".to_string(),
            "create_plan".to_string(),
            "suggest_next_action".to_string(),
            "get_task_status".to_string(),
            "estimate_completion".to_string(),
        ]
    }
    
    async fn execute(&self, function: &str, args: Value) -> Result<ToolResult> {
        match function {
            "create_task" => {
                let title = args["title"].as_str()
                    .ok_or_else(|| anyhow!("Missing 'title' parameter"))?;
                let description = args["description"].as_str()
                    .unwrap_or(title);
                let priority = args["priority"].as_str().unwrap_or("medium");
                let tags: Vec<String> = args["tags"].as_array()
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                    .unwrap_or_default();
                
                let task = Task {
                    id: uuid::Uuid::new_v4().to_string(),
                    title: title.to_string(),
                    description: description.to_string(),
                    priority: self.parse_priority(priority),
                    status: TaskStatus::NotStarted,
                    created_at: Utc::now(),
                    due_date: None,
                    subtasks: Vec::new(),
                    dependencies: Vec::new(),
                    estimated_duration: args["duration"].as_u64().map(|d| d as u32),
                    tags,
                };
                
                let task_id = task.id.clone();
                let mut tasks = self.tasks.lock().unwrap();
                tasks.insert(task_id.clone(), task);
                
                Ok(ToolResult {
                    success: true,
                    result: json!({
                        "task_id": task_id,
                        "title": title,
                        "status": "created"
                    }),
                    metadata: Some(json!({
                        "task_id": task_id,
                        "title": title
                    })),
                })
            }
            
            "break_down_task" => {
                let description = args["description"].as_str()
                    .ok_or_else(|| anyhow!("Missing 'description' parameter"))?;
                
                let subtasks = self.break_down_complex_task(description);
                
                Ok(ToolResult {
                    success: true,
                    result: json!({
                        "original_task": description,
                        "subtasks": subtasks
                    }),
                    metadata: Some(json!({
                        "original_task": description,
                        "subtasks": subtasks,
                        "subtask_count": subtasks.len()
                    })),
                })
            }
            
            "list_tasks" => {
                let status_filter = args["status"].as_str();
                let priority_filter = args["priority"].as_str();
                
                let tasks = self.tasks.lock().unwrap();
                let filtered_tasks: Vec<_> = tasks.values()
                    .filter(|task| {
                        if let Some(status) = status_filter {
                            let status_enum = self.parse_status(status);
                            if !matches!((&task.status, &status_enum), 
                                (TaskStatus::NotStarted, TaskStatus::NotStarted) |
                                (TaskStatus::InProgress, TaskStatus::InProgress) |
                                (TaskStatus::Completed, TaskStatus::Completed) |
                                (TaskStatus::Blocked, TaskStatus::Blocked) |
                                (TaskStatus::Cancelled, TaskStatus::Cancelled)) {
                                return false;
                            }
                        }
                        
                        if let Some(priority) = priority_filter {
                            let priority_enum = self.parse_priority(priority);
                            if !matches!((&task.priority, &priority_enum),
                                (Priority::Low, Priority::Low) |
                                (Priority::Medium, Priority::Medium) |
                                (Priority::High, Priority::High) |
                                (Priority::Critical, Priority::Critical)) {
                                return false;
                            }
                        }
                        
                        true
                    })
                    .collect();
                
                Ok(ToolResult {
                    success: true,
                    result: json!(filtered_tasks),
                    metadata: Some(json!({
                        "total_tasks": tasks.len(),
                        "filtered_tasks": filtered_tasks.len()
                    })),
                })
            }
            
            "suggest_next_action" => {
                let tasks = self.tasks.lock().unwrap();
                
                // Find highest priority, non-completed tasks
                let mut pending_tasks: Vec<_> = tasks.values()
                    .filter(|task| !matches!(task.status, TaskStatus::Completed | TaskStatus::Cancelled))
                    .collect();
                
                // Sort by priority and creation date
                pending_tasks.sort_by(|a, b| {
                    use std::cmp::Ordering;
                    
                    let priority_order = |p: &Priority| match p {
                        Priority::Critical => 0,
                        Priority::High => 1,
                        Priority::Medium => 2,
                        Priority::Low => 3,
                    };
                    
                    let a_priority = priority_order(&a.priority);
                    let b_priority = priority_order(&b.priority);
                    
                    match a_priority.cmp(&b_priority) {
                        Ordering::Equal => a.created_at.cmp(&b.created_at),
                        other => other,
                    }
                });
                
                if let Some(next_task) = pending_tasks.first() {
                    Ok(ToolResult {
                        success: true,
                        result: json!(next_task),
                        metadata: Some(json!({
                            "pending_tasks_count": pending_tasks.len(),
                            "next_task_id": next_task.id
                        })),
                    })
                } else {
                    Ok(ToolResult {
                        success: true,
                        result: json!({
                            "message": "No pending tasks found",
                            "pending_tasks_count": 0
                        }),
                        metadata: Some(json!({
                            "pending_tasks_count": 0,
                            "next_task_id": null
                        })),
                    })
                }
            }
            
            "update_task" => {
                let task_id = args["task_id"].as_str()
                    .ok_or_else(|| anyhow!("Missing 'task_id' parameter"))?;
                
                let mut tasks = self.tasks.lock().unwrap();
                
                if let Some(task) = tasks.get_mut(task_id) {
                    let mut updated_fields = Vec::new();
                    
                    if let Some(status) = args["status"].as_str() {
                        task.status = self.parse_status(status);
                        updated_fields.push(format!("status: {:?}", task.status));
                    }
                    
                    if let Some(priority) = args["priority"].as_str() {
                        task.priority = self.parse_priority(priority);
                        updated_fields.push(format!("priority: {:?}", task.priority));
                    }
                    
                    if let Some(title) = args["title"].as_str() {
                        task.title = title.to_string();
                        updated_fields.push(format!("title: {}", title));
                    }
                    
                    Ok(ToolResult {
                        success: true,
                        result: json!({
                            "task_id": task_id,
                            "updated_fields": updated_fields,
                            "task": task
                        }),
                        metadata: Some(json!({
                            "task_id": task_id,
                            "updated_fields": updated_fields
                        })),
                    })
                } else {
                    Ok(ToolResult {
                        success: false,
                        result: json!(format!("Task not found: {}", task_id)),
                        metadata: None,
                    })
                }
            }
            
            _ => Err(anyhow!("Unknown function: {}", function))
        }
    }
}
