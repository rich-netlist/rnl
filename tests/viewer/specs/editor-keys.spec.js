// The keys of a text box of Edit Text (apps/viewer-ui/editor-keys.js, editor.js), in a browser: Enter in
// a comment begins the next line with its marker and its space, Shift+Enter does not; Tab indents and
// Shift+Tab outdents instead of moving the focus; Esc, then Tab, leaves the text box. What the keys put
// in is an edit of the text like typing, which the page takes as a change not applied. The rules
// themselves, line by line, are tested in apps/web/editor-keys.test.mjs.

import { expect, test } from "../viewer.js";

const area = (page) => page.locator("#source .editor:not([hidden]) textarea");

/** Open the fixture "edit" on the desktop, switch to edit, and put `text` in its text box with the caret at `caret`. */
async function editing(page, viewer, text, caret) {
  await viewer.open("edit", { platform: "desktop" });
  await page.keyboard.press("Control+q");
  await expect(area(page)).toBeVisible();
  await area(page).fill(text);
  await area(page).evaluate((a, at) => { a.focus(); a.setSelectionRange(at, at); }, caret);
}

/** The text of the file shown, as the page keeps it for Apply Changes. */
const kept = (page) => page.evaluate(async () => {
  const path = window.viewer.doc.files[0].path;
  return (await import("./edits.js")).textOf(path);
});

test("Enter in a comment continues it, with its space; Shift+Enter, and Enter before the marker, do not", async ({ page, viewer }) => {
  const text = "* @RNL {\n*   #R1 @PLACE(0, 0)\n* }\nR1 a b 1k\n.end\n";
  await editing(page, viewer, text, text.indexOf("\n* }"));
  await page.keyboard.press("Enter");
  await page.keyboard.type("#R2 @PLACE(4, 0)");
  expect(await area(page).inputValue()).toBe("* @RNL {\n*   #R1 @PLACE(0, 0)\n*   #R2 @PLACE(4, 0)\n* }\nR1 a b 1k\n.end\n");
  // the page has the edit, as it has what is typed
  expect(await kept(page)).toBe(await area(page).inputValue());
  await page.keyboard.press("Shift+Enter");
  expect(await area(page).inputValue()).toContain("*   #R2 @PLACE(4, 0)\n\n* }");
  // before the marker, at the beginning of the line
  await area(page).evaluate((a) => { const at = a.value.indexOf("* }"); a.setSelectionRange(at, at); });
  await page.keyboard.press("Enter");
  expect(await area(page).inputValue()).toContain("*   #R2 @PLACE(4, 0)\n\n\n* }");
});

test("Tab indents and Shift+Tab outdents in the text box, which keeps the focus; Esc, then Tab, leaves it", async ({ page, viewer }) => {
  const text = "* a note\nR1 a b 1k\n.end\n";
  await editing(page, viewer, text, text.indexOf("R1"));
  await page.keyboard.press("Tab");
  expect(await area(page).inputValue()).toBe("* a note\n  R1 a b 1k\n.end\n");
  await expect(area(page)).toBeFocused();
  // the lines selected, indented together
  await area(page).evaluate((a) => a.setSelectionRange(0, a.value.indexOf(".end")));
  await page.keyboard.press("Tab");
  expect(await area(page).inputValue()).toBe("*   a note\n    R1 a b 1k\n.end\n");
  await page.keyboard.press("Shift+Tab");
  await page.keyboard.press("Shift+Tab");
  expect(await area(page).inputValue()).toBe("* a note\nR1 a b 1k\n.end\n");
  expect(await kept(page)).toBe("* a note\nR1 a b 1k\n.end\n");
  await expect(area(page)).toBeFocused();
  // the keyboard's way out
  await page.keyboard.press("Escape");
  await page.keyboard.press("Tab");
  await expect(area(page)).not.toBeFocused();
  expect(await area(page).inputValue()).toBe("* a note\nR1 a b 1k\n.end\n");
});

test("the Keyboard Shortcuts say what Enter, Tab and Esc do in Edit Text", async ({ page, viewer }) => {
  await viewer.open("edit", { platform: "desktop" });
  await page.evaluate(() => window.viewer.command("shortcuts"));
  await expect(page.locator("dialog.dialog")).toContainText("Tab indents and Shift+Tab outdents; Esc, then Tab, leaves the text.");
});
