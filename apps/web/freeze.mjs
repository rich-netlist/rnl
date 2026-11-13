// Put the banner of the frozen viewer into a built web viewer:
//
//   node apps/web/freeze.mjs DIST [--link ../viewer/]
//
// DIST is a web viewer as apps/web/build.mjs wrote it, built from the tag the viewer for 0.3 files is frozen at
// (.github/workflows/pages.yml, FROZEN_VIEWER_TAG). The banner (frozen-banner.js) says it is the old viewer and links the
// new one at --link, relative to DIST as it is published (site/dist/viewer-0.3/, so ../viewer/). It is added to the built
// page and not to apps/viewer-ui, so the frozen page is the tag's, unchanged but for the banner.

import { copyFileSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));

/** The page `html` with the script of the banner before the end of its body; the same page if it has it. */
export function withBanner(html, link = "../viewer/") {
  if (html.includes("frozen-banner.js")) return html;
  if (!/<\/body>/i.test(html)) throw new Error("the page has no end of its body to put the banner before");
  const script = `<script src="frozen-banner.js" data-href="${link.replace(/&/g, "&amp;").replace(/"/g, "&quot;")}"></script>\n`;
  return html.replace(/<\/body>/i, () => `${script}</body>`);
}

/** Freeze the built viewer in `dist`: the banner's script beside its page, and the page that runs it. */
export function freeze(dist, link) {
  const page = join(dist, "index.html");
  writeFileSync(page, withBanner(readFileSync(page, "utf8"), link));
  copyFileSync(join(here, "frozen-banner.js"), join(dist, "frozen-banner.js"));
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  const args = process.argv.slice(2);
  const usage = "usage: node apps/web/freeze.mjs DIST [--link ../viewer/]";
  const link = args.includes("--link") ? args[args.indexOf("--link") + 1] : undefined;
  const dist = args.find((arg, i) => !arg.startsWith("--") && args[i - 1] !== "--link");
  if (!dist || (args.includes("--link") && !link)) {
    console.error(usage);
    process.exit(2);
  }
  try {
    freeze(dist, link);
  } catch (error) {
    console.error(`freeze: ${error.message}`);
    process.exit(1);
  }
}
