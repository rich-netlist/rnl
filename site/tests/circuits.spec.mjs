import { test, expect } from '../../tests/viewer/node_modules/@playwright/test/index.mjs';
import { readFileSync, mkdirSync } from 'node:fs';
import { join } from 'node:path';

// The gallery page reads rnl-gallery's GitHub Pages: here those requests are answered from fixtures.
const DATA = 'https://rich-netlist.github.io/rnl-gallery/';
const fixture = name => readFileSync(new URL(`./fixtures/circuits/${name}`, import.meta.url));
const cors = { 'access-control-allow-origin': '*' };
const INDEX = fixture('index.json').toString('utf8');
const thumbs = { 'thumbs/ota-single.11111111.png': 'thumb-a.png', 'thumbs/pll-multi.22222222.png': 'thumb-b.png' };

async function serve(page, { index = INDEX, status = 200 } = {}) {
  await page.route(`${DATA}**`, route => {
    const path = new URL(route.request().url()).pathname.replace('/rnl-gallery/', '');
    if (path === 'index.json') {
      if (status !== 200) return route.fulfill({ status, headers: cors, body: 'unavailable' });
      return route.fulfill({ headers: { ...cors, 'content-type': 'application/json' }, body: index });
    }
    if (thumbs[path]) return route.fulfill({ headers: cors, contentType: 'image/png', body: fixture(thumbs[path]) });
    return route.fulfill({ status: 404, headers: cors, body: '' });
  });
}

test.beforeEach(async ({ page }) => {
  await page.route('https://rich-netlist.github.io/rnl-packages/**', route => route.abort());
  await page.addInitScript(() => localStorage.setItem('rnl-intro-theme', 'light'));
});

for (const language of ['en', 'zh']) {
  test(`gallery: entries, their addresses and what is shown as text only: ${language}`, async ({ page }) => {
    await serve(page);
    await page.goto(`/${language}/gallery.html`);
    const cards = page.locator('.cg-card');
    await expect(cards).toHaveCount(3);
    await expect(page.locator('#tb-count')).toHaveText('3');

    // one file: opens in the viewer by ?url=, with its level
    const single = page.locator('.cg-card[data-id="ota-single"]');
    await expect(single.locator('.cg-title')).toHaveText(language === 'zh' ? '五管 OTA' : 'Five-transistor OTA');
    const href = await single.locator('a.cg-fig').getAttribute('href');
    expect(href).toBe(`../viewer/?url=${encodeURIComponent('https://raw.githubusercontent.com/example/ota/' + 'a'.repeat(40) + '/ota.sp')}&level=ota`);
    await expect(single.locator('.chip')).toHaveText('CC0-1.0');
    await expect(single.locator('a.cg-src')).toHaveAttribute('href', `https://github.com/example/ota/tree/${'a'.repeat(40)}`);
    await expect(single.locator('a.cg-src')).toHaveText('example/ota@aaaaaaa');
    await expect.poll(() => single.locator('img').evaluate(img => img.complete && img.naturalWidth)).toBe(320);

    // several files wait for the viewer's manifest: thumbnail, source and a note, no link into the viewer
    const multi = page.locator('.cg-card[data-id="pll-multi"]');
    await expect(multi.locator('a.cg-fig, .cg-title a')).toHaveCount(0);
    await expect(multi.locator('.cg-note')).toBeVisible();
    await expect(multi.locator('.cg-files')).toContainText('83');
    await expect(multi.locator('.cg-title')).toHaveText('Phase-locked loop');

    // a missing thumbnail falls back to the name; text from the index is text, never markup
    const broken = page.locator('.cg-card[data-id="broken-thumb"]');
    await expect(broken.locator('.cg-nothumb')).toBeVisible();
    if (language === 'en') await expect(broken.locator('.cg-title')).toContainText('<img src=x');
    else await expect(broken.locator('.cg-title')).toHaveText('缩略图缺失的条目');
    await expect(broken.locator('.cg-by')).toHaveText('<script>window.__xss=2</script>');
    await expect(broken.locator('img, script, b')).toHaveCount(0);
    expect(await page.evaluate(() => window.__xss)).toBeUndefined();
  });
}

test('gallery: search by title, description and author keeps ?q= and the language switch', async ({ page }) => {
  await serve(page);
  await page.goto('/en/gallery.html');
  await expect(page.locator('.cg-card')).toHaveCount(3);
  await page.locator('#cg-q').fill('grace');
  await expect(page.locator('.cg-card')).toHaveCount(1);
  await expect(page.locator('.cg-card')).toHaveAttribute('data-id', 'pll-multi');
  await expect(page).toHaveURL(/\/en\/gallery\.html\?q=grace$/);
  await expect(page.locator('#cg-counts')).toContainText('1');
  await expect(page.locator('[data-lang-link]').first()).toHaveAttribute('href', /gallery\.html\?q=grace/);
  await page.locator('#cg-q').fill('mit apache');
  await expect(page.locator('.cg-card')).toHaveCount(0);
  await expect(page.locator('#cg-main')).toContainText('Nothing found');
  await page.goto('/zh/gallery.html?q=五管');
  await expect(page.locator('#cg-q')).toHaveValue('五管');
  await expect(page.locator('.cg-card')).toHaveCount(1);
  await expect(page.locator('.cg-card')).toHaveAttribute('data-id', 'ota-single');
});

test('gallery: when the data cannot be read', async ({ page, context }) => {
  // nothing saved: a message and the repository
  await serve(page, { status: 503 });
  await page.goto('/en/gallery.html');
  await expect(page.locator('#cg-main')).toContainText('cannot be read right now');
  await expect(page.locator('#cg-main a')).toHaveAttribute('href', 'https://github.com/rich-netlist/rnl-gallery/tree/main/entries');
  await expect(page.locator('.cg-card')).toHaveCount(0);
  // what was read before is shown, as such
  await page.evaluate(index => localStorage.setItem('rnl-gallery-index', index), INDEX);
  await page.reload();
  await expect(page.locator('.cg-card')).toHaveCount(3);
  await expect(page.locator('.cg-warn')).toContainText('last time');
});

test('gallery: a newer format of the data asks for a reload, an empty list says so', async ({ page }) => {
  await serve(page, { index: JSON.stringify({ format: 2, entries: [] }) });
  await page.goto('/en/gallery.html');
  await expect(page.locator('#cg-main')).toContainText('newer than this page');
  await page.unroute(`${DATA}**`);
  await serve(page, { index: JSON.stringify({ format: 1, entries: [] }) });
  await page.goto('/en/gallery.html');
  await expect(page.locator('#cg-main')).toContainText('no entries yet');
});

for (const language of ['zh', 'en']) for (const theme of ['light', 'dark']) for (const width of [1440, 390]) {
  test(`gallery: layout, ${language} ${theme} at ${width}px`, async ({ page }, info) => {
    await page.setViewportSize({ width, height: 900 });
    await page.addInitScript(theme => localStorage.setItem('rnl-intro-theme', theme), theme);
    await serve(page);
    await page.goto(`/${language}/gallery.html#gallery`);
    await expect(page.locator('.cg-card')).toHaveCount(3);
    await page.evaluate(() => document.fonts.ready);
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width + 1);
    const boxes = await page.locator('.cg-card').evaluateAll(cards => cards.map(c => { const r = c.getBoundingClientRect(); return { left: r.left, right: r.right }; }));
    for (const box of boxes) { expect(box.left).toBeGreaterThanOrEqual(0); expect(box.right).toBeLessThanOrEqual(width + 1); }
    if (process.env.RNL_CIRCUITS_SCREENSHOTS && info.project.name === 'chromium') {
      mkdirSync(process.env.RNL_CIRCUITS_SCREENSHOTS, { recursive: true });
      await page.mouse.move(width - 2, 0);
      await page.screenshot({ path: join(process.env.RNL_CIRCUITS_SCREENSHOTS, `gallery-${language}-${theme}-${width}.png`) });
    }
  });
}
