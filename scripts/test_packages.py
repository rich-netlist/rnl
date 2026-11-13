import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import patch
import packages

class PackageDownloads(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.cache = self.root/'cache'
        self.data = b'package bytes\r\n'
        self.sha = hashlib.sha256(self.data).hexdigest()
        self.file = self.root/'example.rnl'
        self.file.write_bytes(self.data)
        self.catalog = self.root/'catalog.json'
        self.record = {'name':'example', 'version':'0.1', 'sha256':self.sha, 'url':'example.rnl',
                       'assets':[{'path':'example.rnl','url':'example.rnl','sha256':self.sha}]}
        self.entry = {k:self.record[k] for k in ('name','version','sha256')}
        self.write_lock(self.entry)
        self.write_catalog()
        self.root_patch = patch.object(packages, 'ROOT', self.root)
        self.cache_patch = patch.object(packages, 'CACHE', self.cache)
        self.root_patch.start(); self.cache_patch.start()
        self.addCleanup(self.root_patch.stop); self.addCleanup(self.cache_patch.stop)

    def write_catalog(self):
        self.catalog.write_text(json.dumps({'schemaVersion':1,'packages':[self.record]}))

    def write_lock(self, *entries):
        (self.root/'packages.lock.json').write_text(json.dumps({'packages':list(entries)}))

    def test_local_relative_catalog_download_and_fixture(self):
        packages.download(str(self.catalog))
        self.assertEqual(packages.require('example').joinpath('example.rnl').read_bytes(), self.data)
        out = self.root/'fixture'
        packages.fixture(out)
        data = json.loads((out/'catalog.json').read_text())
        self.assertEqual(data['packages'][0]['url'], 'packages/example/0.1/example.rnl')

    def test_several_versions_of_a_package(self):
        newer = b'newer bytes\n'
        sha = hashlib.sha256(newer).hexdigest()
        (self.root/'example-0.2.rnl').write_bytes(newer)
        second = {'name':'example', 'version':'0.2', 'sha256':sha, 'url':'example-0.2.rnl',
                  'assets':[{'path':'example.rnl','url':'example-0.2.rnl','sha256':sha}]}
        self.catalog.write_text(json.dumps({'schemaVersion':1,'packages':[self.record, second]}))
        self.write_lock(self.entry, {k:second[k] for k in ('name','version','sha256')})
        packages.download(str(self.catalog))
        # a name alone is its first entry, as before; each version is there and verified
        self.assertEqual(packages.require('example').joinpath('example.rnl').read_bytes(), self.data)
        self.assertEqual(packages.require('example', '0.2').joinpath('example.rnl').read_bytes(), newer)
        out = self.root/'fixture'
        packages.fixture(out)
        urls = [p['url'] for p in json.loads((out/'catalog.json').read_text())['packages']]
        self.assertEqual(urls, ['packages/example/0.1/example.rnl', 'packages/example/0.2/example.rnl'])

    def test_fixture_reports_a_cached_version_missing_from_the_lock(self):
        packages.download(str(self.catalog))
        self.write_lock(dict(self.entry, version='0.2'))
        with self.assertRaisesRegex(ValueError, r'^No lock for example 0\.1$'):
            packages.fixture(self.root/'fixture')

    def test_download_rejects_duplicate_locks(self):
        self.write_lock(self.entry, dict(self.entry, version='0.2'), dict(self.entry, sha256='0'*64))
        with self.assertRaisesRegex(ValueError, r'^Duplicate lock for example 0\.1$'):
            packages.download(str(self.catalog))
        self.assertFalse(self.cache.exists())

    @unittest.skipUnless(shutil.which('node'), 'Node.js is required to test packages.mjs')
    def test_node_rejects_duplicate_locks(self):
        self.write_lock(self.entry, dict(self.entry, version='0.2'), dict(self.entry, sha256='0'*64))
        scripts = self.root/'scripts'
        scripts.mkdir()
        module = shutil.copyfile(Path(__file__).with_name('packages.mjs'), scripts/'packages.mjs')
        result = subprocess.run(['node', str(module)], capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('Error: Duplicate lock for example 0.1', result.stderr)

    def test_corrupt_download_is_rejected_without_populating_cache(self):
        self.file.write_bytes(b'changed')
        with self.assertRaisesRegex(ValueError, 'SHA-256 mismatch'):
            packages.download(str(self.catalog))
        self.assertFalse((self.cache/'example/0.1/example.rnl').exists())

    def test_mismatched_lock_and_parent_asset_paths_are_rejected(self):
        self.record['sha256']='0'*64; self.write_catalog()
        with self.assertRaisesRegex(ValueError, 'differs from packages.lock'):
            packages.download(str(self.catalog))
        self.record['sha256']=self.sha
        self.record['assets'][0]['path']='../escaped.rnl'; self.write_catalog()
        with self.assertRaisesRegex(ValueError, 'invalid asset path'):
            packages.download(str(self.catalog))

if __name__ == '__main__':
    unittest.main()
