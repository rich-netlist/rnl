"""One public-main M04 work diagnostic on CI; no baseline, off pair, or retry."""
import argparse
import os
from pathlib import Path
import platform
import re
import benchmark as b

CASE = 'bench-20-compatible/04-fd-ota-cmfb.sp'
INPUT_SHA = 'c55f1d46d213e0d245b15834c84382c58cb8ac383ac0bcb245feb35b50618063'
MANIFEST_SHA = 'ce445cca7c7ae826a19d4f7dca3276fca24f2e7767b97a07d55bb858e6c9d384'
CAP_SECONDS = 120
MINIMUM_FREE_GIB = 8
OMITTED = {'tableau_scans', 'tableau_rows', 'variables', 'constraints',
           'max_numerator_bits', 'max_denominator_bits'}
WORK_COUNTERS = {
    'work_tableaux_created', 'work_tableaux_cloned', 'work_new_rows', 'work_ratio_calls',
    'work_check_passes', 'work_check_rows_examined', 'work_check_violations',
    'work_entering_terms_visited', 'work_axpy_calls', 'work_axpy_left_terms',
    'work_axpy_right_terms', 'work_axpy_result_terms', 'work_reserve_growth_requests',
    'work_capacity_changes', 'work_rebuild_calls', 'work_fixed_reassertions',
    'work_held_reassertions', 'work_max_tableau_nnz', 'work_max_tableau_rows',
    'work_max_tableau_vars', 'work_live_nnz', 'work_counter_overflow',
} | {'work_'+family+'_rows_'+suffix
     for family in ('ratio', 'update', 'pivot_value', 'pivot_rewrite')
     for suffix in ('visited', 'hit', 'missed')}


def command_for(item, args):
    return [str(args.executable), 'solve', str(args.fixtures/item['input']),
            '-o', str(args.output/'solved'), '--package-dir', str(args.fixtures/'library'),
            '--profile', str(args.output/'profile.json'), '--profile-work']


def work_errors(profile):
    if not isinstance(profile, dict) or profile.get('work_diagnostics_schema_version') != 1:
        return ['Missing work_diagnostics_schema_version=1']
    errors = []
    details = profile.get('tableau_details')
    if (profile.get('diagnostic_mode') != 'work' or not isinstance(details, dict)
        or details.get('status') != 'not_observed'
        or not OMITTED.issubset(set(details.get('omitted_metrics', [])))):
        errors.append('Missing explicit work mode / omitted legacy tableau metrics')
    if not isinstance(profile.get('work_counter_notes'), list) or not profile['work_counter_notes']:
        errors.append('Missing work counter interpretation notes')
    # A timeout retains whatever was observed; full counters/spans are required only for completion.
    if profile.get('status') != 'complete':
        return errors
    levels = profile.get('levels')
    if not isinstance(levels, list) or not levels:
        return errors+['Missing complete work levels']
    for index, level in enumerate(levels):
        if not isinstance(level, dict) or not isinstance(level.get('metrics'), dict):
            errors.append(f'Level {index}: missing work metrics')
            continue
        metrics = level['metrics']
        if any(type(metrics.get(key)) is not int or metrics[key] < 0 for key in WORK_COUNTERS):
            errors.append(f'Level {index}: incomplete or invalid work counters')
        if metrics.get('work_counter_overflow') != 0:
            errors.append(f'Level {index}: work counter overflow')
        if OMITTED.intersection(metrics):
            errors.append(f'Level {index}: omitted metrics must be absent, not zero')
        spans = level.get('spans')
        if not isinstance(spans, list) or not all(isinstance(span, dict) for span in spans):
            errors.append(f'Level {index}: missing work spans')
            continue
        relations = [span for span in spans if span.get('category') == 'with_relations']
        if not {'orientation', 'placement'}.issubset({span.get('name') for span in relations}):
            errors.append(f'Level {index}: missing purpose-labelled with_relations spans')
        for span in relations:
            call_id = span.get('metrics', {}).get('work_call_id')
            if type(call_id) is not int or call_id < 0:
                errors.append(f'Level {index}: invalid with_relations call id')
        phases = {span.get('name') for span in spans if span.get('category') == 'relations_phase'}
        if not {'model_prepare', 'resolver_init_seed', 'kept_replay', 'relations_take', 'consumer'}.issubset(phases):
            errors.append(f'Level {index}: incomplete relations phase coverage')
        coords = {span.get('name') for span in spans if span.get('category') == 'coords_work'}
        if not {'orientation', 'relations_build', 'placement', 'post_geometry'}.issubset(coords):
            errors.append(f'Level {index}: incomplete coarse coords coverage')
        if any(span.get('complete') is not True for span in spans):
            errors.append(f'Level {index}: complete profile has an unfinished span')
        if any(OMITTED.intersection(span.get('metrics', {})) for span in spans):
            errors.append(f'Level {index}: omitted legacy span metrics must be absent')
    return errors


def measure_work(item, args, build):
    folder = args.output
    command = command_for(item, args)
    attempt = {'status': 'ATTEMPT_ENTERED', 'new_dispatch': False}
    row = None
    error = None
    try:
        b.guard(folder/'resources.json', MINIMUM_FREE_GIB)
        env = dict(os.environ)
        env.pop('RNL_SOLVER_CONFIG', None)
        b.save(folder/'dispatch-attempt.json', {
            'command': command, 'utc': b.utc(),
            'note': 'Intent only; native receipt is required to prove dispatch.'})
        attempt.update(status='DISPATCH_UNKNOWN', new_dispatch=None)
        previous = Path.cwd()
        try:
            os.chdir(folder/'empty-cwd')
            result, _, native = b.native.measured_run(
                command, env, deadline_seconds=CAP_SECONDS,
                stdout_path=folder/'stdout.txt', stderr_path=folder/'stderr.txt')
        finally:
            os.chdir(previous)
        b.save(folder/'native-execution.json', {
            'command': command, 'actual_exit_code': result.returncode, 'native': native})
        attempt.update(status='DISPATCHED', new_dispatch=True if native.get('child_pid') is not None else None)
        if native['execution_status'] == 'HARNESS_ERROR' or not native.get('owned_cleanup_completed') or not native.get('tree_empty_confirmed'):
            raise RuntimeError('Owned execution/cleanup failed; preserve evidence, no retry')
        profile = None
        parse_error = None
        if (folder/'profile.json').exists():
            try:
                profile = b.load(folder/'profile.json')
            except (ValueError, OSError) as exception:
                parse_error = str(exception)
        stdout = (folder/'stdout.txt').read_text(encoding='utf-8-sig', errors='replace')
        raw_options, canonical, option_errors = b.stdout_options(stdout)
        outputs = b.outputs(folder/'solved')
        profile_errors = b.complete_profile_errors(profile, native.get('command_seconds'), canonical)
        diagnostics_errors = work_errors(profile)
        status = b.classify(native, result.returncode, 'on', profile, outputs, canonical, profile_errors)
        if status == 'COMPLETE' and diagnostics_errors:
            status = 'WORK_PROFILE_ERROR'
        post = b.resource_sample()
        row = {
            'schema_version': 1, 'item': item, 'mode': 'work', 'status': status,
            'command': command, 'build': build, 'native': native,
            'input_sha256': item['sha256'], 'fixture_manifest_sha256': MANIFEST_SHA,
            'output_files': outputs, 'solved_directory': str(folder/'solved'),
            'measurement_path': str(folder/'measurement.json'),
            'stdout_options_raw': raw_options, 'validated_options_canonical': canonical,
            'options_validation_errors': option_errors, 'profile_parse_error': parse_error,
            'profile_validation_errors': profile_errors, 'work_validation_errors': diagnostics_errors,
            'profile_sha256': b.sha(folder/'profile.json') if (folder/'profile.json').exists() else None,
            'profile_observation': {key: profile.get(key) for key in
                ('status', 'counts_complete', 'current_level', 'current_stage', 'active_span')}
                if isinstance(profile, dict) else None,
            'post_execution_resource_sample': post,
            'resource_pressure_observed': post['free_bytes'] < MINIMUM_FREE_GIB*b.GIB,
            'cwd': str(folder/'empty-cwd'), 'solver_config_environment_removed': True,
            'off_10s_performance_pass': None, 'output_equivalence_validated': False,
            'limit': 'One work diagnostic; partial is incomplete. No off rerun or cross-machine overhead ratio. '
                     'Actual output equivalence is checked against preserved existing outputs after download.'}
        b.save(folder/'measurement.json', row)
        attempt['status'] = status
        if status not in ('COMPLETE', 'TIMEOUT'):
            raise RuntimeError('Work diagnostic returned '+status+'; no retry')
        return row
    except Exception as exception:
        error = {'type': type(exception).__name__, 'message': str(exception)}
        if attempt['status'] == 'ATTEMPT_ENTERED':
            attempt['status'] = 'ATTEMPTED_NOT_DISPATCHED'
        elif attempt['status'] == 'DISPATCHED':
            attempt['status'] = 'DISPATCHED_ERROR'
        raise
    finally:
        b.save(folder/'summary.json', {
            'collection_complete': row is not None,
            'diagnostic_complete': row is not None and row['status'] == 'COMPLETE',
            'attempt': attempt, 'error': error, 'maximum_solver_invocations': 1,
            'external_cap_seconds': CAP_SECONDS, 'off_10s_performance_pass': None,
            'measurement_path': str(folder/'measurement.json') if row is not None else None,
            'native_receipt_path': str(folder/'native-execution.json') if (folder/'native-execution.json').exists() else None,
            'no_retry': True})


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--fixtures', type=Path, required=True)
    parser.add_argument('--executable', type=Path, required=True)
    parser.add_argument('--build-info', type=Path, required=True)
    parser.add_argument('--source-head', required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--execute', action='store_true')
    args = parser.parse_args()
    for name in ('fixtures', 'executable', 'build_info', 'output'):
        setattr(args, name, getattr(args, name).resolve())
    if not re.fullmatch('[0-9a-f]{40}', args.source_head):
        raise RuntimeError('Use the frozen public candidate commit SHA')
    manifest = b.verify_fixtures(args.fixtures)
    if b.sha(args.fixtures/'manifest.json') != MANIFEST_SHA:
        raise RuntimeError('Frozen fixture manifest identity differs')
    item = next(item for item in manifest['representative'] if item['case_id'] == CASE)
    if item['sha256'] != INPUT_SHA:
        raise RuntimeError('Only the frozen M04 input is allowed')
    build = b.load(args.build_info)
    if (build.get('source_head') != args.source_head or build.get('source_kind') != 'public_main'
        or build.get('build_profile') != 'release' or build.get('target') != 'x86_64-pc-windows-gnu'
        or build.get('cargo_build_jobs') != 4 or build.get('binary_sha256') != b.sha(args.executable)):
        raise RuntimeError('Candidate release/EXE/build identity differs')
    if (args.executable.parent/'rnl-solver.toml').exists():
        raise RuntimeError('Adjacent solver config would change defaults')
    if not args.execute:
        print(command_for(item, args))
        return
    if os.name != 'nt' or os.environ.get('GITHUB_ACTIONS') != 'true' or os.environ.get('GITHUB_REPOSITORY') != 'rich-netlist/rnl':
        raise RuntimeError('This entry point executes only on the public-main Windows CI runner')
    args.output.mkdir(parents=True, exist_ok=False)
    (args.output/'empty-cwd').mkdir()
    b.save(args.output/'plan.json', {
        'command': command_for(item, args), 'source_head': args.source_head,
        'build_info_sha256': b.sha(args.build_info), 'fixture_manifest_sha256': MANIFEST_SHA,
        'wrapper_sha256': b.sha(Path(__file__)), 'benchmark_sha256': b.sha(Path(b.__file__)),
        'owned_helper_sha256': b.sha(Path(b.native.__file__)),
        'minimum_free_gib': MINIMUM_FREE_GIB, 'external_cap_seconds': CAP_SECONDS,
        'maximum_solver_invocations': 1, 'hardware': platform.platform(),
        'processor': platform.processor(), 'logical_cpu_count': os.cpu_count(),
        'github_run_id': os.environ.get('GITHUB_RUN_ID'),
        'github_run_attempt': os.environ.get('GITHUB_RUN_ATTEMPT'),
        'diagnostic_only': True, 'off_baseline_reused_not_rerun': True})
    measure_work(item, args, build)


if __name__ == '__main__':
    main()
