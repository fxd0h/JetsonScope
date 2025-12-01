use crate::parser::{CpuCore, EngineStat, MemoryStat, PowerRail, SizeUnit, SwapStat, TegraStats};
use chrono::Local;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io::Read;
use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

#[derive(Debug)]
pub enum CollectorMessage {
    Stats(TegraStats),
    SourceLabel(String),
    Error(String),
}

pub struct StatsCollector {
    pub rx: Receiver<CollectorMessage>,
}

#[derive(Debug, Clone, Copy)]
pub enum CollectorMode {
    #[allow(dead_code)]
    AutoCommand,   // daemon: socket if present else command/emulator/synthetic
    #[allow(dead_code)]
    PreferSocket,  // prefer socket, otherwise command/emulator/synthetic
    SocketOnly,    // socket else synthetic (no command)
}

pub fn start_collector(mode: CollectorMode) -> StatsCollector {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        spawn_collection_loop(tx, mode);
    });
    StatsCollector { rx }
}

fn spawn_collection_loop(tx: Sender<CollectorMessage>, mode: CollectorMode) {
    let choice = select_source(&mode);
    let _ = tx.send(CollectorMessage::SourceLabel(choice.label.clone()));
    match choice.kind {
        SourceKind::Command(mut cmd) => {
            cmd.stdout(Stdio::piped());
            match cmd.spawn() {
                Ok(mut child) => {
                    if let Some(stdout) = child.stdout.take() {
                        let reader = BufReader::new(stdout);
                        for line in reader.lines().flatten() {
                            if let Ok(stats) = TegraStats::parse(&line) {
                                let _ = tx.send(CollectorMessage::Stats(stats));
                            }
                        }
                    }
                }
                Err(err) => {
                    eprintln!("Failed to start stats source ({:?}): {err}", cmd);
                }
            }
            run_synthetic(&tx);
        }
        SourceKind::Socket(path) => {
            let mut retry_count = 0;
            let max_retries = if matches!(mode, CollectorMode::SocketOnly) { usize::MAX } else { 5 };
            let mut backoff_ms = 1000;

            loop {
                match read_once_from_socket(&path) {
                    Ok(resp) => {
                        if let Some(stats) = resp.stats {
                            let _ = tx.send(CollectorMessage::Stats(stats));
                        }
                        let _ = tx.send(CollectorMessage::SourceLabel(resp.source));
                        retry_count = 0; // Reset on success
                        backoff_ms = 1000;
                    }
                    Err(err) => {
                        let _ = tx.send(CollectorMessage::SourceLabel(format!("socket error: {err}")));
                        let _ = tx.send(CollectorMessage::Error(format!("socket error: {err}")));
                        retry_count += 1;

                        if retry_count >= max_retries {
                            run_synthetic(&tx);
                            return;
                        }

                        thread::sleep(Duration::from_millis(backoff_ms));
                        backoff_ms = (backoff_ms * 2).min(10000); // Exponential backoff, max 10s
                    }
                }
                thread::sleep(Duration::from_millis(1000));
            }
        }
        SourceKind::Synthetic => run_synthetic(&tx),
    }
}

struct SourceChoice {
    kind: SourceKind,
    label: String,
}

enum SourceKind {
    Command(Command),
    Socket(PathBuf),
    Synthetic,
}

fn select_source(mode: &CollectorMode) -> SourceChoice {
    if let Ok(sock_path) = env::var("JETSONSCOPE_SOCKET_PATH")
        .or_else(|_| env::var("TEGRA_SOCKET_PATH"))
    {
        let path = PathBuf::from(sock_path.clone());
        return SourceChoice {
            kind: SourceKind::Socket(path),
            label: format!("socket {sock_path}"),
        };
    }
    let default_sock = PathBuf::from("/tmp/jetsonscope.sock");
    let legacy_sock = PathBuf::from("/tmp/tegrastats.sock");
    if default_sock.exists() {
        return SourceChoice {
            kind: SourceKind::Socket(default_sock.clone()),
            label: "socket /tmp/jetsonscope.sock".to_string(),
        };
    }
    if legacy_sock.exists() {
        return SourceChoice {
            kind: SourceKind::Socket(legacy_sock.clone()),
            label: "socket /tmp/tegrastats.sock (legacy)".to_string(),
        };
    }

    match mode {
        CollectorMode::SocketOnly => SourceChoice {
            kind: SourceKind::Synthetic,
            label: "synthetic (socket missing)".to_string(),
        },
        CollectorMode::PreferSocket => select_source_auto(true),
        CollectorMode::AutoCommand => select_source_auto(false),
    }
}

fn select_source_auto(prefer_socket: bool) -> SourceChoice {
    // Allow overriding the stats command (e.g., path to real tegrastats or a custom emulator)
    if let Ok(raw_cmd) = env::var("JETSONSCOPE_STATS_CMD")
        .or_else(|_| env::var("TEGRASTATS_CMD"))
    {
        let mut parts = raw_cmd.split_whitespace();
        if let Some(program) = parts.next() {
            let mut cmd = Command::new(program);
            for arg in parts {
                cmd.arg(arg);
            }
            return SourceChoice {
                kind: SourceKind::Command(cmd),
                label: format!("custom cmd: {raw_cmd}"),
            };
        }
    }

    if prefer_socket {
        return SourceChoice {
            kind: SourceKind::Synthetic,
            label: "synthetic (socket preferred, none found)".to_string(),
        };
    }

    if should_force_emulator() {
        return SourceChoice {
            kind: SourceKind::Command(emulator_command()),
            label: "python emulator".to_string(),
        };
    }

    if is_jetson() {
        let mut cmd = Command::new("tegrastats");
        cmd.arg("--interval").arg("1000");
        SourceChoice {
            kind: SourceKind::Command(cmd),
            label: "tegrastats real".to_string(),
        }
    } else {
        let cmd = emulator_command();
        SourceChoice {
            kind: SourceKind::Command(cmd),
            label: "python emulator".to_string(),
        }
    }
}

fn should_force_emulator() -> bool {
    matches!(
        env::var("JETSONSCOPE_TUI_MODE")
            .or_else(|_| env::var("TEGRA_TUI_MODE"))
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "emulator" | "fake" | "dummy"
    )
}

fn emulator_command() -> Command {
    let mut cmd = Command::new("python3");
    cmd.arg("../tegrastats_emulator.py")
        .arg("--interval")
        .arg("1000");
    cmd
}

fn is_jetson() -> bool {
    // Quick heuristics: /etc/nv_tegra_release and tegrastats binary present
    if fs::metadata("/etc/nv_tegra_release").is_ok() {
        return true;
    }
    which::which("tegrastats").is_ok()
}

fn run_synthetic(tx: &Sender<CollectorMessage>) {
    let _ = tx.send(CollectorMessage::SourceLabel(
        "synthetic generator".to_string(),
    ));
    loop {
        let stats = synthesize_stats();
        let _ = tx.send(CollectorMessage::Stats(stats));
        thread::sleep(Duration::from_millis(1000));
    }
}

fn synthesize_stats() -> TegraStats {
    let mut rng = rand::thread_rng();
    let cpu_count = 8;
    let mut cpus = Vec::with_capacity(cpu_count);
    for _ in 0..cpu_count {
        let load = rng.gen_range(0..100) as u32;
        let freq = [729, 1036, 1190, 1497][rng.gen_range(0..4)];
        cpus.push(CpuCore {
            load_percent: Some(load),
            freq_mhz: Some(freq),
        });
    }

    let engines = {
        let mut map = std::collections::HashMap::new();
        let gpu = rng.gen_range(0..100) as u32;
        map.insert(
            "GR3D".into(),
            EngineStat {
                usage_percent: Some(gpu),
                freq_mhz: Some(1200),
                raw_value: None,
            },
        );
        map.insert(
            "EMC".into(),
            EngineStat {
                usage_percent: Some(rng.gen_range(0..50)),
                freq_mhz: Some(1866),
                raw_value: None,
            },
        );
        map.insert(
            "NVENC".into(),
            EngineStat {
                usage_percent: None,
                freq_mhz: Some(716),
                raw_value: Some(716),
            },
        );
        map
    };

    let temps = ["CPU", "GPU", "soc0", "soc1", "tj"]
        .iter()
        .map(|name| (name.to_string(), rng.gen_range(35.0..75.0)))
        .collect();

    let power = {
        let mut map = std::collections::HashMap::new();
        map.insert(
            "VDD_IN".into(),
            PowerRail {
                current_mw: rng.gen_range(7000..15000),
                average_mw: rng.gen_range(7000..15000),
            },
        );
        map.insert(
            "VDD_CPU".into(),
            PowerRail {
                current_mw: rng.gen_range(1000..4000),
                average_mw: rng.gen_range(1000..4000),
            },
        );
        map
    };

    let ram_total = SizeUnit::MB.to_bytes(16_000);
    let ram_used = ram_total / 2 + rng.gen_range(0..(ram_total / 4));
    let swap_total = SizeUnit::MB.to_bytes(8_000);
    let swap_used = swap_total / 4 + rng.gen_range(0..(swap_total / 4));

    TegraStats {
        timestamp: Some(Local::now().format("%m-%d-%Y %H:%M:%S").to_string()),
        ram: Some(MemoryStat {
            used_bytes: ram_used,
            total_bytes: ram_total,
            unit: SizeUnit::MB,
            largest_free_block: None,
        }),
        swap: Some(SwapStat {
            used_bytes: swap_used,
            total_bytes: swap_total,
            cached_bytes: Some(swap_total / 8),
            unit: SizeUnit::MB,
        }),
        iram: None,
        mts: None,
        cpus,
        engines,
        temps,
        power,
        raw: String::from("synthetic"),
    }
}

#[derive(Serialize, Deserialize)]
struct SocketResponse {
    source: String,
    stats: Option<TegraStats>,
}

fn read_once_from_socket(path: &PathBuf) -> anyhow::Result<SocketResponse> {
    let mut stream = UnixStream::connect(path)?;
    let mut buf = String::new();
    stream.read_to_string(&mut buf)?;
    let resp: SocketResponse = serde_json::from_str(&buf)?;
    Ok(resp)
}
