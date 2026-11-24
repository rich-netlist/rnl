// A file of an earlier version of the language, 0.2 or 0.3, which the core reads none of the RNL of (apps/viewer-ui/old-version.js):
// the page of a level written in one says so in place of the drawing, with the old viewer that reads it. The web viewer
// links to the site's old web viewer on the same circuit: in the same tab for ?url= and ?manifest=, in a new tab for a file
// from this computer and for an embedded canvas; the desktop to the release of RNL Viewer 0.5.3, in the system's browser;
// the local program to that release and to the old web viewer. RNL_OLD_VERSION_SHOTS names a folder for screenshots.

import fs from "node:fs";
import path from "node:path";
import { createHash } from "node:crypto";
import { expect, test } from "../viewer.js";
import { root } from "../fixtures.js";

const OLD_VIEWER = "https://rich-netlist.github.io/rnl/viewer-0.3/";
const OLD_RELEASE = "https://github.com/rich-netlist/rnl/releases/tag/viewer-v0.5.3";
const inputs = path.join(root, "tests/viewer/inputs/old-version");
const notice = (page) => page.locator("#empty .old-version");
const shots = process.env.RNL_OLD_VERSION_SHOTS;

/** Keep a screenshot of the page as `name`.png when RNL_OLD_VERSION_SHOTS names a folder, of the first browser only. */
async function shot(page, name, browserName) {
  if (!shots || browserName !== "chromium") return;
  fs.mkdirSync(shots, { recursive: true });
  await page.screenshot({ path: path.join(shots, `${name}.png`) });
}

/** Whether the page offers the command `id` now. */
const enabled = (page, id) => page.evaluate(async (id) => (await import("./commands.js")).isEnabled(id), id);

/**
 * Nothing offers to solve the level shown: no status of what a solver would place, Solve and Respace not enabled, and F9
 * does nothing, with no dialog. (Solve All Circuits stays: the other files of a document may be of 0.4.)
 */
async function nothingSolves(page, viewer) {
  await expect(page.locator("#status")).toHaveText("");
  expect(await enabled(page, "solve")).toBe(false);
  expect(await enabled(page, "solve-respace")).toBe(false);
  await page.keyboard.press("F9");
  await page.waitForTimeout(300);
  await expect(page.locator("dialog.dialog")).toHaveCount(0);
  await expect(notice(page)).toBeVisible();
  if (viewer) expect(await viewer.calls("solve")).toEqual([]);
}

/** Enter the subcircuit rdiv by its instance X1, written in the library of 0.3 that the netlist of 0.4 includes. */
async function enterRdiv(page, viewer) {
  const at = await viewer.where("X1");
  await page.mouse.dblclick(at.x, at.y);
  await expect.poll(() => page.evaluate(() => window.viewer.doc.levels[window.viewer.level]?.title)).toBe("rdiv");
}

/** The old viewer's address with these parameters, in this order. */
const oldViewer = (params) => `${OLD_VIEWER}?${new URLSearchParams(params)}`;

test.describe("the page, with the fake shell", () => {
  test("the desktop says which release reads a file of 0.3, and opens its page in the system's browser", async ({ page, viewer, browserName }) => {
    await viewer.open("old-rc-0.3", { platform: "desktop" });
    await expect(notice(page)).toHaveText(
      "Unsupported old file format (RNL 0.3).RNL Viewer 0.5.3 opens it: download it from its release page.");
    const link = notice(page).getByRole("link", { name: "its release page" });
    await expect(link).toHaveAttribute("href", OLD_RELEASE);
    // in place of the page that says nothing has a position yet: no Solve link, no bar over the drawing
    await expect(page.locator("#empty .solve-link")).toHaveCount(0);
    await expect(page.locator("#unplaced-bar")).toBeHidden();
    await nothingSolves(page, viewer);
    await shot(page, "desktop", browserName);
    await link.click();
    expect(await viewer.calls("browse")).toEqual([[OLD_RELEASE]]);
  });

  test("a file of 0.2 is told the same, with its version", async ({ page, viewer }) => {
    await viewer.open("old-rc-0.2", { platform: "desktop" });
    await expect(notice(page)).toContainText("Unsupported old file format (RNL 0.2).");
  });

  test("the local program names the release and the old web viewer, where the file is opened by hand", async ({ page, viewer }) => {
    await viewer.open("old-rc-0.3", { platform: "desktop", "old-viewer": "release-and-page" });
    await expect(notice(page)).toHaveText("Unsupported old file format (RNL 0.3).RNL Viewer 0.5.3 opens it: download it from its release page."
      + "Or try the old web viewer in a browser: open the file there with File › Open.");
    await expect(notice(page).getByRole("link", { name: "its release page" })).toHaveAttribute("href", OLD_RELEASE);
    await expect(notice(page).getByRole("link", { name: "the old web viewer" })).toHaveAttribute("href", OLD_VIEWER);
  });

  test("in a document of 0.4, only the level written in a file of 0.3 has the notice", async ({ page, viewer }) => {
    await viewer.open("old-mixed", { platform: "desktop" });
    expect(await page.evaluate(() => window.viewer.doc.levels[window.viewer.level].empty)).toBe(false);
    await expect(notice(page)).toHaveCount(0);
    expect(await enabled(page, "solve")).toBe(true);
    await enterRdiv(page, viewer);
    await expect(notice(page)).toContainText("Unsupported old file format (RNL 0.3).");
    await nothingSolves(page, viewer);
    // back up to the level of 0.4: Solve again
    await page.keyboard.press("Backspace");
    await expect.poll(() => page.evaluate(() => window.viewer.doc.levels[window.viewer.level]?.title)).toBe("mixed-top.sp");
    expect(await enabled(page, "solve")).toBe(true);
  });

  test("the notice is said again in the language chosen", async ({ page, viewer }) => {
    await viewer.open("old-rc-0.3", { platform: "desktop" });
    await expect(notice(page)).toBeVisible();
    await page.evaluate(async () => {
      const i18n = await import("./i18n.js");
      await i18n.setLanguage(i18n.uiLanguages().find((l) => l.lang === "zh").code);
    });
    // its two lines again, in Chinese, the link in the second
    await expect(notice(page).locator("br")).toHaveCount(1);
    await expect(notice(page).getByRole("link")).toHaveAttribute("href", OLD_RELEASE);
    await expect(notice(page)).toContainText("不支持的旧文件格式（RNL 0.3）");
  });
});

test.describe("the web viewer", () => {
  test.skip(!process.env.RNL_WEB_URL, "the site is not built: RNL_WEB=0");
  const base = () => process.env.RNL_WEB_URL;
  const FILES = "https://files.test/old/";
  const MANIFEST = "https://manifests.test/old/";
  const sha256 = (bytes) => createHash("sha256").update(bytes).digest("hex");

  /** Serve the inputs under /old/ of the site and under FILES, a manifest of each entry under MANIFEST, and a stand-in for the old viewer. */
  test.beforeEach(async ({ page }) => {
    await page.route("https://rich-netlist.github.io/rnl-packages/**", (route) => route.abort());
    const file = (url) => {
      const name = new URL(url).pathname.split("/").pop();
      const at = path.join(inputs, name);
      return fs.existsSync(at) ? fs.readFileSync(at) : null;
    };
    const serve = (route) => {
      const body = file(route.request().url());
      return body === null ? route.fulfill({ status: 404, body: "missing" })
        : route.fulfill({ status: 200, contentType: "text/plain", headers: { "Access-Control-Allow-Origin": "*" }, body });
    };
    await page.route(`${base()}/old/**`, serve);
    await page.route(`${FILES}**`, serve);
    await page.route(`${MANIFEST}**`, (route) => {
      const entry = new URL(route.request().url()).pathname.split("/").pop().replace(/\.json$/, "");
      const names = entry === "mixed-top.sp" ? ["mixed-top.sp", "cells-0.3.inc"] : [entry];
      const files = names.map((name) => { const bytes = fs.readFileSync(path.join(inputs, name)); return { path: name, sha256: sha256(bytes), size: bytes.length }; });
      return route.fulfill({ status: 200, contentType: "application/json", headers: { "Access-Control-Allow-Origin": "*" },
        body: JSON.stringify({ format: 1, id: "old", base: FILES, entry, files, pool: [] }) });
    });
    await page.route(`${OLD_VIEWER}**`, (route) => route.fulfill({ contentType: "text/html", body: "<!doctype html><title>old viewer</title><p>old viewer" }));
  });

  /** Open the web viewer with `query` and wait until it shows a document and reads nothing. */
  async function start(page, query) {
    await page.goto(`${base()}/index.html${query}`);
    await page.waitForFunction(() => window.viewer?.doc && !document.body.classList.contains("busy"));
  }

  test("?url= of a file of 0.3 links the old viewer to the same address, in the same tab, with the display it reads", async ({ page, browserName }) => {
    await start(page, "?url=old/rc-0.3.sp&grid=dots&roles=1");
    await expect(notice(page)).toHaveText("Unsupported old file format (RNL 0.3).You can try opening it in the old viewer.");
    const link = notice(page).getByRole("link", { name: "the old viewer" });
    const href = oldViewer({ url: `${base()}/old/rc-0.3.sp`, grid: "dots" });
    await expect(link).toHaveAttribute("href", href);
    expect(await link.getAttribute("target")).toBeNull();
    await nothingSolves(page, null);
    // the page's own Solve menu has Solve and Respace off
    await page.locator("#menubar .menu-title", { hasText: "Solve" }).click();
    await expect(page.locator('#menubar .menu-item[data-id="solve"]')).toBeDisabled();
    await page.keyboard.press("Escape");
    await shot(page, "web-url", browserName);
    await link.click();
    await expect(page).toHaveURL(href);
  });

  test("?url= of a document of 0.4 that includes a file of 0.3: the level written there links the old viewer to that file", async ({ page, viewer }) => {
    await start(page, "?url=old/mixed-top.sp");
    await expect(notice(page)).toHaveCount(0);
    await enterRdiv(page, viewer);
    await expect(notice(page)).toContainText("Unsupported old file format (RNL 0.3).");
    await expect(notice(page).getByRole("link")).toHaveAttribute("href", oldViewer({ url: `${base()}/old/cells-0.3.inc` }));
  });

  test("?manifest= whose entry is of 0.3 links the old viewer to the same manifest, in the same tab", async ({ page, browserName }) => {
    await start(page, `?manifest=${encodeURIComponent(`${MANIFEST}rc-0.3.sp.json`)}`);
    const link = notice(page).getByRole("link", { name: "the old viewer" });
    await expect(link).toHaveAttribute("href", oldViewer({ manifest: `${MANIFEST}rc-0.3.sp.json` }));
    expect(await link.getAttribute("target")).toBeNull();
    await shot(page, "web-manifest", browserName);
  });

  test("?manifest= of a document of 0.4 with a file of 0.3: the level written there links the old viewer to that file's address", async ({ page, viewer }) => {
    await start(page, `?manifest=${encodeURIComponent(`${MANIFEST}mixed-top.sp.json`)}`);
    await expect(notice(page)).toHaveCount(0);
    await enterRdiv(page, viewer);
    await expect(notice(page).getByRole("link")).toHaveAttribute("href", oldViewer({ url: `${FILES}cells-0.3.inc` }));
  });

  test("a file from this computer cannot be passed: the old viewer opens alone, in a new tab, and the notice says to open the file there", async ({ page, browserName }) => {
    await page.goto(`${base()}/index.html`);
    await expect(page.locator("#menubar")).toBeVisible();
    await page.locator("#file-input").setInputFiles({ name: "rc-0.3.sp", mimeType: "text/plain", buffer: fs.readFileSync(path.join(inputs, "rc-0.3.sp")) });
    await expect(notice(page)).toHaveText("Unsupported old file format (RNL 0.3).You can try the old viewer: open the file there with File › Open, "
      + "since a file from this computer cannot be passed to it by a link.");
    const link = notice(page).getByRole("link", { name: "the old viewer" });
    await expect(link).toHaveAttribute("href", OLD_VIEWER);
    await expect(link).toHaveAttribute("target", "_blank");
    await shot(page, "web-local-file", browserName);
  });

  test("an embedded canvas shows the notice in place of the error, links the full old viewer in a new tab, and still answers with the error", async ({ page, browserName }) => {
    const url = new URL("/", base());
    url.search = new URLSearchParams({ embed: "1", url: "old/rc-0.3.sp", background: "dark" });
    await page.route("**/old-embed-host", (route) => route.fulfill({ contentType: "text/html", body: `<!doctype html><html><head>
      <style>body{margin:0;font:14px sans-serif}iframe{width:900px;height:500px;border:1px solid #ccc}</style></head><body>
      <p>A page that embeds the viewer</p><script>window.messages = [];
      window.addEventListener('message', (e) => { if (e.data?.rnl === 1) window.messages.push(e.data); });</script>
      <iframe id="viewer" title="Embedded viewer" src=${JSON.stringify(url.href)}></iframe></body></html>` }));
    await page.goto(`${base()}/old-embed-host`);
    const frame = page.frameLocator("#viewer");
    await expect(frame.locator("#empty .old-version")).toHaveText("Unsupported old file format (RNL 0.3).You can try opening it in the old viewer.");
    const link = frame.locator("#empty .old-version").getByRole("link", { name: "the old viewer" });
    await expect(link).toHaveAttribute("href", oldViewer({ url: `${base()}/old/rc-0.3.sp`, background: "dark" }));
    await expect(link).toHaveAttribute("target", "_blank");
    await expect.poll(() => page.evaluate(() => window.messages.find((m) => m.type === "error")?.message ?? null))
      .toMatch(/^Could not open the netlist: unsupported version 0\.3/);
    await shot(page, "embed", browserName);
  });
});
