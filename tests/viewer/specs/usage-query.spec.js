// The usages of the selected object: when an instance, a net or a terminal that is no usage is selected, the
// lower half of the Usage panel, where a selected usage's details are, is a table of the usages it is a member of,
// one row for each, role · usage · definition, with the line of its statement, and the usages those are members of
// in turn indented under them; a usage that is not in effect is greyed, with its state and its reason. A row
// lights up its usage's members in the drawing while the pointer is on it, and a click selects the usage. In the
// text, a context menu of a reference shows them; View > Show Roles on Hover adds the role to the pointer's box.
// The fixture usage-query is the core's test netlist of its reverse index (`Object.usages`, `rnl explain`).

import { expect, test } from "../viewer.js";
import { repository, translated, uiLanguages, zh } from "../zh.js";

const rows = (page) => page.locator("#usage-detail .usage-membership-row");
const ids = (page) => page.locator("#usage-detail .membership-id");
const NEW = ["Usages of {0}", "It is in no usage.", "Select an object to see its usages, or a usage to inspect it.", "Select this usage",
  "overridden", "disabled", "written back by a solver", "rule of {0}", "rule of this document", "made in {0}", "{0} (terminal {1})",
  "{0} of {1} {2}", "Show its usages", "Show &Roles on Hover"];

/** Select an object of the Document by its kind and name, as a click on a list does, and open the Usage panel. */
async function selectObject(page, kind, name) {
  await page.evaluate(async ([kind, name]) => {
    const o = window.viewer.doc.objects.findIndex((x) => x.kind === kind && x.name === name);
    if (o < 0) throw new Error(`no ${kind} ${name}`);
    (await import("./selection.js")).select(o, "list", { usage: false });
  }, [kind, name]);
  await page.locator("#tab-usages").click();
}

test.beforeEach(async ({ viewer }) => {
  await viewer.open("usage-query");
});

test("an instance's usages are in the lower half of the Usage panel, the usages they are in indented under them", async ({ page, viewer }) => {
  await viewer.click("R1");
  await page.locator("#tab-usages").click();
  await expect(page.locator("#usage-detail h2")).toHaveText("Usages of R1");
  await expect(ids(page)).toHaveText(["%pair1", "%outer", "%top", "%top", "%all", "%bad"]);
  await expect(page.locator("#usage-detail .membership-role")).toHaveText(["left", "first", "first", "second", "{members}", "left"]);
  await expect(page.locator("#usage-detail .membership-definition")).toHaveText(["$PAIR", "$TWO", "$TWO", "$TWO", "$SET", "$PAIR"]);
  // %outer is a member of %top in turn, under %pair1, which it is made of: the chain upward, indented
  const depths = await rows(page).evaluateAll((els) => els.map((e) => Number(e.style.getPropertyValue("--depth"))));
  expect(depths).toEqual([0, 1, 2, 0, 0, 0]);
  // each row says where its statement is: the file's line
  await expect(page.locator("#usage-detail .membership-where")).toHaveText(Array(6).fill(/usage-query\.sp:\d+/));
  expect(await page.locator("#usage-detail .membership-where").first().textContent()).toBe("usage-query.sp:16");
});

test("a usage that is not in effect is greyed, with its state and its reason, and is no usage to select", async ({ page }) => {
  await selectObject(page, "instance", "R1");
  const bad = rows(page).last();
  await expect(bad).toHaveClass(/greyed/);
  await expect(bad).toHaveAttribute("data-state", "failed");
  await expect(bad.locator(".membership-state")).toHaveText("failed");
  await expect(bad.locator(".membership-reason")).toContainText("has no role extra");
  // it made no usage: its id is text
  expect(await bad.locator(".membership-id").evaluate((el) => el.tagName)).toBe("SPAN");
  await selectObject(page, "instance", "R3");
  const states = await rows(page).evaluateAll((els) => els.map((e) => e.dataset.state));
  expect(states).toContain("disabled");
  const off = rows(page).filter({ has: page.locator('.membership-id:text-is("%off")') });
  await expect(off).toHaveClass(/greyed/);
  await expect(off.locator(".membership-state")).toHaveText("disabled");
});

test("a terminal's role, a net's, the origin of a usage a rule made, and an object that is in none", async ({ page }) => {
  await selectObject(page, "instance", "M1");
  // its own terminal d, and the ground stubs of the standard package that name its other terminals
  await expect(ids(page).first()).toHaveText("%drain");
  await expect(page.locator("#usage-detail .membership-role").first()).toHaveText("pin (terminal d)");
  await expect(page.locator("#usage-detail .membership-role")).toHaveText(["pin (terminal d)", "t (terminal s)", "t (terminal b)"]);
  await expect(page.locator("#usage-detail .membership-origin")).toHaveText(["rule of rnl-std-fill", "rule of rnl-std-fill"]);
  await selectObject(page, "net", "b");
  await expect(page.locator("#usage-detail .membership-role")).toContainText(["signal"]);
  // a usage a rule made says so
  await selectObject(page, "instance", "R3");
  await expect(page.locator("#usage-detail .membership-origin")).toContainText(["rule of this document"]);
  // an object that is in no usage
  const none = await page.evaluate(() => window.viewer.doc.objects.find((o) => o.kind === "net" && o.usages.length === 0)?.name);
  expect(none).toBeTruthy();
  await selectObject(page, "net", none);
  await expect(page.locator("#usage-detail .placeholder")).toHaveText("It is in no usage.");
  await expect(rows(page)).toHaveCount(0);
});

test("nothing selected, and a usage selected, keep the panel's own lower half", async ({ page }) => {
  await page.locator("#tab-usages").click();
  await expect(page.locator("#usage-detail .placeholder")).toHaveText("Select an object to see its usages, or a usage to inspect it.");
  await page.locator(".usage-name", { hasText: "%pair1" }).click();
  await expect(page.locator("#usage-detail h2")).toHaveText("$PAIR: %pair1");
  await expect(rows(page)).toHaveCount(0);
});

test("a row lights up its usage's members in the drawing while the pointer is on it; a click selects the usage", async ({ page, viewer }) => {
  await viewer.click("R1");
  await page.locator("#tab-usages").click();
  const muted = page.locator("#canvas .usage-muted");
  await expect(muted).toHaveCount(0);
  const pair = rows(page).filter({ has: page.locator('.membership-id:text-is("%pair1")') }).first();
  await pair.hover();
  // the groups that are not its members, R3 and M1, are dimmed: the members stand out
  await expect.poll(() => muted.count()).toBeGreaterThan(0);
  await page.mouse.move(5, 5);
  await expect(muted).toHaveCount(0);
  // a click selects the usage: the existing selection, and its details in place of the table
  await pair.locator("button.membership-id").click();
  await expect(page.locator("#usage-detail h2")).toHaveText("$PAIR: %pair1");
  await expect(page.locator("#selection")).toContainText("%pair1");
});

test("the link of a row shows its statement in the text", async ({ page, viewer }) => {
  await viewer.click("R1");
  await page.locator("#tab-usages").click();
  await page.locator("#usage-detail .membership-where").first().click();
  await expect(page.locator("#tab-netlist")).toHaveAttribute("aria-selected", "true");
  await expect(viewer.line(16)).toBeVisible();
  await expect(viewer.line(16)).toContainText("%pair1");
});

test("a context menu of a reference in the text shows its usages", async ({ page, viewer }) => {
  await page.locator("#tab-netlist").click();
  const reference = page.locator("#source .ln[data-line='16'] [data-obj]").first();
  await reference.click({ button: "right" });
  const item = page.locator(".context-menu .context-item");
  await expect(item).toHaveText("Show its usages");
  await page.keyboard.press("Escape");
  await expect(item).toHaveCount(0);
  await reference.click({ button: "right" });
  await item.click();
  await expect(page.locator("#tab-usages")).toHaveAttribute("aria-selected", "true");
  await expect(page.locator("#usage-detail h2")).toHaveText(/^Usages of /);
  expect(await rows(page).count()).toBeGreaterThan(0);
  // a click elsewhere closes it
  await page.locator("#tab-netlist").click();
  await reference.click({ button: "right" });
  await expect(item).toBeVisible();
  await page.locator("#canvas").click({ position: { x: 5, y: 5 } });
  await expect(item).toHaveCount(0);
  // a place that names no object has no menu of ours
  await page.locator("#source .ln[data-line='1']").click({ button: "right", position: { x: 3, y: 3 } });
  await expect(item).toHaveCount(0);
});

test("View > Show Roles on Hover is off to begin with; on, the pointer's box adds the object's first usage and the rest", async ({ page, viewer }) => {
  const tip = page.locator("#hover-tip");
  expect(await page.evaluate(() => window.viewer.checked("toggle-hover-roles"))).toBe(false);
  const p = await viewer.where("R1");
  await page.mouse.move(p.x, p.y);
  await expect(tip).toHaveText("R1");
  await page.evaluate(() => window.viewer.command("toggle-hover-roles"));
  expect(await page.evaluate(() => window.viewer.checked("toggle-hover-roles"))).toBe(true);
  await page.mouse.move(p.x + 2, p.y + 2);
  // the first usage in effect, and how many more there are (the failed one is no usage)
  await expect(tip).toHaveText("R1 · left of $PAIR %pair1 · +2");
  // a terminal's role says whose terminal it is
  const q = await viewer.where("M1");
  await page.mouse.move(q.x, q.y);
  await expect(tip).toHaveText("M1 · pin (terminal d) of $PIN %drain · +2");
  // the choice is kept
  await page.reload();
  await viewer.started();
  await viewer.idle();
  expect(await page.evaluate(() => window.viewer.checked("toggle-hover-roles"))).toBe(true);
});

test("the texts are in every language of the interface, and the table follows the language", async ({ page, viewer }) => {
  for (const { lang } of uiLanguages(repository)) {
    if (lang === "en") continue;
    for (const english of NEW) expect(translated(lang, english), `${lang}: ${english}`).not.toBe("");
  }
  await viewer.click("R1");
  await page.locator("#tab-usages").click();
  await page.evaluate(() => window.viewer.command("language-zh"));
  await expect(page.locator("html")).toHaveAttribute("lang", "zh-CN");
  await expect(page.locator("#usage-detail h2")).toHaveText(zh("Usages of {0}", "R1"));
  await expect(rows(page).last().locator(".membership-state")).toHaveText(zh("failed"));
});
