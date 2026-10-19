# eda-symbol

[`eda-symbol.rnl`](eda-symbol.rnl) is the public package `eda-symbol` 0.1 for RNL 0.3: 137 schematic symbol definitions and instance/usage selection rules. Drawings contain geometry, terminals and text anchors, with no fixed colors, line widths or terminal squares. The package can be used independently of [`eda-style`](../eda-style/README.md), and is not built into the core.

| Definitions | Count |
| --- | ---: |
| Standard RNL 0.3 symbol names | 93 |
| Additional sources, switches, amplifiers, RF parts and filters | 44 |
| Total symbols | 137 |

## Use

For geometry with your own appearance, include only `eda-symbol` (use `*` instead of `//` in SPICE):

```spectre
// @RNL(type="header") {
//   @VERSION("0.3");
//   @USE-PACKAGE(name="eda-symbol", version="0.1")
// }
```

Add `@USE-PACKAGE(name="eda-style", version="0.1")` after it for green hairlines, red text and automatic terminal squares. Neither package imports the other. The symbol definitions **replace the corresponding standard names**, including `$RESISTOR`, `$DIODE`, `$NMOS_4TERM` and `$D_FLIP_FLOP`, at the using level and below; hiding-definition hints are expected.

- **Web viewer:** add `eda-symbol` through **Library → Include Public Package…**; add `eda-style` separately if desired.
- **Desktop viewer:** include the `.rnl` file through **Library → Include File…**, or its directory through **Add PATH Directory…**.
- **Command line:** use `--path libraries/eda-symbol`; add `--path libraries/eda-style` for the combined appearance.

```sh
rnl check libraries/eda-symbol/eda-symbol.rnl
rnl render your-netlist.scs --path libraries/eda-symbol -o out/symbols
```

The [combined gallery and custom-symbol example](../eda-style/README.md#examples) demonstrate both packages. Netlists using the previous combined `eda-style` package should now include both package files and both `USE-PACKAGE` lines.

## Terminal orientation

The terminal directions of all 93 shared symbols match RNL's standard series 0.3. The following 18 symbols were adjusted:

| Family | Symbols | Terminal arrangement |
| --- | --- | --- |
| Diodes | `$DIODE`, `$ZENER_DIODE`, `$SCHOTTKY` | Anode left, cathode right |
| Mechanical switches | `$SWITCH`, `$CLOSED_SWITCH`, `$SPDT_SWITCH_SYMBOL`, `$SP3TSWITCH`, `$SP4TSWITCH` | Common/input terminal left, outputs right |
| Controlled switches | `$EXTERNALLY_CONTROLLED_SWITCH`, `$VOLTAGE_CONTROLLED_SWITCH` | External control enters from the left; voltage-controlled switch has horizontal terminal pairs |
| Transformer | `$TRANSFORMER` | Windings run horizontally; terminals 1 and 3, with their dots, are on the right |
| Transmission line | `$TRANSMISSION_LINE` | Signal terminals left/right; reference terminals point down |
| Port | `$IO_PORT_SYMBOL` | Body extends left of the outward-facing terminal at the origin |
| Flip-flops | `$D_FLIP_FLOP`, `$D_FLIP_FLOP_Q_ONLY`, `$D_FLIP_FLOP_WITH_RESET` | D above CLK; the Q-only output is vertically centered |
| Signal flow | `$ADDER`, `$MULTIPLIER` | Inputs from the left and bottom, output to the right |

Symbol sizes and lead lengths may differ from the standard drawings. In the six four-terminal MOS symbols, bulk terminal 4 stays at `(0, 0)` facing right, rather than the standard's `(1, 0)`. Every pin-facing box edge lies on that side's outermost terminal; there are no exceptions.
