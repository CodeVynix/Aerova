# JS DOM bindings v1 (Phase 6b)

- `document.write('text')`: appends to page text. `document.title`: page title. `console.log(...)`: captured logs, not page text. `window` aliases global.
- Stub (default): scans `document.write` args, no real JS. Boa (`--features boa` / `aerova-js/boa`): really runs JS with bindings above.
- Engine: `extract_scripts` runs inline `<script>` in order during `render_text`; writes append to visible text. `src=` scripts TODO.
- Tests: `captures_document_write_stub_or_boa`, Boa-only `boa_bindings_title_and_log`, engine `script_write_appends_text`.
