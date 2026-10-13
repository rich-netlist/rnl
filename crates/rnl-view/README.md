# rnl-view

The core interface of [decision 0006](../../docs/decisions/0006-shared-core-per-platform-frontends.md): a workspace in, a Document out. The desktop viewer and `rnl view` call it natively; the web viewer will call it through WebAssembly.

```rust
pub fn build(entry: &str, files: &dyn Provider, options: &BuildOptions) -> Result<Document, String>
```

- `entry` is the path of the file to open as the frontend names it, `/`-separated (a `\` is read as `/`). It need not exist on disk: an untitled text is an entry in an in-memory provider. `build` fails when the entry cannot be read.
- `files` is the frontend's file access: `FsProvider` from [`rnl-cli`](../rnl-cli/README.md) on the desktop and in the CLI, a `MemoryProvider` of the texts a frontend holds elsewhere. The standard packages in [`libraries/standard`](../../libraries/standard/README.md) are bundled here and layered under it, in the directory `<bundled>`.
- `BuildOptions` holds the dialect, instead of guessing it from the entry, and the runtime's options, such as leaving out the standard package at the root (16.3).

The Document holds, for each drawing level, its SVG, what on it can be clicked, its status without a solver and its subcircuit instances; for the document, the source files with the places of their records and of the statements that do not take effect (7.5), the instances and nets of every level with the places that name them, and the diagnostics. `missing` lists the files the core asked for and did not get, such as an include or a package file that is not in the workspace, each once, in the order first asked; a frontend that can read them adds them to the workspace and builds again until nothing new is missing. `path` is the entry as given; the other paths are as the runtime and the provider name them. The references in the records come from the runtime (`Interpretation::refs`), so aliases, scopes and subscripts are resolved as the language says. The field names are what the viewers read.

The crate also has what the `rnl` command needs that reads no files: dialect detection, reading a host document, interpreting a netlist or a package file checked on its own, and the status lines of `rnl check`.

The crate does no file I/O, so it builds for `wasm32-unknown-unknown`.

See the [roadmap](../../docs/roadmap.md) for planned work.
