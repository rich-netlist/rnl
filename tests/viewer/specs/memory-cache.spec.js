import { expect, test } from '../viewer.js';

const dialog = page => page.locator('dialog.dialog');
const openCache = page => page.evaluate(() => { void window.viewer.command('memory-cache'); });

// File › Memory Cache (memory-cache.js): the page's readings of each file kept (file-reading.js), the same dialog in every
// shell that reads by file. The fake shell builds whole documents; it is given a `reader` here, which nothing reads with.
async function desktop(viewer, page) {
  await viewer.open('04-hierarchy', { platform: 'desktop', settings: '1' });
  await page.evaluate(async () => {
    const { shell } = await import('./shell.js');
    shell.reader = async () => { throw new Error('no reading in this test'); };
  });
}

test('the cache limit validates, persists, can keep none, and Clear Cache leaves the view as it is', async ({ page, viewer }) => {
  await desktop(viewer, page);
  const before = await viewer.state();
  await openCache(page);
  await expect(dialog(page)).toContainText('Readings in memory: 0 (0.0 MiB) · 0 hits, 0 misses');
  const limit = dialog(page).getByLabel('Limit in memory (MiB)');
  await expect(limit).toHaveValue('256');
  await limit.fill('-1');
  await dialog(page).getByRole('button', { name: 'Apply', exact: true }).click();
  await expect(dialog(page).getByRole('alert')).toContainText('Enter a whole number');
  await limit.fill('128');
  await dialog(page).getByRole('button', { name: 'Apply', exact: true }).click();
  await expect(dialog(page)).toHaveCount(0);
  expect(await page.evaluate(() => JSON.parse(sessionStorage.getItem('fake-settings'))['file-cache-memory-mib'])).toBe(128);
  expect(await page.evaluate(async () => (await import('./file-reading.js')).readingStats().limit)).toBe(128 * 1024 * 1024);
  await openCache(page);
  await expect(limit).toHaveValue('128');
  await dialog(page).getByRole('button', { name: 'Clear Cache', exact: true }).click();
  await expect(dialog(page)).toContainText('Readings in memory: 0 (0.0 MiB) · 0 hits, 0 misses');
  expect(await viewer.state()).toEqual(before);
  await limit.fill('0');
  await dialog(page).getByRole('button', { name: 'Apply', exact: true }).click();
  await expect(dialog(page)).toHaveCount(0);
  expect(await page.evaluate(() => JSON.parse(sessionStorage.getItem('fake-settings'))['file-cache-memory-mib'])).toBe(0);
});

test('reload requests a fresh build rather than reusing a cached result', async ({ page, viewer }) => {
  await viewer.open('04-hierarchy', { platform: 'desktop', settings: '1' });
  await page.evaluate(async () => {
    const { shell } = await import('./shell.js');
    const build = shell.build;
    shell.build = (...args) => { window.buildOptions = args[5]; return build(...args); };
  });
  await viewer.build(() => page.keyboard.press('F5'));
  expect(await page.evaluate(() => window.buildOptions)).toEqual({ fresh: true });
});

test('a web page that reads whole documents does not offer it', async ({ page, viewer }) => {
  await viewer.open('04-hierarchy');
  await page.getByRole('menuitem', { name: 'File', exact: true }).click();
  await expect(page.getByRole('menuitem', { name: /Memory Cache/ })).toHaveCount(0);
});
