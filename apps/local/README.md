# RNL Viewer, local

The web viewer served on the loopback address by one program, and drawn by a browser. It is for older enterprise Linux (CentOS 7, RHEL 8, RHEL 9), where the desktop viewer's libraries (WebKitGTK 4.1) are not to be had: the program is built **static** (musl), so it has no library to find on the machine, needs no root and no network, and runs on any x86_64 kernel from 3.2. Over `ssh -L` the browser may be on another computer, so a server with no desktop at all can be used.

```
rnl-viewer-local [OPTIONS] [FILE]...
```

| Option | |
| --- | --- |
| `--port N` | The port to listen on (the system gives one by default). Use the same port on both sides of `ssh -L`. |
| `--no-browser` | Show no browser: print the address, which works once, to open in one. |
| `--browser PATH` | The browser to use: a name on the PATH, or a path (`$RNL_BROWSER` does too). |
| `--allow FOLDER` | Let the page read and write below `FOLDER` too; may be repeated. |
| `--site DIR` | Serve the web viewer from `DIR` instead of the copy in the program. |
| `--settings FILE` | Keep the settings in `FILE` (default `~/.config/rnl-viewer-local/settings.json`). |

The files named on the command line are opened; the first is shown. Close the browser window, or press Ctrl-C, to stop the program; it also stops when the page has said nothing for three minutes.

## The browser

The page needs a browser of 2021 or later: **Firefox 115 ESR or newer, Chrome, Chromium or Edge 89 or newer, Safari 15 or newer** (the core runs in a module Worker as WebAssembly, and the page uses top-level `await`). The program looks, in this order, for the browser named, the Firefox that comes in a `firefox/` folder beside it (the package that bundles one), then Chrome, Chromium, Edge and Firefox on the PATH. A browser below the minimum is **never opened onto a page that would stay blank**: the program says it is too old and gives the two ways out, the package with its own Firefox and `--no-browser` with `ssh -L`. A browser that is too old that is opened by hand gets a notice on the page itself (an ES5 script of `index.html` checks the features the page uses).

A Chromium-family browser is opened as an application window (`--app`) with a profile of its own; Firefox in a profile of its own, `--no-remote`. The user's own browser profile is never touched.

## What it keeps of the desktop viewer, and what it does not

| | |
| --- | --- |
| Kept | Open by the path on the command line, and from a chooser that lists the machine's files (File › Open, Include File, Include Folder); **Save** and Save As write the file where it was read from, in the encoding it had, and ask first about a file that changed on disk (decision 0011); **Solve** with the core's own solver, in the program, on the machine's cores; a note when a file the page has read changes on disk (F5 reloads it); the settings in a file of the user's own. |
| Different | The core runs in the browser, as WebAssembly, as on the web: a big design is slower than in the desktop viewer, and bounded by the browser's memory. The menu bar is the page's own. |
| Not there | Native file dialogs and menus (the page has its own chooser, whose last entry is the browser's), file associations, several windows of one program, the PATH folders of the Library (the web viewer's Include Folder takes their place) and the preloading of Library files. |

## Security

The program listens on `127.0.0.1` only, but that address is shared by every user of the machine and by every page the user's browser has open, so:

- A request is served only if its `Host` is `127.0.0.1`, `localhost` or `[::1]` (with any port: `ssh -L` changes it), which keeps a page of another site, reaching the program by a DNS name it controls, out.
- The first address is `http://127.0.0.1:PORT/__start?t=TOKEN`, whose 256-bit token **works once**; it gives the page a session cookie (`HttpOnly`, `SameSite=Strict`). The browser is told to open a file of the user's own (mode 0600, in `$XDG_RUNTIME_DIR`) that goes on to that address, so the token is **never on a command line**, where `ps` shows it to the other users of the machine. The file is removed once the address has been used.
- Everything that reads or writes the user's files, or solves, needs that cookie; and a request that changes something needs the header `X-RNL: 1` as well, comes from the program's own origin and is not cross-site. Another local port is the same *site*, so the cookie alone would not be enough.
- The page may read and write only below the **folders allowed**: the folder of each file opened and the one above it (not above the user's home folder, nor the root of the file system), the folders chosen to include, and those named with `--allow`. Nothing hidden (a name that starts with a dot) below one is served, and a path is checked as the file system resolves it, so a link that leaves an allowed folder does not count.
- The program reads and writes nothing else of the user's: its settings, the browser profiles and the file with the address are in `~/.config/rnl-viewer-local` and `~/.cache/rnl-viewer-local`.

## Building and testing

The web viewer is embedded when the program is built: `node apps/web/build.mjs` writes `apps/web/dist`, then

```
cd apps/local
RNL_SITE_DIR=../web/dist cargo build --release                       # or --target x86_64-unknown-linux-musl, static
cargo test                                                           # the unit tests: HTTP, security, files, saving, the solver
npx --prefix ../../tests/viewer playwright test --config playwright.config.mjs   # the page against the program (RNL_LOCAL_BIN names it)
```

The program has no dependency but the core's own crates and `serde`: no HTTP crate, no allocator crate. A static musl build is checked in CI to be static and to run in `centos:7`, `rockylinux:8` and `rockylinux:9` containers.
