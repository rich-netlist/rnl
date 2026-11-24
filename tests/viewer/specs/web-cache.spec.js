// File › Memory Cache in the web viewer (memory-cache.js, file-reading.js `memoryCache`): the readings of each file kept
// in memory for the page, with its limit, 0 keeping none, and Clear Cache empties it. Nothing is kept in the browser's
// storage.
import { test, expect } from '../viewer.js';

test.skip(!process.env.RNL_WEB_URL, 'the web site is not built');

const TOP = `* top
.include "cells.inc"
X1 a b cell
.end
`;
const CELLS = `* cells
.subckt cell p n
R1 p n 1k
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
  ]);
  await idle(page);
}
const idle = (page) => page.waitForFunction(() => window.viewer.doc && !document.body.classList.contains('busy'));
const dialog = (page) => page.locator('dialog.dialog');
const stats = (page) => page.evaluate(async () => (await import('./file-reading.js')).readingStats());
async function openCache(page) {
  await page.getByRole('menuitem', { name: 'File', exact: true }).click();
  await page.getByRole('menuitem', { name: /Memory Cache/ }).click();
  await expect(dialog(page)).toBeVisible();
}

test('File › Memory Cache tells what the readings kept hold, and Clear Cache empties them', async ({ page }) => {
  await open(page);
  // the entry is read, and the file of the subcircuit too
  await page.evaluate(() => import('./subcircuits.js').then(s => s.descend(window.viewer.doc.objects.findIndex(o => o.kind === 'instance' && o.name === 'X1'))));
  await idle(page);
  await expect.poll(() => stats(page).then(s => s.entries)).toBe(2);
  await openCache(page);
  await expect(dialog(page).getByLabel('Limit in memory (MiB)')).toHaveValue('256');
  await expect(dialog(page).getByLabel("Limit in this browser's storage (MiB)")).toHaveCount(0);
  await expect(dialog(page)).toContainText(/Readings in memory: 2 \(\d+\.\d MiB\)/);
  await expect(dialog(page)).not.toContainText("browser's storage");
  await dialog(page).getByRole('button', { name: 'Clear Cache', exact: true }).click();
  await expect(dialog(page)).toContainText('Readings in memory: 0 (0.0 MiB) · 0 hits, 0 misses');
  await dialog(page).getByRole('button', { name: 'Cancel', exact: true }).click();
  // the document shown stays as it is
  expect(await page.evaluate(() => window.viewer.doc.file)).toBe('cells.inc');
  expect(await stats(page)).toMatchObject({ entries: 0 });
  // nothing of the readings is in the browser's storage
  expect(await page.evaluate(async () => (await indexedDB.databases()).map((d) => d.name))).toEqual([]);
});

test('a file shown again from its reading kept says so, as every shell does: "Using cached drawing…"', async ({ page }) => {
  await open(page);
  await page.evaluate(() => {
    window.loadingStages = [];
    new MutationObserver(() => window.loadingStages.push(document.getElementById('loading').dataset.stage))
      .observe(document.getElementById('loading'), { attributes: true, attributeFilter: ['data-stage'] });
  });
  // the file of the subcircuit is read: nothing kept yet
  await page.evaluate(() => import('./subcircuits.js').then(s => s.descend(window.viewer.doc.objects.findIndex(o => o.kind === 'instance' && o.name === 'X1'))));
  await idle(page);
  expect(await page.evaluate(() => window.loadingStages)).not.toContain('cached');
  // the entry again: its reading is kept
  await page.evaluate(() => window.loadingStages = []);
  await page.evaluate(() => import('./subcircuits.js').then(s => s.ascend()));
  await idle(page);
  expect(await page.evaluate(() => window.viewer.doc.file)).toBe('top.sp');
  expect(await page.evaluate(() => window.loadingStages)).toContain('cached');
  expect(await page.evaluate(() => window.loadingStages.at(-1))).toBe('displaying');
});

test('the limits are checked and kept, and 0 keeps no reading', async ({ page }) => {
  await open(page);
  await openCache(page);
  const memory = dialog(page).getByLabel('Limit in memory (MiB)');
  await memory.fill('-1');
  await dialog(page).getByRole('button', { name: 'Apply', exact: true }).click();
  await expect(dialog(page).getByRole('alert')).toContainText('Enter a whole number from 0 to 4096.');
  await memory.fill('0');
  await dialog(page).getByRole('button', { name: 'Apply', exact: true }).click();
  await expect(dialog(page)).toHaveCount(0);
  // the readings kept are gone, and a file read now is not kept
  expect(await stats(page)).toMatchObject({ entries: 0, limit: 0 });
  await page.evaluate(() => import('./subcircuits.js').then(s => s.descend(window.viewer.doc.objects.findIndex(o => o.kind === 'instance' && o.name === 'X1'))));
  await idle(page);
  expect(await stats(page)).toMatchObject({ entries: 0 });
  // a later visit keeps the limit, and finds nothing kept
  await open(page);
  expect(await stats(page)).toMatchObject({ entries: 0, limit: 0, hits: 0 });
  await openCache(page);
  await expect(memory).toHaveValue('0');
});

test('after a solve, or with the Solve view open, the readings are still kept and shown again from the cache', async ({ page }) => {
  await open(page);
  // the Solve view opened, as a solve opens it: the readings now carry the constraints of the netlist
  await page.locator('#tab-solve').click();
  await idle(page);
  const descend = () => page.evaluate(() => import('./subcircuits.js').then(s => s.descend(window.viewer.doc.objects.findIndex(o => o.kind === 'instance' && o.name === 'X1'))));
  await descend();
  await idle(page);
  expect(await page.evaluate(() => window.viewer.doc.file)).toBe('cells.inc');
  await page.evaluate(() => import('./subcircuits.js').then(s => s.ascend()));
  await idle(page);
  // the file of the subcircuit again: its reading, with the constraints, is kept
  await page.evaluate(() => {
    window.loadingStages = [];
    new MutationObserver(() => window.loadingStages.push(document.getElementById('loading').dataset.stage))
      .observe(document.getElementById('loading'), { attributes: true, attributeFilter: ['data-stage'] });
  });
  const before = await stats(page);
  await descend();
  await idle(page);
  expect(await page.evaluate(() => window.loadingStages)).toContain('cached');
  expect((await stats(page)).hits).toBe(before.hits + 1);
  // and it has the constraints the Solve view lists
  expect(await page.evaluate(() => import('./constraints.js').then(c => c.hasConstraints(window.viewer.doc)))).toBe(true);
});

test('a reading is kept and given again as it is, never copied nor written to', async ({ page }) => {
  const errors = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  // the copies of a reading the page makes, and the texts it makes of what it keeps
  await page.addInitScript(() => {
    window.copies = 0;
    window.sized = 0;
    const clone = window.structuredClone;
    window.structuredClone = (value, options) => {
      if (value && typeof value === 'object' && 'levels' in value && 'files' in value) window.copies++;
      return clone(value, options);
    };
    const stringify = JSON.stringify;
    JSON.stringify = function (value, ...rest) {
      if (value && typeof value === 'object' && 'result' in value && 'shape' in value) window.sized++;
      return stringify.call(this, value, ...rest);
    };
  });
  // what is kept is frozen through and through, as in every test of the pages (viewer.js `freezeKept`): a write into it
  // anywhere fails
  await open(page);
  expect(await page.evaluate(() => globalThis.rnlFreezeKept)).toBe(true);
  const descend = () => page.evaluate(() => import('./subcircuits.js').then(s => s.descend(window.viewer.doc.objects.findIndex(o => o.kind === 'instance' && o.name === 'X1'))));
  // read, then shown again from what is kept: the page draws, lists and selects in both
  await descend();
  await idle(page);
  await page.evaluate(() => import('./subcircuits.js').then(s => s.ascend()));
  await idle(page);
  await descend();
  await idle(page);
  expect(await stats(page)).toMatchObject({ entries: 2, hits: 2 });
  expect(await page.evaluate(() => window.viewer.doc.file)).toBe('cells.inc');
  await page.evaluate(() => import('./selection.js').then(s => s.select(window.viewer.doc.objects.findIndex(o => o.kind === 'instance' && o.name === 'R1'))));
  await page.locator('#tab-hierarchy').click();
  await page.locator('#tab-unplaced').click();
  await page.locator('#tab-solve').click();
  await idle(page);
  await page.locator('#tab-netlist').click();
  // the texts of the other files, which the reading leaves out, are in the netlist pane
  expect(await page.evaluate(() => window.viewer.doc.files.map(f => f.text.split('\n')[0]))).toEqual(['* top', '* cells']);
  await page.evaluate(() => import('./subcircuits.js').then(s => s.ascend()));
  await idle(page);
  expect(await page.evaluate(() => window.viewer.doc.file)).toBe('top.sp');
  expect(errors).toEqual([]);
  expect(await page.evaluate(() => [window.copies, window.sized])).toEqual([0, 0]);
});
