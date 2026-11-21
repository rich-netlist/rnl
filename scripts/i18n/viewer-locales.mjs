// The languages of the viewers' interface and their catalogs, from the project's list of languages,
// i18n/languages.json, and each language's catalog of the viewers, i18n/<lang>/viewer.json (decision
// 0015). The list is one table, i18n/viewer-languages.json, which this script writes (`node
// scripts/i18n/viewer-locales.mjs`) and a test keeps current (apps/web/viewer-languages.test.mjs): the web
// viewer's build writes it into its locales/ (apps/web/build.mjs), the page tests' server serves it there
// (tests/viewer/server.js), and the desktop app builds it into itself (apps/desktop/build.rs). The page
// reads it in i18n.js.
//
// A language is offered when it is published and has a catalog of the viewers. English, the source
// of every catalog, has none and comes first.

import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

/** The table of the languages of the interface, in the repository at `root`. */
export const TABLE = "i18n/viewer-languages.json";

/** The registry, i18n/languages.json, of the repository at `root`. */
const registry = (root) => JSON.parse(readFileSync(join(root, "i18n", "languages.json"), "utf8"));

/**
 * The languages of the interface, in the registry's order: `code`, the code the viewers know it by and
 * save in their settings, such as zh-CN; `lang`, the registry's, such as zh; `name`, its name in itself;
 * `matches`, the first parts of the browser languages that ask for it; `english`, its name in English, the
 * registry's `names.en`, else its own, for what is said in English about it.
 */
export function uiLanguages(root) {
  return JSON.parse(readFileSync(join(root, TABLE), "utf8"));
}

/** The languages of the interface as the registry and the catalogs give them now: what the table must hold. */
export function languageTable(root) {
  const config = registry(root);
  const english = config.languages.find((entry) => entry.code === config.default);
  return [
    { code: english.code, lang: english.code, name: english.name, english: english.name, matches: english.matches || [english.code] },
    ...config.languages
      .filter((entry) => entry.code !== config.default && entry.status === "published"
        && existsSync(join(root, "i18n", entry.code, "viewer.json")))
      .map((entry) => ({
        code: entry.codes?.viewer || entry.code,
        lang: entry.code,
        name: entry.names?.viewer || entry.name,
        english: entry.names?.en || entry.name,
        matches: entry.matches || [entry.code],
      })),
  ];
}

/** The catalog of a language, by the code the viewers know it by: {English: translation}, without the
 * entries that have none yet (null). */
export function uiCatalog(root, code) {
  const language = uiLanguages(root).find((l) => l.code === code);
  if (!language || language.lang === registry(root).default) return null;
  const catalog = JSON.parse(readFileSync(join(root, "i18n", language.lang, "viewer.json"), "utf8"));
  return Object.fromEntries(Object.entries(catalog).filter(([, value]) => typeof value === "string"));
}

/**
 * The English texts that some catalog of the viewers translates, sorted: what the page binds its static texts to, so
 * that it need read only the catalog of the language it starts in, not every one (i18n.js).
 */
export function uiKeys(root) {
  const keys = new Set();
  for (const { code } of uiLanguages(root).slice(1)) for (const key of Object.keys(uiCatalog(root, code) ?? {})) keys.add(key);
  return [...keys].sort();
}

/** Write locales/languages.json, locales/keys.json and locales/<code>.json, as the page reads them, into `dir`. */
export function writeLocales(root, dir) {
  mkdirSync(dir, { recursive: true });
  const languages = uiLanguages(root);
  writeFileSync(join(dir, "languages.json"), JSON.stringify(languages, null, 1) + "\n");
  writeFileSync(join(dir, "keys.json"), JSON.stringify(uiKeys(root)) + "\n");
  for (const { code } of languages.slice(1)) writeFileSync(join(dir, `${code}.json`), JSON.stringify(uiCatalog(root, code), null, 1) + "\n");
  return languages;
}

/** The table as `languageTable` gives it, as it is written. */
export const tableText = (root) => JSON.stringify(languageTable(root), null, 1) + "\n";

// run as a script: write the table
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const root = resolve(fileURLToPath(new URL("../..", import.meta.url)));
  writeFileSync(join(root, TABLE), tableText(root));
  console.log(`wrote ${TABLE}`);
}
