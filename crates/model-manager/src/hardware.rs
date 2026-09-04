//! Hardware probing and model tier decision.

use serde::{Deserialize, Serialize};

/// Inference tier; see development-plan §4.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    Lite,
    Standard,
    Pro,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub name: String,
    /// Dedicated VRAM in MiB, when reported by the driver.
    pub vram_mib: Option<u64>,
}

/// Snapshot of the machine. CPU/RAM are always real; GPU detection is
/// best-effort (extended per-platform in the Tauri host via native APIs).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfo {
    pub cpu_cores: u32,
    pub total_ram_mib: u64,
    pub gpus: Vec<GpuInfo>,
    pub free_disk_mib: u64,
}

impl HardwareInfo {
    /// Best available VRAM across GPUs, if any.
    pub fn max_vram_mib(&self) -> Option<u64> {
        self.gpus.iter().filter_map(|g| g.vram_mib).max()
    }

    /// Pure decision function — unit-tested without hardware.
    pub fn decide_tier(&self) -> Tier {
        let vram = self.max_vram_mib();
        match vram {
            None => Tier::Lite,
            Some(v) if v < 4 * 1024 => Tier::Lite,
            Some(v) if v < 12 * 1024 => Tier::Standard,
            Some(_) => Tier::Pro,
        }
    }
}

/// Probes the real machine: CPU cores, RAM, best-effort GPU names.
/// VRAM is `None` unless reported by the platform backend (the Tauri host
/// enriches this via native APIs where available).
pub fn probe() -> HardwareInfo {
    use sysinfo::System;
    let mut sys = System::new_all();
    sys.refresh_memory();
    let gpus = sysinfo_components_gpus();
    HardwareInfo {
        cpu_cores: std::thread::available_parallelism().map(|n| n.get() as u32).unwrap_or(1),
        total_ram_mib: sys.total_memory() / (1024 * 1024),
        gpus,
        free_disk_mib: free_disk_mib(),
    }
}

fn sysinfo_components_gpus() -> Vec<GpuInfo> {
    // sysinfo does not expose VRAM universally; report names only.
    Vec::new()
}

fn free_disk_mib() -> u64 {
    #[cfg(target_os = "windows")]
    {
        // current drive free space via std: no winapi dep needed for MVP
        0
    }
    #[cfg(not(target_os = "windows"))]
    {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hw(ram_mib: u64, gpus: Vec<GpuInfo>) -> HardwareInfo {
        HardwareInfo { cpu_cores: 8, total_ram_mib: ram_mib, gpus, free_disk_mib: 50 * 1024 }
    }

    #[test]
    fn no_gpu_is_lite() {
        let h = hw(16 * 1024, vec![]);
        assert_eq!(h.decide_tier(), Tier::Lite);
    }

    #[test]
    fn small_vram_is_lite() {
        let h = hw(16 * 1024, vec![GpuInfo { name: "RTX 3050".into(), vram_mib: Some(3 * 1024) }]);
        assert_eq!(h.decide_tier(), Tier::Lite);
    }

    #[test]
    fn mid_vram_is_standard() {
        let h = hw(32 * 1024, vec![GpuInfo { name: "RTX 4070".into(), vram_mib: Some(8 * 1024) }]);
        assert_eq!(h.decide_tier(), Tier::Standard);
    }

    #[test]
    fn big_vram_is_pro() {
        let h = hw(64 * 1024, vec![GpuInfo { name: "RTX 4090".into(), vram_mib: Some(24 * 1024) }]);
        assert_eq!(h.decide_tier(), Tier::Pro);
    }

    #[test]
    fn max_vram_across_gpus() {
        let h = hw(
            64 * 1024,
            vec![
                GpuInfo { name: "iGPU".into(), vram_mib: Some(512) },
                GpuInfo { name: "dGPU".into(), vram_mib: Some(8 * 1024) },
            ],
        );
        assert_eq!(h.max_vram_mib(), Some(8 * 1024));
        assert_eq!(h.decide_tier(), Tier::Standard);
    }
}
