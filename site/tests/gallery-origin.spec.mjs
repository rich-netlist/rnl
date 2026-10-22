import { test, expect } from "../../tests/viewer/node_modules/@playwright/test/index.mjs";
import { readFileSync, mkdirSync } from "node:fs";
import { resolve } from "node:path";

const group = id => JSON.parse(readFileSync(new URL(`../dist/gallery/${id}.js`, import.meta.url), "utf8").match(/^RNLGallery.group\((.*)\);/s)[1]);
const standard = group("standard-0.3");
const symbols = standard.packages.flatMap(p => p.statements).filter(s => s.svg);
const review = process.env.RNL_GALLERY_ORIGIN_SCREENSHOTS;

// Measure in screen pixels, including the inverse transform used for fixed-size marks.
async function marks(page, selector = ".gl-fig .g-origin, .gl-thumb .g-origin") {
  return page.locator(selector).evaluateAll(origins => origins.map(origin => {
    const svg = origin.ownerSVGElement, dot = origin.querySelector("circle"), lines = [...origin.querySelectorAll("line")];
    const point = (el, x, y) => new DOMPoint(x,y).matrixTransform(el.getScreenCTM());
    const translation = origin.transform.baseVal.getItem(0).matrix;
    const center = point(dot, 0, 0), expected = point(svg, translation.e, translation.f);
    const vb = svg.viewBox.baseVal, rootMatrix = svg.getScreenCTM().inverse();
    const inViewBox = (x,y) => x >= vb.x - .01 && y >= vb.y - .01 && x <= vb.x+vb.width+.01 && y <= vb.y+vb.height+.01;
    const dotMatrix = dot.getScreenCTM();
    return {
      symbol: origin.closest("[data-sym]")?.dataset.sym,
      visible: getComputedStyle(origin).display !== "none",
      hidden: origin.getAttribute("aria-hidden"),
      centerError: Math.hypot(center.x-expected.x, center.y-expected.y),
      lengths: lines.map(line => {
        const a = point(line, line.x1.baseVal.value, line.y1.baseVal.value), b = point(line, line.x2.baseVal.value, line.y2.baseVal.value);
        return Math.hypot(a.x-b.x, a.y-b.y);
      }),
      strokes: lines.map(line => [getComputedStyle(line).strokeWidth, line.getAttribute("vector-effect")]),
      diameter: 3 * Math.hypot(dotMatrix.a, dotMatrix.b),
      inside: [[-6,-6],[6,6]].every(([dx,dy]) => { const p = new DOMPoint(center.x+dx,center.y+dy).matrixTransform(rootMatrix); return inViewBox(p.x,p.y); }),
      color: getComputedStyle(dot).fill,
      last: origin === svg.lastElementChild,
    };
  }));
}

function assertMarks(rows) {
  expect(rows.length).toBeGreaterThan(0);
  for (const row of rows) {
    expect(row.visible, row.symbol).toBe(true);
    expect(row.hidden).toBe("true");
    expect(row.centerError, row.symbol).toBeLessThan(.5);
    for (const length of row.lengths) expect(length, row.symbol).toBeCloseTo(10, 2);
    expect(row.strokes).toEqual([["1.5px", "non-scaling-stroke"], ["1.5px", "non-scaling-stroke"]]);
    expect(row.diameter, row.symbol).toBeCloseTo(3, 2);
    expect(row.inside, row.symbol).toBe(true);
    expect(row.last).toBe(true);
  }
}

async function checkMarks(page, selector) {
  // ResizeObserver updates after responsive layout and web-font changes.
  await expect(async () => assertMarks(await marks(page, selector))).toPass({ timeout: 5000 });
}

test("origin is off by default, accepts old settings and persists across reloads and languages", async ({ page }) => {
  await page.addInitScript(() => {
    if (!localStorage.getItem("rnl-gallery-show")) localStorage.setItem("rnl-gallery-show", JSON.stringify({ pins: false, box: true, labels: false }));
  });
  await page.goto("/zh/packages.html");
  const toggle = page.locator('[data-show="origin"]');
  await expect(page.locator(".gl-card")).toHaveCount(standard.counts.symbols);
  await expect(toggle).toHaveText("原点");
  await expect(toggle).toHaveAttribute("aria-pressed", "false");
  await expect(page.locator('[data-show="pins"]')).toHaveAttribute("aria-pressed", "false");
  await expect(page.locator('[data-show="box"]')).toHaveAttribute("aria-pressed", "true");
  expect((await marks(page)).every(row => !row.visible)).toBe(true);
  const before = await page.locator(".gl-card").evaluateAll(cards => cards.map(c => [c.clientWidth,c.clientHeight]));
  await toggle.click();
  await expect(page.locator("#gl-main")).toHaveClass(/show-origin/);
  await expect(page.locator(".gl-fig .g-origin")).toHaveCount(standard.counts.symbols);
  await checkMarks(page);
  expect(await page.locator(".gl-card").evaluateAll(cards => cards.map(c => [c.clientWidth,c.clientHeight]))).toEqual(before);
  await page.reload();
  await expect(toggle).toHaveAttribute("aria-pressed", "true");
  await expect(page.locator(".gl-card")).toHaveCount(standard.counts.symbols);
  await checkMarks(page);
  await page.locator(".language-menu > summary").click();
  await page.locator('[data-language="en"]').click();
  await expect(toggle).toHaveText("Origin");
  await expect(toggle).toHaveAttribute("aria-pressed", "true");
  await expect(page.locator(".gl-card")).toHaveCount(standard.counts.symbols);
  await toggle.click();
  expect((await marks(page)).every(row => !row.visible)).toBe(true);
  await page.reload();
  await expect(toggle).toHaveAttribute("aria-pressed", "false");
});

test("three registration points agree with the generated coordinates in cards and enlarged views", async ({ page }) => {
  await page.goto("/en/packages.html");
  await page.locator('[data-show="origin"]').click();
  for (const name of ["$RESISTOR", "$NMOS", "$CURRENT_ARROW_SYMBOL"]) {
    const symbol = symbols.find(s => s.name === name);
    expect(symbol.origin).toEqual([0,0]); // Every build instance is explicitly @PLACE(0, 0).
    const card = page.locator(`.gl-card[data-sym="${name}"]`);
    const center = await card.locator(".g-origin circle").evaluate(dot => {
      const svg = dot.ownerSVGElement;
      const actual = new DOMPoint(0,0).matrixTransform(dot.getScreenCTM());
      const expected = new DOMPoint(0,0).matrixTransform(svg.getScreenCTM());
      return Math.hypot(actual.x-expected.x,actual.y-expected.y);
    });
    expect(center).toBeLessThan(.5);
    await card.click();
    await expect(page.locator("#gl-dlg")).toBeVisible();
    await checkMarks(page, ".gl-dlg-fig .g-origin");
    await page.setViewportSize({ width: 390, height: 844 });
    await expect.poll(async () => (await marks(page, ".gl-dlg-fig .g-origin"))[0].lengths[0]).toBeCloseTo(10, 2);
    await checkMarks(page, ".gl-dlg-fig .g-origin");
    await page.locator("#gl-dlg-close").click();
    await page.setViewportSize({ width: 1440, height: 1000 });
  }
});

test("origin outside the artwork is framed, fixed in size and theme aware", async ({ page }, info) => {
  const old = group("standard-0.2").packages.flatMap(p => p.statements).find(s => s.name === "$CURRENT_ARROW_SYM");
  expect(old.origin).toEqual([0,0]);
  const thumb = old.thumb.split(" ").map(Number);
  expect(thumb[1]+thumb[3]).toBeGreaterThan(0);
  await page.goto("/zh/packages.html?series=standard-0.2&package=rnl-std-virtual#gallery");
  await page.locator('[data-show="origin"]').click();
  const card = page.locator('.gl-card[data-sym="$CURRENT_ARROW_SYM"]');
  await checkMarks(page);
  const outside = await card.locator("svg").evaluate(svg => {
    const b = svg.querySelector(".g-art").getBBox();
    return 0 < b.x || 0 > b.x+b.width || 0 < b.y || 0 > b.y+b.height;
  });
  expect(outside).toBe(true);
  if (review && info.project.name === "chromium") {
    mkdirSync(review, { recursive: true });
    await card.screenshot({ path: resolve(review, "origin-outside-artwork.png") });
  }
  await page.locator("#theme-btn").click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  const expected = await page.evaluate(() => {
    const el = document.createElement("span"); el.style.color = "var(--err)"; document.body.append(el);
    const color = getComputedStyle(el).color; el.remove(); return color;
  });
  await expect.poll(async () => [...new Set((await marks(page)).map(row => row.color))]).toEqual([expected]);
});

test("review screenshots: standard 0.3 off, on, dark and enlarged", async ({ page }, info) => {
  test.skip(!review || info.project.name !== "chromium", "Optional review screenshots");
  mkdirSync(review, { recursive: true });
  await page.addInitScript(() => localStorage.setItem("rnl-intro-theme", "light"));
  await page.goto("/zh/packages.html?package=rnl-std-basic#gallery");
  await expect(page.locator(".gl-card").first()).toBeVisible();
  await page.evaluate(() => document.fonts.ready);
  await page.locator('.gl-block[data-block="symbols"]').evaluate(el => window.scrollTo(0, el.getBoundingClientRect().top+scrollY-125));
  await page.mouse.move(0,0);
  await page.screenshot({ path: resolve(review, "standard-0.3-origin-off.png") });
  await page.locator('[data-show="origin"]').click();
  await checkMarks(page);
  await page.screenshot({ path: resolve(review, "standard-0.3-origin-on.png") });
  await page.locator("#theme-btn").click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await page.screenshot({ path: resolve(review, "standard-0.3-origin-dark.png") });
  await page.locator('.gl-card[data-sym="$RESISTOR"]').click();
  await expect(page.locator("#gl-dlg")).toBeVisible();
  await checkMarks(page, ".gl-dlg-fig .g-origin");
  await page.screenshot({ path: resolve(review, "origin-enlarged.png") });
});
