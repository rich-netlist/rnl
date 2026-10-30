// The parameters of the subcircuit shown, in the bar between the path and the drawing: each with the value the
// core gives it (`Level.params`, and `Child.effective` for the instance the level was entered through) and where
// it comes from; folded into a summary when there are many. The fixture's Documents come from `rnl view`.

import { expect, test } from "../viewer.js";
import { repository, translated, uiLanguages, zh } from "../zh.js";

const NEW = ["Parameters", "The parameters of this subcircuit", "from {0}", "changed", "not set", "default",
  "This value cannot be evaluated; it is shown as written.", "{0} parameters", "{0} from {1}", "{0} changed",
  "{0} default", "{0} not set", "Show all", "Fold"];

test.beforeEach(async ({ viewer }) => {
  await viewer.open("subckt-params");
});

/** Enter an instance of the top level by name, as a key does. */
async function enter(page, name) {
  await page.evaluate(async (name) => {
    const o = window.viewer.doc.objects.findIndex((x) => x.kind === "instance" && x.name === name);
    (await import("./selection.js")).select(o, "list", { usage: false });
  }, name);
  await page.evaluate(() => document.activeElement?.blur());
  await page.keyboard.press("e");
  await expect(page.locator("#level-path .crumb.current .via")).toHaveText(name);
}
const bar = (page) => page.locator("#params-bar");
const params = (page) => bar(page).locator(".param");

test("the top level has no bar; a subcircuit entered through an instance shows the values that instance gives, and the defaults of the rest", async ({ page }) => {
  await expect(bar(page)).toBeHidden();
  await enter(page, "X1");
  await expect(bar(page)).toBeVisible();
  await expect(bar(page).locator(".params-title")).toHaveText("Parameters");
  await expect(params(page)).toHaveText(["wn = 2ufrom X1", "l = 0.18udefault", "rl = 20kfrom X1"]);
  await expect(params(page)).toHaveClass(["param instance", "param default", "param instance"]);
  // the drawing's text is not touched, and nothing offers to fold three parameters
  await expect(bar(page).locator(".params-fold")).toHaveCount(0);
  await page.keyboard.press("Backspace");
  await expect(bar(page)).toBeHidden();
  // another instance of the same subcircuit gives its own
  await enter(page, "X2");
  await expect(params(page)).toHaveText(["wn = 1udefault", "l = 0.18udefault", "rl = 10kdefault"]);
});

test("a value that cannot be evaluated stays as written, with a question mark", async ({ page }) => {
  await enter(page, "X4");
  await expect(params(page).first()).toHaveText("wn = {2*wn} ?from X4");
  await expect(params(page).first()).toHaveAttribute("title", "This value cannot be evaluated; it is shown as written.");
});

test("many parameters fold into a summary that unfolds, and stays unfolded for the next subcircuit", async ({ page }) => {
  await enter(page, "X3");
  await expect(params(page)).toHaveCount(0);
  await expect(bar(page)).toContainText("6 parameters: 1 from X3, 5 default");
  await bar(page).locator(".params-fold").click();
  await expect(params(page)).toHaveCount(6);
  await expect(params(page).first()).toHaveText("p1 = 7from X3");
  await expect(bar(page).locator(".params-fold")).toHaveText("Fold ▴");
  await page.keyboard.press("Backspace");
  await enter(page, "X3");
  await expect(params(page)).toHaveCount(6);
  await bar(page).locator(".params-fold").click();
  await expect(params(page)).toHaveCount(0);
});

test("the texts are in every language of the interface, and the bar follows the language", async ({ page }) => {
  for (const { lang } of uiLanguages(repository)) {
    if (lang === "en") continue;
    for (const english of NEW) expect(translated(lang, english), `${lang}: ${english}`).not.toBe("");
  }
  await enter(page, "X1");
  await page.evaluate(() => window.viewer.command("language-zh"));
  await expect(page.locator("html")).toHaveAttribute("lang", "zh-CN");
  await expect(bar(page).locator(".params-title")).toHaveText(zh("Parameters"));
  await expect(params(page).first()).toHaveText(`wn = 2u${zh("from {0}", "X1")}`);
  await expect(params(page).nth(1)).toHaveText(`l = 0.18u${zh("default")}`);
  await page.keyboard.press("Backspace");
  await enter(page, "X3");
  await expect(bar(page)).toContainText(zh("{0} parameters", 6));
  await expect(bar(page)).toContainText(zh("{0} from {1}", 1, "X3"));
});
