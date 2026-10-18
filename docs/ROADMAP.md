# Aerova ROADMAP

## Done (main, green)

- v0.1 scaffold: `net/file`, `html/dom`, `css`, `layout`, `paint/PPM`, `engine`, `shell CLI`, `search TF`, `lumora stub`
- Phase 1 net: `http(s)` via `ureq 2 + rustls`, local-server test
- Phase 2 search: `index_url` crawl hook + `save/load` DB + `crawl/search --db`
- JS: `boa_engine 0.20` optional behind `boa` feature, `eval()` dispatch
- Style: cascade (`*`+tag), `height/width`, `background`, `<style>` extract, styled PPM
- Box model: block `margin`/`padding` (CSS shorthand + longhand) + flex `gap`, item margins/padding
- Tabs: `TabManager` + `PerTabThread` panic containment + `tabs demo`
- Lumora: frozen `BlitTarget` ABI + `MemorySurface` + `blit_to_raw_framebuffer` + `lumora-blit`
- GUI shell: `minifb` window (`gui` feature) on the engine framebuffer, URL bar + status line, click-to-dispatch via live `Page`

## Next (ranked)

1. ~~JS DOM bindings~~ (done) — expose `document/window/timers` to Boa, run `<script>` during `render_text`. Unblocks real pages.
2. **Flexbox + block margin/padding (DONE)** — block `margin`/`padding` + flex `gap` land in `layout`. Next: UA default margins, column gap, inline/text wrapping.
3. **Search ranking v2 (RECOMMENDED)** — BM25 + title boost + `aerova://search?q=` page. Keeps std-only or `tantivy` optional.
4. **Own JS VM (LATER)** — behind `own-vm` flag vs Boa. Do not block web compat on it.
5. **Global crawler (HIGHLY DISCOURAGED now)** — needs fleet + politeness + spam. Stay local + explicit crawl list.

## Lumora gating

Needs: libc/POSIX, net/TLS, framebuffer/compositor/fonts. Then: `BlitTarget` impl in compositor, `render_isolated` process backend, Store entry alongside Chromium/Firefox.
