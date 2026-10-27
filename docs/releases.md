# Release lines

| Line | Tag | Version source | GitHub latest |
| --- | --- | --- | --- |
| Core (CLI and `@rich-netlist/runtime`) | `core-vX.Y.Z` | Root `Cargo.toml`, `[workspace.package].version`; every core crate inherits it | Yes |
| Viewer (desktop and web) | `viewer-vX.Y.Z` | `apps/desktop/Cargo.toml` and `apps/desktop/tauri.conf.json`, which must match | No |
| Language standard | `spec-X.Y-rN` | The language series and revision in `spec/` and the existing spec tag | No |

The web build reads the Viewer version from `tauri.conf.json`. Both About dialogs also name
the core version and build commit. A Viewer release updates its two version declarations,
its own lockfile entry and release notes; a core release updates the workspace version,
the core entries in both lockfiles and core notes. Neither release changes the other line's
version. Existing releases and their latest flags are left as they are. The first core
publication moves latest from the standard to the application.

## Rehearse before publishing

Run **Core release** (`core-release.yml`) manually on the proposed branch. It validates the
versions and `release-notes/core/X.Y.Z.md`, builds four native CLIs and the npm package, tests
each extracted CLI, smoke-tests the npm tarball with Vite, and compares the downloaded Linux
artifact with a debug CLI byte for byte for `check`, `view` and `render` over both conformance
series. The combined `rnl-core-X.Y.Z` artifact contains four archives, the npm tarball and
`SHA256SUMS.txt`. Individual platform artifacts also have checksums.

PRs and manual rehearsals build, pack and smoke-test the npm package even when that core
version is already published. The registry publication dry-run runs only for a pushed
`core-v*` tag; Viewer-only changes do not require a new core version or npm publication.

Windows x64 uses MSVC with `+crt-static` and checks DLL imports for VC++/UCRT dependencies.
Linux x86_64 uses static musl on Ubuntu 22.04 and checks ELF dependencies and interpreter.
macOS Apple Silicon and Intel each build and execute on a matching native runner. Linux
arm64 is not built in the initial matrix. CLI filenames have no version suffix, so the
latest release offers stable links such as
`https://github.com/rich-netlist/rnl/releases/latest/download/rnl-x86_64-pc-windows-msvc.zip`.
Archives contain MIT, CC0 and OFL licences and the font copyright notice.

Run **Viewer release** manually on the same branch to test its Windows and Linux packaging.
Both workflows accept an optional `candidate_tag` to check a proposed tag without creating
one. For example, dispatch Core release with `core-v999.0.0` or Viewer release with
`viewer-v999.0.0`: validation fails and build jobs are skipped. A manual run, even on a real
tag, **never publishes**. Core release also runs as a PR check for relevant changes.

## Publish the core, after the user's release decision

1. Confirm the version and the npm dist-tag. The user chose (2026-10-22) core 0.2.0 as the
   first core release and npm `latest`. The user chose (2026-10-23) Viewer 0.2.0 as the next
   independent Viewer release; preparing it does not publish a new core version. The user chose
   (2026-10-23) core 0.3.0 and Viewer 0.3.0, released together, with npm `latest`, and then
   Viewer 0.3.1 with the macOS packages and the Linux Chinese font fallback, keeping core 0.3.0. The user
   chose (2026-10-24) core 0.4.0 and Viewer 0.4.0, released together, with npm `latest`;
   (2026-10-25) core 0.4.1 and Viewer 0.4.1, released together, with npm `latest`;
   (2026-10-25) core 0.5.0 and Viewer 0.5.0, released together, with npm `latest`;
   (2026-10-25) core 0.5.1 alone, with npm `latest`; Viewer stays 0.5.0; and
   (2026-10-26) core 0.5.2 alone, with npm `latest`; Viewer stays 0.5.0; and
   (2026-10-27) Viewer 0.5.1 alone; core stays 0.5.2; and
   (2026-10-27) core 0.5.3 alone, with npm `latest`; Viewer stays 0.5.1.
2. Update the chosen core version and locks, rename/write `release-notes/core/X.Y.Z.md`, and
   set repository variable `CORE_NPM_DIST_TAG` to the agreed `next` or `latest`. Publication
   fails if this variable is absent or has another value. Do not infer a dist-tag from SemVer.
3. Review a successful rehearsal on that commit, then create and push `core-vX.Y.Z` only
   when release approval has been given. Tag and workspace version must match exactly.

`publish-runtime.yml` listens only to `core-v*` and calls `core-release.yml` to build and
validate everything first. Its filename stays unchanged because npm's trusted publisher is
bound to it. After every build, test, parity check and checksum passes, it publishes the
packed npm tarball via OIDC, smoke-tests the published version, then creates **RNL core X.Y.Z**
with `--latest`. The npm version equals the core tag's version. No `runtime-v*` tag triggers
publication anymore: historical `runtime-v0.1.0-preview.N` tags belonged to the former,
independently numbered npm previews (including preview.15). Their published packages remain
available and unchanged. Use an exact npm version to avoid ambiguity while the first core
dist-tag is undecided; bare `npm i @rich-netlist/runtime` follows npm's `latest` pointer.

### Failure and recovery

GitHub and npm cannot commit a release atomically. Build/test/validation failures publish
neither. If npm publication fails, the GitHub release job cannot run. If the published npm
smoke test or GitHub release creation fails after npm succeeds, npm may contain the version
while GitHub has no core release; its latest pointer has not moved. An interrupted GitHub
upload may also leave a partial release: inspect its assets and checksums before treating it
as complete.

Do not republish or unpublish an existing npm version. Inspect the failed run and compare
the published package's integrity with the tested tarball; resolve the failure, rerun only
the failed smoke/release jobs with that run's retained artifacts. If a partial GitHub release
exists, repair its assets/checksums explicitly before setting it latest. Any recovery still
needs the original release approval. A new version needs a new reviewed tag and notes.

## Other lines

Viewer publication retains `--latest=false` and its own release notes. Standard publication
also uses `--latest=false` with its existing `spec-*` tags and files. To download a Viewer,
select its latest `viewer-v*` release rather than the repository's latest endpoint. The
website continues to resolve Viewer downloads from those tags; CLI links use the fixed
latest endpoints. Until the first core release, those CLI links have no matching assets.

The CLI is unsigned. Windows may show SmartScreen. A macOS browser download may require
`xattr -d com.apple.quarantine rnl` after verifying and extracting the archive.
