// File > Load from Clipboard (open.js): the page reads the clipboard's text, shows it with what it
// will be read as, and on Load asks the shell to build a source that holds the text, named
// clipboard.sp, .scs or .rnl. The test gives the page's clipboard its text; the fake shell builds
// the fixture that fakeShell.next names, as a text has none of its own.

import { expect, test } from "../viewer.js";

/** Give the page's clipboard a text, or with { error } make reading it fail with that message. */
async function clipboard(page, content) {
  await page.evaluate((c) => {
    navigator.clipboard.readText = async () => {
      if (c.error) throw new DOMException(c.error, "NotAllowedError");
      return c.text;
    };
  }, typeof content === "string" ? { text: content } : content);
}

/** The dialog shown. */
const dialog = (page) => page.locator("dialog.dialog");

/** Choose File > Load from Clipboard, and wait for its dialog. */
async function load(page) {
  await page.evaluate(() => window.viewer.command("load-clipboard"));
  await expect(dialog(page)).toBeVisible();
}

test("Load from Clipboard shows the text first, and opens it alone when Load is pressed", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy");
  const text = "* pasted\nR1 a b 1k\nC1 b 0 1p\n";
  await clipboard(page, text);
  await load(page);
  await expect(dialog(page).locator(".dialog-title")).toHaveText("Load from Clipboard");
  await expect(dialog(page).locator(".dialog-warning")).toContainText("in place of the document shown");
  await expect(dialog(page).locator(".clip-preview")).toHaveText("* pasted\nR1 a b 1k\nC1 b 0 1p");
  await expect(dialog(page).locator(".dialog-note").first()).toHaveText(`3 lines, ${text.length} characters.`);
  await expect(page.getByRole("radio", { name: "SPICE netlist (.sp)" })).toBeChecked();
  // Cancel opens nothing
  await dialog(page).getByRole("button", { name: "Cancel" }).click();
  await expect(dialog(page)).toHaveCount(0);
  expect(await viewer.calls("build")).toEqual([["04-hierarchy"]]);
  // Load opens the text, as clipboard.sp, in place of the document shown
  await page.evaluate(() => {
    window.fakeShell.next = "01-rc-lowpass";
  });
  await load(page);
  await viewer.build(() => dialog(page).getByRole("button", { name: "Load" }).click());
  expect((await viewer.calls("build")).slice(1)).toEqual([["clipboard.sp", text]]);
  await expect(dialog(page)).toHaveCount(0);
  // Reload builds the same text again, though the clipboard has changed
  await clipboard(page, "* another\n");
  await page.evaluate(() => {
    window.fakeShell.next = "01-rc-lowpass";
  });
  await viewer.build(() => page.keyboard.press("F5"));
  expect((await viewer.calls("build")).slice(2)).toEqual([["clipboard.sp", text]]);
});

test("what the text is read as is guessed from it, and can be chosen", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy");
  const guesses = [
    ["* SPICE\n.subckt x a b\n* @RNL {\n*   %s = {#R1}\n* }\n.ends\n", "SPICE netlist (.sp)"],
    ["// Spectre\nsubckt inv (a y)\nends inv\n", "Spectre netlist (.scs)"],
    ["simulator lang=spectre\nR1 (a b) resistor r=1k\n", "Spectre netlist (.scs)"],
    ["* a SPICE title\nR1 a b 1k\n// @RNL { }\n", "Spectre netlist (.scs)"],
    ['@RNL(type="header") { @VERSION("0.2")}\n@RNL {\n}\n', "RNL package, checked on its own (.rnl)"],
  ];
  for (const [text, kind] of guesses) {
    await clipboard(page, text);
    await load(page);
    await expect(page.getByRole("radio", { name: kind })).toBeChecked();
    await dialog(page).getByRole("button", { name: "Cancel" }).click();
    await expect(dialog(page)).toHaveCount(0);
  }
  // another choice opens the text as a file of that kind
  await clipboard(page, guesses[0][0]);
  await page.evaluate(() => {
    window.fakeShell.next = "inverter-chain";
  });
  await load(page);
  await page.getByRole("radio", { name: "Spectre netlist (.scs)" }).check();
  await viewer.build(() => dialog(page).getByRole("button", { name: "Load" }).click());
  expect((await viewer.calls("build")).at(-1)).toEqual(["clipboard.scs", guesses[0][0]]);
});

test("a clipboard that cannot be read, or holds no text, says so and opens nothing", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy");
  await clipboard(page, { error: "Read permission denied." });
  await load(page);
  await expect(dialog(page).locator(".dialog-body p").first()).toHaveText(
    "The clipboard could not be read: Read permission denied.",
  );
  await dialog(page).getByRole("button", { name: "Close" }).click();
  // the next command waits for this dialog to be gone: a modal dialog takes no command
  await expect(dialog(page)).toHaveCount(0);
  await clipboard(page, " \n\t\n");
  await load(page);
  await expect(dialog(page).locator(".dialog-body p")).toHaveText("The clipboard holds no text to load.");
  await dialog(page).getByRole("button", { name: "Close" }).click();
  await expect(dialog(page)).toHaveCount(0);
  expect(await viewer.calls("build")).toEqual([["04-hierarchy"]]);
  expect((await viewer.state()).title).toBe("04-hierarchy.sp");
});

test("the preview shows the first 200 lines of a long text, and says how long it is", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy");
  const lines = Array.from({ length: 250 }, (_, k) => `R${k} a${k} b${k} 1k`);
  const text = `* long\n${lines.join("\n")}\n`;
  await clipboard(page, text);
  await load(page);
  const shown = await dialog(page).locator(".clip-preview").textContent();
  expect(shown.split("\n")).toEqual(["* long", ...lines].slice(0, 200));
  await expect(dialog(page).locator(".dialog-note").first()).toHaveText(
    `251 lines, ${text.length} characters; the first 200 are shown.`,
  );
});

test("Ctrl+Shift+V loads from the clipboard, as File's item does", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy");
  await clipboard(page, "* keys\nR1 a b 1k\n");
  await page.keyboard.press("Control+Shift+V");
  await expect(dialog(page).locator(".clip-preview")).toHaveText("* keys\nR1 a b 1k");
  await page.keyboard.press("Escape");
  await expect(dialog(page)).toHaveCount(0);
  await viewer.menuTitle("File").click();
  await viewer.menuItem("load-clipboard").click();
  await expect(dialog(page).locator(".clip-preview")).toHaveText("* keys\nR1 a b 1k");
});
