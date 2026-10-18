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

## Box model (layout)

- Blocks lay out with CSS `margin` and `padding`. Sides accept 1-4 value shorthands
  (`padding: 10px 20px`), and `margin-top`/`padding-left` etc. override per side.
  `auto` and non-px values are 0.
- A block's box is its border box (no borders yet): children start at the content
  origin, and the box grows by `padding` around the content.
- Flex rows support `gap` (`gap: <row> <column>`; a row uses the column part),
  per-item margins, per-item padding, and container padding/margins.
- No UA default margins (browsers give `p`/`body` margins; Aerova does not), so
  text sits flush at the content origin unless styles add spacing.
- `flex-direction: column` still stacks as plain blocks; no column gap yet.

## Text (paint)

- `aerova-paint` owns the 5x7 bitmap font: `glyph`, `draw_text`, `draw_glyph`,
  `text_width`, `chars_per_line`, `wrap`. The GUI chrome uses the same table, so
  page text and URL/status text render identically.
- Layout wraps each text node greedily to its content width (`wrap`, one line per
  `LINE_H` = 20px) and stores the lines on the box's `text` field, joined by `\n`.
- Rendering is uppercase-only ASCII; lowercase renders uppercase. No descenders,
  no bold/italic, no subpixel positioning.

## Search (net)

- `aerova://search?q=` renders a BM25-ranked results page using the on-disk index
  (`aerova-search.db` in `$TMPDIR`). The index is loaded once per process via a
  `Mutex<Index>` behind a `OnceLock`.
- Query params are percent-decoded (`+` → space, `%XX` → byte). Results show
  title, BM25 score, and a 160-char body snippet; titles are boosted 3×.
- `aerova-net` depends on `aerova-search` (new dep).
