// The page tests run in Chromium, Firefox and WebKit. In CI they run in Playwright's own browsers,
// which the workflow installs. Elsewhere the chromium project drives the installed Google Chrome,
// or the channel RNL_CHANNEL names, so that no Chromium is downloaded; the firefox and webkit
// projects need Playwright's builds of theirs. Two workers at most run at a time, as every one
// starts a browser of its own.
//
// specs/long.spec.js, a file of 200 000 lines, runs in a project of its own, long, in Chromium only:
// what it guards against is Chrome's limit on the arguments of a call, and in the other engines it
// would only add load. It depends on the three others, so it runs after them and alone, and does not
// slow down their tests on a busy machine. `--project long --no-deps` runs it on its own.
//
// The mobile tests, specs/mobile*.spec.js, run in projects of their own, on a phone: mobile-chromium
// as a Pixel 7 in Chromium, whose touch goes through the DevTools protocol, and mobile-webkit as an
// iPhone 15 in WebKit; a test may hold the phone lying down, or be a tablet. The three browsers'
// projects leave them out, and they leave out the rest.
//
// The web viewer's smoke tests (specs/web.spec.js) need its site: Playwright builds it with the
// samples and serves it before the tests, on the port RNL_WEB_PORT names or a free one, and the
// tests find it at RNL_WEB_URL. A command line that names spec files, none of them web.spec.js,
// such as `npx playwright test specs/text.spec.js`, builds and serves no site, as the page's own
// tests need neither it nor the WebAssembly target. RNL_WEB=0 leaves the site out in any case, and
// the web viewer's tests skip; RNL_WEB=1 builds it in any case.

import net from "node:net";
import { preparePublicPackageFixture } from "../../scripts/packages.mjs";
import { defineConfig, devices } from "@playwright/test";
import { root } from "./fixtures.js";

process.env.RNL_PACKAGE_FIXTURE ||= preparePublicPackageFixture();

const ci = !!process.env.CI;
const channel = process.env.RNL_CHANNEL || undefined;
// the size of the desktop viewer's window
const viewport = { width: 1200, height: 800 };
// the file of the long netlist's test, which runs in a project of its own
const LONG = /long\.spec\.js$/;
// the files of the mobile tests, which run in projects of their own
const MOBILE = /mobile[^/\\]*\.spec\.js$/;

/** A port on 127.0.0.1 that nothing listens on. */
function freePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const { port } = server.address();
      server.close(() => resolve(port));
    });
  });
}

/** Whether the command line names spec files, and none of them is the web viewer's. */
function pageSpecsOnly() {
  const specs = process.argv.filter((arg) => /\.spec\.js(:\d+)*$/.test(arg));
  return specs.length > 0 && !specs.some((spec) => /(?:web|embed|api)\.spec\.js/.test(spec));
}

// the workers load this file too, and inherit whether the runner builds the site, and its port
process.env.RNL_WEB ??= pageSpecsOnly() ? "0" : "1";
const web = process.env.RNL_WEB !== "0";
if (web) {
  process.env.RNL_WEB_PORT ||= String(await freePort());
  process.env.RNL_WEB_URL = `http://127.0.0.1:${process.env.RNL_WEB_PORT}`;
}

export default defineConfig({
  testDir: "specs",
  globalSetup: "./global-setup.js",
  fullyParallel: true,
  workers: 2,
  forbidOnly: ci,
  retries: 0,
  reporter: ci ? [["list"], ["html", { open: "never" }]] : "list",
  use: { trace: "retain-on-failure" },
  webServer: web
    ? {
        command: `node apps/web/build.mjs --samples ${process.env.RNL_WEB_NO_CORE ? "--no-core" : ""} && node apps/web/serve.mjs --port ${process.env.RNL_WEB_PORT}`,
        cwd: root,
        url: `${process.env.RNL_WEB_URL}/about.json`,
        // building the core from nothing takes minutes
        timeout: 15 * 60 * 1000,
        stdout: "ignore",
        stderr: "pipe",
      }
    : undefined,
  projects: [
    { name: "chromium", use: { ...devices["Desktop Chrome"], channel, viewport }, testIgnore: [LONG, MOBILE] },
    { name: "firefox", use: { ...devices["Desktop Firefox"], viewport }, testIgnore: [LONG, MOBILE, /embed\.spec\.js$/] },
    { name: "webkit", use: { ...devices["Desktop Safari"], viewport }, testIgnore: [LONG, MOBILE] },
    { name: "mobile-chromium", use: { ...devices["Pixel 7"], channel }, testMatch: MOBILE },
    { name: "mobile-webkit", use: { ...devices["iPhone 15"] }, testMatch: MOBILE },
    {
      name: "long",
      use: { ...devices["Desktop Chrome"], channel, viewport },
      testMatch: LONG,
      dependencies: ["chromium", "firefox", "webkit"],
    },
  ],
});
