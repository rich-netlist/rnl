"""File/mock checks for the fixed ten-start queue. Never starts native processes."""
from pathlib import Path
from types import SimpleNamespace
import tempfile
import textwrap
import unittest
from unittest.mock import patch
import measure_pair as p


class PairChecks(unittest.TestCase):
    def test_actual_workflow_policy_rejects_overlap_rerun_or_missing_binding(self):
        workflow = (p.REPO/'.github/workflows/rust.yml').read_text()
        policy = workflow.split('  diagnostic-dispatch-policy:',1)[1].split('\n  solver-performance:',1)[0]
        script = textwrap.dedent(policy.split('        run: |\n',1)[1]).strip()
        env = dict(PERFORMANCE='false', WORK='false', BUILD_ONLY='false', PAIR='true',
                   WORK_SHA='', BASE_SHA='a'*40, HEAD_SHA='b'*40, PAIR_TOOL_SHA='c'*40,
                   GITHUB_SHA='c'*40, GITHUB_RUN_ATTEMPT='1', PAIR_TOKEN='M04-test-token')
        with patch.dict(p.os.environ, env, clear=True):
            exec(compile(script, 'rust.yml pair dispatch policy', 'exec'), {})
        for changed in ({'PERFORMANCE':'true'}, {'WORK':'true'}, {'BUILD_ONLY':'true'},
                        {'PAIR_TOKEN':''}, {'PAIR_TOOL_SHA':'d'*40}, {'GITHUB_RUN_ATTEMPT':'2'},
                        {'HEAD_SHA':''}):
            with self.subTest(changed=changed), patch.dict(p.os.environ, {**env,**changed}, clear=True):
                with self.assertRaises(SystemExit):
                    exec(compile(script, 'rust.yml pair dispatch policy', 'exec'), {})
        reusable = (p.REPO/'.github/workflows/solver-m04-pair.yml').read_text()
        commands = [line for line in reusable.splitlines() if '--execute' in line]
        self.assertEqual(len(commands), 1)
        self.assertIn('measure_pair.py',commands[0])

    def run_queue(self, root, statuses=None, changed_at=None, error_at=None):
        args = SimpleNamespace(output=root, authorization_token='M04-test-token')
        items = {case: {'case_id': case} for case in p.CASES}
        calls = []
        def fake(item, side, folder, args, entry, persist):
            calls.append((item['case_id'], side))
            folder.mkdir(); solved = folder/'solved'; solved.mkdir()
            (solved/'product.sp').write_bytes(b'changed' if len(calls) == changed_at else b'unchanged')
            entry.update(state='DISPATCH_RESERVED', actual_start=None)
            persist()
            if len(calls) == error_at:
                raise RuntimeError('mock lost native receipt')
            entry.update(actual_start=True)
            return {'case_id': item['case_id'], 'side': side, 'solved_directory': str(solved),
                    'status': (statuses or {}).get(len(calls), 'COMPLETE'),
                    'validated_options_canonical': 'defaults'}
        try:
            result = p.collect(args, items, fake)
        except RuntimeError:
            result = 'ERROR'
        return calls, p.b.load(root/'summary.json'), result

    def test_fixed_queue_and_ten_starts(self):
        with tempfile.TemporaryDirectory() as temp:
            calls, summary, result = self.run_queue(Path(temp))
        self.assertEqual(tuple(calls), p.QUEUE)
        self.assertEqual(len(calls), 10)
        self.assertEqual(p.CAP_SECONDS, 10)
        self.assertEqual(summary['actual_starts_confirmed'], 10)
        self.assertEqual(summary['reserved_attempts'], 10)
        self.assertEqual(result, 'QUEUE_FINISHED')
        self.assertTrue(all(summary['cross_version_output_equivalence'].values()))

    def test_first_candidate_timeout_stops_at_two_and_is_unknown(self):
        with tempfile.TemporaryDirectory() as temp:
            calls, summary, result = self.run_queue(Path(temp), {1:'TIMEOUT', 2:'TIMEOUT'})
        self.assertEqual(len(calls), 2)
        self.assertEqual(result, 'FIRST_CANDIDATE_TIMEOUT')
        self.assertEqual(summary['comparisons'], [])
        self.assertIsNone(summary['cross_version_output_equivalence'][p.CASES[0]])

    def test_main_timeout_candidate_complete_can_continue_without_partial_equality(self):
        with tempfile.TemporaryDirectory() as temp:
            calls, summary, _ = self.run_queue(Path(temp), {1:'TIMEOUT', 3:'TIMEOUT', 5:'TIMEOUT'})
        self.assertEqual(len(calls), 10)
        self.assertIsNone(summary['cross_version_output_equivalence'][p.CASES[0]])
        self.assertEqual(len([c for c in summary['comparisons'] if c['case_id'] == p.CASES[0]]), 2)

    def test_first_pair_complete_bytes_change_stops_at_two(self):
        with tempfile.TemporaryDirectory() as temp:
            calls, summary, result = self.run_queue(Path(temp), changed_at=2)
        self.assertEqual(len(calls), 2)
        self.assertEqual(result, 'ERROR')
        self.assertEqual(summary['stop_reason'], 'COMPLETE_OUTPUT_CHANGED')
        self.assertEqual(summary['comparisons'][0]['first_difference']['first_different_byte'], 0)

    def test_repeated_complete_bytes_change_stops_before_next_start(self):
        for changed_at in (3, 4, 6, 8, 10):
            with self.subTest(changed_at=changed_at), tempfile.TemporaryDirectory() as temp:
                calls, summary, _ = self.run_queue(Path(temp), changed_at=changed_at)
                self.assertEqual(len(calls), changed_at)
                self.assertEqual(summary['stop_reason'], 'COMPLETE_OUTPUT_CHANGED')
                changed_case = p.QUEUE[changed_at-1][0]
                self.assertFalse(summary['all_complete_outputs_consistent'][changed_case])
                self.assertFalse(summary['cross_version_output_equivalence'][changed_case])

    def test_candidate_repeat_change_without_main_complete_remains_cross_version_unknown(self):
        with tempfile.TemporaryDirectory() as temp:
            calls, summary, _ = self.run_queue(Path(temp), {1:'TIMEOUT',3:'TIMEOUT'}, changed_at=4)
        self.assertEqual(len(calls), 4)
        self.assertFalse(summary['all_complete_outputs_consistent'][p.CASES[0]])
        self.assertIsNone(summary['cross_version_output_equivalence'][p.CASES[0]])

    def test_error_and_unknown_start_consume_reservation_without_retry(self):
        with tempfile.TemporaryDirectory() as temp:
            calls, summary, _ = self.run_queue(Path(temp), error_at=2)
        self.assertEqual(len(calls), 2)
        self.assertEqual(summary['actual_starts_confirmed'], 1)
        self.assertEqual(summary['reserved_attempts'], 2)
        self.assertTrue(summary['start_count_unknown'])
        self.assertEqual(summary['stop_reason'], 'HARNESS_ERROR')

    def test_non_timeout_error_never_continues(self):
        with tempfile.TemporaryDirectory() as temp:
            calls, summary, _ = self.run_queue(Path(temp), {1:'OPTIONS_ERROR'})
        self.assertEqual(len(calls), 1)
        self.assertEqual(summary['stop_reason'], 'MEASUREMENT_ERROR')

    def native_mock(self, root, metrics, stdout=''):
        args = SimpleNamespace(fixtures=root, expected_options='defaults', builds={side: {
            'executable':root/'rnl.exe', 'identity': {'source_head':'a'*40, 'binary_sha256':'mock'}}
            for side in ('baseline','candidate')})
        entry = {}
        def fake(command, env, **kwargs):
            self.assertEqual(kwargs['deadline_seconds'], 10)
            self.assertEqual(env[p.STATS_ENV], str(root/'run/state-reuse.jsonl'))
            self.assertNotIn('RNL_SOLVER_CONFIG', env)
            self.assertNotIn('--profile', command)
            self.assertNotIn('--profile-work', command)
            kwargs['stdout_path'].write_text(stdout)
            kwargs['stderr_path'].write_text('partial stderr')
            return SimpleNamespace(returncode=0), None, metrics
        with patch.object(p.b, 'guard'), patch.object(p.b.native, 'measured_run', side_effect=fake) as native, \
             patch.object(p.b, 'resource_sample', return_value={'free_bytes': 10*p.b.GIB}):
            try:
                row = p.measure({'input':'m04.sp','case_id':p.CASES[0], 'sha256':'frozen'},
                                'candidate', root/'run', args, entry, lambda: None)
            except RuntimeError:
                row = None
        return row, entry, native.call_count

    def test_late_complete_is_timeout_and_retains_partial(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp)
            row, _, calls = self.native_mock(root, {'child_pid':123, 'execution_status':'COMPLETED',
                'command_seconds':10.00001, 'owned_cleanup_completed':True, 'tree_empty_confirmed':True})
            self.assertEqual(calls, 1)
            self.assertEqual(row['status'], 'TIMEOUT')
            self.assertFalse(row['off_10s_performance_pass'])
            self.assertEqual((root/'run/stderr.txt').read_text(), 'partial stderr')

    def test_cleanup_failure_keeps_native_receipt_no_retry(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp)
            row, entry, calls = self.native_mock(root, {'child_pid':123, 'execution_status':'HARNESS_ERROR',
                'owned_cleanup_completed':False, 'tree_empty_confirmed':False})
            self.assertEqual(calls, 1)
            self.assertIsNone(row)
            self.assertTrue(entry['actual_start'])
            self.assertTrue((root/'run/native-execution.json').exists())

    def test_guard_failure_starts_nothing(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            args = SimpleNamespace(fixtures=root, builds={'baseline':{'executable':root/'rnl.exe',
                'identity':{'source_head':'a'*40,'binary_sha256':'mock'}}})
            with patch.object(p.b, 'guard', side_effect=RuntimeError('resource blocked')), \
                 patch.object(p.b.native, 'measured_run', side_effect=AssertionError('native')) as run:
                with self.assertRaisesRegex(RuntimeError, 'resource blocked'):
                    p.measure({'input':'m04.sp', 'sha256':'frozen'}, 'baseline', root/'run', args, {}, lambda:None)
            self.assertEqual(run.call_count, 0)

    def test_build_binding_rejects_wrong_run_sha_or_binary(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp); exe=root/'rnl.exe'; exe.write_bytes(b'not executable')
            receipt=root/'build.json'
            build={'source_head':'a'*40, 'source_kind':'public_main','build_profile':'release',
                   'target':'x86_64-pc-windows-gnu','cargo_build_jobs':4,'binary_sha256':p.b.sha(exe),
                   'workflow_sha':'b'*40, 'github_run_id':'test-run','github_run_attempt':'1'}
            with patch.dict(p.os.environ, {'GITHUB_RUN_ID':'test-run'}):
                p.b.save(receipt,build)
                p.validate_build(exe,receipt,'a'*40,'b'*40)
                for key, value in (('source_head','c'*40),('workflow_sha','c'*40),
                                   ('github_run_id','other-run'),('binary_sha256','wrong')):
                    p.b.save(receipt,{**build,key:value})
                    with self.assertRaisesRegex(RuntimeError,'identity'):
                        p.validate_build(exe,receipt,'a'*40,'b'*40)


if __name__ == '__main__':
    unittest.main()
