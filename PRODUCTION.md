# Production Setup for RUAI

This document explains how to set up RUAI for production use with actual GGUF model inference.

## Current Status

✅ **Working Now:**
- Smart routing logic between local and cloud models
- Configuration system  
- Cloud API integration (OpenAI, Anthropic)
- Performance monitoring and metrics
- Command-line interface
- **Simulated local inference** (for demo purposes)

⚠️ **For Production Use:**
- Replace simulated inference with actual GGUF model loading

## Production Setup Options

### Option 1: Using llama-cpp-2 (Recommended)

1. **Update Cargo.toml:**
```toml
[dependencies]
llama-cpp-2 = "0.1"
```

2. **Update local provider (`src/providers/local.rs`):**
```rust
use llama_cpp_2::model::Model;
use llama_cpp_2::context::Context;

// Replace simulate_inference with actual model loading:
async fn actual_inference(&self, prompt: &str, max_tokens: u32) -> Result<String> {
    let model = Model::load_from_file(&self.config.model_path, Default::default())?;
    let mut context = Context::new(&model, Default::default())?;
    
    let output = context.generate(prompt, max_tokens)?;
    Ok(output)
}
```

### Option 2: Using Candle-rs (Pure Rust)

1. **Update Cargo.toml:**
```toml
[dependencies]
candle-core = "0.15"
candle-nn = "0.15" 
candle-transformers = "0.15"
candle-examples = "0.15"
```

2. **Implement GGUF loading with candle-rs**

### Option 3: Using External Process

Call external tools like `llama.cpp` executable:

```rust
async fn external_inference(&self, prompt: &str) -> Result<String> {
    let output = tokio::process::Command::new("./llama.cpp")
        .args(&["-m", &self.config.model_path.to_string_lossy(), "-p", prompt])
        .output()
        .await?;
    
    Ok(String::from_utf8(output.stdout)?)
}
```

## Environment Variables

Set these for cloud fallback:

```powershell
# Windows PowerShell
$env:OPENAI_API_KEY = "your-openai-key"
$env:ANTHROPIC_API_KEY = "your-anthropic-key"

# Or create a .env file
echo "OPENAI_API_KEY=your-key" > .env
echo "ANTHROPIC_API_KEY=your-key" >> .env
```

## Model Recommendations

**For TinyLlama (Current Setup):**
- Good for: Quick responses, simple Q&A, testing
- Speed: Very fast (~500ms)
- Quality: Basic but functional

**For Better Quality (Upgrade Options):**
- **Llama 3.2 3B**: Better quality, still fast
- **Llama 3.1 7B**: High quality, moderate speed  
- **Mistral 7B**: Excellent for coding tasks

## Performance Tuning

Edit `config.toml`:

```toml
[local_model]
threads = 8  # Adjust for your CPU
context_length = 4096  # Increase for longer conversations
temperature = 0.7  # Lower for more focused responses

[performance]
local_timeout_seconds = 15  # Adjust based on model size
fallback_threshold_ms = 2000  # When to consider "fast enough"
```

## GPU Acceleration

For CUDA support (if using llama-cpp-2):

```toml
[dependencies]
llama-cpp-2 = { version = "0.1", features = ["cuda"] }
```

Update model loading:
```rust
let params = ModelParams {
    n_gpu_layers: 35,  // Offload layers to GPU
    ..Default::default()
};
let model = Model::load_from_file(&path, params)?;
```

## Deployment

**For development:**
```bash
cargo run -- -p "Your prompt"
```

**For production:**
```bash
cargo build --release
.\target\release\ruai.exe -p "Your prompt"
```

**As a service:**
Consider wrapping in a web API using `axum` or `warp` for HTTP endpoints.

## Troubleshooting

**"Model file not found":**
- Check path in `config.toml`
- Ensure GGUF file exists and is readable

**Slow inference:**
- Reduce `context_length`
- Increase `threads` 
- Consider smaller model variant (Q4_K_M vs Q2_K)

**Cloud fallback not working:**
- Verify API keys are set
- Check network connectivity
- Review logs with `--verbose`

The current implementation gives you a solid foundation - just swap out the simulated inference for real model loading!
