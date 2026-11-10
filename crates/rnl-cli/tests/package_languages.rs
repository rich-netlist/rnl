//! Package references cannot cross the boundary between 0.4 and earlier languages (16.1, 16.2).

use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Code, Severity};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};
use std::path::Path;

fn package(language: &str, name: &str, version: &str, references: &str, body: &str) -> String {
    // VERSION applies even to references written before it in the header.
    format!("@RNL(type=\"header\") {{ @PACKAGE(name=\"{name}\",version=\"{version}\"); {references} @VERSION(\"{language}\") }}\n@RNL {{ {body} }}\n")
}
fn file(language: &str, references: &str, body: &str) -> String {
    format!("* t\n* @RNL(type=\"header\") {{ {references} @VERSION(\"{language}\") }}\nR1 a 0 1k\nR2 b 0 1k\n* @RNL {{ {body} }}\n.end\n")
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
fn explicit_and_unnamed_references_separate_0_4_from_older_languages() {
    for reader in ["0.2", "0.3", "0.4"] {
        for language in ["0.2", "0.3", "0.4"] {
            for reference in [
                "@USE-PACKAGE(name=\"p\",version=\"1\",path=\"pkgs/p.rnl\");",
                "@USE-PACKAGE(name=\"p\",version=\"1\");",
                "@USE-PACKAGE(name=\"p\");",
            ] {
                let mut files = MemoryProvider::new()
                    .with("t.sp", &file(reader, reference, "#R1 @SET-SYMBOL $MARK => @PLACE(0,0)"))
                    .with("pkgs/p.rnl", &package(language, "p", "1", "", &symbol("MARK", "ok")));
                files.package_dirs.push("pkgs".into());
                let r = read(&files, &no_std());
                let e = errors(&r);
                if (reader == "0.4") == (language == "0.4") {
                    assert_eq!(e, [], "{reader} {language} {reference}");
                    assert_eq!(labels(&r, "t.sp"), ["ok"]);
                } else {
                    assert_eq!(e.iter().filter(|(c, _)| *c == codes::PACKAGE_LANGUAGE_MISMATCH).count(), 1, "{e:?}");
                    assert!(e.iter().any(|(_, m)| m.contains("MARK") && m.contains("cannot be found")), "{e:?}");
                    assert!(labels(&r, "t.sp").is_empty());
                    if reader == "0.4" {
                        // SET-SYMBOL fails, but still passes R1 to the independent PLACE (5.5).
                        assert_eq!(r.model.levels[0].items.len(), 1);
                        assert_eq!(r.model.levels[0].items[0].id, "#R1");
                        assert_eq!(r.model.levels[0].items[0].at, (0.0, 0.0));
                    } else {
                        assert!(r.model.levels[0].items.is_empty());
                    }
                }
            }
        }
    }
}

#[test]
fn an_indirect_mismatched_reference_does_not_disable_independent_names() {
    for (reader, dependency) in [("0.4", "0.2"), ("0.4", "0.3"), ("0.2", "0.4"), ("0.3", "0.4")] {
        let files = MemoryProvider::new()
            .with(
                "t.sp",
                &file(
                    reader,
                    "@USE-PACKAGE(name=\"a\",version=\"1\",path=\"a.rnl\");",
                    "#R1 @SET-SYMBOL $GOOD => @PLACE(0,0); #R2 @SET-SYMBOL $BAD => @PLACE(1,0)",
                ),
            )
            .with(
                "a.rnl",
                &package(
                    reader,
                    "a",
                    "1",
                    "@USE-PACKAGE(name=\"b\",version=\"1\",path=\"b.rnl\");",
                    &format!("{}; $BAD = $MARK", symbol("GOOD", "ok")),
                ),
            )
            .with("b.rnl", &package(dependency, "b", "1", "", &symbol("MARK", "bad")));
        let r = read(&files, &no_std());
        let e = errors(&r);
        assert_eq!(e.iter().filter(|(c, _)| *c == codes::PACKAGE_LANGUAGE_MISMATCH).count(), 1, "{e:?}");
        assert!(e.iter().any(|(_, m)| m.contains("MARK") && m.contains("cannot be found")), "{e:?}");
        let d = r.diagnostics().into_iter().find(|d| d.code == codes::PACKAGE_LANGUAGE_MISMATCH).unwrap();
        assert_eq!(d.loc.as_ref().unwrap().file, "a.rnl");
        assert_eq!(labels(&r, "t.sp"), ["ok"]);
    }
}

#[test]
fn includes_keep_their_own_packages_even_with_the_same_package_name_and_version() {
    let parent =
        file("0.3", "@USE-PACKAGE(name=\"p\",version=\"1\",path=\"p3.rnl\");", "#R1 @SET-SYMBOL $MARK => @PLACE(0,0)")
            .replace(".end", ".include child.sp\n.end");
    let files = MemoryProvider::new()
        .with("t.sp", &parent)
        .with("p3.rnl", &package("0.3", "p", "1", "", &symbol("MARK", "three")))
        .with("p4.rnl", &package("0.4", "p", "1", "", &symbol("MARK", "four")))
        .with("child.sp", "* child\n* @RNL(type=\"header\") { @VERSION(\"0.4\"); @USE-PACKAGE(name=\"p\",version=\"1\",path=\"p4.rnl\") }\n.subckt child a\nR1 a 0 1k\n* @RNL { #R1 @SET-SYMBOL $MARK => @PLACE(0,0) }\n.ends\n");
    let r = read(&files, &no_std());
    assert_eq!(errors(&r), []);
    assert_eq!(labels(&r, "t.sp"), ["three"]);
    assert_eq!(labels(&r, "child"), ["four"]);
}

#[test]
fn an_include_does_not_inherit_a_package_from_another_language() {
    let files = MemoryProvider::new()
        .with("t.sp", &file("0.3", "@USE-PACKAGE(name=\"p\",version=\"1\",path=\"p.rnl\");", "").replace(".end", ".include child.sp\n.end"))
        .with("p.rnl", &package("0.3", "p", "1", "", &symbol("MARK", "bad")))
        .with("child.sp", "* child\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n.subckt child a\nR1 a 0 1k\n* @RNL { #R1 @SET-SYMBOL $MARK }\n.ends\n");
    let r = read(&files, &no_std());
    assert_eq!(errors(&r).len(), 1, "{:?}", errors(&r));
    assert!(errors(&r)[0].1.contains("MARK") && errors(&r)[0].1.contains("cannot be found"));
    assert!(r.model.levels.iter().find(|l| l.title == "child").unwrap().packages.is_empty());
}

#[test]
fn a_reference_checks_the_language_even_when_the_package_is_already_cached() {
    for (parent_version, child_version) in [("0.3", "0.4"), ("0.4", "0.3")] {
        let parent = file(
            parent_version,
            "@USE-PACKAGE(name=\"p\",version=\"1\",path=\"p.rnl\");",
            "#R1 @SET-SYMBOL $MARK => @PLACE(0,0)",
        )
        .replace(".end", ".include child.sp\n.end");
        let files = MemoryProvider::new()
            .with("t.sp", &parent)
            .with("p.rnl", &package(parent_version, "p", "1", "", &symbol("MARK", "ok")))
            .with("child.sp", &format!("* child\n* @RNL(type=\"header\") {{ @VERSION(\"{child_version}\"); @USE-PACKAGE(name=\"p\",version=\"1\",path=\"p.rnl\") }}\n.subckt child a\nR1 a 0 1k\n* @RNL {{ #R1 @SET-SYMBOL $MARK }}\n.ends\n"));
        let r = read(&files, &no_std());
        let e = errors(&r);
        assert_eq!(e.iter().filter(|(c, _)| *c == codes::PACKAGE_LANGUAGE_MISMATCH).count(), 1, "{e:?}");
        assert!(e.iter().any(|(_, m)| m.contains("MARK") && m.contains("cannot be found")), "{e:?}");
        let d = r.diagnostics().into_iter().find(|d| d.code == codes::PACKAGE_LANGUAGE_MISMATCH).unwrap();
        assert_eq!(d.loc.as_ref().unwrap().file, "child.sp");
        assert_eq!(labels(&r, "t.sp"), ["ok"]);
        assert!(labels(&r, "child").is_empty());
    }
}

#[test]
fn a_tool_override_cannot_select_a_different_language_but_an_explicit_use_takes_precedence() {
    for (reader, other) in [("0.4", "0.2"), ("0.4", "0.3"), ("0.2", "0.4"), ("0.3", "0.4")] {
        // The override is consulted in 0.4 only when the tool explicitly adds the package.
        let options = Options { std_version: Some(other.into()), include_std: reader == "0.4", ..Options::default() };
        let files = |reference: &str| {
            let mut files = MemoryProvider::new()
                .with("t.sp", &file(reader, reference, ""))
                .with("pkgs/std2.rnl", &package("0.2", "rnl-std", "0.2", "", ""))
                .with("pkgs/std3.rnl", &package("0.3", "rnl-std", "0.3", "", ""))
                .with("pkgs/std4.rnl", &package("0.4", "rnl-std", "0.4", "", ""));
            files.package_dirs.push("pkgs".into());
            files
        };
        let r = read(&files(""), &options);
        assert_eq!(errors(&r).len(), 1, "{:?}", errors(&r));
        assert_eq!(errors(&r)[0].0, codes::PACKAGE_LANGUAGE_MISMATCH);
        assert!(r.model.levels[0].packages.is_empty());
        for reference in [
            format!("@USE-PACKAGE(name=\"rnl-std\",version=\"{reader}\");"),
            "@USE-PACKAGE(name=\"rnl-std\") {};".into(),
        ] {
            assert_eq!(errors(&read(&files(&reference), &options)), []);
        }
    }
}

#[test]
fn alias_imports_obey_the_same_boundary() {
    for reader in ["0.2", "0.3", "0.4"] {
        for language in ["0.2", "0.3", "0.4"] {
            let files = MemoryProvider::new()
                .with("t.sp", &file(reader, "@USE-ALIASES(name=\"p\",version=\"1\",path=\"p.rnl\");", ""))
                .with("p.rnl", &format!("@RNL(type=\"header\") {{ @VERSION(\"{language}\"); @PACKAGE(name=\"p\",version=\"1\"); @ALIAS(\"axe\",\"axis\") }}\n"));
            let r = read(&files, &no_std());
            if (reader == "0.4") == (language == "0.4") {
                assert_eq!(errors(&r), []);
                assert_eq!(r.imported_aliases.len(), 1);
            } else {
                assert_eq!(errors(&r).len(), 1, "{:?}", errors(&r));
                assert_eq!(errors(&r)[0].0, codes::PACKAGE_LANGUAGE_MISMATCH);
                assert!(r.imported_aliases.is_empty());
            }
        }
    }
}

#[test]
fn unnamed_lookup_prefers_the_same_language_and_inline_packages_use_their_file() {
    let mut files = MemoryProvider::new()
        .with(
            "t.sp",
            &file(
                "0.3",
                &format!(
                    "@USE-PACKAGE(name=\"p\"); @USE-PACKAGE(name=\"inline\",version=\"9\") {{ {} }};",
                    symbol("LOCAL", "local")
                ),
                "#R1 @SET-SYMBOL $MARK => @PLACE(0,0); #R2 @SET-SYMBOL $LOCAL => @PLACE(1,0)",
            ),
        )
        .with("pkgs/p3.rnl", &package("0.3", "p", "1", "", &symbol("MARK", "three")))
        .with("pkgs/p4.rnl", &package("0.4", "p", "2", "", &symbol("MARK", "four")));
    files.package_dirs.push("pkgs".into());
    let r = read(&files, &no_std());
    assert_eq!(errors(&r), []);
    assert_eq!(labels(&r, "t.sp"), ["three", "local"]);
}
