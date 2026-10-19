"""Release download links, including earlier tags that only shipped Windows files."""
import importlib.util
import ast
import json
import re
import subprocess
import unittest
from html.parser import HTMLParser
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("site_build", Path(__file__).with_name("build.py"))
build = importlib.util.module_from_spec(spec)
spec.loader.exec_module(build)


class Navigation(HTMLParser):
    """Read the primary links, section links and first-level section headings."""

    def __init__(self, html):
        super().__init__()
        self.pages = []
        self.contents = []
        self.sections = []
        self.section_stack = []
        self.nav = None
        self.link = None
        self.feed(html)

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if tag == "nav":
            self.nav = "pages" if "pages" in attrs.get("class", "").split() else attrs.get("id")
        if tag == "a" and self.nav in ("pages", "toc"):
            self.link = {"href": attrs.get("href"), "text": "", "class": attrs.get("class", ""), "current": attrs.get("aria-current")}
        if tag == "section":
            self.section_stack.append(attrs.get("id"))
        if tag == "h2" and len(self.section_stack) == 1:
            self.sections.append(self.section_stack[-1])

    def handle_data(self, data):
        if self.link is not None:
            self.link["text"] += data

    def handle_endtag(self, tag):
        if tag == "a" and self.link is not None:
            (self.pages if self.nav == "pages" else self.contents).append(self.link)
            self.link = None
        if tag == "nav":
            self.nav = None
        if tag == "section":
            self.section_stack.pop()


class SiteNavigation(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        root = Path(__file__).resolve().parents[1]
        cls.template = (root / "scripts/spec/page.html").read_text(encoding="utf-8")
        # Read the publisher's labels without importing its command-line dependencies.
        tree = ast.parse((root / "scripts/spec/build.py").read_text(encoding="utf-8"))
        cls.labels = next(ast.literal_eval(node.value) for node in tree.body
                          if isinstance(node, ast.Assign) and any(isinstance(t, ast.Name) and t.id == "LANGS" for t in node.targets))
        with patch.object(build, "has_linux_release", return_value=False):
            cls.built = build.build(None)

    def test_primary_navigation_matches_the_standard_in_both_languages(self):
        pages = list(build.PAGES)
        for lang in ("zh", "en"):
            expected = self.template.replace("$site-root$", "./")
            for field, label in (("intro", "intro"), ("tour", "tour"), ("spec", "spec"), ("packages", "packages"), ("tools", "tools"), ("agents", "agents")):
                expected = expected.replace(f"${field}-label$", self.labels[lang][label])
            expected = Navigation(expected).pages
            for link in expected:
                link["href"] = "spec/" if link["href"] == "#top" else link["href"].removeprefix("./") or "./"
            for page in pages:
                with self.subTest(lang=lang, page=page):
                    actual = Navigation(self.built[f"{lang}/{page}"]).pages
                    self.assertEqual([(a["href"], a["text"]) for a in actual], [(a["href"], a["text"]) for a in expected])
                    for link in actual:
                        current = link["href"] == ("./" if page == "index.html" else page)
                        self.assertEqual("is-on" in link["class"].split(), current)
                        self.assertEqual(link["current"], "page" if current else None)

    def test_contents_cover_first_level_sections_and_keep_the_hero_full_width(self):
        for lang in ("zh", "en"):
            for page in ("index.html", "language.html", "tools.html", "agents.html"):
                with self.subTest(lang=lang, page=page):
                    html = self.built[f"{lang}/{page}"]
                    parsed = Navigation(html)
                    self.assertEqual([a["href"] for a in parsed.contents], [f"#{s}" for s in parsed.sections])
                    self.assertLess(html.index('id="top"'), html.index('<div class="layout">'))
                    self.assertNotIn("toc-page", html)

    def test_shared_navigation_css_keeps_the_standard_values(self):
        css = (Path(__file__).parent / "src/assets/intro.css").read_text(encoding="utf-8")
        for selector in (".topbar", ".topbar-in", ".pages", ".pages a", ".pages a:hover", ".pages a.is-on", ".layout", "#toc", "#toc ul ul", "#toc a.here"):
            rule = re.escape(selector) + r"\s*\{([^}]+)\}"
            self.assertEqual(re.search(rule, css)[1], re.search(rule, self.template)[1], selector)


class ReleaseDownloads(unittest.TestCase):
    def test_linux_requires_both_packages_in_the_selected_tag(self):
        for targets, expected in [(["appimage", "deb"], True), (["nsis", "msi"], False), (["deb"], False)]:
            config = SimpleNamespace(stdout=json.dumps({"bundle": {"targets": targets}}))
            with self.subTest(targets=targets), patch.object(build.subprocess, "run", return_value=config) as run:
                self.assertEqual(build.has_linux_release("0.1.2"), expected)
                self.assertEqual(run.call_args.args[0], ["git", "show", "viewer-v0.1.2:apps/desktop/tauri.linux.conf.json"])

    def test_missing_config_or_no_release_has_no_linux_links(self):
        with patch.object(build.subprocess, "run", side_effect=subprocess.CalledProcessError(128, "git")):
            self.assertFalse(build.has_linux_release("0.1.1"))
        with patch.object(build.subprocess, "run") as run:
            self.assertFalse(build.has_linux_release(None))
            run.assert_not_called()

    def test_bilingual_downloads_for_old_new_and_no_release(self):
        for version, linux in [("0.1.1", False), ("0.1.2", True), (None, False)]:
            with self.subTest(version=version), patch.object(build, "has_linux_release", return_value=linux):
                pages = build.build(version)
                for language in ["zh", "en"]:
                    for page in ["tools.html", "rnl-tools-standalone.html"]:
                        html = pages[f"{language}/{page}"]
                        self.assertNotIn("{{viewer-", html)
                        self.assertNotIn("<!--linux-release-->", html)
                        self.assertEqual("-linux-x86_64.AppImage\"" in html, linux)
                        self.assertEqual("-linux-x86_64.deb\"" in html, linux)
                        self.assertEqual("-windows-x64-setup.exe\"" in html, version is not None)
                        if linux:
                            self.assertIn("viewer-v0.1.2/RNL-Viewer-0.1.2-linux-x86_64.deb", html)
                            self.assertIn("--appimage-extract-and-run", html)
                            self.assertIn("sudo apt install ./RNL-Viewer-0.1.2", html)

    def test_linux_blocks_obey_the_release_markup_rules(self):
        self.assertEqual(build.check_release("<!--linux-release-->{{viewer-tag}}<!--/linux-release-->", "test"), [])
        self.assertTrue(build.check_release("<!--linux-release-->{{viewer-tag}}", "test"))
        self.assertTrue(build.check_release("<!--no-linux-release-->{{viewer-tag}}<!--/no-linux-release-->", "test"))


class SiteExamples(unittest.TestCase):
    def test_logo_viewer_is_lazy_and_standalone_points_to_published_viewer(self):
        with patch.object(build, "has_linux_release", return_value=False):
            pages = build.build(None)
        for lang in ("zh", "en"):
            self.assertIn('data-viewer-url="../viewer/"', pages[f"{lang}/index.html"])
            self.assertIn(f'data-viewer-url="{build.SITE_URL}viewer/"', pages[f"{lang}/rnl-intro-standalone.html"])
            self.assertNotIn("<iframe", pages[f"{lang}/index.html"])

    def test_examples_are_published_and_linked_in_both_languages(self):
        with patch.object(build, "has_linux_release", return_value=False):
            pages = build.build(None)
        for name in ("ota5t.sp", "ota5t-zh.sp", "ota5t-edits.sp", "ota5t-resolved.sp", "ota5t_tb.sp"):
            self.assertIn(f"examples/{name}", pages)
        for lang, label in (("zh", "在查看器中打开"), ("en", "Open in viewer")):
            self.assertIn(f'href="../viewer/?url=samples/site/ota5t.sp">{label}</a>', pages[f"{lang}/index.html"])
            self.assertIn(
                f'href="{build.SITE_URL}viewer/?url=samples/site/ota5t.sp">{label}</a>',
                pages[f"{lang}/rnl-intro-standalone.html"],
            )
            self.assertIn(f'href="../viewer/?url=samples/site/ota5t.sp">{label}</a>', pages[f"{lang}/language.html"])
            self.assertIn(f'href="../viewer/?url=samples/site/ota5t_tb.sp"', pages[f"{lang}/language.html"])

    def test_check_examples_detects_drift_between_scenes_and_examples(self):
        files = build.sources()
        ex = build.examples()
        self.assertEqual(build.check_examples(files, ex), [])
        drifted = dict(ex, **{"ota5t.sp": ex["ota5t.sp"].replace("#M1 @PLACE(12, 8)", "#M1 @PLACE(14, 8)")})
        errors = build.check_examples(files, drifted)
        self.assertTrue(any("L_SYM" in e for e in errors), errors)


if __name__ == "__main__":
    unittest.main()
