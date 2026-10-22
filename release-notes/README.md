# Timeline summaries for releases

The homepage history is built by `site/build.py` from `site/src/timeline.json` and Git tags.
Dates use Asia/Shanghai (UTC+08:00): annotated tags use their creation time; lightweight tags
use the commit time. Fetch tags and full history before building, as Pages CI already does.

Every new release needs a one-sentence English summary, with translations when available. Copy the block in
[`TEMPLATE.md`](TEMPLATE.md) into the release notes before tagging:

| Release | Tag | Summary source |
| --- | --- | --- |
| Viewer | `viewer-vX.Y.Z` | `apps/desktop/release-notes/X.Y.Z.md` |
| Language standard | `spec-X.Y-rN` | The annotated tag's message, which already becomes its release notes |
| Core | `core-vX.Y.Z` | `release-notes/core/X.Y.Z.md` |

Core tags are supported before the independent core release pipeline is merged (PR #168).
There are no core tags yet, so no core/CLI release is shown as having happened. The old
`runtime-v*` npm previews are curated historical milestones, not automatically generated.

The summary block is JSON inside an HTML comment named `timeline-summary`. `en` must be a
nonempty, single-paragraph string. Other language codes registered in `site/languages.json`
are optional and fall back to English when absent. Summaries are plain text; markup is escaped.
Missing or invalid summaries fail the site build instead of silently leaving stale history.
The generated entry links to its GitHub Release. The builder first reads the notes at the
tag; for old tags without a summary, it accepts a backfill in the current checkout. This PR
backfills Viewer 0.1.2–0.1.4 without moving tags or editing existing GitHub Releases.

## Editorial milestones

Add non-release milestones to `site/src/timeline.json`, with a unique `id`, an ISO `date`,
localized `title` and `description` (reviewed inline HTML is supported), and a `url` with
optional localized `link_label`. Each dictionary needs `en`; other languages are optional.
A plain-text `summary` can be used instead of `description`.
Use a timezone-qualified PR merge timestamp for `date`; it is converted to Shanghai time.
A date alone is interpreted as midnight in Shanghai. `start_date` gives a date range.
Keep source links to the actual merge, tag or published artifact, rather than planned work.

An optional `tags` list claims those tags for a manually edited or combined milestone. It
takes precedence over automatic entries, including its title, description, links and date.
Duplicate manual claims fail the build. The builder sorts milestones by publication time
and stable id, and marks only the last published entry `now`; the `next` list stays separate.
The homepage's existing timeline elements and CSS are retained. The versioned Stage text
is intentionally unchanged so that this update alters only the timeline on the homepage.

Pages already rebuilds after Standard release and Viewer release. It now also rebuilds
after Publish runtime succeeds, covering the core pipeline when PR #168 lands. Release
publication behavior is unchanged.
