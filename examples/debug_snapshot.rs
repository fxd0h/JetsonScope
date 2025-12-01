use std::io::{Read, Write};
use std::net::TcpStream;

fn main() -> anyhow::Result<()> {
    // Requires JETSONSCOPE_HTTP_ADDR and optional JETSONSCOPE_DEBUG_TOKEN
    let addr = std::env::var("JETSONSCOPE_HTTP_ADDR").unwrap_or_else(|_| "127.0.0.1:9090".into());
    let token = std::env::var("JETSONSCOPE_DEBUG_TOKEN").ok();
    let mut stream = TcpStream::connect(&addr)?;

    let mut req = format!("GET /debug/snapshot HTTP/1.1\r\nHost: {}\r\n", addr);
    if let Some(t) = token {
        req.push_str(&format!("Authorization: Bearer {}\r\n", t));
    }
    req.push_str("\r\n");
    stream.write_all(req.as_bytes())?;

    let mut buf = String::new();
    stream.read_to_string(&mut buf)?;
    println!("{}", buf);
    Ok(())
}
