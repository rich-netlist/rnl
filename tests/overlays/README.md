# Inspection overlay exports

`usage-current.svg` and `usage-current.cell.svg` are full SVG exports of
[`usage-current.sp`](../viewer/inputs/usage-current.sp), with all usages, role labels
and currents enabled. They cover nested/dispersed usages, a virtual symbol, rotated
and mirrored pins, in/out/unspecified precedence, subcircuit instance pins and declaration
ports on explicit port tags. Existing conformance goldens are unchanged.

Run `cargo test -p rnl-view overlays`. To intentionally regenerate only these new goldens,
set `RNL_BLESS_OVERLAYS=1` for that command and review the differences.

After building CLI and the WASM binding, run
`node bindings/runtime-wasm/tests/overlays.mjs` from the repository root to require
byte-identical CLI/npm SVGs, including custom export styles, lazy builds and snapshots.
The Rust CI workflow runs it after building the CLI.

Viewer's `overlays.spec.js` compares exported rectangle coordinates and current paths
with the core Document and projected browser geometry at two zooms in Chromium and WebKit.
It exercises the current switch and persistence on both shells. `RNL_OVERLAY_REVIEW_DIR`
saves English/Chinese light/dark screenshots and CLI SVG/PNG previews there (Chromium only).
