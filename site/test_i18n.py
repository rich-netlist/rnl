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
from site_i18n import CONFIG, Languages  # noqa: E402

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


if __name__ == "__main__":
    unittest.main()
