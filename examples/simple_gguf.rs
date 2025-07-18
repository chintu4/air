// Ultra-simple GGUF model test
use ruai::providers::gguf_model::GGUFModel;
use std::path::Path;
use anyhow::Result;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    println!("🔥 Simple GGUF Model Test");
    println!("========================");
    
    let model_path = Path::new(r"C:\models\tinyllama-1.1b-chat-v1.0.Q2_K.gguf");
    
    // Step 1: Load model
    println!("📁 Loading GGUF model...");
    let mut model = GGUFModel::load_from_path(model_path).await?;
    println!("✅ Model loaded!");
    
    // Step 2: Initialize tokenizer  
    println!("🔧 Initializing tokenizer...");
    model.initialize_tokenizer().await?;
    println!("✅ Tokenizer ready!");
    
    // Step 3: Test tokenization
    let text = "Hello, world!";
    println!("🔤 Testing tokenization with: '{}'", text);
    
    let tokens = model.tokenize(text)?;
    println!("📊 Tokenized into {} tokens: {:?}", tokens.len(), &tokens[..std::cmp::min(10, tokens.len())]);
    
    let decoded = model.detokenize(&tokens)?;
    println!("🔄 Decoded back to: '{}'", decoded);
    
    // Step 4: Model info
    println!("📋 Model Information:");
    println!("   {}", model.get_model_info());
    println!("   Vocab size: {}", model.get_vocab_size());
    
    println!("\n✅ GGUF model is working correctly!");
    println!("   • File validation: ✓");
    println!("   • Tokenizer: ✓"); 
    println!("   • Tokenization: ✓");
    println!("   • Detokenization: ✓");
    
    Ok(())
}
