// The page on a phone and a tablet (the projects mobile-chromium and mobile-webkit). On a phone,
// upright or lying down, the drawing and the side panel are two pages, each the whole screen above a
// low switch bar, `#page-switch`: switching keeps each page as it was, Show in Netlist and Show in
// Drawing go from one to the other, and opening a level or a file goes to the drawing. On a tablet
// upright the side panel is below the drawing, folds to its tabs and drags taller or lower; lying
// down it is beside the drawing. Everywhere the controls are big enough for a finger, the menus open
// with a tap and fit on the screen, the netlist shows lines of the file, and the page never scrolls
// sideways. A narrow desktop window keeps its mouse (tests/viewer/README.md#mobile).
//
// What the page does not do yet is marked test.fixme, until the Viewer's layout for small screens lands.

import { canvasMiddle, expect, pageMoved, test, UNMOVED } from "../mobile.js";

const TOP = "04-hierarchy.sp";
// the least size of a control a finger presses, and of a row of a menu or a list
const TARGET = 44;
const ROW = 40;
// the highest the switch bar of a phone's two pages may be
const SWITCH_BAR = 48;
// the views of the side panel the web viewer has
const TABS = ["files", "netlist", "hierarchy", "symbol", "usages", "unplaced"];
// an iPad Mini's screen, upright and lying down
const TABLET = { upright: { width: 768, height: 1024 }, lying: { width: 1024, height: 768 } };

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

/** Show a page of a phone's two pages, "drawing" or "panel", by a tap on its switch. */
async function toPage(page, name) {
  await page.locator(`#page-${name}`).tap();
  await expect(page.locator(`#page-${name}`)).toHaveAttribute("aria-selected", "true");
}

/**
 * Show a view of the side panel by a tap on its tab: on a phone, on the panel's page; where the
 * page has no switch of pages, the side panel is on the screen with the drawing.
 */
async function showView(page, name) {
  if (await page.locator("#page-switch").isVisible()) await toPage(page, "panel");
  await page.locator(`#tab-${name}`).tap();
  await expect(page.locator(`#pane-${name}`)).toBeVisible();
}

/** How many lines of the file the netlist shows whole. */
function netlistLines(page) {
  return page.evaluate(() => {
    const r = document.getElementById("source").getBoundingClientRect();
    return [...document.querySelectorAll("#source .file:not([hidden]) .ln")].filter((e) => {
      const b = e.getBoundingClientRect();
      return b.height > 0 && b.top >= r.top - 0.5 && b.bottom <= r.bottom + 0.5;
    }).length;
  });
}

/**
 * Expect a phone's two pages: a low switch bar at the foot of the screen, as wide as it, with a
 * switch for each page big enough for a finger, and the status bar above it on both pages; the
 * drawing's page first, the drawing as wide as the screen, the side panel hidden and Show in Netlist
 * there; then the panel's page, the side panel as wide as the screen, the drawing hidden and Show in
 * Drawing there. Nothing goes under the bar; the side panel's buttons and handle of a wider screen
 * are not there; `#app` names the page in `data-page`; and the page does not move.
 */
async function twoPages(page) {
  const size = page.viewportSize();
  const bar = page.locator("#page-switch");
  await expect(bar).toBeVisible();
  const b = await box(bar);
  expect(Math.abs(b.y + b.height - size.height)).toBeLessThanOrEqual(1);
  expect(Math.abs(b.width - size.width)).toBeLessThanOrEqual(1);
  expect(b.height).toBeLessThanOrEqual(SWITCH_BAR);
  bigEnough(await boxes(bar.locator("[role=tab]")), TARGET, ROW);
  for (const sel of ["#hide-sidebar", "#show-sidebar", "#vsplit"]) await expect(page.locator(sel), sel).toBeHidden();
  const above = async (sel) => {
    const r = await box(page.locator(sel));
    expect(r.y + r.height, sel).toBeLessThanOrEqual(b.y + 0.5);
  };

  await expect(page.locator("#page-drawing")).toHaveAttribute("aria-selected", "true");
  await expect(page.locator("#app")).toHaveAttribute("data-page", "drawing");
  await expect(page.locator("#sidebar")).toBeHidden();
  await expect(page.locator("#show-in-netlist")).toBeVisible();
  await expect(page.locator("#show-in-drawing")).toBeHidden();
  const canvas = await box(page.locator("#canvas"));
  expect(Math.abs(canvas.width - size.width)).toBeLessThanOrEqual(1);
  await above("#canvas");
  await above(".statusbar");

  await toPage(page, "panel");
  await expect(page.locator("#page-drawing")).toHaveAttribute("aria-selected", "false");
  await expect(page.locator("#app")).toHaveAttribute("data-page", "panel");
  await expect(page.locator("#canvas")).toBeHidden();
  await expect(page.locator("#show-in-drawing")).toBeVisible();
  await expect(page.locator("#show-in-netlist")).toBeHidden();
  const panel = await box(page.locator("#sidebar"));
  expect(Math.abs(panel.width - size.width)).toBeLessThanOrEqual(1);
  await above("#sidebar");
  await above(".statusbar");
  expect(await pageMoved(page)).toEqual(UNMOVED);
}

test.describe("on a phone held upright", () => {
  test.beforeEach(async ({ viewer }) => {
    await viewer.open("04-hierarchy");
  });

  test("the page never scrolls sideways, whatever view of the side panel is shown", async ({ page }) => {
    expect(await pageMoved(page)).toEqual(UNMOVED);
    for (const name of TABS) {
      await showView(page, name);
      expect(await pageMoved(page), name).toEqual(UNMOVED);
    }
  });

  test("the netlist shows lines of the file beside the problems", async ({ page }) => {
    await showView(page, "netlist");
    await expect(page.locator("#problems-body")).toBeVisible();
    expect(await netlistLines(page)).toBeGreaterThanOrEqual(5);
  });

  test.fixme("the drawing and the side panel are two pages, each the whole screen above a low switch bar", async ({ page }) => {
    await twoPages(page);
  });

  test.fixme("switching pages keeps the view, the selection, the panel's view and where the netlist is scrolled", async ({ page, viewer, fingers }) => {
    await fingers.tap(await viewer.where("X2"));
    await fingers.pinch(await viewer.where("X2"), 60, 120);
    const view = await viewer.viewBox();
    await toPage(page, "panel");
    await expect(page.locator("#pane-netlist")).toBeVisible();
    const scrolled = await page.locator("#source").evaluate((e) => (e.scrollTop = Math.floor(e.scrollHeight / 3)));
    expect(scrolled).toBeGreaterThan(0);
    await toPage(page, "drawing");
    expect(await viewer.viewBox()).toBe(view);
    await viewer.selected(`instance X2 in ${TOP}`);
    await toPage(page, "panel");
    await expect(page.locator("#pane-netlist")).toBeVisible();
    expect(await page.locator("#source").evaluate((e) => e.scrollTop)).toBe(scrolled);
    // and the view of the side panel chosen
    await page.locator("#tab-hierarchy").tap();
    await toPage(page, "drawing");
    await toPage(page, "panel");
    await expect(page.locator("#pane-hierarchy")).toBeVisible();
  });

  test.fixme("Show in Netlist shows the statement of what the drawing has selected, on the panel's page", async ({ page, viewer, fingers }) => {
    // the netlist at its top, where X2's statement is not
    await toPage(page, "panel");
    await page.locator("#source").evaluate((e) => (e.scrollTop = 0));
    await toPage(page, "drawing");
    // there is nothing to show until something is selected
    await expect(page.locator("#show-in-netlist")).toBeDisabled();
    await fingers.tap(await viewer.where("X2"));
    await expect(page.locator("#show-in-netlist")).toBeEnabled();
    await page.locator("#show-in-netlist").tap();
    await expect(page.locator("#page-panel")).toHaveAttribute("aria-selected", "true");
    await expect(page.locator("#pane-netlist")).toBeVisible();
    expect(await viewer.markedLines()).toEqual([50]);
    await expect.poll(() => viewer.within(viewer.line(50))).toBe(true);
  });

  test.fixme("Show in Drawing shows what the netlist has selected, on the drawing's page", async ({ page, viewer, fingers }) => {
    // the view moved away from X2
    const r = await box(page.locator("#canvas"));
    const middle = await canvasMiddle(page);
    await fingers.drag(middle, { x: middle.x + r.width, y: middle.y });
    await expect(viewer.where("X2")).rejects.toThrow(/outside the view/);
    await toPage(page, "panel");
    await expect(page.locator("#show-in-drawing")).toBeDisabled();
    const x2 = await viewer.object("instance", "X2", TOP);
    await viewer.places(x2).first().tap();
    await viewer.selected(`instance X2 in ${TOP}`);
    await page.locator("#show-in-drawing").tap();
    await expect(page.locator("#page-drawing")).toHaveAttribute("aria-selected", "true");
    await viewer.selected(`instance X2 in ${TOP}`);
    // in the view again: where() throws when it is not
    await viewer.where("X2");
  });

  test.fixme("opening a level from the panel's page, or a file, goes to the drawing's page", async ({ page, viewer }) => {
    await showView(page, "hierarchy");
    await page.locator("#index .index-row", { hasText: "rdiv" }).tap();
    await expect(page.locator("#level-name")).toHaveText("rdiv");
    await expect(page.locator("#page-drawing")).toHaveAttribute("aria-selected", "true");
    await expect(page.locator("#canvas")).toBeVisible();
    // a file opened while the panel's page is shown
    await toPage(page, "panel");
    await page.evaluate(() => window.fakeShell.picks.push("01-rc-lowpass"));
    await viewer.build(() => page.evaluate(() => window.viewer.command("open")));
    await expect(page.locator("#page-drawing")).toHaveAttribute("aria-selected", "true");
    await expect(page.locator("#canvas")).toBeVisible();
  });

  test.fixme("the side panel's commands go to the panel's page, and hiding the side panel to the drawing's", async ({ page }) => {
    await page.evaluate(() => window.viewer.command("panel-hierarchy"));
    await expect(page.locator("#app")).toHaveAttribute("data-page", "panel");
    await expect(page.locator("#pane-hierarchy")).toBeVisible();
    await page.evaluate(() => window.viewer.command("panel-hide"));
    await expect(page.locator("#app")).toHaveAttribute("data-page", "drawing");
    await expect(page.locator("#canvas")).toBeVisible();
  });

  test.fixme("the netlist's options fold behind a button, which a tap opens", async ({ page }) => {
    await showView(page, "netlist");
    await expect(page.locator("#auto-include")).toBeHidden();
    const button = page.locator("#netlist-options-button");
    bigEnough(await boxes(button), TARGET, TARGET);
    await button.tap();
    await expect(page.locator("#auto-include")).toBeVisible();
  });

  test.fixme("the menu titles, the side panel's tabs and the buttons are big enough for a finger", async ({ page, viewer, fingers }) => {
    bigEnough(await boxes(page.locator("#menubar .menu-title")), TARGET, TARGET);
    // the open badge of a selected subcircuit instance and the up button of a level below
    await fingers.tap(await viewer.where("X3"));
    bigEnough(await boxes(page.locator("#enter-badge")), 32, 32);
    await page.locator("#enter-badge").tap();
    await expect(page.locator("#up")).toBeVisible();
    bigEnough(await boxes(page.locator("#up")), TARGET, TARGET);
    await showView(page, "netlist");
    bigEnough(await boxes(page.locator(".view-tabs [role=tab]")), TARGET, TARGET);
    bigEnough(await boxes(page.locator("#problems-head")), TARGET, TARGET);
    // the tabs' number keys are for a keyboard
    await expect(page.locator(".view-tabs .tab-key").first()).toBeHidden();
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
    // and how to open it is said for a finger, not for keys
    await expect(page.locator("#selection")).not.toContainText("Enter");
  });
});

test.describe("on a phone lying down", () => {
  // iPhone 15's screen lying down, the lowest of the phones
  test.use({ viewport: { width: 734, height: 343 } });

  test.beforeEach(async ({ viewer }) => {
    await viewer.open("04-hierarchy");
  });

  test("the page never scrolls sideways", async ({ page }) => {
    expect(await pageMoved(page)).toEqual(UNMOVED);
  });

  test.fixme("the drawing and the side panel are two pages here too, and the drawing has half the height", async ({ page }) => {
    const size = page.viewportSize();
    expect((await box(page.locator("#canvas"))).height).toBeGreaterThanOrEqual(0.5 * size.height);
    await twoPages(page);
  });
});

test.describe("on a tablet upright", () => {
  test.use({ viewport: TABLET.upright });

  test.beforeEach(async ({ viewer }) => {
    await viewer.open("04-hierarchy");
  });

  test("the page never scrolls sideways", async ({ page }) => {
    expect(await pageMoved(page)).toEqual(UNMOVED);
  });

  test("the netlist shows lines of the file beside the problems", async ({ page }) => {
    await showView(page, "netlist");
    await expect(page.locator("#problems-body")).toBeVisible();
    expect(await netlistLines(page)).toBeGreaterThanOrEqual(5);
  });

  test.fixme("the side panel is below the drawing, as wide as the screen, with no switch of pages, and the drawing is no lower than it", async ({ page }) => {
    const size = page.viewportSize();
    await expect(page.locator("#page-switch")).toBeHidden();
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

  test.fixme("the controls are big enough for a finger", async ({ page }) => {
    bigEnough(await boxes(page.locator("#menubar .menu-title")), TARGET, TARGET);
    bigEnough(await boxes(page.locator(".view-tabs [role=tab]")), TARGET, TARGET);
  });
});

test.describe("on a tablet lying down", () => {
  test.use({ viewport: TABLET.lying });

  test.beforeEach(async ({ viewer }) => {
    await viewer.open("04-hierarchy");
  });

  test("the page never scrolls sideways", async ({ page }) => {
    expect(await pageMoved(page)).toEqual(UNMOVED);
  });

  test.fixme("the side panel is beside the drawing, with no switch of pages, and the controls are big enough for a finger", async ({ page }) => {
    const size = page.viewportSize();
    await expect(page.locator("#page-switch")).toBeHidden();
    const [canvas, panel] = [await box(page.locator("#canvas")), await box(page.locator("#sidebar"))];
    expect(panel.x + panel.width).toBeLessThanOrEqual(canvas.x + 1);
    expect(canvas.width * canvas.height).toBeGreaterThanOrEqual(0.45 * size.width * size.height);
    bigEnough(await boxes(page.locator("#menubar .menu-title")), TARGET, TARGET);
    bigEnough(await boxes(page.locator(".view-tabs [role=tab]")), TARGET, TARGET);
  });
});

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

  test.fixme("the mouse switches between the drawing's page and the panel's", async ({ page }) => {
    await page.locator("#page-panel").click();
    await expect(page.locator("#sidebar")).toBeVisible();
    await expect(page.locator("#canvas")).toBeHidden();
    await page.locator("#page-drawing").click();
    await expect(page.locator("#canvas")).toBeVisible();
  });
});
