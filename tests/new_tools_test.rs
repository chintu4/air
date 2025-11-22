// use air::tools::{
//     ToolManager, CommandTool, ScreenshotTool, VoiceTool, Tool
// };
// use serde_json::json;

// #[tokio::test]
// async fn test_new_tools_integration() {
//     // Test tool manager with new tools
//     let manager = ToolManager::new();
    
//     // Test command tool detection
//     let command_intent = manager.detect_tool_intent("run ls -la");
//     assert!(command_intent.is_some());
//     let (tool_name, function, _args) = command_intent.unwrap();
//     assert_eq!(tool_name, "command");
//     assert_eq!(function, "execute");
    
//     // Test screenshot tool detection
//     let screenshot_intent = manager.detect_tool_intent("take a screenshot");
//     assert!(screenshot_intent.is_some());
//     let (tool_name, function, _args) = screenshot_intent.unwrap();
//     assert_eq!(tool_name, "screenshot");
//     assert_eq!(function, "capture");
    
//     // Test voice tool detection
//     let voice_intent = manager.detect_tool_intent("speak hello world");
//     assert!(voice_intent.is_some());
//     let (tool_name, function, args) = voice_intent.unwrap();
//     assert_eq!(tool_name, "voice");
//     assert_eq!(function, "speak");
//     assert_eq!(args["text"], "hello world");
    
//     println!("✅ All new tools integrated successfully!");
// }

// #[tokio::test]
// async fn test_individual_tools() {
//     // Test CommandTool
//     let command_tool = CommandTool::new();
//     assert_eq!(command_tool.name(), "command");
//     assert!(command_tool.available_functions().contains(&"execute".to_string()));
    
//     // Test ScreenshotTool
//     let screenshot_tool = ScreenshotTool::new(None);
//     assert_eq!(screenshot_tool.name(), "screenshot");
//     assert!(screenshot_tool.available_functions().contains(&"capture".to_string()));
    
//     // Test VoiceTool
//     let voice_tool = VoiceTool::new(None);
//     assert_eq!(voice_tool.name(), "voice");
//     assert!(voice_tool.available_functions().contains(&"speak".to_string()));
    
//     println!("✅ Individual tools work correctly!");
// }

// #[tokio::test]
// async fn test_voice_commands_detection() {
//     let manager = ToolManager::new();
    
//     // Test various voice command patterns
//     let test_cases = vec![
//         ("speak hello", ("voice", "speak")),
//         ("say good morning", ("voice", "speak")),
//         ("text to speech test", ("voice", "speak")),
//         ("listen", ("voice", "listen")),
//         ("speech to text", ("voice", "listen")),
//         ("list voices", ("voice", "list_voices")),
//     ];
    
//     for (query, expected) in test_cases {
//         let intent = manager.detect_tool_intent(query);
//         assert!(intent.is_some(), "Failed to detect intent for: {}", query);
//         let (tool_name, function, _args) = intent.unwrap();
//         assert_eq!(tool_name, expected.0, "Wrong tool for: {}", query);
//         assert_eq!(function, expected.1, "Wrong function for: {}", query);
//     }
    
//     println!("✅ Voice command detection works!");
// }

// #[tokio::test]
// async fn test_screenshot_commands_detection() {
//     let manager = ToolManager::new();
    
//     let test_cases = vec![
//         ("screenshot", ("screenshot", "capture")),
//         ("take a screenshot", ("screenshot", "capture")),
//         ("screen capture", ("screenshot", "capture")),
//         ("capture screen", ("screenshot", "capture")),
//         ("list screenshots", ("screenshot", "list_screenshots")),
//         ("show screenshots", ("screenshot", "list_screenshots")),
//     ];
    
//     for (query, expected) in test_cases {
//         let intent = manager.detect_tool_intent(query);
//         assert!(intent.is_some(), "Failed to detect intent for: {}", query);
//         let (tool_name, function, _args) = intent.unwrap();
//         assert_eq!(tool_name, expected.0, "Wrong tool for: {}", query);
//         assert_eq!(function, expected.1, "Wrong function for: {}", query);
//     }
    
//     println!("✅ Screenshot command detection works!");
// }
