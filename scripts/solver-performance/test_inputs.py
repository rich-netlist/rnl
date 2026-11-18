"""The inputs of schema 2 against the manifest and the repository; no solver or build."""
import hashlib
from pathlib import Path
import subprocess
import tempfile
import unittest
import benchmark
import prepare_inputs as inputs


class Inputs(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.value = inputs.manifest()

    def test_every_generated_input_is_the_manifest_byte_for_byte(self):
        for item in self.value['synthetic']:
            with self.subTest(family=item['family'], n=item['n']):
                data = inputs.generate(item['family'], item['n'])
                self.assertEqual(hashlib.sha256(data).hexdigest(), item['sha256'])
                self.assertEqual(len(data), item['bytes'])
                self.assertNotIn(b'\r', data)
                self.assertTrue(data.endswith(b'.end\n'))
                self.assertIn(b'@VERSION("0.4")', data)

    def test_representatives_are_the_reviewed_cases_as_git_holds_them(self):
        sources = sorted(item['source'] for item in self.value['representative'])
        listed = subprocess.check_output(['git', 'ls-files', 'tests/integration/solver-0.4'], cwd=inputs.REPO, text=True).split()
        self.assertEqual(sources, sorted(p for p in listed if p.count('/') == 3 and p.endswith(('.sp', '.scs'))))
        for item in self.value['representative']:
            with self.subTest(case=item['case_id']):
                blob = subprocess.check_output(['git', 'show', 'HEAD:' + item['source']], cwd=inputs.REPO)
                self.assertEqual(inputs.representative_bytes(item), blob)
                self.assertEqual(hashlib.sha256(blob).hexdigest(), item['sha256'])

    def test_prepared_inputs_are_verified_and_nothing_is_committed(self):
        with tempfile.TemporaryDirectory(prefix='rnl-inputs-') as temporary:
            root = Path(temporary)/'fixtures'
            inputs.prepare(root)
            self.assertEqual(benchmark.verify_fixtures(root), self.value)
            files = sorted(p.relative_to(root).as_posix() for p in root.rglob('*') if p.is_file())
            expected = sorted(['manifest.json'] + [i['input'] for i in self.value['representative'] + self.value['synthetic']])
            self.assertEqual(files, expected)
        self.assertFalse((inputs.SOURCE/'inputs').exists())
        self.assertFalse((inputs.SOURCE/'library').exists())

    def test_the_previous_manifest_is_named_by_its_commit(self):
        previous = self.value['previous']
        self.assertEqual(previous['schema_version'], 1)
        try:
            old = subprocess.check_output(['git', 'show', f'{previous["commit"]}:{previous["path"]}'], cwd=inputs.REPO, stderr=subprocess.DEVNULL)
        except subprocess.CalledProcessError:
            self.skipTest('the previous manifest commit is not fetched')
        self.assertEqual(hashlib.sha256(old).hexdigest(), previous['sha256'])


if __name__ == '__main__':
    unittest.main()
