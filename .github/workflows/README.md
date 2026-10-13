# workflows

CI workflows.

| Workflow | Purpose |
| --- | --- |
| [`pages.yml`](pages.yml) | Builds the introduction page in [`site/`](../../site/README.md) in both languages. On pull requests that touch `site/` it only checks the build, which fails when a text lacks one of its languages; on `main` it also publishes the page to GitHub Pages. Pages must use "GitHub Actions" as its source. |
| [`desktop.yml`](desktop.yml) | Checks formatting, runs clippy and the tests of the desktop viewer in [`apps/desktop`](../../apps/desktop/README.md) on Linux, Windows and macOS, and checks every script of the page it shows, [`apps/viewer-ui`](../../apps/viewer-ui/README.md). On Windows it also runs the app's smoke test, which drives it through WebView2's DevTools port. Runs on pushes to `main` and on pull requests that touch the viewer, its page, the crates or the standard library. |
| [`viewer.yml`](viewer.yml) | Tests the viewers' shared page in Chromium, Firefox and WebKit with Playwright, against a fake shell ([`tests/viewer`](../../tests/viewer/README.md)), on Linux; installs the browsers with what they need, and keeps the report of a failed run. Runs on pushes to `main` and on pull requests that touch the page, its tests, the conformance corpus, the crates or the standard library. |
| [`rust.yml`](rust.yml) | Checks formatting, runs clippy and the tests of the Rust workspace on Linux, Windows and macOS. On Linux it also builds the core crates and the WebAssembly binding in [`bindings/runtime-wasm`](../../bindings/runtime-wasm/README.md), tests the binding's JavaScript modules, and requires that native and WebAssembly builds give identical Documents on the conformance corpus and the standard packages. Runs on pushes to `main` and on pull requests that touch the crates, the bindings, the standard library or the test corpora. |

See the [roadmap](../../docs/roadmap.md) for planned work.
