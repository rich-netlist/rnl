# workflows

CI workflows.

| Workflow | Purpose |
| --- | --- |
| [`pages.yml`](pages.yml) | Builds the introduction pages in [`site/`](../../site/README.md) in both languages, and the pages of the language standard with [`scripts/spec/`](../../scripts/spec/README.md). On pull requests that touch `site/`, `spec/` or `scripts/spec/` it only checks the build, which fails when a text lacks one of its languages or a check of the standard fails; on `main` it also publishes the pages to GitHub Pages. It runs on `main` again after `spec-release.yml` succeeds, so that the pages of the standard carry the new revision label; only `main` may deploy to the `github-pages` environment. Pages must use "GitHub Actions" as its source. |
| [`spec-release.yml`](spec-release.yml) | On a tag such as `spec-0.2-r1`, builds every form of the standard and attaches them to the GitHub release "RNL 0.2, revision 1"; the tag's message becomes the release notes ([decision 0007](../../docs/decisions/0007-publishing-the-standard.md)). |

Both install pandoc, Typst and the CJK fonts at pinned versions through the local action [`install-spec-tools`](../actions/install-spec-tools/action.yml).

See the [roadmap](../../docs/roadmap.md) for planned work.
