// #[cfg(test)]
// mod calculator_tests {
//     use super::*;
//     use crate::tools::{Tool, ToolResult, calculator::CalculatorTool};
//     use serde_json::{json, Value};
//     use anyhow::Result;

//     #[tokio::test]
//     async fn test_basic_arithmetic() {
//         let calc = CalculatorTool::new();
        
//         // Test addition
//         let result = calc.execute("calculate", json!({"expression": "2+3"})).await.unwrap();
//         assert!(result.success);
//         assert!(result.result.contains("5"));
        
//         // Test multiplication  
//         let result = calc.execute("calculate", json!({"expression": "5*6"})).await.unwrap();
//         assert!(result.success);
//         assert!(result.result.contains("30"));
        
//         // Test division
//         let result = calc.execute("calculate", json!({"expression": "10/2"})).await.unwrap();
//         assert!(result.success);
//         assert!(result.result.contains("5"));
//     }

//     #[tokio::test]
//     async fn test_percentage_calculations() {
//         let calc = CalculatorTool::new();
        
//         // Test "15% of 200"
//         let result = calc.execute("calculate", json!({"expression": "15% of 200"})).await.unwrap();
//         assert!(result.success);
//         assert!(result.result.contains("30"));
        
//         // Test "50%of100" (no space)
//         let result = calc.execute("calculate", json!({"expression": "50%of100"})).await.unwrap();
//         assert!(result.success);
//         assert!(result.result.contains("50"));
//     }

//     #[tokio::test]
//     async fn test_statistics_function() {
//         let calc = CalculatorTool::new();
        
//         let numbers = vec![1.0, 2.0, 3.0, 4.0, 5.0];
//         let result = calc.execute("statistics", json!({"numbers": numbers})).await.unwrap();
        
//         assert!(result.success);
//         assert!(result.result.contains("Mean: 3.00"));
//         assert!(result.result.contains("Sum: 15.00"));
//         assert!(result.result.contains("Median: 3.00"));
//     }

//     #[tokio::test]
//     async fn test_factorial() {
//         let calc = CalculatorTool::new();
        
//         let result = calc.execute("factorial", json!({"number": 5})).await.unwrap();
//         assert!(result.success);
//         assert!(result.result.contains("120"));
        
//         // Test factorial limit
//         let result = calc.execute("factorial", json!({"number": 25})).await.unwrap();
//         assert!(!result.success);
//         assert!(result.result.contains("too large"));
//     }

//     #[tokio::test]
//     async fn test_percentage_function() {
//         let calc = CalculatorTool::new();
        
//         let result = calc.execute("percentage", json!({
//             "value": 25.0,
//             "total": 100.0
//         })).await.unwrap();
        
//         assert!(result.success);
//         assert!(result.result.contains("25.00%"));
//     }

//     #[tokio::test]
//     async fn test_unit_conversions() {
//         let calc = CalculatorTool::new();
        
//         // Test temperature conversion
//         let result = calc.execute("convert_units", json!({
//             "value": 0.0,
//             "from": "celsius",
//             "to": "fahrenheit"
//         })).await.unwrap();
//         assert!(result.success);
//         assert!(result.result.contains("32"));
        
//         // Test length conversion
//         let result = calc.execute("convert_units", json!({
//             "value": 1.0,
//             "from": "meters",
//             "to": "feet"
//         })).await.unwrap();
//         assert!(result.success);
//         assert!(result.result.contains("3.28"));
//     }

//     #[tokio::test]
//     async fn test_power_operations() {
//         let calc = CalculatorTool::new();
        
//         let result = calc.execute("calculate", json!({"expression": "2^3"})).await.unwrap();
//         assert!(result.success);
//         assert!(result.result.contains("8"));
//     }

//     #[tokio::test]
//     async fn test_error_handling() {
//         let calc = CalculatorTool::new();
        
//         // Test division by zero
//         let result = calc.execute("calculate", json!({"expression": "5/0"})).await.unwrap();
//         assert!(!result.success);
        
//         // Test invalid expression
//         let result = calc.execute("calculate", json!({"expression": "invalid_expr"})).await.unwrap();
//         assert!(!result.success);
        
//         // Test missing parameters
//         let result = calc.execute("calculate", json!({})).await;
//         assert!(result.is_err());
//     }

//     #[tokio::test]
//     async fn test_complex_expressions() {
//         let calc = CalculatorTool::new();
        
//         // Current implementation might not support complex expressions
//         // This test documents the limitation
//         let result = calc.execute("calculate", json!({"expression": "2+3*4"})).await.unwrap();
//         // This should fail with current simple parser
//         assert!(!result.success);
//         assert!(result.result.contains("Complex expressions not yet supported"));
//     }

//     #[test]
//     fn test_available_functions() {
//         let calc = CalculatorTool::new();
//         let functions = calc.available_functions();
        
//         assert!(functions.contains(&"calculate".to_string()));
//         assert!(functions.contains(&"statistics".to_string()));
//         assert!(functions.contains(&"convert_units".to_string()));
//         assert!(functions.contains(&"factorial".to_string()));
//         assert!(functions.contains(&"percentage".to_string()));
//     }

//     #[test]
//     fn test_tool_metadata() {
//         let calc = CalculatorTool::new();
//         assert_eq!(calc.name(), "calculator");
//         assert_eq!(calc.description(), "Mathematical calculations: arithmetic, statistics, conversions");
//     }
// }

// // Integration test for calculator tool with file reference enhancement
// #[cfg(test)]
// mod file_reference_enhancement_tests {
//     use super::*;

//     #[tokio::test]
//     async fn test_line_specific_file_reading() {
//         // This is a proposed enhancement test
//         // Shows how line-specific references could work in the future
        
//         // Example of what could be supported:
//         // ruai -p "read file src/calculator.rs lines 12-25"
//         // ruai -p "analyze file src/main.rs line 50"
//         // ruai -p "show file README.md lines 1-20"
        
//         // For now, this test documents the desired functionality
//         // Implementation would require enhancing the file reading logic
//         // in src/tools/filesystem.rs and src/providers/local.rs
        
//         assert!(true, "Line-specific file reading not yet implemented");
//     }
// }