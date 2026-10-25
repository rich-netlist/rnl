use std::path::Path;

fn main() {
    // Refresh the executable's Windows resources when the icon changes.
    println!("cargo:rerun-if-changed=icons/icon.ico");
    ui_languages();
    tauri_build::build()
}

/// The languages of the interface besides English, which the menus list and whose catalogs the page
/// asks for (menu.rs): those of the project's list of languages, `i18n/languages.json`, that are shown
/// (`published`) and have a catalog of the viewers, `i18n/<code>/viewer.json` (decision 0015). Written
/// to `ui_languages.rs` in OUT_DIR, as the web viewer's build writes them for the page
/// (scripts/i18n/viewer-locales.mjs).
fn ui_languages() {
    let i18n = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../i18n");
    println!("cargo:rerun-if-changed={}", i18n.display());
    let registry = i18n.join("languages.json");
    let text = std::fs::read_to_string(&registry).unwrap_or_else(|e| panic!("{}: {e}", registry.display()));
    let config: serde_json::Value = serde_json::from_str(&text).expect("i18n/languages.json is JSON");
    let mut out = String::from(
        "/// The languages of the interface besides English: the code of i18n/languages.json, the one the\n\
         /// page knows it by, its name in itself, the first parts of the system languages that ask for it,\n\
         /// and its catalog of the viewers. Written by build.rs.\n\
         pub const UI_LANGUAGES: &[(&str, &str, &str, &[&str], &str)] = &[\n",
    );
    for entry in config["languages"].as_array().expect("languages") {
        let code = entry["code"].as_str().expect("code");
        let catalog = i18n.join(code).join("viewer.json");
        if code == config["default"] || entry["status"] != "published" || !catalog.is_file() {
            continue;
        }
        let viewer = entry["codes"]["viewer"].as_str().unwrap_or(code);
        let name = entry["names"]["viewer"].as_str().or(entry["name"].as_str()).expect("name");
        let matches: Vec<&str> = entry["matches"]
            .as_array()
            .map_or_else(|| vec![code], |m| m.iter().filter_map(serde_json::Value::as_str).collect());
        out += &format!("    ({code:?}, {viewer:?}, {name:?}, &{matches:?}, include_str!({:?})),\n", catalog.display());
    }
    out += "];\n";
    let file = Path::new(&std::env::var("OUT_DIR").expect("OUT_DIR")).join("ui_languages.rs");
    std::fs::write(&file, out).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
}
