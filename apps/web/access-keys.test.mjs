// The access keys of the menus (apps/viewer-ui/commands.json): in each menu and submenu, the items shown
// on a platform have one letter each, after the `&` of their labels, which no other item there has. The
// same holds in each language of i18n/: the labels there have their own letters, or keep the English
// one as `(&C)`; a label the catalog does not translate shows in English.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readdirSync, readFileSync } from 'node:fs';

const table = JSON.parse(readFileSync(new URL('../viewer-ui/commands.json', import.meta.url), 'utf8'));
const i18n = new URL('../../i18n/', import.meta.url);
const languages = readdirSync(i18n).filter((lang) => existsSync(new URL(`${lang}/viewer.json`, i18n)));

/** The clashes of the access keys of `items` on `platform`, and of their submenus, as "menu: letter (labels)". */
function clashes(label, items, platform, catalog = {}) {
  const found = [];
  const byKey = new Map();
  for (const item of items) {
    if (typeof item === 'string') continue;
    if (item.platforms && !item.platforms.includes(platform)) continue;
    const shown = catalog[item.label] || item.label;
    const key = /&(.)/.exec(shown)?.[1]?.toLowerCase();
    if (key) byKey.set(key, [...(byKey.get(key) ?? []), shown]);
    if (item.items) found.push(...clashes(`${label} › ${shown}`, item.items, platform, catalog));
  }
  for (const [key, labels] of byKey) if (labels.length > 1) found.push(`${label}: ${key} (${labels.join(', ')})`);
  return found;
}

for (const platform of ['desktop', 'web']) {
  test(`no two items of a menu share an access key, on the ${platform}`, () => {
    // (the menu bar's own letters, and each menu's, down its submenus)
    assert.deepEqual(clashes('(menu bar)', table.menus, platform), []);
  });

  test(`no two items of a menu share an access key in a language, on the ${platform}`, () => {
    assert.ok(languages.length > 0);
    for (const lang of languages) {
      const catalog = JSON.parse(readFileSync(new URL(`${lang}/viewer.json`, i18n), 'utf8'));
      assert.deepEqual(clashes(`${lang} (menu bar)`, table.menus, platform, catalog), [], lang);
    }
  });
}
