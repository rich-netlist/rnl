// File > Export Image (apps/viewer-ui/export-image.js) on the web: the core's SVG of the whole level
// shown, with the usages and current directions the page shows, and a PNG the page makes of it.
import { test, expect } from '@playwright/test';
import fs from 'node:fs';
import path from 'node:path';
import { zhMenu } from '../zh.js';

test.skip(!process.env.RNL_WEB_URL, 'the web site is not built');

const fixture = path.join(import.meta.dirname, '..', 'inputs', 'usage-current.sp');

async function open(page) {
  await page.goto(`${process.env.RNL_WEB_URL}/index.html`);
  await expect(page.locator('#menubar')).toBeVisible();
  await page.locator('#file-input').setInputFiles({ name: 'usage-current.sp', mimeType: 'text/plain', buffer: fs.readFileSync(fixture) });
  await page.waitForFunction(() => window.viewer.doc && !document.body.classList.contains('busy'));
}
/** Run File > Export Image > `command` and give the file downloaded: its name and bytes. */
async function exported(page, command) {
  const download = page.waitForEvent('download');
  await page.evaluate((c) => { window.viewer.command(c); }, command);
  const dialog = page.getByRole('dialog');
  const result = await Promise.race([download, dialog.waitFor().then(async () => { throw new Error(await dialog.innerText()); })]);
  return { name: result.suggestedFilename(), bytes: fs.readFileSync(await result.path()) };
}
const count = (text, pattern) => (text.match(pattern) || []).length;

test('Export SVG draws the whole level with the usages and current directions the page shows, on or off', async ({ page }) => {
  await open(page);
  const shown = await page.evaluate(async () => {
    const { overlayChoices } = await import('./usages.js');
    const c = overlayChoices();
    return { usages: c.usages, currents: c.currents, visible: [...c.each.values()].filter((u) => u.visible).length, all: c.each.size };
  });
  expect(shown).toMatchObject({ usages: true, currents: false });
  expect(shown.visible).toBeGreaterThan(0);
  expect(shown.visible).toBeLessThan(shown.all);

  // usages on, as shown: the visible ones only; currents off
  let svg = await exported(page, 'export-svg');
  expect(svg.name).toBe('usage-current.svg');
  let text = svg.bytes.toString();
  expect(text.startsWith('<?xml')).toBe(true);
  expect(count(text, /class="usage-frame"/g)).toBe(shown.visible);
  expect(text).not.toContain('class="current-arrow"');
  // the paper is opaque
  expect(text).toMatch(/<rect class="backdrop"[^>]*fill="#ffffff"/);

  // usages off, currents on
  await page.evaluate(() => { window.viewer.command('toggle-usages'); window.viewer.command('toggle-currents'); });
  svg = await exported(page, 'export-svg');
  text = svg.bytes.toString();
  expect(text).not.toContain('class="usage-frame"');
  const arrows = await page.evaluate(() => window.viewer.doc.levels[window.viewer.level].currents.length);
  expect(count(text, /class="current-arrow"/g)).toBe(arrows);

  // a subcircuit's level is named after the file and the level; a dark page exports a dark paper
  const cell = await page.evaluate(() => window.viewer.doc.levels.findIndex((l) => l.title === 'cell'));
  await page.evaluate(async (level) => (await import('./subcircuits.js')).enterRoot(level, { here: true }), cell);
  await page.evaluate(() => window.viewer.command('background-dark'));
  svg = await exported(page, 'export-svg');
  expect(svg.name).toBe('usage-current-cell.svg');
  const paper = await page.evaluate(() => getComputedStyle(document.body).getPropertyValue('--drawing-background').trim());
  expect(svg.bytes.toString()).toMatch(new RegExp(`<rect class="backdrop"[^>]*fill="${paper}"`));
});

test('Export PNG draws the SVG at twice its size, not empty', async ({ page }) => {
  await open(page);
  await page.evaluate(() => window.viewer.command('toggle-currents'));
  const svg = (await exported(page, 'export-svg')).bytes.toString();
  const width = Number(/<svg[^>]* width="([\d.]+)"/.exec(svg)[1]), height = Number(/<svg[^>]* height="([\d.]+)"/.exec(svg)[1]);
  const png = await exported(page, 'export-png');
  expect(png.name).toBe('usage-current.png');
  expect(png.bytes.subarray(1, 4).toString()).toBe('PNG');
  // IHDR: width and height, big-endian, after the signature and the chunk's length and type
  expect(png.bytes.readUInt32BE(16)).toBe(Math.round(width * 2));
  expect(png.bytes.readUInt32BE(20)).toBe(Math.round(height * 2));
  // the drawing's ink and the current arrows' colour are in it, on white
  const pixels = await page.evaluate(async (data) => {
    const bitmap = await createImageBitmap(new Blob([Uint8Array.from(atob(data), (c) => c.charCodeAt(0))], { type: 'image/png' }));
    const canvas = Object.assign(document.createElement('canvas'), { width: bitmap.width, height: bitmap.height });
    const context = canvas.getContext('2d');
    context.drawImage(bitmap, 0, 0);
    const { data: rgba } = context.getImageData(0, 0, bitmap.width, bitmap.height);
    let ink = 0, arrow = 0, white = 0;
    for (let k = 0; k < rgba.length; k += 4) {
      const [r, g, b] = [rgba[k], rgba[k + 1], rgba[k + 2]];
      if (r > 245 && g > 245 && b > 245) white++;
      else if (r < 80 && g < 80 && b < 80) ink++;
      else if (r > 180 && g > 50 && g < 110 && b < 60) arrow++;
    }
    return { ink, arrow, white };
  }, png.bytes.toString('base64'));
  expect(pixels.white).toBeGreaterThan(0);
  expect(pixels.ink).toBeGreaterThan(100);
  expect(pixels.arrow).toBeGreaterThan(10);
});

test('the File menu has Export Image with Export SVG and Export PNG, in both languages', async ({ page }) => {
  await open(page);
  for (const [file, image, svg, png, shot] of [
    ['File', 'Export Image', 'Export SVG...', 'Export PNG...', 'export-menu-en.png'],
    [zhMenu('&File'), zhMenu('Export Ima&ge'), zhMenu('Export &SVG...'), zhMenu('Export &PNG...'), 'export-menu-zh.png'],
  ]) {
    if (file !== 'File') await page.evaluate(() => window.viewer.command('language-zh'));
    await page.getByRole('menuitem', { name: file, exact: true }).click();
    const entry = page.locator('#menubar .menu-item.opens', { hasText: image });
    await expect(entry).toBeVisible();
    await entry.hover();
    await expect(page.locator('[data-id="export-svg"]')).toHaveText(svg);
    await expect(page.locator('[data-id="export-png"]')).toHaveText(png);
    if (process.env.RNL_REVIEW_DIR && test.info().project.name === 'chromium') {
      await page.screenshot({ path: path.join(process.env.RNL_REVIEW_DIR, shot) });
    }
    await page.keyboard.press('Escape');
    await page.keyboard.press('Escape');
  }
});
