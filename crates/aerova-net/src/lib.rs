//! Aerova net: file:// + aerova:// + http(s) via ureq+rustls (Phase 1).
//! Lumora impact: needs TCP/IP + DNS + TLS in Lumora; file/aerova paths stay offline.

use std::io::Read as _;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use aerova_search::Index;

/// Global search index cache; loaded on first search from the default DB path.
static SEARCH_INDEX: OnceLock<Mutex<Index>> = OnceLock::new();

fn default_db_path() -> std::path::PathBuf {
    std::env::temp_dir().join("aerova-search.db")
}

fn get_index() -> &'static Mutex<Index> {
    SEARCH_INDEX.get_or_init(|| {
        let path = default_db_path();
        Mutex::new(Index::load(&path).unwrap_or_else(|_| Index::new()))
    })
}

fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    for c in s.chars() {
        match c {
            '&' => out.push('&'),
            '<' => out.push('<'),
            '>' => out.push('>'),
            '"' => out.push('"'),
            '\'' => out.push('\''),
            c => out.push(c),
        }
    }
    out
}

fn percent_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            if let (Some(h1), Some(h2)) = (chars.next(), chars.next()) {
                if let Ok(byte) = u8::from_str_radix(&format!("{h1}{h2}"), 16) {
                    out.push(byte as char);
                    continue;
                }
            }
            out.push('%');
        } else if c == '+' {
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out
}

/// Render search results page for a query.
fn render_search(query: &str) -> (Vec<u8>, String) {
    let idx = get_index().lock().unwrap();
    let results = idx.search(query, 20);
    let mut html = String::from("<title>Search: ");
    html.push_str(&escape_html(query));
    html.push_str("</title><h1>Search: ");
    html.push_str(&escape_html(query));
    html.push_str("</h1>");
    if results.is_empty() {
        html.push_str("<p>No results.</p>");
    } else {
        html.push_str("<ul>");
        for (id, score) in results {
            if let Some((title, body)) = idx.get(id) {
                let snippet = body.chars().take(160).collect::<String>();
                html.push_str(&format!(
                    "<li><strong>{}</strong> (score: {:.3})<br>{}</li>",
                    escape_html(title),
                    score,
                    escape_html(&snippet)
                ));
            }
        }
        html.push_str("</ul>");
    }
    (html.into_bytes(), "text/html".to_string())
}

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
        if let Some(q) = url.strip_prefix("aerova://search?") {
            let query = percent_decode(q);
            return Ok(render_search(&query));
        }
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

    #[test]
    fn search_scheme_renders_results() {
        // First populate the index with test data.
        let idx = crate::get_index();
        idx.lock()
            .unwrap()
            .add("Aerova Home", "Welcome to Aerova browser");
        idx.lock()
            .unwrap()
            .add("Rust Guide", "Learn Rust programming");
        let (bytes, ct) = fetch("aerova://search?q=aerova").unwrap();
        assert_eq!(ct, "text/html");
        let html = String::from_utf8(bytes).unwrap();
        assert!(html.contains("Aerova Home"));
        assert!(html.contains("score:"));
    }

    #[test]
    fn percent_decode_works() {
        assert_eq!(crate::percent_decode("hello+world%21"), "hello world!");
        assert_eq!(crate::percent_decode("a%20b"), "a b");
    }
}
