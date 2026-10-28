import { test, expect } from '../../tests/viewer/node_modules/@playwright/test/index.mjs';
import { mkdirSync } from 'node:fs';
import { join } from 'node:path';

// On a narrow screen the button at the top left opens the menu of the pages of the site, the same links as the tabs at
// the top of the desktop page; the contents of the page have a button of their own.
const pages = ['index.html', 'language.html', 'packages.html', 'gallery.html', 'tools.html', 'agents.html', 'spec/0.3/'];
const shots = process.env.RNL_MOBILE_NAV_SCREENSHOTS;

test.beforeEach(async ({ page }) => {
  await page.route('https://rich-netlist.github.io/rnl-packages/**', route => route.abort());
  await page.route('https://rich-netlist.github.io/rnl-gallery/**', route => route.abort());
  await page.addInitScript(() => localStorage.setItem('rnl-intro-theme', 'light'));
});

for (const language of ['zh', 'en']) {
  for (const file of pages) {
    test(`narrow screens: the menu holds the pages of the site: ${language} ${file}`, async ({ page, request }, info) => {
      const path = `/${language}/${file}`;
      // the pages of the standard are built by the Pages job; without them (a local run) there is nothing to look at
      test.skip((await request.get(path)).status() !== 200, `${path} is not built`);

      // the tabs of the desktop page
      await page.setViewportSize({ width: 1440, height: 900 });
      await page.goto(path);
      await expect(page.locator('#menu-btn')).toBeHidden();
      const desktop = await page.locator('nav.pages a').evaluateAll(as => as.map(a => ({ text: a.textContent.trim(), href: a.getAttribute('href') })));
      expect(desktop).toHaveLength(8);

      await page.setViewportSize({ width: 390, height: 844 });
      await page.reload();
      const menu = page.locator('nav.pages'), button = page.locator('#menu-btn');
      await expect(button).toBeVisible();
      await expect(button).toHaveAttribute('aria-expanded', 'false');
      await expect(menu).toBeHidden();
      await button.click();
      await expect(button).toHaveAttribute('aria-expanded', 'true');
      await expect(menu).toBeVisible();
      // all the entries of the first level, in the order of the tabs, the page we are on marked
      const entries = await menu.locator('a').evaluateAll(as => as.map(a => ({ text: a.textContent.trim(), href: a.getAttribute('href'), current: a.getAttribute('aria-current'), on: a.classList.contains('is-on') })));
      expect(entries.map(e => [e.text, e.href])).toEqual(desktop.map(e => [e.text, e.href]));
      expect(entries.filter(e => e.current === 'page' && e.on)).toHaveLength(1);
      const box = await menu.boundingBox();
      expect(box.x).toBeGreaterThanOrEqual(0);
      expect(box.x + box.width).toBeLessThanOrEqual(390);
      expect(box.y + box.height).toBeLessThanOrEqual(844);
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
      if (shots && info.project.name === 'chromium' && file === 'index.html') {
        mkdirSync(shots, { recursive: true });
        await page.screenshot({ path: join(shots, `${language}-menu-index.png`) });
      }

      // the contents of the page have their own button, and the two panels are not open together
      const toc = page.locator('#toc'), contents = page.locator('#toc-btn');
      await expect(contents).toBeVisible();
      await expect(contents).toHaveAttribute('aria-label', language === 'zh' ? '本页目录' : 'On this page');
      await contents.click();
      await expect(toc).toBeVisible();
      await expect(menu).toBeHidden();
      await expect(button).toHaveAttribute('aria-expanded', 'false');
      if (shots && info.project.name === 'chromium' && ['language.html', 'spec/0.3/'].includes(file)) {
        mkdirSync(shots, { recursive: true });
        await page.screenshot({ path: join(shots, `${language}-contents-${file.replace(/\W+/g, '-').replace(/-$/, '')}.png`) });
      }
      await button.click();
      await expect(menu).toBeVisible();
      await expect(toc).toBeHidden();

      // Escape closes it; a link takes us to its page
      await page.keyboard.press('Escape');
      await expect(menu).toBeHidden();
      await button.click();
      // (on the pages of the standard the links lead up out of spec/0.3/)
      const other = entries.find(e => !e.on && /(^|\/)(tools|packages)\.html$/.test(e.href));
      await menu.locator(`a[href="${other.href}"]`).click();
      await expect(page).toHaveURL(new RegExp(`/${language}/${other.href.match(/(tools|packages)\.html$/)[0].replace('.', '\\.')}$`));
      await expect(page.locator('nav.pages')).toBeHidden();
    });
  }
}

test('wide screens keep the tabs: no menu button, the links in the top bar', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto('/en/index.html');
  await expect(page.locator('#menu-btn')).toBeHidden();
  await expect(page.locator('nav.pages')).toBeVisible();
  await expect(page.locator('#toc-btn')).toBeHidden();
});
