"""Missing translations, language menus and adding a locale without changing source pages."""
import copy
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
from site_i18n import CONFIG, Languages, norm  # noqa: E402

spec = importlib.util.spec_from_file_location("i18n_site_build", ROOT / "site/build.py")
build = importlib.util.module_from_spec(spec)
spec.loader.exec_module(build)


def third_language():
    config = json.loads(CONFIG.read_text(encoding="utf-8"))
    config["languages"].append({"code": "ja", "name": "日本語", "short": "JA", "messages": {"intro": "ホーム"}})
    return config


class Localization(unittest.TestCase):
    def setUp(self):
        self.registry = Languages(third_language())

    def test_optional_translations_and_intentional_empty_blocks(self):
        text = '<zh>中文</zh><en>English</en> / <en>Fallback</en> / <zh></zh><en>Visible</en>'
        self.assertEqual(self.registry.errors(text, "fixture"), [])
        self.assertEqual(self.registry.localize(text, "zh"), '中文 / Fallback / ')
        self.assertEqual(self.registry.localize(text, "ja"), 'English / Fallback / Visible')
        self.assertEqual(self.registry.localize('<ja>日本語</ja><en>English</en>', "ja"), '日本語')

    def test_markup_in_attributes_and_scripts_and_dictionary_fallback(self):
        text = '<p title="<en>Title</en>"></p><script>var label = "<zh>中文</zh><en>English</en>";</script>'
        self.assertIn('title="Title"', self.registry.localize(text, "ja"))
        self.assertIn('var label = "English"', self.registry.localize(text, "ja"))
        self.assertEqual(self.registry.text({"en": "Fallback"}, "zh"), "Fallback")
        self.assertEqual(self.registry.text({"en": "Fallback", "zh": ""}, "zh"), "")
        self.assertEqual(self.registry.locales["ja"]["html_lang"], "ja")
        self.assertEqual(self.registry.message("ja", "intro"), "ホーム")
        self.assertEqual(self.registry.message("ja", "tools"), "Tools")
        self.assertEqual(self.registry.standard("ja"), "en")

    def test_malformed_duplicate_nested_and_missing_english_fail(self):
        for text in ('<zh>缺少英文</zh>', '<zh>Broken<en>English</en>',
                     '<zh>One</zh><zh>Two</zh><en>English</en>',
                     '<en>Nested <ja>日本語</ja></en>', '<en>Open'):
            with self.subTest(text=text):
                self.assertTrue(self.registry.errors(text, "fixture"))
        with self.assertRaises(ValueError):
            self.registry.text({"zh": "没有英文"}, "zh")

    def test_registry_rejects_duplicate_codes_and_invalid_default(self):
        config = third_language()
        for change in (lambda c: c["languages"].append(copy.deepcopy(c["languages"][0])),
                       lambda c: c.update(default="ja")):
            candidate = copy.deepcopy(config)
            change(candidate)
            with self.assertRaises(ValueError):
                Languages(candidate)

    def test_timeline_dictionary_and_release_summary_fall_back_to_english(self):
        data = {"milestones": [{"id": "english", "date": "2026-10-22", "title": {"en": "Title"},
                                "summary": {"en": "English summary."}}]}
        for index, code in enumerate(build.CODES):
            html = build.localize(build.timeline_html(data, []), index)
            self.assertIn('English summary.', html)
        with tempfile.TemporaryDirectory() as directory, patch.object(build, "ROOT", Path(directory)), \
                patch.object(build, "git_text", return_value='<!-- timeline-summary\n{"en":"English only."}\n-->'):
            self.assertEqual(build.release_summary("spec-0.4-r1"), {"en": "English only."})

    def test_whole_site_builds_a_third_locale_from_only_one_registry_entry(self):
        with tempfile.TemporaryDirectory() as directory:
            directory = Path(directory)
            config = directory / "languages.json"
            config.write_text(json.dumps(third_language()), encoding="utf-8")
            env = dict(os.environ, RNL_SITE_LANGUAGES=str(config), PYTHONDONTWRITEBYTECODE="1")
            subprocess.run([sys.executable, "site/build.py", "--out", str(directory / "dist")], cwd=ROOT,
                           env=env, check=True, capture_output=True, text=True)
            for file in build.PAGES:
                html = (directory / "dist/ja" / file).read_text(encoding="utf-8")
                self.assertIn('<html lang="ja" data-locale="ja">', html)
                self.assertIn('>ホーム</a>', html)
                self.assertIn('data-language="ja" aria-current="true"', html)
                self.assertNotIn('<en>', html)
                self.assertNotRegex(html, r'\{\{[a-z][a-z-]*\}\}')
            intro = (directory / "dist/ja/index.html").read_text(encoding="utf-8")
            self.assertIn('A netlist you can simulate', intro)
            self.assertIn('href="../en/spec/"', intro)
            self.assertIn('href="../en/spec/0.3/"', intro)
            single = (directory / "dist/ja/rnl-intro-standalone.html").read_text(encoding="utf-8")
            self.assertNotIn('src="assets/', single)
            self.assertIn('https://rich-netlist.github.io/rnl/en/', single)
            script = ('import runpy; m=runpy.run_path("scripts/spec/build.py"); '
                      'print(m["page_meta"]("en", "Title", "spec/0.3/compare-0.2.html", "../../")["language-menu"])')
            menu = subprocess.run([sys.executable, "-c", script], cwd=ROOT, env=env, check=True,
                                  capture_output=True, encoding="utf-8").stdout
            self.assertIn('data-language="ja"', menu)
            self.assertIn('href="../../../en/spec/0.3/compare-0.2.html"', menu)


class Catalogs(unittest.TestCase):
    """Translations from the catalogs in i18n/<lang>/site/, and the notices of pages not wholly translated."""

    def registry(self, directory, catalogs):
        for path, data in catalogs.items():
            file = directory / "ja/site" / (path + ".json")
            file.parent.mkdir(parents=True, exist_ok=True)
            file.write_text(json.dumps(data, ensure_ascii=False), encoding="utf-8")
        return Languages(third_language(), catalogs=directory)

    def test_a_catalog_translates_by_the_english_and_a_block_beside_it_comes_first(self):
        with tempfile.TemporaryDirectory() as directory:
            registry = self.registry(Path(directory), {"page.html": {
                "Two words": "二語", "Hello": None, "Empty": "", "Block wins": "訳"}})
            text = '<en>Two\n  words</en>|<en>Hello</en>|<en>Empty</en>|<ja>枠</ja><en>Block wins</en>'
            self.assertEqual(registry.localize(text, "ja", "page.html"), "二語|Hello||枠")
            self.assertEqual(registry.localize(text, "ja", "other.html"), "Two\n  words|Hello|Empty|枠")
            self.assertEqual(registry.localize(text, "ja", "page.html", english=True), "Two\n  words|Hello|Empty|Block wins")
            self.assertEqual(registry.localize(text, "zh", "page.html"), "Two\n  words|Hello|Empty|Block wins")
            self.assertEqual(registry.coverage(text, "ja", "page.html"), (3, 4))
            self.assertEqual(registry.coverage(text, "en", "page.html"), (4, 4))
            self.assertEqual(registry.catalog_errors("ja", "page.html"), [])

    def test_a_context_tells_two_ways_of_one_english_apart(self):
        with tempfile.TemporaryDirectory() as directory:
            registry = self.registry(Path(directory), {"page.html": {"Yes": "はい", "readable|Yes": "読める"}})
            text = '<en>Yes</en>|<en ctx=readable>Yes</en>|<en ctx=other>Yes</en>'
            self.assertEqual(registry.errors(text, "page.html"), [])
            self.assertEqual(registry.parts("<zh>看得懂</zh><en ctx=readable>Yes</en>"), ({"zh": "看得懂", "en": "Yes"}, "readable"))
            self.assertEqual(registry.localize(text, "ja", "page.html"), "はい|読める|Yes")
            self.assertEqual(registry.localize(text, "en", "page.html"), "Yes|Yes|Yes")
            self.assertEqual(registry.coverage(text, "ja", "page.html"), (2, 3))

    def test_malformed_catalogs_and_what_is_only_other(self):
        with tempfile.TemporaryDirectory() as directory:
            registry = self.registry(Path(directory), {
                "a.html": {"Hello <b>{name}</b>": "<b>{name}</b> {count}",
                           "<b>Bold</b> text": "<b>太字",
                           '<a href="tools.html">Tools</a>': '<a href="index.html">ツール</a>',
                           "<code>x</code> and y": "x と y",
                           "readable|Plain": "It's \"quoted\""},
                "assets/c.js": {"Plain": "It's here"},
                "b.html": ["not an object"]})
            errors = "\n".join(registry.catalog_errors("ja", "a.html"))
            self.assertIn("a placeholder", errors)
            self.assertIn("do not open and close", errors)
            # in a page's text other tags and links, and quotes, are the translator's: they are only other
            for part in ("links", "tags", "straight quotes"):
                self.assertNotIn(part, errors)
            self.assertEqual(len(registry.catalog_errors("ja", "a.html")), 2)
            self.assertEqual(registry.differences('<a href="tools.html">Tools</a>', '<a href="index.html">ツール</a>'),
                             ["its links are not those of the English"])
            self.assertEqual(registry.differences("<code>x</code> and y", "x と y"), ["its tags are not those of the English"])
            # in a script's string, a straight quote would end the string
            self.assertIn("straight quotes", registry.catalog_errors("ja", "assets/c.js")[0])
            self.assertIn("an object", registry.catalog_errors("ja", "b.html")[0])
            (Path(directory) / "ja/site/c.html.json").write_text("{", encoding="utf-8")
            self.assertIn("not valid JSON", registry.catalog_errors("ja", "c.html")[0])

    def test_notices_of_pages_not_wholly_translated(self):
        registry = Languages(third_language())
        self.assertEqual(registry.notice("ja", 10, 10), "")
        self.assertEqual(registry.notice("en", 0, 10), "")
        partial, english = registry.notice("ja", 8, 10), registry.notice("ja", 7, 10)
        self.assertIn('class="wrap translation-notice is-partial" lang="ja"', partial)
        self.assertIn("not yet available in 日本語", partial)
        self.assertIn('class="wrap translation-notice" lang="ja"', english)
        self.assertIn("i18n/README.md", english)
        self.assertIn("本页尚无中文", registry.notice("zh", 0, 10))
        self.assertIn("本页有部分内容尚无中文", registry.notice("zh", 9, 10))

    def test_whole_site_of_a_language_with_catalogs_only(self):
        """agents.html translated wholly, gallery.html but for two texts, index.html hardly at all."""
        registry = Languages(third_language())
        sources = build.sources()

        def as_japanese(path, leave=0, only=None):
            groups = [registry.parts(m.group())[0] for m in registry.group.finditer(sources[path])]
            entries = {norm(g["en"]): "（訳）" + g["en"] for g in groups}
            keys = list(entries)[:only] if only else list(entries)[leave:]
            return {key: entries[key] for key in keys}
        with tempfile.TemporaryDirectory() as directory:
            directory = Path(directory)
            catalogs = {"agents.html": as_japanese("agents.html"), "assets/agents.js": as_japanese("assets/agents.js"),
                        "assets/common.js": as_japanese("assets/common.js"),
                        "gallery.html": as_japanese("gallery.html"), "assets/circuits.js": as_japanese("assets/circuits.js", leave=2),
                        "index.html": as_japanese("index.html", only=1)}
            self.registry(directory, catalogs)
            config = directory / "languages.json"
            config.write_text(json.dumps(third_language()), encoding="utf-8")
            env = dict(os.environ, RNL_SITE_LANGUAGES=str(config), PYTHONDONTWRITEBYTECODE="1")
            subprocess.run([sys.executable, "site/build.py", "--out", str(directory / "dist")], cwd=ROOT,
                           env=env, check=True, capture_output=True, text=True)
            page = lambda lang, name: (directory / "dist" / lang / name).read_text(encoding="utf-8")
            self.assertNotIn("translation-notice", page("ja", "agents.html"))
            self.assertIn("（訳）", page("ja", "agents.html"))
            self.assertIn('translation-notice is-partial', page("ja", "gallery.html"))
            index = page("ja", "index.html")
            self.assertIn('class="wrap translation-notice" lang="ja"', index)
            self.assertNotIn("（訳）", index)  # shown in English: its one translated text too
            self.assertIn("A netlist you can simulate", index)
            self.assertIn('<html lang="ja" data-locale="ja">', index)
            for lang in ("en", "zh"):
                for name in build.PAGES:
                    self.assertNotIn("translation-notice", page(lang, name), (lang, name))


if __name__ == "__main__":
    unittest.main()
