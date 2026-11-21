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

/** What the readings kept say of themselves (result-cache.js `stats`). */
const kept = (page) => page.evaluate(async () => (await import('./shells/web.js')).default.readingStats());
const descendNamed = async (page, name) => {
  await page.evaluate((name) => import('./subcircuits.js').then(s => s.descend(window.viewer.doc.objects.findIndex(o => o.kind === 'instance' && o.name === name))), name);
  await idle(page);
};

test('a file read before is shown again from the readings kept, until a file it depends on changes', async ({ page }) => {
  await open(page);
  expect(await kept(page)).toMatchObject({ hits: 0, misses: 1 });
  await descendNamed(page, 'X1');
  await page.evaluate(() => import('./subcircuits.js').then(s => s.backTo(0)));
  await idle(page);
  await descendNamed(page, 'X1');
  expect(await shown(page)).toMatchObject({ file: 'cells.inc', levels: ['cell'], path: ['top.sp', 'cell'] });
  expect(await kept(page)).toMatchObject({ hits: 2, misses: 2 });
  // the core reads it again for the drawings of what has no position, which a kept reading leaves out
  await page.locator('#tab-unplaced').click();
  await expect(page.locator('#unplaced-list svg').first()).toBeVisible();
  await expect(page.locator('#unplaced-list')).toContainText('X2');
  await expect(page.locator('#unplaced-list')).not.toContainText('document was built again');
  await page.locator('#tab-netlist').click();
  // the subcircuit cells.inc calls changes: its reading is made again, and kept again
  await page.keyboard.press('Control+q');
  await page.evaluate(async () => (await import('./text.js')).showFile(window.viewer.doc.files.findIndex(f => f.path === 'leaf.inc')));
  await page.locator('#source .editor:not([hidden]) textarea').fill(LEAF.replace('R1 a b 1k', 'R1 a b 2k'));
  await page.locator('#apply-edits').click();
  await idle(page);
  await expect.poll(() => kept(page)).toMatchObject({ hits: 2, misses: 3 });
  expect(await shown(page)).toMatchObject({ file: 'cells.inc', levels: ['cell'] });
});

test('the readings are kept in memory alone: another visit of the page reads the file again', async ({ page }) => {
  await open(page);
  await expect.poll(async () => (await kept(page)).entries).toBe(1);
  await open(page);
  expect(await kept(page)).toMatchObject({ entries: 1, hits: 0, misses: 1 });
  expect(await shown(page)).toMatchObject({ file: 'top.sp', levels: ['top.sp', 't'] });
  expect(await page.evaluate(async () => (await indexedDB.databases()).map((d) => d.name))).toEqual([]);
});

test('an included file opens from the Library in the context of the entry, and Add to Library makes it a Library file', async ({ page }) => {
  await open(page);
  await page.locator('#tab-files').click();
  const files = page.locator('.lib-group[data-group="files"]');
  await expect(files.locator('.pool-include .pool-include-name')).toHaveText(['cells.inc', 'leaf.inc']);
  await files.locator('.pool-include').getByRole('button', { name: 'Open cells.inc', exact: true }).click();
  await idle(page);
  expect(await shown(page)).toMatchObject({ file: 'cells.inc', entry: 'top.sp', level: 'cell' });
  await expect(page.locator('#pane-files')).toBeVisible();
  // the rows stay those of the entry, which the Library knows; Add to Library gives leaf.inc a row of its own
  await files.locator('.pool-include').getByRole('button', { name: 'Add leaf.inc to the Library', exact: true }).click();
  await expect(files.locator('.pool-row .pool-open')).toHaveText(['top.sp', 'leaf.inc']);
  await expect(files.locator('.pool-include .pool-include-name')).toHaveText(['cells.inc']);
});
