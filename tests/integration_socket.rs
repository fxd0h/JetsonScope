use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use jetsonscope::protocol::{Request, Response};

fn connect() -> Option<UnixStream> {
    let socket_path = std::env::var("JETSONSCOPE_SOCKET_PATH")
        .or_else(|_| std::env::var("TEGRA_SOCKET_PATH"))
        .unwrap_or_else(|_| "/tmp/jetsonscope.sock".to_string());
    let mut path = PathBuf::from(&socket_path);
    if !path.exists() {
        let legacy = PathBuf::from("/tmp/tegrastats.sock");
        if legacy.exists() {
            path = legacy;
        } else {
            eprintln!("Socket not found at {}, skipping tests", path.display());
            return None;
        }
    }
    match UnixStream::connect(&path) {
        Ok(s) => Some(s),
        Err(e) => {
            eprintln!("Socket unreachable ({}), skipping tests", e);
            None
        }
    }
}

#[test]
fn test_socket_stats_request() {
    let mut stream = match connect() {
        Some(s) => s,
        None => return,
    };

    let req = Request::GetStats;
    let json_req = serde_json::to_string(&req).expect("Failed to serialize request");
    stream
        .write_all(json_req.as_bytes())
        .expect("Failed to write request");

    let mut buf = String::new();
    stream.read_to_string(&mut buf).expect("Failed to read response");

    let resp: Response = serde_json::from_str(&buf).expect("Failed to parse response");

    match resp {
        Response::Stats { source, data } => {
            assert!(!source.is_empty(), "Source should not be empty");
            if let Some(stats) = data {
                assert!(stats.cpus.len() > 0 || stats.ram.is_some());
            }
        }
        _ => panic!("Expected Stats response"),
    }
}

#[test]
fn test_socket_meta_request() {
    let mut stream = match connect() {
        Some(s) => s,
        None => return,
    };

    let req = Request::GetMeta;
    let json_req = serde_json::to_string(&req).expect("Failed to serialize");
    stream
        .write_all(json_req.as_bytes())
        .expect("Failed to write");

    let mut buf = String::new();
    stream.read_to_string(&mut buf).expect("Failed to read");

    let resp: Response = serde_json::from_str(&buf).expect("Failed to parse");

    match resp {
        Response::Meta(hw) => {
            assert!(!hw.model.is_empty() || !hw.soc.is_empty());
        }
        _ => panic!("Expected Meta response"),
    }
}

#[test]
fn test_socket_list_controls() {
    let mut stream = match connect() {
        Some(s) => s,
        None => return,
    };

    let req = Request::ListControls;
    let json_req = serde_json::to_string(&req).expect("Failed to serialize");
    stream
        .write_all(json_req.as_bytes())
        .expect("Failed to write");

    let mut buf = String::new();
    stream.read_to_string(&mut buf).expect("Failed to read");

    let resp: Response = serde_json::from_str(&buf).expect("Failed to parse");

    match resp {
        Response::Controls(controls) => {
            for control in controls {
                assert!(!control.name.is_empty());
            }
        }
        _ => panic!("Expected Controls response"),
    }
}

#[test]
fn test_reconnect_after_close() {
    // Open, send stats, close, reopen to ensure daemon responds again
    let mut stream = match connect() {
        Some(s) => s,
        None => return,
    };
    let req = Request::GetStats;
    let json_req = serde_json::to_string(&req).expect("Failed to serialize request");
    stream
        .write_all(json_req.as_bytes())
        .expect("Failed to write request");
    let mut buf = String::new();
    stream.read_to_string(&mut buf).expect("Failed to read response");
    let _resp: Response = serde_json::from_str(&buf).expect("Failed to parse response");

    drop(stream);

    let mut stream2 = match connect() {
        Some(s) => s,
        None => return,
    };
    let json_req = serde_json::to_string(&Request::GetStats).expect("Failed to serialize request");
    stream2
        .write_all(json_req.as_bytes())
        .expect("Failed to write request");
    let mut buf2 = String::new();
    stream2.read_to_string(&mut buf2).expect("Failed to read response");
    let _resp2: Response = serde_json::from_str(&buf2).expect("Failed to parse response");
}
