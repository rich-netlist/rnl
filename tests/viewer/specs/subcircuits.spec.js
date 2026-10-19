// Going down into a subcircuit instance, by Enter, a double click on the drawing or in the text, or
// the open badge; and back up, by Backspace or the up button, each level with the view it had.

import { expect, test } from "../viewer.js";

const TOP = "04-hierarchy.sp";

test.beforeEach(async ({ viewer }) => {
  await viewer.open("04-hierarchy");
});

test("Enter opens a selected subcircuit instance, and Backspace goes back with the view kept", async ({ page, viewer }) => {
  await page.keyboard.press("Control+-"); // a view of the top level that fitting would not give
  const top = await viewer.viewBox();
  await viewer.click("X2");
  await viewer.selected(`instance X2 in ${TOP}`);
  await page.keyboard.press("Enter");
  await expect(page.locator("#level-name")).toHaveText("amp");
  await expect(page.locator("#level-kind")).toHaveText("subcircuit");
  await viewer.selected(null);
  await expect(page.locator("#up")).toBeVisible();
  await expect(page.locator("#up-name")).toHaveText(TOP);
  await expect(page.locator("#up")).toHaveAttribute("title", `Up to ${TOP} (Ctrl+E or Backspace; Shift opens a new window)`);
  await page.keyboard.press("Control+-");
  const amp = await viewer.viewBox();

  // the instance we came out of is selected, and the view is as it was left
  await page.keyboard.press("Backspace");
  await expect(page.locator("#level-name")).toHaveText(TOP);
  await viewer.selected(`instance X2 in ${TOP}`);
  expect(await viewer.viewBox()).toBe(top);
  await expect(page.locator("#up")).toBeHidden();
  await expect(page.locator("#enter-badge")).toBeVisible();

  // down the same way again, the subcircuit is as it was left
  await page.keyboard.press("Enter");
  await expect(page.locator("#level-name")).toHaveText("amp");
  expect(await viewer.viewBox()).toBe(amp);
  await page.keyboard.press("Backspace");
  await viewer.selected(`instance X2 in ${TOP}`);

  // through the other instance of amp, it has a view of its own: fitted
  await viewer.click("X1");
  await page.keyboard.press("Enter");
  await expect(page.locator("#level-name")).toHaveText("amp");
  const fitted = await viewer.viewBox();
  expect(fitted).not.toBe(amp);
  await page.keyboard.press("f");
  expect(await viewer.viewBox()).toBe(fitted);
  await page.keyboard.press("Backspace");
  await viewer.selected(`instance X1 in ${TOP}`);
});

test("a double click on a subcircuit instance opens it, and the up button goes back", async ({ page, viewer }) => {
  const x3 = await viewer.where("X3");
  await page.mouse.dblclick(x3.x, x3.y);
  await expect(page.locator("#level-name")).toHaveText("rdiv");
  await expect(page.locator("#up-name")).toHaveText(TOP);
  await page.locator("#up").click();
  await expect(page.locator("#level-name")).toHaveText(TOP);
  await viewer.selected(`instance X3 in ${TOP}`);
  await expect(page.locator("#up")).toBeHidden();

  // an instance that opens nothing is only selected
  const vin = await viewer.where("Vin");
  await page.mouse.dblclick(vin.x, vin.y);
  await viewer.selected(`instance Vin in ${TOP}`);
  await expect(page.locator("#level-name")).toHaveText(TOP);
});

test("the badge on a selected subcircuit instance sits on its box and opens it", async ({ page, viewer }) => {
  const badge = page.locator("#enter-badge");
  await expect(badge).toBeHidden();
  await viewer.click("X3");
  await expect(badge).toBeVisible();
  // on the top right corner of the instance's box, wherever the view moves
  const corner = async () => {
    const [u, v] = await page.evaluate(() => {
      const v = window.viewer;
      const box = v.doc.levels[v.level].hits.find((h) => h.kind === "box" && h.objects.includes(v.selected));
      return [box.points[1][0], box.points[0][1]];
    });
    return viewer.point(u, v);
  };
  for (const key of [null, "Control+=", "Control+="]) {
    if (key) await page.keyboard.press(key);
    const [at, want] = [await badge.boundingBox(), await corner()];
    expect(at.x - want.x).toBeCloseTo(4, 1);
    expect(want.y - (at.y + at.height)).toBeCloseTo(4, 1);
  }
  await badge.click();
  await expect(page.locator("#level-name")).toHaveText("rdiv");
  await expect(badge).toBeHidden();
});

test("a double click on an instance's statement or name in the text opens its subcircuit", async ({ page, viewer }) => {
  // X3's statement, by its line number
  await viewer.line(51).locator(".no").dblclick();
  await expect(page.locator("#level-name")).toHaveText("rdiv");
  await page.keyboard.press("Backspace");
  await viewer.selected(`instance X3 in ${TOP}`);

  // X1's name in a reference of a record
  const x1 = await viewer.object("instance", "X1", TOP);
  await viewer.line(58).locator(`[data-obj~="${x1}"]`).first().dblclick();
  await expect(page.locator("#level-name")).toHaveText("amp");
  // the word that the double click selected is let go
  expect(await page.evaluate(() => String(window.getSelection()))).toBe("");

  // a statement that opens nothing opens nothing
  await page.keyboard.press("Backspace");
  await viewer.line(53).locator(".no").dblclick();
  await viewer.selected(`instance Vin in ${TOP}`);
  await expect(page.locator("#level-name")).toHaveText(TOP);
});

test("Enter and Backspace act only on a subcircuit, and not in the search box", async ({ page, viewer }) => {
  // at the top level, with an instance that opens nothing
  await viewer.click("Vin");
  await page.keyboard.press("Enter");
  await page.keyboard.press("Backspace");
  expect(await viewer.state()).toMatchObject({ title: TOP, selected: `instance Vin in ${TOP}` });

  // in the search box they edit the text
  await viewer.click("X2");
  await page.keyboard.press("Enter");
  await expect(page.locator("#level-name")).toHaveText("amp");
  await page.locator("#tab-hierarchy").click();
  await page.locator("#search").click();
  await page.keyboard.type("rdx");
  await page.keyboard.press("Backspace");
  await expect(page.locator("#search")).toHaveValue("rd");
  await expect(page.locator("#level-name")).toHaveText("amp");
});
