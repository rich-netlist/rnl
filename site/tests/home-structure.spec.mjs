import { test, expect } from '../../tests/viewer/node_modules/@playwright/test/index.mjs';

// The structure of the introduction page: the order of the scenarios, and the design flow of
// the vision section, drawn the same way at every width (the arrows between the boxes are kept on a phone).
test.beforeEach(async ({ page }) => {
  await page.route('https://rich-netlist.github.io/**', route => route.abort());
  await page.addInitScript(() => localStorage.setItem('rnl-intro-theme', 'light'));
});

async function open(page, url) {
  await page.goto(url);
  await page.evaluate(() => document.fonts.ready);
  await page.waitForTimeout(400);
}

for (const language of ['en', 'zh']) {
  test(`the scenarios: AI first, the open ecosystem last, ${language}`, async ({ page }) => {
    await open(page, `/${language}/index.html`);
    const tabs = page.locator('#sc-tabs [role="tab"]');
    await expect(tabs).toHaveCount(5);
    expect(await tabs.evaluateAll(els => els.map(el => el.id))).toEqual(['t-ai', 't-git', 't-truth', 't-port', 't-open']);
    await expect(tabs.first()).toHaveAttribute('aria-selected', 'true');
    await expect(page.locator('#sc-ai')).toBeVisible();
    await expect(page.locator('#sc-open')).toBeHidden();
    await tabs.last().click();
    await expect(page.locator('#sc-open')).toBeVisible();
    await expect(page.locator('#sc-ai')).toBeHidden();
  });
}

for (const [width, height] of [[1200, 900], [768, 900], [390, 844]]) {
  test(`the design flow keeps its structure and its arrows at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height });
    await open(page, '/en/index.html');
    const hub = await page.locator('#hub').boundingBox();
    const core = await page.locator('#hub-core').boundingBox();
    const nodes = await page.locator('#hub .hub-node').evaluateAll(els => els.map(el => {
      const b = el.getBoundingClientRect();
      return { x: b.x, y: b.y, w: b.width, h: b.height };
    }));
    expect(nodes).toHaveLength(3);
    // the two above the file, the tools under it, all inside the figure
    expect(nodes[0].y + nodes[0].h).toBeLessThanOrEqual(core.y);
    expect(nodes[1].y + nodes[1].h).toBeLessThanOrEqual(core.y);
    expect(nodes[2].y).toBeGreaterThanOrEqual(core.y + core.height);
    for (const n of nodes.concat([{ x: core.x, w: core.width }])) {
      expect(n.x).toBeGreaterThanOrEqual(hub.x - 1);
      expect(n.x + n.w).toBeLessThanOrEqual(hub.x + hub.width + 1);
    }
    // one wire for each box, and they are drawn
    await expect(page.locator('#hub-wires path')).toHaveCount(3);
    const wires = await page.locator('#hub-wires path').evaluateAll(els => els.map(el => el.getBBox().height));
    for (const h of wires) expect(h).toBeGreaterThan(8);
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
  });
}
