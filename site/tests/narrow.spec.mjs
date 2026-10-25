import { test, expect } from '../../tests/viewer/node_modules/@playwright/test/index.mjs';

// A translation is often longer than the English: at 375px, the width of a small phone, chips, segmented buttons and
// the buttons of the gallery wrap onto more lines, and a title that is one long word breaks, instead of widening the page.
const WIDTH = 375;
const LONG = 'Eine ziemlich lange Beschriftung, die nicht in eine Zeile passt';
const LABEL = 'Ziemlich lange Beschriftung';  // one button's label: wide, but narrower than the screen
const wide = page => page.evaluate(() => document.documentElement.scrollWidth);

test.beforeEach(async ({ page }) => {
  await page.setViewportSize({ width: WIDTH, height: 800 });
  await page.route('https://rich-netlist.github.io/**', route => route.abort());
  await page.addInitScript(() => localStorage.setItem('rnl-intro-theme', 'light'));
});

for (const language of ['en', 'zh']) {
  test(`narrow: the homepage fits ${WIDTH}px, ${language}`, async ({ page }) => {
    await page.goto(`/${language}/index.html`);
    expect(await wide(page)).toBeLessThanOrEqual(WIDTH);
    const hub = await page.locator('.hub-node').first().boundingBox();
    expect(hub.x + hub.width).toBeLessThanOrEqual(WIDTH);
  });
}

test('narrow: a long chip in the hero wraps', async ({ page }) => {
  await page.goto('/en/index.html');
  await page.locator('.eyebrow .chip').evaluate((chip, text) => { chip.textContent = text; }, LONG + ' ' + LONG);
  expect(await wide(page)).toBeLessThanOrEqual(WIDTH);
  const chip = await page.locator('.eyebrow .chip').boundingBox();
  expect(chip.x + chip.width).toBeLessThanOrEqual(WIDTH);
  expect(chip.height).toBeGreaterThan(40); // on more than one line
});

test('narrow: segmented buttons with long labels wrap onto more lines', async ({ page }) => {
  await page.goto('/en/tools.html');
  const seg = page.locator('.seg').first();
  await seg.locator('button').evaluateAll((buttons, text) => buttons.forEach(b => { b.textContent = text; }), LABEL);
  expect(await wide(page)).toBeLessThanOrEqual(WIDTH);
  const box = await seg.boundingBox();
  expect(box.x + box.width).toBeLessThanOrEqual(WIDTH);
});

test('narrow: the buttons of the gallery of packages wrap', async ({ page }) => {
  await page.goto('/en/packages.html');
  await page.locator('.gl-tog').evaluateAll((buttons, text) => buttons.forEach(b => { b.append(text); }), LABEL);
  expect(await wide(page)).toBeLessThanOrEqual(WIDTH);
});

test('narrow: a title that is one long word breaks', async ({ page }) => {
  await page.goto('/en/timeline.html');
  await page.locator('.tour-h1').evaluate(title => { title.textContent = 'Übersetzungsstatusübersicht'; });
  expect(await wide(page)).toBeLessThanOrEqual(WIDTH);
});
