# 0013 — Independent core, Viewer and language release lines

Status: accepted (2026-10-22). Initial version numbers and npm dist-tag await the user's decision.

The user asked for a task that builds the executables automatically each time rnl is released,
and for the skill to tell users that they can download them directly. They then questioned the
shared `viewer-v` tag: the viewer may have many releases while the core does not change, so
should each viewer release also bump the version of the core? They also found it odd that the
latest release stood for the language standard, and asked whether releases should go back to
the applications, with the language under a tag of its own.

The resulting choice is three independent lines:

- **Core** uses `core-vX.Y.Z`. The root workspace version remains the version of every core
  crate, the CLI archives and the npm package. One tag builds, tests and publishes both CLI
  and npm. This replaces the npm-only `runtime-v*` triggers; historic previews remain intact.
- **Viewer** keeps `viewer-vX.Y.Z`. Its version is declared only in
  `apps/desktop/Cargo.toml` and `tauri.conf.json`; web and desktop show that version, plus the
  core version and build commit in About. Releasing one line does not bump the other.
- **Language standard** keeps its existing `spec-*` tags and revision scheme.

The repository's latest GitHub Release belongs to the core. Viewer and standard releases
explicitly opt out. Existing releases are not edited. Version/tag mismatches fail before
builds and publication. Manual runs only build and test artifacts. The first actual core
tag still requires release approval. [Release procedures](../releases.md) describe the
platform matrix, unsigned downloads, npm OIDC and non-atomic publication recovery.
