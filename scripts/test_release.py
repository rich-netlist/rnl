"""Release gates must reject mismatches before any publishing job can run."""
import json
import hashlib
from pathlib import Path
import tempfile
import unittest
import tarfile
import zipfile

from release import TARGETS, package, validate, versions


class ReleaseVersions(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        (self.root / "apps/desktop").mkdir(parents=True)
        (self.root / "Cargo.toml").write_text('[workspace.package]\nversion = "0.2.0"\n')
        (self.root / "apps/desktop/Cargo.toml").write_text('[package]\nversion = "0.1.5"\n')
        (self.root / "apps/desktop/tauri.conf.json").write_text(json.dumps({"version": "0.1.5"}))

    def test_independent_versions_and_tags(self):
        self.assertEqual(versions(self.root), {"core": "0.2.0", "viewer": "0.1.5"})
        for line, tag in (("core", "core-v0.2.0"), ("viewer", "viewer-v0.1.5")):
            self.assertEqual(validate(line, tag, root=self.root), tag.split("-v")[1])

    def test_mismatched_and_retired_tags_fail(self):
        for line, tag in (("core", "core-v0.1.5"), ("viewer", "viewer-v0.2.0"),
                          ("core", "runtime-v0.2.0"), ("core", "core-v0.2.0;echo bad")):
            with self.subTest(line=line, tag=tag), self.assertRaises(ValueError):
                validate(line, tag, root=self.root)

    def test_viewer_config_mismatch_fails(self):
        (self.root / "apps/desktop/tauri.conf.json").write_text('{"version":"0.2.0"}')
        with self.assertRaises(ValueError):
            validate("viewer", root=self.root)

    def test_notes_are_required_for_each_line(self):
        for line, directory, version in (("core", "release-notes/core", "0.2.0"),
                                         ("viewer", "apps/desktop/release-notes", "0.1.5")):
            with self.assertRaises(ValueError):
                validate(line, require_notes=True, root=self.root)
            notes = self.root / directory / f"{version}.md"
            notes.parent.mkdir(parents=True)
            notes.write_text("Changes\n")
            self.assertEqual(validate(line, require_notes=True, root=self.root), version)

    def test_archives_include_all_licences_and_checksums(self):
        exe = self.root / "test-executable"
        exe.write_bytes(b"release packaging fixture")
        exe.chmod(0o755)
        for target in TARGETS:
            out = self.root / target
            package(exe, target, out)
            windows = "windows" in target
            name = f"rnl-{target}" + (".zip" if windows else ".tar.gz")
            archive = out / name
            if windows:
                with zipfile.ZipFile(archive) as contents:
                    files = {entry: contents.read(entry) for entry in contents.namelist()}
            else:
                with tarfile.open(archive) as contents:
                    files = {entry.name: contents.extractfile(entry).read() for entry in contents.getmembers()}
                    self.assertTrue(contents.getmember("rnl").mode & 0o111)
            self.assertEqual(set(files), {"rnl.exe" if windows else "rnl", "LICENSE", "NOTICE.txt",
                                          "LICENSES/CC0-1.0.txt", "LICENSES/OFL-1.1.txt"})
            self.assertIn(b"Copyright 2014-2021 Adobe", files["NOTICE.txt"])
            self.assertEqual((out / "SHA256SUMS.txt").read_text(),
                             f"{hashlib.sha256(archive.read_bytes()).hexdigest()}  {name}\n")


if __name__ == "__main__":
    unittest.main()
