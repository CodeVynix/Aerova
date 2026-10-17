# JS DOM bindings v4 (element API + target-scoped dispatch)

Requires `--features boa`.

## Query
- `document.getElementById(id)` -> wrapped element or null.
- `document.querySelector(sel)` / `querySelectorAll(sel)` -> `*`, `tag`, `.class`, `#id`, `tag.class`, `tag#id`.
- Wrappers are cached per node index, so two queries for the same node return the same object (`===`).

## Elements
- `tag`, `id`, `textContent`, `classes`, `className` (read-only join).
- `getAttribute(name)`, `setAttribute(name, value)`, `removeAttribute(name)`, `hasAttribute(name)`.
- `classList` with `add`, `remove`, `contains`, `toggle`.
- `addEventListener(type, fn)`, `removeEventListener(type, fn)`, `dispatchEvent(ev)`.
- Attribute and class mutations are JS-local: they do not re-render, because Aerova renders before scripts run and has no style recalc yet.
## Mutation
- `document.createElement(tag)` + `appendChild` on elements and `document`; created nodes render after scripts run and are queryable.
- `textContent =`, `setAttribute`, `removeAttribute`, `className =`, `classList` sync back on created nodes only; moving existing nodes is not supported.
- Elements created by click listeners are dropped (no click-time relayout yet).
- Click targeting on created nodes is exact for single and same-parent appends; creation order differing from document order across parents may mis-target clicks.

## Events
- `dispatchEvent` accepts `{type:'..', bubbles:bool}` or a bare string; returns `false` if `preventDefault()` was called.
- Event object: `type`, `target` (originating node), `currentTarget` (node currently handling), `bubbles`, `defaultPrevented`, `preventDefault()`, `stopPropagation()`, `stopImmediatePropagation()`.
- Dispatch walks the parent chain from the target up to `document`, so a listener on `document` receives events from any element with `target` set correctly (delegation).
- `stopPropagation()` halts the walk after the current node; `stopImmediatePropagation()` also skips remaining listeners on the same node.
- `bubbles: false` stops at the target. Listeners only fire when dispatched; nothing fires on its own until an input source exists (GUI shell).

## Timers
- `setTimeout(fn|string, ms)` returns an id; `clearTimeout(id)` cancels.
- Dispatch is delay-ordered. Aerova has no event loop, so `__now` is a **logical** clock that advances to each timer's due time: delays model ordering, not wall clock. A callback scheduling a timer with `setTimeout(f, 0)` lands behind everything already due.
- Drain is capped at 10000 callbacks per script run to stay finite.
- `setInterval(fn, ms)` / `clearInterval(id)` repeat until cleared; an uncleared interval burns the same 10000-callback cap, so pages stay finite.

## Location
- `window.location.href` (also bare `location.href`) reflects the navigated URL on live pages; static `render_text` output sees `""`.
- Assigning `location.href` from JS changes the value but does not navigate yet.

## Lifecycle
- One `DOMContentLoaded` is dispatched on `document` after the main script, before timers.

## Stub (default, no `boa`)
- `document.write` scan only; query/timer/event calls degrade silently.
- Same known gaps as v1: `document.write` appends to the text buffer, not to `__DOM`, so written nodes are not queryable; the DOM snapshot is flat apart from the parent chain used for bubbling, and page scripts share one live session while static `render_text` runs one session per `<script>` tag (created nodes still render, but later tags cannot query earlier tags' nodes).
