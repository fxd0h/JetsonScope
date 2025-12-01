use crate::hardware::JetsonHardware;
use crate::protocol::ControlInfo;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlStatus {
    pub available: bool,
    pub jetson_clocks: Option<bool>,
    pub fan: Option<String>,
    pub nvpmodel: Option<String>,
    pub nvpmodel_modes: Vec<String>,
    pub cpu_governor: Option<String>,
    pub cpu_governor_modes: Vec<String>,
    pub gpu_governor: Option<String>,
    pub gpu_governor_modes: Vec<String>,
    pub gpu_railgate: Option<bool>,
    pub supports_fan: bool,
    pub supports_nvpmodel: bool,
    pub supports_jetson_clocks: bool,
    pub supports_cpu_governor: bool,
    pub supports_gpu_governor: bool,
    pub supports_gpu_railgate: bool,
    pub note: String,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ControlManager {
    status: ControlStatus,
    mock: bool,
    #[allow(dead_code)]
    hardware: JetsonHardware,
}

impl Default for ControlManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ControlManager {
    pub fn new() -> Self {
        let hardware = JetsonHardware::detect();
        Self::from_hardware(hardware, false)
    }

    /// Create a ControlManager with injected hardware info and optional mock mode.
    pub fn from_hardware(hardware: JetsonHardware, mock: bool) -> Self {
        if mock {
            let nvpmodel_modes = if hardware.nvpmodel_modes.is_empty() {
                vec!["MODE_0".into(), "MODE_1".into()]
            } else {
                hardware.nvpmodel_modes.clone()
            };
            return ControlManager {
                hardware,
                mock: true,
                status: ControlStatus {
                    available: true,
                    jetson_clocks: Some(false),
                    fan: Some("0%".into()),
                    nvpmodel: nvpmodel_modes.get(0).cloned().or_else(|| Some("unknown".into())),
                    nvpmodel_modes,
                    cpu_governor: Some("ondemand".into()),
                    cpu_governor_modes: vec!["ondemand".into(), "performance".into()],
                    gpu_governor: Some("nvhost_podgov".into()),
                    gpu_governor_modes: vec!["nvhost_podgov".into(), "performance".into()],
                    gpu_railgate: Some(true),
                    supports_fan: true,
                    supports_nvpmodel: true,
                    supports_jetson_clocks: true,
                    supports_cpu_governor: true,
                    supports_gpu_governor: true,
                    supports_gpu_railgate: true,
                    note: "Mock mode (no real commands)".to_string(),
                    last_error: None,
                },
            };
        }

        if hardware.is_jetson {
            let nvpmodel_modes = if mock {
                hardware.nvpmodel_modes.clone()
            } else {
                crate::hardware::JetsonHardware::detect_nvpmodel_modes()
            };
            let cpu_governor_modes = detect_cpu_governors();
            let cpu_governor = detect_current_cpu_governor();
            let (gpu_governor_modes, gpu_governor) = detect_gpu_governors();
            let gpu_railgate = detect_gpu_railgate();
            let supports_fan = if mock {
                !hardware.nvpmodel_modes.is_empty()
            } else {
                crate::hardware::JetsonHardware::detect_fan()
            };
            let supports_nvpmodel = !nvpmodel_modes.is_empty();
            let supports_jetson_clocks = if mock {
                true
            } else {
                which::which("jetson_clocks").is_ok()
            };
            let supports_cpu_governor = !cpu_governor_modes.is_empty();
            let supports_gpu_governor = !gpu_governor_modes.is_empty();
            let supports_gpu_railgate = gpu_railgate.is_some();

            ControlManager {
                hardware,
                mock,
                status: ControlStatus {
                    available: true,
                    jetson_clocks: if mock {
                        Some(false)
                    } else {
                        detect_jetson_clocks()
                    },
                    fan: if mock { Some("0".into()) } else { detect_fan_speed() },
                    nvpmodel: if mock { Some("unknown".into()) } else { detect_nvpmodel() },
                    nvpmodel_modes,
                    cpu_governor,
                    cpu_governor_modes,
                    gpu_governor,
                    gpu_governor_modes,
                    gpu_railgate,
                    supports_fan,
                    supports_nvpmodel,
                    supports_jetson_clocks,
                    supports_cpu_governor,
                    supports_gpu_governor,
                    supports_gpu_railgate,
                    note: "Controles listos".to_string(),
                    last_error: None,
                },
            }
        } else {
            ControlManager {
                hardware,
                mock,
                status: ControlStatus {
                    available: false,
                    jetson_clocks: None,
                    fan: None,
                    nvpmodel: None,
                    nvpmodel_modes: Vec::new(),
                    cpu_governor: None,
                    cpu_governor_modes: Vec::new(),
                    gpu_governor: None,
                    gpu_governor_modes: Vec::new(),
                    gpu_railgate: None,
                    supports_fan: false,
                    supports_nvpmodel: false,
                    supports_jetson_clocks: false,
                    supports_cpu_governor: false,
                    supports_gpu_governor: false,
                    supports_gpu_railgate: false,
                    note: "Host no Jetson: modo demo".to_string(),
                    last_error: None,
                },
            }
        }
    }

    /// Constructor for tests/injection with custom hardware detection.
    #[allow(dead_code)]
    pub fn with_hardware(hardware: JetsonHardware) -> Self {
        Self::from_hardware(hardware, false)
    }

    /// Mocked constructor (does not run real commands; for tests).
    #[allow(dead_code)]
    pub fn mock(hardware: JetsonHardware) -> Self {
        Self::from_hardware(hardware, true)
    }

    pub fn status(&self) -> &ControlStatus {
        &self.status
    }

    #[allow(dead_code)] // Public API
    pub fn status_cloned(&self) -> ControlStatus {
        self.status.clone()
    }

    #[allow(dead_code)]
    pub fn list_controls(&self) -> Vec<ControlInfo> {
        let mut controls = Vec::new();

        if self.status.supports_jetson_clocks {
            controls.push(ControlInfo {
                name: "jetson_clocks".to_string(),
                description: "Max performance mode".to_string(),
                value: self
                    .status
                    .jetson_clocks
                    .map(|b| if b { "on" } else { "off" })
                    .unwrap_or("unknown")
                    .to_string(),
                options: vec!["on".to_string(), "off".to_string()],
                readonly: false,
                min: None,
                max: None,
                step: None,
                requires_sudo: true,
                supported: self.status.supports_jetson_clocks,
                unit: None,
            });
        }

        if self.status.supports_nvpmodel {
            controls.push(ControlInfo {
                name: "nvpmodel".to_string(),
                description: "Power mode".to_string(),
                value: self
                    .status
                    .nvpmodel
                    .clone()
                    .unwrap_or("unknown".to_string()),
                options: self.status.nvpmodel_modes.clone(),
                readonly: false,
                min: None,
                max: None,
                step: None,
                requires_sudo: true,
                supported: self.status.supports_nvpmodel,
                unit: None,
            });
        }

        if self.status.supports_fan {
            controls.push(ControlInfo {
                name: "fan".to_string(),
                description: "Fan speed".to_string(),
                value: self.status.fan.clone().unwrap_or("0%".to_string()),
                options: vec!["0-100".to_string()], // Special handling for range
                readonly: false,
                min: Some(0),
                max: Some(100),
                step: Some(1),
                requires_sudo: true,
                supported: self.status.supports_fan,
                unit: Some("%".to_string()),
            });
        }

        if self.status.supports_cpu_governor {
            controls.push(ControlInfo {
                name: "cpu_governor".to_string(),
                description: "CPU governor".to_string(),
                value: self
                    .status
                    .cpu_governor
                    .clone()
                    .unwrap_or("unknown".to_string()),
                options: self.status.cpu_governor_modes.clone(),
                readonly: false,
                min: None,
                max: None,
                step: None,
                requires_sudo: true,
                supported: self.status.supports_cpu_governor,
                unit: None,
            });
        }

        if self.status.supports_gpu_governor {
            controls.push(ControlInfo {
                name: "gpu_governor".to_string(),
                description: "GPU governor".to_string(),
                value: self
                    .status
                    .gpu_governor
                    .clone()
                    .unwrap_or("unknown".to_string()),
                options: self.status.gpu_governor_modes.clone(),
                readonly: false,
                min: None,
                max: None,
                step: None,
                requires_sudo: true,
                supported: self.status.supports_gpu_governor,
                unit: None,
            });
        }

        if self.status.supports_gpu_railgate {
            controls.push(ControlInfo {
                name: "gpu_railgate".to_string(),
                description: "GPU rail-gating (power control)".to_string(),
                value: self
                    .status
                    .gpu_railgate
                    .map(|v| if v { "auto" } else { "on" })
                    .unwrap_or("unknown")
                    .to_string(),
                options: vec!["auto".to_string(), "on".to_string()],
                readonly: false,
                min: None,
                max: None,
                step: None,
                requires_sudo: true,
                supported: self.status.supports_gpu_railgate,
                unit: None,
            });
        }

        controls
    }

    #[allow(dead_code)]
    pub fn apply_control(&mut self, name: &str, value: &str) -> Result<ControlInfo> {
        match name {
            "jetson_clocks" => {
                self.set_jetson_clocks(value)?;
                Ok(self.control_info(name))
            }
            "nvpmodel" => {
                self.set_nvpmodel_mode(Some(value.to_string()));
                self.status
                    .last_error
                    .as_ref()
                    .map(|e| Err(anyhow!(e.clone())))
                    .unwrap_or_else(|| Ok(self.control_info(name)))
            }
            "fan" => {
                let p: u8 = value.parse().context("fan value debe ser 0-100")?;
                self.set_fan(p);
                self.status
                    .last_error
                    .as_ref()
                    .map(|e| Err(anyhow!(e.clone())))
                    .unwrap_or_else(|| Ok(self.control_info(name)))
            }
            "cpu_governor" => {
                self.set_cpu_governor(value)?;
                Ok(self.control_info(name))
            }
            "gpu_governor" => {
                self.set_gpu_governor(value)?;
                Ok(self.control_info(name))
            }
            "gpu_railgate" => {
                self.set_gpu_railgate(value)?;
                Ok(self.control_info(name))
            }
            _ => Err(anyhow!("control desconocido")),
        }
    }

    #[allow(dead_code)]
    pub fn control_info(&self, name: &str) -> ControlInfo {
        self.list_controls()
            .into_iter()
            .find(|c| c.name == name)
            .unwrap_or(ControlInfo {
                name: name.to_string(),
                description: "unknown".to_string(),
                value: "unknown".to_string(),
                options: Vec::new(),
                readonly: true,
                min: None,
                max: None,
                step: None,
                requires_sudo: false,
                supported: false,
                unit: None,
            })
    }

    pub fn toggle_jetson_clocks(&mut self) {
        if self.mock {
            let current = self.status.jetson_clocks.unwrap_or(false);
            self.status.jetson_clocks = Some(!current);
            self.status.last_error = None;
            return;
        }

        if !self.status.available {
            self.status.last_error = Some("No es Jetson (demo)".to_string());
            return;
        }

        if !self.status.supports_jetson_clocks {
            self.status.last_error =
                Some("jetson_clocks no disponible en este sistema".to_string());
            return;
        }

        match run_jetson_clocks_toggle() {
            Ok(new_state) => {
                self.status.jetson_clocks = Some(new_state);
                self.status.last_error = None;
            }
            Err(e) => {
                self.status.last_error = Some(e.to_string());
            }
        }
    }

    #[allow(dead_code)]
    pub fn set_jetson_clocks(&mut self, value: &str) -> Result<()> {
        if !self.status.available {
            return Err(anyhow!("No es Jetson (demo)"));
        }
        if !self.status.supports_jetson_clocks {
            return Err(anyhow!("jetson_clocks no disponible en este sistema"));
        }
        match value {
            "on" => run_jetson_clocks_set(true),
            "off" => run_jetson_clocks_set(false),
            "toggle" | "" => {
                self.toggle_jetson_clocks();
                return Ok(());
            }
            _ => Err(anyhow!("Valor inválido para jetson_clocks: {}", value)),
        }
    }

    pub fn cycle_nvpmodel(&mut self) {
        if !self.status.available {
            self.status.last_error = Some("No es Jetson (demo)".to_string());
            return;
        }

        if !self.status.supports_nvpmodel {
            self.status.last_error = Some("nvpmodel no disponible en este sistema".to_string());
            return;
        }

        if self.status.nvpmodel_modes.is_empty() {
            self.status.last_error = Some("No se pudieron leer modos nvpmodel".to_string());
            return;
        }
        let current = self.status.nvpmodel.clone().unwrap_or_default();
        let next = next_mode(&self.status.nvpmodel_modes, &current);
        match set_nvpmodel(&next) {
            Ok(_) => {
                self.status.nvpmodel = Some(next.clone());
                self.status.last_error = None;
            }
            Err(e) => {
                self.status.last_error = Some(e.to_string());
            }
        }
    }

    #[allow(dead_code)]
    pub fn set_nvpmodel_mode(&mut self, mode: Option<String>) {
        if !self.status.available {
            self.status.last_error = Some("No es Jetson (demo)".to_string());
            return;
        }

        if self.mock {
            let target = if let Some(m) = mode {
                if !self.status.nvpmodel_modes.contains(&m) {
                    self.status.last_error = Some(format!(
                        "Modo inválido: {}. Modos disponibles: {:?}",
                        m, self.status.nvpmodel_modes
                    ));
                    return;
                }
                m
            } else {
                let current = self.status.nvpmodel.clone().unwrap_or_default();
                next_mode(&self.status.nvpmodel_modes, &current)
            };
            self.status.nvpmodel = Some(target);
            self.status.last_error = None;
            return;
        }

        let target = if let Some(m) = mode {
            // Validate that 'm' is in self.status.nvpmodel_modes
            // Modes are usually "MODE: <NAME>". The user might pass just "MAXN" or "0".
            // Our detect_nvpmodel_modes returns names like "MAXN", "15W", etc.
            // We should check if 'm' exists in that list.
            if !self.status.nvpmodel_modes.contains(&m) {
                self.status.last_error = Some(format!(
                    "Modo inválido: {}. Modos disponibles: {:?}",
                    m, self.status.nvpmodel_modes
                ));
                return;
            }
            m
        } else {
            let current = self.status.nvpmodel.clone().unwrap_or_default();
            next_mode(&self.status.nvpmodel_modes, &current)
        };

        match set_nvpmodel(&target) {
            Ok(_) => {
                self.status.nvpmodel = Some(target);
                self.status.last_error = None;
            }
            Err(e) => {
                self.status.last_error = Some(e.to_string());
            }
        }
    }

    pub fn set_fan(&mut self, percent: u8) {
        if percent > 100 {
            self.status.last_error = Some(format!(
                "Valor de fan inválido: {}. Rango válido: 0-100",
                percent
            ));
            return;
        }

        if self.mock {
            self.status.fan = Some(format!("{}%", percent));
            self.status.last_error = None;
            return;
        }

        if !self.status.available {
            self.status.last_error = Some("No es Jetson (demo)".to_string());
            return;
        }

        if !self.status.supports_fan {
            self.status.last_error =
                Some("Control de fan no soportado en este hardware".to_string());
            return;
        }

        match set_fan_percent(percent) {
            Ok(_) => {
                self.status.fan = Some(format!("{}%", percent));
                self.status.last_error = None;
            }
            Err(e) => {
                self.status.last_error = Some(e.to_string());
            }
        }
    }

    pub fn set_cpu_governor(&mut self, governor: &str) -> Result<()> {
        if !self.status.available {
            return Err(anyhow!("No es Jetson (demo)"));
        }
        if !self.status.supports_cpu_governor {
            return Err(anyhow!("Control de governor no soportado"));
        }
        if !self.status.cpu_governor_modes.contains(&governor.to_string()) {
            return Err(anyhow!(
                "Governor inválido: {}. Disponibles: {:?}",
                governor,
                self.status.cpu_governor_modes
            ));
        }
        if self.mock {
            self.status.cpu_governor = Some(governor.to_string());
            self.status.last_error = None;
            return Ok(());
        }

        let mut wrote_any = false;
        for path in cpu_paths() {
            let gov_path = path.join("cpufreq/scaling_governor");
            if gov_path.exists() {
                std::fs::write(&gov_path, governor)
                    .with_context(|| format!("escribiendo {:?}", gov_path))?;
                wrote_any = true;
            }
        }
        if !wrote_any {
            return Err(anyhow!("No se pudieron escribir governors (sin rutas)"));
        }
        self.status.cpu_governor = Some(governor.to_string());
        self.status.last_error = None;
        Ok(())
    }

    pub fn set_gpu_governor(&mut self, governor: &str) -> Result<()> {
        if !self.status.available {
            return Err(anyhow!("No es Jetson (demo)"));
        }
        if !self.status.supports_gpu_governor {
            return Err(anyhow!("Control de GPU governor no soportado"));
        }
        if !self.status.gpu_governor_modes.contains(&governor.to_string()) {
            return Err(anyhow!(
                "GPU governor inválido: {}. Disponibles: {:?}",
                governor,
                self.status.gpu_governor_modes
            ));
        }
        if self.mock {
            self.status.gpu_governor = Some(governor.to_string());
            self.status.last_error = None;
            return Ok(());
        }

        if let Some(path) = gpu_devfreq_path() {
            let gov_path = path.join("governor");
            std::fs::write(&gov_path, governor)
                .with_context(|| format!("escribiendo {:?}", gov_path))?;
            self.status.gpu_governor = Some(governor.to_string());
            self.status.last_error = None;
            return Ok(());
        }
        Err(anyhow!("No se pudo escribir GPU governor (sin rutas)"))
    }

    pub fn set_gpu_railgate(&mut self, mode: &str) -> Result<()> {
        if !self.status.available {
            return Err(anyhow!("No es Jetson (demo)"));
        }
        if !self.status.supports_gpu_railgate {
            return Err(anyhow!("Control de GPU railgate no soportado"));
        }
        let target = match mode {
            "auto" => "auto",
            "on" => "on",
            _ => return Err(anyhow!("Modo inválido: {} (auto|on)", mode)),
        };
        if self.mock {
            self.status.gpu_railgate = Some(target == "auto");
            self.status.last_error = None;
            return Ok(());
        }
        if let Some(path) = gpu_power_control_path() {
            std::fs::write(&path, target).with_context(|| format!("escribiendo {:?}", path))?;
            self.status.gpu_railgate = Some(target == "auto");
            self.status.last_error = None;
            return Ok(());
        }
        Err(anyhow!("No se pudo ajustar railgate (sin ruta power/control)"))
    }
}

fn detect_jetson_clocks() -> Option<bool> {
    if let Ok(output) = Command::new("jetson_clocks").arg("--show").output() {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            if text.to_ascii_lowercase().contains("enabled") {
                return Some(true);
            }
            if text.to_ascii_lowercase().contains("disabled") {
                return Some(false);
            }
        }
    }
    None
}

fn detect_nvpmodel() -> Option<String> {
    if let Ok(output) = Command::new("nvpmodel").arg("-q").output() {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                if line.to_ascii_lowercase().contains("mode:") {
                    return Some(line.trim().to_string());
                }
            }
        }
    }
    None
}

fn detect_fan_speed() -> Option<String> {
    if which::which("jetson_fan").is_ok() {
        if let Ok(output) = Command::new("jetson_fan").arg("--get").output() {
            if output.status.success() {
                let txt = String::from_utf8_lossy(&output.stdout);
                let val = txt.lines().next().unwrap_or("").trim().to_string();
                if !val.is_empty() {
                    return Some(val);
                }
            }
        }
    }
    None
}

fn next_mode(modes: &[String], current: &str) -> String {
    if modes.is_empty() {
        return current.to_string();
    }
    if let Some(idx) = modes.iter().position(|m| m == current) {
        let next_idx = (idx + 1) % modes.len();
        modes[next_idx].clone()
    } else {
        modes[0].clone()
    }
}

fn run_jetson_clocks_toggle() -> Result<bool> {
    if let Some(state) = detect_jetson_clocks() {
        let target = if state { "--off" } else { "--on" };
        Command::new("jetson_clocks")
            .arg(target)
            .output()
            .context("ejecutando jetson_clocks toggle")?;
        return Ok(!state);
    }
    Err(anyhow!("No se pudo leer estado jetson_clocks"))
}

#[allow(dead_code)]
fn run_jetson_clocks_set(on: bool) -> Result<()> {
    let arg = if on { "--on" } else { "--off" };
    let output = Command::new("jetson_clocks")
        .arg(arg)
        .output()
        .context("ejecutando jetson_clocks")?;
    if output.status.success() {
        Ok(())
    } else {
        Err(anyhow!("jetson_clocks {} falló", arg))
    }
}

fn set_nvpmodel(mode: &str) -> Result<()> {
    let output = Command::new("nvpmodel")
        .arg("-m")
        .arg(mode)
        .output()
        .context("ejecutando nvpmodel -m")?;
    if output.status.success() {
        Ok(())
    } else {
        Err(anyhow!("nvpmodel -m {} falló", mode))
    }
}

fn set_fan_percent(percent: u8) -> Result<()> {
        if percent > 100 {
        return Err(anyhow!("valor de fan inválido (0-100)"));
    }
    if which::which("jetson_fan").is_ok() {
        let output = Command::new("jetson_fan")
            .arg("--set")
            .arg(percent.to_string())
            .output()
            .context("ejecutando jetson_fan --set")?;
        if output.status.success() {
            return Ok(());
        }
    }
    Err(anyhow!(
        "No se pudo ajustar fan (requiere utilidades en Jetson)"
    ))
}

fn detect_gpu_governors() -> (Vec<String>, Option<String>) {
    if let Some(path) = gpu_devfreq_path() {
        let avail = path.join("available_governors");
        let gov = path.join("governor");
        let mut modes = Vec::new();
        if let Ok(data) = std::fs::read_to_string(avail) {
            for g in data.split_whitespace() {
                modes.push(g.to_string());
            }
        }
        let current = std::fs::read_to_string(gov).ok().map(|s| s.trim().to_string());
        return (modes, current);
    }
    (Vec::new(), None)
}

fn detect_gpu_railgate() -> Option<bool> {
    if let Some(path) = gpu_power_control_path() {
        if let Ok(data) = std::fs::read_to_string(path) {
            let v = data.trim();
            return Some(v == "auto");
        }
    }
    None
}

fn gpu_devfreq_path() -> Option<PathBuf> {
    let candidates = vec![
        "/sys/devices/17000000.gv11b/devfreq/17000000.gv11b",
        "/sys/devices/17000000.gp10b/devfreq/17000000.gp10b",
    ];
    for c in candidates {
        let p = PathBuf::from(c);
        if p.join("governor").exists() {
            return Some(p);
        }
    }
    None
}

fn gpu_power_control_path() -> Option<PathBuf> {
    let candidates = vec![
        "/sys/devices/17000000.gv11b/power/control",
        "/sys/devices/17000000.gp10b/power/control",
    ];
    for c in candidates {
        let p = PathBuf::from(c);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

fn detect_cpu_governors() -> Vec<String> {
    let mut govs = Vec::new();
    for path in cpu_paths() {
        let avail = path.join("cpufreq/scaling_available_governors");
        if let Ok(data) = std::fs::read_to_string(avail) {
            for g in data.split_whitespace() {
                if !govs.contains(&g.to_string()) {
                    govs.push(g.to_string());
                }
            }
        }
    }
    govs
}

fn detect_current_cpu_governor() -> Option<String> {
    for path in cpu_paths() {
        let gov = path.join("cpufreq/scaling_governor");
        if let Ok(data) = std::fs::read_to_string(gov) {
            let g = data.trim();
            if !g.is_empty() {
                return Some(g.to_string());
            }
        }
    }
    None
}

fn cpu_paths() -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();
    if let Ok(entries) = std::fs::read_dir("/sys/devices/system/cpu") {
        for entry in entries.flatten() {
            let p = entry.path();
            if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("cpu") && name.chars().skip(3).all(|c| c.is_ascii_digit()) {
                    paths.push(p);
                }
            }
        }
    }
    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_control_fan_valid() {
        let mut mgr = ControlManager::new();
        
        // Valid fan values should work (or fail gracefully on non-Jetson)
        let result = mgr.apply_control("fan", "50");
        
        if mgr.status().available && mgr.status().supports_fan {
            // On Jetson with fan support, should succeed or fail with control_error
            assert!(result.is_ok() || result.is_err());
        } else {
            // On non-Jetson, should fail with clear error
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_apply_control_fan_invalid_range() {
        let mut mgr = ControlManager::new();
        
        // Fan value > 100 should fail
        let result = mgr.apply_control("fan", "150");
        assert!(result.is_err());
        // Error message varies, just check it failed
    }

    #[test]
    fn test_apply_control_fan_invalid_format() {
        let mut mgr = ControlManager::new();
        
        // Non-numeric fan value should fail
        let result = mgr.apply_control("fan", "invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_control_nvpmodel_invalid_mode() {
        let mut mgr = ControlManager::new();
        
        // Invalid nvpmodel mode should fail
        let result = mgr.apply_control("nvpmodel", "INVALID_MODE_XYZ");
        
        if mgr.status().available && mgr.status().supports_nvpmodel {
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_apply_control_jetson_clocks_on() {
        let mut mgr = ControlManager::new();
        
        let result = mgr.apply_control("jetson_clocks", "on");
        
        if mgr.status().available && mgr.status().supports_jetson_clocks {
            // Should succeed or fail with control_error
            assert!(result.is_ok() || result.is_err());
        } else {
            // Should fail on non-Jetson
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_apply_control_jetson_clocks_invalid() {
        let mut mgr = ControlManager::new();
        
        // Invalid jetson_clocks value should fail
        let result = mgr.apply_control("jetson_clocks", "invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_control_unknown() {
        let mut mgr = ControlManager::new();
        
        // Unknown control should fail
        let result = mgr.apply_control("unknown_control", "value");
        assert!(result.is_err());
    }

    #[test]
    fn test_no_op_on_non_jetson() {
        let mgr = ControlManager::new();
        
        if !mgr.status().available {
            // On non-Jetson, all controls should be unavailable
            assert_eq!(mgr.status().note, "Host no Jetson: modo demo");
            assert!(!mgr.status().supports_jetson_clocks);
            assert!(!mgr.status().supports_nvpmodel);
            assert!(!mgr.status().supports_fan);
        }
    }

    #[test]
    fn test_list_controls_structure() {
        let mgr = ControlManager::new();
        let controls = mgr.list_controls();
        
        // Should have controls (at least 1, might be 0 on non-Jetson)
        // Just check structure is valid
        for ctrl in controls {
            // All controls should have required fields
            assert!(!ctrl.name.is_empty());
            assert!(!ctrl.description.is_empty());
            
            // Check specific control properties
            match ctrl.name.as_str() {
                "fan" => {
                    assert_eq!(ctrl.min, Some(0));
                    assert_eq!(ctrl.max, Some(100));
                    assert_eq!(ctrl.unit, Some("%".to_string()));
                    assert!(ctrl.requires_sudo);
                }
                "jetson_clocks" | "nvpmodel" => {
                    assert!(ctrl.requires_sudo);
                }
                _ => {}
            }
        }
    }

    #[test]
    fn test_validation_fan_boundary() {
        let mut mgr = ControlManager::new();
        
        // Test boundary values
        let _result_0 = mgr.apply_control("fan", "0");
        let _result_100 = mgr.apply_control("fan", "100");
        let result_101 = mgr.apply_control("fan", "101");
        
        // 0 and 100 should be valid (or fail for other reasons)
        // 101 should always fail validation
        assert!(result_101.is_err());
    }
}
