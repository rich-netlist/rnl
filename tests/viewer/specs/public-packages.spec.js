// The public packages that a desktop build bundles: the Library has them as a group of their own, read only,
// when the build has them; Help > Public Packages says what they are, which this build bundles and where
// others come from, and that the viewer downloads none. The web viewer has neither: it downloads on request.

import { expect, test } from "../viewer.js";
import { repository, translated, uiLanguages, zh } from "../zh.js";

const group = (page, key) => page.locator(`.lib-group[data-group=${JSON.stringify(key)}]`);
const dialog = (page) => page.locator("dialog.dialog");
const help = (page) => page.evaluate(() => window.fakeShell.menu("public-packages"));
const BUNDLED = ["analog-canvas-extensions 0.4", "eda-style 0.1", "eda-symbol 0.1", "pdk-sky130 0.1"];
const strings = [
  "Public packages (bundled)", "Public &Packages...", "Public Packages", "This build does not bundle the public packages.",
  "The desktop viewer never downloads packages itself.", "Look for it in Help › Public Packages.", "release page", "rnl-packages site",
  "The bundled packages are under CC0-1.0, as rnl-packages declares; their licences are in the LICENSES folder beside them.",
];

test("a build that bundles the public packages lists them in a read-only group of the Library, after the PATH", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy", { platform: "desktop", pool: "1", bundled: "1" });
  await page.locator("#tab-files").click();
  const bundled = group(page, "public");
  await expect(bundled.locator(".lib-title")).toHaveText("Public packages (bundled)");
  await expect(bundled.locator(".lib-count")).toHaveText("4");
  await bundled.locator(".lib-toggle").click();
  await expect(bundled.locator(".pool-open")).toHaveText(BUNDLED);
  await expect(bundled.locator(".pool-badge")).toHaveText(Array(4).fill("read only"));
  // read only: nothing to exclude, and no format to choose
  await expect(bundled.getByRole("button", { name: /Exclude/ })).toHaveCount(0);
  await expect(bundled.locator(".format-chip")).toHaveCount(0);
  await expect(bundled.locator(".pool-open").first()).toHaveAttribute("title", /eda-style|analog-canvas-extensions/);
  // after the user's own groups, before the built-in ones
  const order = await page.locator("#library-groups > .lib-group").evaluateAll((els) => els.map((e) => e.dataset.group));
  expect(order.at(-2)).toBe("public");
  expect(order.at(-1)).toBe("builtin");
  expect(order[0]).toBe("files");
});

test("a build without the public packages has no such group", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy", { platform: "desktop", pool: "1" });
  await page.locator("#tab-files").click();
  await expect(group(page, "public")).toHaveCount(0);
  await expect(group(page, "builtin")).toHaveCount(1);
});

test("Help > Public Packages names the bundled packages, where they come from and that the viewer downloads none", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy", { platform: "desktop", bundled: "1" });
  await help(page);
  await expect(dialog(page)).toBeVisible();
  await expect(dialog(page).locator("h2, .dialog-title").first()).toHaveText("Public Packages");
  const text = await dialog(page).innerText();
  expect(text).toContain(`This build bundles ${BUNDLED.join(", ")}.`);
  expect(text).toContain("found without a network");
  expect(text).toContain("The bundled packages are under CC0-1.0, as rnl-packages declares");
  expect(text).toContain("A download without them is on the release page.");
  expect(text).toContain("The packages come from the rnl-packages site.");
  expect(text).toContain("add the folder to the PATH in the Library");
  expect(text).toContain("The desktop viewer never downloads packages itself.");
  // the two places are links the default browser opens
  await dialog(page).getByRole("link", { name: "release page" }).click();
  await dialog(page).getByRole("link", { name: "rnl-packages site" }).click();
  const browsed = await page.evaluate(() => window.fakeShell.calls.filter((c) => c[0] === "browse").map((c) => c[1]));
  expect(browsed).toEqual(["https://github.com/rich-netlist/rnl/releases", "https://rich-netlist.github.io/rnl-packages/"]);
  expect(await page.evaluate(() => window.fakeShell.calls.some((c) => /fetch|download|include-?public/i.test(String(c[0]))))).toBe(false);
});

test("Help > Public Packages in the build without them says so and points to the other download", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy", { platform: "desktop" });
  await help(page);
  const text = await dialog(page).innerText();
  expect(text).toContain("This build does not bundle the public packages.");
  expect(text).toContain("A download with the public packages bundled is on the release page, next to this one.");
  expect(text).not.toContain("This build bundles");
  expect(text).not.toContain("CC0-1.0");
});

test("the web viewer has no Help > Public Packages: it offers File > Include Public Package", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy");
  await viewer.menuTitle("Help").click();
  await expect(page.locator('.menu-item[data-id="public-packages"]')).toHaveCount(0);
  await expect(page.locator(".menu-item", { hasText: "Public Packages" })).toHaveCount(0);
});

test("the texts of Help > Public Packages and the Library group are in every language of the interface and follow the language", async ({ page, viewer }) => {
  for (const { lang } of uiLanguages(repository)) {
    if (lang === "en") continue;
    for (const english of strings) expect(translated(lang, english), `${lang}: ${english}`).not.toBe("");
  }
  await viewer.open("04-hierarchy", { platform: "desktop", pool: "1", bundled: "1" });
  await page.evaluate(() => window.viewer.command("language-zh"));
  await expect(page.locator("html")).toHaveAttribute("lang", "zh-CN");
  await page.locator("#tab-files").click();
  await expect(group(page, "public").locator(".lib-title")).toHaveText(zh("Public packages (bundled)"));
  await help(page);
  await expect(dialog(page)).toContainText(zh("The desktop viewer never downloads packages itself."));
  await expect(dialog(page)).toContainText(zh("This build bundles {0}. They are found without a network, as a group of their own in the Library, and searched after the directories of your own PATH.", BUNDLED.join(", ")));
});
