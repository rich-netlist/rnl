import { test } from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { archiveNames, libraryZip, readZip } from '../viewer-ui/zip.js';
import { libraryFiles } from '../viewer-ui/library-files.js';

// An independent ZIP implementation checks headers, Unicode names, body sizes and CRCs.
function unzip(blob) {
  return blob.arrayBuffer().then(bytes => JSON.parse(execFileSync('python', ['-c',
    'import sys,io,zipfile,json,base64,tempfile; z=zipfile.ZipFile(io.BytesIO(sys.stdin.buffer.read())); assert z.testzip() is None; d=tempfile.TemporaryDirectory(); z.extractall(d.name); d.cleanup(); print(json.dumps({n:base64.b64encode(z.read(n)).decode() for n in z.namelist()}))'],
    { input: Buffer.from(bytes), maxBuffer: 16 << 20 }).toString()));
}
const text = (files, name) => Buffer.from(files[name], 'base64').toString();

test('ZIP round trips Unicode paths, binary files, empty files, and all built-in versions', async () => {
  const binary = new Uint8Array(1048600).map((_, i) => i % 256);
  const zipped = await libraryZip([
    { path: '/project/电路.sp', read: () => '* 网表\r\n.end\r\n' },
    { path: '/empty.inc', read: () => '' },
    { path: '/data.bin', read: () => new Blob([binary]) },
    { path: '<bundled>/0.2/rnl-std.rnl', read: () => 'old' },
    { path: '<bundled>/0.3/rnl-std-basic.rnl', read: () => 'new' },
  ]);
  assert.equal(zipped.type, 'application/zip');
  const out = await unzip(zipped);
  assert.equal(text(out, 'project/电路.sp'), '* 网表\r\n.end\r\n');
  assert.equal(text(out, 'empty.inc'), '');
  assert.deepEqual(Buffer.from(out['data.bin'], 'base64'), Buffer.from(binary));
  assert.equal(text(out, 'standard-packages/0.2/rnl-std.rnl'), 'old');
  assert.equal(text(out, 'standard-packages/0.3/rnl-std-basic.rnl'), 'new');
});

test('ZIP paths stay relative, portable and distinct even after sanitization', async () => {
  const paths = ['/../a.sp', '/%2E%2E/a.sp', '/project/a.sp', '/project/A.sp', 'C:\\dir\\a.sp', '/NUL.inc', '/file.'];
  const out = await unzip(await libraryZip(paths.map((path, index) => ({ path, read: () => String(index) }))));
  assert.deepEqual(Object.keys(out), ['%2E%2E/a.sp', '%252E%252E/a.sp', 'project/a.sp', 'project/A (2).sp', 'C%3A/dir/a.sp', '_NUL.inc', 'file%2E']);
  assert.deepEqual(Object.values(out).map(v => Buffer.from(v, 'base64').toString()), paths.map((_, i) => String(i)));
});

test('unreadable files abort the archive with their identity instead of silently omitting them', async () => {
  await assert.rejects(libraryZip([{ path: '/missing.sp', read: () => { throw new Error('removed'); } }]), /missing.sp: removed/);
  await assert.rejects(libraryZip([{ path: '/missing.sp', read: () => null }]), /missing.sp could not be read/);
  await assert.rejects(libraryZip(Array(65535)), /too many files/);
});

test('library captures all files, loaded includes, current editor text and temporary slot without rereads', async () => {
  const folder = new Map([['/project/top.sp', () => 'old'], ['/project/lib/unused.inc', () => 'unused']]);
  const source = { path: '/project/top.sp', files: folder };
  const temporary = { name: 'clipboard.sp', text: 'clipboard', temporary: true };
  const builtIn = { path: '<bundled>/0.3/std.rnl', source: { text: 'standard' } };
  const snap = { items: [{ path: source.path, source, read: () => { throw Error('network is offline'); } }, builtIn], temporary };
  const document = { path: source.path, files: [
    { path: 'top.sp', text: 'applied' }, { path: 'lib/parts.inc', text: 'remote include snapshot' },
  ] };
  const entries = libraryFiles(snap, source, document, path => path === 'top.sp' ? 'pending' : null);
  temporary.text = 'later'; // a picker/network delay cannot change the captured export
  document.files[1].text = 'later';
  const out = await unzip(await libraryZip(entries));
  assert.equal(text(out, 'project/top.sp'), 'pending');
  assert.equal(text(out, 'project/lib/unused.inc'), 'unused');
  assert.equal(text(out, 'project/lib/parts.inc'), 'remote include snapshot');
  assert.equal(text(out, 'standard-packages/0.3/std.rnl'), 'standard');
  assert.equal(text(out, 'temporary/clipboard.sp'), 'clipboard');
});

test('current clipboard edits override only the temporary slot, not a real file of the same name', async () => {
  const temporary = { name: 'clipboard.sp', text: 'old', temporary: true };
  const entries = libraryFiles({ items: [{ path: '/clipboard.sp', source: {}, read: () => 'real file' }], temporary },
    temporary, { path: '/clipboard.sp', files: [{ path: 'clipboard.sp', text: 'applied' }] }, () => 'pending', { 'clipboard.sp': 'pending' });
  const out = await unzip(await libraryZip(entries));
  assert.equal(text(out, 'clipboard.sp'), 'real file');
  assert.equal(text(out, 'temporary/clipboard.sp'), 'pending');
});

test('already applied edits take precedence over the original selected File bytes', async () => {
  const source = { path: '/top.sp' };
  const entries = libraryFiles({ items: [{ path: '/top.sp', source, read: () => 'original' }], temporary: null },
    source, { path: '/top.sp', files: [{ path: 'top.sp', text: 'applied edit' }] }, () => null);
  assert.equal(text(await unzip(await libraryZip(entries)), 'top.sp'), 'applied edit');
});

test('edits of Library files removed from the current include closure are still exported', async () => {
  const source = { path: '/project/top.sp', files: new Map([['/project/top.sp', () => 'original top'], ['/project/parts.inc', () => 'original part']]) };
  const entries = libraryFiles({ items: [{ path: source.path, source, read: () => 'original top' }], temporary: null },
    source, { path: source.path, files: [{ path: 'top.sp', text: 'top without include' }] }, () => null,
    { 'parts.inc': 'edited part', 'excluded.inc': 'not in Library' });
  const out = await unzip(await libraryZip(entries));
  assert.equal(text(out, 'project/parts.inc'), 'edited part');
  assert.equal(Object.hasOwn(out, 'project/excluded.inc'), false);
});

test('original File readers retain BOM and Latin-1 bytes, including the unedited current root', async () => {
  const bytes = new Uint8Array([239, 187, 191, 42, 32, 233, 10]);
  const source = { path: '/a.sp', files: new Map([['/a.sp', () => new Blob([bytes])]]) };
  const entries = libraryFiles({ items: [{ path: '/a.sp', source, read: () => 'decoded string' }] },
    source, { path: '/a.sp', files: [{ path: 'a.sp', text: 'decoded string' }] }, () => 'decoded string');
  assert.deepEqual(Buffer.from((await unzip(await libraryZip(entries)))['a.sp'], 'base64'), Buffer.from(bytes));
});

test('explicit Library roots win over stale companion snapshots regardless of enumeration order', async () => {
  const newest = { path: '/a.sp', source: { files: new Map([['/a.sp', () => 'new a']]) }, read: () => 'decoded new a' };
  const old = { path: '/b.sp', source: { files: new Map([['/a.sp', () => 'stale a'], ['/b.sp', () => 'b']]) }, read: () => 'b' };
  for (const items of [[newest, old], [old, newest]]) {
    const out = await unzip(await libraryZip(libraryFiles({ items }, null, null, () => null)));
    assert.equal(text(out, 'a.sp'), 'new a');
    assert.equal(text(out, 'b.sp'), 'b');
  }
});

test('files and directories with the same path can all be extracted, in either input order', async () => {
  const entries = ['/lib.sp', '/lib.sp/cell.sp', '/standard-packages', '<bundled>/0.3/std.rnl', '/temporary', '<temporary>/clipboard.sp'];
  for (const paths of [entries, [...entries].reverse()]) {
    const out = await unzip(await libraryZip(paths.map(path => ({ path, read: () => path }))));
    assert.equal(text(out, 'lib (2).sp'), '/lib.sp');
    assert.equal(text(out, 'lib.sp/cell.sp'), '/lib.sp/cell.sp');
    assert.equal(text(out, 'standard-packages (2)'), '/standard-packages');
    assert.equal(text(out, 'standard-packages/0.3/std.rnl'), '<bundled>/0.3/std.rnl');
    assert.equal(text(out, 'temporary (2)'), '/temporary');
    assert.equal(text(out, 'temporary/clipboard.sp'), '<temporary>/clipboard.sp');
  }
});

test('readZip reads what libraryZip writes, and archive names give themselves again', async () => {
  const binary = new Uint8Array(70000).map((_, i) => (i * 7) % 256);
  const files = [{ path: '/project/电路.sp', read: () => '* 网表\n' },{ path: '/data.bin', read: () => binary }, { path: '/lib.sp', read: () => 'x' }, { path: '/lib.sp/a.sp', read: () => 'y' }];
  const names = archiveNames(files.map(f => f.path));
  assert.deepEqual(archiveNames(names), names);
  const read = readZip(new Uint8Array(await (await libraryZip(files)).arrayBuffer()));
  assert.deepEqual([...read.keys()], names);
  assert.equal(new TextDecoder().decode(await read.get('project/电路.sp').read()), '* 网表\n');
  assert.deepEqual(await read.get('data.bin').read(), binary);
});

test('readZip inflates deflated entries written by another tool and refuses a damaged archive', async () => {
  const bytes = execFileSync('python', ['-c',
    'import sys,io,zipfile; b=io.BytesIO(); z=zipfile.ZipFile(b,"w",zipfile.ZIP_DEFLATED); z.writestr("dir/","");' +
    'z.writestr("dir/top.sp","* top\\n"*500); z.writestr("ünï.inc","x"); z.close(); sys.stdout.buffer.write(b.getvalue())']);
  const read = readZip(new Uint8Array(bytes));
  assert.deepEqual([...read.keys()], ['dir/top.sp', 'ünï.inc']);
  assert.equal(new TextDecoder().decode(await read.get('dir/top.sp').read()), '* top\n'.repeat(500));
  const damaged = new Uint8Array(bytes);
  const at = Buffer.from(damaged).indexOf('dir/top.sp') + 10;
  damaged[at + 3] ^= 0xff;
  await assert.rejects(readZip(damaged).get('dir/top.sp').read(), /damaged|invalid|incorrect/i);
  assert.throws(() => readZip(new Uint8Array([1, 2, 3])), /not a ZIP archive/);
});
