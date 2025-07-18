use anyhow::Result;
use clap::Parser;
use tracing::info;
use tracing_subscriber;
use std::io::{self, Write};

mod agent;
mod models;
mod providers;
mod config;
mod tools;

use agent::AIAgent;
use config::Config;

#[derive(Parser)]
#[command(name = "ruai")]
#[command(about = "AI Agent with local and cloud model fallback")]
struct Args {
    #[arg(short, long, help = "Input prompt for the AI")]
    prompt: Option<String>,
    
    #[arg(short, long, help = "Run in interactive mode")]
    interactive: bool,
    
    #[arg(short, long, help = "Force cloud model usage")]
    cloud_only: bool,
    
    #[arg(short, long, help = "Force local model usage")]
    local_only: bool,
    
    #[arg(long, help = "Pure local model response without templates")]
    local: bool,
    
    #[arg(short, long, help = "Verbose output")]
    verbose: bool,
}

#[derive(Debug, Clone)]
enum QueryMode {
    Auto,       // Smart fallback (default)
    LocalOnly,  // Force local model
    CloudOnly,  // Force cloud model
    PureLocal,  // Pure local model without templates
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

    info!("Starting RUAI Agent...");

    // Load configuration
    let config = Config::load()?;
    
    // Initialize AI Agent
    let agent = AIAgent::new(config).await?;
    
    // Check if we should run in interactive mode
    if args.interactive || args.prompt.is_none() {
        run_interactive_mode(agent, args).await?;
    } else {
        run_single_query(agent, args).await?;
    }
    
    Ok(())
}

async fn run_interactive_mode(agent: AIAgent, args: Args) -> Result<()> {
    // Initialize the query mode based on command line args
    let mut query_mode = if args.cloud_only {
        QueryMode::CloudOnly
    } else if args.local_only {
        QueryMode::LocalOnly
    } else if args.local {
        QueryMode::PureLocal
    } else {
        QueryMode::Auto
    };

    println!("\n🤖 RUAI Interactive Mode");
    println!("════════════════════════");
    println!("💡 Type your questions and I'll help you!");
    println!("🔄 Current mode: {}", format_mode(&query_mode));
    println!("📝 Special commands:");
    println!("   • 'exit' or 'quit' - Exit the program");
    println!("   • 'help' - Show available commands");
    println!("   • 'stats' - Show usage statistics");
    println!("   • 'clear' - Clear the screen");
    println!("   • 'mode auto' - Smart fallback mode (default)");
    println!("   • 'mode local' - Force local model only");
    println!("   • 'mode cloud' - Force cloud model only");
    println!("   • 'mode pure' - Pure local model (no templates)");
    println!("   • 'mode status' - Show current mode");
    println!("═══════════════════════════════════════");
    
    loop {
        // Display prompt
        print!("\n💬 You: ");
        io::stdout().flush()?;
        
        // Read user input
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                let query = input.trim().to_string();
                
                                // Handle special commands
                match query.trim().to_lowercase().as_str() {
                    "exit" | "quit" | "q" => {
                        println!("
👋 Goodbye! Thanks for using RUAI!");
                        break;
                    }
                    "help" | "h" => {
                        show_help();
                        continue;
                    }
                    "stats" => {
                        show_stats(&agent).await?;
                        continue;
                    }
                    "clear" | "cls" => {
                        // Clear screen (works on both Windows and Unix)
                        print!("\x1B[2J\x1B[1;1H");
                        io::stdout().flush()?;
                        continue;
                    }
                    "mode status" => {
                        println!("
🔄 Current query mode: {}", format_mode(&query_mode));
                        continue;
                    }
                    "mode auto" => {
                        query_mode = QueryMode::Auto;
                        println!("
✅ Switched to Auto mode (smart fallback: local first, then cloud)");
                        continue;
                    }
                    "mode local" => {
                        query_mode = QueryMode::LocalOnly;
                        println!("
🏠 Switched to Local-only mode");
                        continue;
                    }
                    "mode cloud" => {
                        query_mode = QueryMode::CloudOnly;
                        println!("
☁️  Switched to Cloud-only mode");
                        continue;
                    }
                    "mode pure" | "mode pure-local" => {
                        query_mode = QueryMode::PureLocal;
                        println!("
🔓 Switched to Pure Local mode (no templates or formatting)");
                        continue;
                    }
                    "" => {
                        println!("💭 Please enter a question or command. Type 'help' for assistance.");
                        continue;
                    }
                    _ => {}
                }
                
                // Process the query
                println!("\n🤖 RUAI: Processing your request... (Mode: {})", format_mode(&query_mode));
                
                match process_query_with_mode(&agent, &query, &query_mode).await {
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
    let response = process_query(&agent, prompt, &args).await?;
    
    println!("\n🤖 AI Response:");
    println!("{}", response);
    
    Ok(())
}

async fn process_query(agent: &AIAgent, prompt: &str, args: &Args) -> Result<String> {
    let response = if args.cloud_only {
        agent.query_cloud_only(prompt).await?
    } else if args.local_only {
        agent.query_local_only(prompt).await?
    } else if args.local {
        agent.query_pure_local(prompt).await?
    } else {
        // Use the enhanced query with tools
        agent.query_with_tools(prompt).await?
    };
    
    // Format the response nicely
    Ok(format!("{}", response))
}

async fn process_query_with_mode(agent: &AIAgent, prompt: &str, mode: &QueryMode) -> Result<String> {
    let response = match mode {
        QueryMode::CloudOnly => agent.query_cloud_only(prompt).await?,
        QueryMode::LocalOnly => agent.query_local_only(prompt).await?,
        QueryMode::PureLocal => agent.query_pure_local(prompt).await?,
        QueryMode::Auto => agent.query_with_tools(prompt).await?,
    };
    
    // Format the response nicely
    Ok(format!("{}", response))
}

fn format_mode(mode: &QueryMode) -> String {
    match mode {
        QueryMode::Auto => "🔄 Auto (Smart Fallback)".to_string(),
        QueryMode::LocalOnly => "🏠 Local Only".to_string(),
        QueryMode::CloudOnly => "☁️  Cloud Only".to_string(),
        QueryMode::PureLocal => "🔓 Pure Local (No Templates)".to_string(),
    }
}

fn show_help() {
    println!("\n📚 RUAI Help - Available Commands:");
    println!("═══════════════════════════════════");
    println!("🔹 General Commands:");
    println!("   • exit, quit, q    - Exit the program");
    println!("   • help, h          - Show this help message");
    println!("   • stats            - Show usage statistics");
    println!("   • clear, cls       - Clear the screen");
    println!();
    println!("🔹 Mode Control:");
    println!("   • mode auto        - Smart fallback mode (local first, then cloud)");
    println!("   • mode local       - Force local model only");
    println!("   • mode cloud       - Force cloud model only");
    println!("   • mode pure        - Pure local model (no templates or formatting)");
    println!("   • mode status      - Show current processing mode");
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
    println!("💡 Tips:");
    println!("   • You can ask natural questions - RUAI will detect when to use tools");
    println!("   • Commands are case-insensitive");
    println!("   • Auto mode tries local first for speed, then falls back to cloud");
    println!("   • Local mode is faster but may have limited capabilities");
    println!("   • Cloud mode provides better quality but uses API calls");
    println!("═══════════════════════════════════════════════════════════════════");

    println!("   • list voices               - Show available voices");
    println!();
    println!("🔹 Query Examples:");
    println!("   • Math: '2+2', 'calculate 15*7'");
    println!("   • Programming: 'write a Python function', 'explain this code'");
    println!("   • Questions: 'explain AI', 'how does machine learning work'");
    println!("   • Creative: 'write a story', 'create a poem'");
    println!("   • Files: 'read file src/main.rs', 'analyze file config.toml'");
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
    println!("� Tips:");
    println!("   • You can ask natural questions - RUAI will detect when to use tools");
    println!("   • Commands are case-insensitive");
    println!("   • Auto mode tries local first for speed, then falls back to cloud");
    println!("   • Local mode is faster but may have limited capabilities");
    println!("   • Cloud mode provides better quality but uses API calls");
    println!("═══════════════════════════════════════════════════════════════════");
}

async fn show_stats(_agent: &AIAgent) -> Result<()> {
    println!("\n📊 RUAI Usage Statistics:");
    println!("════════════════════════");
    
    // This would require adding a get_stats method to AIAgent
    // For now, we'll show basic information
    println!("🏠 Local Model: Available");
    println!("☁️  Cloud Models: Check configuration");
    println!("⚡ Status: Ready for queries");
    println!("💡 Tip: Use 'help' to see available commands");
    
    Ok(())
}
