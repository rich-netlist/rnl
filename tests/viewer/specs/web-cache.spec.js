// File › Memory Cache in the web viewer (memory-cache.js, file-reading.js `memoryCache`): the readings of each file kept
// in memory for the page, with its limit, 0 keeping none, and Clear Cache empties it. Nothing is kept in the browser's
// storage.
import { test, expect } from '@playwright/test';

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
