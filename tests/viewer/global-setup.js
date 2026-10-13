// Before the tests: build the rnl command, write the fixtures into a temporary directory and serve
// them with the page. The tests find the server at RNL_VIEWER_URL and the rnl command at RNL_BIN,
// which the workers inherit.

import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { buildRnl, writeFixtures } from "./fixtures.js";
import { serve } from "./server.js";

export default async function globalSetup() {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "rnl-viewer-"));
  const rnl = buildRnl();
  process.env.RNL_BIN = rnl;
  writeFixtures(rnl, dir);
  const server = await serve(dir);
  process.env.RNL_VIEWER_URL = server.url;
  return async () => {
    await server.close();
    fs.rmSync(dir, { recursive: true, force: true });
  };
}
