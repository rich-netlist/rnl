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

**What Firefox needs of the system.** Firefox, the one of the package or any other, needs what a desktop has: GTK 3 and a few X libraries. A computer with no desktop, such as a stock minimal install of RHEL 8 or Rocky 8, may lack some, and Firefox then does not start (`XPCOMGlueLoad error for file …/libxul.so: libX11-xcb.so.1: cannot open shared object file`): the program says so, with what the browser said, when a browser ends as soon as it is started. As root, `yum install gtk3 dbus-glib libXt libXtst libX11-xcb alsa-lib libXcomposite libXdamage libXrandr libXi libXcursor` (`dnf` on RHEL 8 and 9) adds them; without root, use `--no-browser` and `ssh -L`, the browser being on another computer. Firefox runs without a display, for a test, with `MOZ_HEADLESS=1`. The message `Sandbox: CanCreateUserNamespace() clone() failure: EPERM` that Firefox may print is its content-process sandbox, which needs unprivileged user namespaces: a container's default seccomp profile and RHEL 7's kernel (`user.max_user_namespaces` is 0 there) do not give them, and Firefox then runs its content processes without the sandbox (the CI containers set `MOZ_DISABLE_CONTENT_SANDBOX=1`). It matters little here: the window shows only this program's page, in a profile of its own; but do not browse other sites in it.

A Chromium-family browser is opened as an application window (`--app`); Firefox with `--no-remote`. Each run has **a browser profile of its own** (a new folder of mode 0700 in `~/.cache/rnl-viewer-local/profiles`, removed when the program ends), so a run never meets the lock of an earlier one (Firefox would then ask, in a window, whether to close it, and the page would never be shown), and the user's own browser profile is never touched. The profiles of runs that ended without removing theirs are removed by the next run **on the same computer** (the folder's name has the computer's and the process's), never those of another computer that shares the home folder. A browser that was started and ended at once, with no page asking for anything, stops the program after three minutes; a page that is silent for three minutes does not, while it had changes not saved (the page says so each time it asks what changed): the program says so once and keeps running.

## What it keeps of the desktop viewer, and what it does not

| | |
| --- | --- |
| Kept | Open by the path on the command line, and from a chooser that lists the machine's files (File › Open, Include File, Include Folder); **Save** and Save As write the file where it was read from, in the encoding it had, and ask first about a file that changed on disk (decision 0011); **Solve** with the core's own solver, in the program, on the machine's cores, one circuit (`level`) or all of them, telling the page the events of the solve as they happen (`stream`, as lines of JSON: the page's progress panel); a note when a file the page has read changes on disk (F5 reloads it); the settings in a file of the user's own. |
| Different | The core runs in the browser, as WebAssembly, as on the web: a big design is slower than in the desktop viewer, and bounded by the browser's memory. The menu bar is the page's own. |
| Not there | Native file dialogs and menus (the page has its own chooser, whose last entry is the browser's), file associations, several windows of one program, the PATH folders of the Library (the web viewer's Include Folder takes their place) and the preloading of Library files. |

## Security

The program listens on `127.0.0.1` only, but that address is shared by every user of the machine and by every page the user's browser has open, so:

- A request is served only if its `Host` is `127.0.0.1`, `localhost` or `[::1]` (with any port: `ssh -L` changes it), which keeps a page of another site, reaching the program by a DNS name it controls, out.
- The first address is `http://127.0.0.1:PORT/__start?t=TOKEN`, whose 256-bit token **works once** (and is spent by the right token only: a wrong one, which any page can send, does not use up the user's address). The browser is told to open a file of the user's own that goes on to that address, so the token is **never on a command line**, where `ps` shows it to the other users of the machine. The file is in a **new folder of mode 0700**, with a name nobody could have made first (made by the call that fails if it exists, in `$XDG_RUNTIME_DIR` or the cache folder), and is made by `create_new`, which does not follow a link; the folder above it is not changed. Both are removed once the address has been used.
- The address gives the page a **key of the session**, which the page sends in the header `X-RNL-Key` with **every** request that reads or writes the user's files, or solves (the files are fetched with it too). It is **not a cookie**: a browser sends a cookie to every port of the same address, so any page the browser has open from another local port (another user's server, another `ssh -L` tunnel) would have been handed it. The key is in the fragment of the address the program goes on to, which no server is sent, and the page keeps it in its own `sessionStorage`; a tab that is closed has lost it, and the program is started again.
- A request that changes something also needs the header `X-RNL: 1`, comes from the program's own origin and is not cross-site.
- A request is judged from **its head**: the `Host`, the key and the route are checked before any body is read. A body is read only for a request that is let in: 1 MiB for most, 512 MiB for a save or a solve, and the bodies being read at one moment may not add up to more than 1 GiB. The head must come in 10 seconds *in all*, not per read; the connections waiting for a head are limited and the one that has waited longest is closed to make room, and places are kept for the page that holds the key, so a client that sends a byte at a time cannot stop the page from asking, and with it the program. A body is judged by its **pace**, not by a time for all of it: it is given up when nothing comes for 30 seconds, or when less than 64 KiB a second comes on average over 30 seconds, so a large save or solve over a slow `ssh -L` is taken. What is not of the session is given a time for its whole answer (30 seconds, and 64 KiB each second), so that a client that does not read cannot hold a place.
- The page may read and write only below the **folders allowed**: the folder of each file opened and the one above it (not above the user's home folder, nor the root of the file system), the folders chosen to include, and those named with `--allow`. A file or folder the *page* chooses (not one named on the command line or with `--allow`, which are the user's own words) must have no hidden part in its path, and a folder that is the root, the home folder or one that holds it is not given (a file in one is given alone). Nothing hidden (a name that starts with a dot) below an allowed folder is served, and a path is checked as the file system resolves it, so a link that leaves an allowed folder does not count. The chooser lists any folder the user can read, to find the file to open: **the page can see the names, sizes and times of the files of any readable folder** (never hidden names; at most 10 000 entries of a folder, which the listing says), and it is served nothing that it lists unless it is allowed. Only regular files are read or written (never a device), and a file read is at most 256 MiB.
- **Saving** writes a file whole or not at all where it can do that without changing anything else of the file: to a new file in its folder with the **owner, group and mode** of the file, flushed to the disk (and the folder), and renamed over it. A file with other names (hard links), another user's or another group's file the user cannot give to a new one (a project folder shared by a group), or one in a folder the user cannot write in, is written **in place** instead, **without emptying it first**: the new bytes go over the old from the start, the file is cut to length, and read back; on any failure (a full disk, a quota) the old bytes, kept in memory, are put back, and the error is given. ACLs and extended attributes are not kept by a rename (the new file has the folder's defaults). **Save As** does not replace a file that is there unless the user says so (the page asks), and **never writes through a link**: a link at the name is refused, whatever it points to.
- The page is not to be framed (`frame-ancestors 'none'`), and its address takes nothing but `shell=local` (and the token of New Window's handoff): `?url=`, `?manifest=`, `?embed=` and `?api=` of the web viewer, with which another site could make the page open what it chose, are taken away by a redirect.
- The program reads and writes nothing else of the user's: its settings, the browser profiles and the file with the address are in `~/.config/rnl-viewer-local` and `~/.cache/rnl-viewer-local` (and `$XDG_RUNTIME_DIR`).

## Building and testing

The web viewer is embedded when the program is built: `node apps/web/build.mjs` writes `apps/web/dist`, then

```
cd apps/local
RNL_SITE_DIR=../web/dist cargo build --release                       # or --target x86_64-unknown-linux-musl, static
cargo test                                                           # the unit tests: HTTP, security, files, saving, the solver
npx --prefix ../../tests/viewer playwright test --config playwright.config.mjs   # the page against the program (RNL_LOCAL_BIN names it)
```

The program has no dependency but the core's own crates and `serde`: no HTTP crate. A **static musl** build also has `mimalloc` as its allocator (a dependency of musl builds only): musl's own allocator made the solver, which allocates the most of the core, about a quarter slower than glibc's did.

```
rustup target add x86_64-unknown-linux-musl     # and musl-tools (musl-gcc) for mimalloc's C
RNL_SITE_DIR=../web/dist CC_x86_64_unknown_linux_musl=musl-gcc cargo build --release --target x86_64-unknown-linux-musl
```

`local-linux.yml` checks in CI that the musl program is static, **times a solve with it against the glibc build** of the same program (the job fails if it is more than 1.3 times as slow; then the program is to be built for glibc 2.17 instead, so that it needs no more of a system than CentOS 7's), makes the packages, and tests the one with Firefox in containers of `centos:7`, `rockylinux:8` and `rockylinux:9`: the program starts the Firefox beside it itself, as a user would, and the page opens the netlist named on the command line and solves it (`ci/`: `container.sh`, `drive.mjs`, which looks at the page through Firefox's Marionette protocol, and `bench.mjs`). A container shares the kernel of the host (a recent one on GitHub's runners), so CentOS 7's own 3.10 kernel is not what is tried; what is tried is its C library (2.17) and its userland, which a static program uses nothing of but the system calls.

## Packages

The release has two files for Linux x86_64 (`local-release.yml`, from a tag `viewer-local-v<version>`, never marked as the repository's latest release, which the core's fixed download links use; the desktop viewer's release, `viewer-v*`, and what looks for the newest of it are not touched):

| File | Holds |
| --- | --- |
| `rnl-viewer-local-VERSION-linux-x86_64.tar.gz` | the program, this README, the licenses, and `get-firefox.sh` with `firefox.lock`, which downloads Mozilla's official Firefox ESR, checks it against Mozilla's own checksum and unpacks it beside the program, where the program looks first (`--from FILE` for a computer with no network, which has the file copied from another) |
| `rnl-viewer-local-VERSION-linux-x86_64-with-firefox.tar.gz` | the same with that Firefox already in the folder `firefox`, and `FIREFOX-NOTICE.txt` |

The Firefox is **Mozilla's official ESR, unmodified**: it is unpacked as it is, nothing is added to it or taken from it, and the program starts it in a profile of its own, so that it is not changed either by use. Mozilla's [policy for the distribution of unaltered software](https://www.mozilla.org/en-US/foundation/trademarks/distribution-policy/) allows that if it is not changed, not charged for, no personal data is collected by the distribution, and **the most recent version** is distributed: so a release is made with the newest ESR that Mozilla publishes (`ci/firefox.lock`, which `local-release.yml` checks against Mozilla's own list of versions), and a package with Firefox is not to be kept for long; a computer whose own Firefox is 115 ESR or newer needs neither. Firefox is under its own licenses (the Mozilla Public License 2.0 and others: `about:license`), and its source is at <https://archive.mozilla.org/pub/firefox/releases/>.
