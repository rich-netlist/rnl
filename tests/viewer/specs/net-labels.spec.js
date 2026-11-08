// View > Show Net Names (apps/viewer-ui/net-labels.js): the name of each net on the drawing, once for each connected part
// of the net, at the places of the level's `net_labels`; off to begin with and kept; one size, one font and grey whatever the
// zoom; no pointer events; the key N, which does not run while typing.

import { expect, test } from "../viewer.js";

const labels = (page) => page.locator("#canvas .net-labels text");

/** The names the level shown is to have written on it, as its Document gives them. */
const wanted = (page) =>
  page.evaluate(() => {
    const { doc, level } = window.viewer;
    return doc.levels[level].net_labels.map((l) => doc.objects[l.net]?.name).filter(Boolean);
  });

for (const platform of ["web", "desktop"]) {
  test.describe(`on the ${platform}`, () => {
    test("it starts off, the View menu has it, and the choice is kept", async ({ page, viewer }) => {
      await viewer.open("04-hierarchy", { platform });
      expect(await page.evaluate(() => window.viewer.checked("toggle-net-labels"))).toBe(false);
      await expect(labels(page)).toHaveCount(0);
      if (platform === "web") {
        await viewer.menuTitle("View").click();
        const item = page.locator('.menu-item[data-id="toggle-net-labels"]');
        await expect(item.locator(".label")).toHaveText("Show Net Names");
        await expect(item.locator(".keys")).toHaveText("N");
        await item.click();
      } else {
        await page.evaluate(() => window.fakeShell.menu("toggle-net-labels"));
      }
      expect(await page.evaluate(() => window.viewer.checked("toggle-net-labels"))).toBe(true);
      const names = await wanted(page);
      expect(names.length).toBeGreaterThan(0);
      await expect(labels(page)).toHaveCount(names.length);
      expect(await labels(page).allTextContents()).toEqual(names);
      // kept: a page opened again has them, and the shell was told the check
      await viewer.open("04-hierarchy", { platform });
      expect(await page.evaluate(() => window.viewer.checked("toggle-net-labels"))).toBe(true);
      await expect(labels(page)).toHaveCount(names.length);
      expect(await viewer.calls("setCheck")).toEqual(expect.arrayContaining([["toggle-net-labels", true]]));
      await page.evaluate(() => window.viewer.command("toggle-net-labels"));
      await expect(labels(page)).toHaveCount(0);
      expect(await page.evaluate(() => window.viewer.checked("toggle-net-labels"))).toBe(false);
    });

    test("each name is at its place, at one size whatever the zoom, in grey, and takes no pointer events", async ({ page, viewer }) => {
      await viewer.open("04-hierarchy", { platform });
      await page.evaluate(() => window.viewer.command("toggle-net-labels"));
      const names = await wanted(page);
      await expect(labels(page)).toHaveCount(names.length);
      // the centre of a name on the screen, from the view, is the place's: the drawing's SVG pixels, projected
      const centres = () =>
        page.evaluate(() => {
          const svg = document.querySelector("#canvas > svg");
          const box = svg.getBoundingClientRect();
          const vb = svg.viewBox.baseVal;
          const k = box.width / vb.width;
          const { doc, level } = window.viewer;
          return {
            k,
            texts: [...document.querySelectorAll("#canvas .net-labels text")].map((t) => {
              const r = t.getBoundingClientRect();
              return [r.left + r.width / 2 - box.left, r.top + r.height / 2 - box.top, r.height];
            }),
            at: doc.levels[level].net_labels.filter((l) => doc.objects[l.net]?.name).map((l) => [(l.at[0] - vb.x) * k, (l.at[1] - vb.y) * k]),
          };
        });
      const one = await centres();
      one.texts.forEach(([x, y], i) => {
        expect(x).toBeCloseTo(one.at[i][0], -1);
        expect(y).toBeCloseTo(one.at[i][1], -1);
      });
      const size = await page.evaluate(() => getComputedStyle(document.querySelector("#canvas .net-labels text")).fontSize);
      const height = one.texts[0][2];
      // another zoom: the names move with the drawing and keep their size
      await page.evaluate(async () => (await import("./drawing.js")).zoomCenter(3));
      const two = await centres();
      expect(two.k).not.toBeCloseTo(one.k, 3);
      two.texts.forEach(([x, y, h], i) => {
        expect(x).toBeCloseTo(two.at[i][0], -1);
        expect(y).toBeCloseTo(two.at[i][1], -1);
        expect(h).toBeCloseTo(height, 0);
      });
      // the same size in every name: one font, and grey, not black
      expect(await page.evaluate(() => new Set([...document.querySelectorAll("#canvas .net-labels text")].map((t) => {
        const s = getComputedStyle(t);
        return `${s.fontSize}|${s.fontFamily}|${s.fill}`;
      })).size)).toBe(1);
      expect(await page.evaluate(() => getComputedStyle(document.querySelector("#canvas .net-labels text")).fill)).not.toBe("rgb(0, 0, 0)");
      expect(parseFloat(size)).toBeGreaterThan(0);
      // no pointer events: a click on a name selects what is under it
      expect(await page.evaluate(() => document.querySelector(".net-labels").getAttribute("pointer-events"))).toBe("none");
    });

    test("another level has the names of its own, and they stay with the level when it is shown again", async ({ page, viewer }) => {
      await viewer.open("usage-current", { platform });
      await page.evaluate(() => window.viewer.command("toggle-net-labels"));
      const top = (await wanted(page)).length;
      await expect(labels(page)).toHaveCount(top);
      await page.evaluate(async () => (await import("./subcircuits.js")).goTo(1));
      const below = (await wanted(page)).length;
      expect(below).not.toBe(top);
      await expect(labels(page)).toHaveCount(below);
      await page.evaluate(async () => (await import("./subcircuits.js")).goTo(0));
      await expect(labels(page)).toHaveCount(top);
    });

    test("the key N shows and hides them, but not while typing", async ({ page, viewer }) => {
      await viewer.open("04-hierarchy", { platform });
      await page.locator("#canvas").click({ position: { x: 5, y: 5 } });
      await page.keyboard.press("n");
      expect(await page.evaluate(() => window.viewer.checked("toggle-net-labels"))).toBe(true);
      await expect(labels(page).first()).toBeAttached();
      await page.keyboard.press("n");
      expect(await page.evaluate(() => window.viewer.checked("toggle-net-labels"))).toBe(false);
      // in the editor, N is a letter
      await page.keyboard.press("Control+q");
      await expect(page.locator("#source textarea:visible")).toBeFocused();
      await page.keyboard.type("nlNL");
      expect(await page.evaluate(() => window.viewer.checked("toggle-net-labels"))).toBe(false);
      expect(await page.evaluate(() => window.viewer.checked("toggle-flylines"))).toBe(false);
      // and with Ctrl it is the browser's: nothing here
      await viewer.open("04-hierarchy", { platform });
      await page.locator("#canvas").click({ position: { x: 5, y: 5 } });
      await page.keyboard.press("Control+n");
      expect(await page.evaluate(() => window.viewer.checked("toggle-net-labels"))).toBe(false);
    });
  });
}

test("the Keyboard Shortcuts dialog lists the keys G, L and N", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy");
  await page.evaluate(() => window.viewer.command("shortcuts"));
  await expect(page.getByRole("dialog", { name: "Keyboard Shortcuts" })).toContainText(
    "G: Show Grid. L: Show Flylines. N: Show Net Names.",
  );
});
