// The real desktop shell's native drop callback, with only the Tauri bridge mocked.
import { dialogGone, expect, test } from "../viewer.js";

async function nativeDrop(page, viewer, fixture = "04-hierarchy") {
  await viewer.open(fixture, { platform: "desktop" });
  await page.evaluate(async () => {
    window.nativePaths = [];
    window.nativeInvokes = [];
    window.nativeOpened = [];
    window.__TAURI__ = {
      event: { listen: async (name, handler) => {
        window.nativeEvents ||= {};
        window.nativeEvents[name] = handler;
        return () => {};
      } },
      core: { invoke: async (command, args) => {
        window.nativeInvokes.push([command, args]);
        if (command === "dropped_files") return window.nativePaths;
      } },
      webview: { getCurrentWebview: () => ({ onDragDropEvent: async (handler) => {
        window.nativeDrop = handler;
        return () => {};
      } }) },
    };
    const desktop = (await import("../shells/desktop.js")).default;
    const { openSource } = await import("../open.js");
    await desktop.onOpen(async (source) => {
      window.nativeOpened.push({ ...source });
      window.fakeShell.next = "04-hierarchy";
      await openSource(source, false);
    });
  });
}

async function deliver(page, type, paths = []) {
  await page.evaluate(({ type, paths }) => {
    window.dropDone = window.nativeDrop({ payload: { type, paths, position: { x: 100, y: 100 } } });
  }, { type, paths });
}

test("native drag feedback clears on leave and opens a Unicode path exactly once", async ({ page, viewer }) => {
  await nativeDrop(page, viewer);
  const path = "C:\\网表 文件\\放大器.NETLIST";
  await page.evaluate((path) => { window.nativePaths = [path]; }, path);
  await deliver(page, "enter", [path]);
  await expect(page.locator("body")).toHaveClass(/dropping/);
  await deliver(page, "leave");
  await expect(page.locator("body")).not.toHaveClass(/dropping/);
  await deliver(page, "drop", [path]);
  await page.evaluate(() => window.dropDone);
  expect(await page.evaluate(() => window.nativeOpened)).toEqual([{ path, name: path }]);
  await expect(page.locator("body")).not.toHaveClass(/dropping|busy/);
  expect(await viewer.calls("build")).toHaveLength(2);
});

test("several dropped netlists ask which one to open; another drop cannot replace the choice", async ({ page, viewer }) => {
  await nativeDrop(page, viewer);
  const paths = ["C:\\a.sp", "C:\\b.spi"];
  await page.evaluate((paths) => { window.nativePaths = paths; }, paths);
  await deliver(page, "drop", paths);
  const dialog = page.locator("dialog.dialog");
  await expect(dialog.locator(".dialog-title")).toHaveText("Open Netlist");
  await deliver(page, "drop", ["C:\\other.sp"]);
  await dialog.getByRole("option", { name: paths[1], exact: true }).click();
  await dialog.getByRole("button", { name: "Open", exact: true }).click();
  await expect.poll(() => page.evaluate(() => window.nativeOpened)).toEqual([{ path: paths[1], name: paths[1] }]);
  expect(await page.evaluate(() => window.nativeInvokes.filter(([cmd]) => cmd === "dropped_files"))).toHaveLength(1);
});

test("a drop asks before leaving unapplied edits, and Cancel retains the text", async ({ page, viewer }) => {
  await nativeDrop(page, viewer, "edit");
  await page.evaluate(() => window.viewer.command("toggle-edit"));
  const area = page.locator("#source .editor:not([hidden]) textarea");
  const edited = (await area.inputValue()).replace(/C1/g, "C2");
  await area.fill(edited);
  const builds = await viewer.builds();
  await page.evaluate(() => { window.nativePaths = ["C:\\new.sp"]; });
  await deliver(page, "drop", ["C:\\new.sp"]);
  const dialog = page.locator("dialog.dialog");
  await expect(dialog).toContainText("open another file");
  await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
  await dialogGone(page);
  await page.evaluate(() => window.dropDone);
  expect(await viewer.builds()).toBe(builds);
  await expect(area).toHaveValue(edited);
  expect(await page.evaluate(() => window.viewer.edits.unapplied)).toEqual(["edit.sp"]);
});

test("Finder opens preserve absolute paths and ask before leaving unapplied edits", async ({page,viewer})=>{
  await nativeDrop(page,viewer,'edit');
  await page.evaluate(()=>window.viewer.command('toggle-edit'));
  const area=page.locator('#source textarea:visible');
  const edited=(await area.inputValue())+'\n* pending edit';
  await area.fill(edited);
  const builds=await viewer.builds();
  const path='/Users/test/网表 文件/放大器.scs';
  await page.evaluate(path=>{
    window.nativePaths=[path]; window.nativeEvents['opened-files']({payload:[path]});
  },path);
  const dialog=page.locator('dialog.dialog');
  await expect(dialog).toContainText('open another file');
  await dialog.getByRole('button',{name:'Cancel',exact:true}).click();
  await expect(area).toHaveValue(edited);
  expect(await viewer.builds()).toBe(builds);
  expect(await page.evaluate(()=>window.nativeOpened)).toEqual([{path,name:path}]);
});

test("unsupported files show an explanation and a busy viewer ignores drops", async ({ page, viewer }) => {
  await nativeDrop(page, viewer);
  await deliver(page, "drop", ["C:\\notes.txt"]);
  await expect(page.locator("dialog.dialog")).toContainText("Drop a netlist or RNL package file");
  await page.getByRole("button", { name: "Close", exact: true }).click();
  await dialogGone(page);
  await page.evaluate(() => window.dropDone);
  await page.evaluate(() => { document.body.classList.add("busy"); window.nativeInvokes = []; });
  await deliver(page, "enter", ["C:\\new.sp"]);
  await deliver(page, "drop", ["C:\\new.sp"]);
  expect(await page.evaluate(() => window.nativeInvokes)).toEqual([]);
  await expect(page.locator("body")).not.toHaveClass(/dropping/);
  await page.evaluate(() => document.body.classList.remove("busy"));
});

test("the file-association entry explains both builds and opens Settings only after confirmation", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy", { platform: "desktop" });
  await page.evaluate(async () => {
    const { shell } = await import("../shell.js");
    window.defaultAppsCalls = 0;
    shell.fileAssociations = async () => { window.defaultAppsCalls++; };
    void window.viewer.command("file-associations");
  });
  const dialog = page.locator("dialog.dialog");
  await expect(dialog).toContainText(".netlist");
  await expect(dialog).toContainText("standalone EXE");
  await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
  await dialogGone(page);
  expect(await page.evaluate(() => window.defaultAppsCalls)).toBe(0);
  await page.evaluate(() => { void window.viewer.command("file-associations"); });
  await dialog.getByRole("button", { name: "Open Default Apps", exact: true }).click();
  await expect.poll(() => page.evaluate(() => window.defaultAppsCalls)).toBe(1);
});
