import { defineConfig } from "../tests/viewer/node_modules/@playwright/test/index.mjs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { execFileSync } from "node:child_process";
import { packageCache, publicPackagesAvailable } from "../scripts/packages.mjs";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
if (publicPackagesAvailable()) {
  execFileSync("python", ["scripts/packages.py", "fixture"], { cwd: root });
  process.env.RNL_PACKAGE_CATALOG = resolve(packageCache, "fixture/catalog.json");
  // Rebuild only gallery data; all browser assertions see the locked fixture, even when the
  // preceding production build used a newer catalog.
  execFileSync(process.execPath, ["site/gallery/build.mjs"], { cwd: root, stdio: "inherit" });
}
export default defineConfig({
  testDir: "./tests",
  workers: 2,
  fullyParallel: true,
  retries: 0,
  reporter: "list",
  outputDir: "./test-results",
  use: { baseURL: "http://127.0.0.1:8767", viewport: { width: 1440, height: 1000 }, reducedMotion: "reduce", trace: "retain-on-failure" },
  webServer: {
    command: "python site/serve.py --port 8767",
    cwd: root,
    url: "http://127.0.0.1:8767/zh/",
    reuseExistingServer: !process.env.CI,
    stderr: "ignore",
  },
  projects: [
    { name: "chromium", use: { browserName: "chromium" } },
    { name: "webkit", use: { browserName: "webkit" } },
  ],
});
