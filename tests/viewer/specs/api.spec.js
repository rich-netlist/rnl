// Real cross-origin opener and real Worker, using only verified local public packages.
import fs from "node:fs";
import http from "node:http";
import { expect, test } from "@playwright/test";

const base = process.env.RNL_WEB_URL;
const token = "test-launch-token-123456";
let host;
let origin;
const circuit = name => `* ${name}\nR1 in out 1k\n* @RNL { #R1 @PLACE(0, 0) }\n.end\n`;

test.beforeEach(async ({ page }) => {
  host = http.createServer((_, response) => {
    response.writeHead(200, { "Content-Type": "text/html", "Cross-Origin-Opener-Policy": "same-origin-allow-popups" });
    response.end(`<button id="open">Open</button><script>
      window.messages=[]; addEventListener('message', e=>{if(e.data?.rnl===1) messages.push(e.data)});
      document.getElementById('open').onclick=()=>{
        const url=new URL(${JSON.stringify(base)}); url.search=new URLSearchParams({api:'1',origin:location.origin,token:${JSON.stringify(token)}});
        window.target=window.open(url,'rnl-viewer');
      };</script>`);
  });
  await new Promise(resolve => host.listen(0, "127.0.0.1", resolve));
  origin = `http://127.0.0.1:${host.address().port}`;
  await page.goto(origin);
});
test.afterEach(async () => { host.closeAllConnections(); await new Promise(resolve => host.close(resolve)); });

async function launch(page) {
  const opened = page.waitForEvent("popup");
  await page.locator("#open").click();
  const viewer = await opened;
  await expect.poll(() => page.evaluate(() => messages.find(m => m.type === "ready"))).toMatchObject({
    token, features: expect.arrayContaining(["requestId", "packages", "opener"]), maxOpenBytes: 67108864,
  });
  return viewer;
}
async function send(page, message) {
  await page.evaluate(({ message, base, token }) => target.postMessage({ rnl: 1, token, ...message }, new URL(base).origin), { message, base, token });
}
async function result(page, requestId, type) {
  await expect.poll(() => page.evaluate(({ requestId, type }) => messages.find(m => m.requestId === requestId && m.type === type), { requestId, type })).toBeTruthy();
  return page.evaluate(({ requestId, type }) => messages.find(m => m.requestId === requestId && m.type === type), { requestId, type });
}

test("API backgrounds affect only this page while menu choices keep saving the preference", async ({ page }) => {
  const viewer = await launch(page);
  await viewer.evaluate(() => window.viewer.command("background-dark"));
  await send(page, { type: "background", value: "light" });
  await expect(viewer.locator("body")).toHaveAttribute("data-background", "light");
  expect(await viewer.evaluate(() => JSON.parse(localStorage.getItem("background")))).toBe("dark");
  await viewer.reload();
  await expect(viewer.locator("body")).toHaveAttribute("data-background", "dark");
  await viewer.evaluate(() => window.viewer.command("background-light"));
  expect(await viewer.evaluate(() => JSON.parse(localStorage.getItem("background")))).toBe("light");
});

test("a complete Viewer imports exact packages and asks before replacing unsaved text", async ({ page, browserName }) => {
  const viewer = await launch(page);
  const text = fs.readFileSync(new URL("../inputs/missing-canvas-package.sp", import.meta.url), "utf8");
  await send(page, { type: "open", requestId: "first", name: "canvas.sp", text,
    packages: [{ name: "analog-canvas-extensions", version: "0.1" }] });
  expect(await result(page, "first", "loaded")).toMatchObject({ errors: 0 });
  await expect(viewer.locator("#canvas > svg")).toBeVisible();
  await viewer.locator("#tab-files").click();
  await expect(viewer.getByRole("button", { name: "Open analog-canvas-extensions 0.1", exact: true })).toBeVisible();
  await viewer.locator("#tab-netlist").click();
  await viewer.evaluate(() => window.viewer.command("toggle-edit"));
  const editor = viewer.locator("#source textarea").first();
  await editor.fill(text.replace("E1", "E2"));
  const edited = await editor.inputValue();
  expect(edited).toContain("E2 b 0 a 0 2");
  await send(page, { type: "open", requestId: "decline", name: "new.sp", text: circuit("new") });
  await expect(viewer.getByRole("dialog")).toBeVisible();
  await viewer.getByRole("button", { name: "Cancel", exact: true }).click();
  expect(await result(page, "decline", "error")).toMatchObject({ reason: "declined" });
  expect(await editor.inputValue()).toBe(edited);
  await send(page, { type: "open", requestId: "accept", name: "new.sp", text: circuit("new") });
  await viewer.getByRole("button", { name: "Discard Changes", exact: true }).click();
  await result(page, "accept", "loaded");
  await expect(viewer.locator("#source")).toContainText("* new");
  if (browserName === "chromium") await viewer.screenshot({ path: "test-results/api-full-viewer.png" });
  // The existing same-origin session mechanism must still work when its source is an API tab.
  const copied = viewer.waitForEvent("popup");
  await viewer.evaluate(() => window.viewer.command("new-window"));
  const duplicate = await copied;
  await expect(duplicate.locator("#source")).toContainText("* new");
  expect(new URL(duplicate.url()).searchParams.has("api")).toBe(false);
  expect(await duplicate.evaluate(() => window.opener)).toBeNull();
});

test("superseded requests finish explicitly and late reads cannot replace the latest drawing", async ({ page }) => {
  const viewer = await launch(page);
  let release;
  const blocked = new Promise(resolve => { release = resolve; });
  await viewer.route("**/slow.sp", async route => { await blocked; await route.fulfill({ body: circuit("slow") }); });
  await send(page, { type: "open", requestId: "slow", url: "slow.sp" });
  await expect.poll(() => viewer.evaluate(() => !!document.querySelector("#loading:not([hidden])"))).toBe(true);
  await send(page, { type: "open", requestId: "latest", name: "latest.sp", text: circuit("latest") });
  expect(await result(page, "slow", "error")).toMatchObject({ reason: "superseded" });
  await result(page, "latest", "loaded");
  release();
  await expect(viewer.locator("#loading")).toBeHidden();
  await expect(viewer.locator("#source")).toContainText("* latest");
  await expect(viewer).toHaveTitle("latest.sp - RNL Viewer");
  expect(await page.evaluate(() => messages.some(m => m.type === "loaded" && m.requestId === "slow"))).toBe(false);
});

test("token, origin and window identity reject unrelated opens; package failures retain the document", async ({ page }) => {
  const viewer = await launch(page);
  await send(page, { type: "open", requestId: "valid", name: "valid.sp", text: circuit("valid") });
  await result(page, "valid", "loaded");
  await send(page, { token: "expired-launch-token", type: "open", requestId: "expired", name: "wrong.sp", text: circuit("wrong") });
  await viewer.evaluate(({ token, origin }) => {
    for (const fields of [{ source: window, origin }, { source: opener, origin: "https://wrong.example" }])
      dispatchEvent(new MessageEvent("message", { ...fields, data: { rnl: 1, token, type: "open", requestId: "unrelated", name: "wrong.sp", text: "wrong" } }));
  }, { token, origin });
  await send(page, { type: "open", requestId: "missing", name: "missing.sp", text: circuit("missing"), packages: [{ name: "does-not-exist", version: "0.1" }] });
  expect(await result(page, "missing", "error")).toMatchObject({ message: expect.stringContaining("does-not-exist 0.1") });
  await expect(viewer.locator("#source")).toContainText("* valid");
  expect(await page.evaluate(() => messages.some(m => ["expired", "unrelated"].includes(m.requestId)))).toBe(false);
  await viewer.route("**/public-packages/packages/analog-canvas-extensions/0.1/*.rnl", route => route.fulfill({ body: "altered bytes" }));
  await send(page, { type: "open", requestId: "corrupt", name: "corrupt.sp", text: circuit("corrupt"), packages: [{ name: "analog-canvas-extensions", version: "0.1" }] });
  expect(await result(page, "corrupt", "error")).toMatchObject({ message: expect.stringContaining("SHA-256 mismatch for analog-canvas-extensions 0.1") });
  await expect(viewer.locator("#source")).toContainText("* valid");
});

test("oversize messages fail with measured bytes instead of truncating", async ({ page }) => {
  await launch(page);
  await page.evaluate(({ base, token }) => target.postMessage({ rnl: 1, token, type: "open", requestId: "large", name: "large.sp", text: "x".repeat(67108865) }, new URL(base).origin), { base, token });
  expect(await result(page, "large", "error")).toMatchObject({ message: expect.stringContaining("67108864 bytes (64 MiB)") });
});

test("edits typed during a delayed import require a fresh confirmation", async ({ page }) => {
  const viewer = await launch(page);
  await send(page, { type: "open", requestId: "initial", name: "initial.sp", text: "* initial\nRkeep a 0 1k\n.end\n" });
  await result(page, "initial", "loaded");
  let release, started;
  const blocked = new Promise(resolve => { release = resolve; });
  const requested = new Promise(resolve => { started = resolve; });
  await viewer.route("**/editing-slow.sp", async route => { started(); await blocked; await route.fulfill({ body: "* incoming\nCdiscard a 0 1n\n.end\n" }); });
  await send(page, { type: "open", requestId: "incoming", url: "editing-slow.sp" });
  await requested;
  await viewer.evaluate(() => window.viewer.command("toggle-edit"));
  await viewer.locator("#source textarea").first().fill(circuit("typed while fetching"));
  release();
  await expect(viewer.getByRole("dialog")).toBeVisible();
  await viewer.getByRole("button", { name: "Cancel", exact: true }).click();
  expect(await result(page, "incoming", "error")).toMatchObject({ reason: "declined" });
  await expect(viewer.locator("#source textarea").first()).toHaveValue(circuit("typed while fetching"));
  await viewer.locator("#tab-unplaced").click();
  await expect(viewer.locator("#unplaced-list svg").first()).toBeVisible();
  await expect(viewer.locator("#unplaced-list")).toContainText("Rkeep");
  await expect(viewer.locator("#unplaced-list")).not.toContainText("document was built again");
});

test("a newer open replaces an obsolete confirmation instead of falsely declining", async ({ page }) => {
  const viewer = await launch(page);
  await send(page, { type: "open", requestId: "initial", name: "initial.sp", text: circuit("initial") });
  await result(page, "initial", "loaded");
  await viewer.evaluate(() => window.viewer.command("toggle-edit"));
  await viewer.locator("#source textarea").first().fill(circuit("edited"));
  await send(page, { type: "open", requestId: "obsolete", name: "obsolete.sp", text: circuit("obsolete") });
  await expect(viewer.getByRole("dialog")).toBeVisible();
  await send(page, { type: "open", requestId: "newest", name: "newest.sp", text: circuit("newest") });
  expect(await result(page, "obsolete", "error")).toMatchObject({ reason: "superseded" });
  await expect(viewer.getByRole("dialog")).toContainText("newest.sp");
  await viewer.getByRole("button", { name: "Discard Changes", exact: true }).click();
  await result(page, "newest", "loaded");
  await expect(viewer.locator("#source")).toContainText("* newest");
});

for (const first of ["api", "ordinary"]) test(`API and ordinary opens share ownership when ${first} starts first`, async ({ page }) => {
  const viewer = await launch(page);
  let release, started;
  const blocked = new Promise(resolve => { release = resolve; });
  const requested = new Promise(resolve => { started = resolve; });
  await viewer.route("**/ownership-slow.sp", async route => { started(); await blocked; await route.fulfill({ body: circuit("obsolete") }); });
  const slow = `${base}/ownership-slow.sp`;
  if (first === "api") await send(page, { type: "open", requestId: "slow", url: slow });
  else {
    await viewer.evaluate(() => { void window.viewer.command("open-url"); });
    await viewer.getByRole("textbox").fill(slow);
    await viewer.getByRole("button", { name: "Open", exact: true }).click();
  }
  await requested;
  if (first === "api") {
    await viewer.locator("#file-input").setInputFiles({ name: "manual.sp", mimeType: "text/plain", buffer: Buffer.from(circuit("manual")) });
    expect(await result(page, "slow", "error")).toMatchObject({ reason: "superseded" });
    await expect(viewer.locator("#source")).toContainText("* manual");
  } else {
    await send(page, { type: "open", requestId: "latest", name: "latest.sp", text: circuit("latest") });
    await result(page, "latest", "loaded");
  }
  release();
  await expect(viewer.locator("#loading")).toBeHidden();
  await expect(viewer.locator("#source")).toContainText(first === "api" ? "* manual" : "* latest");
  await expect(viewer).toHaveTitle(`${first === "api" ? "manual" : "latest"}.sp - RNL Viewer`);
});
