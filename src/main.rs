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
#[command(about = "AI Agent with cloud model support")]
struct Args {
    #[arg(short, long, help = "Input prompt for the AI")]
    prompt: Option<String>,
    
    #[arg(short, long, help = "Run in interactive mode")]
    interactive: bool,
    
    #[arg(short, long, help = "Verbose output")]
    verbose: bool,
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
        run_interactive_mode(agent).await?;
    } else {
        run_single_query(agent, args).await?;
    }
    
    Ok(())
}

async fn run_interactive_mode(agent: AIAgent) -> Result<()> {
    println!("\n🤖 RUAI Interactive Mode");
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
                        println!("\n👋 Goodbye! Thanks for using RUAI!");
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
                    "" => {
                        println!("💭 Please enter a question or command. Type 'help' for assistance.");
                        continue;
                    }
                    _ => {}
                }
                
                // Process the query
                println!("\n🤖 RUAI: Processing your request...");
                
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
    println!("\n📚 RUAI Help - Available Commands:");
    println!("═══════════════════════════════════");
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
    println!("💡 Tips:");
    println!("   • You can ask natural questions - RUAI will detect when to use tools");
    println!("   • Commands are case-insensitive");
    println!("   • Cloud mode provides better quality but uses API calls");
    println!("═══════════════════════════════════════════════════════════════════");
}

async fn show_stats(agent: &AIAgent) -> Result<()> {
    let (successful_queries, failed_queries) = agent.get_stats().await;
    println!("\n📊 RUAI Usage Statistics:");
    println!("════════════════════════");
    println!("✅ Successful Queries: {}", successful_queries);
    println!("❌ Failed Queries: {}", failed_queries);
    println!("☁️  Cloud Models: Check configuration");
    println!("⚡ Status: Ready for queries");
    println!("💡 Tip: Use 'help' to see available commands");
    
    Ok(())
}
