# Dependencies

v0.1 was std-only. Phase 1 adds first external dep below.

- `ureq 2` + `rustls` (via default features) in `aerova-net` for http(s) + TLS. Lumora impact: requires TCP/IP, DNS, TLS; file:// and aerova:// stay offline-capable.
- boa_engine 0.20 optional in aerova-js behind `boa` feature (real JS); default stub keeps Lumora port std-only. tantivy (search, optional), winit+wgpu (desktop GUI shell).

Kernel-style rule: every new dep gets an entry here with why + Lumora impact.
