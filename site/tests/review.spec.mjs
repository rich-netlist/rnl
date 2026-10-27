import { test, expect } from '../../tests/viewer/node_modules/@playwright/test/index.mjs';

// The example of the introduction: a five-transistor OTA that is reviewed by what it says. Picking an item of the
// checklist, or a statement of the file, lights up the devices it is about in the drawing (the RNL core's own drawing)
// and their lines in the netlist. The look switch swaps one core drawing for the other; the simulator view dims the
// RNL lines.
test.beforeEach(async ({ page }) => {
  await page.route('https://rich-netlist.github.io/**', route => route.abort());
  await page.addInitScript(() => localStorage.setItem('rnl-intro-theme', 'light'));
  // no smooth scrolling under the pointer: the page moving while the mouse is placed would hover other lines
  await page.addInitScript(() => document.addEventListener('DOMContentLoaded', () => {
    const style = document.createElement('style');
    style.textContent = 'html { scroll-behavior: auto !important; }';
    document.head.append(style);
  }));
});

// the page settles (fonts, the caption's height) before the pointer is placed on it
async function open(page, url) {
  await page.goto(url);
  await page.evaluate(() => document.fonts.ready);
  await page.waitForTimeout(400);
}

for (const language of ['en', 'zh']) {
  test(`review: an item lights up its devices, its statements and their lines, ${language}`, async ({ page }) => {
    await open(page, `/${language}/index.html`);
    const items = page.locator('#rv-items .rv-item');
    await expect(items).toHaveCount(5);
    const lit = page.locator('#rv-stage svg [data-dev].is-lit');
    const devices = new Set();
    const litDevices = async () => new Set(await lit.evaluateAll(els => els.map(el => el.getAttribute('data-dev'))));
    const hot = page.locator('#rv-code .cl.is-hot'), devs = page.locator('#rv-code .cl.is-dev');
    await expect(lit).toHaveCount(0);

    // the differential pair: M1, M2 and the tail M5, in the drawing and in the netlist, and its statement
    await items.nth(0).hover();
    await expect.poll(async () => [...await litDevices()].sort()).toEqual(['M1', 'M2', 'M5']);
    await expect(devs).toHaveCount(3);
    await expect(hot).toHaveCount(1);
    await expect(hot).toContainText('$DIFFERENTIAL_PAIR');
    await expect(page.locator('#rv-stage .rv-note.is-on')).toHaveCount(1);
    // leaving clears it; a click keeps it
    await page.mouse.move(5, 5);
    await expect(lit).toHaveCount(0);
    await items.nth(1).click();
    await expect(items.nth(1)).toHaveAttribute('aria-pressed', 'true');
    await page.mouse.move(5, 5);
    await expect.poll(async () => [...await litDevices()].sort()).toEqual(['M3', 'M4']);
    await expect(page.locator('#rv-code .cl.is-hot')).toContainText('$CURRENT_MIRROR');
    await items.nth(1).click();
    await expect(lit).toHaveCount(0);

    // a statement of the file picks its item
    await page.locator('#rv-code .cl', { hasText: '@ORDER' }).first().hover();
    await expect.poll(async () => [...await litDevices()].sort()).toEqual(['M1', 'M3', 'M5']);
    // the symmetry: two statements, the four devices and the axis
    await items.nth(2).hover();
    await expect.poll(async () => [...await litDevices()].sort()).toEqual(['M1', 'M2', 'M3', 'M4']);
    await expect(hot).toHaveCount(3);
    devices.clear();

    // a device of the drawing marks its lines in the file
    await page.mouse.move(5, 5);
    await page.locator('#rv-stage svg [data-dev="M2"]').first().dispatchEvent('mouseover');
    await expect(page.locator('#rv-code .cl.is-hot').first()).toContainText('M2');
  });
}

test('review: the look switch redraws the same circuit with the packages, the file shows what changed', async ({ page }) => {
  await open(page, '/en/index.html');
  const stage = page.locator('#rv-stage svg');
  const devices = async () => [...new Set(await page.locator('#rv-stage svg [data-dev]').evaluateAll(els => els.map(el => el.getAttribute('data-dev'))))].sort();
  const standard = await stage.innerHTML();
  const names = await devices();
  expect(names).toEqual(expect.arrayContaining(['M1', 'M2', 'M3', 'M4', 'M5', 'PB', 'PI', 'PN', 'PO', 'VDD', 'VSS']));
  await expect(page.locator('#rv-code')).not.toContainText('@USE-PACKAGE');
  await expect(page.locator('#rv-open')).toHaveAttribute('href', /ota5t-review\.sp/);

  await page.locator('#rv-look [data-look="eda"]').click();
  expect(await stage.innerHTML()).not.toBe(standard);
  expect(await devices()).toEqual(names);
  await expect(page.locator('#rv-code .cl.is-diff')).toContainText('@USE-PACKAGE(name="eda-symbol", version="0.1")');
  await expect(page.locator('#rv-code .cl.is-diff')).toContainText('@USE-PACKAGE(name="eda-style", version="0.1")');
  await expect(page.locator('#rv-open')).toHaveAttribute('href', /ota5t-review-eda\.sp/);
  // the traditional look is drawn on black, the standard one on white
  await expect(page.locator('#rv-stage')).toHaveCSS('background-color', 'rgb(0, 0, 0)');
  // the review works in this look too
  await page.locator('#rv-items .rv-item').nth(0).click();
  await expect(page.locator('#rv-stage svg [data-dev].is-lit')).not.toHaveCount(0);
  await page.locator('#rv-items .rv-item').nth(0).click();

  await page.locator('#rv-look [data-look="standard"]').click();
  expect(await stage.innerHTML()).toBe(standard);
  await expect(page.locator('#rv-stage')).toHaveCSS('background-color', 'rgb(255, 255, 255)');
  await expect(page.locator('#rv-code .cl.is-diff')).toHaveCount(0);
});

test('review: the simulator view dims the RNL lines', async ({ page }) => {
  await open(page, '/en/index.html');
  const rnl = page.locator('#rv-code .cl.is-rnl .tx').first();
  await expect(rnl).toHaveCSS('opacity', '1');
  await page.locator('#rv-view [data-view="sim"]').click();
  await expect(page.locator('#rv-code')).toHaveClass(/is-sim/);
  await expect(rnl).not.toHaveCSS('opacity', '1');
  await expect(page.locator('#rv-stage')).toBeHidden();
  await expect(page.locator('#rv-sim')).toBeVisible();
  await expect(page.locator('#rv-cap')).toContainText('comment');
  // picking an item goes back to the viewer
  await page.locator('#rv-items .rv-item').nth(0).hover();
  await expect(page.locator('#rv-stage')).toBeVisible();
  await expect(rnl).toHaveCSS('opacity', '1');
});

test('review: the writing style of the file, Chinese sentences only on the Chinese page', async ({ page }) => {
  const han = /[一-鿿]/;
  await open(page, '/en/index.html');
  await expect(page.locator('#rv-style button')).toHaveCount(2);
  await page.locator('#rv-style [data-style="bare"]').click();
  await expect(page.locator('#rv-code')).not.toContainText('treated');
  await expect(page.locator('#rv-code')).toContainText('%dp1 = #M1(in_p) #M2(in_n) #M5(tail) @AS $DIFFERENTIAL_PAIR;');
  await expect(page.locator('#rv-open')).toHaveAttribute('href', /ota5t-review-bare\.sp/);
  await page.locator('#rv-style [data-style="en"]').click();
  await expect(page.locator('#rv-code')).toContainText('treated @AS $DIFFERENTIAL_PAIR;');
  expect(han.test(await page.locator('#rv-code').textContent())).toBe(false);

  await open(page, '/zh/index.html');
  await expect(page.locator('#rv-style button')).toHaveCount(3);
  await page.locator('#rv-style [data-style="zh"]').click();
  await expect(page.locator('#rv-code')).toContainText('@视作 $"差分对";');
  // the checklist is in the page's own language, whatever the style of the file
  expect(han.test(await page.locator('#rv-items').textContent())).toBe(true);
  await page.locator('#rv-style [data-style="en"]').click();
  expect(han.test(await page.locator('#rv-code .cl', { hasText: '$DIFFERENTIAL_PAIR' }).textContent())).toBe(false);
});

test('review: the file and the drawing are in one row on a wide screen, and nothing is wider than the screen', async ({ page }) => {
  await page.setViewportSize({ width: 1200, height: 900 });
  await open(page, '/en/index.html');
  const code = await page.locator('.rv-code').boundingBox(), figure = await page.locator('#rv-figure').boundingBox();
  expect(Math.abs(code.y - figure.y)).toBeLessThan(2);
  expect(figure.x).toBeGreaterThan(code.x + code.width - 2);
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(1200);
  await page.setViewportSize({ width: 390, height: 900 });
  await open(page, '/en/index.html');
  await expect(page.locator('#rv-stage svg')).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(390);
});
