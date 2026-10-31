# 0010. The desktop viewer solves as a preview and saves a solved copy; it never changes the files it opens

- Date: 2026-10-17
- Status: accepted

## Context

Until now RNL Viewer, the desktop viewer ([`apps/desktop`](../../apps/desktop/README.md)), only read: it neither edited nor solved, so the objects that only a solver places were not drawn. The solver ([`rnl-solver`](../../crates/rnl-solver/README.md)) ran only in `rnl solve`, which writes its records into the files. To test the solver on their own netlists, users had to go to the command line, write the files into a copy, and open that copy in the viewer.

[Decision 0008](0008-shared-core-per-platform-frontends.md) makes the solver part of the shared core, has the desktop run it natively on a background thread, and has its results written into the workspace as RNL records, like any other edit. It does not say what becomes of the files on disk.

## Decision

1. **The desktop viewer solves the netlist it shows as a preview.** Solve › Solve (F9) solves it as `rnl solve` does, with its options and its default time limit, read without its records of the solver (2.8). The solve runs on a thread of the app's, shows its progress in the status bar and can be cancelled. The texts the solve would write are held in memory over the files, and the viewer draws them. File › Reload, or Solve again, solves again; Solve › Close Preview, or opening a file, shows the file as written.
2. **It never writes into the files it opens.** A preview changes nothing on disk.
3. **A solved copy is saved where the user says.** Solve › Save Solved Copy As saves the files the solve would write:
   - one file, for a netlist that reads no other file;
   - otherwise a folder, holding the files written and the other files read in their places relative to the file solved, as `rnl solve -o DIR` writes them.
   A place that would replace any file that was read is refused. A file of 0.2 is raised to 0.3 as `rnl solve` raises it (16.1).
4. **The solve's report goes with the preview:** every fact kept, discarded, undecided or failed, the hints of its conflicts and the hints by statement, each shown at its line in the netlist when clicked, and a note that the files are unchanged.
5. **The web viewer does not solve, for now.** The Solve menu and its views are the desktop shell's alone; the shared page shows them only where the shell offers a solve.

## Consequences

- The viewer is no longer read-only, but only its memory changes: the files on disk stay as they were opened. Its README, its About box and the first lines of its code say so.
- The work is in the core, as decision 0008 says: [`rnl_cli::preview`](../../crates/rnl-cli/src/preview.rs) solves in memory, builds the Document of the texts solved and the report, and saves a copy, refusing a place over a file that was read. `rnl preview FILE` prints the same as JSON. The app only runs it on a thread and asks for the place of a copy.
- A solve cancelled stops before the next level, and the level it is in stops checking its facts, as the time limit stops them (8.5). With no time limit, it stops at the next level.
- Writing the solve into the files themselves, and editing RNL in the viewer, remain open, as in decision 0008.

## Update, 2026-10-17

Solve › Settings sets the options and the time limit that point 1 says are `rnl solve`'s: the solver's defaults until the user changes them, then the user's, kept by the page. Nothing is written into the files by it.

## Note, 2026-10-17

Point 2, that the viewer never writes into the files it opens, is changed by [decision 0011](0011-viewers-edit-the-text-and-write-only-what-the-user-saves.md): it writes a file only when the user has edited it and saves it. Solving still only previews, and a solved copy is saved as point 3 says.

## Note, 2026-10-31

Point 5 stands for the published web viewer. The web viewer served by `apps/web/serve.mjs --rnl PATH`, the development server, solves with the `rnl` at that path, run by a service of the server for the page that it served (see [apps/web](../../apps/web/README.md#solving-with-a-solver-you-build)), so that a change to the solver is seen without building the viewer. It shows the same preview and report; it saves no copy and has no settings dialog. The Solve view, on both viewers, lists the constraints of the netlist that the core gives with the Document, and after a solve what became of each; that does not need a solver.
