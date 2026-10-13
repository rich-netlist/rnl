# rnl-view

The start of the core interface of [decision 0006](../../docs/decisions/0006-shared-core-per-platform-frontends.md), a workspace in and a Document out: what the frontends share that reads no files. It detects the dialect of a netlist, reads and interprets a netlist or a package file checked on its own through the frontend's own file access, and gives the status of a drawing level that `rnl check` prints. The standard packages in [`libraries/standard`](../../libraries/standard/README.md) are bundled here and layered under the frontend's files, in the directory `<bundled>`.

Paths are `/`-separated, as the runtime reads them; a `\` is read as `/`.

The crate does no file I/O, so it builds for `wasm32-unknown-unknown`.

See the [roadmap](../../docs/roadmap.md) for planned work.
