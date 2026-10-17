# 0011. The viewers edit the text, and write a file only when the user has edited it and saves it

- Date: 2026-10-17
- Status: accepted

## Context

[Decision 0010](0010-desktop-viewer-solves-as-a-preview.md) made the desktop viewer solve as a preview and promised that it never writes into the files it opens. Users who tried it asked to change the text in the viewer, to see the change drawn and checked, and to save it. [Decision 0008](0008-shared-core-per-platform-frontends.md) already has every frontend build a Document from a workspace of texts held in memory over the frontend's own file access, so that opening, reloading, pasting and editing all change the workspace and build again, and leaves it to the frontend how editing is presented.

The desktop viewer reads files and can write them; the web viewer, a page, cannot write a file where it read it from.

## Decision

1. **The netlist view can be switched to edit** (Edit › Edit Text, Ctrl+E): a text box for each file of the document, with the file tabs as they are, so that an included file is edited like the netlist. The first version shows the text plain, with no highlighting while it is edited.
2. **A change is not applied by itself.** The view says *Changes not applied yet*. Edit › Apply Changes (Ctrl+Enter) builds the Document again from the texts held, over the files as they are, with the shared core and as opening the files would: the drawing, the problems and the hierarchy are made again, and the level, the view and the selection are kept. Nothing applies, solves or writes by itself.
3. **A solve is a separate step, of the text applied.** Solve › Solve solves the Document shown, with the changes applied to it and not those still waiting, and draws a preview as decision 0010 says. A preview is not edited; closing it shows the text applied again.
4. **A file is written only when the user has edited it and says to save it.**
   - On the desktop, File › Save writes the files that have changes not saved, where they were read from, each in the encoding and with the line ends it had. File › Save As writes the netlist that was opened under a name the user gives, and opens that file; the files it includes are saved where they are, with Save. A text that has no file, such as the clipboard's, is saved by asking where.
   - On the web, which cannot write a file, Save downloads the file shown.
   - A file that was not edited is never written, and neither is a solve: Solve › Save Solved Copy As is as decision 0010 has it.
5. **Changes not saved are not lost by accident.** Opening another file, reading the file again and closing the window ask first: save them, leave them unsaved, or do not go on. On the web, where saving is a download, the question is to leave them or not; the browser asks again when the page is closed.

## Consequences

- Decision 0010's "it never writes into the files it opens" becomes: it writes a file only when the user has edited it and saves it. Solving still only previews, and a solved copy is saved as decision 0010 says. The desktop README and the About boxes say so.
- The shells give the page more: building and solving over texts held (`build` and `solve` take `edits`), `save` and `saveAs`, whether changes are not saved (`setDirty`, for the window's title and the browser's question), and the window's request to close. The core is not changed: the desktop builds with its own files in memory over the disk, as decision 0008 has every frontend do.
- Saving overwrites what is on disk; it does not look for a change made by another program since the file was read.
- Left for later: highlighting the text while it is edited, editing a preview, writing a solve into the files themselves (decision 0010), and watching the files for changes made elsewhere.

## Note, 2026-10-17

Saving no longer overwrites without looking: the desktop compares each file on disk with the text the page read from it, as the core reads it (UTF-8 without its byte order mark, else Latin-1, line ends apart), before it writes. A file that differs, or is gone, is not written, nor are the others given with it, and the user chooses Overwrite, Reload (read the files again, dropping the changes made in the viewer) or Cancel. The web's Save, a download, has nothing to compare. Watching the files for changes while the viewer is open is still left for later.
