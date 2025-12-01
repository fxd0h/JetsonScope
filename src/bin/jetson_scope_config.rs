use jetsonscope::protocol::{ControlInfo, Request, Response};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() <= 1 {
        print_usage();
        return Ok(());
    }

    let path = socket_path();
    let command = args[1].as_str();
    match command {
        "list" => list_controls(&path)?,
        "preset" => {
            if args.len() < 3 {
                eprintln!("Usage: jetson_scope_config preset <performance|balanced>");
                return Ok(());
            }
            apply_preset(&path, &args[2])?;
        }
        "set" => {
            if args.len() < 4 {
                eprintln!("Usage: jetson_scope_config set <control> <value>");
                return Ok(());
            }
            set_control(&path, &args[2], &args[3])?;
        }
        _ => print_usage(),
    }

    Ok(())
}

fn print_usage() {
    println!("jetson_scope_config commands:");
    println!("  list                            # list controls");
    println!("  preset performance|balanced     # apply preset");
    println!("  set <control> <value>           # set specific control");
    println!("Controls include: jetson_clocks, nvpmodel, fan, cpu_governor, gpu_governor, gpu_railgate");
}

fn socket_path() -> PathBuf {
    std::env::var("JETSONSCOPE_SOCKET_PATH")
        .or_else(|_| std::env::var("TEGRA_SOCKET_PATH"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/jetsonscope.sock"))
}

fn list_controls(path: &PathBuf) -> anyhow::Result<()> {
    let mut stream = UnixStream::connect(path)?;
    let req = Request::ListControls;
    let json = serde_json::to_string(&req)?;
    stream.write_all(json.as_bytes())?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf)?;
    let resp: Response = serde_json::from_slice(&buf)?;
    match resp {
        Response::Controls(list) => {
            for c in list {
                println!(
                    "- {} (value: {}, options: {:?}, sudo: {})",
                    c.name, c.value, c.options, c.requires_sudo
                );
            }
        }
        other => println!("Unexpected response: {:?}", other),
    }
    Ok(())
}

fn apply_preset(path: &PathBuf, preset: &str) -> anyhow::Result<()> {
    let mut stream = UnixStream::connect(path)?;
    // list controls first
    let req = Request::ListControls;
    let json = serde_json::to_string(&req)?;
    stream.write_all(json.as_bytes())?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf)?;
    let resp: Response = serde_json::from_slice(&buf)?;
    let controls = match resp {
        Response::Controls(list) => list,
        other => {
            eprintln!("Unexpected response: {:?}", other);
            return Ok(());
        }
    };
    match preset {
        "performance" => {
            if has_control(&controls, "jetson_clocks") {
                set_control(path, "jetson_clocks", "on")?;
            }
            if has_control(&controls, "cpu_governor") {
                set_control(path, "cpu_governor", "performance")?;
            }
            if has_control(&controls, "gpu_governor") {
                set_control(path, "gpu_governor", "performance")?;
            }
        }
        "balanced" => {
            if has_control(&controls, "jetson_clocks") {
                set_control(path, "jetson_clocks", "off")?;
            }
            if has_control(&controls, "cpu_governor") {
                set_control(path, "cpu_governor", "ondemand")?;
            }
            if has_control(&controls, "gpu_governor") {
                set_control(path, "gpu_governor", "nvhost_podgov")?;
            }
        }
        other => {
            eprintln!("Unknown preset: {}", other);
        }
    }
    Ok(())
}

fn set_control(path: &PathBuf, name: &str, value: &str) -> anyhow::Result<()> {
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
