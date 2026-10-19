import { test, expect } from "../../tests/viewer/node_modules/@playwright/test/index.mjs";
import { readFileSync } from "node:fs";

const index = JSON.parse(readFileSync(new URL("../dist/gallery/index.js", import.meta.url), "utf8").match(/^RNLGallery.index\((.*)\);/s)[1]);

for (const width of [1440, 390]) {
  test(`all gallery SVGs stay inside their viewBoxes and cards at ${width}px`, async ({ page }) => {
    test.setTimeout(90_000);
    await page.setViewportSize({ width, height: 1000 });
    for (const group of index.groups) {
      await page.goto(`/en/packages.html?series=${encodeURIComponent(group.id)}`);
      await expect(page.locator(".gl-card")).toHaveCount(group.counts.symbols);
      await page.evaluate(() => document.fonts.ready);
      await page.locator("#gl-main").evaluate(el => el.classList.add("show-pins", "show-box", "show-labels"));
      const origin = page.locator('[data-show="origin"]');
      if (await origin.getAttribute("aria-pressed") !== "true") await origin.click();
      await expect.poll(() => page.locator(".gl-card").evaluateAll(cards => cards.every(card => {
        const fig = card.querySelector(".gl-fig"), svg = fig.querySelector("svg"), s = getComputedStyle(fig);
        return svg.clientWidth <= fig.clientWidth - parseFloat(s.paddingLeft) - parseFloat(s.paddingRight) + 0.1
          && svg.clientHeight <= fig.clientHeight - parseFloat(s.paddingTop) - parseFloat(s.paddingBottom) + 0.1;
      }))).toBe(true);
      const outside = await page.locator(".gl-fig svg, .gl-thumb svg").evaluateAll(svgs => {
        const errors = [];
        for (const svg of svgs) {
          const vb = svg.viewBox.baseVal;
          const scale = Math.min(svg.getBoundingClientRect().width / vb.width, svg.getBoundingClientRect().height / vb.height);
          for (const el of svg.querySelectorAll("path,rect,circle,ellipse,line,polyline,polygon,text")) {
            if (!el.getClientRects().length || !el.checkVisibility()) continue;
            const local = el.getBBox(), style = getComputedStyle(el);
            const matrix = svg.getCTM().inverse().multiply(el.getCTM());
            const corners = [[local.x,local.y], [local.x+local.width,local.y+local.height]].map(([x,y]) => new DOMPoint(x,y).matrixTransform(matrix));
            const b = { x: corners[0].x, y: corners[0].y, width: corners[1].x-corners[0].x, height: corners[1].y-corners[0].y };
            const stroke = style.stroke === "none" ? 0 : parseFloat(style.strokeWidth) / 2 / (style.vectorEffect === "non-scaling-stroke" ? scale : 1);
            const tolerance = 0.1;
            if (b.x - stroke < vb.x - tolerance || b.y - stroke < vb.y - tolerance || b.x + b.width + stroke > vb.x + vb.width + tolerance || b.y + b.height + stroke > vb.y + vb.height + tolerance) {
              errors.push({ symbol: svg.closest("[data-sym]")?.dataset.sym, tag: el.tagName, b: [b.x,b.y,b.width,b.height], vb: [vb.x,vb.y,vb.width,vb.height], stroke });
            }
          }
        }
        return errors;
      });
      expect(outside, group.id).toEqual([]);
      const markerSizes = await page.locator(".gl-fig .g-origin-mark, .gl-thumb .g-origin-mark").evaluateAll(marks => marks.map(mark => {
        const m = mark.getScreenCTM();
        return { symbol: mark.closest("[data-sym]").dataset.sym, size: 10 * Math.hypot(m.a,m.b) };
      }));
      expect(markerSizes.length).toBeGreaterThanOrEqual(group.counts.symbols);
      for (const marker of markerSizes) expect(marker.size, `${group.id}: ${marker.symbol}`).toBeCloseTo(10, 2);
      const ratios = await page.locator(".gl-card").evaluateAll(cards => cards.map(card => {
        const svg = card.querySelector("svg"), fig = card.querySelector(".gl-fig"), s = getComputedStyle(fig);
        const ratio = Math.min(1, (fig.clientWidth - parseFloat(s.paddingLeft) - parseFloat(s.paddingRight)) / Number(svg.getAttribute("width")),
          (fig.clientHeight - parseFloat(s.paddingTop) - parseFloat(s.paddingBottom)) / Number(svg.getAttribute("height")));
        const badge = card.querySelector(".gl-fit");
        return { ratio, hidden: badge.hidden, badge: badge.textContent, actual: parseFloat(svg.style.width) / Number(svg.getAttribute("width")) };
      }));
      for (const row of ratios) {
        expect(row.actual).toBeCloseTo(row.ratio, 4);
        expect(row.hidden).toBe(row.ratio >= 1);
        if (!row.hidden) expect(row.badge).toBe(`Shown at ×${row.ratio.toFixed(2)}`);
      }
    }
    await page.goto("/en/packages.html?series=eda-symbol&package=eda-symbol");
    const osc = page.locator('.gl-card[data-sym="$OSC"]');
    // The previous group kept the origin setting in localStorage.
    await expect(page.locator('[data-show="origin"]')).toHaveAttribute("aria-pressed", "true");
    await expect(osc.locator(".gl-fit")).toBeVisible();
    await osc.click();
    await expect(page.locator("#gl-dlg")).toBeVisible();
    const boxes = await page.locator(".gl-dlg-fig svg").evaluate(svg => {
      const vb = svg.viewBox.baseVal, b = svg.getBBox();
      return { inside: b.x >= vb.x && b.y >= vb.y && b.x+b.width <= vb.x+vb.width && b.y+b.height <= vb.y+vb.height };
    });
    expect(boxes.inside).toBe(true);
  });
}
