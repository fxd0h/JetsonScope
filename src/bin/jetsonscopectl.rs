use std::env;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use jetsonscope::protocol::{Request, Response};

fn resolve_socket_path() -> PathBuf {
    // Prefer new env var, fall back to legacy, then defaults with legacy compatibility.
    let sock = env::var("JETSONSCOPE_SOCKET_PATH")
        .or_else(|_| env::var("TEGRA_SOCKET_PATH"))
        .unwrap_or_else(|_| "/tmp/jetsonscope.sock".to_string());
    let candidate = PathBuf::from(sock.clone());
    if candidate.exists() {
        return candidate;
    }
    let legacy = PathBuf::from("/tmp/tegrastats.sock");
    if legacy.exists() {
        return legacy;
    }
    candidate
}

fn use_cbor() -> bool {
    env::var("JETSONSCOPE_PROTO")
        .or_else(|_| env::var("TEGRA_PROTO"))
        .map(|v| v.to_ascii_lowercase() == "cbor")
        .unwrap_or(false)
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("stats");

    let req = match cmd {
        "meta" => Request::GetMeta,
        "list" => Request::ListControls,
        "set" => {
            if args.len() < 4 {
                anyhow::bail!("Usage: jetsonscopectl set <control> <value>");
            }
            Request::SetControl {
                control: args[2].clone(),
                value: args[3].clone(),
                token: env::var("TEGRA_AUTH_TOKEN")
                    .ok()
                    .or_else(|| env::var("JETSONSCOPE_AUTH_TOKEN").ok()),
            }
        }
        _ => Request::GetStats,
    };

    let path = resolve_socket_path();
    if !path.exists() {
        anyhow::bail!(format!("Socket not found: {}", path.display()));
    }

    let mut stream = UnixStream::connect(&path)?;
    let use_cbor = use_cbor();

    if use_cbor {
        let bytes = serde_cbor::to_vec(&req)?;
        stream.write_all(&bytes)?;
    } else {
        let json_req = serde_json::to_string(&req)?;
        stream.write_all(json_req.as_bytes())?;
    }

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf)?;

    let resp: Response = if use_cbor {
        serde_cbor::from_slice(&buf)?
    } else {
        serde_json::from_slice(&buf)?
    };

    match resp {
        Response::Stats { source, data } => {
            println!("Source: {}", source);
            if let Some(stats) = data {
                println!("Timestamp: {:?}", stats.timestamp);
                println!("RAM: {:?}", stats.ram);
                println!("SWAP: {:?}", stats.swap);
                println!("CPU cores: {}", stats.cpus.len());
                if let Some(gpu) = stats.gpu_usage() {
                    println!("GPU: {}%", gpu);
                }
            } else {
                println!("No stats available");
            }
        }
        Response::Meta(hw) => {
            println!("Hardware Info:");
            println!("  Model: {}", hw.model);
            println!("  SoC: {}", hw.soc);
            println!("  L4T: {}", hw.l4t_version);
            println!("  JetPack: {}", hw.jetpack_version);
            println!("  Is Jetson: {}", hw.is_jetson);
        }
        Response::Controls(controls) => {
            println!("Available Controls:");
            for ctrl in controls {
                println!("  {} = {} ({})", ctrl.name, ctrl.value, ctrl.description);
                if !ctrl.supported {
                    println!("    [NOT SUPPORTED]");
                }
            }
        }
        Response::ControlState(ctrl) => {
            println!("Control Updated:");
            println!("  {} = {}", ctrl.name, ctrl.value);
        }
        Response::Health(health) => {
            println!("Daemon Health:");
            println!("  Uptime (s): {}", health.uptime_secs);
            println!("  Total requests: {}", health.total_requests);
            println!("  Errors: {}", health.errors);
            println!("  Connected clients: {}", health.connected_clients);
            println!("  Stats collected: {}", health.stats_collected);
            if let Some(err) = health.last_error {
                println!("  Last error: {}", err);
            }
        }
        Response::Error(err) => {
            eprintln!("Error [{}]: {}", err.code, err.message);
            std::process::exit(1);
        }
    }

    Ok(())
}
