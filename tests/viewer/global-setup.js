// Before the tests: build the rnl command, write the fixtures into a temporary directory and serve
// them with the page. The tests find the server at RNL_VIEWER_URL and the rnl command at RNL_BIN,
// which the workers inherit.

import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { buildRnl, writeFixtures } from "./fixtures.js";
import { serve } from "./server.js";

/** The storage every page starts with (playwright.config.js): Ask to Solve on Open off, at the origins of both servers. */
function writeStorageState() {
  const origins = [process.env.RNL_VIEWER_URL, process.env.RNL_WEB_URL].filter(Boolean)
    .map((origin) => ({ origin, localStorage: [{ name: "solve-on-open", value: "false" }] }));
  fs.writeFileSync(process.env.RNL_STORAGE_STATE, JSON.stringify({ cookies: [], origins }));
}

export default async function globalSetup() {
  writeStorageState();
  // Embed tests exercise the real web core and need no native CLI or fake-shell fixtures.
  const specs = process.argv.filter((arg) => /\.spec\.js(:\d+)*$/.test(arg));
  if (specs.length && specs.every((spec) => /(?:embed|api)\.spec\.js/.test(spec))) return;
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "rnl-viewer-"));
  const rnl = buildRnl();
  process.env.RNL_BIN = rnl;
  writeFixtures(rnl, dir);
  const server = await serve(dir);
  process.env.RNL_VIEWER_URL = server.url;
  writeStorageState();
  return async () => {
    await server.close();
    fs.rmSync(process.env.RNL_STORAGE_STATE, { force: true });
    fs.rmSync(dir, { recursive: true, force: true });
  };
}
