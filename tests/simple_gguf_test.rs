// Simple test for GGUF model basic functionality
use ruai::providers::gguf_model::GGUFModel;
use std::path::Path;
use anyhow::Result;

#[tokio::test]
async fn test_gguf_model_basic() -> Result<()> {
    // Test with the actual model path
    let model_path = Path::new(r"C:\models\tinyllama-1.1b-chat-v1.0.Q2_K.gguf");
    
    // Test 1: Load and validate GGUF file
    let mut model = GGUFModel::load_from_path(model_path).await?;
    
    // Test 2: Initialize tokenizer
    model.initialize_tokenizer().await?;
    
    // Test 3: Test tokenization
    let test_text = "Hello world";
    let tokens = model.tokenize(test_text)?;
    assert!(!tokens.is_empty(), "Tokenization should produce tokens");
    
    // Test 4: Test detokenization
    let decoded = model.detokenize(&tokens)?;
    assert!(!decoded.is_empty(), "Detokenization should produce text");
    
    // Test 5: Model info
    let info = model.get_model_info();
    assert!(info.contains("GGUF"), "Model info should mention GGUF");
    
    println!("✅ All basic GGUF tests passed!");
    println!("Model info: {}", info);
    println!("Tokenized '{}' to {} tokens", test_text, tokens.len());
    println!("Decoded back to: '{}'", decoded);
    
    Ok(())
}

#[tokio::test]
async fn test_gguf_validation() -> Result<()> {
    let model_path = Path::new(r"C:\models\tinyllama-1.1b-chat-v1.0.Q2_K.gguf");
    
    // This should work if the file exists and is valid
    let result = GGUFModel::load_from_path(model_path).await;
    
    match result {
        Ok(_) => {
            println!("✅ GGUF file validation passed");
            Ok(())
        }
        Err(e) => {
            println!("❌ GGUF validation failed: {}", e);
            // Don't fail the test if file doesn't exist - just report it
            if e.to_string().contains("No such file") {
                println!("ℹ️  Model file not found at expected location");
                Ok(())
            } else {
                Err(e)
            }
        }
    }
}
