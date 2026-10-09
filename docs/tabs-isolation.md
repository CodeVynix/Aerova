# Tabs + isolation (Phase 4 stub)

- `TabManager`: per-tab history, back/forward, list/close. No shared mutable page state.
- `IsolationPolicy::PerTabThread`: `render_isolated` spawns a thread per render; `join` contains panics so one tab cannot kill the shell.
- Lumora path: replace thread spawn with per-tab process once syscalls/process model lands. Engine stays unchanged; only `render_isolated` backend swaps.
