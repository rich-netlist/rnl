// A violation (7.6), the fourth level of report: ranked after the errors and before the warnings and the hints, counted and
// switched on its own, in its own colour, on the lines of the text as well (apps/viewer-ui/problems.js, text.js).

import { expect, test } from "../viewer.js";

const ORDER = ["error", "violation", "warning", "hint"];
const rows = (page) => page.locator("#diags > li.diag:not(.package-hint)");

/** A diagnostic of a severity, in the line `line` of the first file. */
const make = (severity, line, name) => ({
  severity, code: null, name, message: `${severity} at ${line}`, file: "12-diagnostics.sp", full_path: "12-diagnostics.sp", file_index: 0, line, col: 0, origin: "", cause: null,
});

for (const platform of ["web", "desktop"]) {
  test.describe(`on the ${platform}`, () => {
    test("the four levels are in the order of the list, the heading and the colours of the lines", async ({ page, viewer }) => {
      await viewer.open("12-diagnostics", { platform });
      // given in the opposite order, and one line with a warning and a violation, another with an error and a violation
      const list = [make("hint", 5, "h"), make("warning", 6, "w"), make("violation", 6, "v"), make("warning", 7, "w"), make("violation", 7, "v"),
        make("error", 7, "e"), make("violation", 8, "v")];
      await page.evaluate(async (list) => {
        const problems = await import("./problems.js");
        problems.renderDiagnostics(list);
        problems.markDiagnostics(list);
      }, list);
      // the list: by severity, in the order
      const kinds = await page.evaluate((order) => [...document.querySelectorAll("#diags > li.diag:not(.geo)")].map((e) => order.find((s) => e.classList.contains(s))), ORDER);
      expect(kinds).toEqual(["error", "violation", "violation", "violation", "warning", "warning", "hint"]);
      // the heading: a count for each level that has any, in the order, each a switch
      const counts = await page.locator("#counts > .chip").allTextContents();
      expect(counts[0]).toBe("1 error");
      expect(counts[1]).toBe("3 violations");
      await expect(page.locator("#counts > .n-violation")).toHaveCount(1);
      // a switch for the violations shows only them, in their own colour
      await page.locator("#counts > .n-violation").click();
      expect(await page.evaluate(() => [...document.querySelectorAll("#diags > li.diag")].every((e) => e.classList.contains("violation")))).toBe(true);
      await expect(rows(page)).toHaveCount(3);
      const [violation, error, warning] = await page.evaluate(() => ["violation", "error", "warning"].map((s) => {
        const el = document.createElement("span");
        el.className = `diag ${s}`;
        el.innerHTML = '<span class="sev">x</span>';
        document.querySelector("#diags").append(el);
        const colour = getComputedStyle(el.querySelector(".sev")).color;
        el.remove();
        return colour;
      }));
      expect(new Set([violation, error, warning]).size, "three colours").toBe(3);
      await page.locator("#counts > .n-violation").click();
      // the numbers of the lines: a line takes the colour of its most serious problem, the violation between the error and the warning
      const colours = (n) => viewer.line(n).evaluate((e) => [...e.classList].filter((c) => c.startsWith("d-")));
      expect(await colours(5)).toEqual(["d-hint"]);
      expect(await colours(6)).toEqual(["d-violation"]);
      expect(await colours(7)).toEqual(["d-error"]);
      expect(await colours(8)).toEqual(["d-violation"]);
    });

    test("a violation of the core is a line of its own kind, and nothing fails", async ({ page, viewer }) => {
      await viewer.open("violation", { platform });
      const diagnostics = await page.evaluate(() => window.viewer.doc.diagnostics);
      const violation = diagnostics.find((d) => d.severity === "violation");
      expect(violation).toMatchObject({ code: "RNL6003", name: "violation-current-one-way" });
      expect(diagnostics.some((d) => d.severity === "error")).toBe(false);
      const row = page.locator("#diags > li.diag.violation").first();
      await expect(row.locator(".sev")).toHaveText("violation");
      await expect(row.locator(".code")).toHaveText("RNL6003");
      await expect(row.locator(".name")).toHaveText("violation-current-one-way");
      await expect(page.locator("#counts > .n-violation")).toHaveText("1 violation");
      // its line in the text has its colour, and a click shows it
      await expect(viewer.line(violation.line)).toHaveClass(/\bd-violation\b/);
      await row.locator(".name").click();
      await expect(viewer.line(violation.line)).toHaveClass(/flash/);
      // the drawing is shown: a violation does not stop the reading
      await expect(page.locator("#canvas > svg")).toHaveCount(1);
    });
  });
}
