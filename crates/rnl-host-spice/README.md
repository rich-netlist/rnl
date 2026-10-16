# rnl-host-spice

Reads SPICE netlists into an `rnl_runtime::host::HostDocument`.

- Elements by their first letter, with the default terminal names the standard symbols use (`M`: d g s b; `Q`: c b e s; `D`: a k; two-terminal elements: p n; `E`, `G`: p n cp cn).
- `.subckt`/`.ends` as levels, `.include`, and `.lib` with the selected section only; `+` continuation lines; `.control` blocks skipped.
- `.prot … .unprot` (also `.protect … .unprotect`), regions written to be encrypted, are read; a record in one gives a warning, since it is gone once the region is encrypted, and the regions are listed in `HostDocument::protected` for the tools that write (2.2).
- Comment lines starting with `*` are carriers of RNL records (2.2); names are case-insensitive and `gnd` is ground like `0`.

Writeback goes through `rnl_runtime::writeback`, which keeps the netlist's own lines unchanged.

See the [roadmap](../../docs/roadmap.md) for planned work.
