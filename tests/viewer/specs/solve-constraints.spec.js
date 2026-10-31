// The Constraints view of the Solve pane (apps/viewer-ui/constraints.js): the constraints the solver takes in, after the
// structures are expanded, one row for each, which the core gives with the Document when a build asks for them
// (`Level.constraints`; the fixture T4-miller-conflicts-constraints is `rnl view --constraints` of the netlist). Both
// platforms list them, with no solve; after a solve the rows say what became of each fact, and a click on a row lights
// up the objects it acts on in the drawing, and boxes them.

import { expect, test } from "../viewer.js";

const DESKTOP = { platform: "desktop" };
const T4 = "T4-miller-conflicts";
const rows = (page) => page.locator("#constraints-list li.constraint");
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

/** The box a chosen constraint has on the drawing: its title, the numbers beside the objects, and the frame. */
const boxTexts = (page) => page.evaluate(() => [...document.querySelectorAll(".constraint-box text")].map((t) => t.textContent));
const frameBox = (page) => page.locator(".constraint-box .constraint-frame");

for (const platform of ["web", "desktop"]) {
  test.describe(`the box of a chosen constraint, on the ${platform}`, () => {
    test.beforeEach(async ({ page, viewer }) => {
      await viewer.open(T4, { platform });
      await open(page, viewer);
    });

    test("a box is drawn around what the constraint acts on, with its title, and the dashed frame of a usage", async ({ page }) => {
      await expect(frameBox(page)).toHaveCount(0);
      const row = rows(page).filter({ hasText: /ALIGN y \{#M1 #M2\}/ }).first();
      await row.click();
      await expect(frameBox(page)).toHaveCount(1);
      const texts = await boxTexts(page);
      expect(texts[0]).toBe("ALIGN y");
      // the numbers follow the order of the members; the fixture as read draws M1 only
      expect(texts.slice(1)).toEqual(["1"]);
      await expect(frameBox(page)).toHaveAttribute("stroke-dasharray", "1 5");
      // it holds the object it acts on
      const held = await page.evaluate(() => {
        const r = document.querySelector(".constraint-box .constraint-frame").getBoundingClientRect();
        const m = window.viewer.doc.levels[window.viewer.level].hits.find((h) => h.kind === "box" && h.objects[0] === window.viewer.doc.objects.findIndex((o) => o.name === "M1" && o.kind === "instance"));
        const drawing = document.querySelector(".constraint-box").ownerSVGElement;
        const svg = drawing.getBoundingClientRect();
        const vb = drawing.viewBox.baseVal;
        const k = svg.width / vb.width;
        const [[x0, y0], [x1, y1]] = m.points;
        const box = [svg.left + (x0 - vb.x) * k, svg.top + (y0 - vb.y) * k, svg.left + (x1 - vb.x) * k, svg.top + (y1 - vb.y) * k];
        return r.left <= box[0] && r.top <= box[1] && r.right >= box[2] && r.bottom >= box[3];
      });
      expect(held).toBe(true);
    });

    test("the box is the one frame at full strength: the usages' frames, the one the constraint comes from too, are dimmed and untitled", async ({ page }) => {
      const frames = page.locator(".usage-overlays .usage-frame");
      const total = await frames.count();
      expect(total).toBeGreaterThan(0);
      await expect(page.locator(".usage-overlays [data-muted]")).toHaveCount(0);
      // a constraint that comes from a usage
      await rows(page).filter({ has: page.locator(".struct:not(:text('written in the netlist'))") }).first().click();
      await expect(page.locator(".usage-overlays [data-muted]")).toHaveCount(total);
      await expect(page.locator(".usage-overlays .usage-title:not([display=none])")).toHaveCount(0);
    });

    test("the box takes no pointer events and is not in the way of a click on the drawing", async ({ page }) => {
      await rows(page).filter({ hasText: /ALIGN y \{#M1 #M2\}/ }).first().click();
      await expect(frameBox(page)).toHaveCount(1);
      expect(await page.evaluate(() => getComputedStyle(document.querySelector(".constraint-box")).pointerEvents)).toBe("none");
      expect(await page.evaluate(() => getComputedStyle(document.querySelector(".constraint-box .constraint-frame")).pointerEvents)).toBe("none");
    });

    test("it goes away with a second click, another row replaces it, and a selection ends it", async ({ page }) => {
      const row = rows(page).filter({ hasText: /ALIGN y \{#M1 #M2\}/ }).first();
      await row.click();
      await expect(frameBox(page)).toHaveCount(1);
      await row.click();
      await expect(frameBox(page)).toHaveCount(0);
      // another row replaces it: one box, with the title of the other
      await row.click();
      const other = rows(page).filter({ hasText: /ORIENT-RELATION MY \{#M1\}/ }).first();
      await other.click();
      await expect(frameBox(page)).toHaveCount(1);
      expect(await boxTexts(page)).toEqual(["ORIENT-RELATION MY", "from1"]);
      // a selection in the drawing
      await row.click();
      await expect(frameBox(page)).toHaveCount(1);
      expect(await boxTexts(page)).toEqual(["ALIGN y", "1"]);
      await expect(frameBox(page)).toHaveCount(1);
      await page.evaluate(async () => {
        const o = window.viewer.doc.objects.findIndex((x) => x.kind === "instance");
        (await import("./selection.js")).select(o, "list", { usage: false });
      });
      await expect(frameBox(page)).toHaveCount(0);
    });

    test("a constraint none of whose objects is on the drawing has no box, and no error", async ({ page }) => {
      const errors = [];
      page.on("pageerror", (e) => errors.push(e.message));
      // the rows of the levels that are not shown have no objects in the drawing
      const all = await rows(page).count();
      for (let k = 0; k < all; k++) {
        await rows(page).nth(k).click();
        await rows(page).nth(k).click();
      }
      expect(errors).toEqual([]);
      await expect(frameBox(page)).toHaveCount(0);
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
    await expect(page.locator("#constraints-list li.constraint.discarded .st").first()).toHaveText("✗");
    await expect(page.locator("#constraints-list li.constraint.discarded .more").first()).toContainText("it conflicts with");
    await expect(page.locator("#constraints-list li.constraint.kept .st").first()).toHaveText("✓");
    // the report is a button away, and so are the constraints again
    await page.locator("#solve-view-report").click();
    await expect(page.locator("#solve-report")).toBeVisible();
    await expect(page.locator("#solve-constraints")).toBeHidden();
    await page.locator("#solve-view-constraints").click();
    await expect(page.locator("#solve-constraints")).toBeVisible();
  });

  test("a click on a row lights up the objects it acts on, boxes them, and dims the rest", async ({ page }) => {
    // the netlist as solved draws every object; as the fixture is read, only M1 is placed
    await solve(page);
    await expect(muted(page)).toHaveCount(0);
    const row = rows(page).filter({ hasText: /ALIGN y \{#M1 #M2\}/ }).first();
    await row.click();
    await expect.poll(() => muted(page).count()).toBeGreaterThan(0);
    const lit = await page.locator(".drawing svg [data-id]:not(.usage-muted)").count();
    expect(lit).toBeGreaterThan(0);
    // the frames of the usages are all muted, the one it comes from too: the box is the frame at full strength
    expect(await page.locator(".usage-overlays [data-muted]").count()).toBeGreaterThan(0);
    await row.click();
    await expect(muted(page)).toHaveCount(0);
    await expect(page.locator(".usage-overlays [data-muted]")).toHaveCount(0);
  });

  test("after a solve every object is on the drawing: the box numbers them in the order the constraint lists them", async ({ page }) => {
    await solve(page);
    await rows(page).filter({ hasText: /ALIGN y \{#M1 #M2\}/ }).first().click();
    await expect(frameBox(page)).toHaveCount(1);
    expect(await boxTexts(page)).toEqual(["ALIGN y", "1", "2"]);
    // a distance has two lists, numbered each from its own start
    await rows(page).filter({ hasText: /DISTANCE x increasing min=3 \{#M1 #M2 #M3 #M4 #M5\} \{#M6 #M7 #CC\}/ }).first().click();
    expect(await boxTexts(page)).toEqual(["DISTANCE x increasing ≥ 3", "from1", "from2", "from3", "from4", "from5", "to1", "to2", "to3"]);
    // a set in a list is one number for each of its objects
    await rows(page).filter({ hasText: /^.*ORDER x increasing \{\{#VIP #VIN\}/ }).first().click();
    expect(await boxTexts(page)).toEqual(["ORDER x increasing", "1", "1", "2", "2", "2", "2", "2", "3", "3", "3", "4"]);
    // the pin of an object is marked at the pin
    await rows(page).filter({ hasText: /DISTANCE y increasing min=2 max=2 \{#VDD:p\} \{%s:p\}/ }).first().click();
    expect((await boxTexts(page))[0]).toBe("DISTANCE y increasing = 2");
    // and the box goes with the preview
    await page.evaluate(() => window.viewer.command("solve-close"));
    await expect(page.locator("#preview-badge")).toBeHidden();
    await expect(frameBox(page)).toHaveCount(0);
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
