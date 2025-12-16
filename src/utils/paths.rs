use std::path::PathBuf;
use anyhow::Result;

/// Returns the application data directory.
/// Uses `dirs::data_dir()` + "air" (e.g., %APPDATA%/air or ~/.local/share/air).
/// Creates the directory if it doesn't exist.
pub fn get_air_data_dir() -> Result<PathBuf> {
    let app_data = dirs::data_dir()
        .map(|p| p.to_string_lossy().to_string())
        .or_else(|| std::env::var("APPDATA").ok())
        .or_else(|| std::env::var("LOCALAPPDATA").ok())
        .unwrap_or_else(|| std::env::temp_dir().to_string_lossy().to_string());

    let path = PathBuf::from(app_data).join("air");

    if !path.exists() {
        std::fs::create_dir_all(&path)?;
    }

    Ok(path)
}
