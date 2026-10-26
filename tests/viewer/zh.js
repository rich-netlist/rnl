// The translations of the viewers' texts, read from the catalogs the page loads (i18n/<lang>/viewer.json),
// so that a test of the interface in another language follows a revised translation without a change of its own.
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const catalog = JSON.parse(readFileSync(new URL('../../i18n/zh/viewer.json', import.meta.url), 'utf8'));

/** The Chinese of an English text, with its placeholders {0}, {1}… filled in by `values`. */
export function zh(english, ...values) {
  const text = catalog[english];
  if (typeof text !== 'string') throw new Error(`i18n/zh/viewer.json has no translation of ${JSON.stringify(english)}`);
  return text.replace(/\{(\d+)\}/g, (_, i) => String(values[Number(i)]));
}

/** A menu label as the page's menus show it: the access key's & left out, and the shortcut after a tab too. */
export const zhMenu = (english) => zh(english).split('\t')[0].replace('&', '');

/** The translation of an English text in another language's catalog of the viewers, i18n/<lang>/viewer.json. */
export function translated(lang, english) {
  const text = JSON.parse(readFileSync(new URL(`../../i18n/${lang}/viewer.json`, import.meta.url), 'utf8'))[english];
  if (typeof text !== 'string') throw new Error(`i18n/${lang}/viewer.json has no translation of ${JSON.stringify(english)}`);
  return text;
}

/** The languages of the interface as the page offers them: English, then those the registry publishes. */
export { uiLanguages } from '../../scripts/i18n/viewer-locales.mjs';
export const repository = fileURLToPath(new URL('../../', import.meta.url));
