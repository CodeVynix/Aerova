# JS DOM bindings v3 (real timers + event dispatch)

Requires `--features boa`.

## Query
- `document.getElementById(id)` -> wrapped element or null.
- `document.querySelector(sel)` / `querySelectorAll(sel)` -> `*`, `tag`, `.class`, `#id`, `tag.class`, `tag#id`.
- Wrappers are cached per node index, so two queries for the same node return the same object (`===`).

## Elements
- `tag`, `id`, `textContent`, `classes`.
- `el.addEventListener(type, fn)`, `el.removeEventListener(type, fn)`, `el.dispatchEvent(ev)`.
- Event object: `type`, `target`, `currentTarget`, `bubbles` (flag only), `defaultPrevented`, `preventDefault()`, `stopPropagation()`.
- `dispatchEvent` accepts `{type:'..'}` or a bare string; returns `false` if `preventDefault()` was called.

## Timers
- `setTimeout(fn|string, ms)` returns an id; `clearTimeout(id)` cancels.
- Dispatch is delay-ordered. Aerova has no event loop, so `__now` is a **logical** clock that advances to each timer's due time: delays model ordering, not wall clock. A callback scheduling a timer with `setTimeout(f, 0)` lands behind everything already due.
- Drain is capped at 10000 callbacks per script run to stay finite.
- `setInterval` is not implemented yet.

## Lifecycle
- One `DOMContentLoaded` is dispatched on `document` after the main script, before timers.
- Listeners only fire when dispatched. Nothing fires on its own until an input source exists (GUI shell).

## Stub (default, no `boa`)
- `document.write` scan only; query/timer/event calls degrade silently.
- Same known gaps as v1: `document.write` appends to the text buffer, not to `__DOM`, so written nodes are not queryable; the DOM snapshot is flat, so there is no parent chain to bubble through, and no DOM mutation API (`createElement`/`appendChild`/`setAttribute`) yet.
