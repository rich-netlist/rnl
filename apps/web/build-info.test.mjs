import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { buildVersions } from "./build-info.mjs";

test("web metadata follows independent Viewer and core releases", () => {
  const root = mkdtempSync(join(tmpdir(), "rnl-versions-"));
  try {
    mkdirSync(join(root, "apps", "desktop"), { recursive: true });
    const core = version => writeFileSync(join(root, "Cargo.toml"), `[workspace.package]\nversion = "${version}"\n`);
    const viewer = version => {
      writeFileSync(join(root, "apps", "desktop", "Cargo.toml"), `[package]\nversion = "${version}"\n`);
      writeFileSync(join(root, "apps", "desktop", "tauri.conf.json"), JSON.stringify({ version }));
    };
    core("0.2.0"); viewer("0.1.5");
    assert.deepEqual(buildVersions(root), { version: "0.1.5", coreVersion: "0.2.0" });
    viewer("0.1.6");
    assert.deepEqual(buildVersions(root), { version: "0.1.6", coreVersion: "0.2.0" });
    core("0.2.1");
    assert.deepEqual(buildVersions(root), { version: "0.1.6", coreVersion: "0.2.1" });
    writeFileSync(join(root, "apps", "desktop", "tauri.conf.json"), '{"version":"0.1.7"}');
    assert.throws(() => buildVersions(root), /mismatched Viewer/);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
