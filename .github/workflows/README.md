# workflows

CI workflows.

| Workflow | Purpose |
| --- | --- |
| [`pages.yml`](pages.yml) | Builds the introduction page in [`site/`](../../site/README.md) in both languages. On pull requests that touch `site/` it only checks the build, which fails when a text lacks one of its languages; on `main` it also publishes the page to GitHub Pages. Pages must use "GitHub Actions" as its source. |
| [`desktop.yml`](desktop.yml) | Checks formatting, runs clippy and the tests of the desktop viewer in [`apps/desktop`](../../apps/desktop/README.md) on Linux, Windows and macOS, and checks every script of the page it shows, [`apps/viewer-ui`](../../apps/viewer-ui/README.md). Runs on pushes to `main` and on pull requests that touch the viewer, its page, the crates or the standard library. |
| [`rust.yml`](rust.yml) | Checks formatting, runs clippy and the tests of the Rust workspace on Linux, Windows and macOS. On Linux it also builds the core crates and the WebAssembly binding in [`bindings/runtime-wasm`](../../bindings/runtime-wasm/README.md), tests the binding's JavaScript modules, and requires that native and WebAssembly builds give identical Documents on the conformance corpus and the standard packages. Runs on pushes to `main` and on pull requests that touch the crates, the bindings, the standard library or the test corpora. |
| [`web.yml`](web.yml) | Builds the web viewer in [`apps/web`](../../apps/web/README.md) on Linux: the core as WebAssembly and the site with the samples; checks every script of the site with `node --check`, and that the site is served with the MIME types a browser needs. Runs on pushes to `main` and on pull requests that touch the web viewer, the page, the bindings, the crates, the standard library or the conformance corpus, which gives the samples. |

See the [roadmap](../../docs/roadmap.md) for planned work.
