// The geometry hints in the problems list (apps/viewer-ui/problems.js): the geometry hints of the level shown, `Level.hints`,
// are rows of the list with a "geo" tag and their own severity, counted in that severity as `rnl check` counts; each count
// of the heading, by severity, is a switch that shows only the rows of that severity.

import { expect, test } from "../viewer.js";

const rows = (page) => page.locator("#diags > li.diag:not(.package-hint)");
const geoRows = (page) => page.locator("#diags > li.diag.geo");
const chip = (page, severity) => page.locator(`#counts > .n-${severity}`);
const ORDER = ["error", "warning", "hint"];

/** The hints of the level shown, as its Document gives them, and the diagnostics of the document. */
const documentOf = (page) =>
  page.evaluate(() => ({ hints: window.viewer.doc.levels[window.viewer.level].hints ?? [], diagnostics: window.viewer.doc.diagnostics }));

for (const platform of ["web", "desktop"]) {
  test.describe(`on the ${platform}`, () => {
    test("the geometry hints are rows of the list with a geo tag and their own severity, counted in it", async ({ page, viewer }) => {
      await viewer.open("12-diagnostics", { platform });
      const { hints, diagnostics } = await documentOf(page);
      expect(hints.length).toBeGreaterThan(0);
      const count = (list, s) => list.filter((d) => d.severity === s).length;
      const plural = (n, word) => `${n} ${word}${n === 1 ? "" : "s"}`;
      // the counts are by severity alone: a hint is counted in its own, as rnl check counts, and there is no count of its own
      const all = [...diagnostics, ...hints];
      await expect(page.locator("#counts > *")).toHaveText(ORDER.map((s) => plural(count(all, s), s)));
      await expect(page.locator("#counts > .n-geo")).toHaveCount(0);
      // the list: the diagnostics and the hints, each of the severities in turn
      await expect(rows(page)).toHaveCount(diagnostics.length + hints.length);
      await expect(geoRows(page)).toHaveCount(hints.length);
      const severities = await rows(page).evaluateAll((els) => els.map((e) => ["error", "warning", "hint"].find((s) => e.classList.contains(s))));
      expect(severities).toEqual([...severities].sort((a, b) => ORDER.indexOf(a) - ORDER.indexOf(b)));
      // a hint's row: the tag, its own severity, the message without the file it began with, and where it is
      const first = geoRows(page).first();
      const warning = hints.find((h) => h.severity === "warning");
      await expect(first).toHaveClass(/\bwarning\b/);
      await expect(first.locator(".tag")).toHaveText("geo");
      await expect(first.locator(".sev")).toHaveText("warning");
      expect(await first.locator(".msg").textContent()).toBe(`geo${warning.message.replace(`${warning.file}: `, "")}`);
      await expect(first.locator(".loc")).toHaveText(`${warning.file}:${warning.line}`);
      // and no diagnostic row has the tag
      await expect(page.locator("#diags > li.diag:not(.geo) .tag")).toHaveCount(0);
    });

    test("each count is a switch: it shows only the rows of its severity, and again shows all, without folding the problems", async ({ page, viewer }) => {
      await viewer.open("12-diagnostics", { platform });
      const { hints, diagnostics } = await documentOf(page);
      const all = [...diagnostics, ...hints];
      const of = (s) => all.filter((r) => r.severity === s).length;
      const heading = page.locator("#problems-head");
      await chip(page, "warning").click();
      await expect(chip(page, "warning")).toHaveAttribute("aria-pressed", "true");
      await expect(rows(page)).toHaveCount(of("warning"));
      expect(await rows(page).evaluateAll((els) => els.every((e) => e.classList.contains("warning")))).toBe(true);
      // the geometry hints of that severity are among them, with their tag
      await expect(geoRows(page)).toHaveCount(hints.filter((h) => h.severity === "warning").length);
      await expect(heading).toHaveAttribute("aria-expanded", "true");
      await chip(page, "warning").click();
      await expect(chip(page, "warning")).toHaveAttribute("aria-pressed", "false");
      await expect(rows(page)).toHaveCount(all.length);
      // the keyboard works the same, one count at a time, and keeps the focus
      await chip(page, "hint").focus();
      await page.keyboard.press("Enter");
      await expect(rows(page)).toHaveCount(of("hint"));
      await expect(chip(page, "warning")).toHaveAttribute("aria-pressed", "false");
      await page.keyboard.press("Space");
      await expect(rows(page)).toHaveCount(all.length);
    });

    test("a click on a hint shows its line in the text, as one on a diagnostic does", async ({ page, viewer }) => {
      await viewer.open("12-diagnostics", { platform });
      const { hints } = await documentOf(page);
      await expect(geoRows(page).first()).toHaveClass(/located/);
      await geoRows(page).first().click();
      await expect(viewer.line(hints.filter((h) => h.severity === "warning")[0].line)).toHaveClass(/flash/);
    });

    test("another level has hints of its own; a switch whose rows are gone is off", async ({ page, viewer }) => {
      await viewer.open("04-hierarchy", { platform });
      // no diagnostics here, only the hints of the level: counted, and listed, with no "nothing" row
      await expect(page.locator("#diags > li.nothing")).toHaveCount(0);
      expect(await page.locator("#counts > .n-error, #counts > .n-warning").count()).toBe(0);
      for (const level of [0, 1, 0]) {
        await page.evaluate(async (level) => (await import("./subcircuits.js")).goTo(level), level);
        const { hints } = await documentOf(page);
        if (hints.length) await expect(chip(page, "hint")).toHaveText(`${hints.length} hint${hints.length === 1 ? "" : "s"}`);
        else await expect(chip(page, "hint")).toHaveCount(0);
        await expect(geoRows(page)).toHaveCount(hints.length);
      }
      await page.evaluate(async () => (await import("./subcircuits.js")).goTo(0));
      await chip(page, "hint").click();
      await expect(chip(page, "hint")).toHaveAttribute("aria-pressed", "true");
      await page.evaluate(() => {
        window.viewer.doc.levels[window.viewer.level].hints = [];
        window.dispatchEvent(new Event("viewer-level"));
      });
      await expect(chip(page, "hint")).toHaveCount(0);
      await expect(page.locator("#diags > li.nothing")).toHaveText("No errors, warnings or hints.");
    });

    test("a long list is made a batch at a time", async ({ page, viewer }) => {
      await viewer.open("12-diagnostics", { platform });
      const { diagnostics } = await documentOf(page);
      // 34 warnings and 785 hints of a big drawing, in the Document as the core gives them
      await page.evaluate(() => {
        const level = window.viewer.doc.levels[window.viewer.level];
        level.hints = Array.from({ length: 819 }, (_, i) => ({
          kind: "looks-joined", severity: i < 34 ? "warning" : "hint", message: `hint number ${i}`, file: "x.sp", file_index: 0, line: 1 + (i % 5),
        }));
        window.dispatchEvent(new Event("viewer-level"));
      });
      await expect(rows(page)).toHaveCount(100);
      await expect(page.locator("#diags > .diagnostics-more")).toContainText(`100 of ${diagnostics.length + 819} shown`);
      // only the hints' severity: a batch at a time as well, by the button and by the scroll
      await chip(page, "hint").click();
      await expect(rows(page)).toHaveCount(100);
      await page.locator("#diags > .diagnostics-more button").evaluate((button) => button.click());
      await expect(rows(page)).toHaveCount(200);
      await page.locator("#problems-body").evaluate((e) => { e.scrollTop = e.scrollHeight; });
      await expect.poll(() => rows(page).count()).toBeGreaterThan(200);
    });
  });
}
