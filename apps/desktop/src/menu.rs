//! The menu bar, built from the command table that the page shares with the web viewer
//! (`apps/viewer-ui/commands.json`), so that the menus and the page's keys cannot drift apart.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use serde::Deserialize;
use tauri::menu::{
    CheckMenuItem, CheckMenuItemBuilder, IsMenuItem, Menu, MenuItemBuilder, MenuItemKind, Submenu, SubmenuBuilder,
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
    #[serde(default, rename = "windowsOnly")]
    windows_only: bool,
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

/// macOS reserves the first menu for the application. Quit still uses our unsaved-edit flow.
#[cfg(any(target_os = "macos", test))]
fn macos_plan(table: &Table) -> Vec<(String, Vec<Shown>)> {
    let mut menus = plan(table, false);
    for (_, entries) in &mut menus {
        entries.retain(|entry| {
            !matches!(entry, Shown::About(_)) && !matches!(entry, Shown::Item { id, .. } if id == "exit")
        });
        while matches!(entries.last(), Some(Shown::Separator)) {
            entries.pop();
        }
    }
    menus.insert(
        0,
        (
            "RNL Viewer".into(),
            vec![
                Shown::About("About RNL Viewer".into()),
                Shown::Separator,
                Shown::Item {
                    id: "exit".into(),
                    text: "Quit RNL Viewer".into(),
                    accelerator: Some("CmdOrCtrl+Q".into()),
                    check: false,
                },
            ],
        ),
    );
    menus
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
            Entry::Command(c) if c.windows_only && !windows => continue,
            Entry::Command(c) if c.id == "about" => Shown::About(c.label.clone()),
            Entry::Command(c) => {
                let (text, accelerator) = match (c.shown.as_ref().or(c.keys.first()), c.keys.first()) {
                    (Some(shown), _) if windows => (format!("{}\t{shown}", c.label), None),
                    (_, Some(key)) if !c.page => {
                        (c.label.clone(), Some(key.replacen("Ctrl+", "CmdOrCtrl+", 1).replacen("Control+", "Ctrl+", 1)))
                    }
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
    let mut labels = Vec::new();
    #[cfg(target_os = "macos")]
    let planned = macos_plan(&table);
    #[cfg(not(target_os = "macos"))]
    let planned = plan(&table, cfg!(windows));
    for (label, entries) in planned {
        menus.push(submenu(app, &label, entries, &mut checks, &mut labels)?);
    }
    app.manage(Checks(checks));
    let items: Vec<&dyn IsMenuItem<Wry>> = menus.iter().map(|s| s as &dyn IsMenuItem<Wry>).collect();
    let menu = Menu::with_items(app, &items)?;

    app.manage(Labels(Mutex::new(("en".into(), labels))));
    Ok(menu)
}

type LabelList = Vec<(MenuItemKind<Wry>, String)>;
pub struct Labels(Mutex<(String, LabelList)>);
static CHINESE: LazyLock<HashMap<String, String>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../../viewer-ui/locales/zh-CN.json")).expect("Chinese UI catalog")
});

fn translated(language: &str, source: &str) -> String {
    let (label, keys) = source.split_once('\t').map_or((source, None), |(a, b)| (a, Some(b)));
    let label = if language == "zh-CN" { CHINESE.get(label).map_or(label, String::as_str) } else { label };
    keys.map_or_else(|| label.to_string(), |keys| format!("{label}\t{keys}"))
}

/// Change labels in place: command IDs, accelerators, enabled state and check marks stay intact.
#[tauri::command]
pub fn set_ui_language(labels: State<'_, Labels>, language: String) -> Result<(), String> {
    if language != "en" && language != "zh-CN" {
        return Err("Unsupported UI language".into());
    }
    let mut state = labels.0.lock().map_err(|e| e.to_string())?;
    for (item, source) in &state.1 {
        let text = translated(&language, source);
        match item {
            MenuItemKind::MenuItem(i) => i.set_text(&text),
            MenuItemKind::Check(i) => i.set_text(&text),
            MenuItemKind::Submenu(i) => i.set_text(&text),
            _ => continue,
        }
        .map_err(|e| e.to_string())?;
    }
    state.0 = language;
    Ok(())
}

pub fn text(app: &AppHandle, source: &str) -> String {
    let labels = app.state::<Labels>();
    let state = labels.0.lock().unwrap_or_else(|e| e.into_inner());
    translated(&state.0, source)
}

/// A menu of the menu bar, or a submenu, with its entries; its check items go into `checks`.
fn submenu(
    app: &AppHandle,
    label: &str,
    entries: Vec<Shown>,
    checks: &mut HashMap<String, CheckMenuItem<Wry>>,
    labels: &mut LabelList,
) -> tauri::Result<Submenu<Wry>> {
    let mut menu = SubmenuBuilder::new(app, label);
    #[cfg(target_os = "macos")]
    if label == "&Edit" {
        // WKWebView's responder actions need native Edit items for standard Cmd+C/V/X/A.
        use tauri::menu::PredefinedMenuItem;
        for item in [
            PredefinedMenuItem::undo(app, None)?,
            PredefinedMenuItem::redo(app, None)?,
            PredefinedMenuItem::cut(app, None)?,
            PredefinedMenuItem::copy(app, None)?,
            PredefinedMenuItem::paste(app, None)?,
            PredefinedMenuItem::select_all(app, None)?,
        ] {
            menu = menu.item(&item);
        }
        menu = menu.separator();
    }
    for entry in entries {
        menu = match entry {
            Shown::Separator => menu.separator(),
            Shown::About(text) => {
                let item = MenuItemBuilder::with_id("about", &text).build(app)?;
                labels.push((MenuItemKind::MenuItem(item.clone()), text));
                menu.item(&item)
            }
            Shown::Submenu(text, inner) => menu.item(&submenu(app, &text, inner, checks, labels)?),
            Shown::Item { id, text, accelerator, check: true } => {
                let mut b = CheckMenuItemBuilder::with_id(&id, &text).checked(true);
                if let Some(a) = accelerator {
                    b = b.accelerator(a);
                }
                let item = b.build(app)?;
                labels.push((MenuItemKind::Check(item.clone()), text));
                let menu = menu.item(&item);
                checks.insert(id, item);
                menu
            }
            Shown::Item { id, text, accelerator, check: false } => {
                let mut b = MenuItemBuilder::with_id(id, &text);
                if let Some(a) = accelerator {
                    b = b.accelerator(a);
                }
                let item = b.build(app)?;
                labels.push((MenuItemKind::MenuItem(item.clone()), text));
                menu.item(&item)
            }
        };
    }
    let menu = menu.build()?;
    labels.push((MenuItemKind::Submenu(menu.clone()), label.to_string()));
    Ok(menu)
}

/// The About box. Its copyright, which every platform shows, is the notice of the fonts the drawing
/// is in, which the app embeds with the page (decision 0009, OFL 2).
#[cfg(test)]
fn about() -> tauri::menu::AboutMetadata<'static> {
    tauri::menu::AboutMetadata {
        name: Some("RNL Viewer".into()),
        version: Some(env!("CARGO_PKG_VERSION").into()),
        comments: Some(
            "Draws netlists with Rich Netlist (RNL) records. It can edit a netlist, solve it as a preview and save \
             a solved copy; it writes a file only when you have edited it and choose Save."
                .into(),
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

#[tauri::command]
pub fn set_command_enabled(labels: State<'_, Labels>, id: String, enabled: bool) {
    // Menu::get only searches direct children; Labels retains handles at every submenu depth.
    let item = labels
        .0
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .1
        .iter()
        .find(|(item, _)| item.id().as_ref() == id.as_str())
        .map(|(item, _)| item.clone());
    match item {
        Some(MenuItemKind::MenuItem(item)) => {
            let _ = item.set_enabled(enabled);
        }
        Some(MenuItemKind::Check(item)) => {
            let _ = item.set_enabled(enabled);
        }
        _ => {}
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

    #[test]
    fn translations_keep_shortcuts_and_fall_back_to_english() {
        assert_eq!(translated("zh-CN", "&Open File...\tCtrl+O"), "打开文件(&O)...\tCtrl+O");
        assert_eq!(translated("en", "&Open File...\tCtrl+O"), "&Open File...\tCtrl+O");
        assert_eq!(translated("zh-CN", "untranslated"), "untranslated");
    }

    #[test]
    fn macos_has_one_application_about_and_quit_and_keeps_control_q_for_editing() {
        let menus = macos_plan(&table());
        assert_eq!(menus[0].0, "RNL Viewer");
        assert_eq!(
            menus[0].1,
            vec![
                Shown::About("About RNL Viewer".into()),
                Shown::Separator,
                item("exit", "Quit RNL Viewer", Some("CmdOrCtrl+Q"))
            ]
        );
        assert!(menus[1..].iter().all(|(_, items)| items
            .iter()
            .all(|entry| !matches!(entry, Shown::About(_)) && !matches!(entry, Shown::Item {id,..} if id == "exit"))));
        assert!(menus.iter().any(|(_, items)| items.contains(&check("toggle-edit", "&Edit Text", Some("Ctrl+Q")))));
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

    /// Panel digits are page-handled, so native accelerators cannot intercept text input.
    fn panels(tabbed: bool) -> Shown {
        let mut entries: Vec<_> = [
            ("files", "&Library"),
            ("netlist", "&Netlist"),
            ("hierarchy", "&Hierarchy"),
            ("symbol", "&Symbol"),
            ("usages", "&Usage"),
            ("unplaced", "Not &Drawn"),
            ("solve", "S&olve"),
        ]
        .iter()
        .enumerate()
        .map(|(i, (id, label))| {
            let text = if tabbed { format!("{label}\t{}", i + 1) } else { label.to_string() };
            check(&format!("panel-{id}"), &text, None)
        })
        .collect();
        entries.extend([Shown::Separator, item("panel-hide", "&Hide", None)]);
        Shown::Submenu("Side P&anels".into(), entries)
    }

    fn languages() -> Shown {
        Shown::Submenu(
            "&Language".into(),
            vec![check("language-en", "English", None), check("language-zh", "简体中文", None)],
        )
    }

    /// The Edit menu (decision 0011): switching the text to edit, a check item, and applying its
    /// changes, with their keys shown after a tab on Windows (`tabbed`) and as accelerators elsewhere.
    fn edit(tabbed: bool) -> (String, Vec<Shown>) {
        let (switch, apply) = if tabbed {
            (
                check("toggle-edit", "&Edit Text\tCtrl+Q", None),
                item("apply-edits", "&Apply and Exit Editing\tCtrl+Enter", None),
            )
        } else {
            (
                check("toggle-edit", "&Edit Text", Some("Ctrl+Q")),
                item("apply-edits", "&Apply and Exit Editing", Some("CmdOrCtrl+Enter")),
            )
        };
        (
            "&Edit".to_string(),
            vec![
                switch,
                apply,
                item("discard-edits", "&Discard Changes", None),
                item("find-text", if tabbed { "&Find in Netlist…\tCtrl+F" } else { "&Find in Netlist…" }, None),
                Shown::Separator,
                item("descend", if tabbed { "Open &Subcircuit\tE" } else { "Open &Subcircuit" }, None),
                item(
                    "descend-new-window",
                    if tabbed {
                        "Open Subcircuit in New &Window…\tShift+E"
                    } else {
                        "Open Subcircuit in New &Window…"
                    },
                    None,
                ),
                item("ascend", if tabbed { "&Up to Parent Level\tCtrl+E" } else { "&Up to Parent Level" }, None),
            ],
        )
    }

    /// The Solve menu, which only the desktop offers (decision 0010): Solve, with its key shown after
    /// a tab on Windows (`tabbed`) and as the accelerator elsewhere, Close Preview, Save Solved Copy
    /// As and Settings.
    fn solve(tabbed: bool) -> (String, Vec<Shown>) {
        let keyed = |id: &str, text: &str, shown: &str, accelerator: &str| {
            if tabbed {
                item(id, &format!("{text}\t{shown}"), None)
            } else {
                item(id, text, Some(accelerator))
            }
        };
        (
            "&Solve".to_string(),
            vec![
                keyed("solve", "&Solve", "F9", "F9"),
                item("solve-close", "&Close Preview", None),
                Shown::Separator,
                keyed("solve-save", "Save Solved &Copy As...", "Ctrl+Shift+S", "CmdOrCtrl+Shift+S"),
                item("solve-save-utf8", "Save Solved Copy As UTF-8...", None),
                Shown::Separator,
                item("solve-settings", "S&ettings...", None),
            ],
        )
    }

    /// The Help menu, alike on every platform: Check for Updates, which only the desktop offers and
    /// which has no keys, then About.
    fn help() -> (String, Vec<Shown>) {
        let updates = item("check-updates", "Check for &Updates...", None);
        (
            "&Help".to_string(),
            vec![
                item("shortcuts", "&Keyboard Shortcuts...", None),
                updates,
                Shown::Separator,
                Shown::About("&About RNL Viewer".into()),
            ],
        )
    }

    /// The menu bar as it was written by hand before the table, on Windows, and the grid's submenu.
    #[test]
    fn windows_shows_the_keys_after_a_tab() {
        let menus = plan(&table(), true);
        let expected = vec![
            (
                "&File".to_string(),
                vec![
                    item("new-window", "&New Window", None),
                    item("open", "&Open File...\tCtrl+O", None),
                    item("include-file", "&Include File...", None),
                    item("include-folder", "Include &Folder...", None),
                    item("load-clipboard", "Load from &Clipboard...\tCtrl+Shift+V", None),
                    item("reload", "&Reload\tF5", None),
                    item("reload-library", "Reload &Library", None),
                    check("toggle-auto-include", "&Auto Include", None),
                    item("memory-cache", "Memory &Cache…", None),
                    Shown::Separator,
                    item("save", "&Save\tCtrl+S", None),
                    item("save-as", "Save &As...", None),
                    item("save-utf8", "Save Changes Using UTF-8", None),
                    item("save-as-utf8", "Save As UTF-8...", None),
                    Shown::Separator,
                    item("file-associations", "File &Associations...", None),
                    item("exit", "E&xit", None),
                ],
            ),
            edit(true),
            (
                "&View".to_string(),
                vec![
                    item("fit", "&Fit to Window\tF", None),
                    item("zoom-in", "Zoom &In\tCtrl++", None),
                    item("zoom-out", "Zoom &Out\tCtrl+-", None),
                    item("netlist-colors", "&Color Scheme...", None),
                    Shown::Submenu(
                        "&Background".into(),
                        vec![check("background-light", "&Light", None), check("background-dark", "&Dark", None)],
                    ),
                    grid("Show &Grid\tG"),
                    check("toggle-hover-names", "Show &Names on Hover", None),
                    check("toggle-usages", "Show &Usages\tCtrl+R", None),
                    check("toggle-currents", "Show Current &Directions\tShift+R", None),
                    languages(),
                    Shown::Separator,
                    panels(true),
                ],
            ),
            solve(true),
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
                    item("new-window", "&New Window", None),
                    item("open", "&Open File...", Some("CmdOrCtrl+O")),
                    item("include-file", "&Include File...", None),
                    item("include-folder", "Include &Folder...", None),
                    item("load-clipboard", "Load from &Clipboard...", Some("CmdOrCtrl+Shift+V")),
                    item("reload", "&Reload", Some("F5")),
                    item("reload-library", "Reload &Library", None),
                    check("toggle-auto-include", "&Auto Include", None),
                    item("memory-cache", "Memory &Cache…", None),
                    Shown::Separator,
                    item("save", "&Save", Some("CmdOrCtrl+S")),
                    item("save-as", "Save &As...", None),
                    item("save-utf8", "Save Changes Using UTF-8", None),
                    item("save-as-utf8", "Save As UTF-8...", None),
                    Shown::Separator,
                    item("exit", "E&xit", None),
                ],
            ),
            edit(false),
            (
                "&View".to_string(),
                vec![
                    item("fit", "&Fit to Window", Some("F")),
                    item("zoom-in", "Zoom &In", Some("CmdOrCtrl+=")),
                    item("zoom-out", "Zoom &Out", Some("CmdOrCtrl+-")),
                    item("netlist-colors", "&Color Scheme...", None),
                    Shown::Submenu(
                        "&Background".into(),
                        vec![check("background-light", "&Light", None), check("background-dark", "&Dark", None)],
                    ),
                    grid("Show &Grid"),
                    check("toggle-hover-names", "Show &Names on Hover", None),
                    check("toggle-usages", "Show &Usages", Some("CmdOrCtrl+R")),
                    check("toggle-currents", "Show Current &Directions", None),
                    languages(),
                    Shown::Separator,
                    panels(false),
                ],
            ),
            solve(false),
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
