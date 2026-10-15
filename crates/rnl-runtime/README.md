# rnl-runtime

The language core of RNL 0.2, the language standard in [`spec/`](../../spec/README.md). It takes a host document that a host adapter read (levels, instances, nets, and the records cut out of comment carriers) and returns an interpretation: the diagnostics and the effective model of every drawing level.

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
| `problem` | The solver's view of a drawing level, `LevelProblem`, and its hash `sig1` (see below). |
| `solution` | A solver's result, `Solution`, written as the one generated record of a level: `write_record`. |
| `writeback` | Removing, appending and replacing records in host text while keeping everything else byte for byte. |
| `diag` | Diagnostics with level, place and cause. |

`Options::draft`, for development only, reads the files that declare the draft 0.3 as well; the rules the draft changes apply only to those files, so a file of 0.2 reads the same with it on or off.

Without a solver, constraints are checked against the written coordinates and get the status `ok`, `failed`, `needs-solver`, `unsolved`, `dropped` or `disabled`; nothing is placed by the runtime. Two coordinates within `Options::tolerance`, 1e-6 by default, count as equal (11.7).

## For a solver

A solver must not read the records with `type="generated"` (2.8). `Options::generated` set to false leaves them out of every file of the document, with whatever they would report; `Options::solver()` does that and sets `Options::problems`, so that `Interpretation::problems` holds one `LevelProblem` per drawing level:

- the objects that have or need a position, in a fixed order: instances and bundles, virtual parts (one drawn on the point of its main object has only its orientation), points (free ones too) and texts. Each has x and y fixed by a fact or free, its orientation fixed or free, its symbol (box, pins with their directions and terminals), and how the level's text refers to it for write-back: `#M1`, `%g0`, `#M1::name`, `%dp::mid`, through the usage a rule made (10.5), or not at all (5.7);
- the geometric facts in the order of 8.5, `"disabled"` ones left out: PLACE coordinates, the coordinates written in points, TEXT `dx` and `dy`, ORIENT, ALIGN, ORDER, DISTRIBUTE, DISTANCE and ORIENT-RELATION, each with its members as points (an origin, a pin, the centre or the extent of a set), its parameters, effective priority, source and the usages it was expanded from. The index of a fact is its rank;
- the wires with their anchors, the ROUTEs with the pins to join, the groups (without those PRIORITY turns off: a set with GROUP takes PRIORITY as the group itself, 14.8), every usage with its definition and roles, the packages in effect and the level's `%` names.

`solution::write_record` turns a `Solution` (positions, orientations, routed wires, branch points, junctions) into the record the solver writes, `@RNL(type="generated", priority="high", author="…", hash="…") {…}`, numbers by `geom::fmt_num`; `writeback::replace_generated` puts it in place of the level's previous one. The usages that rules made are written out first as explicit statements under names that stay from one solve to the next, such as `%gnd_stub_M1_s = $GND_STUB @ATTACH(gap=2, priority="normal") #M1:s`, so that `%gnd_stub_M1_s::g @PLACE(…)` can place the ground; objects that cannot be referred to are reported, not written.

### The hash

`problem::sig1(problem, solver_options)` is the `hash` of the record head (17): `sig1:` and two digests of 16 hexadecimal digits joined by `.`, such as `sig1:ff3359ad9a333ccc.44669707c032ab3e`.

- The first digest is of the solver's input: a canonical text of the `LevelProblem`, which already holds the level after inheritance (the rules that flow down and take effect, the symbols chosen, the packages and their versions), built without the generated records; metadata never reaches it. Where things are written (files, lines, columns, the index of the level) is left out, so white space and comments do not change it. It is 64-bit FNV-1a over the UTF-8 text, numbers written by `fmt_num`, so it is the same on every platform. `problem::input_digest` gives it alone.
- The second digest is of the solver's options as one string, the tolerance included, such as `tolerance=0.000001`: `problem::options_digest`.

A viewer has no solver and does not know its options, so it compares only the first segment: `problem::same_input(hash, &problem)` is `Some(true)` when the input is unchanged, `Some(false)` when the solution is stale, and `None` for a hash of another algorithm, which a tool does not compare (17). The problem it compares with is built as the solver builds it, with `Options::solver()`.

The crate does no file I/O, so it builds for `wasm32-unknown-unknown`. Diagnostic messages are in English; the language fixes which item fails and where, not the wording (7.6).

See the [roadmap](../../docs/roadmap.md) for planned work.
