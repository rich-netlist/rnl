// The page's own menu bar, shown on a platform without a native one: the menus the command table
// gives the web, opening, switching and closing them and their submenus by the mouse and the
// keyboard, a click outside that only closes a menu, the commands, and the check marks. With a
// native menu the page shows no bar and tells the shell the marks.

import { expect, test } from "../viewer.js";

const TOP = "04-hierarchy.sp";

/** The list of a menu, by its title, and of a submenu, by the text of its entry. */
const list = (page, title) => page.locator("#menubar .menu", { has: page.locator(".menu-title", { hasText: title }) }).locator(".menu-list");
const submenu = (page, text) => page.locator("#menubar .menu-sub", { has: page.locator(".opens", { hasText: text }) }).locator(".submenu");
const entry = (page, text) => page.locator("#menubar .menu-item.opens", { hasText: text });

/**
 * The menus that the command table gives the web, as its menu bar shows them: per menu its title,
 * the letter underlined in it, and its entries' texts with their keys, "" for a separator, a
 * submenu's entry followed by its own. Only the items the web offers, a separator only between two
 * entries, and no menu or submenu left without items.
 */
async function webMenus(page) {
  const table = await page.evaluate(() => fetch("commands.json").then((r) => r.json()));
  const plain = (label) => label.replace("&", "");
  const entries = (items) => {
    const shown = [];
    let gap = false;
    for (const item of items) {
      if (item === "-") {
        gap = shown.length > 0;
        continue;
      }
      const inner = item.items ? entries(item.items) : null;
      if (inner ? inner.length === 0 : item.platforms && !item.platforms.includes("web")) continue;
      if (gap) shown.push("");
      shown.push(plain(item.label) + (item.shown || (item.keys ? item.keys[0] : "")), ...(inner || []));
      gap = false;
    }
    return shown;
  };
  const menus = [];
  for (const m of table.menus) {
    const items = entries(m.items);
    if (items.length > 0) menus.push({ title: plain(m.label), letter: m.label[m.label.indexOf("&") + 1], items });
  }
  return menus;
}

test.describe("on the web", () => {
  test.beforeEach(async ({ viewer }) => {
    await viewer.open("04-hierarchy");
  });

  test("the menu bar has the web's menus of the command table, and their keys", async ({ page, viewer }) => {
    await expect(page.locator("#menubar")).toBeVisible();
    const menus = await webMenus(page);
    await expect(page.locator("#menubar .menu-title")).toHaveText(menus.map((m) => m.title));
    await expect(page.locator("#menubar .menu-title u")).toHaveText(menus.map((m) => m.letter));
    for (const m of menus) await expect(list(page, m.title).locator(".menu-item, .menu-separator")).toHaveText(m.items);
    // File has the web's own items, and not the desktop's Save As and Exit nor the separators above
    // them; Save downloads the file shown
    expect(menus.slice(0, 3)).toEqual([
      {
        title: "File",
        letter: "F",
        items: [
          "New Window", "Open File...Ctrl+O", "Include File...", "Include Folder...", "Include Public Package…", "Open URL...", "Load from Clipboard...Ctrl+Shift+V", "ReloadF5",
          "Reload Library", "Auto Include",
          "", "SaveCtrl+S",
        ],
      },
      { title: "Edit", letter: "E", items: ["Edit TextCtrl+Q", "Apply and Exit EditingCtrl+Enter", "Discard Changes", "Find in Netlist…Ctrl+F", "", "Open SubcircuitE", "Open Subcircuit in New Window…Shift+E", "Up to Parent LevelCtrl+E"] },
      {
        title: "View",
        letter: "V",
        items: [
          "Fit to WindowF", "Zoom InCtrl++", "Zoom OutCtrl+-",
          "Background", "Light", "Dark",
          "Grid", "Show GridG", "", "None", "Dots", "Lines", "", "Axes", "", "Grid Settings...", "Show Names on Hover", "",
          "Netlist PanelCtrl+B", "ProblemsCtrl+Shift+M",
        ],
      },
    ]);
    // the Grid submenu's entry opens it, and its style is one of three
    await expect(entry(page, "Grid")).toHaveAttribute("aria-haspopup", "menu");
    await expect(submenu(page, "Grid")).toBeHidden();
    for (const id of ["grid-none", "grid-dots", "grid-lines"]) await expect(viewer.menuItem(id)).toHaveAttribute("role", "menuitemradio");
    for (const id of ["toggle-grid", "grid-axes"]) await expect(viewer.menuItem(id)).toHaveAttribute("role", "menuitemcheckbox");
    // Help has the web's own Desktop Viewer, before About
    expect(menus.at(-1)).toEqual({ title: "Help", letter: "H", items: ["Keyboard Shortcuts...", "Desktop Viewer...", "", "About RNL Viewer"] });
  });

  test("Help > Desktop Viewer shows the site's page of the tools, in Chinese if the browser puts Chinese first", async ({ page, viewer }) => {
    /** The pages the shell is asked to show when the browser's languages are `languages`. */
    const shown = async (languages) => {
      await page.addInitScript((list) => {
        Object.defineProperty(Navigator.prototype, "languages", { get: () => list, configurable: true });
      }, languages);
      await viewer.open("04-hierarchy");
      await viewer.menuTitle("Help").click();
      await viewer.menuItem("desktop-viewer").click();
      await expect(page.locator("#menubar .menu-list:visible")).toHaveCount(0);
      return viewer.calls("browse");
    };
    const zh = "https://rich-netlist.github.io/rnl/zh/tools.html";
    const en = "https://rich-netlist.github.io/rnl/en/tools.html";
    expect(await shown(["en-US", "zh-CN"])).toEqual([[en]]);
    expect(await shown(["zh-CN", "en-US"])).toEqual([[zh]]);
    expect(await shown(["de-DE", "en-GB", "zh-TW"])).toEqual([[en]]);
    // without either, English, the site's default
    expect(await shown(["fr-FR"])).toEqual([[en]]);
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
    await choose("File", "include-folder");
    expect(await viewer.calls("include")).toEqual([["folder"]]);
    await choose("File", "open-url");
    expect(await viewer.calls("pick")).toEqual([["url"]]);
    await page.evaluate(() => window.fakeShell.picks.push("01-rc-lowpass"));
    await viewer.build(() => choose("File", "open"));
    expect(await viewer.calls("pick")).toEqual([["url"], ["file"]]);
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
    await choose("Edit", "descend");
    await expect(page.locator("#level-name")).toHaveText("rdiv");
    await choose("Edit", "ascend");
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
    await expect(marked).toHaveText("Edit");
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
    await expect(viewer.menuItem("save")).toHaveClass(/\bactive\b/);
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

  test("pointing at a submenu's entry opens the submenu beside it, and its items run", async ({ page, viewer }) => {
    const grid = submenu(page, "Grid");
    const shown = page.locator("#menubar .menu-list:visible, #menubar .submenu:visible");
    await viewer.menuTitle("View").click();
    await expect(grid).toBeHidden();
    await entry(page, "Grid").hover();
    await expect(grid).toBeVisible();
    await expect(entry(page, "Grid")).toHaveAttribute("aria-expanded", "true");
    // on the right of its entry, its first item level with it
    const [e, first] = [await entry(page, "Grid").boundingBox(), await viewer.menuItem("toggle-grid").boundingBox()];
    expect(first.x).toBeGreaterThanOrEqual(e.x + e.width - 4);
    expect(Math.abs(first.y - e.y)).toBeLessThanOrEqual(1);
    // the marks: the grid shown, as dots, without the axes
    for (const [id, checked] of [["toggle-grid", "true"], ["grid-none", "false"], ["grid-dots", "true"], ["grid-lines", "false"], ["grid-axes", "false"]]) {
      await expect(viewer.menuItem(id)).toHaveAttribute("aria-checked", checked);
    }
    // pointing at an item marks it, and its entry stays marked; pointing at another entry closes it
    await viewer.menuItem("grid-lines").hover();
    await expect(viewer.menuItem("grid-lines")).toHaveClass(/\bactive\b/);
    await expect(entry(page, "Grid")).toHaveClass(/\bactive\b/);
    await viewer.menuItem("zoom-in").hover();
    await expect(grid).toBeHidden();
    await expect(entry(page, "Grid")).toHaveAttribute("aria-expanded", "false");
    await expect(entry(page, "Grid")).not.toHaveClass(/\bactive\b/);
    // a click on the entry opens it as well, and leaves the menu open
    await entry(page, "Grid").click();
    await expect(grid).toBeVisible();
    await expect(list(page, "View")).toBeVisible();
    // an item chosen runs, and closes the menus
    await viewer.menuItem("grid-lines").click();
    await expect(shown).toHaveCount(0);
    expect(await page.evaluate(() => window.viewer.grid)).toMatchObject({ shown: true, style: "lines" });
    await viewer.menuTitle("View").click();
    await entry(page, "Grid").hover();
    await expect(viewer.menuItem("grid-lines")).toHaveAttribute("aria-checked", "true");
    await expect(viewer.menuItem("grid-dots")).toHaveAttribute("aria-checked", "false");
    await viewer.menuItem("grid-axes").click();
    expect(await page.evaluate(() => window.viewer.grid.axes)).toBe(true);
    // pointing at another title closes the submenu with its menu; a click elsewhere closes both
    await viewer.menuTitle("View").click();
    await entry(page, "Grid").hover();
    await viewer.menuTitle("Help").hover();
    await expect(grid).toBeHidden();
    await expect(list(page, "Help")).toBeVisible();
    await viewer.menuTitle("View").hover();
    await entry(page, "Grid").hover();
    await expect(grid).toBeVisible();
    const p = await viewer.empty();
    await page.mouse.click(p.x, p.y);
    await expect(shown).toHaveCount(0);
  });

  test("ArrowRight, Enter and its letter open a submenu; ArrowLeft and Escape close it", async ({ page, viewer }) => {
    const grid = submenu(page, "Grid");
    const shown = page.locator("#menubar .menu-list:visible, #menubar .submenu:visible");
    await page.keyboard.press("Alt+V");
    await expect(viewer.menuItem("fit")).toHaveClass(/\bactive\b/);
    for (let i = 0; i < 4; i++) await page.keyboard.press("ArrowDown");
    await expect(entry(page, "Grid")).toHaveClass(/\bactive\b/);
    await page.keyboard.press("ArrowRight");
    await expect(grid).toBeVisible();
    await expect(viewer.menuItem("toggle-grid")).toHaveClass(/\bactive\b/);
    // the arrows move in the submenu, past its separators and round
    await page.keyboard.press("ArrowDown");
    await expect(viewer.menuItem("grid-none")).toHaveClass(/\bactive\b/);
    await page.keyboard.press("ArrowUp");
    await page.keyboard.press("ArrowUp");
    await expect(viewer.menuItem("grid-settings")).toHaveClass(/\bactive\b/);
    // ArrowLeft closes it and keeps its entry marked; Enter opens it again
    await page.keyboard.press("ArrowLeft");
    await expect(grid).toBeHidden();
    await expect(list(page, "View")).toBeVisible();
    await expect(entry(page, "Grid")).toHaveClass(/\bactive\b/);
    await page.keyboard.press("Enter");
    await expect(grid).toBeVisible();
    await expect(viewer.menuItem("toggle-grid")).toHaveClass(/\bactive\b/);
    // Escape closes the submenu, then the menu, then leaves the bar
    await page.keyboard.press("Escape");
    await expect(grid).toBeHidden();
    await expect(list(page, "View")).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(shown).toHaveCount(0);
    await expect(page.locator("#menubar .menu-title.at")).toHaveText("View");
    await page.keyboard.press("Escape");
    await expect(page.locator("#menubar .menu-title.at")).toHaveCount(0);

    // the underlined letters: G opens the submenu, and L chooses Lines in it
    await page.keyboard.press("Alt+V");
    await page.keyboard.press("g");
    await expect(grid).toBeVisible();
    await page.keyboard.press("l");
    await expect(shown).toHaveCount(0);
    expect(await page.evaluate(() => window.viewer.grid.style)).toBe("lines");
    // Enter chooses the item marked in the submenu
    await page.keyboard.press("Alt+V");
    await page.keyboard.press("g");
    await page.keyboard.press("ArrowUp");
    await page.keyboard.press("ArrowUp");
    await expect(viewer.menuItem("grid-axes")).toHaveClass(/\bactive\b/);
    await page.keyboard.press("Enter");
    await expect(shown).toHaveCount(0);
    expect(await page.evaluate(() => window.viewer.grid.axes)).toBe(true);
    // ArrowRight on an item that opens nothing goes on to the next menu, and ArrowLeft back
    await page.keyboard.press("Alt+V");
    await page.keyboard.press("g");
    await page.keyboard.press("ArrowRight");
    await expect(list(page, "Help")).toBeVisible();
    await expect(grid).toBeHidden();
    await page.keyboard.press("ArrowLeft");
    await expect(list(page, "View")).toBeVisible();
    await expect(grid).toBeHidden();
  });
});

test.describe("with a native menu", () => {
  test.beforeEach(async ({ viewer }) => {
    await viewer.open("04-hierarchy", { platform: "desktop" });
  });

  test("the page shows no menu bar, and tells the shell the check marks", async ({ page, viewer }) => {
    await expect(page.locator("#menubar")).toBeHidden();
    const marks = [
      ["background-light", true], ["background-dark", false],
      ["toggle-netlist", true], ["toggle-problems", true], ["toggle-edit", false],
      ["toggle-grid", true], ["grid-none", false], ["grid-dots", true], ["grid-lines", false], ["grid-axes", false],
      ["toggle-hover-names", true],
      ["toggle-auto-include", true],
    ];
    expect(await viewer.calls("setCheck")).toEqual(marks);
    await page.keyboard.press("Control+B");
    await page.keyboard.press("Control+Shift+M");
    expect((await viewer.calls("setCheck")).slice(marks.length)).toEqual([["toggle-netlist", false], ["toggle-problems", false]]);
    expect(await page.evaluate(() => window.fakeShell.checks)).toEqual({
      ...Object.fromEntries(marks), "toggle-netlist": false, "toggle-problems": false,
    });
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

  test("a grid command chosen in the native menu sets all the grid's marks, also those that did not change", async ({ page, viewer }) => {
    const menu = (id) => page.evaluate((id) => window.fakeShell.menu(id), id);
    const ids = ["toggle-grid", "grid-none", "grid-dots", "grid-lines", "grid-axes"];
    const marks = () => page.evaluate((ids) => ids.map((id) => window.fakeShell.checks[id]), ids);
    await menu("grid-lines");
    expect(await marks()).toEqual([true, false, false, true, false]);
    expect(await page.evaluate(() => window.viewer.grid.style)).toBe("lines");
    // Dots chosen while it is checked: a native menu has cleared its mark, and the page sets it again
    await menu("grid-dots");
    const before = (await viewer.calls("setCheck")).length;
    await menu("grid-dots");
    expect((await viewer.calls("setCheck")).slice(before)).toEqual(ids.map((id) => [id, id === "toggle-grid" || id === "grid-dots"]));
    await menu("grid-none");
    expect(await marks()).toEqual([false, true, false, false, false]);
    await menu("toggle-grid");
    expect(await marks()).toEqual([true, false, true, false, false]);
    await menu("grid-axes");
    expect(await marks()).toEqual([true, false, true, false, true]);
    // G, which the page handles with a native menu too
    await page.keyboard.press("g");
    expect(await marks()).toEqual([false, true, false, false, true]);
  });
});
