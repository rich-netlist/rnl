"""Full-byte compatibility against immutable PR #152 blobs; no solver or build."""
import hashlib
from pathlib import Path
import tempfile
import unittest
import benchmark
import prepare_inputs as inputs

HISTORY = '3899379423891d7288583364e3fa2acf5701b09d'
MANIFEST_SHA = 'ce445cca7c7ae826a19d4f7dca3276fca24f2e7767b97a07d55bb858e6c9d384'


class Compatibility(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.value = inputs.manifest()

    def test_every_generated_byte_matches_all_40_historical_originals(self):
        self.assertEqual(self.value['historical_source']['commit'], HISTORY)
        self.assertEqual(self.value['historical_source']['sha256'], MANIFEST_SHA)
        for item in self.value['synthetic']:
            with self.subTest(family=item['family'], n=item['n']):
                original = inputs.git_bytes(HISTORY, 'tests/performance/'+item['input'])
                generated = inputs.generate(item['family'], item['n'])
                self.assertEqual(generated, original)
                self.assertEqual(hashlib.sha256(original).hexdigest(), item['sha256'])

    def test_prepared_inputs_and_pinned_packages_match_every_historical_byte(self):
        with tempfile.TemporaryDirectory(prefix='rnl-frozen-inputs-') as temporary:
            root = Path(temporary)/'fixtures'
            inputs.prepare(root)
            self.assertEqual(benchmark.verify_fixtures(root), self.value)
            for item in self.value['representative'] + self.value['synthetic'] + self.value['library']:
                path = item.get('input', item.get('path'))
                with self.subTest(path=path):
                    self.assertEqual((root/path).read_bytes(),
                                     inputs.git_bytes(HISTORY, 'tests/performance/'+path))
            self.assertFalse((inputs.SOURCE/'library').exists())
            self.assertFalse((inputs.SOURCE/'inputs/synthetic').exists())


if __name__ == '__main__':
    unittest.main()
