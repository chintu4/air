#!/usr/bin/env pwsh
# Example usage of RUAI

Write-Host "🤖 RUAI Examples" -ForegroundColor Green
Write-Host "=================" -ForegroundColor Green

Write-Host ""
Write-Host "1. Testing local model (fast)..." -ForegroundColor Yellow
cargo run -- --prompt "Hello! Can you tell me what 2+2 equals?" --local-only --verbose

Write-Host ""
Write-Host "2. Testing smart routing..." -ForegroundColor Blue  
cargo run -- --prompt "Explain the benefits of using Rust for systems programming" --verbose

if ($env:OPENAI_API_KEY -or $env:ANTHROPIC_API_KEY) {
    Write-Host ""
    Write-Host "3. Testing cloud model (if API key available)..." -ForegroundColor Cyan
    cargo run -- --prompt "Write a haiku about artificial intelligence" --cloud-only --verbose
} else {
    Write-Host ""
    Write-Host "⚠️  Skipping cloud test - no API keys found" -ForegroundColor Red
    Write-Host "Set OPENAI_API_KEY or ANTHROPIC_API_KEY to test cloud features" -ForegroundColor Red
}

Write-Host ""
Write-Host "✅ Examples complete!" -ForegroundColor Green
