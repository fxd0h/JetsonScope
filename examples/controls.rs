use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use jetsonscope::protocol::{Request, Response};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let use_cbor = std::env::var("JETSONSCOPE_PROTO")
        .or_else(|_| std::env::var("TEGRA_PROTO"))
        .map(|v| v.to_ascii_lowercase() == "cbor")
        .unwrap_or(false);

    let req = if args.len() >= 4 && args[1] == "set" {
        Request::SetControl {
            control: args[2].clone(),
            value: args[3].clone(),
            token: std::env::var("JETSONSCOPE_AUTH_TOKEN")
                .ok()
                .or_else(|| std::env::var("TEGRA_AUTH_TOKEN").ok()),
        }
    } else {
        Request::ListControls
    };

    let path = socket_path();
    println!("Connecting to socket: {}", path.display());
    let mut stream = UnixStream::connect(&path)?;

    if use_cbor {
        stream.write_all(&serde_cbor::to_vec(&req)?)?;
    } else {
        stream.write_all(serde_json::to_string(&req)?.as_bytes())?;
    }

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf)?;

    let resp: Response = if use_cbor {
        serde_cbor::from_slice(&buf)?
    } else {
        serde_json::from_slice(&buf)?
    };

    match resp {
        Response::Controls(ctrls) => {
            println!("Controls:");
            for c in ctrls {
                println!(
                    "  {} = {} (supported: {}, requires_sudo: {}) options: {:?}, range: {:?}-{:?}",
                    c.name, c.value, c.supported, c.requires_sudo, c.options, c.min, c.max
                );
            }
        }
        Response::ControlState(c) => {
            println!("Updated: {} = {}", c.name, c.value);
        }
        Response::Error(err) => {
            eprintln!("Error [{}]: {}", err.code, err.message);
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
