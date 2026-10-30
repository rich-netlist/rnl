// Groups (@GROUP, 11.5) are a special kind of usage: the Usage panel lists them under @GROUP, after the definitions,
// folded to begin with like every kind; a group is drawn as a frame titled `@GROUP: %core`, and selecting one lights up
// its members; the table of an object's usages says which groups it is a member of and which groups those are within,
// indented as the usages of usages are; a group that PRIORITY has turned off is greyed, with its state.
// The fixture ota-groups is the five-transistor OTA with %core holding all five transistors, %amp the first four, %inp
// the input pair, and %bias, which overlaps %core in M5 and is disabled.

import { expect, test } from "../viewer.js";
import { repository, translated, uiLanguages, zh } from "../zh.js";

const NEW = ["Show all groups", "member of group {0}", "member (through terminal {0}) of group {1}", "{0} is in turn a member of group {1}",
  "{0} is in turn within group {1}", "within group {0}", "group"];
const row = (page, id) => page.locator("#usage-list .usage-row", { hasText: id });
const sentences = (page) => page.locator("#usage-detail .membership-sentence");

/** Select an object of the Document by its kind and name, as a click on a list does, and open the Usage panel. */
async function selectObject(page, kind, name) {
  await page.evaluate(async ([kind, name]) => {
    const o = window.viewer.doc.objects.findIndex((x) => x.kind === kind && x.name === name);
    if (o < 0) throw new Error(`no ${kind} ${name}`);
    (await import("./selection.js")).select(o, "list", { usage: false });
  }, [kind, name]);
  await page.locator("#tab-usages").click();
}

test.beforeEach(async ({ page, viewer }) => {
  await viewer.open("ota-groups");
  await page.locator("#tab-usages").click();
});

test("the groups are listed last, under @GROUP, folded to begin with, and a turned-off one is greyed with its state", async ({ page }) => {
  const heads = page.locator("#usage-list .usage-expand");
  await expect(heads.last()).toContainText("@GROUP");
  await expect(heads.last()).toContainText("(4)");
  await expect(heads.last()).toHaveAttribute("aria-expanded", "false");
  await expect(page.locator("#usage-list .usage-row", { hasText: "%core" })).toHaveCount(0);
  await heads.last().click();
  await expect(page.locator('#usage-list [data-type="@GROUP"] .usage-name')).toHaveText(["%core", "%amp", "%inp", "%bias"]);
  // only %bias is turned off (PRIORITY): its whole row is greyed and says why
  await expect(row(page, "%bias")).toHaveClass(/greyed/);
  await expect(row(page, "%bias").locator(".usage-state")).toHaveText("disabled");
  for (const id of ["%core", "%amp", "%inp"]) {
    await expect(row(page, id)).not.toHaveClass(/greyed/);
    await expect(row(page, id).locator(".usage-state")).toHaveCount(0);
  }
  // one checkbox shows and hides the groups' frames, as for any kind
  await expect(page.locator('#usage-list [data-type="@GROUP"] .usage-group-head input[type="checkbox"]')).toHaveAccessibleName("Show all groups");
});

test("a group is drawn as a frame titled @GROUP: its id, and selecting one lights up its members", async ({ page }) => {
  const frames = page.locator("#canvas .usage-frame .usage-title text");
  await expect(frames.filter({ hasText: "@GROUP: %core" })).toHaveCount(1);
  await expect(frames.filter({ hasText: "@GROUP: %inp" })).toHaveCount(1);
  // the frame of the group that is turned off is drawn faint, the others at full strength
  const off = page.locator("#canvas .usage-frame.disabled");
  await expect(off).toHaveCount(1);
  await expect(off.locator(".usage-title text")).toHaveText("@GROUP: %bias");
  await expect(off).toHaveAttribute("opacity", "0.4");
  await expect(page.locator("#canvas .usage-frame:not(.disabled)[opacity]")).toHaveCount(0);
  await page.locator("#usage-list .usage-expand").last().click();
  await row(page, "%amp").locator(".usage-name").click();
  await expect(page.locator("#usage-detail h2")).toHaveText("@GROUP: %amp");
  await expect(page.locator("#usage-detail .usage-roles")).toContainText("members");
  await expect(page.locator("#usage-detail .usage-roles")).toContainText("#M1, #M2, #M3, #M4");
  await expect(page.locator("#usage-detail .usage-source")).toContainText("%amp = {#M1 #M2 #M3 #M4} @GROUP");
  await expect(page.locator("#canvas .usage-frame.selected .usage-title text")).toHaveText("@GROUP: %amp");
  // the members stay at full strength, the rest of the drawing is dimmed
  await expect(page.locator("#canvas .usage-muted").first()).toBeAttached();
  const lit = await page.evaluate(() => {
    const doc = window.viewer.doc;
    const named = (name) => doc.levels[0].hits.find((h) => h.id && h.objects.some((o) => doc.objects[o].name === name))?.id;
    const muted = (id) => document.querySelector(`#canvas [data-id=${JSON.stringify(id)}]`)?.classList.contains("usage-muted");
    return { M1: muted(named("M1")), M4: muted(named("M4")), M5: muted(named("M5")) };
  });
  expect(lit).toEqual({ M1: false, M4: false, M5: true });
  await expect(page.locator("#selection")).toContainText("%amp");
  await expect(page.locator("#selection")).toContainText("group");
});

test("the frames of nested groups are set out a level each, so that their labels do not lie on one another", async ({ page }) => {
  const box = (id) => page.evaluate((id) => {
    const object = window.viewer.doc.levels[0].usages.find((u) => u.id === id).object;
    const frame = document.querySelector(`#canvas .usage-frame[data-usage="${object}"]`);
    const rect = frame.querySelector("rect").getBoundingClientRect();
    const title = frame.querySelector(".usage-title").getBoundingClientRect();
    return { left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom, titleTop: title.top, titleLeft: title.left };
  }, id);
  const [core, amp, inp] = [await box("%core"), await box("%amp"), await box("%inp")];
  // %inp is within %amp, which is within %core: each is outside the one within it, by a label's height
  for (const [outer, inner] of [[core, amp], [amp, inp]]) {
    expect(outer.left).toBeLessThan(inner.left);
    expect(outer.top).toBeLessThan(inner.top);
    expect(outer.right).toBeGreaterThan(inner.right);
    expect(outer.bottom).toBeGreaterThan(inner.bottom);
    expect(inner.titleTop - outer.titleTop).toBeGreaterThanOrEqual(17);
  }
});

test("the usages of an instance say which groups it is a member of, and which groups those are within, indented", async ({ page }) => {
  await selectObject(page, "instance", "M1");
  const groups = page.locator('#usage-detail .usage-membership-row[data-kind="group"]');
  const text = await groups.evaluateAll((els) => els.map((e) => e.querySelector(".membership-sentence").textContent));
  expect(text).toEqual([
    "member of group %core",
    "member of group %amp",
    "%amp is in turn within group %core",
    "member of group %inp",
    "%inp is in turn within group %amp",
    "%amp is in turn within group %core",
  ]);
  const depths = await groups.evaluateAll((els) => els.map((e) => Number(e.style.getPropertyValue("--depth"))));
  expect(depths).toEqual([0, 0, 1, 0, 1, 2]);
  // the usages of the instance come first, then its groups; a group is selected from its row like a usage
  const kinds = await page.locator("#usage-detail .usage-membership-row").evaluateAll((els) => els.map((e) => e.dataset.kind));
  expect(kinds.indexOf("group")).toBe(kinds.lastIndexOf("usage") + 1);
  await groups.nth(1).locator(".membership-id").click();
  await expect(page.locator("#usage-detail h2")).toHaveText("@GROUP: %amp");
});

test("a group that is turned off is greyed in the table of its members' usages, with its state", async ({ page }) => {
  await selectObject(page, "instance", "M5");
  const off = page.locator('#usage-detail .usage-membership-row[data-kind="group"]', { hasText: "%bias" });
  await expect(off).toHaveClass(/greyed/);
  await expect(off).toHaveAttribute("data-state", "disabled");
  await expect(off.locator(".membership-state")).toHaveText("disabled");
  await expect(off.locator(".membership-sentence")).toHaveText("member of group %bias");
});

test("the texts are in every language of the interface, and the list and the table follow the language", async ({ page }) => {
  for (const { lang } of uiLanguages(repository)) {
    if (lang === "en") continue;
    for (const english of NEW) expect(translated(lang, english), `${lang}: ${english}`).not.toBe("");
  }
  await page.evaluate(() => window.viewer.command("language-zh"));
  await expect(page.locator("html")).toHaveAttribute("lang", "zh-CN");
  await page.locator("#usage-list .usage-expand").last().click();
  await expect(row(page, "%bias").locator(".usage-state")).toHaveText(zh("disabled"));
  await expect(page.locator('#usage-list [data-type="@GROUP"] .usage-group-head input[type="checkbox"]')).toHaveAccessibleName(zh("Show all groups"));
  await selectObject(page, "instance", "M1");
  const groups = page.locator('#usage-detail .usage-membership-row[data-kind="group"] .membership-sentence');
  await expect(groups.nth(0)).toHaveText(zh("member of group {0}", "%core"));
  await expect(groups.nth(2)).toHaveText(zh("{0} is in turn within group {1}", "%amp", "%core"));
  await selectObject(page, "instance", "M5");
  await expect(page.locator('#usage-detail .usage-membership-row[data-kind="group"] .membership-state').last()).toHaveText(zh("disabled"));
});
