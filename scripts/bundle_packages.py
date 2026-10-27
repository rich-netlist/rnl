#!/usr/bin/env python3
"""The public packages that the desktop viewer bundles, for the release workflow.

  fetch   the latest version of each package of the catalog of rnl-packages, every file checked against
          the catalog's SHA-256, with the licence files of the repository, into a directory and a lock
          file beside them, public-packages.json: what the viewer shows as the versions it bundles
  check   a directory against its lock file
  zip     the zip of the single-file Windows viewer, or of the AppImage, with the packages: the same
          file and the directory as `public-packages/` beside it, which is where the viewer looks
  deb     the Debian package of the public packages, which puts them in /usr/share/rnl-viewer,
          where the viewer of its own Debian package looks for them
  wix     the WiX fragment of the MSI that has the directory's files as a feature of its own, "Public
          packages", which the user may leave out (PUBLICPACKAGES=0 on the command line leaves it out too),
          and the Tauri configuration that names the fragment, to give to `tauri build --config`

The directory is `<name>/<version>/<files of the package>`, as the viewer searches it (each `<name>`
directory is one directory of its PATH, whose `<version>/<name>.rnl` the package lookup finds, 16.2).
Nothing here is for the viewer to run: it never downloads a package, and copies none anywhere. They
change only with a new release of the viewer.
"""
import argparse
import datetime
import json
import os
import sys
import xml.sax.saxutils
import zipfile
import urllib.error
import urllib.request
from pathlib import Path, PurePosixPath

sys.path.insert(0, str(Path(__file__).resolve().parent))
import packages  # noqa: E402  (the catalog's reading and checks, and its URL)

ROOT = Path(__file__).resolve().parents[1]
LOCK = 'public-packages.json'
LICENSE_SOURCE = 'https://raw.githubusercontent.com/rich-netlist/rnl-packages/main/'
# the licence files of the repository of the packages: the text of each licence its files carry
LICENSE_FILES = ['LICENSE', 'LICENSES/MIT.txt', 'LICENSES/CC0-1.0.txt']


def version_key(version):
    return tuple(int(part) for part in version.split('.'))


def latest(entries):
    """The latest version of each package of a catalog, by name."""
    by_name = {}
    for p in entries:
        if p['name'] not in by_name or version_key(p['version']) > version_key(by_name[p['name']]['version']):
            by_name[p['name']] = p
    return [by_name[name] for name in sorted(by_name)]


def safe(path):
    posix = PurePosixPath(path)
    if posix.is_absolute() or '..' in posix.parts or '\\' in path or not path:
        raise ValueError(f'invalid path: {path}')
    return posix


def fetch(out, url, licenses=LICENSE_SOURCE):
    data, catalog_hash = packages.catalog(url)
    chosen = latest(data['packages'])
    if not chosen:
        raise ValueError('the catalog lists no package')
    written, lock = {}, []
    for p in chosen:
        files = []
        for asset in p['assets']:
            path = safe(asset['path'])
            raw = packages.read_url(asset['url'])
            if packages.digest(raw) != asset['sha256']:
                raise ValueError(f"{p['name']}/{asset['path']}: SHA-256 mismatch")
            written[PurePosixPath(p['name']) / p['version'] / path] = raw
            files.append({'path': str(path), 'sha256': asset['sha256']})
        primary = next((f for f in files if f['path'] == p['name'] + '.rnl'), None)
        if primary is None or primary['sha256'] != p['sha256']:
            raise ValueError(f"{p['name']}: primary file SHA-256 mismatch")
        lock.append({'name': p['name'], 'version': p['version'], 'kind': p.get('kind', ''), 'sha256': p['sha256'],
                     'source': p.get('source', ''), 'files': files})
    texts = []
    for name in LICENSE_FILES:
        try:
            raw = packages.read_url(licenses + name)
        except (urllib.error.URLError, OSError) as error:
            raise ValueError(f'{name}: the licence of the packages cannot be read: {error}')
        written[PurePosixPath('LICENSES') / PurePosixPath(name).name] = raw
        texts.append({'path': 'LICENSES/' + PurePosixPath(name).name, 'sha256': packages.digest(raw)})
    for path, raw in written.items():
        dest = out / path
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_bytes(raw)
    index = {'schemaVersion': 1, 'catalog': url, 'catalogSha256': catalog_hash,
             'fetched': datetime.date.today().isoformat(), 'packages': lock, 'licenses': texts}
    (out / LOCK).write_text(json.dumps(index, ensure_ascii=False, indent=2) + '\n', encoding='utf-8', newline='\n')
    print(f'Fetched {len(lock)} packages into {out}: ' + ', '.join(f"{p['name']} {p['version']}" for p in lock))
    return index


def check(directory):
    """Every file of the lock is there and as it says, and nothing else is."""
    lock = json.loads((directory / LOCK).read_text(encoding='utf-8'))
    expected = {}
    for p in lock['packages']:
        for f in p['files']:
            expected[str(PurePosixPath(p['name']) / p['version'] / safe(f['path']))] = f['sha256']
    for f in lock['licenses']:
        expected[str(safe(f['path']))] = f['sha256']
    found = {str(path.relative_to(directory).as_posix()) for path in directory.rglob('*') if path.is_file()} - {LOCK}
    if found != set(expected):
        raise ValueError(f'the files differ from {LOCK}: missing {sorted(set(expected) - found)}, extra {sorted(found - set(expected))}')
    for name, digest in expected.items():
        if packages.digest((directory / name).read_bytes()) != digest:
            raise ValueError(f'{name}: SHA-256 differs from {LOCK}')
    print(f'ok: {len(lock["packages"])} packages as {LOCK} says')


def make_zip(exe, directory, out):
    """A zip of the executable and the directory as `public-packages/` beside it, in a folder named as the zip is.

    The files have no time and the same order and permissions, so that the same files make the same zip.
    """
    check(directory)
    top = out.stem
    entries = [(exe, f'{top}/{exe.name}')]
    entries += [(p, f'{top}/public-packages/{p.relative_to(directory).as_posix()}') for p in sorted(directory.rglob('*')) if p.is_file()]
    with zipfile.ZipFile(out, 'w', zipfile.ZIP_DEFLATED, compresslevel=9) as z:
        for source, name in entries:
            info = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = (0o100755 if source == exe else 0o100644) << 16
            z.writestr(info, source.read_bytes())
    print(f'Wrote {out}: {exe.name} and {len(entries) - 1} files of {directory} as {top}/public-packages/')


def make_deb(directory, version, out):
    """The Debian package `rnl-viewer-public-packages` of the files of `directory`; it needs dpkg-deb."""
    import shutil
    import subprocess
    import tempfile
    check(directory)
    with tempfile.TemporaryDirectory() as tmp:
        tree = Path(tmp) / 'tree'
        shutil.copytree(directory, tree / 'usr/share/rnl-viewer/public-packages')
        (tree / 'DEBIAN').mkdir()
        size = sum(p.stat().st_size for p in directory.rglob('*') if p.is_file()) // 1024 + 1
        lines = [
            'Package: rnl-viewer-public-packages',
            f'Version: {version}',
            'Section: electronics',
            'Priority: optional',
            'Architecture: all',
            f'Installed-Size: {size}',
            'Maintainer: Rich Netlist <noreply@github.com>',
            'Homepage: https://rich-netlist.github.io/rnl-packages/',
            'Description: Public packages for RNL Viewer, found without a network',
            ' The packages of the rnl-packages site, in the versions of this release of RNL Viewer:',
            ' extensions, symbol and style libraries and PDKs that a netlist uses with @USE-PACKAGE.',
            ' They are installed in /usr/share/rnl-viewer, where RNL Viewer looks for them. RNL Viewer',
            ' never downloads packages itself.',
        ]
        (tree / 'DEBIAN/control').write_text('\n'.join(lines) + '\n', encoding='utf-8', newline='\n')
        subprocess.run(['dpkg-deb', '--root-owner-group', '--build', str(tree), str(out)], check=True)
    print(f'Wrote {out}')


def wix(directory, out):
    """The fragment and the configuration that add the files of `directory` to the MSI as a feature."""
    check(directory)
    out.mkdir(parents=True, exist_ok=True)
    files = sorted(p for p in directory.rglob('*') if p.is_file())
    tree = {}
    for file in files:
        node = tree
        for part in file.relative_to(directory).parts[:-1]:
            node = node.setdefault(part, {})
        node.setdefault('', []).append(file)
    refs, counter = [], [0]
    q = xml.sax.saxutils.quoteattr

    def folder(name, node, depth):
        counter[0] += 1
        pad = '  ' * depth
        lines = [f'{pad}<Directory Id="ppd{counter[0]}" Name={q(name)}>']
        for file in node.get('', []):
            counter[0] += 1
            cid = f'ppc{counter[0]}'
            refs.append(cid)
            lines.append(f'{pad}  <Component Id="{cid}" Guid="*" Win64="yes">'
                         f'<File Id="ppf{counter[0]}" Source={q(str(file.resolve()))} KeyPath="yes" /></Component>')
        for child in sorted(k for k in node if k):
            lines += folder(child, node[child], depth + 1)
        lines.append(f'{pad}</Directory>')
        return lines

    body = folder('public-packages', tree, 3)
    feature = ['    <Feature Id="PublicPackages" Title="Public packages" Level="1" AllowAdvertise="no" Absent="allow"',
               '             Description="The public packages of the rnl-packages site, in the versions of this release, which a netlist uses with @USE-PACKAGE. They are found without a network; RNL Viewer never downloads packages.">',
               '      <Condition Level="0"><![CDATA[PUBLICPACKAGES = "0"]]></Condition>']
    feature += [f'      <ComponentRef Id="{cid}" />' for cid in refs] + ['    </Feature>']
    text = '\n'.join(['<?xml version="1.0" encoding="UTF-8"?>', '<Wix xmlns="http://schemas.microsoft.com/wix/2006/wi">',
                      '  <Fragment>', '    <DirectoryRef Id="INSTALLDIR">'] + body + ['    </DirectoryRef>', '  </Fragment>',
                      '  <Fragment>'] + feature + ['  </Fragment>', '</Wix>', ''])
    fragment = out / 'public-packages.wxs'
    fragment.write_text(text, encoding='utf-8', newline='\n')
    windows = json.loads((ROOT / 'apps' / 'desktop' / 'tauri.windows.conf.json').read_text(encoding='utf-8'))
    base = windows['bundle']['windows']['wix']
    config = {'bundle': {'windows': {'wix': {
        'fragmentPaths': [*base['fragmentPaths'], str(fragment.resolve())],
        'componentRefs': base['componentRefs'],
        'featureRefs': ['PublicPackages'],
    }}}}
    (out / 'public-packages.conf.json').write_text(json.dumps(config, indent=2) + '\n', encoding='utf-8', newline='\n')
    print(f'Wrote {fragment} ({len(refs)} components) and public-packages.conf.json')


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest='command', required=True)
    f = sub.add_parser('fetch')
    f.add_argument('--out', type=Path, required=True)
    f.add_argument('--catalog', default=os.environ.get('RNL_PACKAGE_CATALOG', packages.CATALOG_URL))
    c = sub.add_parser('check')
    c.add_argument('directory', type=Path)
    z = sub.add_parser('zip')
    z.add_argument('--exe', type=Path, required=True)
    z.add_argument('--packages', type=Path, required=True)
    z.add_argument('--out', type=Path, required=True)
    d = sub.add_parser('deb')
    d.add_argument('--dir', type=Path, required=True)
    d.add_argument('--version', required=True)
    d.add_argument('--out', type=Path, required=True)
    w = sub.add_parser('wix')
    w.add_argument('--dir', type=Path, required=True)
    w.add_argument('--out', type=Path, required=True)
    args = ap.parse_args()
    if args.command == 'wix':
        wix(args.dir, args.out)
    elif args.command == 'deb':
        make_deb(args.dir, args.version, args.out)
    elif args.command == 'fetch':
        fetch(args.out, args.catalog)
    elif args.command == 'check':
        check(args.directory)
    else:
        make_zip(args.exe, args.packages, args.out)


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, KeyError) as error:
        sys.exit(str(error))
