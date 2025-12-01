use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JetsonHardware {
    pub is_jetson: bool,
    pub model: String,
    pub codename: String,
    pub soc: String,
    pub module: String,
    pub board_id: String,
    pub serial_number: String,
    pub l4t_version: String,
    pub jetpack_version: String,
    pub cuda_arch: String,
    pub governors: Vec<String>,
    pub sensors: Vec<String>,
    pub power_rails: Vec<String>,
    pub engines: Vec<String>,
    pub nvpmodel_modes: Vec<String>,
}

static MODULE_NAME_TABLE: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("p3701-0000", "NVIDIA Jetson AGX Orin");
    m.insert("p3701-0004", "NVIDIA Jetson AGX Orin (32GB)");
    m.insert("p3701-0005", "NVIDIA Jetson AGX Orin (64GB)");
    m.insert("p3767-0000", "NVIDIA Jetson Orin NX (16GB)");
    m.insert("p3767-0001", "NVIDIA Jetson Orin NX (8GB)");
    m.insert("p3767-0003", "NVIDIA Jetson Orin Nano (8GB)");
    m.insert("p3767-0004", "NVIDIA Jetson Orin Nano (4GB)");
    m.insert("p3668-0000", "NVIDIA Jetson Xavier NX (DevKit)");
    m.insert("p3668-0001", "NVIDIA Jetson Xavier NX");
    m.insert("p2888-0001", "NVIDIA Jetson AGX Xavier (16GB)");
    m.insert("p2888-0004", "NVIDIA Jetson AGX Xavier (32GB)");
    m.insert("p3448-0000", "NVIDIA Jetson Nano (4GB)");
    m.insert("p3448-0002", "NVIDIA Jetson Nano (eMMC)");
    m.insert("p3448-0003", "NVIDIA Jetson Nano (2GB)");
    m.insert("p3310-1000", "NVIDIA Jetson TX2");
    m.insert("p2180-1000", "NVIDIA Jetson TX1");
    m
});

static CUDA_ARCH_TABLE: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("tegra234", "8.7"); // Orin
    m.insert("tegra194", "7.2"); // Xavier
    m.insert("tegra186", "6.2"); // TX2
    m.insert("tegra210", "5.3"); // TX1/Nano
    m
});

impl JetsonHardware {
    pub fn detect() -> Self {
        let mut hw = JetsonHardware::default();

        // 1. Check if it's a Jetson (nv_tegra_release exists)
        if Path::new("/etc/nv_tegra_release").exists() {
            hw.is_jetson = true;
            hw.l4t_version = Self::read_l4t_version();
            hw.jetpack_version = Self::map_l4t_to_jetpack(&hw.l4t_version);
            hw.governors = Self::detect_governors();
            hw.sensors = Self::detect_thermal_sensors();
            hw.power_rails = Self::detect_power_rails();
            hw.engines = Self::detect_engines();
            hw.nvpmodel_modes = Self::detect_nvpmodel_modes();
        } else {
            // Fallback for dev/emulator
            hw.is_jetson = false;
            hw.model = "Generic Host (Emulator Mode)".to_string();
            return hw;
        }

        // 2. Read Model from device tree
        if let Ok(model) = fs::read_to_string("/sys/firmware/devicetree/base/model") {
            hw.model = model.trim_matches('\0').trim().to_string();
        }

        // 3. Read SoC (compatible)
        if let Ok(compatible) = fs::read_to_string("/proc/device-tree/compatible") {
            let parts: Vec<&str> = compatible.split('\0').collect();
            let maybe_last = parts.iter().rev().find(|item| match item {
                s if !s.is_empty() => true,
                _ => false,
            });
            if let Some(last) = maybe_last {
                // usually something like "nvidia,tegra234"
                if let Some(soc) = last.split(',').nth(1) {
                    hw.soc = soc.to_string();
                    if let Some(arch) = CUDA_ARCH_TABLE.get(soc) {
                        hw.cuda_arch = arch.to_string();
                    }
                }
            }
        }

        // 4. Read Serial Number
        if let Ok(serial) = fs::read_to_string("/sys/firmware/devicetree/base/serial-number") {
            hw.serial_number = serial.trim_matches('\0').trim().to_string();
        }

        // 5. Try to identify specific module via dtsfilename or boardids
        // This is a simplified version of jtop's logic
        if let Ok(dts) = fs::read_to_string("/proc/device-tree/nvidia,dtsfilename") {
            // Example: /dvs/git/dirty/git-master_linux/kernel/kernel-5.10/arch/arm64/boot/dts/../../../../../../hardware/nvidia/platform/t23x/p3768/kernel-dts/tegra234-p3701-0000-p3737-0000.dts
            // We look for pXXXX-XXXX patterns
            let parts: Vec<&str> = dts.split('/').collect();
            if let Some(filename) = parts.last() {
                // Try to match pXXXX-XXXX
                for (id, name) in MODULE_NAME_TABLE.iter() {
                    if filename.contains(id) {
                        hw.module = name.to_string();
                        hw.board_id = id.to_string();
                        break;
                    }
                }
            }
        }

        hw
    }

    pub fn detect_governors() -> Vec<String> {
        let mut govs = Vec::new();
        if let Ok(entries) = fs::read_dir("/sys/devices/system/cpu") {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.starts_with("cpu"))
                    .unwrap_or(false)
                {
                    continue;
                }
                let gov_path = path.join("cpufreq/scaling_governor");
                if let Ok(gov) = fs::read_to_string(gov_path) {
                    let g = gov.trim().to_string();
                    if !g.is_empty() && !govs.contains(&g) {
                        govs.push(g);
                    }
                }
            }
        }
        govs
    }

    fn read_l4t_version() -> String {
        if let Ok(content) = fs::read_to_string("/etc/nv_tegra_release") {
            // # R35 (release), REVISION: 4.1, GCID: 33958178, BOARD: t186ref, EABI: aarch64, DATE: Tue Aug  1 19:57:35 UTC 2023
            let parts: Vec<&str> = content.split(',').collect();
            if parts.len() >= 2 {
                let release = parts[0].trim().replace("# R", ""); // "35 (release)"
                let release = release.split(' ').next().unwrap_or("").trim(); // "35"
                let revision = parts[1].trim().replace("REVISION: ", ""); // "4.1"
                return format!("{}.{}", release, revision);
            }
        }
        "Unknown".to_string()
    }

    fn map_l4t_to_jetpack(l4t: &str) -> String {
        // Simplified mapping table
        match l4t {
            "36.3.0" => "6.0",
            "36.2.0" => "6.0 DP",
            "35.5.0" => "5.1.3",
            "35.4.1" => "5.1.2",
            "35.3.1" => "5.1.1",
            "35.2.1" => "5.1",
            "35.1.0" => "5.0.2",
            "32.7.4" => "4.6.4",
            "32.7.1" => "4.6.1",
            "32.6.1" => "4.6",
            "32.5.1" => "4.5.1",
            "32.4.4" => "4.4.1",
            _ => "Unknown",
        }
        .to_string()
    }

    pub fn detect_nvpmodel_modes() -> Vec<String> {
        let mut modes = Vec::new();
        if let Ok(content) = fs::read_to_string("/etc/nvpmodel.conf") {
            for line in content.lines() {
                if line.trim().starts_with("< MODEL") {
                    // < MODEL ID=0 NAME=MAXN >
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    for part in parts {
                        if part.starts_with("NAME=") {
                            let name = part.replace("NAME=", "").replace(">", "");
                            modes.push(name);
                        }
                    }
                }
            }
        }
        modes
    }

    pub fn detect_fan() -> bool {
        // Check for pwm-fan in hwmon
        if let Ok(entries) = fs::read_dir("/sys/class/hwmon") {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Ok(name) = fs::read_to_string(path.join("name")) {
                    if name.trim() == "pwm-fan" {
                        return true;
                    }
                }
            }
        }
        false
    }

    pub fn detect_thermal_sensors() -> Vec<String> {
        let mut sensors = Vec::new();
        if let Ok(entries) = fs::read_dir("/sys/devices/virtual/thermal") {
            for entry in entries.flatten() {
                let path = entry.path();
                if path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.starts_with("thermal_zone"))
                    .unwrap_or(false)
                {
                    if let Ok(name) = fs::read_to_string(path.join("type")) {
                        let sensor = name.trim().to_string();
                        if !sensor.is_empty() && !sensors.contains(&sensor) {
                            sensors.push(sensor);
                        }
                    }
                }
            }
        }
        sensors
    }

    pub fn detect_power_rails() -> Vec<String> {
        let mut rails = Vec::new();
        if let Ok(content) = fs::read_to_string("/etc/nvpmodel.conf") {
            for line in content.lines() {
                for token in line.split_whitespace() {
                    if token.starts_with("VDD_") && !rails.contains(&token.to_string()) {
                        rails.push(token.to_string());
                    }
                }
            }
        }
        if rails.is_empty() {
            rails.extend(
                ["VDD_IN", "VDD_CPU", "VDD_GPU", "VDD_SOC", "VDD_WIFI"]
                    .iter()
                    .map(|s| s.to_string()),
            );
        }
        rails
    }

    pub fn detect_engines() -> Vec<String> {
        vec![
            "GR3D".to_string(),
            "EMC".to_string(),
            "NVENC".to_string(),
            "NVDEC".to_string(),
            "VIC".to_string(),
            "NVJPG".to_string(),
        ]
    }
}
