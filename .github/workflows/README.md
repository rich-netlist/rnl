# workflows

CI workflows.

| Workflow | Purpose |
| --- | --- |
| [`pages.yml`](pages.yml) | Builds the introduction page in [`site/`](../../site/README.md) in both languages. On pull requests that touch `site/` it only checks the build, which fails when a text lacks one of its languages; on `main` it also publishes the page to GitHub Pages. Pages must use "GitHub Actions" as its source. |
| [`desktop.yml`](desktop.yml) | Checks formatting, runs clippy and the tests of the desktop viewer in [`apps/desktop`](../../apps/desktop/README.md) on Linux, Windows and macOS, and checks the page's script. Runs on pushes to `main` and on pull requests that touch the viewer, the crates or the standard library. |
| [`rust.yml`](rust.yml) | Checks formatting, runs clippy and the tests of the Rust workspace on Linux, Windows and macOS, and builds the core crates for WebAssembly. Runs on pushes to `main` and on pull requests that touch the crates, the standard library or the test corpora. |

See the [roadmap](../../docs/roadmap.md) for planned work.
