#!/usr/bin/env pwsh
# Build script for RUAI

Write-Host "🦀 Building RUAI - Rust AI Agent" -ForegroundColor Green
Write-Host "=================================" -ForegroundColor Green

Write-Host "📦 Building release version..." -ForegroundColor Yellow
cargo build --release

if ($LASTEXITCODE -eq 0) {
    Write-Host "✅ Build successful!" -ForegroundColor Green
    Write-Host ""
    Write-Host "🚀 Ready to run:" -ForegroundColor Cyan
    Write-Host "  .\target\release\ruai.exe --prompt 'Your question here'" -ForegroundColor White
    Write-Host ""
    Write-Host "📋 Quick commands:" -ForegroundColor Blue
    Write-Host "  Local only:  .\target\release\ruai.exe -p 'Hello' -l" -ForegroundColor White
    Write-Host "  Cloud only:  .\target\release\ruai.exe -p 'Complex task' -c" -ForegroundColor White
    Write-Host "  Smart route: .\target\release\ruai.exe -p 'Your prompt'" -ForegroundColor White
    Write-Host "  Verbose:     .\target\release\ruai.exe -p 'Your prompt' -v" -ForegroundColor White
} else {
    Write-Host "❌ Build failed!" -ForegroundColor Red
}
