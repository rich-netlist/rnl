"""Lightweight policy/schema tests; real owned-process fixture is explicit and CI-only."""
import argparse
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch
from types import SimpleNamespace
import subprocess
import benchmark as b
import compare as comparison
import windows_process as native
import prepare_inputs


FIXTURE = b.load(Path(__file__).resolve().parents[2]/'tests/performance/canonical-main.json')
RUN = 'run=work:100000000'


def valid_profile():
    return {'schema_version':1,'status':'complete','counts_complete':True,'active_span':None,'solver_total_seconds':1.,'cli_elapsed_seconds':1.1,
            'levels':[{'status':'complete','total_seconds':1.,'problem_hash':'sig2:test.asked','options_canonical':FIXTURE['profile_options_canonical'][0],'stage_seconds':dict(zip(b.STAGES,[.1,.1,.1,.1,.1,.1,.2,.1,.1]))}]}


def replay_stdout(*lines):
    return ''.join(f'level {i}\n  replay options: {line}\n' for i,line in enumerate(lines))


def off_row(status='COMPLETE'):
    return {'item':{'case_id':'fixture'},'mode':'off','status':status,'off_10s_performance_pass':status=='COMPLETE',
            'native':{'command_seconds':1},'output_files':[],'options_canonical_from_stdout':[],
            'validated_options_canonical':None,'level_observations':[],'resource_pressure_observed':False,'measurement_path':'fixture.json'}


class Policy(unittest.TestCase):
    def test_terminal_profile_requires_explicit_subtotal(self):
        self.assertEqual(b.complete_profile_errors(valid_profile()),[])
        p=valid_profile();p.pop('solver_total_seconds');p['elapsed_seconds']=1
        self.assertTrue(b.complete_profile_errors(p))
        p=valid_profile();p['levels'][0]['status']='running'
        self.assertTrue(b.complete_profile_errors(p))
        p=valid_profile();p['levels'][0]['stage_seconds']['coords']=2
        self.assertTrue(b.complete_profile_errors(p))
        p=valid_profile();del p['levels'][0]['stage_seconds']['lines']
        self.assertTrue(b.complete_profile_errors(p))
        self.assertEqual(b.complete_profile_errors(valid_profile(),None,RUN),[])
        self.assertIn('profile options do not ask for the fixed work of the manifest',b.complete_profile_errors(valid_profile(),None,'run=work:5'))
        for malformed in (None, [], 'bad'):
            p=valid_profile();p['levels']=[malformed]
            self.assertTrue(b.complete_profile_errors(p))
        p=valid_profile();p['levels'][0]['stage_seconds']=None
        self.assertTrue(b.complete_profile_errors(p))
        p=valid_profile();p['cli_elapsed_seconds']=.5
        self.assertIn('solver subtotal exceeds CLI time',b.complete_profile_errors(p))
        self.assertTrue(b.complete_profile_errors(valid_profile(),.5))

    def test_late_is_censored_failure_even_with_output(self):
        solved=[{'record_hashes':['sig2:test.replayed']}]
        for status in ('TIMEOUT','LATE_RETURN'):
            self.assertEqual(b.classify({'execution_status':status},0,'on',valid_profile(),solved),'TIMEOUT')
        self.assertEqual(native._completion_status(10.00001,10),'LATE_RETURN')
        self.assertEqual(native._completion_status(10,10),'COMPLETED')

    def test_scale_and_diagnostic_caps(self):
        caps=json.loads((prepare_inputs.SOURCE/'manifest.json').read_text(encoding='utf-8'))['caps']
        self.assertEqual(b.profile_cap({'family':'chain'},{'status':'TIMEOUT'},caps),120)
        self.assertEqual(b.profile_cap({'family':'chain'},{'status':'COMPLETE'},caps),30)
        self.assertEqual(b.off_cap({'family':'chain'},caps),10)
        self.assertEqual(b.off_cap({'case_id':'reviewed-0.4/x.sp'},caps),60)
        self.assertEqual(b.profile_cap({'case_id':'reviewed-0.4/x.sp'},{'status':'TIMEOUT'},caps),60)
        self.assertEqual(b.scale_stop({'status':'TIMEOUT'},2),'OFF_10S_PERFORMANCE_FAILURE_CENSORED')
        self.assertEqual(b.scale_stop({'status':'COMPLETE'},1024),'QUEUE_GUARD_REACHED_BELOW_10S')
        self.assertIsNone(b.scale_stop({'status':'COMPLETE'},32))
        self.assertIn('ERROR',b.scale_stop({'status':'ERROR'},2))

    def test_on_timeout_cannot_claim_equality(self):
        off=off_row()
        on={**off,'status':'TIMEOUT'}
        self.assertIsNone(b.pair(off,on)['output_bytes_equal'])
        self.assertIsNone(b.pair(off,on)['observer_overhead_ratio'])

    def test_missing_options_are_unknown_and_not_complete(self):
        for text in ('', '  replay options: {}', '  replay options: null', 'solver options: '+FIXTURE['stdout_replay_lines'][0]):
            self.assertIsNone(b.stdout_options(text)[1])
        line=FIXTURE['stdout_replay_lines'][0]
        self.assertEqual(b.stdout_options(replay_stdout(line))[1],[line])
        self.assertEqual(len(b.stdout_options(replay_stdout(line,line))[1]),2)
        self.assertIsNone(b.stdout_options(replay_stdout(line,line+';tree_work=1'))[1])
        native_ok={'execution_status':'COMPLETED','command_seconds':1.2}
        self.assertEqual(b.classify(native_ok,0,'off',None,[{'record_hashes':['sig2:test.replayed']}]),'OPTIONS_ERROR')
        # one replay line for each level written
        self.assertEqual(b.classify(native_ok,0,'off',None,[{'record_hashes':['sig2:a.b','sig2:c.d']}],[line]),'OPTIONS_ERROR')
        self.assertEqual(b.classify(native_ok,0,'off',None,[{'record_hashes':['sig2:a.b']}],[line]),'COMPLETE')

    def test_scale_error_dispatches_no_on(self):
        off=off_row('OUTPUT_ERROR');off['item']={'family':'chain','n':2}
        rows=[]
        with patch.object(b,'measure',return_value=off) as measure:
            pair,reason=b.collect_scale_point(off['item'],SimpleNamespace(caps={'synthetic_off_seconds':10}),{},rows)
        self.assertEqual(measure.call_count,1)
        self.assertEqual(pair['on_status'],'NOT_RUN')
        self.assertIn('OUTPUT_ERROR',reason)

    def test_unpaired_timeout_is_preserved_after_on_failure(self):
        off=off_row('TIMEOUT')
        summary=b.collection_summary({},[off],[],{},False,{'message':'on cleanup failed'})
        self.assertFalse(summary['collection_complete'])
        self.assertEqual(summary['representative_off_10s_failures'],1)
        self.assertEqual(len(summary['unpaired_off_measurements']),1)

    def test_not_run_error_and_pressure_do_not_become_speedups(self):
        passed={'off_status':'COMPLETE','off_10s_performance_pass':True,'off_options_canonical':'observed'}
        self.assertEqual(comparison.transition(None,passed),'candidate_only_measured')
        self.assertEqual(comparison.transition({'off_status':'TIMEOUT','off_10s_performance_pass':False},passed),'newly_completed_off_points')
        self.assertEqual(comparison.transition({'off_status':'ERROR','off_10s_performance_pass':False},passed),'other_observed_state_transitions')
        self.assertFalse(comparison.clean_comparison({**passed,'off_resource_pressure_observed':True},passed))
        self.assertFalse(comparison.clean_comparison({**passed,'off_options_canonical':None},passed))

    def test_bytes_are_checked_even_when_hash_dictionaries_match(self):
        with tempfile.TemporaryDirectory() as temp:
            left,right=Path(temp)/'off',Path(temp)/'on';left.mkdir();right.mkdir()
            (left/'out').write_bytes(b'a');(right/'out').write_bytes(b'b')
            off={**off_row(),'solved_directory':str(left)};on={**off,'solved_directory':str(right)}
            self.assertFalse(b.pair(off,on)['output_bytes_equal'])
            self.assertTrue(b.pair(off,on)['output_hashes_equal'])
            self.assertIsNone(b.pair(off,on)['options_equal'])

    def test_bad_profile_receipt_keeps_errors_with_native_mock(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);(root/'runs/empty-cwd').mkdir(parents=True)
            args=SimpleNamespace(output=root/'runs',fixtures=root,executable=root/'fake.exe',minimum_free_gib=8)
            item={'case_id':'fixture','input':'unused','sha256':'fixture'}
            args.solve,args.run,args.within=['--solver-config','limit="work",work=100000000'],RUN,10
            def fake(command,env,**kwargs):
                stdout=kwargs['stdout_path'];folder=stdout.parent
                stdout.write_text(replay_stdout(FIXTURE['stdout_replay_lines'][0]),encoding='utf-8')
                kwargs['stderr_path'].write_text('',encoding='utf-8')
                (folder/'solved').mkdir();(folder/'solved/out').write_text('@RNL(type="generated", hash="sig2:test.replayed")',encoding='utf-8')
                p=valid_profile();p['levels']=[None];b.save(folder/'profile.json',p)
                return subprocess.CompletedProcess(command,0,b'',b''),1.2,{'execution_status':'COMPLETED','command_seconds':1.2,'owned_cleanup_completed':True,'tree_empty_confirmed':True}
            with patch.object(b,'guard'),patch.object(b,'resource_sample',return_value={'free_bytes':9*b.GIB}),patch.object(b.native,'measured_run',side_effect=fake):
                row=b.measure(item,'on',10,args,{})
            self.assertEqual(row['status'],'PROFILE_ERROR')
            self.assertIn('malformed level object',row['profile_validation_errors'])
            self.assertEqual(row['command'][5:5+len(args.solve)],args.solve)
            self.assertNotIn('--package-dir',row['command'])
            self.assertTrue((Path(row['measurement_path']).parent/'native-execution.json').exists())

    def test_prepared_inputs(self):
        with tempfile.TemporaryDirectory(prefix='rnl-policy-inputs-') as temporary:
            root=Path(temporary)/'fixtures'
            prepare_inputs.prepare(root)
            manifest=b.verify_fixtures(root)
            self.assertEqual(b.solve_arguments(manifest),['--solver-options','time_limit=none','--solver-config','limit="work",work=100000000,safety_seconds=600'])
            self.assertEqual(b.requested_run(manifest),RUN)

    def test_main_replay_lines_and_profile_of_0_4_agree_on_the_problem(self):
        lines,canonical,errors=b.stdout_options(replay_stdout(*FIXTURE['stdout_replay_lines']))
        self.assertEqual(errors,[])
        self.assertEqual(canonical,FIXTURE['stdout_replay_lines'])
        for options in FIXTURE['profile_options_canonical']:
            self.assertEqual(b.normalize_options(options),options)
            self.assertIn(RUN,options.split(';'))
        # the profile hashes the options asked for, the record those of the replay
        self.assertNotEqual(sorted(FIXTURE['profile_problem_hashes']),sorted(FIXTURE['record_hashes']))
        self.assertEqual(sorted(map(b.problem_part,FIXTURE['profile_problem_hashes'])),sorted(map(b.problem_part,FIXTURE['record_hashes'])))
        canonical=canonical[0]
        for invalid in ('tolerance=0.000001;grid=1',canonical.replace('tolerance=0.000001;grid=1','grid=1;tolerance=0.000001'),
                        canonical+';run=work:1',canonical+';Bad=1',canonical+';empty='):
            self.assertIsNone(b.stdout_options(replay_stdout(invalid))[1],invalid)


def owned_fixture(folder):
    if os.name!='nt' or os.environ.get('GITHUB_ACTIONS')!='true':
        raise RuntimeError('Real process fixture is explicit Windows CI-only; never local')
    folder.mkdir(parents=True,exist_ok=False)
    env=dict(os.environ)
    rows=[]
    commands=[('natural',[sys.executable,'-c','print("owned fixture")'],3),
              ('timeout',[sys.executable,'-c','import json,pathlib,subprocess,sys,time;pathlib.Path(sys.argv[1]).write_text(json.dumps({"status":"running","stage":"fixture"}));subprocess.Popen([sys.executable,"-c","import time;time.sleep(30)"]);time.sleep(30)',str(folder/'partial.json')],1)]
    for label,command,deadline in commands:
        result,_,metrics=native.measured_run(command,env,deadline_seconds=deadline,stdout_path=folder/(label+'.stdout'),stderr_path=folder/(label+'.stderr'))
        b.save(folder/(label+'.json'),{'command':command,'actual_exit_code':result.returncode,'native':metrics})
        expected='COMPLETED' if label=='natural' else 'TIMEOUT'
        if metrics['execution_status']!=expected or not metrics.get('owned_cleanup_completed') or not metrics.get('tree_empty_confirmed'):
            raise RuntimeError('Owned helper fixture failed; no solver dispatch')
        rows.append(metrics)
    if b.load(folder/'partial.json').get('stage')!='fixture':
        raise RuntimeError('Expected valid partial did not survive timeout')
    print('CI owned helper natural / subtree timeout / valid partial fixture passed; zero solver calls')


if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--owned-helper-fixture',type=Path)
    args,rest=parser.parse_known_args()
    if args.owned_helper_fixture:
        owned_fixture(args.owned_helper_fixture.resolve())
    else:
        unittest.main(argv=[sys.argv[0],*rest])
