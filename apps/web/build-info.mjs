// The web and desktop shells share the Viewer version, independently of the core workspace.
import { readFileSync } from "node:fs";
import { join } from "node:path";

export function buildVersions(root) {
  const manifest = readFileSync(join(root, "Cargo.toml"), "utf8");
  const coreVersion = /^\[workspace\.package\][^[]*?^version\s*=\s*"([^"]*)"/m.exec(manifest)?.[1];
  const version = JSON.parse(readFileSync(join(root, "apps", "desktop", "tauri.conf.json"), "utf8")).version;
  const viewerManifest = readFileSync(join(root, "apps", "desktop", "Cargo.toml"), "utf8");
  if (!coreVersion || !version || /^version\s*=\s*"([^"]*)"/m.exec(viewerManifest)?.[1] !== version) {
    throw new Error("Missing core version or mismatched Viewer manifest and Tauri versions");
  }
  return { version, coreVersion };
}
