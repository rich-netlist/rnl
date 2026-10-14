//! The menu bar, built from the command table that the page shares with the web viewer
//! (`apps/viewer-ui/commands.json`), so that the menus and the page's keys cannot drift apart.

use std::collections::HashMap;

use serde::Deserialize;
use tauri::menu::{
    AboutMetadata, CheckMenuItem, CheckMenuItemBuilder, IsMenuItem, Menu, MenuItemBuilder, PredefinedMenuItem, Submenu,
    SubmenuBuilder,
};
use tauri::{AppHandle, Manager, State, Wry};

/// The command table: the menus, their items, and the keys of the items.
const COMMANDS: &str = include_str!("../../viewer-ui/commands.json");

/// This platform, as the command table names it: an item that names its platforms is only shown
/// when they include this one.
const PLATFORM: &str = "desktop";

#[derive(Deserialize)]
struct Table {
    menus: Vec<TableMenu>,
}

/// A menu, or a submenu: its label and its entries.
#[derive(Deserialize)]
struct TableMenu {
    label: String,
    items: Vec<Entry>,
}

/// A menu entry: a submenu, which has entries of its own, a command, or `"-"` for a separator.
#[derive(Deserialize)]
#[serde(untagged)]
enum Entry {
    Submenu(TableMenu),
    Command(Command),
    Separator(Rule),
}

/// The separator, the only string a menu's items may hold.
#[derive(Deserialize)]
enum Rule {
    #[serde(rename = "-")]
    Rule,
}

/// A command of the table.
#[derive(Deserialize)]
struct Command {
    /// What the page's `command` takes; "exit" and "about" are the app's own.
    id: String,
    /// The text, with `&` before the mnemonic letter.
    label: String,
    /// The keys that run it, written like "Ctrl+Shift+M", where Ctrl is Cmd on macOS. The first is
    /// the menu's accelerator.
    #[serde(default)]
    keys: Vec<String>,
    /// How the menu shows the keys, when not as the first of them.
    shown: Option<String>,
    /// Whether it has a check mark. An item of a group of which one is checked, which the table
    /// marks as `radio` for the page's own menu bar, is a check item here: the page sets the marks.
    #[serde(default)]
    check: bool,
    /// Whether the page always handles its key, such as Enter or Backspace: a menu accelerator
    /// would take the key from the search box too.
    #[serde(default)]
    page: bool,
    /// The platforms that offer it; all when not given.
    platforms: Option<Vec<String>>,
}

/// An entry of the menu bar as this platform shows it.
#[derive(Debug, PartialEq)]
enum Shown {
    /// A command: its id, its text with the keys shown, the accelerator that runs it, and whether it
    /// has a check mark.
    Item {
        id: String,
        text: String,
        accelerator: Option<String>,
        check: bool,
    },
    /// A submenu: its text, and its entries as this platform shows them.
    Submenu(String, Vec<Shown>),
    /// The predefined About item, with its text.
    About(String),
    Separator,
}

/// The menus as this platform shows them, with a separator only between two entries. On Windows the
/// page handles the shortcut keys, which reach the WebView before any menu accelerator, so the
/// shortcut is only shown, after a tab; elsewhere the menu handles it, and the page never sees the
/// key. The keys the page always handles have no accelerator and are only shown on Windows.
fn plan(table: &Table, windows: bool) -> Vec<(String, Vec<Shown>)> {
    table
        .menus
        .iter()
        .map(|menu| (menu.label.clone(), entries(&menu.items, windows)))
        .filter(|(_, shown)| !shown.is_empty())
        .collect()
}

/// The entries of a menu or a submenu as this platform shows them, as [`plan`] says; a submenu with
/// nothing to show is left out.
fn entries(items: &[Entry], windows: bool) -> Vec<Shown> {
    let mut shown = Vec::new();
    let mut gap = false;
    for entry in items {
        let next = match entry {
            Entry::Separator(Rule::Rule) => {
                gap = !shown.is_empty();
                continue;
            }
            Entry::Submenu(menu) => match entries(&menu.items, windows) {
                inner if inner.is_empty() => continue,
                inner => Shown::Submenu(menu.label.clone(), inner),
            },
            Entry::Command(c) if c.platforms.as_ref().is_some_and(|p| !p.iter().any(|p| p == PLATFORM)) => continue,
            Entry::Command(c) if c.id == "about" => Shown::About(c.label.clone()),
            Entry::Command(c) => {
                let (text, accelerator) = match (c.shown.as_ref().or(c.keys.first()), c.keys.first()) {
                    (Some(shown), _) if windows => (format!("{}\t{shown}", c.label), None),
                    (_, Some(key)) if !c.page => (c.label.clone(), Some(key.replacen("Ctrl+", "CmdOrCtrl+", 1))),
                    _ => (c.label.clone(), None),
                };
                Shown::Item { id: c.id.clone(), text, accelerator, check: c.check }
            }
        };
        if gap {
            shown.push(Shown::Separator);
            gap = false;
        }
        shown.push(next);
    }
    shown
}

/// The menu bar: the menus of the command table with the items the desktop offers. Every item but
/// Exit and About is passed to the page as a `menu` event carrying the item's id.
pub fn menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let table: Table = serde_json::from_str(COMMANDS).expect("commands.json is a command table");
    let mut checks = HashMap::new();
    let mut menus = Vec::new();
    for (label, entries) in plan(&table, cfg!(windows)) {
        menus.push(submenu(app, &label, entries, &mut checks)?);
    }
    app.manage(Checks(checks));
    let items: Vec<&dyn IsMenuItem<Wry>> = menus.iter().map(|s| s as &dyn IsMenuItem<Wry>).collect();
    Menu::with_items(app, &items)
}

/// A menu of the menu bar, or a submenu, with its entries; its check items go into `checks`.
fn submenu(
    app: &AppHandle,
    label: &str,
    entries: Vec<Shown>,
    checks: &mut HashMap<String, CheckMenuItem<Wry>>,
) -> tauri::Result<Submenu<Wry>> {
    let mut menu = SubmenuBuilder::new(app, label);
    for entry in entries {
        menu = match entry {
            Shown::Separator => menu.separator(),
            Shown::About(text) => menu.item(&PredefinedMenuItem::about(app, Some(&text), Some(about()))?),
            Shown::Submenu(text, inner) => menu.item(&submenu(app, &text, inner, checks)?),
            Shown::Item { id, text, accelerator, check: true } => {
                let mut b = CheckMenuItemBuilder::with_id(&id, text).checked(true);
                if let Some(a) = accelerator {
                    b = b.accelerator(a);
                }
                let item = b.build(app)?;
                let menu = menu.item(&item);
                checks.insert(id, item);
                menu
            }
            Shown::Item { id, text, accelerator, check: false } => {
                let mut b = MenuItemBuilder::with_id(id, text);
                if let Some(a) = accelerator {
                    b = b.accelerator(a);
                }
                menu.item(&b.build(app)?)
            }
        };
    }
    menu.build()
}

/// The About box. Its copyright, which every platform shows, is the notice of the fonts the drawing
/// is in, which the app embeds with the page (decision 0009, OFL 2).
fn about() -> AboutMetadata<'static> {
    AboutMetadata {
        name: Some("RNL Viewer".into()),
        version: Some(env!("CARGO_PKG_VERSION").into()),
        comments: Some(
            "Draws netlists with Rich Netlist (RNL) records. It only reads: it neither edits nor solves.".into(),
        ),
        copyright: Some(
            "Texts are drawn in Noto Sans SC, copyright 2014-2021 Adobe (http://www.adobe.com/), under the SIL Open \
             Font License 1.1 (https://openfontlicense.org)."
                .into(),
        ),
        ..Default::default()
    }
}

/// The menu's check marks by command id, which follow the panels however they are shown or hidden.
pub struct Checks(HashMap<String, CheckMenuItem<Wry>>);

/// Set the check mark of a command, such as "toggle-netlist".
#[tauri::command]
pub fn set_view_check(checks: State<'_, Checks>, id: String, checked: bool) {
    if let Some(item) = checks.0.get(&id) {
        let _ = item.set_checked(checked);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, text: &str, accelerator: Option<&str>) -> Shown {
        Shown::Item { id: id.into(), text: text.into(), accelerator: accelerator.map(Into::into), check: false }
    }

    fn check(id: &str, text: &str, accelerator: Option<&str>) -> Shown {
        Shown::Item { id: id.into(), text: text.into(), accelerator: accelerator.map(Into::into), check: true }
    }

    fn table() -> Table {
        serde_json::from_str(COMMANDS).expect("commands.json is a command table")
    }

    /// View › Grid: the grid shown or not, by G, which the page always handles; its style, one of
    /// three; the axes; and the dialog of its settings.
    fn grid(show: &str) -> Shown {
        Shown::Submenu(
            "&Grid".into(),
            vec![
                check("toggle-grid", show, None),
                Shown::Separator,
                check("grid-none", "&None", None),
                check("grid-dots", "&Dots", None),
                check("grid-lines", "&Lines", None),
                Shown::Separator,
                check("grid-axes", "&Axes", None),
                Shown::Separator,
                item("grid-settings", "Grid &Settings...", None),
            ],
        )
    }

    /// The Help menu, alike on every platform: Check for Updates, which only the desktop offers and
    /// which has no keys, then About.
    fn help() -> (String, Vec<Shown>) {
        let updates = item("check-updates", "Check for &Updates...", None);
        ("&Help".to_string(), vec![updates, Shown::Separator, Shown::About("&About RNL Viewer".into())])
    }

    /// The menu bar as it was written by hand before the table, on Windows, and the grid's submenu.
    #[test]
    fn windows_shows_the_keys_after_a_tab() {
        let menus = plan(&table(), true);
        let expected = vec![
            (
                "&File".to_string(),
                vec![
                    item("open", "&Open...\tCtrl+O", None),
                    item("load-clipboard", "Load from &Clipboard...\tCtrl+Shift+V", None),
                    item("reload", "&Reload\tF5", None),
                    Shown::Separator,
                    item("exit", "E&xit", None),
                ],
            ),
            (
                "&View".to_string(),
                vec![
                    item("fit", "&Fit to Window\tF", None),
                    item("zoom-in", "Zoom &In\tCtrl++", None),
                    item("zoom-out", "Zoom &Out\tCtrl+-", None),
                    grid("Show &Grid\tG"),
                    Shown::Separator,
                    item("descend", "Open &Subcircuit\tEnter", None),
                    item("ascend", "&Up to Parent Level\tBackspace", None),
                    Shown::Separator,
                    check("toggle-netlist", "&Netlist Panel\tCtrl+B", None),
                    check("toggle-problems", "&Problems\tCtrl+Shift+M", None),
                ],
            ),
            help(),
        ];
        assert_eq!(menus, expected);
    }

    /// The menu bar as it was written by hand before the table, on macOS and Linux, and the grid's
    /// submenu.
    #[test]
    fn elsewhere_the_menu_has_the_accelerators() {
        let menus = plan(&table(), false);
        let expected = vec![
            (
                "&File".to_string(),
                vec![
                    item("open", "&Open...", Some("CmdOrCtrl+O")),
                    item("load-clipboard", "Load from &Clipboard...", Some("CmdOrCtrl+Shift+V")),
                    item("reload", "&Reload", Some("F5")),
                    Shown::Separator,
                    item("exit", "E&xit", None),
                ],
            ),
            (
                "&View".to_string(),
                vec![
                    item("fit", "&Fit to Window", Some("F")),
                    item("zoom-in", "Zoom &In", Some("CmdOrCtrl+=")),
                    item("zoom-out", "Zoom &Out", Some("CmdOrCtrl+-")),
                    grid("Show &Grid"),
                    Shown::Separator,
                    item("descend", "Open &Subcircuit", None),
                    item("ascend", "&Up to Parent Level", None),
                    Shown::Separator,
                    check("toggle-netlist", "&Netlist Panel", Some("CmdOrCtrl+B")),
                    check("toggle-problems", "&Problems", Some("CmdOrCtrl+Shift+M")),
                ],
            ),
            help(),
        ];
        assert_eq!(menus, expected);
    }

    /// The About box names the fonts the app embeds, their copyright and their licence.
    #[test]
    fn the_about_box_names_the_fonts_and_their_licence() {
        let notice = about().copyright.unwrap_or_default();
        for part in ["Noto Sans SC", "copyright 2014-2021 Adobe", "SIL Open Font License 1.1"] {
            assert!(notice.contains(part), "{notice}");
        }
    }

    /// Separators stand only between two items, whichever items a platform leaves out.
    #[test]
    fn separators_only_between_items() {
        let table: Table = serde_json::from_str(
            r#"{ "menus": [
                { "label": "A", "items": ["-", { "id": "a", "label": "a", "platforms": ["web"] }, "-",
                    { "id": "b", "label": "b" }, "-", "-", { "id": "c", "label": "c" }, "-",
                    { "id": "d", "label": "d", "platforms": ["web"] }] },
                { "label": "B", "items": [{ "id": "e", "label": "e", "platforms": ["web"] }] }
            ] }"#,
        )
        .unwrap();
        let menus = plan(&table, false);
        assert_eq!(menus, vec![("A".to_string(), vec![item("b", "b", None), Shown::Separator, item("c", "c", None)])]);
    }

    /// A string in a menu's items is a separator, "-", and nothing else.
    #[test]
    fn only_a_dash_is_a_separator() {
        let table = serde_json::from_str::<Table>(r#"{ "menus": [{ "label": "A", "items": ["--"] }] }"#);
        assert!(table.is_err());
    }

    /// A submenu's entries are planned as a menu's, at any depth: the items the platform offers, a
    /// separator only between two entries, and the keys as the platform shows them. A submenu with
    /// nothing to show is left out, and so is a menu that has nothing else.
    #[test]
    fn submenus_are_planned_as_menus() {
        let table: Table = serde_json::from_str(
            r#"{ "menus": [
                { "label": "A", "items": [
                    { "id": "a", "label": "a", "keys": ["Ctrl+A"] }, "-",
                    { "label": "B", "items": ["-", { "id": "b", "label": "b", "keys": ["G"], "check": true, "radio": true },
                        "-", { "label": "C", "items": [{ "id": "c", "label": "c", "keys": ["Enter"], "page": true }] }, "-"] },
                    "-", { "label": "D", "items": [{ "id": "d", "label": "d", "platforms": ["web"] }, "-"] },
                    "-", { "id": "e", "label": "e" }
                ] },
                { "label": "F", "items": [{ "label": "G", "items": [{ "id": "g", "label": "g", "platforms": ["web"] }] }] }
            ] }"#,
        )
        .unwrap();
        let sub = |text: &str, entries: Vec<Shown>| Shown::Submenu(text.into(), entries);
        let windows = vec![(
            "A".to_string(),
            vec![
                item("a", "a\tCtrl+A", None),
                Shown::Separator,
                sub("B", vec![check("b", "b\tG", None), Shown::Separator, sub("C", vec![item("c", "c\tEnter", None)])]),
                Shown::Separator,
                item("e", "e", None),
            ],
        )];
        assert_eq!(plan(&table, true), windows);
        let elsewhere = vec![(
            "A".to_string(),
            vec![
                item("a", "a", Some("CmdOrCtrl+A")),
                Shown::Separator,
                sub("B", vec![check("b", "b", Some("G")), Shown::Separator, sub("C", vec![item("c", "c", None)])]),
                Shown::Separator,
                item("e", "e", None),
            ],
        )];
        assert_eq!(plan(&table, false), elsewhere);
    }

    /// An entry with items is a submenu and one with an id a command; one with neither is no entry.
    #[test]
    fn a_submenu_has_items_and_a_command_an_id() {
        let entry = |json: &str| serde_json::from_str::<Entry>(json);
        assert!(matches!(entry(r#"{ "label": "A", "items": [] }"#), Ok(Entry::Submenu(_))));
        assert!(matches!(entry(r#"{ "id": "a", "label": "A" }"#), Ok(Entry::Command(_))));
        assert!(entry(r#"{ "label": "A" }"#).is_err());
    }
}
