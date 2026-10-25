"""Tests of scripts/i18n/status.py: python -m unittest discover -s scripts/i18n -p 'test_*.py'"""

import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import status  # noqa: E402

REGISTRY = {"default": "en", "legacy": "zh", "languages": [
    {"code": "en", "name": "English", "nav_breakpoint": 960, "fonts": {}, "messages": {"intro": "Introduction", "tools": "Tools"}},
    {"code": "zh", "name": "中文", "status": "published", "messages": {"intro": "介绍"}},
    {"code": "ja", "name": "日本語"},
]}


class Repository(unittest.TestCase):
    """A small repository in a temporary folder, with its registry, surfaces and history."""

    def setUp(self):
        self.root = Path(tempfile.mkdtemp(prefix="rnl-i18n-"))
        self.addCleanup(shutil.rmtree, self.root, True)
        self.git("init", "-q")
        self.git("config", "user.email", "test@example.com")
        self.git("config", "user.name", "test")
        self.write("i18n/languages.json", json.dumps(REGISTRY))

    def git(self, *args):
        subprocess.run(["git", "-C", str(self.root), *args], check=True, capture_output=True)

    def write(self, rel, text):
        path = self.root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8", newline="\n")

    def commit(self, message="change"):
        self.git("add", "-A")
        self.git("commit", "-q", "-m", message)

    def surfaces(self, *surfaces):
        self.write("i18n/surfaces.json", json.dumps({"format": 1, "source_language": "en", "surfaces": list(surfaces)}))

    def survey(self, base=None):
        p = status.Project(self.root, base)
        return p, {s.id: s for s in status.survey(p)}


class Documents(Repository):
    SURFACE = {"id": "readme", "title": "README", "kind": "document", "source": "README.md",
               "translation": "README.{lang}.md", "stamp": "repo", "structure": "position"}

    def test_a_stamp_tells_current_from_stale_and_names_the_commit_translated_from(self):
        self.surfaces(self.SURFACE)
        self.write("README.md", "# Title\n\nOne.\n")
        self.commit()
        first = subprocess.run(["git", "-C", str(self.root), "rev-parse", "HEAD"], capture_output=True, text=True).stdout.strip()
        self.write("README.zh.md", "# 标题\n\n一。\n")
        p, _ = self.survey()
        self.assertIn("stamped", status.stamp(p, str(self.root / "README.zh.md")))
        self.commit()
        _, s = self.survey()
        self.assertEqual((s["readme"].langs["zh"].done, s["readme"].langs["zh"].stale), (1, 0))
        self.assertEqual([f.state for f in s["readme"].langs["ja"].findings], ["missing"])
        self.write("README.md", "# Title\n\nOne, changed.\n")
        self.commit()
        _, s = self.survey()
        stale = [f for f in s["readme"].langs["zh"].findings if f.state == "stale"]
        self.assertEqual(len(stale), 1)
        self.assertIn(first[:8], stale[0].text)
        self.assertIn("changed in 1 commit since", stale[0].text)

    def test_without_a_stamp_the_history_judges(self):
        self.surfaces(self.SURFACE)
        self.write("README.md", "# Title\n")
        self.write("README.zh.md", "# 标题\n")
        self.commit()
        _, s = self.survey()
        self.assertEqual([f.state for f in s["readme"].langs["zh"].findings], ["unstamped"])
        self.write("README.md", "# Title, changed\n")
        self.commit()
        _, s = self.survey()
        self.assertEqual(s["readme"].langs["zh"].stale, 1)

    def test_a_stamp_of_another_source_is_an_error_and_stamp_as_of_a_commit(self):
        self.surfaces(self.SURFACE)
        self.write("README.md", "# Title\n")
        self.write("README.zh.md", "<!-- translated from OTHER.md sha256:0123456789ab -->\n# 标题\n")
        self.commit()
        _, s = self.survey()
        self.assertTrue(any("OTHER.md" in e for e in s["readme"].errors))
        self.write("README.zh.md", "<!-- translated from README.md -->\n# 标题\n")
        _, s = self.survey()
        self.assertTrue(any("not a stamp" in e for e in s["readme"].errors))
        self.write("README.md", "# Title, later\n")
        p, _ = self.survey()
        status.stamp(p, str(self.root / "README.zh.md"), "HEAD")
        self.assertTrue((self.root / "README.zh.md").read_text(encoding="utf-8").startswith(
            f"<!-- translated from README.md sha256:{status.source_hash('# Title' + chr(10))} -->\n# 标题"))
        _, s = self.survey()
        self.assertEqual(s["readme"].langs["zh"].stale, 1)


class Inline(Repository):
    SURFACE = {"id": "pages", "title": "Pages", "kind": "inline", "source": ["site/src/*.html"],
               "translation": {"*": "i18n/{lang}/site/{path}.json"}, "inline": ["zh"]}

    def test_groups_are_counted_per_language_from_the_source_and_from_catalogs(self):
        self.surfaces(self.SURFACE)
        self.write("site/src/index.html", "<h1><zh>你好</zh><en>Hello</en></h1>\n<p><en>Two  words</en></p>\n")
        self.write("i18n/ja/site/index.html.json", json.dumps({"Two words": "二語", "Hello": None}))
        _, s = self.survey()
        self.assertEqual(s["pages"].total, 2)
        self.assertEqual(s["pages"].langs["zh"].done, 1)
        self.assertEqual(s["pages"].langs["ja"].done, 1)  # null is not yet translated; the key's spaces are folded

    def test_markup_errors_and_new_placeholders(self):
        self.surfaces(self.SURFACE)
        self.write("site/src/index.html", "<p><zh>交叉 {n} 处 {m}</zh><en>{n} crossings</en></p>\n<p><zh>x</zh></p>\n")
        _, s = self.survey()
        self.assertTrue(s["pages"].errors)
        self.assertEqual([f.state for f in s["pages"].langs["zh"].findings], ["differs"])

    def test_against_a_base_an_english_change_beside_an_unchanged_translation_is_reported(self):
        self.surfaces(self.SURFACE)
        self.write("site/src/index.html", "<p><zh>一个电路</zh><en>A circuit drawn by hand</en></p>\n"
                                          "<p><zh>二</zh><en>Second paragraph here</en></p>\n")
        self.commit()
        self.write("site/src/index.html", "<p><zh>一个电路</zh><en>A circuit drawn by the solver</en></p>\n"
                                          "<p><zh>第二段</zh><en>Second paragraph, changed</en></p>\n")
        _, s = self.survey(base="HEAD")
        changed = [f for f in s["pages"].langs["zh"].findings if f.state == "changed"]
        self.assertEqual(len(changed), 1)
        self.assertIn("by hand", changed[0].text)


class Dictionaries(Repository):
    def test_json_and_registry_messages_and_release_summaries(self):
        self.surfaces(
            {"id": "timeline", "title": "T", "kind": "dictionary", "format": "json", "source": "timeline.json"},
            {"id": "messages", "title": "M", "kind": "dictionary", "format": "registry-messages", "source": "i18n/languages.json"},
            {"id": "notes", "title": "N", "kind": "dictionary", "format": "release-summary", "source": "notes/*.md"})
        self.write("timeline.json", json.dumps({"milestones": [{"id": "a", "title": {"en": "Old", "zh": "旧"}},
                                                               {"id": "b", "title": {"en": "Only English"}}]}))
        self.write("notes/1.md", '<!-- timeline-summary\n{"en": "Released.", "zh": "已发布。"}\n-->\n# 1\n')
        self.commit()
        self.write("timeline.json", json.dumps({"milestones": [{"id": "a", "title": {"en": "New", "zh": "旧"}},
                                                               {"id": "b", "title": {"en": "Only English"}}]}))
        _, s = self.survey(base="HEAD")
        self.assertEqual((s["timeline"].total, s["timeline"].langs["zh"].done, s["timeline"].langs["zh"].stale), (2, 1, 1))
        self.assertEqual((s["messages"].total, s["messages"].langs["zh"].done, s["messages"].langs["ja"].done), (2, 1, 0))
        self.assertEqual((s["notes"].total, s["notes"].langs["zh"].done), (1, 1))


class Messages(Repository):
    SURFACE = {"id": "viewer", "title": "Viewer", "kind": "messages",
               "source": {"js": ["ui/*.js"], "commands": "ui/commands.json", "html": "ui/index.html", "rust": ["app/*.rs"]},
               "translation": {"zh": "ui/zh-CN.json", "*": "i18n/{lang}/viewer.json"}, "keep": ["RNL Viewer"]}

    def test_the_scan_finds_every_form_and_pairs_a_changed_text_with_its_old_entry(self):
        self.surfaces(self.SURFACE)
        self.write("ui/a.js", 'x = m("Open File");\ny = m`Open ${name} now ${m`and ${k} more`}`;\n'
                              "z = m('It\\'s here\\u2026');\nw = m`Line\\nbreak`;\n"
                              'const LABELS = [tr("Keep"), tr(\'Drop\')]; v = m("{0} lines", n);\n')
        self.write("ui/commands.json", json.dumps({"menus": [{"label": "&File", "items": [
            {"id": "language-en", "label": "English"}, {"id": "quit", "label": "&Quit"}]}]}))
        self.write("ui/index.html", '<html><head><title>RNL Viewer</title></head><body><h1>RNL Viewer</h1>'
                                    '<button title="Close the panel">×</button><p>Drop a netlist here</p>'
                                    '<script>var s = "not this";</script></body></html>')
        self.write("app/menu.rs", 'let t = menu::text(window.app_handle(), "Export Image");\n')
        self.write("ui/zh-CN.json", json.dumps({"Open File": "打开文件", "Drop netlists here": "把网表拖到这里",
                                                "&Quit": "退出(&Q)", "Unused entry": "旧"}))
        p, s = self.survey()
        found = status.scan_viewer(p, self.SURFACE["source"])
        for text in ("Open File", "Open {0} now {1}", "and {0} more", "It's here…", "Line\nbreak", "&File", "&Quit",
                     "Close the panel", "Drop a netlist here", "Export Image", "Keep", "Drop", "{0} lines"):
            self.assertIn(text, found)
        self.assertNotIn("English", found)
        self.assertNotIn("not this", found)
        zh = s["viewer"].langs["zh"]
        self.assertEqual(zh.done, 2)
        changed = [f for f in zh.findings if f.state == "changed"]
        self.assertEqual(len(changed), 1)
        self.assertIn("Drop netlists here", changed[0].text)
        self.assertEqual([f.text for f in zh.findings if f.state == "unused"], ["'Unused entry' is not found by the scan"])

    def test_format_errors_of_a_catalog(self):
        self.surfaces(self.SURFACE)
        self.write("ui/a.js", 'm`Saved ${n} file${s}`; m("&Open...\\tCtrl+O");\n')
        self.write("ui/commands.json", "{}")
        self.write("ui/zh-CN.json", json.dumps({"Saved {0} file{1}": "已保存 {0} 个文件",  # a plural's {1} left out: fine
                                                "&Open...\tCtrl+O": "打开(&O)(&P)...\tCtrl+P"}))
        self.write("i18n/ja/viewer.json", json.dumps({"Saved {0} file{1}": "{2}"}))
        _, s = self.survey()
        errors = " ".join(s["viewer"].errors)
        self.assertIn("shortcut", errors)
        self.assertIn("access key", errors)
        self.assertIn("a placeholder", errors)
        self.assertIn("ui/index.html does not exist", errors)
        self.assertEqual(sum("zh-CN.json" in e and "placeholder" in e for e in s["viewer"].errors), 0)


class Aliases(Repository):
    def test_coverage_of_the_words_and_names_without_the_header_words(self):
        self.surfaces({"id": "alias", "title": "A", "kind": "aliases", "words": "words.json", "packages": "lib/*.rnl",
                       "translation": "lib/rnl-{lang}.rnl", "no_alias": ["$NMOS"]})
        self.write("words.json", json.dumps({"keywords": [
            {"keyword": "VERSION", "where": ["header"], "params": [{"name": "version"}]},
            {"keyword": "PLACE", "where": ["level"], "params": [{"name": "x"}]}], "kinds": [{"name": "net"}]}))
        self.write("lib/rnl-std.rnl", '$R = @DEFINE-SYMBOL(); $NMOS = @DEFINE-SYMBOL();\n')
        self.write("lib/rnl-zh.rnl", '@ALIAS("@位于", "@PLACE"); @ALIAS("网络", "net");\n')
        _, s = self.survey()
        self.assertEqual(s["alias"].total, 4)  # @PLACE, x, net, $R
        self.assertEqual(s["alias"].langs["zh"].done, 2)
        self.assertEqual(sorted(f.text for f in s["alias"].langs["zh"].findings), ["$R", "x"])
        self.assertEqual(s["alias"].langs["ja"].done, 0)


class TheRepository(unittest.TestCase):
    """The repository's own registry and surfaces."""

    def test_every_surface_is_read_without_format_errors(self):
        p = status.Project(status.ROOT)
        surfaces = status.survey(p)
        self.assertEqual(p.errors + [e for s in surfaces for e in s.errors], [])
        for s in surfaces:
            if s.kind in ("document", "inline", "dictionary", "messages", "aliases"):
                self.assertGreater(s.total, 0, s.id)

    def test_files_of_a_new_language_and_the_cli(self):
        p = status.Project(status.ROOT)
        listed = "\n".join(status.files(p, "zh"))
        self.assertIn("README.zh.md", listed)
        self.assertIn("spec/0.3/zh/ch01.md", listed)
        r = subprocess.run([sys.executable, str(HERE / "status.py"), "check"], capture_output=True, text=True,
                           encoding="utf-8")
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertIn("site-pages", r.stdout)


if __name__ == "__main__":
    unittest.main()
