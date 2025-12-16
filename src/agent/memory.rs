use anyhow::Result;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool, Row};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{info, warn};
use md5;
use crate::rag::store::KnowledgeStore;
use crate::rag::langchain_embedding::CandleEmbedder;

#[derive(Debug, Clone)]
pub struct Conversation {
    pub id: i64,
    pub user_input: String,
    pub ai_response: String,
    pub timestamp: String,
    pub context: Option<String>,
    pub tools_used: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Mistake {
    pub id: i64,
    pub session_id: String,
    pub user_input: String,
    pub ai_response: Option<String>,
    pub error_type: String,
    pub error_message: String,
    pub context: Option<String>,
    pub timestamp: String,
    pub learned: bool,
}

#[derive(Debug, Clone)]
pub struct LearningPattern {
    pub pattern: String,
    pub mistake_count: i32,
    pub success_count: i32,
    pub last_updated: String,
}

pub struct MemoryManager {
    ram_pool: SqlitePool,
    rom_pool: SqlitePool,
    about_pool: SqlitePool,
    knowledge_store: Option<KnowledgeStore<CandleEmbedder>>,
}

impl MemoryManager {
    pub async fn new(app_data: &str) -> Result<Self> {
        let ram_db_path = std::path::Path::new(app_data).join("air").join("ram_memory.db");
        let rom_db_path = std::path::Path::new(app_data).join("air").join("rom_memory.db");
        let about_db_path = std::path::Path::new(app_data).join("air").join("about_memory.db");

        // Ensure directory exists
        if let Some(parent) = ram_db_path.parent() {
            if !parent.exists() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }

        // Initialize RAM memory (clear it)
        if ram_db_path.exists() {
             tokio::fs::remove_file(&ram_db_path).await.ok();
        }
        tokio::fs::File::create(&ram_db_path).await?;

        let ram_pool = SqlitePoolOptions::new()
            .connect(&format!("sqlite://{}", ram_db_path.to_string_lossy()))
            .await?;

        sqlx::query(
            "CREATE TABLE conversations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_input TEXT NOT NULL,
                ai_response TEXT NOT NULL,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                context TEXT,
                tools_used TEXT
            )"
        ).execute(&ram_pool).await?;

        sqlx::query(
            "CREATE TABLE memory (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
            )"
        ).execute(&ram_pool).await?;

        // Initialize ROM memory
        if !rom_db_path.exists() {
            tokio::fs::File::create(&rom_db_path).await?;
        }
        let rom_pool = SqlitePoolOptions::new()
            .connect(&format!("sqlite://{}", rom_db_path.to_string_lossy()))
            .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS persistent_memory (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
            )"
        ).execute(&rom_pool).await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS user_preferences (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
            )"
        ).execute(&rom_pool).await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS mistakes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT,
                user_input TEXT NOT NULL,
                ai_response TEXT,
                error_type TEXT NOT NULL,
                error_message TEXT NOT NULL,
                context TEXT,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                learned BOOLEAN DEFAULT FALSE
            )"
        ).execute(&rom_pool).await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS learning_patterns (
                pattern TEXT PRIMARY KEY,
                mistake_count INTEGER DEFAULT 0,
                success_count INTEGER DEFAULT 0,
                last_updated DATETIME DEFAULT CURRENT_TIMESTAMP
            )"
        ).execute(&rom_pool).await?;

        // Initialize ABOUT memory
        if !about_db_path.exists() {
             tokio::fs::File::create(&about_db_path).await?;
        }
        let about_pool = SqlitePoolOptions::new()
            .connect(&format!("sqlite://{}", about_db_path.to_string_lossy()))
            .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS air_info (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )"
        ).execute(&about_pool).await?;

        // Defaults
        let defaults = vec![
            ("creator", "Chintu (dsjapnc)"),
            ("version", "0.1.0"),
            ("description", "I am air, an AI Agent with local and cloud model fallback"),
            ("repository", "https://github.com/chintu4/air"),
        ];

        for (key, value) in defaults {
            sqlx::query("INSERT OR IGNORE INTO air_info (key, value) VALUES (?, ?)")
                .bind(key)
                .bind(value)
                .execute(&about_pool)
                .await?;
        }

        // Initialize Knowledge Store with CandleEmbedder
        let knowledge_store = match KnowledgeStore::new(app_data).await {
            Ok(store) => Some(store),
            Err(e) => {
                warn!("⚠️ Failed to initialize Memory Knowledge Store: {}. Context recall disabled.", e);
                None
            }
        };

        Ok(Self {
            ram_pool,
            rom_pool,
            about_pool,
            knowledge_store,
        })
    }

    pub async fn store_conversations_batch(&self, conversations: Vec<(String, String, Option<String>, Option<String>)>) -> Result<()> {
        if conversations.is_empty() {
            return Ok(());
        }

        let mut tx = self.ram_pool.begin().await?;

        for (user_input, ai_response, context, tools_used) in conversations {
            let compressed_input = if user_input.len() > 500 {
                format!("{}... (truncated)", &user_input[..200])
            } else {
                user_input
            };

            let compressed_response = if ai_response.len() > 1000 {
                format!("{}... (truncated)", &ai_response[..500])
            } else {
                ai_response
            };

            sqlx::query("INSERT INTO conversations (user_input, ai_response, context, tools_used) VALUES (?, ?, ?, ?)")
                .bind(compressed_input)
                .bind(compressed_response)
                .bind(context.unwrap_or_default())
                .bind(tools_used.unwrap_or_default())
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn store_ram_memory(&self, key: &str, value: &str) -> Result<()> {
        sqlx::query("INSERT OR REPLACE INTO memory (key, value) VALUES (?, ?)")
            .bind(key)
            .bind(value)
            .execute(&self.ram_pool)
            .await?;
        Ok(())
    }

    pub async fn get_ram_memory(&self, key: &str) -> Result<Option<String>> {
        let result = sqlx::query("SELECT value FROM memory WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.ram_pool)
            .await?;

        if let Some(row) = result {
            Ok(Some(row.get(0)))
        } else {
            Ok(None)
        }
    }

    pub async fn store_persistent_memory(&self, key: &str, value: &str) -> Result<()> {
        sqlx::query("INSERT OR REPLACE INTO persistent_memory (key, value) VALUES (?, ?)")
            .bind(key)
            .bind(value)
            .execute(&self.rom_pool)
            .await?;
        Ok(())
    }

    pub async fn get_persistent_memory(&self, key: &str) -> Result<Option<String>> {
        let result = sqlx::query("SELECT value FROM persistent_memory WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.rom_pool)
            .await?;

        if let Some(row) = result {
            Ok(Some(row.get(0)))
        } else {
            Ok(None)
        }
    }

    pub async fn store_user_preference(&self, key: &str, value: &str) -> Result<()> {
        sqlx::query("INSERT OR REPLACE INTO user_preferences (key, value) VALUES (?, ?)")
            .bind(key)
            .bind(value)
            .execute(&self.rom_pool)
            .await?;
        Ok(())
    }

    pub async fn get_user_preference(&self, key: &str) -> Result<Option<String>> {
        let result = sqlx::query("SELECT value FROM user_preferences WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.rom_pool)
            .await?;

        if let Some(row) = result {
            Ok(Some(row.get(0)))
        } else {
            Ok(None)
        }
    }

    pub async fn get_air_info(&self, key: &str) -> Result<Option<String>> {
        let result = sqlx::query("SELECT value FROM air_info WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.about_pool)
            .await?;

        if let Some(row) = result {
            Ok(Some(row.get(0)))
        } else {
            Ok(None)
        }
    }

    pub async fn get_recent_conversations(&self, limit: usize) -> Result<Vec<(String, String, String)>> {
        // Cleanup if needed
        let count: i64 = sqlx::query("SELECT COUNT(*) FROM conversations")
            .fetch_one(&self.ram_pool)
            .await?
            .get(0);

        if count > 1000 {
            info!("🧹 Cleaning up old conversations ({} > 1000)", count);
            sqlx::query("DELETE FROM conversations WHERE id IN (SELECT id FROM conversations ORDER BY timestamp DESC LIMIT -1 OFFSET 500)")
                .execute(&self.ram_pool)
                .await?;
        }

        let rows = sqlx::query("SELECT user_input, ai_response, timestamp FROM conversations ORDER BY timestamp DESC LIMIT ?")
            .bind(limit as i64)
            .fetch_all(&self.ram_pool)
            .await?;

        let mut conversations = Vec::new();
        for row in rows {
            conversations.push((
                row.get(0),
                row.get(1),
                row.get(2),
            ));
        }
        conversations.reverse();
        Ok(conversations)
    }

    pub async fn perform_maintenance(&self) -> Result<()> {
        info!("🔧 Performing database maintenance...");

        sqlx::query("VACUUM").execute(&self.ram_pool).await?;
        sqlx::query("DELETE FROM conversations WHERE timestamp < datetime('now', '-1 day')").execute(&self.ram_pool).await?;
        sqlx::query("DELETE FROM memory WHERE timestamp < datetime('now', '-1 day')").execute(&self.ram_pool).await?;

        sqlx::query("VACUUM").execute(&self.rom_pool).await?;
        sqlx::query("DELETE FROM mistakes WHERE timestamp < datetime('now', '-30 days')").execute(&self.rom_pool).await?;

        sqlx::query("VACUUM").execute(&self.about_pool).await?;

        info!("✅ Database maintenance completed");
        Ok(())
    }

    pub async fn store_mistake(&self, session_id: &str, user_input: &str, ai_response: Option<&str>,
                        error_type: &str, error_message: &str, context: Option<&str>) -> Result<i64> {
        let result = sqlx::query(
            "INSERT INTO mistakes (session_id, user_input, ai_response, error_type, error_message, context)
             VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(session_id)
        .bind(user_input)
        .bind(ai_response.unwrap_or(""))
        .bind(error_type)
        .bind(error_message)
        .bind(context.unwrap_or(""))
        .execute(&self.rom_pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn mark_mistake_learned(&self, mistake_id: i64) -> Result<()> {
        sqlx::query("UPDATE mistakes SET learned = TRUE WHERE id = ?")
            .bind(mistake_id)
            .execute(&self.rom_pool)
            .await?;
        Ok(())
    }

    pub async fn get_unlearned_mistakes(&self, error_type: Option<&str>, limit: usize) -> Result<Vec<(i64, String, String, String, String)>> {
        let query_str = if error_type.is_some() {
            "SELECT id, user_input, error_type, error_message, context FROM mistakes
             WHERE learned = FALSE AND error_type = ? ORDER BY timestamp DESC LIMIT ?"
        } else {
            "SELECT id, user_input, error_type, error_message, context FROM mistakes
             WHERE learned = FALSE ORDER BY timestamp DESC LIMIT ?"
        };

        let rows = if let Some(err_type) = error_type {
            sqlx::query(query_str)
                .bind(err_type)
                .bind(limit as i64)
                .fetch_all(&self.rom_pool)
                .await?
        } else {
            sqlx::query(query_str)
                .bind(limit as i64)
                .fetch_all(&self.rom_pool)
                .await?
        };

        let mut mistakes = Vec::new();
        for row in rows {
            mistakes.push((
                row.get(0),
                row.get(1),
                row.get(2),
                row.get(3),
                row.get(4),
            ));
        }
        Ok(mistakes)
    }

    pub async fn update_learning_pattern(&self, pattern: &str, was_success: bool) -> Result<()> {
        if was_success {
             sqlx::query("INSERT OR IGNORE INTO learning_patterns (pattern, success_count) VALUES (?, 1)")
                .bind(pattern)
                .execute(&self.rom_pool)
                .await?;
             sqlx::query(
                "UPDATE learning_patterns SET success_count = success_count + 1, last_updated = CURRENT_TIMESTAMP
                 WHERE pattern = ?"
             ).bind(pattern).execute(&self.rom_pool).await?;
        } else {
            sqlx::query("INSERT OR IGNORE INTO learning_patterns (pattern, mistake_count) VALUES (?, 1)")
                .bind(pattern)
                .execute(&self.rom_pool)
                .await?;
            sqlx::query(
                "UPDATE learning_patterns SET mistake_count = mistake_count + 1, last_updated = CURRENT_TIMESTAMP
                 WHERE pattern = ?"
            ).bind(pattern).execute(&self.rom_pool).await?;
        }
        Ok(())
    }

    pub async fn get_learning_insights(&self, pattern: &str) -> Result<Option<(i32, i32, f64)>> {
        let result = sqlx::query("SELECT mistake_count, success_count FROM learning_patterns WHERE pattern = ?")
            .bind(pattern)
            .fetch_optional(&self.rom_pool)
            .await?;

        if let Some(row) = result {
            let mistake_count: i32 = row.get(0);
            let success_count: i32 = row.get(1);
            let total = mistake_count + success_count;
            let success_rate = if total > 0 { success_count as f64 / total as f64 } else { 0.0 };
            Ok(Some((mistake_count, success_count, success_rate)))
        } else {
            Ok(None)
        }
    }

    pub async fn get_mistake_insights(&self, prompt: &str) -> Result<Vec<String>> {
        let rows = sqlx::query(
            "SELECT user_input, error_message, context FROM mistakes
             WHERE learned = FALSE ORDER BY timestamp DESC LIMIT 10"
        ).fetch_all(&self.rom_pool).await?;

        let mut insights = Vec::new();

        for row in rows {
            let user_input: String = row.get(0);
            let error_message: String = row.get(1);
            let context: Option<String> = row.get(2);

            if self.fuzzy_match(prompt, &user_input) > 0.3 {
                 let insight = if let Some(ctx) = context {
                    format!("Similar query '{}' failed with: {} (Context: {})",
                           user_input, error_message, ctx)
                } else {
                    format!("Similar query '{}' failed with: {}", user_input, error_message)
                };
                insights.push(insight);
            }
        }
        Ok(insights)
    }

    fn fuzzy_match(&self, s1: &str, s2: &str) -> f64 {
        if s1.is_empty() && s2.is_empty() { return 1.0; }
        if s1.is_empty() || s2.is_empty() { return 0.0; }

        let s1_lower = s1.to_lowercase();
        let s2_lower = s2.to_lowercase();

        let s1_words: std::collections::HashSet<&str> = s1_lower.split_whitespace().collect();
        let s2_words: std::collections::HashSet<&str> = s2_lower.split_whitespace().collect();

        let intersection = s1_words.intersection(&s2_words).count();
        let union = s1_words.union(&s2_words).count();

        if union == 0 { return 0.0; }
        intersection as f64 / union as f64
    }

    pub async fn record_query_error(&self, session_id: &str, user_input: &str, error: &anyhow::Error, context: Option<&str>) -> Result<()> {
        let error_type = if error.to_string().contains("timeout") {
            "timeout"
        } else if error.to_string().contains("API") {
            "api_error"
        } else if error.to_string().contains("model") {
            "model_error"
        } else {
            "general_error"
        };

        self.store_mistake(
            session_id,
            user_input,
            None,
            error_type,
            &error.to_string(),
            context,
        ).await?;

        let pattern = format!("{}:{}", error_type, user_input.len());
        self.update_learning_pattern(&pattern, false).await?;

        Ok(())
    }

    // Knowledge Store Delegation
    pub async fn add_to_knowledge(&self, content: &str, metadata: serde_json::Value) -> Result<()> {
        if let Some(store) = &self.knowledge_store {
            store.add_text(content, metadata).await
        } else {
            // Silently ignore or return error?
            // Since this is memory enhancement, maybe silent ignore or log is better than crashing
            warn!("Knowledge store unavailable, skipping memory add");
            Ok(())
        }
    }

    pub async fn search_knowledge(&self, query: &str, limit: usize) -> Result<Vec<(String, f64)>> {
        if let Some(store) = &self.knowledge_store {
            let results = store.search(query, limit).await?;
            Ok(results.into_iter().map(|(doc, score)| (doc.page_content, score)).collect())
        } else {
            Ok(vec![])
        }
    }

    pub async fn build_enhanced_prompt(&self, base_prompt: &str, prompt_cache: &Arc<Mutex<std::collections::HashMap<String, (String, std::time::Instant)>>>) -> Result<String> {
        let cache_key = format!("{:x}", md5::compute(base_prompt));

        {
            let cache = prompt_cache.lock().unwrap();
            if let Some((cached_prompt, timestamp)) = cache.get(&cache_key) {
                if timestamp.elapsed() < Duration::from_secs(300) {
                    return Ok(cached_prompt.clone());
                }
            }
        }

        let mut enhanced_prompt = base_prompt.to_string();

        if let Ok(Some(creator)) = self.get_air_info("creator").await {
            enhanced_prompt.push_str(&format!("\n\nSystem Info: Created by {}", creator));
        }
        enhanced_prompt.push_str("\n\nIdentity: You are 'air', an AI assistant.");

        // Hard rules for system capabilities
        enhanced_prompt.push_str("\n\nOperational Rules:");
        enhanced_prompt.push_str("\n1. If system-provided tool output (like system time) is present, use it verbatim.");
        enhanced_prompt.push_str("\n2. Do not invent shell commands. Only suggest commands if they are real and platform-specific.");
        enhanced_prompt.push_str("\n3. If you do not have tool output for a system query, state that you do not have access.");

        if let Ok(Some(version)) = self.get_air_info("version").await {
            enhanced_prompt.push_str(&format!(" (v{})", version));
        }

        if let Ok(Some(preferences)) = self.get_user_preference("response_style").await {
            enhanced_prompt.push_str(&format!("\n\nUser Preference: Response style - {}", preferences));
        }

        if let Ok(recent_convs) = self.get_recent_conversations(3).await {
            if !recent_convs.is_empty() {
                enhanced_prompt.push_str("\n\nRecent Conversation Context:");
                for (user, ai, _) in recent_convs {
                    enhanced_prompt.push_str(&format!("\nUser: {}\nAI: {}", user, ai));
                }
            }
        }

        if let Ok(insights) = self.get_mistake_insights(base_prompt).await {
            if !insights.is_empty() {
                enhanced_prompt.push_str("\n\nPast Issues to Avoid:");
                for insight in insights {
                    enhanced_prompt.push_str(&format!("\n- {}", insight));
                }
            }
        }

        // RAG Integration
        // Automatically search knowledge base for relevant info
        match self.search_knowledge(base_prompt, 2).await {
            Ok(results) => {
                if !results.is_empty() {
                    enhanced_prompt.push_str("\n\nRelevant Knowledge from Memory:");
                    for (content, score) in results {
                        if score > 0.5 { // Only show highly relevant stuff
                             enhanced_prompt.push_str(&format!("\n- {}", content));
                        }
                    }
                }
            },
            Err(e) => {
                // Ignore RAG errors silently or log them
                info!("RAG search failed: {}", e);
            }
        }

        {
            let mut cache = prompt_cache.lock().unwrap();
            cache.insert(cache_key, (enhanced_prompt.clone(), std::time::Instant::now()));
            if cache.len() > 100 {
                let keys_to_remove: Vec<String> = cache.iter()
                    .filter(|(_, (_, timestamp))| timestamp.elapsed() > Duration::from_secs(600))
                    .map(|(k, _)| k.clone())
                    .collect();
                for key in keys_to_remove {
                    cache.remove(&key);
                }
            }
        }

        Ok(enhanced_prompt)
    }
}
