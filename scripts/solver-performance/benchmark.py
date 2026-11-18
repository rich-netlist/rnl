"""Collect a bounded optimized CLI baseline of the language 0.4, within a fixed work. No build or retries.

Every solve runs with the options of the manifest's `solve`: a fixed work (`limit="work"`) and no
time limit of the resolution, so that a run writes the same drawing on every machine and its time
measures how fast the command does that work. Only --execute starts a native command. A
successful collection is not performance acceptance. Windows owned-Job execution includes
CreateProcess setup and complete CLI writeback in the cap.
"""
import argparse
import csv
import ctypes
from ctypes import wintypes
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import re
import time
from datetime import datetime, timezone
import windows_process as native
from prepare_inputs import FAMILIES, REPRESENTATIVES, SIZES

# The exclusive stages of a level's profile (rnl solve --profile), the lines layout's among them.
STAGES = ('resolve', 'structure', 'search', 'tree', 'place', 'lines', 'coords', 'route', 'score')
RECORD = re.compile(r'@RNL\s*\([^)]*\btype\s*=\s*"generated"[^)]*\bhash\s*=\s*"([^"]+)"', re.S)
REPLAY = 'replay options: '
GIB = 1024**3


def utc():
    return datetime.now(timezone.utc).isoformat()


def load(path):
    return json.loads(Path(path).read_text(encoding='utf-8-sig'))


def save(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + '.tmp')
    temporary.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    temporary.replace(path)


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def finite(value):
    return isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(value) and value >= 0


def verify_fixtures(root):
    manifest = load(root / 'manifest.json')
    if manifest.get('schema_version') != 2 or manifest.get('language') != '0.4':
        raise RuntimeError('Expected the fixtures of schema 2, for the language 0.4')
    if len(manifest['representative']) != REPRESENTATIVES or len(manifest['synthetic']) != len(FAMILIES)*len(SIZES) or 'library' in manifest:
        raise RuntimeError(f'Expected {REPRESENTATIVES} representative and {len(FAMILIES)*len(SIZES)} synthetic files, and no packages')
    if len({p['sha256'] for p in manifest['representative']}) != REPRESENTATIVES:
        raise RuntimeError('Representative inputs are not SHA-distinct')
    for item in manifest['representative'] + manifest['synthetic']:
        path = root / item['input']
        if not path.resolve().is_relative_to(root.resolve()) or sha(path) != item['sha256']:
            raise RuntimeError('Fixture bytes differ: ' + str(path))
    for family in FAMILIES:
        if [p['n'] for p in manifest['synthetic'] if p['family'] == family] != list(SIZES):
            raise RuntimeError('Synthetic queue changed')
    solve = manifest['solve']
    if f'work={solve["work"]}' not in solve['solver_config'] or 'limit="work"' not in solve['solver_config'] or solve['solver_options'] != 'time_limit=none':
        raise RuntimeError('The solve of the manifest is not a fixed work without a time limit')
    return manifest


def solve_arguments(manifest):
    solve = manifest['solve']
    return ['--solver-options', solve['solver_options'], '--solver-config', solve['solver_config']]


def requested_run(manifest):
    """The run a profile's requested options name: the fixed work of the manifest."""
    return f'run=work:{manifest["solve"]["work"]}'


def resource_sample():
    if os.name != 'nt':
        raise RuntimeError('This measurement entry point requires Windows')
    class Memory(ctypes.Structure):
        _fields_ = [('length', wintypes.DWORD), ('load', wintypes.DWORD)] + [(n, ctypes.c_ulonglong) for n in ('total', 'free', 'page_total', 'page_free', 'virtual_total', 'virtual_free', 'extended')]
    memory = Memory()
    memory.length = ctypes.sizeof(memory)
    kernel = ctypes.WinDLL('kernel32', use_last_error=True)
    if not kernel.GlobalMemoryStatusEx(ctypes.byref(memory)):
        raise RuntimeError('Cannot read physical RAM')
    class Entry(ctypes.Structure):
        _fields_ = [('size', wintypes.DWORD), ('usage', wintypes.DWORD), ('pid', wintypes.DWORD), ('heap', ctypes.c_size_t), ('module', wintypes.DWORD), ('threads', wintypes.DWORD), ('parent', wintypes.DWORD), ('priority', wintypes.LONG), ('flags', wintypes.DWORD), ('exe', wintypes.WCHAR * 260)]
    snapshot = kernel.CreateToolhelp32Snapshot
    snapshot.argtypes, snapshot.restype = [wintypes.DWORD, wintypes.DWORD], wintypes.HANDLE
    handle = snapshot(2, 0)
    if handle == ctypes.c_void_p(-1).value:
        raise RuntimeError('Cannot enumerate active build/solve/test processes')
    busy = []
    first, following = kernel.Process32FirstW, kernel.Process32NextW
    for method in (first, following):
        method.argtypes, method.restype = [wintypes.HANDLE, ctypes.POINTER(Entry)], wintypes.BOOL
    close = kernel.CloseHandle
    close.argtypes, close.restype = [wintypes.HANDLE], wintypes.BOOL
    item = Entry(); item.size = ctypes.sizeof(item)
    try:
        exists = first(handle, ctypes.byref(item))
        while exists:
            if item.exe.lower() in ('cargo.exe', 'rustc.exe', 'rnl.exe', 'chromedriver.exe', 'msedgedriver.exe', 'geckodriver.exe'):
                busy.append({'pid':item.pid, 'exe':item.exe})
            exists = following(handle, ctypes.byref(item))
    finally:
        close(handle)
    return {'utc':utc(), 'monotonic':time.perf_counter(), 'free_bytes':memory.free, 'total_bytes':memory.total, 'busy':busy}


def guard(path, minimum_free_gib):
    samples = [resource_sample()]
    time.sleep(5.05)
    samples.append(resource_sample())
    save(path, {'minimum_free_gib':minimum_free_gib, 'minimum_sample_gap_seconds':5, 'samples':samples})
    if any(s['free_bytes'] < minimum_free_gib * GIB or s['busy'] for s in samples):
        raise RuntimeError('Resource gate blocked; preserve evidence and stop without dispatch')


def normalize_options(value):
    # Main's sig2 formatter is legacy text; S2's sig3 formatter is structured JSON.
    # Preserve every main field and its canonical order rather than translating it. The six fields
    # of 0.3 come first; a level of 0.4 adds its own after them (run, report_hints, tree_work, ...).
    if isinstance(value,str) and value.startswith('tolerance='):
        fields = value.split(';')
        keys = ('tolerance','grid','objectives','beam','budget','time_limit')
        if len(fields)<len(keys) or any(part.partition('=')[0]!=key for part,key in zip(fields,keys)) or any(c.isspace() for c in value):
            raise ValueError('invalid legacy canonical field order/completeness')
        extra = [part.partition('=') for part in fields[len(keys):]]
        if any(not re.fullmatch(r'[a-z_][a-z0-9_]*',key) or not sep or not rest for key,sep,rest in extra) or len({key for key,_,_ in extra})!=len(extra) or any(key in keys for key,_,_ in extra):
            raise ValueError('invalid legacy canonical added field')
        settings = dict(part.split('=',1) for part in fields[:len(keys)])
        for name,allow_none,strict_positive in (('tolerance',False,False),('grid',True,True),('time_limit',True,False)):
            number=settings[name]
            if allow_none and number=='none':
                continue
            parsed=float(number)
            if not math.isfinite(parsed) or parsed<0 or (strict_positive and parsed==0):
                raise ValueError('invalid legacy canonical '+name)
        if not re.fullmatch(r'[1-9][0-9]*',settings['beam']) or not re.fullmatch(r'candidates:[0-9]+,astar_expansions:[0-9]+',settings['budget']):
            raise ValueError('invalid legacy canonical beam/budget')
        if settings['objectives']!='none':
            for layer in settings['objectives'].split(','):
                terms,sep,tolerance=layer.partition('~')
                if not re.fullmatch(r'[a-z_][a-z0-9_]*(?:\*[1-9][0-9]*)?(?:\+[a-z_][a-z0-9_]*(?:\*[1-9][0-9]*)?)*',terms):
                    raise ValueError('invalid legacy canonical objectives')
                if sep and (not math.isfinite(float(tolerance)) or float(tolerance)<0):
                    raise ValueError('invalid legacy objective tolerance')
        return value
    if isinstance(value, str):
        value = json.loads(value)
    if not isinstance(value, dict) or value.get('format') != 'rnl-solver-config/1' or not isinstance(value.get('level'), list) or not value['level'] or not isinstance(value.get('rules'), dict):
        raise ValueError('invalid canonical solver options')
    return json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False)


def stdout_options(stdout):
    """The replay options `rnl solve` prints for each level it solves, in its order."""
    lines = [line.strip().removeprefix(REPLAY) for line in stdout.splitlines() if line.strip().startswith(REPLAY)]
    if not lines:
        return lines, None, ['expected the replay options of each level solved']
    try:
        return lines, [normalize_options(line) for line in lines], []
    except (ValueError, TypeError) as error:
        return lines, None, [str(error)]


def problem_part(signature):
    """A level's hash without the digest of its options: a profile hashes the options asked for,
    the record those of the replay, which name the work taken."""
    return signature.rpartition('.')[0] if isinstance(signature, str) and '.' in signature else signature


def complete_profile_errors(profile, command_seconds=None, expected_run=None):
    errors = []
    if not isinstance(profile, dict) or profile.get('schema_version') != 1:
        return ['missing or unsupported profile schema']
    if profile.get('status') != 'complete' or profile.get('counts_complete') is not True or profile.get('active_span') is not None:
        errors.append('profile not terminal complete')
    levels = profile.get('levels')
    if not isinstance(levels, list) or not levels:
        return errors + ['missing levels']
    for key in ('solver_total_seconds', 'cli_elapsed_seconds'):
        if not finite(profile.get(key)):
            errors.append('invalid ' + key)
    totals = []
    for level in levels:
        if not isinstance(level, dict):
            errors.append('malformed level object')
            continue
        total = level.get('total_seconds')
        stage_times = level.get('stage_seconds')
        stages = [stage_times.get(k) for k in STAGES] if isinstance(stage_times,dict) else [None]*len(STAGES)
        if level.get('status') != 'complete' or not finite(total) or not all(finite(v) for v in stages):
            errors.append('invalid level completion or timing')
        elif sum(stages) > total + 1e-6:
            errors.append('exclusive stages exceed level subtotal')
        if finite(total):
            totals.append(total)
        if not isinstance(level.get('problem_hash'), str) or not level['problem_hash']:
            errors.append('missing level problem hash')
        try:
            canonical = normalize_options(level.get('options_canonical'))
            if expected_run is not None and expected_run not in canonical.split(';'):
                errors.append('profile options do not ask for the fixed work of the manifest')
        except (ValueError, TypeError) as error:
            errors.append('invalid profile options: ' + str(error))
    if finite(profile.get('solver_total_seconds')) and not math.isclose(sum(totals), profile['solver_total_seconds'], rel_tol=1e-8, abs_tol=1e-6):
        errors.append('solver subtotal differs from sum(levels)')
    solver, cli = profile.get('solver_total_seconds'), profile.get('cli_elapsed_seconds')
    if finite(solver) and finite(cli) and solver > cli + 1e-6:
        errors.append('solver subtotal exceeds CLI time')
    if command_seconds is not None and (not finite(command_seconds) or (finite(cli) and cli > command_seconds + 1e-6)):
        errors.append('CLI time exceeds complete command time or native time is invalid')
    return errors


def outputs(folder):
    if not folder.exists():
        return []
    return [{'path':p.relative_to(folder).as_posix(), 'sha256':sha(p), 'bytes':p.stat().st_size,
             'record_hashes':RECORD.findall(p.read_text(encoding='utf-8-sig', errors='replace'))}
            for p in sorted(folder.rglob('*')) if p.is_file()]


def classify(native_metrics, exit_code, mode, profile, solved, validated_options=None, profile_errors=None, expected_run=None):
    raw = native_metrics['execution_status']
    if raw in ('TIMEOUT', 'LATE_RETURN'):
        return 'TIMEOUT'
    if raw != 'COMPLETED' or exit_code != 0:
        return 'ERROR'
    if not finite(native_metrics.get('command_seconds')):
        return 'NATIVE_TIME_ERROR'
    if not solved or not any(p['record_hashes'] for p in solved):
        return 'OUTPUT_ERROR'
    if validated_options is None or len(validated_options) != sum(len(p['record_hashes']) for p in solved):
        return 'OPTIONS_ERROR'
    if mode == 'on':
        errors = profile_errors if profile_errors is not None else complete_profile_errors(profile, native_metrics.get('command_seconds'), expected_run)
        if errors:
            return 'PROFILE_ERROR'
        if sorted(problem_part(p['problem_hash']) for p in profile['levels']) != sorted(problem_part(h) for p in solved for h in p['record_hashes']):
            return 'PROFILE_HASH_ERROR'
    return 'COMPLETE'


def scale_stop(off, n):
    if off['status'] == 'TIMEOUT':
        return 'OFF_10S_PERFORMANCE_FAILURE_CENSORED'
    if off['status'] != 'COMPLETE':
        return 'OFF_' + off['status'] + '_NO_LARGER_INPUT'
    return 'QUEUE_GUARD_REACHED_BELOW_10S' if n == 1024 else None


def off_cap(item, caps):
    return caps['synthetic_off_seconds'] if item.get('family') else caps['representative_seconds']


def profile_cap(item, off, caps):
    if not item.get('family'):
        return caps['representative_seconds']
    return caps['synthetic_profile_after_timeout_seconds'] if off['status'] == 'TIMEOUT' else caps['synthetic_profile_seconds']


def measure(item, mode, cap, args, build):
    label = item.get('case_id', item.get('family', '') + '/' + str(item.get('n', '')))
    folder = args.output / ('representative' if 'case_id' in item else 'synthetic') / label.replace('/', '__') / mode
    folder.mkdir(parents=True, exist_ok=False)
    guard(folder / 'resources.json', args.minimum_free_gib)
    command = [str(args.executable), 'solve', str(args.fixtures / item['input']), '-o', str(folder / 'solved')] + args.solve
    if mode == 'on':
        command += ['--profile', str(folder / 'profile.json')]
    env = dict(os.environ); env.pop('RNL_SOLVER_CONFIG', None)
    previous = Path.cwd()
    try:
        os.chdir(args.output / 'empty-cwd')
        result, _, metrics = native.measured_run(command, env, deadline_seconds=cap, stdout_path=folder/'stdout.txt', stderr_path=folder/'stderr.txt')
    finally:
        os.chdir(previous)
    # Save exact native identity and cleanup evidence even when validation must stop.
    save(folder / 'native-execution.json', {'command':command, 'actual_exit_code':result.returncode, 'native':metrics})
    if metrics['execution_status'] == 'HARNESS_ERROR' or not metrics.get('owned_cleanup_completed') or not metrics.get('tree_empty_confirmed'):
        raise RuntimeError('Owned execution/cleanup failed; no further dispatch')
    post = resource_sample()
    profile = None; parse_error = None
    if mode == 'on' and (folder / 'profile.json').exists():
        try:
            profile = load(folder / 'profile.json')
        except (ValueError, OSError) as error:
            parse_error = str(error)
    solved = outputs(folder / 'solved')
    stdout = (folder / 'stdout.txt').read_text(encoding='utf-8-sig', errors='replace')
    option_lines, canonical, option_errors = stdout_options(stdout)
    profile_errors = complete_profile_errors(profile, metrics.get('command_seconds'), args.run) if mode == 'on' else []
    status = classify(metrics, result.returncode, mode, profile, solved, canonical, profile_errors)
    if status == 'PROFILE_HASH_ERROR':
        profile_errors.append('profile problem hashes differ from generated record hashes')
    row = {'schema_version':1, 'item':item, 'mode':mode, 'status':status, 'command':command, 'input_sha256':item['sha256'],
           'build':build, 'native':metrics, 'output_files':solved, 'solved_directory':str(folder/'solved'),
           'measurement_path':str(folder/'measurement.json'), 'options_canonical_from_stdout':option_lines,
           'validated_options_canonical':canonical, 'options_validation_errors':option_errors,
           'off_10s_performance_pass':status == 'COMPLETE' and metrics.get('command_seconds', math.inf) <= args.within if mode == 'off' else None,
           'profile_parse_error':parse_error, 'profile_validation_errors':profile_errors,
           'profile_validation_scope':'Full completion requirements; expected partials are nonterminal and remain TIMEOUT, never COMPLETE.',
           'cli_elapsed_seconds':profile.get('cli_elapsed_seconds') if isinstance(profile, dict) else None,
           'solver_total_seconds':profile.get('solver_total_seconds') if status == 'COMPLETE' and mode == 'on' else None,
           'profile_observation':{k:profile.get(k) for k in ('status','last_updated_seconds','current_level','current_stage','active_span','counts_complete')} if isinstance(profile,dict) else None,
           'level_observations':[{k:level.get(k) for k in ('level_id','title','status','objects','facts','host_instances','total_seconds','stage_seconds','metrics','options_canonical','problem_hash')} for level in profile.get('levels',[]) if isinstance(level,dict)] if isinstance(profile,dict) and isinstance(profile.get('levels'),list) else [],
           'profile_sha256':sha(folder/'profile.json') if (folder/'profile.json').exists() else None,
           'post_execution_resource_sample':post, 'resource_pressure_observed':post['free_bytes'] < args.minimum_free_gib * GIB,
           'profile_note':'Partial is the last observed work boundary. It does not identify the entire unobserved tail. Nested spans are inclusive, never add to stages.'}
    save(folder / 'measurement.json', row)
    return row


def actual_bytes_equal(left, right):
    paths_left = {p.relative_to(left) for p in left.rglob('*') if p.is_file()}
    paths_right = {p.relative_to(right) for p in right.rglob('*') if p.is_file()}
    return bool(paths_left) and paths_left == paths_right and all((left/p).read_bytes() == (right/p).read_bytes() for p in paths_left)


def pair(off, on):
    complete = bool(on and off['status'] == on['status'] == 'COMPLETE')
    return {'item':off['item'], 'off_status':off['status'], 'on_status':on['status'] if on else 'NOT_RUN',
            'on_not_run_reason':'Off failed with an error; only COMPLETE or TIMEOUT dispatches on' if on is None else None,
            'off_10s_performance_pass':off['off_10s_performance_pass'],
            'off_command_seconds_observed':off['native'].get('command_seconds'), 'on_command_seconds_observed':on['native'].get('command_seconds') if on else None,
            'output_bytes_equal':actual_bytes_equal(Path(off['solved_directory']),Path(on['solved_directory'])) if complete else None,
            'output_hashes_equal':off['output_files'] == on['output_files'] if complete else None,
            'options_equal':off['validated_options_canonical'] == on['validated_options_canonical'] if complete and off.get('validated_options_canonical') and on.get('validated_options_canonical') else None,
            'off_output_files':off['output_files'], 'on_output_files':on['output_files'] if on else [],
            'off_options_canonical':off.get('validated_options_canonical'),
            'on_level_observations':on['level_observations'] if on else [],
            'off_resource_pressure_observed':off['resource_pressure_observed'],
            'on_resource_pressure_observed':on['resource_pressure_observed'] if on else None,
            'on_cannot_rescue_off_failure':True, 'observer_overhead_ratio':None,
            'limitation':'One off/on observation, no repeat distribution. TIMEOUT is censored failure; on partial cannot prove complete equality or precise observer overhead.'}


def collect_scale_point(item, args, build, rows):
    off = measure(item,'off',off_cap(item,args.caps),args,build); rows.append(off)
    if off['status'] not in ('COMPLETE','TIMEOUT'):
        return pair(off,None), scale_stop(off,item['n'])
    on = measure(item,'on',profile_cap(item,off,args.caps),args,build); rows.append(on)
    return pair(off,on), scale_stop(off,item['n'])


def collection_summary(build, rows, pairs, stops, complete, error):
    off_rows = [r for r in rows if r['mode'] == 'off']
    paired_keys = {json.dumps(p['item'],sort_keys=True) for p in pairs if p['on_status'] != 'NOT_RUN'}
    return {'generated_utc':utc(),'build':build,'pairs':pairs,'stops':stops,
        'collection_complete':complete,'collection_error':error,'measurements':len(rows),
        'observed_representative_off_runs':sum('case_id' in r['item'] for r in off_rows),'expected_representative_off_runs':REPRESENTATIVES,
        'observed_off_measurements':[{'item':r['item'],'status':r['status'],'off_10s_performance_pass':r['off_10s_performance_pass'],'measurement_path':r['measurement_path'],'resource_pressure_observed':r['resource_pressure_observed']} for r in off_rows],
        'unpaired_off_measurements':[{'item':r['item'],'status':r['status'],'measurement_path':r['measurement_path']} for r in off_rows if json.dumps(r['item'],sort_keys=True) not in paired_keys],
        'representative_off_10s_failures':sum(r['off_10s_performance_pass'] is False for r in off_rows if 'case_id' in r['item']),
        'complete_output_mismatches':sum(p['output_bytes_equal'] is False for p in pairs),
        'collection_success_does_not_mean_performance_acceptance':True}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--fixtures', type=Path, required=True)
    parser.add_argument('--executable', type=Path)
    parser.add_argument('--build-info', type=Path)
    parser.add_argument('--output', type=Path)
    parser.add_argument('--minimum-free-gib', type=float, default=8)
    parser.add_argument('--execute', action='store_true')
    args = parser.parse_args()
    args.fixtures = args.fixtures.resolve()
    manifest = verify_fixtures(args.fixtures)
    args.solve, args.run, args.caps = solve_arguments(manifest), requested_run(manifest), manifest['caps']
    args.within = args.caps['within_seconds']
    if not args.execute:
        print(f'Verified fixtures of 0.4; dry run only: {REPRESENTATIVES} off/on pairs within {manifest["solve"]["work"]} work; '
              f'{len(FAMILIES)} adaptive families; off cap {args.caps["representative_seconds"]}s and {args.caps["synthetic_off_seconds"]}s; no native execution')
        return
    if not args.executable or not args.build_info or not args.output:
        parser.error('--execute requires executable, build-info and a new output directory')
    args.executable = args.executable.resolve(strict=True); args.output = args.output.resolve()
    build = load(args.build_info)
    if build.get('build_profile') != 'release' or build.get('target') != 'x86_64-pc-windows-gnu' or build.get('binary_sha256') != sha(args.executable) or not re.fullmatch('[0-9a-f]{40}', build.get('source_head', '')):
        raise RuntimeError('Unbound optimized executable / source SHA / target')
    if (args.executable.parent / 'rnl-solver.toml').exists():
        raise RuntimeError('Adjacent solver config would change defaults')
    args.output.mkdir(parents=True, exist_ok=False); (args.output / 'empty-cwd').mkdir()
    import winreg
    with winreg.OpenKey(winreg.HKEY_LOCAL_MACHINE, r'HARDWARE\DESCRIPTION\System\CentralProcessor\0') as key:
        cpu = winreg.QueryValueEx(key, 'ProcessorNameString')[0]
    save(args.output / 'environment.json', {'utc':utc(), 'platform':platform.platform(), 'cpu':cpu, 'logical_cpus':os.cpu_count(),
         'build':build, 'fixture_manifest_sha256':sha(args.fixtures/'manifest.json'), 'initial_resources':resource_sample(),
         'scope':'One Windows runner / native CLI optimized build; not a universal Windows desktop or Web guarantee',
         'harness_sha256':sha(Path(__file__)), 'owned_helper_sha256':sha(Path(native.__file__))})
    rows = []; pairs = []; stops = {}; collection_complete = False; collection_error = None
    def persist():
        save(args.output/'summary.json',collection_summary(build,rows,pairs,stops,collection_complete,collection_error))
    try:
        for item in manifest['representative']:
            off = measure(item, 'off', off_cap(item, args.caps), args, build); rows.append(off)
            on = measure(item, 'on', profile_cap(item, off, args.caps), args, build); rows.append(on)
            pairs.append(pair(off, on)); persist()
        for family in FAMILIES:
            for item in (p for p in manifest['synthetic'] if p['family'] == family):
                observation, reason = collect_scale_point(item,args,build,rows)
                pairs.append(observation)
                if reason:
                    stops[family] = {'reason':reason, 'n':item['n'], 'off_status':observation['off_status']}
                persist()
                if reason:
                    break
        collection_complete = True
    except Exception as error:
        collection_error = {'type':type(error).__name__,'message':str(error)}
        raise
    finally:
        persist()
    print(f'Collection complete: {len(rows)} measurements. Inspect explicit off_10s_performance_pass and output equality; collection success is not performance acceptance.')


if __name__ == '__main__':
    main()
