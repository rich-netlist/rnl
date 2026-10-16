# standard

The standard packages of RNL, one directory for each version of the standard series: [`0.2/`](0.2/) holds those of RNL 0.2. They are drafts: the language standard leaves the content of the standard package open, and names may change with it. Packages are found by the name and version their header record declares, not by file name (16.2), so the files of two versions can have the same names.

## 0.2

The standard package `rnl-std` 0.2 is a package collection (16.3): its file has only a header record that references other packages, and so does `rnl-std-core`. Used, a package brings in the packages it references first, each once (16.2), so `rnl-std` brings in seven packages and then itself:

| Order | File | Package | Content |
| --- | --- | --- | --- |
| 1 | [`rnl-std-style.rnl`](0.2/rnl-std-style.rnl) | `rnl-std-style` | The styles `$base`, `$bus`, `$signal`, `$note` and `$grey`, and the default style rules. |
| 2 | [`rnl-std-symbols.rnl`](0.2/rnl-std-symbols.rnl) | `rnl-std-symbols` | The symbols of instances, and the rules that give an instance its symbol by element letter, by Spectre master and by model type. |
| 3 | [`rnl-std-virtual.rnl`](0.2/rnl-std-virtual.rnl) | `rnl-std-virtual` | The virtual elements (ground, supply, port, no-connect, junction, current arrow), their symbols and the rules that give them, and the stubs `$GND_STUB` and `$SUPPLY_STUB`. |
| 4 | [`rnl-std-core.rnl`](0.2/rnl-std-core.rnl) | `rnl-std-core` | References 1 to 3: it draws the netlist and adds nothing. |
| 5 | [`rnl-std-fill.rnl`](0.2/rnl-std-fill.rnl) | `rnl-std-fill` | References 3. The sets that decide where grounds, supplies and port tags are filled in, and the rules that fill them in. |
| 6 | [`rnl-std-semantics.rnl`](0.2/rnl-std-semantics.rnl) | `rnl-std-semantics` | The semantic definitions, such as `$DIFFPAIR`: roles only, nothing drawn. |
| 7 | [`rnl-std-layout.rnl`](0.2/rnl-std-layout.rnl) | `rnl-std-layout` | The layout definitions `$MIRROR_PAIR`, `$TAIL_CENTER` and `$FLOW`. |
| 8 | [`rnl-std.rnl`](0.2/rnl-std.rnl) | `rnl-std` | References 4 to 7. Used implicitly at the root of every document. |
| | [`rnl-zh.rnl`](0.2/rnl-zh.rnl) | `rnl-zh` | Chinese aliases of the standard words and of the standard package's names. A file loads it with `@USE-ALIASES(name="rnl-zh", version="0.2")` in its header record. |

Together, in this order, the sub-packages hold the statements of the single file `rnl-std.rnl` of the RNL toolkit 0.1, the same 0.2: every rule, set edit and fill statement in its order there, and the definitions unchanged, so documents are drawn as before. The names do not repeat across the sub-packages, so a file need not know which one a name comes from.

A file takes less of the standard package by dropping a sub-package with an empty inline package of its name in its header record, such as `@USE-PACKAGE(name="rnl-std-fill") {}` for no automatic fill, and more by writing packages of its own that reference the sub-packages they use (16.2, 16.3). Without the fill, the stubs can still be applied by hand. For the smallest collection only, it drops `rnl-std` with `@USE-PACKAGE(name="rnl-std") {}` and writes `@USE-PACKAGE(name="rnl-std-core", version="0.2")`.

The core interface [`rnl-view`](../../crates/rnl-view/README.md) bundles every `.rnl` file of every version directory, so the `rnl` command and the viewers can use them without a package directory.

## Symbols

| Symbol | For | Terminals (pin order) |
| --- | --- | --- |
| `$RES`, `$CAP`, `$IND` | R, C, L; Spectre `resistor`, `capacitor`, `inductor` | 1 at the top, 2 at the bottom |
| `$VSRC`, `$ISRC` | V, I; Spectre `vsource`, `isource` | 1 (+) at the top; the current arrow points from 1 to 2 |
| `$DIODE` | D; Spectre `diode` and diode models | anode at the top, cathode at the bottom |
| `$VCVS`, `$VCCS` | E, G; Spectre `vcvs`, `vccs` | output 1, 2 up and down, control 3 (+), 4 (−) on the left |
| `$NMOS4`, `$PMOS4` (and three-terminal `$NMOS`, `$PMOS`) | M by name, `nmos` and `pmos` model types | d g s b; NMOS drain up with the arrow out of the source, PMOS source up with the arrow into the source |
| `$NPN`, `$PNP` | Q by name, `npn` and `pnp` model types | c b e; the emitter arrow points out for NPN, in for PNP |
| `$OPAMP` | given by a rule or a statement | 1 (+) lower left, 2 (−) upper left, 3 output |
| `$GROUND`, `$SUPPLY_BAR`, `$PORT_TAG`, `$NC_CROSS`, `$JUNCTION_DOT`, `$CURRENT_ARROW_SYM` | the virtual elements | one pin at the origin |

Every symbol writes its box (`box=`): the extent of its graphics, which a viewer uses for selection and a solver for keeping symbols apart. Pins sit on integer points, so instances on integer points keep wires on the grid.

The geometry comes from the language prototype's standard package, reviewed against the terminal order of the SPICE elements. The prototype took it from Analog Canvas's symbols; their copyright holder publishes it here under CC0-1.0, like the rest of this directory. The rules that pick symbols by Spectre master and by model type are new: Spectre instance names carry no element letter. Elements without a standard symbol yet (J, F, H, B and others) are drawn with the viewer's own stand-in, a dashed box with the terminals on its sides.

The data in this directory is licensed under CC0-1.0, so symbol definitions can be copied into netlists freely.
