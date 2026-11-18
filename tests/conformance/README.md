# conformance

Conformance corpus for RNL 0.4, the only version of the language that the toolkit reads. Section numbers in file comments and diagnostics are those of the standard 0.4.

| Path | Content |
| --- | --- |
| [`cases-0.4/from-0.2/`](cases-0.4/README.md), [`cases-0.4/from-0.3/`](cases-0.4/README.md) | Netlists with RNL records, `NAME.sp` or `NAME.scs`, each with `NAME.expected.json` and `NAME.snapshot.json`. Files a case includes or packages it names sit next to it. The two directories hold the cases first written for 0.2 and for 0.3, written in 0.4 under their old names, so that each case's purpose can be traced; a few declare a version that the toolkit does not read, and are read as such files are (16.1). |
| [`equivalence-0.4/NAME.sp`, `NAME.scs`](equivalence-0.4/) | The same circuit in SPICE and in Spectre with the same RNL. Both must give the same effective model. |

[`cases-0.4/README.md`](cases-0.4/README.md) says how the cases were carried over to 0.4. The comment lines starting with `*!` describe each case. The corpora of 0.2 and 0.3 that they replace are in the cores of 0.5.x, up to 0.5.7.

## The expected files

An expected file is written by `rnl`'s conformance dump (`rnl_cli::dump::conformance`). It records:

- for each drawing level, the symbols in use with their pins, graphics and boxes; the drawn instances, bundles and usages with their symbol, position, orientation, style and layer; the wires with their routed vertices; the scene graphics; the texts with their anchors and parsed content; the constraint statuses; and the objects left without a position;
- for each diagnostic shown to the user, its level, its kind (`code`, the number of the toolkit's list of diagnostics such as `"RNL3120"`, and `name`, its short name), file, line and column.

Messages are not recorded: the language fixes which items fail and where, not how a tool words it (7.6). The kind is recorded, so that a test tells the diagnostics apart by kind, not by place alone. Hints are recorded too, although the standard does not require them, because this corpus also guards the reference implementation against unintended changes.

A snapshot file is what `rnl snapshot` prints for the case, the snapshot of the editor interface that a converter or an editor reads ([`rnl-view`](../../crates/rnl-view/README.md)): for each drawing level the host circuit, and the model by the ids a transaction writes back. The runtime's version, the revision and the diagnostics are left out: the first two change with the toolkit and its bundled packages rather than with the case, and the expected file holds the diagnostics. The snapshot is not a part of the language; its files guard the toolkit's interface, and the tests of the WebAssembly binding check that the WebAssembly core gives the same snapshots, the revision included.

## Running

```bash
cargo test -p rnl-cli --test cli conformance::
```

After a change that is meant to alter a result, write the expected files again and review the difference:

```bash
RNL_BLESS_0_4=1 cargo test -p rnl-cli --test cli conformance::
```

The data in this directory is licensed under CC0-1.0, like the rest of `tests/`.
