# RUAI - Rust AI Agent

A high-performance AI agent that intelligently routes queries between local GGUF models and cloud APIs for optimal speed and quality.

## Features

🏠 **Local Model Support**: Uses your local GGUF models (like TinyLlama) for fast inference  
☁️ **Cloud Fallback**: Automatically falls back to OpenAI/Anthropic for complex queries  
⚡ **Smart Routing**: Optimizes for both speed and quality  
📊 **Performance Monitoring**: Tracks metrics and response times  
🛠️ **Configurable**: Easy TOML configuration file  

## Model Configuration

Your local model is configured at:
```
C:\models\tinyllama-1.1b-chat-v1.0.Q2_K.gguf
```

## Quick Start

1. **Build the project:**
   ```bash
   cargo build --release
   ```

2. **Set up API keys (optional):**
   ```bash
   # Windows PowerShell
   $env:OPENAI_API_KEY="your-openai-api-key"
   $env:ANTHROPIC_API_KEY="your-anthropic-api-key"
   ```

3. **Run with smart fallback (default):**
   ```bash
   cargo run -- --prompt "Explain quantum computing in simple terms"
   ```

4. **Force local model only:**
   ```bash
   cargo run -- --prompt "Hello, how are you?" --local-only
   ```

5. **Force cloud model only:**
   ```bash
   cargo run -- --prompt "Write a complex analysis of market trends" --cloud-only
   ```

## Usage Examples

### Basic Query (Smart Routing)
```bash
cargo run -- -p "What is machine learning?"
```
- Tries local model first for speed
- Falls back to cloud if local is slow/unavailable
- May use cloud for quality improvement on complex queries

### Local Only (Fastest)
```bash
cargo run -- -p "Simple question?" -l
```
- Uses only your local TinyLlama model
- Fastest response time
- Good for simple queries and testing

### Cloud Only (Best Quality)
```bash
cargo run -- -p "Write a detailed technical specification" -c
```
- Uses only cloud providers (OpenAI/Anthropic)
- Best quality responses
- Higher latency but better for complex tasks

### Verbose Output
```bash
cargo run -- -p "Your question" -v
```
- Shows detailed logging
- Performance metrics
- Provider selection reasoning

## Configuration

Edit `config.toml` to customize:

- **Local model settings**: Path, tokens, temperature
- **Cloud providers**: API endpoints, models, timeouts  
- **Performance tuning**: Timeouts, quality thresholds
- **Routing behavior**: When to prefer local vs cloud

## Architecture

```
┌─────────────────┐    ┌──────────────────┐
│   User Query    │────│   AI Agent       │
└─────────────────┘    └──────────────────┘
                              │
                ┌─────────────┼─────────────┐
                │             │             │
        ┌───────▼────┐ ┌──────▼─────┐ ┌─────▼──────┐
        │Local Model │ │OpenAI API  │ │Anthropic   │
        │(TinyLlama) │ │            │ │API         │
        └────────────┘ └────────────┘ └────────────┘
             ⚡              ☁️              ☁️
          Fast/Local    High Quality    High Quality
```

## Performance Strategy

1. **Speed First**: Local model for immediate responses
2. **Quality Check**: Evaluate if cloud could do better  
3. **Smart Fallback**: Automatic failover on timeout/error
4. **Adaptive**: Learn from response patterns over time

## Dependencies

- **Local Inference**: llama-cpp-rs (optional), candle-rs (optional)
- **Cloud APIs**: reqwest, serde_json
- **Async Runtime**: tokio
- **Configuration**: toml, clap
- **Logging**: tracing

## Building for Production

For production deployment, you might want to:

1. **Enable specific features:**
   ```bash
   cargo build --release --features llama-cpp
   # or
   cargo build --release --features candle
   ```

2. **Optimize for your hardware:**
   - Adjust thread count in config
   - Tune context length for memory usage
   - Set appropriate timeouts

3. **Monitor performance:**
   - Enable verbose logging
   - Track response times and success rates
   - Tune quality thresholds based on your use case

This gives you the best approach for faster execution while maintaining quality through intelligent model routing!