use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HwMeta {
    pub is_jetson: bool,
    pub nv_tegra_release: Option<String>,
    pub hostname: Option<String>,
}

pub fn detect_hw_meta() -> HwMeta {
    let nv_tegra_release = fs::read_to_string("/etc/nv_tegra_release")
        .ok()
        .map(|s| s.trim().to_string());
    let is_jetson = nv_tegra_release.is_some() || which::which("tegrastats").is_ok();
    let hostname = fs::read_to_string("/etc/hostname")
        .ok()
        .map(|s| s.trim().to_string());
    HwMeta {
        is_jetson,
        nv_tegra_release,
        hostname,
    }
}
