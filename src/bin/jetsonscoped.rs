use std::fs;
use std::io::{Cursor, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use jetsonscope::collector::{start_collector, CollectorMessage, CollectorMode};
use jetsonscope::control::ControlManager;
use jetsonscope::health::HealthTracker;
use jetsonscope::hardware::JetsonHardware;
use jetsonscope::metrics_auth;
use jetsonscope::parser::TegraStats;
use jetsonscope::protocol::{ErrorInfo, Request, Response};
use jetsonscope::processes::ProcessMonitor;
use tiny_http::{Header, Response as HttpResponse, Server};

fn socket_path() -> String {
    std::env::var("JETSONSCOPE_SOCKET_PATH")
        .or_else(|_| std::env::var("TEGRA_SOCKET_PATH"))
        .unwrap_or_else(|_| "/tmp/jetsonscope.sock".to_string())
}

fn main() -> anyhow::Result<()> {
    let socket_path = socket_path();
    if Path::new(&socket_path).exists() {
        fs::remove_file(&socket_path)?;
    }
    let listener = UnixListener::bind(&socket_path)?;

    let collector = start_collector(CollectorMode::AutoCommand);
    let latest_stats: Arc<Mutex<Option<TegraStats>>> = Arc::new(Mutex::new(None));
    let source_label: Arc<Mutex<String>> = Arc::new(Mutex::new(String::from("initializing")));
    let control = Arc::new(Mutex::new(ControlManager::new()));
    let hardware = Arc::new(JetsonHardware::detect());
    let health = Arc::new(Mutex::new(HealthTracker::new()));

    // Telemetry: file logging
    if let Some(cfg) = TelemetryConfig::from_env() {
        spawn_telemetry_logger(cfg, health.clone());
    }
    // Metrics/Debug HTTP
    if let Ok(addr) = std::env::var("JETSONSCOPE_HTTP_ADDR") {
        spawn_http_metrics(addr, health.clone(), latest_stats.clone(), control.clone());
    }

    // Thread to receive stats from collector
    {
        let latest_stats = Arc::clone(&latest_stats);
        let source_label = Arc::clone(&source_label);
        let health = Arc::clone(&health);
        thread::spawn(move || {
            for msg in collector.rx.iter() {
                match msg {
                    CollectorMessage::Stats(s) => {
                        if let Ok(mut guard) = latest_stats.lock() {
                            *guard = Some(s);
                        }
                        if let Ok(mut h) = health.lock() {
                            h.record_stats_collection();
                        }
                    }
                    CollectorMessage::SourceLabel(label) => {
                        if let Ok(mut guard) = source_label.lock() {
                            *guard = label;
                        }
                    }
                    CollectorMessage::Error(_) => {}
                }
            }
        });
    }

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let stats = latest_stats.clone();
                let label = source_label.clone();
                let control = control.clone();
                let hw = hardware.clone();
                let health = health.clone();
                thread::spawn(move || {
                    handle_client(stream, stats, label, control, hw, health);
                });
            }
            Err(err) => eprintln!("Error accepting client: {err}"),
        }
    }

    Ok(())
}

#[derive(Clone)]
struct TelemetryConfig {
    path: PathBuf,
    interval: Duration,
}

impl TelemetryConfig {
    fn from_env() -> Option<Self> {
        let path = std::env::var("JETSONSCOPE_TELEMETRY_LOG").ok()?;
        let interval_secs = std::env::var("JETSONSCOPE_TELEMETRY_INTERVAL")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(30);
        Some(TelemetryConfig {
            path: PathBuf::from(path),
            interval: Duration::from_secs(interval_secs),
        })
    }
}

fn spawn_telemetry_logger(cfg: TelemetryConfig, health: Arc<Mutex<HealthTracker>>) {
    thread::spawn(move || loop {
        thread::sleep(cfg.interval);
        if let Ok(h) = health.lock() {
            let snapshot = h.get_health(0);
            if let Ok(json) = serde_json::to_string(&snapshot) {
                let _ = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&cfg.path)
                    .and_then(|mut f| {
                        use std::io::Write;
                        writeln!(f, "{}", json)
                    });
            }
        }
    });
}

fn handle_client(
    mut stream: UnixStream,
    stats: Arc<Mutex<Option<TegraStats>>>,
    label: Arc<Mutex<String>>,
    control: Arc<Mutex<ControlManager>>,
    hardware: Arc<JetsonHardware>,
    health: Arc<Mutex<HealthTracker>>,
) {
    let mut buf = Vec::new();
    let _ = stream.read_to_end(&mut buf);

    if let Ok(mut h) = health.lock() {
        h.record_request();
    }

    let (req, respond_cbor) = match serde_json::from_slice::<Request>(&buf) {
        Ok(r) => (r, false),
        Err(_) => match serde_cbor::from_slice::<Request>(&buf) {
            Ok(r) => (r, true),
            Err(_) => (Request::GetStats, false),
        },
    };

    let response = match req {
        Request::GetStats => {
            let s = stats.lock().ok().and_then(|g| g.clone());
            let l = label.lock().ok().map(|g| g.clone()).unwrap_or_default();
            Response::Stats { source: l, data: s }
        }
        Request::GetHealth => {
            let h = health
                .lock()
                .map(|hh| hh.get_health(0))
                .unwrap_or_else(|_| HealthTracker::new().get_health(0));
            Response::Health(h)
        }
        Request::GetMeta => Response::Meta((*hardware).clone()),
        Request::ListControls => match control.lock() {
            Ok(ctrl) => Response::Controls(ctrl.list_controls()),
            Err(_) => Response::Error(ErrorInfo {
                code: "lock_error".to_string(),
                message: "Lock error".to_string(),
            }),
        },
        Request::SetControl {
            control: name,
            value,
            token,
        } => {
            if !auth_ok(token) {
                let err = ErrorInfo {
                    code: "auth_failed".to_string(),
                    message: "Auth failed (set JETSONSCOPE_AUTH_TOKEN)".to_string(),
                };
                record_error(&health, &err.message);
                Response::Error(err)
            } else if let Ok(mut ctrl) = control.lock() {
                let mut err = None;
                match name.as_str() {
                    "jetson_clocks" => ctrl.toggle_jetson_clocks(),
                    "nvpmodel" => ctrl.set_nvpmodel_mode(Some(value)),
                    "fan" => {
                        if let Ok(p) = value.parse::<u8>() {
                            ctrl.set_fan(p);
                        } else {
                            err = Some("Invalid fan value (0-100)".to_string());
                        }
                    }
                    "cpu_governor" => {
                        if let Err(e) = ctrl.set_cpu_governor(&value) {
                            err = Some(e.to_string());
                        }
                    }
                    _ => err = Some("Unknown control".to_string()),
                }

                if let Some(e) = err {
                    let error_info = ErrorInfo {
                        code: "invalid_control".to_string(),
                        message: e,
                    };
                    record_error(&health, &error_info.message);
                    Response::Error(error_info)
                } else if let Some(last_err) = &ctrl.status().last_error {
                    let error_info = ErrorInfo {
                        code: "control_error".to_string(),
                        message: last_err.clone(),
                    };
                    record_error(&health, &error_info.message);
                    Response::Error(error_info)
                } else {
                    Response::ControlState(ctrl.control_info(&name))
                }
            } else {
                let err = ErrorInfo {
                    code: "lock_error".to_string(),
                    message: "Lock error".to_string(),
                };
                record_error(&health, &err.message);
                Response::Error(err)
            }
        }
    };

    write_response(&mut stream, response, respond_cbor);
}

fn write_response(stream: &mut UnixStream, resp: Response, as_cbor: bool) {
    if as_cbor {
        if let Ok(bytes) = serde_cbor::to_vec(&resp) {
            let _ = stream.write_all(&bytes);
            return;
        }
    }
    let json = serde_json::to_string(&resp).unwrap_or_else(|_| "{}".to_string());
    let _ = stream.write_all(json.as_bytes());
}

fn auth_ok(token: Option<String>) -> bool {
    if let Ok(expected) = std::env::var("JETSONSCOPE_AUTH_TOKEN")
        .or_else(|_| std::env::var("TEGRA_AUTH_TOKEN"))
    {
        if expected.is_empty() {
            return true;
        }
        token.map(|t| t == expected).unwrap_or(false)
    } else {
        true
    }
}

fn record_error(health: &Arc<Mutex<HealthTracker>>, message: &str) {
    if let Ok(mut h) = health.lock() {
        h.record_error(message.to_string());
    }
}

// HTTP metrics/debug
fn spawn_http_metrics(
    addr: String,
    health: Arc<Mutex<HealthTracker>>,
    stats: Arc<Mutex<Option<TegraStats>>>,
    control: Arc<Mutex<ControlManager>>,
) {
    thread::spawn(move || {
        if let Ok(server) = Server::http(&addr) {
            for request in server.incoming_requests() {
                let path = request.url().to_string();
                let resp = handle_http_request(&request, &path, &health, &stats, &control)
                    .unwrap_or_else(|| HttpResponse::from_string("not found").with_status_code(404));
                let _ = request.respond(resp);
            }
        }
    });
}

fn handle_http_request(
    request: &tiny_http::Request,
    path: &str,
    health: &Arc<Mutex<HealthTracker>>,
    stats: &Arc<Mutex<Option<TegraStats>>>,
    control: &Arc<Mutex<ControlManager>>,
) -> Option<HttpResponse<Cursor<Vec<u8>>>> {
    if path.starts_with("/metrics") {
        if !metrics_auth::authorize_request(request, "JETSONSCOPE_METRICS_TOKEN") {
            return Some(HttpResponse::from_string("unauthorized").with_status_code(401));
        }
        let metrics = build_metrics(health, stats, control);
        let resp = HttpResponse::from_string(metrics)
            .with_status_code(200)
            .with_header(
                Header::from_bytes(b"Content-Type", b"text/plain; version=0.0.4").unwrap(),
            );
        return Some(resp);
    }

    if path.starts_with("/debug") {
        if !metrics_auth::authorize_request(request, "JETSONSCOPE_DEBUG_TOKEN") {
            return Some(HttpResponse::from_string("unauthorized").with_status_code(401));
        }
        if path.starts_with("/debug/processes") {
            let body = debug_processes();
            let resp = HttpResponse::from_string(body)
                .with_status_code(200)
                .with_header(Header::from_bytes(b"Content-Type", b"application/json").unwrap());
            return Some(resp);
        }
        if path.starts_with("/debug/snapshot") {
            let body = debug_snapshot(health, stats, control);
            let resp = HttpResponse::from_string(body)
                .with_status_code(200)
                .with_header(Header::from_bytes(b"Content-Type", b"application/json").unwrap());
            return Some(resp);
        }
    }
    None
}

fn debug_processes() -> String {
    let mut mon = ProcessMonitor::new();
    let top = mon.top_processes(15, false);
    serde_json::to_string(&top).unwrap_or_else(|_| "[]".to_string())
}

fn debug_snapshot(
    health: &Arc<Mutex<HealthTracker>>,
    stats: &Arc<Mutex<Option<TegraStats>>>,
    control: &Arc<Mutex<ControlManager>>,
) -> String {
    #[derive(serde::Serialize)]
    struct Snapshot {
        health: Option<jetsonscope::health::DaemonHealth>,
        stats: Option<TegraStats>,
        control: jetsonscope::control::ControlStatus,
    }

    let h = health.lock().ok().map(|hh| hh.get_health(0));
    let s = stats.lock().ok().and_then(|ss| ss.clone());
    let ctrl = control
        .lock()
        .ok()
        .map(|c| c.status_cloned())
        .unwrap_or_else(|| jetsonscope::control::ControlStatus {
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
            note: "unavailable".into(),
            last_error: None,
        });

    serde_json::to_string(&Snapshot { health: h, stats: s, control: ctrl })
        .unwrap_or_else(|_| "{}".to_string())
}

fn build_metrics(
    health: &Arc<Mutex<HealthTracker>>,
    stats: &Arc<Mutex<Option<TegraStats>>>,
    control: &Arc<Mutex<ControlManager>>,
) -> String {
    let mut out = String::new();
    if let Ok(h) = health.lock() {
        let snap = h.get_health(0);
        out.push_str(&format!(
            concat!(
                "# HELP jetsonscope_uptime_seconds Daemon uptime in seconds\n",
                "# TYPE jetsonscope_uptime_seconds gauge\n",
                "jetsonscope_uptime_seconds {}\n",
                "# HELP jetsonscope_requests_total Total requests handled\n",
                "# TYPE jetsonscope_requests_total counter\n",
                "jetsonscope_requests_total {}\n",
                "# HELP jetsonscope_errors_total Total errors\n",
                "# TYPE jetsonscope_errors_total counter\n",
                "jetsonscope_errors_total {}\n",
                "# HELP jetsonscope_stats_collected_total Total stats collected\n",
                "# TYPE jetsonscope_stats_collected_total counter\n",
                "jetsonscope_stats_collected_total {}\n",
                "# HELP jetsonscope_connected_clients Connected clients (observed)\n",
                "# TYPE jetsonscope_connected_clients gauge\n",
                "jetsonscope_connected_clients {}\n"
            ),
            snap.uptime_secs,
            snap.total_requests,
            snap.errors,
            snap.stats_collected,
            snap.connected_clients
        ));
    }

    if let Ok(snap) = stats.lock() {
        if let Some(s) = snap.as_ref() {
            // RAM/SWAP
            if let Some(ram) = &s.ram {
                out.push_str("# HELP jetsonscope_ram_bytes_total RAM total bytes\n");
                out.push_str("# TYPE jetsonscope_ram_bytes_total gauge\n");
                out.push_str(&format!("jetsonscope_ram_bytes_total {}\n", ram.total_bytes));
                out.push_str("# HELP jetsonscope_ram_bytes_used RAM used bytes\n");
                out.push_str("# TYPE jetsonscope_ram_bytes_used gauge\n");
                out.push_str(&format!("jetsonscope_ram_bytes_used {}\n", ram.used_bytes));
                if let Some(lfb) = &ram.largest_free_block {
                    match lfb {
                        jetsonscope::parser::LargestFreeBlock::Blocks { count, size_bytes } => {
                            out.push_str("# HELP jetsonscope_ram_lfb_blocks Largest free blocks count\n");
                            out.push_str("# TYPE jetsonscope_ram_lfb_blocks gauge\n");
                            out.push_str(&format!("jetsonscope_ram_lfb_blocks {}\n", count));
                            out.push_str("# HELP jetsonscope_ram_lfb_block_size_bytes LFB block size bytes\n");
                            out.push_str("# TYPE jetsonscope_ram_lfb_block_size_bytes gauge\n");
                            out.push_str(&format!("jetsonscope_ram_lfb_block_size_bytes {}\n", size_bytes));
                        }
                        jetsonscope::parser::LargestFreeBlock::Size { size_bytes } => {
                            out.push_str("# HELP jetsonscope_ram_lfb_size_bytes Largest free block size bytes\n");
                            out.push_str("# TYPE jetsonscope_ram_lfb_size_bytes gauge\n");
                            out.push_str(&format!("jetsonscope_ram_lfb_size_bytes {}\n", size_bytes));
                        }
                    }
                }
            }
            if let Some(sw) = &s.swap {
                out.push_str("# HELP jetsonscope_swap_bytes_total SWAP total bytes\n");
                out.push_str("# TYPE jetsonscope_swap_bytes_total gauge\n");
                out.push_str(&format!("jetsonscope_swap_bytes_total {}\n", sw.total_bytes));
                out.push_str("# HELP jetsonscope_swap_bytes_used SWAP used bytes\n");
                out.push_str("# TYPE jetsonscope_swap_bytes_used gauge\n");
                out.push_str(&format!("jetsonscope_swap_bytes_used {}\n", sw.used_bytes));
            }

            // CPU
            out.push_str("# HELP jetsonscope_cpu_core_load_percent CPU core load percent\n");
            out.push_str("# TYPE jetsonscope_cpu_core_load_percent gauge\n");
            for (idx, core) in s.cpus.iter().enumerate() {
                if let Some(load) = core.load_percent {
                    out.push_str(&format!(
                        "jetsonscope_cpu_core_load_percent{{core=\"{}\"}} {}\n",
                        idx, load
                    ));
                }
                if let Some(freq) = core.freq_mhz {
                    out.push_str(
                        "# HELP jetsonscope_cpu_core_freq_mhz CPU core frequency MHz\n# TYPE jetsonscope_cpu_core_freq_mhz gauge\n"
                    );
                    out.push_str(&format!(
                        "jetsonscope_cpu_core_freq_mhz{{core=\"{}\"}} {}\n",
                        idx, freq
                    ));
                }
            }

            // Engines (GPU, etc.)
            out.push_str("# HELP jetsonscope_engine_usage_percent Engine usage percent\n");
            out.push_str("# TYPE jetsonscope_engine_usage_percent gauge\n");
            for (name, eng) in s.engines.iter() {
                if let Some(u) = eng.usage_percent {
                    out.push_str(&format!(
                        "jetsonscope_engine_usage_percent{{engine=\"{}\"}} {}\n",
                        name, u
                    ));
                }
                if let Some(f) = eng.freq_mhz {
                    out.push_str(
                        "# HELP jetsonscope_engine_freq_mhz Engine frequency MHz\n# TYPE jetsonscope_engine_freq_mhz gauge\n"
                    );
                    out.push_str(&format!(
                        "jetsonscope_engine_freq_mhz{{engine=\"{}\"}} {}\n",
                        name, f
                    ));
                }
                if let Some(raw) = eng.raw_value {
                    out.push_str("# HELP jetsonscope_engine_raw_value Engine raw value\n");
                    out.push_str("# TYPE jetsonscope_engine_raw_value gauge\n");
                    out.push_str(&format!(
                        "jetsonscope_engine_raw_value{{engine=\"{}\"}} {}\n",
                        name, raw
                    ));
                }
            }

            // Temperatures
            if !s.temps.is_empty() {
                out.push_str("# HELP jetsonscope_temp_celsius Sensor temperature in Celsius\n");
                out.push_str("# TYPE jetsonscope_temp_celsius gauge\n");
                for (sensor, temp) in s.temps.iter() {
                    out.push_str(&format!(
                        "jetsonscope_temp_celsius{{sensor=\"{}\"}} {}\n",
                        sensor, temp
                    ));
                }
            }

            // Power rails
            if !s.power.is_empty() {
                out.push_str("# HELP jetsonscope_power_mw_current Power rail current mW\n");
                out.push_str("# TYPE jetsonscope_power_mw_current gauge\n");
                out.push_str("# HELP jetsonscope_power_mw_average Power rail average mW\n");
                out.push_str("# TYPE jetsonscope_power_mw_average gauge\n");
                for (rail, val) in s.power.iter() {
                    out.push_str(&format!(
                        "jetsonscope_power_mw_current{{rail=\"{}\"}} {}\n",
                        rail, val.current_mw
                    ));
                    out.push_str(&format!(
                        "jetsonscope_power_mw_average{{rail=\"{}\"}} {}\n",
                        rail, val.average_mw
                    ));
                }
            }

            // IRAM
            if let Some(iram) = &s.iram {
                out.push_str("# HELP jetsonscope_iram_bytes_total IRAM total bytes\n");
                out.push_str("# TYPE jetsonscope_iram_bytes_total gauge\n");
                out.push_str(&format!("jetsonscope_iram_bytes_total {}\n", iram.total_bytes));
                out.push_str("# HELP jetsonscope_iram_bytes_used IRAM used bytes\n");
                out.push_str("# TYPE jetsonscope_iram_bytes_used gauge\n");
                out.push_str(&format!("jetsonscope_iram_bytes_used {}\n", iram.used_bytes));
                if let Some(lfb) = iram.lfb_bytes {
                    out.push_str("# HELP jetsonscope_iram_lfb_bytes IRAM largest free block bytes\n");
                    out.push_str("# TYPE jetsonscope_iram_lfb_bytes gauge\n");
                    out.push_str(&format!("jetsonscope_iram_lfb_bytes {}\n", lfb));
                }
            }

            // MTS
            if let Some(mts) = &s.mts {
                out.push_str("# HELP jetsonscope_mts_usage_fg_percent MTS FG usage percent\n");
                out.push_str("# TYPE jetsonscope_mts_usage_fg_percent gauge\n");
                out.push_str(&format!("jetsonscope_mts_usage_fg_percent {}\n", mts.fg_percent));
                out.push_str("# HELP jetsonscope_mts_usage_bg_percent MTS BG usage percent\n");
                out.push_str("# TYPE jetsonscope_mts_usage_bg_percent gauge\n");
                out.push_str(&format!("jetsonscope_mts_usage_bg_percent {}\n", mts.bg_percent));
            }
        }
    }

    // Control status
    if let Ok(ctrl) = control.lock() {
        let status = ctrl.status_cloned();
        out.push_str("# HELP jetsonscope_control_supported Control supported flag\n");
        out.push_str("# TYPE jetsonscope_control_supported gauge\n");
        out.push_str(&format!(
            "jetsonscope_control_supported{{control=\"fan\"}} {}\n",
            if status.supports_fan { 1 } else { 0 }
        ));
        out.push_str(&format!(
            "jetsonscope_control_supported{{control=\"nvpmodel\"}} {}\n",
            if status.supports_nvpmodel { 1 } else { 0 }
        ));
        out.push_str(&format!(
            "jetsonscope_control_supported{{control=\"jetson_clocks\"}} {}\n",
            if status.supports_jetson_clocks { 1 } else { 0 }
        ));
        out.push_str(&format!(
            "jetsonscope_control_supported{{control=\"cpu_governor\"}} {}\n",
            if status.supports_cpu_governor { 1 } else { 0 }
        ));
        out.push_str(&format!(
            "jetsonscope_control_supported{{control=\"gpu_governor\"}} {}\n",
            if status.supports_gpu_governor { 1 } else { 0 }
        ));
        out.push_str(&format!(
            "jetsonscope_control_supported{{control=\"gpu_railgate\"}} {}\n",
            if status.supports_gpu_railgate { 1 } else { 0 }
        ));

        if let Some(on) = status.jetson_clocks {
            out.push_str("# HELP jetsonscope_control_jetson_clocks_on Jetson clocks state\n");
            out.push_str("# TYPE jetsonscope_control_jetson_clocks_on gauge\n");
            out.push_str(&format!(
                "jetsonscope_control_jetson_clocks_on {}\n",
                if on { 1 } else { 0 }
            ));
        }
        if let Some(fan) = status.fan {
            if let Some(pct) = parse_percent_value(&fan) {
                out.push_str("# HELP jetsonscope_control_fan_percent Fan setpoint percent\n");
                out.push_str("# TYPE jetsonscope_control_fan_percent gauge\n");
                out.push_str(&format!("jetsonscope_control_fan_percent {}\n", pct));
            }
        }
        if let Some(mode) = status.nvpmodel {
            out.push_str("# HELP jetsonscope_control_nvpmodel_mode Current nvpmodel mode\n");
            out.push_str("# TYPE jetsonscope_control_nvpmodel_mode gauge\n");
            out.push_str(&format!(
                "jetsonscope_control_nvpmodel_mode{{mode=\"{}\"}} 1\n",
                mode
            ));
        }
        if !status.nvpmodel_modes.is_empty() {
            out.push_str("# HELP jetsonscope_control_nvpmodel_supported_modes Nvpmodel modes supported (info)\n");
            out.push_str("# TYPE jetsonscope_control_nvpmodel_supported_modes gauge\n");
            for m in status.nvpmodel_modes {
                out.push_str(&format!(
                    "jetsonscope_control_nvpmodel_supported_modes{{mode=\"{}\"}} 1\n",
                    m
                ));
            }
        }
        if let Some(gov) = status.cpu_governor {
            out.push_str("# HELP jetsonscope_control_cpu_governor Current CPU governor\n");
            out.push_str("# TYPE jetsonscope_control_cpu_governor gauge\n");
            out.push_str(&format!(
                "jetsonscope_control_cpu_governor{{governor=\"{}\"}} 1\n",
                sanitize_label(&gov)
            ));
        }
        if let Some(gov) = status.gpu_governor {
            out.push_str("# HELP jetsonscope_control_gpu_governor Current GPU governor\n");
            out.push_str("# TYPE jetsonscope_control_gpu_governor gauge\n");
            out.push_str(&format!(
                "jetsonscope_control_gpu_governor{{governor=\"{}\"}} 1\n",
                sanitize_label(&gov)
            ));
        }
        if let Some(auto) = status.gpu_railgate {
            out.push_str("# HELP jetsonscope_control_gpu_railgate GPU rail-gating state (auto=1/on=0)\n");
            out.push_str("# TYPE jetsonscope_control_gpu_railgate gauge\n");
            out.push_str(&format!(
                "jetsonscope_control_gpu_railgate {}\n",
                if auto { 1 } else { 0 }
            ));
        }
        if let Some(err) = status.last_error {
            out.push_str("# HELP jetsonscope_control_last_error Last control error (info)\n");
            out.push_str("# TYPE jetsonscope_control_last_error gauge\n");
            out.push_str(&format!(
                "jetsonscope_control_last_error{{message=\"{}\"}} 1\n",
                sanitize_label(&err)
            ));
        }
    }

    out
}

fn parse_percent_value(s: &str) -> Option<f64> {
    let cleaned = s.trim().trim_end_matches('%');
    cleaned.parse::<f64>().ok()
}

fn sanitize_label(s: &str) -> String {
    s.replace('"', "'")
}
