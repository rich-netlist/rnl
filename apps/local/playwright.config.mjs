// The browser tests of the local program: Playwright from tests/viewer, which has it, on a program started by each test
// file (tests/local.spec.mjs), in Chromium and WebKit. RNL_LOCAL_BIN names the program; by default the debug build.
import { defineConfig } from "../../tests/viewer/node_modules/@playwright/test/index.mjs";

export default defineConfig({
  testDir: "./tests",
  workers: 1,
  fullyParallel: false,
  retries: 0,
  reporter: "list",
  outputDir: "./test-results",
  timeout: 120000,
  use: { viewport: { width: 1280, height: 800 }, trace: "retain-on-failure" },
  projects: [
    { name: "chromium", use: { browserName: "chromium" } },
    { name: "webkit", use: { browserName: "webkit" } },
  ],
});
