import { test, expect } from "../../tests/viewer/node_modules/@playwright/test/index.mjs";
import { mkdirSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

const review = process.env.RNL_EMBED_REVIEW_DIR;
const pageErrors = new WeakMap();
const label = lang => lang === "zh" ? "打开 logo 的电路" : "Open the logo's circuit";

test.beforeEach(async ({ page }) => {
  pageErrors.set(page, []);
  page.on("pageerror", error => pageErrors.get(page).push(error.message));
  await page.addInitScript(() => {
    window.logoMessages = [];
    window.addEventListener("message", event => {
      const frame = document.querySelector("#hero-logo-panel iframe");
      if (frame && event.source === frame.contentWindow && event.origin === new URL(frame.src).origin && event.data?.rnl === 1) {
        window.logoMessages.push(event.data);
      }
    });
  });
});

test.afterEach(async ({ page }) => {
  expect(pageErrors.get(page)).toEqual([]);
});

async function loaded(page) {
  await page.waitForFunction(() => window.logoMessages.some(message => message.type === "loaded"));
  const message = await page.evaluate(() => window.logoMessages.find(message => message.type === "loaded"));
  expect(message).toMatchObject({ rnl: 1, name: "rnl-logo.sp", errors: 0, warnings: 0 });
  const frame = page.frames().find(frame => frame.url().includes("embed=1"));
  expect(frame).toBeTruthy();
  await expect(frame.locator("#canvas > svg")).toBeVisible();
  await expect(frame.locator("#canvas > svg text").filter({ hasText: /^RNL$/ })).toBeVisible();
  await expect(frame.locator("#canvas > svg text").filter({ hasText: /^logo by RNL$/ })).toBeVisible();
  await frame.evaluate(() => document.fonts.ready);
  return frame;
}

async function capture(page, name, project) {
  if (!review || project !== "chromium") return;
  mkdirSync(review, { recursive: true });
  await page.evaluate(() => document.fonts.ready);
  await page.mouse.move(0, 0);
  await page.screenshot({ path: resolve(review, name), animations: "disabled" });
}

async function documentTop(locator) {
  return locator.evaluate(element => element.getBoundingClientRect().top + scrollY);
}

async function brandReady(page) {
  await page.evaluate(async () => {
    await Promise.all([600, 700].map(weight => document.fonts.load(`${weight} 172px "IBM Plex Sans Condensed"`)));
    await document.fonts.ready;
    await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
  });
}

for (const lang of ["zh", "en"]) {
  test(`${lang}: click lazily opens the logo circuit, closes by Escape and ×, and reuses its iframe`, async ({ page }, info) => {
    const requests = [];
    page.on("request", request => requests.push(request.url()));
    await page.emulateMedia({ colorScheme: "light" });
    await page.goto(`/${lang}/`);
    await brandReady(page);
    const button = page.locator("#hero-logo");
    const panel = page.locator("#hero-logo-panel");
    await expect(button).toHaveAttribute("title", label(lang));
    await expect(button).toHaveAttribute("aria-expanded", "false");
    await expect(panel).toBeHidden();
    await expect(panel.locator("iframe")).toHaveCount(0);
    expect(requests.filter(url => new URL(url).pathname.includes("/viewer/"))).toEqual([]);
    const beforeRequests = [...requests];
    const beforeDemo = await documentTop(page.locator("#hero-demo"));
    const beforeThesis = await documentTop(page.locator(".thesis"));
    const beforeHeading = await page.locator("#hero-title").boundingBox();
    if (lang === "zh") await capture(page, "home-before.png", info.project.name);
    await button.click();
    const frame = await loaded(page);
    await expect(button).toHaveAttribute("aria-expanded", "true");
    await expect(panel).toBeVisible();
    await expect(page.locator("#hero-title")).toBeHidden();
    await expect(page.locator(".wordmark")).toBeHidden();
    const replacement = await panel.boundingBox();
    for (const key of ["x", "y", "width", "height"]) expect(replacement[key]).toBeCloseTo(beforeHeading[key], 1);
    expect(replacement.width / replacement.height).toBeGreaterThan(2);
    const iframeUrl = new URL(frame.url());
    expect(iframeUrl.searchParams.get("url")).toBe("samples/brand/rnl-logo.sp");
    expect(iframeUrl.searchParams.get("grid")).toBe("off");
    expect(iframeUrl.searchParams.get("origin")).toBe(new URL(page.url()).origin);
    await expect(frame.locator("body")).toHaveAttribute("data-background", "light");
    expect(await frame.evaluate(() => window.viewer.grid.shown)).toBe(false);
    expect(await documentTop(page.locator(".thesis"))).toBeCloseTo(beforeThesis, 1);
    expect(await documentTop(page.locator("#hero-demo"))).toBeCloseTo(beforeDemo, 1);
    const textSizes = await frame.evaluate(() => [...document.querySelectorAll("#canvas > svg text")]
      .map(text => ({ name: text.textContent, size: parseFloat(getComputedStyle(text).fontSize) })));
    expect(textSizes.find(text => text.name === "RNL").size).toBeGreaterThan(textSizes.find(text => text.name === "logo by RNL").size * 5);
    const circuitRight = await frame.locator("#canvas > svg > g[data-id]").evaluateAll(groups =>
      Math.max(...groups.filter(group => !group.querySelector("text")).map(group => group.getBoundingClientRect().right)));
    for (const text of await frame.locator("#canvas > svg text").all()) {
      expect(await text.evaluate(element => element.getBoundingClientRect().left)).toBeGreaterThan(circuitRight);
    }
    await frame.evaluate(() => { window.logoMarker = "reused"; });
    if (lang === "zh") {
      await capture(page, "home-after.png", info.project.name);
      if (review && info.project.name === "chromium") {
        const summary = {
          beforeClick: { totalRequests: beforeRequests.length, viewerRequests: [] },
          afterClick: { totalRequests: requests.length, viewerRequests: requests.filter(url => new URL(url).pathname.includes("/viewer/")) },
          addedRequests: requests.slice(beforeRequests.length),
        };
        writeFileSync(resolve(review, "homepage-network.json"), JSON.stringify(summary, null, 2) + "\n");
      }
    }
    // The iframe has focus: Escape must cross the browsing-context boundary on the same-origin site.
    await frame.locator("#canvas").focus();
    await page.keyboard.press("Escape");
    await expect(panel).toBeHidden();
    await expect(page.locator("#hero-title")).toBeVisible();
    await expect(button).toBeFocused();
    expect(await documentTop(page.locator("#hero-demo"))).toBeCloseTo(beforeDemo, 1);
    await button.click();
    expect(await frame.evaluate(() => window.logoMarker)).toBe("reused");
    await expect(panel.locator("iframe")).toHaveCount(1);
    await panel.getByRole("button", { name: lang === "zh" ? "关闭" : "Close", exact: true }).click();
    await expect(panel).toBeHidden();
    await expect(button).toBeFocused();
    // Resizing while hidden must refit on reopening, without recreating or reloading the iframe.
    await page.setViewportSize({ width: 390, height: 1100 });
    await button.click();
    await expect.poll(() => frame.locator("#canvas > svg text").evaluateAll(texts => texts.every(text => {
      const box = text.getBoundingClientRect();
      return box.left >= 0 && box.right <= innerWidth && box.top >= 0 && box.bottom <= innerHeight;
    }))).toBe(true);
    await page.keyboard.press("Escape");
    await expect(panel).toBeHidden();
    await expect(button).toBeFocused();
    expect(await page.evaluate(() => window.logoMessages.filter(message => message.type === "loaded").length)).toBe(1);
    expect(requests.filter(url => new URL(url).pathname === "/viewer/")).toHaveLength(1);
  });

  test(`${lang}: system and manual themes stay synchronized, including while closed`, async ({ page }, info) => {
    await page.emulateMedia({ colorScheme: "dark" });
    await page.goto(`/${lang}/`);
    await page.getByRole("button", { name: label(lang), exact: true }).click();
    const frame = await loaded(page);
    await expect(frame.locator("body")).toHaveAttribute("data-background", "dark");
    if (lang === "zh") await capture(page, "home-dark.png", info.project.name);
    await page.emulateMedia({ colorScheme: "light" });
    await expect(frame.locator("body")).toHaveAttribute("data-background", "light");
    await page.locator("#theme-btn").click();
    await expect(frame.locator("body")).toHaveAttribute("data-background", "dark");
    await page.emulateMedia({ colorScheme: "dark" });
    await page.emulateMedia({ colorScheme: "light" });
    await expect(frame.locator("body")).toHaveAttribute("data-background", "dark");
    await page.locator("#hero-logo-close").click();
    await page.locator("#theme-btn").click();
    await page.getByRole("button", { name: label(lang), exact: true }).click();
    await expect(frame.locator("body")).toHaveAttribute("data-background", "light");
  });

  test(`${lang}: the short viewer replaces the same row at 390px and keeps its controls usable`, async ({ page }, info) => {
    await page.setViewportSize({ width: 390, height: 1100 });
    await page.emulateMedia({ colorScheme: "light" });
    await page.goto(`/${lang}/`);
    await brandReady(page);
    const heading = await page.locator("#hero-title").boundingBox();
    await page.getByRole("button", { name: label(lang), exact: true }).click();
    const frame = await loaded(page);
    const panel = await page.locator("#hero-logo-panel").boundingBox();
    expect(panel.height).toBeCloseTo(heading.height, 1);
    expect(panel.width).toBeCloseTo(heading.width, 1);
    expect(panel.height).toBeLessThan(130);
    expect(panel.x + panel.width).toBeLessThanOrEqual(390);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    await expect(page.locator("#hero-logo-close")).toBeInViewport();
    await frame.locator("#canvas").focus();
    for (const button of await frame.locator(".embed-toolbar button").all()) {
      expect(await button.evaluate(element => {
        const box = element.getBoundingClientRect();
        return box.top >= 0 && box.bottom <= innerHeight && box.right <= innerWidth && getComputedStyle(element.parentElement).opacity === "1";
      })).toBe(true);
    }
    await frame.locator("#embed-zoom-in").click();
    await frame.locator("#embed-fit").click();
    await page.locator("#hero-logo-close").focus();
    if (lang === "zh") await capture(page, "home-narrow.png", info.project.name);
  });
}

test("a theme switch during core startup is applied when ready arrives", async ({ page }) => {
  let release;
  let requested;
  const held = new Promise(resolve => { release = resolve; });
  const waiting = new Promise(resolve => { requested = resolve; });
  await page.route("**/rnl_runtime_wasm.wasm", async route => {
    requested();
    await held;
    await route.continue();
  });
  await page.emulateMedia({ colorScheme: "light" });
  await page.goto("/en/");
  await page.getByRole("button", { name: label("en"), exact: true }).click();
  await waiting;
  await page.locator("#theme-btn").click();
  release();
  const frame = await loaded(page);
  await expect(frame.locator("body")).toHaveAttribute("data-background", "dark");
});
