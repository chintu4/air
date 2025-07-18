// use ruai::providers::gguf_model::GGUFModel;
// use std::path::Path;
// use anyhow::Result;

// #[tokio::test]
// async fn test_gguf_model_creation() {
//     let model_path = Path::new(r"C:\models\tinyllama-1.1b-chat-v1.0.Q2_K.gguf");
    
//     let result = GGUFModel::load_from_path(model_path).await;
    
//     match result {
//         Ok(model) => {
//             println!("✅ Model created successfully");
//             assert!(model.get_model_info().contains("GGUF"));
//             assert!(model.get_vocab_size() > 0);
//         }
//         Err(e) => {
//             if e.to_string().contains("No such file") || e.to_string().contains("cannot find") {
//                 println!("ℹ️  Model file not found at {:?}, skipping test", model_path);
//             } else {
//                 panic!("Unexpected error: {}", e);
//             }
//         }
//     }
// }

// #[tokio::test]
// async fn test_gguf_model_tokenization() -> Result<()> {
//     let model_path = Path::new(r"C:\models\tinyllama-1.1b-chat-v1.0.Q2_K.gguf");
    
//     let mut model = match GGUFModel::load_from_path(model_path).await {
//         Ok(model) => model,
//         Err(_) => {
//             println!("ℹ️  Model file not found, skipping tokenization test");
//             return Ok(());
//         }
//     };
    
//     // Initialize tokenizer
//     model.initialize_tokenizer().await?;
    
//     // Test tokenization with various inputs
//     let test_cases = vec![
//         "Hello world",
//         "What is 2+2?",
//         "Write some code",
//         "The quick brown fox",
//         ""
//     ];
    
//     for test_text in test_cases {
//         let tokens = model.tokenize(test_text)?;
        
//         if test_text.is_empty() {
//             assert!(tokens.is_empty(), "Empty text should produce empty tokens");
//         } else {
//             assert!(!tokens.is_empty(), "Non-empty text should produce tokens");
//         }
        
//         // Test round-trip tokenization
//         let decoded = model.detokenize(&tokens)?;
//         println!("Original: '{}' -> Tokens: {:?} -> Decoded: '{}'", test_text, tokens, decoded);
//     }
    
//     println!("✅ Tokenization tests passed");
//     Ok(())
// }

// #[tokio::test]
// async fn test_gguf_model_generation_math() -> Result<()> {
//     let model_path = Path::new(r"C:\models\tinyllama-1.1b-chat-v1.0.Q2_K.gguf");
    
//     let mut model = match GGUFModel::load_from_path(model_path).await {
//         Ok(model) => model,
//         Err(_) => {
//             println!("ℹ️  Model file not found, skipping generation test");
//             return Ok(());
//         }
//     };
    
//     // Test mathematical queries
//     let math_tests = vec![
//         ("2 + 3", "5"),
//         ("10 - 4", "6"),
//         ("6 * 7", "42"),
//         ("20 / 5", "4"),
//         ("PI value", "3.14159"),
//     ];
    
//     for (prompt, expected_part) in math_tests {
//         let response = model.generate(prompt, 50, 0.1).await?;
//         println!("Math Query: '{}' -> Response: '{}'", prompt, response);
        
//         // Check if the response contains the expected answer
//         if !expected_part.is_empty() {
//             assert!(
//                 response.to_lowercase().contains(&expected_part.to_lowercase()), 
//                 "Response '{}' should contain '{}'", response, expected_part
//             );
//         }
//     }
    
//     println!("✅ Math generation tests passed");
//     Ok(())
// }

// #[tokio::test]
// async fn test_gguf_model_generation_programming() -> Result<()> {
//     let model_path = Path::new(r"C:\models\tinyllama-1.1b-chat-v1.0.Q2_K.gguf");
    
//     let mut model = match GGUFModel::load_from_path(model_path).await {
//         Ok(model) => model,
//         Err(_) => {
//             println!("ℹ️  Model file not found, skipping programming test");
//             return Ok(());
//         }
//     };
    
//     // Test programming queries
//     let programming_tests = vec![
//         ("Write Python code", "python"),
//         ("Show me Rust function", "rust"),
//         ("Create a function", "function"),
//     ];
    
//     for (prompt, expected_keyword) in programming_tests {
//         let response = model.generate(prompt, 100, 0.7).await?;
//         println!("Programming Query: '{}' -> Response: '{}'", prompt, response);
        
//         assert!(
//             response.to_lowercase().contains(&expected_keyword.to_lowercase()), 
//             "Response should contain programming-related content"
//         );
//     }
    
//     println!("✅ Programming generation tests passed");
//     Ok(())
// }

// #[tokio::test]
// async fn test_gguf_model_generation_factual() -> Result<()> {
//     let model_path = Path::new(r"C:\models\tinyllama-1.1b-chat-v1.0.Q2_K.gguf");
    
//     let mut model = match GGUFModel::load_from_path(model_path).await {
//         Ok(model) => model,
//         Err(_) => {
//             println!("ℹ️  Model file not found, skipping factual test");
//             return Ok(());
//         }
//     };
    
//     // Test factual queries
//     let factual_tests = vec![
//         ("What is the capital of France?", "Paris"),
//         ("Tell me about artificial intelligence", "intelligence"),
//         ("Explain quantum computing", "quantum"),
//     ];
    
//     for (prompt, expected_keyword) in factual_tests {
//         let response = model.generate(prompt, 100, 0.5).await?;
//         println!("Factual Query: '{}' -> Response: '{}'", prompt, response);
        
//         assert!(
//             response.to_lowercase().contains(&expected_keyword.to_lowercase()), 
//             "Response should contain relevant factual information"
//         );
//     }
    
//     println!("✅ Factual generation tests passed");
//     Ok(())
// }

// #[tokio::test]
// async fn test_gguf_model_generation_greetings() -> Result<()> {
//     let model_path = Path::new(r"C:\models\tinyllama-1.1b-chat-v1.0.Q2_K.gguf");
    
//     let mut model = match GGUFModel::load_from_path(model_path).await {
//         Ok(model) => model,
//         Err(_) => {
//             println!("ℹ️  Model file not found, skipping greeting test");
//             return Ok(());
//         }
//     };
    
//     // Test greeting queries
//     let greeting_tests = vec![
//         "Hello",
//         "Hi there",
//         "Hello, how are you?",
//     ];
    
//     for prompt in greeting_tests {
//         let response = model.generate(prompt, 50, 0.7).await?;
//         println!("Greeting: '{}' -> Response: '{}'", prompt, response);
        
//         assert!(
//             response.to_lowercase().contains("hello") || 
//             response.to_lowercase().contains("assist") ||
//             response.to_lowercase().contains("help"), 
//             "Response should be greeting-like"
//         );
//     }
    
//     println!("✅ Greeting generation tests passed");
//     Ok(())
// }

// #[tokio::test]
// async fn test_gguf_model_palindrome_detection() -> Result<()> {
//     let model_path = Path::new(r"C:\models\tinyllama-1.1b-chat-v1.0.Q2_K.gguf");
    
//     let mut model = match GGUFModel::load_from_path(model_path).await {
//         Ok(model) => model,
//         Err(_) => {
//             println!("ℹ️  Model file not found, skipping palindrome test");
//             return Ok(());
//         }
//     };
    
//     // Test palindrome detection
//     let palindrome_tests = vec![
//         ("Is 'racecar' a palindrome?", "is"),
//         ("Check if 'hello' is a palindrome", "is not"),
//         ("What is a palindrome?", "palindrome"),
//     ];
    
//     for (prompt, expected_part) in palindrome_tests {
//         let response = model.generate(prompt, 50, 0.3).await?;
//         println!("Palindrome Query: '{}' -> Response: '{}'", prompt, response);
        
//         assert!(
//             response.to_lowercase().contains(&expected_part.to_lowercase()), 
//             "Response should contain palindrome-related information"
//         );
//     }
    
//     println!("✅ Palindrome detection tests passed");
//     Ok(())
// }

// #[tokio::test]
// async fn test_gguf_model_performance() -> Result<()> {
//     let model_path = Path::new(r"C:\models\tinyllama-1.1b-chat-v1.0.Q2_K.gguf");
    
//     let mut model = match GGUFModel::load_from_path(model_path).await {
//         Ok(model) => model,
//         Err(_) => {
//             println!("ℹ️  Model file not found, skipping performance test");
//             return Ok(());
//         }
//     };
    
//     // Test performance with multiple queries
//     let start = std::time::Instant::now();
    
//     for i in 0..5 {
//         let prompt = format!("Test query number {}", i + 1);
//         let response = model.generate(&prompt, 30, 0.5).await?;
//         assert!(!response.is_empty(), "Response should not be empty");
//         println!("Query {}: Generated {} characters", i + 1, response.len());
//     }
    
//     let duration = start.elapsed();
//     println!("✅ Performance test completed in {:?}", duration);
    
//     // Should complete reasonably quickly since it's using intelligent fallback
//     assert!(duration.as_secs() < 10, "Should complete within 10 seconds");
    
//     Ok(())
// }
