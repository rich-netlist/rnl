// The native menu bar of the desktop app, planned by the page (native-menu.js) from the command table as its own menu
// bar is, and drawn by the app (src/menu.rs, `setMenu`): the commands the shell offers, by `platforms`, `with` and
// `unless`; on Windows the keys shown after a tab, the page handling them; elsewhere the menu's accelerators, but for
// the keys the page always handles; on macOS the application's menu first, with About and Quit; and the menus planned
// again in another language, with the texts of the app's file dialogs.
import { test, expect } from "../viewer.js";
import { zh } from "../zh.js";

const DESKTOP = { platform: "desktop" };

/** The plan the page gives for `os`, of the menus it offers now. */
const plan = (page, os) => page.evaluate(async (os) => {
  const [{ nativeMenus }, { menus }] = await Promise.all([import("./native-menu.js"), import("./commands.js")]);
  return nativeMenus(menus(), os);
}, os);

/** The entries of a plan, its submenus' too, depth first. */
const flat = (entries) => entries.flatMap((e) => (e.kind === "submenu" ? [e, ...flat(e.items)] : [e]));
const item = (menus, id) => flat(menus).find((e) => e.kind === "item" && e.id === id);

test("the desktop is given the menu bar the page plans, at start", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy", DESKTOP);
  const sent = await viewer.calls("setMenu");
  expect(sent.length).toBeGreaterThan(0);
  const [drawn] = sent.at(-1);
  // the menus the page's own menu bar would show, by their text, in order
  const titles = await page.evaluate(async () => (await import("./commands.js")).menus().map((m) => m.label));
  expect(drawn.menus.filter((m) => m.text !== "RNL Viewer").map((m) => m.text)).toEqual(titles);
  // and the texts of the app's file dialogs
  expect(drawn.texts["Save As"]).toBe("Save As");
  expect(drawn.texts["Netlists and packages"]).toBe("Netlists and packages");
});

test("a command that names `with` is in the desktop's menu only where the shell has it", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy", DESKTOP);
  const { menus } = await plan(page, "windows");
  // the fake desktop solves: the Solve menu's commands, which name `with: solve`
  expect(item(menus, "solve")).toBeTruthy();
  // it quits (`with: quit`), and sets no file associations (`with: fileAssociations`), as the local program's page sets none
  expect(item(menus, "exit")).toBeTruthy();
  expect(item(menus, "file-associations")).toBeUndefined();
  // Save writes; Download File is the web's
  const saves = flat(menus).filter((e) => e.kind === "item" && e.id === "save");
  expect(saves.map((e) => e.text.split("\t")[0])).toEqual(["&Save"]);
});

test("on Windows the keys are shown after a tab and the page handles them; elsewhere the menu has the accelerators", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy", DESKTOP);
  const windows = (await plan(page, "windows")).menus;
  expect(item(windows, "open")).toEqual({ kind: "item", id: "open", text: "&Open File...\tCtrl+O", accelerator: null, check: false });
  expect(item(windows, "find-text")).toMatchObject({ text: "&Find in Netlist...\tCtrl+F", accelerator: null });
  const other = (await plan(page, "other")).menus;
  expect(item(other, "open")).toEqual({ kind: "item", id: "open", text: "&Open File...", accelerator: "CmdOrCtrl+O", check: false });
  // a key the page always handles has no accelerator, which would take it from the search box too
  expect(item(other, "find-text")).toMatchObject({ text: "&Find in Netlist...", accelerator: null });
  // a check mark is a check item
  expect(item(other, "grid-dots")).toMatchObject({ check: true });
  // About is the app's item, which the page answers
  expect(flat(other).filter((e) => e.kind === "about")).toHaveLength(1);
});

test("on macOS the first menu is the application's, with About and Quit, which leave the others", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy", DESKTOP);
  const { menus } = await plan(page, "macos");
  expect(menus[0]).toEqual({ kind: "submenu", text: "RNL Viewer", items: [
    { kind: "about", text: "About RNL Viewer" },
    { kind: "separator" },
    { kind: "item", id: "exit", text: "Quit RNL Viewer", accelerator: "CmdOrCtrl+Q", check: false },
  ] });
  const rest = flat(menus.slice(1));
  expect(rest.filter((e) => e.kind === "about" || e.id === "exit")).toEqual([]);
  // the Edit menu is marked, for the system's editing commands
  expect(menus.filter((m) => m.edit).map((m) => m.text)).toEqual(["&Edit"]);
});

test("a separator stands only between two entries, whatever a platform leaves out", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy", DESKTOP);
  for (const os of ["windows", "macos", "other"]) {
    const check = (entries) => {
      expect(entries[0]?.kind).not.toBe("separator");
      expect(entries.at(-1)?.kind).not.toBe("separator");
      entries.forEach((e, i) => { if (e.kind === "separator") expect(entries[i + 1].kind).not.toBe("separator"); });
      for (const e of entries) if (e.kind === "submenu") { expect(e.items.length).toBeGreaterThan(0); check(e.items); }
    };
    for (const menu of (await plan(page, os)).menus) check(menu.items);
  }
});

test("in another language the menu bar is planned again, translated, with the dialogs' texts", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy", DESKTOP);
  const before = (await viewer.calls("setMenu")).length;
  await page.evaluate(() => window.viewer.command("language-zh"));
  await expect.poll(async () => (await viewer.calls("setMenu")).length).toBeGreaterThan(before);
  const [drawn] = (await viewer.calls("setMenu")).at(-1);
  const menus = drawn.menus.filter((m) => m.text !== "RNL Viewer");
  // the View menu has its English word after the translation
  expect(menus.map((m) => m.text)).toContain(`${zh("&View")} (View)`);
  expect(drawn.texts["Save As"]).toBe(zh("Save As"));
  // the check marks are set again on the menu drawn
  const calls = await viewer.calls("setCheck");
  expect(calls.some(([id, on]) => id === "language-zh" && on === true)).toBe(true);
});
