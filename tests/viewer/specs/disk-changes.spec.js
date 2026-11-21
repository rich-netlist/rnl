// A file of the document shown that changed on disk since it was read (disk-changes.js): the shell looks at the files
// the page gives it, and the page asks once a second what changed. The desktop's and the local program's shells do;
// the web reads no file of a disk.
import { expect, test } from "../viewer.js";

test("desktop: the files of the document shown are looked at, and one that changed on disk is told of, with F5", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy", { platform: "desktop" });
  // the shell is given the files of the document shown, by the paths the page names them by
  const shown = await page.evaluate(async () => {
    const { sourcePath } = await import("./source-path.js");
    return window.viewer.doc.files.map((f) => sourcePath(window.viewer.doc, f));
  });
  await expect.poll(() => page.evaluate(() => window.fakeShell.watched)).toEqual(shown);
  // a file that is not of the document shown is not told of
  await page.evaluate(() => { window.fakeShell.changesOnDisk = { version: 1, changed: ["/elsewhere/other.sp"] }; });
  await page.waitForTimeout(1500);
  await expect(page.locator("#note")).toBeHidden();
  // one of it is
  await page.evaluate((path) => { window.fakeShell.changesOnDisk = { version: 2, changed: [path] }; }, shown[0]);
  await expect(page.locator("#note")).toHaveText(`${shown[0].split("/").pop()} changed on disk. F5 reloads the netlist.`);
});

test("web: the page asks nothing of a disk", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy");
  expect(await page.evaluate(async () => (await import("./shell.js")).shell.fileChanges)).toBeUndefined();
});
