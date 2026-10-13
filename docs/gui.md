# Aerova GUI v1 (`gui` feature)

Run: `cargo run -q -p aerova-shell --features gui,boa --bin aerova-gui -- [url]` (needs a display; WSLg works).

- Window 800x648: 16px tab strip, 16px URL bar, 800x600 page pixels, 16px status line.
- URL bar: type, backspace, enter loads (`file://` assumed without `://`). Echo is an uppercase-only 5x7 bitmap; the buffer keeps real case.
- Tabs: Ctrl+T new tab (home), Ctrl+W closes, Ctrl+Tab cycles, click a strip cell to switch. Each tab keeps its own live page.
- Alt+Left / Alt+Right walk the current tab's history, F5 reloads; address-bar loads push history.
- Click: hit-tests layout boxes, dispatches `click` with bubbling into the live page; new writes show in the status line.
- Esc or window close quits. Page text also prints to stdout on load.
- Limits: no scroll, no text selection, no `setInterval`, attribute/class writes are JS-local, clicks never relayout (no mutation API yet).
