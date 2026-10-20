"""Pure file/mock checks for the one-command work diagnostic. No resource/native probes."""
from pathlib import Path
from types import SimpleNamespace
import tempfile
import unittest
from unittest.mock import patch
import measure_work as w


def complete_profile():
    spans = []
    for category, names in (
        ('with_relations', ('orientation', 'placement')),
        ('relations_phase', ('model_prepare', 'resolver_init_seed', 'kept_replay', 'relations_take', 'consumer')),
        ('coords_work', ('orientation', 'relations_build', 'placement', 'post_geometry')),
    ):
        for name in names:
            spans.append({'category': category, 'name': name, 'complete': True,
                          'metrics': {'work_call_id': len(spans)+1}})
    return {'schema_version': 1, 'work_diagnostics_schema_version': 1, 'diagnostic_mode': 'work',
            'tableau_details': {'status': 'not_observed', 'omitted_metrics': sorted(w.OMITTED)},
            'work_counter_notes': ['inclusive spans'], 'status': 'complete',
            'levels': [{'metrics': {key: 0 for key in w.WORK_COUNTERS}, 'spans': spans}]}


class WorkChecks(unittest.TestCase):
    def test_exact_one_command_cap_and_defaults(self):
        args = SimpleNamespace(executable=Path('rnl.exe'), fixtures=Path('fixtures'), output=Path('out'))
        command = w.command_for({'input': 'm04.sp'}, args)
        self.assertEqual(command, [
            'rnl.exe', 'solve', str(Path('fixtures/m04.sp')), '-o', str(Path('out/solved')),
            '--package-dir', str(Path('fixtures/library')), '--profile', str(Path('out/profile.json')),
            '--profile-work'])
        self.assertEqual(w.CAP_SECONDS, 120)
        self.assertEqual(w.CASE, 'bench-20-compatible/04-fd-ota-cmfb.sp')

    def test_complete_work_requires_counters_and_explicit_omissions(self):
        profile = complete_profile()
        self.assertEqual(w.work_errors(profile), [])
        profile['levels'][0]['metrics']['tableau_rows'] = 0
        self.assertTrue(w.work_errors(profile))
        profile['levels'][0]['metrics'].pop('tableau_rows')
        profile['levels'][0]['metrics'].pop('work_axpy_calls')
        self.assertTrue(w.work_errors(profile))
        profile['levels'][0]['metrics']['work_axpy_calls'] = 0
        profile['levels'][0]['metrics']['work_counter_overflow'] = 1
        self.assertTrue(w.work_errors(profile))

    def test_partial_is_not_required_to_close_all_work(self):
        profile = complete_profile()
        profile.update(status='running', levels=[])
        self.assertEqual(w.work_errors(profile), [])
        profile['tableau_details']['status'] = 'complete'
        self.assertTrue(w.work_errors(profile))
        self.assertEqual(w.b.classify(
            {'execution_status': 'TIMEOUT'}, None, 'on', profile, [], None, []), 'TIMEOUT')

    def test_guard_failure_dispatches_nothing_and_preserves_summary(self):
        with tempfile.TemporaryDirectory(prefix='work-file-check-') as root:
            args = SimpleNamespace(executable=Path(root)/'rnl.exe', fixtures=Path(root), output=Path(root))
            with patch.object(w.b, 'guard', side_effect=RuntimeError('mock resource blocked')), \
                 patch.object(w.b.native, 'measured_run', side_effect=AssertionError('native dispatch')) as run:
                with self.assertRaisesRegex(RuntimeError, 'mock resource'):
                    w.measure_work({'input': 'm04.sp'}, args, {})
            self.assertEqual(run.call_count, 0)
            result = w.b.load(Path(root)/'summary.json')
            self.assertEqual(result['attempt']['status'], 'ATTEMPTED_NOT_DISPATCHED')
            self.assertFalse(result['diagnostic_complete'])

    def test_started_cleanup_failure_keeps_receipt_and_never_retries(self):
        with tempfile.TemporaryDirectory(prefix='work-file-check-') as root:
            root = Path(root)
            (root/'empty-cwd').mkdir()
            args = SimpleNamespace(executable=root/'rnl.exe', fixtures=root, output=root)
            native = {'child_pid': 321, 'execution_status': 'HARNESS_ERROR',
                      'owned_cleanup_completed': False, 'tree_empty_confirmed': False}
            with patch.object(w.b, 'guard'), \
                 patch.object(w.b.native, 'measured_run', return_value=(SimpleNamespace(returncode=1), None, native)) as run:
                with self.assertRaisesRegex(RuntimeError, 'cleanup'):
                    w.measure_work({'input': 'm04.sp'}, args, {})
            self.assertEqual(run.call_count, 1)
            self.assertEqual(run.call_args.kwargs['deadline_seconds'], 120)
            result = w.b.load(root/'summary.json')
            self.assertEqual(result['attempt']['status'], 'DISPATCHED_ERROR')
            self.assertTrue(result['attempt']['new_dispatch'])
            self.assertFalse(result['diagnostic_complete'])
            self.assertEqual(w.b.load(root/'native-execution.json')['native'], native)

    def test_timeout_measurement_is_incomplete_and_has_no_off_verdict(self):
        with tempfile.TemporaryDirectory(prefix='work-file-check-') as root:
            root = Path(root)
            (root/'empty-cwd').mkdir()
            (root/'stdout.txt').write_text('')
            args = SimpleNamespace(executable=root/'rnl.exe', fixtures=root, output=root)
            native = {'child_pid': 321, 'execution_status': 'TIMEOUT', 'command_seconds': 120.1,
                      'owned_cleanup_completed': True, 'tree_empty_confirmed': True}
            with patch.object(w.b, 'guard'), \
                 patch.object(w.b.native, 'measured_run', return_value=(SimpleNamespace(returncode=None), None, native)) as run, \
                 patch.object(w.b, 'resource_sample', return_value={'free_bytes': 10*w.b.GIB}):
                row = w.measure_work({'input': 'm04.sp', 'sha256': 'mock'}, args, {})
            self.assertEqual(run.call_count, 1)
            self.assertEqual(row['status'], 'TIMEOUT')
            self.assertIsNone(row['off_10s_performance_pass'])
            self.assertFalse(row['output_equivalence_validated'])
            result = w.b.load(root/'summary.json')
            self.assertTrue(result['collection_complete'])
            self.assertFalse(result['diagnostic_complete'])


if __name__ == '__main__':
    unittest.main()
