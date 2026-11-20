// Per-file loading in the web viewer (shells/web.js `perFile`, visit-source.js, level-tree.js): the page reads the
// file shown alone, in the context of its entry, with the outline of the whole document. The hierarchy lists the
// levels of every file from the outline; entering a subcircuit another file defines shows that file's reading, with
// the same entry, and the path bar and the tree follow the way back.
import { test, expect } from '@playwright/test';

test.skip(!process.env.RNL_WEB_URL, 'the web site is not built');

const TOP = `* top
.include "cells.inc"
.subckt t p q
R9 p q 1k
.ends
X1 a b cell
X2 a b t
.end
`;
const CELLS = `* cells
.include "leaf.inc"
.subckt cell p n
X1 p n leaf
X2 n p leaf
.ends
`;
const LEAF = `* leaf
.subckt leaf a b
R1 a b 1k
.ends
`;

async function open(page) {
  await page.route('https://rich-netlist.github.io/rnl-packages/**', route => route.abort());
  await page.goto(`${process.env.RNL_WEB_URL}/index.html`);
  await expect(page.locator('#menubar')).toBeVisible();
  await page.locator('#file-input').setInputFiles([
    { name: 'top.sp', mimeType: 'text/plain', buffer: Buffer.from(TOP) },
    { name: 'cells.inc', mimeType: 'text/plain', buffer: Buffer.from(CELLS) },
    { name: 'leaf.inc', mimeType: 'text/plain', buffer: Buffer.from(LEAF) },
  ]);
  await idle(page);
}
const idle = (page) => page.waitForFunction(() => window.viewer.doc && !document.body.classList.contains('busy'));

/** What the page shows: the file read, the entry, the levels it has, the level drawn and the path's names. */
const shown = (page) => page.evaluate(() => {
  const d = window.viewer.doc;
  return { file: d.file, entry: d.path.split('/').pop(), levels: d.levels.map(l => l.title), level: d.levels[window.viewer.level].title,
    path: [...document.querySelectorAll('#level-path .crumb')].map(c => c.querySelector('.name, #level-name')?.textContent) };
});

test('the entry is read alone, and the hierarchy lists the levels of every file from the outline', async ({ page }) => {
  await open(page);
  expect(await shown(page)).toMatchObject({ file: 'top.sp', entry: 'top.sp', levels: ['top.sp', 't'], level: 'top.sp' });
  // the other files' texts are in the netlist pane, though only the entry was read
  expect(await page.evaluate(() => window.viewer.doc.files.map(f => [f.path, f.text.split('\n')[0]])))
    .toEqual([['top.sp', '* top'], ['cells.inc', '* cells'], ['leaf.inc', '* leaf']]);
  // the index has the subcircuits of every file, with their uses
  const index = page.locator('#index .index-row:not(.top)');
  await expect(index.locator('.name')).toHaveText(['cell', 'leaf', 't']);
  await expect(index.locator('.uses')).toHaveText(['1 use', '2 uses', '1 use']);
  // the tree, under the top level: the instances, each with the subcircuit it calls
  const rows = page.locator('#tree .tree-row');
  await expect(rows.locator('.inst')).toHaveText(['top.sp', 'X1', 'X2']);
});

test('entering a subcircuit of another file reads that file in the context of the entry, and the path leads back', async ({ page }) => {
  await open(page);
  const x1 = await page.evaluate(() => window.viewer.doc.objects.findIndex(o => o.kind === 'instance' && o.name === 'X1'));
  expect(await page.evaluate((o) => import('./subcircuits.js').then(s => s.canOpen(o)), x1)).toBe(true);
  await page.evaluate((o) => import('./subcircuits.js').then(s => s.descend(o)), x1);
  await idle(page);
  expect(await shown(page)).toMatchObject({ file: 'cells.inc', entry: 'top.sp', levels: ['cell'], level: 'cell', path: ['top.sp', 'cell'] });
  // the tree keeps the way open to the level shown, from the top level
  await expect(page.locator('#tree .tree-row.active .inst')).toHaveText('X1');
  // and on down into the third file
  const leaf = await page.evaluate(() => window.viewer.doc.objects.findIndex(o => o.kind === 'instance' && o.name === 'X2'));
  await page.evaluate((o) => import('./subcircuits.js').then(s => s.descend(o)), leaf);
  await idle(page);
  expect(await shown(page)).toMatchObject({ file: 'leaf.inc', entry: 'top.sp', level: 'leaf', path: ['top.sp', 'cell', 'leaf'] });
  // back up the path: the entry's reading again, with the instance it was left through selected
  await page.evaluate(() => import('./subcircuits.js').then(s => s.backTo(0)));
  await idle(page);
  expect(await shown(page)).toMatchObject({ file: 'top.sp', level: 'top.sp', path: ['top.sp'] });
  await expect.poll(() => page.evaluate(() => window.viewer.doc.objects[window.viewer.selected]?.name)).toBe('X1');
});

test('a level of another file picked in the index is that file\'s reading, as the root of the path', async ({ page }) => {
  await open(page);
  await page.locator('#tab-hierarchy').click();
  await page.locator('#index .index-row:not(.top)', { hasText: 'leaf' }).click();
  await idle(page);
  expect(await shown(page)).toMatchObject({ file: 'leaf.inc', entry: 'top.sp', levels: ['leaf'], level: 'leaf', path: ['leaf'] });
  await expect(page.locator('#index .index-row.current .name')).toHaveText('leaf');
  // and a level of the entry's own file, from there: the entry's reading
  await page.locator('#index .index-row:not(.top)', { hasText: /^t/ }).click();
  await idle(page);
  expect(await shown(page)).toMatchObject({ file: 'top.sp', level: 't', path: ['t'] });
});
