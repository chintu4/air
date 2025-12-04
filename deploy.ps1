param (
    [string]$Configuration = "release"
)

$ErrorActionPreference = "Stop"

function Log ($Message, $Color="White") {
    $Time = Get-Date -Format "HH:mm:ss"
    Write-Host "[$Time] $Message" -ForegroundColor $Color
}

# Define paths
$SourceDir = "$PSScriptRoot\target\$Configuration"
$DestDir = "C:\Users\chintu\scripts" 
$ExeName = "air.exe"
$SourceExe = "$SourceDir\$ExeName"
$DestExe = "$DestDir\$ExeName"

Log "Starting deployment for configuration: $Configuration" "Cyan"

# 1. Build the project
Log "Step 1: Building the project..." "Yellow"
if ($Configuration -eq "release") {
    cargo build --release
} else {
    cargo build
}

# Check if build was successful
if (-not (Test-Path $SourceExe)) {
    Write-Error "Build failed! Binary not found at $SourceExe"
}
Log "Build successful. Binary located at $SourceExe" "Green"

# 2. Ensure destination directory exists
Log "Step 2: Checking destination directory..." "Yellow"
if (-not (Test-Path $DestDir)) {
    Log "Directory $DestDir does not exist. Creating it..."
    New-Item -ItemType Directory -Force -Path $DestDir | Out-Null
} else {
    Log "Destination directory $DestDir exists."
}

# 3. Move/Replace the binary
Log "Step 3: Updating binary..." "Yellow"
if (Test-Path $DestExe) {
    Log "Removing existing binary at $DestExe"
    Remove-Item $DestExe -Force
}

Log "Moving new binary from $SourceExe to $DestExe"
Move-Item -Path $SourceExe -Destination $DestExe -Force

Log "Deployment Successfully Completed!" "Green"

