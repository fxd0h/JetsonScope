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

    let req = Request::GetMeta;
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
        Response::Meta(hw) => {
            println!("Model: {}", hw.model);
            println!("SoC: {}", hw.soc);
            println!("L4T: {}", hw.l4t_version);
            println!("JetPack: {}", hw.jetpack_version);
            println!("Is Jetson: {}", hw.is_jetson);
            println!("NVPModel modes: {:?}", hw.nvpmodel_modes);
            println!("Sensors: {:?}", hw.sensors);
        }
        other => println!("Unexpected response: {:?}", other),
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
