import { test, expect } from '../../tests/viewer/node_modules/@playwright/test/index.mjs';

// The example of the introduction: an op-amp that is reviewed by what it says. Picking an item of the checklist, or a
// statement of the file, lights up the devices it is about in the drawing and their lines in the netlist.
test.beforeEach(async ({ page }) => {
  await page.route('https://rich-netlist.github.io/**', route => route.abort());
  await page.addInitScript(() => localStorage.setItem('rnl-intro-theme', 'light'));
});

for (const language of ['en', 'zh']) {
  test(`review: an item lights up its devices, its statements and their lines, ${language}`, async ({ page }) => {
    await page.goto(`/${language}/index.html`);
    const items = page.locator('#rv-items .rv-item');
    await expect(items).toHaveCount(7);
    const lit = page.locator('#rv-svg .sc-dev.is-hl');
    const hot = page.locator('#rv-code .cl.is-hot'), devs = page.locator('#rv-code .cl.is-dev');
    await expect(lit).toHaveCount(0);

    // the differential pair: M1, M2 and the tail M5, in the drawing and in the netlist, and its statement
    await items.nth(0).hover();
    await expect(lit).toHaveCount(3);
    await expect(devs).toHaveCount(3);
    await expect(hot).toHaveCount(1);
    await expect(hot).toContainText('$DIFFERENTIAL_PAIR');
    await expect(page.locator('#rv-svg .sc-dev.is-hl[data-ref="M5"]')).toHaveCount(1);
    // leaving clears it; a click keeps it
    await page.mouse.move(5, 5);
    await expect(lit).toHaveCount(0);
    await items.nth(3).click();
    await expect(items.nth(3)).toHaveAttribute('aria-pressed', 'true');
    await page.mouse.move(5, 5);
    await expect(lit).toHaveCount(3);
    await expect(page.locator('#rv-code .cl.is-hot')).toContainText('$MILLER_COMPENSATION');
    await items.nth(3).click();
    await expect(lit).toHaveCount(0);

    // a statement of the file picks its item
    await page.locator('#rv-code .cl', { hasText: '$CURRENT_MIRROR' }).first().hover();
    await expect(lit).toHaveCount(2);
    // the symmetry: two statements, the four devices and the axis
    await items.nth(4).hover();
    await expect(lit).toHaveCount(4);
    await expect(hot).toHaveCount(3);
  });
}

test('review: the writing style of the file, Chinese sentences only on the Chinese page', async ({ page }) => {
  const han = /[一-鿿]/;
  await page.goto('/en/index.html');
  await expect(page.locator('#rv-style button')).toHaveCount(2);
  await page.locator('#rv-style [data-style="bare"]').click();
  await expect(page.locator('#rv-code')).not.toContainText('treated');
  await expect(page.locator('#rv-code')).toContainText('%dp1 = #M1(in_p) #M2(in_n) #M5(tail) @AS $DIFFERENTIAL_PAIR;');
  await page.locator('#rv-style [data-style="en"]').click();
  await expect(page.locator('#rv-code')).toContainText('treated @AS $DIFFERENTIAL_PAIR;');
  expect(han.test(await page.locator('#rv-code').textContent())).toBe(false);

  await page.goto('/zh/index.html');
  await expect(page.locator('#rv-style button')).toHaveCount(3);
  await page.locator('#rv-style [data-style="zh"]').click();
  await expect(page.locator('#rv-code')).toContainText('@视作 $"差分对";');
  // the checklist is in the page's own language, whatever the style of the file
  expect(han.test(await page.locator('#rv-items').textContent())).toBe(true);
  // the statements are in English again (the note about the lines left out is the page's own text)
  await page.locator('#rv-style [data-style="en"]').click();
  expect(han.test(await page.locator('#rv-code .cl', { hasText: '$DIFFERENTIAL_PAIR' }).textContent())).toBe(false);
  await expect(page.locator('#rv-code .cl', { hasText: '$DIFFERENTIAL_PAIR' })).toContainText('treated @AS');
});

test('review: the drawing has every device of the file and nothing is wider than the screen', async ({ page }) => {
  for (const width of [1200, 390]) {
    await page.setViewportSize({ width, height: 900 });
    await page.goto('/en/index.html');
    await expect(page.locator('#rv-svg .sc-dev:not(.is-virt)')).toHaveCount(10);
    await expect(page.locator('#rv-svg .sc-dev.is-virt')).toHaveCount(10);
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
  }
});
