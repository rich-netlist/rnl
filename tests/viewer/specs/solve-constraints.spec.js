// The Constraints view of the Solve pane (apps/viewer-ui/constraints.js): the constraints the solver takes in, after the
// structures are expanded, one row for each, which the core gives with the Document when a build asks for them
// (`Level.constraints`; the fixture T4-miller-conflicts-constraints is `rnl view --constraints` of the netlist). Both
// platforms list them, with no solve; after a solve the rows say what became of each fact, and a click on a row lights
// up the objects it acts on and the usages it comes from in the drawing.

import { readFileSync } from "node:fs";
import { expect, test } from "../viewer.js";

const DESKTOP = { platform: "desktop" };
const T4 = "T4-miller-conflicts";
const rows = (page) => page.locator("#constraints-list li.constraint");
/** A text of a language's catalog of the viewer, which is what the page shows for the English text in that language. */
const catalog = (lang) => JSON.parse(readFileSync(new URL(`../../../i18n/${lang}/viewer.json`, import.meta.url), "utf8"));
const muted = (page) => page.locator(".drawing svg [data-id].usage-muted");

/** The facts of the fixture's constraints, by level. */
const facts = (page) => page.evaluate(async () => (await fetch("/fixtures/T4-miller-conflicts-constraints.json").then((r) => r.json())).levels.flatMap((l) => l.constraints?.facts ?? []));

/** Open the Solve view, which asks for the constraints, and wait for the rows. */
async function open(page, viewer) {
  await page.keyboard.press("7");
  await expect(page.locator("#pane-solve")).toBeVisible();
  await expect.poll(() => rows(page).count()).toBeGreaterThan(0);
}

/** Solve, and wait for the preview. */
async function solve(page) {
  await page.evaluate(() => window.viewer.command("solve"));
  await page.waitForFunction(() => document.body.classList.contains("preview") && !document.body.classList.contains("solving"));
}

for (const platform of ["web", "desktop"]) {
  test.describe(`on the ${platform}`, () => {
    test.beforeEach(async ({ viewer }) => {
      await viewer.open(T4, { platform });
    });

    test("the Solve view lists the constraints with no solve, after a build that asks the core for them", async ({ page, viewer }) => {
      // no build asks for them before the Solve view is used
      expect(await page.evaluate(() => window.fakeShell.buildConstraints)).toEqual([false]);
      await page.locator("#tab-solve").click();
      await expect.poll(() => rows(page).count()).toBeGreaterThan(0);
      expect(await page.evaluate(() => window.fakeShell.buildConstraints)).toEqual([false, true]);
      const list = await facts(page);
      expect(await rows(page).count()).toBe(list.length);
      await expect(page.locator("#constraints-count")).toHaveText(`${list.length} constraints`);
      await expect(page.locator("#solve-views")).toBeVisible();
      await expect(page.locator("#solve-view-constraints")).toHaveAttribute("aria-pressed", "true");
      // the Report is there only for a solve
      await expect(page.locator("#solve-view-report")).toBeHidden();
      // each row: rank, what it says, priority, and where it comes from; no result before a solve
      await expect(rows(page).first().locator(".rank")).toHaveText("[0]");
      await expect(rows(page).first().locator(".msg")).toContainText(list[0].what);
      await expect(rows(page).first().locator(".pri")).toHaveText(list[0].priority);
      await expect(page.locator("#constraints-list .st")).toHaveCount(0);
      await expect(page.locator("#solve-constraints .constraints-note")).toBeVisible();
      // the netlist is not built again by showing the view again, and a build later asks for the constraints too
      await page.keyboard.press("2");
      await page.keyboard.press("7");
      expect(await page.evaluate(() => window.fakeShell.buildConstraints)).toEqual([false, true]);
      await viewer.build(() => page.keyboard.press("F5"));
      expect((await page.evaluate(() => window.fakeShell.buildConstraints)).at(-1)).toBe(true);
      await expect.poll(() => rows(page).count()).toBe(list.length);
    });

    test("the filter keeps the rows that hold every word, and says how many are left", async ({ page, viewer }) => {
      await open(page, viewer);
      const all = await rows(page).count();
      const word = (await rows(page).first().locator(".pri").innerText()).toLowerCase();
      await page.locator("#constraints-filter").fill(word);
      const left = await page.locator("#constraints-list li.constraint:not([hidden])").count();
      expect(left).toBeGreaterThan(0);
      await expect(page.locator("#constraints-count")).toHaveText(left === all ? `${all} of ${all} constraints` : `${left} of ${all} constraints`);
      await page.locator("#constraints-filter").fill("no such constraint");
      await expect(page.locator("#constraints-count")).toHaveText(`0 of ${all} constraints`);
      await page.locator("#constraints-filter").fill("");
      await expect(page.locator("#constraints-count")).toHaveText(`${all} constraints`);
    });

    test("a click on a row chooses it, a second click ends it, and another row replaces it", async ({ page, viewer }) => {
      await open(page, viewer);
      const row = rows(page).filter({ hasText: /ALIGN y \{#M1 #M2\}/ }).first();
      await row.click();
      await expect(row).toHaveClass(/selected/);
      await expect(row).toHaveAttribute("aria-pressed", "true");
      await row.click();
      await expect(row).not.toHaveClass(/selected/);
      await expect(row).toHaveAttribute("aria-pressed", "false");
      // another row replaces it
      const other = rows(page).nth(1);
      await row.click();
      await other.click();
      await expect(row).not.toHaveClass(/selected/);
      await expect(other).toHaveClass(/selected/);
    });

    test("a selection in the drawing ends what a row lit; Enter and Space on a row do what a click does", async ({ page, viewer }) => {
      await open(page, viewer);
      const row = rows(page).filter({ hasText: /ALIGN y \{#M1 #M2\}/ }).first();
      await row.focus();
      await page.keyboard.press("Enter");
      await expect(row).toHaveClass(/selected/);
      await page.evaluate(async () => {
        const o = window.viewer.doc.objects.findIndex((x) => x.kind === "instance");
        (await import("./selection.js")).select(o, "list", { usage: false });
      });
      await expect(row).not.toHaveClass(/selected/);
      // the row has the focus still: Space chooses it again
      await page.keyboard.press("Space");
      await expect(row).toHaveClass(/selected/);
    });

    test("the place of a row is an arrow button, with no number, that shows the line in the netlist and does not choose the row", async ({ page, viewer }) => {
      await open(page, viewer);
      const row = rows(page).filter({ has: page.locator(".where") }).first();
      const button = row.locator(".where");
      await expect(button).toHaveText("");
      await expect(button).toHaveAttribute("title", "Show in the netlist");
      await expect(button).toHaveAttribute("aria-label", "Show in the netlist");
      await expect(button.locator("svg")).toBeVisible();
      // the line is the one the fact says, by the rank of the row
      const rank = Number((await row.locator(".rank").innerText()).replace(/\D/g, ""));
      const line = (await facts(page))[rank].source.line;
      expect(line).toBeGreaterThan(0);
      await button.click();
      await expect(row).not.toHaveClass(/selected/);
      await expect(page.locator("#pane-netlist")).toBeVisible();
      await expect(viewer.line(line)).toHaveClass(/flash/);
    });

    test("a row with no place has no arrow button", async ({ page, viewer }) => {
      await open(page, viewer);
      const list = await facts(page);
      const arrows = await page.locator("#constraints-list .where").count();
      expect(arrows).toBeGreaterThan(0);
      expect(arrows).toBeLessThanOrEqual(list.filter((f) => f.source).length);
      expect(arrows).toBeLessThan(list.length);
    });
  });
}

test.describe("with a solve, on the desktop", () => {
  test.beforeEach(async ({ viewer }) => {
    await viewer.open(T4, DESKTOP);
  });

  test("after a solve the rows say what became of each fact, and Report shows the report again", async ({ page, viewer }) => {
    await solve(page);
    await expect(page.locator("#solve-view-constraints")).toHaveAttribute("aria-pressed", "true");
    await expect(page.locator("#solve-constraints")).toBeVisible();
    await expect(page.locator("#solve-report")).toBeHidden();
    const report = await page.evaluate(async () => (await fetch("/fixtures/T4-miller-conflicts-preview.json").then((r) => r.json())).report.levels.flatMap((l) => l.facts));
    expect(await rows(page).count()).toBe(report.length);
    const discarded = report.filter((f) => f.status === "discarded").length;
    expect(discarded).toBeGreaterThan(0);
    await expect(page.locator("#constraints-list li.constraint.discarded")).toHaveCount(discarded);
    await expect(page.locator("#constraints-list li.constraint.discarded .st").first()).toHaveText("!");
    await expect(page.locator("#constraints-list li.constraint.discarded .more").first()).toContainText("it conflicts with");
    // a kept fact that the report says holds on the drawing has the check; the solver measures them where it can
    const holding = report.findIndex((f) => f.status === "kept" && f.holds === true);
    expect(holding).toBeGreaterThanOrEqual(0);
    await expect(rows(page).nth(holding).locator(".st")).toHaveText("✓");
    // the report is a button away, and so are the constraints again
    await page.locator("#solve-view-report").click();
    await expect(page.locator("#solve-report")).toBeVisible();
    await expect(page.locator("#solve-constraints")).toBeHidden();
    await page.locator("#solve-view-constraints").click();
    await expect(page.locator("#solve-constraints")).toBeVisible();
  });

  /** Every combination of what became of a fact and whether it holds, and the mark and the tooltip the row has for it. */
  const COMBOS = [
    { status: "kept", holds: true, glyph: "✓", tip: "Kept, and it holds on the drawing.", kind: "holds" },
    { status: "kept", holds: false, glyph: "✗", tip: "Kept, but it does not hold on the drawing: a fault of the solver.", kind: "broken" },
    { status: "kept", holds: null, glyph: "", tip: "Kept; it cannot be measured on the drawing.", kind: "unmeasured" },
    { status: "discarded", holds: true, glyph: "!", tip: "Discarded: it conflicts with other constraints.", kind: "dropped" },
    { status: "discarded", holds: null, glyph: "!", tip: "Discarded: it conflicts with other constraints.", kind: "dropped" },
    { status: "failed", holds: false, glyph: "!", tip: "Failed: the constraint contradicts itself.", kind: "failed" },
    { status: "undecided", holds: true, glyph: "?", tip: "Undecided: the check did not finish. Measured on the drawing, it holds.", kind: "undecided" },
    { status: "undecided", holds: false, glyph: "?", tip: "Undecided: the check did not finish. Measured on the drawing, it does not hold.", kind: "undecided" },
    { status: "undecided", holds: null, glyph: "?", tip: "Undecided: the check did not finish. It cannot be measured on the drawing.", kind: "undecided" },
  ];

  test("the mark of a row is told by its status first, and by whether it holds only when it is kept; the tooltip says it in words", async ({ page, viewer }) => {
    // the report of the fixture, with each fact given one of the combinations
    await page.route("**/fixtures/T4-miller-conflicts-preview.json", async (route) => {
      const preview = await (await route.fetch()).json();
      preview.report.levels[0].facts.forEach((f, k) => Object.assign(f, { status: COMBOS[k % COMBOS.length].status, holds: COMBOS[k % COMBOS.length].holds }));
      await route.fulfill({ json: preview });
    });
    await solve(page);
    const count = await rows(page).count();
    expect(count).toBeGreaterThan(COMBOS.length);
    for (let k = 0; k < count; k++) {
      const want = COMBOS[k % COMBOS.length];
      const mark = rows(page).nth(k).locator(".st");
      await expect(mark, `row ${k}`).toHaveText(want.glyph);
      await expect(mark, `row ${k}`).toHaveClass(new RegExp(`(^| )${want.kind}( |$)`));
      await expect(mark, `row ${k}`).toHaveAttribute("title", want.tip);
      await expect(mark, `row ${k}`).toHaveAttribute("aria-label", want.tip);
    }
    // a kept fact that does not hold is shown as the solver's fault: the colour of an error, and larger than the others
    const broken = rows(page).nth(1).locator(".st");
    const look = (el) => el.evaluate((e) => ({ color: getComputedStyle(e).color, size: parseFloat(getComputedStyle(e).fontSize) }));
    const check = await look(rows(page).nth(0).locator(".st"));
    const cross = await look(broken);
    expect(cross.color).not.toBe(check.color);
    expect(cross.size).toBeGreaterThan(check.size);
    // the two tooltips of `!` say which it is
    expect(COMBOS[3].tip).not.toBe(COMBOS[5].tip);
    // the tooltips are the catalog's in another language, read from it, and a kept fact that cannot be measured has no symbol
    await page.evaluate(() => window.viewer.command("language-zh"));
    const zh = catalog("zh");
    for (const want of COMBOS) {
      expect(zh[want.tip], want.tip).toBeTruthy();
      expect(zh[want.tip]).not.toBe(want.tip);
    }
    for (let k = 0; k < COMBOS.length; k++) {
      await expect(rows(page).nth(k).locator(".st"), `row ${k}`).toHaveAttribute("title", zh[COMBOS[k].tip]);
    }
    // the report's own grouping, by status, is as it was
    await page.evaluate(() => window.viewer.command("language-en"));
    await page.locator("#solve-view-report").click();
    // (the kept group is folded to begin with)
    expect(await page.locator("#solve-report .solve-line.fact.kept").count()).toBeGreaterThan(0);
    await expect(page.locator("#solve-report .solve-line.fact.discarded").first()).toBeVisible();
  });

  test("a click on a row lights up the objects it acts on and the usages it comes from, and dims the rest", async ({ page }) => {
    // the netlist as solved draws every object; as the fixture is read, only M1 is placed
    await solve(page);
    await expect(muted(page)).toHaveCount(0);
    const row = rows(page).filter({ hasText: /ALIGN y \{#M1 #M2\}/ }).first();
    await row.click();
    await expect.poll(() => muted(page).count()).toBeGreaterThan(0);
    const lit = await page.locator(".drawing svg [data-id]:not(.usage-muted)").count();
    expect(lit).toBeGreaterThan(0);
    // the frames of the usages it comes from stay, the others are muted
    expect(await page.locator(".usage-overlays [data-muted]").count()).toBeGreaterThan(0);
    await row.click();
    await expect(muted(page)).toHaveCount(0);
    await expect(page.locator(".usage-overlays [data-muted]")).toHaveCount(0);
  });

  test("Close Preview keeps the constraints, without the results, and a later build still asks for them", async ({ page, viewer }) => {
    await solve(page);
    const count = await rows(page).count();
    await page.evaluate(() => window.viewer.command("solve-close"));
    await expect(page.locator("#preview-badge")).toBeHidden();
    await expect.poll(() => rows(page).count()).toBe(count);
    await expect(page.locator("#constraints-list .st")).toHaveCount(0);
    await expect(page.locator("#solve-view-report")).toBeHidden();
    expect((await page.evaluate(() => window.fakeShell.buildConstraints)).at(-1)).toBe(true);
  });
});
