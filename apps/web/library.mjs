// Publish the built-in standard series; optional packages come from the public catalog.
import { mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { readPackage } from "../../site/gallery/package-file.mjs";

export function publishLibrary(libraries, out) {
  const index = [];
  const bundled = {};
  function walk(dir, prefix = "") {
    for (const entry of readdirSync(dir, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name))) {
      const path = prefix + entry.name;
      if (entry.isDirectory() && (prefix || entry.name === "standard")) walk(join(dir, entry.name), path + "/");
      else if (entry.isFile() && entry.name.endsWith(".rnl")) {
        const text = readFileSync(join(dir, entry.name), "utf8").replace(/\r\n/g, "\n");
        const { header, description } = readPackage(text);
        if (!header.name || !header.version) throw new Error(`${path} has no package name or version`);
        const builtin = /^standard\/[^/]+\/[^/]+$/.test(path);
        index.push({ name: header.name, version: header.version, path, description: description.split(/\n\s*\n/)[0], builtin });
        const dest = join(out, path);
        mkdirSync(dirname(dest), { recursive: true });
        writeFileSync(dest, text);
        if (builtin) bundled[`<bundled>/${path.replace(/^standard\//, "")}`] = text;
      }
    }
  }
  walk(libraries);
  mkdirSync(out, { recursive: true });
  writeFileSync(join(out, "index.json"), JSON.stringify(index, null, 2) + "\n");
  writeFileSync(join(out, "bundled.json"), JSON.stringify(bundled) + "\n");
  return index;
}
