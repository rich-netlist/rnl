# rnl-cli

The `rnl` command.

```text
rnl check FILE [options]          print the diagnostics; exit 1 on errors
rnl render FILE -o DIR [options]  write one SVG per drawing level
rnl dump FILE [options]           print the effective model and diagnostics as JSON
rnl view FILE [options]           print the Document a viewer shows, as JSON
rnl problem FILE [options]        print what a solver reads of each level, with its hash, as JSON
rnl snapshot FILE [options]       print the editor interface's snapshot, as JSON
rnl strip-generated FILE -o OUT   write FILE without its type="generated" records
rnl write REQUEST.json -o OUT     write an edit transaction's ops into its netlist

FILE is a SPICE netlist, a Spectre netlist (.scs) or a package file (.rnl).
options:
  --package-dir DIR   look for packages in DIR as well (16.2); repeatable
  --no-std            do not use the standard package at the root (16.3)
  --no-generated      read the netlist without its type="generated" records, as a solver does (2.8)
  --dialect NAME      spice or spectre, instead of guessing from the file
  --draft             for development only: read the files that declare the draft 0.3 as well
view options:
  --primitives        give each level's primitives, in scene units with y up
  --no-svg            leave out each level's SVG
  --status            give each level its solve status: fixed, unsolved, solved or stale
problem options (the levels are read without their type="generated" records):
  --level TITLE       only the level with this title: a subcircuit name, or the file name
  --solver-options S  the solver's options the hash covers (default: tolerance=0.000001)
  --previous          give each level what its type="generated" record decided, as hints
snapshot options:
  --parts LIST        the parts to give, of drawing, host and model; host,model by default
write:
  REQUEST.json is {"dialect", "netlist", "author", "ops"}, the ops as the editor interface
  draft writes them; --dialect (spice by default) and --author NAME give the fields it leaves out
```

`snapshot` prints the snapshot of [`rnl-view`](../rnl-view/README.md#snapshots): the host circuit and the model of each drawing level, by ids a transaction can write back. `write` writes the ops into the netlist with [`rnl-edit`](../rnl-edit/README.md) and saves the result as OUT; `rnl check OUT` then says how it reads.

Without a solver, `render` draws only what has coordinates. `check` says what is missing: after the diagnostics it prints, for each drawing level, how many objects have no position and how many constraints hold, need a solver to move something, or could not be checked. `view` prints the Document of [`rnl-view`](../rnl-view/README.md), what the desktop viewer shows, for tools and for comparing builds; with `--primitives` each level has its primitives as well, with `--no-svg` no SVG, and with `--status` its solve status, `solve`: `fixed`, `unsolved`, `solved` or `stale`. The standard packages in [`libraries/standard`](../../libraries/standard/README.md) are built into the command through `rnl-view`; this crate adds the files and package directories on disk (`FsProvider`).

`--draft` is for development only: the commands that interpret a file also read the files that declare the draft 0.3 of the language, which are otherwise of an unsupported version. The rules the draft changes apply only to those files, so a file of 0.2 reads the same with or without it. In such a file the record of the solver refers to the objects without a name by number, and WIRE given a wire rewrites it, which `check`, `dump`, `view` and `render` draw and `problem` gives as rewritten ([`rnl-runtime`](../rnl-runtime/README.md#wire-given-a-wire-the-draft-03)); the hint of a fact discarded in a conflict names every set of facts taken in that it conflicts with, by the places of their statements, each of which gets a hint that names the facts discarded because of it (8.5); a file drops the standard rule package of the other dialect, `rnl-std-spectre` in a file of SPICE and `rnl-std-spice` in a file of Spectre (16.3), and T, O, U and Z have terminal names by default (6.12).

`problem` prints the `LevelProblem` of each level that [`rnl-runtime`](../rnl-runtime/README.md#for-a-solver) gives a solver: the objects with their slots and references, the symbols, the geometric facts in the order of 8.5, the wires, ROUTEs, groups and usages, and the level's `sig1` hash. It is for looking at what a solver will get; `--no-generated` shows how the other commands see a level before it is solved. A reference is the text that refers to the object, such as `"#M1"` or `"%dp::mid"`; `{"generated_by": 3, "path": "::g"}` for an object reached through the usage `usages[3]` that a rule made; or `null`. In a file of the draft 0.3 read with `--draft`, an object without a name has its number instead (5.9): `{"number": 3, "path": ""}`, and `{"number": 3, "path": "::mid"}` for a local object of the usage numbered 3. One without a name in the body of the usage `usages[0]` that a rule made has the number it takes among the objects of the record that writes that usage out: `{"generated_by": 0, "number": 1, "path": ""}` is the first of the objects that the statement writing it out makes, which are numbered after the level's `authored_numbers` and the `written_numbers` of the usages written out before it; both counts are given only in such a file. In a file of 0.2 such an object has `null` and a constraint no reference, as before, with or without `--draft`.

With `--previous`, each level also has `previous`, the decisions of its record of the solver mapped onto the problem, as hints for a warm start ([`rnl-runtime`](../rnl-runtime/README.md#the-previous-result-and-the-solve-status)), or `null` when it has none: `hash` as written, `same_input` (`null` for a hash that is not `sig1`), `numbering` (`null` when the record uses no number), `placements` with the index of each object, its position `[x, y]` when the record gives both coordinates and its orientation, `wire_routes` with the index of each wire that the record rewrites and its points as drawn, and `wires`, the wires that the record writes itself, as for ROUTE, with their nets and points. The levels are read twice, without their records of the solver and with them, or once when the file has none; the rest of the output is as without the option.

The crate's integration tests run the [conformance corpus](../../tests/conformance/README.md) and the writeback round trips, check the problems given to a solver over the corpus and the [solver cases](../../tests/integration/README.md), how PRIORITY goes through nested usages and turns groups off, the records written into a subcircuit with `scope=`: the names they bind, their syntax errors and their rules, which reach the levels below it, the numbers of the objects without a name in a file of the draft 0.3 with their check, WIRE given a wire there, the hints of its conflicts, the rule package of the other dialect that its files drop the terminal names of T, O, U and Z, and the constraints that have no solution on their own in every version, write a hand-made solution back and read it again, in a file of the draft with numbers, rewritten wires and the check, and in a file of 0.2 without them, with what the writer leaves out, give the record of the solver back as hints, by name, by number and through the usages that rules make, with the solve status of each level, and build the Document over the disk as the desktop viewer does: by absolute paths, with the files read by `FsProvider`.

See the [roadmap](../../docs/roadmap.md) for planned work.
