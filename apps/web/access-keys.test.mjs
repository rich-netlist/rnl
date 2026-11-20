// The access keys of the menus (apps/viewer-ui/commands.json): in each menu and submenu, the items shown
// on a platform have one letter each, after the `&` of their labels, which no other item there has. The
// same holds in each language of i18n/: the labels there have their own letters, or keep the English
// one as `(&C)`; a label the catalog does not translate shows in English. An item is shown where its platforms say, and
// where the shell has the capability it names `with` (and not the one it names `unless`): the page of the local program
// is the web viewer with the files of a machine behind it (`overwrites`, `pathPackages`).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readdirSync, readFileSync } from 'node:fs';

const table = JSON.parse(readFileSync(new URL('../viewer-ui/commands.json', import.meta.url), 'utf8'));
const i18n = new URL('../../i18n/', import.meta.url);
const languages = readdirSync(i18n).filter((lang) => existsSync(new URL(`${lang}/viewer.json`, i18n)));

/** The shells whose menus are checked: the platform their items name, and the capabilities of the shell (shell.js). */
const SHELLS = {
  desktop: { platform: 'desktop', has: ['solve', 'solverTable', 'overwrites', 'pathPackages'] },
  web: { platform: 'web', has: ['solve', 'solverTable'] },
  local: { platform: 'web', has: ['solve', 'solverTable', 'overwrites', 'pathPackages'] },
};

/** Whether `shell` shows `item`, as commands.js offers it. */
function shown(item, { platform, has }) {
  const here = !item.platforms || item.platforms.includes(platform) || (item.with !== undefined && has.includes(item.with));
  return here && !(item.unless !== undefined && has.includes(item.unless));
}

/** The clashes of the access keys of `items` in `shell`, and of their submenus, as "menu: letter (labels)". */
function clashes(label, items, shell, catalog = {}) {
  const found = [];
  const byKey = new Map();
  for (const item of items) {
    if (typeof item === 'string') continue;
    if (!shown(item, shell)) continue;
    const text = catalog[item.label] || item.label;
    const key = /&(.)/.exec(text)?.[1]?.toLowerCase();
    if (key) byKey.set(key, [...(byKey.get(key) ?? []), text]);
    if (item.items) found.push(...clashes(`${label} › ${text}`, item.items, shell, catalog));
  }
  for (const [key, labels] of byKey) if (labels.length > 1) found.push(`${label}: ${key} (${labels.join(', ')})`);
  return found;
}

for (const [name, shell] of Object.entries(SHELLS)) {
  test(`no two items of a menu share an access key, on the ${name}`, () => {
    // (the menu bar's own letters, and each menu's, down its submenus)
    assert.deepEqual(clashes('(menu bar)', table.menus, shell), []);
  });

  test(`no two items of a menu share an access key in a language, on the ${name}`, () => {
    assert.ok(languages.length > 0);
    for (const lang of languages) {
      const catalog = JSON.parse(readFileSync(new URL(`${lang}/viewer.json`, i18n), 'utf8'));
      assert.deepEqual(clashes(`${lang} (menu bar)`, table.menus, shell, catalog), [], lang);
    }
  });
}
