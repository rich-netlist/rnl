//! A package written in a language before 0.4 is not read, wherever a file of 0.4 refers to it,
//! and the names it would give cannot be found (16.1, 16.2).

use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Code, Severity};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};
use std::path::Path;

fn package(language: &str, name: &str, version: &str, references: &str, body: &str) -> String {
    // VERSION applies even to references written before it in the header.
    format!("@RNL(type=\"header\") {{ @PACKAGE(name=\"{name}\",version=\"{version}\"); {references} @VERSION(\"{language}\") }}\n@RNL {{ {body} }}\n")
}
fn file(references: &str, body: &str) -> String {
    format!("* t\n* @RNL(type=\"header\") {{ {references} @VERSION(\"0.4\") }}\nR1 a 0 1k\nR2 b 0 1k\n* @RNL {{ {body} }}\n.end\n")
}
fn symbol(name: &str, text: &str) -> String {
    format!("${name} = @DEFINE-SYMBOL {{ (0,0) @TEXT(\"{text}\") }}")
}
fn read(files: &MemoryProvider, options: &Options) -> Interpretation {
    interpret_file(Path::new("t.sp"), None, files, options).unwrap()
}
fn no_std() -> Options {
    Options { std: false, ..Options::default() }
}
fn errors(r: &Interpretation) -> Vec<(Code, String)> {
    r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).map(|d| (d.code, r.sink.render(d))).collect()
}
/// The files of the errors that a version of the language the reading does not interpret gives.
fn unsupported(r: &Interpretation) -> Vec<String> {
    r.diagnostics()
        .into_iter()
        .filter(|d| d.code == codes::VERSION_UNSUPPORTED)
        .map(|d| d.loc.as_ref().unwrap().file.clone())
        .collect()
}
fn labels(r: &Interpretation, title: &str) -> Vec<String> {
    r.model
        .levels
        .iter()
        .find(|l| l.title == title)
        .unwrap()
        .items
        .iter()
        .flat_map(|i| i.texts.values().flatten().map(|line| rnl_runtime::markup::plain(line)))
        .collect()
}

#[test]
fn a_package_of_an_earlier_language_is_not_read() {
    for language in ["0.2", "0.3", "0.4"] {
        for reference in [
            "@USE-PACKAGE(name=\"p\",version=\"1.0\",path=\"pkgs/p.rnl\");",
            "@USE-PACKAGE(name=\"p\",version=\"1.0\");",
            "@USE-PACKAGE(name=\"p\");",
        ] {
            let mut files = MemoryProvider::new()
                .with("t.sp", &file(reference, "#R1 @SET-SYMBOL $MARK => @PLACE(0,0)"))
                .with("pkgs/p.rnl", &package(language, "p", "1.0", "", &symbol("MARK", "ok")));
            files.package_dirs.push("pkgs".into());
            let r = read(&files, &no_std());
            let e = errors(&r);
            if language == "0.4" {
                assert_eq!(e, [], "{language} {reference}");
                assert_eq!(labels(&r, "t.sp"), ["ok"]);
            } else {
                assert_eq!(unsupported(&r), ["p.rnl"], "{e:?}");
                assert!(e.iter().any(|(_, m)| m.contains("MARK") && m.contains("cannot be found")), "{e:?}");
                assert!(labels(&r, "t.sp").is_empty());
                // SET-SYMBOL fails, but still passes R1 to the independent PLACE (5.5).
                assert_eq!(r.model.levels[0].items.len(), 1);
                assert_eq!(r.model.levels[0].items[0].id, "#R1");
                assert_eq!(r.model.levels[0].items[0].at, (0.0, 0.0));
            }
        }
    }
}

#[test]
fn an_indirect_reference_to_an_earlier_language_does_not_disable_independent_names() {
    for dependency in ["0.2", "0.3"] {
        let files = MemoryProvider::new()
            .with(
                "t.sp",
                &file(
                    "@USE-PACKAGE(name=\"a\",version=\"1.0\",path=\"a.rnl\");",
                    "#R1 @SET-SYMBOL $GOOD => @PLACE(0,0); #R2 @SET-SYMBOL $BAD => @PLACE(1,0)",
                ),
            )
            .with(
                "a.rnl",
                &package(
                    "0.4",
                    "a",
                    "1.0",
                    "@USE-PACKAGE(name=\"b\",version=\"1.0\",path=\"b.rnl\");",
                    &format!("{}; $BAD = $MARK", symbol("GOOD", "ok")),
                ),
            )
            .with("b.rnl", &package(dependency, "b", "1.0", "", &symbol("MARK", "bad")));
        let r = read(&files, &no_std());
        let e = errors(&r);
        assert_eq!(unsupported(&r), ["b.rnl"], "{e:?}");
        assert!(e.iter().any(|(_, m)| m.contains("MARK") && m.contains("cannot be found")), "{e:?}");
        assert_eq!(labels(&r, "t.sp"), ["ok"]);
    }
}

#[test]
fn a_tool_setting_of_an_earlier_standard_series_is_not_read_but_an_explicit_use_takes_precedence() {
    for other in ["0.2", "0.3"] {
        // The setting is consulted only when the tool adds the package itself.
        let options = Options { std_version: Some(other.into()), include_std: true, ..Options::default() };
        let files = |reference: &str| {
            let mut files = MemoryProvider::new()
                .with("t.sp", &file(reference, ""))
                .with("pkgs/std2.rnl", &package("0.2", "rnl-std-all", "0.2", "", ""))
                .with("pkgs/std3.rnl", &package("0.3", "rnl-std-all", "0.3", "", ""))
                .with("pkgs/std4.rnl", &package("0.4", "rnl-std-all", "0.4", "", ""));
            files.package_dirs.push("pkgs".into());
            files
        };
        let r = read(&files(""), &options);
        assert_eq!(errors(&r).len(), 1, "{:?}", errors(&r));
        assert_eq!(errors(&r)[0].0, codes::VERSION_UNSUPPORTED);
        assert!(r.model.levels[0].packages.is_empty());
        for reference in
            ["@USE-PACKAGE(name=\"rnl-std-all\",version=\"0.4\");", "@USE-PACKAGE(name=\"rnl-std-all\") {};"]
        {
            assert_eq!(errors(&read(&files(reference), &options)), []);
        }
    }
}

#[test]
fn an_alias_package_of_an_earlier_language_is_not_read() {
    for language in ["0.2", "0.3", "0.4"] {
        let files = MemoryProvider::new()
            .with("t.sp", &file("@USE-ALIASES(name=\"p\",version=\"1.0\",path=\"p.rnl\");", ""))
            .with("p.rnl", &format!("@RNL(type=\"header\") {{ @VERSION(\"{language}\"); @PACKAGE(name=\"p\",version=\"1.0\"); @ALIAS(\"axe\",\"axis\") }}\n"));
        let r = read(&files, &no_std());
        if language == "0.4" {
            assert_eq!(errors(&r), []);
            assert_eq!(r.imported_aliases.len(), 1);
        } else {
            assert_eq!(errors(&r).len(), 1, "{:?}", errors(&r));
            assert_eq!(errors(&r)[0].0, codes::VERSION_UNSUPPORTED);
            assert!(r.imported_aliases.is_empty());
        }
    }
}

#[test]
fn unnamed_lookup_prefers_a_package_it_can_read_and_inline_packages_use_their_file() {
    // the newest p is written in 0.3: a USE-PACKAGE that names no version takes the newest it
    // can read
    let mut files = MemoryProvider::new()
        .with(
            "t.sp",
            &file(
                &format!(
                    "@USE-PACKAGE(name=\"p\"); @USE-PACKAGE(name=\"inline\",version=\"9.0\") {{ {} }};",
                    symbol("LOCAL", "local")
                ),
                "#R1 @SET-SYMBOL $MARK => @PLACE(0,0); #R2 @SET-SYMBOL $LOCAL => @PLACE(1,0)",
            ),
        )
        .with("pkgs/p3.rnl", &package("0.3", "p", "2.0", "", &symbol("MARK", "three")))
        .with("pkgs/p4.rnl", &package("0.4", "p", "1.0", "", &symbol("MARK", "four")));
    files.package_dirs.push("pkgs".into());
    let r = read(&files, &no_std());
    assert_eq!(errors(&r), []);
    assert_eq!(labels(&r, "t.sp"), ["four", "local"]);
}
