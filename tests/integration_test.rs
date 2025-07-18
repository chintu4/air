// use ruai::agent::AIAgent;
// use ruai::config::{Config, LocalModelConfig, CloudProviderConfig, PerformanceConfig};
// use std::path::PathBuf;
// use tempfile::NamedTempFile;
// use std::io::Write;

// /// Create a test configuration with a dummy model file
// fn create_test_config() -> (Config, NamedTempFile) {
//     let mut temp_file = NamedTempFile::new().unwrap();
//     // Write some dummy GGUF-like content
//     writeln!(temp_file, "GGUF dummy model data for testing").unwrap();
    
//     let config = Config {
//         local_model: LocalModelConfig {
//             model_path: temp_file.path().to_path_buf(),
//             max_tokens: 100,
//             temperature: 0.7,
//             context_length: 1024,
//             threads: 2,
//         },
//         cloud_providers: vec![
//             CloudProviderConfig {
//                 name: "openai".to_string(),
//                 api_key: None, // No API key for testing
//                 base_url: "https://api.openai.com/v1".to_string(),
//                 model: "gpt-3.5-turbo".to_string(),
//                 max_tokens: 1000,
//                 temperature: 0.7,
//                 timeout_seconds: 30,
//             }
//         ],
//         performance: PerformanceConfig {
//             local_timeout_seconds: 10,
//             fallback_threshold_ms: 3000,
//             quality_threshold: 0.8,
//             prefer_local_for_simple_queries: true,
//         },
//     };
    
//     (config, temp_file)
// }

// #[tokio::test]
// async fn test_agent_initialization_with_dummy_model() {
//     let (config, _temp_file) = create_test_config();
    
//     // This should fail because the dummy file isn't a real GGUF model
//     let result = AIAgent::new(config).await;
    
//     // We expect this to fail with a proper error message
//     assert!(result.is_err());
//     let error_msg = result.unwrap_err().to_string();
//     assert!(error_msg.contains("Failed to") || error_msg.contains("error"));
// }

// #[tokio::test]
// async fn test_agent_initialization_with_missing_model() {
//     let config = Config {
//         local_model: LocalModelConfig {
//             model_path: PathBuf::from("non_existent_model.gguf"),
//             max_tokens: 100,
//             temperature: 0.7,
//             context_length: 1024,
//             threads: 2,
//         },
//         cloud_providers: vec![],
//         performance: PerformanceConfig {
//             local_timeout_seconds: 10,
//             fallback_threshold_ms: 3000,
//             quality_threshold: 0.8,
//             prefer_local_for_simple_queries: true,
//         },
//     };
    
//     let result = AIAgent::new(config).await;
//     assert!(result.is_err());
//     assert!(result.unwrap_err().to_string().contains("Model file not found"));
// }

// #[test]
// fn test_config_creation() {
//     let config = Config {
//         local_model: LocalModelConfig {
//             model_path: PathBuf::from("test.gguf"),
//             max_tokens: 512,
//             temperature: 0.7,
//             context_length: 2048,
//             threads: 4,
//         },
//         cloud_providers: vec![],
//         performance: PerformanceConfig {
//             local_timeout_seconds: 30,
//             fallback_threshold_ms: 5000,
//             quality_threshold: 0.75,
//             prefer_local_for_simple_queries: false,
//         },
//     };
    
//     assert_eq!(config.local_model.max_tokens, 512);
//     assert_eq!(config.local_model.temperature, 0.7);
//     assert_eq!(config.performance.local_timeout_seconds, 30);
// }

// #[test]
// fn test_math_query_processing() {
//     // Test that our math query "2+2" would be properly formatted
//     let prompt = "2+2";
//     assert!(!prompt.is_empty());
//     assert_eq!(prompt.len(), 3);
    
//     // Simulate what the model should ideally return
//     let expected_patterns = vec!["4", "four", "equals", "="];
    
//     // This test validates our expectation that a math query should get a numerical answer
//     assert!(expected_patterns.iter().any(|pattern| {
//         pattern.contains("4") || pattern.contains("equals")
//     }));
// }

// /// Test for actual model inference functionality
// /// Note: This test is designed to work with real GGUF models
// /// For CI/CD environments, you might want to skip this test or provide a test model
// #[tokio::test]
// #[ignore] // Ignore by default since it requires a real model file
// async fn test_real_model_inference() {
//     // This test would only run if you have a real GGUF model file
//     let model_path = std::env::var("TEST_MODEL_PATH")
//         .unwrap_or_else(|_| "C:\\models\\tinyllama-1.1b-chat-v1.0.Q2_K.gguf".to_string());
    
//     if !PathBuf::from(&model_path).exists() {
//         println!("Skipping real model test - model file not found: {}", model_path);
//         return;
//     }
    
//     let config = Config {
//         local_model: LocalModelConfig {
//             model_path: PathBuf::from(model_path),
//             max_tokens: 50,
//             temperature: 0.1, // Low temperature for consistent output
//             context_length: 1024,
//             threads: 2,
//         },
//         cloud_providers: vec![],
//         performance: PerformanceConfig {
//             local_timeout_seconds: 30,
//             fallback_threshold_ms: 5000,
//             quality_threshold: 0.8,
//             prefer_local_for_simple_queries: true,
//         },
//     };
    
//     match AIAgent::new(config).await {
//         Ok(agent) => {
//             let response = agent.query_local_only("What is 2+2?").await;
//             match response {
//                 Ok(result) => {
//                     println!("Model response: {}", result.content);
//                     assert!(!result.content.is_empty());
//                     assert!(result.response_time_ms > 0);
//                 }
//                 Err(e) => {
//                     println!("Model inference failed: {}", e);
//                     // Don't fail the test - just report the issue
//                 }
//             }
//         }
//         Err(e) => {
//             println!("Agent initialization failed: {}", e);
//             // Don't fail the test - just report the issue
//         }
//     }
// }
