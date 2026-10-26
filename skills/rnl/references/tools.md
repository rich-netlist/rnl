# Checking and drawing RNL files

Choose a checker or renderer for validation, and a viewer when the user wants to explore the circuit interactively. An agent can run the checker, construct a web link, or install and launch the desktop viewer when requested.

| You have | Use | It gives |
| --- | --- | --- |
| a shell on Windows x64, Linux x86_64 or macOS | the `rnl` command, downloaded from a core Release (or built from source) | diagnostics with line numbers, SVG drawings, JSON of the model; the most complete |
| Node 18 or later and a network | the npm package `@rich-netlist/runtime` | the same core as WebAssembly: a Document with diagnostics and an SVG for each level |
| a person with a browser | the web viewer <https://rich-netlist.github.io/rnl/viewer/> | the drawing, the text and the problems list, with nothing to install |
| Windows, Linux or macOS with a desktop session | the [desktop viewer downloads](https://rich-netlist.github.io/rnl/en/tools.html#desktop) | the same page in an app that reads local files, edits/saves text and solves as a preview; an agent can launch it with the netlist path |

All four are the same core, so they read a file the same way. The language is a draft that changes between versions (0.x makes no promise of compatibility), and so does the toolkit. Core releases use `core-vX.Y.Z`: the CLI and npm package share X.Y.Z, independently of Viewer and language versions. Historical npm previews used `runtime-v0.1.0-preview.N`. The npm dist-tag `latest` follows the latest core release; install an exact version to pin one.

## Optional inspection overlays

In a core build containing this feature, request shared usage frames and terminal currents:

```sh
rnl render circuit.sp -o drawings --usages semantic --currents
rnl render circuit.sp -o drawings --usages all --usage-roles --currents --overlay-style overlays.json
```

`semantic` shows only usages with no drawing of their own or in descendants; `all`
includes virtual symbols and drawing wrappers. All export overlays default off, preserving
the original SVG bytes. These are inspection aids, independent of language styles and solver
constraints. An optional JSON style file accepts `palette` (nonempty hex colors), `width`
(positive SVG pixels), `dashes` (positive dash/gap lengths; `[]` for solid) and
`current_color` (hex); omitted fields keep fixed renderer defaults. For example:

```json
{"palette":["#4263eb","#2f9e44"],"width":2,"dashes":[1,5],"current_color":"#d9480f"}
```

The npm request accepts `overlays: {usages: "all", currents: true, roles: true, style: {...}}`.
The Document always supplies style-free `levels[].usages[].frame` and `levels[].currents`
geometry in SVG pixels (y down), with identical native/WASM results. In/out arrows follow
oriented terminal exits; unspecified or unplaced/directionless pins have none. Declaration
ports use explicit `$IO_PORT` symbols on the port net: `in` points into the circuit from the tag.
Viewer keeps its own usage colors, visibility and selection preferences. View > Show Current
Directions starts off and remembers its state. These changes arrive in the next separately
approved core/Viewer releases; no release is implied by the source changes.

## Reading the standard source

The full normative Markdown is `spec/rnl-0.3.md` in the RNL repository. Read that
file directly when a checkout is available. For a standalone skill installation,
fetch [the raw English text](https://raw.githubusercontent.com/rich-netlist/rnl/main/spec/rnl-0.3.md)
or [its Chinese translation](https://raw.githubusercontent.com/rich-netlist/rnl/main/spec/rnl-0.3.zh.md).
English prevails. These are the complete texts assembled from `spec/0.3/en/` and
`spec/0.3/zh/`; the bundled `language.md` is only a short guide. Search the full
text for the relevant section or keyword, then read it in context.

If raw.githubusercontent.com is unavailable, GitHub's contents API can return
the Markdown directly, without reading an HTML page:

```sh
gh api 'repos/rich-netlist/rnl/contents/spec/rnl-0.3.md?ref=main' -H 'Accept: application/vnd.github.raw+json'
```

For a specified revision, replace `main` with that tag/commit in the raw URL or
API request, and use matching word-list/package sources. Do not silently switch
a task pinned to an older revision to the latest standard. Do not keep another
manually maintained full-standard copy in this skill.

## The `rnl` command

### Download a ready-to-run CLI

These fixed links follow the latest core release; the source build below gives a
specific revision. The archives contain the executable, MIT, CC0 and OFL licences
and a font-metrics copyright notice. They need no Rust installation.

| Platform | Archive |
| --- | --- |
| Windows x64 | [rnl-x86_64-pc-windows-msvc.zip](https://github.com/rich-netlist/rnl/releases/latest/download/rnl-x86_64-pc-windows-msvc.zip) |
| Linux x86_64, static musl | [rnl-x86_64-unknown-linux-musl.tar.gz](https://github.com/rich-netlist/rnl/releases/latest/download/rnl-x86_64-unknown-linux-musl.tar.gz) |
| macOS Apple Silicon | [rnl-aarch64-apple-darwin.tar.gz](https://github.com/rich-netlist/rnl/releases/latest/download/rnl-aarch64-apple-darwin.tar.gz) |
| macOS Intel | [rnl-x86_64-apple-darwin.tar.gz](https://github.com/rich-netlist/rnl/releases/latest/download/rnl-x86_64-apple-darwin.tar.gz) |

Windows PowerShell, in an empty download directory (the C runtime is statically linked):

```powershell
$base = 'https://github.com/rich-netlist/rnl/releases/latest/download'
$archive = 'rnl-x86_64-pc-windows-msvc.zip'
Invoke-WebRequest "$base/$archive" -OutFile $archive
Invoke-WebRequest "$base/SHA256SUMS.txt" -OutFile SHA256SUMS.txt
$line = @(Get-Content SHA256SUMS.txt | Where-Object { $_.EndsWith("  $archive") })
if ($line.Count -ne 1) { throw 'Archive checksum is missing or ambiguous' }
$expected = ($line[0] -split '  ')[0]
if ((Get-FileHash $archive -Algorithm SHA256).Hash.ToLower() -ne $expected) { throw 'SHA256 mismatch' }
Expand-Archive $archive -DestinationPath rnl-cli
.\rnl-cli\rnl.exe --version
.\rnl-cli\rnl.exe check my.sp
```

Linux or macOS, in an empty download directory:

```bash
set -eu
case "$(uname -s):$(uname -m)" in
  Linux:x86_64) target=x86_64-unknown-linux-musl ;;
  Darwin:arm64) target=aarch64-apple-darwin ;;
  Darwin:x86_64) target=x86_64-apple-darwin ;;
  *) echo 'No prebuilt CLI for this platform; build from source.' >&2; exit 1 ;;
esac
base=https://github.com/rich-netlist/rnl/releases/latest/download
archive=rnl-$target.tar.gz
curl -fL "$base/$archive" -o "$archive"
curl -fL "$base/SHA256SUMS.txt" -o SHA256SUMS.txt
if [ "$(uname -s)" = Darwin ]; then
  grep "  $archive$" SHA256SUMS.txt | shasum -a 256 -c -
else
  grep "  $archive$" SHA256SUMS.txt | sha256sum --check -
fi
tar -xzf "$archive"
if [ "$(uname -s)" = Darwin ] && xattr -p com.apple.quarantine rnl >/dev/null 2>&1; then
  xattr -d com.apple.quarantine rnl
fi
./rnl --version
./rnl check my.sp
```

These binaries have no code signature: Windows may show SmartScreen; macOS browser
downloads may require the quarantine removal above. Verify SHA256 before running. For a
reproducible installation, use `releases/download/core-vX.Y.Z/` for both files instead of
`releases/latest/download/`. Linux arm64 currently needs a source build.

### Build from source

With Rust 1.85 or later:

```bash
git clone https://github.com/rich-netlist/rnl
cd rnl
cargo build --release -p rnl-cli       # writes target/release/rnl
target/release/rnl check my.sp
```

The commands (`rnl --help` has the full text):

```text
rnl check FILE [options]          print the diagnostics; exit 1 on errors
rnl render FILE -o DIR [options]  write one SVG per drawing level
rnl view FILE [options]           print the Document a viewer shows, as JSON
rnl dump FILE [options]           print the effective model and diagnostics as JSON
rnl catalog [options]             print the symbols, definitions and sets of the packages, as JSON
rnl strip-generated FILE -o OUT   write FILE without its type="generated" records
rnl problem | solve | snapshot | write | ...   what a solver reads, solving, the editor interface
```

FILE is a SPICE netlist, a Spectre netlist (`.scs`) or a package file (`.rnl`). Options worth knowing: `--package-dir DIR` looks for packages in DIR as well (repeatable), `--no-std` does not use the standard package, `--std-version V` reads a file with the series V (`0.2` to read a file of 0.3 with the series 0.2), `--dialect spice|spectre` instead of guessing from the file.

`rnl check` prints one line per diagnostic, `error`, `warning` or `hint`, with the file and line, and ends with a count. It exits with 1 only when there are errors; read the warnings as well. For the example in `SKILL.md`:

```text
hint ota5t.sp:3: ota5t: 9 objects have no position and are left to a solver; they are not drawn: #M1, #M2, %g0, a usage of $IO_PORT, ...
status ota5t: 9 without a position; constraints: 2 unsolved
0 errors, 0 warnings, 1 hints
```

The hint is expected for a file without positions. Without a solver, `render` draws only what has coordinates; `examples/ota5t-placed.sp` gives every object a position and is drawn completely. `rnl catalog` prints what [packages.md](packages.md) lists, from the packages the command bundles.

`rnl solve FILE -o DIR` places what has no position and writes the answer back as `type="generated"` records, in files under DIR. It is a first solver: its result can differ from one run to the next when it hits its time limit, and a person should look at it before keeping it.

`rnl solve` and `rnl strip-generated` write each file in the encoding it was read in: UTF-8, or Latin-1 for a file that is not valid UTF-8. If the written text needs a character Latin-1 cannot hold, they write nothing and name the character; add `--encoding utf8` to write UTF-8 instead.

## The npm package `@rich-netlist/runtime`

The core as WebAssembly, for Node and browsers. It reads the same files, in memory.

```bash
npm i @rich-netlist/runtime@X.Y.Z  # replace X.Y.Z with the chosen published core version
```

A complete checker, `check.mjs`; run it with `node check.mjs top.sp [outdir]`:

```js
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { basename } from "node:path";
import { startCore } from "@rich-netlist/runtime";

const [file, outdir] = process.argv.slice(2);
const entry = "/" + basename(file);
const core = startCore();
const document = await core.build({ entry, files: { [entry]: readFileSync(file, "utf8") } });
core.terminate();

for (const d of document.diagnostics) console.log(`${d.severity} ${d.file}:${d.line}: ${d.message}`);
if (document.failed) console.log(`failed: ${document.failed}`);
if (outdir) {
  mkdirSync(outdir, { recursive: true });
  document.levels.forEach((level, i) => writeFileSync(`${outdir}/${i}.svg`, level.svg));
}
const errors = document.diagnostics.filter((d) => d.severity === "error").length;
process.exit(errors || document.failed ? 1 : 0);
```

`build` takes `files`, the texts of the workspace by path, with the entry among them: put the files a netlist includes there too, under the paths it includes them by. `core.catalog({ catalog: {} })` lists the symbols, definitions and sets of the packages; `core.snapshot(...)` gives the model by ids an editor can write back. The package bundles the standard packages, series 0.2 and 0.3, so a netlist that names them needs no package files. Its README, with the requests and the settings a Vite frontend needs: <https://github.com/rich-netlist/rnl/tree/main/packages/runtime#readme>.

## The web viewer

<https://rich-netlist.github.io/rnl/viewer/> is a static page with the core as WebAssembly; nothing you open is sent anywhere. A person opens a file with File › Open File..., drops files or a folder on the page, pastes with File › Load from Clipboard (Ctrl+Shift+V), or gives an address: `?url=ADDRESS` in the page's address opens a netlist on a web server, with the files it includes fetched relative to it, when the server allows it (CORS). It draws each level, lists the problems under the text (click one to see its line), and edits the text: Edit › Edit Text, then Apply Changes; Save downloads the file shown. Its samples are the conformance corpus, such as <https://rich-netlist.github.io/rnl/viewer/?url=samples/cases/03-five-transistor-ota.sp>.

### Links that open a specific netlist

Choose by what the agent can deliver. The viewer supports both URL fetching and
a public `postMessage` API that accepts RNL text from another page:

| Available content | Link or control to give the user | What opens |
| --- | --- | --- |
| An HTTP(S) netlist and its dependencies | A direct `viewer/?url=...` link | The complete viewer loads the file immediately |
| RNL text in an agent-created HTTP(S) page | A link to that page, containing `viewer/?embed=1` in an iframe | The page sends the text after `ready`; the circuit appears automatically in a canvas-only viewer |
| RNL text in a page, with the complete viewer wanted | An **Open in RNL Viewer** button on that page, using `viewer/?api=1&origin=...&token=...` | A new tab receives the text through its opener and shows the complete viewer |

A link in a chat can point to the first two destinations. The third needs a
click in the calling page to open the tab reliably; navigating from a chat to a
launcher page does not authorize an automatic popup there. An `api=1` address
alone contains no circuit and is not a standalone share link. Use an existing
authorized host, or serve a local page for that user's machine; a local server
must stay running. Publishing a page containing the circuit publishes its text
too, even though the transfer to the viewer uses browser messages.

#### A direct link to a hosted netlist

Give the user a clickable viewer URL whose `url` query parameter is the entry
file's HTTP(S) address, URL-encoded as one value:

```js
const entry = "https://example.org/circuits/top.scs"; // actual hosted netlist
const link = new URL("https://rich-netlist.github.io/rnl/viewer/");
link.searchParams.set("url", entry);
console.log(link.href);
```

A working public example is
[open the hierarchical sample](https://rich-netlist.github.io/rnl/viewer/?url=samples%2Fcases%2F04-hierarchy.sp).
Use a raw file URL, not a GitHub `blob` HTML page. Encode the whole address,
including any `?`, `&` or `#`, rather than concatenating it unescaped.

The entry and every required include or symbol/style file must be reachable by
the user's browser. Includes resolve relative to the entry's final URL after
redirects, so preserve the directory structure. A different origin must permit
the viewer with CORS. A successful shell download alone does not prove CORS;
when browser access is available, open the finished link and check the drawing
and Problems. An HTTPS file host avoids mixed-content restrictions from the
public HTTPS viewer. Prefer a commit-pinned URL for a reproducible example.

This is a link to hosted content, not an upload facility. Use an already hosted
file or a publication the user has authorized; do not publish a private circuit
merely to make a link. Local paths, `file:` URLs and `data:` URLs are not accepted
by this entry point. To pass text held by a page, use the public API below;
neither a text/base64 URL parameter nor the internal `viewer-session` handoff
provides a durable inline-content share link.

For files that should remain local, use the desktop viewer, or open the folder
in the web viewer. If serving a local copy of the viewer and circuit, use one
loopback origin; such a link works only on that computer while the server runs,
and is not a public share link. When only a static picture is needed, return the
SVG from `rnl render` instead.

### Opening text from another page

Read the upstream [web viewer API documentation](https://github.com/rich-netlist/rnl/blob/main/apps/web/README.md#unified-api-and-complete-viewer-tabs)
when implementing a launcher (also available as [raw Markdown](https://raw.githubusercontent.com/rich-netlist/rnl/main/apps/web/README.md)).
This public interface is separate from File > New Window's internal session
handoff. Check the deployed viewer's
[`viewer-api.json`](https://rich-netlist.github.io/rnl/viewer/viewer-api.json)
for `requestId`, `packages` and `opener`; an older complete viewer may never send
`ready`.

For a complete viewer tab:

1. Prepare the text before enabling the page's button. In its click handler,
   synchronously call `window.open` with `api=1`, the calling page's exact
   HTTP(S) `origin`, and a random 16–128-character alphanumeric/hyphen `token`
   (`crypto.randomUUID()` fits). Do not put an `await` before opening or use
   `noopener`. The calling page's COOP must retain the opener: omit it or use
   `same-origin-allow-popups`, subject to that page's isolation requirements.
2. Register the message listener before opening. Accept messages only from the
   returned window and the viewer's exact origin, with `rnl: 1` and the matching
   token. Wait for `ready` and check its features before sending `open`:

   ```js
   viewerWindow.postMessage({
     rnl: 1, type: "open", token, requestId: crypto.randomUUID(),
     name: "circuit.scs", text: completedRnlText,
     packages: [] // or exact public package { name, version } pairs
   }, "https://rich-netlist.github.io");
   ```

3. Match `loaded` or `error` to that request ID. Report blocked/closed tabs and
   timeouts. When reusing a ready window, send another `open` without navigating
   it again; this preserves the viewer's confirmation for unsaved edits.

For a page that displays the circuit immediately, use an iframe with `embed=1`
and the page's exact `origin`. After `ready`, send the same `open` payload to
`iframe.contentWindow`, without `token`; verify the iframe window, viewer origin
and `rnl: 1` on replies. This mode needs no popup and shows the interactive canvas
rather than the full editor. The upstream [embedding example](https://github.com/rich-netlist/rnl/blob/main/apps/web/README.md#embedding)
shows the complete handshake. Serve the calling page over HTTP(S), not `file:`.

The filename selects the dialect (`.scs` for Spectre). Text opens have no sibling
include files, and the API has no arbitrary workspace `files` map. Use a
self-contained export, exact public packages via `packages`, or `url` with
reachable dependencies. Public package files are resolved by name/version and
checked against SHA-256 before opening. The whole `open` message is limited to
64 MiB of UTF-8 JSON; it is not a URL-length limit. Check `loaded.errors` and the
drawing when UI access is available before saying that the circuit loaded.

## The desktop viewer

The [desktop viewer](https://github.com/rich-netlist/rnl/tree/main/apps/desktop)
reads local files by their paths, including dependencies. It draws, shows
diagnostics, edits/saves text and offers solving as a preview. Published binaries
are available for Windows x64 and Linux x86_64; use the selected release's notes
for OS/runtime requirements. Windows needs WebView2; Linux packages have a glibc
baseline and WebKitGTK requirements. macOS currently requires a source build.

### Installing and launching the desktop viewer

An agent with local shell access can do this when the user requests it. A request
to install and open the viewer authorizes that workflow; a request only to check
or generate a netlist does not require installing an application.

1. Check for an existing compatible viewer and reuse it. On Windows check the
   registered `RNL Viewer` installation (including a custom install location),
   `%LOCALAPPDATA%\RNL Viewer\rnl-viewer.exe`, and any user-specified portable
   executable. Do not upgrade it just to open a file.
2. If installation is needed and authorized, discover the latest non-draft,
   non-prerelease **`viewer-v*`** release and its exact assets from
   [GitHub Releases](https://github.com/rich-netlist/rnl/releases), using the
   [releases API](https://api.github.com/repos/rich-netlist/rnl/releases?per_page=100)
   or `gh release list`, then `gh release view TAG --json tagName,url,assets`.
   Filter the API's `tag_name` (the CLI's `tagName`) by that prefix and compare
   viewer versions, paging through results if needed. GitHub's `?q=viewer-v`
   is a release text search, not a tag-prefix filter, and can omit newer viewers.
   This repository also publishes `spec-*` releases, so an unfiltered "latest
   release" may be the standard, not the application. A version in source or a
   Git tag is not proof of published binaries: if its release is missing, check
   the `viewer-release.yml` workflow and report that status rather than inventing
   download URLs. The [tools page](https://rich-netlist.github.io/rnl/en/tools.html#desktop)
   is the user-facing download entry point.
   Download the OS/architecture-appropriate asset and `SHA256SUMS.txt`, and
   compare the SHA-256 before executing it. An unrequested machine-wide install
   is unnecessary; on Windows prefer the current-user `*-setup.exe`. The
   standalone Windows `.exe` and Linux AppImage are alternatives without an
   installer. Do not bypass an OS security prompt automatically.
3. The Windows NSIS installer supports `/S` for a silent current-user install:

   ```powershell
   # $setupPath is the downloaded, checksum-verified installer.
   $install = Start-Process -FilePath $setupPath -ArgumentList '/S' -WindowStyle Hidden -Wait -PassThru
   if ($install.ExitCode -ne 0) { throw "RNL Viewer installation failed: $($install.ExitCode)" }
   ```

   Its default directory is `%LOCALAPPDATA%\RNL Viewer`. The installer's `/D=`
   option, if needed, must be last and unquoted even when its path contains
   spaces; follow the [upstream installer instructions](https://github.com/rich-netlist/rnl/tree/main/apps/desktop#readme).
   Confirm the installed executable and registered version rather than assuming
   success from the download. The installer can also install a missing WebView2
   runtime. Linux AppImage/deb instructions and prerequisites are in the release
   notes; use an authorized target and the appropriate package for that machine.
4. When asked to show a circuit, start the viewer visibly with the absolute entry
   file as its argument, keeping its dependencies beside it:

   ```powershell
   $viewerExe = Join-Path $env:LOCALAPPDATA 'RNL Viewer\rnl-viewer.exe'
   $netlistPath = (Resolve-Path -LiteralPath 'C:\work\circuit\top.scs').Path
   Start-Process -FilePath $viewerExe -ArgumentList ('"{0}"' -f $netlistPath)
   ```

   Use the detected executable path for a custom/portable install. On Linux,
   pass the absolute netlist path to `rnl-viewer` or the selected AppImage in a
   graphical session. Launch on the user's viewing machine, not a remote
   headless EDA server. Fetch an entire remote export directory first if needed.
5. Verify the process/window started and, when UI inspection is available, that
   it loaded the requested file and shows any diagnostics. Distinguish "launched"
   from "drawing verified" when only process checks are possible. Do not invoke
   the repository's installer smoke tests for a normal installation: those tests
   uninstall the application afterward.

## Licences

The `rnl` command, the npm package's JavaScript and the viewers are under the MIT licence. The standard packages and the examples are under CC0-1.0, the text of the standard and of the guide under CC-BY-4.0. The npm package holds the metrics of the font Noto Sans SC, copyright 2014-2021 Adobe, and the viewers embed the fonts they draw texts in, Noto Sans SC Regular and Bold: both under the SIL Open Font License 1.1. This package's own terms are in `LICENSE.md`.
