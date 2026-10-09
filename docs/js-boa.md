# JS Boa

- Default: `eval_stub` (`int+int` only), std-only, Lumora-safe.
- Real JS: `cargo test -p aerova-js --features boa`, `cargo run -p aerova-shell --features aerova-js/boa -- js "(function(a,b){return a*b;})(6,7)"`.
- `eval()` dispatches by feature. DOM bindings TODO Phase 6b.
