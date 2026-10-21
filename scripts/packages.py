#!/usr/bin/env python3
"""Download locked public packages, verify hashes, or prepare an offline browser catalog."""
import argparse
import hashlib
import json
import os
import re
from pathlib import Path, PurePosixPath
import shutil
import sys
import urllib.parse
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
CACHE = ROOT / '.rnl-packages'
CATALOG_URL = 'https://rich-netlist.github.io/rnl-packages/catalog.json'
DOWNLOAD = 'python scripts/packages.py download'

def digest(data):
    return hashlib.sha256(data).hexdigest()

def read_url(url):
    if urllib.parse.urlparse(url).scheme in ('http', 'https', 'file'):
        with urllib.request.urlopen(url, timeout=60) as response:
            return response.read()
    return Path(url).read_bytes()

def catalog(url):
    data = read_url(url)
    result = json.loads(data)
    if result.get('schemaVersion') != 1 or not isinstance(result.get('packages'), list):
        raise ValueError('unsupported public package catalog')
    base = url if urllib.parse.urlparse(url).scheme in ('http', 'https', 'file') else Path(url).resolve().as_uri()
    seen = set()
    for p in result['packages']:
        key = p['name'], p['version']
        if (not re.fullmatch(r'[a-z0-9-]+', p['name']) or not re.fullmatch(r'[0-9]+\.[0-9]+', p['version'])
                or not re.fullmatch(r'[a-f0-9]{64}', p['sha256']) or key in seen):
            raise ValueError(f'invalid or duplicate package record: {key}')
        seen.add(key)
        p['url'] = urllib.parse.urljoin(base, p['url'])
        for asset in p['assets']:
            asset['url'] = urllib.parse.urljoin(base, asset['url'])
    return result, digest(data)

def locks():
    return json.loads((ROOT/'packages.lock.json').read_text(encoding='utf-8'))['packages']

def package_dir(name):
    entry = next(p for p in locks() if p['name'] == name)
    return CACHE / name / entry['version']

def require(name):
    p = next(p for p in locks() if p['name'] == name)
    file = package_dir(name) / f'{name}.rnl'
    if not file.is_file():
        raise FileNotFoundError(f'Public packages are missing; run `{DOWNLOAD}` from the checkout root.')
    if digest(file.read_bytes()) != p['sha256']:
        raise ValueError(f'{file}: SHA-256 mismatch; run `{DOWNLOAD}`')
    return file.parent

def available(names):
    try:
        for name in names:
            require(name)
        return True
    except FileNotFoundError as error:
        if os.environ.get('CI'):
            raise
        print(f'SKIP public package tests: {error}', file=sys.stderr)
        return False

def download(url, latest=False):
    data, catalog_hash = catalog(url)
    selected = data['packages']
    if latest:
        by_name = {}
        for p in selected:
            if p['name'] not in by_name or tuple(map(int,p['version'].split('.'))) > tuple(map(int,by_name[p['name']]['version'].split('.'))):
                by_name[p['name']] = p
        selected = list(by_name.values())
    else:
        selected = []
        for lock in locks():
            p = next((p for p in data['packages'] if (p['name'],p['version']) == (lock['name'],lock['version'])), None)
            if p is None or p['sha256'] != lock['sha256']:
                raise ValueError(f"{lock['name']} {lock['version']}: catalog differs from packages.lock.json")
            selected.append(p)
    for p in selected:
        directory = CACHE / p['name'] / p['version']
        # Verify all bytes before replacing any of this version's cached files.
        assets = []
        for asset in p['assets']:
            path = PurePosixPath(asset['path'])
            if path.is_absolute() or '..' in path.parts or '\\' in asset['path']:
                raise ValueError('invalid asset path')
            raw = read_url(asset['url'])
            if digest(raw) != asset['sha256']:
                raise ValueError(f"{p['name']}/{asset['path']}: SHA-256 mismatch")
            assets.append((path, raw))
        primary = next((raw for path,raw in assets if str(path) == p['name']+'.rnl'), None)
        if primary is None or digest(primary) != p['sha256']:
            raise ValueError(f"{p['name']}: primary file SHA-256 mismatch")
        for path, raw in assets:
            dest = directory / str(path)
            dest.parent.mkdir(parents=True, exist_ok=True)
            dest.write_bytes(raw)
    CACHE.mkdir(exist_ok=True)
    (CACHE/'catalog.json').write_text(json.dumps({'schemaVersion':1, 'packages':selected}, ensure_ascii=False, indent=2)+'\n', encoding='utf-8', newline='\n')
    (CACHE/'provenance.json').write_text(json.dumps({'url':url, 'sha256':catalog_hash})+'\n', encoding='utf-8')
    print(f'Downloaded and verified {len(selected)} package versions into {CACHE}')

def fixture(out):
    data = json.loads((CACHE/'catalog.json').read_text(encoding='utf-8'))
    for p in data['packages']:
        require(p['name'])
        prefix = f"packages/{p['name']}/{p['version']}/"
        for asset in p['assets']:
            source = CACHE/p['name']/p['version']/asset['path']
            if digest(source.read_bytes()) != asset['sha256']:
                raise ValueError(f'{source}: SHA-256 mismatch')
            dest = out/prefix/asset['path']
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, dest)
            asset['url'] = prefix+asset['path']
        p['url'] = prefix+p['name']+'.rnl'
    out.mkdir(parents=True, exist_ok=True)
    (out/'catalog.json').write_text(json.dumps(data,ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')
    print(f'Wrote offline catalog: {out / "catalog.json"}')

def main():
    ap = argparse.ArgumentParser(description=__doc__)
    sub = ap.add_subparsers(dest='command', required=True)
    dl = sub.add_parser('download')
    dl.add_argument('--catalog', default=os.environ.get('RNL_PACKAGE_CATALOG', CATALOG_URL))
    dl.add_argument('--latest', action='store_true', help='daily compatibility check only; do not use for tests')
    fx = sub.add_parser('fixture')
    fx.add_argument('--out', type=Path, default=CACHE/'fixture')
    sub.add_parser('check')
    args = ap.parse_args()
    if args.command == 'download': download(args.catalog, args.latest)
    elif args.command == 'fixture': fixture(args.out)
    else:
        for p in locks(): require(p['name'])
        print('ok: locked public package hashes')

if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, KeyError) as error:
        sys.exit(str(error))
