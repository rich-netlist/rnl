import { test, expect } from '../../tests/viewer/node_modules/@playwright/test/index.mjs';

// The example of the introduction: a five-transistor OTA that is reviewed by what it says. Picking an item of the
// checklist, or a statement of the file, marks what it is about in the drawing (the RNL core's own drawing) as an RNL
// viewer does, the frame of its usage with the usage's title, and the devices' lines in the netlist; a device's line
// selects the device, in the colour of the selection with its box; nothing is dimmed. The look switch swaps one core
// drawing for the other, framed alike; the simulator view dims the RNL lines.
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
    await expect(items).toHaveCount(3);
    const lit = page.locator('#rv-stage svg [data-dev].is-lit');
    const litDevices = async () => [...new Set(await lit.evaluateAll(els => els.map(el => el.getAttribute('data-dev'))))].sort();
    const marks = page.locator('#rv-stage svg .rv-marks');
    const title = async () => (await marks.locator('text').allTextContents())[0] || null;
    const hot = page.locator('#rv-code .cl.is-hot'), devs = page.locator('#rv-code .cl.is-dev');
    // nothing of the drawing is dimmed, whatever is picked
    const dimmed = async () => page.locator('#rv-stage svg [data-id]').evaluateAll(els => els.filter(el => getComputedStyle(el).opacity !== '1').length);
    await expect(lit).toHaveCount(0);
    await expect(marks).toHaveCount(0);

    // the differential pair: the frame of %dp1 with its title and its roles, and M1, M2 and the tail M5 in the
    // netlist, and its statement; hovered again from outside while it is not all shown: a layout shift can leave the
    // pointer on the code panel, whose own hover marks a line's devices without marking their lines
    await expect.poll(async () => {
      const state = { title: await title(), devs: await devs.count() };
      if (state.devs !== 3) { await page.mouse.move(5, 5); await items.nth(0).hover(); }
      return state;
    }).toEqual({ title: '$DIFFERENTIAL_PAIR_N: %dp1', devs: 3 });
    expect(await marks.locator('text').allTextContents()).toEqual(['$DIFFERENTIAL_PAIR_N: %dp1', 'left', 'right', 'tail']);
    await expect(lit).toHaveCount(0);
    expect(await dimmed()).toBe(0);
    await expect(hot).toHaveCount(1);
    await expect(hot).toContainText('$DIFFERENTIAL_PAIR');
    // leaving clears it; a click keeps it
    await page.mouse.move(5, 5);
    await expect(marks).toHaveCount(0);
    await items.nth(1).click();
    await expect(items.nth(1)).toHaveAttribute('aria-pressed', 'true');
    await page.mouse.move(5, 5);
    await expect.poll(title).toBe('$CURRENT_MIRROR_P: %cm1');
    await expect(page.locator('#rv-code .cl.is-hot')).toContainText('$CURRENT_MIRROR_P');
    await items.nth(1).click();
    await expect(marks).toHaveCount(0);

    // a statement of the file picks its item; the tail current makes no usage: a frame around M5, and its arrow
    await page.locator('#rv-code .cl', { hasText: '@CURRENT' }).first().hover();
    await expect.poll(async () => marks.locator('rect').count()).toBe(2);
    expect(await title()).toBe(null);
    await expect(page.locator('#rv-stage .rv-note.is-on')).toHaveCount(1);
    await items.nth(2).hover();
    await expect.poll(async () => marks.locator('rect').count()).toBe(2);
    await expect(hot).toHaveCount(1);
    expect(await dimmed()).toBe(0);

    // the line of a device selects it: its parts in the colour of the selection, and its box
    await page.mouse.move(5, 5);
    await page.locator('#rv-code .cl', { hasText: /M1 x\s+inp/ }).first().hover();
    await expect.poll(litDevices).toEqual(['M1']);
    await expect(marks.locator('rect[stroke-dasharray="5 3"]')).toHaveCount(1);
    expect(await lit.evaluateAll(els => els.some(g => [...g.querySelectorAll('[stroke]:not([stroke="none"])')].some(e => getComputedStyle(e).stroke === 'rgb(232, 89, 12)')))).toBe(true);
    expect(await dimmed()).toBe(0);

    // a device of the drawing marks its lines in the file
    await page.mouse.move(5, 5);
    await page.locator('#rv-stage svg [data-dev="M2"]').first().dispatchEvent('mouseover');
    await expect(page.locator('#rv-code .cl.is-hot').first()).toContainText('M2');
  });
}

// The two looks are framed alike: the drawing keeps its size, and the circuit fills it, in the same place.
test('review: the two looks frame the circuit alike', async ({ page }) => {
  await open(page, '/en/index.html');
  const framing = async () => page.locator('#rv-stage svg').evaluate((svg) => {
    const s = svg.getBoundingClientRect();
    let b = null;
    for (const el of svg.querySelectorAll('[data-dev]')) {
      const r = el.getBoundingClientRect();
      if (!r.width && !r.height) continue;
      b = b ? { l: Math.min(b.l, r.left), t: Math.min(b.t, r.top), r: Math.max(b.r, r.right), b: Math.max(b.b, r.bottom) } : { l: r.left, t: r.top, r: r.right, b: r.bottom };
    }
    return { w: s.width, h: s.height, cx: ((b.l + b.r) / 2 - s.left) / s.width, cy: ((b.t + b.b) / 2 - s.top) / s.height,
      fw: (b.r - b.l) / s.width, fh: (b.b - b.t) / s.height };
  });
  const standard = await framing();
  await page.locator('#rv-look [data-look="eda"]').click();
  const eda = await framing();
  expect(Math.abs(eda.w - standard.w)).toBeLessThan(1);
  expect(Math.abs(eda.h - standard.h)).toBeLessThan(1);
  for (const k of ['cx', 'cy']) expect(Math.abs(eda[k] - standard[k])).toBeLessThan(0.04);
  for (const k of ['fw', 'fh']) { expect(standard[k]).toBeGreaterThan(0.7); expect(eda[k]).toBeGreaterThan(0.7); }
});

test('review: the look switch redraws the same circuit with the packages, the file shows what changed', async ({ page }) => {
  await open(page, '/en/index.html');
  const stage = page.locator('#rv-stage svg');
  const devices = async () => [...new Set(await page.locator('#rv-stage svg [data-dev]').evaluateAll(els => els.map(el => el.getAttribute('data-dev'))))].sort();
  const standard = await stage.innerHTML();
  const names = await devices();
  expect(names).toEqual(expect.arrayContaining(['M1', 'M2', 'M3', 'M4', 'M5', 'P_BIAS', 'P_INN', 'P_INP', 'P_OUT', 'VDD', 'VSS']));
  await expect(page.locator('#rv-code')).not.toContainText('eda-symbol');
  await expect(page.locator('#rv-open')).toHaveAttribute('href', /ota5t-review\.sp/);

  await page.locator('#rv-look [data-look="eda"]').click();
  expect(await stage.innerHTML()).not.toBe(standard);
  expect(await devices()).toEqual(names);
  await expect(page.locator('#rv-code .cl.is-diff')).toContainText('@USE-PACKAGE(name="eda-symbol", version="0.2")');
  await expect(page.locator('#rv-code .cl.is-diff')).toContainText('@USE-PACKAGE(name="eda-style", version="0.2")');
  await expect(page.locator('#rv-open')).toHaveAttribute('href', /ota5t-review-eda\.sp/);
  // the traditional look is drawn on black, the standard one on white
  await expect(page.locator('#rv-stage')).toHaveCSS('background-color', 'rgb(0, 0, 0)');
  // the review works in this look too
  await page.locator('#rv-items .rv-item').nth(0).click();
  await expect(page.locator('#rv-stage svg .rv-marks text').first()).toHaveText('$DIFFERENTIAL_PAIR_N: %dp1');
  await page.locator('#rv-items .rv-item').nth(0).click();
  await expect(page.locator('#rv-stage svg .rv-marks')).toHaveCount(0);

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
  await expect(page.locator('#rv-code')).toContainText('%dp1 = #M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N;');
  await expect(page.locator('#rv-open')).toHaveAttribute('href', /ota5t-review-bare\.sp/);
  await page.locator('#rv-style [data-style="en"]').click();
  await expect(page.locator('#rv-code')).toContainText('treated @AS $DIFFERENTIAL_PAIR_N;');
  expect(han.test(await page.locator('#rv-code').textContent())).toBe(false);

  await open(page, '/zh/index.html');
  await expect(page.locator('#rv-style button')).toHaveCount(3);
  await page.locator('#rv-style [data-style="zh"]').click();
  await expect(page.locator('#rv-code')).toContainText('@视作 $"N型差分对";');
  // the checklist is in the page's own language, whatever the style of the file
  expect(han.test(await page.locator('#rv-items').textContent())).toBe(true);
  await page.locator('#rv-style [data-style="en"]').click();
  expect(han.test(await page.locator('#rv-code .cl', { hasText: '$DIFFERENTIAL_PAIR' }).textContent())).toBe(false);
});

// The heading of the example, the file, the drawing and the checklist are in one view on a 16:9 screen: 1080p without the
// browser's bars, and a laptop at 125%. The page stops at the heading when it is scrolled close (proximity: it never holds the scroll).
for (const [width, height] of [[1920, 950], [1536, 730]]) {
  for (const language of ['en', 'zh']) {
    test(`review: heading, file, drawing and checklist fit one screen at ${width}x${height}, in both views, ${language}`, async ({ page }) => {
      await page.setViewportSize({ width, height });
      await open(page, `/${language}/index.html`);
      const snap = await page.evaluate(() => getComputedStyle(document.documentElement).scrollSnapType);
      expect(snap).toMatch(/^y( proximity)?$/);   // proximity is the default strictness, and is how the browsers print it
      await page.locator('#example').evaluate(el => el.scrollIntoView({ behavior: 'instant' }));
      await page.waitForTimeout(300);
      const bar = await page.locator('.topbar').boundingBox();
      const inView = async () => {
        const title = await page.locator('#example').boundingBox();
        expect(title.y).toBeGreaterThanOrEqual(bar.y + bar.height - 1);   // the heading is not under the bar
        const box = await page.locator('#review').boundingBox();
        expect(box.y + box.height).toBeLessThanOrEqual(height);
        return box.height;
      };
      for (const part of ['#example', '#journey', '.rv-code', '#rv-figure', '.rv-list']) await expect(page.locator(part)).toBeInViewport({ ratio: 1 });
      const viewer = await inView();
      await page.locator('#rv-view [data-view="sim"]').click();
      await expect(page.locator('#rv-sim')).toBeVisible();
      expect(Math.abs(await inView() - viewer)).toBeLessThan(2);
    });
  }
}

test('review: on a phone the file, the drawing and the checklist stay one above the other', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await open(page, '/en/index.html');
  const figure = await page.locator('#rv-figure').boundingBox(), code = await page.locator('.rv-code').boundingBox(), list = await page.locator('.rv-list').boundingBox();
  expect(code.y).toBeGreaterThan(figure.y + figure.height - 2);
  expect(list.y).toBeGreaterThan(code.y + code.height - 2);
  expect(await page.evaluate(() => getComputedStyle(document.documentElement).scrollSnapType)).toBe('none');
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

// The legend under the file says what the picked lines look like: it has the colour of those lines, in both themes and both looks.
for (const theme of ['light', 'dark']) {
  for (const look of ['standard', 'eda']) {
    test(`review: the legend chip of the picked lines has the colour of the lines, ${theme} theme, ${look} look`, async ({ page }) => {
      await page.addInitScript(t => localStorage.setItem('rnl-intro-theme', t), theme);
      await open(page, '/en/index.html');
      if (look === 'eda') await page.locator('#rv-look [data-look="eda"]').click();
      await page.locator('#rv-items .rv-item').nth(0).click();
      await page.mouse.move(5, 5);
      const chip = await page.locator('.rv-code .jr-legend .chip', { hasText: 'Lines of the picked item' })
        .evaluate(el => ({ bg: getComputedStyle(el).backgroundColor, color: getComputedStyle(el).color }));
      const hot = page.locator('#rv-code .cl.is-hot').first();
      // the lines fade to their colour (a transition): wait for it
      await expect.poll(() => hot.evaluate(el => getComputedStyle(el).backgroundColor)).toBe(chip.bg);
      expect(await hot.evaluate(el => getComputedStyle(el).boxShadow)).toContain(chip.color);
    });
  }
}

// The example is a stop of the scroll (proximity), but a link to it must land where it points: below the top bar, on its heading.
for (const hash of ['example', 'journey']) {
  test(`review: the link #${hash} lands on the example, not moved on by the scroll stop`, async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 1000 });
    await open(page, `/en/index.html#${hash}`);
    await page.waitForTimeout(600);
    const bar = await page.locator('.topbar').boundingBox();
    const top = await page.locator(`#${hash}`).evaluate(el => el.getBoundingClientRect().top);
    expect(top).toBeGreaterThanOrEqual(bar.y + bar.height - 1);
    expect(top).toBeLessThan(300);
    // scrolled by hand, the stop is back
    await page.mouse.move(700, 500);
    await page.mouse.wheel(0, 60);
    await expect.poll(() => page.evaluate(() => document.documentElement.classList.contains('no-snap'))).toBe(false);
  });
}

// Scrolled by hand to somewhere close, the page comes to rest with the heading of the example under the bar.
test('review: the scroll comes to rest at the heading of the example', async ({ page, browserName }) => {
  test.skip(browserName !== 'chromium', 'proximity snapping after a wheel scroll is checked in Chromium');
  await page.setViewportSize({ width: 1536, height: 730 });
  await open(page, '/en/index.html');
  const top = () => page.locator('#example').evaluate(el => Math.round(el.getBoundingClientRect().top));
  await page.mouse.move(700, 400);
  for (let i = 0; i < 40 && (await top()) > 220; i++) await page.mouse.wheel(0, 100);
  await expect.poll(top, { timeout: 5000 }).toBe(66);
});
