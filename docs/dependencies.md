# Dependencies

v0.1 was std-only. Phase 1 adds first external dep below.

- `ureq 2` + `rustls` (via default features) in `aerova-net` for http(s) + TLS. Lumora impact: requires TCP/IP, DNS, TLS; file:// and aerova:// stay offline-capable.
- Planned: boa_engine (js, feature-gated), tantivy (search, optional), winit+wgpu (desktop GUI shell).

Kernel-style rule: every new dep gets an entry here with why + Lumora impact.
