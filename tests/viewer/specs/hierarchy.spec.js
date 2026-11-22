// The hierarchy view: the tree of subcircuit instances under the top level, and the index of every
// subcircuit by name, with its search and its initials. Both open levels and mark the one shown.

import { SUBCIRCUITS } from "../fixtures.js";
import { expect, test } from "../viewer.js";

const TOP = "04-hierarchy.sp";

test("the tree lists the instances under the top level and opens their levels", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy");
  await page.locator("#tab-hierarchy").click();
  await expect(page.locator("#pane-hierarchy")).toBeVisible();
  await expect(page.locator("#pane-netlist")).toBeHidden();
  const rows = page.locator("#tree .tree-row");
  const row = (path) => page.locator(`#tree .tree-row[data-path="${path}"]`);
  await expect(rows).toHaveText([`${TOP}top`, "X1amp", "X2amp", "X3rdiv"]);
  await expect(row("0")).toHaveClass(/\bactive\b/);

  // a row opens its level, and is marked as the way there
  await row("0/2").click();
  await expect(page.locator("#level-name")).toHaveText("rdiv");
  await expect(row("0/2")).toHaveClass(/\bactive\b/);
  await expect(row("0/2")).toHaveClass(/\bcurrent\b/);
  await expect(page.locator("#index .index-row.current")).toHaveText("rdiv1 use");
  // A tree pick starts a new visit chain; Backspace has no parent to return to.
  await page.keyboard.press("Backspace");
  await expect(page.locator("#level-name")).toHaveText("rdiv");
  await expect(page.locator("#up")).toBeHidden();
  await row("0").click();
  await expect(page.locator("#level-name")).toHaveText(TOP);

  // Picking either instance directly starts at amp; the tree marks its first use.
  await row("0/1").click();
  await expect(page.locator("#level-name")).toHaveText("amp");
  await expect(row("0/0")).toHaveClass(/\bactive\b/);
  await expect(row("0/0")).toHaveClass(/\bcurrent\b/);
  await expect(page.locator("#up")).toBeHidden();
  await row("0").click();

  // the top row's twisty folds and unfolds the tree, and opens nothing
  const twisty = row("0").locator(".twisty");
  await expect(twisty).toHaveAttribute("aria-expanded", "true");
  await twisty.click();
  await expect(rows).toHaveCount(1);
  await expect(twisty).toHaveAttribute("aria-expanded", "false");
  await twisty.click();
  await expect(rows).toHaveCount(4);
  await expect(page.locator("#level-name")).toHaveText(TOP);
  // opening a level below a folded branch unfolds it again
  await twisty.click();
  await expect(rows).toHaveCount(1);
  await page.locator("#index .index-row", { hasText: "rdiv" }).click();
  await expect(rows).toHaveCount(4);
  await expect(row("0/2")).toHaveClass(/\bactive\b/);
});

test("a double click on the top row folds and unfolds the tree", async ({ page, viewer }) => {
  // The click on a row that is marked already does not draw the tree again, so the second click of
  // the double click presses the same row, and the dblclick comes to it: Firefox sends none to a row
  // out of the page (hierarchy.js, openLevel).
  await viewer.open("04-hierarchy");
  await page.locator("#tab-hierarchy").click();
  const rows = page.locator("#tree .tree-row");
  await expect(rows).toHaveCount(4);
  await rows.first().dblclick();
  await expect(rows).toHaveCount(1);
  await expect(page.locator("#level-name")).toHaveText(TOP);
  await rows.first().dblclick();
  await expect(rows).toHaveCount(4);
});

test("the index lists the subcircuits under their initials and opens them", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy");
  await page.locator("#tab-hierarchy").click();
  await expect(page.locator("#index-count")).toHaveText("2");
  await expect(page.locator("#index > *")).toHaveText([`${TOP}top level`, "A", "amp2 uses", "R", "rdiv1 use"]);
  await expect(page.locator("#letters button")).toHaveText(["A", "R"]);
  await expect(page.locator("#index .index-row.top")).toHaveAttribute("aria-selected", "true");

  await page.locator("#index .index-row", { hasText: "amp" }).click();
  await expect(page.locator("#level-name")).toHaveText("amp");
  await expect(page.locator("#index .index-row.current")).toHaveText("amp2 uses");
  // the tree marks the first way to it
  await expect(page.locator("#tree .tree-row.active")).toHaveAttribute("data-path", "0/0");
  // the top level's row goes back to the top
  await page.locator("#index .index-row.top").click();
  await expect(page.locator("#level-name")).toHaveText(TOP);
  await expect(page.locator("#tree .tree-row.active")).toHaveAttribute("data-path", "0");
});

test("the search narrows the index; Enter opens the first match and Escape empties it", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy");
  await viewer.click("X2");
  await page.locator("#tab-hierarchy").click();
  const search = page.locator("#search");
  await search.fill("DI");
  await expect(page.locator("#index-count")).toHaveText("1 of 2");
  await expect(page.locator("#index > *")).toHaveText([`${TOP}top level`, "R", "rdiv1 use"]);
  await expect(page.locator("#index mark")).toHaveText("di");
  await expect(page.locator("#letters button")).toHaveText(["R"]);
  await search.press("Enter");
  await expect(page.locator("#level-name")).toHaveText("rdiv");

  await search.fill("zz");
  await expect(page.locator("#index-count")).toHaveText("0 of 2");
  await expect(page.locator("#index .nothing")).toHaveText("No subcircuit matches.");
  await search.press("Enter");
  await expect(page.locator("#level-name")).toHaveText("rdiv");

  // a subcircuit opened from the index is the root of its path: no way up from it
  await expect(page.locator("#up")).toBeHidden();
  // Escape empties the box, and leaves the rest as it is
  await page.locator("#index .index-row.top").click();
  await viewer.click("X3");
  await viewer.selected(`instance X3 in ${TOP}`);
  await search.fill("a");
  await search.press("Escape");
  await expect(search).toHaveValue("");
  await expect(page.locator("#index-count")).toHaveText("2");
  await viewer.selected(`instance X3 in ${TOP}`);
  // and in an empty box it does nothing either
  await search.press("Escape");
  await viewer.selected(`instance X3 in ${TOP}`);
});

test.describe("with a subcircuit for each initial", () => {
  /** The subcircuits under an initial, "#" for those that start with no letter. */
  const under = (initial) => SUBCIRCUITS.filter((n) => (/[a-z]/.test(n[0]) ? n[0].toUpperCase() : "#") === initial);

  test.beforeEach(async ({ page, viewer }) => {
    await viewer.open("subcircuits");
    await page.locator("#tab-hierarchy").click();
  });

  test("the index groups them under their initials, and opens them", async ({ page }) => {
    const initials = [...new Set(SUBCIRCUITS.map((n) => (/[a-z]/.test(n[0]) ? n[0].toUpperCase() : "#")))].sort();
    await expect(page.locator("#letters button")).toHaveText(initials);
    await expect(page.locator("#index-count")).toHaveText(String(SUBCIRCUITS.length));
    await expect(page.locator("#index .index-group")).toHaveText(initials);
    await expect(page.locator("#initial-M ~ .index-row").first()).toHaveText("mixer1 use");
    // a subcircuit's row opens it, whatever is drawn there
    await page.locator("#index .index-row", { hasText: "clock" }).click();
    await expect(page.locator("#level-name")).toHaveText("clock");
    await expect(page.locator("#empty")).toHaveText("Nothing on this level has a position yet, so nothing is drawn.");
    await expect(page.locator("#tree .tree-row.active")).toHaveText("X3clock");
  });

  test("an initial scrolls the index to its subcircuits", async ({ page, viewer }) => {
    // The headings are sticky: once the list has scrolled, a heading that sticks at its top is not
    // where its group starts, so the page scrolls by the group's first row (hierarchy.js,
    // scrollToGroup). Going back up to an initial scrolls too. No heading covers the rows shown.
    const list = page.locator("#index");
    /** Whether a click in the middle of a row hits it. */
    const uncovered = (row) =>
      row.evaluate((e) => {
        const r = e.getBoundingClientRect();
        return e.contains(document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2));
      });
    expect(await list.evaluate((l) => l.scrollHeight > 2 * l.clientHeight)).toBe(true);
    expect(await viewer.within(page.locator("#index .index-row", { hasText: "zero" }), "#index")).toBe(false);
    for (const initial of ["M", "Z", "#", "C"]) {
      await page.locator("#letters button").filter({ hasText: initial }).click();
      for (const name of under(initial)) {
        const row = page.locator("#index .index-row", { hasText: name });
        await expect.poll(() => viewer.within(row, "#index"), { timeout: 2000 }).toBe(true);
        expect(await uncovered(row)).toBe(true);
      }
    }
  });
});

/** Show the netlist of 04-hierarchy with `extra` more instances of rdiv on its top level, after X3, its own. */
async function manyInstances(page, viewer, extra) {
  await viewer.open("04-hierarchy");
  await page.evaluate(async (n) => {
    const { doc } = await import("./state.js");
    const rdiv = doc.levels[0].children.find((c) => c.master === "rdiv").level;
    const more = Array.from({ length: n }, (_, i) => ({ instance: `XD${i}`, master: "rdiv", level: rdiv, params: [] }));
    const levels = doc.levels.map((l, k) => (k === 0 ? { ...l, children: [...l.children, ...more] } : l));
    (await import("./open.js")).showBuilt({ ...doc, levels }, false);
  }, extra);
}

test("many instances of one subcircuit are one row, which lists them a batch at a time", async ({ page, viewer }) => {
  await manyInstances(page, viewer, 450);
  await page.locator("#tab-hierarchy").click();
  const rows = page.locator("#tree .tree-row");
  // X3 and the 450 others call rdiv: one row, where X3 was, with their number
  await expect(rows).toHaveText([`${TOP}top`, "X1amp", "X2amp", "451×rdiv"]);
  const group = page.locator('#tree .tree-row[data-path="0/g2"]');
  await group.click();
  await expect(group).toHaveAttribute("aria-expanded", "true");
  await expect(rows).toHaveCount(4 + 200);
  await expect(rows.nth(4)).toHaveText("X3rdiv");
  await expect(rows.nth(5)).toHaveAttribute("data-path", "0/3");
  await expect(page.locator("#tree .tree-more button")).toHaveText("200 of 451 shown: scroll for more");
  // a click lists 200 more in its place (pressed where it is: the scroll of a click would bring it into view); brought
  // into view, the line lists the rest
  await page.locator("#tree .tree-more button").evaluate((b) => b.click());
  await expect(rows).toHaveCount(4 + 400);
  await expect(rows.nth(204)).toHaveAttribute("data-path", "0/202");
  await page.locator("#tree .tree-more").scrollIntoViewIfNeeded();
  await expect(rows).toHaveCount(4 + 451);
  await expect(page.locator("#tree .tree-more")).toHaveCount(0);
  // an instance opens its level, as any row does; the group row closes and opens the group alone
  await page.locator('#tree .tree-row[data-path="0/300"]').click();
  await expect(page.locator("#level-name")).toHaveText("rdiv");
  await group.click();
  await expect(rows).toHaveText([`${TOP}top`, "X1amp", "X2amp", "451×rdiv"]);
});

test("the row of the level shown is listed in its group, and the tree is drawn only while it is shown", async ({ page, viewer }) => {
  await manyInstances(page, viewer, 450);
  // XD399, the 401st instance of the group, entered from the top level: its group opens, listing as far as it
  await page.evaluate(async () => {
    const { doc, path } = await import("./state.js");
    const level = doc.levels[0].children.find((c) => c.instance === "XD399").level;
    (await import("./navigation.js")).navigate({ path: [path[0], { level, via: "XD399" }], selection: null, reveal: true });
  });
  await expect(page.locator("#level-name")).toHaveText("rdiv");
  await expect(page.locator("#tree .tree-row")).toHaveCount(0);
  await page.locator("#tab-hierarchy").click();
  const active = page.locator("#tree .tree-row.active");
  await expect(active).toHaveAttribute("data-path", "0/402");
  await expect(active).toHaveText("XD399rdiv");
  await expect(page.locator('#tree .tree-row[data-path="0/g2"]')).toHaveAttribute("aria-expanded", "true");
  // (the rows up to it are listed: brought into view below them, the line of the rest lists those too)
  expect(await page.locator("#tree .tree-row").count()).toBeGreaterThanOrEqual(4 + 400);
  // hidden, the hierarchy is not drawn as another level is shown: it is emptied, and drawn again when shown
  await page.locator("#tab-netlist").click();
  await page.evaluate(async () => (await import("./subcircuits.js")).ascend());
  await expect(page.locator("#level-name")).toHaveText(TOP);
  await expect(page.locator("#tree .tree-row")).toHaveCount(0);
  await page.locator("#tab-hierarchy").click();
  await expect(active).toHaveAttribute("data-path", "0");
});
