// The menu bar of a shell that draws a native one (the desktop app's), planned here, from the command table, as the
// page's own menu bar is (commands.js `menus`: the commands this shell offers, by `platforms`, `with` and `unless`, the
// texts translated, a separator only between two entries), with what a native menu adds: the keys, the About item and,
// on macOS, the application's menu. The shell only draws the plan (`shell.setMenu`), and is given with it the few texts
// it shows itself, in the file dialogs, translated.
//
// On Windows the page handles the shortcut keys, which reach the WebView before any menu accelerator, so a shortcut is
// only shown, after a tab; elsewhere the menu handles it, and the page never sees the key. The keys the page always
// handles (`page`) have no accelerator, and are only shown on Windows.

import { m } from "./i18n.js";

/**
 * @typedef {{ kind: "item", id: string, text: string, accelerator: string | null, check: boolean }
 *   | { kind: "submenu", text: string, items: NativeEntry[], edit?: boolean }
 *   | { kind: "about", text: string }
 *   | { kind: "separator" }} NativeEntry
 */

/** The texts the desktop app shows itself, in its file dialogs, by their English: translated when the menu is drawn. */
const DIALOG_TEXTS = () => Object.fromEntries([
  m("Open Netlist"), m("Save As"), m("Export Image"), m("Open Project"), m("Netlists and packages"), m("Project archives"),
  m("All files"), m("Save Project"), m("Choose the folder to open the project in"), m("Save Solved Copy As"),
  m("Save the Solved Files Into a Folder"),
].map((text) => [text.key, String(text)]));

/** The accelerator of a key as the table writes it ("Ctrl+Shift+M"): Ctrl is Cmd on macOS, Control is Ctrl alone. */
const accelerator = (key) => key.replace("Ctrl+", "CmdOrCtrl+").replace("Control+", "Ctrl+");

/** The entries of a menu, as `menus()` gives them, for a native menu. */
function entries(items, windows) {
  return items.map((item) => {
    if (item === "-") return { kind: "separator" };
    if (item.items) return { kind: "submenu", text: item.label, items: entries(item.items, windows) };
    if (item.id === "about") return { kind: "about", text: item.label };
    const shown = item.shown || item.keys?.[0];
    if (windows && shown) return { kind: "item", id: item.id, text: `${item.label}\t${shown}`, accelerator: null, check: !!item.check };
    const key = !windows && !item.page ? item.keys?.[0] : undefined;
    return { kind: "item", id: item.id, text: item.label, accelerator: key ? accelerator(key) : null, check: !!item.check };
  });
}

/**
 * The menu bar as a native menu shows it, from `planned`, the menus this shell offers (commands.js `menus`), each
 * `{ label, items, source }`, `source` being the table's English label: `{ menus, texts }`, the menus as submenus of the
 * bar, and the texts of the file dialogs. `os` is "windows", "macos" or another. On macOS the first menu is the
 * application's, with About and Quit, which leave the others; and the Edit menu is marked, for the editing commands
 * the system has there.
 */
export function nativeMenus(planned, os) {
  const windows = os === "windows";
  let menus = planned.map((menu) => ({ kind: "submenu", text: menu.label, items: entries(menu.items, windows), ...(menu.source === "&Edit" ? { edit: true } : {}) }));
  if (os === "macos") {
    menus = menus.map((menu) => {
      const items = menu.items.filter((e) => e.kind !== "about" && !(e.kind === "item" && e.id === "exit"));
      while (items.at(-1)?.kind === "separator") items.pop();
      return { ...menu, items };
    }).filter((menu) => menu.items.length > 0);
    menus.unshift({ kind: "submenu", text: "RNL Viewer", items: [
      { kind: "about", text: String(m("About RNL Viewer")) },
      { kind: "separator" },
      { kind: "item", id: "exit", text: String(m("Quit RNL Viewer")), accelerator: "CmdOrCtrl+Q", check: false },
    ] });
  }
  return { menus, texts: DIALOG_TEXTS() };
}

/** The operating system the page runs on, as a native menu tells them apart. */
export function nativeOs() {
  const platform = navigator.userAgentData?.platform || navigator.platform || "";
  return /^Win/i.test(platform) ? "windows" : /^Mac/i.test(platform) ? "macos" : "other";
}
