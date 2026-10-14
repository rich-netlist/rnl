# Contributing

RNL is at an early draft stage. The language, module boundaries, and package names can still change, so discuss before investing in a large change.

## Before you start

- Open an issue or a draft pull request before changing the language (`spec/`) or a module boundary described in the [toolchain architecture](docs/rnl-toolchain-architecture.md).
- Keep status explicit. A draft stays labeled as a draft, a proposal as a proposal, and nothing is described as implemented or verified unless it is.
- Record a settled project decision as a new numbered file in [`docs/decisions/`](docs/decisions/README.md). Do not rewrite an accepted record; supersede it with a new one.
- Keep the [roadmap](docs/roadmap.md) as the single list of open and completed work.

## License of contributions

You keep the copyright in your contribution. By submitting it, you license it under the license that applies to the path it changes, as listed in the [README](README.md#license) and recorded in [`REUSE.toml`](REUSE.toml):

| Path | Your contribution is licensed under |
| --- | --- |
| `spec/`, `docs/` | CC-BY-4.0 |
| `libraries/`, `tests/`, `examples/` | CC0-1.0, which waives your rights as far as the law allows |
| Everything else, including code | MIT |

By submitting a contribution you also confirm that you have the right to license it this way. No separate sign-off or contributor agreement is required.

Do not copy material that cannot be licensed this way. This includes code or text from Analog Canvas, which is licensed under AGPL-3.0, unless its copyright holder has relicensed it. If you need to add third-party material under another compatible license, say so in the pull request and record its license in `REUSE.toml` and `LICENSES/`.

## Checks

When you add files or change licensing, run [`reuse lint`](https://reuse.software/) from the repository root; it must report the project as compliant. Keep relative Markdown links working.

When you change the Rust crates, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test --workspace` must pass. When a change is meant to alter the results of the [conformance corpus](tests/conformance/README.md), write its expected files again with `RNL_BLESS=1` and review the difference.

When you change the pages of the site, edit `site/src/` and write every text in both languages, as described in [`site/README.md`](site/README.md); `python site/build.py --check` must pass.
