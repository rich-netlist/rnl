// The parameters of the subcircuit shown, in the bar between the path and the drawing: each with the value the
// core gives it (`Level.params`, and `Child.effective` for the instance the level was entered through) and where
// it comes from; folded into a summary when there are many. The fixture's Documents come from `rnl view`.

import { expect, test } from "../viewer.js";
import { repository, translated, uiLanguages, zh } from "../zh.js";

const NEW = ["Parameters", "The parameters of this subcircuit", "from {0}", "changed", "not set", "default",
  "This value cannot be evaluated; it is shown as written.", "{0} parameters", "{0} from {1}", "{0} changed",
  "{0} default", "{0} not set", "Show all", "Fold", "Parameters of {0}",
  "A change applies to this view only; the netlist is not changed. Values are written as in the netlist, and a value left empty is the parameter's default.",
  "Parameter", "Default", "Value", "Source", "Value of {0}", "Restore the values of {0}", "Edit Parameters...",
  "Subcircuit &Parameters...",
  "Some parameters have no default, so this view has no value for them. Write the values to use; a change applies to this view only, and the netlist is not changed."];

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

const dialog = (page) => page.locator("dialog.dialog");
const binds = (page) => page.evaluate(() => window.fakeShell.buildBinds.filter((b) => b !== undefined));

test("Edit Parameters builds the Document again with the values written, and shows the same level with them", async ({ page }) => {
  await enter(page, "X1");
  await bar(page).locator(".params-edit").click();
  await expect(dialog(page).locator(".dialog-title")).toHaveText("Parameters of amp");
  const rows = dialog(page).locator(".params-table tbody tr");
  await expect(rows).toHaveCount(3);
  await expect(rows.nth(0).locator(".params-name")).toHaveText("wn");
  await expect(rows.nth(0).locator(".params-default")).toHaveText("1u");
  await expect(rows.nth(0).locator("input")).toHaveValue("2u");
  await expect(rows.nth(0).locator(".params-source")).toHaveText("from X1");
  await expect(rows.nth(1).locator("input")).toHaveValue("0.18u");
  await expect(rows.nth(1).locator(".params-source")).toHaveText("default");
  await rows.nth(1).locator("input").fill("0.25u");
  await dialog(page).getByRole("button", { name: "Apply" }).click();
  await expect(params(page)).toHaveText(["wn = 2ufrom X1", "l = 0.25uchanged", "rl = 20kfrom X1"]);
  // what the build was asked for: the values that are not the defaults, as written
  expect(await binds(page)).toEqual([{ subckt: "amp", params: { wn: "2u", l: "0.25u", rl: "20k" } }]);
  // the same level, through the same instance
  await expect(page.locator("#level-name")).toHaveText("amp");
  await expect(page.locator("#level-path .crumb.current .via")).toHaveText("X1");
  // Restore the values of X1 gives the instance's back, and an empty field the default
  await bar(page).locator(".params-edit").click();
  await expect(dialog(page).locator(".params-table tbody tr").nth(1).locator("input")).toHaveValue("0.25u");
  await dialog(page).getByRole("button", { name: "Restore the values of X1" }).click();
  await expect(dialog(page).locator(".params-table tbody tr").nth(1).locator("input")).toHaveValue("");
  await expect(dialog(page).locator(".params-table tbody tr").nth(0).locator("input")).toHaveValue("2u");
  await dialog(page).getByRole("button", { name: "Apply" }).click();
  await expect(params(page)).toHaveText(["wn = 2ufrom X1", "l = 0.18udefault", "rl = 20kfrom X1"]);
  expect((await binds(page)).at(-1)).toEqual({ subckt: "amp", params: { wn: "2u", rl: "20k" } });
});

test("Apply with nothing changed builds nothing, and Cancel too; the command is for a subcircuit with parameters", async ({ page, viewer }) => {
  await viewer.menuTitle("Edit").click();
  await expect(viewer.menuItem("edit-params")).toBeDisabled();
  await page.keyboard.press("Escape");
  await enter(page, "X1");
  await viewer.menuTitle("Edit").click();
  await expect(viewer.menuItem("edit-params")).toBeEnabled();
  await page.keyboard.press("Escape");
  await page.evaluate(() => { void window.viewer.command("edit-params"); });
  await expect(dialog(page)).toBeVisible();
  await dialog(page).getByRole("button", { name: "Apply" }).click();
  await expect(dialog(page)).toHaveCount(0);
  await bar(page).locator(".params-edit").click();
  await dialog(page).locator("input").first().fill("9u");
  await dialog(page).getByRole("button", { name: "Cancel" }).click();
  await expect(params(page).first()).toHaveText("wn = 2ufrom X1");
  expect(await binds(page)).toEqual([]);
  
});

test("a subcircuit shown by itself has its defaults to change, and Restore Defaults empties the fields", async ({ page }) => {
  await page.evaluate(async () => {
    const k = window.viewer.doc.levels.findIndex((l) => l.title === "amp");
    (await import("./navigation.js")).navigate({ path: [{ level: k, via: null }], selection: null });
  });
  await expect(params(page)).toHaveText(["wn = 1udefault", "l = 0.18udefault", "rl = 10kdefault"]);
  await bar(page).locator(".params-edit").click();
  await dialog(page).locator("input").first().fill("3u");
  await dialog(page).getByRole("button", { name: "Apply" }).click();
  await expect(params(page)).toHaveText(["wn = 3uchanged", "l = 0.18udefault", "rl = 10kdefault"]);
  expect(await binds(page)).toEqual([{ subckt: "amp", params: { wn: "3u" } }]);
  await bar(page).locator(".params-edit").click();
  await dialog(page).getByRole("button", { name: "Restore Defaults" }).click();
  await expect(dialog(page).locator("input").first()).toHaveValue("");
  await page.keyboard.press("Escape");
});

/** Show a subcircuit by itself, as a pick in the index does. */
const showAlone = (page, title) => page.evaluate(async (title) => {
  const k = window.viewer.doc.levels.findIndex((l) => l.title === title);
  (await import("./navigation.js")).navigate({ path: [{ level: k, via: null }], selection: null });
}, title);

test("a subcircuit shown by itself with a parameter that has no default asks for the value, once; through an instance it does not", async ({ page }) => {
  // through its instance: the bar says it has none
  await enter(page, "X5");
  await expect(params(page)).toHaveText(["k = ?not set"]);
  await expect(dialog(page)).toHaveCount(0);
  await page.keyboard.press("Backspace");
  // alone: the dialog comes up with the field to fill
  await showAlone(page, "gain");
  await expect(dialog(page).locator(".dialog-title")).toHaveText("Parameters of gain");
  await expect(dialog(page)).toContainText("Some parameters have no default, so this view has no value for them.");
  await expect(dialog(page).locator("input")).toBeFocused();
  await dialog(page).locator("input").fill("2");
  await dialog(page).getByRole("button", { name: "Apply" }).click();
  await expect(params(page)).toHaveText(["k = 2changed"]);
  expect(await binds(page)).toEqual([{ subckt: "gain", params: { k: "2" } }]);
  await expect(dialog(page)).toHaveCount(0);
});

test("cancelling the question leaves the value missing, and it is not asked again for that Document; a subcircuit with defaults is not asked", async ({ page }) => {
  await showAlone(page, "amp");
  await expect(params(page)).toHaveCount(3);
  await expect(dialog(page)).toHaveCount(0);
  await showAlone(page, "gain");
  await expect(dialog(page)).toBeVisible();
  await dialog(page).getByRole("button", { name: "Cancel" }).click();
  await expect(params(page)).toHaveText(["k = ?not set"]);
  await showAlone(page, "amp");
  await showAlone(page, "gain");
  await expect(params(page)).toHaveText(["k = ?not set"]);
  await expect(dialog(page)).toHaveCount(0);
  expect(await binds(page)).toEqual([]);
});
