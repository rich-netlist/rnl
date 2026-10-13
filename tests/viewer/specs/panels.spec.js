// The side panel and its parts: the splitters within their limits and kept for the next start, the
// side panel and the problems folded, the folded problems' header within reach, and the narrowest
// side panel beside its splitter, with its file tabs within reach.

import { expect, test } from "../viewer.js";

/** The size of an element in page pixels: "width" or "height". */
function size(page, selector, side) {
  return page.locator(selector).evaluate((e, side) => e.getBoundingClientRect()[side], side);
}

/** Drag a splitter by its middle to a page point; `x` or `y` may be left as it is. */
async function dragSplitter(page, viewer, selector, to) {
  const from = await viewer.middle(page.locator(selector));
  await viewer.drag(from, { x: to.x ?? from.x, y: to.y ?? from.y });
}

test("the side panel's splitter stays within its limits, and the width is kept", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy");
  expect(await size(page, "#sidebar", "width")).toBe(420);
  // as narrow as 220 pixels
  await dragSplitter(page, viewer, "#vsplit", { x: 0 });
  expect(await size(page, "#sidebar", "width")).toBe(220);
  expect(await viewer.stored("sidebar-width")).toBe(220);
  // as wide as three quarters of the window, which leaves the drawing 280 pixels or more
  await dragSplitter(page, viewer, "#vsplit", { x: 1199 });
  expect(await size(page, "#sidebar", "width")).toBe(900);
  expect(await viewer.stored("sidebar-width")).toBe(900);
  await dragSplitter(page, viewer, "#vsplit", { x: 600 });
  const width = await size(page, "#sidebar", "width");
  expect(Math.abs(width - 600)).toBeLessThanOrEqual(3);
  expect(await viewer.stored("sidebar-width")).toBe(width);
  // the drawing has the rest, and its view follows
  await expect.poll(() => size(page, "#canvas", "width")).toBe(1200 - width - 5);

  await viewer.open("04-hierarchy");
  expect(await size(page, "#sidebar", "width")).toBe(width);
  // a narrower window leaves the drawing its 280 pixels
  await page.setViewportSize({ width: 800, height: 800 });
  await expect.poll(() => size(page, "#sidebar", "width")).toBe(520);
  expect(await size(page, "#canvas", "width")).toBe(275);
});

test("the problems' splitter stays within its limits, and the height is kept", async ({ page, viewer }) => {
  await viewer.open("12-diagnostics");
  expect(await size(page, "#problems-body", "height")).toBe(180);
  // the text keeps 120 pixels, less the splitter and the borders
  await dragSplitter(page, viewer, "#hsplit", { y: 0 });
  const tallest = await size(page, "#problems-body", "height");
  expect(tallest).toBeGreaterThan(180);
  expect(await size(page, "#source", "height")).toBeGreaterThanOrEqual(110);
  expect(await size(page, "#source", "height")).toBeLessThanOrEqual(120);
  expect(await viewer.stored("problems-height")).toBe(tallest);
  // the problems keep 60
  await dragSplitter(page, viewer, "#hsplit", { y: 799 });
  expect(await size(page, "#problems-body", "height")).toBe(60);
  expect(await viewer.stored("problems-height")).toBe(60);

  await viewer.open("12-diagnostics");
  expect(await size(page, "#problems-body", "height")).toBe(60);
});

test("the index's splitter stays within its limits, and the height and the view are kept", async ({ page, viewer }) => {
  await viewer.open("subcircuits");
  await page.locator("#tab-hierarchy").click();
  expect(await viewer.stored("side-pane")).toBe("hierarchy");
  expect(await size(page, "#index-section", "height")).toBe(260);
  // the tree keeps 100 pixels, less the splitter
  await dragSplitter(page, viewer, "#hsplit2", { y: 0 });
  const tallest = await size(page, "#index-section", "height");
  expect(tallest).toBeGreaterThan(260);
  expect(await size(page, "#tree", "height")).toBeGreaterThanOrEqual(94);
  expect(await size(page, "#tree", "height")).toBeLessThanOrEqual(100);
  expect(await viewer.stored("index-height")).toBe(tallest);
  // the index keeps 120
  await dragSplitter(page, viewer, "#hsplit2", { y: 799 });
  expect(await size(page, "#index-section", "height")).toBe(120);
  expect(await viewer.stored("index-height")).toBe(120);

  await viewer.open("subcircuits");
  await expect(page.locator("#pane-hierarchy")).toBeVisible();
  expect(await size(page, "#index-section", "height")).toBe(120);
});

test("the buttons and Ctrl+B hide and show the side panel, which stays so", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy");
  await expect(page.locator("#show-sidebar")).toBeHidden();
  await page.locator("#hide-sidebar").click();
  await expect(page.locator("#sidebar")).toBeHidden();
  await expect(page.locator("#vsplit")).toBeHidden();
  await expect.poll(() => size(page, "#canvas", "width")).toBe(1200);
  expect(await viewer.stored("sidebar-shown")).toBe(false);

  await viewer.open("04-hierarchy");
  await expect(page.locator("#sidebar")).toBeHidden();
  await page.locator("#show-sidebar").click();
  await expect(page.locator("#sidebar")).toBeVisible();
  expect(await size(page, "#sidebar", "width")).toBe(420);
  expect(await viewer.stored("sidebar-shown")).toBe(true);
  await page.keyboard.press("Control+B");
  await expect(page.locator("#sidebar")).toBeHidden();
  await page.keyboard.press("Control+B");
  await expect(page.locator("#sidebar")).toBeVisible();
});

test("the folded problems keep their header within reach, above the status bar", async ({ page, viewer }) => {
  await viewer.open("12-diagnostics");
  const head = page.locator("#problems-head");
  /** Whether the header is on screen above the status bar, and is what a click there hits. */
  const reachable = () =>
    head.evaluate((h) => {
      const [b, bar] = [h.getBoundingClientRect(), document.querySelector(".statusbar").getBoundingClientRect()];
      const hit = document.elementFromPoint(b.left + b.width / 2, b.top + b.height / 2);
      return b.height > 0 && b.bottom <= bar.top + 0.5 && !!hit && h.contains(hit);
    });
  expect(await reachable()).toBe(true);
  await page.keyboard.press("Control+Shift+M");
  await expect(head).toHaveAttribute("aria-expanded", "false");
  await expect(page.locator("#problems-body")).toBeHidden();
  await expect(page.locator("#hsplit")).toBeHidden();
  expect(await reachable()).toBe(true);
  expect(await viewer.stored("problems-shown")).toBe(false);
  // the text takes the room
  const [source, headBox] = [await page.locator("#source").boundingBox(), await head.boundingBox()];
  expect(Math.abs(source.y + source.height - headBox.y)).toBeLessThanOrEqual(1);

  // in a low window too
  await page.setViewportSize({ width: 1200, height: 360 });
  await expect.poll(reachable).toBe(true);
  await head.click();
  await expect(head).toHaveAttribute("aria-expanded", "true");
  await expect(page.locator("#problems-body")).toBeVisible();
  expect(await viewer.stored("problems-shown")).toBe(true);
  // the stored folding is kept for the next start
  await head.click();
  await viewer.open("12-diagnostics");
  await expect(head).toHaveAttribute("aria-expanded", "false");
  await expect.poll(reachable).toBe(true);
});

test("at its narrowest the side panel stays beside its splitter", async ({ page, viewer }) => {
  /**
   * Whether the parts of the side panel's view keep within its width, and a press on the splitter
   * reaches it all along its height.
   */
  const apart = () =>
    page.evaluate(() => {
      const box = (el) => el.getBoundingClientRect();
      const [side, split, drawing] = [box(document.getElementById("sidebar")), box(document.getElementById("vsplit")), box(document.querySelector(".drawing"))];
      const parts = [".side-head", ".pane:not([hidden])", ".pane:not([hidden]) > *", "#problems-head", "#hide-sidebar"];
      const wide = [...document.querySelectorAll(parts.join(", "))].filter((e) => box(e).width > 0 && box(e).right > side.right + 0.5);
      const covered = [];
      for (let y = split.top + 10; y < split.bottom; y += 20) {
        if (document.elementFromPoint(split.left + split.width / 2, y) !== document.getElementById("vsplit")) covered.push(Math.round(y));
      }
      return {
        width: side.width,
        wide: wide.map((e) => e.id || e.className),
        covered,
        drawing: drawing.left >= split.right - 0.5,
      };
    });
  const want = { width: 220, wide: [], covered: [], drawing: true };
  // a long file name and the failed statements' box make the file bar wide
  await viewer.open("12-diagnostics");
  await dragSplitter(page, viewer, "#vsplit", { x: 0 });
  expect(await apart()).toEqual(want);
  await page.locator("#tab-hierarchy").click();
  expect(await apart()).toEqual(want);
  await page.locator("#tab-netlist").click();
  await viewer.open("04-hierarchy");
  expect(await apart()).toEqual(want);
  // and the splitter still drags
  await dragSplitter(page, viewer, "#vsplit", { x: 500 });
  expect(Math.abs((await size(page, "#sidebar", "width")) - 500)).toBeLessThanOrEqual(3);
});

test("at its narrowest the side panel keeps a tab of each file within reach", async ({ page, viewer }) => {
  // The failed statements' box does not shrink, and at 220 pixels it takes the whole file bar: the
  // file tabs get no width, and the box covers them (style.css, .file-bar).
  test.fail(true, "the failed statements' box covers the file tabs");
  /** Per file tab, whether a part of it is what a click there hits. */
  const reached = () =>
    page.locator("#file-tabs button").evaluateAll((tabs) =>
      tabs.map((b) => {
        const r = b.getBoundingClientRect();
        for (let x = r.left + 1; x < r.right; x += 2) if (document.elementFromPoint(x, r.top + r.height / 2) === b) return true;
        return false;
      }),
    );
  await viewer.open("04-hierarchy");
  expect(await reached()).toEqual([true, true]);
  await dragSplitter(page, viewer, "#vsplit", { x: 0 });
  expect(await size(page, "#sidebar", "width")).toBe(220);
  expect(await reached()).toEqual([true, true]);
  await page.locator("#file-tabs button", { hasText: "04-hierarchy-lib.inc" }).click({ timeout: 2000 });
  await expect(page.locator("#file-tabs button[aria-selected=true]")).toHaveText("04-hierarchy-lib.inc");
});
