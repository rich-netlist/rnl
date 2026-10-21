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
      return { limit: (values['render-cache-mib'] ?? 256) * 1024 * 1024,
        used: entries * 1024 * 1024, entries, hits: 3, misses: 2 };
    };
  });
}

test('desktop cache limit validates, persists, can disable caching and clears without replacing the view', async ({ page, viewer }) => {
  await desktop(viewer, page);
  const before = await viewer.state();
  await openCache(page);
  await expect(dialog(page)).toContainText('2 cached documents');
  const limit = dialog(page).getByLabel('Cache limit (MiB)');
  await expect(limit).toHaveValue('256');
  await limit.fill('-1');
  await dialog(page).getByRole('button', { name: 'Apply', exact: true }).click();
  await expect(dialog(page).getByRole('alert')).toContainText('Enter a whole number');
  await limit.fill('128');
  await dialog(page).getByRole('button', { name: 'Apply', exact: true }).click();
  await expect(dialog(page)).toHaveCount(0);
  expect(await page.evaluate(() => JSON.parse(sessionStorage.getItem('fake-settings'))['render-cache-mib'])).toBe(128);
  await openCache(page);
  await expect(limit).toHaveValue('128');
  await dialog(page).getByRole('button', { name: 'Clear Cache', exact: true }).click();
  await expect(dialog(page)).toContainText('0 cached documents');
  expect(await viewer.state()).toEqual(before);
  await limit.fill('0');
  await dialog(page).getByRole('button', { name: 'Apply', exact: true }).click();
  await expect(dialog(page)).toHaveCount(0);
  expect(await page.evaluate(() => JSON.parse(sessionStorage.getItem('fake-settings'))['render-cache-mib'])).toBe(0);
});

test('reload requests a fresh build rather than reusing a cached result', async ({ page, viewer }) => {
  await desktop(viewer, page);
  await page.evaluate(async () => {
    const { shell } = await import('./shell.js');
    const build = shell.build;
    shell.build = (...args) => { window.buildOptions = args[3]; return build(...args); };
  });
  await viewer.build(() => page.keyboard.press('F5'));
  expect(await page.evaluate(() => window.buildOptions)).toEqual({ fresh: true });
});

test('web File menu does not offer a native cache setting', async ({ page, viewer }) => {
  await viewer.open('04-hierarchy');
  await page.getByRole('menuitem', { name: 'File', exact: true }).click();
  await expect(page.getByRole('menuitem', { name: /Memory Cache/ })).toHaveCount(0);
});
