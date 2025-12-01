use crate::hardware::JetsonHardware;
use crate::health::DaemonHealth;
use crate::parser::TegraStats;
use serde::{Deserialize, Serialize};

/// Request types for client-daemon communication.
/// Supports both JSON and CBOR serialization (auto-detected by daemon).
#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub enum Request {
    /// Get current stats snapshot
    GetStats,
    /// Get hardware metadata (model, L4T version, capabilities)
    GetMeta,
    /// List available controls with their current state
    ListControls,
    /// Get daemon health and telemetry
    GetHealth,
    /// Set a control value
    /// - `control`: control name (e.g., "fan", "nvpmodel", "jetson_clocks")
    /// - `value`: new value (e.g., "80", "MAXN", "on")
    /// - `token`: optional auth token (set via JETSONSCOPE_AUTH_TOKEN / TEGRA_AUTH_TOKEN env var)
    SetControl {
        control: String,
        value: String,
        token: Option<String>,
    },
}

/// Response types from daemon to client.
/// Always matches the request type or returns Error.
#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub enum Response {
    /// Stats response (for GetStats)
    Stats {
        source: String,
        data: Option<TegraStats>,
    },
    /// Hardware metadata (for GetMeta)
    Meta(JetsonHardware),
    /// List of controls (for ListControls)
    Controls(Vec<ControlInfo>),
    /// Daemon health (for GetHealth)
    Health(DaemonHealth),
    /// Control state after successful SetControl
    ControlState(ControlInfo),
    /// Error response with structured error info
    Error(ErrorInfo),
}

/// Detailed control information including capabilities and current state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlInfo {
    /// Control name (e.g., "fan", "nvpmodel")
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Current value
    pub value: String,
    /// Available options (e.g., ["on", "off"] or ["MAXN", "15W"])
    pub options: Vec<String>,
    /// Whether control is read-only
    pub readonly: bool,
    /// Minimum value (for numeric controls like fan)
    pub min: Option<u32>,
    /// Maximum value (for numeric controls like fan)
    pub max: Option<u32>,
    /// Step size (for numeric controls)
    pub step: Option<u32>,
    /// Whether control requires sudo/root
    pub requires_sudo: bool,
    /// Whether control is supported on this hardware
    pub supported: bool,
    /// Unit of measurement (e.g., "%", "MHz")
    pub unit: Option<String>,
}

/// Structured error information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ErrorInfo {
    /// Error code (e.g., "auth_failed", "control_error", "lock_error")
    pub code: String,
    /// Human-readable error message
    pub message: String,
}
