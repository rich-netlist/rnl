// The page's own menu bar, shown on a platform without a native one: opening, switching and closing
// its menus by the mouse and the keyboard, a click outside that only closes a menu, the commands,
// and the check marks. With a native menu the page shows no bar and tells the shell the marks.

import { expect, test } from "../viewer.js";

const TOP = "04-hierarchy.sp";

/** The list of a menu, by its title. */
const list = (page, title) => page.locator("#menubar .menu", { has: page.locator(".menu-title", { hasText: title }) }).locator(".menu-list");

test.describe("on the web", () => {
  test.beforeEach(async ({ viewer }) => {
    await viewer.open("04-hierarchy");
  });

  test("the menu bar has the web's menus and their keys", async ({ page }) => {
    await expect(page.locator("#menubar")).toBeVisible();
    // Help has only the desktop's About, so it is not shown
    await expect(page.locator("#menubar .menu-title")).toHaveText(["File", "View"]);
    await expect(page.locator("#menubar .menu-title u")).toHaveText(["F", "V"]);
    await expect(list(page, "File").locator(".menu-item")).toHaveText(["Open...Ctrl+O", "Open Folder...", "Open URL...", "ReloadF5"]);
    await expect(list(page, "File").locator(".menu-separator")).toHaveCount(0);
    await expect(list(page, "View").locator(".menu-item, .menu-separator")).toHaveText([
      "Fit to WindowF", "Zoom InCtrl++", "Zoom OutCtrl+-", "",
      "Open SubcircuitEnter", "Up to Parent LevelBackspace", "",
      "Netlist PanelCtrl+B", "ProblemsCtrl+Shift+M",
    ]);
  });

  test("a click opens a menu, pointing switches it, and Escape or a click on its title closes it", async ({ page, viewer }) => {
    const [file, view] = [viewer.menuTitle("File"), viewer.menuTitle("View")];
    await view.hover(); // nothing opens while no menu is open
    await expect(list(page, "View")).toBeHidden();
    await file.click();
    await expect(list(page, "File")).toBeVisible();
    await expect(file).toHaveAttribute("aria-expanded", "true");
    await view.hover();
    await expect(list(page, "View")).toBeVisible();
    await expect(list(page, "File")).toBeHidden();
    await expect(file).toHaveAttribute("aria-expanded", "false");
    // pointing at an item marks it
    await viewer.menuItem("zoom-out").hover();
    await expect(viewer.menuItem("zoom-out")).toHaveClass(/\bactive\b/);
    await file.hover();
    await expect(list(page, "File")).toBeVisible();
    await file.click();
    await expect(list(page, "File")).toBeHidden();

    // Escape closes the menu, and leaves the selection
    await viewer.click("X2");
    await view.click();
    await page.keyboard.press("Escape");
    await expect(list(page, "View")).toBeHidden();
    await expect(page.locator("#menubar .menu-title.at")).toHaveCount(0);
    await viewer.selected(`instance X2 in ${TOP}`);
    // with no menu open, it clears the selection as before
    await page.keyboard.press("Escape");
    await viewer.selected(null);
  });

  test("a click outside an open menu only closes it", async ({ page, viewer }) => {
    const view = viewer.menuTitle("View");
    await viewer.click("X2");
    const box = await viewer.viewBox();
    const width = await page.locator("#sidebar").evaluate((s) => s.getBoundingClientRect().width);

    // on an empty place of the drawing, on a symbol, and a drag: no selection changes, no view moves
    for (const act of [
      async () => { const p = await viewer.empty(); await page.mouse.click(p.x, p.y); },
      () => viewer.click("X1"),
      async () => { const p = await viewer.where("X1"); await viewer.drag(p, { x: p.x + 80, y: p.y + 30 }); },
    ]) {
      await view.click();
      await expect(list(page, "View")).toBeVisible();
      await act();
      await expect(list(page, "View")).toBeHidden();
      await viewer.selected(`instance X2 in ${TOP}`);
      expect(await viewer.viewBox()).toBe(box);
    }
    // a double click whose first press closes the menu opens nothing
    await view.click();
    const x1 = await viewer.where("X1");
    await page.mouse.dblclick(x1.x, x1.y);
    await expect(list(page, "View")).toBeHidden();
    await expect(page.locator("#level-name")).toHaveText(TOP);
    // a button, and a splitter's drag
    await view.click();
    await page.locator("#problems-head").click();
    await expect(page.locator("#problems-head")).toHaveAttribute("aria-expanded", "true");
    await view.click();
    const split = await viewer.middle(page.locator("#vsplit"));
    await viewer.drag(split, { x: split.x - 100, y: split.y });
    expect(await page.locator("#sidebar").evaluate((s) => s.getBoundingClientRect().width)).toBe(width);

    // with no menu open, a click acts
    await viewer.click("X1");
    await viewer.selected(`instance X1 in ${TOP}`);
  });

  test("the menus' commands run", async ({ page, viewer }) => {
    const choose = async (title, id) => {
      await viewer.menuTitle(title).click();
      await viewer.menuItem(id).click();
      await expect(page.locator("#menubar .menu-list:visible")).toHaveCount(0);
    };
    await choose("File", "open-folder");
    await choose("File", "open-url");
    expect(await viewer.calls("pick")).toEqual([["folder"], ["url"]]);
    await page.evaluate(() => window.fakeShell.picks.push("01-rc-lowpass"));
    await viewer.build(() => choose("File", "open"));
    expect(await viewer.calls("pick")).toEqual([["folder"], ["url"], ["file"]]);
    await expect(page.locator("#level-name")).toHaveText("01-rc-lowpass.sp");
    await viewer.build(() => choose("File", "reload"));
    expect((await viewer.calls("build")).at(-1)).toEqual(["01-rc-lowpass"]);

    const fitted = await viewer.viewBox();
    const width = () => viewer.viewBox().then((b) => Number(b.split(" ")[2]));
    const w = await width();
    await choose("View", "zoom-in");
    expect(await width()).toBeCloseTo(w / 1.25, 6);
    await choose("View", "zoom-out");
    await choose("View", "zoom-out");
    expect(await width()).toBeCloseTo(w / 0.8, 6);
    await choose("View", "fit");
    expect(await viewer.viewBox()).toBe(fitted);

    await page.evaluate(() => window.fakeShell.picks.push("04-hierarchy"));
    await viewer.build(() => choose("File", "open"));
    await viewer.click("X3");
    await choose("View", "descend");
    await expect(page.locator("#level-name")).toHaveText("rdiv");
    await choose("View", "ascend");
    await expect(page.locator("#level-name")).toHaveText(TOP);
    await viewer.selected(`instance X3 in ${TOP}`);
  });

  test("the check marks follow Ctrl+B and Ctrl+Shift+M", async ({ page, viewer }) => {
    const marks = async () => {
      await viewer.menuTitle("View").click();
      const both = [
        await viewer.menuItem("toggle-netlist").getAttribute("aria-checked"),
        await viewer.menuItem("toggle-problems").getAttribute("aria-checked"),
      ];
      await page.keyboard.press("Escape");
      return both;
    };
    expect(await marks()).toEqual(["true", "true"]);
    await page.keyboard.press("Control+B");
    await expect(page.locator("#sidebar")).toBeHidden();
    expect(await marks()).toEqual(["false", "true"]);
    await page.keyboard.press("Control+Shift+M");
    expect(await marks()).toEqual(["false", "false"]);
    await page.keyboard.press("Control+B");
    await expect(page.locator("#pane-netlist")).toHaveClass(/\bproblems-folded\b/);
    expect(await marks()).toEqual(["true", "false"]);
    // the problems' header, and the items themselves
    await page.locator("#problems-head").click();
    expect(await marks()).toEqual(["true", "true"]);
    await viewer.menuTitle("View").click();
    await viewer.menuItem("toggle-problems").click();
    await expect(page.locator("#problems-head")).toHaveAttribute("aria-expanded", "false");
    expect(await marks()).toEqual(["true", "false"]);
    await viewer.menuTitle("View").click();
    await viewer.menuItem("toggle-netlist").click();
    await expect(page.locator("#sidebar")).toBeHidden();
    expect(await marks()).toEqual(["false", "false"]);
    // the shell is told as well, whether it shows a menu or not
    expect(await viewer.calls("setCheck")).toEqual(
      expect.arrayContaining([["toggle-netlist", false], ["toggle-problems", false]]),
    );
  });

  test("the keyboard uses the bar: Alt, F10, the arrows, Enter and the underlined letters", async ({ page, viewer }) => {
    const bar = page.locator("#menubar");
    const marked = page.locator("#menubar .menu-title.at");
    const width = () => viewer.viewBox().then((b) => Number(b.split(" ")[2]));
    const w = await width();

    // Alt alone marks the first title and shows the letters
    await page.keyboard.press("Alt");
    await expect(marked).toHaveText("File");
    await expect(bar).toHaveClass(/\bkeys\b/);
    await expect(page.locator("#menubar .menu-list:visible")).toHaveCount(0);
    await page.keyboard.press("ArrowRight");
    await expect(marked).toHaveText("View");
    await page.keyboard.press("ArrowDown");
    await expect(list(page, "View")).toBeVisible();
    await expect(viewer.menuItem("fit")).toHaveClass(/\bactive\b/);
    await page.keyboard.press("ArrowDown");
    await expect(viewer.menuItem("zoom-in")).toHaveClass(/\bactive\b/);
    await page.keyboard.press("Enter");
    await expect(marked).toHaveCount(0);
    expect(await width()).toBeCloseTo(w / 1.25, 6);

    // Alt with a title's letter opens it, and an item's letter chooses it
    await page.keyboard.press("Alt+V");
    await expect(list(page, "View")).toBeVisible();
    await page.keyboard.press("o");
    await expect(list(page, "View")).toBeHidden();
    expect(await width()).toBeCloseTo(w, 6);

    // F10 marks the first title; ArrowUp opens its menu at the last item
    await page.keyboard.press("F10");
    await expect(marked).toHaveText("File");
    await page.keyboard.press("ArrowUp");
    await expect(viewer.menuItem("reload")).toHaveClass(/\bactive\b/);
    // Escape closes the menu and keeps the title, then leaves the bar
    await page.keyboard.press("Escape");
    await expect(list(page, "File")).toBeHidden();
    await expect(marked).toHaveText("File");
    await page.keyboard.press("Escape");
    await expect(marked).toHaveCount(0);
    await expect(bar).not.toHaveClass(/\bkeys\b/);

    // a key with Ctrl leaves the bar and acts
    await page.keyboard.press("Alt");
    await expect(marked).toHaveText("File");
    await page.keyboard.press("Control+B");
    await expect(marked).toHaveCount(0);
    await expect(page.locator("#sidebar")).toBeHidden();
  });
});

test.describe("with a native menu", () => {
  test.beforeEach(async ({ viewer }) => {
    await viewer.open("04-hierarchy", { platform: "desktop" });
  });

  test("the page shows no menu bar, and tells the shell the check marks", async ({ page, viewer }) => {
    await expect(page.locator("#menubar")).toBeHidden();
    expect(await viewer.calls("setCheck")).toEqual([["toggle-netlist", true], ["toggle-problems", true]]);
    await page.keyboard.press("Control+B");
    await page.keyboard.press("Control+Shift+M");
    expect((await viewer.calls("setCheck")).slice(2)).toEqual([["toggle-netlist", false], ["toggle-problems", false]]);
    expect(await page.evaluate(() => window.fakeShell.checks)).toEqual({ "toggle-netlist": false, "toggle-problems": false });
    // Alt and F10 do nothing on the page
    await page.keyboard.press("Alt");
    await page.keyboard.press("F10");
    await expect(page.locator("#menubar")).toBeHidden();
  });

  test("the commands chosen in the native menu run", async ({ page, viewer }) => {
    const menu = (id) => page.evaluate((id) => window.fakeShell.menu(id), id);
    await menu("toggle-netlist");
    await expect(page.locator("#sidebar")).toBeHidden();
    expect(await page.evaluate(() => window.fakeShell.checks["toggle-netlist"])).toBe(false);
    await menu("toggle-netlist");
    await menu("toggle-problems");
    await expect(page.locator("#problems-head")).toHaveAttribute("aria-expanded", "false");

    await page.keyboard.press("f"); // fitted to the canvas as the panels left it
    const fitted = await viewer.viewBox();
    await menu("zoom-in");
    await expect.poll(() => viewer.viewBox()).not.toBe(fitted);
    await menu("fit");
    expect(await viewer.viewBox()).toBe(fitted);
    await viewer.click("X2");
    await menu("descend");
    await expect(page.locator("#level-name")).toHaveText("amp");
    await menu("ascend");
    await viewer.selected(`instance X2 in ${TOP}`);
    await viewer.build(() => menu("reload"));

    // the platform's own commands do nothing on the page
    await menu("about");
    await menu("exit");
    await expect(page.locator("#level-name")).toHaveText(TOP);
    await menu("open");
    expect(await viewer.calls("pick")).toEqual([["file"]]);
  });
});
