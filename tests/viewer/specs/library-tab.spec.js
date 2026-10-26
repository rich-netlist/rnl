import { showSource } from "../viewer.js";
import fs from "node:fs";
import path from "node:path";
import { dialogGone, test, expect } from "../viewer.js";

const group = (page, key) => page.locator(`.lib-group[data-group=${JSON.stringify(key)}]`);
const files = (page) => group(page, "files");
const builtin = (page) => group(page, "builtin");
const dialog = (page) => page.locator("dialog.dialog");

test("desktop: missing public package keeps its error and offers the website in the system browser", async ({ page, viewer }, info) => {
  await viewer.open("missing-canvas-package", { platform: "desktop", pool: "1" });
  await expect(page.locator(".diag.error .msg")).toContainText(["package analog-canvas-extensions 0.1 cannot be found"]);
  const hint = page.locator(".package-hint");
  await expect(hint).toContainText("Add the package file to Library or a PATH directory");
  await expect(hint.getByRole("link")).toHaveAttribute("href", "https://rich-netlist.github.io/rnl/en/packages.html");
  if (process.env.RNL_PACKAGE_SCREENSHOTS && info.project.name === "chromium") {
    await page.screenshot({ path: path.join(process.env.RNL_PACKAGE_SCREENSHOTS, "desktop-missing-canvas-hint.png") });
  }
  await hint.getByRole("link").click();
  expect(await page.evaluate(() => window.fakeShell.calls)).toContainEqual(["browse", "https://rich-netlist.github.io/rnl/en/packages.html"]);
});

async function paste(page, viewer, text) {
  await page.evaluate((text) => {
    navigator.clipboard.readText = async () => text;
    window.fakeShell.next = "04-hierarchy";
    window.viewer.command("load-clipboard");
  }, text);
  await expect(dialog(page).locator(".dialog-title")).toHaveText("Load from Clipboard");
  await viewer.build(() => dialog(page).getByRole("button", { name: "Load", exact: true }).click());
}

for (const platform of ["desktop", "web"]) {
  test(`${platform}: opened files, drops, deduplication, root highlight and session collapse`, async ({ page, viewer }) => {
    await viewer.open("04-hierarchy", { platform, pool: "1" });
    await page.locator("#tab-files").click();
    await expect(page.getByRole("tab", { name: "Library", exact: true })).toHaveAttribute("aria-selected", "true");
    await expect(files(page).locator(".lib-toggle")).toHaveAttribute("aria-expanded", "true");
    await expect(builtin(page).locator(".lib-toggle")).toHaveAttribute("aria-expanded", "false");
    await expect(files(page).locator(".current")).toHaveAttribute("aria-current", "true");
    await expect(files(page).locator(".pool-row")).toHaveCount(1);
    if (platform === "web") {
      await expect(page.locator("#include-library")).toHaveText("Include Public Package...");
      await viewer.menuTitle("File").click();
      await expect(viewer.menuItem("include-library")).toHaveText("Include Public Package...");
      await page.keyboard.press("Escape");
      await expect(page.locator("#add-path")).toBeHidden();
      await expect(page.locator(".lib-path")).toHaveCount(0);
    }
    // Include tabs change the text shown, never the Library's root highlight.
    const root = await files(page).locator(".current").getAttribute("data-path");
    await page.locator("#tab-netlist").click();
    await showSource(page, 1);
    await page.locator("#tab-files").click();
    await expect(files(page).locator(".current")).toHaveAttribute("data-path", root);
    await files(page).locator(".lib-toggle").click();
    await viewer.build(() => page.evaluate(() => window.fakeShell.open({ name: "drop.sp", path: "/drop.sp", fixture: "01-rc-lowpass" })));
    await expect(files(page).locator(".lib-toggle")).toHaveAttribute("aria-expanded", "true");
    await expect(files(page).locator(".pool-row")).toHaveCount(2);
    await expect(files(page).locator(".current")).toHaveAttribute("data-path", "/drop.sp");
    // Opening an existing file changes the highlight without expanding its collapsed group.
    await files(page).locator(".lib-toggle").click();
    await viewer.build(() => page.evaluate(() => {
      window.fakeShell.picks = [{ name: "drop.sp", path: "/drop.sp", fixture: "01-rc-lowpass" }];
      window.viewer.command("open");
    }));
    await expect(files(page).locator(".lib-toggle")).toHaveAttribute("aria-expanded", "false");
    await expect(files(page).locator(".pool-row")).toHaveCount(2);
    await files(page).locator(".lib-toggle").click();
    await viewer.build(() => files(page).getByRole("button", { name: "Exclude drop.sp", exact: true }).click());
    await expect(page.locator("#level-name")).toHaveText("01-rc-lowpass.sp");
    await expect(files(page).locator(".current")).toHaveCount(0);
    await builtin(page).locator(".lib-toggle").click();
    await expect(builtin(page).locator(".pool-badge")).toHaveText("read only");
    await expect(builtin(page).getByRole("button", { name: /Exclude/ })).toHaveCount(0);
    await viewer.build(() => builtin(page).locator(".pool-open").click());
    await page.evaluate(() => window.viewer.command("toggle-edit"));
    expect(await page.evaluate(() => window.viewer.checked("toggle-edit"))).toBe(false);
    await page.locator("#tab-files").click();
    await expect(builtin(page).locator(".current")).toHaveAttribute("aria-current", "true");
    await expect(files(page).locator(".pool-row")).toHaveCount(1);
    await page.reload();
    await viewer.started();
    await viewer.idle();
    await expect(builtin(page).locator(".lib-toggle")).toHaveAttribute("aria-expanded", "false");
    await expect(files(page).locator(".lib-toggle")).toHaveAttribute("aria-expanded", "true");
  });

  test(`${platform}: one temporary row retains only applied text and follows leave confirmation`, async ({ page, viewer }) => {
    await viewer.open("04-hierarchy", { platform, pool: "1" });
    const original = "* pasted\nR1 a 0 1k\n.end\n";
    const applied = original.replace("1k", "2k");
    await paste(page, viewer, original);
    await page.locator("#tab-files").click();
    const temp = page.locator(".pool-temporary");
    await expect(temp).toHaveCount(1);
    await expect(temp).toHaveClass(/current/);
    await expect(temp).toHaveAttribute("aria-current", "true");
    await expect(temp.locator(".pool-badge")).toHaveText("temporary · not searched");
    await expect(temp.getByRole("button", { name: /Exclude/ })).toHaveCount(0);
    await expect(files(page).locator(".pool-row")).toHaveCount(1);
    expect(await page.evaluate(() => window.fakeShell.pool.some((p) => p.includes("clipboard")))).toBe(false);
    await page.locator("#tab-netlist").click();
    await page.evaluate(() => window.viewer.command("toggle-edit"));
    const area = page.locator("#source textarea:not([hidden])").first();
    await area.fill(applied);
    await viewer.build(() => page.locator("#apply-edits").click());
    await page.evaluate(() => window.viewer.command("toggle-edit"));
    await area.fill(applied.replace("2k", "3k"));
    await page.locator("#tab-files").click();
    await files(page).locator(".pool-open").click();
    await expect(dialog(page).locator(".dialog-title")).toHaveText("Changes Not Saved");
    await dialog(page).getByRole("button", { name: "Cancel", exact: true }).click();
    await dialogGone(page);
    await expect(temp).toHaveClass(/current/);
    await files(page).locator(".pool-open").click();
    await viewer.build(() => dialog(page).getByRole("button", { name: platform === "desktop" ? "Don't Save" : "Discard Changes", exact: true }).click());
    await page.locator("#tab-files").click();
    await viewer.build(() => temp.locator(".pool-open").click());
    expect((await viewer.calls("build")).at(-1)).toEqual(["clipboard.sp", applied]);
    await expect(temp).toHaveClass(/current/);
    await paste(page, viewer, "* replacement\nR2 b 0 4k\n.end\n");
    await page.locator("#tab-files").click();
    await expect(temp).toHaveCount(1);
    expect((await viewer.calls("build")).at(-1)[1]).toContain("R2 b 0 4k");
    await page.reload();
    await viewer.started();
    await expect(temp).toHaveCount(0);
  });
}

test("desktop: PATH groups scan on entry and changes, preserve collapse and highlight matching roots", async ({ page, viewer }) => {
  await page.emulateMedia({ colorScheme: "dark" });
  await viewer.open(null, { platform: "desktop", pool: "1" });
  await page.evaluate(() => {
    localStorage.setItem("search-paths", JSON.stringify(["/pdk", "/empty", "/missing", "/denied"]));
    window.fakeShell.packages["/pdk"] = ["/pdk/direct.rnl", "/pdk/0.3/versioned.rnl"];
    window.fakeShell.pathErrors = { "/missing": "Directory not found.", "/denied": "Permission denied (os error 13)" };
  });
  await page.locator("#tab-files").click();
  await expect(page.locator(".lib-ordinal")).toHaveText(["PATH 1", "PATH 2", "PATH 3", "PATH 4"]);
  const pdk = group(page, "path:/pdk");
  await expect(pdk.locator(".lib-path")).toHaveAttribute("title", "/pdk");
  await expect(pdk.locator(".lib-count")).toHaveText("2");
  await expect(pdk.locator(".lib-toggle")).toHaveAttribute("aria-expanded", "false");
  await expect(pdk.getByRole("button", { name: "Move up in PATH", exact: true })).toBeDisabled();
  await expect(group(page, "path:/denied").getByRole("button", { name: "Move down in PATH", exact: true })).toBeDisabled();
  await pdk.locator(".lib-toggle").click();
  await expect(pdk.locator(".pool-open")).toHaveText(["direct.rnl", "0.3/versioned.rnl"]);
  await expect(pdk.getByRole("button", { name: /Exclude/ })).toHaveCount(0);
  await viewer.build(() => pdk.locator(".pool-open").first().click());
  await page.locator("#tab-files").click();
  await expect(page.locator('.pool-row.current[data-path="/pdk/direct.rnl"]')).toHaveCount(2);
  await expect(page.locator('.pool-row.current[data-path="/pdk/direct.rnl"][aria-current="true"]')).toHaveCount(2);
  await viewer.build(() => pdk.getByRole("button", { name: "Move down in PATH", exact: true }).click());
  await expect(page.locator(".lib-path")).toHaveText(["/empty", "/pdk", "/missing", "/denied"]);
  await expect(pdk.locator(".lib-toggle")).toHaveAttribute("aria-expanded", "true");
  for (const dir of ["/empty", "/missing", "/denied"]) await group(page, `path:${dir}`).locator(".lib-toggle").click();
  await expect(group(page, "path:/empty").locator(".lib-empty")).toHaveText("No packages or netlists found.");
  await expect(group(page, "path:/missing").locator(".lib-warning")).toHaveText("Directory not found.");
  await expect(group(page, "path:/denied").locator(".lib-warning")).toHaveText("Permission denied (os error 13)");
  const warning = await group(page, "path:/missing").locator(".lib-warning").evaluate((el) => {
    const probe = document.createElement("span");
    probe.style.color = "var(--warning)";
    el.append(probe);
    const expected = getComputedStyle(probe).color;
    probe.remove();
    return { actual: getComputedStyle(el).color, expected };
  });
  expect(warning.actual).toBe(warning.expected);
  expect(warning.actual).not.toBe("rgb(104, 115, 134)");
  await page.evaluate(() => { window.fakeShell.packages["/pdk"].push("/pdk/new.rnl"); });
  await page.locator("#tab-netlist").click();
  await page.locator("#tab-files").click();
  await expect(pdk.locator(".lib-count")).toHaveText("3");
  await viewer.build(() => pdk.getByRole("button", { name: "Remove from PATH", exact: true }).click());
  await expect(pdk).toHaveCount(0);
  await expect(files(page).locator(".current")).toHaveAttribute("data-path", "/pdk/direct.rnl");
});

test("desktop: Windows chooser paths match PATH rows and forward-slash opens without duplicates", async ({ page, viewer }) => {
  await viewer.open(null, { platform: "desktop", pool: "1" });
  await page.evaluate(() => {
    localStorage.setItem("search-paths", JSON.stringify(["C:\\libs\\pdk"]));
    window.fakeShell.packages["C:\\libs\\pdk"] = ["C:/libs/pdk/shared.rnl"];
  });
  await viewer.build(() => page.evaluate(() => window.fakeShell.open({
    name: "shared.rnl", path: "C:\\libs\\pdk\\shared.rnl", fixture: "04-hierarchy",
  })));
  await page.locator("#tab-files").click();
  const dir = group(page, "path:C:\\libs\\pdk");
  await dir.locator(".lib-toggle").click();
  await expect(dir.locator(".pool-open")).toHaveText("shared.rnl");
  await expect(page.locator('.pool-row.current[data-path="C:/libs/pdk/shared.rnl"][aria-current="true"]')).toHaveCount(2);
  await files(page).locator(".lib-toggle").click();
  await viewer.build(() => page.evaluate(() => window.fakeShell.open({
    name: "shared.rnl", path: "C:/libs/pdk/shared.rnl", fixture: "04-hierarchy",
  })));
  await expect(files(page).locator(".pool-row")).toHaveCount(1);
  await expect(files(page).locator(".lib-toggle")).toHaveAttribute("aria-expanded", "false");
  await viewer.build(() => dir.locator(".pool-open").click());
  await expect(files(page).locator(".pool-row")).toHaveCount(1);
});

for (const platform of ["desktop", "web"]) for (const theme of ["light", "dark"]) {
  test(`${platform}: ${theme} Library layout and theme colors`, async ({ page, viewer }, testInfo) => {
    await page.emulateMedia({ colorScheme: theme });
    await viewer.open(null, { platform, pool: "1", settings: platform === "desktop" ? "1" : "" });
    await page.evaluate((platform) => {
      window.fakeShell.includes = ["ota5t.sp", "amp.sp"].map((name) => ({
        path: `/work/${name}`, name, source: { name, path: `/work/${name}`, fixture: "04-hierarchy" },
      }));
      if (platform === "desktop") {
        window.fakeShell.packages = { "/libs/pdk": ["/libs/pdk/pdk-sky130.rnl", "/libs/pdk/0.3/rnl-std-basic.rnl"], "/work/rnl": [] };
        window.fakeShell.path = "/libs/pdk";
      }
    }, platform);
    await page.locator("#tab-files").click();
    await page.locator("#include-file").click();
    await expect(files(page).locator(".pool-row")).toHaveCount(2);
    await viewer.build(() => files(page).locator(".pool-open").first().click());
    await page.locator("#tab-files").click();
    if (platform === "desktop") {
      await viewer.build(() => page.locator("#add-path").click());
      await expect(page.locator(".lib-path")).toHaveText(["/libs/pdk"]);
      await page.evaluate(() => { window.fakeShell.path = "/work/rnl"; });
      await viewer.build(() => page.locator("#add-path").click());
      await group(page, "path:/libs/pdk").locator(".lib-toggle").click();
    } else {
      await paste(page, viewer, "* pasted\nR1 a 0 1k\n.end\n");
      await page.locator("#tab-files").click();
      await builtin(page).locator(".lib-toggle").click();
    }
    const colors = await page.evaluate(() => {
      const css = getComputedStyle(document.documentElement);
      const color = (selector, prop = "color") => getComputedStyle(document.querySelector(selector))[prop];
      const resolve = (name) => {
        const probe = document.createElement("span");
        probe.style.color = css.getPropertyValue(name);
        document.body.append(probe);
        const value = getComputedStyle(probe).color;
        probe.remove();
        return value;
      };
      return { header: color(".lib-title"), count: color(".lib-count"), badge: color(".pool-badge"),
        current: color(".pool-row.current", "backgroundColor"), fg: resolve("--fg"), muted: resolve("--muted"), selected: resolve("--select-text"),
        overflow: color(".library-groups", "overflowY") };
    });
    expect(colors.header).toBe(colors.fg);
    expect(colors.count).toBe(colors.muted);
    expect(colors.badge).toBe(colors.muted);
    expect(colors.current).toBe(colors.selected);
    expect(colors.count).not.toBe("rgb(104, 115, 134)");
    expect(colors.overflow).toBe("auto");
    if (process.env.RNL_LIBRARY_REVIEW_DIR && testInfo.project.name === "chromium") {
      fs.mkdirSync(process.env.RNL_LIBRARY_REVIEW_DIR, { recursive: true });
      await page.mouse.move(1190, 790);
      // Finish the existing chevron transition before capturing its expanded state.
      await expect(page.locator('.lib-toggle[aria-expanded="true"] .chevron').last()).toHaveCSS("transform", "none");
      await page.screenshot({ path: path.join(process.env.RNL_LIBRARY_REVIEW_DIR, `${platform}-${theme}.png`) });
    }
  });
}
