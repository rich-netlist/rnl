//! The menu bar, drawn as the page plans it (apps/viewer-ui/native-menu.js): from the command table it shares with the
//! web viewer (`apps/viewer-ui/commands.json`), with the commands this app offers, translated, with their keys. The page
//! sends the plan at start and again in another language; this module only draws it, and keeps the items whose check
//! marks and enabled state the page sets. It also keeps the texts of the file dialogs the app shows, as the page
//! translated them.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use serde::Deserialize;
use tauri::menu::{CheckMenuItem, CheckMenuItemBuilder, Menu, MenuItemBuilder, MenuItemKind, Submenu, SubmenuBuilder};
use tauri::{AppHandle, Manager, State, Wry};

// UI_LANGUAGES, the languages of the interface besides English, from i18n/ (build.rs, decision 0015)
include!(concat!(env!("OUT_DIR"), "/ui_languages.rs"));

/// An entry of the menu bar, as the page plans it.
#[derive(Deserialize, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Entry {
    /// A command: its id, its text (with the keys shown after a tab, on Windows), the accelerator that runs it, and
    /// whether it has a check mark.
    Item {
        id: String,
        text: String,
        accelerator: Option<String>,
        check: bool,
    },
    /// A submenu, or a menu of the bar: its text and its entries; `edit` marks the Edit menu, which on macOS begins
    /// with the system's editing commands.
    Submenu {
        text: String,
        items: Vec<Entry>,
        #[serde(default)]
        edit: bool,
    },
    /// The About item, which the page answers.
    About {
        text: String,
    },
    Separator,
}

/// The menu bar as the page plans it, and the texts of the file dialogs, by their English.
#[derive(Deserialize)]
pub struct Plan {
    menus: Vec<Entry>,
    texts: HashMap<String, String>,
}

/// The items drawn last, whose check marks and enabled state the page sets, and the texts of the file dialogs.
#[derive(Default)]
pub struct Drawn(Mutex<DrawnItems>);

#[derive(Default)]
struct DrawnItems {
    checks: HashMap<String, CheckMenuItem<Wry>>,
    items: Vec<MenuItemKind<Wry>>,
    texts: HashMap<String, String>,
}

/// The menu bar at start, before the page has planned it: none.
pub fn menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    Menu::new(app)
}

/// Draw the menu bar the page planned, in place of the one drawn before.
#[tauri::command]
pub fn set_menu(app: AppHandle, drawn: State<'_, Drawn>, plan: Plan) -> Result<(), String> {
    let mut items = DrawnItems { texts: plan.texts, ..Default::default() };
    let menu = Menu::new(&app).map_err(|e| e.to_string())?;
    for entry in plan.menus {
        if let Entry::Submenu { text, items: inner, edit } = entry {
            let submenu = submenu(&app, &text, inner, edit, &mut items).map_err(|e| e.to_string())?;
            menu.append(&submenu).map_err(|e| e.to_string())?;
        }
    }
    app.set_menu(menu).map_err(|e| e.to_string())?;
    *drawn.0.lock().map_err(|e| e.to_string())? = items;
    Ok(())
}

/// A text of the file dialogs, by its English, as the page translated it; the English before the page sent them.
pub fn text(app: &AppHandle, source: &str) -> String {
    let drawn = app.state::<Drawn>();
    let items = drawn.0.lock().unwrap_or_else(|e| e.into_inner());
    items.texts.get(source).cloned().unwrap_or_else(|| source.to_string())
}

/// A menu of the menu bar, or a submenu, with its entries; its items go into `drawn`.
fn submenu(
    app: &AppHandle,
    label: &str,
    entries: Vec<Entry>,
    edit: bool,
    drawn: &mut DrawnItems,
) -> tauri::Result<Submenu<Wry>> {
    let mut menu = SubmenuBuilder::new(app, label);
    #[cfg(target_os = "macos")]
    if edit {
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
    #[cfg(not(target_os = "macos"))]
    let _ = edit;
    for entry in entries {
        menu = match entry {
            Entry::Separator => menu.separator(),
            Entry::About { text } => {
                let item = MenuItemBuilder::with_id("about", &text).build(app)?;
                drawn.items.push(MenuItemKind::MenuItem(item.clone()));
                menu.item(&item)
            }
            Entry::Submenu { text, items, edit } => menu.item(&submenu(app, &text, items, edit, drawn)?),
            Entry::Item { id, text, accelerator, check: true } => {
                let mut b = CheckMenuItemBuilder::with_id(&id, &text);
                if let Some(a) = accelerator {
                    b = b.accelerator(a);
                }
                let item = b.build(app)?;
                drawn.items.push(MenuItemKind::Check(item.clone()));
                let menu = menu.item(&item);
                drawn.checks.insert(id, item);
                menu
            }
            Entry::Item { id, text, accelerator, check: false } => {
                let mut b = MenuItemBuilder::with_id(id, &text);
                if let Some(a) = accelerator {
                    b = b.accelerator(a);
                }
                let item = b.build(app)?;
                drawn.items.push(MenuItemKind::MenuItem(item.clone()));
                menu.item(&item)
            }
        };
    }
    menu.build()
}

/// Set the check mark of a command, such as "toggle-netlist".
#[tauri::command]
pub fn set_view_check(drawn: State<'_, Drawn>, id: String, checked: bool) {
    if let Some(item) = drawn.0.lock().unwrap_or_else(|e| e.into_inner()).checks.get(&id) {
        let _ = item.set_checked(checked);
    }
}

/// Enable or disable a command, at whatever depth of submenus it is.
#[tauri::command]
pub fn set_command_enabled(drawn: State<'_, Drawn>, id: String, enabled: bool) {
    let drawn = drawn.0.lock().unwrap_or_else(|e| e.into_inner());
    match drawn.items.iter().find(|item| item.id().as_ref() == id.as_str()) {
        Some(MenuItemKind::MenuItem(item)) => {
            let _ = item.set_enabled(enabled);
        }
        Some(MenuItemKind::Check(item)) => {
            let _ = item.set_enabled(enabled);
        }
        _ => {}
    }
}

/// The catalog of each language of the interface, by the code the page knows it by, without the
/// entries that have no translation yet (null).
static CATALOGS: LazyLock<HashMap<&str, HashMap<String, String>>> = LazyLock::new(|| {
    UI_LANGUAGES
        .iter()
        .map(|language| {
            let catalog: HashMap<String, Option<String>> =
                serde_json::from_str(language.catalog).expect("a UI catalog");
            (language.code, catalog.into_iter().filter_map(|(key, text)| Some((key, text?))).collect())
        })
        .collect()
});

/// A language of the interface, as the page takes it (i18n.js) and scripts/i18n/viewer-locales.mjs gives it: the
/// code it knows it by, the code of i18n/languages.json, its name in itself, its name in English, and the first parts
/// of the system languages that ask for it.
#[derive(serde::Serialize)]
pub struct UiLanguage {
    code: &'static str,
    lang: &'static str,
    name: &'static str,
    english: &'static str,
    matches: &'static [&'static str],
}

/// The languages of the interface, English first.
#[tauri::command]
pub fn ui_languages() -> Vec<UiLanguage> {
    UI_LANGUAGES
        .iter()
        .map(|l| UiLanguage { code: l.code, lang: l.lang, name: l.name, english: l.english, matches: l.matches })
        .collect()
}

/// The catalog of a language of the interface, {English: translation}.
#[tauri::command]
pub fn ui_catalog(language: String) -> Result<HashMap<String, String>, String> {
    CATALOGS.get(language.as_str()).cloned().ok_or_else(|| format!("No UI catalog for {language}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_plan_the_page_sends_is_read() {
        let plan: Plan = serde_json::from_str(
            r#"{"menus":[{"kind":"submenu","text":"&File","items":[
                {"kind":"item","id":"open","text":"&Open...","accelerator":"CmdOrCtrl+O","check":false},
                {"kind":"separator"},
                {"kind":"submenu","text":"&Grid","items":[{"kind":"item","id":"grid-dots","text":"&Dots","accelerator":null,"check":true}]},
                {"kind":"about","text":"&About RNL Viewer"}]},
              {"kind":"submenu","text":"&Edit","edit":true,"items":[]}],
              "texts":{"Save As":"另存为"}}"#,
        )
        .unwrap();
        assert_eq!(plan.texts["Save As"], "另存为");
        let Entry::Submenu { text, items, edit } = &plan.menus[0] else { panic!("a menu") };
        assert_eq!((text.as_str(), *edit), ("&File", false));
        assert_eq!(
            items[0],
            Entry::Item {
                id: "open".into(),
                text: "&Open...".into(),
                accelerator: Some("CmdOrCtrl+O".into()),
                check: false
            }
        );
        assert_eq!(items[1], Entry::Separator);
        assert!(
            matches!(&items[2], Entry::Submenu { items, .. } if matches!(&items[0], Entry::Item { check: true, .. }))
        );
        assert_eq!(items[3], Entry::About { text: "&About RNL Viewer".into() });
        assert!(matches!(&plan.menus[1], Entry::Submenu { edit: true, .. }));
    }

    #[test]
    fn the_languages_are_those_of_the_one_table_english_first() {
        let languages = ui_languages();
        assert_eq!(languages[0].code, "en");
        assert_eq!(languages[0].english, "English");
        assert!(languages.len() > 1);
        for language in &languages[1..] {
            assert!(!language.english.is_empty(), "{}", language.code);
            assert!(CATALOGS.contains_key(language.code), "{}", language.code);
        }
    }
}
