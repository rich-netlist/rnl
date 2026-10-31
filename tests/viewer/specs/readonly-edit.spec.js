// A read-only file (a manifest's, a built-in package, a public package): its text can be edited and the changes applied,
// like any file's, and what Save never does is overwrite its source (the desktop saves it as another file, the web
// downloads it, which overwrites nothing); a solve is made of it as of any other; and a solve's result, like Close
// Preview's, is shown fitted to the window, since its layout can lie outside the view of the layout before it.

import { expect, test } from "../viewer.js";

const T4 = "T4-miller-conflicts";
const dialog = (page) => page.locator("dialog.dialog");

/** A source the shell gives as read only: a file it holds, not one to overwrite. */
const READONLY = { name: "T4-miller-conflicts", fixture: T4, path: "/lib/T4-miller-conflicts.sp", readonly: true };

/** Open the read-only source in place of the page's file, as File > Open does. */
async function openReadonly(page, viewer) {
  await page.evaluate((source) => (window.fakeShell.picks = [source]), READONLY);
  await viewer.build(() => page.evaluate(() => window.viewer.command("open")));
  await viewer.idle();
}

/** The first text box's text with `text` added at its end, written into it. */
async function type(page, text) {
  const area = page.locator("#source textarea:not([hidden])").first();
  await area.fill(`${await area.inputValue()}${text}`);
}

for (const platform of ["web", "desktop"]) {
  test.describe(`on the ${platform}`, () => {
    test.beforeEach(async ({ page, viewer }) => {
      await viewer.open("04-hierarchy", { platform, pool: "1" });
      await openReadonly(page, viewer);
    });

    test("a read-only file is edited and the changes are applied, as any file's are", async ({ page, viewer }) => {
      await page.evaluate(() => window.viewer.command("toggle-edit"));
      expect(await page.evaluate(() => window.viewer.checked("toggle-edit"))).toBe(true);
      const area = page.locator("#source textarea:not([hidden])").first();
      await expect(area).toBeVisible();
      expect(await area.evaluate((el) => el.readOnly)).toBe(false);
      await type(page, "\n* edited\n");
      await expect(page.locator("#edit-state")).toHaveText("Changes not applied yet");
      const file = await page.evaluate(() => window.viewer.doc.files[0].path);
      await viewer.build(() => page.locator("#apply-edits").click());
      await expect(page.locator("#edit-state")).toBeHidden();
      // the edit fed the build, by the file's path, and is kept as text not saved
      const sent = await page.evaluate(() => window.fakeShell.buildEdits.at(-1));
      expect(Object.keys(sent)).toEqual([file]);
      expect(sent[file]).toContain("* edited");
      expect((await page.evaluate(() => window.viewer.edits)).unsaved).toEqual([file]);
      expect(await page.evaluate(() => window.viewer.doc.files[0].text)).toContain("* edited");
      // Reload asks before it drops them, as for any file; Cancel keeps them
      const builds = await viewer.builds();
      await page.keyboard.press("F5");
      await expect(dialog(page).locator(".dialog-title")).toHaveText("Changes Not Saved");
      await dialog(page).getByRole("button", { name: "Cancel" }).click();
      expect(await viewer.builds()).toBe(builds);
      expect((await page.evaluate(() => window.viewer.edits)).unsaved).toEqual([file]);
    });

    test("changes not saved are asked about before another file is opened, as for any file", async ({ page, viewer }) => {
      await page.evaluate(() => window.viewer.command("toggle-edit"));
      await type(page, "\n* edited\n");
      await viewer.build(() => page.locator("#apply-edits").click());
      await page.evaluate(() => (window.fakeShell.picks = ["04-hierarchy"]));
      await page.evaluate(() => window.viewer.command("open"));
      await expect(dialog(page)).toContainText("Do you want to open another file?");
      await dialog(page).getByRole("button", { name: "Cancel" }).click();
      expect((await page.evaluate(() => window.viewer.edits)).unsaved).toHaveLength(1);
    });
  });
}

test.describe("saving a read-only file", () => {
  test("on the desktop it is saved as another file, and its source is not written", async ({ page, viewer }) => {
    await viewer.open("04-hierarchy", { platform: "desktop", pool: "1" });
    await openReadonly(page, viewer);
    await page.evaluate(() => window.viewer.command("toggle-edit"));
    await type(page, "\n* edited\n");
    await page.evaluate(() => (window.fakeShell.savedAs = { name: "copy", fixture: "04-hierarchy", path: "/copies/copy.sp" }));
    await viewer.build(() => page.evaluate(() => window.viewer.command("save")));
    expect(await viewer.calls("save")).toEqual([]);
    expect(await viewer.calls("saveAs")).toEqual([[READONLY.name]]);
    const [written] = await page.evaluate(() => window.fakeShell.writesAs);
    expect(written.text).toContain("* edited");
    // the file written is the one shown now, and it is not read only
    await expect(page.locator("#edit-state")).toHaveCount(1);
    expect(await page.evaluate(() => window.viewer.edits)).toEqual({ unsaved: [], unapplied: [] });
  });

  test("on the desktop Save As does the same", async ({ page, viewer }) => {
    await viewer.open("04-hierarchy", { platform: "desktop", pool: "1" });
    await openReadonly(page, viewer);
    await page.evaluate(() => window.viewer.command("toggle-edit"));
    await type(page, "\n* edited\n");
    await page.evaluate(() => (window.fakeShell.savedAs = { name: "copy", fixture: "04-hierarchy", path: "/copies/copy.sp" }));
    await viewer.build(() => page.evaluate(() => window.viewer.command("save-as")));
    expect(await viewer.calls("save")).toEqual([]);
    expect(await viewer.calls("saveAs")).toEqual([[READONLY.name]]);
  });

  test("on the web Download File downloads the file shown, with the changes", async ({ page, viewer }) => {
    await viewer.open("04-hierarchy", { platform: "web", pool: "1" });
    await openReadonly(page, viewer);
    await page.evaluate(() => window.viewer.command("toggle-edit"));
    await type(page, "\n* edited\n");
    await page.evaluate(() => window.viewer.command("save"));
    await expect.poll(async () => (await viewer.calls("save")).length).toBe(1);
    const [[file]] = await page.evaluate(() => window.fakeShell.writes);
    expect(file.text).toContain("* edited");
    await expect(page.locator("#note")).toContainText("Downloaded");
  });
});

test.describe("solving a read-only file, on the desktop", () => {
  test.beforeEach(async ({ page, viewer }) => {
    await viewer.open("04-hierarchy", { platform: "desktop", pool: "1" });
    await openReadonly(page, viewer);
  });

  test("F9 solves it, with the changes applied, and shows the preview", async ({ page, viewer }) => {
    await page.evaluate(() => window.viewer.command("toggle-edit"));
    await type(page, "\n* edited\n");
    await viewer.build(() => page.locator("#apply-edits").click());
    await page.keyboard.press("F9");
    await page.waitForFunction(() => document.body.classList.contains("preview") && !document.body.classList.contains("solving"));
    expect(await viewer.calls("solve")).toHaveLength(1);
    const [edits] = await page.evaluate(() => window.fakeShell.solveEdits);
    expect(Object.values(edits).join("")).toContain("* edited");
    await expect(page.locator("#preview-badge")).toBeVisible();
  });
});

test.describe("the drawing is fitted to the window after a solve and after Close Preview, on both platforms", () => {
  for (const platform of ["web", "desktop"]) {
    if (platform === "web") continue; // the web has a Solve only through a development server (web.spec.js)
    test(`on the ${platform}`, async ({ page, viewer }) => {
      await viewer.open(T4, { platform });
      const centre = () => page.evaluate(async () => (await import("./drawing.js")).viewCentre());
      const fitted = await centre();
      // a view the solve's layout is not in: zoomed in, and away from the middle
      await page.evaluate(async () => {
        const drawing = await import("./drawing.js");
        drawing.zoomCenter(8);
        drawing.pan(-300, 200);
      });
      const before = await centre();
      expect(before.scale).not.toBeCloseTo(fitted.scale, 3);
      await page.evaluate(() => window.viewer.command("solve"));
      await page.waitForFunction(() => document.body.classList.contains("preview") && !document.body.classList.contains("solving"));
      const solved = await centre();
      expect(solved.scale).not.toBeCloseTo(before.scale, 3);
      // it is the view a fit gives: nothing changes when it is asked for
      await page.evaluate(async () => (await import("./drawing.js")).fit());
      const again = await centre();
      expect(solved.scale).toBeCloseTo(again.scale, 6);
      expect(solved.center[0]).toBeCloseTo(again.center[0], 6);
      expect(solved.center[1]).toBeCloseTo(again.center[1], 6);
      // and Close Preview shows the netlist fitted too
      await page.evaluate(async () => {
        const drawing = await import("./drawing.js");
        drawing.zoomCenter(8);
        drawing.pan(-300, 200);
      });
      await viewer.build(() => page.evaluate(() => window.viewer.command("solve-close")));
      await expect(page.locator("#preview-badge")).toBeHidden();
      const closed = await centre();
      expect(closed.scale).toBeCloseTo(fitted.scale, 3);
    });
  }
});
