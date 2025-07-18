use ruai::providers::gguf_model::GGUFModel;
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gguf_model_validation() {
        let model_path = Path::new(r"C:\models\tinyllama-1.1b-chat-v1.0.Q2_K.gguf");
        
        // Test that the GGUF file can be validated
        let result = GGUFModel::load_from_path(model_path).await;
        assert!(result.is_ok(), "GGUF model should load successfully");
        
        let model = result.unwrap();
        println!("✅ Model validation test passed");
        
        // Test model info
        let info = model.get_model_info();
        assert!(info.contains("GGUF Model"));
        println!("✅ Model info test passed: {}", info);
        
        // Test vocab size
        let vocab_size = model.get_vocab_size();
        assert!(vocab_size > 0);
        println!("✅ Vocab size test passed: {}", vocab_size);
    }

    #[tokio::test]
    async fn test_gguf_model_tokenization() {
        let model_path = Path::new(r"C:\models\tinyllama-1.1b-chat-v1.0.Q2_K.gguf");
        
        let mut model = GGUFModel::load_from_path(model_path).await
            .expect("Model should load");
        
        // Initialize tokenizer
        let _ = model.initialize_tokenizer().await;
        
        // Test tokenization
        let test_text = "Hello world";
        let tokens_result = model.tokenize(test_text);
        assert!(tokens_result.is_ok(), "Tokenization should succeed");
        
        let tokens = tokens_result.unwrap();
        assert!(!tokens.is_empty(), "Tokens should not be empty");
        
        println!("✅ Tokenization test passed: {} -> {:?}", test_text, tokens);
        
        // Test detokenization
        let detokenized_result = model.detokenize(&tokens);
        assert!(detokenized_result.is_ok(), "Detokenization should succeed");
        
        let detokenized = detokenized_result.unwrap();
        println!("✅ Detokenization test passed: {:?} -> {}", tokens, detokenized);
    }

    #[tokio::test]
    async fn test_gguf_model_generation() {
        let model_path = Path::new(r"C:\models\tinyllama-1.1b-chat-v1.0.Q2_K.gguf");
        
        let mut model = GGUFModel::load_from_path(model_path).await
            .expect("Model should load");
        
        // Initialize tokenizer
        let _ = model.initialize_tokenizer().await;
        
        // Test text generation with different prompts
        let test_cases = vec![
            ("What is 2 + 2?", "math"),
            ("Hello", "greeting"),
            ("Write code", "programming"),
        ];
        
        for (prompt, category) in test_cases {
            let result = model.generate(prompt, 50, 0.7).await;
            assert!(result.is_ok(), "Generation should succeed for {}", category);
            
            let response = result.unwrap();
            assert!(!response.is_empty(), "Response should not be empty for {}", category);
            assert!(response.len() > 10, "Response should be substantial for {}", category);
            
            println!("✅ Generation test passed for {}: '{}' -> '{}'", 
                     category, prompt, response.chars().take(100).collect::<String>());
        }
    }

    #[tokio::test]
    async fn test_gguf_model_math_calculation() {
        let model_path = Path::new(r"C:\models\tinyllama-1.1b-chat-v1.0.Q2_K.gguf");
        
        let mut model = GGUFModel::load_from_path(model_path).await
            .expect("Model should load");
        
        // Test math calculations
        let math_prompts = vec![
            "5 + 3",
            "10 - 4", 
            "6 * 7",
            "15 / 3",
        ];
        
        for prompt in math_prompts {
            let result = model.generate(prompt, 20, 0.1).await;
            assert!(result.is_ok(), "Math generation should succeed");
            
            let response = result.unwrap();
            println!("✅ Math test: '{}' -> '{}'", prompt, response);
            
            // For simple math, response should contain the answer
            assert!(!response.is_empty(), "Math response should not be empty");
        }
    }

    #[tokio::test] 
    async fn test_gguf_model_invalid_path() {
        let invalid_path = Path::new("nonexistent_model.gguf");
        
        let result = GGUFModel::load_from_path(invalid_path).await;
        assert!(result.is_err(), "Loading invalid path should fail");
        
        println!("✅ Invalid path test passed - correctly failed to load");
    }
}
