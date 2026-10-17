# analog-canvas-extensions

The package `analog-canvas-extensions` 0.1, [`analog-canvas-extensions.rnl`](analog-canvas-extensions.rnl): the symbols of Analog Canvas that the standard series 0.3 does not draw. They are its appearance variants of standard symbols and its chopper. It is not part of the standard series: a file that uses it names it in its header record, `@USE-PACKAGE(name="analog-canvas-extensions", version="0.1")` (16.2), as Analog Canvas does in the files it exports. The core interface [`rnl-view`](../../crates/rnl-view/README.md) bundles it, as `<bundled>/analog-canvas-extensions/`, so the `rnl` command and the viewers find it without a package directory.

Each symbol is named `$CANVAS_` and the Analog Canvas component it draws, and comments in the file give that component and the order of its pins. Pins bind by terminal number, so each symbol fits either dialect. Coordinates are y-up, one unit a grid step, ten Analog Canvas units.

| Symbols | Count |
| --- | --- |
| Op amps and comparators: `$CANVAS_OPAMP_LETTERED`, `$CANVAS_OPAMP_WIDE`, `$CANVAS_OPAMP_WIDE_LETTERED`, `$CANVAS_COMPARATOR_UNMARKED`, and seven fully differential op amps, `$CANVAS_OPAMP_DIFFERENTIAL_…` (crossed, lettered, wide, and their combinations) | 11 |
| `$CANVAS_VOLTAGE_AMPLIFIER_LETTERED` | 1 |
| `$CANVAS_INDUCTOR_LARGE`, `$CANVAS_PULSE_VOLTAGE_SOURCE`, and the diamond sources `$CANVAS_CURRENT_SOURCE_DIAMOND`, `$CANVAS_VCVS_DIAMOND`, `$CANVAS_VCCS_DIAMOND`, `$CANVAS_CCVS_DIAMOND`, `$CANVAS_CCCS_DIAMOND` | 7 |
| `$CANVAS_SIMPLE_SWITCH`, `$CANVAS_SIMPLE_SPDT_SWITCH` (a drawing of `$SPDT_SWITCH`), `$CANVAS_PORT_FILLED` (a drawing of `$IO_PORT`) | 3 |
| `$CANVAS_CHOPPER` | 1 |
| The gates with four inputs as Analog Canvas draws them: `$CANVAS_AND_4INPUT`, `$CANVAS_NAND_4INPUT`, `$CANVAS_OR_4INPUT`, `$CANVAS_NOR_4INPUT`, `$CANVAS_XOR_4INPUT`, `$CANVAS_XNOR_4INPUT` | 6 |

The file is written by `build-std.mjs` in rnl-dev, with the standard series 0.3, from Analog Canvas's definitions and its symbol list; a symbol that Analog Canvas adds to the list is written the same way and taken here, such as the diamond VCVS and CCVS (Analog Canvas #1740). It is written in the language 0.3, as the series is. The six gates keep Analog Canvas's spacing of their inputs, `y` = 1.2, 0.4, −0.4 and −1.2, while the standard gates with four inputs of `rnl-std-logic` have theirs on integer points. Checked on its own, the package gives a hint for each of those 24 pins (9.2.4) and nothing else.

Like the rest of `libraries/`, it is under CC0-1.0 ([`REUSE.toml`](../../REUSE.toml)).
