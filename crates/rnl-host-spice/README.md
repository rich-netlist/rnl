# rnl-host-spice

Reads SPICE netlists into an `rnl_runtime::host::HostDocument`.

- Elements by their first letter, with the default terminal names the standard symbols use (`M`: d g s b; `Q`: c b e s; `D`: a k; two-terminal elements: p n; `E`, `G`: p n cp cn).
- The words of an element line: a quoted text (`'2 * V(in)'`) or a braced expression (`{ 2*V(in) }`) is one word whatever spaces it holds, and `w = 1u`, `w= 1u` and `w =1u` are `w=1u`. A word with a `=` is a parameter, but for one that starts with a quote or a brace.
- What an element's words are, once every `.model` card is known: of an `M` or a `Q`, the nodes are the words before the model name, three or more, and the model name is the first word after the third that a `.model` card names, else the last word that is no number and no `off` (so `M1 d g s mod`, `M1 d g s e b mod`, `Q1 c b e s dt mod` and `Q1 c b e mod 2.0` all read). The model of a resistor, capacitor or inductor is the word after its nodes when a card names it (`R1 a b rpoly 10k`). A value written by name, `r=10k`, `c=1p`, `l=2n`, `dc=1.8`, `V=...`, `I=...`, is the value when none is written by position. `.model pch.1` is known as `pch.1` and as `pch`; LTspice's `vdmos` has the type `pmos` with the word `pchan`, else `nmos`.
- `E` and `G` whose value is an expression or a table (`VALUE`, `VOL`, `CUR`, `TABLE`, `LAPLACE`, `FREQ`, `CHEBYSHEV`, `POLY(k)`, with `=` and `{}` or `''` or without) have their two output nodes (a POLY(k) the 2k control nodes after them too) and that expression as their value, not four nodes. An `X` ends its nodes at `PARAMS:` and ignores a lone `/` before the subcircuit's name. HSPICE's `W` (with `N=`, `L=`, `RLGCMODEL=`, or more than `n+ n- Vname model` words) and `U` (more than `n1 n2 nref model` words) are transmission lines, of the type `tline`; four nodes are named `p1 n1 p2 n2`.
- A hint where a number is written with a capital `M`, `10M`: in SPICE it is milli, as `10m` is, and mega is `meg`.
- `.subckt`/`.ends` as levels, `.include`, and `.lib` with the selected section only; `+` continuation lines, those of a `.subckt` adding ports up to its parameters (`params:` or a `name=value`), and an end-of-line comment (`;`, or `$` after white space) holding none; `.control` blocks skipped. The Spectre adapter reads a `.subckt` in SPICE mode the same way.
- `.prot … .unprot` (also `.protect … .unprotect`), regions written to be encrypted, are read; a record in one gives a warning, since it is gone once the region is encrypted, and the regions are listed in `HostDocument::protected` for the tools that write (2.2).
- Comment lines starting with `*` are carriers of RNL records (2.2); names are case-insensitive and `gnd` is ground like `0`.

Writeback goes through `rnl_runtime::writeback`, which keeps the netlist's own lines unchanged.

See the [roadmap](../../docs/roadmap.md) for planned work.
