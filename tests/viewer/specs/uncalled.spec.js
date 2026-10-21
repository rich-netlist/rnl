// Subcircuits that nothing calls (apps/viewer-ui/roots.js): the tree lists them below the top level,
// each with the subcircuits it calls; a document whose top level has no objects opens on the first of
// them and says so; and a level with nothing to draw says why, whether its objects have no position
// or it has none.

import { expect, test } from "../viewer.js";

const NOT_PLACED = "Nothing on this level has a position yet, so nothing is drawn.";
const NO_OBJECTS = "This level has no objects, so nothing is drawn.";
const IN_HIERARCHY = "The subcircuits of this netlist are in the Hierarchy.";

/** The segments of the bar shown, each as its level's name and the instance it was reached through. */
function segments(page) {
  return page
    .locator("#level-path .crumb")
    .evaluateAll((els) => els.filter((e) => !e.hidden).map((e) => [...e.children].map((c) => c.textContent).join(" ")));
}

const row = (page, path) => page.locator(`#tree .tree-row[data-path="${path}"]`);

for (const platform of ["web", "desktop"]) {
  test.describe(`on the ${platform} viewer`, () => {
    const query = platform === "desktop" ? { platform } : {};

    test("a cell and its testbench open on the testbench, which nothing calls, and the tree lists both", async ({ page, viewer }) => {
      await viewer.open("uncalled", query);
      // the top level has nothing: the testbench is drawn, and the status bar says why
      await expect(page.locator("#level-name")).toHaveText("Testbench");
      await expect(page.locator("#level-kind")).toHaveText("subcircuit");
      await expect(page.locator("#notice")).toHaveText("The top level is empty; showing Testbench.");
      await expect(page.locator("#empty")).toBeHidden();
      await expect(page.locator("#canvas > svg")).toHaveCount(1);
      expect(await segments(page)).toEqual(["Testbench"]);

      // the tree: the top level, then the group of what nothing calls, the testbench with its instance
      await page.locator("#tab-hierarchy").click();
      await expect(page.locator("#tree .tree-group")).toHaveText("Not called from the top level");
      await expect(page.locator("#tree .tree-row")).toHaveText(["uncalled.sptop", "Testbenchnot called", "XDUTota_5t"]);
      await expect(row(page, "r2")).toHaveClass(/\bactive\b/);
      await expect(row(page, "r2")).toHaveClass(/\bcurrent\b/);
      // the cell is below the testbench, not a root of its own
      await expect(page.locator("#tree .tree-row .badge", { hasText: "not called" })).toHaveCount(1);

      // A tree pick starts a new visit history; only descent through an instance keeps its parent.
      await row(page, "r2/0").click();
      await expect(page.locator("#level-name")).toHaveText("ota_5t");
      expect(await segments(page)).toEqual(["ota_5t"]);
      await expect(page.locator("#notice")).toBeHidden();
      await expect(row(page, "r2/0")).toHaveClass(/\bactive\b/);
      await page.keyboard.press("Backspace");
      await expect(page.locator("#level-name")).toHaveText("ota_5t");
      await row(page, "r2").click();
      await page.evaluate(async () => {
        const { descend } = await import('./subcircuits.js');
        await descend(window.viewer.doc.objects.findIndex(o => o.name === 'XDUT'));
      });
      expect(await segments(page)).toEqual(["Testbench", "ota_5t XDUT"]);
      await page.keyboard.press("Backspace");
      await expect(page.locator("#level-name")).toHaveText("Testbench");

      // the root opens the testbench again; the top level opens, and says it has nothing
      await page.locator("#index .index-row", { hasText: "ota_5t" }).click();
      await expect(page.locator("#level-name")).toHaveText("ota_5t");
      expect(await segments(page)).toEqual(["ota_5t"]);
      await expect(row(page, "r2/0")).toHaveClass(/\bactive\b/);
      await row(page, "0").click();
      await expect(page.locator("#level-name")).toHaveText("uncalled.sp");
      await expect(page.locator("#level-kind")).toHaveText("top level");
      await expect(page.locator("#empty")).toHaveText(`${NO_OBJECTS} ${IN_HIERARCHY}`);
      await expect(page.locator("#notice")).toBeHidden();
      await row(page, "r2").click();
      await expect(page.locator("#level-name")).toHaveText("Testbench");
      await expect(page.locator("#notice")).toBeHidden();
      await expect(page.locator("#index .index-row:not(.top)")).toHaveText(["ota_5t1 use", "Testbench0 uses"]);
    });
  });
}

test("a Spectre netlist of subcircuits alone opens on the one nothing calls", async ({ page, viewer }) => {
  await viewer.open("uncalled-spectre");
  await expect(page.locator("#level-name")).toHaveText("tb");
  await expect(page.locator("#notice")).toHaveText("The top level is empty; showing tb.");
  await page.locator("#tab-hierarchy").click();
  await expect(page.locator("#tree .tree-row")).toHaveText(["uncalled-spectre.scstop", "tbnot called", "XDUTota"]);
  await row(page, "r2/0").click();
  await expect(page.locator("#level-name")).toHaveText("ota");
});

test("a library lists each subcircuit nothing calls, with what it calls, and opens on the first that is drawn", async ({
  page,
  viewer,
}) => {
  await viewer.open("uncalled-lib");
  // lone has no position, stage has: stage is shown
  await expect(page.locator("#level-name")).toHaveText("stage");
  await expect(page.locator("#notice")).toHaveText("The top level is empty; showing stage.");
  await expect(page.locator("#empty")).toBeHidden();
  await page.locator("#tab-hierarchy").click();
  // cell is below stage, and ping calls pong: neither is a root; ping and pong call each other, and ping, the first, starts their tree
  await expect(page.locator("#tree .tree-row")).toHaveText([
    "uncalled-lib.sptop",
    "lonenot called",
    "stagenot called",
    "XLcell",
    "pingnot called",
    "X1pong",
  ]);
  await expect(row(page, "r2")).toHaveClass(/\bactive\b/);
  // the call around the cycle is followed once
  await row(page, "r4/0").locator(".twisty").click();
  await expect(page.locator("#tree .tree-row").last()).toHaveText("X1pingrecursive");

  // a root that has no position shows why nothing is drawn
  await row(page, "r1").click();
  await expect(page.locator("#level-name")).toHaveText("lone");
  await expect(page.locator("#empty")).toHaveText(NOT_PLACED);
  await expect(page.locator("#notice")).toBeHidden();
  // the index shows what the tree does: cell is called once, by stage
  await expect(page.locator("#index .index-row:not(.top)")).toHaveText([
    "cell1 use",
    "lone0 uses",
    "ping1 use",
    "pong1 use",
    "stage0 uses",
  ]);
  await page.locator("#index .index-row", { hasText: "cell" }).click();
  await expect(page.locator("#level-name")).toHaveText("cell");
  await expect(row(page, "r2/0")).toHaveClass(/\bactive\b/);
});

test("with no subcircuit drawn, the first of the roots opens, whatever it shows", async ({ page, viewer }) => {
  await viewer.open("uncalled-unplaced");
  await expect(page.locator("#level-name")).toHaveText("first");
  await expect(page.locator("#notice")).toHaveText("The top level is empty; showing first.");
  await expect(page.locator("#empty")).toHaveText(NOT_PLACED);
});

test("a top level with objects that have no position opens as it did, with the roots listed below it", async ({ page, viewer }) => {
  await viewer.open("uncalled-top");
  await expect(page.locator("#level-name")).toHaveText("uncalled-top.sp");
  await expect(page.locator("#notice")).toBeHidden();
  await expect(page.locator("#empty")).toHaveText(NOT_PLACED);
  await page.locator("#tab-hierarchy").click();
  await expect(page.locator("#tree .tree-row")).toHaveText(["uncalled-top.sptop", "sparenot called"]);
  await row(page, "r1").click();
  await expect(page.locator("#level-name")).toHaveText("spare");
  await expect(page.locator("#empty")).toHaveText(NOT_PLACED);
});

test("a netlist with no object and no subcircuit says only that the level has none", async ({ page, viewer }) => {
  await viewer.open("nothing");
  await expect(page.locator("#level-name")).toHaveText("nothing.sp");
  await expect(page.locator("#empty")).toHaveText(NO_OBJECTS);
  await expect(page.locator("#notice")).toBeHidden();
  await page.locator("#tab-hierarchy").click();
  await expect(page.locator("#tree .tree-row")).toHaveText(["nothing.sptop"]);
  await expect(page.locator("#tree .tree-group")).toHaveCount(0);
});

test("a netlist whose top level is drawn lists no group of roots", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy");
  await page.locator("#tab-hierarchy").click();
  await expect(page.locator("#tree .tree-group")).toHaveCount(0);
  await expect(page.locator("#notice")).toBeHidden();
});

test("reading again keeps the level, and a root's tree folded stays folded", async ({ page, viewer }) => {
  await viewer.open("uncalled");
  await page.locator("#tab-hierarchy").click();
  await page.evaluate(async () => {
    const { descend } = await import('./subcircuits.js');
    await descend(window.viewer.doc.objects.findIndex(o => o.name === 'XDUT'));
  });
  await expect(page.locator("#level-name")).toHaveText("ota_5t");
  await viewer.build(() => page.keyboard.press("F5"));
  await expect(page.locator("#level-name")).toHaveText("ota_5t");
  expect(await segments(page)).toEqual(["Testbench", "ota_5t XDUT"]);
  await expect(row(page, "r2/0")).toHaveClass(/\bactive\b/);
  await expect(page.locator("#notice")).toBeHidden();
  // back at the root, its tree folded stays folded; the level it holds is not a branch of the root
  await page.keyboard.press("Backspace");
  await expect(page.locator("#level-name")).toHaveText("Testbench");
  await row(page, "r2").locator(".twisty").click();
  await expect(page.locator("#tree .tree-row")).toHaveCount(2);
  await viewer.build(() => page.keyboard.press("F5"));
  await expect(page.locator("#tree .tree-row")).toHaveCount(2);
  await expect(page.locator("#level-name")).toHaveText("Testbench");
});
