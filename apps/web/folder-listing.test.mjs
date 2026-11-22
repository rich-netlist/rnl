// A folder included, as the web shell lists the files a browser gives it (apps/viewer-ui/folder-listing.js): by the rules
// of rnl_cli::walk, which the desktop app and the local program list a folder with on disk.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { folderListing, INCLUDE_FOLDER } from '../viewer-ui/folder-listing.js';

const listed = (path) => /\.(sp|inc|rnl)$/i.test(path);
const paths = (items) => folderListing(items, (p) => p, listed);

test('the rules are those of rnl_cli::walk::INCLUDE_FOLDER', () => {
  const walk = readFileSync(new URL('../../crates/rnl-cli/src/walk.rs', import.meta.url), 'utf8');
  const [, depth, limit] = /pub const INCLUDE_FOLDER: Walk = Walk \{ depth: Some\((\d+)\), limit: (\d+) \};/.exec(walk) ?? [];
  assert.deepEqual({ depth: Number(depth), limit: Number(limit) }, { ...INCLUDE_FOLDER });
});

test('a folder lists the files of its suffixes, nothing hidden, down to its depth, sorted', () => {
  const deep = (levels) => `lib/${Array.from({ length: levels }, (_, i) => `d${i}`).join('/')}/cell.inc`;
  const { items, truncated } = paths([
    'lib/top.sp', 'lib/notes.txt', 'lib/.hidden.sp', 'lib/.git/x.sp', 'lib/sub/parts.RNL', deep(8), deep(9), 'alone.sp', 'alone.txt',
  ]);
  assert.deepEqual(items, ['alone.sp', deep(8), 'lib/sub/parts.RNL', 'lib/top.sp']);
  assert.equal(truncated, false);
});

test('a folder with more files than the limit keeps those the walk meets first: a folder\'s files, then its folders', () => {
  const walk = { depth: 8, limit: 3 };
  const given = ['lib/b/1.sp', 'lib/z.sp', 'lib/a/1.sp', 'lib/a.sp', 'other/x.sp'];
  const { items, truncated } = folderListing(given, (p) => p, listed, walk);
  // lib: its files a.sp and z.sp, then a/1.sp; b/1.sp is not met. other has its own limit. Sorted by parts, as Rust
  // sorts paths: lib/a/1.sp before lib/a.sp
  assert.deepEqual(items, ['lib/a/1.sp', 'lib/a.sp', 'lib/z.sp', 'other/x.sp']);
  assert.equal(truncated, true);
});

test('a leading slash is not a part: dropped files are named so', () => {
  assert.deepEqual(paths(['/lib/top.sp', '/lib/.cache/top.sp', '/top.sp']).items, ['/lib/top.sp', '/top.sp']);
});
