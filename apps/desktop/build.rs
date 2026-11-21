use std::path::Path;

fn main() {
    // Refresh the executable's Windows resources when the icon changes.
    println!("cargo:rerun-if-changed=icons/icon.ico");
    ui_languages();
    tauri_build::build()
}

/// The languages of the interface, English first, whose catalogs the page asks for (menu.rs): the one table of them,
/// `i18n/viewer-languages.json`, which scripts/i18n/viewer-locales.mjs writes from the project's list of languages and
/// the catalogs of the viewers (decision 0015), as the web viewer's build gives it to the page. Written to
/// `ui_languages.rs` in OUT_DIR, with each language's catalog, `i18n/<lang>/viewer.json`.
fn ui_languages() {
    let i18n = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../i18n");
    let table = i18n.join("viewer-languages.json");
    println!("cargo:rerun-if-changed={}", table.display());
    let text = std::fs::read_to_string(&table).unwrap_or_else(|e| panic!("{}: {e}", table.display()));
    let languages: Vec<serde_json::Value> = serde_json::from_str(&text).expect("i18n/viewer-languages.json is a list");
    let mut out = String::from(
        "/// A language of the interface, as i18n/viewer-languages.json has it, with its catalog of the viewers.\n\
         pub struct UiLanguageRow {\n\
         \x20   pub code: &'static str,\n\
         \x20   pub lang: &'static str,\n\
         \x20   pub name: &'static str,\n\
         \x20   pub english: &'static str,\n\
         \x20   pub matches: &'static [&'static str],\n\
         \x20   pub catalog: &'static str,\n\
         }\n\n\
         /// The languages of the interface, English first, which has no catalog. Written by build.rs.\n\
         pub const UI_LANGUAGES: &[UiLanguageRow] = &[\n",
    );
    for (n, language) in languages.iter().enumerate() {
        let field = |key: &str| language[key].as_str().unwrap_or_else(|| panic!("{key} of a language")).to_string();
        let (code, lang, name, english) = (field("code"), field("lang"), field("name"), field("english"));
        let matches: Vec<&str> =
            language["matches"].as_array().expect("matches").iter().filter_map(|m| m.as_str()).collect();
        let catalog = if n == 0 {
            "\"{}\"".to_string()
        } else {
            let file = i18n.join(&lang).join("viewer.json");
            println!("cargo:rerun-if-changed={}", file.display());
            format!("include_str!({:?})", file.display().to_string())
        };
        out += &format!(
            "    UiLanguageRow {{ code: {code:?}, lang: {lang:?}, name: {name:?}, english: {english:?}, matches: &{matches:?}, catalog: {catalog} }},\n"
        );
    }
    out += "];\n";
    let file = Path::new(&std::env::var("OUT_DIR").expect("OUT_DIR")).join("ui_languages.rs");
    std::fs::write(&file, out).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
}
