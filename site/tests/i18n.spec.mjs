import { test, expect } from '../../tests/viewer/node_modules/@playwright/test/index.mjs';
import { readFileSync, mkdirSync } from 'node:fs';
import { join, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

// English and the languages the registry publishes, as the language menu offers them
const registry = JSON.parse(readFileSync(new URL('../../i18n/languages.json', import.meta.url), 'utf8'));
const offered = registry.languages.filter(entry => entry.code === registry.default || entry.status === 'published').length;
// whether a published language matches a browser language
const offers = tag => registry.languages.some(entry => entry.status === 'published'
  && (entry.matches || [entry.code]).some(m => tag.toLowerCase().startsWith(m)));

for (const language of ['zh', 'en']) for (const theme of ['light', 'dark']) for (const width of [1440, 390]) {
  test(`language menu: ${language} ${theme} ${width}px`, async ({ page }, info) => {
    await page.setViewportSize({ width, height: 720 });
    await page.addInitScript(theme => localStorage.setItem('rnl-intro-theme', theme), theme);
    await page.goto(`/${language}/`);
    const menu = page.locator('.language-menu');
    const summary = menu.locator('summary');
    await expect(menu.locator('[data-lang-link]')).toHaveCount(offered);
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
    // an outside click, at the left edge of the title, which the open menu does not cover however long it is
    await page.locator('.hero h1').click({ position: { x: 4, y: 4 } });
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
  // a language the site does not offer first, then the fixture's Japanese
  expect(offers('pt-BR')).toBe(false);
  await page.addInitScript(() => Object.defineProperty(navigator, 'languages', { value: ['pt-BR', 'ja-JP'] }));
  await page.goto('/i18n-fixture/?review=1#status');
  await expect(page).toHaveURL(/\/i18n-fixture\/ja\/\?review=1#status$/);
  await expect(page.locator('html')).toHaveAttribute('lang', 'ja');
  await expect(page.locator('.hero')).toContainText('A netlist you can simulate');
  await expect(page.locator('nav.pages a').first()).toHaveText('ホーム');
  // translated by AI: a notice under the top bar, in English, with a link to report an error on this page
  await expect(page.locator('.ai-notice')).toHaveAttribute('lang', 'en');
  await expect(page.locator('.ai-notice')).toContainText('Translated from English by AI; it may contain errors.');
  await expect(page.locator('.ai-notice a').first()).toHaveAttribute('href', /^https:\/\/github\.com\/rich-netlist\/rnl\/issues\/new\?labels=translation&title=Translation%20%28Japanese%29%3A%20index\.html&/);
  // and, as the language has a glossary, that the English terms are normative, with a link to it (decision 0016)
  await expect(page.locator('.ai-notice a').nth(1)).toHaveAttribute('href', 'glossary.html');
  await expect(page.locator('#status .tl')).toContainText('Release notes');
  await page.locator('.language-menu > summary').click();
  // the fixture's Japanese in place of the repository's own, if that is published
  const japanese = registry.languages.some(entry => entry.code === 'ja' && entry.status === 'published');
  await expect(page.locator('[data-lang-link]')).toHaveCount(offered + (japanese ? 0 : 1));
  await page.locator('[data-language="zh"]').click();
  await expect(page).toHaveURL(/\/i18n-fixture\/zh\/\?review=1#status$/);
  await expect(page.locator('.ai-notice')).toHaveCount(0);
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
      await expect(notice).toHaveAttribute('lang', 'en'); // as every notice of the system (decision 0015)
      await expect(notice).toHaveClass(partial ? /\bis-partial\b/ : /^wrap translation-notice$/);
      await expect(notice).toContainText(partial ? 'Parts of this page are not yet available in Japanese' : 'This page is not yet available in Japanese');
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
        // the notice with the end of the hero above it and the start of the page below, where a tall hero
        // puts it out of the viewport: a clip of the whole page, in its own coordinates
        const top = await notice.evaluate(el => el.getBoundingClientRect().top + scrollY);
        const y = Math.max(0, top - 160);
        await page.screenshot({ path: join(process.env.RNL_I18N_SCREENSHOTS, `notice-${partial ? 'partial' : 'english'}-${theme}-${width}.png`),
          fullPage: true, clip: { x: 0, y, width, height: top - y + box.height + 120 }, animations: 'disabled' });
      }
    }
    // English, the source, has none; how complete the repository's Chinese is, which a release in English
    // alone changes until it is translated, is not this test's concern (decision 0015)
    await page.goto('/en/');
    await expect(page.locator('.translation-notice')).toHaveCount(0);
  });
}

test('unsupported browser languages fall back to English, including with blocked storage', async ({ page }) => {
  expect(offers('pt-BR') || offers('it-IT')).toBe(false);
  await page.addInitScript(() => {
    Object.defineProperty(navigator, 'languages', { value: ['pt-BR', 'it-IT'] });
    Object.defineProperty(window, 'localStorage', { get() { throw new Error('Storage blocked'); } });
  });
  await page.goto('/?review=1#status');
  await expect(page).toHaveURL(/\/en\/\?review=1#status$/);
});

// RNL written with the Chinese alias package is shown on the pages of Simplified Chinese only: the other
// languages' pages have no Chinese writing style, no Chinese example and no Chinese sample.
const chinese = registry.languages.find(entry => (entry.html_lang || entry.code) === 'zh-CN').code;
const han = /[\u3400-\u9fff\uf900-\ufaff]/;
for (const language of registry.languages.filter(entry => entry.code === registry.default || entry.status === 'published').map(entry => entry.code)) {
  const examples = language === chinese;
  test(`Chinese examples are ${examples ? '' : 'not '}on the ${language} pages`, async ({ page }) => {
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    // content marked as Chinese, apart from the language menu's name of the language
    const marked = page.locator('body [lang="zh-CN"]:not([data-lang-link])');
    // the code a writing style shows has Chinese only in the Chinese style
    async function styles(seg, code) {
      const buttons = page.locator(`${seg} button`);
      await expect(page.locator(`${seg} [data-style="zh"]`)).toHaveCount(examples ? 1 : 0);
      for (let i = 0; i < await buttons.count(); i++) {
        await buttons.nth(i).click();
        const zh = await buttons.nth(i).getAttribute('data-style') === 'zh';
        expect(han.test(await page.locator(code).textContent()), `${seg} ${i}`).toBe(zh);
      }
    }

    await page.goto(`/${language}/index.html`);
    expect(await marked.count() > 0).toBe(examples);
    await styles('#jr-style', '#jr-code');

    await page.goto(`/${language}/language.html`);
    expect(await marked.count() > 0).toBe(examples);
    await expect(page.locator('.ways .way')).toHaveCount(examples ? 3 : 2);
    expect(han.test((await page.locator('.ways code').allTextContents()).join(' '))).toBe(examples);
    await styles('#lab-style', '#lab-line');
    await styles('#file-style', '#file-code');
    for (const keyword of ['@AS', '@ALIAS']) {
      await page.locator('#fams .kw', { hasText: new RegExp(`^${keyword}$`) }).click();
      await expect(page.locator('#kw-card h4')).toHaveText(keyword);
      expect(await page.locator('#kw-card [lang="zh-CN"]').count() > 0).toBe(examples && keyword === '@AS');
      // that a header keyword has no alias is said only where the others show theirs
      await expect(page.locator('#kw-card .chip-mute')).toHaveCount(examples && keyword === '@ALIAS' ? 1 : 0);
      expect(han.test(await page.locator('#kw-card').evaluate(card => [...card.querySelectorAll('.minicode, code')].map(c => c.textContent).join(' ')))).toBe(examples);
    }

    await page.goto(`/${language}/tools.html`);
    await expect(page.locator('a[href*="samples/cases/15-chinese.sp"]')).toHaveCount(examples ? 1 : 0);
    expect(errors).toEqual([]);
  });
}
