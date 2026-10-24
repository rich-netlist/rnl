"""Fixed M04 prototype pair: at most ten starts, external 10s caps, no retries.

File/mock tests may call collect with an injected runner. Native execution is CI-only.
"""
import argparse
import os
from pathlib import Path
import platform
import re
import subprocess
import benchmark as b
import measure_work as frozen

CASES = (frozen.CASE, 't-0.3/T4-miller-conflicts.sp',
         'bench-20-compatible/15-pfd-charge-pump.sp')
INPUT_SHAS = (frozen.INPUT_SHA,
              '5bce1378f0cefe0fbebde7c62f4c2546cc8cdc8a6c8b7ee0227dd0ae49ef5afa',
              'bb3dfe4426cbc5523cc3bf6249aa5cc174cb256b207e3d1475637aea2798e371')
QUEUE = tuple((case, side) for case in (CASES[0],)*3 + CASES[1:]
              for side in ('baseline', 'candidate'))
CAP_SECONDS = 10
MAX_STARTS = 10
STATS_ENV = 'RNL_STATE_REUSE_STATS'
REPO = Path(__file__).resolve().parents[2]


def command_for(item, side, folder, args):
    return [str(args.builds[side]['executable']), 'solve', str(args.fixtures/item['input']),
            '-o', str(folder/'solved'), '--package-dir', str(args.fixtures/'library')]


def validate_build(executable, receipt, source_sha, tool_sha):
    build = b.load(receipt)
    if (build.get('source_head') != source_sha or build.get('source_kind') != 'public_main'
        or build.get('build_profile') != 'release' or build.get('target') != 'x86_64-pc-windows-gnu'
        or build.get('cargo_build_jobs') != 4 or build.get('binary_sha256') != b.sha(executable)
        or build.get('workflow_sha') != tool_sha
        or build.get('github_run_id') != os.environ.get('GITHUB_RUN_ID')
        or build.get('github_run_attempt') != '1'):
        raise RuntimeError('Frozen release/source/EXE/same-run build identity differs')
    if (executable.parent/'rnl-solver.toml').exists():
        raise RuntimeError('Adjacent solver config would change defaults')
    return {'executable': executable, 'receipt': receipt, 'identity': build}


def measure(item, side, folder, args, entry, persist):
    folder.mkdir()
    (folder/'empty-cwd').mkdir()
    command = command_for(item, side, folder, args)
    entry.update(command=command, source_sha=args.builds[side]['identity']['source_head'],
                 executable_sha256=args.builds[side]['identity']['binary_sha256'],
                 input_sha256=item['sha256'], measurement_directory=str(folder))
    # Resource preparation is outside timing, and happens before reservation/dispatch.
    b.guard(folder/'resources.json', 8)
    env = dict(os.environ)
    env.pop('RNL_SOLVER_CONFIG', None)
    env.pop(STATS_ENV, None)
    if side == 'candidate':
        env[STATS_ENV] = str(folder/'state-reuse.jsonl')
    entry.update(state='DISPATCH_RESERVED', dispatch_utc=b.utc(), actual_start=None,
                 default_options=args.expected_options, stats_environment=env.get(STATS_ENV))
    persist()  # A crash after this point consumes the reservation; never retry.
    previous = Path.cwd()
    try:
        os.chdir(folder/'empty-cwd')
        result, _, metrics = b.native.measured_run(
            command, env, deadline_seconds=CAP_SECONDS,
            stdout_path=folder/'stdout.txt', stderr_path=folder/'stderr.txt')
    finally:
        os.chdir(previous)
    entry.update(actual_start=metrics.get('child_pid') is not None, native=metrics,
                 state='NATIVE_RETURNED')
    b.save(folder/'native-execution.json', {'command': command,
           'actual_exit_code': result.returncode, 'native': metrics})
    persist()
    if (metrics['execution_status'] == 'HARNESS_ERROR' or not metrics.get('child_pid')
        or not metrics.get('owned_cleanup_completed') or not metrics.get('tree_empty_confirmed')):
        raise RuntimeError('Owned execution/cleanup failed; stop without retry')
    stdout = (folder/'stdout.txt').read_text(encoding='utf-8-sig', errors='replace')
    raw_options, canonical, errors = b.stdout_options(stdout)
    outputs = b.outputs(folder/'solved')
    status = b.classify(metrics, result.returncode, 'off', None, outputs, canonical)
    # Completion after the wall deadline can never unlock the remaining queue.
    if b.finite(metrics.get('command_seconds')) and metrics['command_seconds'] > CAP_SECONDS:
        status = 'TIMEOUT'
    if status == 'COMPLETE' and canonical != args.expected_options:
        status = 'OPTIONS_ERROR'
        errors.append('Default options differ from the frozen main defaults')
    post = b.resource_sample()
    row = {'case_id': item['case_id'], 'side': side, 'status': status,
           'native': metrics, 'build': args.builds[side]['identity'], 'command': command,
           'input_sha256': item['sha256'], 'output_files': outputs,
           'solved_directory': str(folder/'solved'), 'stdout_options_raw': raw_options,
           'validated_options_canonical': canonical, 'options_validation_errors': errors,
           'external_cap_seconds': CAP_SECONDS,
           'off_10s_performance_pass': status == 'COMPLETE',
           'post_execution_resources': post, 'resource_pressure_observed': post['free_bytes'] < 8*b.GIB,
           'stats_path': str(folder/'state-reuse.jsonl') if side == 'candidate' else None,
           'stats_sha256': b.sha(folder/'state-reuse.jsonl') if (folder/'state-reuse.jsonl').exists() else None,
           'stats_observation': 'partial-or-complete; absent counters are unknown, no extra diagnostic',
           'stats_overhead': 'Candidate counters and file writes are included; no overhead calibration run'}
    b.save(folder/'measurement.json', row)
    return row


def compare_complete(reference, row):
    left, right = Path(reference['solved_directory']), Path(row['solved_directory'])
    equal = b.actual_bytes_equal(left, right)
    difference = None
    if not equal:
        paths = sorted({p.relative_to(left) for p in left.rglob('*') if p.is_file()} |
                       {p.relative_to(right) for p in right.rglob('*') if p.is_file()})
        for path in paths:
            old, new = left/path, right/path
            before = old.read_bytes() if old.exists() else None
            after = new.read_bytes() if new.exists() else None
            if before != after:
                difference = {'path': path.as_posix(), 'before_bytes': len(before) if before is not None else None,
                              'after_bytes': len(after) if after is not None else None,
                              'first_different_byte': next((i for i, (x,y) in enumerate(zip(before or b'', after or b'')) if x != y),
                                                           min(len(before or b''), len(after or b'')))}
                break
    return {'case_id': row['case_id'], 'reference_side': reference['side'], 'side': row['side'],
            'reference_directory': str(left), 'directory': str(right), 'output_bytes_equal': equal,
            'options_equal': reference['validated_options_canonical'] == row['validated_options_canonical'],
            'first_difference': difference}


def collect(args, items, runner=measure):
    """One fixed queue; native starts only through measure, tests inject file-only runners."""
    ledger, rows, comparisons, references = [], [], [], {}
    reason, error = 'QUEUE_FINISHED', None
    def persist():
        b.save(args.output/'ledger.json', ledger)
        b.save(args.output/'summary.json', {
            'authorization_token': args.authorization_token, 'maximum_solver_starts': MAX_STARTS,
            'reserved_attempts': sum(e['state'] != 'PREPARING' for e in ledger),
            'actual_starts_confirmed': sum(e.get('actual_start') is True for e in ledger),
            'start_count_unknown': any(e.get('actual_start') is None and e['state'] != 'PREPARING' for e in ledger),
            'measurements': rows, 'comparisons': comparisons, 'stop_reason': reason, 'error': error,
            'external_cap_seconds': CAP_SECONDS, 'no_retry': True,
            'cross_version_output_equivalence': {case:
                all(c['output_bytes_equal'] and c['options_equal'] for c in comparisons if c['case_id'] == case)
                if any(c['case_id'] == case and c['reference_side'] != c['side'] for c in comparisons)
                else None for case in CASES},
            'all_complete_outputs_consistent': {case:
                all(c['output_bytes_equal'] and c['options_equal'] for c in comparisons if c['case_id'] == case)
                if any(c['case_id'] == case for c in comparisons) else None for case in CASES},
            'note': 'TIMEOUT is censored failure. Partial output never proves equality. Green collection is not performance acceptance.'})
    persist()
    try:
        for ordinal, (case, side) in enumerate(QUEUE, 1):
            if ordinal > MAX_STARTS:
                raise RuntimeError('Hard start budget exceeded')
            entry = {'ordinal': ordinal, 'case_id': case, 'side': side, 'state': 'PREPARING',
                     'attempt_utc': b.utc(), 'actual_start': False}
            ledger.append(entry)
            persist()
            row = runner(items[case], side, args.output/f'{ordinal:02d}-{side}', args, entry, persist)
            entry.update(state='FINISHED', status=row['status'])
            rows.append(row)
            if row['status'] not in ('COMPLETE', 'TIMEOUT'):
                reason = 'MEASUREMENT_ERROR'
                raise RuntimeError('Measurement returned '+row['status'])
            if row.get('resource_pressure_observed'):
                reason = 'RESOURCE_PRESSURE'
                raise RuntimeError('Resource pressure; stop without further dispatch')
            if row['status'] == 'COMPLETE':
                if case in references:
                    comparison = compare_complete(references[case], row)
                    comparisons.append(comparison)
                    if not comparison['output_bytes_equal'] or not comparison['options_equal']:
                        reason = 'COMPLETE_OUTPUT_CHANGED'
                        persist()
                        raise RuntimeError('Complete product bytes/default options changed; report for review')
                else:
                    references[case] = row
            if ordinal == 2 and row['status'] != 'COMPLETE':
                reason = 'FIRST_CANDIDATE_TIMEOUT'
                persist()
                return reason
            persist()
    except Exception as exception:
        error = {'type': type(exception).__name__, 'message': str(exception)}
        if reason == 'QUEUE_FINISHED':
            reason = 'HARNESS_ERROR'
        raise
    finally:
        persist()
    return reason


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--fixtures', type=Path, required=True)
    parser.add_argument('--artifact', type=Path, required=True)
    parser.add_argument('--baseline-sha', required=True)
    parser.add_argument('--candidate-sha', required=True)
    parser.add_argument('--tool-sha', required=True)
    parser.add_argument('--authorization-token', required=True)
    parser.add_argument('--execute', action='store_true')
    args = parser.parse_args()
    for value in (args.baseline_sha, args.candidate_sha, args.tool_sha):
        if not re.fullmatch('[0-9a-f]{40}', value):
            raise RuntimeError('Require frozen baseline, candidate and tool SHAs')
    if args.baseline_sha == args.candidate_sha or not re.fullmatch('M04-[A-Za-z0-9_-]{8,100}', args.authorization_token):
        raise RuntimeError('Require distinct source SHAs and an explicit root execution token')
    args.fixtures, args.artifact = args.fixtures.resolve(), args.artifact.resolve()
    args.output = args.artifact/'pair'
    manifest = b.verify_fixtures(args.fixtures)
    if b.sha(args.fixtures/'manifest.json') != frozen.MANIFEST_SHA:
        raise RuntimeError('Fixture manifest identity differs')
    items = {item['case_id']: item for item in manifest['representative'] if item['case_id'] in CASES}
    if tuple(items[case]['sha256'] for case in CASES) != INPUT_SHAS:
        raise RuntimeError('Only the frozen M04/T4/PFD inputs are allowed')
    args.expected_options = b.normalize_options(b.load(REPO/'tests/performance/canonical-main.json')['profile_options_canonical'])
    if not args.execute:
        print({'queue': QUEUE, 'maximum_solver_starts': MAX_STARTS, 'external_cap_seconds': CAP_SECONDS,
               'first_candidate_timeout_starts': 2, 'native_execution': False})
        return
    if (os.name != 'nt' or os.environ.get('GITHUB_ACTIONS') != 'true'
        or os.environ.get('GITHUB_REPOSITORY') != 'rich-netlist/rnl'
        or os.environ.get('GITHUB_RUN_ATTEMPT') != '1'
        or os.environ.get('GITHUB_EVENT_NAME') != 'workflow_dispatch'
        or os.environ.get('GITHUB_SHA') != args.tool_sha):
        raise RuntimeError('Only one first-attempt public Windows CI dispatch may execute this queue')
    if (subprocess.check_output(['git','rev-parse','HEAD'], cwd=REPO, text=True).strip() != args.tool_sha
        or subprocess.check_output(['git','status','--porcelain'], cwd=REPO, text=True).strip()):
        raise RuntimeError('Tool checkout differs from the root-bound tool SHA or is dirty')
    args.builds = {side: validate_build(args.artifact/side/'bin/rnl.exe', args.artifact/side/'build-info.json', sha, args.tool_sha)
                   for side, sha in (('baseline', args.baseline_sha), ('candidate', args.candidate_sha))}
    args.output.mkdir(parents=True, exist_ok=False)
    b.save(args.output/'plan.json', {'queue': QUEUE, 'authorization_token': args.authorization_token,
        'baseline_sha': args.baseline_sha, 'candidate_sha': args.candidate_sha, 'tool_sha': args.tool_sha,
        'fixture_manifest_sha256': frozen.MANIFEST_SHA, 'input_shas': dict(zip(CASES, INPUT_SHAS)),
        'expected_options': args.expected_options, 'options_identity_sha256': b.sha(REPO/'tests/performance/canonical-main.json'),
        'wrapper_sha256': b.sha(Path(__file__)), 'owned_helper_sha256': b.sha(Path(b.native.__file__)),
        'benchmark_sha256': b.sha(Path(b.__file__)), 'external_cap_seconds': CAP_SECONDS,
        'maximum_solver_starts': MAX_STARTS, 'github_run_id': os.environ.get('GITHUB_RUN_ID'),
        'github_run_attempt': os.environ.get('GITHUB_RUN_ATTEMPT'), 'github_job': os.environ.get('GITHUB_JOB'),
        'runner_name': os.environ.get('RUNNER_NAME'), 'hardware': platform.platform(),
        'timing_scope': 'CreateProcess through owned-tree exit and CLI writeback; setup before CreateProcess and compilation excluded',
        'stats_environment': STATS_ENV, 'stats_overhead': 'Included in candidate timing; no extra profiling execution'})
    collect(args, items)


if __name__ == '__main__':
    main()
