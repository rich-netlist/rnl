// The objects that have no position (apps/viewer-ui/unplaced.js): a view of the side panel that
// lists those of the level shown, each with its symbol, drawn by the core, and the nets at its
// terminals, and its name; the list loads a part at a time, as it is scrolled, and is searched by
// name; picking an object shows its statement in the netlist; and the statements of these objects
// are greyed in the netlist. The fixture "unplaced" has 60 resistors Rn01 to Rn60, a capacitor C1, a
// transistor M1 and an instance X1 of a subcircuit without a position, and R1, which is placed.
// The Documents of the fake shell do not carry the drawings of these objects, as those of the real
// cores do not: the list asks the shell for the rows it adds (`unplacedDrawings`).

import { expect, test } from "../viewer.js";

const DESKTOP = { platform: "desktop" };
const PART = 20; // the objects the list adds each time

/** The rows of the list. */
const rows = (page) => page.locator("#unplaced-list .unplaced-row");

/** The names in the rows of the list, in order. */
const names = (page) => rows(page).locator(".name").allTextContents();

/** The statement of an instance of the top level: [first, last] line in the first file. */
const statement = (page, name) =>
  page.evaluate((name) => {
    const o = window.viewer.doc.objects.find((x) => x.kind === "instance" && x.name === name && x.level === 0);
    return o.lines;
  }, name);

/** How many objects the top level has without a position. */
const unplacedCount = (page) => page.evaluate(() => window.viewer.doc.levels[0].unplaced.length);

/** Show the list. */
async function show(page) {
  await page.locator("#tab-unplaced").click();
  await expect(page.locator("#pane-unplaced")).toBeVisible();
}

/** The drawings the page has asked the shell for, in order: [level, items], the items being places in the level's list. */
const asked = (viewer) => viewer.calls("unplacedDrawings");

/** The places in the top level's list of the objects with these names. */
const places = (page, wanted) =>
  page.evaluate((wanted) => {
    const names = window.viewer.doc.levels[0].unplaced.map((u) => u.name);
    return wanted.map((name) => names.indexOf(name));
  }, wanted);

/** Scroll the list to its end. */
const toEnd = (page) => page.locator("#unplaced-list").evaluate((el) => (el.scrollTop = el.scrollHeight));

test.describe("the objects not drawn", () => {
  test.beforeEach(async ({ viewer }) => {
    await viewer.open("unplaced");
  });

  test("the tab counts the objects of the level shown, and the list names them in the order of the netlist", async ({ page }) => {
    const n = await unplacedCount(page);
    // the three instances, the 60 resistors, and the grounds of net 0 that nothing places either
    expect(n).toBeGreaterThanOrEqual(63);
    await expect(page.locator("#tab-unplaced")).toHaveText(`Not drawn (${n})`);
    await show(page);
    await expect(page.locator("#unplaced-count")).toHaveText(String(n));
    const shown = await names(page);
    expect(shown.slice(0, 5)).toEqual(["C1", "M1", "X1", "Rn01", "Rn02"]);
    // R1 has a position
    expect(shown).not.toContain("R1");
    await expect(rows(page).first().locator(".symbol")).toContainText("instance");
  });

  test("each object is drawn with its symbol, and the nets at its terminals stand beside them", async ({ page }) => {
    await show(page);
    const nets = (name) =>
      rows(page).filter({ has: page.locator(".name", { hasText: new RegExp(`^${name}$`) }) }).locator(".thumb svg text").allTextContents();
    expect((await nets("M1")).sort()).toEqual(["0", "d", "g", "s"]);
    expect((await nets("C1")).sort()).toEqual(["0", "a"]);
    const thumb = rows(page).first().locator(".thumb svg");
    await expect(thumb).toBeVisible();
    // the symbol itself: lines and curves, in the core's colours, not only the labels
    expect(await thumb.locator("path, rect, circle, ellipse").count()).toBeGreaterThan(0);
    // large enough to see, and within its box
    const box = await thumb.boundingBox();
    expect(box.width).toBeGreaterThan(40);
    expect(box.height).toBeGreaterThan(30);
  });

  test("the list loads a part and the next when it is scrolled to its end", async ({ page }) => {
    const n = await unplacedCount(page);
    await show(page);
    await expect(rows(page)).toHaveCount(PART);
    await expect(page.locator("#unplaced-list .more")).toHaveText(`${PART} of ${n} shown: scroll for more`);
    await toEnd(page);
    await expect(rows(page)).toHaveCount(2 * PART);
    // every part comes in turn, until all are there and the note is gone
    for (let loaded = 2 * PART; loaded < n; ) {
      loaded = Math.min(loaded + PART, n);
      await toEnd(page);
      await expect(rows(page)).toHaveCount(loaded);
    }
    await expect(page.locator("#unplaced-list .more")).toHaveCount(0);
    expect((await names(page)).filter((name) => name.startsWith("Rn")).length).toBe(60);
  });

  test("a search finds objects by name, and the part loaded starts over", async ({ page }) => {
    const n = await unplacedCount(page);
    await show(page);
    await toEnd(page);
    await expect(rows(page)).toHaveCount(2 * PART);
    await page.locator("#unplaced-search").fill("rn5");
    // Rn50 to Rn59
    await expect(rows(page)).toHaveCount(10);
    await expect(page.locator("#unplaced-count")).toHaveText(`10 of ${n}`);
    expect(await names(page)).toEqual(Array.from({ length: 10 }, (_, k) => `Rn${50 + k}`));
    await expect(rows(page).first().locator("mark")).toHaveText(/rn5/i);
    await expect(page.locator("#unplaced-list .more")).toHaveCount(0);
    await page.locator("#unplaced-search").fill("zzz");
    await expect(page.locator("#unplaced-list .nothing")).toHaveText("No object matches.");
    await expect(rows(page)).toHaveCount(0);
    // Escape clears it
    await page.locator("#unplaced-search").press("Escape");
    await expect(page.locator("#unplaced-search")).toHaveValue("");
    await expect(rows(page)).toHaveCount(PART);
    await expect(page.locator("#unplaced-count")).toHaveText(String(n));
  });

  test("picking an object selects it and shows its statement in the netlist", async ({ page, viewer }) => {
    const [file, first, last] = await statement(page, "M1");
    expect(file).toBe(0);
    await show(page);
    await rows(page).filter({ has: page.locator(".name", { hasText: /^M1$/ }) }).click();
    await expect(page.locator("#pane-netlist")).toBeVisible();
    await expect(page.locator("#pane-unplaced")).toBeHidden();
    await expect(viewer.line(first)).toHaveClass(/flash/);
    expect(await viewer.within(viewer.line(first))).toBe(true);
    await viewer.selected("instance M1 in unplaced.sp");
    expect(await viewer.markedLines()).toEqual(Array.from({ length: last - first + 1 }, (_, k) => first + k));
    // the list is as it was left
    await show(page);
    await expect(rows(page)).toHaveCount(PART);
  });

  test("the keyboard picks an object too, and Enter in the search picks the first it finds", async ({ page, viewer }) => {
    await show(page);
    await rows(page).nth(1).focus();
    await page.keyboard.press("Enter");
    await viewer.selected("instance M1 in unplaced.sp");
    await show(page);
    await page.locator("#unplaced-search").fill("rn07");
    await page.locator("#unplaced-search").press("Enter");
    await viewer.selected("instance Rn07 in unplaced.sp");
    const [, line] = await statement(page, "Rn07");
    await expect(viewer.line(line)).toHaveClass(/flash/);
  });

  test("the statements of the objects not drawn are greyed in the netlist, and the others are not", async ({ page, viewer }) => {
    const greyed = (n) => viewer.line(n).evaluate((ln) => ln.classList.contains("not-drawn") && getComputedStyle(ln.querySelector(".code")).opacity !== "1");
    for (const name of ["C1", "M1", "X1", "Rn01", "Rn60"]) {
      const [, line] = await statement(page, name);
      expect(await greyed(line), name).toBe(true);
    }
    // R1, the title, the subcircuit, its end and the records are not
    const [, r1] = await statement(page, "R1");
    expect(await greyed(r1)).toBe(false);
    for (const n of [1, 2, 3, 4]) expect(await greyed(n), `line ${n}`).toBe(false);
    const greyedLines = await page.locator("#source .ln.not-drawn").count();
    expect(greyedLines).toBe(63);
    // the statements say why
    await expect(viewer.line((await statement(page, "C1"))[1])).toHaveAttribute("title", /Not drawn/);
  });

  test("the greyed statements are those of the level shown", async ({ page, viewer }) => {
    // X1's subcircuit has R1 placed, and only its two ports, which no statement writes: going down
    // into it greys none of the statements, and its list is theirs
    await show(page);
    await rows(page).filter({ has: page.locator(".name", { hasText: /^X1$/ }) }).click();
    await viewer.selected("instance X1 in unplaced.sp");
    await page.evaluate(() => window.viewer.command("descend"));
    await expect.poll(() => page.evaluate(() => window.viewer.path.length)).toBe(2);
    await expect(page.locator("#source .ln.not-drawn")).toHaveCount(0);
    await expect(page.locator("#tab-unplaced")).toHaveText("Not drawn (2)");
    await show(page);
    await expect(rows(page)).toHaveCount(2);
    await expect(rows(page).locator(".symbol")).toHaveText([/^usage · /, /^usage · /]);
    await expect(page.locator("#unplaced-search")).toHaveValue("");
    await page.evaluate(() => window.viewer.command("ascend"));
    await expect.poll(() => page.evaluate(() => window.viewer.path.length)).toBe(1);
    await expect(page.locator("#source .ln.not-drawn")).toHaveCount(63);
    await expect(rows(page)).toHaveCount(PART);
  });

  test("the status bar opens the list", async ({ page }) => {
    const n = await unplacedCount(page);
    await expect(page.locator("#status")).toContainText(`${n} without a position`);
    await page.locator("#status").click();
    await expect(page.locator("#pane-unplaced")).toBeVisible();
    await expect(page.locator("#tab-unplaced")).toHaveAttribute("aria-selected", "true");
  });

  test("a document read again lists the objects again, and an empty search", async ({ page, viewer }) => {
    await show(page);
    await page.locator("#unplaced-search").fill("rn1");
    await expect(rows(page)).toHaveCount(10);
    await viewer.build(() => page.evaluate(() => window.viewer.command("reload")));
    await expect(page.locator("#unplaced-search")).toHaveValue("");
    await expect(rows(page)).toHaveCount(PART);
  });
});

test.describe("the drawings of the objects not drawn", () => {
  test.beforeEach(async ({ viewer }) => {
    await viewer.open("unplaced");
  });

  test("the Document carries none, and the list asks for the part it adds, in one request", async ({ page, viewer }) => {
    const carried = await page.evaluate(() => window.viewer.doc.levels[0].unplaced.some((u) => u.svg || u.primitives));
    expect(carried).toBe(false);
    // nothing is asked for while the list is hidden
    expect(await asked(viewer)).toEqual([]);
    await show(page);
    await expect(rows(page).locator(".thumb svg")).toHaveCount(PART);
    const first = Array.from({ length: PART }, (_, k) => k);
    expect(await asked(viewer)).toEqual([[0, first]]);
    // the next part when the list is scrolled to its end
    await toEnd(page);
    await expect(rows(page).locator(".thumb svg")).toHaveCount(2 * PART);
    expect(await asked(viewer)).toEqual([[0, first], [0, first.map((k) => k + PART)]]);
    // each is drawn with its own symbol and nets
    const nets = (name) =>
      rows(page).filter({ has: page.locator(".name", { hasText: new RegExp(`^${name}$`) }) }).locator(".thumb svg text").allTextContents();
    expect((await nets("M1")).sort()).toEqual(["0", "d", "g", "s"]);
    expect((await nets("C1")).sort()).toEqual(["0", "a"]);
  });

  test("a search asks for the rows it brings by their place in the level's list, and not again for those it has", async ({ page, viewer }) => {
    await show(page);
    await expect(rows(page).locator(".thumb svg")).toHaveCount(PART);
    await page.locator("#unplaced-search").fill("rn5");
    await expect(rows(page)).toHaveCount(10);
    await expect(rows(page).locator(".thumb svg")).toHaveCount(10);
    const rn5 = await places(page, Array.from({ length: 10 }, (_, k) => `Rn${50 + k}`));
    expect(rn5[0]).toBeGreaterThan(PART);
    expect((await asked(viewer)).at(-1)).toEqual([0, rn5]);
    // the first part again: drawn as it was, from what came, without another request
    await page.locator("#unplaced-search").press("Escape");
    await expect(rows(page)).toHaveCount(PART);
    await expect(rows(page).locator(".thumb svg")).toHaveCount(PART);
    expect((await asked(viewer)).length).toBe(2);
    // Rn10 to Rn19: Rn18 and Rn19 are the only ones of them not drawn yet
    await page.locator("#unplaced-search").fill("rn1");
    await expect(rows(page)).toHaveCount(10);
    await expect(rows(page).locator(".thumb svg")).toHaveCount(10);
    expect((await asked(viewer)).slice(2)).toEqual([[0, await places(page, ["Rn18", "Rn19"])]]);
  });

  test("a drawing the shell cannot give is told on its thumbnail, and its row still works", async ({ page, viewer }) => {
    await page.evaluate(() => (window.fakeShell.drawingsFail = "the core stopped"));
    await show(page);
    await expect(rows(page)).toHaveCount(PART);
    const thumb = rows(page).first().locator(".thumb");
    await expect(thumb).toHaveAttribute("title", "The drawing could not be made: the core stopped");
    await expect(rows(page).locator(".thumb svg")).toHaveCount(0);
    await rows(page).nth(1).click();
    await viewer.selected("instance M1 in unplaced.sp");
    // the next part is asked for as usual
    await show(page);
    await toEnd(page);
    await expect(rows(page).locator(".thumb svg")).toHaveCount(PART);
    await expect(rows(page).nth(PART).locator(".thumb")).not.toHaveAttribute("title", /./);
  });

  test("drawings that are late come to the rows they were asked for, whatever the list shows by then", async ({ page, viewer }) => {
    await page.evaluate(() => (window.fakeShell.holdDrawings = true));
    await show(page);
    await expect(rows(page)).toHaveCount(PART);
    await expect(rows(page).locator(".thumb svg")).toHaveCount(0);
    // another list, while the first part is awaited: M1 is among that part, and is not asked for again
    await page.locator("#unplaced-search").fill("rn5");
    await expect(rows(page)).toHaveCount(10);
    await page.locator("#unplaced-search").fill("m1");
    await expect(rows(page)).toHaveCount(1);
    await page.evaluate(() => {
      window.fakeShell.holdDrawings = false;
      window.fakeShell.releaseDrawings();
    });
    const m1 = rows(page).first().locator(".thumb svg text");
    await expect(m1).toHaveCount(4);
    expect((await m1.allTextContents()).sort()).toEqual(["0", "d", "g", "s"]);
    expect((await asked(viewer)).length).toBe(2);
    await page.locator("#unplaced-search").fill("rn5");
    await expect(rows(page).locator(".thumb svg")).toHaveCount(10);
  });

  test("a Document that carries the drawings is not asked for them", async ({ page, viewer }) => {
    await page.evaluate(() => (window.fakeShell.carry = true));
    await viewer.build(() => page.evaluate(() => window.viewer.command("reload")));
    expect(await page.evaluate(() => window.viewer.doc.levels[0].unplaced.every((u) => u.svg))).toBe(true);
    await show(page);
    await expect(rows(page).locator(".thumb svg")).toHaveCount(PART);
    expect(await asked(viewer)).toEqual([]);
  });
});

test("a device whose model card is not found is listed with the reason, has no drawing, and none is asked for", async ({ page, viewer }) => {
  await viewer.open("undrawn");
  // M1 has its card and a symbol, but no position; M2, M3 and Q1 cannot be drawn at all
  const listed = await page.evaluate(() => window.viewer.doc.levels[0].unplaced.map((u) => [u.name, u.reason ?? null]));
  expect(listed.filter(([, reason]) => reason).map(([name]) => name)).toEqual(["M2", "M3", "Q1"]);
  expect(listed.find(([name]) => name === "M1")[1]).toBeNull();
  await expect(page.locator("#tab-unplaced")).toHaveText(`Not drawn (${listed.length})`);
  await show(page);
  const row = (name) => rows(page).filter({ has: page.locator(".name", { hasText: new RegExp(`^${name}$`) }) });
  await expect(row("M2").locator(".symbol")).toHaveText("instance · not drawn: model card not found");
  await expect(row("M2")).toHaveClass(/undrawn/);
  await expect(row("M2").locator(".thumb svg")).toHaveCount(0);
  await expect(row("M1").locator(".thumb svg")).toHaveCount(1);
  // only the objects that can be drawn are asked for
  const items = (await asked(viewer)).flatMap(([, wanted]) => wanted);
  const reasons = await page.evaluate((items) => items.map((i) => window.viewer.doc.levels[0].unplaced[i].reason ?? null), items);
  expect(reasons.every((r) => r === null)).toBe(true);
});

test("a level with everything placed has no count and says so", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy");
  await expect(page.locator("#tab-unplaced")).toHaveText("Not drawn");
  await show(page);
  await expect(page.locator("#unplaced-list .nothing")).toHaveText("Every object of this level has a position.");
  await expect(page.locator("#unplaced-count")).toHaveText("");
  await expect(page.locator("#status")).not.toHaveClass(/opens-unplaced/);
});

test("an object written in a record has no statement to show, and picking it does nothing", async ({ page, viewer }) => {
  // the grounds of a solver's netlist stand in no statement
  await viewer.open("T4-miller-conflicts");
  await show(page);
  const usage = rows(page).filter({ hasText: "usage" }).first();
  await expect(usage).toBeVisible();
  await expect(usage).not.toHaveClass(/located/);
  await usage.click();
  await expect(page.locator("#pane-unplaced")).toBeVisible();
  expect((await viewer.state()).selected).toBeNull();
});

test("the desktop viewer has it too, and so has the page without a document open", async ({ page, viewer }) => {
  await viewer.open(null, DESKTOP);
  await show(page);
  await expect(page.locator("#unplaced-list .nothing")).toHaveText("No file is open.");
  await viewer.open("unplaced", DESKTOP);
  await show(page);
  await expect(rows(page)).toHaveCount(PART);
});
