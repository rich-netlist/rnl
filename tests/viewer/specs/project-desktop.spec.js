// Projects on the desktop (apps/viewer-ui/project.js) through the fake shell: Save Project names the
// Library's files relative to the directory they share and copies each PATH directory under
// PATH/<n>-<name>/; Open Project makes the project's pool the Library's files, with its formats for
// this window only.
import { test, expect } from "../viewer.js";
import { execFileSync } from "node:child_process";

/** Python reads the archive independently, checking every CRC, and gives each file's text. */
function unzip(bytes) {
  return JSON.parse(execFileSync("python", ["-c",
    "import sys,io,zipfile,json; z=zipfile.ZipFile(io.BytesIO(sys.stdin.buffer.read())); assert z.testzip() is None; print(json.dumps({n:z.read(n).decode() for n in z.namelist()}))"],
  { input: Buffer.from(bytes), maxBuffer: 32 << 20 }).toString());
}

test("desktop: Save Project keeps paths relative to the files' directory, PATH copies and the formats chosen; Open Project reads it back", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy", { platform: "desktop", pool: "1", settings: "1" });
  await page.evaluate(() => {
    window.fakeShell.packages = { "Z:/pdk": ["Z:/pdk/a.rnl", "Z:/pdk/1.0/b.rnl"] };
    window.fakeShell.path = "Z:/pdk";
  });
  await page.locator("#tab-files").click();
  await page.locator("#add-path").click();
  await expect(page.locator('.lib-group[data-group="path:Z:/pdk"]')).toHaveCount(1);
  await viewer.idle();
  const doc = await page.evaluate(() => window.viewer.doc);
  const entry = await page.evaluate(async () => {
    const { sourcePath } = await import("./source-path.js");
    return sourcePath(window.viewer.doc, window.viewer.doc.files[0]);
  });
  const dir = entry.slice(0, entry.lastIndexOf("/"));
  const extra = `${dir}/lib/extra.inc`;
  await page.evaluate(async ({ extra }) => {
    window.fakeShell.disk = { [extra]: "* extra\n", "Z:/pdk/a.rnl": "* a\n", "Z:/pdk/1.0/b.rnl": "* b\n" };
    const { restorePool } = await import("./pool.js");
    await restorePool([{ path: extra, name: extra, source: { path: extra, name: extra } }]);
    await (await import("./formats.js")).chooseFormat(extra, "spectre");
  }, { extra });
  await page.evaluate(() => window.viewer.command("save-project"));
  await expect.poll(() => page.evaluate(() => !!window.fakeShell.savedProject)).toBe(true);
  const saved = await page.evaluate(() => window.fakeShell.savedProject);
  const files = unzip(saved.bytes);
  const manifest = JSON.parse(files["rnl-project.json"]);
  const name = entry.split("/").pop();
  expect(saved.name).toBe(name.replace(/\.[^.]*$/, ".zip"));
  expect(manifest.entry).toBe(name);
  expect(files["lib/extra.inc"]).toBe("* extra\n");
  expect(files["PATH/1-pdk/a.rnl"]).toBe("* a\n");
  expect(files["PATH/1-pdk/1.0/b.rnl"]).toBe("* b\n");
  for (const file of doc.files) expect(Object.keys(files)).toContain(file.path.split("/").pop());
  expect(manifest.pool.slice(-3)).toEqual(["lib/extra.inc", "PATH/1-pdk/a.rnl", "PATH/1-pdk/1.0/b.rnl"]);
  expect(manifest.paths).toEqual([{ path: "PATH/1-pdk", subfolders: false }]);
  expect(manifest.formats).toEqual({ "lib/extra.inc": "spectre" });

  // Open Project: the files go in the folder chosen; its pool becomes the Library's files
  await page.evaluate((saved) => {
    window.fakeShell.project = { name: saved.name, bytes: saved.bytes };
    window.fakeShell.projectRoot = "C:/opened/";
  }, saved);
  await page.evaluate(() => window.viewer.command("open-project"));
  await expect.poll(() => page.evaluate(() => window.fakeShell.written?.length)).toBe(Object.keys(files).length - 1);
  await expect(page.locator('.lib-group[data-group="files"] .pool-open')).toContainText(["C:/opened/lib/extra.inc", "C:/opened/PATH/1-pdk/a.rnl"]);
  // the project's format holds in this window, but the settings keep only what the user chose
  expect(await page.evaluate(async () => (await import("./formats.js")).chosenFormat("C:/opened/lib/extra.inc"))).toBe("spectre");
  const kept = await page.evaluate(() => JSON.parse(sessionStorage.getItem("fake-settings"))["file-formats"] || {});
  expect(Object.keys(kept)).not.toContain("C:/opened/lib/extra.inc");
  // the desktop's PATH is not changed by a project
  expect(await page.evaluate(() => JSON.parse(sessionStorage.getItem("fake-settings"))["search-paths"])).toEqual(["Z:/pdk"]);
});
