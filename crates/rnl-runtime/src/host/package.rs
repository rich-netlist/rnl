//! The RNL package format: a package file (2.7), a pure RNL file, which USE-PACKAGE finds (16.2)
//! and which is checked on its own when it is opened ([`crate::check_package`]). It claims the files
//! whose suffix is `.rnl`, the suggested one, and a file of any other suffix whose first record is a
//! header record with PACKAGE that starts a line, which no netlist writes: a netlist carries its
//! records in comments. A package file is not loaded with an include (2.7): one that a netlist
//! includes is kept as text and not read, and the include is an error.

use crate::host::format::{FileRead, Format, Reading};
use crate::host::{file_name, Dialect};

/// The name of the RNL package format, as settings and requests give it.
pub const NAME: &str = "rnl";

/// The format of the package files.
pub struct Package;

/// Whether `text` is that of a package file by its content: its first record starts a line, as
/// only a package file's can (2.7), is a header record, and holds PACKAGE (16.2).
pub fn is_package_text(text: &str) -> bool {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let Some(at) = text.find("@RNL") else {
        return false;
    };
    if at != text[..at].rfind('\n').map_or(0, |k| k + 1) {
        return false;
    }
    let rest = &text[at + "@RNL".len()..];
    let head: String = rest.chars().filter(|c| !c.is_whitespace()).take("(type=\"header\")".len()).collect();
    if head != "(type=\"header\")" {
        return false;
    }
    // PACKAGE in the header, before the next record
    let header = &rest[..rest.find("@RNL").unwrap_or(rest.len())];
    header
        .match_indices("@PACKAGE")
        .any(|(k, w)| header[k + w.len()..].chars().find(|c| !c.is_whitespace()).is_some_and(|c| c == '('))
}

impl Format for Package {
    fn name(&self) -> &'static str {
        NAME
    }

    fn title(&self) -> &'static str {
        "RNL package"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["rnl"]
    }

    fn dialect(&self) -> Dialect {
        Dialect { name: NAME.into(), case_sensitive: false, ground_aliases: Vec::new() }
    }

    fn claims(&self, path: &str, text: &str) -> bool {
        let name = file_name(path).to_lowercase();
        name.rsplit_once('.').is_some_and(|(stem, e)| !stem.is_empty() && e == "rnl") || is_package_text(text)
    }

    /// Keep the file's text; a package file that a netlist includes is an error at its include, as
    /// it is not loaded so (2.7).
    fn read_file(&self, cx: &mut Reading, file: &FileRead) {
        let Some(text) = cx.provider.read(file.path) else {
            return;
        };
        let fname = cx.display(file.path);
        if !cx.doc.sources.iter().any(|(f, _)| *f == fname) {
            cx.doc.sources.push((fname.clone(), text));
        }
        if let Some(site) = cx.doc.levels[file.level].include_site.clone() {
            let shown = file_name(&fname);
            cx.sink.error(
                format!(
                    "`{shown}` is a package file and is not loaded with an include (2.7); USE-PACKAGE finds it (16.2)"
                ),
                Some(site),
            );
        }
    }

    fn finish(&self, _cx: &mut Reading) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_package_file_is_known_by_its_first_record() {
        let package = "@RNL(type=\"header\") { @VERSION(\"0.3\"); @PACKAGE(name=\"acme\", version=\"1.0\")}\n@RNL {}\n";
        assert!(is_package_text(package));
        // remarks before it, a byte order mark, and the header over several lines
        assert!(is_package_text(&format!("ACME's package\n\n{package}")));
        assert!(is_package_text(&format!("\u{feff}{package}")));
        assert!(is_package_text("@RNL (type = \"header\") {\n  @VERSION(\"0.3\");\n  @PACKAGE (name=\"acme\")\n}\n"));
        // a netlist carries its records in comments; a header without PACKAGE, and a first record
        // that is no header, are no package's
        assert!(!is_package_text(&format!("* t\n* {package}")));
        assert!(!is_package_text("// t\n// @RNL(type=\"header\") { @PACKAGE(name=\"acme\")}\n"));
        assert!(!is_package_text("@RNL(type=\"header\") { @VERSION(\"0.3\")}\n@RNL { @PACKAGE(name=\"x\") }\n"));
        assert!(!is_package_text("@RNL(type=\"header\") { @USE-PACKAGE(name=\"acme\")}\n"));
        assert!(!is_package_text("@RNL { @PACKAGE(name=\"acme\") }\n"));
        assert!(!is_package_text("* title\nR1 a b 1k\n.end\n"));
        assert!(Package.claims("/w/acme.RNL", ""));
        assert!(Package.claims("/w/acme.pkg", package));
        assert!(!Package.claims("/w/.rnl", ""));
        assert!(!Package.claims("/w/top.sp", "* t\n* @RNL(type=\"header\") { @PACKAGE(name=\"acme\")}\n"));
    }
}
