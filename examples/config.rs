use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use jetsonscope::protocol::{ControlInfo, Request, Response};

fn main() -> anyhow::Result<()> {
    let path = socket_path();
    let mut stream = UnixStream::connect(&path)?;

    // First, list controls
    send_request(&mut stream, &Request::ListControls)?;
    let controls = read_controls(&mut stream)?;

    println!("Available controls:");
    for c in &controls {
        println!("- {} (options: {:?})", c.name, c.options);
    }

    // Apply a simple preset if provided: CPU gov, nvpmodel, jetson_clocks, fan
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        let preset = args[1].as_str();
        match preset {
            "performance" => {
                set(&path, "jetson_clocks", "on")?;
                if has_control(&controls, "cpu_governor") {
                    set(&path, "cpu_governor", "performance")?;
                }
            }
            "balanced" => {
                set(&path, "jetson_clocks", "off")?;
                if has_control(&controls, "cpu_governor") {
                    set(&path, "cpu_governor", "ondemand")?;
                }
            }
            other => {
                println!("Unknown preset: {}", other);
            }
        }
    }

    Ok(())
}

fn send_request(stream: &mut UnixStream, req: &Request) -> anyhow::Result<()> {
    let json = serde_json::to_string(req)?;
    stream.write_all(json.as_bytes())?;
    Ok(())
}

fn read_controls(stream: &mut UnixStream) -> anyhow::Result<Vec<ControlInfo>> {
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf)?;
    let resp: Response = serde_json::from_slice(&buf)?;
    match resp {
        Response::Controls(list) => Ok(list),
        other => anyhow::bail!("Unexpected response: {:?}", other),
    }
}

fn set(path: &PathBuf, name: &str, value: &str) -> anyhow::Result<()> {
    let token = std::env::var("JETSONSCOPE_AUTH_TOKEN")
        .or_else(|_| std::env::var("TEGRA_AUTH_TOKEN"))
        .ok();
    let mut stream = UnixStream::connect(path)?;
    let req = Request::SetControl {
        control: name.to_string(),
        value: value.to_string(),
        token,
    };
    let json = serde_json::to_string(&req)?;
    stream.write_all(json.as_bytes())?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf)?;
    let resp: Response = serde_json::from_slice(&buf)?;
    println!("set {}={} -> {:?}", name, value, resp);
    Ok(())
}

fn has_control(list: &[ControlInfo], name: &str) -> bool {
    list.iter().any(|c| c.name == name)
}

fn socket_path() -> PathBuf {
    std::env::var("JETSONSCOPE_SOCKET_PATH")
        .or_else(|_| std::env::var("TEGRA_SOCKET_PATH"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/jetsonscope.sock"))
}
