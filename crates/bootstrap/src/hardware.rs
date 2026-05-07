//! Hardware detection — RAM, CPU, GPU for model tier recommendations.

use sysinfo::System;

use crate::types::{HardwareInfo, ModelTier};

/// Detect hardware capabilities of the host machine.
pub fn detect() -> HardwareInfo {
    let sys = System::new_all();
    let ram_gb = (sys.total_memory() / (1024 * 1024 * 1024)) as u32;
    let cpu_cores = sys.cpus().len() as u32;
    let gpu = detect_gpu();
    let metal_support = cfg!(target_os = "macos") && gpu.is_some();
    let recommended_tier = ModelTier::from_ram(ram_gb);

    HardwareInfo { ram_gb, cpu_cores, gpu, metal_support, recommended_tier }
}

/// Detect GPU name. On macOS, reads from system_profiler.
fn detect_gpu() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("sysctl")
            .args(["-n", "machdep.cpu.brand_string"])
            .output()
            .ok()?;
        let brand = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if brand.contains("Apple") {
            return Some(brand);
        }
    }

    #[cfg(target_os = "linux")]
    {
        let output = std::process::Command::new("lspci")
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            if line.contains("VGA") || line.contains("3D") {
                return Some(line.trim().to_string());
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_returns_nonzero_ram() {
        let hw = detect();
        assert!(hw.ram_gb > 0, "should detect at least 1GB RAM");
    }

    #[test]
    fn detect_returns_nonzero_cores() {
        let hw = detect();
        assert!(hw.cpu_cores > 0, "should detect at least 1 CPU core");
    }

    #[test]
    fn detect_sets_model_tier() {
        let hw = detect();
        // Whatever tier is assigned, it should be valid
        assert!(matches!(
            hw.recommended_tier,
            ModelTier::Minimum | ModelTier::Recommended | ModelTier::Full
        ));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn detect_finds_gpu_on_macos() {
        let hw = detect();
        assert!(hw.gpu.is_some(), "should detect GPU on macOS");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn metal_support_on_macos() {
        let hw = detect();
        if hw.gpu.as_ref().is_some_and(|g| g.contains("Apple")) {
            assert!(hw.metal_support, "Apple Silicon should have Metal support");
        }
    }
}
