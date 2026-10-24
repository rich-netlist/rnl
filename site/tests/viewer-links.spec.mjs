import { test, expect } from '../../tests/viewer/node_modules/@playwright/test/index.mjs';
import { readFileSync } from 'node:fs';

// Every link on the site that opens the web viewer opens it in a new tab, so that the page that sent the reader stays.
const pages = ['index.html', 'language.html', 'packages.html', 'gallery.html', 'tools.html', 'agents.html',
  'rnl-intro-standalone.html', 'rnl-language-standalone.html', 'rnl-tools-standalone.html'];
const INDEX = readFileSync(new URL('./fixtures/circuits/index.json', import.meta.url), 'utf8');

test.beforeEach(async ({ page }) => {
  await page.route('https://rich-netlist.github.io/rnl-packages/**', route => route.abort());
  await page.route('https://rich-netlist.github.io/rnl-gallery/**', route => {
    const path = new URL(route.request().url()).pathname;
    if (path.endsWith('/index.json')) return route.fulfill({ headers: { 'access-control-allow-origin': '*' }, contentType: 'application/json', body: INDEX });
    return route.fulfill({ status: 404, body: '' });
  });
});

for (const language of ['zh', 'en']) {
  test(`every link to the web viewer opens a new tab: ${language}`, async ({ page }) => {
    let seen = 0;
    for (const file of pages) {
      await page.goto(`/${language}/${file}`);
      if (file === 'gallery.html') await expect(page.locator('.cg-card')).toHaveCount(3);
      const links = await page.locator('a[href*="viewer/"]').evaluateAll(as => as.map(a => ({ href: a.getAttribute('href'), target: a.target, rel: a.rel })));
      for (const link of links) {
        expect(link.target, `${file}: ${link.href}`).toBe('_blank');
        expect(link.rel.split(/\s+/), `${file}: ${link.href}`).toContain('noopener');
      }
      seen += links.length;
      if (!file.startsWith('rnl-')) expect(links.length, `${file} has links to the viewer`).toBeGreaterThan(0);
    }
    expect(seen).toBeGreaterThan(15);
  });
}

test('the homepage and the tour keep their viewer links in a new tab when a script changes them', async ({ page }) => {
  await page.goto('/en/index.html');
  const link = page.locator('#jr-open');
  await expect(link).toHaveAttribute('target', '_blank');
  await page.locator('#jr-next').click();
  await expect(link).toHaveAttribute('target', '_blank');
  await page.goto('/en/language.html');
  await expect(page.locator('#file-open')).toHaveAttribute('target', '_blank');
});
