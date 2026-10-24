import { publicPackagesAvailable } from "../../scripts/packages.mjs";
const hasPublicPackages = publicPackagesAvailable();
import { test, expect } from "../../tests/viewer/node_modules/@playwright/test/index.mjs";
import { readFileSync, mkdirSync } from "node:fs";
import { resolve } from "node:path";

const files = ["index.html", "language.html", "packages.html", "gallery.html", "tools.html", "agents.html"];
const index = JSON.parse(readFileSync(new URL("../dist/gallery/index.js", import.meta.url), "utf8").match(/^RNLGallery.index\((.*)\);/s)[1]);
const review = process.env.RNL_SITE_SCREENSHOTS;
const pageErrors = new WeakMap();

async function ready(page) {
  await page.evaluate(() => document.fonts.ready);
  await expect(page.locator(".topbar")).toBeVisible();
}

async function capture(page, name, project) {
  if (!review || project !== "chromium") return;
  mkdirSync(review, { recursive: true });
  await ready(page);
  await page.mouse.move(1430, 0);
  await page.screenshot({ path: resolve(review, `${name}.png`) });
}

test.beforeEach(async ({ page }) => {
  await page.route("https://rich-netlist.github.io/rnl-packages/**", route => route.abort());
  await page.route("https://rich-netlist.github.io/rnl-gallery/**", route => route.abort());
  pageErrors.set(page, []);
  page.on("pageerror", error => pageErrors.get(page).push(error.message));
  await page.addInitScript(() => localStorage.setItem("rnl-intro-theme", "light"));
});

test.afterEach(async ({ page }) => {
  expect(pageErrors.get(page)).toEqual([]);
});

for (const file of files) {
  test(`${file}: primary pages, sticky contents and section tracking`, async ({ page }, info) => {
    await page.goto(`/zh/${file}`);
    await ready(page);
    await expect(page.locator("nav.pages a")).toHaveCount(8);
    await expect(page.locator("nav.pages a").last()).toHaveText("GitHub");
    await expect(page.locator("nav.pages a").last()).toHaveAttribute("href", "https://github.com/rich-netlist/rnl");
    await expect(page.locator("nav.pages [aria-current=page]")).toHaveCount(1);
    await expect(page.locator("nav.pages [aria-current=page]")).toHaveAttribute("href", file === "index.html" ? "./" : file);
    if (file === "packages.html") {
      await expect(page.locator(".gl-group")).toHaveCount(index.groups.length);
      await expect(page.locator(".gl-card").first()).toBeVisible();
    }
    await page.locator(".layout").scrollIntoViewIfNeeded();
    // The gallery page's list is short, and empty here (its data is not served): too short to scroll the layout up under the top bar.
    if (file !== "gallery.html") {
      // Put the start of the layout immediately below the sticky top bar.
      await page.evaluate(() => window.scrollTo(0, document.querySelector(".layout").getBoundingClientRect().top + scrollY - 58));
      await expect.poll(() => page.locator("#toc").evaluate(el => Math.round(el.getBoundingClientRect().top))).toBe(59);
    }
    if (file !== "packages.html" && file !== "gallery.html") {
      const links = page.locator('#toc a[href^="#"]');
      const target = await links.nth(1).getAttribute("href");
      await links.nth(1).click();
      await expect(links.nth(1)).toHaveClass("here");
      await expect(page).toHaveURL(new RegExp(`${target}$`));
      await page.locator(target).evaluate(el => window.scrollTo(0, el.getBoundingClientRect().top + scrollY - 70));
      await expect(links.nth(1)).toHaveClass("here");
      await page.evaluate(() => window.scrollTo(0, document.querySelector(".layout").getBoundingClientRect().top + scrollY - 59));
      await expect(links.first()).toHaveClass("here");
    }
    await capture(page, `zh-light-${file.replace(".html", "")}-1440`, info.project.name);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  });
}

test("existing deep links and the full-width introduction hero remain usable", async ({ page }, info) => {
  for (const [file, hash] of [["index.html", "journey"], ["language.html", "examples"], ["tools.html", "desktop"]]) {
    await page.goto(`/zh/${file}#${hash}`);
    await expect(page.locator(`#${hash}`)).toBeInViewport();
    await expect(page).toHaveURL(new RegExp(`#${hash}$`));
  }
  await page.goto("/zh/index.html");
  const heroWidth = await page.locator(".hero").evaluate(el => el.getBoundingClientRect().width);
  expect(heroWidth).toBe(1440);
  await capture(page, "zh-light-index-hero-1440", info.project.name);
  await page.locator(".hero").getByRole("link", { name: "三分钟速览", exact: true }).click();
  await expect(page).toHaveURL(/#overview$/);
  await expect(page.locator('#toc a[href="#overview"]')).toHaveClass("here");
  await page.locator(".language-menu > summary").click();
  await page.locator('[data-language="en"]').click();
  await expect(page).toHaveURL(/\/en\/#overview$/);
});

test("gallery: directory categories, search, disclosure, packages, history and shared URLs", async ({ page }) => {
  test.skip(!hasPublicPackages, "Run python scripts/packages.py download");
  for (const g of index.groups) {
    expect(g.kind).toBe(g.dir.startsWith("libraries/standard/") ? "standard" : g.dir.startsWith("libraries/pdk/") ? "pdk" : "extension");
  }
  const versions = index.groups.filter(g => g.kind === "standard").map(g => g.series);
  expect(versions).toEqual([...versions].sort((a, b) => b.localeCompare(a, undefined, { numeric: true })));
  await page.goto("/zh/packages.html");
  await expect(page.locator(".gl-group")).toHaveCount(index.groups.length);
  await expect(page.locator('.gl-fold[aria-expanded="true"]')).toHaveCount(1);
  await expect(page.locator(".gl-readonly")).toHaveText("只读");
  const current = index.groups[0];
  const fold = page.locator(`[data-fold="${current.id}"]`);
  await fold.click();
  await expect(fold).toHaveAttribute("aria-expanded", "false");
  await fold.click();
  const packageName = "rnl-std-basic";
  const pkg = page.locator(`#gl-nav [data-group="${current.id}"][data-pkg="${packageName}"]`);
  await pkg.click();
  await expect(page.locator(".gl-pkg")).toHaveCount(1);
  await expect(pkg).toHaveClass(/here/);
  await expect(page).toHaveURL(/package=rnl-std-basic/);
  await page.reload();
  await expect(pkg).toHaveClass(/here/);
  await expect(page.locator(".gl-pkg")).toHaveCount(1);
  await page.locator(`#gl-nav [data-group="${current.id}"][data-pkg=""]`).click();
  await expect(page.locator(".gl-pkg")).toHaveCount(current.counts.packages);
  const other = index.groups.find(g => g.kind === "extension");
  await page.locator(`.gl-group[data-group="${other.id}"]`).click();
  await expect(page).toHaveURL(new RegExp(`series=${other.id}`));
  await expect(page.locator(".gl-pkg")).toHaveCount(other.counts.packages);
  await page.goBack();
  await expect(page.locator(".gl-pkg")).toHaveCount(current.counts.packages);
  await expect(page.locator(`.gl-group[data-group="${current.id}"]`)).toHaveClass(/here/);
  await page.goBack();
  await expect(pkg).toHaveClass(/here/);
  await page.goForward();
  await expect(page.locator(".gl-pkg")).toHaveCount(current.counts.packages);
  await page.locator("#gl-q").fill("RNL-STD-BASIC");
  await expect(page.locator("#gl-nav [data-pkg]").filter({ hasText: packageName })).toHaveCount(1);
  await expect(page).toHaveURL(/q=RNL-STD-BASIC/);
  await page.locator("#gl-q").fill("nmos");
  await expect(page.locator(".gl-card").first()).toBeVisible();
  expect(await page.locator(".gl-card-name").allTextContents()).not.toHaveLength(0);
  for (const title of await page.locator(".gl-card-name").allTextContents()) expect(title.toLowerCase()).toContain("nmos");
  await page.locator("#gl-q").fill("");
  await expect(page.locator(".gl-group")).toHaveCount(index.groups.length);
  await expect(page.locator(".gl-card")).toHaveCount(current.counts.symbols);
  await page.locator('[data-show="box"]').click();
  await expect(page.locator("#gl-main")).toHaveClass(/show-box/);
  await page.locator(".gl-card").first().click();
  await expect(page.locator("#gl-dlg")).toBeVisible();
  await page.locator("#gl-dlg-close").click();
  await page.goto("/zh/packages.html?series=standard-0.2&package=rnl-std-symbols&q=RNL-STD-SYMBOLS#gallery");
  await expect(page.locator('.gl-group[data-group="standard-0.2"]')).toHaveClass(/here/);
  await expect(page.locator('#gl-nav [data-group="standard-0.2"][data-pkg="rnl-std-symbols"]')).toHaveClass(/here/);
  await page.locator(".language-menu > summary").click();
  await page.locator('[data-language="en"]').click();
  await expect(page).toHaveURL(/\/en\/packages.html\?series=standard-0.2&q=RNL-STD-SYMBOLS&package=rnl-std-symbols#gallery/);
});

for (const file of ["index.html", "packages.html"]) {
  test(`${file}: narrow menu opens, scrolls and closes on selection`, async ({ page }, info) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto(`/zh/${file}`);
    await ready(page);
    if (file === "packages.html") await expect(page.locator(".gl-group")).toHaveCount(index.groups.length);
    await expect(page.locator("#toc")).toBeHidden();
    await page.locator("#toc-btn").click();
    await expect(page.locator("#toc")).toBeVisible();
    await expect.poll(() => page.locator("#toc").evaluate(el => Math.round(el.getBoundingClientRect().top))).toBe(66);
    if (file === "packages.html") await expect(page.locator("#gl-q")).toBeInViewport();
    await expect(page.locator("#toc-btn")).toHaveAttribute("aria-expanded", "true");
    await capture(page, `zh-light-${file.replace(".html", "")}-390-menu`, info.project.name);
    if (file === "packages.html") {
      await page.locator('.gl-group[data-group="standard-0.2"]').click();
      await expect(page.locator("#toc")).toBeHidden();
      await page.locator("#toc-btn").click();
      await page.locator('#gl-nav [data-group="standard-0.2"][data-pkg="rnl-std-symbols"]').click();
      await expect(page.locator(".gl-pkg")).toHaveCount(1);
    } else {
      await page.locator('#toc a[href="#what"]').click();
      await expect(page).toHaveURL(/#what$/);
    }
    await expect(page.locator("#toc")).toBeHidden();
    await expect(page.locator("#toc-btn")).toHaveAttribute("aria-expanded", "false");
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  });

  test(`${file}: English dark theme and standalone pages`, async ({ page }, info) => {
    await page.goto(`/en/${file}`);
    await page.locator("#theme-btn").click();
    await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
    if (file === "packages.html") await expect(page.locator(".gl-card").first()).toBeVisible();
    await page.evaluate(() => window.scrollTo(0, document.querySelector(".layout").getBoundingClientRect().top + scrollY - 59));
    await capture(page, `en-dark-${file.replace(".html", "")}-1440`, info.project.name);
    if (file === "index.html") {
      for (const single of ["intro", "language", "tools"]) {
        await page.goto(`/en/rnl-${single}-standalone.html`);
        await expect(page.locator("nav.pages a")).toHaveCount(8);
        await expect(page.locator("#toc")).toBeVisible();
        expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
      }
    }
  });
}

test("gallery scales to 15 groups with long names and an independently scrolling tree", async ({ page }, info) => {
  const fake = structuredClone(index);
  for (let n = fake.groups.length; n < 15; n++) fake.groups.push({
    id: `review-extension-${n}`, dir: `libraries/review-extension-${n}`, kind: "extension", series: null,
    packages: [{ name: `review-extension-${n}-with-a-very-long-package-name`, version: "0.1" }], counts: { packages: 1, symbols: 0 },
  });
  await page.route("**/gallery/index.js", route => route.fulfill({ contentType: "text/javascript", body: `RNLGallery.index(${JSON.stringify(fake)});` }));
  await page.route(/\/gallery\/review-extension-\d+\.js$/, route => {
    const id = new URL(route.request().url()).pathname.split("/").pop().replace(".js", "");
    const meta = fake.groups.find(g => g.id === id);
    const pkg = { ...meta.packages[0], language: "0.3", description: "Review fixture", file: "review.rnl", path: `${meta.dir}/review.rnl`, text: "", statements: [], references: [], referencedBy: [], expansion: [] };
    return route.fulfill({ contentType: "text/javascript", body: `RNLGallery.group(${JSON.stringify({ ...meta, packages: [pkg] })});` });
  });
  await page.goto("/zh/packages.html");
  await expect(page.locator(".gl-group")).toHaveCount(15);
  await expect(page.locator(".gl-card").first()).toBeVisible();
  await page.evaluate(() => window.scrollTo(0, document.querySelector(".layout").getBoundingClientRect().top + scrollY - 59));
  const last = page.locator('.gl-group[data-group="review-extension-14"]');
  await expect(last).toHaveAttribute("title", "review-extension-14-with-a-very-long-package-name 0.1");
  expect(await last.locator(".gl-nav-name").evaluate(el => el.scrollWidth > el.clientWidth)).toBe(true);
  expect(await page.locator("#toc").evaluate(el => el.scrollHeight > el.clientHeight)).toBe(true);
  await page.locator(`[data-fold="${index.groups[0].id}"]`).click();
  await page.locator("#toc").evaluate(el => { el.scrollTop = 0; });
  await capture(page, "zh-light-packages-15-groups-1440", info.project.name);
  await last.click();
  await expect(page.locator(".gl-pkg")).toHaveCount(1);
  await expect(page).toHaveURL(/series=review-extension-14/);
  await page.locator("#gl-q").fill("REVIEW-EXTENSION-14");
  await expect(page.locator(".gl-group")).toHaveCount(1);
  await page.locator("#gl-q").fill("");
  await expect(page.locator(".gl-group")).toHaveCount(15);
});
