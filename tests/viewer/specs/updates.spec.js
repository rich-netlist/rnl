// Help > Check for Updates, which only the desktop offers: the newest release of the viewer that
// GitHub lists against the version that runs, with GitHub's answer faked. A newer release is offered
// with a button that opens its page; the same version is the latest; no answer, an error or no
// release of the viewer gives a link to the releases' page. GitHub is asked nothing until the command
// runs, and the web never asks.

import { expect, test } from "../viewer.js";

const API = "https://api.github.com/repos/rich-netlist/rnl/releases?per_page=30";
const RELEASES = "https://github.com/rich-netlist/rnl/releases";

/** A release as GitHub's API lists it, with what the page reads of it. */
function release(tag, more = {}) {
  return { tag_name: tag, name: tag, draft: false, prerelease: false, html_url: `${RELEASES}/tag/${tag}`, ...more };
}

/** The repository's releases, newest first: a draft and a prerelease of the viewer, its releases and the standard's. */
const LISTED = [
  release("viewer-v0.4.0", { draft: true }),
  release("viewer-v0.3.0-rc.1", { prerelease: true }),
  release("viewer-v0.2.0"),
  release("spec-0.2-r1"),
  release("viewer-v0.1.0"),
];

/** Answer the page's requests to GitHub's API with `answer(route)`; resolves to the URLs asked for, as they come. */
async function github(page, answer) {
  const asked = [];
  await page.route(
    (url) => url.hostname === "api.github.com",
    (route) => {
      asked.push(route.request().url());
      return answer(route);
    },
  );
  return asked;
}

/** An answer of GitHub's API: JSON, which any page may read with the rate limit's headers, as GitHub allows (CORS). */
function json(body, status = 200, headers = {}) {
  return (route) =>
    route.fulfill({
      status,
      contentType: "application/json; charset=utf-8",
      headers: {
        "access-control-allow-origin": "*",
        "access-control-expose-headers": "X-RateLimit-Limit, X-RateLimit-Remaining, X-RateLimit-Reset",
        ...headers,
      },
      body: JSON.stringify(body),
    });
}

/** Choose Help > Check for Updates in the desktop's native menu. */
const checkForUpdates = (page) => page.evaluate(() => window.fakeShell.menu("check-updates"));

test.describe("on the desktop", () => {
  test("a newer release is offered, and its button opens the release's page", async ({ page, viewer }) => {
    const asked = await github(page, json(LISTED));
    await viewer.open("04-hierarchy", { platform: "desktop" });
    expect(asked).toEqual([]);

    // chosen again while it looks, it does nothing more
    await checkForUpdates(page);
    await checkForUpdates(page);
    const dialog = page.locator("dialog.dialog");
    await expect(dialog.locator(".dialog-title")).toHaveText("Check for Updates");
    await expect(dialog.locator(".dialog-name")).toHaveText("RNL Viewer 0.2.0 is available.");
    await expect(dialog).toContainText("This is version 0.1.0.");
    await expect(dialog.getByRole("button")).toHaveText(["Cancel", "Open Release Page"]);
    expect(asked).toEqual([API]);
    expect(await viewer.calls("version")).toEqual([[]]);
    await dialog.getByRole("button", { name: "Open Release Page" }).click();
    await expect(dialog).toHaveCount(0);
    expect(await viewer.calls("browse")).toEqual([[`${RELEASES}/tag/viewer-v0.2.0`]]);

    // Cancel, or Escape, opens nothing
    await checkForUpdates(page);
    await dialog.getByRole("button", { name: "Cancel" }).click();
    await expect(dialog).toHaveCount(0);
    await checkForUpdates(page);
    await expect(dialog).toHaveCount(1);
    await page.keyboard.press("Escape");
    await expect(dialog).toHaveCount(0);
    expect(asked).toEqual([API, API, API]);
    expect(await viewer.calls("browse")).toHaveLength(1);
    await expect(page.locator("#level-name")).toHaveText("04-hierarchy.sp");
  });

  test("the same version is the latest, and a later one is newer than the latest release", async ({ page, viewer }) => {
    await github(page, json(LISTED));
    const dialog = page.locator("dialog.dialog");
    await viewer.open("04-hierarchy", { platform: "desktop", version: "0.2.0" });
    await checkForUpdates(page);
    await expect(dialog.locator(".dialog-name")).toHaveText("RNL Viewer 0.2.0 is the latest version.");
    await expect(dialog.getByRole("button")).toHaveText(["Close"]);
    await page.keyboard.press("Enter");
    await expect(dialog).toHaveCount(0);

    await viewer.open("04-hierarchy", { platform: "desktop", version: "0.3.0-dev.1" });
    await checkForUpdates(page);
    await expect(dialog.locator(".dialog-name")).toHaveText("RNL Viewer 0.3.0-dev.1 is newer than the latest release, 0.2.0.");
    await dialog.getByRole("button", { name: "Close" }).click();
    await expect(dialog).toHaveCount(0);
    expect(await viewer.calls("browse")).toEqual([]);
  });

  test("without an answer, with an error, or with no release of the viewer, it links to the releases", async ({ page, viewer }) => {
    const lookup = "The latest version could not be looked up: ";
    const cases = [
      [(route) => route.abort("internetdisconnected"), `${lookup}GitHub could not be reached.`],
      [json({ message: "Server Error" }, 500), new RegExp(`^${lookup}GitHub answered 500\\b.*\\.$`)],
      [
        json({ message: "API rate limit exceeded" }, 403, { "x-ratelimit-remaining": "0" }),
        `${lookup}GitHub takes no more requests from this network for a while.`,
      ],
      [json({ message: "Not Found" }), `${lookup}GitHub's answer is not a list of releases.`],
      [json([release("spec-0.2-r1"), release("viewer-v0.5.0", { prerelease: true })]), "GitHub lists no release of RNL Viewer yet."],
    ];
    let answer = null;
    await github(page, (route) => answer(route));
    await viewer.open("04-hierarchy", { platform: "desktop" });
    const dialog = page.locator("dialog.dialog");
    const page0 = page.url();
    for (const [given, text] of cases) {
      answer = given;
      await checkForUpdates(page);
      await expect(dialog.locator("p").first()).toHaveText(text);
      const link = dialog.getByRole("link", { name: "github.com/rich-netlist/rnl/releases" });
      await expect(link).toHaveAttribute("href", RELEASES);
      // the link opens the releases in the default browser, by the mouse or by Enter, and the page stays
      const opened = (await viewer.calls("browse")).length;
      await link.click();
      await link.focus();
      await page.keyboard.press("Enter");
      await expect.poll(async () => (await viewer.calls("browse")).slice(opened)).toEqual([[RELEASES], [RELEASES]]);
      await expect(dialog).toHaveCount(1);
      expect(page.url()).toBe(page0);
      await dialog.getByRole("button", { name: "Close" }).click();
      await expect(dialog).toHaveCount(0);
    }
  });
});

test("the web offers no Check for Updates, and asks GitHub nothing", async ({ page, viewer }) => {
  const asked = await github(page, json(LISTED));
  await viewer.open("04-hierarchy");
  await viewer.menuTitle("Help").click();
  const help = page.locator("#menubar .menu", { has: page.locator(".menu-title", { hasText: "Help" }) });
  await expect(help.locator(".menu-item, .menu-separator")).toHaveText(["About RNL Viewer"]);
  await page.keyboard.press("Escape");
  await page.evaluate(() => window.viewer.command("check-updates"));
  await expect(page.locator("dialog.dialog")).toHaveCount(0);
  expect(asked).toEqual([]);
});

test("versions are ordered as Semantic Versioning orders them, and the newest release is the highest", async ({ page, viewer }) => {
  await viewer.open(null, { platform: "desktop" });
  const got = await page.evaluate(async () => {
    const { compareVersions, newestRelease, parseVersion } = await import("./updates.js");
    // in order, as the examples of Semantic Versioning 2.0.0 have them
    const texts = [
      "0.9.0", "0.10.0", "1.0.0-alpha", "1.0.0-alpha.1", "1.0.0-alpha.beta", "1.0.0-beta", "1.0.0-beta.2",
      "1.0.0-beta.11", "1.0.0-rc.1", "1.0.0", "1.0.1", "1.1.0", "2.0.0",
    ];
    const wrong = [];
    for (const [i, a] of texts.entries()) {
      for (const [j, b] of texts.entries()) {
        if (Math.sign(compareVersions(parseVersion(a), parseVersion(b))) !== Math.sign(i - j)) wrong.push([a, b]);
      }
    }
    const newest = newestRelease([
      { tag_name: "viewer-v0.1.5" },
      { tag_name: "viewer-v0.2.0" },
      { tag_name: "viewer-v0.10.0", draft: true },
      { tag_name: "viewer-v0.9.0", prerelease: true },
      { tag_name: "spec-0.2-r1" },
      { tag_name: "viewer-vnext" },
    ]);
    return {
      wrong,
      build: compareVersions(parseVersion("1.0.0+build.5"), parseVersion("1.0.0")),
      invalid: ["1.0", "v1.0.0", "1.0.0-", "01.0.0", "1.0.0-beta..1", ""].map((t) => parseVersion(t)),
      newest: newest && [newest.version, newest.url],
      none: newestRelease([{ tag_name: "spec-0.2-r1" }, { tag_name: "viewer-v1.0.0", draft: true }]),
    };
  });
  expect(got.wrong).toEqual([]);
  expect(got.build).toBe(0);
  expect(got.invalid).toEqual([null, null, null, null, null, null]);
  expect(got.newest).toEqual(["0.2.0", `${RELEASES}/tag/viewer-v0.2.0`]);
  expect(got.none).toBeNull();
});
