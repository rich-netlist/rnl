// The table of an object's usages when the usages are nested deep: seven levels, with long names, and one usage that is a
// member of two outer usages (the fixture usage-deep). The rows are indented to three levels and no further, so each
// says whose member it is in its sentence; a long name wraps within the panel, which is no wider than its narrowest,
// without a horizontal scrollbar; the usages listed on the way up are not listed again (a chain that loops ends).

import { expect, test } from "../viewer.js";

const rows = (page) => page.locator("#usage-detail .usage-membership-row");

/** Select an object by its kind and name, as a click on a list does, and open the Usage panel. */
async function selectObject(page, kind, name) {
  await page.evaluate(async ([kind, name]) => {
    const o = window.viewer.doc.objects.findIndex((x) => x.kind === kind && x.name === name);
    if (o < 0) throw new Error(`no ${kind} ${name}`);
    (await import("./selection.js")).select(o, "list", { usage: false });
  }, [kind, name]);
  await page.locator("#tab-usages").click();
}

test.beforeEach(async ({ viewer }) => {
  await viewer.open("usage-deep");
});

test("seven levels are listed, indented to three, each row naming whose member it is", async ({ page }) => {
  await selectObject(page, "instance", "R1");
  await expect(rows(page)).toHaveCount(8);
  const depths = await rows(page).evaluateAll((els) => els.map((e) => Number(e.dataset.depth)));
  // the chain of seven, then the other usage that wraps the first one
  expect(depths).toEqual([0, 1, 2, 3, 4, 5, 6, 1]);
  const indents = await rows(page).evaluateAll((els) => els.map((e) => e.style.getPropertyValue("--depth")));
  expect(indents).toEqual(["0", "1", "2", "3", "3", "3", "3", "1"]);
  // the first row is the object's own, a sentence; the others say whose member their usage is
  await expect(rows(page).first().locator(".membership-sentence")).toHaveText(
    "input_side_resistor of usage %analog_front_end_input_bias_network (definition $RESISTOR_PAIR_WITH_A_LONG_DEFINITION_NAME)");
  await expect(rows(page).nth(6).locator(".membership-sentence")).toHaveText(
    "%signal_path_section_wrapper_five is in turn wrapped_stage_usage of usage %complete_receiver_channel_wrapper_six (definition $HIERARCHICAL_STAGE_WRAPPER_DEFINITION)");
  // a usage in two outer ones is under each: the one it is in turn a member of is named by the row
  await expect(rows(page).nth(7).locator(".membership-subject")).toHaveText("%analog_front_end_input_bias_network");
  await expect(rows(page).nth(7).locator(".membership-id")).toHaveText("%alternative_input_stage_wrapper");
});

test("the other instance has the same chain, and the usages are listed once each", async ({ page }) => {
  await selectObject(page, "instance", "R2");
  expect(await rows(page).count()).toBe(9); // the same chain, and a usage of the standard package that names its terminal
  const ids = await page.locator("#usage-detail .membership-id").allTextContents();
  expect(new Set(ids).size).toBe(ids.length);
});

for (const width of [0, 420]) {
  test(`long names wrap in a panel ${width ? "of its usual width" : "as narrow as it goes"}, with no horizontal scrollbar`, async ({ page }) => {
    await page.evaluate(async (w) => (await import("./panels.js")).setSidebarWidth(w), width);
    await selectObject(page, "instance", "R1");
    await expect(rows(page)).toHaveCount(8);
    const wide = await page.evaluate(() => {
      const detail = document.querySelector("#usage-detail");
      const panel = detail.getBoundingClientRect();
      return {
        scroll: detail.scrollWidth - detail.clientWidth,
        outside: [...detail.querySelectorAll(".usage-membership-row *")].filter((e) => e.getBoundingClientRect().right > panel.right + 1).length,
        width: panel.width,
      };
    });
    expect(wide.scroll).toBeLessThanOrEqual(0);
    expect(wide.outside).toBe(0);
    if (!width) expect(wide.width).toBeLessThan(420);
  });
}

test("the usages on the way up are not listed again: a usage that is a member of itself ends the chain", async ({ page }) => {
  // a Document in which the first usage is, by a hand-made entry, a member of the usage that wraps it
  const ids = await page.evaluate(async () => {
    const doc = window.viewer.doc;
    const find = (name) => doc.objects.findIndex((o) => o.name === name);
    const first = find("%analog_front_end_input_bias_network");
    const second = find("%analog_input_stage_wrapper_one");
    doc.objects[second].usages = [...doc.objects[second].usages, { ...doc.objects[second].usages[0], usage: first, id: "%loop" }];
    const { objectUsages } = await import("./usage-table.js");
    const r1 = doc.objects.findIndex((o) => o.kind === "instance" && o.name === "R1");
    const out = objectUsages(doc, r1, { select() {}, hover() {}, reveal() {} });
    return [...out.querySelectorAll(".membership-id")].map((e) => e.textContent);
  });
  // the loop's row is there, once, and its usage, already on the way up, is not listed under it again
  expect(ids.filter((id) => id === "%loop")).toHaveLength(1);
  expect(ids).toHaveLength(9);
});
