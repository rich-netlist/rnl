// The text while a subcircuit is drawn (text.js): what lies outside its definition, from its header
// to its end, is dimmed, in its file and in the others; entering the subcircuit, through an
// instance, the index or the tree, brings the start of its definition to the top of the text; going
// back up shows the statement of the instance it was left through; and a name picked in the text
// leaves the text where it is. In 04-hierarchy.sp, amp is defined on lines 8 to 48; rdiv on lines 2
// to 19 of the include, 04-hierarchy-lib.inc.

import { expect, test } from "../viewer.js";

/** Whether a line is dimmed: its text faded, on a grey background unlike the pane's. */
const DIMMED = (ln) => {
  const pane = getComputedStyle(document.getElementById("source")).backgroundColor;
  return getComputedStyle(ln.querySelector(".code")).opacity !== "1" && getComputedStyle(ln).backgroundColor !== pane;
};

/** Per file, the dimmed lines as ranges [first, last]. */
function dimmed(page) {
  return page.evaluate((test) => {
    const isDimmed = new Function(`return (${test})`)();
    return [...document.querySelectorAll("#source .file")].map((file) => {
      const ranges = [];
      for (const ln of file.querySelectorAll(".ln")) {
        if (!isDimmed(ln)) continue;
        const n = Number(ln.dataset.line);
        const last = ranges[ranges.length - 1];
        if (last && last[1] === n - 1) last[1] = n;
        else ranges.push([n, n]);
      }
      return ranges;
    });
  }, DIMMED.toString());
}

/** The file shown in the text, and its first line whose top is in view: the line at the top. */
function top(page) {
  return page.evaluate(() => {
    const source = document.getElementById("source");
    const edge = source.getBoundingClientRect().top;
    const lines = [...source.querySelectorAll(".file:not([hidden]) .ln")];
    const first = lines.find((l) => l.getBoundingClientRect().top >= edge - 1);
    return {
      file: document.querySelector("#file-tabs button[aria-selected=true]").textContent,
      line: first ? Number(first.dataset.line) : null,
      gap: first ? Math.round(first.getBoundingClientRect().top - edge) : null,
    };
  });
}

/** Go down into a subcircuit instance of the level shown: select it on the drawing, and press Enter. */
async function down(page, viewer, name) {
  await viewer.click(name);
  await page.keyboard.press("Enter");
  await expect.poll(() => page.evaluate(() => window.viewer.path.length)).toBeGreaterThan(1);
}

test.beforeEach(async ({ viewer }) => {
  await viewer.open("04-hierarchy");
});

test("while a subcircuit is drawn, the text outside its definition is dimmed", async ({ page, viewer }) => {
  // at the top level nothing is
  expect(await dimmed(page)).toEqual([[], []]);
  await down(page, viewer, "X1");
  expect((await viewer.state()).title).toBe("amp");
  await expect(page.locator("#source")).toHaveClass(/\bscoped\b/);
  expect(await dimmed(page)).toEqual([[[1, 7], [49, 78]], [[1, 19]]]);
  // the definition's lines are as at the top level
  expect(await viewer.line(11).evaluate((e) => getComputedStyle(e.querySelector(".code")).opacity)).toBe("1");
  // reading the file again keeps it so
  await viewer.build(() => page.keyboard.press("F5"));
  expect(await dimmed(page)).toEqual([[[1, 7], [49, 78]], [[1, 19]]]);
  // back up, nothing is
  await page.keyboard.press("Backspace");
  await expect.poll(async () => (await viewer.state()).title).toBe("04-hierarchy.sp");
  await expect(page.locator("#source")).not.toHaveClass(/\bscoped\b/);
  expect(await dimmed(page)).toEqual([[], []]);
});

test("going down into an instance brings the start of its subcircuit's definition to the top", async ({ page, viewer }) => {
  await down(page, viewer, "X2");
  expect(await top(page)).toEqual({ file: "04-hierarchy.sp", line: 8, gap: 4 });
});

test("going back up shows the statement of the instance the subcircuit was left through", async ({ page, viewer }) => {
  await down(page, viewer, "X1");
  expect(await viewer.within(viewer.line(49))).toBe(false);
  await page.keyboard.press("Backspace");
  await expect.poll(async () => (await viewer.state()).selected).toBe("instance X1 in 04-hierarchy.sp");
  expect(await viewer.within(viewer.line(49))).toBe(true);
});

test("a subcircuit opened from the index or the tree shows its definition, in its file", async ({ page, viewer }) => {
  await page.locator("#tab-hierarchy").click();
  await page.locator("#index .index-row", { hasText: "rdiv" }).click();
  await expect(page.locator("#level-name")).toHaveText("rdiv");
  expect(await dimmed(page)).toEqual([[[1, 78]], [[1, 1]]]);
  // the text, hidden behind the hierarchy, shows the definition once it is shown again
  await page.locator("#tab-netlist").click();
  await expect(page.locator("#file-tabs button[aria-selected=true]")).toHaveText("04-hierarchy-lib.inc");
  expect(await viewer.within(viewer.line(2))).toBe(true);
  // the tree: X2 of the top level, which opens amp
  await page.locator("#tab-hierarchy").click();
  await page.locator('#tree .tree-row[data-path="0/1"]').click();
  await expect(page.locator("#level-name")).toHaveText("amp");
  await page.locator("#tab-netlist").click();
  expect(await top(page)).toEqual({ file: "04-hierarchy.sp", line: 8, gap: 4 });
});

test("a name picked in the text shows its level, and leaves the text where it is", async ({ page, viewer }) => {
  // the text scrolled down at the top level, to the statements of amp's placements
  await viewer.line(43).evaluate((e) => {
    document.getElementById("source").scrollTop = e.offsetTop - 40;
  });
  const scroll = await page.locator("#source").evaluate((e) => e.scrollTop);
  await viewer.line(43).locator("[data-obj]").first().click();
  await expect(page.locator("#level-name")).toHaveText("amp");
  expect((await viewer.state()).selected).toBe("instance M1 in amp");
  await expect(page.locator("#source")).toHaveClass(/\bscoped\b/);
  expect(await page.locator("#source").evaluate((e) => e.scrollTop)).toBe(scroll);
});
