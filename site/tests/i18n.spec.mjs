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

test('third locale inherits English content and switches without changing source templates', async ({ page }) => {
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

test('unsupported browser languages fall back to English, including with blocked storage', async ({ page }) => {
  await page.addInitScript(() => {
    Object.defineProperty(navigator, 'languages', { value: ['fr-FR', 'de-DE'] });
    Object.defineProperty(window, 'localStorage', { get() { throw new Error('Storage blocked'); } });
  });
  await page.goto('/?review=1#status');
  await expect(page).toHaveURL(/\/en\/\?review=1#status$/);
});
