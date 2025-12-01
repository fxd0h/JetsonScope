use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use jetsonscope::protocol::{Request, Response};

fn main() -> anyhow::Result<()> {
    let use_cbor = std::env::var("JETSONSCOPE_PROTO")
        .or_else(|_| std::env::var("TEGRA_PROTO"))
        .map(|v| v.to_ascii_lowercase() == "cbor")
        .unwrap_or(false);

    let path = socket_path();
    println!("Connecting to socket: {}", path.display());
    let mut stream = UnixStream::connect(&path)?;

    // Request stats snapshot
    let req = Request::GetStats;
    if use_cbor {
        let bytes = serde_cbor::to_vec(&req)?;
        stream.write_all(&bytes)?;
    } else {
        let json = serde_json::to_string(&req)?;
        stream.write_all(json.as_bytes())?;
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
            println!("Source: {source}");
            if let Some(stats) = data {
                if let Some(ref ram) = stats.ram {
                    println!(
                        "RAM: used {} / total {} (bytes)",
                        ram.used_bytes, ram.total_bytes
                    );
                }
                if let Some(gpu) = stats.gpu_usage() {
                    println!("GPU: {}%", gpu);
                }
                println!("CPU cores: {}", stats.cpus.len());
            } else {
                println!("No stats available");
            }
        }
        other => {
            println!("Unexpected response: {:?}", other);
        }
    }

    Ok(())
}

fn socket_path() -> PathBuf {
    std::env::var("JETSONSCOPE_SOCKET_PATH")
        .or_else(|_| std::env::var("TEGRA_SOCKET_PATH"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let p = PathBuf::from("/tmp/jetsonscope.sock");
            if p.exists() {
                p
            } else {
                PathBuf::from("/tmp/tegrastats.sock")
            }
        })
}
