use anyhow::Result;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::info;
use md5;

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
    ram_memory: Arc<Mutex<Connection>>,  // Session memory - clears each session
    rom_memory: Arc<Mutex<Connection>>,  // Persistent memory - survives sessions
    about_memory: Arc<Mutex<Connection>>, // Static info about air
}

impl MemoryManager {
    pub fn new(app_data: &str) -> Result<Self> {
        let ram_db = std::path::Path::new(app_data).join("air").join("ram_memory.db");
        let rom_db = std::path::Path::new(app_data).join("air").join("rom_memory.db");
        let about_db = std::path::Path::new(app_data).join("air").join("about_memory.db");

        // Initialize RAM memory (session-based, recreate tables each time)
        let ram_conn = Connection::open(&ram_db)?;
        ram_conn.execute("DROP TABLE IF EXISTS conversations", [])?;
        ram_conn.execute("DROP TABLE IF EXISTS memory", [])?;
        ram_conn.execute(
            "CREATE TABLE conversations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_input TEXT NOT NULL,
                ai_response TEXT NOT NULL,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                context TEXT,
                tools_used TEXT
            )",
            [],
        )?;
        ram_conn.execute(
            "CREATE TABLE memory (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        // Initialize ROM memory (persistent)
        let rom_conn = Connection::open(&rom_db)?;
        rom_conn.execute(
            "CREATE TABLE IF NOT EXISTS persistent_memory (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;
        rom_conn.execute(
            "CREATE TABLE IF NOT EXISTS user_preferences (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;
        rom_conn.execute(
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
            )",
            [],
        )?;
        rom_conn.execute(
            "CREATE TABLE IF NOT EXISTS learning_patterns (
                pattern TEXT PRIMARY KEY,
                mistake_count INTEGER DEFAULT 0,
                success_count INTEGER DEFAULT 0,
                last_updated DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        // Initialize ABOUT memory (static info)
        let about_conn = Connection::open(&about_db)?;
        about_conn.execute(
            "CREATE TABLE IF NOT EXISTS air_info (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )?;

        // Insert default air information if not exists
        about_conn.execute(
            "INSERT OR IGNORE INTO air_info (key, value) VALUES (?, ?)",
            ["creator", "Chintu (dsjapnc)"],
        )?;
        about_conn.execute(
            "INSERT OR IGNORE INTO air_info (key, value) VALUES (?, ?)",
            ["version", "0.1.0"],
        )?;
        about_conn.execute(
            "INSERT OR IGNORE INTO air_info (key, value) VALUES (?, ?)",
            ["description", "I am air, an AI Agent with local and cloud model fallback"],
        )?;
        about_conn.execute(
            "INSERT OR IGNORE INTO air_info (key, value) VALUES (?, ?)",
            ["repository", "https://github.com/chintu4/air"],
        )?;

        Ok(Self {
            ram_memory: Arc::new(Mutex::new(ram_conn)),
            rom_memory: Arc::new(Mutex::new(rom_conn)),
            about_memory: Arc::new(Mutex::new(about_conn)),
        })
    }

    /// Store multiple conversations in batch for better performance
    pub fn store_conversations_batch(&self, conversations: Vec<(String, String, Option<String>, Option<String>)>) -> Result<()> {
        if conversations.is_empty() {
            return Ok(());
        }

        let conn = self.ram_memory.lock().unwrap();
        let tx = conn.unchecked_transaction()?;

        for (user_input, ai_response, context, tools_used) in conversations {
            // Compress long conversations
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

            tx.execute(
                "INSERT INTO conversations (user_input, ai_response, context, tools_used) VALUES (?, ?, ?, ?)",
                [&compressed_input, &compressed_response, &context.unwrap_or_default(), &tools_used.unwrap_or_default()],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// Store key-value pair in RAM memory
    pub fn store_ram_memory(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.ram_memory.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO memory (key, value) VALUES (?, ?)",
            [key, value],
        )?;
        Ok(())
    }

    /// Retrieve value from RAM memory
    pub fn get_ram_memory(&self, key: &str) -> Result<Option<String>> {
        let conn = self.ram_memory.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM memory WHERE key = ?")?;
        let mut rows = stmt.query([key])?;

        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    /// Store persistent memory in ROM
    pub fn store_persistent_memory(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.rom_memory.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO persistent_memory (key, value) VALUES (?, ?)",
            [key, value],
        )?;
        Ok(())
    }

    /// Retrieve from persistent memory
    pub fn get_persistent_memory(&self, key: &str) -> Result<Option<String>> {
        let conn = self.rom_memory.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM persistent_memory WHERE key = ?")?;
        let mut rows = stmt.query([key])?;

        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    /// Store user preference
    pub fn store_user_preference(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.rom_memory.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO user_preferences (key, value) VALUES (?, ?)",
            [key, value],
        )?;
        Ok(())
    }

    /// Get user preference
    pub fn get_user_preference(&self, key: &str) -> Result<Option<String>> {
        let conn = self.rom_memory.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM user_preferences WHERE key = ?")?;
        let mut rows = stmt.query([key])?;

        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    /// Get air information
    pub fn get_air_info(&self, key: &str) -> Result<Option<String>> {
        let conn = self.about_memory.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM air_info WHERE key = ?")?;
        let mut rows = stmt.query([key])?;

        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    /// Get recent conversations from RAM with automatic cleanup
    pub fn get_recent_conversations(&self, limit: usize) -> Result<Vec<(String, String, String)>> {
        let conn = self.ram_memory.lock().unwrap();

        // First, check total conversation count and cleanup if needed
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM conversations", [], |row| row.get(0))?;
        if count > 1000 { // Limit to 1000 conversations max
            info!("🧹 Cleaning up old conversations ({} > 1000)", count);
            conn.execute("DELETE FROM conversations WHERE id IN (SELECT id FROM conversations ORDER BY timestamp DESC LIMIT -1 OFFSET 500)", [])?;
        }

        let mut stmt = conn.prepare(
            "SELECT user_input, ai_response, timestamp FROM conversations ORDER BY timestamp DESC LIMIT ?"
        )?;
        let mut rows = stmt.query([limit])?;

        let mut conversations = Vec::new();
        while let Some(row) = rows.next()? {
            conversations.push((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
            ));
        }
        conversations.reverse(); // Return in chronological order
        Ok(conversations)
    }

    /// Periodic maintenance task for database optimization
    pub fn perform_maintenance(&self) -> Result<()> {
        info!("🔧 Performing database maintenance...");

        // RAM memory maintenance
        {
            let conn = self.ram_memory.lock().unwrap();
            // Vacuum to reclaim space
            conn.execute("VACUUM", [])?;
            // Remove very old entries (keep last 24 hours)
            conn.execute("DELETE FROM conversations WHERE timestamp < datetime('now', '-1 day')", [])?;
            conn.execute("DELETE FROM memory WHERE timestamp < datetime('now', '-1 day')", [])?;
        }

        // ROM memory maintenance
        {
            let conn = self.rom_memory.lock().unwrap();
            conn.execute("VACUUM", [])?;
            // Remove very old mistakes (keep last 30 days)
            conn.execute("DELETE FROM mistakes WHERE timestamp < datetime('now', '-30 days')", [])?;
        }

        // ABOUT memory maintenance
        {
            let conn = self.about_memory.lock().unwrap();
            conn.execute("VACUUM", [])?;
        }

        info!("✅ Database maintenance completed");
        Ok(())
    }

    /// Store a mistake in ROM memory for learning
    pub fn store_mistake(&self, session_id: &str, user_input: &str, ai_response: Option<&str>,
                        error_type: &str, error_message: &str, context: Option<&str>) -> Result<i64> {
        let conn = self.rom_memory.lock().unwrap();
        conn.execute(
            "INSERT INTO mistakes (session_id, user_input, ai_response, error_type, error_message, context)
             VALUES (?, ?, ?, ?, ?, ?)",
            [session_id, user_input, &ai_response.unwrap_or(""), error_type, error_message, &context.unwrap_or("")],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Mark a mistake as learned
    pub fn mark_mistake_learned(&self, mistake_id: i64) -> Result<()> {
        let conn = self.rom_memory.lock().unwrap();
        conn.execute(
            "UPDATE mistakes SET learned = TRUE WHERE id = ?",
            [mistake_id],
        )?;
        Ok(())
    }

    /// Get unlearned mistakes for a specific error type
    pub fn get_unlearned_mistakes(&self, error_type: Option<&str>, limit: usize) -> Result<Vec<(i64, String, String, String, String)>> {
        let conn = self.rom_memory.lock().unwrap();
        let query = if let Some(_err_type) = error_type {
            "SELECT id, user_input, error_type, error_message, context FROM mistakes
             WHERE learned = FALSE AND error_type = ? ORDER BY timestamp DESC LIMIT ?"
        } else {
            "SELECT id, user_input, error_type, error_message, context FROM mistakes
             WHERE learned = FALSE ORDER BY timestamp DESC LIMIT ?"
        };

        let mut stmt = conn.prepare(query)?;
        let mut rows = if let Some(err_type) = error_type {
            stmt.query([err_type, &limit.to_string()])?
        } else {
            stmt.query([limit])?
        };

        let mut mistakes = Vec::new();
        while let Some(row) = rows.next()? {
            mistakes.push((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ));
        }
        Ok(mistakes)
    }

    /// Store learning pattern (success/failure tracking)
    pub fn update_learning_pattern(&self, pattern: &str, was_success: bool) -> Result<()> {
        let conn = self.rom_memory.lock().unwrap();

        if was_success {
            conn.execute(
                "INSERT OR IGNORE INTO learning_patterns (pattern, success_count) VALUES (?, 1)",
                [pattern],
            )?;
            conn.execute(
                "UPDATE learning_patterns SET success_count = success_count + 1, last_updated = CURRENT_TIMESTAMP
                 WHERE pattern = ?",
                [pattern],
            )?;
        } else {
            conn.execute(
                "INSERT OR IGNORE INTO learning_patterns (pattern, mistake_count) VALUES (?, 1)",
                [pattern],
            )?;
            conn.execute(
                "UPDATE learning_patterns SET mistake_count = mistake_count + 1, last_updated = CURRENT_TIMESTAMP
                 WHERE pattern = ?",
                [pattern],
            )?;
        }
        Ok(())
    }

    /// Get learning insights for a pattern
    pub fn get_learning_insights(&self, pattern: &str) -> Result<Option<(i32, i32, f64)>> {
        let conn = self.rom_memory.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT mistake_count, success_count FROM learning_patterns WHERE pattern = ?"
        )?;
        let mut rows = stmt.query([pattern])?;

        if let Some(row) = rows.next()? {
            let mistake_count: i32 = row.get(0)?;
            let success_count: i32 = row.get(1)?;
            let total = mistake_count + success_count;
            let success_rate = if total > 0 { success_count as f64 / total as f64 } else { 0.0 };
            Ok(Some((mistake_count, success_count, success_rate)))
        } else {
            Ok(None)
        }
    }

    /// Get mistake insights with fuzzy matching for better similarity detection
    pub fn get_mistake_insights(&self, prompt: &str) -> Result<Vec<String>> {
        let conn = self.rom_memory.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT user_input, error_message, context FROM mistakes
             WHERE learned = FALSE ORDER BY timestamp DESC LIMIT 10"
        )?;

        let mut rows = stmt.query([])?;
        let mut insights = Vec::new();

        while let Some(row) = rows.next()? {
            let user_input: String = row.get(0)?;
            let error_message: String = row.get(1)?;
            let context: Option<String> = row.get(2)?;

            // Use fuzzy matching for better similarity detection
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

    /// Fuzzy string matching using Levenshtein distance approximation
    fn fuzzy_match(&self, s1: &str, s2: &str) -> f64 {
        if s1.is_empty() && s2.is_empty() {
            return 1.0;
        }
        if s1.is_empty() || s2.is_empty() {
            return 0.0;
        }

        let s1_lower = s1.to_lowercase();
        let s2_lower = s2.to_lowercase();

        // Simple word-based similarity
        let s1_words: std::collections::HashSet<&str> = s1_lower.split_whitespace().collect();
        let s2_words: std::collections::HashSet<&str> = s2_lower.split_whitespace().collect();

        let intersection = s1_words.intersection(&s2_words).count();
        let union = s1_words.union(&s2_words).count();

        if union == 0 {
            return 0.0;
        }

        intersection as f64 / union as f64
    }

    /// Helper method to store errors that occur during query processing
    pub fn record_query_error(&self, session_id: &str, user_input: &str, error: &anyhow::Error, context: Option<&str>) -> Result<()> {
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
            None, // No AI response since it failed
            error_type,
            &error.to_string(),
            context,
        )?;

        // Update learning pattern
        let pattern = format!("{}:{}", error_type, user_input.len());
        self.update_learning_pattern(&pattern, false)?;

        Ok(())
    }

    /// Build an enhanced prompt with relevant context from memories (with caching)
    pub fn build_enhanced_prompt(&self, base_prompt: &str, prompt_cache: &Arc<Mutex<std::collections::HashMap<String, (String, std::time::Instant)>>>) -> Result<String> {
        let cache_key = format!("{:x}", md5::compute(base_prompt));

        // Check cache first
        {
            let cache = prompt_cache.lock().unwrap();
            if let Some((cached_prompt, timestamp)) = cache.get(&cache_key) {
                // Cache valid for 5 minutes
                if timestamp.elapsed() < Duration::from_secs(300) {
                    return Ok(cached_prompt.clone());
                }
            }
        }

        // Build enhanced prompt
        let mut enhanced_prompt = base_prompt.to_string();

        // Add air info from about_db
        if let Ok(Some(creator)) = self.get_air_info("creator") {
            enhanced_prompt.push_str(&format!("\n\nSystem Info: Created by {}", creator));
        }
        // Always identify as air
        enhanced_prompt.push_str("\n\nIdentity: You are 'air', an advanced AI agent.");
        
        // Inject Capabilities
        enhanced_prompt.push_str("\n\nCapabilities:\n");
        enhanced_prompt.push_str("- Filesystem: Read, write, list, and analyze files and directories\n");
        enhanced_prompt.push_str("- Web: Search the internet and extract content from URLs\n");
        enhanced_prompt.push_str("- Command: Execute system commands (safe and unsafe with approval)\n");
        enhanced_prompt.push_str("- Memory: Store and recall information, manage long-term memory\n");
        enhanced_prompt.push_str("- Planning: Create and execute step-by-step plans\n");
        enhanced_prompt.push_str("- Calculator: Perform mathematical calculations\n");
        enhanced_prompt.push_str("- Screenshot: Capture screen content for analysis\n");
        enhanced_prompt.push_str("- Voice: Speak text and listen to voice input\n");
        enhanced_prompt.push_str("- Knowledge: RAG system for indexing and querying documents\n");

        // Inject Operational Procedures
        enhanced_prompt.push_str("\nOperational Procedures:\n");
        enhanced_prompt.push_str("1. VERIFICATION LOOP: After making changes, ALWAYS verify the fix. If verification fails, analyze, fix again, and re-verify.\n");
        enhanced_prompt.push_str("2. PLANNING: Before complex tasks, create a step-by-step plan.\n");
        enhanced_prompt.push_str("3. ANTI-HALLUCINATION: Do not invent information. If unsure, check knowledge base or search web.\n");
        enhanced_prompt.push_str("4. RECALL: Use memory tools to recall context if needed.\n");

        // Inject Personality
        enhanced_prompt.push_str("\nPersonality:\n");
        enhanced_prompt.push_str("- Traits: Helpful, precise, proactive, transparent\n");
        enhanced_prompt.push_str("- Tone: Professional yet conversational\n");
        enhanced_prompt.push_str("- Style: Clear, concise, and structured. Be brief and avoid unnecessary verbosity. Only provide detailed explanations when explicitly asked or when critical for understanding.\n");

        if let Ok(Some(version)) = self.get_air_info("version") {
            enhanced_prompt.push_str(&format!(" (v{})", version));
        }

        // Add user preferences from rom_db
        if let Ok(Some(preferences)) = self.get_user_preference("response_style") {
            enhanced_prompt.push_str(&format!("\n\nUser Preference: Response style - {}", preferences));
        }

        // Add recent conversation context from ram_db (last 3 conversations)
        if let Ok(recent_convs) = self.get_recent_conversations(3) {
            if !recent_convs.is_empty() {
                enhanced_prompt.push_str("\n\nRecent Conversation Context:");
                for (user, ai, _) in recent_convs {
                    enhanced_prompt.push_str(&format!("\nUser: {}\nAI: {}", user, ai));
                }
            }
        }

        // Add mistake insights from rom_db if relevant
        if let Ok(insights) = self.get_mistake_insights(base_prompt) {
            if !insights.is_empty() {
                enhanced_prompt.push_str("\n\nPast Issues to Avoid:");
                for insight in insights {
                    enhanced_prompt.push_str(&format!("\n- {}", insight));
                }
            }
        }

        // Add learning patterns from rom_db
        let pattern = format!("query_length:{}", base_prompt.len());
        if let Ok(Some((_mistakes, _successes, rate))) = self.get_learning_insights(&pattern) {
            if rate < 0.7 { // If success rate is low, add caution
                enhanced_prompt.push_str(&format!("\n\nCaution: This type of query has a {:.1}% success rate based on past interactions.", rate * 100.0));
            }
        }

        // Cache the result
        {
            let mut cache = prompt_cache.lock().unwrap();
            cache.insert(cache_key, (enhanced_prompt.clone(), std::time::Instant::now()));

            // Limit cache size to 100 entries
            if cache.len() > 100 {
                // Remove oldest entries (simple LRU approximation)
                let keys_to_remove: Vec<String> = cache.iter()
                    .filter(|(_, (_, timestamp))| timestamp.elapsed() > Duration::from_secs(600)) // 10 minutes
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
