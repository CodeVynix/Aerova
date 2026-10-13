# Aerova GUI v1 (`gui` feature)

Run: `cargo run -q -p aerova-shell --features gui,boa --bin aerova-gui -- [url]` (needs a display; WSLg works).

- Window 800x632: 16px URL bar, 800x600 page pixels, 16px status line.
- URL bar: type, backspace, enter loads (`file://` assumed without `://`). Echo is an uppercase-only 5x7 bitmap; the buffer keeps real case.
- Click: hit-tests layout boxes, dispatches `click` with bubbling into the live page; new writes show in the status line.
- Esc or window close quits. Page text also prints to stdout on load.
- Limits: no scroll, no text selection, no `setInterval`, attribute/class writes are JS-local, clicks never relayout (no mutation API yet).
