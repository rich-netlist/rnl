# conformance

Conformance corpus for RNL 0.2, the language standard in [`spec/`](../../spec/README.md). The cases were written against the draft the standard was consolidated from; section numbers in file comments and diagnostics are the standard's.

| Path | Content |
| --- | --- |
| `cases/NAME.sp` | A netlist with RNL records. Files it includes or packages it names sit next to it (`04-hierarchy-lib.inc`, `11-acme-style.rnl`). |
| `cases/NAME.expected.json` | What the case must give: the effective model of every drawing level, and the level and place of every diagnostic. |
| `cases/NAME.snapshot.json` | The editor interface's snapshot of the case, with the host circuit and the model of every drawing level. |
| `equivalence/NAME.sp`, `NAME.scs` | The same circuit in SPICE and in Spectre with the same RNL. Both must give the same effective model. |

The first 15 cases were written with the Python prototype of the language and cover its features one area at a time: symbols and standard rules, hierarchy and packages, definitions, sets and selections, styles and text, buses and bundles, constraints and priorities, the header record and aliases, diagnostics, version handling, `ROUTE`, and a document written with Chinese aliases. Case 16 was added with the Rust toolkit: names equally close to an unknown one, and several overridden symbol statements, whose order once followed a hash map and changed from run to run. Its expected file records neither the messages nor the overridden statements; the tests of the WebAssembly binding build it again and again and compare whole Documents. Case 17 was added with the Rust renderer's line styles: every style on every shape kind, with and without a width, in a symbol and on a wire. Its expected file records the styles; the tests of [`rnl-view`](../../crates/rnl-view/README.md) check the strokes the renderer draws them with, such as a double line as two strokes on rectangles, circles and ellipses as on paths. Case 18 was added with the texts the shipped fonts do not map ([decision 0009](../../docs/decisions/0009-fonts-shipped-with-the-core.md)): hangul, emoji sequences, Arabic, Devanagari, Thai, combining marks and a zero width space, among characters the fonts map, in every kind of markup and alignment. Its expected file records the texts; the tests of `rnl-view` check that those characters are laid out in runs of their own, as wide as estimated, and that the SVG draws them centred and not stretched. Case 19 was added with the revised rule on cycles of sets (5.2.2): a cycle of additions, one through a removal, and one that closes in a subcircuit through package sets that hold selections. In each, the statement that closes the cycle in order of position fails, and the rest is drawn; before, the removal made the whole interpretation fail, and the sets holding selections overflowed the stack. Case 20 was added with the rule that places a text axis by axis (13.3): on each axis the PLACE or the offset written later, else the position the symbol gives a label it defines. Before, a scene offset on a label the symbol defines did nothing, a PLACE of one axis was dropped there, and an offset on one axis only was dropped wherever the other axis was free, so the texts with only a `dx` in cases 02, 08, 09 and 11 moved too. Case 21 was added with the order of the facts of one pipe (8.5): when later segments of a statement place or turn the same object again, the later segment is written later and, at equal priority, is kept. Before, the facts of the segments shared their statement's position and the first one was kept. The comment lines starting with `*!` describe each case.

## The expected files

An expected file is written by `rnl`'s conformance dump (`rnl_cli::dump::conformance`). It records:

- for each drawing level, the symbols in use with their pins, graphics and boxes; the drawn instances, bundles and usages with their symbol, position, orientation, style and layer; the wires with their routed vertices; the scene graphics; the texts with their anchors and parsed content; the lines a viewer may draw for `ROUTE`; the constraint statuses; and the objects left without a position;
- for each diagnostic shown to the user, its level, file, line and column.

Messages are not recorded: the language fixes which items fail and where, not how a tool words it (7.6). Hints are recorded too, although the standard does not require them, because this corpus also guards the reference implementation against unintended changes.

A snapshot file is what `rnl snapshot` prints for the case, the snapshot of the editor interface that a converter or an editor reads ([`rnl-view`](../../crates/rnl-view/README.md)): for each drawing level the host circuit, and the model by the ids a transaction writes back. The runtime's version, the revision and the diagnostics are left out: the first two change with the toolkit and its bundled packages rather than with the case, and the expected file holds the diagnostics. The snapshot is not a part of the language; its files guard the toolkit's interface, and the tests of the WebAssembly binding check that the WebAssembly core gives the same snapshots, the revision included.

## Running

```bash
cargo test -p rnl-cli --test conformance
```

After a change that is meant to alter a result, write the expected files again and review the difference:

```bash
RNL_BLESS=1 cargo test -p rnl-cli --test conformance
```

The data in this directory is licensed under CC0-1.0, like the rest of `tests/`.
