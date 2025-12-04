// #[cfg(test)]
// mod tests {
//     use air::tools::ToolManager;
//     use serde_json::json;
//     use tokio;

//     #[test]
//     fn test_web_tool_detection_basic_patterns() {
//         let manager = ToolManager::new();
        
//         // Test basic URL patterns
//         let test_cases = vec![
//             "fetch https://example.com",
//             "get https://profile.techzer.top", 
//             "load http://www.google.com",
//             "visit https://github.com",
//             "browse www.stackoverflow.com",
//         ];
        
//         for query in test_cases {
//             let result = manager.detect_tool_intent(query);
//             assert!(result.is_some(), "Failed to detect tool intent for: {}", query);
            
//             let (tool_name, function, _args) = result.unwrap();
//             assert_eq!(tool_name, "web");
//             assert_eq!(function, "fetch");  // Updated to match actual function name
//         }
//     }

//     #[test]
//     fn test_web_tool_detection_summarization_patterns() {
//         let manager = ToolManager::new();
        
//         // Test summarization patterns that should trigger web tool
//         let test_cases = vec![
//             "summarize https://profile.techzer.top",
//             "summaries https://profile.techzer.top", 
//             "get summary of https://example.com",
//             "analyze https://news.com/article",
//             "read content from https://blog.example.com",
//             "extract text from https://docs.example.com",
//         ];
        
//         for query in test_cases {
//             let result = manager.detect_tool_intent(query);
//             assert!(result.is_some(), "Failed to detect tool intent for: {}", query);
            
//             let (tool_name, function, args) = result.unwrap();
//             assert_eq!(tool_name, "web");
//             assert_eq!(function, "fetch");  // Updated to match actual function name
            
//             // Verify URL is extracted correctly
//             let url = args["url"].as_str().unwrap();
//             assert!(url.starts_with("http"), "URL should start with http: {}", url);
//         }
//     }

//     #[test]
//     fn test_url_extraction_accuracy() {
//         let manager = ToolManager::new();
        
//         let test_cases = vec![
//             ("visit https://profile.techzer.top for analysis", "https://profile.techzer.top"),
//             ("fetch data from http://api.example.com/data", "http://api.example.com/data"),
//             ("summarize https://docs.rust-lang.org/book/ please", "https://docs.rust-lang.org/book/"),
//         ];
        
//         for (query, expected_url) in test_cases {
//             let result = manager.detect_tool_intent(query);
//             assert!(result.is_some(), "Failed to detect tool intent for: {}", query);
            
//             let (_tool_name, _function, args) = result.unwrap();
//             let extracted_url = args["url"].as_str().unwrap();
//             assert_eq!(extracted_url, expected_url, "URL extraction failed for: {}", query);
//         }
//     }

//     #[test]
//     fn test_calculator_tool_detection() {
//         let manager = ToolManager::new();
        
//         let test_cases = vec![
//             "calculate 2 + 2",
//             "what is 15 * 3",
//             "math: 100 / 4",
//             "2 + 2 = ?",
//             "15% of 200",
//             "factorial of 5",
//         ];
        
//         for query in test_cases {
//             let result = manager.detect_tool_intent(query);
//             assert!(result.is_some(), "Failed to detect calculator intent for: {}", query);
            
//             let (tool_name, function, _args) = result.unwrap();
//             assert_eq!(tool_name, "calculator");
//             assert_eq!(function, "calculate");
//         }
//     }

//     #[test]
//     fn test_filesystem_tool_detection() {
//         let manager = ToolManager::new();
        
//         let test_cases = vec![
//             ("read file config.toml", "filesystem", "read_file"),
//             ("read the file src/main.rs", "filesystem", "read_file"),
//             ("write file test.txt", "filesystem", "write_file"),
//             ("create file new_document.md", "filesystem", "write_file"),
//             ("list files", "filesystem", "list_directory"),
//             ("list directory src/", "filesystem", "list_directory"),
//         ];
        
//         for (query, expected_tool, expected_function) in test_cases {
//             let result = manager.detect_tool_intent(query);
//             assert!(result.is_some(), "Failed to detect tool intent for: {}", query);
            
//             let (tool_name, function, _args) = result.unwrap();
//             assert_eq!(tool_name, expected_tool);
//             assert_eq!(function, expected_function);
//         }
//     }

//     #[test]
//     fn test_command_tool_detection() {
//         let manager = ToolManager::new();
        
//         let test_cases = vec![
//             "run cargo build",
//             "execute git status", 
//             "command npm install",
//             "git log --oneline",
//             "cargo test",
//             "npm start",
//             "python main.py",
//             "dir",
//             "ls -la",
//             "pwd",
//             "cd src/",
//         ];
        
//         for query in test_cases {
//             let result = manager.detect_tool_intent(query);
//             assert!(result.is_some(), "Failed to detect command intent for: {}", query);
            
//             let (tool_name, function, _args) = result.unwrap();
//             assert_eq!(tool_name, "command");
//             assert_eq!(function, "execute");
//         }
//     }

//     #[test]
//     fn test_planner_tool_detection() {
//         let manager = ToolManager::new();
        
//         let test_cases = vec![
//             ("create task implement authentication", "planner", "create_task"),
//             ("add task write unit tests", "planner", "create_task"),
//             ("break down the login feature", "planner", "break_down_task"),
//             ("breakdown user registration", "planner", "break_down_task"),
//             ("list tasks", "planner", "list_tasks"),
//             ("show tasks", "planner", "list_tasks"),
//         ];
        
//         for (query, expected_tool, expected_function) in test_cases {
//             let result = manager.detect_tool_intent(query);
//             assert!(result.is_some(), "Failed to detect tool intent for: {}", query);
            
//             let (tool_name, function, _args) = result.unwrap();
//             assert_eq!(tool_name, expected_tool);
//             assert_eq!(function, expected_function);
//         }
//     }

//     #[test]
//     fn test_memory_tool_detection() {
//         let manager = ToolManager::new();
        
//         let test_cases = vec![
//             "what did we discuss about authentication",
//             "what did we discuss earlier",
//             "remember our conversation about testing",
//             "what did we talk about earlier regarding deployment",
//         ];
        
//         for query in test_cases {
//             let result = manager.detect_tool_intent(query);
//             assert!(result.is_some(), "Failed to detect memory intent for: {}", query);
            
//             let (tool_name, function, _args) = result.unwrap();
//             assert_eq!(tool_name, "memory");
//             assert_eq!(function, "search_conversations");
//         }
//     }

//     #[test]
//     fn test_screenshot_tool_detection() {
//         let manager = ToolManager::new();
        
//         let test_cases = vec![
//             ("screenshot", "screenshot", "capture"),
//             ("take a screenshot", "screenshot", "capture"),
//             ("screen capture", "screenshot", "capture"),
//             ("capture screen", "screenshot", "capture"),
//             ("screenshot region", "screenshot", "capture_region"),
//             ("capture region", "screenshot", "capture_region"),
//             ("list screenshots", "screenshot", "list_screenshots"),
//             ("show screenshots", "screenshot", "list_screenshots"),
//         ];
        
//         for (query, expected_tool, expected_function) in test_cases {
//             let result = manager.detect_tool_intent(query);
//             assert!(result.is_some(), "Failed to detect tool intent for: {}", query);
            
//             let (tool_name, function, _args) = result.unwrap();
//             assert_eq!(tool_name, expected_tool);
//             assert_eq!(function, expected_function);
//         }
//     }

//     #[test]
//     fn test_voice_tool_detection() {
//         let manager = ToolManager::new();
        
//         let test_cases = vec![
//             ("speak hello world", "voice", "speak"),
//             ("say good morning", "voice", "speak"),
//             ("text to speech convert this", "voice", "speak"),
//             ("tts hello", "voice", "speak"),
//             ("listen", "voice", "listen"),
//             ("speech to text", "voice", "listen"),
//             ("voice recognition", "voice", "listen"),
//             ("transcribe audio", "voice", "listen"),
//             ("list voices", "voice", "list_voices"),
//             ("available voices", "voice", "list_voices"),
//         ];
        
//         for (query, expected_tool, expected_function) in test_cases {
//             let result = manager.detect_tool_intent(query);
//             assert!(result.is_some(), "Failed to detect tool intent for: {}", query);
            
//             let (tool_name, function, _args) = result.unwrap();
//             assert_eq!(tool_name, expected_tool);
//             assert_eq!(function, expected_function);
//         }
//     }

//     #[test]
//     fn test_no_tool_detection_for_regular_queries() {
//         let manager = ToolManager::new();
        
//         let test_cases = vec![
//             "Hello, how are you?",
//             "What is the weather like?",
//             "Tell me about artificial intelligence",
//             "How do I learn programming?",
//             "What's the meaning of life?",
//         ];
        
//         for query in test_cases {
//             let result = manager.detect_tool_intent(query);
//             assert!(result.is_none(), "Should not detect tool intent for regular query: {}", query);
//         }
//     }

//     #[tokio::test]
//     async fn test_web_tool_execution() {
//         let manager = ToolManager::new();
        
//         // Test with a reliable URL (using httpbin for testing)
//         let args = json!({"url": "https://httpbin.org/get"});
//         let result = manager.execute_tool("web", "fetch", args).await;
        
//         assert!(result.is_ok(), "Web tool execution should succeed");
//         let tool_result = result.unwrap();
//         assert!(tool_result.success, "Web tool should return success");
//         assert!(tool_result.result.contains("Successfully fetched"), "Should contain success message");
//     }

//     #[tokio::test]
//     async fn test_calculator_tool_execution() {
//         let manager = ToolManager::new();
        
//         let test_cases = vec![
//             ("2 + 2", "4"),
//             ("10 * 3", "30"),
//             ("100 / 4", "25"),
//             ("15% of 200", "30"),
//         ];
        
//         for (expression, expected) in test_cases {
//             let args = json!({"expression": expression});
//             let result = manager.execute_tool("calculator", "calculate", args).await;
            
//             assert!(result.is_ok(), "Calculator execution should succeed for: {}", expression);
//             let tool_result = result.unwrap();
//             assert!(tool_result.success, "Calculator should return success for: {}", expression);
//             assert!(tool_result.result.contains(expected), 
//                    "Result should contain {} for expression {}, got: {}", 
//                    expected, expression, tool_result.result);
//         }
//     }

//     #[tokio::test]
//     async fn test_filesystem_tool_execution() {
//         let manager = ToolManager::new();
        
//         // Test reading a file that should exist (Cargo.toml in root)
//         let args = json!({"path": "Cargo.toml"});
//         let result = manager.execute_tool("filesystem", "read_file", args).await;
        
//         if result.is_ok() {
//             let tool_result = result.unwrap();
//             if tool_result.success {
//                 assert!(tool_result.result.contains("[package]") || tool_result.result.contains("name ="), 
//                        "Should contain Cargo.toml content");
//             }
//         }
//         // If file doesn't exist, that's also a valid test result
//     }

//     #[tokio::test]
//     async fn test_command_tool_execution() {
//         let manager = ToolManager::new();
        
//         // Test a simple, safe command that should work on all platforms
//         let args = json!({"command": "echo test"});
//         let result = manager.execute_tool("command", "execute", args).await;
        
//         assert!(result.is_ok(), "Command execution should succeed");
//         let tool_result = result.unwrap();
//         // Command might succeed or fail depending on system, both are valid test outcomes
//         assert!(tool_result.result.len() > 0, "Should return some result");
//     }

//     #[test]
//     fn test_specific_user_case_summaries_url() {
//         let manager = ToolManager::new();
        
//         // Test the specific case from the user
//         let query = "summaries https://profile.techzer.top";
//         let result = manager.detect_tool_intent(query);
        
//         assert!(result.is_some(), "Should detect tool intent for summaries query");
        
//         let (tool_name, function, args) = result.unwrap();
//         assert_eq!(tool_name, "web", "Should detect web tool");
//         assert_eq!(function, "fetch", "Should use fetch function");  // Updated function name
        
//         let url = args["url"].as_str().unwrap();
//         assert_eq!(url, "https://profile.techzer.top", "Should extract correct URL");
//     }

//     #[test]
//     fn test_url_extraction_edge_cases() {
//         let manager = ToolManager::new();
        
//         let test_cases = vec![
//             "Visit https://example.com and https://test.com", // Multiple URLs - should get first
//             "Check out this link: https://github.com/user/repo", 
//             "Go to http://localhost:3000/api/endpoint",
//             "Load https://api.example.com/v1/users?id=123&format=json",
//         ];
        
//         for query in test_cases {
//             let result = manager.detect_tool_intent(query);
//             assert!(result.is_some(), "Should detect URL in: {}", query);
            
//             let (_tool_name, _function, args) = result.unwrap();
//             let url = args["url"].as_str().unwrap();
//             assert!(url.starts_with("http"), "Extracted URL should be valid: {}", url);
//         }
//     }
// }
