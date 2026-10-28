"""Release history, manual overrides and the homepage's unchanged surrounding markup."""
import importlib.util
import json
from pathlib import Path
import re
import subprocess
import tempfile
import unittest
from unittest.mock import Mock, patch

spec = importlib.util.spec_from_file_location("timeline_build", Path(__file__).with_name("build.py"))
build = importlib.util.module_from_spec(spec)
spec.loader.exec_module(build)


def milestone(key="manual", stamp="2026-10-19", **fields):
    return {"id": key, "date": stamp, "title": {"zh": "里程碑", "en": "Milestone"},
            "summary": {"zh": "已经发布。", "en": "Released."}, **fields}


class Timeline(unittest.TestCase):
    def test_all_three_release_lines_generate_entries_in_shanghai_order(self):
        tags = [{"tag": "viewer-v0.1.5", "date": "2026-10-21T15:59:59Z"},
                {"tag": "spec-0.4-r1", "date": "2026-10-21T16:00:00Z"},
                {"tag": "core-v0.2.0", "date": "2026-10-21T16:00:01Z"},
                {"tag": "runtime-v0.1.0-preview.99", "date": "2026-10-21T16:00:02Z"}]
        summary = Mock(return_value={"zh": "新功能。", "en": "New features."})
        events = build.timeline_events({"milestones": []}, list(reversed(tags)), summary)
        self.assertEqual([e["id"] for e in events], [t["tag"] for t in tags[:3]])
        self.assertEqual([str(e["stamp"].date()) for e in events], ["2026-10-21", "2026-10-22", "2026-10-22"])
        self.assertEqual(events[-1]["title"]["en"], "RNL core 0.2.0")
        self.assertEqual(events[1]["title"]["zh"], "语言标准 0.4，修订 1")
        self.assertEqual(summary.call_count, 3)

    def test_manual_entry_can_override_and_combine_tags_without_reading_notes(self):
        event = milestone(tags=["spec-0.3-r1", "spec-0.3-r2"])
        summary = Mock(side_effect=AssertionError("overridden notes must not be read"))
        events = build.timeline_events({"milestones": [event]},
                                      [{"tag": tag, "date": "2026-10-18"} for tag in event["tags"]], summary)
        self.assertEqual([e["id"] for e in events], ["manual"])
        summary.assert_not_called()

    def test_only_the_latest_published_item_is_current_even_with_ties_and_plans(self):
        data = {"milestones": [milestone("later", "2026-10-21T16:00:01Z"),
                               milestone("older"), milestone("earlier", "2026-10-21T16:00:00Z")],
                "next": [{"title": {"zh": "计划", "en": "Plan"},
                          "description": {"zh": "计划中。", "en": "Planned."}}]}
        html = build.timeline_html(data, [])
        self.assertEqual(html.count('class="now"'), 1)
        self.assertIn('<li class="now"><time>2026-10-22</time>', html)
        self.assertLess(html.index('class="now"'), html.index('class="tl-sep"'))
        self.assertLess(html.index('class="tl-sep"'), html.index('class="next"'))

    def test_the_homepage_keeps_the_latest_entries_and_links_to_the_full_timeline(self):
        count = build.HOME_ENTRIES + 2
        data = {"milestones": [milestone(f"m{i}", f"2026-10-{10 + i}") for i in range(count)],
                "next": [{"title": {"zh": "计划", "en": "Plan"}, "description": {"zh": "计划中。", "en": "Planned."}}]}
        home = build.timeline_html(data, [])
        self.assertEqual(home.count("<time>"), build.HOME_ENTRIES)
        self.assertIn(f"<time>2026-10-{10 + count - 1}</time>", home)
        self.assertNotIn("<time>2026-10-10</time>", home)
        self.assertEqual(home.count('class="now"'), 1)
        self.assertEqual(home.count('href="timeline.html"'), 1)
        self.assertLess(home.index("<time>"), home.index('class="tl-more"'))
        self.assertLess(home.index('class="tl-more"'), home.index('class="tl-sep"'))
        # nothing to link to while the homepage shows everything
        few = build.timeline_html({**data, "milestones": data["milestones"][:build.HOME_ENTRIES]}, [])
        self.assertNotIn("timeline.html", few)

    def test_the_full_timeline_has_every_entry_the_latest_first_and_no_plans(self):
        count = build.HOME_ENTRIES + 2
        data = {"milestones": [milestone(f"m{i}", f"2026-10-{10 + i}") for i in range(count)],
                "next": [{"title": {"zh": "计划", "en": "Plan"}, "description": {"zh": "计划中。", "en": "Planned."}}]}
        full = build.timeline_html(data, [], full=True)
        dates = re.findall(r"<time>([\d-]+)</time>", full)
        self.assertEqual(dates, sorted(dates, reverse=True))
        self.assertEqual(len(dates), count)
        self.assertEqual(full.count('class="now"'), 1)
        self.assertTrue(full.lstrip().startswith('<li class="now">'))
        self.assertNotIn("tl-sep", full)
        self.assertNotIn("tl-more", full)
        self.assertNotIn('class="next"', full)

    def test_dates_reject_missing_timezone_and_reversed_ranges(self):
        with self.assertRaises(build.BuildError):
            build.timeline_date("2026-10-21T16:00:00")
        with self.assertRaises(build.BuildError):
            build.timeline_events({"milestones": [milestone(start_date="2026-10-20")]}, [])

    def test_duplicate_manual_tags_and_missing_english_fail(self):
        for events in ([milestone("a", tags=["viewer-v0.1.4"]), milestone("b", tags=["viewer-v0.1.4"])],
                       [milestone(summary={"zh": "缺少英文"})]):
            with self.subTest(events=events), self.assertRaises(build.BuildError):
                build.timeline_events({"milestones": events}, [])

    def test_plain_summaries_are_escaped_and_unsafe_links_fail(self):
        data = {"milestones": [milestone(summary={"zh": "<script>", "en": "<script>"})]}
        self.assertIn("&lt;script&gt;", build.timeline_html(data, []))
        data["milestones"][0]["url"] = "javascript:alert(1)"
        with self.assertRaises(build.BuildError):
            build.timeline_html(data, [])

    def test_builder_changes_only_the_timeline_in_regular_and_standalone_homepages(self):
        template = build.sources()
        with patch.object(build, "has_linux_release", return_value=False):
            actual = build.build(None)
        # the timeline, and the notice its texts may bring: a release published in English alone leaves a
        # language's homepage partly translated until its summary is translated (decision 0015)
        remove = lambda html: re.sub(r'\n<div class="wrap translation-notice[^"]*" lang="en">.*?</div>', '',
                                    re.sub(r'(<ol class="tl">).*?(</ol>)', r'\1\2', html, flags=re.S), flags=re.S)
        for index, (language, _) in enumerate(build.LANGS):
            if language not in build.I18N.published:
                continue  # an incubating language is built only with --incubating
            expected = build.build_language(template, index, None)
            for page in ("index.html", "rnl-intro-standalone.html"):
                self.assertEqual(remove(actual[f"{language}/{page}"]), remove(expected[page]))


class ReleaseSummaries(unittest.TestCase):
    def test_real_tags_use_their_notes_and_dates_including_future_core_support(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            def git(*args, env=None):
                return subprocess.run(["git", *args], cwd=root, check=True, capture_output=True, env=env)
            git("init")
            git("config", "user.name", "Timeline fixture")
            git("config", "user.email", "timeline@example.invalid")
            summary = {"zh": "一句摘要。", "en": "One-sentence summary."}
            notes = "<!-- timeline-summary\n" + json.dumps(summary) + "\n-->\n"
            for path in ("release-notes/core/0.2.0.md", "apps/desktop/release-notes/0.1.5.md"):
                target = root / path
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_text(notes, encoding="utf-8")
            git("add", ".")
            git("commit", "-m", "Fixture release notes")
            for tag in ("core-v0.2.0", "viewer-v0.1.5", "spec-0.4-r1"):
                git("tag", "-a", tag, "-m", notes)
            # A later checkout edit must not replace the summary actually attached to a release.
            (root / "release-notes/core/0.2.0.md").write_text(notes.replace("One-sentence", "Edited"), encoding="utf-8")
            with patch.object(build, "ROOT", root):
                tags = build.timeline_tags()
                self.assertEqual({t["tag"] for t in tags}, {"core-v0.2.0", "viewer-v0.1.5", "spec-0.4-r1"})
                for item in tags:
                    self.assertIsNotNone(build.timeline_date(item["date"]).tzinfo)
                    self.assertEqual(build.release_summary(item["tag"]), summary)

    def test_legacy_notes_can_be_backfilled_without_changing_old_tags(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            notes = root / "apps/desktop/release-notes/0.1.2.md"
            notes.parent.mkdir(parents=True)
            notes.write_text('<!-- timeline-summary\n{"zh":"已有摘要。","en":"Backfilled summary."}\n-->\n', encoding="utf-8")
            with patch.object(build, "ROOT", root), patch.object(build, "git_text", return_value=None):
                self.assertEqual(build.release_summary("viewer-v0.1.2")["en"], "Backfilled summary.")

    def test_missing_malformed_and_missing_english_summaries_fail(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(build, "ROOT", Path(directory)):
            for notes in ("No summary", "<!-- timeline-summary\nnot JSON\n-->",
                          '<!-- timeline-summary\n{"zh":"没有英文"}\n-->'):
                with self.subTest(notes=notes), patch.object(build, "git_text", return_value=notes):
                    with self.assertRaises(build.BuildError):
                        build.release_summary("spec-0.4-r1")


if __name__ == "__main__":
    unittest.main()
