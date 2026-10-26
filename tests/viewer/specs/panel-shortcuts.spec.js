import { dialogGone, expect, test } from "../viewer.js";

const panes = ["files", "netlist", "hierarchy", "symbol", "usages", "unplaced", "solve"];

for (const [resource, platform, key, pane] of [
  ["main.js", "web", "6", "unplaced"],
  ["commands.json", "web", "6", "unplaced"],
  ["commands.json", "desktop", "7", "solve"],
  ["commands.json", "web", "7", "unplaced"],
]) {
  test(`${platform}: startup preserves panel shortcut ${key} while ${resource} is loading`, async ({ page, viewer }) => {
    let release;
    const ready = new Promise(resolve => release = resolve);
    await page.route(`**/${resource}`, async route => { await ready; await route.continue(); });
    await page.goto(`${process.env.RNL_VIEWER_URL}/?shell=fake&platform=${platform}&open=04-hierarchy`, { waitUntil: "commit" });
    // startup.js is independent of the main module graph and the command table.
    await page.waitForFunction(() => document.getElementById("viewer-style")?.rel === "stylesheet");
    await page.keyboard.press("1");
    await page.keyboard.press("6");
    await page.keyboard.press(key);
    // Modified/composing keys and actual text input must not replace that request.
    await page.keyboard.press("Control+3");
    await page.keyboard.press("Shift+4");
    await page.locator("body").dispatchEvent("keydown", { key: "5", isComposing: true });
    await page.evaluate(() => {
      const input = document.createElement("input");
      input.id = "startup-test-input";
      document.body.append(input);
      input.focus();
    });
    await page.keyboard.type("123");
    await expect(page.locator("#startup-test-input")).toHaveValue("123");
    await page.locator("#startup-test-input").evaluate(el => el.remove());
    release();
    await viewer.started();
    await viewer.idle();
    await expect(page.locator("#startup")).toHaveCount(0);
    await expect(page.locator(`#pane-${pane}`)).toBeVisible();
    await expect(page.locator(`#tab-${pane}`)).toBeFocused();
    expect(await viewer.calls("solve")).toEqual([]);
    await page.keyboard.press("3");
    await expect(page.locator("#pane-hierarchy")).toBeVisible();
  });
}

for (const platform of ["web", "desktop"]) {
  test(`${platform}: panel keys work at once while the core is still building the first document`, async ({ page, viewer }) => {
    let release;
    const ready = new Promise(resolve => release = resolve);
    // the fixture is what the shell's core gives: holding it holds the first document's build
    await page.route("**/fixtures/04-hierarchy.json", async route => { await ready; await route.continue(); });
    await page.goto(`${process.env.RNL_VIEWER_URL}/?shell=fake&platform=${platform}&open=04-hierarchy`, { waitUntil: "commit" });
    await viewer.started();
    await expect(page.locator("#startup")).toHaveCount(0);
    expect(await page.evaluate(() => window.viewer.doc)).toBeNull();
    await page.keyboard.press("3");
    await expect(page.locator("#pane-hierarchy")).toBeVisible();
    await page.keyboard.press("1");
    await expect(page.locator("#pane-files")).toBeVisible();
    release();
    await viewer.idle();
    expect(await page.evaluate(() => window.viewer.doc?.name)).toBeTruthy();
    await expect(page.locator("#pane-files")).toBeVisible();
  });
}

test("a panel key pressed before startup.js arrives still runs once the viewer has started", async ({ page, viewer }) => {
  let release;
  const ready = new Promise(resolve => release = resolve);
  await page.route("**/startup.js", async route => { await ready; await route.continue(); });
  await page.goto(`${process.env.RNL_VIEWER_URL}/?shell=fake&open=04-hierarchy`, { waitUntil: "commit" });
  await page.waitForSelector("#startup");
  await page.keyboard.press("6");
  await page.keyboard.press("Shift+4");
  await page.keyboard.press("3");
  release();
  await viewer.started();
  await viewer.idle();
  await expect(page.locator("#startup")).toHaveCount(0);
  await expect(page.locator("#pane-hierarchy")).toBeVisible();
  // the inline listener is gone once startup.js has taken over
  await page.keyboard.press("2");
  await expect(page.locator("#pane-netlist")).toBeVisible();
});

test("a later panel key wins while the native startup handoff is pending", async ({ page, viewer }) => {
  await page.addInitScript(() => {
    window.__TAURI__ = {
      app: { getVersion: async () => "0.2.0" },
      core: { invoke: () => new Promise(resolve => { window.finishNativeHandoff = resolve; }) },
    };
  });
  await page.route("**/shells/desktop.js", route => route.fulfill({
    contentType: "text/javascript", body: "export { default } from './fake.js';",
  }));
  let release;
  const ready = new Promise(resolve => release = resolve);
  await page.route("**/main.js", async route => { await ready; await route.continue(); });
  await page.goto(`${process.env.RNL_VIEWER_URL}/?platform=desktop&open=04-hierarchy`, { waitUntil: "commit" });
  await page.waitForFunction(() => document.getElementById("viewer-style")?.rel === "stylesheet");
  await page.keyboard.press("6");
  release();
  await viewer.started();
  await viewer.idle();
  await page.waitForFunction(() => window.finishNativeHandoff && !document.body.classList.contains("starting"));
  await page.keyboard.press("3");
  await expect(page.locator("#pane-hierarchy")).toBeVisible();
  await page.evaluate(async () => {
    window.finishNativeHandoff();
    // A paint opportunity lets the pending startup continuation finish too.
    await new Promise(requestAnimationFrame);
  });
  await expect(page.locator("#pane-hierarchy")).toBeVisible();
  await expect(page.locator("#tab-hierarchy")).toBeFocused();
});

for (const platform of ["desktop", "web"]) {
  test(`${platform}: digits select panes and reopen the collapsed sidebar without rebuilding`, async ({ page, viewer }) => {
    await viewer.open("04-hierarchy", { platform });
    const builds = await viewer.builds();
    const offered = platform === "desktop" ? panes : panes.slice(0, 6);
    for (const [i, pane] of offered.entries()) {
      await page.keyboard.press("Control+B");
      await expect(page.locator("#sidebar")).toBeHidden();
      await page.keyboard.press(String(i + 1));
      await expect(page.locator(`#pane-${pane}`)).toBeVisible();
      await expect(page.locator(`#tab-${pane}`)).toBeFocused();
      await expect(page.locator(`#tab-${pane}`)).toHaveAttribute("aria-selected", "true");
      await expect(page.locator(`#tab-${pane} .tab-key`)).toHaveText(String(i + 1));
      expect(await viewer.stored("sidebar-shown")).toBe(true);
      expect(await page.evaluate(() => window.viewer.checked("toggle-netlist"))).toBe(true);
    }
    if (platform === "desktop") {
      await expect(page.locator("#solve-report")).toContainText("Run Solve (F9)");
    } else {
      await page.keyboard.press("7");
      await expect(page.locator("#tab-solve")).toBeHidden();
      await expect(page.locator("#pane-unplaced")).toBeVisible();
    }
    expect(await viewer.calls("solve")).toEqual([]);
    expect(await viewer.builds()).toBe(builds);
  });
}

test("opening Not drawn from a collapsed sidebar fills its first page immediately", async ({ page, viewer }) => {
  await viewer.open("unplaced", { platform: "desktop" });
  await page.keyboard.press("Control+B");
  await page.keyboard.press("6");
  await expect(page.locator("#unplaced-list .unplaced-row")).toHaveCount(20);
  await expect(page.locator("#unplaced-list .unplaced-row").first()).toBeVisible();
});

test("digits preserve text input, modifiers, dialogs and IME composition", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy", { platform: "desktop" });
  await page.locator("#show-failed").focus();
  await page.keyboard.press("3");
  await expect(page.locator("#show-failed")).toBeChecked();
  const search = page.locator("#search");
  await search.focus();
  await page.keyboard.type("1234567");
  await expect(search).toHaveValue("1234567");
  await expect(page.locator("#pane-hierarchy")).toBeVisible();
  // Hiding the sidebar while its input is focused must not leave focus in a hidden input.
  await page.keyboard.press("Control+B");
  await page.keyboard.press("2");
  await expect(page.locator("#pane-netlist")).toBeVisible();
  await page.keyboard.press("Control+q");
  const editor = page.locator("#source textarea:visible");
  await expect(editor).toBeFocused();
  await page.keyboard.press("Control+a");
  await page.keyboard.type("1234567");
  await expect(editor).toHaveValue("1234567");
  await expect(page.locator("#pane-netlist")).toBeVisible();
  await page.locator("#tab-netlist").focus();
  for (const modifiers of [{ ctrlKey: true }, { metaKey: true }, { altKey: true }, { shiftKey: true }, { isComposing: true }]) {
    for (const key of ["1", "3", "4", "5", "6", "7"]) {
      await page.locator("#tab-netlist").dispatchEvent("keydown", { key, ...modifiers });
      await expect(page.locator("#pane-netlist")).toBeVisible();
    }
  }
  await page.evaluate(() => window.viewer.command("shortcuts"));
  const dialog = page.locator("dialog.dialog");
  await expect(dialog).toBeVisible();
  await page.keyboard.type("134567");
  await expect(page.locator("#pane-netlist")).toBeVisible();
  await dialog.getByRole("button", { name: "Close" }).click();
  await dialogGone(page);
});

test("digits switch from lists and the symbol viewer, and native menu commands open panes", async ({ page, viewer }) => {
  await viewer.open("symbol-inspector", { platform: "desktop" });
  await viewer.click("Xparent");
  await page.keyboard.press("Enter");
  await page.keyboard.press("3");
  await page.locator("#tree .twisty").first().focus();
  await expect(page.locator("#tree .twisty").first()).toBeFocused();
  await page.keyboard.press("4");
  await expect(page.locator("#pane-symbol")).toBeVisible();
  await page.locator(".symbol-preview").focus();
  await page.keyboard.press("6");
  await expect(page.locator("#pane-unplaced")).toBeVisible();
  await page.keyboard.press("Control+B");
  await page.evaluate(() => window.fakeShell.menu("panel-solve"));
  await expect(page.locator("#pane-solve")).toBeVisible();
  expect(await viewer.calls("solve")).toEqual([]);
});

test("panel shortcuts work with no open netlist and the badges fit a narrow sidebar", async ({ page, viewer }) => {
  await viewer.open(null, { platform: "desktop" });
  await page.evaluate(async () => (await import("./panels.js")).setSidebarWidth(220));
  for (const [i, pane] of panes.entries()) {
    await page.keyboard.press(String(i + 1));
    await expect(page.locator(`#pane-${pane}`)).toBeVisible();
    const tab = page.locator(`#tab-${pane}`);
    const [box, key, side] = await Promise.all([tab.boundingBox(), tab.locator(".tab-key").boundingBox(), page.locator("#sidebar").boundingBox()]);
    expect(box.x + box.width).toBeLessThanOrEqual(side.x + side.width);
    expect(key.x).toBeGreaterThanOrEqual(box.x);
    expect(key.x + key.width).toBeLessThan(box.x + 14);
    expect(key.y).toBeGreaterThanOrEqual(box.y);
    expect(key.y + key.height).toBeLessThan(box.y + box.height);
  }
});
