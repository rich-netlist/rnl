import { test, expect } from "../../tests/viewer/node_modules/@playwright/test/index.mjs";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";

const review = process.env.RNL_BRAND_SCREENSHOTS;

for (const lang of ["zh", "en"]) {
  test(`${lang}: logo follows system and manual themes and fits 390px`, async ({ page }, info) => {
    await page.emulateMedia({ colorScheme: "light" });
    await page.goto(`/${lang}/`);
    const light = page.locator(".hero-logo .logo-light"), dark = page.locator(".hero-logo .logo-dark");
    async function navTheme(theme) {
      await expect(page.locator(`.brand-logo .logo-${theme}`)).toBeVisible();
      await expect(page.locator(`.brand-logo .logo-${theme === "light" ? "dark" : "light"}`)).toBeHidden();
    }
    await expect(light).toBeVisible();
    await expect(dark).toBeHidden();
    await navTheme("light");
    await page.evaluate(() => document.fonts.ready);
    await expect(page.locator("#hero-title")).toHaveText("RNL");
    for (const image of await page.locator(".theme-logo img").all()) {
      await expect(image).toHaveAttribute("alt", "");
      expect(await image.evaluate(el => el.complete && el.naturalWidth > 0)).toBe(true);
    }
    async function capture(name) {
      if (!review || info.project.name !== "chromium") return;
      mkdirSync(review, { recursive: true });
      await page.screenshot({ path: resolve(review, `${lang}-home-${name}.png`), animations: "disabled" });
    }
    await capture("light-1440");
    await page.emulateMedia({ colorScheme: "dark" });
    await expect(light).toBeHidden();
    await expect(dark).toBeVisible();
    await navTheme("dark");
    await capture("dark-1440");
    // Manual light overrides a dark system, then manual dark overrides a light system.
    await page.locator("#theme-btn").click();
    await expect(light).toBeVisible();
    await navTheme("light");
    await page.emulateMedia({ colorScheme: "light" });
    await page.locator("#theme-btn").click();
    await expect(dark).toBeVisible();
    await navTheme("dark");
    await page.reload();
    await expect(dark).toBeVisible();
    await navTheme("dark");
    await page.setViewportSize({ width: 390, height: 844 });
    await capture("dark-390");
    await page.locator("#theme-btn").click();
    await expect(light).toBeVisible();
    await navTheme("light");
    await capture("light-390");
    const fits = await page.locator("#hero-title").evaluate(el => {
      const logo = el.querySelector(".hero-logo").getBoundingClientRect();
      const text = el.querySelector(".wordmark").getBoundingClientRect();
      return Math.abs(text.left - logo.right) < 1 && logo.top < text.bottom && logo.bottom > text.top &&
        text.right <= innerWidth && document.documentElement.scrollWidth <= innerWidth;
    });
    expect(fits).toBe(true);
    for (const width of [641, 760, 761, 768, 960, 961, 1000, 1001]) {
      await page.setViewportSize({ width, height: 1000 });
      expect(await page.locator(".topbar-in").evaluate(el =>
        document.documentElement.scrollWidth <= innerWidth && [...el.children]
          .filter(child => getComputedStyle(child).display !== "none")
          .every(child => child.getBoundingClientRect().right <= innerWidth)),
      `${lang} navigation should fit at ${width}px`).toBe(true);
    }
  });
}

test("built favicon URLs resolve for both languages, standalone pages, viewer and standard", async ({ page, request }) => {
  const pages = ["index.html", "language.html", "tools.html", "packages.html", "gallery.html", "agents.html", "timeline.html", "translation.html",
    "rnl-intro-standalone.html", "rnl-language-standalone.html", "rnl-tools-standalone.html",
    "spec/0.2/index.html", "spec/0.3/index.html", "spec/0.3/compare-0.2.html"];
  for (const path of [...["zh", "en"].flatMap(lang => pages.map(p => `/${lang}/${p}`)), "/viewer/"]) {
    const response = await request.get(path);
    expect(response.ok(), path).toBe(true);
    const html = await response.text();
    const links = [...html.matchAll(/<link\b[^>]*rel="(?:icon|apple-touch-icon)"[^>]*>/g)].map(m => m[0]);
    expect(links.length, path).toBeGreaterThanOrEqual(2);
    for (const link of links) {
      const href = link.match(/href="([^"]+)"/)[1];
      if (href.startsWith("data:")) {
        expect(href).toMatch(/^data:image\/(svg\+xml|png);base64,/);
        continue;
      }
      // Resolve as deployed under /rnl/, then request that same asset in the local site build.
      const url = new URL(href, `https://rich-netlist.github.io/rnl${path}`);
      expect(url.pathname).toMatch(/^\/rnl\/(?:zh|en|viewer)\//);
      const icon = await request.get(url.pathname.slice("/rnl".length));
      expect(icon.ok(), `${path}: ${href}`).toBe(true);
      expect(icon.headers()["content-type"]).toMatch(/^image\//);
    }
  }
  // Standalone HTML embeds the homepage images as well as the favicons.
  await page.goto("/en/rnl-intro-standalone.html");
  for (const image of await page.locator(".theme-logo img").all()) {
    await expect(image).toHaveAttribute("src", /^data:image\/svg\+xml;base64,/);
  }
});
