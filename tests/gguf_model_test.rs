// use ruai::providers::GGUFModel;
// use std::path::PathBuf;

// #[tokio::test]
// async fn test_gguf_model_loading() {
//     // Test with the actual TinyLlama model path
//     let model_path = PathBuf::from(r"C:\models\tinyllama-1.1b-chat-v1.0.Q2_K.gguf");
    
//     if !model_path.exists() {
//         println!("Model file not found at {:?}, skipping test", model_path);
//         return;
//     }
    
//     println!("Attempting to load GGUF model from: {:?}", model_path);
    
//     match GGUFModel::load_from_path(&model_path).await {
//         Ok(mut model) => {
//             println!("✅ Model loaded successfully!");
//             println!("Model info: {}", model.get_model_info());
//             println!("Vocab size: {}", model.get_vocab_size());
            
//             // Test tokenization
//             let test_text = "Hello, world!";
//             match model.tokenize(test_text) {
//                 Ok(tokens) => {
//                     println!("✅ Tokenization successful: {:?}", tokens);
                    
//                     // Test detokenization
//                     match model.detokenize(&tokens) {
//                         Ok(decoded) => {
//                             println!("✅ Detokenization successful: '{}'", decoded);
//                         }
//                         Err(e) => {
//                             println!("❌ Detokenization failed: {}", e);
//                         }
//                     }
//                 }
//                 Err(e) => {
//                     println!("❌ Tokenization failed: {}", e);
//                 }
//             }
            
//             // Test simple generation
//             println!("Testing text generation...");
//             match model.generate("Hello", 10, 0.7).await {
//                 Ok(generated) => {
//                     println!("✅ Generation successful: '{}'", generated);
//                 }
//                 Err(e) => {
//                     println!("❌ Generation failed: {}", e);
//                 }
//             }
//         }
//         Err(e) => {
//             println!("❌ Failed to load model: {}", e);
//             // Print the error chain for debugging
//             let mut source = e.source();
//             while let Some(err) = source {
//                 println!("  Caused by: {}", err);
//                 source = err.source();
//             }
//         }
//     }
// }

// #[tokio::test]
// async fn test_gguf_model_fallback_tokenizer() {
//     // Test loading with a dummy path to test fallback tokenizer
//     let dummy_path = PathBuf::from("dummy.gguf");
    
//     // This should fail gracefully and we can test individual components
//     match GGUFModel::load_from_path(&dummy_path).await {
//         Ok(_) => {
//             println!("Unexpected success with dummy path");
//         }
//         Err(e) => {
//             println!("Expected failure with dummy path: {}", e);
//         }
//     }
// }

// #[test]
// fn test_model_path_validation() {
//     let model_path = PathBuf::from(r"C:\models\tinyllama-1.1b-chat-v1.0.Q2_K.gguf");
    
//     if model_path.exists() {
//         println!("✅ Model file exists at: {:?}", model_path);
        
//         // Check file size
//         match std::fs::metadata(&model_path) {
//             Ok(metadata) => {
//                 let size_mb = metadata.len() / (1024 * 1024);
//                 println!("Model file size: {} MB", size_mb);
                
//                 // TinyLlama should be around 600-800 MB
//                 if size_mb > 100 && size_mb < 2000 {
//                     println!("✅ File size looks reasonable for TinyLlama");
//                 } else {
//                     println!("⚠️  Unusual file size for TinyLlama model");
//                 }
//             }
//             Err(e) => {
//                 println!("❌ Failed to get file metadata: {}", e);
//             }
//         }
        
//         // Try to read the first few bytes to check GGUF magic
//         match std::fs::File::open(&model_path) {
//             Ok(mut file) => {
//                 use std::io::Read;
//                 let mut buffer = [0; 4];
//                 match file.read_exact(&mut buffer) {
//                     Ok(_) => {
//                         if &buffer == b"GGUF" {
//                             println!("✅ Valid GGUF magic header detected");
//                         } else {
//                             println!("❌ Invalid GGUF magic header: {:?}", buffer);
//                         }
//                     }
//                     Err(e) => {
//                         println!("❌ Failed to read file header: {}", e);
//                     }
//                 }
//             }
//             Err(e) => {
//                 println!("❌ Failed to open file: {}", e);
//             }
//         }
//     } else {
//         println!("❌ Model file not found at: {:?}", model_path);
//         println!("Please ensure the TinyLlama model is downloaded to this location");
//     }
// }
