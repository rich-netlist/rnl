// The desktop's Solve menu (apps/viewer-ui/solve.js, decision 0010): a solve drawn as a preview,
// the files unchanged; its report in the Solve view, each line shown in the netlist when clicked;
// its progress in the status bar and a Cancel button; File > Reload solving again and Close
// Preview showing the file as written; and a copy saved through the shell. The web shows none of it.
// The solver's options, Solve > Settings, are in solve-settings.spec.js.

import { dialogGone, expect, test } from "../viewer.js";

const DESKTOP = { platform: "desktop" };
const T4 = "T4-miller-conflicts";

/** The dialog shown. */
const dialog = (page) => page.locator("dialog.dialog");

/** Run a command and wait until the solve it begins is shown as a preview. */
async function solve(page, how = () => page.evaluate(() => window.viewer.command("solve"))) {
  await how();
  await page.waitForFunction(() => document.body.classList.contains("preview") && !document.body.classList.contains("solving"));
  // the Solve view shows the constraints after a solve; these tests read the report
  await page.locator("#solve-view-report").click();
}

/** The solves the shell was asked for: [name, settings] each. */
const solves = (viewer) => viewer.calls("solve");

test("the web offers no solver commands, and running them does nothing; the Solve menu and Solve view remain for overlays and constraints", async ({ page, viewer }) => {
  await viewer.open(T4);
  await expect(page.locator("#menubar .menu-title")).toContainText(["Solve"]);
  await expect(page.locator('#menubar .menu-item[data-id="solve"]')).toHaveCount(0);
  await expect(page.locator('#menubar .menu-item[data-id="toggle-layout"]')).toHaveCount(1);
  await page.evaluate(() => window.viewer.command("solve"));
  await page.keyboard.press("F9");
  expect(await solves(viewer)).toEqual([]);
  await expect(page.locator("#tab-solve")).toBeVisible();
  await expect(page.locator("#preview-badge")).toBeHidden();
});

test.describe("on the desktop", () => {
  test.beforeEach(async ({ viewer }) => {
    await viewer.open(T4, DESKTOP);
  });

  test("F9 solves the netlist and draws it as a preview, the files unchanged", async ({ page, viewer }) => {
    const builds = await viewer.builds();
    await solve(page, () => page.keyboard.press("F9"));
    // the default options of rnl solve, and nothing read again for the drawing
    expect(await solves(viewer)).toEqual([[T4, { config: "", timeall: null, level: `${T4}.sp` }]]);
    expect(await viewer.builds()).toBe(builds);
    await expect(page.locator("#preview-badge")).toHaveText("Solved preview: the files are unchanged");
    await expect(page.locator("#solve-step")).toHaveText(/^Solved: (1 level|\d+ levels), (1 fact|\d+ facts) not kept\. A preview: the files are unchanged\./);
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
    await page.evaluate(() => {
      window.fakeShell.step({ event: "run", progress: 1, levels: 2 });
      window.fakeShell.step({ event: "level", level: 0, title: "amp", again: false, limit: "time", seconds: 5 });
    });
    await expect(page.locator("#solve-step")).toHaveText("Solving amp (1 of 2)…");
    await page.evaluate(() => window.fakeShell.step({ event: "level", level: 1, title: "top.sp", again: true, limit: "time", seconds: 5 }));
    await expect(page.locator("#solve-step")).toHaveText("Solving top.sp (2 of 2, again after the upgrade to RNL 0.3)…");
    await page.locator("#solve-cancel").click();
    expect(await viewer.calls("cancelSolve")).toEqual([[]]);
    await expect(page.locator("#solve-step")).toHaveText("The solve was canceled; the netlist is shown as it was.");
    await expect(page.locator("#solve-cancel")).toBeHidden();
    await expect(page.locator("#preview-badge")).toBeHidden();
    expect(await page.evaluate(() => window.viewer.doc.files[0].text)).toBe(before);
  });

  test("F9 solves the circuit shown, by its title, and Solve All Circuits every circuit", async ({ page, viewer }) => {
    await viewer.open("04-hierarchy", DESKTOP);
    await solve(page);
    // the circuit shown is another one: the index of the Hierarchy view opens it
    await page.locator("#tab-hierarchy").click();
    await page.locator("#index .index-row", { hasText: "amp" }).click();
    await expect(page.locator("#level-name")).toHaveText("amp");
    await solve(page);
    await solve(page, () => page.evaluate(() => window.viewer.command("solve-all")));
    expect((await solves(viewer)).map(([name, settings]) => [name, settings.level])).toEqual([
      ["04-hierarchy", "04-hierarchy.sp"],
      ["04-hierarchy", "amp"],
      ["04-hierarchy", null],
    ]);
  });

  test("the panel over the drawing follows the events of the solve, and is gone when it ends", async ({ page }) => {
    const panel = page.locator("#solve-progress");
    const tell = (...events) => page.evaluate((list) => list.forEach((e) => window.fakeShell.step(e)), events);
    await page.evaluate(() => (window.fakeShell.hold = true));
    await expect(panel).toBeHidden();
    await page.evaluate(() => window.viewer.command("solve-all"));
    // before the solver says anything: the file, and the clock
    await expect(panel).toBeVisible();
    await expect(panel.locator(".solve-progress-title")).toHaveText(`${T4}`);
    await expect(panel.locator(".solve-progress-stage")).toHaveText("Starting");
    await expect(panel.locator(".solve-progress-time")).toHaveText(/^\d+\.\d s$/);
    await expect(panel.locator(".solve-progress-bar")).toBeHidden();

    await tell(
      { event: "run", progress: 1, levels: 2 },
      { event: "level", level: 0, title: "amp", again: false, limit: "time", seconds: 5 },
      { event: "stage", level: 0, stage: "resolve", work: 0, elapsed: 0.2 },
    );
    await expect(panel.locator(".solve-progress-title")).toHaveText("amp");
    await expect(panel.locator(".solve-progress-of")).toHaveText("1 of 2");
    await expect(panel.locator(".solve-progress-stage")).toHaveText("Checking the facts against each other");
    await expect(panel.locator(".solve-progress-time")).toContainText("s of 5.0 s");
    await expect(panel.locator(".solve-progress-bar")).toBeVisible();
    await expect(panel.locator(".solve-progress-tried")).toBeHidden();

    // the candidates tried and the better ones found; a stage the page has no words for is shown by its name
    await tell(
      { event: "work", level: 0, stage: "search", work: 1000, elapsed: 0.5, fraction: 0.1, tried: 1 },
      { event: "best", level: 0, tried: 1, work: 1000, elapsed: 0.5, cost: [3, 1] },
    );
    await expect(panel.locator(".solve-progress-tried")).toHaveText("1 candidate tried · 1 better one found");
    await tell(
      { event: "work", level: 0, stage: "search", work: 9000, elapsed: 1.5, fraction: 0.3, tried: 1200 },
      { event: "best", level: 0, tried: 1200, work: 9000, elapsed: 1.5, cost: [2, 1] },
      { event: "stage", level: 0, stage: "frobnicate", work: 9000, elapsed: 1.6 },
      { event: "from-the-future", level: 0, anything: true },
    );
    await expect(panel.locator(".solve-progress-tried")).toHaveText("1,200 candidates tried · 2 better ones found");
    await expect(panel.locator(".solve-progress-stage")).toHaveText("frobnicate");

    // a level ends, and the next begins, under a work limit: its share is the fraction the solver tells
    await tell(
      { event: "level-done", level: 0, status: "time", work: 9000, seconds: 5.04, kept: 3, discarded: 0 },
      { event: "level", level: 1, title: "top.sp", again: false, limit: "work", work: 100000 },
      { event: "work", level: 1, stage: "search", work: 50000, elapsed: 0.4, fraction: 0.5, tried: 7 },
    );
    await expect(panel.locator(".solve-progress-title")).toHaveText("top.sp");
    await expect(panel.locator(".solve-progress-of")).toHaveText("2 of 2");
    await expect(panel.locator(".solve-progress-time")).toContainText("50% of the work limit");
    await expect(panel.locator(".solve-progress-bar")).toHaveAttribute("aria-valuenow", "50");
    await expect(panel.locator(".solve-progress-tried")).toHaveText("7 candidates tried");
    const done = panel.locator(".solve-progress-done li");
    await expect(done).toHaveCount(1);
    await expect(done.locator(".name")).toHaveText("amp");
    await expect(done.locator(".sec")).toHaveText("5.0 s");
    await expect(done.locator(".end")).toHaveText("stopped at the time limit");

    // the status bar follows the same events
    await expect(page.locator("#solve-step")).toHaveText("Solving top.sp (2 of 2)…");
    // a level solved again, after the upgrade, takes the place of its first line
    await tell(
      { event: "level-done", level: 1, status: "complete", work: 1, seconds: 0.5, kept: 1, discarded: 0 },
      { event: "run", progress: 1, levels: 2 },
      { event: "level", level: 1, title: "top.sp", again: true, limit: "none" },
      { event: "level-done", level: 1, status: "complete", work: 1, seconds: 0.7, kept: 1, discarded: 0 },
    );
    await expect(done.locator(".name")).toHaveText(["amp", "top.sp"]);
    await expect(done.locator(".sec")).toHaveText(["5.0 s", "0.7 s"]);

    await page.evaluate(() => window.fakeShell.release());
    await page.waitForFunction(() => document.body.classList.contains("preview") && !document.body.classList.contains("solving"));
    await expect(panel).toBeHidden();
  });

  test("the panel is gone when the solve is canceled, and a new solve starts it again", async ({ page }) => {
    const panel = page.locator("#solve-progress");
    await page.evaluate(() => (window.fakeShell.hold = true));
    await page.evaluate(() => window.viewer.command("solve"));
    await expect(panel).toBeVisible();
    await page.evaluate(() => window.fakeShell.step({ event: "level", level: 0, title: "amp", again: false, limit: "none" }));
    await expect(panel.locator(".solve-progress-time")).toHaveText(/^\d+\.\d s$/);
    await page.locator("#solve-cancel").click();
    await expect(panel).toBeHidden();
    await page.evaluate(() => window.viewer.command("solve"));
    await expect(panel).toBeVisible();
    // none of the last solve is left in it
    await expect(panel.locator(".solve-progress-stage")).toHaveText("Starting");
    await expect(panel.locator(".solve-progress-done li")).toHaveCount(0);
    await page.evaluate(() => window.fakeShell.release());
    await expect(panel).toBeHidden();
  });

  test("Reload solves again while a preview is shown, and Close Preview reads the file as written", async ({ page, viewer }) => {
    await solve(page);
    await solve(page, () => page.keyboard.press("F5"));
    expect(await solves(viewer)).toHaveLength(2);
    // Close Preview: the file read again, as it is written
    await viewer.build(() => page.locator("#preview-close").click());
    await expect(page.locator("#preview-badge")).toBeHidden();
    await expect(page.locator("#tab-solve")).toBeVisible();
    await expect(page.locator("#pane-netlist")).toBeVisible();
    await page.keyboard.press("6");
    await expect(page.locator("#solve-report")).toHaveText("Run Solve (F9) to show the preview report.");
    await expect(page.locator("#solve-counts")).toBeEmpty();
    expect(await page.evaluate(() => window.viewer.doc.files[0].text.includes('@RNL(type="generated"'))).toBe(false);
    // and Reload reads the file again, as before any solve
    await viewer.build(() => page.keyboard.press("F5"));
    expect(await solves(viewer)).toHaveLength(2);
  });

  test("a copy is saved through the shell, of a solve shown alone", async ({ page, viewer }) => {
    // before a solve: nothing to save
    await page.evaluate(() => window.viewer.command("solve-save"));
    await expect(dialog(page)).toContainText("Run Solve › Solve (F9) first");
    await dialog(page).getByRole("button", { name: "Close" }).click();
    await dialogGone(page);
    expect(await viewer.calls("saveSolved")).toEqual([]);
    await solve(page);
    await page.keyboard.press("Control+Shift+S");
    await expect(dialog(page)).toContainText("Saved the solved copy as /copies/solved.sp.");
    await expect(dialog(page)).toContainText("The files that were opened are unchanged.");
    await dialog(page).getByRole("button", { name: "Close" }).click();
    await dialogGone(page);
    // a place refused, as over a file that was read, says why
    await page.evaluate(() => (window.fakeShell.saveFail = "a copy is never saved over a file that was read"));
    await page.evaluate(() => window.viewer.command("solve-save"));
    await expect(dialog(page)).toContainText("The copy was not saved: a copy is never saved over a file that was read");
    await dialog(page).getByRole("button", { name: "Close" }).click();
    await dialogGone(page);
    expect(await viewer.calls("saveSolved")).toHaveLength(2);
  });

  test("a solve that fails says why, and shows no preview", async ({ page, viewer }) => {
    await page.evaluate(() => (window.fakeShell.solveFail = "cannot read lib.inc"));
    await page.evaluate(() => window.viewer.command("solve"));
    await expect(dialog(page)).toContainText("The netlist could not be solved: cannot read lib.inc");
    await dialog(page).getByRole("button", { name: "Close" }).click();
    await dialogGone(page);
    await expect(page.locator("#preview-badge")).toBeHidden();
    await expect(page.locator("#tab-solve")).toBeVisible();
  });
});


test("desktop solved-copy UTF-8 command carries an explicit choice", async ({ page, viewer }) => {
  await viewer.open(T4, DESKTOP);
  await solve(page);
  await page.evaluate(() => window.viewer.command("solve-save-utf8"));
  await expect(dialog(page)).toContainText("Saved the solved copy");
  expect(await page.evaluate(() => window.fakeShell.encodings)).toEqual([["saveSolved", "utf8"]]);
});
