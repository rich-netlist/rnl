// The page tests run in Chromium, Firefox and WebKit. In CI they run in Playwright's own browsers,
// which the workflow installs. Elsewhere the chromium project drives the installed Google Chrome,
// or the channel RNL_CHANNEL names, so that no Chromium is downloaded; the firefox and webkit
// projects need Playwright's builds of theirs. Two workers at most run at a time, as every one
// starts a browser of its own.

import { defineConfig, devices } from "@playwright/test";

const ci = !!process.env.CI;
const channel = process.env.RNL_CHANNEL || (ci ? undefined : "chrome");
// the size of the desktop viewer's window
const viewport = { width: 1200, height: 800 };

export default defineConfig({
  testDir: "specs",
  globalSetup: "./global-setup.js",
  fullyParallel: true,
  workers: 2,
  forbidOnly: ci,
  retries: 0,
  reporter: ci ? [["list"], ["html", { open: "never" }]] : "list",
  use: { trace: "retain-on-failure" },
  projects: [
    { name: "chromium", use: { ...devices["Desktop Chrome"], channel, viewport } },
    { name: "firefox", use: { ...devices["Desktop Firefox"], viewport } },
    { name: "webkit", use: { ...devices["Desktop Safari"], viewport } },
  ],
});
