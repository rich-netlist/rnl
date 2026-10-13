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
  // up goes back along that way
  await page.keyboard.press("Backspace");
  await expect(page.locator("#level-name")).toHaveText(TOP);
  await viewer.selected(`instance X3 in ${TOP}`);

  // the two instances of amp reach the same level, each its own way
  await row("0/1").click();
  await expect(page.locator("#level-name")).toHaveText("amp");
  await expect(row("0/1")).toHaveClass(/\bactive\b/);
  await expect(row("0/0")).toHaveClass(/\bcurrent\b/);
  await expect(row("0/0")).not.toHaveClass(/\bactive\b/);
  await page.locator("#up").click();
  await viewer.selected(`instance X2 in ${TOP}`);

  // the top row's twisty folds and unfolds the tree, and opens nothing
  const twisty = row("0").locator(".twisty");
  await expect(twisty).toHaveAttribute("aria-expanded", "true");
  await twisty.click();
  await expect(rows).toHaveCount(1);
  await expect(twisty).toHaveAttribute("aria-expanded", "false");
  await twisty.click();
  await expect(rows).toHaveCount(4);
  // so does a double click on the row
  await row("0").dblclick();
  await expect(rows).toHaveCount(1);
  await expect(page.locator("#level-name")).toHaveText(TOP);
  // opening a level below a folded branch unfolds it again
  await page.locator("#index .index-row", { hasText: "rdiv" }).click();
  await expect(rows).toHaveCount(4);
  await expect(row("0/2")).toHaveClass(/\bactive\b/);
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

  // Escape empties the box, and leaves the rest as it is
  await page.locator("#up").click();
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
    // The page scrolls the list by the heading's offsetTop, which counts from the page's body, not
    // from the list, so every initial scrolls to the end of the list (hierarchy.js, renderIndex).
    test.fail(true, "an initial scrolls to the end of the index");
    const list = page.locator("#index");
    expect(await list.evaluate((l) => l.scrollHeight > 2 * l.clientHeight)).toBe(true);
    expect(await viewer.within(page.locator("#index .index-row", { hasText: "zero" }), "#index")).toBe(false);
    for (const initial of ["M", "Z", "#", "C"]) {
      await page.locator("#letters button").filter({ hasText: initial }).click();
      for (const name of under(initial)) {
        const row = page.locator("#index .index-row", { hasText: name });
        await expect.poll(() => viewer.within(row, "#index"), { timeout: 2000 }).toBe(true);
      }
    }
  });
});
