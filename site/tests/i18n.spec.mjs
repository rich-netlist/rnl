import { test, expect } from '../../tests/viewer/node_modules/@playwright/test/index.mjs';
import { readFileSync, mkdirSync } from 'node:fs';
import { join, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

for (const language of ['zh', 'en']) for (const theme of ['light', 'dark']) for (const width of [1440, 390]) {
  test(`language menu: ${language} ${theme} ${width}px`, async ({ page }, info) => {
    await page.setViewportSize({ width, height: 720 });
    await page.addInitScript(theme => localStorage.setItem('rnl-intro-theme', theme), theme);
    await page.goto(`/${language}/`);
    const menu = page.locator('.language-menu');
    const summary = menu.locator('summary');
    await expect(menu.locator('[data-lang-link]')).toHaveCount(2);
    await expect(menu.locator('[aria-current]')).toHaveAttribute('data-language', language);
    await summary.focus();
    await page.keyboard.press('Enter');
    await expect(menu).toHaveAttribute('open', '');
    await expect(menu.getByRole('link', { name: /English/ })).toBeVisible();
    await page.keyboard.press('Tab');
    await page.keyboard.press('Escape');
    await expect(menu).not.toHaveAttribute('open');
    await expect(summary).toBeFocused();
    await summary.click();
    await page.locator('.hero h1').click();
    await expect(menu).not.toHaveAttribute('open');
    await summary.click();
    expect(await menu.locator('.language-options').evaluate(el => {
      const box = el.getBoundingClientRect();
      return box.left >= 0 && box.right <= innerWidth && box.bottom <= innerHeight;
    })).toBe(true);
    if (process.env.RNL_I18N_SCREENSHOTS && info.project.name === 'chromium') {
      mkdirSync(process.env.RNL_I18N_SCREENSHOTS, { recursive: true });
      await page.evaluate(() => document.fonts.ready);
      await page.screenshot({ path: join(process.env.RNL_I18N_SCREENSHOTS, `${language}-${theme}-${width}.png`),
        clip: { x: 0, y: 0, width, height: 310 }, animations: 'disabled' });
    }
  });
}

test('standard and comparison menus preserve the document, query and section', async ({ page }) => {
  for (const file of ['index.html', 'compare-0.2.html']) {
    await page.goto(`/zh/spec/0.3/${file}?review=1#9.4`);
    await page.locator('.language-menu > summary').click();
    await page.locator('[data-language="en"]').click();
    const path = file === 'index.html' ? '' : file;
    await expect(page).toHaveURL(new RegExp(`/en/spec/0.3/${path}\\?review=1#9.4$`));
    await page.locator('.language-menu > summary').click();
    await page.locator('[data-language="zh"]').click();
    await expect(page).toHaveURL(new RegExp(`/zh/spec/0.3/${path}\\?review=1#9.4$`));
  }
});

test('native language picker works without JavaScript', async ({ browser, baseURL }) => {
  const context = await browser.newContext({ javaScriptEnabled: false });
  try {
    const page = await context.newPage();
    await page.goto(`${baseURL}/zh/agents.html`);
    await page.locator('.language-menu > summary').click();
    await page.locator('[data-language="en"]').click();
    await expect(page).toHaveURL(/\/en\/agents.html$/);
  } finally { await context.close(); }
});

/* The site built with a third locale (test-setup.mjs), served at /i18n-fixture/. */
async function serveFixture(page) {
  const fixture = resolve(process.env.RNL_I18N_FIXTURE);
  await page.route('**/i18n-fixture/**', async route => {
    let path = decodeURIComponent(new URL(route.request().url()).pathname.split('/i18n-fixture/')[1]);
    if (!path || path.endsWith('/')) path += 'index.html';
    const base = path.startsWith('gallery/') ? fileURLToPath(new URL('../dist/', import.meta.url)) : fixture;
    const file = resolve(base, path);
    if (!file.startsWith(resolve(base) + sep)) return route.abort();
    const contentType = file.endsWith('.css') ? 'text/css' : file.endsWith('.js') ? 'text/javascript'
      : file.endsWith('.svg') ? 'image/svg+xml' : file.endsWith('.png') ? 'image/png' : 'text/html';
    try {
      let body = readFileSync(file);
      if (file.endsWith('standard-0.3.js')) {
        const data = JSON.parse(body.toString().match(/^RNLGallery.group\((.*)\);\s*$/s)[1]);
        data.packages[0].summary = { en: 'English catalog fallback.' };
        data.packages[1].summary = { en: 'English catalog source.', ja: '日本語のカタログ説明。' };
        body = `RNLGallery.group(${JSON.stringify(data)});`;
      }
      await route.fulfill({ contentType, body });
    }
    catch { await route.fulfill({ status: 404, body: 'Not found' }); }
  });
  // the gallery page's entries, from rnl-gallery's Pages: none, as the notice is what is looked at here
  await page.route('https://rich-netlist.github.io/rnl-gallery/**', route => route.fulfill({
    headers: { 'access-control-allow-origin': '*', 'content-type': 'application/json' },
    body: JSON.stringify({ format: 1, generated: '2026-10-25T00:00:00Z', entries: [] }) }));
}

test('third locale inherits English content and switches without changing source templates', async ({ page }) => {
  await serveFixture(page);
  await page.addInitScript(() => Object.defineProperty(navigator, 'languages', { value: ['fr-FR', 'ja-JP'] }));
  await page.goto('/i18n-fixture/?review=1#status');
  await expect(page).toHaveURL(/\/i18n-fixture\/ja\/\?review=1#status$/);
  await expect(page.locator('html')).toHaveAttribute('lang', 'ja');
  await expect(page.locator('.hero')).toContainText('A netlist you can simulate');
  await expect(page.locator('nav.pages a').first()).toHaveText('ホーム');
  await expect(page.locator('#status .tl')).toContainText('Release notes');
  await page.locator('.language-menu > summary').click();
  await expect(page.locator('[data-lang-link]')).toHaveCount(3);
  await page.locator('[data-language="zh"]').click();
  await expect(page).toHaveURL(/\/i18n-fixture\/zh\/\?review=1#status$/);
  await page.goto('/i18n-fixture/');
  await expect(page).toHaveURL(/\/i18n-fixture\/zh\/$/);
  await page.goto('/i18n-fixture/ja/packages.html');
  await expect(page.locator('.gl-sum').filter({ hasText: 'English catalog fallback.' })).toHaveCount(1);
  await expect(page.locator('.gl-sum').filter({ hasText: '日本語のカタログ説明。' })).toHaveCount(1);
});

for (const theme of ['light', 'dark']) for (const width of [1440, 390]) {
  test(`a page not wholly translated says so below its hero: ${theme} ${width}px`, async ({ page }, info) => {
    await serveFixture(page);
    await page.setViewportSize({ width, height: 900 });
    await page.addInitScript(theme => localStorage.setItem('rnl-intro-theme', theme), theme);
    for (const [file, partial] of [['index.html', false], ['gallery.html', true]]) {
      await page.goto(`/i18n-fixture/ja/${file}`);
      const notice = page.locator('.translation-notice');
      await expect(notice).toHaveCount(1);
      await expect(notice).toBeVisible();
      await expect(notice).toHaveAttribute('lang', 'ja');
      await expect(notice).toHaveClass(partial ? /\bis-partial\b/ : /^wrap translation-notice$/);
      await expect(notice).toContainText(partial ? 'Parts of this page are not yet available in 日本語' : 'This page is not yet available in 日本語');
      await expect(notice.locator('a')).toHaveAttribute('href', /i18n\/README\.md$/);
      // below the hero, before the page's first part, and within the width of the page
      const box = await notice.boundingBox();
      const hero = await page.locator('.hero').boundingBox();
      expect(box.y).toBeGreaterThanOrEqual(hero.y + hero.height - 1);
      expect(box.x).toBeGreaterThanOrEqual(0);
      expect(box.x + box.width).toBeLessThanOrEqual(width);
      if (partial) await expect(page.locator('.hero h1')).toContainText('（訳）');
      else await expect(page.locator('.hero')).toContainText('A netlist you can simulate');
      if (process.env.RNL_I18N_SCREENSHOTS && info.project.name === 'chromium') {
        mkdirSync(process.env.RNL_I18N_SCREENSHOTS, { recursive: true });
        await page.evaluate(() => document.fonts.ready);
        const y = Math.max(0, box.y - 160);
        await page.screenshot({ path: join(process.env.RNL_I18N_SCREENSHOTS, `notice-${partial ? 'partial' : 'english'}-${theme}-${width}.png`),
          clip: { x: 0, y, width, height: box.y - y + box.height + 120 }, animations: 'disabled' });
      }
    }
    for (const language of ['zh', 'en']) {
      await page.goto(`/${language}/`);
      await expect(page.locator('.translation-notice')).toHaveCount(0);
    }
  });
}

test('unsupported browser languages fall back to English, including with blocked storage', async ({ page }) => {
  await page.addInitScript(() => {
    Object.defineProperty(navigator, 'languages', { value: ['fr-FR', 'de-DE'] });
    Object.defineProperty(window, 'localStorage', { get() { throw new Error('Storage blocked'); } });
  });
  await page.goto('/?review=1#status');
  await expect(page).toHaveURL(/\/en\/\?review=1#status$/);
});
