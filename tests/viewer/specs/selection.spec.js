// Selecting an instance or a net, on the drawing or in the text, and following it to the other.

import { expect, test } from "../viewer.js";

const TOP = "04-hierarchy.sp";

/** The data-id of the groups that draw an object on the level shown, sorted. */
function groupsOf(page, o) {
  return page.evaluate((o) => {
    const v = window.viewer;
    const ids = v.doc.levels[v.level].hits.filter((h) => h.id && h.objects.includes(o)).map((h) => h.id);
    return [...new Set(ids)].sort();
  }, o);
}

test.beforeEach(async ({ viewer }) => {
  await viewer.open("04-hierarchy");
});

test("clicking a symbol selects its instance and marks its statement and the places that name it", async ({ page, viewer }) => {
  const x2 = await viewer.object("instance", "X2", TOP);
  await viewer.click("X2");
  await viewer.selected(`instance X2 in ${TOP}`);
  expect(await viewer.markedLines()).toEqual([50]);
  // its name in the statement and the references to it in the records
  const lines = await viewer.spanLines(x2);
  expect(lines.length).toBeGreaterThan(2);
  expect(await viewer.markedPlaces()).toEqual(lines);
  await expect(viewer.places(x2).first()).toHaveClass(/\bhl\b/);
  // its symbol, its box and the badge that opens it
  expect(await viewer.selectedGroups()).toEqual(await groupsOf(page, x2));
  await expect(page.locator("#canvas > svg .overlay rect:not(.label)")).toHaveCount(1);
  // and the labels attached to it: its name, and the subcircuit it is an instance of
  await expect(page.locator("#canvas > svg .overlay rect.label")).toHaveCount(2);
  await expect(page.locator("#enter-badge")).toBeVisible();
  await expect(page.locator("#canvas")).toHaveClass(/\bsel-sub\b/);
  await expect(page.locator("#selection")).toHaveText(`X2 instance of amp in ${TOP} · Enter opens it`);
  // the statement is scrolled into view
  const [line, source] = [await viewer.line(50).boundingBox(), await page.locator("#source").boundingBox()];
  expect(line.y).toBeGreaterThanOrEqual(source.y);
  expect(line.y + line.height).toBeLessThanOrEqual(source.y + source.height);

  // an instance that opens nothing
  await viewer.click("Vin");
  await viewer.selected(`instance Vin in ${TOP}`);
  expect(await viewer.markedLines()).toEqual([53]);
  await expect(page.locator("#enter-badge")).toBeHidden();
  await expect(page.locator("#canvas")).not.toHaveClass(/\bsel-sub\b/);
  await expect(page.locator("#selection")).toHaveText(`Vin instance in ${TOP}`);
});

test("clicking a wire, a pin or a ground selects the net", async ({ page, viewer }) => {
  // d has no wire, and its pin is on X3's symbol: only the pin tells the net from the instance there
  const d = await viewer.object("net", "d", TOP);
  expect(await page.evaluate((o) => window.viewer.doc.levels[0].hits.filter((h) => h.objects.includes(o)).map((h) => h.kind), d)).toEqual(["pin"]);
  for (const [name, kind] of [["b", "wire"], ["d", "pin"], ["0", "box"]]) {
    const net = await viewer.object("net", name, TOP);
    await viewer.click(name, kind);
    await viewer.selected(`net ${name} in ${TOP}`);
    await expect(page.locator("#selection")).toHaveText(`${name} net in ${TOP}`);
    expect(await viewer.selectedGroups()).toEqual(await groupsOf(page, net));
    // a net has no statement of its own, only the places that name it
    expect(await viewer.markedLines()).toEqual([]);
    expect(await viewer.markedPlaces()).toEqual(await viewer.spanLines(net));
    // its pins are marked, and no instance box
    const pins = await page.evaluate((o) => window.viewer.doc.levels[0].hits.filter((h) => h.kind === "pin" && h.objects.includes(o)).length, net);
    await expect(page.locator("#canvas > svg .overlay circle")).toHaveCount(pins);
    await expect(page.locator("#canvas > svg .overlay rect:not(.label)")).toHaveCount(0);
    await expect(page.locator("#enter-badge")).toBeHidden();
  }
});

test("clicking a statement or a node in the text selects it on the drawing, on its level", async ({ page, viewer }) => {
  // M1's statement, in the subcircuit amp
  await viewer.line(9).locator(".no").click();
  await viewer.selected("instance M1 in amp");
  expect(await viewer.state()).toMatchObject({ title: "amp" });
  await expect(page.locator("#level-name")).toHaveText("amp");
  const m1 = await viewer.object("instance", "M1", "amp");
  expect(await viewer.selectedGroups()).toEqual(await groupsOf(page, m1));
  expect(await viewer.markedLines()).toEqual([9]);

  // the node b of X1's statement, on the top level
  const b = await viewer.object("net", "b", TOP);
  await viewer.line(49).locator(`[data-obj~="${b}"]`).click();
  await viewer.selected(`net b in ${TOP}`);
  await expect(page.locator("#level-name")).toHaveText(TOP);
  expect(await viewer.selectedGroups()).toEqual(await groupsOf(page, b));

  // a reference in a record, such as #X3 in #X3:top, which names its instance first
  const x3 = await viewer.object("instance", "X3", TOP);
  const ref = (await viewer.spanLines(x3)).find((l) => l !== 51);
  await viewer.line(ref).locator(`[data-obj~="${x3}"]`).first().click();
  await viewer.selected(`instance X3 in ${TOP}`);
  expect(await viewer.markedLines()).toEqual([51]);

  // a node in the included file, in the subcircuit rdiv
  await page.locator("#file-tabs button", { hasText: "04-hierarchy-lib.inc" }).click();
  const mid = await viewer.object("net", "mid", "rdiv");
  await viewer.line(3).locator(`[data-obj~="${mid}"]`).click();
  await viewer.selected("net mid in rdiv");
  await expect(page.locator("#level-name")).toHaveText("rdiv");
  expect(await viewer.selectedGroups()).toEqual(await groupsOf(page, mid));
});

test("a terminal's reference names its instance and, all of it, the net at the terminal", async ({ page, viewer }) => {
  // line 60, &c @WIRE(...) {#X2:out #X3:top}: #X3 is X3, and #X3:top, like #X2:out, is the net c
  const [x3, c] = [await viewer.object("instance", "X3", TOP), await viewer.object("net", "c", TOP)];
  const marked = () => viewer.line(60).locator(".hl").evaluateAll((els) => els.map((e) => e.textContent).join(""));
  await viewer.click("X3");
  await viewer.selected(`instance X3 in ${TOP}`);
  expect(await marked()).toBe("#X3");
  await viewer.click("c", "wire");
  await viewer.selected(`net c in ${TOP}`);
  expect(await marked()).toBe("&c#X2:out#X3:top");
  // a click on the terminal's name selects the net, and on the instance's the instance
  await viewer.click("X1");
  await viewer.line(60).locator(`[data-obj="${c}"]`, { hasText: "top" }).click();
  await viewer.selected(`net c in ${TOP}`);
  await viewer.line(60).locator(`[data-obj="${x3} ${c}"]`).click();
  await viewer.selected(`instance X3 in ${TOP}`);
});

test("a selected double line keeps the colour between its lines", async ({ page, viewer }) => {
  // R1's symbol is a double rectangle, R2 is drawn in the double style and the wire of a is double:
  // each is two strokes, the second narrower in the colour between the lines (class "inner")
  const LEVEL = "17-line-styles.sp";
  await viewer.open("17-line-styles");
  /** The computed stroke of each stroked element of the groups that draw an object, and whether it is inner. */
  const strokes = (o) =>
    page.evaluate((o) => {
      const v = window.viewer;
      const ids = new Set(v.doc.levels[v.level].hits.filter((h) => h.id && h.objects.includes(o)).map((h) => h.id));
      const groups = [...document.querySelectorAll("#canvas > svg g[data-id]")].filter((g) => ids.has(g.dataset.id));
      return groups
        .flatMap((g) => [...g.querySelectorAll("[stroke]:not([stroke=none])")])
        .map((e) => [getComputedStyle(e).stroke, e.classList.contains("inner")]);
    }, o);
  for (const [name, kind, what] of [["R1", "box", "instance"], ["R2", "box", "instance"], ["a", "wire", "net"]]) {
    const o = await viewer.object(what, name, LEVEL);
    const before = await strokes(o);
    expect(before.filter(([, inner]) => inner).length).toBeGreaterThan(0);
    await viewer.click(name, kind);
    await viewer.selected(`${what} ${name} in ${LEVEL}`);
    const after = await strokes(o);
    // the inner strokes as they were, and the others all in the selection's colour
    expect(after.filter(([, inner]) => inner)).toEqual(before.filter(([, inner]) => inner));
    const selected = after.filter(([, inner]) => !inner).map(([stroke]) => stroke);
    expect(new Set(selected).size).toBe(1);
    expect(before.map(([stroke]) => stroke)).not.toContain(selected[0]);
    await page.keyboard.press("Escape");
    await viewer.selected(null);
  }
});

test("Escape and a click on an empty place clear the selection", async ({ page, viewer }) => {
  const cleared = async () => {
    await viewer.selected(null);
    await expect(page.locator("#source .hl, #source .hl-line")).toHaveCount(0);
    await expect(page.locator("#canvas > svg g.sel")).toHaveCount(0);
    await expect(page.locator("#canvas > svg .overlay > *")).toHaveCount(0);
    await expect(page.locator("#selection")).toHaveText("");
    await expect(page.locator("#enter-badge")).toBeHidden();
  };
  await viewer.click("X2");
  await viewer.selected(`instance X2 in ${TOP}`);
  await page.keyboard.press("Escape");
  await cleared();

  await viewer.click("b", "wire");
  await viewer.selected(`net b in ${TOP}`);
  const empty = await viewer.empty();
  await page.mouse.click(empty.x, empty.y);
  await cleared();

  // in the text: a line that is no statement and names nothing
  await viewer.line(50).locator(".no").click();
  await viewer.selected(`instance X2 in ${TOP}`);
  await viewer.line(1).click();
  await cleared();
});

test("dragging with the left or the middle button moves the view and selects nothing", async ({ page, viewer }) => {
  await viewer.click("X2");
  await viewer.selected(`instance X2 in ${TOP}`);
  for (const [button, dx, dy] of [["left", 120, 40], ["middle", -120, -40]]) {
    const before = (await viewer.viewBox()).split(" ").map(Number);
    const from = await viewer.where("X1");
    await viewer.drag(from, { x: from.x + dx, y: from.y + dy }, button);
    const after = (await viewer.viewBox()).split(" ").map(Number);
    const k = (await page.locator("#canvas").evaluate((c) => c.clientWidth)) / before[2];
    expect(after[0]).toBeCloseTo(before[0] - dx / k, 6);
    expect(after[1]).toBeCloseTo(before[1] - dy / k, 6);
    expect(after.slice(2)).toEqual(before.slice(2));
    await viewer.selected(`instance X2 in ${TOP}`);
  }
  // a click of the middle button selects nothing either
  const x1 = await viewer.where("X1");
  await page.mouse.click(x1.x, x1.y, { button: "middle" });
  await viewer.selected(`instance X2 in ${TOP}`);
});
