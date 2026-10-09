# Aerova

From-scratch browser + web engine + search engine in Rust. Tested on Windows/WSL first, ported to Lumora OS after Lumora Build 1 (net/libc/GPU) lands.

- `cargo xtask build` — build
- `cargo xtask test` — tests
- `cargo xtask lint` — clippy `-D warnings`
- `cargo xtask fmt` — fmt check
- `cargo run -p aerova-shell -- home`
- `cargo run -p aerova-shell -- open aerova://home`
