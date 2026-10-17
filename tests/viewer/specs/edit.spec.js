// Editing the text (apps/viewer-ui/editor.js, edits.js; decision 0011): the netlist pane switched to
// edit, a text box for each file; a change shown as not applied until Apply Changes builds the
// Document again from the texts in memory (the fake shell answers with a fixture, as the core would
// answer with the Document of the edited text); nothing written until Save, which the fake shell
// records instead of writing; changes not saved asked about before a document, or the window, is left;
// and a solve of the text applied. The fixture "edit" has one file, "edit-changed" is it after C1 was
// renamed C2; "04-hierarchy" has an include.

import { expect, test } from "../viewer.js";

const DESKTOP = { platform: "desktop" };

/** Whether a check-mark command is checked, as the page says. */
const checked = (page, id) => page.evaluate((id) => window.viewer.checked(id), id);

/** The dialog shown. */
const dialog = (page) => page.locator("dialog.dialog");

/** The text box of the file shown, while the pane is in edit. */
const area = (page) => page.locator("#source .editor:not([hidden]) textarea");

/** The state the bar above the text shows. */
const state = (page) => page.locator("#edit-state");

/** The tab of a file, by its name. */
const tab = (page, name) => page.locator("#file-tabs button", { hasText: name });

/** The text of the file shown: what the page's `edit` would give. */
const text = (page) => area(page).inputValue();

/** Switch the pane to edit, with Ctrl+E, and wait for the text box. */
async function begin(page) {
  await page.keyboard.press("Control+e");
  await expect(page.locator("#source .editor:not([hidden]) textarea")).toBeVisible();
}

/** Put `edited` in the text box of the file shown, as typing it would. */
const type = (page, edited) => area(page).fill(edited);

/** The Document's text of the file with this name, from the page. */
const original = (page, name) =>
  page.evaluate((n) => window.viewer.doc.files.find((f) => f.name === n).text, name);

/** Open the fixture on the desktop and switch to edit. */
async function open(viewer, page, fixture = "edit") {
  await viewer.open(fixture, DESKTOP);
  await begin(page);
}

/** Rename C1 as C2 in the text of the file shown, as the fixture "edit-changed" has it. */
async function rename(page) {
  const was = await text(page);
  await type(page, was.replace(/C1/g, "C2"));
}

/** Press a button of the dialog shown, and wait for it to go. */
async function answer(page, button) {
  await dialog(page).getByRole("button", { name: button }).click();
  await expect(dialog(page)).toHaveCount(0);
}

test.describe("on the desktop", () => {
  test("Edit Text switches the pane to a text box for each file, and back", async ({ page, viewer }) => {
    await viewer.open("04-hierarchy", DESKTOP);
    await expect(page.locator("#source .editor")).toHaveCount(0);
    await begin(page);
    expect(await checked(page, "toggle-edit")).toBe(true);
    // a text box for each of the two files, the tabs as they were, and the first shown
    await expect(page.locator("#source .editor")).toHaveCount(2);
    await expect(page.locator("#file-tabs button")).toHaveText(["04-hierarchy.sp", "04-hierarchy-lib.inc"]);
    expect(await text(page)).toBe((await original(page, "04-hierarchy.sp")).replace(/\r\n/g, "\n"));
    await expect(page.locator("#source .file:not([hidden])")).toHaveCount(0);
    // line numbers beside it, one for each line
    const lines = (await text(page)).split("\n").length;
    await expect(page.locator("#source .editor:not([hidden]) .gutter")).toHaveText(
      Array.from({ length: lines }, (_, k) => k + 1).join("\n"),
    );
    await expect(state(page)).toHaveText("No changes");
    await expect(page.locator("#apply-edits")).toBeDisabled();
    // the other file's tab shows the other file's text box
    await tab(page, "lib.inc").click();
    expect(await text(page)).toBe((await original(page, "04-hierarchy-lib.inc")).replace(/\r\n/g, "\n"));
    // Ctrl+E again: the highlighted text, as before
    await page.keyboard.press("Control+e");
    await expect(page.locator("#source .editor")).toHaveCount(0);
    await expect(page.locator("#source .file:not([hidden])")).toBeVisible();
    expect(await checked(page, "toggle-edit")).toBe(false);
    // the menu command does the same as the key
    await page.evaluate(() => window.viewer.command("toggle-edit"));
    await expect(area(page)).toBeVisible();
  });

  test("a change is shown as not applied, and nothing is built until it is applied", async ({ page, viewer }) => {
    await open(viewer, page);
    const builds = await viewer.builds();
    await rename(page);
    await expect(state(page)).toHaveText("Changes not applied yet");
    await expect(page.locator("#apply-edits")).toBeEnabled();
    await expect(tab(page, "edit.sp")).toHaveAttribute("data-unsaved", "true");
    await expect(tab(page, "edit.sp")).toHaveAttribute("data-unapplied", "true");
    expect(await page.evaluate(() => window.viewer.edits)).toEqual({ unsaved: ["edit.sp"], unapplied: ["edit.sp"] });
    // the drawing and the problems are those of the text applied
    expect(await viewer.builds()).toBe(builds);
    expect(await page.evaluate(() => window.viewer.doc.objects.some((o) => o.name === "C1"))).toBe(true);
    // the platform is told that changes are not saved
    expect(await viewer.calls("setDirty")).toEqual([[true]]);
    // and nothing applies itself, however long the text is left
    await page.keyboard.type(" ");
    expect(await viewer.builds()).toBe(builds);
  });

  test("Apply Changes builds the Document from the text held, and keeps the level and the mode", async ({ page, viewer }) => {
    await open(viewer, page);
    await rename(page);
    const edited = await text(page);
    await page.evaluate(() => (window.fakeShell.next = "edit-changed"));
    await viewer.build(() => page.locator("#apply-edits").click());
    // the shell was given the text, by the path the Document names the file by, with the original's
    // line ends
    const edits = await page.evaluate(() => window.fakeShell.buildEdits);
    expect(edits.at(-1)).toEqual({ "edit.sp": edited });
    // the Document is the new one, drawn and checked; the bar says so, and what is applied is not saved
    expect(await page.evaluate(() => window.viewer.doc.objects.some((o) => o.name === "C2"))).toBe(true);
    await expect(state(page)).toHaveText("Applied, not saved");
    await expect(page.locator("#apply-edits")).toBeDisabled();
    await expect(tab(page, "edit.sp")).toHaveAttribute("data-unapplied", "false");
    await expect(tab(page, "edit.sp")).toHaveAttribute("data-unsaved", "true");
    // still editing, with the text as it was typed
    await expect(area(page)).toBeVisible();
    expect(await text(page)).toBe(edited);
    // the highlighted text, shown again, is the applied one
    await page.keyboard.press("Control+e");
    await expect(page.locator("#source .file:not([hidden]) .ln").nth(2)).toContainText("C2 b 0 1p");
  });

  test("Ctrl+Enter applies too, from the text box, and a build that fails keeps the changes", async ({ page, viewer }) => {
    await open(viewer, page);
    await rename(page);
    await page.evaluate(() => (window.fakeShell.fail = "cannot read edit.sp"));
    await page.keyboard.press("Control+Enter");
    await expect(dialog(page)).toContainText("The changes could not be applied: cannot read edit.sp");
    await answer(page, "Close");
    await expect(state(page)).toHaveText("Changes not applied yet");
    expect(await text(page)).toContain("C2");
    // the next try goes through
    await page.evaluate(() => (window.fakeShell.next = "edit-changed"));
    await viewer.build(() => page.keyboard.press("Control+Enter"));
    await expect(state(page)).toHaveText("Applied, not saved");
  });

  test("each file of an include has its own text, its own marks, and its own changes", async ({ page, viewer }) => {
    await open(viewer, page, "04-hierarchy");
    await tab(page, "lib.inc").click();
    const lib = await text(page);
    await type(page, `* a comment\n${lib}`);
    await expect(tab(page, "lib.inc")).toHaveAttribute("data-unsaved", "true");
    await expect(tab(page, "04-hierarchy.sp")).toHaveAttribute("data-unsaved", "false");
    await expect(page.locator("#file-tabs button[data-unsaved='true']")).toHaveCount(1);
    // the first file is as it was
    await tab(page, "04-hierarchy.sp").click();
    expect(await text(page)).toBe((await original(page, "04-hierarchy.sp")).replace(/\r\n/g, "\n"));
    // the include's edit is built by its path, and only it
    await viewer.build(() => page.locator("#apply-edits").click());
    const edits = await page.evaluate(() => window.fakeShell.buildEdits.at(-1));
    expect(Object.keys(edits)).toEqual(["04-hierarchy-lib.inc"]);
    expect(edits["04-hierarchy-lib.inc"].startsWith("* a comment")).toBe(true);
  });

  test("leaving edit with changes not applied asks to apply them, to discard them, or to stay", async ({ page, viewer }) => {
    await open(viewer, page);
    const was = await text(page);
    await rename(page);
    // Cancel stays in edit with the changes
    await page.keyboard.press("Control+e");
    await expect(dialog(page)).toContainText("Some changes are not applied yet");
    await answer(page, "Cancel");
    expect(await text(page)).toContain("C2");
    // Discard Changes puts the text back as the Document has it, and leaves
    await page.keyboard.press("Control+e");
    await answer(page, "Discard Changes");
    await expect(page.locator("#source .editor")).toHaveCount(0);
    await expect(page.locator("#source .file:not([hidden])")).toBeVisible();
    expect(await page.evaluate(() => window.viewer.edits)).toEqual({ unsaved: [], unapplied: [] });
    await begin(page);
    expect(await text(page)).toBe(was);
    // Apply Changes applies and leaves
    await rename(page);
    await page.evaluate(() => (window.fakeShell.next = "edit-changed"));
    await page.keyboard.press("Control+e");
    await viewer.build(() => dialog(page).getByRole("button", { name: "Apply Changes" }).click());
    await expect(page.locator("#source .editor")).toHaveCount(0);
    expect(await page.evaluate(() => window.viewer.edits)).toEqual({ unsaved: ["edit.sp"], unapplied: [] });
  });

  test("a preview is not edited, and a solve is of the text applied", async ({ page, viewer }) => {
    await open(viewer, page, "T4-miller-conflicts");
    // changes are not applied: the solve has none of them
    await type(page, `${await text(page)}\n* a note`);
    await page.evaluate(() => window.viewer.command("solve"));
    await page.waitForFunction(() => document.body.classList.contains("preview") && !document.body.classList.contains("solving"));
    expect(await page.evaluate(() => window.fakeShell.solveEdits.at(-1))).toEqual({});
    // the pane is not in edit while the preview is shown, and says that the change was left out
    await expect(page.locator("#source .editor")).toHaveCount(0);
    await expect(page.locator("#solve-step")).toContainText("Changes not applied yet are not in it.");
    expect(await checked(page, "toggle-edit")).toBe(false);
    await page.evaluate(() => window.viewer.command("toggle-edit"));
    await expect(dialog(page)).toContainText("A solve is shown as a preview");
    await answer(page, "Close");
    // closing the preview shows the netlist again, and the changes are still there to apply
    await viewer.build(() => page.locator("#preview-close").click());
    expect(await page.evaluate(() => window.fakeShell.buildEdits.at(-1))).toEqual({});
    await begin(page);
    expect((await text(page)).endsWith("\n* a note")).toBe(true);
    await expect(state(page)).toHaveText("Changes not applied yet");
  });

  test("a solve is of the text applied, and closing the preview shows that text, not the file", async ({ page, viewer }) => {
    await open(viewer, page);
    await rename(page);
    await page.evaluate(() => (window.fakeShell.next = "edit-changed"));
    await viewer.build(() => page.locator("#apply-edits").click());
    const applied = (await page.evaluate(() => window.fakeShell.buildEdits.at(-1)))["edit.sp"];
    await page.evaluate(() => (window.fakeShell.next = null));
    // the fake shell has no preview of this fixture: it says so, and the solve is asked for all the same
    await page.evaluate(() => window.viewer.command("solve"));
    await expect(dialog(page)).toContainText("there is no such preview");
    expect(await page.evaluate(() => window.fakeShell.solveEdits.at(-1))).toEqual({ "edit.sp": applied });
    await answer(page, "Close");
  });

  test("a click on a problem shows its line in the text box", async ({ page, viewer }) => {
    await viewer.open("12-diagnostics", DESKTOP);
    await begin(page);
    const located = page.locator("#diags .diag.located").first();
    const line = await located.evaluate((el) => Number(el.querySelector(".loc").textContent.split(":")[1]));
    await located.click();
    // the line is selected in the text box
    const selected = await area(page).evaluate((el) => el.value.slice(el.selectionStart, el.selectionEnd));
    expect(selected).toBe((await text(page)).split("\n")[line - 1]);
  });
});

test.describe("saving", () => {
  test("Save writes the files with changes, those only, and the marks go", async ({ page, viewer }) => {
    await open(viewer, page, "04-hierarchy");
    await tab(page, "lib.inc").click();
    await type(page, `* a comment\n${await text(page)}`);
    await tab(page, "04-hierarchy.sp").click();
    await type(page, `${await text(page)}\n* another`);
    await page.keyboard.press("Control+s");
    await expect(page.locator("#note")).toHaveText("Saved 04-hierarchy.sp, 04-hierarchy-lib.inc.");
    const written = await page.evaluate(() => window.fakeShell.writes);
    expect(written).toHaveLength(1);
    expect(written[0].map((f) => f.path).sort()).toEqual(["04-hierarchy-lib.inc", "04-hierarchy.sp"]);
    expect(written[0].find((f) => f.path === "04-hierarchy-lib.inc").text.startsWith("* a comment")).toBe(true);
    await expect(page.locator("#file-tabs button[data-unsaved='true']")).toHaveCount(0);
    expect(await page.evaluate(() => window.viewer.edits.unsaved)).toEqual([]);
    expect(await viewer.calls("setDirty")).toEqual([[true], [false]]);
    // changes not applied are saved too: Save writes what the text boxes hold; and a second Save
    // has nothing to write
    await page.keyboard.press("Control+s");
    await expect(page.locator("#note")).toHaveText("No changes to save.");
    expect(await page.evaluate(() => window.fakeShell.writes)).toHaveLength(1);
    // the file saved is what was read from now on: changing it back is a change
    await tab(page, "lib.inc").click();
    await type(page, (await text(page)).replace(/^\* a comment\n/, ""));
    await expect(tab(page, "lib.inc")).toHaveAttribute("data-unsaved", "true");
  });

  test("Save outside edit mode writes the applied changes too, and a write that fails says why", async ({ page, viewer }) => {
    await open(viewer, page);
    await rename(page);
    await page.evaluate(() => (window.fakeShell.next = "edit-changed"));
    await viewer.build(() => page.locator("#apply-edits").click());
    await page.keyboard.press("Control+e");
    await page.evaluate(() => (window.fakeShell.writeFail = "access is denied"));
    await page.evaluate(() => window.viewer.command("save"));
    await expect(dialog(page)).toContainText("The changes were not saved: access is denied");
    await answer(page, "Close");
    expect(await page.evaluate(() => window.viewer.edits.unsaved)).toEqual(["edit.sp"]);
    await page.evaluate(() => window.viewer.command("save"));
    await expect(page.locator("#note")).toHaveText("Saved edit.sp.");
    expect(await page.evaluate(() => window.viewer.edits.unsaved)).toEqual([]);
  });

  test("a file is written with the line ends it had", async ({ page, viewer }) => {
    await open(viewer, page, "edit-crlf");
    // the text box has `\n`, as a text box does; the file has `\r\n`
    expect(await text(page)).not.toContain("\r");
    await rename(page);
    await page.evaluate(() => window.viewer.command("save"));
    await expect(page.locator("#note")).toHaveText("Saved edit-crlf.sp.");
    const [file] = (await page.evaluate(() => window.fakeShell.writes))[0];
    expect(file.text).toContain("C2 b 0 1p\r\n");
    expect(file.text.replace(/\r\n/g, "")).not.toContain("\n");
    expect(file.text.split("\r\n")).toHaveLength(6);
  });

  test("Save As writes the netlist that was opened, and opens the file written", async ({ page, viewer }) => {
    await open(viewer, page);
    await rename(page);
    await page.evaluate(() => (window.fakeShell.savedAs = { name: "/copies/other.sp", fixture: "edit-changed" }));
    const edited = await text(page);
    await viewer.build(() => page.evaluate(() => window.viewer.command("save-as")));
    const [file] = await page.evaluate(() => window.fakeShell.writesAs);
    expect(file).toEqual({ path: "edit.sp", name: "edit.sp", text: edited });
    await expect(page.locator("#note")).toHaveText("Saved as /copies/other.sp.");
    // the file written is the document now, and has nothing unsaved
    expect(await page.evaluate(() => window.viewer.edits)).toEqual({ unsaved: [], unapplied: [] });
    expect((await viewer.calls("build")).at(-1)).toEqual(["/copies/other.sp"]);
    // a place not chosen changes nothing: the change stays unsaved
    await type(page, `${await text(page)}* more\n`);
    await page.evaluate(() => window.viewer.command("save-as"));
    await expect.poll(async () => (await viewer.calls("saveAs")).length).toBe(2);
    expect(await page.evaluate(() => window.viewer.edits.unsaved)).toEqual(["edit.sp"]);
  });

  test("Save As is of the netlist opened: not of an include, nor while another file has changes", async ({ page, viewer }) => {
    await open(viewer, page, "04-hierarchy");
    await tab(page, "lib.inc").click();
    await type(page, `* a comment\n${await text(page)}`);
    await page.evaluate(() => window.viewer.command("save-as"));
    await expect(dialog(page)).toContainText("Save As saves the netlist that was opened, the first file");
    await answer(page, "Close");
    await tab(page, "04-hierarchy.sp").click();
    await page.evaluate(() => window.viewer.command("save-as"));
    await expect(dialog(page)).toContainText("Save the other files with changes first, with File › Save: 04-hierarchy-lib.inc.");
    await answer(page, "Close");
    expect(await viewer.calls("saveAs")).toEqual([]);
  });

  test("a text from the clipboard has no file to save to: Save asks where, and then it is that file", async ({ page, viewer }) => {
    await viewer.open(null, DESKTOP);
    await page.evaluate(() => {
      navigator.clipboard.readText = async () => "* pasted\nR1 a b 1k\nC1 b 0 1p\n";
      window.fakeShell.next = "edit";
    });
    await page.evaluate(() => window.viewer.command("load-clipboard"));
    await viewer.build(() => dialog(page).getByRole("button", { name: "Load" }).click());
    await begin(page);
    await rename(page);
    await page.evaluate(() => (window.fakeShell.savedAs = { name: "/saved/clip.sp", fixture: "edit-changed" }));
    await viewer.build(() => page.keyboard.press("Control+s"));
    // asked where, not written where it was read from, and the file written is the document now
    expect(await page.evaluate(() => window.fakeShell.writes)).toEqual([]);
    expect(await viewer.calls("saveAs")).toEqual([["clipboard.sp"]]);
    expect((await viewer.calls("build")).at(-1)).toEqual(["/saved/clip.sp"]);
    expect(await page.evaluate(() => window.viewer.edits)).toEqual({ unsaved: [], unapplied: [] });
  });
});

test.describe("leaving with changes not saved", () => {
  test("reading the file again asks, and Cancel leaves everything as it was", async ({ page, viewer }) => {
    await open(viewer, page);
    await rename(page);
    await page.evaluate(() => (window.fakeShell.next = "edit-changed"));
    await viewer.build(() => page.locator("#apply-edits").click());
    const builds = await viewer.builds();
    await page.keyboard.press("F5");
    await expect(dialog(page).locator(".dialog-title")).toHaveText("Changes Not Saved");
    await expect(dialog(page)).toContainText("This file has changes that are not saved: edit.sp.");
    await expect(dialog(page)).toContainText("Do you want to read the file again?");
    await answer(page, "Cancel");
    expect(await viewer.builds()).toBe(builds);
    expect(await page.evaluate(() => window.viewer.edits.unsaved)).toEqual(["edit.sp"]);
    expect(await text(page)).toContain("C2");
  });

  test("Don't Save leaves them behind, Save writes them first", async ({ page, viewer }) => {
    await open(viewer, page);
    await rename(page);
    await page.evaluate(() => (window.fakeShell.next = "edit-changed"));
    await viewer.build(() => page.locator("#apply-edits").click());
    // Save: written, then the file is read again as it now is
    await page.keyboard.press("F5");
    await viewer.build(() => dialog(page).getByRole("button", { name: "Save", exact: true }).click());
    expect(await page.evaluate(() => window.fakeShell.writes)).toHaveLength(1);
    expect(await page.evaluate(() => window.viewer.edits.unsaved)).toEqual([]);
    // Don't Save: the changes are gone, and the file is read again; the pane is still in edit
    await expect(area(page)).toBeVisible();
    await rename(page);
    await page.keyboard.press("F5");
    await viewer.build(() => dialog(page).getByRole("button", { name: "Don't Save" }).click());
    expect(await page.evaluate(() => window.fakeShell.writes)).toHaveLength(1);
    expect(await page.evaluate(() => window.viewer.edits)).toEqual({ unsaved: [], unapplied: [] });
    await expect(area(page)).toBeVisible();
    expect(await text(page)).toContain("C1");
  });

  test("opening another file asks too, and the file is not opened while Cancel is chosen", async ({ page, viewer }) => {
    await open(viewer, page);
    await rename(page);
    await page.evaluate(() => (window.fakeShell.picks = ["04-hierarchy"]));
    const builds = await viewer.builds();
    await page.evaluate(() => window.viewer.command("open"));
    await expect(dialog(page)).toContainText("Do you want to open another file?");
    await answer(page, "Cancel");
    expect(await viewer.builds()).toBe(builds);
    await expect(page.locator("#level-name")).toHaveText("edit.sp");
    // chosen again, and left unsaved, it opens
    await page.evaluate(() => (window.fakeShell.picks = ["04-hierarchy"]));
    await page.evaluate(() => window.viewer.command("open"));
    await viewer.build(() => dialog(page).getByRole("button", { name: "Don't Save" }).click());
    await expect(page.locator("#level-name")).toHaveText("04-hierarchy.sp");
    expect(await page.evaluate(() => window.viewer.edits)).toEqual({ unsaved: [], unapplied: [] });
    // and the editor is as it was, in edit, with the new document's files
    await expect(page.locator("#source .editor")).toHaveCount(2);
  });

  test("the window asks before it closes, and closes when the user says", async ({ page, viewer }) => {
    await open(viewer, page);
    await rename(page);
    await page.evaluate(() => window.fakeShell.requestClose());
    await expect(dialog(page)).toContainText("Do you want to close the window?");
    await answer(page, "Cancel");
    expect(await viewer.calls("quit")).toEqual([]);
    await page.evaluate(() => window.fakeShell.requestClose());
    await dialog(page).getByRole("button", { name: "Don't Save" }).click();
    await expect.poll(() => viewer.calls("quit")).toEqual([[]]);
    // saved first, it closes once it is saved
    await page.evaluate(() => window.fakeShell.requestClose());
    await dialog(page).getByRole("button", { name: "Save", exact: true }).click();
    await expect.poll(() => viewer.calls("quit")).toEqual([[], []]);
    expect(await page.evaluate(() => window.fakeShell.writes)).toHaveLength(1);
  });

  test("a failed save keeps the document", async ({ page, viewer }) => {
    await open(viewer, page);
    await rename(page);
    await page.evaluate(() => (window.fakeShell.writeFail = "disk full"));
    await page.evaluate(() => window.fakeShell.requestClose());
    await dialog(page).getByRole("button", { name: "Save", exact: true }).click();
    await expect(dialog(page)).toContainText("The changes were not saved: disk full");
    await answer(page, "Close");
    expect(await viewer.calls("quit")).toEqual([]);
    expect(await page.evaluate(() => window.viewer.edits.unsaved)).toEqual(["edit.sp"]);
  });
});

test.describe("on the web", () => {
  test("the page offers Edit and Save, which downloads, and no Save As", async ({ page, viewer }) => {
    await viewer.open("edit");
    await page.keyboard.press("Control+e");
    await expect(area(page)).toBeVisible();
    await rename(page);
    // Save is offered to the shell as it is on the desktop; of what is saved, the web shell
    // downloads the file shown
    await page.keyboard.press("Control+s");
    const written = await page.evaluate(() => window.fakeShell.writes);
    expect(written.map((files) => files.map((f) => f.path))).toEqual([["edit.sp"]]);
    expect(await viewer.calls("saveAs")).toEqual([]);
    await page.evaluate(() => window.viewer.command("save-as"));
    await expect(dialog(page)).toHaveCount(0);
    // leaving with changes unsaved asks, and offers only to leave them
    await type(page, `${await text(page)}* more\n`);
    await page.keyboard.press("F5");
    await expect(dialog(page).getByRole("button", { name: "Don't Save" })).toHaveCount(0);
    await expect(dialog(page).getByRole("button", { name: "Discard Changes" })).toBeVisible();
    await answer(page, "Cancel");
    const menus = await page.evaluate(async () => {
      const table = await (await fetch("commands.json")).json();
      return table.menus.map((m) => [m.label, m.items.filter((i) => i !== "-").map((i) => i.id)]);
    });
    expect(menus.find(([label]) => label === "&Edit")[1]).toEqual(["toggle-edit", "apply-edits"]);
    expect(menus.find(([label]) => label === "&File")[1]).toContain("save");
  });
});
