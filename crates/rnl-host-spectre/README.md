# rnl-host-spectre

Reads Spectre netlists into an `rnl_runtime::host::HostDocument`.

- Instances with or without parentheses around their nodes; the primitives `resistor`, `capacitor`, `inductor`, `vsource`, `isource`, `diode`, `vcvs`, `vccs`, `ccvs`, `cccs`, `bsource`, `switch`, `relay`, `transformer` and `port` with their terminal names and value parameters. A `switch` has a terminal for each of its nodes, named as Spectre numbers them: `t0`, its common terminal, and `t1` to `tn`, its throws. A `relay` has `p n cp cn`, the switch between `p` and `n` controlled by the voltage between `cp` and `cn`, also as the master of a model card, whose instances have the model type `sw`, as S of SPICE. A `transformer` has `p_plus p_minus s_plus s_minus`, its two windings.
- `model` cards: an instance of a model gets the terminals and type of the model's family (MOS, BJT, diode, JFET, MESFET, R, C, L), including binned models, which are known by their full name, `pch.1`, and by their base name, `pch`. `type = p` is `type=p`. A model of a family that 6.12 does not list yet is still of its family: `hisim_hv`, `utsoi`, `letisoi` and `bsimcmg_va` are MOS, `dio500` and `junb` diodes, `gaas`, `angelov`, `tom2` and `tom3` MESFETs (the type `nmf` or `pmf`, drawn as a JFET). The value of an instance of a resistor, capacitor or inductor model is its `r`, `c` or `l`.
- The words of a statement: `name = value`, `name= value` and `name =value` are `name=value`, and a value that holds parentheses or brackets, `r=(1k + 2k)`, `v=2*v(a)` and `table=[1 2 3]`, is one word with the spaces it holds. The value of a `bsource` is its `v` or its `i`. `tline` has four nodes and no terminal names yet, and `mutual_inductor`, which has no nodes and no symbol, as K of SPICE has none, is no instance.
- `subckt`/`ends` (also `inline subckt`), `global`, `include` with `section=`, `library`/`section`/`endsection`.
- `simulator lang=spice` parts are read with the SPICE adapter's rules, and the `.model` cards of SPICE and the `model` statements of Spectre are one table, so an instance of either language names a card of the other. An element of SPICE that names no card, `R1 a b 10k`, has the master of Spectre it equals (`resistor`), since the rule package of Spectre is the only one of a Spectre file. A `.scs` file starts in the Spectre mode and any other file in the SPICE mode, also when it says `simulator lang=spectre` near its top; each `simulator lang=` line switches the mode of the lines after it (2.2). The lines read in SPICE mode are listed in `HostDocument::spice_mode`, where the tools that write carry a record with `*` (2.2).
- `protect … unprotect`, and `.prot … .unprot` in SPICE mode, regions written to be encrypted, are read; a record in one gives a warning, and the regions are listed in `HostDocument::protected` (2.2).
- Comment lines starting with `//` or `*` are carriers of RNL records (2.2); names are case-sensitive and only `0` is ground.

A Spectre netlist and its SPICE translation with the same records give the same effective model; [`tests/conformance/equivalence`](../../tests/conformance/README.md) checks this.

See the [roadmap](../../docs/roadmap.md) for planned work.

## Non-ASCII comment compatibility (standard 3.1)

The set of simulator/version combinations verified by this adapter for non-ASCII host
comments is currently **empty**. This is a generic structural adapter, and neither a
SPICE/Spectre language marker nor a filename identifies the simulator version that will
run the netlist. Reading Unicode successfully in the toolkit is not simulator validation.

The adapter emits one hint per file, at the first non-ASCII character found in a whole-line
or inline host comment. This includes ordinary comments, not just RNL records. Quoted
host strings and SPICE braced expressions are not mistaken for comments. The hint does
not reject the input or change its drawing. Imported pure-RNL packages are not simulator
input and are not subject to this host-comment hint. This conservative policy can be
narrowed when a frontend supplies a simulator/version and there is recorded verification.
