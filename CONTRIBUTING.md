# Contributing

## Releases

The core (`core-vX.Y.Z`, CLI plus npm), Viewer (`viewer-vX.Y.Z`) and language (`spec-*`)
have independent versions. Only core releases become GitHub latest. See
[release procedures](docs/releases.md) for version sources, manual rehearsals, required notes,
the npm dist-tag decision and recovery after partial publication. Historical `runtime-v*`
tags no longer publish. Do not push a release tag without release approval.

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

When you change the Rust crates, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test --workspace` must pass. When a change is meant to alter the results of the [conformance corpus](tests/conformance/README.md), write its expected files again with `RNL_BLESS=1`, or `RNL_BLESS_0_3=1` for the cases of 0.3, and review the difference.

When you change the pages of the site, edit `site/src/` and write every text in both languages, as described in [`site/README.md`](site/README.md); `python site/build.py --check` must pass.

Every Viewer, standard or core release needs a bilingual `timeline-summary` in its release
notes before tagging; see the [summary template and checklist](release-notes/README.md).
The homepage is generated from tags plus `site/src/timeline.json`; add non-release milestones
there with the actual merge time and source link. Manual entries can override or combine tags.

## Public packages

From the root of **each checkout or Git worktree**, run:

```sh
python scripts/packages.py download
```

This reads `packages.lock.json`, looks up each exact name/version in the public catalog,
checks the locked SHA-256, downloads and verifies all version assets, and writes the ignored
`.rnl-packages/NAME/VERSION/` directory beside that checkout's `Cargo.toml`. No shared Git-dir
assumption is made; invoking the script by absolute path from another directory also works.
Use `python scripts/packages.py check` to verify the cache, or `python scripts/packages.py fixture`
to generate `.rnl-packages/fixture/catalog.json` and offline assets for browser tests.

Without the download, local tests that require public packages skip with this command in their
message; other tests still run and compile. In CI, missing packages or hash mismatches fail.
Rust package tests use Python to verify the cache once per integration-test process.
Browser tests use only the locked fixture, and never the online catalog. To update a lock, review
the new catalog record, change the one lock file, download again and run the relevant tests.

The public catalog and immutable version files are maintained in
[rnl-packages](https://github.com/rich-netlist/rnl-packages); publish changed content under a new
`major.minor` package version. The site/viewer follow the latest catalog; tests and skill generation
use the lock. The Pages workflow rebuilds **main** once daily at 02:17 UTC, and its Actions page
retains **Run workflow**. GitHub disables scheduled workflows in public repositories after
60 days without repository activity; re-enable the schedule in Actions if this happens.
The separate daily Public package compatibility workflow checks main against every public
package's latest version and reports failures in Actions without blocking unrelated merges.

The runtime no longer packages the optional Canvas file starting with the next npm version
after preview.14. This change does not create a tag or publish npm; release approval is separate.
