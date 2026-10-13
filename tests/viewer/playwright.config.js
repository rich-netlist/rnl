// The page tests run in Chromium, Firefox and WebKit, as CI installs them with Playwright. Locally,
// RNL_CHANNEL names an installed browser that Playwright drives instead, "msedge" or "chrome", and
// only the chromium project runs, so that no browser is downloaded.

import { defineConfig, devices } from "@playwright/test";

const channel = process.env.RNL_CHANNEL;
// the size of the desktop viewer's window
const viewport = { width: 1200, height: 800 };
const engines = [
  { name: "chromium", use: { ...devices["Desktop Chrome"], channel, viewport } },
  { name: "firefox", use: { ...devices["Desktop Firefox"], viewport } },
  { name: "webkit", use: { ...devices["Desktop Safari"], viewport } },
];

export default defineConfig({
  testDir: "specs",
  globalSetup: "./global-setup.js",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: 0,
  reporter: process.env.CI ? [["list"], ["html", { open: "never" }]] : "list",
  use: { trace: "retain-on-failure" },
  projects: channel ? engines.slice(0, 1) : engines,
});
