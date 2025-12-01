use crate::collector::{CollectorMessage, start_collector, CollectorMode};
use crate::control::ControlManager;
use crate::parser::TegraStats;
use std::collections::VecDeque;
use std::sync::mpsc::Receiver;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Dashboard,
    Processes,
    GpuEngines,
    Clocks,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryWindow {
    OneMinute,
    FiveMinutes,
    OneHour,
    SixHours,
    TwentyFourHours,
}

impl HistoryWindow {
    pub fn max_points(&self) -> usize {
        match self {
            HistoryWindow::OneMinute => 60,
            HistoryWindow::FiveMinutes => 300,
            HistoryWindow::OneHour => 360,
            HistoryWindow::SixHours => 360,
            HistoryWindow::TwentyFourHours => 288,
        }
    }

    pub fn duration_secs(&self) -> u64 {
        match self {
            HistoryWindow::OneMinute => 60,
            HistoryWindow::FiveMinutes => 300,
            HistoryWindow::OneHour => 3600,
            HistoryWindow::SixHours => 21600,
            HistoryWindow::TwentyFourHours => 86400,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            HistoryWindow::OneMinute => "1m",
            HistoryWindow::FiveMinutes => "5m",
            HistoryWindow::OneHour => "1h",
            HistoryWindow::SixHours => "6h",
            HistoryWindow::TwentyFourHours => "24h",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            HistoryWindow::OneMinute => HistoryWindow::FiveMinutes,
            HistoryWindow::FiveMinutes => HistoryWindow::OneHour,
            HistoryWindow::OneHour => HistoryWindow::SixHours,
            HistoryWindow::SixHours => HistoryWindow::TwentyFourHours,
            HistoryWindow::TwentyFourHours => HistoryWindow::OneMinute,
        }
    }
}

pub struct App {
    pub stats_history: Vec<TegraStats>,
    pub latest_stats: TegraStats,
    pub rx: Receiver<CollectorMessage>,
    pub tick_count: u64,
    pub source_label: String,
    pub connection_status: String,
    pub last_update_tick: u64,
    pub retry_count: usize,
    pub reconnect_requested: bool,
    pub history: History,
    pub history_window: HistoryWindow,
    pub control: ControlManager,
    pub view_mode: ViewMode,
    pub process_sort_by_mem: bool,
    pub show_help: bool,
}

pub struct History {
    pub ram: VecDeque<(Instant, f64)>,
    pub gpu: VecDeque<(Instant, f64)>,
    pub cpu: VecDeque<(Instant, f64)>,
    #[allow(dead_code)]
    start_time: Instant,
}

impl Default for History {
    fn default() -> Self {
        Self {
            ram: VecDeque::new(),
            gpu: VecDeque::new(),
            cpu: VecDeque::new(),
            start_time: Instant::now(),
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        let collector = start_collector(CollectorMode::SocketOnly);

        Self {
            stats_history: Vec::new(),
            latest_stats: TegraStats::default(),
            rx: collector.rx,
            tick_count: 0,
            source_label: "Conectando...".to_string(),
            connection_status: "conectando".to_string(),
            last_update_tick: 0,
            retry_count: 0,
            reconnect_requested: false,
            history: History::default(),
            history_window: HistoryWindow::OneMinute,
            control: ControlManager::new(),
            view_mode: ViewMode::Dashboard,
            process_sort_by_mem: false,
            show_help: false,
        }
    }

    pub fn cycle_history_window(&mut self) {
        self.history_window = self.history_window.next();
    }

    pub fn request_reconnect(&mut self) {
        self.reconnect_requested = true;
        self.connection_status = "reconectando...".to_string();
        self.retry_count = 0;
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    pub fn toggle_process_sort(&mut self) {
        self.process_sort_by_mem = !self.process_sort_by_mem;
    }

    pub fn cycle_view(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::Dashboard => ViewMode::Processes,
            ViewMode::Processes => ViewMode::GpuEngines,
            ViewMode::GpuEngines => ViewMode::Clocks,
            ViewMode::Clocks => ViewMode::Dashboard,
        };
    }

    pub fn on_tick(&mut self) {
        self.tick_count += 1;

        // Check for new stats
        while let Ok(event) = self.rx.try_recv() {
            match event {
                CollectorMessage::Stats(stats) => {
                    self.latest_stats = stats.clone();
                    self.stats_history.push(stats.clone());
                    self.last_update_tick = self.tick_count;
                    self.retry_count = 0;
                    self.connection_status = "conectado".to_string();
                    
                    // Update history with timestamps
                    let now = Instant::now();
                    let ram_pct = stats.ram.as_ref().map_or(0.0, |r| {
                        if r.total_bytes == 0 { 0.0 } else { r.used_bytes as f64 / r.total_bytes as f64 * 100.0 }
                    });
                    let gpu_pct = stats.gpu_usage().map_or(0.0, |g| g as f64);
                    let cpu_pct = if stats.cpus.is_empty() {
                        0.0
                    } else {
                        let sum: f32 = stats.cpus.iter()
                            .filter_map(|c| c.load_percent)
                            .map(|v| v as f32)
                            .sum();
                        sum as f64 / stats.cpus.len() as f64
                    };
                    
                    self.history.ram.push_back((now, ram_pct));
                    self.history.gpu.push_back((now, gpu_pct));
                    self.history.cpu.push_back((now, cpu_pct));
                    
                    // Trim to max points for current window
                    let max_points = self.history_window.max_points();
                    while self.history.ram.len() > max_points {
                        self.history.ram.pop_front();
                    }
                    while self.history.gpu.len() > max_points {
                        self.history.gpu.pop_front();
                    }
                    while self.history.cpu.len() > max_points {
                        self.history.cpu.pop_front();
                    }
                    
                    if self.stats_history.len() > 100 {
                        self.stats_history.remove(0);
                    }
                }
                CollectorMessage::SourceLabel(label) => {
                    self.source_label = label.clone();
                    if label.contains("synthetic") {
                        self.connection_status = "modo demo (sintético)".to_string();
                    } else if label.contains("socket") {
                        self.connection_status = "conectado (socket)".to_string();
                    } else {
                        self.connection_status = "conectado".to_string();
                    }
                }
                CollectorMessage::Error(err) => {
                    // Parse retry info from error message
                    if err.contains("retry") || err.contains("Retrying") {
                        // Extract retry count if present
                        if let Some(count_str) = err.split('/').next() {
                            if let Some(num) = count_str.split_whitespace().last() {
                                self.retry_count = num.parse().unwrap_or(0);
                            }
                        }
                        self.connection_status = format!("reintentando ({}/5)", self.retry_count);
                    } else if err.contains("Max retries") || err.contains("fallback") {
                        self.connection_status = "offline (max reintentos)".to_string();
                    } else {
                        self.connection_status = format!("error: {}", err);
                    }
                }
            }
        }

        // Timeout detection
        if self.tick_count.saturating_sub(self.last_update_tick) > 50 {
            // ~5s sin datos
            if self.connection_status.starts_with("conectado") {
                self.connection_status = "sin datos (timeout)".to_string();
            }
        }
    }
}
