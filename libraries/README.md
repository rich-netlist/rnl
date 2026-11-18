# libraries

The standard series stays here and is built into the core. Optional packages (`analog-canvas-extensions`, `eda-symbol`, `eda-style`, `pdk-sky130`) and the SKY130 generator moved, with their history, to [rnl-packages](https://github.com/rich-netlist/rnl-packages). Its [catalog](https://rich-netlist.github.io/rnl-packages/catalog.json) provides immutable version URLs, descriptions and SHA-256 hashes. Use **Include Public Package…** in the web viewer, or `python scripts/packages.py download` and `rnl check file.sp --path .rnl-packages/NAME/VERSION` locally.

| Path | Content |
| --- | --- |
| [`standard/`](standard/README.md) | Drafts of the standard packages of RNL, the series 0.4 in [`0.4/`](standard/0.4/): its sub-packages and symbols, the rule packages that give the elements of a netlist their symbols (one for SPICE and one for Spectre), and the Chinese alias package. Bundled with the core. The series 0.2 and 0.3 are in the cores of 0.5.x, up to 0.5.7. |

See the [roadmap](../docs/roadmap.md) for planned work.

## Package names

The `rnl-` prefix is reserved for the standard series: `rnl-std*`, and `rnl-zh` of the series 0.2 and 0.3. Extension, PDK and style packages do not use that prefix (for example `analog-canvas-extensions`, `pdk-sky130` and `eda-style`).
