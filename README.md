# Aerova

From-scratch browser + web engine + search engine in Rust.
Tested on WSL Ubuntu 26.04 / Windows first. Lumora OS port uses frozen `BlitTarget` ABI — no engine rewrite.

Repo: `https://github.com/CodeVynix/Aerova`

## Quick review (copy-paste)

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cd ~/aerova
cargo xtask build
cargo xtask test
cargo xtask lint
cargo xtask fmt
```

## Demo matrix (all verified green, 22 tests)

| Demo | Command | Expected |
|---|---|---|
| Home | `./target/debug/aerova home` | `Welcome to Aerova.` |
| Open file + shot | `./target/debug/aerova open file:///tmp/x.html --shot /tmp/out.ppm` | text + `saved /tmp/out.ppm` |
| Open web (TLS) | `./target/debug/aerova open https://example.com` | `Example Domain ...` |
| Styled shot | `./target/debug/aerova open file:///tmp/aero_style2.html --shot /tmp/s.ppm` | `red box green text` |
| Crawl | `./target/debug/aerova crawl https://example.com --db /tmp/a.db` | `indexed 0 Example Domain` |
| Search | `./target/debug/aerova search example --db /tmp/a.db` | `3 Example Domain` |
| JS stub | `./target/debug/aerova js "40+2"` | `42` |
| JS Boa | `cargo run -q -p aerova-shell --features aerova-js/boa -- js "(function(a,b){return a*b;})(6,7)"` | `42` |
| Tabs isolation | `./target/debug/aerova tabs demo` | `tab 0 ok` + `tab 1 isolated failure (tab survives)` |
| Lumora blit | `./target/debug/aerova lumora-blit --shot /tmp/lum.ppm` | `lumora blit 800x600` |

## Layout

- `crates/aerova-{net,html,dom,css,layout,paint,js,engine,search,lumora,shell}` + `xtask`
- `docs/arch.md` pipeline, `docs/lumora-port.md` blit ABI, `docs/tabs-isolation.md`, `docs/js-boa.md`, `docs/ROADMAP.md`
- `cargo xtask <build|test|lint|fmt>`

## Lumora Store ready

- Native today: WSL/Windows CLI + PPM shots.
- Lumora tomorrow: engine unchanged, `MemorySurface` swaps for compositor `BlitTarget`, `render_isolated` thread → process.
- Needs from Lumora OS: libc/POSIX, net/TLS, framebuffer + compositor + fonts. See `docs/lumora-port.md`.
