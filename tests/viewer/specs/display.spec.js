// A manifest's display settings (apps/viewer-ui/display.js): written into a project when the user
// asks, offered when it is opened, each with a box, and applied for the page only.
import { test, expect } from '@playwright/test';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';

test.skip(!process.env.RNL_WEB_URL, 'the web site is not built');

async function ready(page) {
  await page.waitForFunction(() => window.viewer.doc && !document.body.classList.contains('busy'));
}
/** The display settings the page shows, and the ones its settings keep. */
function state(page) {
  return page.evaluate(async () => {
    const { overlayChoices } = await import('./usages.js');
    const { gridSettings } = await import('./grid.js');
    const { currentColors } = await import('./netlist-colors.js');
    const { load } = await import('./util.js');
    const c = overlayChoices(), g = gridSettings();
    return {
      background: document.body.dataset.background, usages: c.usages, currents: c.currents, grid: g.shown ? g.style : 'none',
      component: currentColors().component,
      kept: { background: load('background', 'light'), currents: load('currents-shown', false), colors: load('netlist-colors', null) },
    };
  });
}
const dialog = (page) => page.getByRole('dialog', { name: 'Display Settings', exact: true });

test('Download Project writes the display settings when asked; Open Project offers them and applies those kept, for the page only', async ({ page }) => {
  // the browser's ordinary download, not a picker
  await page.addInitScript(() => Object.defineProperty(window, 'showSaveFilePicker', { value: undefined, configurable: true, writable: true }));
  await page.goto(`${process.env.RNL_WEB_URL}/index.html?url=samples/cases/04-hierarchy.sp`);
  await ready(page);
  // the page as the project will propose it
  await page.evaluate(async () => {
    window.viewer.command('background-dark');
    window.viewer.command('toggle-currents');
    window.viewer.command('grid-lines');
    (await import('./netlist-colors.js')).showColors({ component: '#123456' });
  });
  const download = page.waitForEvent('download');
  await page.evaluate(() => { window.viewer.command('save-project'); });
  await page.locator('#write-display').check();
  if (process.env.RNL_REVIEW_DIR && test.info().project.name === 'chromium') {
    await page.screenshot({ path: path.join(process.env.RNL_REVIEW_DIR, 'display-settings-save.png') });
  }
  await page.getByRole('dialog').getByRole('button', { name: 'Download', exact: true }).click();
  const saved = path.join(test.info().outputDir, 'project.zip');
  await (await download).saveAs(saved);
  const manifest = JSON.parse(execFileSync('python', ['-c', 'import sys,zipfile; print(zipfile.ZipFile(sys.argv[1]).read("rnl-project.json").decode())', saved]).toString());
  expect(manifest.display).toMatchObject({ background: 'dark', currents: true, usages: true, grid: 'lines', colors: { component: '#123456' } });
  expect(manifest.display.view.center).toHaveLength(2);
  expect(manifest.display.view.scale).toBeGreaterThan(0);

  // a new page, with the user's own settings: light, no currents
  await page.goto(`${process.env.RNL_WEB_URL}/index.html`);
  await page.evaluate(() => { localStorage.clear(); });
  await page.reload();
  const chooser = page.waitForEvent('filechooser');
  await page.getByRole('menuitem', { name: 'File', exact: true }).click();
  await page.locator('[data-id="open-project"]').click();
  await (await chooser).setFiles(saved);
  await expect(dialog(page)).toBeVisible();
  await expect(dialog(page).locator('.display-setting')).toHaveText([
    'Background: dark', 'Color scheme: 1 of its colors', 'Usages: shown', 'Current directions: shown', 'Grid: lines', 'View: the part of the level it shows',
  ]);
  if (process.env.RNL_REVIEW_DIR && test.info().project.name === 'chromium') {
    await page.screenshot({ path: path.join(process.env.RNL_REVIEW_DIR, 'display-settings-dialog.png') });
  }
  // keep all but the colours
  await dialog(page).locator('[data-setting="colors"]').uncheck();
  await dialog(page).getByRole('button', { name: 'Apply Selected', exact: true }).click();
  await ready(page);
  const shown = await state(page);
  expect(shown).toMatchObject({ background: 'dark', currents: true, grid: 'lines', component: '#2275E2' });
  // the page shows them; the settings keep the user's
  expect(shown.kept).toEqual({ background: 'light', currents: false, colors: null });
  const view = await page.evaluate(async () => (await import('./drawing.js')).viewCentre());
  expect(view.scale).toBeCloseTo(manifest.display.view.scale, 2);
  expect(view.center[0]).toBeCloseTo(manifest.display.view.center[0], 1);
  expect(view.center[1]).toBeCloseTo(manifest.display.view.center[1], 1);
});

test("Don't Apply keeps the user's own display; a manifest's unknown setting is told, and the view it names is shown", async ({ page }) => {
  const base = 'https://raw.githubusercontent.com/someone/design/0123456789abcdef0123456789abcdef01234567/';
  const text = fs.readFileSync(path.join(import.meta.dirname, '..', 'inputs', 'usage-current.sp'));
  const manifest = (display) => ({ format: 1, id: 'demo', base, entry: 'top.sp', level: '(top)', display,
    files: [{ path: 'top.sp', sha256: createHash('sha256').update(text).digest('hex'), size: text.length }] });
  let served = manifest({ background: 'dark', currents: true });
  await page.route('https://manifests.test/demo.json', (route) => route.fulfill({ status: 200, contentType: 'application/json',
    headers: { 'Access-Control-Allow-Origin': '*' }, body: JSON.stringify(served) }));
  await page.route(`${base}**`, (route) => route.fulfill({ status: 200, contentType: 'text/plain', headers: { 'Access-Control-Allow-Origin': '*' }, body: text }));
  const address = `${process.env.RNL_WEB_URL}/index.html?manifest=${encodeURIComponent('https://manifests.test/demo.json')}`;
  await page.goto(address);
  await expect(dialog(page)).toBeVisible();
  await dialog(page).getByRole('button', { name: "Don't Apply", exact: true }).click();
  expect(await state(page)).toMatchObject({ background: 'light', currents: false });

  served = manifest({ view: { center: [20, 0], scale: 3 }, sparkle: true });
  await page.goto(address);
  await expect(dialog(page).locator('.display-setting')).toHaveText(['View: the part of the level it shows']);
  await dialog(page).getByRole('button', { name: 'Apply Selected', exact: true }).click();
  await expect(page.locator('#notice')).toContainText('sparkle');
  const view = await page.evaluate(async () => (await import('./drawing.js')).viewCentre());
  expect(view.scale).toBeCloseTo(3, 6);
  expect(view.center[0]).toBeCloseTo(20, 6);
  expect(view.center[1]).toBeCloseTo(0, 6);
});

test("a manifest's settings hold while a file of it is shown, go with another file and come back unasked", async ({ page }) => {
  const base = 'https://raw.githubusercontent.com/someone/design/0123456789abcdef0123456789abcdef01234567/';
  const files = { 'top.sp': fs.readFileSync(path.join(import.meta.dirname, '..', 'inputs', 'usage-current.sp')), 'other.sp': Buffer.from('* other\nR1 a 0 1k\n.end\n') };
  const sha = (b) => createHash('sha256').update(b).digest('hex');
  const manifest = { format: 1, id: 'demo', base, entry: 'top.sp', level: '(top)', display: { background: 'dark', currents: true },
    files: Object.entries(files).map(([p, b]) => ({ path: p, sha256: sha(b), size: b.length })) };
  await page.route('https://manifests.test/demo.json', (route) => route.fulfill({ status: 200, contentType: 'application/json',
    headers: { 'Access-Control-Allow-Origin': '*' }, body: JSON.stringify(manifest) }));
  await page.route(`${base}**`, (route) => route.fulfill({ status: 200, contentType: 'text/plain', headers: { 'Access-Control-Allow-Origin': '*' },
    body: files[new URL(route.request().url()).pathname.split('/').pop()] }));
  await page.goto(`${process.env.RNL_WEB_URL}/index.html?manifest=${encodeURIComponent('https://manifests.test/demo.json')}`);
  await dialog(page).getByRole('button', { name: 'Apply Selected', exact: true }).click();
  expect(await state(page)).toMatchObject({ background: 'dark', currents: true });
  const open = async (file) => {
    await page.locator('#tab-files').click();
    await page.locator('.lib-group[data-group="external"] .pool-open', { hasText: new RegExp(`^${file.replace('.', '\\.')}$`) }).click();
    await ready(page);
  };
  // another file of the manifest: still its settings
  await open('other.sp');
  expect(await page.evaluate(() => window.viewer.doc.path)).toBe('/demo/other.sp');
  expect(await state(page)).toMatchObject({ background: 'dark', currents: true });
  // a file of the computer: the user's own
  await page.locator('#file-input').setInputFiles({ name: 'local.sp', mimeType: 'text/plain', buffer: Buffer.from('* local\nR1 a 0 1k\n.end\n') });
  await expect.poll(() => page.evaluate(() => window.viewer.doc?.path)).toBe('/local.sp');
  await ready(page);
  expect(await state(page)).toMatchObject({ background: 'light', currents: false });
  // the manifest's entry again: its settings, without asking
  await open('top.sp');
  await expect(dialog(page)).toHaveCount(0);
  expect(await state(page)).toMatchObject({ background: 'dark', currents: true, kept: { background: 'light', currents: false } });
});
