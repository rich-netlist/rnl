import { test, expect } from "../../tests/viewer/node_modules/@playwright/test/index.mjs";

const pages = ["language.html", "packages.html", "gallery.html", "tools.html", "agents.html", "timeline.html", "translation.html",
  "rnl-language-standalone.html", "rnl-tools-standalone.html"];

for (const lang of ["zh", "en"]) {
  for (const width of [1440, 390]) {
    test(`${lang}: page titles keep their own typography at ${width}px`, async ({ page }) => {
      await page.setViewportSize({ width, height: 1000 });
      const titleSize = width === 1440 ? "84px" : "44px";
      for (const file of pages) {
        await page.goto(`/${lang}/${file}`);
        const title = page.locator(".tour-h1");
        await expect(title, `${file}: page title size`).toHaveCSS("font-size", titleSize);
        await expect(title).toHaveCSS("font-weight", lang === "zh" ? "900" : "700");
        await expect(title).toHaveCSS("display", "block");
        expect(await title.evaluate(el => {
          const style = getComputedStyle(el);
          return Number.parseFloat(style.lineHeight) / Number.parseFloat(style.fontSize);
        })).toBeCloseTo(1.05, 2);
      }
      // the homepage's "RNL" is the picture the viewer draws (site/render-logo.py), not text in a font of the page
      for (const file of ["index.html", "rnl-intro-standalone.html"]) {
        await page.goto(`/${lang}/${file}`);
        await expect(page.locator(".wordmark")).toHaveCount(0);
        await expect(page.locator("#hero-title")).toHaveText("RNL");
        await expect(page.locator("#hero-title .hero-logo img:visible")).toHaveCount(1);
      }
    });
  }
}
