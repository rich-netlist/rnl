# conformance

Conformance corpus for the RNL language draft 0.2. The draft itself is not yet in [`spec/`](../../spec/README.md); section numbers in file comments and diagnostics refer to it.

| Path | Content |
| --- | --- |
| `cases/NAME.sp` | A netlist with RNL records. Files it includes or packages it names sit next to it (`04-hierarchy-lib.inc`, `11-acme-style.rnl`). |
| `cases/NAME.expected.json` | What the case must give: the effective model of every drawing level, and the level and place of every diagnostic. |
| `equivalence/NAME.sp`, `NAME.scs` | The same circuit in SPICE and in Spectre with the same RNL. Both must give the same effective model. |

The 15 cases were written with the Python prototype of the language and cover its features one area at a time: symbols and standard rules, hierarchy and packages, definitions, sets and selections, styles and text, buses and bundles, constraints and priorities, the header record and aliases, diagnostics, version handling, `ROUTE`, and a document written with Chinese aliases. The comment lines starting with `*!` describe each case.

## The expected files

An expected file is written by `rnl`'s conformance dump (`rnl_cli::dump::conformance`). It records:

- for each drawing level, the symbols in use with their pins, graphics and boxes; the drawn instances, bundles and usages with their symbol, position, orientation, style and layer; the wires with their routed vertices; the scene graphics; the texts with their anchors and parsed content; the lines a viewer may draw for `ROUTE`; the constraint statuses; and the objects left without a position;
- for each diagnostic shown to the user, its level, file, line and column.

Messages are not recorded: the language fixes which items fail and where, not how a tool words it (7.6). Hints are recorded too, although the draft does not require them, because this corpus also guards the reference implementation against unintended changes.

## Running

```bash
cargo test -p rnl-cli --test conformance
```

After a change that is meant to alter a result, write the expected files again and review the difference:

```bash
RNL_BLESS=1 cargo test -p rnl-cli --test conformance
```

The data in this directory is licensed under CC0-1.0, like the rest of `tests/`.
