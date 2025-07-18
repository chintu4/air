// use ruai::models::{ModelProvider, QueryContext};
// use ruai::providers::LocalLlamaProvider;
// use ruai::config::LocalModelConfig;
// use std::time::Duration;
// use std::path::PathBuf;

// #[tokio::test]
// async fn test_pure_local_mode_response() {
//     let config = LocalModelConfig {
//         model_path: PathBuf::from("dummy.gguf"),
//         max_tokens: 100,
//         temperature: 0.7,
//         context_length: 2048,
//         threads: 4,
//     };
    
//     let provider = LocalLlamaProvider::new_for_testing(config);
    
//     // Test pure mode response
//     let context_pure = QueryContext {
//         prompt: "hello tell me".to_string(),
//         max_tokens: 50,
//         temperature: 0.7,
//         timeout: Duration::from_secs(5),
//         pure_mode: true,
//     };
    
//     let result_pure = provider.generate(&context_pure).await;
//     assert!(result_pure.is_ok());
    
//     let response_pure = result_pure.unwrap();
//     // Pure mode should not contain emojis or formatted templates
//     assert!(!response_pure.content.contains("🤖"));
//     assert!(!response_pure.content.contains("**"));
//     assert!(!response_pure.content.contains("════"));
    
//     // Test normal mode for comparison
//     let context_normal = QueryContext {
//         prompt: "hello tell me".to_string(),
//         max_tokens: 50,
//         temperature: 0.7,
//         timeout: Duration::from_secs(5),
//         pure_mode: false,
//     };
    
//     let result_normal = provider.generate(&context_normal).await;
//     assert!(result_normal.is_ok());
    
//     let response_normal = result_normal.unwrap();
//     // Normal mode should contain formatting
//     assert!(response_normal.content.contains("Temperature setting:") || 
//             response_normal.content.contains("Based on my analysis"));
    
//     // Responses should be different
//     assert_ne!(response_pure.content, response_normal.content);
// }

// #[tokio::test]
// async fn test_pure_mode_math_query() {
//     let config = LocalModelConfig {
//         model_path: PathBuf::from("dummy.gguf"),
//         max_tokens: 50,
//         temperature: 0.1,
//         context_length: 2048,
//         threads: 4,
//     };
    
//     let provider = LocalLlamaProvider::new_for_testing(config);
    
//     let context = QueryContext {
//         prompt: "2+2".to_string(),
//         max_tokens: 50,
//         temperature: 0.1,
//         timeout: Duration::from_secs(5),
//         pure_mode: true,
//     };
    
//     let result = provider.generate(&context).await;
//     assert!(result.is_ok());
    
//     let response = result.unwrap();
//     // Should still handle basic math even in pure mode
//     assert_eq!(response.content, "The answer is 4.");
// }

// #[tokio::test]
// async fn test_pure_mode_programming_query() {
//     let config = LocalModelConfig {
//         model_path: PathBuf::from("dummy.gguf"),
//         max_tokens: 100,
//         temperature: 0.5,
//         context_length: 2048,
//         threads: 4,
//     };
    
//     let provider = LocalLlamaProvider::new_for_testing(config);
    
//     let context = QueryContext {
//         prompt: "write some code".to_string(),
//         max_tokens: 100,
//         temperature: 0.5,
//         timeout: Duration::from_secs(5),
//         pure_mode: true,
//     };
    
//     let result = provider.generate(&context).await;
//     assert!(result.is_ok());
    
//     let response = result.unwrap();
//     // Pure mode should give a simple response without emojis or formatting
//     assert!(response.content.contains("Programming involves"));
//     assert!(!response.content.contains("💻"));
//     assert!(!response.content.contains("**"));
// }

// #[tokio::test]
// async fn test_pure_mode_what_is_query() {
//     let config = LocalModelConfig {
//         model_path: PathBuf::from("dummy.gguf"),
//         max_tokens: 100,
//         temperature: 0.5,
//         context_length: 2048,
//         threads: 4,
//     };
    
//     let provider = LocalLlamaProvider::new_for_testing(config);
    
//     let context = QueryContext {
//         prompt: "what is AI".to_string(),
//         max_tokens: 100,
//         temperature: 0.5,
//         timeout: Duration::from_secs(5),
//         pure_mode: true,
//     };
    
//     let result = provider.generate(&context).await;
//     assert!(result.is_ok());
    
//     let response = result.unwrap();
//     // Should give a simple explanation without formatting
//     assert!(response.content.contains("refers to a concept"));
//     assert!(!response.content.contains("🧠"));
//     assert!(!response.content.contains("**"));
// }

// #[test]
// fn test_pure_mode_flag_propagation() {
//     // Test that pure_mode flag is properly included in QueryContext
//     let context_pure = QueryContext {
//         prompt: "test".to_string(),
//         max_tokens: 50,
//         temperature: 0.7,
//         timeout: Duration::from_secs(5),
//         pure_mode: true,
//     };
    
//     assert!(context_pure.pure_mode);
    
//     let context_normal = QueryContext {
//         prompt: "test".to_string(),
//         max_tokens: 50,
//         temperature: 0.7,
//         timeout: Duration::from_secs(5),
//         pure_mode: false,
//     };
    
//     assert!(!context_normal.pure_mode);
// }
