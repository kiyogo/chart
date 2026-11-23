//! Automatic Hardware & Resource Capability Detector (Sections XVIII.7 & XXX).
//! Reads real host CPU/SIMD/RAM/GPU capabilities on Linux/Desktop and accepts browser/mobile telemetry cleanly.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceTier {
    Constrained, // Low RAM (< 4GB), mobile low-battery, single-core or no GPU
    Standard,    // Typical laptop / mid-range phone / browser WebGL/CPU SIMD
    HighEnd,     // Desktop / workstation / dedicated GPU / WebGPU / >= 16GB RAM
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccelerationBackend {
    WebGpu,
    Vulkan,
    Metal,
    DirectMl,
    NnApi,
    CoreMl,
    OnnxCpuSimd,
    SpabLiteCpuSimd,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HardwareProfile {
    pub platform: String,
    pub cpu_threads: usize,
    pub simd_instruction_set: String,
    pub total_ram_mb: u64,
    pub available_ram_mb: u64,
    pub has_gpu_compute: bool,
    pub gpu_name: Option<String>,
    pub vram_mb: Option<u64>,
    pub preferred_backend: AccelerationBackend,
    pub device_tier: DeviceTier,
    pub battery_percent: Option<u8>,
    pub power_saver_active: bool,
    pub thermal_throttled: bool,
    pub avif_decode_supported: bool,
    pub av1_hw_encode_supported: bool,
}

impl HardwareProfile {
    /// Detect real hardware capabilities on the current runtime platform.
    pub fn detect_local() -> Self {
        let cpu_threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);

        let simd_instruction_set = detect_simd_label();
        let (total_ram_mb, available_ram_mb) = detect_system_ram_mb();
        let (has_gpu_compute, gpu_name) = detect_gpu_basic();
        let av1_probe = crate::av1::probe_av1_hardware();

        let platform = if cfg!(target_arch = "wasm32") {
            "web_wasm".to_string()
        } else {
            format!("{}_{}", std::env::consts::OS, std::env::consts::ARCH)
        };

        let preferred_backend = if cfg!(target_arch = "wasm32") {
            AccelerationBackend::SpabLiteCpuSimd
        } else if cfg!(target_os = "macos") || cfg!(target_os = "ios") {
            if has_gpu_compute { AccelerationBackend::Metal } else { AccelerationBackend::OnnxCpuSimd }
        } else if cfg!(target_os = "windows") {
            if has_gpu_compute { AccelerationBackend::DirectMl } else { AccelerationBackend::OnnxCpuSimd }
        } else if cfg!(target_os = "android") {
            if has_gpu_compute { AccelerationBackend::Vulkan } else { AccelerationBackend::OnnxCpuSimd }
        } else if has_gpu_compute {
            AccelerationBackend::Vulkan
        } else {
            AccelerationBackend::OnnxCpuSimd
        };

        let device_tier = if total_ram_mb > 0 && total_ram_mb < 4096 || cpu_threads <= 2 {
            DeviceTier::Constrained
        } else if total_ram_mb >= 12288 && (cpu_threads >= 8 || has_gpu_compute) {
            DeviceTier::HighEnd
        } else {
            DeviceTier::Standard
        };

        Self {
            platform,
            cpu_threads,
            simd_instruction_set,
            total_ram_mb,
            available_ram_mb,
            has_gpu_compute,
            gpu_name,
            vram_mb: None,
            preferred_backend,
            device_tier,
            battery_percent: None,
            power_saver_active: false,
            thermal_throttled: false,
            avif_decode_supported: true,
            av1_hw_encode_supported: av1_probe.hardware_av1_encode_available,
        }
    }

    /// True when the device is under battery, thermal, or RAM pressure and must shed heavy AI work.
    pub fn is_under_resource_pressure(&self) -> bool {
        self.power_saver_active
            || self.thermal_throttled
            || self.battery_percent.map(|b| b < 20).unwrap_or(false)
            || (self.available_ram_mb > 0 && self.available_ram_mb < 1024)
    }
}

fn detect_simd_label() -> String {
    #[cfg(target_arch = "x86_64")]
    {
        if std::is_x86_feature_detected!("avx2") {
            return "x86_64_avx2_fma".into();
        }
        if std::is_x86_feature_detected!("sse4.2") {
            return "x86_64_sse4.2".into();
        }
        return "x86_64_sse2".into();
    }
    #[cfg(target_arch = "aarch64")]
    {
        return "aarch64_neon".into();
    }
    #[cfg(target_arch = "wasm32")]
    {
        #[cfg(target_feature = "simd128")]
        {
            return "wasm32_simd128".into();
        }
        #[cfg(not(target_feature = "simd128"))]
        {
            return "wasm32_scalar".into();
        }
    }
    #[allow(unreachable_code)]
    "generic_scalar".into()
}

fn detect_system_ram_mb() -> (u64, u64) {
    #[cfg(target_os = "linux")]
    {
        if let Ok(txt) = std::fs::read_to_string("/proc/meminfo") {
            let mut total_kb = 0u64;
            let mut avail_kb = 0u64;
            for line in txt.lines() {
                if let Some(rest) = line.strip_prefix("MemTotal:") {
                    total_kb = rest.split_whitespace().next().and_then(|s| s.parse().ok()).unwrap_or(0);
                } else if let Some(rest) = line.strip_prefix("MemAvailable:") {
                    avail_kb = rest.split_whitespace().next().and_then(|s| s.parse().ok()).unwrap_or(0);
                }
            }
            return (total_kb / 1024, avail_kb / 1024);
        }
    }
    (8192, 4096)
}

fn detect_gpu_basic() -> (bool, Option<String>) {
    #[cfg(target_os = "linux")]
    {
        if let Ok(entries) = std::fs::read_dir("/sys/class/drm") {
            for e in entries.flatten() {
                let uevent = e.path().join("device/uevent");
                if let Ok(txt) = std::fs::read_to_string(uevent) {
                    if let Some(driver) = txt.lines().find(|l| l.starts_with("DRIVER=")) {
                        let drv = driver.trim_start_matches("DRIVER=").to_string();
                        let pci = txt
                            .lines()
                            .find(|l| l.starts_with("PCI_ID="))
                            .map(|l| l.trim_start_matches("PCI_ID="))
                            .unwrap_or("");
                        if !drv.is_empty() {
                            return (true, Some(format!("{drv} ({pci})")));
                        }
                    }
                }
            }
        }
    }
    (false, None)
}
