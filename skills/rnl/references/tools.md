# Checking and drawing RNL files

Four ways, from the one an agent can run itself to the ones a person uses. Pick the first that you can use here.

| You have | Use | It gives |
| --- | --- | --- |
| a shell on Windows x64, Linux x86_64 or macOS | the `rnl` command, downloaded from a core Release (or built from source) | diagnostics with line numbers, SVG drawings, JSON of the model; the most complete |
| Node 18 or later and a network | the npm package `@rich-netlist/runtime` | the same core as WebAssembly: a Document with diagnostics and an SVG for each level |
| a person with a browser | the web viewer <https://rich-netlist.github.io/rnl/viewer/> | the drawing, the text and the problems list, with nothing to install |
| a person on Windows or Linux | the desktop viewer | the same page in an app that reads files where they are, and edits, saves and solves as a preview |

All four are the same core, so they read a file the same way. The language is a draft that changes between versions (0.x makes no promise of compatibility), and so does the toolkit. Core releases use `core-vX.Y.Z`: the CLI and npm package share X.Y.Z, independently of Viewer and language versions. Historical npm previews used `runtime-v0.1.0-preview.N`. Until the first core version and npm dist-tag are chosen, install an exact published npm version rather than assuming `next` or `latest`.

## The `rnl` command

### Download a ready-to-run CLI

After the first core release, these fixed links follow the latest core Release. Before it,
use the source build below. The archives contain the executable, MIT, CC0 and OFL licences
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

<https://rich-netlist.github.io/rnl/viewer/> is a static page with the core as WebAssembly; nothing you open is sent anywhere. A person opens a file with File › Open, drops files or a folder on the page, pastes with File › Load from Clipboard (Ctrl+Shift+V), or gives an address: `?url=ADDRESS` in the page's address opens a netlist on a web server, with the files it includes fetched relative to it, when the server allows it (CORS). It draws each level, lists the problems under the text (click one to see its line), and edits the text: Edit › Edit Text, then Apply Changes; Save downloads the file shown. Its samples are the conformance corpus, such as <https://rich-netlist.github.io/rnl/viewer/?url=samples/cases/03-five-transistor-ota.sp>.

An agent that cannot drive a browser should ask the person to open the file there when a picture matters, and check the text with one of the tools above.

## The desktop viewer

For Windows 10 and 11 (x64) and Linux (x86_64, from 0.1.2): the same page as the web viewer in an app that reads the files on disk by their paths, so the files a netlist includes are found wherever they are. It draws, lists the problems and shows the text; it edits the text, applies the changes and saves only the files you changed (the web viewer downloads the file instead); and its Solve menu shows what a solver would place as a preview that writes nothing into the files, unless you save a solved copy (release 0.1.0 only reads). Download the newest release, `viewer-vX.Y.Z`, from <https://github.com/rich-netlist/rnl/releases> and check the file against `SHA256SUMS.txt`: it has `RNL-Viewer-X.Y.Z-windows-x64-setup.exe` (installer for the current user), `RNL-Viewer-X.Y.Z-windows-x64.msi` (installer for the machine) and `RNL-Viewer-X.Y.Z-windows-x64.exe` (the app alone), and for Linux `RNL-Viewer-X.Y.Z-linux-x86_64.AppImage` (runs without installing) and `RNL-Viewer-X.Y.Z-linux-x86_64.deb` (Debian and Ubuntu). The files are not signed, so Windows SmartScreen may warn before they run. Open a netlist with File › Open, or start the app with the file as its argument.

## Licences

The `rnl` command, the npm package's JavaScript and the viewers are under the MIT licence. The standard packages and the examples are under CC0-1.0, the text of the standard and of the guide under CC-BY-4.0. The npm package holds the metrics of the font Noto Sans SC, copyright 2014-2021 Adobe, and the viewers embed the fonts they draw texts in, Noto Sans SC Regular and Bold: both under the SIL Open Font License 1.1. This package's own terms are in `LICENSE.md`.
