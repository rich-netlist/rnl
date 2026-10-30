"""Release download links, including earlier tags that only shipped Windows files."""
import importlib.util
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
        cls.labels = {code: locale["messages"] for code, locale in build.I18N.locales.items()}
        with patch.object(build, "has_linux_release", return_value=False):
            cls.built = build.build(None)

    def test_primary_navigation_matches_the_standard_in_both_languages(self):
        pages = list(build.PAGES)
        for lang in ("zh", "en"):
            expected = self.template.replace("$site-root$", "./")
            for field, label in (("intro", "intro"), ("tour", "tour"), ("spec", "spec"), ("packages", "packages"), ("gallery", "gallery"), ("tools", "tools"), ("agents", "agents")):
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
    def test_macos_requires_app_and_dmg_in_the_selected_tag(self):
        for targets, expected in [(["app", "dmg"], True), (["dmg"], False), (["nsis", "msi"], False)]:
            with self.subTest(targets=targets), patch.object(build.subprocess, "run", return_value=SimpleNamespace(stdout=json.dumps({"bundle": {"targets": targets}}))) as run:
                self.assertEqual(build.has_macos_release("0.2.0"), expected)
                self.assertEqual(run.call_args.args[0], ["git", "show", "viewer-v0.2.0:apps/desktop/tauri.macos.conf.json"])
        with patch.object(build.subprocess, "run", side_effect=subprocess.CalledProcessError(128, "git")):
            self.assertFalse(build.has_macos_release("0.1.4"))
        with patch.object(build.subprocess, "run") as run:
            self.assertFalse(build.has_macos_release(None))
            run.assert_not_called()

    def test_bilingual_macos_links_are_independent_of_linux_and_absent_from_old_tags(self):
        for version, macos in [("0.2.0", True), ("0.1.4", False), (None, False)]:
            for linux in [True, False]:
                with self.subTest(version=version, macos=macos, linux=linux), patch.object(build, "has_macos_release", return_value=macos), patch.object(build, "has_linux_release", return_value=linux):
                    pages = build.build(version)
                    for language in ["en", "zh"]:
                        html = pages[f"{language}/tools.html"]
                        self.assertEqual("-macos-arm64.dmg\"" in html, macos)
                        self.assertEqual("-macos-x86_64.dmg\"" in html, macos)
                        self.assertNotIn("<!--macos-release-->", html)
                        self.assertNotIn("{{viewer-", html)
                        if macos:
                            self.assertIn("xattr -dr com.apple.quarantine", html)
                            self.assertIn("Library/Application Support/", html)

    def test_macos_blocks_obey_the_release_markup_rules(self):
        self.assertEqual(build.check_release("<!--macos-release-->{{viewer-tag}}<!--/macos-release-->", "test"), [])
        self.assertTrue(build.check_release("<!--macos-release-->{{viewer-tag}}", "test"))
        self.assertTrue(build.check_release("<!--no-macos-release-->{{viewer-tag}}<!--/no-macos-release-->", "test"))

    def test_the_public_packages_need_their_script_in_the_selected_tag(self):
        with patch.object(build.subprocess, "run", return_value=SimpleNamespace(stdout="")) as run:
            self.assertTrue(build.has_packages_release("0.5.2"))
            self.assertEqual(run.call_args.args[0], ["git", "cat-file", "-e", "viewer-v0.5.2:scripts/bundle_packages.py"])
        with patch.object(build.subprocess, "run", side_effect=subprocess.CalledProcessError(128, "git")):
            self.assertFalse(build.has_packages_release("0.5.1"))
        with patch.object(build.subprocess, "run") as run:
            self.assertFalse(build.has_packages_release(None))
            run.assert_not_called()

    def test_bilingual_downloads_with_the_public_packages_only_for_tags_that_have_them(self):
        files = ["-with-packages-windows-x64.zip", "-with-packages-linux-x86_64.AppImage", "-linux-public-packages.deb",
                 "-with-packages-macos-arm64.dmg", "-with-packages-macos-x86_64.dmg"]
        for version, packages in [("0.5.2", True), ("0.5.1", False), (None, False)]:
            with self.subTest(version=version), patch.object(build, "has_packages_release", return_value=packages),                     patch.object(build, "has_linux_release", return_value=True), patch.object(build, "has_macos_release", return_value=True):
                pages = build.build(version)
                for language in ["en", "zh", "de"]:
                    html = pages[f"{language}/tools.html"]
                    self.assertNotIn("<!--packages-release-->", html)
                    self.assertNotIn("{{viewer-", html)
                    for name in files:
                        self.assertEqual(f"RNL-Viewer-{version}{name}\"" in html, packages, name)
                    self.assertEqual('id="package-downloads"' in html, packages)
                    if packages:
                        self.assertIn("viewer-v0.5.2/RNL-Viewer-0.5.2-linux-public-packages.deb", html)
                        self.assertIn("sudo apt install ./RNL-Viewer-0.5.2-linux-public-packages.deb", html)

    def test_the_blocks_of_the_public_packages_obey_the_release_markup_rules(self):
        self.assertEqual(build.check_release("<!--packages-release-->{{viewer-tag}}<!--/packages-release-->", "test"), [])
        self.assertTrue(build.check_release("<!--packages-release-->{{viewer-tag}}", "test"))
        self.assertTrue(build.check_release("<!--no-packages-release-->{{viewer-tag}}<!--/no-packages-release-->", "test"))

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
        for name in ("ota5t.sp", "ota5t-zh.sp", "ota5t-edits.sp", "ota5t_tb.sp", "ota5t-review.sp", "ota5t-review-bare.sp", "ota5t-review-zh.sp", "ota5t-review-eda.sp"):
            self.assertIn(f"examples/{name}", pages)
        for lang, label in (("zh", "在查看器中打开"), ("en", "Open in viewer")):
            self.assertIn(f'href="../viewer/?url=samples/site/ota5t-review.sp" target="_blank" rel="noopener">{label}</a>', pages[f"{lang}/index.html"])
            self.assertIn(
                f'href="{build.SITE_URL}viewer/?url=samples/site/ota5t-review.sp" target="_blank" rel="noopener">{label}</a>',
                pages[f"{lang}/rnl-intro-standalone.html"],
            )
            self.assertIn(f'href="../viewer/?url=samples/site/ota5t.sp" target="_blank" rel="noopener">{label}</a>', pages[f"{lang}/language.html"])
            self.assertIn(f'href="../viewer/?url=samples/site/ota5t_tb.sp"', pages[f"{lang}/language.html"])

    def test_check_examples_detects_drift_between_scenes_and_examples(self):
        files = build.sources()
        ex = build.examples()
        self.assertEqual(build.check_examples(files, ex), [])
        drifted = dict(ex, **{"ota5t.sp": ex["ota5t.sp"].replace("#M1 @PLACE(12, 8)", "#M1 @PLACE(14, 8)")})
        errors = build.check_examples(files, drifted)
        self.assertTrue(any("L_SYM" in e for e in errors), errors)

    def test_check_examples_detects_drift_between_the_ota_and_its_files(self):
        files = build.sources()
        ex = build.examples()
        self.assertEqual(build.check_ota(files, ex), [])
        # a statement of the English file, one of the notation, one of the Chinese file, a line of the netlist
        for name, old, new, where in (
                ("ota5t-review.sp", "%cm1 = #M3(ref) and #M4(out) treated @AS $CURRENT_MIRROR;", "%cm1 = #M3(ref) and #M4(out) treated @AS $MIRRORED;", "T[cm1]"),
                ("ota5t-review-bare.sp", "{#M5 #M1 #M3} @ORDER(\"bottom-to-top\");", "{#M5 #M1 #M3} @ORDER(\"top-to-bottom\");", "T[ord]"),
                ("ota5t-review-zh.sp", "#M5:d 有 @电流(\"流入\")", "#M5:d 有 @电流(\"流出\")", "TZ[cur]"),
                ("ota5t-review.sp", "M5 tail vbias vss  vss nch W=4u L=1u", "M5 tail vbias vss  vss nch W=8u L=1u", "lines")):
            self.assertIn(old, ex[name])
            errors = build.check_ota(files, dict(ex, **{name: ex[name].replace(old, new)}))
            self.assertTrue(any(where in e for e in errors), (where, errors))
        # a line that the script does not have
        errors = build.check_ota(files, dict(ex, **{"ota5t-review.sp": ex["ota5t-review.sp"].replace("* }\n.ends", "* }\n* one more line\n.ends")}))
        self.assertTrue(any("one more line" in e for e in errors), errors)
        # the core's drawings were made from other files than these: the examples were changed and not drawn again
        errors = build.check_ota(files, dict(ex, **{"ota5t-review.sp": ex["ota5t-review.sp"].replace("W=4u L=0.5u", "W=5u L=0.5u")}))
        self.assertTrue(any("render-examples" in e for e in errors), errors)
        # the simulation was run on a testbench that is not the example's present text
        self.assertEqual(build.check_sim(files, ex), [])
        errors = build.check_sim(files, dict(ex, **{"ota5t-review.sp": ex["ota5t-review.sp"].replace("W=4u L=0.5u", "W=5u L=0.5u")}))
        self.assertTrue(any("ota5t-review-tb.sp" in e for e in errors), errors)
        errors = build.check_ota(files, dict(ex, **{"ota5t-review-eda.sp": ex["ota5t-review-eda.sp"].replace("W=4u L=0.5u", "W=5u L=0.5u")}))
        self.assertTrue(any("ota5t-review-eda.sp" in e for e in errors), errors)


if __name__ == "__main__":
    unittest.main()
