//! Aerova Lumora port shim (gated by Lumora OS completion).
//! Target: Lumora framebuffer + lumora-ui + compositor + POSIX libc.
//! Status: STUB until Lumora Build 1 steps 11-13 (net/libc/GPU) land.
//!
//! Port needs only: files, sockets, mmap, threads, blit rect, font draw.
//! No V8/Skia/NSS unlike Chromium/Firefox — see docs/lumora-port.md.

// SAFETY: no unsafe in stub.
const TODO_PORT: &str = "waiting on Lumora libc/compositor";

/// Returns port readiness string.
#[must_use]
pub fn status() -> &'static str {
    TODO_PORT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_reports() {
        assert!(!status().is_empty());
    }
}
