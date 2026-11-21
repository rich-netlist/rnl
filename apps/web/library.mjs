// Publish the standard series; optional packages come from the public catalog. Every package file
// under libraries/standard is published, whether its standard is released on the website or not
// (spec/<series>/RELEASED gates the pages of the standard and the gallery). The built-in packages,
// those the Library lists and bundled.json holds, are the ones the core lists (`listed`, its
// `bundled()`, rnl_view::bundled_listed), as on the desktop: all the core bundles but those of a draft
// series, which the core still reads.
import { mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { readPackage } from "../../site/gallery/package-file.mjs";

/**
 * @param {string} libraries the libraries/ folder
 * @param {string} out where to publish it
 * @param {Record<string, string>} listed the bundled packages the core lists, by their `<bundled>/` paths
 */
export function publishLibrary(libraries, out, listed) {
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
        const key = `<bundled>/${path.replace(/^standard\//, "")}`;
        const builtin = /^standard\/[^/]+\/[^/]+$/.test(path) && key in listed;
        index.push({ name: header.name, version: header.version, path, description: description.split(/\n\s*\n/)[0], builtin });
        const dest = join(out, path);
        mkdirSync(dirname(dest), { recursive: true });
        writeFileSync(dest, text);
        if (builtin) bundled[key] = text;
      }
    }
  }
  walk(libraries);
  const missing = Object.keys(listed).filter((key) => !(key in bundled));
  if (missing.length) throw new Error(`the core lists packages that libraries/standard does not have: ${missing.join(", ")}`);
  mkdirSync(out, { recursive: true });
  writeFileSync(join(out, "index.json"), JSON.stringify(index, null, 2) + "\n");
  writeFileSync(join(out, "bundled.json"), JSON.stringify(bundled) + "\n");
  return index;
}
