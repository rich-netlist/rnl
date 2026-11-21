// The table of the languages of the interface (i18n/viewer-languages.json), which the web viewer, the page tests and
// the desktop app read, is what the registry and the catalogs give now: run `node scripts/i18n/viewer-locales.mjs`
// after a language is published or gains its catalog of the viewers.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

import { TABLE, tableText } from "../../scripts/i18n/viewer-locales.mjs";

const root = fileURLToPath(new URL("../..", import.meta.url));

test("the table of the interface's languages is current", () => {
  assert.equal(readFileSync(join(root, TABLE), "utf8").replace(/\r\n/g, "\n"), tableText(root),
    `${TABLE} is not current: run node scripts/i18n/viewer-locales.mjs`);
});

test("each language of the table has its name in English, English first", () => {
  const languages = JSON.parse(tableText(root));
  assert.equal(languages[0].code, "en");
  for (const l of languages) assert.ok(l.code && l.lang && l.name && l.english && l.matches.length, JSON.stringify(l));
});
