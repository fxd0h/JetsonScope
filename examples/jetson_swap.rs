use jetsonscope::protocol::{Request, Response};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let path = socket_path();
    let mut stream = UnixStream::connect(&path)?;
    let req = Request::GetStats;
    let json = serde_json::to_string(&req)?;
    stream.write_all(json.as_bytes())?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf)?;
    let resp: Response = serde_json::from_slice(&buf)?;
    match resp {
        Response::Stats { data, .. } => {
            if let Some(stats) = data {
                if let Some(sw) = stats.swap {
                    println!(
                        "SWAP used {} / total {} bytes (cached: {:?})",
                        sw.used_bytes, sw.total_bytes, sw.cached_bytes
                    );
                } else {
                    println!("No SWAP info");
                }
            } else {
                println!("No stats available");
            }
        }
        other => println!("Unexpected response: {:?}", other),
    }
    Ok(())
}

fn socket_path() -> PathBuf {
    std::env::var("JETSONSCOPE_SOCKET_PATH")
        .or_else(|_| std::env::var("TEGRA_SOCKET_PATH"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/jetsonscope.sock"))
}
