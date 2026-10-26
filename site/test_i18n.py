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
    # in place of the repository's own Japanese, if it has one
    config["languages"] = [entry for entry in config["languages"] if entry["code"] != "ja"]
    config["languages"].append({"code": "ja", "name": "日本語", "short": "JA", "status": "published"})
    return config


def third_messages(directory, messages=None):
    """The catalog of the third language's shared messages, in `directory` as i18n/ja/site/ is in i18n/."""
    file = Path(directory) / "ja/site/messages.json"
    file.parent.mkdir(parents=True, exist_ok=True)
    file.write_text(json.dumps(messages or {"intro": "ホーム"}, ensure_ascii=False), encoding="utf-8")
    return directory


class Localization(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.registry = Languages(third_language(), catalogs=third_messages(directory.name))

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

    def test_an_incubating_language_is_built_only_on_demand_and_offered_nowhere(self):
        config = third_language()
        del config["languages"][-1]["status"]  # no status: incubating, as a language not yet reviewed
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        registry = Languages(config, catalogs=third_messages(directory.name))
        self.assertEqual(registry.published, ["en", "zh"])
        self.assertNotIn('data-language="ja"', registry.menu("en", lambda target: target))
        self.assertIn('data-language="ja" aria-current="true"', registry.menu("ja", lambda target: target))
        with patch.object(build, "I18N", registry), patch.object(build, "has_linux_release", return_value=False), \
                patch.object(build, "LANGS", [(code, code + "/") for code in registry.codes]), \
                patch.object(build, "CODES", registry.codes), patch.object(build, "DIRS", {c: c + "/" for c in registry.codes}):
            published, everything = build.build(None), build.build(None, incubating=True)
        self.assertFalse(any(path.startswith("ja/") for path in published))
        self.assertIn("ja/index.html", everything)
        self.assertNotIn('"ja"', published["index.html"])  # the root never leads to it
        self.assertNotIn('data-language="ja"', everything["en/index.html"])

    def test_the_translation_status_page_and_the_footer_link_to_it(self):
        with patch.object(build, "has_linux_release", return_value=False):
            result = build.build(None)
        page = result["en/translation.html"]
        self.assertIn('<div class="ts-report" lang="en">', page)  # the report in English, as its texts are
        self.assertIn("<code>site-pages</code>", page)
        self.assertIn("<h3>Not counted</h3>", page)
        self.assertIn('<span lang="zh-CN">中文</span>', result["zh/translation.html"])
        for lang, label in (("en", "Translation status"), ("zh", "翻译进度")):
            for name in build.PAGES:
                if name != "translation.html":
                    self.assertIn(f'<a href="translation.html">{label}</a>', result[f"{lang}/{name}"], (lang, name))

    def test_the_glossary_pages_and_the_notice_that_english_terms_are_normative(self):
        with patch.object(build, "has_linux_release", return_value=False):
            result = build.build(None)
        self.assertIn('<a href="../zh/glossary.html">Chinese', result["en/glossary.html"])
        page = result["zh/glossary.html"]
        self.assertIn('<div class="gs-report" lang="en">', page)
        self.assertIn('<td lang="zh-CN" class="gs-term">导线</td>', page)
        for name in build.PAGES:  # every page of a language with a glossary links to it, in English
            self.assertIn('<div class="terms-notice" lang="en">', result[f"zh/{name}"], name)
            self.assertIn('<a href="glossary.html">Glossary</a>', result[f"zh/{name}"], name)
            self.assertNotIn("terms-notice", result[f"en/{name}"], name)

    def test_the_build_refuses_translations_in_the_registry_and_the_timeline(self):
        config = third_language()
        config["languages"][-1]["messages"] = {"intro": "ホーム"}
        with patch.object(build, "I18N", Languages(config)), self.assertRaises(build.BuildError) as raised:
            build.build()
        self.assertIn("i18n/ja/site/messages.json", str(raised.exception))
        data = {"milestones": [{"id": "one", "date": "2026-10-22", "title": {"en": "Title", "zh": "标题"},
                                "summary": {"en": "Summary."}}]}
        with tempfile.TemporaryDirectory() as directory, patch.object(build, "SRC", Path(directory)):
            (Path(directory) / "timeline.json").write_text(json.dumps(data, ensure_ascii=False), encoding="utf-8")
            with self.assertRaises(build.BuildError) as raised:
                build.timeline_html(tags=[])
        self.assertIn("src/timeline.json.milestones[one].title: a translation written beside the English", str(raised.exception))

    def test_whole_site_builds_a_third_locale_from_one_registry_entry_and_its_catalogs(self):
        with tempfile.TemporaryDirectory() as directory:
            directory = Path(directory)
            config = directory / "languages.json"
            config.write_text(json.dumps(third_language()), encoding="utf-8")
            third_messages(directory)
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

    def test_the_timeline_and_the_shared_messages_from_their_catalogs(self):
        with tempfile.TemporaryDirectory() as directory:
            registry = self.registry(Path(directory), {"timeline.json": {"Two words": "二語", "Not yet": None},
                                                       "messages": {"tools": "ツール"}})
            self.assertEqual(registry.with_catalog({"en": "Two\n  words"}, "timeline.json"), {"en": "Two\n  words", "ja": "二語"})
            self.assertEqual(registry.with_catalog({"en": "Not yet"}, "timeline.json"), {"en": "Not yet"})
            # released notes keep the translations written in them
            self.assertEqual(registry.with_catalog({"en": "Two words", "ja": "既に"}, "timeline.json")["ja"], "既に")
            self.assertEqual(registry.message("ja", "tools"), "ツール")
            self.assertEqual(registry.message("ja", "language_menu"), registry.message("en", "language_menu"))

    def test_a_language_translated_by_ai_says_so_at_the_top_of_every_page(self):
        config = third_language()
        config["languages"][-1].update(names={"en": "Japanese"}, ai_translated=True)
        registry = Languages(config)
        notice = registry.ai_notice("ja", "tools.html")
        self.assertIn('<div class="ai-notice" lang="en">', notice)  # in English, as every notice about translation
        self.assertIn("Translated from English by AI; it may contain errors.", notice)
        self.assertIn('href="https://github.com/rich-netlist/rnl/issues/new?labels=translation&amp;'
                      'title=Translation%20%28Japanese%29%3A%20tools.html&amp;body=Page%3A%20https%3A//rich-netlist.github.io/rnl/ja/tools.html', notice)
        self.assertIn(">Report an issue</a>", notice)
        self.assertEqual(registry.ai_notice("zh", "tools.html"), "")  # Chinese is reviewed
        self.assertEqual(registry.ai_notice("en", "tools.html"), "")
        with tempfile.TemporaryDirectory() as directory:
            directory = Path(directory)
            (directory / "languages.json").write_text(json.dumps(config), encoding="utf-8")
            third_messages(directory)
            env = dict(os.environ, RNL_SITE_LANGUAGES=str(directory / "languages.json"), PYTHONDONTWRITEBYTECODE="1")
            subprocess.run([sys.executable, "site/build.py", "--out", str(directory / "dist")], cwd=ROOT,
                           env=env, check=True, capture_output=True, text=True)
            for name in build.PAGES:
                ja = (directory / "dist/ja" / name).read_text(encoding="utf-8")
                self.assertEqual(ja.count('class="ai-notice"'), 1, name)
                self.assertLess(ja.index("</header>"), ja.index('class="ai-notice"'))  # under the top bar
                for lang in ("en", "zh"):
                    self.assertNotIn('class="ai-notice"', (directory / "dist" / lang / name).read_text(encoding="utf-8"))

    def test_notices_of_pages_not_wholly_translated_are_in_english(self):
        """As every notice of the system, in every language (decision 0015)."""
        config = third_language()
        config["languages"][-1]["names"] = {"en": "Japanese"}
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        registry = Languages(config, catalogs=third_messages(directory.name, {"not_translated": "この頁はまだ訳されていません"}))  # not used
        self.assertEqual(registry.notice("ja", 10, 10), "")
        self.assertEqual(registry.notice("en", 0, 10), "")
        partial, english = registry.notice("ja", 8, 10), registry.notice("ja", 7, 10)
        self.assertIn('class="wrap translation-notice is-partial" lang="en"', partial)
        self.assertIn("Parts of this page are not yet available in Japanese; they are shown in English.", partial)
        self.assertIn('class="wrap translation-notice" lang="en"', english)
        self.assertIn("This page is not yet available in Japanese; it is shown in English.", english)
        self.assertNotIn("訳されて", english)
        self.assertIn(">Help translate it</a>", english)
        self.assertIn("i18n/README.md", english)
        plain = Languages(third_language())
        self.assertIn("not yet available in Chinese", plain.notice("zh", 0, 10))
        self.assertIn("not yet available in 日本語", plain.notice("ja", 0, 10))  # no English name: its own

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
            self.assertIn('class="wrap translation-notice" lang="en"', index)
            self.assertNotIn("（訳）", index)  # shown in English: its one translated text too
            self.assertIn("A netlist you can simulate", index)
            self.assertIn('<html lang="ja" data-locale="ja">', index)
            # English, the source, has none; how complete the repository's Chinese is, which a release in
            # English alone changes until it is translated, is not this test's concern
            for name in build.PAGES:
                self.assertNotIn("translation-notice", page("en", name), name)


if __name__ == "__main__":
    unittest.main()
