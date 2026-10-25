// The page on a phone and a tablet (the projects mobile-chromium and mobile-webkit): upright, the side
// panel is below the drawing, as wide as the screen, and folds, unfolds and drags taller or lower;
// the controls are big enough for a finger; the menus open with a tap and fit on the screen; and the
// page never scrolls sideways. Lying down and on a tablet the drawing keeps most of the screen. A
// narrow desktop window keeps its mouse (tests/viewer/README.md#mobile).
//
// What the page does not do yet is marked test.fixme, until the Viewer's layout for small screens lands.

import { expect, pageMoved, test, UNMOVED } from "../mobile.js";

const TOP = "04-hierarchy.sp";
// the least size of a control a finger presses, and of a row of a menu or a list
const TARGET = 44;
const ROW = 40;
// the views of the side panel the web viewer has
const TABS = ["files", "netlist", "hierarchy", "symbol", "usages", "unplaced"];

/** A box on the page, or null when the element is not shown. */
const box = (locator) => locator.boundingBox();

/** The boxes of the shown elements a locator finds, with a name for each. */
function boxes(locator) {
  return locator.evaluateAll((els) =>
    els
      .filter((e) => e.getClientRects().length && getComputedStyle(e).visibility !== "hidden")
      .map((e) => {
        const r = e.getBoundingClientRect();
        return { name: e.id || e.dataset.id || e.textContent.trim().slice(0, 20), width: r.width, height: r.height };
      }),
  );
}

/** Expect every box of `found` at least `width` by `height`. */
function bigEnough(found, width, height) {
  const small = found.filter((b) => b.width < width - 0.5 || b.height < height - 0.5);
  expect(small, `smaller than ${width}x${height}`).toEqual([]);
}

/** A corner of the drawing's canvas outside a box, or else the middle of the status bar. */
async function outside(page, b) {
  const r = await box(page.locator("#canvas"));
  const corners = [
    { x: r.x + r.width - 6, y: r.y + r.height - 6 },
    { x: r.x + 6, y: r.y + r.height - 6 },
    { x: r.x + r.width - 6, y: r.y + 6 },
    { x: r.x + 6, y: r.y + 6 },
  ];
  const inside = (p) => p.x >= b.x && p.x <= b.x + b.width && p.y >= b.y && p.y <= b.y + b.height;
  const s = await box(page.locator(".statusbar"));
  return corners.find((p) => !inside(p)) || { x: s.x + s.width / 2, y: s.y + s.height / 2 };
}

/** Expect a box within the screen. */
function onScreen(b, size) {
  expect(b.x).toBeGreaterThanOrEqual(-0.5);
  expect(b.y).toBeGreaterThanOrEqual(-0.5);
  expect(b.x + b.width).toBeLessThanOrEqual(size.width + 0.5);
  expect(b.y + b.height).toBeLessThanOrEqual(size.height + 0.5);
}

test.describe("on a phone held upright", () => {
  test.beforeEach(async ({ viewer }) => {
    await viewer.open("04-hierarchy");
  });

  test("the page never scrolls sideways, whatever view of the side panel is shown", async ({ page }) => {
    expect(await pageMoved(page)).toEqual(UNMOVED);
    for (const name of TABS) {
      await page.locator(`#tab-${name}`).tap();
      await expect(page.locator(`#pane-${name}`)).toBeVisible();
      expect(await pageMoved(page), name).toEqual(UNMOVED);
    }
  });

  test.fixme("the side panel is below the drawing, as wide as the screen, and the drawing is no lower than it", async ({ page }) => {
    const size = page.viewportSize();
    const [canvas, panel] = [await box(page.locator("#canvas")), await box(page.locator("#sidebar"))];
    expect(panel.y).toBeGreaterThanOrEqual(canvas.y + canvas.height - 1);
    expect(Math.abs(panel.width - size.width)).toBeLessThanOrEqual(1);
    expect(Math.abs(canvas.width - size.width)).toBeLessThanOrEqual(1);
    expect(canvas.height).toBeGreaterThanOrEqual(0.4 * size.height);
    expect(canvas.height).toBeGreaterThanOrEqual(panel.height - 1);
  });

  test.fixme("the side panel folds to its tabs, a tap on a tab unfolds it, and the drawing takes the room", async ({ page }) => {
    const size = page.viewportSize();
    const open = await box(page.locator("#sidebar"));
    await page.locator("#hide-sidebar").tap();
    await expect.poll(async () => (await box(page.locator("#canvas"))).height).toBeGreaterThanOrEqual(0.7 * size.height);
    // the tabs stay on the screen, big enough, and a tap on one shows its view at the height it had
    const tab = page.locator("#tab-hierarchy");
    await expect(tab).toBeVisible();
    bigEnough(await boxes(tab), TARGET, TARGET);
    await tab.tap();
    await expect(page.locator("#pane-hierarchy")).toBeVisible();
    expect(Math.abs((await box(page.locator("#sidebar"))).height - open.height)).toBeLessThanOrEqual(1);
    expect(await pageMoved(page)).toEqual(UNMOVED);
  });

  test.fixme("a finger drags the side panel's handle up and down, within limits, and the height is kept", async ({ page, viewer, fingers }) => {
    const size = page.viewportSize();
    const handle = page.locator("#vsplit");
    await expect(handle).toBeVisible();
    await expect(handle).toHaveAttribute("aria-orientation", "horizontal");
    const grip = await box(handle);
    expect(grip.width).toBeGreaterThanOrEqual(TARGET);
    expect(grip.height).toBeGreaterThanOrEqual(24);
    const height = async () => (await box(page.locator("#sidebar"))).height;
    const before = await height();
    const at = await viewer.middle(handle);
    await fingers.drag(at, { x: at.x, y: at.y - 120 });
    expect(Math.abs((await height()) - (before + 120))).toBeLessThanOrEqual(2);
    // to the top: the drawing keeps a strip, and to the bottom: the panel keeps its tabs
    await fingers.drag(await viewer.middle(handle), { x: at.x, y: 0 });
    expect((await box(page.locator("#canvas"))).height).toBeGreaterThanOrEqual(80);
    await fingers.drag(await viewer.middle(handle), { x: at.x, y: size.height - 1 });
    await expect(page.locator(".view-tabs")).toBeVisible();
    const lowest = await height();
    expect(lowest).toBeGreaterThanOrEqual(TARGET);
    // the next start has it
    await viewer.open("04-hierarchy");
    expect(Math.abs((await height()) - lowest)).toBeLessThanOrEqual(1);
    expect(await pageMoved(page)).toEqual(UNMOVED);
  });

  test.fixme("the menu titles, the side panel's tabs and the buttons are big enough for a finger", async ({ page, viewer }) => {
    bigEnough(await boxes(page.locator("#menubar .menu-title")), TARGET, TARGET);
    bigEnough(await boxes(page.locator(".view-tabs [role=tab]")), TARGET, TARGET);
    bigEnough(await boxes(page.locator("#hide-sidebar")), TARGET, TARGET);
    bigEnough(await boxes(page.locator("#problems-head")), TARGET, TARGET);
    // the open badge of a selected subcircuit instance and the up button of a level below
    await viewer.click("X3");
    bigEnough(await boxes(page.locator("#enter-badge")), 32, 32);
    await page.locator("#enter-badge").tap();
    await expect(page.locator("#up")).toBeVisible();
    bigEnough(await boxes(page.locator("#up")), TARGET, TARGET);
  });

  test("every menu opens with a tap and fits on the screen, and a tap elsewhere only closes it", async ({ page, viewer, fingers }) => {
    const size = page.viewportSize();
    await fingers.tap(await viewer.where("X2"));
    await viewer.selected(`instance X2 in ${TOP}`);
    const titles = page.locator("#menubar .menu-title");
    const count = await titles.count();
    expect(count).toBeGreaterThan(3);
    for (let i = 0; i < count; i++) {
      await titles.nth(i).tap();
      const list = page.locator("#menubar .menu").nth(i).locator(":scope > .menu-list");
      await expect(list).toBeVisible();
      const open = await box(list);
      onScreen(open, size);
      // a tap on the drawing outside the menu closes it, and neither selects nor clears, as a click does
      await fingers.tap(await outside(page, open));
      await expect(list).toBeHidden();
      await viewer.selected(`instance X2 in ${TOP}`);
    }
  });

  test.fixme("the menus' items are big enough for a finger, and a submenu fits on the screen", async ({ page, viewer }) => {
    const size = page.viewportSize();
    await viewer.menuTitle("View").tap();
    bigEnough(await boxes(page.locator("#menubar .menu-list .menu-item")), TARGET, ROW);
    // a submenu, which a desktop opens beside its entry
    await page.locator("#menubar .opens", { hasText: "Background" }).tap();
    const submenu = page.locator("#menubar .menu-sub", { has: page.locator(".opens", { hasText: "Background" }) }).locator(".submenu");
    await expect(submenu).toBeVisible();
    onScreen(await box(submenu), size);
    // a tap on an item runs it
    await viewer.menuItem("background-dark").tap();
    await expect(page.locator("body")).toHaveAttribute("data-background", "dark");
  });

  test("a tap on a menu item runs its command", async ({ page, viewer, browserName }) => {
    test.fixme(browserName === "webkit", "WebKit sends no click after a pointerdown whose default is prevented, as a menu item's is");
    const fitted = await viewer.viewBox();
    await viewer.menuTitle("View").tap();
    await viewer.menuItem("zoom-in").tap();
    await expect.poll(() => viewer.viewBox()).not.toBe(fitted);
    await expect(page.locator("#menubar .menu-list").first()).toBeHidden();
  });

  test("the menu bar, the level bar and the status bar fit in the screen's width", async ({ page }) => {
    await barsFit(page);
  });

  test.fixme("the status bar fits in the screen's width with an object selected, which it names", async ({ page, viewer, fingers }) => {
    await fingers.tap(await viewer.where("X3"));
    await viewer.selected(`instance X3 in ${TOP}`);
    await barsFit(page);
  });
});

/**
 * Expect the menu bar, the level bar and the status bar within the screen's width, with what does
 * not fit left out or cut short in the bar, not pushed past the screen's edge; and the page unmoved.
 */
async function barsFit(page) {
  const size = page.viewportSize();
  for (const sel of ["#menubar", ".level-bar", ".statusbar"]) {
    const b = await box(page.locator(sel));
    expect(b.x + b.width, sel).toBeLessThanOrEqual(size.width + 0.5);
    const over = await page.locator(sel).evaluate((e) => e.scrollWidth - e.clientWidth);
    expect(over, sel).toBeLessThanOrEqual(1);
  }
  expect(await pageMoved(page)).toEqual(UNMOVED);
}

test.describe("on a phone lying down", () => {
  // iPhone 15's screen lying down, the lowest of the phones
  test.use({ viewport: { width: 734, height: 343 } });

  test.beforeEach(async ({ viewer }) => {
    await viewer.open("04-hierarchy");
  });

  test("the page never scrolls sideways", async ({ page }) => {
    expect(await pageMoved(page)).toEqual(UNMOVED);
  });

  test.fixme("the drawing has most of the screen, and all of it with the side panel folded", async ({ page }) => {
    const size = page.viewportSize();
    const canvas = await box(page.locator("#canvas"));
    expect(canvas.width * canvas.height).toBeGreaterThanOrEqual(0.45 * size.width * size.height);
    expect(canvas.height).toBeGreaterThanOrEqual(0.55 * size.height);
    await page.locator("#hide-sidebar").tap();
    const folded = await box(page.locator("#canvas"));
    expect(Math.abs(folded.width - size.width)).toBeLessThanOrEqual(1);
    expect(folded.height).toBeGreaterThanOrEqual(0.55 * size.height);
  });
});

for (const [name, viewport] of [["upright", { width: 768, height: 1024 }], ["lying down", { width: 1024, height: 768 }]]) {
  test.describe(`on a tablet ${name}`, () => {
    // iPad Mini's screen
    test.use({ viewport });

    test.beforeEach(async ({ viewer }) => {
      await viewer.open("04-hierarchy");
    });

    test("the page never scrolls sideways", async ({ page }) => {
      expect(await pageMoved(page)).toEqual(UNMOVED);
    });

    test.fixme("the drawing has at least half the screen, and the controls are big enough for a finger", async ({ page }) => {
      const canvas = await box(page.locator("#canvas"));
      expect(canvas.width * canvas.height).toBeGreaterThanOrEqual(0.5 * viewport.width * viewport.height);
      bigEnough(await boxes(page.locator("#menubar .menu-title")), TARGET, TARGET);
      bigEnough(await boxes(page.locator(".view-tabs [role=tab]")), TARGET, TARGET);
    });
  });
}

test.describe("in a narrow desktop window", () => {
  test.use({ isMobile: false, hasTouch: false, viewport: { width: 420, height: 800 }, deviceScaleFactor: 1 });

  test.beforeEach(async ({ viewer }) => {
    await viewer.open("04-hierarchy");
  });

  test("the mouse drags and zooms the drawing", async ({ page, viewer }) => {
    const before = (await viewer.viewBox()).split(" ").map(Number);
    const from = await viewer.where("X1");
    await viewer.drag(from, { x: from.x + 40, y: from.y + 30 });
    await expect.poll(() => viewer.viewBox()).not.toBe(before.join(" "));
    await page.mouse.wheel(0, -300);
    await expect.poll(async () => (await viewer.viewBox()).split(" ").map(Number)[2]).toBeLessThan(before[2]);
  });

  test.fixme("the mouse drags the side panel's handle", async ({ page, viewer }) => {
    const height = async () => (await box(page.locator("#sidebar"))).height;
    const before = await height();
    const at = await viewer.middle(page.locator("#vsplit"));
    await viewer.drag(at, { x: at.x, y: at.y - 100 });
    expect(Math.abs((await height()) - (before + 100))).toBeLessThanOrEqual(2);
  });
});
