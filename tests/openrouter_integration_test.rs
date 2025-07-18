// use ruai::models::{ModelProvider, QueryContext};
// use ruai::providers::OpenRouterProvider;
// use ruai::config::CloudProviderConfig;
// use std::time::Duration;

// #[tokio::test]
// async fn test_openrouter_provider_creation() {
//     let config = CloudProviderConfig {
//         name: "openrouter".to_string(),
//         api_key: Some("test-key".to_string()),
//         base_url: "https://openrouter.ai/api/v1".to_string(),
//         model: "anthropic/claude-3.5-haiku".to_string(),
//         max_tokens: 1000,
//         temperature: 0.7,
//         timeout_seconds: 30,
//     };

//     let provider = OpenRouterProvider::new(config).unwrap();
//     assert_eq!(provider.name(), "OpenRouter");
//     assert!(provider.is_available());
//     assert_eq!(provider.estimated_latency_ms(), 1200);
//     assert_eq!(provider.quality_score(), 0.90);
// }

// #[tokio::test]
// async fn test_openrouter_provider_without_api_key() {
//     let config = CloudProviderConfig {
//         name: "openrouter".to_string(),
//         api_key: None,
//         base_url: "https://openrouter.ai/api/v1".to_string(),
//         model: "anthropic/claude-3.5-haiku".to_string(),
//         max_tokens: 1000,
//         temperature: 0.7,
//         timeout_seconds: 30,
//     };

//     let provider = OpenRouterProvider::new(config).unwrap();
//     assert_eq!(provider.name(), "OpenRouter");
//     assert!(!provider.is_available());
// }

// #[tokio::test]
// async fn test_openrouter_different_models() {
//     let models = [
//         "anthropic/claude-3.5-haiku",
//         "openai/gpt-4o-mini",
//         "google/gemini-2.0-flash-exp",
//         "meta-llama/llama-3.1-8b-instruct",
//     ];

//     for model in &models {
//         let config = CloudProviderConfig {
//             name: "openrouter".to_string(),
//             api_key: Some("test-key".to_string()),
//             base_url: "https://openrouter.ai/api/v1".to_string(),
//             model: model.to_string(),
//             max_tokens: 1000,
//             temperature: 0.7,
//             timeout_seconds: 30,
//         };

//         let provider = OpenRouterProvider::new(config).unwrap();
//         assert!(provider.is_available());
//         println!("✅ OpenRouter provider works with model: {}", model);
//     }
// }

// // Integration test - only runs if OPEN_ROUTER environment variable is set
// #[tokio::test]
// #[ignore = "requires actual API key"]
// async fn test_openrouter_real_api_call() {
//     let api_key = std::env::var("OPEN_ROUTER").ok();
//     if api_key.is_none() {
//         println!("Skipping real API test - no OPEN_ROUTER environment variable");
//         return;
//     }

//     let config = CloudProviderConfig {
//         name: "openrouter".to_string(),
//         api_key,
//         base_url: "https://openrouter.ai/api/v1".to_string(),
//         model: "anthropic/claude-3.5-haiku".to_string(),
//         max_tokens: 100,
//         temperature: 0.7,
//         timeout_seconds: 30,
//     };

//     let provider = OpenRouterProvider::new(config).unwrap();
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
//             println!("OpenRouter response: {}", response.content);
//             assert!(!response.content.is_empty());
//             assert!(response.model_used.contains("OpenRouter"));
//             assert!(response.response_time_ms > 0);
//         }
//         Err(e) => {
//             println!("API call failed (expected if no valid key): {}", e);
//         }
//     }
// }
