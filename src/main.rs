use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::info;
use tracing_subscriber;
use std::io::{self, Write};

use air::agent::AIAgent;
use air::config::Config;
use air::tools;

#[derive(Parser)]
#[command(name = "air")]
#[command(about = "AI Agent with cloud model support")]
struct Args {
    #[arg(help = "Input prompt for the AI")]
    prompt: Option<String>,
    
    #[arg(short, long, help = "Run in interactive mode")]
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
    // Load environment variables from .env file
    dotenv::dotenv().ok();
    
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
                    let tool = tools::KnowledgeTool::new()?;
                    match tool.add_file(&path) {
                        Ok(msg) => println!("✅ {}", msg),
                        Err(e) => println!("❌ Failed to add file: {}", e),
                    }
                }
            }
            return Ok(());
        },
        None => {}
    }

    info!("Starting AIR Agent...");

    // Load configuration
    let config = Config::load()?;
    
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

    // Read existing .env or create new
    let env_path = std::env::current_dir()?.join(".env");
    let mut env_content = String::new();

    if env_path.exists() {
        env_content = std::fs::read_to_string(&env_path)?;
    }

    // Update or append GEMINI_KEY
    let mut new_lines = Vec::new();
    let mut found = false;

    for line in env_content.lines() {
        if line.starts_with("GEMINI_KEY=") {
            new_lines.push(format!("GEMINI_KEY={}", key));
            found = true;
        } else {
            new_lines.push(line.to_string());
        }
    }

    if !found {
        new_lines.push(format!("GEMINI_KEY={}", key));
    }

    // Write back to .env
    let mut file = std::fs::File::create(&env_path)?;
    for line in new_lines {
        writeln!(file, "{}", line)?;
    }

    println!("\n✅ Gemini API Key saved successfully to .env!");
    println!("You can now use 'air' to chat with Gemini.");

    Ok(())
}

async fn handle_local_setup() -> Result<()> {
    println!("\n🏠 Local Model Setup (Ollama)");
    println!("═════════════════════════════");
    println!("This will help you set up Ollama for private, local AI.");

    // Check if Ollama is installed
    println!("\n🔍 Checking for Ollama...");

    let output = std::process::Command::new("ollama")
        .arg("--version")
        .output();

    match output {
        Ok(out) => {
            if out.status.success() {
                let version = String::from_utf8_lossy(&out.stdout).trim().to_string();
                println!("✅ Ollama is installed: {}", version);

                // Check for models
                println!("\n🔍 Checking for models...");
                let list_output = std::process::Command::new("ollama")
                    .arg("list")
                    .output()?;

                let list = String::from_utf8_lossy(&list_output.stdout);
                if list.contains("llama3") || list.contains("mistral") {
                     println!("✅ Found existing models!");
                     println!("{}", list);
                } else {
                    println!("⚠️  No standard models found (looked for llama3/mistral).");
                    println!("Downloading llama3 (8B) - this might take a while...");

                    let status = std::process::Command::new("ollama")
                        .args(&["pull", "llama3"])
                        .status()?;

                    if status.success() {
                        println!("✅ Successfully pulled llama3!");
                    } else {
                        println!("❌ Failed to pull llama3.");
                    }
                }

                // Update configuration to prefer local
                println!("\n📝 Updating configuration to use local provider...");

                let config_path = std::env::current_dir()?.join("config.toml");
                if config_path.exists() {
                     match std::fs::read_to_string(&config_path) {
                        Ok(content) => {
                             // Simple TOML modification using string replacement or just appending
                             // A proper robust solution would use toml_edit, but for this task we want to enable local preference

                             let mut new_config = content;
                             // Ensure prefer_local_for_simple_queries is true
                             if new_config.contains("prefer_local_for_simple_queries = false") {
                                 new_config = new_config.replace("prefer_local_for_simple_queries = false", "prefer_local_for_simple_queries = true");
                             }

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

            } else {
                 println!("❌ Ollama found but returned error.");
            }
        }
        Err(_) => {
            println!("❌ Ollama is NOT installed or not in PATH.");
            println!("\nPlease install Ollama from: https://ollama.com");
            println!("After installing, run 'air setup --local' again.");

            if cfg!(target_os = "windows") {
                println!("\n💡 Tip: On Windows, you can download the installer directly.");
                // We could attempt to download/install here, but better to let user handle it for now
                if let Err(e) = open::that("https://ollama.com/download/windows") {
                     println!("Could not open browser: {}", e);
                }
            } else if cfg!(target_os = "macos") {
                 if let Err(e) = open::that("https://ollama.com/download/mac") {
                     println!("Could not open browser: {}", e);
                }
            } else {
                println!("Run: curl -fsSL https://ollama.com/install.sh | sh");
            }
        }
    }

    Ok(())
}

async fn run_interactive_mode(agent: AIAgent) -> Result<()> {
    println!("\n🤖 AIR Interactive Mode");
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
