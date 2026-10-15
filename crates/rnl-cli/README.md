# rnl-cli

The `rnl` command.

```text
rnl check FILE [options]          print the diagnostics; exit 1 on errors
rnl render FILE -o DIR [options]  write one SVG per drawing level
rnl dump FILE [options]           print the effective model and diagnostics as JSON
rnl view FILE [options]           print the Document a viewer shows, as JSON
rnl problem FILE [options]        print what a solver reads of each level, with its hash, as JSON
rnl strip-generated FILE -o OUT   write FILE without its type="generated" records

FILE is a SPICE netlist, a Spectre netlist (.scs) or a package file (.rnl).
options:
  --package-dir DIR   look for packages in DIR as well (16.2); repeatable
  --no-std            do not use the standard package at the root (16.3)
  --no-generated      read the netlist without its type="generated" records, as a solver does (2.8)
  --dialect NAME      spice or spectre, instead of guessing from the file
view options:
  --primitives        give each level's primitives, in scene units with y up
  --no-svg            leave out each level's SVG
problem options (the levels are read without their type="generated" records):
  --level TITLE       only the level with this title: a subcircuit name, or the file name
  --solver-options S  the solver's options the hash covers (default: tolerance=0.000001)
```

Without a solver, `render` draws only what has coordinates. `check` says what is missing: after the diagnostics it prints, for each drawing level, how many objects have no position and how many constraints hold, need a solver to move something, or could not be checked. `view` prints the Document of [`rnl-view`](../rnl-view/README.md), what the desktop viewer shows, for tools and for comparing builds; with `--primitives` each level has its primitives as well, and with `--no-svg` no SVG. The standard packages in [`libraries/standard`](../../libraries/standard/README.md) are built into the command through `rnl-view`; this crate adds the files and package directories on disk (`FsProvider`).

`problem` prints the `LevelProblem` of each level that [`rnl-runtime`](../rnl-runtime/README.md#for-a-solver) gives a solver: the objects with their slots and references, the symbols, the geometric facts in the order of 8.5, the wires, ROUTEs, groups and usages, and the level's `sig1` hash. It is for looking at what a solver will get; `--no-generated` shows how the other commands see a level before it is solved.

The crate's integration tests run the [conformance corpus](../../tests/conformance/README.md) and the writeback round trips, check the problems given to a solver over the corpus and the [solver cases](../../tests/integration/README.md), write a hand-made solution back and read it again, and build the Document over the disk as the desktop viewer does: by absolute paths, with the files read by `FsProvider`.

See the [roadmap](../../docs/roadmap.md) for planned work.
