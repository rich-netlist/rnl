# rnl-view

The core interface of [decision 0006](../../docs/decisions/0006-shared-core-per-platform-frontends.md): a workspace in, a Document out. The desktop viewer and `rnl view` call it natively; the web viewer will call it through WebAssembly.

```rust
pub fn build(entry: &str, files: &dyn Provider, options: &BuildOptions) -> Result<Document, String>
```

- `entry` is the path of the file to open as the frontend names it, `/`-separated (a `\` is read as `/`). It need not exist on disk: an untitled text is an entry in an in-memory provider. `build` fails when the entry cannot be read. Name the files of a workspace by rooted paths, such as `/w/top.sp`, as the desktop names them by absolute ones (see below).
- `files` is the frontend's file access: `FsProvider` from [`rnl-cli`](../rnl-cli/README.md) on the desktop and in the CLI, a `MemoryProvider` of the texts a frontend holds elsewhere. The standard packages in [`libraries/standard`](../../libraries/standard/README.md) are bundled here and layered under it, in the directory `<bundled>`.
- `BuildOptions` holds the dialect, instead of guessing it from the entry, and the runtime's options, such as leaving out the standard package at the root (16.3).

The Document holds, for each drawing level, its SVG, what on it can be clicked, its status without a solver and its subcircuit instances; for the document, the source files with the places of their records and of the statements that do not take effect (7.5), the instances and nets of every level with the places that name them, and the diagnostics. The references in the records come from the runtime (`Interpretation::refs`), so aliases, scopes and subscripts are resolved as the language says. The field names are what the viewers read; `rnl view FILE` prints the Document as JSON.

`path` is the entry as given. A file of the document is named relative to the entry's directory when it is in it or below, and otherwise as the provider names it, so the entry's directory joined with a file's path (`rnl_runtime::source::join`) is the file in the workspace. That needs a rooted or absolute entry: with a relative entry in a directory, such as `w/top.sp`, a file outside it keeps a relative name, such as `x.inc`, which can be that of a file inside it (`w/x.inc`), and the Document then holds only one of the two. The host adapters name the files this way.

`missing` lists the files the core asked for by path and did not get, such as an include or a package file named by `path=` that is not in the workspace, each once, as the provider was asked for it, in the order first asked. A frontend that can read them adds them to the workspace and builds again until nothing new is missing. A package found by name comes only from the provider's package directories (`Provider::package_files`, such as `MemoryProvider::package_dirs`), so it is never missing: a frontend gives its package directories before it builds.

The crate also has what the `rnl` command needs that reads no files: dialect detection, reading a host document, interpreting a netlist or a package file checked on its own, and the status lines of `rnl check`.

The crate does no file I/O, so it builds for `wasm32-unknown-unknown`.

See the [roadmap](../../docs/roadmap.md) for planned work.
