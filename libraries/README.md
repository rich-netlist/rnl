# libraries

External data libraries, separate from viewer code.

| Path | Content |
| --- | --- |
| [`standard/`](standard/README.md) | Drafts of the standard packages of RNL, a directory for each version of the standard series: [`0.3/`](standard/0.3/), the series that a file of the language 0.3 uses, and [`0.2/`](standard/0.2/), the series of files of 0.2. Each has its sub-packages and symbols, the rule packages that give the elements of a netlist their symbols (in 0.3, one for SPICE and one for Spectre), and the Chinese alias package. Bundled with the core. |
| [`analog-canvas-extensions/`](analog-canvas-extensions/README.md) | The package `analog-canvas-extensions` 0.1: the symbols of Analog Canvas that the standard series 0.3 does not draw. Not standard or built-in; a file names it with USE-PACKAGE and supplies it in the Library, pool or a search path. |
| [`pdk/sky130/`](pdk/sky130/README.md) | The package `pdk-sky130` 0.1: terminal declarations (`@DEVICE`), multi-terminal symbols, and model-to-symbol selection rules for the SkyWater SKY130 PDK (`sky130_fd_pr`), in both SPICE and Spectre dialects, plus example schematics and a guide to writing packages for other PDKs. |
| [`eda-symbol/`](eda-symbol/README.md) | The public extension package `eda-symbol` 0.1: 137 style-independent schematic symbols and selection rules, with standard-compatible pin directions. Use it on its own or combine it with `eda-style`. |
| [`eda-style/`](eda-style/README.md) | The independent visual package `eda-style` 0.1: green hairlines, red symbol text, automatic red terminal squares and monospaced labels. Works with standard, `eda-symbol` or user-defined symbols; includes a gallery using both EDA packages. Load either package through Include Public Package, Library, or `--path`. |

See the [roadmap](../docs/roadmap.md) for planned work.

## Package names

The `rnl-` prefix is reserved for the standard series: `rnl-std*` and `rnl-zh`. Extension, PDK and style packages do not use that prefix (for example `analog-canvas-extensions`, `pdk-sky130` and `eda-style`).
