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
