// The hierarchy path in the bar above the drawing, from its root to the level shown: going down
// into an instance adds it; a segment, the up button and Backspace go back along it; the index makes
// a subcircuit the root and the tree gives the way from the top level; a level reached otherwise cuts
// the path back to it or starts it there; reading again keeps it while its instances are there; the
// views are kept by path; and a long path leaves its middle behind an ellipsis in a narrow bar.

import { expect, test } from "../viewer.js";

const TOP = "nested.sp";

/** The segments of the bar shown, each as its level's name and the instance it was reached through. */
function segments(page) {
  return page
    .locator("#level-path .crumb")
    .evaluateAll((els) => els.filter((e) => !e.hidden).map((e) => [...e.children].map((c) => c.textContent).join(" ")));
}

/** The path, by the debug handle, in the same form, also what the bar leaves out. */
function path(page) {
  return page.evaluate(() => {
    const v = window.viewer;
    return v.path.map((s) => [v.doc.levels[s.level].title, s.via].filter((x) => x !== null).join(" "));
  });
}

/** Go down into a subcircuit instance of the level shown: select it on the drawing, and press Enter. */
async function down(page, viewer, name) {
  const length = () => page.evaluate(() => window.viewer.path.length);
  const before = await length();
  await viewer.click(name);
  await page.keyboard.press("Enter");
  await expect.poll(length).toBe(before + 1);
}

test.beforeEach(async ({ viewer }) => {
  await viewer.open("nested");
});

test("going down adds each instance to the path; a segment names its subcircuit, the instance beside it", async ({ page, viewer }) => {
  expect(await segments(page)).toEqual([TOP]);
  await expect(page.locator("#level-path .crumb.current")).toHaveAttribute("aria-current", "location");
  await expect(page.locator("#level-kind")).toHaveText("top level");
  await expect(page.locator("#up")).toBeHidden();

  // by Enter, a double click, the open badge, and View > Open Subcircuit
  await down(page, viewer, "XK1");
  await expect(page.locator("#level-name")).toHaveText("bank");
  expect(await segments(page)).toEqual([TOP, "bank XK1"]);
  const xb2 = await viewer.where("XB2");
  await page.mouse.dblclick(xb2.x, xb2.y);
  await expect(page.locator("#level-name")).toHaveText("block");
  await viewer.click("XC1");
  await page.locator("#enter-badge").click();
  await expect(page.locator("#level-name")).toHaveText("cell");
  await viewer.click("XL2");
  await viewer.menuTitle("Edit").click();
  await viewer.menuItem("descend").click();
  await expect(page.locator("#level-name")).toHaveText("leaf");
  expect(await segments(page)).toEqual([TOP, "bank XK1", "block XB2", "cell XC1", "leaf XL2"]);
  expect(await path(page)).toEqual(await segments(page));

  // all but the last go back; the instance is smaller and dimmer than the name
  await expect(page.locator("#level-path button.crumb")).toHaveCount(4);
  await expect(page.locator("#level-path .crumb.current")).toHaveText("leafXL2");
  const looks = await page.locator("#level-path .crumb.current").evaluate((c) => {
    const style = (e) => getComputedStyle(e);
    const [name, via] = [style(c.querySelector(".level-name")), style(c.querySelector(".via"))];
    return { sizes: [parseFloat(name.fontSize), parseFloat(via.fontSize)], colours: [name.color, via.color] };
  });
  expect(looks.sizes[1]).toBeLessThan(looks.sizes[0]);
  expect(looks.colours[1]).not.toBe(looks.colours[0]);
  await expect(page.locator("#up-name")).toHaveText("cell");
  await expect(page.locator("#level-kind")).toHaveText("subcircuit");
  // the bar is wide enough for all of it
  await expect(page.locator("#level-path .crumb-more")).toBeHidden();
  await expect(page.locator("#level-kind")).toBeVisible();
});

test("a segment goes back to its level with its view and the instance left selected; up and Backspace go a step", async ({ page, viewer }) => {
  await down(page, viewer, "XK1");
  await page.keyboard.press("Control+-"); // a view of bank that fitting would not give
  const bank = await viewer.viewBox();
  await down(page, viewer, "XB2");
  await down(page, viewer, "XC1");
  await down(page, viewer, "XL2");
  expect(await segments(page)).toEqual([TOP, "bank XK1", "block XB2", "cell XC1", "leaf XL2"]);

  await page.locator("#level-path button.crumb", { hasText: "bank" }).click();
  await expect(page.locator("#level-name")).toHaveText("bank");
  expect(await segments(page)).toEqual([TOP, "bank XK1"]);
  expect(await viewer.viewBox()).toBe(bank);
  await viewer.selected("instance XB2 in bank");
  await expect(page.locator("#up-name")).toHaveText(TOP);

  // the up button goes a step, to the root, where there is no up button and Backspace does nothing
  await page.locator("#up").click();
  await expect(page.locator("#level-name")).toHaveText(TOP);
  await viewer.selected(`instance XK1 in ${TOP}`);
  await expect(page.locator("#up")).toBeHidden();
  await page.keyboard.press("Backspace");
  expect(await segments(page)).toEqual([TOP]);
  await viewer.selected(`instance XK1 in ${TOP}`);

  // Backspace goes a step at a time
  await page.keyboard.press("Enter");
  await down(page, viewer, "XB1");
  await down(page, viewer, "XC1");
  expect(await segments(page)).toEqual([TOP, "bank XK1", "block XB1", "cell XC1"]);
  await page.keyboard.press("Backspace");
  expect(await segments(page)).toEqual([TOP, "bank XK1", "block XB1"]);
  await viewer.selected("instance XC1 in block");
  await page.keyboard.press("Backspace");
  expect(await segments(page)).toEqual([TOP, "bank XK1"]);
  await viewer.selected("instance XB1 in bank");
  // the root's segment goes back to the root
  await page.keyboard.press("Enter");
  await page.locator("#level-path button.crumb", { hasText: TOP }).click();
  expect(await segments(page)).toEqual([TOP]);
  await viewer.selected(`instance XK1 in ${TOP}`);
});

test("the up button that hides at the root leaves the focus on the drawing, so that Enter goes back down at once", async ({ page, viewer }) => {
  // WebKit moves the focus off a hidden element only some milliseconds later, and on Linux a click
  // gives a button the focus: an Enter in between went to the hidden button, which the page leaves
  // Enter to, and did nothing. Pressed and followed by Enter in one go, as a keyboard can:
  await down(page, viewer, "XK1");
  await page.locator("#up").focus();
  const after = await page.evaluate(() => {
    document.getElementById("up").click();
    const focused = document.activeElement;
    focused.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
    return { focused: focused.id, path: window.viewer.path.length };
  });
  expect(after).toEqual({ focused: "canvas", path: 2 });
  await expect(page.locator("#level-name")).toHaveText("bank");
  // and from the keyboard alone
  await page.locator("#up").focus();
  await page.keyboard.press("Enter");
  await page.keyboard.press("Enter");
  await expect(page.locator("#level-name")).toHaveText("bank");
  expect(await segments(page)).toEqual([TOP, "bank XK1"]);
});

test("a subcircuit picked in the index is the root, with no up button; going down from it extends the path", async ({ page, viewer }) => {
  await page.locator("#tab-hierarchy").click();
  await page.locator("#index .index-row", { hasText: "block" }).click();
  await expect(page.locator("#level-name")).toHaveText("block");
  expect(await segments(page)).toEqual(["block"]);
  await expect(page.locator("#level-kind")).toHaveText("subcircuit");
  await expect(page.locator("#up")).toBeHidden();
  await page.keyboard.press("Backspace");
  expect(await segments(page)).toEqual(["block"]);
  // the tree marks the first row that reaches it, and the index the level shown
  await expect(page.locator("#tree .tree-row.active")).toHaveAttribute("data-path", "0/0/0");
  await expect(page.locator("#index .index-row.current")).toHaveText(/^block/);

  await down(page, viewer, "XC1");
  expect(await segments(page)).toEqual(["block", "cell XC1"]);
  await expect(page.locator("#up-name")).toHaveText("block");
  await page.keyboard.press("Backspace");
  expect(await segments(page)).toEqual(["block"]);
  await viewer.selected("instance XC1 in block");

  // the index's top row makes the top level the root
  await page.locator("#index .index-row.top").click();
  expect(await segments(page)).toEqual([TOP]);
  // and the level shown, picked in the index, becomes the root, its view as it was
  await down(page, viewer, "XK1");
  await page.keyboard.press("Control+-");
  const view = await viewer.viewBox();
  await page.locator("#index .index-row", { hasText: "bank" }).click();
  expect(await segments(page)).toEqual(["bank"]);
  expect(await viewer.viewBox()).toBe(view);
  await expect(page.locator("#up")).toBeHidden();
});

test("a tree pick starts a new history head regardless of the structural ancestors", async ({ page }) => {
  await page.locator("#tab-hierarchy").click();
  const row = (p) => page.locator(`#tree .tree-row[data-path="${p}"]`);
  await row("0/0").locator(".twisty").click();
  await row("0/0/1").locator(".twisty").click();
  await row("0/0/1/0").click();
  await expect(page.locator("#level-name")).toHaveText("cell");
  expect(await segments(page)).toEqual(["cell"]);
  await expect(row("0/0/0/0")).toHaveClass(/\bactive\b/);
  await expect(page.locator("#up")).toBeHidden();

  // the other way to cell
  await row("0/1").click();
  expect(await segments(page)).toEqual(["cell"]);
  await expect(row("0/0/1/0")).toHaveClass(/\bcurrent\b/);
  await expect(row("0/0/1/0")).not.toHaveClass(/\bactive\b/);
  await page.keyboard.press("Backspace");
  expect(await segments(page)).toEqual(["cell"]);
});

test("an object of another level selected in the text starts a new history head", async ({ page, viewer }) => {
  await down(page, viewer, "XK1");
  await down(page, viewer, "XB2");
  await down(page, viewer, "XC1");
  expect(await segments(page)).toEqual([TOP, "bank XK1", "block XB2", "cell XC1"]);

  // XB1's statement, on bank, which is on the path
  await viewer.line(14).locator(".no").click();
  await viewer.selected("instance XB1 in bank");
  expect(await segments(page)).toEqual(["bank"]);
  // R1's, on leaf, which is not: leaf is the root
  await viewer.line(3).locator(".no").click();
  await viewer.selected("instance R1 in leaf");
  expect(await segments(page)).toEqual(["leaf"]);
  await expect(page.locator("#up")).toBeHidden();
  // R9's, on the top level, which is not either
  await viewer.line(19).locator(".no").click();
  await viewer.selected(`instance R9 in ${TOP}`);
  expect(await segments(page)).toEqual([TOP]);

  // a double click on an instance's statement opens it below its level, wherever the path was
  await viewer.line(10).locator(".no").dblclick();
  await expect(page.locator("#level-name")).toHaveText("cell");
  expect(await segments(page)).toEqual(["block", "cell XC1"]);
  await viewer.line(17).locator(".no").dblclick();
  expect(await segments(page)).toEqual([TOP, "bank XK1"]);
});

test("reading again keeps circuit visits even if a previously used instance was renamed", async ({ page, viewer }) => {
  await down(page, viewer, "XK1");
  await down(page, viewer, "XB2");
  await down(page, viewer, "XC1");
  await page.keyboard.press("Control+-");
  const view = await viewer.viewBox();
  await viewer.build(() => page.keyboard.press("F5"));
  expect(await segments(page)).toEqual([TOP, "bank XK1", "block XB2", "cell XC1"]);
  expect(await viewer.viewBox()).toBe(view);
  await page.keyboard.press("Backspace");
  expect(await segments(page)).toEqual([TOP, "bank XK1", "block XB2"]);
  await viewer.selected("instance XC1 in block");

  // XB2 is named XB3 now: these circuits were still visited, independently of current edges.
  await page.keyboard.press("Enter");
  expect(await viewer.viewBox()).toBe(view);
  await page.evaluate(() => {
    window.fakeShell.next = "nested-changed";
  });
  await viewer.build(() => page.keyboard.press("F5"));
  await expect(page.locator("#level-name")).toHaveText("cell");
  expect(await segments(page)).toEqual([TOP, "bank XK1", "block XB2", "cell XC1"]);
  await expect(page.locator("#up")).toBeVisible();
  expect(await viewer.viewBox()).toBe(view);
});

test("a subcircuit reached two ways keeps a view for each", async ({ page, viewer }) => {
  await down(page, viewer, "XC9");
  await page.keyboard.press("Control+-");
  const xc9 = await viewer.viewBox();
  await page.keyboard.press("Backspace");
  await down(page, viewer, "XK1");
  await down(page, viewer, "XB1");
  await down(page, viewer, "XC1");
  const fitted = await viewer.viewBox();
  expect(fitted).not.toBe(xc9);
  await page.keyboard.press("Control+=");
  const xc1 = await viewer.viewBox();

  // Repeating the descents restores their own views. Tree picks intentionally reset history.
  await page.locator('#level-path .crumb[data-depth="0"]').click();
  await down(page, viewer, "XC9");
  expect(await segments(page)).toEqual([TOP, "cell XC9"]);
  expect(await viewer.viewBox()).toBe(xc9);
  await page.locator('#level-path .crumb[data-depth="0"]').click();
  await down(page, viewer, "XK1");
  await down(page, viewer, "XB1");
  await down(page, viewer, "XC1");
  expect(await segments(page)).toEqual([TOP, "bank XK1", "block XB1", "cell XC1"]);
  expect(await viewer.viewBox()).toBe(xc1);
});

test("a long path keeps the level shown in a narrow bar, and leaves the levels between behind an ellipsis", async ({ page, viewer }) => {
  await down(page, viewer, "XK1");
  await down(page, viewer, "XB2");
  await down(page, viewer, "XC1");
  await down(page, viewer, "XL2");
  const more = page.locator("#level-path .crumb-more");
  const menu = page.locator("#path-menu");
  await expect(more).toBeHidden();
  /** Whether the bar's parts shown are within it, and the name of the level shown is whole. */
  const fits = () =>
    page.evaluate(() => {
      const bar = document.getElementById("level-path").getBoundingClientRect();
      const name = document.getElementById("level-name");
      return (
        [...document.querySelectorAll("#level-path > *")]
          .filter((e) => !e.hidden)
          .every((e) => e.getBoundingClientRect().left >= bar.left - 0.5 && e.getBoundingClientRect().right <= bar.right + 0.5) &&
        name.scrollWidth <= name.clientWidth
      );
    });

  // the drawing as narrow as the side panel leaves it: the kind gives way, then the levels after the root
  const split = await viewer.middle(page.locator("#vsplit"));
  await viewer.drag(split, { x: 1199, y: split.y });
  await expect(more).toBeVisible();
  await expect(page.locator("#level-kind")).toBeHidden();
  // once the bar is laid out at its new width
  await expect.poll(fits).toBe(true);
  const shown = await segments(page);
  expect(shown[0]).toBe(TOP);
  expect(shown[shown.length - 1]).toBe("leaf XL2");
  expect(shown.length).toBeLessThan(5);
  // the ellipsis lists them in order, and goes back to the one picked
  const left = (await path(page)).filter((s) => !shown.includes(s));
  expect(left).toContain("block XB2");
  await more.click();
  await expect(menu).toBeVisible();
  await expect(more).toHaveAttribute("aria-expanded", "true");
  await expect(menu.locator(".path-menu-item")).toHaveText(left.map((s) => s.replace(" ", "")));
  await menu.locator(".path-menu-item", { hasText: "block" }).click();
  await expect(menu).toBeHidden();
  await expect(page.locator("#level-name")).toHaveText("block");
  expect(await path(page)).toEqual([TOP, "bank XK1", "block XB2"]);
  await viewer.selected("instance XC1 in block");

  // the views kept while the drawing was wider may not show the instances in it now: fitted
  const fitDown = async (name) => {
    await page.keyboard.press("f");
    await down(page, viewer, name);
  };

  // from the keyboard: Enter on the ellipsis lists them with the first focused, the arrows move,
  // Escape closes the list and leaves the selection, and Enter on one goes back to it
  await fitDown("XC1");
  await fitDown("XL2");
  await viewer.click("R1");
  await more.focus();
  await page.keyboard.press("Enter");
  await expect(menu.locator(".path-menu-item").first()).toBeFocused();
  await page.keyboard.press("ArrowDown");
  await expect(menu.locator(".path-menu-item").nth(1)).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(menu).toBeHidden();
  await expect(more).toBeFocused();
  await viewer.selected("instance R1 in leaf");
  await page.keyboard.press("Enter");
  await page.keyboard.press("ArrowUp");
  await expect(menu.locator(".path-menu-item").last()).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.locator("#level-name")).toHaveText(left[left.length - 1].split(" ")[0]);
  // a press elsewhere closes the list
  await page.locator("#level-path button.crumb", { hasText: TOP }).click();
  for (const name of ["XK1", "XB2", "XC1", "XL2"]) await fitDown(name);
  await more.click();
  await expect(menu).toBeVisible();
  const status = await viewer.middle(page.locator(".statusbar"));
  await page.mouse.click(status.x, status.y);
  await expect(menu).toBeHidden();
  expect(await segments(page)).toEqual(shown);

  // narrower still, the root gives way too, and the level shown stays (a phone's width: the drawing
  // is a page of its own, and the bar as wide as the screen)
  await page.setViewportSize({ width: 180, height: 400 });
  await expect.poll(() => segments(page)).toEqual(["leaf XL2"]);
  await expect.poll(fits).toBe(true);
  await more.click();
  await expect(menu.locator(".path-menu-item")).toHaveText([TOP, "bankXK1", "blockXB2", "cellXC1"]);
  await page.keyboard.press("Escape");

  // wide again, the whole path and the kind
  await page.setViewportSize({ width: 1200, height: 800 });
  await page.keyboard.press("Control+B");
  await expect.poll(() => segments(page)).toEqual([TOP, "bank XK1", "block XB2", "cell XC1", "leaf XL2"]);
  await expect(more).toBeHidden();
  await expect(page.locator("#level-kind")).toBeVisible();
});

test('removing a same-file ancestor cannot leave an invalid history link', async ({page,viewer}) => {
  await down(page,viewer,'XK1');
  await down(page,viewer,'XB1');
  await down(page,viewer,'XC1');
  await page.evaluate(async()=>{
    const d=structuredClone(window.viewer.doc);
    d.levels.find(l=>l.title==='bank').title='renamed-bank';
    (await import('./open.js')).showBuilt(d,true);
  });
  expect(await segments(page)).toEqual(['cell']);
  await expect(page.locator('#up')).toBeHidden();
  await page.keyboard.press('Backspace');
  await expect(page.locator('#level-name')).toHaveText('cell');
});
