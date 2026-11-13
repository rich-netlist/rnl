"""Which versions of the standard the pages show: python -m unittest discover -s scripts/spec -p 'test_build.py'

The build itself needs pandoc (and Typst on PATH for its fonts); the tests that run it are skipped without pandoc."""
import importlib.util
import hashlib
import json
import re
import shutil
import tempfile
import unittest
import zipfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
spec_module = importlib.util.spec_from_file_location("spec_build", HERE / "build.py")
build = importlib.util.module_from_spec(spec_module)
spec_module.loader.exec_module(build)

REPO_SPEC = HERE.parents[1] / "spec"


def folder(root: Path, v: str, released: bool, zh: bool = True) -> None:
    (root / v / "en").mkdir(parents=True)
    if zh:
        (root / v / "zh").mkdir()
    if released:
        (root / v / build.RELEASED).write_text("released\n", encoding="utf-8")


class Selection(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.spec = Path(self.dir.name)
        folder(self.spec, "0.2", True)
        folder(self.spec, "0.3", True)
        folder(self.spec, "0.4", False)  # a draft
        self.all = build.versions(self.spec)

    def tearDown(self):
        self.dir.cleanup()

    def test_the_versions_are_the_folders_released_or_not(self):
        self.assertEqual(self.all, ["0.2", "0.3", "0.4"])

    def test_a_draft_is_not_shown_and_not_the_newest(self):
        self.assertEqual(build.shown_versions(self.spec, self.all), (["0.2", "0.3"], ["0.2", "0.3"]))

    def test_with_drafts_it_is_shown_but_still_not_the_newest(self):
        self.assertEqual(build.shown_versions(self.spec, self.all, drafts=True), (["0.2", "0.3", "0.4"], ["0.2", "0.3"]))

    def test_a_version_asked_for_is_shown_as_the_release_of_its_tag_builds_it(self):
        self.assertEqual(build.shown_versions(self.spec, self.all, asked="0.4"), (["0.2", "0.3", "0.4"], ["0.2", "0.3"]))

    def test_once_the_file_is_there_it_is_the_newest(self):
        (self.spec / "0.4" / build.RELEASED).write_text("released\n", encoding="utf-8")
        self.assertEqual(build.shown_versions(self.spec, self.all), (["0.2", "0.3", "0.4"], ["0.2", "0.3", "0.4"]))

    def test_a_version_in_english_only_is_a_version(self):
        folder(self.spec, "0.5", False, zh=False)
        self.assertEqual(build.versions(self.spec), ["0.2", "0.3", "0.4", "0.5"])
        self.assertEqual(build.langs(self.spec, "0.5"), ["en"])
        self.assertEqual(build.langs(self.spec, "0.4"), ["en", "zh"])

    def test_a_folder_without_en_is_not_a_version(self):
        (self.spec / "0.6" / "zh").mkdir(parents=True)
        self.assertEqual(build.versions(self.spec), self.all)

    def test_the_checks_of_the_translation_leave_an_english_only_version_alone(self):
        folder(self.spec, "0.5", False, zh=False)
        self.assertEqual(build.check_structure(self.spec, "0.5"), [])
        self.assertEqual(build.check_stamps(self.spec, "0.5"), [])

    def test_a_released_version_is_archived_by_its_file_and_stays_shown(self):
        self.assertFalse(build.archived(self.spec, "0.2"))
        (self.spec / "0.2" / build.ARCHIVED).write_text("archived\n", encoding="utf-8")
        self.assertTrue(build.archived(self.spec, "0.2"))
        self.assertEqual(build.shown_versions(self.spec, self.all), (["0.2", "0.3"], ["0.2", "0.3"]))

    def test_the_version_line_marks_an_archived_version_and_the_page_of_one_in_bold(self):
        shown = ["0.2", "0.3", "0.4"]
        line = build.version_links("en", "0.4", shown, lambda x: f"../{x}/", is_archived=lambda x: x != "0.4")
        self.assertIn('<a href="../0.3/">0.3</a> (archived)', line)
        self.assertIn("<b>0.4</b>", line)
        self.assertNotIn("0.4</b> (archived)", line)
        line = build.version_links("en", "0.3", shown, lambda x: f"../{x}/", is_archived=lambda x: x != "0.4")
        self.assertIn("<b>0.3</b> (archived)", line)
        both = build.version_links("zh", "0.4", shown, lambda x: f"../{x}/", has_text=lambda x: x != "0.2",
                                   is_archived=lambda x: x != "0.4")
        self.assertIn("0.2</a> (已归档, 仅英文)", both)

    def test_the_banner_names_the_version_the_current_one_and_the_upgrade(self):
        banner = build.archived_banner("en", "0.3", "0.4", "../../tools.html#upgrade")
        self.assertIn("archived version (0.3)", banner)
        self.assertIn("The current standard is 0.4", banner)
        self.assertIn('<a href="../../tools.html#upgrade">', banner)
        self.assertIn("已归档", build.archived_banner("zh", "0.3", "0.4", "x"))

    def test_the_versions_published_so_far_have_the_file(self):
        # not every folder: a draft in spec/ has none until it is released. Add a version here when it is.
        for v in ("0.2", "0.3"):
            self.assertTrue(build.released(REPO_SPEC, v), f"spec/{v}/{build.RELEASED}: the file that marks it as released")


class WordSchemas(unittest.TestCase):
    def test_each_word_list_uses_a_schema_that_declares_its_top_level_fields(self):
        for v in ("0.2", "0.3", "0.4"):
            with self.subTest(version=v):
                words = json.loads(build.read(REPO_SPEC / f"rnl-words-{v}.json"))
                schema_name = words["$schema"]
                schema = json.loads(build.read(REPO_SPEC / schema_name))
                self.assertEqual(schema["$id"], schema_name)
                self.assertFalse(schema["additionalProperties"])
                self.assertFalse(set(words) - set(schema["properties"]))

    def test_chapter_archives_include_the_schema_named_by_the_word_list(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            revision = {"n": None, "after": 0, "dirty": False, "date": "2026-01-01", "commit": "test", "epoch": 0}
            for v in ("0.2", "0.3", "0.4"):
                folder(root, v, False)
                for lang in ("en", "zh"):
                    (root / v / lang / "front.md").write_text("# Test\n", encoding="utf-8")
                    (root / v / lang / "ch01.md").write_text("## 1 Test\n", encoding="utf-8")
                word_name = f"rnl-words-{v}.json"
                words = json.loads(build.read(REPO_SPEC / word_name))
                schema_name = words["$schema"]
                for name in (word_name, schema_name):
                    shutil.copyfile(REPO_SPEC / name, root / name)
                for lang in ("en", "zh"):
                    with self.subTest(version=v, language=lang):
                        build.build_zip(root, lang, v, revision, root)
                        prefix = build.name(v, lang, "") + "/"
                        with zipfile.ZipFile(root / build.name(v, lang, "-chapters.zip")) as archive:
                            archived_words = json.loads(archive.read(prefix + word_name))
                            self.assertEqual(archived_words["$schema"], schema_name)
                            self.assertEqual(
                                {name for name in archive.namelist() if name.endswith(".schema.json")},
                                {prefix + schema_name},
                            )
                            manifest = archive.read(prefix + "MANIFEST.txt").decode("utf-8")
                            for name in (word_name, schema_name):
                                data = archive.read(prefix + name)
                                self.assertEqual(data.decode("utf-8"), build.read(REPO_SPEC / name))
                                self.assertIn(f"{hashlib.sha256(data).hexdigest()}  {name}", manifest)
                            self.assertIn(schema_name, archive.read(prefix + "README.md").decode("utf-8"))


@unittest.skipUnless(shutil.which("pandoc"), "needs pandoc")
class Site(unittest.TestCase):
    """The pages of a copy of spec/ in which 0.2 is released and 0.3, the real text, is a draft."""

    @classmethod
    def setUpClass(cls):
        cls.dir = tempfile.TemporaryDirectory()
        root = Path(cls.dir.name)
        cls.spec = root / "spec"
        shutil.copytree(REPO_SPEC, cls.spec)
        (cls.spec / "0.3" / build.RELEASED).unlink()
        cls.root = root

    @classmethod
    def tearDownClass(cls):
        cls.dir.cleanup()

    def run_build(self, name: str, *flags: str) -> Path:
        site = self.root / name
        code = build.main(["--spec", str(self.spec), "--site", str(site), "--out", str(self.root / (name + "-out")), "--no-pdf", *flags])
        self.assertEqual(code, 0)
        return site

    def target(self, page: Path) -> str:
        return re.search(r'<link rel="canonical" href="([^"]+)"', page.read_text(encoding="utf-8")).group(1)

    def test_a_draft_has_no_pages_and_the_addresses_lead_to_the_newest_released_version(self):
        site = self.run_build("released")
        self.assertTrue((site / "en" / "spec" / "0.2" / "index.html").is_file())
        self.assertFalse((site / "en" / "spec" / "0.3").exists())
        self.assertFalse((site / "zh" / "spec" / "0.3").exists())
        self.assertEqual(self.target(site / "en" / "spec" / "index.html"), "0.2/")
        self.assertEqual(self.target(site / "zh" / "spec" / "index.html"), "0.2/")
        self.assertEqual(self.target(site / "spec" / "index.html"), "../zh/spec/0.2/")
        for page in (site / "en" / "spec" / "0.2").glob("*.html"):
            self.assertNotIn("../0.3/", page.read_text(encoding="utf-8"), page.name)

    def test_with_drafts_the_draft_has_pages_and_the_addresses_still_lead_to_the_released_one(self):
        site = self.run_build("drafts", "--drafts")
        self.assertTrue((site / "en" / "spec" / "0.3" / "index.html").is_file())
        self.assertTrue((site / "en" / "spec" / "0.3" / "compare-0.2.html").is_file())
        self.assertEqual(self.target(site / "en" / "spec" / "index.html"), "0.2/")
        self.assertEqual(self.target(site / "zh" / "spec" / "index.html"), "0.2/")

    def test_a_site_needs_a_released_version_to_lead_to(self):
        (self.spec / "0.2" / build.RELEASED).rename(self.root / "marker")
        try:
            self.assertEqual(build.main(["--spec", str(self.spec), "--site", str(self.root / "none"), "--out", str(self.root / "none-out"), "--no-pdf"]), 1)
        finally:
            (self.root / "marker").rename(self.spec / "0.2" / build.RELEASED)


@unittest.skipUnless(shutil.which("pandoc"), "needs pandoc")
class EnglishOnly(unittest.TestCase):
    """The pages of a copy of spec/ in which 0.3, released, is English only: the Chinese address leads to the
    English page, and the Chinese version line lists it as English only."""

    @classmethod
    def setUpClass(cls):
        cls.dir = tempfile.TemporaryDirectory()
        root = Path(cls.dir.name)
        spec = root / "spec"
        shutil.copytree(REPO_SPEC, spec)
        shutil.rmtree(spec / "0.3" / "zh")
        (spec / "rnl-0.3.zh.md").unlink()
        (spec / "rnl-0.3-guide.zh.md").unlink()
        cls.site = root / "site"
        cls.code = build.main(["--spec", str(spec), "--site", str(cls.site), "--out", str(root / "out"), "--no-pdf"])

    @classmethod
    def tearDownClass(cls):
        cls.dir.cleanup()

    def page(self, *path: str) -> str:
        return self.site.joinpath(*path).read_text(encoding="utf-8")

    def test_it_builds_and_checks(self):
        self.assertEqual(self.code, 0)

    def test_the_english_page_and_its_comparison_are_built(self):
        self.assertTrue((self.site / "en" / "spec" / "0.3" / "index.html").is_file())
        self.assertTrue((self.site / "en" / "spec" / "0.3" / "compare-0.2.html").is_file())
        self.assertNotIn("English only", self.page("en", "spec", "0.3", "index.html"))

    def test_the_chinese_address_leads_to_the_english_page_and_has_nothing_else(self):
        self.assertEqual([p.name for p in (self.site / "zh" / "spec" / "0.3").iterdir()], ["index.html"])
        page = self.page("zh", "spec", "0.3", "index.html")
        self.assertIn('<link rel="canonical" href="../../../en/spec/0.3/">', page)

    def test_the_chinese_version_line_lists_it_as_english_only_and_has_no_comparison_with_it(self):
        page = self.page("zh", "spec", "0.2", "index.html")
        self.assertRegex(page, r'<a href="\.\./0\.3/">0\.3</a> \(仅英文\)')
        self.assertFalse((self.site / "zh" / "spec" / "0.2" / "compare-0.3.html").exists())

    def test_the_addresses_that_lead_to_the_newest_version_lead_to_it(self):
        self.assertIn('href="0.3/"', self.page("zh", "spec", "index.html"))
        self.assertIn('href="../zh/spec/0.3/"', self.page("spec", "index.html"))


if __name__ == "__main__":
    unittest.main()
