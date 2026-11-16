// Solve > Settings (apps/viewer-ui/solve-settings.js): a dialog for the settings of the solver, drawn from the
// table the shell gives (docs/solver-settings-table.md), so that the page keeps no key, bound or default of its
// own. The page keeps only what differs from the table's defaults, and a solve gets that written as RNL named
// parameters, as `rnl solve --solver-config` takes them, which the shell can ask the solver to check. The fake
// shell's table is the solver's at the time of writing; the page is tested against its shape.

import { readFileSync } from "node:fs";
import { dialogGone, expect, test } from "../viewer.js";

const DESKTOP = { platform: "desktop" };
const T4 = "T4-miller-conflicts";

// The objectives as the fake shell's copy of the solver's table has them: the default layers in order,
// then the others in the table's order, which the dialog shows off after them.
const SHELL = readFileSync(new URL("../fake-shell.js", import.meta.url), "utf8");
const listed = (key) => {
  const from = SHELL.indexOf(`\n  ${key}: [`);
  return [...SHELL.slice(from, SHELL.indexOf("\n  ],", from)).matchAll(/"name":"(\w+)"/g)].map((m) => m[1]);
};
const DEFAULT_ORDER = listed("default_objectives");
const OFF = listed("objectives").filter((n) => !DEFAULT_ORDER.includes(n));
const LEVELS = DEFAULT_ORDER.length;

/** The dialog shown. */
const dialog = (page) => page.locator("dialog.dialog");

/** The setting of a key in the dialog. */
const setting = (page, key) => dialog(page).locator(`.setting[data-key="${key}"]`);

/** The control of a setting, by its key. */
const field = (page, key) => dialog(page).getByLabel(key, { exact: true });

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
  // the settings, not which circuit (solve.spec.js)
  const { level, ...settings } = (await viewer.calls("solve")).at(-1)[1];
  return settings;
}

test("the web has no Solve settings, and its command does nothing", async ({ page, viewer }) => {
  await viewer.open(T4);
  await page.evaluate(() => window.viewer.command("solve-settings"));
  await expect(dialog(page)).toHaveCount(0);
  expect(await viewer.calls("solverTable")).toEqual([]);
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
    expect(solve.map(([id]) => id)).toEqual([
      "solve", "solve-all", "solve-respace", "solve-respace-all", "solve-close", "solve-save", "solve-save-utf8",
      "toggle-layout", "toggle-skeleton-symbols", "toggle-skeleton-wires",
      "solve-settings",
    ]);
    expect(solve.at(-1)).toEqual(["solve-settings", "S&ettings..."]);
  });

  test("the dialog is drawn from the solver's table: its groups, its defaults, what it says of each, and what these settings are not", async ({ page, viewer }) => {
    await open(page);
    expect(await viewer.calls("solverTable")).toEqual([[]]);
    // the groups, in the table's order, with its titles
    await expect(dialog(page).locator("fieldset > legend")).toHaveText([
      "Layout", "Search and preferences", "Limit of each level's solve", "Caps on the search and the router", "Report", "Time for the whole file",
    ]);
    // the defaults of the table
    await expect(field(page, "layout")).toHaveValue("lines");
    await expect(field(page, "spacing_x")).toHaveValue("0");
    await expect(field(page, "spacing_y")).toHaveValue("0");
    await expect(field(page, "beam")).toHaveValue("4");
    await expect(field(page, "grid")).toHaveValue("1");
    await expect(field(page, "tolerance")).toHaveValue("0.000001");
    await expect(field(page, "limit")).toHaveValue("time");
    await expect(field(page, "seconds")).toHaveValue("10");
    await expect(field(page, "candidates")).toHaveValue("256");
    await expect(field(page, "astar_expansions")).toHaveValue("200000");
    await expect(field(page, "report_hints")).not.toBeChecked();
    // what the solver says of a setting and of a choice
    await expect(setting(page, "beam").locator(".settings-hint")).toContainText("How many partial candidates the row search keeps");
    await expect(setting(page, "layout").locator(".choice-what")).toHaveText("Components placed along the lines of their wires, then the drawing compacted on the grid.");
    // a setting with no default is given or not: junction is not, and its box waits until it is
    await expect(setting(page, "junction").getByLabel("set")).not.toBeChecked();
    await expect(field(page, "junction")).toBeDisabled();
    // the priority of a constraint is written in the netlist
    await expect(dialog(page).locator(".settings-note")).toContainText("priority of a constraint, low, normal or high, is written in the netlist");
    await expect(dialog(page).locator(".settings-note")).toContainText("not set here");
    // the objectives: the default layers in order, then those that are off
    expect(await order(page)).toEqual([...DEFAULT_ORDER.map((n) => [n, true]), ...OFF.map((n) => [n, false])]);
    // (the default layout is lines: it takes four objectives, crowding of them on, and the rest are hidden, their tiers not counted)
    await expect(dialog(page).locator(".objective:not([hidden])")).toHaveCount(4);
    await expect(objective(page, "crowding").locator(".objective-level")).toHaveText("1");
    await expect(objective(page, "turns").locator(".objective-level")).toHaveText("–");
    // an objective that this version does not compute says so
    await expect(objective(page, "labels").locator(".objective-what")).toContainText("(not computed by this version)");
    await expect(objective(page, "turns").locator(".objective-what")).not.toContainText("not computed");
    await expect(asText(page)).toHaveText("The defaults of rnl solve.");
    // Cancel keeps nothing, and a solve gets the defaults
    await dialog(page).getByRole("button", { name: "Cancel" }).click();
    await dialogGone(page);
    expect(await viewer.stored("solve-config")).toBeNull();
    expect(await solved(page, viewer)).toEqual({ config: "", timeall: null });
  });

  test("a setting's conditions: the spacing is not used outside the lines layout, the work and the seconds follow the limit", async ({ page }) => {
    await open(page);
    // the lines layout uses the spacing; the beam is the rows': hidden, and shown greyed on request
    await expect(setting(page, "spacing_x")).toBeVisible();
    await expect(setting(page, "spacing_x")).not.toHaveClass(/unused/);
    await expect(setting(page, "beam")).toBeHidden();
    await field(page, "layout").selectOption("rows");
    await expect(setting(page, "spacing_x")).toBeHidden();
    await expect(setting(page, "beam")).toBeVisible();
    await expect(setting(page, "beam")).not.toHaveClass(/unused/);
    await expect(setting(page, "candidates")).toBeVisible();
    // a time limit has seconds; the work is shown only for a limit of work, where it is required
    await expect(setting(page, "seconds")).toBeVisible();
    await expect(setting(page, "work")).toBeHidden();
    await expect(setting(page, "safety_seconds")).toBeHidden();
    await field(page, "limit").selectOption("work");
    await expect(setting(page, "seconds")).toBeHidden();
    await expect(setting(page, "work")).toBeVisible();
    await expect(setting(page, "work").getByLabel("set")).toBeChecked();
    await expect(setting(page, "work").getByLabel("set")).toBeDisabled();
    await expect(setting(page, "safety_seconds")).toBeVisible();
  });

  test("the settings reach the solve as named parameters, and the solver is asked whether it takes them", async ({ page, viewer }) => {
    await open(page);
    await field(page, "spacing_x").fill("3");
    await field(page, "layout").selectOption("rows");
    await field(page, "beam").fill("8");
    // the spacing is the lines layout's: kept, but not given to the solver under rows
    await expect(asText(page)).toHaveText(`Equivalent CLI command: rnl solve --solver-config 'beam=8,layout="rows"'`);
    await ok(page);
    expect(await viewer.stored("solve-config")).toEqual({ beam: 8, layout: "rows", spacing_x: 3 });
    expect((await viewer.calls("checkSettings")).at(-1)).toEqual(['beam=8,layout="rows"']);
    expect(await solved(page, viewer)).toEqual({ config: 'beam=8,layout="rows"', timeall: null });
    // what was kept shows again
    await open(page);
    await expect(field(page, "layout")).toHaveValue("rows");
    await expect(field(page, "spacing_x")).toHaveValue("3");
    // Restore Defaults sets the fields, and OK keeps no change at all
    await dialog(page).getByRole("button", { name: "Restore Defaults" }).click();
    await expect(field(page, "layout")).toHaveValue("lines");
    await expect(field(page, "spacing_x")).toHaveValue("0");
    await ok(page);
    expect(await viewer.stored("solve-config")).toEqual({});
    expect(await solved(page, viewer)).toEqual({ config: "", timeall: null });
  });

  test("a limit of work, no rounding to a grid, and a junction given or not: each as the table types it", async ({ page, viewer }) => {
    await open(page);
    await field(page, "limit").selectOption("work");
    await field(page, "work").fill("5000");
    // `grid` is a number or false: the box beside it says what false means
    await field(page, "Positions are not rounded to a grid.").check();
    await expect(field(page, "grid")).toBeDisabled();
    // a string that is given: the junction, whose empty text draws none
    await setting(page, "junction").getByLabel("set").check();
    await expect(field(page, "junction")).toBeEnabled();
    await field(page, "junction").fill("$JUNCTION");
    await expect(asText(page)).toHaveText(`Equivalent CLI command: rnl solve --solver-config 'grid=false,junction="$JUNCTION",limit="work",work=5000'`);
    await ok(page);
    expect(await solved(page, viewer)).toEqual({ config: 'grid=false,junction="$JUNCTION",limit="work",work=5000', timeall: null });
    // a number for the grid again, and the junction not given
    await open(page);
    await expect(field(page, "Positions are not rounded to a grid.")).toBeChecked();
    await field(page, "Positions are not rounded to a grid.").uncheck();
    await field(page, "grid").fill("0.5");
    await setting(page, "junction").getByLabel("set").uncheck();
    await ok(page);
    expect(await viewer.stored("solve-config")).toEqual({ grid: 0.5, limit: "work", work: 5000 });
  });

  test("the group-tree start's work is a number, false, or left out; it follows the lines layout and a limit of time or work", async ({ page, viewer }) => {
    await open(page);
    const row = setting(page, "tree_work");
    const set = row.getByLabel("set");
    const unlimited = row.getByLabel("The group-tree start refines until its search ends or the budget runs out.");
    // lines with the time limit: the row is there, left out (no default), and nothing of it is given
    await expect(row).toBeVisible();
    await expect(set).not.toBeChecked();
    await expect(field(page, "tree_work")).toBeDisabled();
    await expect(asText(page)).toHaveText("The defaults of rnl solve.");
    // a number: given with the others, in the order of their names
    await set.check();
    await expect(field(page, "tree_work")).toBeEnabled();
    await field(page, "tree_work").fill("81000");
    await expect(asText(page)).toHaveText(`Equivalent CLI command: rnl solve --solver-config 'tree_work=81000'`);
    await field(page, "limit").selectOption("work");
    await field(page, "work").fill("243750");
    await expect(row).toBeVisible();
    await expect(asText(page)).toHaveText(`Equivalent CLI command: rnl solve --solver-config 'limit="work",tree_work=81000,work=243750'`);
    // false says what it means, and puts the number box out
    await unlimited.check();
    await expect(field(page, "tree_work")).toBeDisabled();
    await expect(asText(page)).toHaveText(`Equivalent CLI command: rnl solve --solver-config 'limit="work",tree_work=false,work=243750'`);
    await unlimited.uncheck();
    // the bounds of the table
    await field(page, "tree_work").fill("-1");
    await dialog(page).getByRole("button", { name: "OK" }).click();
    await expect(dialog(page).locator(".dialog-error")).toHaveText("tree_work must be at least 0.");
    await field(page, "tree_work").fill("81000");
    // the seed's limit searches no further: the row is hidden, its value kept (as the work's is, which a limit of the seed does not show either)
    await field(page, "limit").selectOption("seed");
    await expect(row).toBeHidden();
    await field(page, "limit").selectOption("work");
    await expect(row).toBeVisible();
    await expect(field(page, "tree_work")).toHaveValue("81000");
    // rows does not have a group-tree start
    await field(page, "layout").selectOption("rows");
    await expect(row).toBeHidden();
    await expect(asText(page)).not.toContainText("tree_work");
    await dialog(page).locator(".settings-other").getByLabel("Show").check();
    await expect(row).toBeVisible();
    await expect(row).toHaveClass(/unused/);
    await field(page, "layout").selectOption("lines");
    await ok(page);
    expect(await viewer.stored("solve-config")).toEqual({ limit: "work", tree_work: 81000, work: 243750 });
    expect(await solved(page, viewer)).toEqual({ config: 'limit="work",tree_work=81000,work=243750', timeall: null });
    expect((await viewer.calls("checkSettings")).at(-1)).toEqual(['limit="work",tree_work=81000,work=243750']);
    // kept for the next time, still a number
    await open(page);
    await expect(set).toBeChecked();
    await expect(field(page, "tree_work")).toHaveValue("81000");
    await set.uncheck();
    await ok(page);
    expect(await viewer.stored("solve-config")).toEqual({ limit: "work", work: 243750 });
  });

  test("the weights of a crossing and of a bend are two boxes of one array that follows the lines layout, and the order of its ranking is shown beside the objectives", async ({ page, viewer }) => {
    await open(page);
    const row = setting(page, "lines_weights");
    const crossing = field(page, "lines_weights crossing");
    const bend = field(page, "lines_weights bend");
    const refusal = dialog(page).locator(".dialog-error");
    // lines: a box for each weight, named as the table names them, with the defaults
    await expect(row).toBeVisible();
    await expect(row.locator(".settings-element")).toHaveText(["crossing", "bend"]);
    await expect(crossing).toHaveValue("1");
    await expect(bend).toHaveValue("4");
    await expect(asText(page)).toHaveText("The defaults of rnl solve.");
    // the weights are given as one array, in the order of the table, and as the CLI writes it
    await crossing.fill("2");
    await bend.fill("3");
    await expect(asText(page)).toHaveText(`Equivalent CLI command: rnl solve --solver-config 'lines_weights=[2,3]'`);
    // out of the bounds of each, or not a number, is refused where it is, in the words of the box
    await bend.fill("1001");
    await dialog(page).getByRole("button", { name: "OK" }).click();
    await expect(refusal).toHaveText("lines_weights bend must be at most 1000.");
    await crossing.fill("");
    await dialog(page).getByRole("button", { name: "OK" }).click();
    await expect(refusal).toHaveText("lines_weights crossing must be a number.");
    await crossing.fill("2.5");
    await dialog(page).getByRole("button", { name: "OK" }).click();
    await expect(refusal).toHaveText("lines_weights crossing must be a whole number.");
    // both 0 is the solver's to refuse: its words are shown at the setting
    await page.evaluate(() => (window.fakeShell.settingErrors = [{ key: "lines_weights", message: "lines_weights must not be [0, 0]." }]));
    await crossing.fill("0");
    await bend.fill("0");
    await dialog(page).getByRole("button", { name: "OK" }).click();
    await expect(refusal).toHaveText("lines_weights must not be [0, 0].");
    await expect(row).toHaveClass(/refused/);
    await page.evaluate(() => (window.fakeShell.settingErrors = []));
    await crossing.fill("2");
    await bend.fill("3");
    // the ranking of the layout is read beside the objectives, and the rank of the objectives is marked
    const ranking = dialog(page).locator(".lines-order");
    await expect(ranking).toBeVisible();
    await ranking.locator("summary").click();
    await expect(ranking.locator("li .objective-name")).toHaveText(["complete", "facts", "annotations", "rules", "current", "crossings_and_bends", "hidden_ports"]);
    await expect(ranking.locator("li.rules")).toHaveCount(1);
    await expect(ranking.locator("li").nth(3)).toHaveClass(/rules/);
    // rows has neither the weights nor the ranking, and keeps what was set
    await field(page, "layout").selectOption("rows");
    await expect(row).toBeHidden();
    await expect(ranking).toBeHidden();
    await expect(asText(page)).toHaveText(`Equivalent CLI command: rnl solve --solver-config 'layout="rows"'`);
    await field(page, "layout").selectOption("lines");
    await expect(crossing).toHaveValue("2");
    await ok(page);
    expect(await viewer.stored("solve-config")).toEqual({ lines_weights: [2, 3] });
    expect(await solved(page, viewer)).toEqual({ config: "lines_weights=[2,3]", timeall: null });
    expect((await viewer.calls("checkSettings")).at(-1)).toEqual(["lines_weights=[2,3]"]);
    // the weights the table gives as the default are no change at all
    await open(page);
    await expect(crossing).toHaveValue("2");
    await expect(bend).toHaveValue("3");
    await crossing.fill("1");
    await bend.fill("4");
    await ok(page);
    expect(await viewer.stored("solve-config")).toEqual({});
  });

  test("the objectives move up and down, and the order is what the solver gets", async ({ page, viewer }) => {
    await open(page);
    await field(page, "layout").selectOption("rows");
    await expect(objective(page, "current_direction").getByRole("button", { name: "Move up: current_direction" })).toBeDisabled();
    await expect(objective(page, "wire_over_symbol").getByRole("button", { name: "Move down: wire_over_symbol" })).toBeDisabled();
    // area, the eighth, goes up past terminal_wire_alignment and symmetry, and keeps the focus
    const up = objective(page, "area").getByRole("button", { name: "Move up: area" });
    await up.click();
    await up.click();
    await expect(up).toBeFocused();
    const moved = [...DEFAULT_ORDER];
    moved.splice(7, 1);
    moved.splice(5, 0, "area");
    expect((await order(page)).slice(0, LEVELS).map(([n]) => n)).toEqual(moved);
    await expect(asText(page)).toContainText(`objectives="${moved.join(",")}"`);
    // an objective that is off can move too: signal_flow goes to the top
    for (let k = 0; k < LEVELS; k++) await objective(page, "signal_flow").getByRole("button", { name: "Move up: signal_flow" }).click();
    expect((await order(page))[0]).toEqual(["signal_flow", false]);
    await ok(page);
    expect(await viewer.stored("solve-config")).toEqual({ layout: "rows", objectives: moved.join(",") });
    expect((await solved(page, viewer)).config).toBe(`layout="rows",objectives="${moved.join(",")}"`);
    // moved back, they are the defaults again, and nothing is kept for them
    await open(page);
    await expect(objective(page, "signal_flow")).toHaveClass(/off/);
    await expect(objective(page, "area").locator(".objective-level")).toHaveText("6");
    await dialog(page).getByRole("button", { name: "Restore Defaults" }).click();
    await ok(page);
    expect(await viewer.stored("solve-config")).toEqual({});
  });

  test("objectives are turned on and off, and the solver gets those that are on", async ({ page, viewer }) => {
    await open(page);
    await field(page, "layout").selectOption("rows");
    await objective(page, "turns").getByLabel("turns is on").uncheck();
    await objective(page, "signal_flow").getByLabel("signal_flow is on").check();
    await expect(objective(page, "turns")).toHaveClass(/off/);
    await expect(objective(page, "turns").locator(".objective-level")).toHaveText("off");
    // the levels count the objectives that are on; signal_flow is where it was, after the others
    await expect(objective(page, "ground_direction").locator(".objective-level")).toHaveText("2");
    await expect(objective(page, "signal_flow").locator(".objective-level")).toHaveText(String(LEVELS));
    const on = DEFAULT_ORDER.filter((n) => n !== "turns").concat("signal_flow");
    await expect(asText(page)).toContainText(`objectives="${on.join(",")}"`);
    await ok(page);
    expect((await solved(page, viewer)).config).toBe(`layout="rows",objectives="${on.join(",")}"`);
    // all of them off is no objective at all: the solver only satisfies the facts
    await open(page);
    for (const name of on) await objective(page, name).getByLabel(`${name} is on`).uncheck();
    await expect(asText(page)).toContainText('objectives="none"');
    await ok(page);
    expect((await solved(page, viewer)).config).toBe('layout="rows",objectives="none"');
  });

  test("objectives share a level with weights, and the level has a tolerance", async ({ page, viewer }) => {
    await open(page);
    await field(page, "layout").selectOption("rows");
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
    const shared = [...DEFAULT_ORDER.slice(0, 8), "crossings+wire_length*2~0.5", ...DEFAULT_ORDER.slice(10)].join(",");
    await expect(asText(page)).toContainText(`objectives="${shared}"`);
    await ok(page);
    expect((await viewer.stored("solve-config")).objectives).toBe(shared);
    // shown again as it was kept
    await open(page);
    await expect(objective(page, "wire_length").getByLabel("with the one above")).toBeChecked();
    await expect(objective(page, "wire_length").getByLabel("weight")).toHaveValue("2");
    await expect(objective(page, "crossings").getByLabel("equal within")).toHaveValue("0.5");
    await objective(page, "wire_length").getByRole("button", { name: "Move up: wire_length" }).click();
    await expect(objective(page, "wire_length").locator(".objective-level")).toHaveText("+");
    await expect(objective(page, "crossings").locator(".objective-level")).toHaveText("9");
    await dialog(page).getByRole("button", { name: "Cancel" }).click();
    await dialogGone(page);
    // Cancel kept the first order
    expect((await viewer.stored("solve-config")).objectives).toContain("crossings+wire_length*2~0.5");
  });

  test("the time for the whole file is beside the settings, not one of them: kept on its own, given to a solve as timeall and not to the check", async ({ page, viewer }) => {
    await open(page);
    const box = dialog(page).locator("fieldset", { hasText: "Time for the whole file" });
    await expect(box).toHaveCount(1);
    // none to begin with: each level has its own limit
    await expect(field(page, "timeall")).toBeDisabled();
    await box.getByLabel("set").check();
    await field(page, "timeall").fill("0");
    await dialog(page).getByRole("button", { name: "OK" }).click();
    await expect(dialog(page).locator(".dialog-error")).toHaveText("timeall must be a positive number of seconds.");
    await field(page, "timeall").fill("90");
    await expect(asText(page)).toHaveText("Equivalent CLI command: rnl solve --timeall 90");
    await field(page, "layout").selectOption("rows");
    await field(page, "beam").fill("8");
    await expect(asText(page)).toHaveText(`Equivalent CLI command: rnl solve --timeall 90 --solver-config 'beam=8,layout="rows"'`);
    await ok(page);
    expect(await viewer.stored("solve-timeall")).toBe(90);
    expect(await viewer.stored("solve-config")).toEqual({ beam: 8, layout: "rows" });
    // the solver is asked about the settings alone (rnl solve --check refuses a time for the file)
    expect((await viewer.calls("checkSettings")).at(-1)).toEqual(['beam=8,layout="rows"']);
    expect(await solved(page, viewer)).toEqual({ config: 'beam=8,layout="rows"', timeall: 90 });
    // what was kept shows again, and Restore Defaults takes it away
    await open(page);
    await expect(field(page, "timeall")).toHaveValue("90");
    await dialog(page).getByRole("button", { name: "Restore Defaults" }).click();
    await expect(field(page, "timeall")).toBeDisabled();
    await ok(page);
    expect(await viewer.stored("solve-timeall")).toBeNull();
    expect(await solved(page, viewer)).toEqual({ config: "", timeall: null });
  });

  test("a value out of the bounds the table gives is refused where it is, and the solver's refusal is shown at its setting", async ({ page, viewer }) => {
    await open(page);
    await field(page, "layout").selectOption("rows");
    await field(page, "beam").fill("");
    await dialog(page).getByRole("button", { name: "OK" }).click();
    await expect(dialog(page).locator(".dialog-error")).toHaveText("beam must be a number.");
    await field(page, "beam").fill("2.5");
    await dialog(page).getByRole("button", { name: "OK" }).click();
    await expect(dialog(page).locator(".dialog-error")).toHaveText("beam must be a whole number.");
    await field(page, "beam").fill("0");
    await dialog(page).getByRole("button", { name: "OK" }).click();
    await expect(dialog(page).locator(".dialog-error")).toHaveText("beam must be at least 1.");
    await field(page, "beam").fill("4");
    // (the spacing is the lines layout's, shown greyed under rows on request: its bounds hold all the same)
    await dialog(page).locator(".settings-other").getByLabel("Show").check();
    await field(page, "spacing_x").fill("21");
    await dialog(page).getByRole("button", { name: "OK" }).click();
    await expect(dialog(page).locator(".dialog-error")).toHaveText("spacing_x must be at most 20.");
    await field(page, "spacing_x").fill("0");
    await field(page, "Positions are not rounded to a grid.").uncheck();
    await field(page, "grid").fill("0");
    await dialog(page).getByRole("button", { name: "OK" }).click();
    await expect(dialog(page).locator(".dialog-error")).toHaveText("grid must be more than 0.");
    await field(page, "grid").fill("1");
    await objective(page, "area").getByLabel("weight").fill("1.5");
    await dialog(page).getByRole("button", { name: "OK" }).click();
    await expect(dialog(page).locator(".dialog-error")).toHaveText("Weight of area must be a whole number.");
    await objective(page, "area").getByLabel("weight").fill("1");
    // nothing reached the solver or was kept while it would not do
    expect(await viewer.calls("checkSettings")).toEqual([]);
    expect(await viewer.stored("solve-config")).toBeNull();
    // what the solver itself says is wrong stays in the dialog, in its words, and marks its setting
    await page.evaluate(() => (window.fakeShell.settingErrors = [{ key: "tolerance", message: "the tolerance is not a number the solver takes" }]));
    await field(page, "tolerance").fill("0.5");
    await dialog(page).getByRole("button", { name: "OK" }).click();
    await expect(dialog(page).locator(".dialog-error")).toHaveText("the tolerance is not a number the solver takes");
    await expect(setting(page, "tolerance")).toHaveClass(/refused/);
    await expect(setting(page, "beam")).not.toHaveClass(/refused/);
    await expect(dialog(page)).toHaveCount(1);
    expect(await viewer.calls("checkSettings")).toEqual([['layout="rows",tolerance=0.5']]);
    expect(await viewer.stored("solve-config")).toBeNull();
    // and the dialog is usable again: a value that will do keeps it
    await page.evaluate(() => (window.fakeShell.settingErrors = []));
    await field(page, "tolerance").fill("0.001");
    await ok(page);
    expect(await viewer.stored("solve-config")).toEqual({ layout: "rows", tolerance: 0.001 });
  });

  test("the settings stay for the next start", async ({ page, viewer }) => {
    await open(page);
    await field(page, "layout").selectOption("rows");
    await field(page, "beam").fill("6");
    await ok(page);
    await page.reload();
    await viewer.started();
    await viewer.idle();
    await open(page);
    await expect(field(page, "beam")).toHaveValue("6");
    await dialog(page).getByRole("button", { name: "Cancel" }).click();
    await dialogGone(page);
    expect(await solved(page, viewer)).toEqual({ config: 'beam=6,layout="rows"', timeall: null });
  });

  test("the dialog says what the netlist and its packages give, from the last solve, which a value set here wins over", async ({ page, viewer }) => {
    await page.evaluate(() => {
      window.fakeShell.levelCheck = {
        settings: {
          layout: { value: "lines", from: "default" },
          spacing_x: { value: 2, from: "header", at: "t4.sp:3" },
          objectives: { value: "turns", from: "package:style" },
          lines_weights: { value: [2, 3], from: "header", at: "t4.sp:4" },
          beam: { value: 8, from: "command-line" },
        },
        warnings: [],
        errors: [],
      };
    });
    // before a solve there is nothing of the netlist to say
    await open(page);
    await expect(dialog(page).locator(".settings-given")).toHaveCount(0);
    await dialog(page).getByRole("button", { name: "Cancel" }).click();
    await dialogGone(page);
    await solved(page, viewer);
    await open(page);
    await expect(dialog(page).locator(".settings-given-list li")).toHaveText([
      "spacing_x = 2 (header, t4.sp:3)",
      'objectives = "turns" (package:style)',
      "lines_weights = [2,3] (header, t4.sp:4)",
    ]);
    // the settings of the default and of the command line are not the netlist's
    await expect(dialog(page).locator(".settings-given")).not.toContainText("layout");
  });

  test("a setting the solver does not take is an error of the solve, told apart from a solve that failed", async ({ page, viewer }) => {
    await page.evaluate(() => {
      window.fakeShell.levelCheck = {
        settings: {},
        warnings: [{ key: "tolerance", message: "ignored" }],
        errors: [{ key: "beam", message: "the beam must be at least 1", refused: 0 }],
      };
    });
    await solved(page, viewer);
    // the solve was shown: a preview, with the report; the status says how many errors the settings have
    await expect(page.locator("#preview-badge")).toBeVisible();
    await expect(page.locator("#solve-step")).toContainText("1 error in the settings: the solve went on without that value.");
    await expect(page.locator("#solve-counts")).toContainText("1 error in the settings");
    // the report lists them under Settings, each with the key it is about
    const group = page.locator("#solve-report .solve-group", { hasText: "Settings (2)" });
    await expect(group.locator(".solve-line .st")).toHaveText(["beam", "tolerance"]);
    await expect(group.locator(".solve-line.error .msg")).toHaveText("the beam must be at least 1");
    await expect(group.locator(".solve-line.warning .msg")).toHaveText("ignored");
    // a solve that failed is another thing: no preview, and a dialog says why
    await page.evaluate(() => (window.fakeShell.solveFail = "nothing was solved"));
    await page.evaluate(() => window.viewer.command("solve-close"));
    await expect(page.locator("#preview-badge")).toBeHidden();
    await expect(page.locator("body")).not.toHaveClass(/busy/);
    await page.evaluate(() => window.viewer.command("solve"));
    await expect(dialog(page)).toContainText("The netlist could not be solved: nothing was solved");
  });

  test("the layout decides what the dialog shows: what others take is hidden, its values are kept, and it is shown greyed on request", async ({ page, viewer }) => {
    await open(page);
    const names = () => dialog(page).locator(".objective:not([hidden])").evaluateAll((rows) => rows.map((r) => r.dataset.name));
    const other = dialog(page).locator(".settings-other");
    // lines, the default: its spacing, and four objectives; the rows' beam and caps and the other objectives are hidden
    await expect(setting(page, "spacing_x")).toBeVisible();
    await expect(setting(page, "beam")).toBeHidden();
    await expect(setting(page, "candidates")).toBeHidden();
    expect(await names()).toEqual(["crowding", "overlaps", "misleading", "wire_over_symbol"]);
    await expect(other).toContainText("2 settings and 14 objectives of other layouts are hidden.");
    // on request they are shown, greyed
    await other.getByLabel("Show").check();
    await expect(setting(page, "beam")).toBeVisible();
    await expect(setting(page, "beam")).toHaveClass(/unused/);
    await expect(objective(page, "turns")).toBeVisible();
    await expect(objective(page, "turns")).toHaveClass(/unused/);
    await expect(other).toContainText("of other layouts are shown greyed.");
    await other.getByLabel("Show").uncheck();
    await expect(setting(page, "beam")).toBeHidden();
    // rows: the spacing is hidden, the beam and the caps are there, and all the objectives
    await field(page, "layout").selectOption("rows");
    await expect(setting(page, "spacing_x")).toBeHidden();
    await expect(setting(page, "spacing_y")).toBeHidden();
    await expect(setting(page, "beam")).toBeVisible();
    await expect(setting(page, "candidates")).toBeVisible();
    expect((await names()).length).toBe(OFF.length + LEVELS);
    await expect(other).toContainText("4 settings of other layouts are hidden.");
    // a value set for one layout is kept while another is shown, and the dialog says so
    await field(page, "beam").fill("9");
    await field(page, "layout").selectOption("lines");
    await expect(setting(page, "beam")).toBeHidden();
    await expect(other).toContainText("1 of the settings keeps a value you set.");
    await field(page, "layout").selectOption("rows");
    await expect(field(page, "beam")).toHaveValue("9");
    await expect(asText(page)).toHaveText(`Equivalent CLI command: rnl solve --solver-config 'beam=9,layout="rows"'`);
    // and under lines it is not what the solver or the command line gets, though it is kept
    await field(page, "layout").selectOption("lines");
    await expect(asText(page)).toHaveText("The defaults of rnl solve.");
    await ok(page);
    expect(await viewer.stored("solve-config")).toEqual({ beam: 9 });
    expect(await solved(page, viewer)).toEqual({ config: "", timeall: null });
    expect((await viewer.calls("checkSettings")).at(-1)).toEqual([""]);
    // rows again: what was kept is there, and now given
    await open(page);
    await field(page, "layout").selectOption("rows");
    await expect(field(page, "beam")).toHaveValue("9");
    await ok(page);
    expect((await solved(page, viewer)).config).toBe('beam=9,layout="rows"');
  });

  test.describe("a tier of objectives that a layout takes whole", () => {
    // lines takes a tier whole that holds one of its four objectives: turns counts with crowding, crossings is no one's
    test.use({ keptSettings: { "solve-config": { objectives: "overlaps,turns+crowding,crossings" } } });

    test("is shown with the objective that is the layout's, and what the solver gets leaves out the tiers that hold none", async ({ page, viewer }) => {
      await open(page);
      const names = () => dialog(page).locator(".objective:not([hidden])").evaluateAll((rows) => rows.map((r) => r.dataset.name));
      expect(await names()).toEqual(["overlaps", "turns", "crowding", "misleading", "wire_over_symbol"]);
      await expect(objective(page, "turns").locator(".objective-tag")).toBeVisible();
      await expect(objective(page, "turns").locator(".objective-tag")).toHaveText("taken with its tier");
      await expect(objective(page, "crowding").locator(".objective-tag")).toBeHidden();
      await expect(objective(page, "overlaps").locator(".objective-level")).toHaveText("1");
      await expect(objective(page, "turns").locator(".objective-level")).toHaveText("2");
      await expect(objective(page, "crowding").locator(".objective-level")).toHaveText("+");
      // crossings is in a tier of its own that lines does not take: hidden, but where the user put it
      await expect(objective(page, "crossings")).toBeHidden();
      await expect(asText(page)).toHaveText(`Equivalent CLI command: rnl solve --solver-config 'objectives="overlaps,turns+crowding"'`);
      await ok(page);
      // kept whole, given without the tier that holds none
      expect(await viewer.stored("solve-config")).toEqual({ objectives: "overlaps,turns+crowding,crossings" });
      expect((await solved(page, viewer)).config).toBe('objectives="overlaps,turns+crowding"');
      expect((await viewer.calls("checkSettings")).at(-1)).toEqual(['objectives="overlaps,turns+crowding"']);
      // rows takes them all, crossings too
      await open(page);
      await field(page, "layout").selectOption("rows");
      await expect(objective(page, "crossings")).toBeVisible();
      await expect(objective(page, "turns").locator(".objective-tag")).toBeHidden();
      await ok(page);
      expect((await solved(page, viewer)).config).toBe('layout="rows",objectives="overlaps,turns+crowding,crossings"');
    });
  });
});
