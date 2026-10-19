# eda-style

[`eda-style.rnl`](eda-style.rnl) is the independent visual package `eda-style` 0.1 for RNL 0.3. It styles standard, [`eda-symbol`](../eda-symbol/README.md), and user-defined symbols without supplying their geometry. Neither EDA package imports the other; both are public extensions, outside the core's built-in packages.

## Use

To keep the EDA drawings as well as their appearance, include both packages, in this order (use `*` instead of `//` in SPICE):

```spectre
// @RNL(type="header") {
//   @VERSION("0.3");
//   @USE-PACKAGE(name="eda-symbol", version="0.1");
//   @USE-PACKAGE(name="eda-style", version="0.1")
// }
```

Use only the `eda-style` line to apply the appearance to the standard drawings or to your own symbols. Existing netlists that used the previous combined `eda-style` package should add `eda-symbol` to retain its symbol definitions and selection rules.

- **Web viewer:** in **Library → Include Public Package…**, add `eda-style` and, for EDA geometry, `eda-symbol`; then open the netlist. Importing one package does not import the other.
- **Desktop viewer:** include the required `.rnl` files through **Library → Include File…**, or add both directories through **Add PATH Directory…**.
- **Command line:** pass each required directory with `--path`:

```sh
rnl check libraries/eda-style/eda-style.rnl
rnl check libraries/eda-style/eda-style-gallery.scs --path libraries/eda-symbol --path libraries/eda-style
rnl render libraries/eda-style/eda-style-gallery.scs --path libraries/eda-symbol --path libraries/eda-style -o out/eda-style
```

## Appearance

- Symbol outlines use green hairlines (`#00cc66`, `linewidth=0`); symbol text and labels use red (`#ff0000`).
- `$PIN_SQUARE` attaches solid red 0.8 × 0.8 terminal squares through a terminal-selection set. Markers are drawn on the top layer, including on user-defined symbols; geometry files do not repeat the squares. Ground and supply usages are included explicitly. Port models use solid red polygons instead of an additional square.
- Wires and scene annotations use dark hairlines (`#1d1d1f`), with wires on the bottom layer. Net and wire labels keep the base text style.
- Labels hint `Lucida Console, Monaco, DejaVu Sans Mono, Consolas, monospace`; the viewer chooses an available font.

The viewer supports light and dark backgrounds. Green outlines and red terminals remain explicit package colors; the viewer adapts the dark base colors to a dark background.

## Examples

[`eda-style-gallery.scs`](eda-style-gallery.scs) imports both packages and places 130 visually distinct symbols in a 12-column root gallery, with six thematic subcircuits. [`examples/custom-symbol.sp`](examples/custom-symbol.sp) shows the same visual rules on a user-defined symbol, a standard resistor, ground and supply usages, and a wire. Both run with the two `--path` options above. The public package index and website package gallery read only `.rnl` files.
