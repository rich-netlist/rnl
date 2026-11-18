import { test } from 'node:test';
import assert from 'node:assert/strict';
import { commentOf, editKey } from '../viewer-ui/editor-keys.js';

// the text with the edit made, and the selection after it, written as | (caret) or [ ] (selection); the text as it was
// for a key taken that changes nothing; null for a key left to the text box
function press(marked, key, options = {}) {
  const caret = marked.indexOf('|');
  const start = caret >= 0 ? caret : marked.indexOf('[');
  const end = caret >= 0 ? caret : marked.indexOf(']') - 1;
  const text = marked.replace(/[|[\]]/g, '');
  const edit = editKey({ text, start, end, key, format: 'spice', ...options });
  if (!edit) return null;
  if (edit.none) return marked;
  const after = text.slice(0, edit.from) + edit.insert + text.slice(edit.to);
  return edit.start === edit.end
    ? after.slice(0, edit.start) + '|' + after.slice(edit.start)
    : after.slice(0, edit.start) + '[' + after.slice(edit.start, edit.end) + ']' + after.slice(edit.end);
}

test('a comment is its marker and the space after it, by the format of the file', () => {
  assert.deepEqual(commentOf('*   #R1 @PLACE(0, 0)', 'spice'), { marker: 1, prefix: '*   ' });
  assert.deepEqual(commentOf('// a note', 'spectre'), { marker: 2, prefix: '// ' });
  assert.deepEqual(commentOf('* a note', 'spectre'), { marker: 1, prefix: '* ' });
  assert.equal(commentOf('// no comment of SPICE', 'spice'), null);
  assert.equal(commentOf('R1 a b 1k', 'spice'), null);
  assert.equal(commentOf('* in a package', 'rnl'), null);
});

test('Enter in a comment begins the next line with its marker and its space', () => {
  assert.equal(press('* @RNL {\n*   #R1 @PLACE(0, 0);|\n* }', 'Enter'), '* @RNL {\n*   #R1 @PLACE(0, 0);\n*   |\n* }');
  assert.equal(press('// a note|', 'Enter', { format: 'spectre' }), '// a note\n// |');
  // in the middle of the text, the rest goes to the new line after the marker
  assert.equal(press('* first| second', 'Enter'), '* first\n* | second');
  // what is selected is replaced
  assert.equal(press('* [one ]two', 'Enter'), '* \n* |two');
  // right after the marker it still continues
  assert.equal(press('*|', 'Enter'), '*\n*|');
});

test('Enter before the marker, on a line that is no comment, with Shift or with Ctrl is a plain Enter', () => {
  assert.equal(press('|* a note', 'Enter'), null);
  assert.equal(press('  |* a note', 'Enter'), null);
  assert.equal(press('R1 a b 1k|', 'Enter'), null);
  assert.equal(press('* a note|', 'Enter', { shift: true }), null);
  assert.equal(press('* a note|', 'Enter', { other: true }), null);
  assert.equal(press('// a note|', 'Enter'), null, 'no comment of SPICE');
});

test('Tab puts in the spaces of an indent; after Esc it is left to move the focus', () => {
  assert.equal(press('R1 a| b 1k', 'Tab'), 'R1 a  | b 1k');
  assert.equal(press('R1 [a] b 1k', 'Tab'), 'R1   | b 1k');
  assert.equal(press('R1 a| b 1k', 'Tab', { escaped: true }), null);
  assert.equal(press('R1 a| b 1k', 'Tab', { shift: true, escaped: true }), null);
  assert.equal(press('R1 a| b 1k', 'Tab', { other: true }), null);
});

test('Tab over several lines indents each, after the marker of a comment, and leaves empty lines', () => {
  assert.equal(press('[R1 a b 1k\n\n*   #R1 @PLACE(0, 0)\n]R2 b c 1k', 'Tab'), '  [R1 a b 1k\n\n*     #R1 @PLACE(0, 0)\n]R2 b c 1k');
});

test('Shift+Tab outdents each line of the selection, and keeps one space after a marker', () => {
  assert.equal(press('    R1 a| b 1k', 'Tab', { shift: true }), '  R1 a| b 1k');
  assert.equal(press(' R1 a| b 1k', 'Tab', { shift: true }), 'R1 a| b 1k');
  assert.equal(press('*   #R1| @PLACE(0, 0)', 'Tab', { shift: true }), '* #R1| @PLACE(0, 0)');
  assert.equal(press('*  #R1| @PLACE(0, 0)', 'Tab', { shift: true }), '* #R1| @PLACE(0, 0)');
  // nothing to take out: the key is taken, and changes nothing, so that the focus stays
  assert.equal(press('* #R1| @PLACE(0, 0)', 'Tab', { shift: true }), '* #R1| @PLACE(0, 0)');
  assert.equal(press('R1 a| b 1k', 'Tab', { shift: true }), 'R1 a| b 1k');
  assert.equal(press('[R1 a b 1k\n\n]', 'Tab', { shift: true }), '[R1 a b 1k\n\n]');
  assert.equal(press('[  R1 a b 1k\n*     x\n]', 'Tab', { shift: true }), '[R1 a b 1k\n*   x\n]');
  // a caret in the space taken out goes to where it was
  assert.equal(press(' | R1 a b 1k', 'Tab', { shift: true }), '|R1 a b 1k');
});
