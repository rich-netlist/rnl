// The desktop's Solve menu (apps/viewer-ui/solve.js, decision 0010): a solve drawn as a preview,
// the files unchanged; its report in the Solve view, each line shown in the netlist when clicked;
// its progress in the status bar and a Cancel button; File > Reload solving again and Close
// Preview showing the file as written; the time limit; and a copy saved through the shell. The web
// shows none of it.

import { expect, test } from "../viewer.js";

const DESKTOP = { platform: "desktop" };
const T4 = "T4-miller-conflicts";

/** The dialog shown. */
const dialog = (page) => page.locator("dialog.dialog");

/** Run a command and wait until the solve it begins is shown as a preview. */
async function solve(page, how = () => page.evaluate(() => window.viewer.command("solve"))) {
  await how();
  await page.waitForFunction(() => document.body.classList.contains("preview") && !document.body.classList.contains("solving"));
}

/** The solves the shell was asked for: [name, timeLimit] each. */
const solves = (viewer) => viewer.calls("solve");

test("the web has no Solve menu, and its commands do nothing", async ({ page, viewer }) => {
  await viewer.open(T4);
  await expect(page.locator("#menubar .menu-title")).not.toContainText(["Solve"]);
  await page.evaluate(() => window.viewer.command("solve"));
  await page.keyboard.press("F9");
  expect(await solves(viewer)).toEqual([]);
  await expect(page.locator("#tab-solve")).toBeHidden();
  await expect(page.locator("#preview-badge")).toBeHidden();
});

test.describe("on the desktop", () => {
  test.beforeEach(async ({ viewer }) => {
    await viewer.open(T4, DESKTOP);
  });

  test("F9 solves the netlist and draws it as a preview, the files unchanged", async ({ page, viewer }) => {
    const builds = await viewer.builds();
    await solve(page, () => page.keyboard.press("F9"));
    // the default time limit of rnl solve, and nothing read again for the drawing
    expect(await solves(viewer)).toEqual([[T4, null]]);
    expect(await viewer.builds()).toBe(builds);
    await expect(page.locator("#preview-badge")).toHaveText("Solved preview: the files are unchanged");
    await expect(page.locator("#preview-close")).toBeVisible();
    // the Document shown is the solve's: the text has the record of the solver
    expect(await page.evaluate(() => window.viewer.doc.files[0].text.includes('@RNL(type="generated"'))).toBe(true);
    // the report, in the Solve view, which says that it is a preview
    await expect(page.locator("#tab-solve")).toHaveAttribute("aria-selected", "true");
    await expect(page.locator("#pane-solve")).toBeVisible();
    await expect(page.locator(".solve-note")).toContainText("nothing was written, and the files are as they were");
    await expect(page.locator(".solve-level h3 .title")).toHaveText([`${T4}.sp`]);
    const discarded = page.locator(".solve-line.fact.discarded");
    expect(await discarded.count()).toBeGreaterThan(0);
    await expect(discarded.first().locator(".more")).toContainText("it conflicts with");
    await expect(page.locator("#solve-counts")).toContainText("discarded");
    await expect(page.locator("#solve-step")).toContainText("A preview: the files are unchanged");
  });

  test("a line of the report shows its place in the netlist", async ({ page, viewer }) => {
    await solve(page);
    const at = await page.evaluate(() => {
      const facts = window.viewer.doc && document.querySelectorAll(".solve-line.fact.discarded.located");
      return facts.length;
    });
    expect(at).toBeGreaterThan(0);
    // the line of the first fact discarded, as the report gives it
    const line = await page.evaluate(async () => {
      const r = await fetch("/fixtures/T4-miller-conflicts-preview.json").then((x) => x.json());
      return r.report.levels[0].facts.find((f) => f.status === "discarded" && f.at).at.line;
    });
    await page.locator(".solve-line.fact.discarded.located").first().click();
    await expect(page.locator("#pane-netlist")).toBeVisible();
    await expect(viewer.line(line)).toHaveClass(/flash/);
    // the Solve view stays a tab away
    await page.locator("#tab-solve").click();
    await expect(page.locator("#pane-solve")).toBeVisible();
  });

  test("the status bar shows the level being solved, and Cancel leaves the netlist as it was", async ({ page, viewer }) => {
    const before = await page.evaluate(() => window.viewer.doc.files[0].text);
    await page.evaluate(() => (window.fakeShell.hold = true));
    await page.evaluate(() => window.viewer.command("solve"));
    await expect.poll(() => solves(viewer)).toHaveLength(1);
    await page.evaluate(() => window.fakeShell.step({ level: 0, levels: 2, title: "amp", again: false }));
    await expect(page.locator("#solve-step")).toHaveText("Solving amp (1 of 2)…");
    await page.evaluate(() => window.fakeShell.step({ level: 1, levels: 2, title: "top.sp", again: true }));
    await expect(page.locator("#solve-step")).toHaveText("Solving top.sp (2 of 2, again, as raised to 0.3)…");
    await page.locator("#solve-cancel").click();
    expect(await viewer.calls("cancelSolve")).toEqual([[]]);
    await expect(page.locator("#solve-step")).toHaveText("The solve was cancelled; the netlist is shown as it was.");
    await expect(page.locator("#solve-cancel")).toBeHidden();
    await expect(page.locator("#preview-badge")).toBeHidden();
    expect(await page.evaluate(() => window.viewer.doc.files[0].text)).toBe(before);
  });

  test("Reload solves again while a preview is shown, and Close Preview reads the file as written", async ({ page, viewer }) => {
    await solve(page);
    await solve(page, () => page.keyboard.press("F5"));
    expect(await solves(viewer)).toHaveLength(2);
    // Close Preview: the file read again, as it is written
    await viewer.build(() => page.locator("#preview-close").click());
    await expect(page.locator("#preview-badge")).toBeHidden();
    await expect(page.locator("#tab-solve")).toBeHidden();
    await expect(page.locator("#pane-netlist")).toBeVisible();
    expect(await page.evaluate(() => window.viewer.doc.files[0].text.includes('@RNL(type="generated"'))).toBe(false);
    // and Reload reads the file again, as before any solve
    await viewer.build(() => page.keyboard.press("F5"));
    expect(await solves(viewer)).toHaveLength(2);
  });

  test("the time limit is the default of rnl solve until it is set", async ({ page, viewer }) => {
    const set = async (choose) => {
      await page.evaluate(() => window.viewer.command("solve-time-limit"));
      await expect(dialog(page).locator(".dialog-title")).toHaveText("Solver Time Limit");
      await choose();
      await dialog(page).getByRole("button", { name: "OK" }).click();
      await expect(dialog(page)).toHaveCount(0);
    };
    await set(async () => {
      await dialog(page).locator("input.solve-seconds").fill("2.5");
    });
    expect(await viewer.stored("solve-time-limit")).toBe(2.5);
    await solve(page);
    // a number that will not do keeps the dialog, and says so
    await page.evaluate(() => window.viewer.command("solve-time-limit"));
    await dialog(page).locator("input.solve-seconds").fill("-1");
    await dialog(page).getByRole("button", { name: "OK" }).click();
    await expect(dialog(page).locator(".dialog-error")).toHaveText("Give a number of seconds, 0 or more.");
    await dialog(page).getByRole("button", { name: "Cancel" }).click();
    await set(() => dialog(page).getByLabel("No limit").check());
    await solve(page);
    await set(() => dialog(page).getByLabel("The default of rnl solve").check());
    await solve(page);
    expect((await solves(viewer)).map((c) => c[1])).toEqual([2.5, "none", null]);
  });

  test("a copy is saved through the shell, of a solve shown alone", async ({ page, viewer }) => {
    // before a solve: nothing to save
    await page.evaluate(() => window.viewer.command("solve-save"));
    await expect(dialog(page)).toContainText("Solve the netlist first");
    await dialog(page).getByRole("button", { name: "Close" }).click();
    expect(await viewer.calls("saveSolved")).toEqual([]);
    await solve(page);
    await page.keyboard.press("Control+Shift+S");
    await expect(dialog(page)).toContainText("Saved the solved copy as /copies/solved.sp.");
    await expect(dialog(page)).toContainText("The files that were opened are unchanged.");
    await dialog(page).getByRole("button", { name: "Close" }).click();
    // a place refused, as over a file that was read, says why
    await page.evaluate(() => (window.fakeShell.saveFail = "a copy is never saved over a file that was read"));
    await page.evaluate(() => window.viewer.command("solve-save"));
    await expect(dialog(page)).toContainText("The copy was not saved: a copy is never saved over a file that was read");
    await dialog(page).getByRole("button", { name: "Close" }).click();
    expect(await viewer.calls("saveSolved")).toHaveLength(2);
  });

  test("a solve that fails says why, and shows no preview", async ({ page, viewer }) => {
    await page.evaluate(() => (window.fakeShell.solveFail = "cannot read lib.inc"));
    await page.evaluate(() => window.viewer.command("solve"));
    await expect(dialog(page)).toContainText("The netlist could not be solved: cannot read lib.inc");
    await dialog(page).getByRole("button", { name: "Close" }).click();
    await expect(page.locator("#preview-badge")).toBeHidden();
    await expect(page.locator("#tab-solve")).toBeHidden();
  });
});
