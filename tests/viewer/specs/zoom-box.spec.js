// The rectangle a right drag zooms to keeps the plain highlight colour, whatever is selected: a
// selected subcircuit instance colours the selection in its own colour, not this rectangle.
import { test, expect } from "../viewer.js";

/** The zoom rectangle's border and fill while a right drag over the drawing is under way. */
async function dragColours(page) {
  const r = await page.locator("#canvas").boundingBox();
  await page.mouse.move(r.x + r.width * 0.2, r.y + r.height * 0.3);
  await page.mouse.down({ button: "right" });
  await page.mouse.move(r.x + r.width * 0.6, r.y + r.height * 0.6, { steps: 5 });
  await expect(page.locator(".zoom-box")).toBeVisible();
  const colours = await page.locator(".zoom-box").evaluate((el) => {
    const style = getComputedStyle(el);
    return { border: style.borderTopColor, fill: style.backgroundColor };
  });
  return { colours, finish: () => page.mouse.up({ button: "right" }) };
}

for (const background of ["light", "dark"]) {
  test(`${background}: a right drag's rectangle is the plain highlight with a subcircuit instance selected`, async ({ page, viewer }, info) => {
    if (background === "dark") await page.emulateMedia({ colorScheme: "dark" });
    await viewer.open("04-hierarchy");
    if (background === "dark") await page.evaluate(() => window.viewer.command("background-dark"));
    const plain = await dragColours(page);
    await plain.finish();
    await page.keyboard.press("f");
    const instance = await page.evaluate(() => window.viewer.doc.objects.find((o) => o.kind === "instance" && o.opens !== null && o.level === 0).name);
    await viewer.click(instance);
    await expect(page.locator("#canvas")).toHaveClass(/sel-sub/);
    const selected = await dragColours(page);
    if (process.env.RNL_REVIEW_DIR && info.project.name === "chromium") {
      await page.locator("#canvas").screenshot({ path: `${process.env.RNL_REVIEW_DIR}/zoom-box-${background}.png` });
    }
    await selected.finish();
    expect(selected.colours).toEqual(plain.colours);
    // the selection itself keeps the subcircuit colour
    const sub = await page.locator("#canvas").evaluate((c) => getComputedStyle(c).getPropertyValue("--select").trim());
    expect(sub).not.toBe("");
    expect(await page.locator("#canvas").evaluate((c) => getComputedStyle(c).getPropertyValue("--drawing-sub").trim())).toBe(sub);
  });
}
