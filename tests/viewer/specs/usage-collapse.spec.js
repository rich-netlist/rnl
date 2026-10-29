// The Usage panel's list of usages by definition: every group is folded when a document or a level is shown; the button
// above the list opens all of them, or folds all of them when all are open; and a usage that is selected, on the drawing,
// in the table of an object's usages or in the text, has its group open and its row in view, the user's own folding of
// the group after that left as it is.

import { expect, test } from "../viewer.js";
import { repository, translated, uiLanguages, zh } from "../zh.js";

const groups = (page) => page.locator(".usage-group");
const open = (page) => page.locator('.usage-expand[aria-expanded="true"]');
const toggle = (page) => page.locator("#usage-expand-all");

/** The Usage panel, shown by its key. */
async function usagePanel(page) {
  await page.keyboard.press("5");
  await expect(page.locator("#pane-usages")).toBeVisible();
}

/** Select an object of the Document by its kind and name, as a list does. */
async function selectObject(page, kind, name, from = "list", usage = false) {
  await page.evaluate(async ([kind, name, from, usage]) => {
    const o = window.viewer.doc.objects.findIndex((x) => x.kind === kind && x.name === name);
    if (o < 0) throw new Error(`no ${kind} ${name}`);
    (await import("./selection.js")).select(o, from, { usage });
  }, [kind, name, from, usage]);
}

test("every group is folded to begin with, and the button opens and folds all of them", async ({ page, viewer }) => {
  await viewer.open("usage-inspector");
  await usagePanel(page);
  const total = await groups(page).count();
  expect(total).toBeGreaterThan(1);
  await expect(open(page)).toHaveCount(0);
  await expect(page.locator(".usage-row")).toHaveCount(0);
  await expect(toggle(page)).toHaveText("Expand All");
  await toggle(page).click();
  await expect(open(page)).toHaveCount(total);
  await expect(page.locator(".usage-row").first()).toBeVisible();
  await expect(toggle(page)).toHaveText("Collapse All");
  // one group folded by hand: the button offers to open all again
  await groups(page).first().locator(".usage-expand").click();
  await expect(toggle(page)).toHaveText("Expand All");
  await toggle(page).click();
  await expect(open(page)).toHaveCount(total);
  await toggle(page).click();
  await expect(open(page)).toHaveCount(0);
  await expect(page.locator(".usage-row")).toHaveCount(0);
  await expect(toggle(page)).toHaveText("Expand All");
});

test("the button is above the list and stays there while the list scrolls", async ({ page, viewer }) => {
  await viewer.open("usage-inspector");
  await usagePanel(page);
  const first = await toggle(page).boundingBox();
  const list = await page.locator("#usage-list").boundingBox();
  expect(first.y).toBeGreaterThanOrEqual(list.y);
  expect(first.y).toBeLessThan(list.y + 40);
  await page.evaluate(() => {
    const l = document.getElementById("usage-list");
    for (const d of Array.from({ length: 30 }, (_, i) => i)) l.append(Object.assign(document.createElement("div"), { style: "height:40px" }));
    l.scrollTop = l.scrollHeight;
  });
  const after = await toggle(page).boundingBox();
  expect(Math.abs(after.y - first.y)).toBeLessThan(2);
});

test("a level shown, or another document, starts folded again", async ({ page, viewer }) => {
  await viewer.open("usage-inspector");
  await usagePanel(page);
  await toggle(page).click();
  await expect(open(page).first()).toBeVisible();
  await page.evaluate(async () => {
    const v = window.viewer;
    await (await import("./subcircuits.js")).goTo(v.doc.levels.findIndex((l) => l.title === "other"));
  });
  await expect(page.locator(".usage-row")).toHaveCount(0);
  await expect(open(page)).toHaveCount(0);
  await expect(toggle(page)).toHaveText("Expand All");
});

test("a usage selected in the table of an object's usages opens its group and is in view", async ({ page, viewer }) => {
  await viewer.open("usage-query");
  await selectObject(page, "instance", "R1");
  await usagePanel(page);
  await expect(open(page)).toHaveCount(0);
  await page.locator('#usage-detail button.membership-id:text-is("%pair1")').first().click();
  await expect(page.locator('.usage-group[data-type="$PAIR"] .usage-expand')).toHaveAttribute("aria-expanded", "true");
  await expect(page.locator(".usage-row.selected .usage-name")).toHaveText("%pair1");
  await expect(page.locator(".usage-row.selected")).toBeInViewport();
  // the groups that are not its own stay folded
  await expect(open(page)).toHaveCount(1);
});

test("a usage selected on the drawing or in the text opens its group", async ({ page, viewer }) => {
  await viewer.open("usage-inspector");
  await usagePanel(page);
  await expect(open(page)).toHaveCount(0);
  await selectObject(page, "usage", "%pair1", "drawing", true);
  await expect(page.locator('.usage-group[data-type="$PAIR"] .usage-expand')).toHaveAttribute("aria-expanded", "true");
  await expect(page.locator(".usage-row.selected .usage-name")).toHaveText("%pair1");
  await toggle(page).click(); // all open, then folded: the selected usage's group is not opened again by that
  await toggle(page).click();
  await expect(open(page)).toHaveCount(0);
  await selectObject(page, "usage", "%pair2", "source", true);
  await expect(page.locator(".usage-row.selected .usage-name")).toHaveText("%pair2");
});

test("folding the selected usage's group by hand is left as it is", async ({ page, viewer }) => {
  await viewer.open("usage-inspector");
  await usagePanel(page);
  await selectObject(page, "usage", "%pair1", "drawing", true);
  const group = page.locator('.usage-group[data-type="$PAIR"]');
  await expect(group.locator(".usage-expand")).toHaveAttribute("aria-expanded", "true");
  await group.locator(".usage-expand").click();
  await expect(group.locator(".usage-expand")).toHaveAttribute("aria-expanded", "false");
  await expect(page.locator("#usage-detail h2")).toHaveText("$PAIR: %pair1");
});

test("a selected usage far down a long list, in a group past its first hundred rows, is brought into view", async ({ page, viewer }) => {
  await viewer.open("usage-anonymous");
  await page.evaluate(() => {
    const d = window.viewer.doc, l = d.levels[0], u = l.usages.find((x) => x.definition === "$PAIR");
    l.usages = ["$AAA", "$BBB", "$CCC"].flatMap((definition, k) => Array.from({ length: k === 2 ? 250 : 40 }, (_, i) => {
      const object = d.objects.length;
      d.objects.push({ ...d.objects[u.object], name: `%u${k}_${i}` });
      return { ...u, object, id: `%u${k}_${i}`, definition };
    }));
    window.dispatchEvent(new Event("viewer-level"));
  });
  await usagePanel(page);
  await expect(open(page)).toHaveCount(0);
  const last = await page.evaluate(() => window.viewer.doc.levels[0].usages.find((x) => x.id === "%u2_220").object);
  await page.evaluate(async (o) => (await import("./selection.js")).select(o, "usage"), last);
  await expect(page.locator(".usage-row.selected .usage-name")).toHaveText("%u2_220");
  await expect(page.locator(".usage-row.selected")).toBeInViewport();
  const [row, list] = await Promise.all([page.locator(".usage-row.selected").boundingBox(), page.locator("#usage-list").boundingBox()]);
  expect(row.y).toBeGreaterThanOrEqual(list.y);
  expect(row.y + row.height).toBeLessThanOrEqual(list.y + list.height + 1);
});

test("the button's texts are in every language of the interface, and follow the language", async ({ page, viewer }) => {
  for (const { lang } of uiLanguages(repository)) {
    if (lang === "en") continue;
    for (const english of ["Expand All", "Collapse All"]) expect(translated(lang, english), `${lang}: ${english}`).not.toBe("");
  }
  await viewer.open("usage-inspector");
  await usagePanel(page);
  await page.evaluate(() => window.viewer.command("language-zh"));
  await expect(toggle(page)).toHaveText(zh("Expand All"));
  await toggle(page).click();
  await expect(toggle(page)).toHaveText(zh("Collapse All"));
});
