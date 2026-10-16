# Aerova GUI v1 (`gui` feature)

Run: `cargo run -q -p aerova-shell --features gui,boa --bin aerova-gui -- [url]` (needs a display; WSLg works).

- Window 800x648: 16px tab strip, 16px URL bar, 800x600 page viewport, 16px status line.
- Scroll: mouse wheel, Up/Down (40px, viewport with Ctrl), PageUp/PageDown (viewport), Home/End (ends), or drag the 6px scrollbar thumb (12px grab zone). The thumb only appears when the page is taller than the viewport. Loop runs at 60fps so quick taps register.
- URL bar: type, backspace, enter loads (`file://` assumed without `://`). Echo is an uppercase-only 5x7 bitmap; the buffer keeps real case.
- Tabs: Ctrl+T new tab (home), Ctrl+W closes (quits on the last tab), Ctrl+Tab cycles, click a strip cell to switch. Cells split the strip width evenly with a 48px floor, and every in-window x maps to a tab. Each tab keeps its own live page.
- Typing is ignored while Ctrl/Alt is held. Gestures read keys with `KeyRepeat::No`, so each press fires exactly once; typing uses `KeyRepeat::Yes` so holding a key repeats characters.
- Alt+Left / Alt+Right walk the current tab's history, F5 reloads; address-bar loads push history.
- Click: hit-tests layout boxes, dispatches `click` with bubbling into the live page; new writes show in the status line.
- Perf: the content framebuffer converts to u32 once per page load; every frame copies only the 800x600 viewport rows, so short taps register instead of falling between slow frames.
- Esc or window close quits. Page text also prints to stdout on load.
- Limits: no text selection, no `setInterval`, attribute/class writes are JS-local, clicks never relayout (no mutation API yet).
