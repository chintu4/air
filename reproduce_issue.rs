use hf_hub::api::sync::Api;
use tokenizers::Tokenizer;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing hf_hub download...");
    match Api::new() {
        Ok(api) => {
            let repo = api.model("TinyLlama/TinyLlama-1.1B-Chat-v1.0".to_string());
            match repo.get("tokenizer.json") {
                Ok(path) => println!("Download success: {:?}", path),
                Err(e) => println!("Download failed: {}", e),
            }
        },
        Err(e) => println!("Api creation failed: {}", e),
    }

    println!("\nTesting tokenizer loading...");
    // Simulate the bad file
    std::fs::write("bad_tokenizer.json", "{}")?;
    match Tokenizer::from_file(Path::new("bad_tokenizer.json")) {
        Ok(_) => println!("Loaded bad tokenizer (unexpected)"),
        Err(e) => println!("Failed to load bad tokenizer: {}", e),
    }

    Ok(())
}
