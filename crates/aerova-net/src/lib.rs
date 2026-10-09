//! Aerova net: file:// + aerova:// + http(s) via ureq+rustls (Phase 1).
//! Lumora impact: needs TCP/IP + DNS + TLS in Lumora; file/aerova paths stay offline.

use std::io::Read as _;
use std::time::Duration;

/// Fetch error.
#[derive(Debug)]
pub enum FetchError {
    /// Unsupported scheme.
    Unsupported(String),
    /// IO failure (file).
    Io(String),
    /// HTTP failure (network, TLS, status).
    Http(String),
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(s) => write!(f, "unsupported: {s}"),
            Self::Io(s) => write!(f, "io: {s}"),
            Self::Http(s) => write!(f, "http: {s}"),
        }
    }
}

impl std::error::Error for FetchError {}

fn fetch_http(url: &str) -> Result<(Vec<u8>, String), FetchError> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(15))
        .build();
    let resp = agent
        .get(url)
        .set("User-Agent", "Aerova/0.2")
        .call()
        .map_err(|e| FetchError::Http(e.to_string()))?;
    let ct = resp
        .header("Content-Type")
        .unwrap_or("application/octet-stream")
        .to_string();
    let mut body = Vec::new();
    resp.into_reader()
        .read_to_end(&mut body)
        .map_err(|e| FetchError::Http(e.to_string()))?;
    Ok((body, ct))
}

/// Fetch URL bytes. Supports `file://`, `aerova://`, `http://`, `https://`.
pub fn fetch(url: &str) -> Result<(Vec<u8>, String), FetchError> {
    if let Some(path) = url.strip_prefix("file://") {
        return std::fs::read(path)
            .map(|b| {
                let ct = if path.ends_with(".html") || path.ends_with(".htm") {
                    "text/html".to_string()
                } else {
                    "application/octet-stream".to_string()
                };
                (b, ct)
            })
            .map_err(|e| FetchError::Io(e.to_string()));
    }
    if url.starts_with("aerova://") {
        let body = "<title>Aerova</title><p>Welcome to Aerova.</p>"
            .as_bytes()
            .to_vec();
        return Ok((body, "text/html".to_string()));
    }
    if url.starts_with("http://") || url.starts_with("https://") {
        return fetch_http(url);
    }
    Err(FetchError::Unsupported(format!("scheme unknown: {url}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    #[test]
    fn builtin_scheme() {
        let (b, ct) = fetch("aerova://home").unwrap();
        assert_eq!(ct, "text/html");
        assert!(!b.is_empty());
    }

    #[test]
    fn http_local_server() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0_u8; 1024];
            let _ = stream.read(&mut buf);
            let body = "<p>hello local</p>";
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(resp.as_bytes());
        });
        let url = format!("http://{addr}/");
        let (bytes, ct) = fetch(&url).unwrap();
        assert!(ct.contains("text/html"));
        assert!(String::from_utf8_lossy(&bytes).contains("hello local"));
        handle.join().unwrap();
    }
}
