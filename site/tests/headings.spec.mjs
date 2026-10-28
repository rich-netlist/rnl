import { test, expect } from '../../tests/viewer/node_modules/@playwright/test/index.mjs';

// No heading of the site ends with a full stop, in any language: the headings h1 to h6, the one-line subtitle under the title of
// a page, the titles of cards and of the lists of contents, the tabs, the questions of the FAQ. (A text that is a sentence of the
// body may end with one.) The standard's own chapters (spec/) are not checked here.
const pages = ['index', 'language', 'packages', 'gallery', 'tools', 'agents', 'timeline', 'glossary', 'translation'];
const TITLES = [
  'h1', 'h2', 'h3', 'h4', 'h5', 'h6', '[role=heading]', '[role=tab]', 'p.thesis', '.sheet-no', '.eyebrow',
  '#toc a', '#toc .toc-title', 'summary', 'th', 'dt', 'legend', 'figcaption', 'caption',
  '.tour-map b', '.eco b', '.ph-title', '.vs-head', '.rv-own b', '.hub-node h4', '.tl h3', '.tl h4', '.faq h3',
].join(', ');

for (const language of ['en', 'zh']) {
  for (const name of pages) {
    test(`no heading ends with a full stop: ${language}/${name}.html`, async ({ page }) => {
      await page.route(/^https?:\/\/(?!127\.0\.0\.1|localhost)/, route => route.abort());
      await page.goto(`/${language}/${name}.html`);
      await page.evaluate(() => document.fonts.ready);
      await page.waitForTimeout(500);
      const ends = await page.locator(TITLES).evaluateAll(
        els => els.map(el => el.textContent.trim().replace(/\s+/g, ' ')).filter(t => /[.。．]$/.test(t)));
      expect(ends).toEqual([]);
    });
  }
}
