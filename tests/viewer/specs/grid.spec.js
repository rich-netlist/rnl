// The grid under the drawing: dots or lines at the integer points of the scene, coarser where they
// would be dense, the axes through the origin, its settings in View > Grid and their dialog, kept for
// the next start; it takes no clicks and changes neither fitting nor the drawing. And the pointer's
// place in the scene, in the status bar. The pixels are read from the grid's canvas.

import { expect, test } from "../viewer.js";

const TOP = "04-hierarchy.sp";
const SCALE = 14; // pixels of the SVG per unit, as the core draws it

/** The colour of the grid's canvas at a page point, as [r, g, b, a]; [0, 0, 0, 0] where nothing is drawn. */
function pixel(page, { x, y }) {
  return page.evaluate(([x, y]) => {
    const grid = document.getElementById("grid");
    const r = grid.getBoundingClientRect();
    const k = grid.width / r.width;
    return [...grid.getContext("2d").getImageData(Math.floor((x - r.left) * k), Math.floor((y - r.top) * k), 1, 1).data];
  }, [x, y]);
}

/** A colour of style.css, such as "--grid-dot", as [r, g, b, a]. */
async function colour(page, name) {
  const hex = await page.evaluate((n) => getComputedStyle(document.getElementById("grid")).getPropertyValue(n).trim(), name);
  return [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16)).concat(255);
}

/** The page point of a point of the scene, in units with y up. */
const at = (viewer, u, v) => viewer.point(u * SCALE, -v * SCALE);

/** How many pixels of the grid's canvas are drawn. */
function drawnPixels(page) {
  return page.evaluate(() => {
    const grid = document.getElementById("grid");
    const data = grid.getContext("2d").getImageData(0, 0, grid.width, grid.height).data;
    let n = 0;
    for (let i = 3; i < data.length; i += 4) if (data[i] > 0) n += 1;
    return n;
  });
}

const NONE = [0, 0, 0, 0];

test.beforeEach(async ({ viewer }) => {
  await viewer.open("04-hierarchy");
});

test("the grid is dots at the integer points by default, a pin and the origin on them", async ({ page, viewer }) => {
  expect(await page.evaluate(() => window.viewer.grid)).toMatchObject({ shown: true, style: "dots", spacing: "auto", axes: false, step: 1 });
  const dot = await colour(page, "--grid-dot");
  // X2's pin "in" is at (10, 0): the core draws it at 140 pixels of the SVG, and a dot is there
  const pins = await page.evaluate(() => window.viewer.doc.levels[0].hits.filter((h) => h.kind === "pin").map((h) => h.points[0]));
  expect(pins.some(([x, y]) => x === 10 * SCALE && y === 0)).toBe(true);
  expect(await pixel(page, await at(viewer, 10, 0))).toEqual(dot);
  // X1 is placed at the origin, on a dot, and so are the points one unit away
  for (const [u, v] of [[0, 0], [1, 0], [0, 1], [-3, -2], [17, 4]]) expect(await pixel(page, await at(viewer, u, v))).toEqual(dot);
  // between them nothing is drawn
  for (const [u, v] of [[0.5, 0.5], [10.5, 0], [10, 0.5], [-2.5, 3.5]]) expect(await pixel(page, await at(viewer, u, v))).toEqual(NONE);
  // and the canvas is white under it, the SVG's own white backdrop left out
  expect(await page.locator("#canvas > svg > rect").first().evaluate((r) => getComputedStyle(r).display)).toBe("none");

  // the dots keep their place and size as the view moves and zooms
  for (const key of ["Control+=", "Control+=", "Control+-", "Control+-", "Control+-"]) {
    await page.keyboard.press(key);
    await expect.poll(async () => pixel(page, await at(viewer, 10, 0))).toEqual(dot);
    expect(await pixel(page, await at(viewer, 10.5, 0.5))).toEqual(NONE);
  }
  const p = await viewer.where("X1");
  await viewer.drag(p, { x: p.x + 37, y: p.y - 23 });
  expect(await pixel(page, await at(viewer, 10, 0))).toEqual(dot);
  expect(await pixel(page, await at(viewer, 0, 0))).toEqual(dot);
});

test("lines have a darker line every fifth, the axes cross at the origin, and None and G hide the grid", async ({ page, viewer }) => {
  const [line, major, axis] = [await colour(page, "--grid-line"), await colour(page, "--grid-major"), await colour(page, "--grid-axis")];
  expect(new Set([line, major, axis].map(String)).size).toBe(3);
  await page.evaluate(() => window.viewer.command("grid-lines"));
  expect(await page.evaluate(() => window.viewer.grid)).toMatchObject({ style: "lines", step: 1, majorStep: 5 });
  // x = 1, 2 and so on are lines; x = 5 and x = -10 major ones; y likewise
  expect(await pixel(page, await at(viewer, 1, 0.5))).toEqual(line);
  expect(await pixel(page, await at(viewer, 3, 0.5))).toEqual(line);
  expect(await pixel(page, await at(viewer, 5, 0.5))).toEqual(major);
  expect(await pixel(page, await at(viewer, -10, 2.5))).toEqual(major);
  expect(await pixel(page, await at(viewer, 0.5, -5))).toEqual(major);
  expect(await pixel(page, await at(viewer, 0.5, 0.5))).toEqual(NONE);
  // without the axes the origin's lines are major lines too
  expect(await pixel(page, await at(viewer, 0, 0.5))).toEqual(major);

  // the axes, a little stronger, through the origin where X1 is placed, across the whole view
  await page.evaluate(() => window.viewer.command("grid-axes"));
  for (const [u, v] of [[0, 0], [0, 0.5], [0, -2.5], [12.5, 0], [-6.5, 0]]) expect(await pixel(page, await at(viewer, u, v))).toEqual(axis);
  const box = await page.locator("#canvas").boundingBox();
  const origin = await at(viewer, 0, 0);
  expect(await pixel(page, { x: origin.x, y: box.y + 1 })).toEqual(axis);
  expect(await pixel(page, { x: origin.x, y: box.y + box.height - 1 })).toEqual(axis);
  expect(await pixel(page, { x: box.x + 1, y: origin.y })).toEqual(axis);
  expect(await pixel(page, { x: box.x + box.width - 1, y: origin.y })).toEqual(axis);

  // None leaves only the axes
  await page.evaluate(() => window.viewer.command("grid-none"));
  expect(await page.evaluate(() => window.viewer.grid)).toMatchObject({ shown: false, style: "lines", step: null });
  expect(await pixel(page, await at(viewer, 5, 0.5))).toEqual(NONE);
  expect(await pixel(page, await at(viewer, 0, 0.5))).toEqual(axis);
  // G shows the grid as it was, and hides it again
  await page.keyboard.press("g");
  expect(await page.evaluate(() => window.viewer.grid)).toMatchObject({ shown: true, style: "lines", step: 1 });
  expect(await pixel(page, await at(viewer, 5, 0.5))).toEqual(major);
  await page.evaluate(() => window.viewer.command("grid-axes"));
  await page.keyboard.press("G");
  expect(await page.evaluate(() => window.viewer.grid)).toMatchObject({ shown: false, axes: false });
  expect(await drawnPixels(page)).toBe(0);

  // in a text box G is typed
  await page.locator("#tab-hierarchy").click();
  await page.locator("#search").click();
  await page.keyboard.type("g");
  await expect(page.locator("#search")).toHaveValue("g");
  expect(await page.evaluate(() => window.viewer.grid.shown)).toBe(false);
});

test("zoomed out, the grid takes 2, 5, 10, 20, 50 units and so on, 10 pixels apart or more", async ({ page, viewer }) => {
  /** The grid drawn, and the pixels of one unit. */
  const now = () =>
    page.evaluate(() => {
      const [, , w] = document.querySelector("#canvas > svg").getAttribute("viewBox").split(" ").map(Number);
      return { ...window.viewer.grid, unit: (document.getElementById("canvas").clientWidth / w) * 14 };
    });
  const sequence = [1, 2, 5, 10, 20, 50, 100, 200, 500];
  // the major lines: every fifth line, or the next step of the sequence that is a multiple of the step
  const majors = { 1: 5, 2: 10, 5: 50, 10: 50, 20: 100, 50: 500 };
  const seen = new Set();
  for (const style of ["dots", "lines"]) {
    await page.keyboard.press("f");
    await page.evaluate((s) => window.viewer.command(`grid-${s}`), style);
    for (let i = 0; i < 25; i++) {
      const g = await now();
      expect(sequence).toContain(g.step);
      // the least step of the sequence that keeps 10 pixels between neighbours
      expect(g.step * g.unit).toBeGreaterThanOrEqual(10 - 1e-9);
      if (g.step > 1) expect(sequence[sequence.indexOf(g.step) - 1] * g.unit).toBeLessThan(10);
      if (style === "lines") expect(g.majorStep).toBe(majors[g.step]);
      seen.add(g.step);
      // the dots or lines drawn are that far apart: one at the origin, the next a step away
      const [a, b] = [await at(viewer, 0, 0), await at(viewer, g.step, 0)];
      expect(b.x - a.x).toBeGreaterThanOrEqual(10 - 1e-6);
      if (style === "dots") {
        const dot = await colour(page, "--grid-dot");
        const inView = await page.locator("#canvas").boundingBox();
        if (a.x > inView.x + 2 && b.x < inView.x + inView.width - 2 && a.y > inView.y + 2 && a.y < inView.y + inView.height - 2) {
          expect(await pixel(page, a)).toEqual(dot);
          expect(await pixel(page, b)).toEqual(dot);
          expect(await pixel(page, { x: (a.x + b.x) / 2, y: a.y })).toEqual(NONE);
        }
      }
      await page.keyboard.press("Control+-");
    }
  }
  expect([...seen].sort((x, y) => x - y)).toEqual([1, 2, 5, 10, 20, 50]);
});

test("the settings dialog sets the style, a spacing, the major lines and the axes, and checks them", async ({ page, viewer }) => {
  const dialog = page.locator("dialog.dialog");
  const open = async () => {
    await viewer.menuTitle("View").click();
    await page.locator("#menubar .menu-item.opens", { hasText: "Grid" }).hover();
    await viewer.menuItem("grid-settings").click();
    await expect(dialog.locator(".dialog-title")).toHaveText("Grid Settings");
  };
  await open();
  // the settings as they are, the major lines only for lines
  await expect(dialog.getByRole("radio", { name: "Dots" })).toBeChecked();
  await expect(dialog.getByRole("radio", { name: /^Auto/ })).toBeChecked();
  await expect(dialog.getByRole("spinbutton", { name: "Fixed spacing in units" })).toHaveValue("1");
  const major = dialog.getByRole("spinbutton", { name: /major line/ });
  await expect(major).toHaveValue("5");
  await expect(major).toBeDisabled();
  await expect(dialog.getByRole("checkbox", { name: "Axes through the origin" })).not.toBeChecked();
  // Cancel changes nothing
  await dialog.getByRole("radio", { name: "Lines" }).check();
  await dialog.getByRole("button", { name: "Cancel" }).click();
  await expect(dialog).toHaveCount(0);
  expect(await page.evaluate(() => window.viewer.grid.style)).toBe("dots");

  await open();
  await dialog.getByRole("radio", { name: "Lines" }).check();
  await expect(major).toBeEnabled();
  // typing a spacing chooses it; one that will not do keeps the dialog, and says why
  const units = dialog.getByRole("spinbutton", { name: "Fixed spacing in units" });
  await units.fill("0");
  await expect(dialog.getByRole("radio", { name: /^Fixed/ })).toBeChecked();
  await dialog.getByRole("button", { name: "OK" }).click();
  await expect(dialog.locator(".dialog-error")).toHaveText(/spacing is a number of units above 0/);
  await expect(units).toBeFocused();
  await units.fill("2");
  await major.fill("1");
  await major.press("Enter");
  await expect(dialog.locator(".dialog-error")).toHaveText(/A major line comes every 2 to 100 lines/);
  await major.fill("4");
  await dialog.getByRole("checkbox", { name: "Axes through the origin" }).check();
  await major.press("Enter");
  await expect(dialog).toHaveCount(0);
  // every fourth line would be at 8 units, which is no step of 2, 4, 10, 20 ... times the spacing: 10
  expect(await page.evaluate(() => window.viewer.grid)).toMatchObject({
    shown: true, style: "lines", spacing: 2, major: 4, axes: true, step: 2, majorStep: 10,
  });
  const [line, major10, axis] = [await colour(page, "--grid-line"), await colour(page, "--grid-major"), await colour(page, "--grid-axis")];
  expect(await pixel(page, await at(viewer, 2, 0.5))).toEqual(line);
  expect(await pixel(page, await at(viewer, 1, 0.5))).toEqual(NONE);
  expect(await pixel(page, await at(viewer, 10, 0.5))).toEqual(major10);
  expect(await pixel(page, await at(viewer, 0, 0.5))).toEqual(axis);

  // None in the dialog hides the grid and keeps its style; Escape cancels
  await open();
  await dialog.getByRole("radio", { name: "None" }).check();
  await dialog.getByRole("radio", { name: /^Auto/ }).check();
  await dialog.getByRole("button", { name: "OK" }).click();
  await expect(dialog).toHaveCount(0);
  expect(await page.evaluate(() => window.viewer.grid)).toMatchObject({ shown: false, style: "lines", spacing: "auto", step: null });
  await open();
  await expect(dialog.getByRole("radio", { name: "None" })).toBeChecked();
  await page.keyboard.press("Escape");
  await expect(dialog).toHaveCount(0);
});

test("the settings are kept for the next start", async ({ page, viewer }) => {
  await page.evaluate(() => {
    window.viewer.command("grid-lines");
    window.viewer.command("grid-axes");
  });
  await page.evaluate(() => window.viewer.command("grid-settings"));
  await page.locator("dialog.dialog").getByRole("radio", { name: /^Fixed/ }).check();
  await page.locator("dialog.dialog").getByRole("spinbutton", { name: "Fixed spacing in units" }).fill("5");
  await page.locator("dialog.dialog").getByRole("button", { name: "OK" }).click();
  await expect(page.locator("dialog.dialog")).toHaveCount(0);
  const kept = { "grid-shown": true, "grid-style": "lines", "grid-spacing": 5, "grid-major": 5, "grid-axes": true };
  for (const [key, value] of Object.entries(kept)) expect(await viewer.stored(key)).toBe(value);

  await viewer.open("04-hierarchy");
  expect(await page.evaluate(() => window.viewer.grid)).toMatchObject({ shown: true, style: "lines", spacing: 5, axes: true, step: 5 });
  expect(await pixel(page, await at(viewer, 5, 0.5))).toEqual(await colour(page, "--grid-line"));
  // and shown in the menu
  await viewer.menuTitle("View").click();
  await page.locator("#menubar .menu-item.opens", { hasText: "Grid" }).hover();
  for (const [id, checked] of [["toggle-grid", "true"], ["grid-none", "false"], ["grid-dots", "false"], ["grid-lines", "true"], ["grid-axes", "true"]]) {
    await expect(viewer.menuItem(id)).toHaveAttribute("aria-checked", checked);
  }

  // a kept value that will not do leaves the default
  await page.evaluate(() => {
    localStorage.setItem("grid-style", JSON.stringify("hatched"));
    localStorage.setItem("grid-spacing", JSON.stringify(-3));
  });
  await viewer.open("04-hierarchy");
  expect(await page.evaluate(() => window.viewer.grid)).toMatchObject({ style: "dots", spacing: "auto", axes: true });
});

test("the grid takes no clicks, and changes neither fitting nor the drawing", async ({ page, viewer }) => {
  await page.evaluate(() => window.viewer.command("grid-none"));
  const fitted = await viewer.viewBox();
  const children = () => page.locator("#canvas > svg").evaluate((s) => s.children.length);
  const drawing = await children();
  await page.evaluate(() => {
    window.viewer.command("grid-lines");
    window.viewer.command("grid-axes");
  });
  await page.keyboard.press("Control+=");
  await page.keyboard.press("f");
  expect(await viewer.viewBox()).toBe(fitted);
  expect(await children()).toBe(drawing);
  // the core's SVG, its groups as the Document has them
  const groups = await page.evaluate(() => window.viewer.doc.levels[0].svg.split("<g data-id=").length - 1);
  await expect(page.locator("#canvas > svg g[data-id]")).toHaveCount(groups);
  // what a click there hits is the drawing
  const origin = await at(viewer, 0, 0);
  expect(await page.evaluate(([x, y]) => document.elementFromPoint(x, y).closest("svg") !== null, [origin.x, origin.y])).toBe(true);
  await viewer.click("X2");
  await viewer.selected(`instance X2 in ${TOP}`);
  const empty = await viewer.empty();
  await page.mouse.click(empty.x, empty.y);
  await viewer.selected(null);
  // a level with nothing drawn has no grid
  await viewer.open("12-diagnostics");
  await page.locator("#tab-hierarchy").click();
  await page.locator("#index .index-row", { hasText: "sub" }).click();
  await expect(page.locator("#empty")).toBeVisible();
  expect(await drawnPixels(page)).toBe(0);
});

test("the status bar shows where the pointer is, in units with y up, as precisely as the zoom tells", async ({ page, viewer }) => {
  const pointer = page.locator("#pointer");
  await expect(pointer).toHaveText("");
  /** The pixels of a unit, and the scene at the middle of the view. */
  const view = () =>
    page.evaluate(() => {
      const canvas = document.getElementById("canvas");
      const [x, y, w, h] = document.querySelector("#canvas > svg").getAttribute("viewBox").split(" ").map(Number);
      return { unit: (canvas.clientWidth / w) * 14, middle: [(x + w / 2) / 14, -(y + h / 2) / 14] };
    });
  /**
   * Point at a point of the scene, and check that the status bar says where it is, x then y, with
   * `digits` digits after the point, within a pixel and the rounding.
   */
  const check = async (u, v, digits) => {
    const p = await at(viewer, u, v);
    await page.mouse.move(p.x, p.y);
    const number = digits === 0 ? "-?\\d+" : `-?\\d+\\.\\d{${digits}}`;
    await expect(pointer).toHaveText(new RegExp(`^x ${number}, y ${number}$`));
    const [x, y] = (await pointer.textContent()).match(/-?[\d.]+/g).map(Number);
    const slack = 1.5 / (await view()).unit + 0.5 * 10 ** -digits;
    expect(Math.abs(x - u)).toBeLessThanOrEqual(slack);
    expect(Math.abs(y - v)).toBeLessThanOrEqual(slack);
  };
  // fitted, a unit is 10 to 100 pixels: a digit after the point
  expect((await view()).unit).toBeGreaterThan(10);
  expect((await view()).unit).toBeLessThan(100);
  await check(10, 0, 1);
  await check(-3, 2, 1);
  await check(17.4, -2.6, 1);
  // zoomed in, more digits; zoomed out, none
  for (let i = 0; i < 12; i++) await page.keyboard.press("Control+=");
  expect((await view()).unit).toBeGreaterThan(100);
  const [cu, cv] = (await view()).middle;
  await check(cu + 0.13, cv - 0.27, 2);
  await page.keyboard.press("f");
  for (let i = 0; i < 6; i++) await page.keyboard.press("Control+-");
  expect((await view()).unit).toBeLessThan(10);
  await check(14, -3, 0);
  // off the drawing, nothing
  await page.mouse.move(5, 5);
  await expect(pointer).toHaveText("");
});

test.describe("in dark mode", () => {
  test.use({ colorScheme: "dark" });

  test("the grid is light grey on the drawing's white", async ({ page, viewer }) => {
    await expect(page.locator("#canvas")).toHaveCSS("background-color", "rgb(255, 255, 255)");
    const dot = await pixel(page, await at(viewer, 10, 0));
    expect(dot).toEqual(await colour(page, "--grid-dot"));
    // grey, lighter than the drawing's black and darker than its white
    expect(dot[0]).toBe(dot[1]);
    expect(dot[0]).toBeGreaterThan(128);
    expect(dot[0]).toBeLessThan(230);
  });
});
