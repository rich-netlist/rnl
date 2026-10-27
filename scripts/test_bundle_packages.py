import hashlib
import json
from pathlib import Path
import tempfile
import unittest
import zipfile
from unittest.mock import patch
import bundle_packages as bundle
import packages


def sha(data):
    return hashlib.sha256(data).hexdigest()


class Bundling(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.source = self.root / 'source'
        self.files = {
            'eda-style/0.1/eda-style.rnl': b'@RNL(type="header") { @PACKAGE(name="eda-style", version="0.1") }\n',
            'eda-style/0.1/examples/a.sp': b'* a\n',
            'LICENSES/MIT.txt': b'MIT\n',
        }

    def folder(self, name='out'):
        """A folder as `fetch` writes it, with its lock file."""
        out = self.root / name
        for path, data in self.files.items():
            (out / path).parent.mkdir(parents=True, exist_ok=True)
            (out / path).write_bytes(data)
        lock = {'schemaVersion': 1, 'catalog': 'x', 'catalogSha256': 'y', 'fetched': '2026-10-27',
                'packages': [{'name': 'eda-style', 'version': '0.1', 'kind': 'extension', 'sha256': sha(self.files['eda-style/0.1/eda-style.rnl']),
                              'source': '', 'files': [{'path': p.split('/', 2)[2], 'sha256': sha(d)} for p, d in self.files.items() if p.startswith('eda-style')]}],
                'licenses': [{'path': 'LICENSES/MIT.txt', 'sha256': sha(self.files['LICENSES/MIT.txt'])}]}
        (out / bundle.LOCK).write_text(json.dumps(lock))
        return out

    def test_the_latest_version_of_each_package_is_taken_by_number_not_by_text(self):
        entries = [{'name': 'a', 'version': v} for v in ('0.2', '0.10', '0.9')] + [{'name': 'b', 'version': '1.0'}]
        self.assertEqual([(p['name'], p['version']) for p in bundle.latest(entries)], [('a', '0.10'), ('b', '1.0')])

    def test_a_folder_is_checked_against_its_lock_for_changed_missing_and_extra_files(self):
        out = self.folder()
        bundle.check(out)
        (out / 'LICENSES/MIT.txt').write_bytes(b'changed')
        with self.assertRaisesRegex(ValueError, 'SHA-256 differs'):
            bundle.check(out)
        (out / 'LICENSES/MIT.txt').write_bytes(self.files['LICENSES/MIT.txt'])
        (out / 'extra.txt').write_bytes(b'x')
        with self.assertRaisesRegex(ValueError, 'extra'):
            bundle.check(out)
        (out / 'extra.txt').unlink()
        (out / 'eda-style/0.1/examples/a.sp').unlink()
        with self.assertRaisesRegex(ValueError, 'missing'):
            bundle.check(out)

    def test_the_zip_has_the_executable_and_the_packages_beside_it_and_is_the_same_every_time(self):
        out = self.folder()
        exe = self.root / 'RNL-Viewer-0.5.1-windows-x64.exe'
        exe.write_bytes(b'MZ program')
        first, second = self.root / 'RNL-Viewer-0.5.1-with-packages-windows-x64.zip', self.root / 'again' / 'RNL-Viewer-0.5.1-with-packages-windows-x64.zip'
        second.parent.mkdir()
        bundle.make_zip(exe, out, first)
        bundle.make_zip(exe, out, second)
        with zipfile.ZipFile(first) as z:
            top = 'RNL-Viewer-0.5.1-with-packages-windows-x64/'
            names = z.namelist()
            self.assertEqual(names[0], top + exe.name)
            self.assertEqual(z.read(names[0]), b'MZ program')
            packages_files = {n[len(top + 'public-packages/'):]: z.read(n) for n in names[1:]}
            self.assertEqual(packages_files, {**self.files, bundle.LOCK: (out / bundle.LOCK).read_bytes()})
        self.assertEqual(first.read_bytes(), second.read_bytes())
        # a folder that is not as its lock says makes no zip
        (out / 'LICENSES/MIT.txt').write_bytes(b'changed')
        with self.assertRaisesRegex(ValueError, 'SHA-256 differs'):
            bundle.make_zip(exe, out, self.root / 'bad.zip')

    def test_the_wix_fragment_has_a_component_per_file_and_a_feature_that_leaves_with_publicpackages_0(self):
        out = self.folder()
        windows = {'bundle': {'windows': {'wix': {'fragmentPaths': ['file-associations.wxs'], 'componentRefs': ['RnlDefaultApps']}}}}
        with patch.object(bundle, 'ROOT', self.root):
            (self.root / 'apps' / 'desktop').mkdir(parents=True)
            (self.root / 'apps/desktop/tauri.windows.conf.json').write_text(json.dumps(windows))
            bundle.wix(out, self.root / 'wix')
        text = (self.root / 'wix/public-packages.wxs').read_text(encoding='utf-8')
        self.assertEqual(text.count('<Component '), 4)  # three files and the lock
        self.assertEqual(text.count('<ComponentRef '), 4)
        self.assertIn('<Feature Id="PublicPackages"', text)
        self.assertIn('PUBLICPACKAGES = "0"', text)
        self.assertIn('Name="public-packages"', text)
        config = json.loads((self.root / 'wix/public-packages.conf.json').read_text())['bundle']['windows']['wix']
        self.assertEqual(config['featureRefs'], ['PublicPackages'])
        self.assertEqual(config['componentRefs'], ['RnlDefaultApps'])
        self.assertEqual(config['fragmentPaths'][0], 'file-associations.wxs')
        self.assertTrue(config['fragmentPaths'][1].endswith('public-packages.wxs'))

    def test_fetch_takes_the_latest_versions_checks_every_file_and_writes_the_lock(self):
        data = {'a.rnl': b'package a 0.2', 'README.md': b'readme', 'b.rnl': b'package b'}
        for name, raw in data.items():
            (self.source / name).parent.mkdir(parents=True, exist_ok=True)
            (self.source / name).write_bytes(raw)
        catalog = {'schemaVersion': 1, 'packages': [
            {'name': 'a', 'version': '0.1', 'sha256': sha(b'old'), 'url': 'a.rnl', 'assets': [{'path': 'a.rnl', 'url': 'a.rnl', 'sha256': sha(b'old')}]},
            {'name': 'a', 'version': '0.2', 'sha256': sha(data['a.rnl']), 'url': 'a.rnl',
             'assets': [{'path': 'a.rnl', 'url': 'a.rnl', 'sha256': sha(data['a.rnl'])}, {'path': 'README.md', 'url': 'README.md', 'sha256': sha(data['README.md'])}]},
        ]}
        (self.source / 'catalog.json').write_text(json.dumps(catalog))
        licenses = self.root / 'licenses'
        for name in bundle.LICENSE_FILES:
            (licenses / name).parent.mkdir(parents=True, exist_ok=True)
            (licenses / name).write_bytes(b'licence ' + name.encode())
        out = self.root / 'fetched'
        index = bundle.fetch(out, str(self.source / 'catalog.json'), licenses.as_uri() + '/')
        self.assertEqual([(p['name'], p['version']) for p in index['packages']], [('a', '0.2')])
        self.assertEqual((out / 'a/0.2/a.rnl').read_bytes(), data['a.rnl'])
        self.assertTrue((out / 'a/0.2/README.md').is_file())
        self.assertFalse((out / 'a/0.1').exists())
        self.assertTrue((out / 'LICENSES/MIT.txt').is_file())
        bundle.check(out)
        # a changed file is refused before anything is written
        (self.source / 'README.md').write_bytes(b'tampered')
        with self.assertRaisesRegex(ValueError, 'SHA-256 mismatch'):
            bundle.fetch(self.root / 'again', str(self.source / 'catalog.json'), licenses.as_uri() + '/')
        self.assertFalse((self.root / 'again').exists())


if __name__ == '__main__':
    unittest.main()
