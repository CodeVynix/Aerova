//! Aerova net: file:// + aerova:// now, http(s) via ureq later (Phase 1).
//! Keeps v0.1 offline-buildable and honest about TODOs.

/// Fetch error.
#[derive(Debug)]
pub enum FetchError {
    /// Unsupported scheme.
    Unsupported(String),
    /// IO failure.
    Io(String),
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(s) => write!(f, "unsupported: {s}"),
            Self::Io(s) => write!(f, "io: {s}"),
        }
    }
}

impl std::error::Error for FetchError {}

/// Fetch URL bytes. Supports `file://` now.
/// `http(s)://` returns [`FetchError::Unsupported`] until ureq/TLS lands.
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
    Err(FetchError::Unsupported(
        "http(s) not yet wired; use file:// or aerova:// for v0.1".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_scheme() {
        let (b, ct) = fetch("aerova://home").unwrap();
        assert_eq!(ct, "text/html");
        assert!(!b.is_empty());
    }
}
