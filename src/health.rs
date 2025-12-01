use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Daemon health and telemetry information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonHealth {
    /// Daemon uptime in seconds
    pub uptime_secs: u64,
    /// Total requests processed
    pub total_requests: u64,
    /// Total errors encountered
    pub errors: u64,
    /// Last error message (if any)
    pub last_error: Option<String>,
    /// Number of currently connected clients
    pub connected_clients: usize,
    /// Total stats collected
    pub stats_collected: u64,
}

/// Health tracker for daemon
#[allow(dead_code)]
pub struct HealthTracker {
    start_time: Instant,
    total_requests: u64,
    errors: u64,
    last_error: Option<String>,
    stats_collected: u64,
}

impl Default for HealthTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl HealthTracker {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            total_requests: 0,
            errors: 0,
            last_error: None,
            stats_collected: 0,
        }
    }

    pub fn record_request(&mut self) {
        self.total_requests += 1;
    }

    pub fn record_error(&mut self, error: String) {
        self.errors += 1;
        self.last_error = Some(error);
    }

    pub fn record_stats_collection(&mut self) {
        self.stats_collected += 1;
    }

    pub fn get_health(&self, connected_clients: usize) -> DaemonHealth {
        DaemonHealth {
            uptime_secs: self.start_time.elapsed().as_secs(),
            total_requests: self.total_requests,
            errors: self.errors,
            last_error: self.last_error.clone(),
            connected_clients,
            stats_collected: self.stats_collected,
        }
    }
}
