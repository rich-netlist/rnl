# rnl-host-spectre

Reads Spectre netlists into an `rnl_runtime::host::HostDocument`.

- Instances with or without parentheses around their nodes; the primitives `resistor`, `capacitor`, `inductor`, `vsource`, `isource`, `diode`, `vcvs`, `vccs`, `ccvs`, `cccs`, `bsource`, `switch` and `port` with their terminal names and value parameters.
- `model` cards: an instance of a model gets the terminals and type of the model's family (MOS, BJT, diode, JFET, R, C, L), including binned models.
- `subckt`/`ends` (also `inline subckt`), `global`, `include` with `section=`, `library`/`section`/`endsection`.
- `simulator lang=spice` parts are read with the SPICE adapter's rules.
- Comment lines starting with `//` or `*` are carriers of RNL records (2.2); names are case-sensitive and only `0` is ground.

A Spectre netlist and its SPICE translation with the same records give the same effective model; [`tests/conformance/equivalence`](../../tests/conformance/README.md) checks this.

See the [roadmap](../../docs/roadmap.md) for planned work.
