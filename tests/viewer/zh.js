// The Simplified Chinese of the viewers' texts, read from the catalog the page loads (i18n/zh/viewer.json),
// so that a test of the interface in Chinese follows a revised translation without a change of its own.
import { readFileSync } from 'node:fs';

const catalog = JSON.parse(readFileSync(new URL('../../i18n/zh/viewer.json', import.meta.url), 'utf8'));

/** The Chinese of an English text, with its placeholders {0}, {1}… filled in by `values`. */
export function zh(english, ...values) {
  const text = catalog[english];
  if (typeof text !== 'string') throw new Error(`i18n/zh/viewer.json has no translation of ${JSON.stringify(english)}`);
  return text.replace(/\{(\d+)\}/g, (_, i) => String(values[Number(i)]));
}

/** A menu label as the page's menus show it: the access key's & left out, and the shortcut after a tab too. */
export const zhMenu = (english) => zh(english).split('\t')[0].replace('&', '');
