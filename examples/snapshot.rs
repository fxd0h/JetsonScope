use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use jetsonscope::protocol::{Request, Response};

fn main() -> anyhow::Result<()> {
    let path = socket_path();
    let mut stream = UnixStream::connect(&path)?;

    let req = Request::GetStats;
    let json = serde_json::to_string(&req)?;
    stream.write_all(json.as_bytes())?;

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf)?;
    let resp: Response = serde_json::from_slice(&buf)?;

    let out = match resp {
        Response::Stats { data, .. } => serde_json::to_string_pretty(&data)?,
        other => serde_json::to_string_pretty(&other)?,
    };

    let mut file = File::create("snapshot.json")?;
    file.write_all(out.as_bytes())?;
    println!("Wrote snapshot.json");
    Ok(())
}

fn socket_path() -> PathBuf {
    std::env::var("JETSONSCOPE_SOCKET_PATH")
        .or_else(|_| std::env::var("TEGRA_SOCKET_PATH"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/jetsonscope.sock"))
}
