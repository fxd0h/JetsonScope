use jetsonscope::protocol::{Request, Response};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let path = socket_path();
    let mut stream = UnixStream::connect(&path)?;
    let req = Request::GetMeta;
    let json = serde_json::to_string(&req)?;
    stream.write_all(json.as_bytes())?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf)?;
    let resp: Response = serde_json::from_slice(&buf)?;
    match resp {
        Response::Meta(meta) => {
            println!("Model: {}", meta.model);
            println!("SoC: {}", meta.soc);
            println!("L4T: {}", meta.l4t_version);
            println!("Jetpack: {}", meta.jetpack_version);
            println!("CUDA arch: {}", meta.cuda_arch);
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
