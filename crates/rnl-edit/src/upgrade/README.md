# Upgrading RNL to 0.4

The upgrade library reads a supported source document before making changes, edits only RNL records, and reads the proposed result with the same core. Its result contains rewritten texts and a structured report; it performs no filesystem writes. Without a write target, `rnl upgrade FILE` previews the migration and can save a JSON report with `--report`. With `-o DIR` or `--in-place`, the CLI saves the proposed texts using the same encoding-preserving writer as `rnl solve`.

An upgrade is successful only when all changes are supported, target interpretation succeeds, and comparison of objects, networks, positions and facts finds no differences. Unsupported statements retain their original text with an explanatory comment. A successful parse alone does not establish equivalence. Comparison includes unplaced symbol geometry, labels and pin networks; algebraically reversed single-point distance intervals use one canonical signed form. During development, the core may report target features that it has not implemented yet; these remain failures in the report.

## Optional approximations

`--approximate` is off by default. It opts into polarity-based differential-pair and current-mirror replacements (`_N` / `_P`, including unwrapping singleton `{#M5}(tail)` on differential pairs, and `PSEUDO_DIFFERENTIAL_PAIR` for a pair with `{}(tail)` or no tail), choosing an explicit axis direction from source positions, authored directed constraints or written order, and accepting the stronger combination semantics of held sets in ORDER. Each choice is reported in `approximations` with its original and proposed text and a `risk` explanation; the terminal summary counts it separately from exact edits. ALIGN/DISTRIBUTE on multi-member held sets remain manual. Approximation does not suppress target errors, assertion violations or comparison differences.

Verified directed collection minima and existing standard main-point ties no longer need review. Old explicit stub applications retain their default gap by writing it out. Additional main-point ties use ALIGN x/y, with fresh names only when needed. Assertion-only package changes are `notices`, and actual violations are also collected in `assertion_violations`. See [the definition audit](STANDARD-DEFINITIONS.md) for each standard definition and for the limits of each rule.

## Implementation status

The implementation changes language headers and standard package references, renames ROUTE, makes implicit recursive selections explicit when their rule has no mixed recursion, and separates references from adjacent sets. Standard usage terminal paths become named pins when package provenance and the binding are known; supported standard role renames update both arguments and references. Local overrides keep their own names. With a verified, uniform series 0.3 fill environment, removed routing sets become fresh file-owned sets with the same selection and exclusions, and automatic port tags get an explicit generation rule. Included references share that entry policy; aliases, inline overrides and mixed package environments are left for contextual migration. A terminal selection used directly by TEXT becomes electrical filtering followed by SELECT-PIN; selections that escape through bindings or feed electrical consumers remain explicitly unsupported. Nonpositive directed distance intervals between individual points reverse direction and bounds; zero single-point axis distances can supply an equivalent explicit direction. Coordinates are discardable facts and do not justify choosing a direction. Crossing intervals, set extents and other undirected constraints are retained with a specific explanation. The inventory below distinguishes implemented rewrites from constructs retained for review; the latter never count as successful conversions. Known unsupported constructs receive specific notes for absolute definition coordinates, nonequivalent held collection constraints, symbol pin overrides/inclusion, local names, fixed-text point results, numeric DEVICE names, selection-dependent creation and newly meaningful markup. Usage notes describe a lost main-role point relation or a removed/changed standard definition, rather than flagging every application; target diagnostics and structural differences also prevent success. Verified ordinal standard-symbol pins receive explicit terminal maps for host assignments, direct host sets, block symbols and instance selections, matched by unique local coordinates and directions. Simple local symbols receive fresh pin names, and old PIN/fixed-TEXT result names remain bound to their input points. Known named usage-role pins can then replace usage terminal references. Inherited or displaced pins, ambiguous scopes and unknown terminal bindings remain explicit review cases. The native writer preserves the supported UTF-8 (including BOM) and Latin-1 encodings.

Known series 0.2 names and merged packages migrate only with bundled provenance and no local name capture. A path-loaded package with a standard name but different contents remains an owned package: preserve its path and package version, and upgrade its language with the file group. Numeric host-port names and the earlier Spectre transformer terminal names use their verified host positions. Named collection dependencies are traced through aliases and membership edits, so unsupported set extents and selection-fed creation receive notes even when the set is held in a variable. Alias environments, inline overrides of merged packages, definition-local object paths and numeric PIN names remain explicit review cases.

## Rewrite inventory

Sections refer to RNL 0.4. Standard package changes refer to the bundled series 0.3 and 0.4.

| Source form | Target form / handling | Class | Basis |
| --- | --- | --- | --- |
| VERSION 0.2 or 0.3 | VERSION 0.4 | Mechanical | 16.1 |
| Implicit standard series | Explicit USE-PACKAGE for `rnl-std-all` 0.4 | Mechanical | 16.3 |
| Standard USE-PACKAGE / USE-ALIASES version | Matching package in series 0.4 (`rnl-std`/`rnl-std-full` → `rnl-std-all`, `rnl-std-structures` → `rnl-std-semantics`, `rnl-zh` → `alias-zh`); preserve 1-to-1 inline overrides | Mechanical | 16.2, 16.3, 16.5 |
| Included files, own package files, inline packages | Apply rules using each source's effective language | Context | 2.8, 16.1, 16.2 |
| Reference immediately followed by a separate set | Insert whitespace so it cannot become member selection | Mechanical | 5.2.3 |
| ROUTE | ROUTE-NET with original parameters | Mechanical | 13.7 |
| Selection without recursive | Add recursive=true; report mixed false/implicit rule propagation | Context | 10.1, 10.4 |
| ORDER/DISTANCE with x or y; DISTRIBUTE axis | Only zero single-point distances gain a fixed direction; retain other axes and report the missing choice | Context | 14.3-14.5 |
| Negative DISTANCE interval | Reverse direction only when an equivalent nonnegative interval exists | Context | 14.5 |
| Nested/held sets in constraints | Retain and report centre/bounds semantics, including named held sets | Context | 5.2, 14.1 |
| Set ::position or set-wide PLACE | No direct equivalent; retain and report | Unsupported | 5.2, 6.5, 11.1 |
| ORIENT-RELATION with two sets | Retain and report: splitting changes conflict treatment | Context | 14.9 |
| Ordinary definition's absolute coordinates | Retain and report the required origin/priority decision | Context | 9.4, 12.1 |
| Usage terminal reference | Corresponding named symbol pin | Context | 6.4, 9.2.4, 12.1 |
| Usage in the member list of ALIGN, ORDER, DISTRIBUTE or DISTANCE, counted by its point | `%u::position`, the same point: in 0.4 a usage without a symbol stands for its members there (rule `usage-position-in-constraint`); the main-point ties are written with it as well | Mechanical | 6.5, 14.1 |
| Custom symbol pin binding roles by number | Name simple pins; retain unresolved numeric usage-role references for review | Context | 9.2.4, 11.3 |
| Names bound to PIN or fixed TEXT results | Preserve separately named input points; give pins and text their own names | Context | 9.2.4, 9.2.5 |
| Reused local names or names inherited from symbols | Retain and report ambiguous/reused local identities | Context | 9.2.7 |
| Repeated PIN bindings in a symbol | Retain and report displaced bindings; do not guess which points remain live | Context | 9.2.4 |
| Symbol inheritance overriding a terminal's pin | Retain and report inclusion and displaced-binding ambiguity | Context | 9.2.7 |
| SELECT-TERMINAL wired | Electrical filter followed by SELECT-PIN for direct TEXT; retain electrical consumers for review | Context | 10.1, 10.2 |
| Other terminal selections | Rewrite direct TEXT; report expanded membership for electrical consumers and traversals | Context | 10.1 |
| Selection-fed WIRE | Retain and report the required topology/routing choice | Context | 13.1, 13.7 |
| Selection-fed BUNDLE | Retain and report selection dependencies, including named aliases | Context | 11.8 |
| Literal markup gaining meaning in 0.4 | Retain and report changed rendering intent | Context | 13.6 |
| Spectre transformer default terminal names | Verified default host names become stable terminal positions | Context | 6.12 |
| Numeric DEVICE/port names | Resolve host port positions; report invalid numeric/repeated DEVICE names | Context | 6.4, 6.12 |
| Series 0.2/0.3 renamed symbols and definitions, and merged packages | Provenance-checked target names (`$MIRRORED` and `$MIRROR_PAIR` of 0.2 → `$MIRRORED_LAYOUT`, `$FLOW` → `$SIGNAL_FLOW`, `$OPAMP_5TERM` → `$OPAMP`, `$COMPARATOR_5TERM` → `$COMPARATOR`, 2-terminal controlled sources, `$CURRENT_ARROW_SYMBOL` → `$ARROW_SYMBOL`, `$ADDER`/`$MULTIPLIER` → `$*_SYMBOL`, `$DELAY_CELL_SYMBOL` → `$DELAY_CELL`) with explicit terminal maps, destination packages, and `MX` orientation composition on series 0.2 `$OPAMP`; report removed names | Context | Standard series |
| Instance of the diodes (`$DIODE`, `$ZENER_DIODE`, and Analog Canvas's LED, Schottky, Shockley and PUT) or the transformer of series 0.3, which series 0.4 draws upright | Each ORIENT that orients it is composed after the turn back (`c` becomes R90 then `c` for a diode, R270 then `c` for the transformer), so each pin stays where 0.3 drew it; an instance no ORIENT orients stays free. An ORIENT or ORIENT-RELATION of such instances together with others is left for review (rule `turned-symbol-orientation`). The comparison turns the new symbol back. | Context | 11.2 |
| Instance of Analog Canvas's UJT, which series 0.4 draws as `$UJT` with its origin on the column of b1 and b2, one unit left of where the package had it | Each coordinate a PLACE writes for its origin moves by that unit, turned as the ORIENT of 0.3 turns the instance (as R0 when none does, the orientation a viewer draws), so each pin stays where it was. A PLACE of such an instance together with others, or by a coordinate that is not a number, and an ALIGN, ORDER, DISTRIBUTE, DISTANCE, TABLE or text offset that measures from its origin along the axis it moves on, are left for review (rule `moved-symbol-origin`). The comparison moves the new symbol and its PLACE back. | Context | 11.1 |
| Standard net-part text key `name` | `netname` on bundled `$GROUND`, `$SUPPLY` and `$IO_PORT` usages (`label` for `formula` on `$TRANSCONDUCTOR` and `$DIFFERENTIAL_TRANSCONDUCTOR`), also in references to the text such as `%port1::name(label)` (rule `standard-text-key`) | Context | Standard series, 13.3 |
| `analog-canvas-extensions` 0.1–0.6 | USE-PACKAGE of the published 0.7, without `path=` (rule `canvas-package`); symbols the standard series has take its names and package (rule `canvas-name`), with an explicit terminal map on instances (rule `canvas-terminal-map`); see [the table](#analog-canvas-extensions) | Context | Standard series, 9.2.4 |
| `analog-canvas-extensions` by `path=` with other bytes than the published version, or names the file defines itself | Retain and report (`canvas-package-changed`); a name the file defines is left alone | Context | 2.8, 16.2 |
| Own TEXT with key `name`, `value` or `model` in a file that uses `rnl-std-no-labels` | Add `textvisible=true` outside symbol bodies: the 0.4 package hides labels with the style `textvisible=false`, which a later TEXT keeps unless it writes `true` (rule `no-labels-shown`) | Context | Standard series, 9.3, 13.3 |
| Standard drawing-only role p | at for the matching standard definition | Mechanical | Standard series |
| Standard role names with underscores | Their matching definition's current role names | Context | Standard series |
| DIFFERENTIAL_PAIR, CURRENT_MIRROR | Retain and report changed constraints and the required N/P choice; opt in to `_N`/`_P` when resolved model polarities agree (single-object `tail` on differential pairs) | Context | Standard series |
| MIRRORED roles a/b with a non-net singleton axis | Opt in to left/right shared with differential pairs; report direction conflicts and spacing changes | Context | Standard series |
| CHAIN, SUPPLY_HIGH, SUPPLY_LOW, DELAY_CELL (definition) | No direct replacement definition; retain and report | Unsupported | Standard series |
| Signal-flow point definitions (`INTEGRATOR`, `QUANTIZER`, `UNIT_DELAY`) | Target definitions attach `in`/`out` nets; retain point usages for review | Unsupported | Standard series |
| FLOW | Preserve the usage and restore its expanded ORDER; multi-member path groups require opt-in | Context | Standard series |
| WIRE_NET_SET modification / route | Explicit file-owned selection and exclusions, preserving user edits and ROUTE-NET | Context | Standard series, 13.7 |
| Instance with no symbol, drawn with the viewer's box of 0.3 | The same box as a symbol of the file, `$BOX_<cell>` (dashed outline, stubs, pins by terminal number with their labels, the name): the block symbol (`# @SET-SYMBOL`) of a subcircuit the file defines, else in the level of the instances, one `SET-SYMBOL` each; one box per cell; a notice says to delete the record for the default block symbol of 0.4, whose pins lie elsewhere (rule `default-box`) | Context | 9.2.6 |
| Default net-label, port-tag and 4-term MOS bulk fill | Empty `$LABEL_FILL_SET`, retain `$LABEL_TERMINAL_SET` consumers, generate port tags, and preserve 0.3 4-terminal MOS/switch symbol defaults and ground/supply bulk-pin stubs | Context | Standard series, 10.5, 12.4 |
| Ground/supply attached to a terminal | Preserve a directly written host terminal attachment through its net and explicit ALIGN x/y; report unresolved contexts | Context | 12.1, 12.3 |
| Generated numbered references | Discard validated solver records (kept and rewritten with `--keep-generated`); authored numbered references fail source validation | Context | 5.9 |
| New assertion violations or changed default fill | Report every target diagnostic and semantic difference | Verification | 7.6, 15.2-15.4 |

Additive language features (TABLE, orientation lists, assertions, solver settings, pin orientation, text rotation and math) require no rewrite unless an old spelling gains different meaning. Clarifications still participate in validation. Do not alter host netlist lines to hide a difference.

## Analog canvas extensions

A file of the language 0.3 that uses the package `analog-canvas-extensions` of a published version, 0.1 to 0.6, is known by the name and the version alone. Only when it gives the package by `path=` are the bytes of that file compared with the published version (SHA-256); a file that differs is the file's own and is left alone with the reason `canvas-package-changed`. The upgraded file uses the published 0.7, the version of the package for the language 0.4, and the `path=` is dropped with a notice. A name of the table that the file or its own package defines is the file's and keeps its meaning.

The pins of the earlier versions bind terminal numbers; those of the standard symbols and of 0.7 bind terminal names. An instance drawn with a symbol of the table gets `@SET-SYMBOL(terminals=[...])` with the map below, so that each terminal keeps its pin. A terminal that a symbol never drew keeps no pin (the controlled sources and `$CANVAS_CROSSED` draw two of four). The 4-terminal MOS symbols follow the standard 0.4 rule for the bulk pin, as other files do. The text key `name` of `$CANVAS_PORT_FILLED` is `netname`, as on the standard ports.

The table is generated from `canvas::TABLE`; a test keeps the two the same (`RNL_BLESS=1` rewrites it).

<!-- canvas-table:begin -->
| 0.1–0.6 | 0.4 | Package | Terminals |
|---|---|---|---|
| `$CANVAS_AMMETER` | `$AMMETER` | rnl-std-basic | 1→p, 2→n |
| `$CANVAS_CRYSTAL` | `$CRYSTAL` | rnl-std-basic | 1→n1, 2→n2 |
| `$CANVAS_LED` | `$LED` | rnl-std-semiconductors | 1→a, 2→k |
| `$CANVAS_MICROPHONE` | `$MICROPHONE` | rnl-std-basic | 1→p, 2→n |
| `$CANVAS_PHOTORESISTOR` | `$PHOTORESISTOR` | rnl-std-basic | 1→p, 2→n |
| `$CANVAS_POLARIZED_CAPACITOR` | `$POLARIZED_CAPACITOR` | rnl-std-basic | 1→p, 2→n |
| `$CANVAS_POTENTIOMETER` | `$POTENTIOMETER` | rnl-std-basic | 1→n1, 2→nw, 3→n2 |
| `$CANVAS_PULSE_VOLTAGE_SOURCE` | `$PULSE_VOLTAGE_SOURCE` | rnl-std-basic | 1→p, 2→n |
| `$CANVAS_PUT` | `$PUT` | rnl-std-semiconductors | 1→a, 2→g, 3→k |
| `$CANVAS_SCHOTTKY_DIODE` | `$SCHOTTKY_DIODE` | rnl-std-semiconductors | 1→a, 2→k |
| `$CANVAS_SHOCKLEY_DIODE` | `$SHOCKLEY_DIODE` | rnl-std-semiconductors | 1→a, 2→k |
| `$CANVAS_SINE_VOLTAGE_SOURCE` | `$SINE_VOLTAGE_SOURCE` | rnl-std-basic | 1→p, 2→n |
| `$CANVAS_SPEAKER` | `$SPEAKER` | rnl-std-basic | 1→p, 2→n |
| `$CANVAS_UJT` | `$UJT` | rnl-std-semiconductors | 1→e, 2→b1, 3→b2 |
| `$CANVAS_CHOPPER` | `$CHOPPER_SYMBOL` | rnl-std-functional | 1→inp, 2→inn, 3→outp, 4→outn |
| `$CANVAS_OPAMP_DIFFERENTIAL_CROSSED` | `$DIFFERENTIAL_OPAMP_TWISTED` | rnl-std-functional | 1→inp, 2→inn, 3→outp, 4→outn |
| `$CANVAS_VCVS_DIAMOND` | `$CONTROLLED_VOLTAGE_SOURCE` | rnl-std-basic | 1→p, 2→n |
| `$CANVAS_CCVS_DIAMOND` | `$CONTROLLED_VOLTAGE_SOURCE` | rnl-std-basic | 1→p, 2→n |
| `$CANVAS_VCCS_DIAMOND` | `$CONTROLLED_CURRENT_SOURCE` | rnl-std-basic | 1→p, 2→n |
| `$CANVAS_CCCS_DIAMOND` | `$CONTROLLED_CURRENT_SOURCE` | rnl-std-basic | 1→p, 2→n |
| `$CANVAS_CURRENT_SOURCE_DIAMOND` | `$CONTROLLED_CURRENT_SOURCE` | rnl-std-basic | 1→p, 2→n |
| `$CANVAS_VCVS_CIRCLE` | `$VOLTAGE_SOURCE` | rnl-std-basic | 1→p, 2→n |
| `$CANVAS_CCVS_CIRCLE` | `$VOLTAGE_SOURCE` | rnl-std-basic | 1→p, 2→n |
| `$CANVAS_VCCS_CIRCLE` | `$CURRENT_SOURCE` | rnl-std-basic | 1→p, 2→n |
| `$CANVAS_CCCS_CIRCLE` | `$CURRENT_SOURCE` | rnl-std-basic | 1→p, 2→n |
| `$CANVAS_OPAMP_LETTERED` | `$CANVAS_OPAMP_LETTERED` | analog-canvas-extensions 0.7 | 1→inp, 2→inn, 5→out |
| `$CANVAS_OPAMP_WIDE` | `$CANVAS_OPAMP_WIDE` | analog-canvas-extensions 0.7 | 1→inp, 2→inn, 5→out |
| `$CANVAS_OPAMP_WIDE_LETTERED` | `$CANVAS_OPAMP_WIDE_LETTERED` | analog-canvas-extensions 0.7 | 1→inp, 2→inn, 5→out |
| `$CANVAS_COMPARATOR_UNMARKED` | `$CANVAS_COMPARATOR_UNMARKED` | analog-canvas-extensions 0.7 | 1→inp, 2→inn, 3→out |
| `$CANVAS_OPAMP_DIFFERENTIAL_LETTERED` | `$CANVAS_OPAMP_DIFFERENTIAL_LETTERED` | analog-canvas-extensions 0.7 | 1→inp, 2→inn, 3→outp, 4→outn |
| `$CANVAS_OPAMP_DIFFERENTIAL_CROSSED_LETTERED` | `$CANVAS_OPAMP_DIFFERENTIAL_CROSSED_LETTERED` | analog-canvas-extensions 0.7 | 1→inp, 2→inn, 3→outp, 4→outn |
| `$CANVAS_OPAMP_DIFFERENTIAL_WIDE` | `$CANVAS_OPAMP_DIFFERENTIAL_WIDE` | analog-canvas-extensions 0.7 | 1→inp, 2→inn, 3→outp, 4→outn |
| `$CANVAS_OPAMP_DIFFERENTIAL_WIDE_CROSSED` | `$CANVAS_OPAMP_DIFFERENTIAL_WIDE_CROSSED` | analog-canvas-extensions 0.7 | 1→inp, 2→inn, 3→outp, 4→outn |
| `$CANVAS_OPAMP_DIFFERENTIAL_WIDE_LETTERED` | `$CANVAS_OPAMP_DIFFERENTIAL_WIDE_LETTERED` | analog-canvas-extensions 0.7 | 1→inp, 2→inn, 3→outp, 4→outn |
| `$CANVAS_OPAMP_DIFFERENTIAL_WIDE_CROSSED_LETTERED` | `$CANVAS_OPAMP_DIFFERENTIAL_WIDE_CROSSED_LETTERED` | analog-canvas-extensions 0.7 | 1→inp, 2→inn, 3→outp, 4→outn |
| `$CANVAS_VOLTAGE_AMPLIFIER_LETTERED` | `$CANVAS_VOLTAGE_AMPLIFIER_LETTERED` | analog-canvas-extensions 0.7 | 1→in, 2→out |
| `$CANVAS_INDUCTOR_LARGE` | `$CANVAS_INDUCTOR_LARGE` | analog-canvas-extensions 0.7 | 1→p, 2→n |
| `$CANVAS_SIMPLE_SWITCH` | `$CANVAS_SIMPLE_SWITCH` | analog-canvas-extensions 0.7 | 1→p, 2→n |
| `$CANVAS_SIMPLE_SPDT_SWITCH` | `$CANVAS_SIMPLE_SPDT_SWITCH` | analog-canvas-extensions 0.7 | as before |
| `$CANVAS_PORT_FILLED` | `$CANVAS_PORT_FILLED` | analog-canvas-extensions 0.7 | as before |
| `$CANVAS_AND_4INPUT` | `$CANVAS_AND_4INPUT` | analog-canvas-extensions 0.7 | 1→a, 2→b, 3→c, 4→d, 5→y |
| `$CANVAS_NAND_4INPUT` | `$CANVAS_NAND_4INPUT` | analog-canvas-extensions 0.7 | 1→a, 2→b, 3→c, 4→d, 5→y |
| `$CANVAS_OR_4INPUT` | `$CANVAS_OR_4INPUT` | analog-canvas-extensions 0.7 | 1→a, 2→b, 3→c, 4→d, 5→y |
| `$CANVAS_NOR_4INPUT` | `$CANVAS_NOR_4INPUT` | analog-canvas-extensions 0.7 | 1→a, 2→b, 3→c, 4→d, 5→y |
| `$CANVAS_XOR_4INPUT` | `$CANVAS_XOR_4INPUT` | analog-canvas-extensions 0.7 | 1→a, 2→b, 3→c, 4→d, 5→y |
| `$CANVAS_XNOR_4INPUT` | `$CANVAS_XNOR_4INPUT` | analog-canvas-extensions 0.7 | 1→a, 2→b, 3→c, 4→d, 5→y |
| `$CANVAS_NAME` | `$CANVAS_NAME` | analog-canvas-extensions 0.7 | as before |
| `$CANVAS_LABEL` | `$CANVAS_LABEL` | analog-canvas-extensions 0.7 | as before |
| `$CANVAS_TYPESETTING` | `$CANVAS_TYPESETTING` | analog-canvas-extensions 0.7 | as before |
| `$CANVAS_PARAMETER_TEXT` | `$CANVAS_PARAMETER_TEXT` | analog-canvas-extensions 0.7 | as before |
| `$CANVAS_TYPOGRAPHY` | `$CANVAS_TYPOGRAPHY` | analog-canvas-extensions 0.7 | as before |
| `$CANVAS_PARAMETER` | `$CANVAS_PARAMETER` | analog-canvas-extensions 0.7 | as before |
| `$CANVAS_UNNAMED` | `$CANVAS_UNNAMED` | analog-canvas-extensions 0.7 | as before |
| `$CANVAS_VARIANT` | `$CANVAS_VARIANT` | analog-canvas-extensions 0.7 | as before |
| `$CANVAS_CELL_NAME` | `$CANVAS_CELL_NAME` | analog-canvas-extensions 0.7 | as before |
| `$CANVAS_TOP` | `$CANVAS_TOP` | analog-canvas-extensions 0.7 | as before |
<!-- canvas-table:end -->

## Solver records

After validating the source, migration discards its solver-generated records. The report marks each affected file with `needs_solve: true`; this requests a new solve and does not count as a conversion failure. Verification compares the authored inputs on both sides, so discarded solver decisions do not create artificial differences.

`--keep-generated` (`UpgradeOptions::keep_generated`) keeps them. A record with `type="generated"` is then rewritten by the rules for authored records, its other header fields (`author`, `priority`) are kept, the file is not marked `needs_solve`, and verification compares both sides with the records of the solver read, so a placement that does not survive shows as a difference. A `sig2` hash that matched its level before the upgrade gets the input digest of the upgraded level and keeps its digest of the options; a hash that did not match, or is not `sig2`, is left as written, so a stale record stays stale. What cannot be rewritten is reported as unconverted, with the record's line, as for authored records. The option is off by default.

## Report

Each change records its rule, file, original line, before and after text. Unconverted spans record a reason. Informational notices do not fail migration by themselves; approximation records remain separate from exact changes. Target diagnostics and comparison differences are separate lists. Unsupported or different output has exit status 1; unreadable input has exit status 2 and writes nothing.

## Regression coverage

The integration tests in `../../tests/upgrade_*.rs` cover headers and includes, record-only edits, direction intervals, standard provenance, local symbols, terminal maps, legacy names, host terminals, collection dependencies, drawing selections and manual review reasons. `rnl-cli/tests/upgrade.rs` covers writing, reports, exit codes and source preservation. All fixtures are small independently authored examples.

## Default look

The 0.4 standard package draws some things differently by default. A change that only this explains goes to `default_look_changes` in the report instead of `differences`, and does not fail the upgrade: a style property such as `bold`, the place and turning of a symbol's labels, whether a text written for a label turns with its owner (`fix_rotate`, which it takes from the label), and the offset of a label that its symbol places (a position no scene gives). The text and key of a label and the number and shapes of drawing primitives stay differences. Everything else stays a difference and exits with status 1: objects, kinds, hosts, orientations, pin positions and networks, the box of a symbol, facts, wires, written positions and any change that is not sure to come from the default look. The terminal summary counts both lists.

## How the comparison pairs items

The comparison pairs items by identity, not by index. An object is known by its id; one with no name (its id is where it was written, which the upgrade moves) by what it is and its place among the like ones. A fact, wire, ROUTE or group is known by what it says about those identities, so the numbers of the facts that rank a position, `#M1:2` against `pin(#M1,1)`, a pin reached as `Pin` or `SymbolPin`, an ALIGN pair against an object drawn at a point, two ROUTEs for one net and a DISTANCE between sets (with a direction and a lower bound, as pairs of their points) are the same thing written two ways. What remains falls in lists of the report:

- `differences`: anything that appeared, disappeared or changed and that no list below explains. Exit status 1.
- `default_look_changes`: style, label place and turning, the offset of a symbol-given label, the symbol drawing of the standard package, and what a rule of the package made in either reading (the default fill: junction dots, grounds and stubs, default labels). Not counted.
- `rule_changes`: what a conversion rule of the upgrade does on purpose, today `$IO_PORT` becoming `$NET_LABEL`. Not counted; the rule is in the changes of the file.
- `discarded_placements`: with the default (without `--keep-generated`) the records of the solver are discarded; per level, how many facts only they gave. Not counted: `needs_solve` asks for the new solve that makes them again.

A symbol that a rule of the upgrade renames (a legacy standard name or a row of the canvas table) is the same symbol on both sides when its pins keep their places, so only its name and drawing box are left out of the comparison of the object. A text whose key changes from `name` to `netname` is paired by the new key on both sides, as are the references to it. The free point that a symbol-less usage gains in 0.4 is default look.

The default symbol is default look as a whole, and so is a `$BOX_<cell>` the upgrade writes for it, except where a wire goes: the place and the way of each pin of the default symbol of 0.3 that a wire goes to are compared (`wired_pins`), so a box that moves such a pin is a difference.
