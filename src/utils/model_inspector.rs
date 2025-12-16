use std::path::Path;
use sysinfo::System;
use tracing::info;

#[derive(Debug, Clone)]
pub struct SystemContext {
    pub total_ram_gb: f64,
    pub model_size_gb: f64,
    pub is_constrained: bool,
}

pub fn inspect_system(model_path: &str) -> SystemContext {
    // 1. Get System RAM
    let mut sys = System::new();
    sys.refresh_memory();
    let total_ram_bytes = sys.total_memory();
    let total_ram_gb = total_ram_bytes as f64 / 1024.0 / 1024.0 / 1024.0;

    // 2. Get Model File Size
    let path = Path::new(model_path);
    let model_size_bytes = if path.exists() {
        std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
    } else {
        0
    };
    let model_size_gb = model_size_bytes as f64 / 1024.0 / 1024.0 / 1024.0;

    // 3. Determine Constraints
    // Heuristic: If model takes up more than 40% of TOTAL RAM, we are constrained.
    // Or if the model is small (< 2.8GB file usually implies < 4B params).
    let ram_pressure = if total_ram_gb > 0.0 {
        model_size_gb / total_ram_gb
    } else {
        1.0 // Assume worst case if sysinfo fails
    };

    // A model is "small/constrained" if:
    // 1. It is explicitly a small model (approx < 2.8GB file size for < 4B params)
    // 2. OR it is too big for the host system (> 40% RAM usage implies context will submit to swap)
    let is_small_model = model_size_gb < 2.8; // Approx threshold for 3B Q4 model
    let is_memory_tight = ram_pressure > 0.4;

    let is_constrained = is_small_model || is_memory_tight;

    info!(
        "🖥️  System Check: RAM: {:.1} GB | Model: {:.1} GB (Pressure: {:.0}%)",
        total_ram_gb, model_size_gb, ram_pressure * 100.0
    );

    if is_constrained {
        info!("🚀 Optimization: System constrained or small model detected. Engaging efficient mode.");
    } else {
        info!("🧠 Optimization: Ample resources detected. Engaging full context mode.");
    }

    SystemContext {
        total_ram_gb,
        model_size_gb,
        is_constrained,
    }
}
