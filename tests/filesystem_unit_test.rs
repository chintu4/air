// #[cfg(test)]
// mod filesystem_tests {
//     use air::tools::{Tool, ToolResult, filesystem::FileSystemTool};
//     use serde_json::{json, Value};
//     use std::fs;
//     use tempfile::{NamedTempFile, TempDir};
//     use std::io::Write;

//     #[tokio::test]
//     async fn test_read_file_relative_path() {
//         let temp_dir = TempDir::new().unwrap();
//         let file_path = temp_dir.path().join("test.txt");
//         fs::write(&file_path, "Hello, World!").unwrap();
        
//         let fs_tool = FileSystemTool::new(Some(temp_dir.path().to_string_lossy().to_string()));
        
//         let result = fs_tool.execute("read_file", json!({
//             "path": "test.txt"
//         })).await.unwrap();
        
//         assert!(result.success);
//         assert!(result.result.contains("Hello, World!"));
//     }

//     #[tokio::test]
//     async fn test_read_file_absolute_path() {
//         let mut temp_file = NamedTempFile::new().unwrap();
//         writeln!(temp_file, "Test content for absolute path").unwrap();
//         temp_file.flush().unwrap();
        
//         let fs_tool = FileSystemTool::new(None);
//         let absolute_path = temp_file.path().to_string_lossy().to_string();
        
//         let result = fs_tool.execute("read_file", json!({
//             "path": absolute_path
//         })).await.unwrap();
        
//         assert!(result.success);
//         assert!(result.result.contains("Test content for absolute path"));
//     }

//     #[tokio::test]
//     async fn test_write_file_absolute_path() {
//         let temp_dir = TempDir::new().unwrap();
//         let file_path = temp_dir.path().join("write_test.txt");
//         let absolute_path = file_path.to_string_lossy().to_string();
        
//         let fs_tool = FileSystemTool::new(None);
        
//         let result = fs_tool.execute("write_file", json!({
//             "path": absolute_path,
//             "content": "This is a test file written via absolute path"
//         })).await.unwrap();
        
//         assert!(result.success);
        
//         // Verify file was actually written
//         let content = fs::read_to_string(&file_path).unwrap();
//         assert_eq!(content, "This is a test file written via absolute path");
//     }

//     #[tokio::test]
//     async fn test_list_directory_absolute_path() {
//         let temp_dir = TempDir::new().unwrap();
        
//         // Create some test files
//         fs::write(temp_dir.path().join("file1.txt"), "content1").unwrap();
//         fs::write(temp_dir.path().join("file2.txt"), "content2").unwrap();
//         fs::create_dir(temp_dir.path().join("subdir")).unwrap();
        
//         let fs_tool = FileSystemTool::new(None);
//         let absolute_path = temp_dir.path().to_string_lossy().to_string();
        
//         let result = fs_tool.execute("list_directory", json!({
//             "path": absolute_path
//         })).await.unwrap();
        
//         assert!(result.success);
//         assert!(result.result.contains("file1.txt"));
//         assert!(result.result.contains("file2.txt"));
//         assert!(result.result.contains("subdir"));
//     }

//     #[tokio::test]
//     async fn test_file_exists_absolute_path() {
//         let mut temp_file = NamedTempFile::new().unwrap();
//         writeln!(temp_file, "Test").unwrap();
//         temp_file.flush().unwrap();
        
//         let fs_tool = FileSystemTool::new(None);
//         let absolute_path = temp_file.path().to_string_lossy().to_string();
        
//         let result = fs_tool.execute("file_exists", json!({
//             "path": absolute_path
//         })).await.unwrap();
        
//         assert!(result.success);
//         assert!(result.result.contains("File exists"));
//     }

//     #[tokio::test]
//     async fn test_get_file_info_absolute_path() {
//         let mut temp_file = NamedTempFile::new().unwrap();
//         writeln!(temp_file, "Test content for file info").unwrap();
//         temp_file.flush().unwrap();
        
//         let fs_tool = FileSystemTool::new(None);
//         let absolute_path = temp_file.path().to_string_lossy().to_string();
        
//         let result = fs_tool.execute("get_file_info", json!({
//             "path": absolute_path
//         })).await.unwrap();
        
//         assert!(result.success);
//         assert!(result.result.contains("File Information"));
        
//         if let Some(metadata) = result.metadata {
//             assert!(metadata["is_file"].as_bool().unwrap());
//             assert!(!metadata["is_directory"].as_bool().unwrap());
//             assert!(metadata["size"].as_u64().unwrap() > 0);
//         }
//     }

//     #[tokio::test]
//     async fn test_invalid_file_path() {
//         let fs_tool = FileSystemTool::new(None);
        
//         // Test with null byte (should fail)
//         let result = fs_tool.execute("read_file", json!({
//             "path": "test\0file.txt"
//         })).await.unwrap();
        
//         assert!(!result.success);
//         assert!(result.result.contains("Invalid file path"));
//     }

//     #[tokio::test]
//     async fn test_nonexistent_file() {
//         let fs_tool = FileSystemTool::new(None);
        
//         let result = fs_tool.execute("read_file", json!({
//             "path": "/nonexistent/path/file.txt"
//         })).await.unwrap();
        
//         assert!(!result.success);
//         assert!(result.result.contains("Failed to read file"));
//     }

//     #[tokio::test]
//     async fn test_cross_platform_paths() {
//         let fs_tool = FileSystemTool::new(None);
        
//         // Test Windows-style absolute path (on Windows)
//         #[cfg(windows)]
//         {
//             let mut temp_file = NamedTempFile::new().unwrap();
//             writeln!(temp_file, "Windows path test").unwrap();
//             temp_file.flush().unwrap();
            
//             let windows_path = temp_file.path().to_string_lossy().to_string();
            
//             let result = fs_tool.execute("read_file", json!({
//                 "path": windows_path
//             })).await.unwrap();
            
//             assert!(result.success);
//             assert!(result.result.contains("Windows path test"));
//         }
        
//         // Test Unix-style paths would work on Unix systems
//         #[cfg(unix)]
//         {
//             let mut temp_file = NamedTempFile::new().unwrap();
//             writeln!(temp_file, "Unix path test").unwrap();
//             temp_file.flush().unwrap();
            
//             let unix_path = temp_file.path().to_string_lossy().to_string();
            
//             let result = fs_tool.execute("read_file", json!({
//                 "path": unix_path
//             })).await.unwrap();
            
//             assert!(result.success);
//             assert!(result.result.contains("Unix path test"));
//         }
//     }

//     #[test]
//     fn test_available_functions() {
//         let fs_tool = FileSystemTool::new(None);
//         let functions = fs_tool.available_functions();
        
//         assert!(functions.contains(&"read_file".to_string()));
//         assert!(functions.contains(&"write_file".to_string()));
//         assert!(functions.contains(&"list_directory".to_string()));
//         assert!(functions.contains(&"file_exists".to_string()));
//         assert!(functions.contains(&"get_file_info".to_string()));
//     }

//     #[test]
//     fn test_tool_metadata() {
//         let fs_tool = FileSystemTool::new(None);
//         assert_eq!(fs_tool.name(), "filesystem");
//         assert_eq!(fs_tool.description(), "File system operations: read, write, list");
//     }

//     #[tokio::test]
//     async fn test_json_file_reading() {
//         // Test specifically for JSON files like the user's LOR.json
//         let temp_dir = TempDir::new().unwrap();
//         let json_file = temp_dir.path().join("test.json");
        
//         let json_content = r#"{
//     "name": "TestContract",
//     "version": "1.0.0",
//     "description": "A test smart contract",
//     "abi": [
//         {
//             "type": "function",
//             "name": "getValue",
//             "inputs": [],
//             "outputs": [{"type": "uint256", "name": "value"}]
//         }
//     ],
//     "bytecode": "0x608060405234801561001057600080fd5b50..."
// }"#;
        
//         fs::write(&json_file, json_content).unwrap();
        
//         let fs_tool = FileSystemTool::new(None);
//         let absolute_path = json_file.to_string_lossy().to_string();
        
//         let result = fs_tool.execute("read_file", json!({
//             "path": absolute_path
//         })).await.unwrap();
        
//         assert!(result.success);
//         assert!(result.result.contains("TestContract"));
//         assert!(result.result.contains("getValue"));
//         assert!(result.result.contains("bytecode"));
        
//         // Verify metadata contains correct information
//         if let Some(metadata) = result.metadata {
//             assert!(metadata["path"].as_str().unwrap().contains("test.json"));
//             assert!(metadata["size"].as_u64().unwrap() > 0);
//             assert!(metadata["lines"].as_u64().unwrap() > 10);
//         }
//     }
// }
