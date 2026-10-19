import fs from "node:fs";
import path from "node:path";
import { expect, test } from "../viewer.js";

const area = (page) => page.locator("#source .editor:not([hidden]) textarea");
const state = (page) => page.evaluate(() => ({ level: window.viewer.level, path: window.viewer.path,
  selected: window.viewer.selected, doc: window.viewer.doc.path, edits: window.viewer.edits }));
const down = async (page, viewer, name = "XK1") => {
  await viewer.click(name);
  await page.keyboard.press("e");
};
const cases = [
  { name: "drawing double click", level: "bank", act: async (p, v) => { const at = await v.where("XK1"); await p.mouse.dblclick(at.x, at.y); } },
  { name: "open badge", level: "bank", setup: (p, v) => v.click("XK1"), act: (p) => p.locator("#enter-badge").click() },
  { name: "E", level: "bank", setup: (p, v) => v.click("XK1"), act: (p) => p.keyboard.press("e") },
  { name: "Enter", level: "bank", setup: (p, v) => v.click("XK1"), act: (p) => p.keyboard.press("Enter") },
  { name: "Ctrl+E", level: "nested.sp", setup: down, act: (p) => p.keyboard.press("Control+e") },
  { name: "Backspace", level: "nested.sp", setup: down, act: (p) => p.keyboard.press("Backspace") },
  { name: "up button", level: "nested.sp", setup: down, act: (p) => p.locator("#up").click() },
  { name: "path segment", level: "nested.sp", setup: down, act: (p) => p.locator('#level-path .crumb[data-depth="0"]').click() },
  { name: "hierarchy tree", level: "bank", setup: (p) => p.locator("#tab-hierarchy").click(), act: (p) => p.locator('.tree-row[data-path="0/0"]').click() },
  { name: "subcircuit index", level: "bank", setup: (p) => p.locator("#tab-hierarchy").click(), act: (p) => p.locator(".index-row", { hasText: /^bank/ }).click() },
  { name: "text double click", level: "bank", act: async (p, v) => {
    const o = await v.object("instance", "XK1", "nested.sp");
    await p.locator(`#source [data-obj~="${o}"]`).first().dblclick();
  } },
  { name: "text selects another level", level: "leaf", act: async (p, v) => {
    const o = await v.object("instance", "R1", "leaf");
    await p.locator(`#source [data-obj~="${o}"]`).first().click();
  } },
  { name: "path overflow menu", level: "block", setup: async (p, v) => {
    for (const name of ["XK1", "XB2", "XC1", "XL2"]) await down(p, v, name);
    const split = await v.middle(p.locator("#vsplit"));
    await v.drag(split, { x: 1199, y: split.y });
    await p.locator("#level-path .crumb-more").click();
  }, act: (p) => p.locator("#path-menu .path-menu-item", { hasText: "block" }).click() },
];

for (const c of cases) for (const shift of [false, true]) {
  test(`${c.name}: ${shift ? "Shift opens the target and preserves this viewer" : "opens in this viewer"}`, async ({ page, viewer }) => {
    await viewer.open("nested");
    await c.setup?.(page, viewer);
    const before = await state(page);
    if (shift) await page.keyboard.down("Shift");
    await c.act(page, viewer);
    if (shift) await page.keyboard.up("Shift");
    if (shift) {
      await expect.poll(() => viewer.calls("newWindow")).toHaveLength(1);
      const [[session]] = await viewer.calls("newWindow");
      expect(session.entry.fixture).toBe("nested");
      expect(session.level).toBe(c.level);
      expect(session.path.at(-1).title).toBe(c.level);
      expect(await state(page)).toEqual(before);
    } else {
      await expect(page.locator("#level-name")).toHaveText(c.level);
      expect(await viewer.calls("newWindow")).toEqual([]);
    }
  });
}

test("Library stays visible, and Shift transfers its pool without opening locally", async ({ page, viewer }) => {
  await viewer.open("nested", { pool: "1" });
  await page.evaluate(() => { window.fakeShell.includes = [{ path: "/other.sp", name: "other.sp", source: { path: "/other.sp", name: "other.sp", fixture: "01-rc-lowpass" } }]; });
  await page.locator("#tab-files").click();
  await viewer.build(() => page.locator("#include-file").click());
  const before = await state(page);
  const row = page.getByRole("button", { name: "Open other.sp", exact: true });
  await row.click({ modifiers: ["Shift"] });
  await expect.poll(() => viewer.calls("newWindow")).toHaveLength(1);
  const [[session]] = await viewer.calls("newWindow");
  expect(session.entry.path).toBe("/other.sp");
  expect(session.pool.some((i) => i.path === "/other.sp")).toBe(true);
  expect(await state(page)).toEqual(before);
  await viewer.build(() => row.click());
  await expect(page.locator("#pane-files")).toBeVisible();
});

test("New Window copies current path and selection; its subcircuit command requires an openable instance", async ({ page, viewer }, info) => {
  await viewer.open("nested");
  await viewer.menuTitle("Edit").click();
  await expect(viewer.menuItem("descend-new-window")).toBeDisabled();
  await page.keyboard.press("Escape");
  await down(page, viewer);
  await viewer.click("XB1");
  const before = await state(page);
  await viewer.menuTitle("File").click();
  if (process.env.RNL_NAV_SCREENSHOTS && info.project.name === "chromium") {
    fs.mkdirSync(process.env.RNL_NAV_SCREENSHOTS, { recursive: true });
    await page.screenshot({ path: path.join(process.env.RNL_NAV_SCREENSHOTS, "file-menu-new-window.png") });
  }
  await viewer.menuItem("new-window").click();
  const [[session]] = await viewer.calls("newWindow");
  expect(session.level).toBe("bank");
  expect(session.path.map((s) => s.via)).toEqual([null, "XK1"]);
  expect(session.select.name).toBe("XB1");
  await viewer.menuTitle("Edit").click();
  await expect(viewer.menuItem("descend-new-window")).toBeEnabled();
  expect(await viewer.menuItem("descend-new-window").evaluate((el) =>
    el.querySelector(".label").getBoundingClientRect().right <= el.querySelector(".keys").getBoundingClientRect().left)).toBe(true);
  if (process.env.RNL_NAV_SCREENSHOTS && info.project.name === "chromium") {
    fs.mkdirSync(process.env.RNL_NAV_SCREENSHOTS, { recursive: true });
    await page.screenshot({ path: path.join(process.env.RNL_NAV_SCREENSHOTS, "edit-menu-shortcuts.png") });
  }
  await viewer.menuItem("descend-new-window").click();
  expect((await viewer.calls("newWindow"))[1][0].level).toBe("block");
  expect(await state(page)).toEqual(before);
});

test("Control shortcuts prevent browser defaults; letters stay in input and list controls", async ({ page, viewer }) => {
  await viewer.open("nested");
  await page.evaluate(() => {
    window.keyDefaults = [];
    document.addEventListener("keydown", (e) => { if (e.ctrlKey && /^[eq]$/i.test(e.key)) window.keyDefaults.push([e.key.toLowerCase(), e.defaultPrevented]); });
  });
  await down(page, viewer);
  await page.keyboard.press("Control+e");
  await expect(page.locator("#level-name")).toHaveText("nested.sp");
  await page.keyboard.press("Control+q");
  await expect(area(page)).toBeVisible();
  await page.keyboard.type("eE");
  expect(await viewer.calls("newWindow")).toEqual([]);
  await page.evaluate(() => window.viewer.command("discard-edits"));
  await page.getByRole("button", { name: "Discard Changes", exact: true }).click();
  await page.locator("#tab-hierarchy").click();
  await page.locator("#search").fill("e");
  await page.keyboard.press("Shift+e");
  expect(await viewer.calls("newWindow")).toEqual([]);
  await page.locator("#search").fill("");
  await page.locator("#tree").evaluate((el) => { el.tabIndex = 0; el.focus(); });
  await page.keyboard.press("Shift+e");
  expect(await viewer.calls("newWindow")).toEqual([]);
  expect(await page.evaluate(() => window.keyDefaults)).toEqual([["e", true], ["q", true]]);
  // The new Shift+E binding leaves the existing case-insensitive page shortcuts intact.
  await page.locator("#canvas").click({ position: { x: 5, y: 5 } });
  const grid = await page.evaluate(() => window.viewer.grid.shown);
  await page.keyboard.press("Shift+g");
  expect(await page.evaluate(() => window.viewer.grid.shown)).toBe(!grid);
});

test("sidebar context menus are suppressed except in editable controls", async ({ page, viewer }) => {
  await viewer.open("nested");
  const prevented = (selector) => page.locator(selector).evaluate((el) => !el.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true })));
  expect(await prevented("#source .file:not([hidden])")).toBe(true);
  expect(await prevented("#tab-files")).toBe(true);
  await page.keyboard.press("Control+q");
  expect(await prevented("#source textarea")).toBe(false);
  await page.keyboard.press("Control+q");
  await page.locator("#tab-hierarchy").click();
  expect(await prevented("#search")).toBe(false);
});

test("editing preserves the first visible line across different line heights and rebuilds", async ({ page, viewer }, info) => {
  await viewer.open("subcircuits");
  const output = process.env.RNL_NAV_SCREENSHOTS;
  const screenshot = async (name) => {
    if (output && info.project.name === "chromium") {
      fs.mkdirSync(output, { recursive: true });
      await page.screenshot({ path: path.join(output, name) });
    }
  };
  await page.addStyleTag({ content: ".source .ln { min-height: 0; line-height: 19.2px; } .editor .area, .editor .gutter { line-height: 27px; }" });
  // Reproduce integer scroll rounding even in engines which retain fractional scroll offsets.
  await page.locator("#source").evaluate((el) => { el.scrollTop = Math.floor(4 + 40 * el.querySelector(".ln").getBoundingClientRect().height); });
  await screenshot("scroll-01-reading.png");
  await page.keyboard.press("Control+q");
  await expect(area(page)).toBeVisible();
  expect(await area(page).evaluate((el) => Math.floor((el.scrollTop - 4) / 27))).toBe(40);
  await screenshot("scroll-02-editing.png");
  await area(page).evaluate((el) => { el.setSelectionRange(12, 12); });
  // Gecko reveals a programmatically moved caret on a later paint. A user's subsequent scroll
  // happens after that reveal, so settle it before setting the viewport we intend to preserve.
  const settled = () => page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  await settled();
  await area(page).evaluate((el) => { el.scrollTop = 4 + 47 * 27; });
  expect(await area(page).evaluate((el) => Math.round((el.scrollTop - 4) / 27))).toBe(47);
  await page.keyboard.press("Control+q");
  const firstLine = () => page.locator("#source").evaluate((el) => Math.round((el.scrollTop - 4) / el.querySelector(".ln").getBoundingClientRect().height));
  expect(await firstLine()).toBe(47);
  await screenshot("scroll-03-exited.png");
  await page.keyboard.press("Control+q");
  expect(await area(page).evaluate((el) => el.selectionStart)).toBe(12);
  await area(page).evaluate((el) => { el.value += "\n* edited"; el.dispatchEvent(new Event("input", { bubbles: true })); });
  await settled();
  await area(page).evaluate((el) => { el.scrollTop = 4 + 49 * 27; });
  await viewer.build(() => page.keyboard.press("Control+Enter"));
  expect(await firstLine()).toBe(49);
  await screenshot("scroll-04-applied.png");
});
