#!/usr/bin/env pwsh
# PowerShell script for easy RUAI usage

param(
    [Parameter(Mandatory=$true)]
    [string]$Prompt,
    
    [switch]$LocalOnly,
    [switch]$CloudOnly, 
    [switch]$VerboseOutput
)

Write-Host "🤖 RUAI - Rust AI Agent" -ForegroundColor Green
Write-Host "=========================" -ForegroundColor Green

$args = @("--prompt", $Prompt)

if ($LocalOnly) {
    $args += "--local-only"
    Write-Host "🏠 Using local model only" -ForegroundColor Yellow
}
elseif ($CloudOnly) {
    $args += "--cloud-only"
    Write-Host "☁️  Using cloud models only" -ForegroundColor Cyan
}
else {
    Write-Host "🔄 Using smart routing (local first, cloud fallback)" -ForegroundColor Blue
}

if ($VerboseOutput) {
    $args += "--verbose"
}

Write-Host ""

# Run the Rust application
# cargo run -- @args
./target/debug/ruai @args
