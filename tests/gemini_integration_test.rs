// use air::models::{ModelProvider, QueryContext};
// use air::providers::GeminiProvider;
// use air::config::CloudProviderConfig;
// use std::time::Duration;

// #[tokio::test]
// async fn test_gemini_provider_creation() {
//     let config = CloudProviderConfig {
//         name: "gemini".to_string(),
//         api_key: Some("test-key".to_string()),
//         base_url: "https://generativelanguage.googleapis.com".to_string(),
//         model: "gemini-1.5-flash".to_string(),
//         max_tokens: 1000,
//         temperature: 0.7,
//         timeout_seconds: 30,
//     };

//     let provider = GeminiProvider::new(config).unwrap();
//     assert_eq!(provider.name(), "Gemini");
//     assert!(provider.is_available());
//     assert_eq!(provider.estimated_latency_ms(), 1000);
//     assert_eq!(provider.quality_score(), 0.92);
// }

// #[tokio::test]
// async fn test_gemini_provider_without_api_key() {
//     let config = CloudProviderConfig {
//         name: "gemini".to_string(),
//         api_key: None,
//         base_url: "https://generativelanguage.googleapis.com".to_string(),
//         model: "gemini-1.5-flash".to_string(),
//         max_tokens: 1000,
//         temperature: 0.7,
//         timeout_seconds: 30,
//     };

//     let provider = GeminiProvider::new(config).unwrap();
//     assert_eq!(provider.name(), "Gemini");
//     assert!(!provider.is_available());
// }

// #[tokio::test]
// async fn test_gemini_query_context() {
//     let context = QueryContext {
//         prompt: "Hello, how are you?".to_string(),
//         max_tokens: 100,
//         temperature: 0.5,
//         timeout: Duration::from_secs(10),
//         pure_mode: false,
//     };

//     assert_eq!(context.prompt, "Hello, how are you?");
//     assert_eq!(context.max_tokens, 100);
//     assert_eq!(context.temperature, 0.5);
// }

// #[tokio::test]
// async fn test_gemini_provider_metrics() {
//     let config = CloudProviderConfig {
//         name: "gemini".to_string(),
//         api_key: Some("test-key".to_string()),
//         base_url: "https://generativelanguage.googleapis.com".to_string(),
//         model: "gemini-1.5-flash".to_string(),
//         max_tokens: 1000,
//         temperature: 0.7,
//         timeout_seconds: 30,
//     };

//     let provider = GeminiProvider::new(config).unwrap();
    
//     // Test provider properties
//     assert!(provider.quality_score() > 0.9);
//     assert!(provider.estimated_latency_ms() < 2000);
//     assert_eq!(provider.name(), "Gemini");
// }

// // Integration test - only runs if GEMINI_KEY environment variable is set
// #[tokio::test]
// #[ignore = "requires actual API key"]
// async fn test_gemini_real_api_call() {
//     let api_key = std::env::var("GEMINI_KEY").ok();
//     if api_key.is_none() {
//         println!("Skipping real API test - no GEMINI_KEY environment variable");
//         return;
//     }

//     let config = CloudProviderConfig {
//         name: "gemini".to_string(),
//         api_key,
//         base_url: "https://generativelanguage.googleapis.com".to_string(),
//         model: "gemini-1.5-flash".to_string(),
//         max_tokens: 100,
//         temperature: 0.7,
//         timeout_seconds: 30,
//     };

//     let provider = GeminiProvider::new(config).unwrap();
//     let context = QueryContext {
//         prompt: "Say hello in exactly 3 words".to_string(),
//         max_tokens: 50,
//         temperature: 0.1,
//         timeout: Duration::from_secs(30),
//         pure_mode: false,
//     };

//     let result = provider.generate(&context).await;
//     match result {
//         Ok(response) => {
//             println!("Gemini response: {}", response.content);
//             assert!(!response.content.is_empty());
//             assert!(response.model_used.contains("Gemini"));
//             assert!(response.response_time_ms > 0);
//         }
//         Err(e) => {
//             println!("API call failed (expected if no valid key): {}", e);
//         }
//     }
// }
