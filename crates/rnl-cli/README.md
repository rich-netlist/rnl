# rnl-cli

The `rnl` command. Download a prebuilt executable from the core Release (`core-vX.Y.Z`),
or build from source. The CLI and npm package share the workspace version; Viewer and
language versions are independent. See the [download/checksum guide](../../skills/rnl/references/tools.md#the-rnl-command)
and [release procedures](../../docs/releases.md). Archives include MIT, CC0 and OFL licences.
`rnl --version` (also `-V`) prints the core version and build commit, or `unknown` when
built from a source archive without Git metadata.

```text
rnl --version                   print the core version and build commit
rnl check FILE [options]          print the diagnostics; exit 1 on errors
rnl render FILE -o DIR [options]  write one SVG per drawing level
rnl dump FILE [options]           print the effective model and diagnostics as JSON
rnl view FILE [options]           print the Document a viewer shows, as JSON
rnl explain FILE REF [options]    print the usages and groups the object REF is in, such as #M1, &vdd,
                                  #M5:d or %dp1, written as RNL refers to it
rnl problem FILE [options]        print what a solver reads of each level, with its hash, as JSON
rnl solve FILE -o DIR [options]   solve each drawing level and write its record of the solver back
rnl snapshot FILE [options]       print the editor interface's snapshot, as JSON
rnl catalog [options]             print the symbols, definitions and sets of the packages, as JSON
rnl strip-generated FILE -o OUT   write FILE without its type="generated" records
rnl write REQUEST.json -o OUT     write an edit transaction's ops into its netlist

FILE is a SPICE netlist, a Spectre netlist (.scs) or a package file (.rnl), which is checked on its
own; catalog takes none. A package file is one of the format rnl: a .rnl file, or a file of another
suffix whose first record, at the start of a line, is a header record with @PACKAGE.
options:
  --path DIR          search packages and bare include filenames here; repeatable
  --package-dir DIR   look for packages in DIR as well (16.2), its .rnl files; repeatable
  --no-std            do not use the standard package at the root (16.3)
  --std-version V     use rnl-std V at the root, whatever the files' language version, such as 0.2
                      to read a file of 0.3 with the series 0.2; a version pinned in a header
                      record still comes first (16.3), and V not found is an error only for the
                      files that do not pin one. rnl solve does not take it
  --no-generated      read the netlist without its type="generated" records, as a solver does (2.8)
  --dialect NAME      spice, spectre, rnl or text, instead of guessing from the file
view options:
  --primitives        give each level's primitives, in scene units with y up
  --no-svg            leave out each level's SVG
  --status            give each level its solve status: fixed, unsolved, solved or stale
  --bind SUBCKT       show the parameters of the subcircuit SUBCKT with the values --param gives
  --param NAME=VALUE  a value for a parameter of the subcircuit --bind names, as text; repeatable.
                      Only the Document's lists of parameters change; nothing drawn does
render options (inspection only; all disabled by default):
  --usages MODE       usage frames: semantic (no drawing of their own) or all
  --currents          show in/out terminal currents, including explicit subcircuit port symbols
  --usage-roles       show role labels on enabled usage frames
  --overlay-style FILE  JSON: palette, width, dashes, current_color; independent of RNL styles
problem options (the levels are read without their type="generated" records):
  --level TITLE       only the level with this title: a subcircuit name, or the file name
  --solver-options S  key=value;... over the solver's defaults, as for solve: the options the hash covers
  --previous          give each level what its type="generated" record decided, as hints
solve options (the levels are read without their type="generated" records; -o or --in-place is needed):
  -o DIR              write the files solved under DIR, named as they are relative to FILE's directory,
                      with a copy of the other files they read from there, such as includes
  --in-place          write them over the files read instead
  --level TITLE       only the level with this title
  --solver-options S  key=value;... over the solver's defaults, for tolerance, grid, objectives, beam,
                      budget and time_limit, as rnl-solver's canonical text writes them; the hash
                      covers them. time_limit is the seconds the resolution of a level may spend on
                      checking (8.5), 5 by default, or none for no limit: a fact whose check does not
                      finish within it is discarded with a warning, and the result may then differ
                      from one run to the next; when the conflict groups of a fact are not all found
                      within it, its hint says that the list is not complete
  --report            print what became of every fact, not only of the facts discarded, and the hints
                      of the facts of priority low and of the statements only they lose (8.5), whose
                      minimal conflict groups the solver then seeks all, not only the first
  a fact that has no solution on its own contradicts itself and fails (8.5): solve prints an error line
  for it, and solves the level and writes it back without it
  solve exits with 1 when an error was reported, once the files are written: one of the netlist as a
  solver reads it, whose number it gives on the standard error as rnl check --no-generated counts
  them, or one of the solver
  the solver is a preview, whose layouts may be poor: solve and preview say so in a line on the
  standard error each time they solve; what they print on the standard output and write is the same
saving options (solve and strip-generated only):
  --encoding MODE     preserve (default) or utf8; UTF-8 adds a BOM only to protect leading U+FEFF
                      copied includes that are not rewritten retain their original bytes
snapshot options:
  --parts LIST        the parts to give, of drawing, host and model; host,model by default
catalog options (the packages are those of --package-dir and the bundled ones, each read as rnl
check reads a package file, with --no-std and --std-version):
  --package NAME      only the packages of this name; repeatable
  --all-versions      every version of each package, not only the newest
write:
  REQUEST.json is {"dialect", "netlist", "author", "ops"}, the ops as the editor interface
  draft writes them; --dialect (spice by default) and --author NAME give the fields it leaves out
```

`explain` prints the usages and groups that the object REF is a member of. REF is written as RNL refers to the object: `#M1`, `&vdd`, `#M5:d` or `#M5:1` for a terminal, `%dp1` or `%core` for a usage or group, or `@file:line:col` for one without a name. It is looked for in every level, or only in the level `--level TITLE` names; each level that has it is printed in turn. There is one line for each usage, which says:
- the roles the object fills, of which usage, of which definition: `in_p of usage %dp1 (definition $DIFFERENTIAL_PAIR)`, with `one of {members}` for a member of a set role and `(through terminal s)` for an instance whose terminal fills the role;
- where the statement is;
- where the usage comes from: `written`, `made by the rule at FILE:LINE` (of a package, `made by the rule of rnl-std-fill at …`), `made by the body of %vdd1`, the usage whose definition made it, or `written back by a solver`;
- when it does not take effect, its state in capitals, with the reason: `FAILED: …`, `OVERRIDDEN: …` or `DISABLED`. A usage in effect says nothing more.

The usages that contain that usage follow, indented one step for each level up the chain, each line naming the usage it is about: `%dp1 is in turn input of usage %stage1 (definition $CASCADE)`. A usage that is a member of two usages has both, at the same indent. A chain that comes back to a usage already on it stops there, with `the chain loops back to %u`. An application that failed, and so made no usage, is listed with its error.

The groups (11.5) are listed the same way, after the usages, and the first line counts them apart: `#R1 in amp: 1 usage, 2 groups`. A group's line says `member of group %stage1`, or `member (through terminal d) of group …` for a terminal written into it, and a group's own lines say `within group %core`: a reference to a set is spread in a set (5.2), so one group is within another when all its members are among the other's, and only the smallest such groups are named. A group is in effect or `DISABLED`, which PRIORITY makes it; it is never failed or overridden. A group without a name goes by where GROUP is written, `@file:line:col`.

```text
$ rnl explain groups.sp '#R1'
#R1 in groups.sp: 1 usage, 4 groups
  left of usage %dp (definition $PAIR) · groups.sp:11 · written
    %dp is in turn a member of group %blocks · groups.sp:16 · written
  member of group %stage1 · groups.sp:12 · written
    %stage1 is in turn within group %core · groups.sp:15 · written
  member of group %pair · groups.sp:14 · written
    %pair is in turn within group %stage1 · groups.sp:12 · written
      %stage1 is in turn within group %core · groups.sp:15 · written
  member of group %core · groups.sp:15 · written
  member of group @groups.sp:20:5 · groups.sp:20 · written
    @groups.sp:20:5 is in turn within group %core · groups.sp:15 · written
``` A REF that names no object is an error that lists up to five close names. `--json` prints the same as `rnl-view`'s `MemberOf` entries, each with `outer`, the usages that contain it. The lists are [`rnl-view`'s](../rnl-view/README.md#the-usages-of-an-object) `Object.usages`. On the five-transistor OTA of the site's examples:

```text
$ rnl explain site/examples/ota5t.sp '#M1'
#M1 in ota5t: 2 usages
  in_p of usage %dp1 (definition $DIFFERENTIAL_PAIR) · ota5t.sp:12 · written
  a of usage %sym12 (definition $MIRRORED) · ota5t.sp:19 · written
```

`snapshot` prints the snapshot of [`rnl-view`](../rnl-view/README.md#snapshots): the host circuit and the model of each drawing level, by ids a transaction can write back. `catalog` prints the catalog of [`rnl-view`](../rnl-view/README.md#catalog): the symbols, with their pins, boxes and labels, the ordinary definitions and the sets of the bundled packages and of those in the directories `--package-dir` gives, each package file read as `rnl check PACKAGE.rnl` reads it; of each name the newest version, every version with `--all-versions`, and only the names `--package` gives, when it gives any. It is the catalog the WebAssembly core gives for a catalog request. `write` writes the ops into the netlist with [`rnl-edit`](../rnl-edit/README.md) and saves the result as OUT, reading the netlist under the name of OUT when the request names no `file`, so that a Spectre netlist saved without the `.scs` suffix gets its header where Spectre reads it (2.2); `rnl check OUT` then says how it reads.

Without a solver, `render` draws only what has coordinates. `check` says what is missing: after the diagnostics it prints, for each drawing level, how many objects have no position and how many constraints hold, need a solver to move something, or could not be checked. `view` prints the Document of [`rnl-view`](../rnl-view/README.md), what the desktop viewer shows, for tools and for comparing builds; with `--primitives` each level has its primitives as well, with `--no-svg` no SVG, and with `--status` its solve status, `solve`: `fixed`, `unsolved`, `solved` or `stale`. The standard packages in [`libraries/standard`](../../libraries/standard/README.md) are built into the command through `rnl-view`; this crate adds the files and package directories on disk (`FsProvider`).

The commands read the language versions 0.2 and 0.3 (16.1): a file in the version it declares, an included file without a header record in that of the file that includes it, and a main file without one in 0.3, the tool's default. A file of any other version is of an unsupported version. The rules that 0.3 changes apply only to the files of 0.3, so a file that declares 0.2 reads as it did. In a file of 0.3 the record of the solver refers to the objects without a name by number, and WIRE given a wire rewrites it, which `check`, `dump`, `view` and `render` draw and `problem` gives as rewritten ([`rnl-runtime`](../rnl-runtime/README.md#wire-given-a-wire-the-draft-03)); the hint of a fact discarded in a conflict names every set of facts taken in that it conflicts with, by the places of their statements, each of which gets a hint that names the facts discarded because of it (8.5); a file drops the standard rule package of the other dialect, `rnl-std-spectre` in a file of SPICE and `rnl-std-spice` in a file of Spectre (16.3), and T, O, U and Z have terminal names by default (6.12).

`problem` prints the `LevelProblem` of each level that [`rnl-runtime`](../rnl-runtime/README.md#for-a-solver) gives a solver: the objects with their slots and references, the symbols, the geometric facts in the order of 8.5, the wires, ROUTEs, groups and usages, the nets with the pins, wires and ROUTEs on each and the nets of each virtual part and text, and the level's `sig2` hash. It is for looking at what a solver will get; `--no-generated` shows how the other commands see a level before it is solved. The options part of the hash is the digest of the solver's canonical text of its options, the defaults or those `--solver-options` gives over them as for `solve`, so that the hash is the one `rnl solve` writes with the same options. A text that its owner's symbol places is `{"in_symbol": 3, "local": [1, 1]}`, with `"x"` or `"y"` and its slot when a PLACE or an offset in the scene replaces the symbol's position on that axis (13.3). A reference is the text that refers to the object, such as `"#M1"` or `"%dp::mid"`; `{"generated_by": 3, "path": "::g"}` for an object reached through the usage `usages[3]` that a rule made; or `null`. In a file of 0.3, an object without a name has its number instead (5.9): `{"number": 3, "path": ""}`, and `{"number": 3, "path": "::mid"}` for a local object of the usage numbered 3. One without a name in the body of the usage `usages[0]` that a rule made has the number it takes among the objects of the record that writes that usage out: `{"generated_by": 0, "number": 1, "path": ""}` is the first of the objects that the statement writing it out makes, which are numbered after the level's `authored_numbers` and the `written_numbers` of the usages written out before it; both counts are given only in such a file. In a file of 0.2 such an object has `null` and a constraint no reference, as before.

With `--previous`, each level also has `previous`, the decisions of its record of the solver mapped onto the problem, as hints for a warm start ([`rnl-runtime`](../rnl-runtime/README.md#the-previous-result-and-the-solve-status)), or `null` when it has none: `hash` as written, `same_input` (`null` for a hash that is neither `sig2` nor `sig1`), `numbering` (`null` when the record uses no number), `placements` with the index of each object, its coordinates `x` and `y`, each `null` when the record does not give it, and its orientation, `wire_routes` with the index of each wire that the record rewrites and its points as drawn, and `wires`, the wires that the record writes itself, as for ROUTE, with their nets and points. The levels are read twice, without their records of the solver and with them, or once when the file has none; the rest of the output is as without the option.

## preview

`preview` solves FILE as `solve` does, with its options, and writes nothing: it prints, as JSON, `{"document": …, "report": …}`, the Document of the files as solved, the texts of the solve held over the files, and the report of what became of each fact. The desktop viewer's Solve menu shows the same ([decision 0010](../../docs/decisions/0010-desktop-viewer-solves-as-a-preview.md)), which [`rnl_cli::preview`](src/preview.rs) gives:

- **The report, per level:** every fact in the order of 8.5, `kept`, `discarded` with the groups of facts it conflicts with, `undecided` within the time limit, or `failed`; the hints by statement; the solver's errors, warnings and notes; what the record cannot hold; and the ROUTEs left in parts.
- **Located:** each line has `at`, the index of a file of the Document and the line there, found by reading the texts solved again as a solver reads them. A fact that a rule of a package makes has none.
- **The notes:** the lines of the files raised to 0.3 and of the levels not solved.
- **Saving it:** `preview::save_copy` saves a copy of a solve, refusing any place over a file that was read. It saves one file when the solve writes one and reads no other (`preview::one_file`). Otherwise it saves a folder, as `solve -o DIR` writes it.
- **Watching it:** `solve::solve_watched` tells a watcher of each level before it is solved, and stops at its word. `preview::StoppableClock` is past the time limit once stopped, so that the level under way stops checking its facts.

## solve

The solver is a preview, version 0.1, and its layouts may be poor. Each time `solve` or `preview` solves, it says so in one line on the standard error, before anything else it prints:

```text
note: the solver is a preview, and its layouts may be poor; for comparison, have the model write the exact RNL placement directly
```

What they print on the standard output, `preview`'s JSON among it, and the files `solve` writes do not change.

`solve` reads FILE as a solver reads it, without the records with `type="generated"` in any of its files (2.8), solves each drawing level with [`rnl-solver`](../rnl-solver/README.md), or only the level `--level` names, and writes the level's record of the solver at the end of its text in place of its old ones (14.8, 17): `@RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:...") {...}`, with the positions (those the file fixes too, but where a text follows what it is attached to, 13.3, and but what the solver leaves free: a text that no fact reaches attached to a wire, a net or anything else the solver has no point for, such as a graphic or a terminal without a pin, named or not, which the viewer places next to what it is attached to, 13.3, and a supply, ground or port symbol without a name that no fact reaches, on a net that no ROUTE or wire written draws, which the viewer leaves unplaced, 11.1), the orientations, the wires routed for ROUTE and their junctions, and the check of its numbers (`numbering`) in a file of the draft 0.3 (5.9). A level of an included file is written into that file. The old records replaced are those that apply to the level: written in its text, and, although a solver does not write scope= (2.6), written for it with scope= in a level above, in whatever file; a record with scope= written in a level's text for a subcircuit below stays with that subcircuit. A record removed from a line whose rest holds tokens, which fail after a record (2.4), leaves `@RNL {}` before them, so that they still do and are still reported. Every file written has a header record with its version (16.1): one without gets `@RNL(type="header") { @VERSION("0.3")}`, in the version it was read in, or, read in 0.2, raised with the pin of the series 0.2 (see below), on the line after its title, or after its first line when that is a comment of an included file, and after a comment line put first when that is a statement; a header record without VERSION gets it. The header record stays the first record of its file (2.8): one written after a subcircuit that gets its record, which would come before it, moves where a new header goes (the draft 0.3, 16.3 and 2.2), on line 2 after the title, or after the `simulator lang=spectre` that is the first statement of a file in `//`, and otherwise above it in `*`, with its version and the indentation of its lines; in a file read only through library sections it moves above the subcircuit instead, as line 2 is not read there. It moves before any level of its file is written, so that every record written into the file, that of the file level solved before the subcircuit too, follows the carrier it has there, `*` above the switch, and the next solve writes the same; a record replaced leaves no line that holds only its marker. Writing a level back moves the lines of the levels after it in its file, so the document is read again before the next level, with the texts written so far over the files. A record that would go into a region written to be encrypted (`.prot … .unprot`, `protect … unprotect`), where it is gone once the region is encrypted (2.2), goes before the region; a level whose own text lies in one, as a subcircuit of an encrypted library does, is solved but not written, and its report has a note that says so. In a `simulator lang=spice` section of a Spectre file, where `//` is no comment, a record is written with `*` (2.2). A tool writes only 0.3 (16.1): a file of 0.2 that `solve` writes a record into, the file of a level solved, is raised to 0.3; one that only loses a record of the solver written there with scope= is written as it is otherwise and keeps its version, as with `strip-generated`. Its header record says `@VERSION("0.3")`, followed by `@USE-PACKAGE(name="rnl-std", version="0.2")` unless the file or a file that includes it pins the standard package, so that it keeps the series 0.2 it was read with, its names and its drawing (16.3), and its levels are solved again as they read in 0.3; the output says so in a line of its own, `t.sp: raised from 0.2 to 0.3, as a tool writes only 0.3 (16.1); …`. A library section read is a file of its own (2.8): of two sections of one library, only one written into is raised, its line naming the section, `lib.inc (new): raised from 0.2 to 0.3 …`, and the text of the other stays as it is. A name in a file raised that is spelled like `numbering`, the word that 0.3 adds, gets a warning, as the raise may change its meaning, and the file is raised all the same; so does an alias that its header record imports from an alias package with USE-ALIASES, which in 0.3 is an error as one the file declares (16.5). A file it includes that writes no VERSION takes its version (16.1), so it reads in 0.3 too: a warning for each such file, `child.inc: it writes no VERSION and takes that of t.sp: it reads in 0.3 too, its text unchanged …`, and one for each name in it spelled like `numbering`; its text is not changed, and `@VERSION("0.2")` in its header record keeps it at 0.2. A file whose header record cannot say 0.3, such as a library section without one, stays at 0.2, with a warning, and what only 0.3 can write is unsolved. `strip-generated` writes nothing into a file but removes the records of the solver, and raises no version.

A level whose file, or a file above it, declares a language version that the reading does not interpret is not solved: no RNL of that file is read, and the file is kept as it is (16.1), so nothing is written into it, and the level's line says so and why. A file of 0.3 is read as every file is.

One of `-o DIR` and `--in-place` is needed. `-o DIR` writes each file a level was written into under DIR, by its name relative to the directory of FILE; a file outside that directory, such as one included from `../`, has no name there, which is an error, and nothing is written. So that the files under DIR read as the files solved do, the other files the document reads from the directory of FILE, such as an include of models or a package named by `path=` (16.2), are copied there as they are; a file it reads from outside that directory is not copied, and a warning names it. `--in-place` writes the files over themselves. A file is written in the encoding it was read in: UTF-8, with its byte order mark when it had one, or Latin-1. When a Latin-1 file would need a character Latin-1 cannot hold, nothing is written and the error names the character; `--encoding utf8` writes such files as UTF-8 instead. `strip-generated` follows the same rule ([output encoding](../../docs/output-encoding.md)).

`--solver-options` gives settings over the solver's defaults, `key=value` pairs separated by `;` as the solver's canonical text writes them, such as `beam=2;grid=none;objectives=signal_flow,area+wire_length*2;budget=candidates:16`; an unknown key, a key given twice or a value that does not read is an error. The canonical text of the options used is printed first; its digest is the second segment of the hash (17), the first being the digest of the level's input, which is what a viewer compares.

`time_limit` limits the time the resolution of each level spends on checking for a solution, as the draft 0.3 allows (8.5): 5 seconds by default, measured on the wall clock (`solve::WallClock`), another number of seconds, or `none` for no limit. It covers the checks of the facts, each on its own first and then in the order of 8.5, and the search for all their minimal conflict groups, nothing else: the witness taken as written, the search, the coordinates and the routing are counted in work. When the groups of a fact are not all found within it, its line gives those found, each minimal, and ends with `(the list is not complete: the time limit was reached)`, as does the head of the hints by statement. A fact whose check does not finish within it is discarded undecided, and so is every later fact that needs a check, each with a warning (7.6); a later fact that the placement reached by then already satisfies is kept, and every fact kept still holds. A fact whose check on its own does not finish is undecided too, not failed. The positions are still taken from the facts kept with their constants as written, which is not timed: a `PLACE(5.5, 0)` decided before the time ran out is written as `5.5`, not a tolerance off, so that a viewer reading the record back does not drop the author's own PLACE. The result may then differ from that of 8.5, and from one run to the next; with `time_limit=none`, or whenever no check reaches the limit, it is that of 8.5, the same on every run. The limit is part of the options the hash covers, so a record solved without one has another hash than one solved with the default. `solve::solve_with_clock` takes the clock from the caller, such as a test's clock that advances by a step at each reading.

For each level, `solve` prints the facts kept and discarded in the order of 8.5 (every fact with `--report`), each discarded fact with its hint (the draft 0.3, 8.5): every minimal group of facts kept before it that it conflicts with, such as `it conflicts with [2] and [4]; or with [2] and [6]`, each fact of them described once below, or, when its check did not finish within the time limit, `not decided within the time limit of the solver (8.5)`; then the hints by statement, each statement whose facts are in such a group with the facts discarded because of it, such as `T4-miller-conflicts.sp:39 [3] DISTANCE x either min=2 {#M6} {#M7} took part in discarding [7]`, a statement in the body of a definition named by its usage, where the usage is written, with the roles as the level writes them, a role that cannot be referred to left out, and the line of the definition (`the usage of $MIRROR_PAIR at T4-miller-conflicts.sp:22 with #M1(a) #M2(b) %ax(axis) (rnl-std-layout.rnl:10 of its definition) ...`); for a statement in a definition used in the body of another, the line is of the innermost and every definition of the chain is named, as for a fact (`%o1, the usage of $OUTER with #R1(a) #R2(b) (p.sp:6 of $PAIR, from $OUTER in $PAIR) ...`). Discarding a fact of priority low is expected, and its hints may be left out (7.6): without `--report` its line says only that, and a statement that only such facts lose is not listed; nor does the solver seek its groups beyond the first, which its deletion filter gives (`SolverOptions::hints_for_low`), and a note says that the groups of such facts are listed only with `--report`, as enumerating them can cost far more than the rest of the resolution: about 9 million resolve nodes for the low ORDER of corpus 03; the solver's errors, each on a line of its own beginning with `error:`, one for each fact that has no solution on its own, which contradicts itself and fails (the draft 0.3, 8.5; 7.6), such as `  error: the fact [3] ALIGN (amp.sp:12) contradicts itself and fails (8.5, 7.6)`: it takes no part in the resolution, the level is solved and written back without it, the head of the level says how many facts failed, and `--report` lists it as `failed`; the solver's warnings, each on a line of its own beginning with `warning:`, such as `  warning: the fact [12] ALIGN (amp.sp:40) was not decided within the time limit of 5 s and is discarded (8.5)`; what the record cannot hold (`unsolved`, such as the position of an object without a name in a file of 0.2, which stays without a position); the ROUTEs left in parts; the objectives configured but inert; the solver's notes; its work counters and the time it took; and the hash. Then it prints the files written and the files copied. A fact is shown by its rank, what it says, its priority, where it is written and the definitions it was expanded from:

```text
level T4-miller-conflicts.sp (file in T4-miller-conflicts.sp): 38 kept, 3 discarded of 41 facts
  discarded [7] ALIGN x {#M6 #M7} (normal, T4-miller-conflicts.sp:28): it conflicts with [3]
      [3] DISTANCE x either min=2 {#M6} {#M7} (normal, T4-miller-conflicts.sp:39)
  discarded [16] ALIGN y {#M1 #M2} (normal, rnl-std-layout.rnl:10, from $MIRROR_PAIR): it conflicts with [0] and [4]
      [0] the y of #M1 = 0 (high, T4-miller-conflicts.sp:37)
      [4] the y of #M2 = 3 (normal, T4-miller-conflicts.sp:38)
  discarded [40] ALIGN y {#VIP #VIN} (low, T4-miller-conflicts.sp:31): of priority low, which is expected; --report lists what it conflicts with (8.5)
  statements whose facts took part in discarding (8.5):
    T4-miller-conflicts.sp:37 [0] the y of #M1 = 0 took part in discarding [16]
    T4-miller-conflicts.sp:38 [4] the y of #M2 = 3 took part in discarding [16]
    T4-miller-conflicts.sp:39 [3] DISTANCE x either min=2 {#M6} {#M7} took part in discarding [7]
  ...
  work: 1604 resolve nodes, 5942 pivots, 80 candidates, 21293 A* expansions
  time: 2.202 s
  hash: sig2:63355df81d54e09a.ffc36700559c31e7
wrote out/T4-miller-conflicts.sp
```

It exits with 1 when an error was reported, once every file is written: an error of the netlist as a solver reads it, whose number it gives on the standard error, as `rnl check --no-generated` counts them (7.6), or an error the solver printed, a fact that contradicts itself; with 0 when there is none; and with 2 when a file cannot be read or written, having printed the files it wrote before, or when the arguments do not read. The library has the same steps for tools and tests: `solve::solve` gives the texts without writing a file, `solve::write` writes them where `solve::Target` says and gives what it wrote and copied (`solve::Writing`), `solve::lines` gives what the command prints of the levels, `solve::report` of one level solved, `solve::errors` how many errors the solver reported. The hints of the facts of priority low are in `solve::report(level, true)` when the level was solved with `SolverOptions::hints_for_low`, as `--report` solves it; without it the line of such a fact says that the solver did not seek what it conflicts with.

## Tests

The crate's integration tests run the [conformance corpus](../../tests/conformance/README.md) and the writeback round trips, check the problems given to a solver over the corpus and the [solver cases](../../tests/integration/README.md), how PRIORITY goes through nested usages and turns groups off, the records written into a subcircuit with `scope=`: the names they bind, their syntax errors and their rules, which reach the levels below it, the numbers of the objects without a name in a file of the draft 0.3 with their check, WIRE given a wire there, the hints of its conflicts, the rule package of the other dialect that its files drop the terminal names of T, O, U and Z, and the constraints that have no solution on their own in every version, write a hand-made solution back and read it again, in a file of the draft with numbers, rewritten wires and the check, and in a file of 0.2 without them, with what the writer leaves out, give the record of the solver back as hints, by name, by number and through the usages that rules make, with the solve status of each level, a text that no fact reaches on a wire or a net leaving nothing to decide, and build the Document over the disk as the desktop viewer does: by absolute paths, with the files read by `FsProvider`.

`tests/solve.rs` runs `rnl solve` through the library on the [solver cases](../../tests/integration/README.md), T1-T6 and T2b: every level solved, written back into a temporary directory and read again, with no error, nothing unplaced, every decision of the solver in the record, every constraint `ok` but those that 8.5 discarded, which are `dropped`, and, fact by fact, the facts kept holding where the solution puts the objects and those discarded not; the hash still the same; and the text byte for byte the snapshot in `tests/integration/solver/solved` (`RNL_BLESS=1 cargo test -p rnl-cli --test solve` writes them again after a change to the solver that is meant). These solves, and every other of the tests, have no time limit, so that they are the resolution of 8.5 exactly and the same on every run; the snapshots are what `rnl solve --solver-options time_limit=none` writes. A solve is the same twice and the same again on its own output; T4 discards what its reference says, each fact with its one group, with the hints of the statements on lines 37 to 39 of the file, and those of the low fact and of line 40 only with `--report`, which has the solver seek every group of that fact, the record the same, and otherwise its first group only, with a note, the report of every fact then saying that the solver did not seek them; T3 and T4 declaring the draft 0.3, with their `$TAIL_CENTER` without a name, place its middle point by its number, so that every constraint not discarded is `ok`; a file of an unsupported version and a subcircuit in an included file of an unsupported version are not solved and nothing is written into them, while a file of 0.3 is solved without any switch; the files read beside FILE are copied under DIR, one from outside it is named; a file is written back in its encoding; a write that fails says what it wrote before; one level alone, the levels of an included file, a file outside the directory of FILE, and the command's exit codes and output are checked too. Every case of the conformance corpus is solved, written and read again: it has the errors it has as written, without its records of the solver, no fact that its authors wrote is dropped that is not dropped there, every level solved is placed but for what its record cannot hold and what the solver leaves free, and has the input its hash says, and it comes out the same solved again; the labels of corpus 20, which the scene places axis by axis, read back where they were written. A file without a header gets one, with the version it was read in, 0.3 for a main file, and reads back without the hint; a record of the solver written with `scope=` is replaced by solving its subcircuit, in whatever file; a text that no fact places goes beside what it is attached to; the bus label of corpus 09, and in a small file a label on a wire and one on a net, that no fact reaches have no decision in the record, so that the small file stays 0.2, and read back are drawn above the middle of the bus or the wire and at the start of the net's wire, as unsolved, while a label of the wire whose y an ALIGN gives is written, its x in the middle of the wire; the ground without a name of the subcircuit of 0.3/09, on a net that nothing draws, has no decision either, so that its included file stays 0.2, and reads back unplaced, as unsolved; and `problem` hashes the options as `solve` writes them, the default with its time limit of 5 seconds and `time_limit=none` the snapshots'. Facts that contradict themselves only through the pins of a symbol and its orientations, which the runtime gives the solver (the drain and the gate of an NMOS aligned on y, an ORDER of the NMOS and the centre of its drain and its source, its drain and its source at least 5 apart on x, its gate and its drain exactly 1 apart on y), fail, each with its error line, in no list of the level but its own; the rest is solved as without them, the file is written and the command exits with 1; with no time at all they are undecided instead. Those that the runtime sees from what is written (an ORDER of one set written twice, one of a resistor and the set of it alone) it fails itself and leaves out of the problem: the solver fails nothing, the level is solved as without them, the file is written, and the command exits with 1 for the errors of the netlist, as it does on the corpus cases with errors, 12, 16 and 19. A header record after a subcircuit solved, in an included file, one whose first line is the subcircuit's, and in a main file solved a level at a time, moves to line 2, where a new header goes, and gets its version there; one in `//` in a main file whose first statement is not `simulator lang=spectre` moves above it in `*`, before the file level is written, whose record is in `*` too, and solves the same twice, as it does after its file level was solved alone; corpus 06 and 12, whose headers come after a subcircuit, read back with no new error and solve the same twice. No fact of T1-T6 or of the corpus fails. The time limit is tested on a clock that advances one second at each reading, so that it runs out at the same point on every run: with no time at all only the facts that every object at the origin satisfies are kept, the others discarded undecided, each with its line and its warning, every fact kept holding and the record reading back without an error; T4 with 35 readings, of which the checks of its facts on their own take about 25, keeps what 8.5 keeps before its first fact left undecided, two facts discarded by then, and after it only facts that the placement reached by then satisfies; both are the same again on a fresh clock; a `PLACE(5.5, 0)` decided before the limit cuts a later fact is written as 5.5, every fact kept holding at 1e-6 where the solution puts the objects, and read back no fact kept is dropped against the record; each warning names where its fact is written; an ALIGN of three resistors placed apart names its three minimal groups, and a limit that cuts their enumeration after every fact is decided, before the first, after it or after the second, keeps the facts of 8.5, gives the groups found and says on the line of the ALIGN, and in the head of the hints by statement, that the list is not complete. The command prints its options with `time_limit=5` by default and writes the snapshot with `time_limit=none`; every other run of the command in the tests has `time_limit=none`.

See the [roadmap](../../docs/roadmap.md) for planned work.

## Search paths

`rnl check top.sp --path ./models --path ./packages` searches both packages and bare include filenames. `--package-dir DIR` remains package-only. Both options participate in package directory order as written on the command line; only `--path` participates in include fallback. These options work for checks, rendering, views, previews and solving.

Includes are first resolved relative to the file containing the include. Only a filename without directory components falls back to `--path`, in order, with a hint naming the file used; explicit relative and absolute paths keep their previous meaning. Within each package directory, try `<name>.rnl`, then `<version>/<name>.rnl`, then scan the other `.rnl` files. Package names and versions always come from the `PACKAGE` header, and a conventional filename with a mismatching header warns. Filenames need not match package names, but a package directory, and `--path` as one, is searched for its `.rnl` files only: a package file of another suffix is found by name when `--format PATH=rnl` sets it, before the package directories, and a `.rnl` file set to another format is no package. A package file is of the format `rnl` (`rnl formats`), which a `.rnl` file has, and a file of another suffix whose first record starts a line, as no netlist's can, and is a header record with PACKAGE: `rnl check acme.pkg` checks it on its own as `rnl check acme.rnl` does, and a file set to `rnl` whose header has no PACKAGE is an error (16.2). A netlist that includes a package file does not read it: the include is an error, as a package file is not loaded with `.include` (2.7). Version selection follows 16.2 and copies of the same version use the first directory. User directories precede bundled packages, including user replacements of `rnl-std` (16.3). The include fallback and filename preference are tool behavior, not changes to the language standard.


`rnl solve FILE -o DIR --profile OUT.json` writes optional timing observations separately
from all solver records. The destination must not already exist, and its parent directory must exist. It must differ from every solved output path. The file is valid JSON
from before solving begins and is replaced atomically at stage transitions and at throttled
work checkpoints (normally at most every 0.5 seconds; a single long arithmetic operation can
delay an update). A killed run retains its last snapshot. `status`, `last_updated_seconds`,
`current_stage`, `active_span`, `counts_complete` and each span's `complete` distinguish partial
work from a finished solve. A writer error makes the command fail.

Schema version 1 has `levels[]`, each with input counts, `problem_hash`, `options_canonical`,
`total_seconds`, `stage_seconds`, `metrics`, and `spans[]`. The six top-level stage times are
mutually exclusive; nested span times are inclusive and must not be added to those totals.
Spans have an `id`/`parent`, a category (`stage`, `resolve_phase`, `lp`, `lex_layer`,
`route_net`, or `route_path`), a name, candidate ID where applicable, and counters in `metrics`.
A coordinate LP span covers one axis and its successive objectives and grid passes. LP
`variables` includes slack variables, `tableau_rows` counts defining equalities, and
`constraints` counts active lower/upper bounds (an equality is two bounds), each a high-water
mark. Rational bit maxima cover stored tableau values, bounds and coefficients at observations,
not temporary arithmetic intermediates. Missing metrics mean unobserved; peak memory is `null`
and can be collected by the process supervisor. Input host-instance counts exclude bundles.

Profiling is not a solver option and does not enter `sig2`, alter budgets, change comparisons,
or add deadline-clock reads. Disabled profiling performs no timing or number-size scans. Tests
compare complete solutions and written record bytes, including their hashes, with profiling
on/off and with deterministic clocks that stop resolution at the same check. With a real wall
clock, observation overhead can make a resolution reach its existing deadline sooner. Exact
result equality is guaranteed when neither run reaches that deadline; a reached deadline
(`deadline_reached` in level metrics) requires treating the comparison as time-dependent.
The tool never adjusts `time_limit` to conceal that boundary. Timings include observation
costs; counters and rational scans also add cost, so compare against unprofiled runs when
estimating that overhead.

The document-level `solver_total_seconds` is the sum of all level solver times.
`cli_elapsed_seconds` (also retained as `elapsed_seconds`) additionally includes interpretation,
version re-reading and record construction. Use solver time for solver-stage shares.

Router `join_retries` counts attempts for a part that already had a failed join in that net;
`failed_join_attempts` includes seed and later tree joins. These are not the number of nets left unrouted.

`disjunction_facts` counts facts with encoded unit choices; `selectors` counts those choices.
Dynamic centre/orientation alternatives are reflected in branch/node counts, not in that input count.
`deadline_reached` is explicitly zero when monitored resolution has not reached its deadline.
