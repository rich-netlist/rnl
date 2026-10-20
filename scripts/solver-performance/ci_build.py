"""Build public main revisions on a Windows GitHub runner, never on a local workstation."""
import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import benchmark


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path, required=True)
    parser.add_argument('--source-head', required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--target-dir', type=Path, required=True)
    parser.add_argument('--toolchain', default='1.97.1-x86_64-pc-windows-gnu')
    args = parser.parse_args()
    if os.environ.get('GITHUB_ACTIONS') != 'true' or os.environ.get('GITHUB_REPOSITORY') != 'rich-netlist/rnl' or os.name != 'nt':
        raise RuntimeError('Public-main CI build entry point is forbidden on local hosts / other repositories')
    if not re.fullmatch('[0-9a-f]{40}', args.source_head):
        raise RuntimeError('Use a fixed 40-character public repository commit')
    args.source = args.source.resolve(); args.output = args.output.resolve(); args.target_dir = args.target_dir.resolve()
    actual = subprocess.check_output(['git','rev-parse','HEAD'], cwd=args.source, text=True).strip()
    if actual != args.source_head or subprocess.check_output(['git','status','--porcelain'], cwd=args.source, text=True).strip():
        raise RuntimeError('Source HEAD differs or checkout is dirty')
    args.output.mkdir(parents=True, exist_ok=False)
    benchmark.guard(args.output/'build-resources.json', 8)
    env = dict(os.environ)
    for key in env:
        if key in ('RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER') or key.startswith('CARGO_PROFILE_RELEASE_'):
            if env[key]:
                raise RuntimeError('Custom compiler/profile environment: ' + key)
    env['CARGO_BUILD_JOBS'] = '4'
    target = 'x86_64-pc-windows-gnu'
    rustc = subprocess.check_output(['rustc','+'+args.toolchain,'-vV'], text=True).replace('\r\n','\n').strip()
    cargo = subprocess.check_output(['cargo','+'+args.toolchain,'--version'], text=True).strip()
    command = ['cargo','+'+args.toolchain,'build','--locked','--release','-p','rnl-cli','--target',target,'--target-dir',str(args.target_dir)]
    with (args.output/'build.stdout.txt').open('wb') as stdout, (args.output/'build.stderr.txt').open('wb') as stderr:
        result = subprocess.run(command, cwd=args.source, env=env, stdout=stdout, stderr=stderr, check=False)
    benchmark.save(args.output/'build-command.json', {'command':command,'exit_code':result.returncode,'source_head':actual})
    if result.returncode:
        raise RuntimeError('Release build failed; preserve logs')
    binary = args.output/'bin/rnl.exe'; binary.parent.mkdir()
    shutil.copyfile(args.target_dir/target/'release/rnl.exe',binary)
    benchmark.save(args.output/'build-info.json', {'build_profile':'release','target':target,'source_kind':'public_main',
        'source_head':actual,'binary_sha256':benchmark.sha(binary),'toolchain':args.toolchain,'rustc_verbose':rustc,
        'cargo_version':cargo,'cargo_build_jobs':4,'command':command,'cargo_lock_sha256':benchmark.sha(args.source/'Cargo.lock'),
        'workflow_sha':os.environ.get('GITHUB_SHA'),'github_run_id':os.environ.get('GITHUB_RUN_ID'),
        'github_run_attempt':os.environ.get('GITHUB_RUN_ATTEMPT'),'runner_os':os.environ.get('RUNNER_OS'),
        'runner_arch':os.environ.get('RUNNER_ARCH'),'captured_utc':benchmark.utc()})


if __name__ == '__main__':
    main()
