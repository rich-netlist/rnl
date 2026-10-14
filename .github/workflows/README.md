# workflows

CI workflows.

| Workflow | Purpose |
| --- | --- |
| [`pages.yml`](pages.yml) | Builds the introduction page in [`site/`](../../site/README.md) in both languages. On pull requests that touch `site/` it only checks the build, which fails when a text lacks one of its languages; on `main` it also publishes the page to GitHub Pages. Pages must use "GitHub Actions" as its source. |
| [`spec-release.yml`](spec-release.yml) | On a tag such as `spec-0.2-r1`, builds every form of the standard and attaches them to the GitHub release "RNL 0.2, revision 1"; the tag's message becomes the release notes ([decision 0007](../../docs/decisions/0007-publishing-the-standard.md)). It installs pandoc, Typst and the CJK fonts at pinned versions through the local action [`install-spec-tools`](../actions/install-spec-tools/action.yml). |

See the [roadmap](../../docs/roadmap.md) for planned work.
