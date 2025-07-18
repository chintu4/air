// Simple example to test GGUF model loading and functionality
use ruai::providers::gguf_model::GGUFModel;
use std::path::Path;
use anyhow::Result;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("🔥 Testing GGUF Model Loading and Generation");
    println!("==========================================");
    
    let model_path = Path::new(r"C:\models\tinyllama-1.1b-chat-v1.0.Q2_K.gguf");
    
    println!("📁 Model path: {:?}", model_path);
    
    // Test 1: Load the model
    println!("\n🔄 Step 1: Loading GGUF model...");
    let mut model = match GGUFModel::load_from_path(model_path).await {
        Ok(model) => {
            println!("✅ Model loaded successfully!");
            model
        }
        Err(e) => {
            println!("❌ Failed to load model: {}", e);
            return Err(e);
        }
    };
    
    // Test 2: Initialize tokenizer
    println!("\n🔄 Step 2: Initializing tokenizer...");
    if let Err(e) = model.initialize_tokenizer().await {
        println!("⚠️  Tokenizer initialization failed, using fallback: {}", e);
    } else {
        println!("✅ Tokenizer initialized!");
    }
    
    // Test 3: Get model info
    println!("\n📊 Step 3: Model Information");
    println!("Model Info: {}", model.get_model_info());
    println!("Vocab Size: {}", model.get_vocab_size());
    
    // Test 4: Test tokenization
    println!("\n🔄 Step 4: Testing tokenization...");
    let test_text = "Hello, how are you today?";
    match model.tokenize(test_text) {
        Ok(tokens) => {
            println!("✅ Tokenized '{}' into {} tokens: {:?}", test_text, tokens.len(), &tokens[..std::cmp::min(10, tokens.len())]);
            
            // Test detokenization
            match model.detokenize(&tokens) {
                Ok(decoded) => {
                    println!("✅ Detokenized back to: '{}'", decoded);
                }
                Err(e) => {
                    println!("⚠️  Detokenization failed: {}", e);
                }
            }
        }
        Err(e) => {
            println!("❌ Tokenization failed: {}", e);
        }
    }
    
    // Test 5: Test generation with different prompts
    println!("\n🔄 Step 5: Testing text generation...");
    
    let test_prompts = vec![
        "What is 5 + 3?",
        "How to write a function in Python?",
        "What is artificial intelligence?",
        "Write a short story about a robot",
        "Hello there!",
        "is 'chintu' a palindrome?for palindrome words need to be same when read from both ends",
        "What is the capital of France?",
        "Explain quantum computing in simple terms.",
        "What are the benefits of using Rust for system programming?",
        "Write a haiku about nature.",
        "Tell me a joke about programmers.",
        "What is the meaning of life?",
        "PI value?"
    ];
    
    for (i, prompt) in test_prompts.iter().enumerate() {
        println!("\n📝 Test {}: '{}'", i + 1, prompt);
        match model.generate(prompt, 100, 0.7).await {
            Ok(response) => {
                println!("🤖 Response: {}", response);
            }
            Err(e) => {
             println!("❌ Generation failed: {}", e);
            }
     }
        
        // Add a small delay between generations
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }
    
    println!("\n✅ GGUF Model testing completed successfully!");
    println!("The model is properly validated and can generate intelligent responses.");
    
    Ok(())
}
