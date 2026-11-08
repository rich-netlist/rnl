// Publish the built-in standard series; optional packages come from the public catalog. A series
// whose standard is not released, without spec/<series>/RELEASED (scripts/spec/README.md), such as
// the draft 0.4, is left out of the files, index.json and bundled.json, as the gallery leaves it out
// (site/gallery/build.mjs), unless `drafts` names it: by default the series RNL_DRAFT_SERIES names,
// such as RNL_DRAFT_SERIES=0.4, for a local build or page tests that need the draft.
import { mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { readPackage } from "../../site/gallery/package-file.mjs";

/** The draft series RNL_DRAFT_SERIES names, separated by commas: "0.4" or "0.4,0.5". */
const draftSeries = () => (process.env.RNL_DRAFT_SERIES ?? "").split(",").map((s) => s.trim()).filter(Boolean);

export function publishLibrary(libraries, out, { spec = join(libraries, "..", "spec"), drafts = draftSeries() } = {}) {
  const index = [];
  const bundled = {};
  const released = (series) => statSync(join(spec, series, "RELEASED"), { throwIfNoEntry: false })?.isFile() ?? false;
  function walk(dir, prefix = "") {
    for (const entry of readdirSync(dir, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name))) {
      const path = prefix + entry.name;
      if (entry.isDirectory() && prefix === "standard/" && !released(entry.name) && !drafts.includes(entry.name)) {
        console.log(`libraries/${path}: not published, since spec/${entry.name}/RELEASED does not exist (RNL_DRAFT_SERIES=${entry.name} publishes it)`);
        continue;
      }
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
