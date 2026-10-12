# standard

The standard packages of the RNL language draft 0.2. Both are drafts: the content of the standard package is still open in the language draft, and names may change with it.

| File | Package | Content |
| --- | --- | --- |
| [`rnl-std.rnl`](rnl-std.rnl) | `rnl-std` 0.2 | Used implicitly at the root of every document: styles, symbols, semantic and layout definitions, virtual elements (ground, supply, port, no-connect, junction, current arrow), the sets that decide where ground, supply and port symbols are filled in, and the default rules that give symbols and styles. |
| [`rnl-zh.rnl`](rnl-zh.rnl) | `rnl-zh` 0.2 | Chinese aliases of the standard words and of the standard package's names. A file loads it with `@USE-ALIASES(name="rnl-zh", version="0.2")` in its header record. |

The `rnl` command bundles both, so a document can use them without a package directory.

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

The geometry comes from the language prototype's standard package, reviewed against the terminal order of the SPICE elements. The rules that pick symbols by Spectre master and by model type are new: Spectre instance names carry no element letter. Elements without a standard symbol yet (J, F, H, B and others) are drawn with the viewer's own stand-in, a dashed box with the terminals on its sides.

The data in this directory is licensed under CC0-1.0, so symbol definitions can be copied into netlists freely.
