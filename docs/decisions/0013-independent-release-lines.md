# 0013 — Independent core, Viewer and language release lines

Status: accepted (2026-10-22). Initial version numbers and npm dist-tag await the user's decision.

The user asked: “再写一个每次发布rnl时自动编译可执行文件的任务单, 顺便把skill也补充一下,
让用户可以直接下载可执行文件用”. They also asked: “这个共用viewer-v有点奇怪?
viewer可能会在核心没有更新的情况下更新很多个版本, 那每次更新viewer都会附带更新rnl核心的版本?
另外latest release作为语言标准是否也有点奇怪, 是不是应该让release回归应用程序,
语言单独弄一个language之类的tag?”

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
