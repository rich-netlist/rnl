import { test } from 'node:test';
import assert from 'node:assert/strict';
import * as edits from '../viewer-ui/edits.js';

const original = '* original\r\nR1 a 0 1k\r\n.end\r\n';
const doc = text => ({files:[{path:'top.sp',name:'top.sp',text}]});
const snapshot = () => ({path:'top.sp',text:edits.fileText('top.sp')});

test('a file of mixed line ends is written with the end of each line: its own, the one it replaces, or the most', () => {
  const mixed = '* a\r\nR1 a 0 1k\nR2 a 0 1k\r\n.end\n';
  edits.resetBuffers(doc(mixed));
  assert.equal(edits.fileText('top.sp'), mixed);
  // a line changed keeps the end of the line it replaces
  edits.setText('top.sp', edits.textOf('top.sp').replace('R1 a 0 1k', 'R1 a 0 2k'));
  assert.equal(edits.fileText('top.sp'), mixed.replace('R1 a 0 1k', 'R1 a 0 2k'));
  // a line added after it has the end the file has most of (two CRLF, two LF: LF)
  edits.setText('top.sp', edits.textOf('top.sp').replace('R1 a 0 2k\n', 'R1 a 0 2k\nR3 a 0 1k\n'));
  assert.equal(edits.fileText('top.sp'), '* a\r\nR1 a 0 2k\nR3 a 0 1k\nR2 a 0 1k\r\n.end\n');
  // a line removed: the others keep theirs
  edits.setText('top.sp', '* a\nR2 a 0 1k\n.end\n');
  assert.equal(edits.fileText('top.sp'), '* a\r\nR2 a 0 1k\r\n.end\n');
  // once saved, the ends written are the file's
  edits.markSaved([{ path: 'top.sp', text: edits.fileText('top.sp') }], true);
  edits.setText('top.sp', '* b\nR2 a 0 1k\n.end\n');
  assert.equal(edits.fileText('top.sp'), '* b\r\nR2 a 0 1k\r\n.end\n');
});

test('a downloaded pending edit remains a build overlay after Save, Apply and repeated rebuilds', () => {
  edits.resetBuffers(doc(original));
  const changed = original.replace('1k','2k');
  edits.setText('top.sp',changed);
  edits.markSaved([snapshot()],false);
  assert.deepEqual(edits.dirtyPaths(),[]);
  assert.deepEqual(edits.unappliedPaths(),['top.sp']);
  assert.deepEqual(edits.changes(false),{'top.sp':changed});
  edits.adoptBuffers(doc(changed));
  assert.deepEqual(edits.unappliedPaths(),[]);
  assert.deepEqual(edits.changes(true),{'top.sp':changed});
});

test('a native save updates the disk baseline but keeps unapplied text out of applied-only rebuilds', () => {
  edits.resetBuffers(doc(original));
  const changed = original.replace('1k','2k');
  edits.setText('top.sp',changed);
  edits.markSaved([snapshot()],true);
  assert.deepEqual(edits.dirtyPaths(),[]);
  assert.deepEqual(edits.changes(false),{}); // the next disk read supplies the newly saved text
  assert.deepEqual(edits.changes(true),{'top.sp':original}); // Library/Solve keep the old applied view
  edits.adoptBuffers(doc(changed));
  assert.deepEqual(edits.unappliedPaths(),[]);
  assert.deepEqual(edits.changes(true),{});
});

test('saving records only its snapshot, preserving later typing as unsaved with original line endings', () => {
  for (const native of [false,true]) {
    edits.resetBuffers(doc(original));
    const saved = original.replace('1k','2k');
    edits.setText('top.sp',saved);
    const file = snapshot();
    edits.setText('top.sp',saved.replace('2k','3k'));
    edits.markSaved([file],native);
    assert.deepEqual(edits.dirtyPaths(),['top.sp']);
    assert.equal(edits.dirty()[0].read,saved.replace(/\r\n/g,'\n'));
    assert.equal(edits.dirty()[0].text,saved.replace('2k','3k'));
    assert.deepEqual(edits.changes(false),{'top.sp':saved.replace('2k','3k')});
  }
});

test('a clipboard source rebased to applied text can subsequently be edited back to its first text', () => {
  edits.resetBuffers(doc(original));
  const changed = original.replace('1k','2k');
  edits.setText('top.sp',changed);
  edits.markSaved([snapshot()],false);
  edits.adoptBuffers(doc(changed));
  edits.rebaseSource('top.sp',changed);
  edits.setText('top.sp',original);
  assert.deepEqual(edits.changes(false),{'top.sp':original});
  assert.deepEqual(edits.dirtyPaths(),['top.sp']);
});
