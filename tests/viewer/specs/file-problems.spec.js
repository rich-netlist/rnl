// The problems of a document read one file at a time (file-checks.js, problems.js): the list has those of the file
// shown, though its reading has those of the files above it too; All Files adds those of the other files read, and
// says how many were; Edit > Check All Files reads the rest.
import { test, expect } from '../viewer.js';

test.skip(!process.env.RNL_WEB_URL, 'the web site is not built');

const TOP = `* top
.include "cells.inc"
X1 a b cell
* @RNL { #ZZTOP @PLACE(1, 1) }
.end
`;
const CELLS = `* cells
.include "leaf.inc"
.subckt cell p n
X1 p n leaf
* @RNL { #ZZCELL @PLACE(1, 1) }
.ends
`;
const LEAF = `* leaf
.subckt leaf a b
R1 a b 1k
* @RNL { #ZZLEAF @PLACE(1, 1) }
.ends
`;

async function open(page) {
  await page.route('https://rich-netlist.github.io/rnl-packages/**', route => route.abort());
  // no preloading, which reads files while the page is idle: what is kept is what the test reads
  await page.addInitScript(() => localStorage.setItem('file-preload', 'false'));
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

/** The instances the errors listed name, in the order listed. */
const listed = (page) => page.locator('#diags .diag.error .details .msg');
const named = (page) => listed(page).evaluateAll((rows) => rows.map((r) => /ZZ\w+/.exec(r.textContent)?.[0] ?? r.textContent));

async function descend(page, name) {
  const o = await page.evaluate((name) => window.viewer.doc.objects.findIndex(o => o.kind === 'instance' && o.name === name), name);
  await page.evaluate((o) => import('./subcircuits.js').then(s => s.descend(o)), o);
  await idle(page);
}

test('the problems of the file shown are listed, and All Files adds those of the other files read', async ({ page }) => {
  await open(page);
  const scope = page.locator('#problems-scope');
  const all = scope.locator('.all-files');
  await expect(all).toHaveAttribute('aria-pressed', 'false');
  await expect(scope.locator('.checked')).toHaveCount(0);
  expect(await named(page)).toEqual(['ZZTOP']);
  // the reading of cells.inc has the error of the entry too, which it reads above it: the list has its own file's
  await descend(page, 'X1');
  expect(await page.evaluate(() => window.viewer.doc.file)).toBe('cells.inc');
  expect(await page.evaluate(() => window.viewer.doc.diagnostics.map(d => /ZZ\w+/.exec(d.message)?.[0]).sort())).toEqual(['ZZCELL', 'ZZTOP']);
  await expect.poll(() => named(page)).toEqual(['ZZCELL']);
  // with All Files, the entry's error too, as its reading gave it, and how many files were read
  await all.click();
  await expect(all).toHaveAttribute('aria-pressed', 'true');
  await expect.poll(() => named(page)).toEqual(['ZZCELL', 'ZZTOP']);
  await expect(scope.locator('.checked')).toHaveText('2 of 3 files checked');
  // the heading is not folded by the switch
  await expect(page.locator('#problems-head')).toHaveAttribute('aria-expanded', 'true');
  // the entry's row shows its line in the entry's text
  await listed(page).filter({ hasText: 'ZZTOP' }).locator('xpath=ancestor::li[1]').click();
  await expect.poll(() => page.evaluate(async () => (await import('./text.js')).fileShown)).toBe(0);
  // and off again: the file shown's alone
  await all.click();
  await expect.poll(() => named(page)).toEqual(['ZZCELL']);
});

test('Check All Files reads each file not read yet, and the list has the problems of every file', async ({ page }) => {
  await open(page);
  const scope = page.locator('#problems-scope');
  await page.locator('#problems-scope .all-files').click();
  await expect(scope.locator('.checked')).toHaveText('1 of 3 files checked');
  await expect(scope.locator('.check-all')).toHaveText('Check All Files');
  // the command of the Edit menu, on the web
  expect(await page.evaluate(() => document.querySelectorAll('#menubar [data-id="check-all-files"]').length)).toBe(1);
  await page.evaluate(() => window.viewer.command('check-all-files'));
  await expect(scope.locator('.checked')).toHaveText('3 of 3 files checked');
  await expect(scope.locator('.check-all')).toHaveCount(0);
  await expect.poll(() => named(page)).toEqual(['ZZTOP', 'ZZCELL', 'ZZLEAF']);
  // the document shown is the entry's still, read once
  expect(await page.evaluate(() => window.viewer.doc.file)).toBe('top.sp');
  // a file checked is not read again when it is shown: its reading is the one kept
  await descend(page, 'X1');
  await expect.poll(() => named(page)).toEqual(['ZZCELL', 'ZZTOP', 'ZZLEAF']);
  await expect(scope.locator('.checked')).toHaveText('3 of 3 files checked');
});

test('a file edited starts the problems of the other files again', async ({ page }) => {
  await open(page);
  await page.evaluate(() => window.viewer.command('check-all-files'));
  await expect(page.locator('#problems-scope .checked')).toHaveText('3 of 3 files checked');
  // the entry's error mended, as Edit > Apply does
  expect(await page.evaluate(async () => {
    const edits = await import('./edits.js');
    const path = window.viewer.doc.files[0].path;
    edits.setText(path, edits.textOf(path).replace('#ZZTOP @PLACE(1, 1)', '#X1 @PLACE(1, 1)'));
    return (await import('./open.js')).applyEdits();
  })).toBe(true);
  await idle(page);
  await expect(page.locator('#problems-scope .checked')).toHaveText('1 of 3 files checked');
  await expect.poll(() => named(page)).toEqual([]);
});
