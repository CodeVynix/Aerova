# JS DOM bindings v2 (deeper)

- `document.getElementById(id)` -> `{tag,id,textContent}` or null.
- `document.querySelector(sel)` / `querySelectorAll(sel)`: `*`, `tag`, `.class`, `#id`, `tag.class`, `tag#id`.
- `document.write`, `document.title`, `console.log`, `window` alias (v1, kept).
- `setTimeout(fn|string, ms)`: v1 queues in JS heap, fires after main script, ignores delay (TODO real clock).
- `addEventListener(type, fn)`: v1 stores in JS heap, fires all listeners after script+timers (TODO real dispatch/targets).
- Callbacks stay as real function refs in the JS heap (Boa stringifies functions as `[native code]`, so no source round-trip).
- Stub (default): `document.write` scan only; query/timer/event calls degrade gracefully. Full DOM needs `--features boa`.
- Engine builds `DomEl` snapshot per render and passes to `run_scripts_with_dom`.
