// The languages of the viewers' interface and their catalogs, from the project's list of languages,
// i18n/languages.json, and each language's catalog of the viewers, i18n/<lang>/viewer.json (decision
// 0015). The web viewer's build writes them into its locales/ (apps/web/build.mjs), the page tests'
// server serves them there (tests/viewer/server.js), and the desktop app builds the same list into
// itself (apps/desktop/build.rs). The page reads them in i18n.js.
//
// A language is offered when it is published and has a catalog of the viewers. English, the source
// of every catalog, has none and comes first.

import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

/** The registry, i18n/languages.json, of the repository at `root`. */
const registry = (root) => JSON.parse(readFileSync(join(root, "i18n", "languages.json"), "utf8"));

/**
 * The languages of the interface, in the registry's order: `code`, the code the viewers know it by and
 * save in their settings, such as zh-CN; `lang`, the registry's, such as zh; `name`, its name in itself;
 * `matches`, the first parts of the browser languages that ask for it.
 */
export function uiLanguages(root) {
  const config = registry(root);
  const english = config.languages.find((entry) => entry.code === config.default);
  return [
    { code: english.code, lang: english.code, name: english.name, matches: english.matches || [english.code] },
    ...config.languages
      .filter((entry) => entry.code !== config.default && entry.status === "published"
        && existsSync(join(root, "i18n", entry.code, "viewer.json")))
      .map((entry) => ({
        code: entry.codes?.viewer || entry.code,
        lang: entry.code,
        name: entry.names?.viewer || entry.name,
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

/** Write locales/languages.json and locales/<code>.json, as the page reads them, into `dir`. */
export function writeLocales(root, dir) {
  mkdirSync(dir, { recursive: true });
  const languages = uiLanguages(root);
  writeFileSync(join(dir, "languages.json"), JSON.stringify(languages, null, 1) + "\n");
  for (const { code } of languages.slice(1)) writeFileSync(join(dir, `${code}.json`), JSON.stringify(uiCatalog(root, code), null, 1) + "\n");
  return languages;
}
