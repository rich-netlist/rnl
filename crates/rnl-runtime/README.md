# rnl-runtime

The language core of RNL draft 0.2. It takes a host document that a host adapter read (levels, instances, nets, and the records cut out of comment carriers) and returns an interpretation: the diagnostics and the effective model of every drawing level.

| Module | Content |
| --- | --- |
| `records` | Cutting records out of carrier lines: entries, braces, strings, the disabling `/* */` (2.3–2.5). |
| `syntax` | The record parser and its syntax tree. |
| `words` | The standard word list, read from [`data/words-0.2.json`](data/words-0.2.json): every keyword's signature, kind, attributes and style attributes (5.3). |
| `host` | The host document that adapters fill in. |
| `source` | The `Provider` trait through which the runtime asks for included files and packages, and an in-memory provider. |
| `interp` | Interpretation: headers, aliases, packages, lazy name resolution, definitions, usages, selections, symbol and style queues, geometry facts and their priorities, constraint statuses. |
| `markup` | Text markup and `${…}` attribute paths (13). |
| `model` | The effective model handed to renderers and tools. Each drawn item names the host instances it draws and the nets at its pins, and `Interpretation::refs` lists where the records refer to instances and nets, so a viewer can go from a name to its drawing and back. `Interpretation::failed` lists the statements, or parts of them, that do not take effect: failed with an error (7.2, 7.4), or overridden in a conflict (7.5): dropped in the order of 8.5, or a symbol that loses the appearance queue of every object it is given to (8.3). |
| `writeback` | Removing, appending and replacing records in host text while keeping everything else byte for byte. |
| `diag` | Diagnostics with level, place and cause. |

Without a solver, constraints are checked against the written coordinates and get the status `ok`, `failed`, `needs-solver`, `unsolved`, `dropped` or `disabled`; nothing is placed by the runtime.

The crate does no file I/O, so it builds for `wasm32-unknown-unknown`. Diagnostic messages are in English; the language fixes which item fails and where, not the wording (7.6).

See the [roadmap](../../docs/roadmap.md) for planned work.
