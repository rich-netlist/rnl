// The page tests run in Chromium, Firefox and WebKit. In CI they run in Playwright's own browsers,
// which the workflow installs. Elsewhere the chromium project drives the installed Google Chrome,
// or the channel RNL_CHANNEL names, so that no Chromium is downloaded; the firefox and webkit
// projects need Playwright's builds of theirs. Two workers at most run at a time, as every one
// starts a browser of its own.
//
// The web viewer's smoke tests (specs/web.spec.js) need its site: Playwright builds it with the
// samples and serves it before the tests, on the port RNL_WEB_PORT names or a free one, and the
// tests find it at RNL_WEB_URL. A command line that names spec files, none of them web.spec.js,
// such as `npx playwright test specs/text.spec.js`, builds and serves no site, as the page's own
// tests need neither it nor the WebAssembly target. RNL_WEB=0 leaves the site out in any case, and
// the web viewer's tests skip; RNL_WEB=1 builds it in any case.

import net from "node:net";
import { defineConfig, devices } from "@playwright/test";
import { root } from "./fixtures.js";

const ci = !!process.env.CI;
const channel = process.env.RNL_CHANNEL || (ci ? undefined : "chrome");
// the size of the desktop viewer's window
const viewport = { width: 1200, height: 800 };

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
  return specs.length > 0 && !specs.some((spec) => /web\.spec\.js/.test(spec));
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
        command: `node apps/web/build.mjs --samples && node apps/web/serve.mjs --port ${process.env.RNL_WEB_PORT}`,
        cwd: root,
        url: `${process.env.RNL_WEB_URL}/about.json`,
        // building the core from nothing takes minutes
        timeout: 15 * 60 * 1000,
        stdout: "ignore",
        stderr: "pipe",
      }
    : undefined,
  projects: [
    { name: "chromium", use: { ...devices["Desktop Chrome"], channel, viewport } },
    { name: "firefox", use: { ...devices["Desktop Firefox"], viewport } },
    { name: "webkit", use: { ...devices["Desktop Safari"], viewport } },
  ],
});
