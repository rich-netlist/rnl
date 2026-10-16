# rnl-cli

The `rnl` command.

```text
rnl check FILE [options]          print the diagnostics; exit 1 on errors
rnl render FILE -o DIR [options]  write one SVG per drawing level
rnl dump FILE [options]           print the effective model and diagnostics as JSON
rnl view FILE [options]           print the Document a viewer shows, as JSON
rnl problem FILE [options]        print what a solver reads of each level, with its hash, as JSON
rnl solve FILE -o DIR [options]   solve each drawing level and write its record of the solver back
rnl strip-generated FILE -o OUT   write FILE without its type="generated" records

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
problem options (the levels are read without their type="generated" records):
  --level TITLE       only the level with this title: a subcircuit name, or the file name
  --solver-options S  key=value;... over the solver's defaults, as for solve: the options the hash covers
solve options (the levels are read without their type="generated" records; -o or --in-place is needed):
  -o DIR              write the files solved under DIR, named as they are relative to FILE's directory,
                      with a copy of the other files they read from there, such as includes
  --in-place          write them over the files read instead
  --level TITLE       only the level with this title
  --solver-options S  key=value;... over the solver's defaults, for tolerance, grid, objectives, beam
                      and budget, as rnl-solver's canonical text writes them; the hash covers them
  --report            print what became of every fact, not only of the facts discarded
```

Without a solver, `render` draws only what has coordinates. `check` says what is missing: after the diagnostics it prints, for each drawing level, how many objects have no position and how many constraints hold, need a solver to move something, or could not be checked. `view` prints the Document of [`rnl-view`](../rnl-view/README.md), what the desktop viewer shows, for tools and for comparing builds; with `--primitives` each level has its primitives as well, and with `--no-svg` no SVG. The standard packages in [`libraries/standard`](../../libraries/standard/README.md) are built into the command through `rnl-view`; this crate adds the files and package directories on disk (`FsProvider`).

`--draft` is for development only: the commands that interpret a file also read the files that declare the draft 0.3 of the language, which are otherwise of an unsupported version. The rules the draft changes apply only to those files, so a file of 0.2 reads the same with or without it. In such a file the record of the solver refers to the objects without a name by number, and WIRE given a wire rewrites it, which `check`, `dump`, `view` and `render` draw and `problem` gives as rewritten ([`rnl-runtime`](../rnl-runtime/README.md#wire-given-a-wire-the-draft-03)).

`problem` prints the `LevelProblem` of each level that [`rnl-runtime`](../rnl-runtime/README.md#for-a-solver) gives a solver: the objects with their slots and references, the symbols, the geometric facts in the order of 8.5, the wires, ROUTEs, groups and usages, and the level's `sig1` hash. It is for looking at what a solver will get; `--no-generated` shows how the other commands see a level before it is solved. The options part of the hash is the digest of the solver's canonical text of its options, the defaults or those `--solver-options` gives over them as for `solve`, so that the hash is the one `rnl solve` writes with the same options. A text that its owner's symbol places is `{"in_symbol": 3, "local": [1, 1]}`, with `"x"` or `"y"` and its slot when a PLACE or an offset in the scene replaces the symbol's position on that axis (13.3). A reference is the text that refers to the object, such as `"#M1"` or `"%dp::mid"`; `{"generated_by": 3, "path": "::g"}` for an object reached through the usage `usages[3]` that a rule made; or `null`. In a file of the draft 0.3 read with `--draft`, an object without a name has its number instead (5.9): `{"number": 3, "path": ""}`, and `{"number": 3, "path": "::mid"}` for a local object of the usage numbered 3. One without a name in the body of the usage `usages[0]` that a rule made has the number it takes among the objects of the record that writes that usage out: `{"generated_by": 0, "number": 1, "path": ""}` is the first of the objects that the statement writing it out makes, which are numbered after the level's `authored_numbers` and the `written_numbers` of the usages written out before it; both counts are given only in such a file. In a file of 0.2 such an object has `null` and a constraint no reference, as before, with or without `--draft`.

## solve

`solve` reads FILE as a solver reads it, without the records with `type="generated"` in any of its files (2.8), solves each drawing level with [`rnl-solver`](../rnl-solver/README.md), or only the level `--level` names, and writes the level's record of the solver at the end of its text in place of its old ones (14.8, 17): `@RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig1:...") {...}`, with the positions (those the file fixes too, but where a text follows what it is attached to, 13.3), the orientations, the wires routed for ROUTE and their junctions, and the check of its numbers (`numbering`) in a file of the draft 0.3 (5.9). A level of an included file is written into that file. The old records replaced are those that apply to the level: written in its text, and, although a solver does not write scope= (2.6), written for it with scope= in a level above, in whatever file; a record with scope= written in a level's text for a subcircuit below stays with that subcircuit. A record removed from a line whose rest holds tokens, which fail after a record (2.4), leaves `@RNL {}` before them, so that they still do and are still reported. Every file written has a header record with its version (16.1): one without gets `@RNL(type="header") { @VERSION("0.2")}`, in the version it was read in, on the line after its title, or after its first line when that is a comment of an included file, and after a comment line put first when that is a statement; a header record without VERSION gets it. Writing a level back moves the lines of the levels after it in its file, so the document is read again before the next level, with the texts written so far over the files.

A level whose file, or a file above it, declares a language version that the reading does not interpret is not solved: no RNL of that file is read, and the file is kept as it is (16.1), so nothing is written into it, and the level's line says so and why. A file of the draft 0.3 is read with `--draft`.

One of `-o DIR` and `--in-place` is needed. `-o DIR` writes each file a level was written into under DIR, by its name relative to the directory of FILE; a file outside that directory, such as one included from `../`, has no name there, which is an error, and nothing is written. So that the files under DIR read as the files solved do, the other files the document reads from the directory of FILE, such as an include of models or a package named by `path=` (16.2), are copied there as they are; a file it reads from outside that directory is not copied, and a warning names it. `--in-place` writes the files over themselves. A file is written in the encoding it was read in: UTF-8, with its byte order mark when it had one, or Latin-1.

`--solver-options` gives settings over the solver's defaults, `key=value` pairs separated by `;` as the solver's canonical text writes them, such as `beam=2;grid=none;objectives=signal_flow,area+wire_length*2;budget=candidates:16`; an unknown key, a key given twice or a value that does not read is an error. The canonical text of the options used is printed first; its digest is the second segment of the hash (17), the first being the digest of the level's input, which is what a viewer compares.

For each level, `solve` prints the facts kept and discarded in the order of 8.5, each discarded fact with the minimal set of facts kept before it that it conflicts with (every fact with `--report`); what the record cannot hold (`unsolved`, such as the position of an object without a name in a file of 0.2, which stays without a position); the ROUTEs left in parts; the objectives configured but inert; the solver's notes; its work counters and the time it took; and the hash. Then it prints the files written and the files copied. A fact is shown by its rank, what it says, its priority, where it is written and the definitions it was expanded from:

```text
level T4-miller-conflicts.sp (file in T4-miller-conflicts.sp): 38 kept, 3 discarded of 41 facts
  discarded [7] ALIGN x {#M6 #M7} (normal, T4-miller-conflicts.sp:28): it conflicts with
      [3] DISTANCE x either min=2 {#M6} {#M7} (normal, T4-miller-conflicts.sp:39)
  discarded [16] ALIGN y {#M1 #M2} (normal, rnl-std.rnl:227, from $MIRROR_PAIR): it conflicts with
      [0] the y of #M1 = 0 (high, T4-miller-conflicts.sp:37)
      [4] the y of #M2 = 3 (normal, T4-miller-conflicts.sp:38)
  ...
  work: 1408 resolve nodes, 5839 pivots, 80 candidates, 21293 A* expansions
  time: 2.119 s
  hash: sig1:2e2eb6c29c369899.e17f1522b2b059c0
wrote out/T4-miller-conflicts.sp
```

It exits with 0, also when the netlist has errors, whose number it gives on the standard error, and with 2 when a file cannot be read or written, having printed the files it wrote before, or when the arguments do not read. The library has the same steps for tools and tests: `solve::solve` gives the texts without writing a file, `solve::write` writes them where `solve::Target` says and gives what it wrote and copied (`solve::Writing`), `solve::lines` gives what the command prints of the levels, `solve::report` of one level solved.

## Tests

The crate's integration tests run the [conformance corpus](../../tests/conformance/README.md) and the writeback round trips, check the problems given to a solver over the corpus and the [solver cases](../../tests/integration/README.md), how PRIORITY goes through nested usages and turns groups off, the records written into a subcircuit with `scope=`: the names they bind, their syntax errors and their rules, which reach the levels below it, the numbers of the objects without a name in a file of the draft 0.3 with their check, and WIRE given a wire there, write a hand-made solution back and read it again, in a file of the draft with numbers, rewritten wires and the check, and in a file of 0.2 without them, with what the writer leaves out, and build the Document over the disk as the desktop viewer does: by absolute paths, with the files read by `FsProvider`.

`tests/solve.rs` runs `rnl solve` through the library on the [solver cases](../../tests/integration/README.md), T1-T6 and T2b: every level solved, written back into a temporary directory and read again, with no error, nothing unplaced, every decision of the solver in the record, every constraint `ok` but those that 8.5 discarded, which are `dropped`, and, fact by fact, the facts kept holding where the solution puts the objects and those discarded not; the hash still the same; and the text byte for byte the snapshot in `tests/integration/solver/solved` (`RNL_BLESS=1 cargo test -p rnl-cli --test solve` writes them again after a change to the solver that is meant). A solve is the same twice and the same again on its own output; T4 discards what its reference says; T3 and T4 declaring the draft 0.3, with their `$TAIL_CENTER` without a name, place its middle point by its number, so that every constraint not discarded is `ok`; a file of an unsupported version, a file of the draft read without `--draft` and a subcircuit in an included file of an unsupported version are not solved and nothing is written into them; the files read beside FILE are copied under DIR, one from outside it is named; a file is written back in its encoding; a write that fails says what it wrote before; one level alone, the levels of an included file, a file outside the directory of FILE, and the command's exit codes and output are checked too. Every case of the conformance corpus is solved, written and read again: it has the errors it has as written, without its records of the solver, no fact that its authors wrote is dropped that is not dropped there, every level solved is placed but for what its record cannot hold and has the input its hash says, and it comes out the same solved again; the labels of corpus 20, which the scene places axis by axis, read back where they were written. A file without a header gets one, and reads back without the hint; a record of the solver written with `scope=` is replaced by solving its subcircuit, in whatever file; a text that no fact places goes beside what it is attached to; and `problem` hashes the options as `solve` writes them.

See the [roadmap](../../docs/roadmap.md) for planned work.
