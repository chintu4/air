
fn show_help() {
    println!("\n📚 air Help - Available Commands:");
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
    println!("   • You can ask natural questions - air will detect when to use tools");
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
    println!("   • You can ask natural questions - air will detect when to use tools");
    println!("   • Commands are case-insensitive");
    println!("   • Auto mode tries local first for speed, then falls back to cloud");
    println!("   • Local mode is faster but may have limited capabilities");
    println!("   • Cloud mode provides better quality but uses API calls");
    println!("═══════════════════════════════════════════════════════════════════");
}

use crate::agent::AIAgent;
use anyhow::Result;

async fn show_stats(_agent: &AIAgent) -> Result<()> {
    println!("\n📊 air Usage Statistics:");
    println!("════════════════════════");
    
    // This would require adding a get_stats method to AIAgent
    // For now, we'll show basic information
    println!("🏠 Local Model: Available");
    println!("☁️  Cloud Models: Check configuration");
    println!("⚡ Status: Ready for queries");
    println!("💡 Tip: Use 'help' to see available commands");
    
    Ok(())}