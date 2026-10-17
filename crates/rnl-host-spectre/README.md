# rnl-host-spectre

Reads Spectre netlists into an `rnl_runtime::host::HostDocument`.

- Instances with or without parentheses around their nodes; the primitives `resistor`, `capacitor`, `inductor`, `vsource`, `isource`, `diode`, `vcvs`, `vccs`, `ccvs`, `cccs`, `bsource`, `switch`, `relay`, `transformer` and `port` with their terminal names and value parameters. A `switch` has a terminal for each of its nodes, named as Spectre numbers them: `t0`, its common terminal, and `t1` to `tn`, its throws. A `relay` has `p n cp cn`, the switch between `p` and `n` controlled by the voltage between `cp` and `cn`, also as the master of a model card, whose instances have the model type `sw`, as S of SPICE. A `transformer` has `p_plus p_minus s_plus s_minus`, its two windings.
- `model` cards: an instance of a model gets the terminals and type of the model's family (MOS, BJT, diode, JFET, R, C, L), including binned models.
- `subckt`/`ends` (also `inline subckt`), `global`, `include` with `section=`, `library`/`section`/`endsection`.
- `simulator lang=spice` parts are read with the SPICE adapter's rules. A `.scs` file starts in the Spectre mode and any other file in the SPICE mode, also when it says `simulator lang=spectre` near its top; each `simulator lang=` line switches the mode of the lines after it (2.2). The lines read in SPICE mode are listed in `HostDocument::spice_mode`, where the tools that write carry a record with `*` (2.2).
- `protect … unprotect`, and `.prot … .unprot` in SPICE mode, regions written to be encrypted, are read; a record in one gives a warning, and the regions are listed in `HostDocument::protected` (2.2).
- Comment lines starting with `//` or `*` are carriers of RNL records (2.2); names are case-sensitive and only `0` is ground.

A Spectre netlist and its SPICE translation with the same records give the same effective model; [`tests/conformance/equivalence`](../../tests/conformance/README.md) checks this.

See the [roadmap](../../docs/roadmap.md) for planned work.
