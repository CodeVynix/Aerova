# Aerova arch v0.1

Pipeline: net -> html -> dom -> css -> layout -> paint -> shell. Search + JS are side crates.

- net: file:// + aerova:// now; http(s) TODO ureq+rustls.
- html/dom: tiny parser, never panics on malformed.
- css: rule parser stub.
- layout: block stacking stub (20px rows, 800px wide).
- paint: RGBA framebuffer + PPM dump, no GPU yet.
- js: `int+int` stub; Boa in Phase 6 behind feature.
- search: std-only TF index; tantivy later.
- lumora: stub until Lumora libc/compositor ready.
