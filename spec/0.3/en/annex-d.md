## Annex D Changes from version 0.2 (informative)

Version 0.3 lets the solver write back every decision it makes, so that a viewer that does not solve draws what the solver drew. Versions 0.x make no promise of compatibility with each other (16.1); this annex lists what changed and how a file moves to 0.3.

| Change | See |
|---|---|
| Objects that cannot be reached by a name have numbers, such as `%#3`, used only in records of the solver. The record head field `numbering` holds a check of the numbers; when it does not match, the statements that use numbers take no effect | 5.9, 2.8, 3.2, 5.7, 7.6, 9.4, 13.1, 13.3 |
| Given a wire in place of a net, WIRE rewrites that wire; the solver writes the routes of `"auto"` wires this way | 13.1, 5.1, 5.3, 5.8, 10.2 |
| The solver must write back every decision it makes, and passes its results to other tools only in a form that a record of the solver can hold | 14.8, 1.6, 8.2, 8.5, 12.1, 13.3, 17 |
| When the solver solves, the records with `type="generated"` are left out of the interpretation; how else the solver uses them is up to it. Version 0.2 said that the solver must not read them | 2.8, 14.8, 17 |
| Clarified: the priority of a usage governs the PRIORITY statements in the body of its definition; a PRIORITY written from outside through `::` on a local usage governs what that usage expands to; a group in a disabled usage can be turned on again from outside | 14.8, 11.5 |
| Clarified: the names bound in a record with `scope` are published to the target level, where binding one again to the same object is allowed | 2.6 |
| A note on how conflicts may be resolved | 8.5 |
| Clarified: facts have a solution when real coordinates satisfy them within the tolerance, and integer points are only an aim; a value without a finite decimal form is written back with enough digits | 8.5, 11.7 |
| An implementation may limit the time it spends on checking for a solution; a fact whose check does not finish is discarded with a warning. This limit and the tolerance are options of the solver that the user can set; the tolerance is greater than 0 | 8.5, 7.6, 11.7 |
| Clarified: an object that the solver writes back takes the place of the one that a rule generates only when the rule leaves out what is written, as the fill of the standard package does | 10.5 |
| The hint of a discarded fact names every set of facts that it conflicts with, and each statement in these sets is told which facts were discarded because of it; for a fact of priority `"low"` these hints may be left out | 8.5 |
| Clarified: a fact that has no solution on its own contradicts itself; it fails and takes no part in resolution, as 9.5 of version 0.2 already shows | 8.5, 9.5 |
| Clarified: each segment of a pipe has a place of its own in the text, so a later segment counts as written later wherever the place of writing matters, such as in override by key, the appearance queues and the order of resolution | 5.5, 1.5, 8.3, 8.4, 8.5, 13.1 |
| How an editor changes an object that the solver wrote out | 17 |
| In a file of version 0.3, the rule package of the other dialect is dropped implicitly: a file in the SPICE dialect does not use `rnl-std-spectre`, and a file in the Spectre dialect does not use `rnl-std-spice`. The dialect of a file is the language mode of its first statement; a USE-PACKAGE of either package on the chain of levels keeps it there | 16.3 |
| Terminal names by default for T and O (`p1`, `n1`, `p2`, `n2`), U (`a`, `b`, `ref`) and Z (`d`, `g`, `s`), by the version of the file in which the instance line stands | 6.12 |
| The default terminal names of Spectre are listed: of its primitives, of `switch`, and of the instances of model cards by the family of the model, with the type that the condition `type` compares | 6.12, 10.1 |
| Clarified: a section of `.lib` that is read counts as a file, and its header record is the first record in the section | 2.8 |
| Clarified: when a name of a file has the spelling of a standard word that a newer version adds, raising the version gives a warning and is done all the same | 16.1 |
| The priority of the record head is no default for AS and ATTACH, except `"disabled"`: a usage keeps the priorities of its body unless it is given one; a disabled record still turns off its usages with their bodies. The solver writes out a usage that a rule made with the priority that the rule writes on it, as written, or with the `"disabled"` of a disabled record, and otherwise with none | 14.8, 2.8, 10.5 |
| The implicit standard package follows the language version of the file: the series 0.2 for a file of version 0.2, the series 0.3 for a file of version 0.3; a pinned version comes first | 16.3, 16.1 |
| Version 0.3 replaces 0.2, which tools may still read: they write 0.3, and raise a file of 0.2 that they write into, with a pin of the standard series 0.2, so that the file keeps its names and its drawing | 16.1, 16.3 |

**Moving a file from 0.2 to 0.3.** Raise the version in the header record (16.1); a tool that writes into a file of 0.2 does so itself, and pins the series 0.2 (16.1). Version 0.3 adds one standard word, the record head field `numbering`; in 0.2 it was an unknown field. Raising the version warns about the names that have its spelling (16.1), and in 0.3 an alias of that spelling is an error (16.5). A record of the solver written for 0.2 uses no numbers and stays valid; the next solve also writes back the decisions that 0.2 could not hold. Some results can differ from what a tool made of 0.2: a priority written from outside on a local usage decides for everything that the usage expands to (14.8); a file no longer uses the rule package of the other dialect, so that in a Spectre netlist an instance such as `I0` is no longer drawn as a current source (16.3); and T, O, U and Z instances have terminal names by default, which a DEVICE of the file still overrides (6.12). Raised to 0.3, a file that does not pin the standard package also moves from the series 0.2 to the series 0.3, which renames some definitions and symbols; to keep the names of 0.2, pin it with `@USE-PACKAGE(name="rnl-std", version="0.2")` in the header record (16.3); see the standard series 0.3 below.

**The standard series 0.3.** The standard package is not part of this standard (16.3), and the README of the standard packages lists in full what its series 0.3 changes. What follows is what matters to a file that moves from the series 0.2 to the series 0.3.

**Renamed.** The names of 0.3 are written out in full, and the symbol of a virtual part is the name of its definition followed by `_SYMBOL`. The roles and parameters keep their names, but for the role `series` of `$CHAIN` (below); only these names change:

| Series 0.2 | Series 0.3 | Kind |
|---|---|---|
| `$GND` | `$GROUND` | definition |
| `$IOPORT` | `$IO_PORT` | definition |
| `$NC` | `$NO_CONNECT` | definition |
| `$DIFFPAIR` | `$DIFFERENTIAL_PAIR` | definition |
| `$DIFFPAIR_TAIL` | `$DIFFERENTIAL_PAIR`, with its `tail` filled | definition |
| `$MIRROR_PAIR` | `$MIRRORED` | definition |
| `$GND_STUB` | `$GROUND_STUB` | definition |
| `$GND_FILL_SET` | `$GROUND_FILL_SET` | set |
| `$LABEL_TERM_SET` | `$LABEL_TERMINAL_SET` | set |
| `$GROUND` | `$GROUND_SYMBOL` | symbol |
| `$SUPPLY_BAR` | `$SUPPLY_SYMBOL` | symbol |
| `$PORT_TAG` | `$IO_PORT_SYMBOL` | symbol |
| `$NC_CROSS` | `$NO_CONNECT_SYMBOL` | symbol |
| `$JUNCTION_DOT` | `$JUNCTION_SYMBOL` | symbol |
| `$CURRENT_ARROW_SYM` | `$CURRENT_ARROW_SYMBOL` | symbol |
| `$ARROW_HEAD` | `$ARROW_SYMBOL` | symbol |
| `$RES` | `$RESISTOR` | symbol |
| `$CAP` | `$CAPACITOR` | symbol |
| `$IND` | `$INDUCTOR` | symbol |
| `$VSRC` | `$VOLTAGE_SOURCE` | symbol |
| `$ISRC` | `$CURRENT_SOURCE` | symbol |
| `$NMOS4` | `$NMOS_4TERM` | symbol |
| `$PMOS4` | `$PMOS_4TERM` | symbol |

`$GROUND` is in both series with another meaning: in 0.2 it is the symbol of a ground, in 0.3 the definition of a ground, which 0.2 calls `$GND`. So a file renames `$GROUND` first and `$GND` after it, and only whole names: the `$GND` in `$GND_STUB` is not a name of its own.

**Same name, other drawing or meaning.**

- `$VCVS` and `$VCCS` draw only the two output terminals. The symbols with all four are `$VCVS_4TERM` and `$VCCS_4TERM`, which E and G instances, and the masters `vcvs` and `vccs` of Spectre, get by default. In a file that gives `$VCVS` or `$VCCS` to an instance itself, the instance now has two pins, and the wires to its terminals 3 and 4 fail (9.2.4); the file gives it the symbol with `_4TERM` instead.
- The symbols are drawn anew, after those of Analog Canvas, and many have their pins at other points: the pins of a two-terminal part such as `$RESISTOR` are 4 apart, not 6; the diode lies across; the + input of `$OPAMP`, terminal 1, is at the top. Lines are 0.16 wide, where 0.2 draws hairlines. Positions and routes written back for the symbols of 0.2 may therefore no longer meet the pins; such a file is solved again, or pins the series 0.2.
- `$DIFFERENTIAL_PAIR` has a third role, the set role `tail`, which may be left out, so a pair written as in 0.2 means the same.
- Both roles of `$CURRENT_MIRROR` are set roles: a mirror with several reference or output transistors is one usage, where `#M3(ref) {#M4 #M6}(out)` made two usages in 0.2.
- The set role `series` of `$CHAIN` is called `chain`.
- A constraint of priority `"low"` in the body of `$DIFFERENTIAL_PAIR` puts the tail under the middle of the pair (9.5), so a file that does not place its tail is drawn otherwise than in 0.2; `%dp1 @PRIORITY("disabled")` turns it off for one pair (14.8).
- Every constraint in the body of a definition of the standard package is of priority `"low"` (9.5). `$MIRRORED` puts a on the left of b (14.7); `$BRANCH` puts its members in one column, and `$CHAIN` its source, its chain and its load from left to right.
- The four roles of `$CASCADE` are set roles: a stage of several objects is one set, where `{#RZ #CC}(feedback)` made one usage for each member in 0.2.
- `$OPAMP` is no longer in `rnl-std` but in `rnl-std-functional`, which a file names in its header record, alone or through `rnl-std-full` (16.3).
- `$GROUND_STUB` and `$SUPPLY_STUB` give only the least distance `gap` from the pin, no longer an exact one, so that a solver may draw the wire longer, and point the ground down and the supply up, `@ORIENT("R0")`, all at priority `"low"` (10.5).
- The fill puts no port tag on a ground net or a supply net, and draws the grounds of a ground net other than 0, such as vss added to `$GROUND_NET_SET`, with `$NAMED_GROUND_SYMBOL`, which shows the name of the net (10.5).
- Instances that the series 0.2 drew without a symbol of its own now get one by default: S with `$VOLTAGE_CONTROLLED_SWITCH`, W with `$SWITCH`, B with `$BEHAVIORAL_SOURCE`, and F, H, J, Z, T, O and U of SPICE with theirs, as do the masters of Spectre for these elements, such as `relay` and `switch`.

**Removed.** `$INPUT_STAGE`: its tail goes into the role `tail` of its differential pair, as in `#M1(in_p) #M2(in_n) #M5(tail) @AS $DIFFERENTIAL_PAIR`. `$TAIL_CENTER`: the standard package centres the tail of each differential pair itself; to centre other objects, a file writes a definition of its own, such as `$TAIL_MID` of 9.4. `rnl-std-semantics` and `rnl-std-layout`: they are one package, `rnl-std-structures` (16.3); a file that dropped `rnl-std-layout` to do without the default layouts writes, for each definition, a rule such as `# @SELECT-USAGE $MIRRORED => @PRIORITY("disabled")` (14.8).

**New.** `rnl-std-basic` and `rnl-std-semiconductors` add symbols, such as switches, current-controlled sources, transmission lines and JFETs. The series also adds packages that `rnl-std` does not bring in: `rnl-std-functional`, `rnl-std-rf`, `rnl-std-logic` and `rnl-std-signal-flow`, the collection `rnl-std-full` of these with `rnl-std`, and the optional `rnl-std-no-labels` (16.3). A name that a file defines itself and that the series 0.3 adds, such as `$SWITCH`, hides the standard one with a hint (6.8), so the file keeps its own.

**Revision 2.** An editorial revision; the language does not change. The examples of 1.3 and 5.9 write the model card of their transistors, `.model nch nmos level=1`, so that the standard package can tell their polarity and give them a symbol.
