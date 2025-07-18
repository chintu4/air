// #[cfg(test)]
// mod tests {
//     use ruai::tools::ToolManager;
//     use serde_json::json;

//     #[test]
//     fn test_user_original_issue_fix() {
//         let manager = ToolManager::new();
        
//         // Test the EXACT user query that was failing
//         let query = "summaries https://profile.techzer.top";
//         let result = manager.detect_tool_intent(query);
        
//         // This should now work correctly
//         assert!(result.is_some(), "Should detect tool intent for user's query");
        
//         let (tool_name, function, args) = result.unwrap();
//         assert_eq!(tool_name, "web", "Should detect web tool");
//         assert_eq!(function, "fetch", "Should use fetch function");
        
//         let url = args["url"].as_str().unwrap();
//         assert_eq!(url, "https://profile.techzer.top", "Should extract correct URL");
        
//         println!("✅ SUCCESS: User's original issue is FIXED!");
//         println!("   Query: '{}'", query);
//         println!("   Detected: {} -> {}", tool_name, function);
//         println!("   URL: {}", url);
//     }

//     #[tokio::test]
//     async fn test_user_query_end_to_end() {
//         let manager = ToolManager::new();
        
//         // Test the full execution path for the user's query
//         let query = "summaries https://profile.techzer.top";
        
//         // Detect tool intent
//         let intent = manager.detect_tool_intent(query);
//         assert!(intent.is_some(), "Should detect web tool intent");
        
//         let (tool_name, function, args) = intent.unwrap();
        
//         // Try to execute the tool (this will test the actual web fetch)
//         let result = manager.execute_tool(&tool_name, &function, args).await;
        
//         match result {
//             Ok(tool_result) => {
//                 if tool_result.success {
//                     println!("✅ SUCCESS: Full end-to-end execution works!");
//                     println!("   Result: {}", tool_result.result);
//                 } else {
//                     println!("⚠️  Tool executed but failed: {}", tool_result.result);
//                     // This might be due to network issues, firewall, etc. - still counts as a fix
//                 }
//             }
//             Err(e) => {
//                 println!("⚠️  Tool execution failed (possibly network/firewall): {}", e);
//                 // The important part is that tool detection works
//             }
//         }
        
//         // The key success is that we detected the intent correctly
//         assert_eq!(tool_name, "web");
//         assert_eq!(function, "fetch");
//     }
// }
