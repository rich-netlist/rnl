# 0006. One shared Rust core decides the drawing; frontends are per platform

- Date: 2026-10-13
- Status: accepted

## Context

The first desktop viewer ([`apps/desktop`](../../apps/desktop/README.md)) is a Tauri application. Its page shows the drawing, the netlist text, the diagnostics and the subcircuit hierarchy, and it asks the Rust core, called natively, for everything it shows. A web viewer is wanted next. Later the viewers may edit RNL text and redraw it, including text pasted without a file.

The [architecture proposal](../rnl-toolchain-architecture.md) gives the desktop and the web one front-end path, with the runtime and the renderer compiled to WebAssembly in both (sections 6.2 and 6.3). Discussing the web viewer showed that the priority is the drawing, not the user interface: a schematic must look the same on every platform, while the interface around it may differ from platform to platform.

Measurements and facts that bear on the choice:

- The desktop viewer's release build on Windows 11 waits 0.33 to 0.5 s for the WebView engine before its page loads, and shows the first drawing after 0.6 to 1.0 s.
- WebView2 no longer supports Windows 7 and 8.1; its last version there is 109. Since Rust 1.78, binaries built for the standard Windows targets require Windows 10.
- Computing a document's drawing takes milliseconds for the test corpus, so running the core natively or as WebAssembly makes no visible difference for the viewer. The future solver is where speed matters.

## Decision

1. **The shared core is a set of Rust crates with no file input or output and no user-interface dependency:** the runtime, the host adapters, the renderer, a new view-model crate (`rnl-view`), and later the solver. Each builds natively and for `wasm32`. Frontends depend on the core, never the other way round.
2. **The core decides the drawing.** The renderer turns each level into positioned primitives: paths, arcs, shapes, and text runs with their final positions and sizes. It lays out text with the metrics of fonts shipped with the core. A frontend only rasterizes the primitives with those fonts and relies on no layout feature of its drawing engine. SVG is one output written from the primitives, not the contract.
3. **Every frontend uses one core interface: a workspace in, a Document out.** The workspace is the file texts a document needs, held in memory, untitled texts included, over the frontend's own file access. The Document holds, for each level, the primitives and what on them can be clicked, and for the document the instances and nets, their places in the text, the diagnostics, the statements that do not take effect, and the hierarchy. Opening, reloading, pasting and editing all change the workspace and build again. The Document can be serialized, and type definitions for frontends in other languages are generated from the Rust types.
4. **Frontends are per platform and choose their own interface technology.** The desktop viewer calls the core natively through Tauri, as it does now. The web viewer calls it through a WebAssembly binding, in a Worker. The CLI calls it natively, and other tools natively or through a C interface. Frontends may share interface code, and the web viewer starts from the desktop viewer's page, but differences in the interface between platforms are acceptable.
5. **The solver's code is part of the shared core; where it runs is up to the frontend:** in a native background thread on the desktop and in the CLI, and as WebAssembly in a Worker on the web. Both use the message protocol of section 5.1 of the architecture proposal. The solver's results are RNL records written into the workspace, like any other edit.
6. **Consistency is tested at the core.** Continuous integration requires that native and WebAssembly builds give identical Documents, primitives included, on the conformance corpus. The core takes transcendental functions from a pure-Rust math library, so both builds compute the same values bit for bit.
7. **Platform floors.** The desktop viewer supports Windows 10 and later; its macOS and Linux floors are set after measuring. The web viewer keeps a conservative browser baseline, Chromium 109 and a Safari of matching age, so that it also serves older systems the desktop viewer cannot.

## Consequences

- This amends the architecture proposal. In section 6.3 the desktop no longer runs the core as WebAssembly, and the desktop and web frontends need not be the same code. In section 6.1, TypeScript and React become options for a frontend rather than requirements. In chapter 7, fonts become part of the core's drawing contract, so text looks the same across platforms up to rasterization; the font set and its licence are still to be chosen. Section 4.3 stands, with the primitives, not SVG, as the shared output.
- New work, tracked in the [roadmap](../roadmap.md): the `rnl-view` crate, which takes over the desktop viewer's document builder and the parts of `rnl-cli` that read no files; primitives and fonts in the renderer; the WebAssembly binding; the web viewer; and the test that native and WebAssembly builds agree.
- A frontend that does not suit a platform, for example where the system WebView is missing or too old, can be replaced by a native one without touching the core.
- Identical pixels across drawing engines are still not promised, because anti-aliasing and glyph rasterization differ. Geometry and text placement are identical.
- How a frontend presents editing, for example an explicit refresh after changing the text, is the frontend's decision and is not fixed here.
