//! Empty alias packages keep their package kind; empty ordinary and inline packages remain usable.
use std::path::Path;

use rnl_runtime::diag::{codes, Severity};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn package(version: &str, aliases: bool, content: bool) -> String {
    let aliases = if aliases { "@ALIAS(\"@locate\",\"@PLACE\")" } else { "" };
    let content = if content { "@RNL {}\n" } else { "" };
    format!("@RNL(type=\"header\") {{ @VERSION(\"{version}\"); @PACKAGE(name=\"empty\",version=\"1.0\"); {aliases} }}\n{content}")
}

fn read(version: &str, keyword: &str, package: &str, inline: bool) -> Interpretation {
    let reference = if inline {
        "@USE-PACKAGE(name=\"empty\") {}".into()
    } else {
        format!("@{keyword}(name=\"empty\",path=\"empty.rnl\")")
    };
    let source =
        format!("* alias boundary\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\"); {reference} }}\nR1 a b 1k\n");
    let files = MemoryProvider::new().with("main.sp", &source).with("empty.rnl", package);
    rnl_cli::interpret_file(Path::new("main.sp"), None, &files, &Options { std: false, ..Default::default() }).unwrap()
}

#[test]
fn an_alias_package_does_not_need_an_alias_to_have_its_kind() {
    for version in ["0.3", "0.4"] {
        for aliases in [false, true] {
            let package = package(version, aliases, false);
            let result = read(version, "USE-PACKAGE", &package, false);
            let errors: Vec<_> = result.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).collect();
            if version == "0.4" || aliases {
                assert_eq!(errors.len(), 1, "{:?}", result.diagnostics());
                assert_eq!(errors[0].code, codes::USE_PACKAGE_ALIAS_PACKAGE);
                assert_eq!(errors[0].loc.as_ref().unwrap().line, 2);
            } else {
                assert!(errors.is_empty(), "{errors:?}");
            }
            let result = read(version, "USE-ALIASES", &package, false);
            assert!(!result.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", result.diagnostics());
        }
    }
}

#[test]
fn empty_ordinary_records_and_inline_packages_keep_their_meaning() {
    for version in ["0.3", "0.4"] {
        for inline in [false, true] {
            let result = read(version, "USE-PACKAGE", &package(version, false, true), inline);
            assert!(!result.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", result.diagnostics());
        }
    }
}
