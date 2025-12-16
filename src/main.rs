use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::info;
use tracing_subscriber;
use std::io::{self, Write};
use dotenv;
use std::path::PathBuf;
use std::collections::HashSet;

use air::agent::AIAgent;
use air::config::Config;
use air::tools;

#[derive(Parser)]
#[command(name = "air")]
#[command(about = "AI Agent with cloud model support")]
struct Args {
    #[arg(help = "Input prompt for the AI")]
    prompt: Option<String>,
    
    #[arg(short, long, help = "Run in agent mode (interactive)")]
    interactive: bool,
    
    #[arg(short, long, help = "Verbose output")]
    verbose: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Login to cloud providers (e.g., Gemini)
    Login,
    /// Setup local environment (Ollama, models, etc.)
    Setup {
        #[arg(long, help = "Setup local models")]
        local: bool,
    },
    /// Memory and knowledge management
    Memory {
        #[command(subcommand)]
        command: MemoryCommands,
    },
    /// Configure model availability
    Config,
}

#[derive(Subcommand)]
enum MemoryCommands {
    /// Add a file to the knowledge base
    Add {
        /// Path to the file to index
        path: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from AppData .env file
    if let Ok(air_dir) = air::utils::paths::get_air_data_dir() {
        let env_path = air_dir.join(".env");
        if env_path.exists() {
            dotenv::from_path(env_path).ok();
        }
    }
    
    let args = Args::parse();
    
    // Initialize logging
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(if args.verbose { 
            tracing::Level::DEBUG 
        } else { 
            tracing::Level::INFO 
        })
        .finish();
    
    tracing::subscriber::set_global_default(subscriber)?;

    // Handle subcommands first
    match args.command {
        Some(Commands::Login) => {
            handle_login().await?;
            return Ok(());
        },
        Some(Commands::Setup { local }) => {
            if local {
                handle_local_setup().await?;
            } else {
                println!("Please specify what to setup (e.g., --local)");
            }
            return Ok(());
        },
        Some(Commands::Memory { command }) => {
            match command {
                MemoryCommands::Add { path } => {
                    let tool = tools::KnowledgeTool::new().await?;
                    match tool.add_file(&path).await {
                        Ok(msg) => println!("✅ {}", msg),
                        Err(e) => println!("❌ Failed to add file: {}", e),
                    }
                }
            }
            return Ok(());
        },
        Some(Commands::Config) => {
            handle_config_mode().await?;
            return Ok(());
        }
        None => {}
    }

    info!("Starting AIR Agent...");

    // Load configuration
    let mut config = Config::load()?;

    // Ensure model is selected if local is enabled
    if config.local_model.enabled {
        ensure_model_selected(&mut config)?;
    }
    
    // Initialize AI Agent
    let agent = AIAgent::new(config).await?;
    
    // Check if we should run in interactive mode
    if args.interactive || args.prompt.is_none() {
        run_interactive_mode(agent).await?;
    } else {
        run_single_query(agent, args).await?;
    }
    
    Ok(())
}

async fn handle_config_mode() -> Result<()> {
    use inquire::{Select, Text, validator::Validation};

    println!("\n⚙️  AIR Model Configuration");
    println!("══════════════════════════");

    // Load config directly (no need for agent)
    let mut config = match Config::load() {
        Ok(c) => c,
        Err(e) => {
            println!("❌ Failed to load configuration: {}", e);
            return Ok(());
        }
    };

    loop {
        // Build menu options dynamically
        let local_status = if config.local_model.enabled { "✅ Enabled" } else { "❌ Disabled" };
        let local_model_name = std::path::Path::new(&config.local_model.model_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();

        let mut options = vec![
            format!("Save & Exit"),
            format!("Local Model: {} [{}]", local_model_name, local_status),
            format!("Set Local Timeout (Current: {}s)", config.performance.local_timeout_seconds),
            format!("Change Local Model File"),
        ];

        // Cloud providers
        for provider in &config.cloud_providers {
            let status = if provider.enabled { "✅ Enabled" } else { "❌ Disabled" };
            options.push(format!("Cloud: {} [{}]", provider.name, status));
        }

        let selection = Select::new("Select an option:", options.clone())
            .with_page_size(10)
            .prompt();

        match selection {
            Ok(choice) => {
                // Find index of choice in original list to determine action
                let index = options.iter().position(|r| r == &choice).unwrap();

                match index {
                    0 => { // Save & Exit
                        save_config(&config)?;
                        println!("✅ Configuration saved!");
                        break;
                    }
                    1 => { // Toggle Local Model
                        config.local_model.enabled = !config.local_model.enabled;
                    }
                    2 => { // Set Timeout
                        let current = config.performance.local_timeout_seconds.to_string();
                        let validator = |input: &str| {
                            if input.parse::<u64>().is_ok() {
                                Ok(Validation::Valid)
                            } else {
                                Ok(Validation::Invalid("Please enter a valid number of seconds".into()))
                            }
                        };

                        let ans = Text::new("Enter timeout in seconds:")
                            .with_default(&current)
                            .with_validator(validator)
                            .prompt();

                        if let Ok(value) = ans {
                            if let Ok(seconds) = value.parse::<u64>() {
                                config.performance.local_timeout_seconds = seconds;
                                println!("✅ Timeout updated to {}s", seconds);
                            }
                        }
                    }
                    3 => { // Change Local Model File
                         let models = scan_for_models(&config);
                         if models.is_empty() {
                             println!("❌ No models found to select.");
                         } else {
                             prompt_model_selection(&mut config, &models)?;
                         }
                    }
                    _ => { // Toggle Cloud Provider
                        // Cloud providers start at index 4
                        let provider_idx = index - 4;
                        if let Some(provider) = config.cloud_providers.get_mut(provider_idx) {
                             provider.enabled = !provider.enabled;
                        }
                    }
                }
            }
            Err(_) => {
                println!("Operation cancelled.");
                break;
            }
        }
    }

    Ok(())
}

async fn handle_login() -> Result<()> {
    println!("\n🔑 Login Setup for Gemini (Google)");
    println!("══════════════════════════════════");
    println!("To use Gemini, you need an API key from Google AI Studio.");
    println!();
    println!("1. I will open the Google AI Studio page for you.");
    println!("2. Click 'Create API key' or copy an existing one.");
    println!("3. Come back here and paste the key.");
    println!();

    print!("👉 Press Enter to open browser...");
    io::stdout().flush()?;
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer)?;

    // Open browser
    if let Err(e) = open::that("https://aistudio.google.com/app/apikey") {
        println!("⚠️  Could not open browser automatically: {}", e);
        println!("Please verify this URL manually: https://aistudio.google.com/app/apikey");
    }

    println!();
    print!("🔑 Paste your Gemini API Key here: ");
    io::stdout().flush()?;

    let mut key = String::new();
    io::stdin().read_line(&mut key)?;
    let key = key.trim();

    if key.is_empty() {
        println!("❌ No key provided. Aborting.");
        return Ok(());
    }

    // Determine config directory
    let air_dir = air::utils::paths::get_air_data_dir()?;
    let env_path = air_dir.join(".env");
    let mut env_content = String::new();

    if env_path.exists() {
        env_content = std::fs::read_to_string(&env_path)?;
    }

    // Update or append GEMINI_API_KEY
    let mut new_lines = Vec::new();
    let mut found = false;

    for line in env_content.lines() {
        if line.starts_with("GEMINI_API_KEY=") {
            new_lines.push(format!("GEMINI_API_KEY={}", key));
            found = true;
        } else {
            new_lines.push(line.to_string());
        }
    }

    if !found {
        new_lines.push(format!("GEMINI_API_KEY={}", key));
    }

    // Write back to .env
    let mut file = std::fs::File::create(&env_path)?;
    for line in new_lines {
        writeln!(file, "{}", line)?;
    }

    println!("\n✅ Gemini API Key saved successfully to {:?}", env_path);
    println!("You can now use 'air' to chat with Gemini.");

    Ok(())
}

async fn handle_local_setup() -> Result<()> {
    println!("\n🏠 Local Model Setup (Pure Rust via Candle)");
    println!("═══════════════════════════════════════════");
    println!("This will help you set up a GGUF model for local inference.");

    // Check for models directory
    let air_dir = air::utils::paths::get_air_data_dir()?;
    let models_dir = air_dir.join("models");

    if !models_dir.exists() {
        std::fs::create_dir_all(&models_dir)?;
        println!("Created models directory: {:?}", models_dir);
    }

    let model_filename = "tinyllama-1.1b-chat-v1.0.Q2_K.gguf";
    let model_path = models_dir.join(model_filename);

    if model_path.exists() {
        println!("✅ Model already exists at: {:?}", model_path);
    } else {
        println!("⚠️  Model not found.");
        println!("Downloading TinyLlama (approx 480MB)...");

        let url = "https://huggingface.co/TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF/resolve/main/tinyllama-1.1b-chat-v1.0.Q2_K.gguf";
        let response = reqwest::get(url).await?;

        if response.status().is_success() {
            let content = response.bytes().await?;
            std::fs::write(&model_path, content)?;
            println!("✅ Successfully downloaded model to: {:?}", model_path);
        } else {
            println!("❌ Failed to download model: {}", response.status());
            return Ok(());
        }
    }

    // Download tokenizer.json
    let tokenizer_filename = "tokenizer.json";
    let tokenizer_path = models_dir.join(tokenizer_filename);

    if tokenizer_path.exists() {
        println!("✅ Tokenizer already exists at: {:?}", tokenizer_path);
    } else {
        println!("⚠️  Tokenizer not found.");
        println!("Downloading tokenizer...");

        let url = "https://huggingface.co/TinyLlama/TinyLlama-1.1B-Chat-v1.0/resolve/main/tokenizer.json";
        let response = reqwest::get(url).await?;

        if response.status().is_success() {
            let content = response.bytes().await?;
            std::fs::write(&tokenizer_path, content)?;
            println!("✅ Successfully downloaded tokenizer to: {:?}", tokenizer_path);
        } else {
            println!("❌ Failed to download tokenizer: {}", response.status());
        }
    }

    // Update configuration to point to the model
    println!("\n📝 Updating configuration...");

    let config_path = std::env::current_dir()?.join("config.toml");
    if config_path.exists() {
         match std::fs::read_to_string(&config_path) {
            Ok(content) => {
                 let mut new_config = content;

                 // Enable preference for local
                 if new_config.contains("prefer_local_for_simple_queries = false") {
                     new_config = new_config.replace("prefer_local_for_simple_queries = false", "prefer_local_for_simple_queries = true");
                 }

                 // Update model path
                 // Note: This regex-like replacement is simple; ideal would be proper TOML parsing
                 // We look for the model_path line and replace it
                 let lines: Vec<&str> = new_config.lines().collect();
                 let mut updated_lines = Vec::new();

                 let path_str = model_path.to_string_lossy().replace("\\", "\\\\");

                 for line in lines {
                     if line.trim().starts_with("model_path =") {
                         updated_lines.push(format!("model_path = \"{}\"", path_str));
                     } else {
                         updated_lines.push(line.to_string());
                     }
                 }

                 new_config = updated_lines.join("\n");

                 match std::fs::write(&config_path, new_config) {
                     Ok(_) => println!("✅ Configuration updated successfully."),
                     Err(e) => println!("❌ Failed to write config: {}", e),
                 }
            },
            Err(e) => println!("❌ Failed to read config: {}", e),
         }
    } else {
        println!("⚠️ config.toml not found. Skipping update.");
    }

    println!("\n🎉 You are ready to go! Run 'air --local-only' to force local mode.");

    Ok(())
}

async fn run_interactive_mode(agent: AIAgent) -> Result<()> {
    println!("\n🤖 AIR Agent Mode");
    println!("════════════════════════");
    println!("💡 Type your questions and I'll help you!");
    println!("📝 Special commands:");
    println!("   • 'exit' or 'quit' - Exit the program");
    println!("   • 'help' - Show available commands");
    println!("   • 'stats' - Show usage statistics");
    println!("   • 'clear' - Clear the screen");
    println!("═══════════════════════════════════════");
    
    loop {
        // Display prompt
        print!("\n💬 You: ");
        io::stdout().flush()?;
        
        // Read user input
        let mut input = String::new();
        match std::io::stdin().read_line(&mut input) {
            Ok(_) => {
                let query = input.trim().to_string();
                
                // Handle special commands
                match query.trim().to_lowercase().as_str() {
                    "exit" | "quit" | "q" => {
                        println!("\n👋 Goodbye! Thanks for using AIR!");
                        break;
                    }
                    "help" | "h" => {
                        show_help();
                        continue;
                    }
                    "stats" => {
                        show_stats().await?;
                        continue;
                    }
                    "clear" | "cls" => {
                        // Clear screen (works on both Windows and Unix)
                        print!("\x1B[2J\x1B[1;1H");
                        io::stdout().flush()?;
                        continue;
                    }
                    "" => {
                        println!("💭 Please enter a question or command. Type 'help' for assistance.");
                        continue;
                    }
                    _ => {}
                }
                
                // Process the query
                println!("\n🤖 AIR: Processing your request...");
                
                match agent.query_with_tools(&query).await {
                    Ok(response) => {
                        println!("\n🤖 AI Response:");
                        println!("{}", response);
                    }
                    Err(e) => {
                        println!("\n❌ Error: {}", e);
                        println!("💡 Try rephrasing your question or check your configuration.");
                    }
                }
            }
            Err(e) => {
                println!("\n❌ Error reading input: {}", e);
                break;
            }
        }
    }
    
    Ok(())
}

// --- Model Selection Helpers ---

fn save_config(config: &Config) -> Result<()> {
    let config_dir = air::utils::paths::get_air_data_dir()?;
    let config_path = config_dir.join("config.toml");

    let toml_string = toml::to_string_pretty(config)?;
    std::fs::write(&config_path, toml_string)?;
    Ok(())
}

fn scan_for_models(config: &Config) -> Vec<PathBuf> {
    let mut models = Vec::new();
    let mut visited = HashSet::new();
    let mut search_dirs = Vec::new();

    // 1. Configured path's parent
    if let Some(parent) = PathBuf::from(&config.local_model.model_path).parent() {
        if parent.exists() {
             search_dirs.push(parent.to_path_buf());
        }
    }

    // 2. C:\models (Windows)
    if cfg!(windows) {
        search_dirs.push(PathBuf::from(r"C:\models"));
    }

    // 3. App Data Models
    if let Ok(air_dir) = air::utils::paths::get_air_data_dir() {
        search_dirs.push(air_dir.join("models"));
    }

    // 4. Current dir models
    if let Ok(cwd) = std::env::current_dir() {
        search_dirs.push(cwd.join("models"));
    }

    for dir in search_dirs {
         if dir.exists() {
            scan_dir_recursive(&dir, &mut models, &mut visited);
         }
    }

    models
}

fn scan_dir_recursive(dir: &PathBuf, models: &mut Vec<PathBuf>, visited: &mut HashSet<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext.eq_ignore_ascii_case("gguf") {
                         // Normalize path to avoid duplicates
                         if visited.insert(path.clone()) {
                            models.push(path);
                        }
                    }
                }
            } else if path.is_dir() {
                 // Prevent infinite recursion loops with symlinks if necessary,
                 // but simple recursion is usually fine for this depth.
                 // We don't track visited dirs here, but visited files handles dupes.
                 scan_dir_recursive(&path, models, visited);
            }
        }
    }
}

fn prompt_model_selection(config: &mut Config, models: &[PathBuf]) -> Result<()> {
    use inquire::Select;

    let options: Vec<String> = models.iter()
        .map(|p| p.display().to_string())
        .collect();

    let selection = Select::new("🔍 Select a local model:", options).prompt();

    match selection {
        Ok(choice) => {
            println!("✅ Selected: {}", choice);
            config.local_model.model_path = choice;
            save_config(config)?;
        }
        Err(_) => println!("❌ Selection cancelled. Using current default."),
    }

    Ok(())
}

fn ensure_model_selected(config: &mut Config) -> Result<()> {
    let models = scan_for_models(config);

    if models.is_empty() {
        println!("⚠️  No local models (GGUF) found.");
        println!("   Please run 'air setup --local' to download a model,");
        println!("   or place your .gguf files in C:\\models or the 'models' folder in your data directory.");
        return Ok(());
    }

    let default_path = "C:\\models\\tinyllama-1.1b-chat-v1.0.Q2_K.gguf";
    let current_path = PathBuf::from(&config.local_model.model_path);

    // Check if the current configured path actually exists
    let current_exists = current_path.exists();

    // Logic:
    // 1. If currently configured model exists, we respect it (persistence).
    //    UNLESS it's the hardcoded default AND we have multiple choices (ambiguous first run).
    // 2. If currently configured model MISSING, we must select one.

    let is_default_string = config.local_model.model_path == default_path;

    if !current_exists {
        // Current config is broken/missing.
        if models.len() == 1 {
            // Auto-select the only one
            let path = models[0].to_string_lossy().to_string();
            println!("ℹ️  Auto-selecting available model: {}", path);
            config.local_model.model_path = path;
            save_config(config)?;
        } else {
            // Multiple choices, must ask
            prompt_model_selection(config, &models)?;
        }
    } else if is_default_string && models.len() > 1 {
        // It exists (so user has the default model), BUT they have others too.
        // And they haven't changed the config string (it's still default).
        // This implies they might want to choose.
        prompt_model_selection(config, &models)?;
    }

    Ok(())
}

async fn run_single_query(agent: AIAgent, args: Args) -> Result<()> {
    let prompt = args.prompt.as_ref().unwrap();
    
    // Process the request
    let response = agent.query_with_tools(prompt).await?;
    
    println!("\n🤖 AI Response:");
    println!("{}", response);
    
    Ok(())
}

fn show_help() {
    println!("\n📚 AIR Help - Available Commands:");
    println!("══════════════════════════════════");
    println!("🔹 General Commands:");
    println!("   • exit, quit, q    - Exit the program");
    println!("   • help, h          - Show this help message");
    println!("   • stats            - Show usage statistics");
    println!("   • clear, cls       - Clear the screen");
    println!();
    println!("🔹 File System Operations:");
    println!("   • read file [path]          - Read and analyze a file");
    println!("   • write file [path]         - Get help creating a file");
    println!("   • list files                - Show project structure");
    println!("   • project structure         - Analyze directory tree");
    println!();
    println!("🔹 Command Execution:");
    println!("   • run [command]             - Execute OS commands with permission");
    println!("   • execute [command]         - Run system commands safely");
    println!("   • git status                - Git commands (safe ones auto-approved)");
    println!("   • cargo build               - Rust development commands");
    println!("   • dir / ls                  - Directory listing");
    println!();
    println!("🔹 Screenshot & Media:");
    println!("   • screenshot                - Take full screen capture");
    println!("   • screenshot region         - Capture specific screen region");
    println!("   • list screenshots          - Show saved screenshots");
    println!();
    println!("🔹 Voice Commands:");
    println!("   • speak [text]              - Text-to-speech synthesis");
    println!("   • say [text]                - Generate speech from text");
    println!("   • listen                    - Speech-to-text recognition");
    println!("   • list voices               - Show available voices");
    println!();
    println!("🔹 Web Operations:");
    println!("   • fetch [url]               - Download and analyze web pages");
    println!("   • web search [query]        - Search the web for information");
    println!("   • check [url]               - Check website status");
    println!();
    println!("🔹 Development Tools:");
    println!("   • calculate [expression]    - Mathematical calculations");
    println!("   • remember [key] [value]    - Store information in memory");
    println!("   • recall [key]              - Retrieve stored information");
    println!("   • plan [goal]               - Create step-by-step plans");
    println!();
    println!("🔹 Setup:");
    println!("   • login                     - Configure API keys for cloud providers");
    println!();
    println!("💡 Tips:");
    println!("   • You can ask natural questions - AIR will detect when to use tools");
    println!("   • Commands are case-insensitive");
    println!("   • Cloud mode provides better quality but uses API calls");
    println!("═══════════════════════════════════════════════════════════════════");
}

async fn show_stats() -> Result<()> {
    println!("\n📊 AIR Usage Statistics:");
    println!("═══════════════════════");
    println!("☁️  Cloud Models: Check configuration");
    println!("⚡ Status: Ready for queries");
    println!("💡 Tip: Use 'help' to see available commands");
    
    Ok(())
}
