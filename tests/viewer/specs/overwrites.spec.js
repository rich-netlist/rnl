// A shell of the web platform that writes the files where they were read from (`overwrites`, which the local shell has,
// as its server writes them): Save is Save, not Download File, with the desktop's Save As; Ctrl+S writes; the question
// before leaving files with changes offers Save. A web shell without it is as it was.

import { expect, test } from "../viewer.js";

const dialog = (page) => page.locator("dialog.dialog");

/** The labels of the File menu's items, by command id. */
async function fileMenu(page, viewer) {
  await viewer.menuTitle("File").click();
  const labels = await page.locator("#menubar .menu-item").evaluateAll((items) => Object.fromEntries(items.map((i) => [i.dataset.id, i.querySelector(".label")?.textContent])));
  await page.keyboard.press("Escape");
  return labels;
}

async function edit(page, viewer) {
  await page.evaluate(() => window.viewer.command("toggle-edit"));
  const area = page.locator("#source .editor:not([hidden]) textarea");
  await area.fill(`${await area.inputValue()}\n* edited\n`);
  await viewer.build(() => page.locator("#apply-edits").click());
}

test("a web shell that downloads has Download File and no Save As, as it had", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy");
  const labels = await fileMenu(page, viewer);
  expect(labels.save).toBe("Download File");
  expect(labels["save-as"]).toBeUndefined();
});

test("a web shell that overwrites has Save, Save As and the UTF-8 forms, and no Download File", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy", { overwrites: "1" });
  const labels = await fileMenu(page, viewer);
  expect(labels.save).toBe("Save");
  expect(labels["save-as"]).toBe("Save As...");
  expect(labels["save-utf8"]).toBe("Save Changes Using UTF-8");
  expect(labels["save-as-utf8"]).toBe("Save As UTF-8...");
  expect(Object.values(labels)).not.toContain("Download File");
});

test("Save writes the files with changes, where they were read from, and marks them saved", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy", { overwrites: "1" });
  await edit(page, viewer);
  expect((await page.evaluate(() => window.viewer.edits)).unsaved).toHaveLength(1);
  await page.keyboard.press("Control+s");
  await expect.poll(async () => (await viewer.calls("save")).length).toBe(1);
  const [[written]] = await page.evaluate(() => window.fakeShell.writes);
  expect(written.text).toContain("* edited");
  await expect(page.locator("#note")).toContainText("Saved");
  expect(await page.evaluate(() => window.viewer.edits)).toEqual({ unsaved: [], unapplied: [] });
});

test("a file changed on disk is not overwritten before the user says so, as on the desktop", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy", { overwrites: "1" });
  await edit(page, viewer);
  await page.evaluate(() => (window.fakeShell.changedOnDisk = ["04-hierarchy.sp"]));
  await page.keyboard.press("Control+s");
  await expect(dialog(page).locator(".dialog-title")).toHaveText("File Changed on Disk");
  await dialog(page).getByRole("button", { name: "Cancel" }).click();
  expect((await page.evaluate(() => window.viewer.edits)).unsaved).toHaveLength(1);
});

test("leaving files with changes offers Save, where a downloading shell offers only to discard them", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy", { overwrites: "1" });
  await edit(page, viewer);
  await page.evaluate(() => (window.fakeShell.picks = ["04-hierarchy"]));
  await page.evaluate(() => window.viewer.command("open"));
  await expect(dialog(page)).toContainText("Do you want to open another file?");
  await expect(dialog(page).getByRole("button", { name: "Save", exact: true })).toBeVisible();
  await dialog(page).getByRole("button", { name: "Cancel" }).click();
  // the shell that downloads
  await viewer.open("04-hierarchy");
  await edit(page, viewer);
  await page.evaluate(() => (window.fakeShell.picks = ["04-hierarchy"]));
  await page.evaluate(() => window.viewer.command("open"));
  await expect(dialog(page).getByRole("button", { name: "Save", exact: true })).toHaveCount(0);
  await expect(dialog(page).getByRole("button", { name: "Discard Changes" })).toBeVisible();
  await dialog(page).getByRole("button", { name: "Cancel" }).click();
});
