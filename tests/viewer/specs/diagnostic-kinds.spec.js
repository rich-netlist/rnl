// A problem of the problems list (apps/viewer-ui/problems.js) is one line, as in an IDE: its severity, the number the toolkit
// gives its kind of problem (`code`, such as RNL3120), the kind's short name (`name`) and the line; the explanation is under
// it when the row is open. A click opens or closes it and shows its line; a geometry hint's row is the same, with its tag.

import { expect, test } from "../viewer.js";

const rows = (page) => page.locator("#diags > li.diag:not(.package-hint):not(.geo)");

for (const platform of ["web", "desktop"]) {
  test.describe(`on the ${platform}`, () => {
    test("each diagnostic is a line of severity, kind and place, and its message is below it, closed", async ({ page, viewer }) => {
      await viewer.open("12-diagnostics", { platform });
      const diagnostics = await page.evaluate(() => window.viewer.doc.diagnostics);
      expect(diagnostics.length).toBeGreaterThan(0);
      // the core gives every diagnostic a short name, and a number but for the kinds of 0.2 and 0.3 alone
      expect(diagnostics.every((d) => typeof d.name === "string" && d.name.length > 0)).toBe(true);
      expect(diagnostics.some((d) => /^RNL\d{4}$/.test(d.code ?? ""))).toBe(true);
      // the list shows them in the order of severity: each row is found by its message, and has its own kind and place
      const shown = await rows(page).evaluateAll((els) => els.map((e) => ({
        message: e.querySelector(".details .msg").textContent,
        loc: e.querySelector(".loc").textContent,
        place: e.querySelector(".loc").place,
        code: e.querySelector(".code").textContent || null,
        name: e.querySelector(".name").textContent,
        open: e.classList.contains("open"),
      })));
      expect(shown).toHaveLength(diagnostics.length);
      for (const row of shown) {
        const d = diagnostics.find((x) => x.message === row.message && x.line === row.place[2] && x.col === row.place[3]);
        expect(d, `${row.message} at ${row.loc}`).toBeTruthy();
        expect(row.code).toBe(d.code ?? null);
        expect(row.name).toBe(d.name);
        // the line is of the file in the text pane: the place is the line and the column, without the file
        expect(row.loc).toBe(`${d.line}${d.col ? `:${d.col}` : ""}`);
        expect(row.open).toBe(false);
      }
      expect(shown.some((r) => r.code !== null)).toBe(true);
      // closed, the message is not on the screen; the row is one line
      const first = rows(page).first();
      await expect(first.locator(".details")).toBeHidden();
      expect((await first.boundingBox()).height).toBeLessThan(30);
    });

    test("a click opens the row and shows its line; another closes it; the twisty only opens and closes", async ({ page, viewer }) => {
      await viewer.open("12-diagnostics", { platform });
      const row = rows(page).filter({ has: page.locator(".code:not(:empty)") }).first();
      const twisty = row.locator(".twisty");
      const line = await row.locator(".loc").evaluate((e) => e.place[2]);
      await expect(row).toHaveClass(/located/);
      await row.locator(".name").click();
      await expect(row).toHaveClass(/\bopen\b/);
      await expect(row.locator(".details .msg")).toBeVisible();
      await expect(twisty).toHaveAttribute("aria-expanded", "true");
      await expect(viewer.line(line)).toHaveClass(/flash/);
      // the explanation is text: a click in it opens nothing and closes nothing
      await row.locator(".details .msg").click();
      await expect(row).toHaveClass(/\bopen\b/);
      // the same click again closes it
      await row.locator(".name").click();
      await expect(row).not.toHaveClass(/\bopen\b/);
      await expect(row.locator(".details")).toBeHidden();
      // the twisty opens and closes, and shows no line
      await page.evaluate(() => document.querySelectorAll("#text .flash, #source .flash").forEach((e) => e.classList.remove("flash")));
      await twisty.click();
      await expect(row).toHaveClass(/\bopen\b/);
      await expect(viewer.line(line)).not.toHaveClass(/flash/);
      await twisty.click();
      await expect(row).not.toHaveClass(/\bopen\b/);
    });

    test("the number and the short name have their tooltips, the number is text to select; a place in another file names the file", async ({ page, viewer }) => {
      await viewer.open("04-hierarchy", { platform });
      const geo = page.locator("#diags > li.diag.geo");
      // a level below the top has hints in the included file, not in the one the text pane shows
      const below = await page.evaluate(() => window.viewer.doc.levels.findIndex((l) => (l.hints ?? []).some((h) => h.file_index !== 0)));
      expect(below, "a public case with a hint in an included file").toBeGreaterThan(0);
      await page.evaluate(async (level) => (await import("./subcircuits.js")).goTo(level), below);
      const hints = await page.evaluate(() => window.viewer.doc.levels[window.viewer.level].hints);
      const other = hints.find((h) => h.file_index !== 0);
      const row = geo.filter({ hasText: other.file }).first();
      const handle = await row.elementHandle();
      await expect(row.locator(".loc")).toHaveText(`${other.file}:${other.line}`);
      await expect(row.locator(".code")).toHaveAttribute("title", "The kind of problem: its number and its short name");
      expect(await row.locator(".code").evaluate((e) => getComputedStyle(e).userSelect)).not.toBe("none");
      // a click shows that file, and the places say where they are by it: the lines of that file need no name now
      await row.locator(".name").click();
      expect(await page.evaluate(async () => (await import("./text.js")).fileShown)).toBe(other.file_index);
      await expect.poll(() => handle.evaluate((e) => e.querySelector(".loc").textContent)).toBe(`${other.line}`);
      expect(other.col ?? 0).toBe(0);
    });

    test("a geometry hint is a line with its tag and kind; a problem with no kind has its message in the line", async ({ page, viewer }) => {
      await viewer.open("12-diagnostics", { platform });
      const hints = await page.evaluate(() => window.viewer.doc.levels[window.viewer.level].hints);
      const first = page.locator("#diags > li.diag.geo").first();
      const warning = hints.find((h) => h.severity === "warning");
      expect(warning.code).toMatch(/^RNL7\d{3}$/);
      await expect(first.locator(".tag")).toHaveText("geo");
      await expect(first.locator(".code")).toHaveText(warning.code);
      await expect(first.locator(".name")).toHaveText(warning.kind);
      await expect(first.locator(".loc")).toHaveText(String(warning.line));
      // its explanation is the core's message, without the file it begins with
      await expect(first.locator(".details .msg")).toHaveText(warning.message.replace(`${warning.file}: `, ""));
      // a reading that failed has no kind: its line has the message, which is also its explanation
      await page.evaluate(async () => {
        (await import("./problems.js")).renderDiagnostics([{ severity: "error", message: "the reading failed", file: "", line: 0, col: 0, origin: "" }]);
      });
      const failed = page.locator("#diags > li.diag.error").first();
      await expect(failed.locator(".code")).toHaveText("");
      await expect(failed.locator(".name")).toHaveText("the reading failed");
      await expect(failed.locator(".name")).toHaveClass(/plain/);
      await expect(failed.locator(".details .msg")).toHaveText("the reading failed");
    });
  });
}
