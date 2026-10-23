// The desktop's Solve > Settings (apps/viewer-ui/solve-settings.js): a dialog for the options of the
// solver, with the time limit that Solve > Time Limit had. The defaults come from the shell, the page
// keeps only what differs from them, and a solve gets that written as `rnl solve --solver-options`
// takes it, which the shell can ask the solver to check. The fake shell's defaults are those of the
// solver at the time of writing; the page is tested against their shape.

import { expect, test } from "../viewer.js";

const DESKTOP = { platform: "desktop" };
const T4 = "T4-miller-conflicts";
const DEFAULT_ORDER = [
  "current_direction", "turns", "ground_direction", "component_alignment", "distribution", "symmetry",
  "terminal_wire_alignment", "area", "crossings", "wire_length", "aspect",
];
const OFF = ["signal_flow", "labels", "bend_alignment"];

/** The dialog shown. */
const dialog = (page) => page.locator("dialog.dialog");

/** A field of the dialog, by its label. */
const field = (page, label) => dialog(page).getByLabel(label, { exact: true });

/** An objective's row of the dialog. */
const objective = (page, name) => dialog(page).locator(`.objective[data-name="${name}"]`);

/** Open Solve > Settings. */
async function open(page) {
  await page.evaluate(() => window.viewer.command("solve-settings"));
  await expect(dialog(page).locator(".dialog-title")).toHaveText("Solver Settings");
}

/** Press OK, and wait until the dialog is gone. */
async function ok(page) {
  await dialog(page).getByRole("button", { name: "OK" }).click();
  await expect(dialog(page)).toHaveCount(0);
}

/** The objectives of the dialog in the order shown, each with whether it is on. */
const order = (page) =>
  dialog(page)
    .locator(".objective")
    .evaluateAll((rows) => rows.map((r) => [r.dataset.name, !r.classList.contains("off")]));

/** The text of the settings the dialog says the solver gets. */
const asText = (page) => dialog(page).locator(".settings-text");

/** Run a solve and wait until it is shown as a preview; the settings the shell was last given. */
async function solved(page, viewer) {
  const before = (await viewer.calls("solve")).length;
  await page.evaluate(() => window.viewer.command("solve"));
  await expect.poll(async () => (await viewer.calls("solve")).length).toBe(before + 1);
  await page.waitForFunction(() => document.body.classList.contains("preview") && !document.body.classList.contains("solving"));
  return (await viewer.calls("solve")).at(-1)[1];
}

test("the web has no Solve settings, and its command does nothing", async ({ page, viewer }) => {
  await viewer.open(T4);
  await page.evaluate(() => window.viewer.command("solve-settings"));
  await expect(dialog(page)).toHaveCount(0);
  expect(await viewer.calls("solverDefaults")).toEqual([]);
});

test.describe("on the desktop", () => {
  test.beforeEach(async ({ viewer }) => {
    await viewer.open(T4, DESKTOP);
  });

  test("Settings is in the Solve menu, where Time Limit was", async ({ page }) => {
    const solve = await page.evaluate(async () => {
      const table = await (await fetch("commands.json")).json();
      return table.menus.find((m) => m.label === "&Solve").items.filter((i) => i !== "-").map((i) => [i.id, i.label]);
    });
    expect(solve.map(([id]) => id)).toEqual(["solve", "solve-close", "solve-save", "solve-save-utf8", "solve-settings"]);
    expect(solve.at(-1)).toEqual(["solve-settings", "S&ettings..."]);
  });

  test("the dialog shows the solver's defaults, and says what these settings are not", async ({ page, viewer }) => {
    await open(page);
    expect(await viewer.calls("solverDefaults")).toEqual([[]]);
    await expect(field(page, "Time limit")).toHaveValue("5");
    await expect(field(page, "No limit")).not.toBeChecked();
    await expect(field(page, "Tolerance")).toHaveValue("0.000001");
    await expect(field(page, "Grid")).toHaveValue("1");
    await expect(field(page, "No rounding")).not.toBeChecked();
    await expect(field(page, "Beam")).toHaveValue("4");
    await expect(field(page, "Candidates")).toHaveValue("256");
    await expect(field(page, "Router cells")).toHaveValue("200000");
    await expect(field(page, "Seek every conflict group of the facts of priority low")).not.toBeChecked();
    // the priority of a constraint is written in the netlist
    await expect(dialog(page).locator(".settings-note")).toContainText("priority of a constraint, low, normal or high, is written in the netlist");
    await expect(dialog(page).locator(".settings-note")).toContainText("not set here");
    // the objectives: the default layers in order, then those that are off
    expect(await order(page)).toEqual([...DEFAULT_ORDER.map((n) => [n, true]), ...OFF.map((n) => [n, false])]);
    await expect(dialog(page).locator(".objective-level")).toHaveText([
      "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "off", "off", "off",
    ]);
    await expect(asText(page)).toHaveText("The defaults of rnl solve.");
    // Cancel keeps nothing, and a solve gets the defaults
    await dialog(page).getByRole("button", { name: "Cancel" }).click();
    expect(await viewer.stored("solve-options")).toBeNull();
    expect(await solved(page, viewer)).toEqual({ options: "", hintsForLow: false });
  });

  test("a time limit set, and no limit, reach the solve; Restore Defaults takes them back", async ({ page, viewer }) => {
    await open(page);
    await field(page, "Time limit").fill("2.5");
    await expect(asText(page)).toHaveText('As rnl solve takes them: rnl solve --solver-options "time_limit=2.5"');
    await ok(page);
    expect(await viewer.stored("solve-options")).toEqual({ time_limit: 2.5 });
    expect(await solved(page, viewer)).toEqual({ options: "time_limit=2.5", hintsForLow: false });
    // the solver is asked whether it takes them, as `--solver-options` writes them
    expect((await viewer.calls("checkSolverOptions")).at(-1)).toEqual(["time_limit=2.5"]);
    // what was kept shows again, and no limit disables the number
    await open(page);
    await expect(field(page, "Time limit")).toHaveValue("2.5");
    await field(page, "No limit").check();
    await expect(field(page, "Time limit")).toBeDisabled();
    await ok(page);
    expect(await solved(page, viewer)).toEqual({ options: "time_limit=none", hintsForLow: false });
    // Restore Defaults sets the fields, and OK keeps no change at all
    await open(page);
    await expect(field(page, "No limit")).toBeChecked();
    await dialog(page).getByRole("button", { name: "Restore Defaults" }).click();
    await expect(field(page, "Time limit")).toHaveValue("5");
    await expect(field(page, "No limit")).not.toBeChecked();
    await ok(page);
    expect(await viewer.stored("solve-options")).toEqual({});
    expect(await solved(page, viewer)).toEqual({ options: "", hintsForLow: false });
  });

  test("the other options reach the solve in the order the solver writes its own", async ({ page, viewer }) => {
    await open(page);
    await field(page, "Tolerance").fill("0.001");
    await field(page, "No rounding").check();
    await field(page, "Beam").fill("8");
    await field(page, "Candidates").fill("10");
    await field(page, "Router cells").fill("5000");
    await field(page, "Seek every conflict group of the facts of priority low").check();
    await expect(asText(page)).toHaveText(
      'As rnl solve takes them: rnl solve --report --solver-options "tolerance=0.001;grid=none;beam=8;budget=candidates:10,astar_expansions:5000"',
    );
    await ok(page);
    expect(await viewer.stored("solve-options")).toEqual({
      tolerance: 0.001,
      grid: "none",
      beam: 8,
      budget: { candidates: 10, astar_expansions: 5000 },
      hints_for_low: true,
    });
    expect(await solved(page, viewer)).toEqual({
      options: "tolerance=0.001;grid=none;beam=8;budget=candidates:10,astar_expansions:5000",
      hintsForLow: true,
    });
    // a number for the grid again
    await open(page);
    await expect(field(page, "No rounding")).toBeChecked();
    await field(page, "No rounding").uncheck();
    await field(page, "Grid").fill("0.5");
    await field(page, "Candidates").fill("256");
    await ok(page);
    expect((await viewer.stored("solve-options")).grid).toBe(0.5);
    expect((await viewer.stored("solve-options")).budget).toEqual({ astar_expansions: 5000 });
  });

  test("the objectives move up and down, and the order is what the solver gets", async ({ page, viewer }) => {
    await open(page);
    await expect(objective(page, "current_direction").getByRole("button", { name: "Move up: current_direction" })).toBeDisabled();
    await expect(objective(page, "bend_alignment").getByRole("button", { name: "Move down: bend_alignment" })).toBeDisabled();
    // area, the eighth, goes up past terminal_wire_alignment and symmetry, and keeps the focus
    const up = objective(page, "area").getByRole("button", { name: "Move up: area" });
    await up.click();
    await up.click();
    await expect(up).toBeFocused();
    const moved = [...DEFAULT_ORDER];
    moved.splice(7, 1);
    moved.splice(5, 0, "area");
    expect((await order(page)).slice(0, 11).map(([n]) => n)).toEqual(moved);
    await expect(asText(page)).toContainText(`objectives=${moved.join(",")}`);
    // an objective that is off can move too: signal_flow goes to the top
    for (let k = 0; k < 11; k++) await objective(page, "signal_flow").getByRole("button", { name: "Move up: signal_flow" }).click();
    expect((await order(page))[0]).toEqual(["signal_flow", false]);
    await ok(page);
    expect(await viewer.stored("solve-options")).toEqual({ objectives: moved.map((name) => ({ items: [{ name, weight: 1 }], tolerance: 0 })) });
    expect((await solved(page, viewer)).options).toBe(`objectives=${moved.join(",")}`);
    // moved back, they are the defaults again, and nothing is kept for them
    await open(page);
    await expect(objective(page, "signal_flow")).toHaveClass(/off/);
    await expect(objective(page, "area").locator(".objective-level")).toHaveText("6");
    await dialog(page).getByRole("button", { name: "Restore Defaults" }).click();
    await ok(page);
    expect(await viewer.stored("solve-options")).toEqual({});
  });

  test("objectives are turned on and off, and the solver gets those that are on", async ({ page, viewer }) => {
    await open(page);
    await objective(page, "turns").getByLabel("turns is on").uncheck();
    await objective(page, "signal_flow").getByLabel("signal_flow is on").check();
    await expect(objective(page, "turns")).toHaveClass(/off/);
    await expect(objective(page, "turns").locator(".objective-level")).toHaveText("off");
    // the levels count the objectives that are on; signal_flow is where it was, after the others
    await expect(objective(page, "ground_direction").locator(".objective-level")).toHaveText("2");
    await expect(objective(page, "signal_flow").locator(".objective-level")).toHaveText("11");
    const on = DEFAULT_ORDER.filter((n) => n !== "turns").concat("signal_flow");
    await expect(asText(page)).toContainText(`objectives=${on.join(",")}`);
    await ok(page);
    expect((await solved(page, viewer)).options).toBe(`objectives=${on.join(",")}`);
    // all of them off is no objective at all: the solver only satisfies the facts
    await open(page);
    for (const name of on) await objective(page, name).getByLabel(`${name} is on`).uncheck();
    await expect(asText(page)).toContainText("objectives=none");
    await ok(page);
    expect((await solved(page, viewer)).options).toBe("objectives=none");
  });

  test("objectives share a level with weights, and the level has a tolerance", async ({ page, viewer }) => {
    await open(page);
    // the first objective has no level above to join, and an objective that is off has no weight to set
    await expect(objective(page, "current_direction").getByLabel("with the one above")).toBeHidden();
    await expect(objective(page, "signal_flow").getByLabel("weight")).toBeDisabled();
    // wire_length joins crossings, the objective above it, with the weight 2; the level has a tolerance
    await objective(page, "wire_length").getByLabel("with the one above").check();
    await objective(page, "wire_length").getByLabel("weight").fill("2");
    await objective(page, "crossings").getByLabel("equal within").fill("0.5");
    await expect(objective(page, "wire_length").locator(".objective-level")).toHaveText("+");
    await expect(objective(page, "aspect").locator(".objective-level")).toHaveText("10");
    // only the objective that begins a level has a tolerance of the level
    await expect(objective(page, "wire_length").getByLabel("equal within")).toBeHidden();
    await expect(asText(page)).toContainText(
      `objectives=${DEFAULT_ORDER.slice(0, 8).join(",")},crossings+wire_length*2~0.5,aspect"`,
    );
    await ok(page);
    const kept = await viewer.stored("solve-options");
    expect(kept.objectives[8]).toEqual({
      items: [{ name: "crossings", weight: 1 }, { name: "wire_length", weight: 2 }],
      tolerance: 0.5,
    });
    expect(kept.objectives).toHaveLength(10);
    // shown again as it was kept; "with the one above" follows the order: wire_length moved up over
    // crossings is together with area now, and crossings begins a level, with a tolerance to set
    await open(page);
    await expect(objective(page, "wire_length").getByLabel("with the one above")).toBeChecked();
    await expect(objective(page, "wire_length").getByLabel("weight")).toHaveValue("2");
    await expect(objective(page, "crossings").getByLabel("equal within")).toHaveValue("0.5");
    await objective(page, "wire_length").getByRole("button", { name: "Move up: wire_length" }).click();
    await expect(objective(page, "wire_length").locator(".objective-level")).toHaveText("+");
    await expect(objective(page, "crossings").locator(".objective-level")).toHaveText("9");
    await expect(objective(page, "crossings").getByLabel("equal within")).toBeVisible();
    await expect(objective(page, "area").getByLabel("equal within")).toBeVisible();
    await dialog(page).getByRole("button", { name: "Cancel" }).click();
    // Cancel kept the first order
    expect((await viewer.stored("solve-options")).objectives[8].items[0].name).toBe("crossings");
  });

  test("a value that is not a number is refused where it is, and the solver's refusal is shown", async ({ page, viewer }) => {
    await open(page);
    await field(page, "Beam").fill("");
    await dialog(page).getByRole("button", { name: "OK" }).click();
    await expect(dialog(page).locator(".dialog-error")).toHaveText("The beam is a whole number.");
    await field(page, "Beam").fill("2.5");
    await dialog(page).getByRole("button", { name: "OK" }).click();
    await expect(dialog(page).locator(".dialog-error")).toHaveText("The beam is a whole number.");
    await field(page, "Beam").fill("4");
    await field(page, "Time limit").fill("");
    await dialog(page).getByRole("button", { name: "OK" }).click();
    await expect(dialog(page).locator(".dialog-error")).toHaveText("The time limit is a number of seconds, or no limit.");
    await field(page, "Time limit").fill("5");
    await objective(page, "area").getByLabel("weight").fill("1.5");
    await dialog(page).getByRole("button", { name: "OK" }).click();
    await expect(dialog(page).locator(".dialog-error")).toHaveText("The weight of area is a whole number.");
    await objective(page, "area").getByLabel("weight").fill("1");
    // nothing reached the solver or was kept while it would not do
    expect(await viewer.calls("checkSolverOptions")).toEqual([]);
    expect(await viewer.stored("solve-options")).toBeNull();
    // what the solver itself says is wrong stays in the dialog, in its words
    await page.evaluate(() => (window.fakeShell.optionsError = "the beam must be at least 1"));
    await field(page, "Beam").fill("0");
    await dialog(page).getByRole("button", { name: "OK" }).click();
    await expect(dialog(page).locator(".dialog-error")).toHaveText("the beam must be at least 1");
    await expect(dialog(page)).toHaveCount(1);
    expect(await viewer.calls("checkSolverOptions")).toEqual([["beam=0"]]);
    expect(await viewer.stored("solve-options")).toBeNull();
    // and the dialog is usable again: a beam that will do keeps it
    await page.evaluate(() => (window.fakeShell.optionsError = null));
    await field(page, "Beam").fill("2");
    await ok(page);
    expect(await viewer.stored("solve-options")).toEqual({ beam: 2 });
  });

  test("the settings stay for the next start, and the time limit of Solve > Time Limit is one of them", async ({ page, viewer }) => {
    await open(page);
    await field(page, "Beam").fill("6");
    await ok(page);
    await page.reload();
    await viewer.started();
    await viewer.idle();
    await open(page);
    await expect(field(page, "Beam")).toHaveValue("6");
    await dialog(page).getByRole("button", { name: "Cancel" }).click();
    expect(await solved(page, viewer)).toEqual({ options: "beam=6", hintsForLow: false });
    // a time limit kept by the menu item that the settings replaced still counts, until they are kept
    await page.evaluate(() => {
      localStorage.removeItem("solve-options");
      localStorage.setItem("solve-time-limit", JSON.stringify("none"));
    });
    expect((await solved(page, viewer)).options).toBe("time_limit=none");
    await open(page);
    await expect(field(page, "No limit")).toBeChecked();
    await ok(page);
    expect(await viewer.stored("solve-options")).toEqual({ time_limit: "none" });
  });
});
