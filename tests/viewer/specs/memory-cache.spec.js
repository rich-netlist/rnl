import { expect, test } from '../viewer.js';

const dialog = page => page.locator('dialog.dialog');
const openCache = page => page.evaluate(() => { void window.viewer.command('memory-cache'); });

async function desktop(viewer, page) {
  await viewer.open('04-hierarchy', { platform: 'desktop', settings: '1' });
  await page.evaluate(async () => {
    const { shell } = await import('./shell.js');
    window.cacheCalls = [];
    let entries = 2;
    shell.memoryCache = async (clear = false) => {
      window.cacheCalls.push(clear);
      if (clear) entries = 0;
      const values = JSON.parse(sessionStorage.getItem('fake-settings'));
      // as build_cache.rs gives them, with the readings on disk
      return { limit: (values['file-cache-memory-mib'] ?? 256) * 1024 * 1024,
        used: entries * 1024 * 1024, entries, hits: 3, misses: 2, prefetched: 0,
        storedEntries: entries * 3, stored: entries * 3 * 1024 * 1024, storeLimit: (values['file-cache-store-mib'] ?? 2048) * 1024 * 1024,
        persistent: true, disk: true };
    };
  });
}

test('the desktop limits in memory and on disk validate, persist, can turn caching off and clear without replacing the view', async ({ page, viewer }) => {
  await desktop(viewer, page);
  const before = await viewer.state();
  await openCache(page);
  await expect(dialog(page)).toContainText('Readings in memory: 2 (2.0 MiB) · 3 hits, 2 misses');
  await expect(dialog(page)).toContainText('Readings on disk: 6 (6.0 MiB)');
  await expect(dialog(page)).toContainText("in the app's cache folder");
  const limit = dialog(page).getByLabel('Limit in memory (MiB)');
  await expect(limit).toHaveValue('256');
  const disk = dialog(page).getByLabel('Limit on disk (MiB)');
  await expect(disk).toHaveValue('2048');
  await disk.fill('65537');
  await dialog(page).getByRole('button', { name: 'Apply', exact: true }).click();
  await expect(dialog(page).getByRole('alert')).toContainText('from 0 to 65536');
  await disk.fill('512');
  await limit.fill('-1');
  await dialog(page).getByRole('button', { name: 'Apply', exact: true }).click();
  await expect(dialog(page).getByRole('alert')).toContainText('Enter a whole number');
  await limit.fill('128');
  await dialog(page).getByRole('button', { name: 'Apply', exact: true }).click();
  await expect(dialog(page)).toHaveCount(0);
  expect(await page.evaluate(() => JSON.parse(sessionStorage.getItem('fake-settings'))['file-cache-memory-mib'])).toBe(128);
  expect(await page.evaluate(() => JSON.parse(sessionStorage.getItem('fake-settings'))['file-cache-store-mib'])).toBe(512);
  await openCache(page);
  await expect(limit).toHaveValue('128');
  await expect(disk).toHaveValue('512');
  await dialog(page).getByRole('button', { name: 'Clear Cache', exact: true }).click();
  await expect(dialog(page)).toContainText('Readings in memory: 0');
  await expect(dialog(page)).toContainText('Readings on disk: 0');
  expect(await viewer.state()).toEqual(before);
  await limit.fill('0');
  await dialog(page).getByRole('button', { name: 'Apply', exact: true }).click();
  await expect(dialog(page)).toHaveCount(0);
  expect(await page.evaluate(() => JSON.parse(sessionStorage.getItem('fake-settings'))['file-cache-memory-mib'])).toBe(0);
});

test('reload requests a fresh build rather than reusing a cached result', async ({ page, viewer }) => {
  await desktop(viewer, page);
  await page.evaluate(async () => {
    const { shell } = await import('./shell.js');
    const build = shell.build;
    shell.build = (...args) => { window.buildOptions = args[5]; return build(...args); };
  });
  await viewer.build(() => page.keyboard.press('F5'));
  expect(await page.evaluate(() => window.buildOptions)).toEqual({ fresh: true });
});

test('web File menu does not offer a native cache setting', async ({ page, viewer }) => {
  await viewer.open('04-hierarchy');
  await page.getByRole('menuitem', { name: 'File', exact: true }).click();
  await expect(page.getByRole('menuitem', { name: /Memory Cache/ })).toHaveCount(0);
});
