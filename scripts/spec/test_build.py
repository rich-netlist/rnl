"""Which versions of the standard the pages show: python -m unittest discover -s scripts/spec -p 'test_build.py'

The build itself needs pandoc (and Typst on PATH for its fonts); the tests that run it are skipped without pandoc."""
import importlib.util
import re
import shutil
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
spec_module = importlib.util.spec_from_file_location("spec_build", HERE / "build.py")
build = importlib.util.module_from_spec(spec_module)
spec_module.loader.exec_module(build)

REPO_SPEC = HERE.parents[1] / "spec"


def folder(root: Path, v: str, released: bool) -> None:
    (root / v / "en").mkdir(parents=True)
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

    def test_every_version_in_the_repository_that_is_published_has_the_file(self):
        for v in build.versions(REPO_SPEC):
            self.assertTrue(build.released(REPO_SPEC, v), f"spec/{v}/{build.RELEASED}: the file that marks it as released")


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


if __name__ == "__main__":
    unittest.main()
