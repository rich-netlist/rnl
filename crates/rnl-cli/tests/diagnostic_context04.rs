//! Draft diagnostics retain package ownership and explain parameter types (5.3, 7.6, 16.5).
use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Code};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};
use std::path::Path;

fn read(files: &MemoryProvider, path: &str) -> Interpretation {
    interpret_file(Path::new(path), None, files, &Options { std: false, ..Options::solver() }).unwrap()
}
fn package(version: &str, header: &str) -> String {
    format!("@RNL(type=\"header\") {{ @VERSION(\"{version}\"); @PACKAGE(name=\"p\",version=\"1.0\"); {header} }}\n@RNL {{ $D = @DEFINE(roles=[\"i\"]) {{}}; $E = @DEFINE(roles=[\"j\"]) {{}} }}\n")
}
fn source(version: &str, header: &str, body: &str) -> String {
    format!("* diagnostic context\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\"); {header} }}\nR1 a 0 1k\n* @RNL {{ {body} }}\n")
}
fn cases() -> [(Code, &'static str); 5] {
    [
        (codes::ALIAS_CONFLICT, r#"@ALIAS("$A","$D"); @ALIAS("$A","$E")"#),
        (codes::ALIAS_ORIGINAL_IS_ALIAS, r#"@ALIAS("$A","$D"); @ALIAS("$B","$A")"#),
        (codes::ALIAS_IS_ORIGINAL, r#"@ALIAS("$B","$A"); @ALIAS("$A","$D")"#),
        (codes::ALIAS_SCOPED_LIKE_PRIORITY, r#"@ALIAS("priority","other",definition="$D")"#),
        (
            codes::ALIAS_SCOPED_CONFLICT,
            r#"@ALIAS("input","first",definition="$D"); @ALIAS("input","second",definition="$D")"#,
        ),
    ]
}

#[test]
fn alias_conflicts_keep_the_declaring_package_when_checked_or_imported() {
    for (code, header) in cases() {
        let files = MemoryProvider::new()
            .with("p.rnl", &package("0.4", header))
            .with("main.sp", &source("0.4", r#"@USE-PACKAGE(name="p",path="p.rnl")"#, ""));
        for path in ["p.rnl", "main.sp"] {
            let r = read(&files, path);
            let diagnostics: Vec<_> = r.diagnostics().into_iter().filter(|d| d.code == code).collect();
            assert_eq!(diagnostics.len(), 1, "{path}: {:?}", r.diagnostics());
            assert_eq!(diagnostics[0].in_package, "p");
            assert_eq!(diagnostics[0].loc.as_ref().unwrap().file, "p.rnl");
        }
    }
}

#[test]
fn file_alias_conflicts_are_not_misattributed_to_an_imported_package() {
    let (code, header) = cases()[0];
    let header = format!("@USE-PACKAGE(name=\"p\",path=\"p.rnl\"); {header}");
    let r = read(
        &MemoryProvider::new().with("main.sp", &source("0.4", &header, "")).with("p.rnl", &package("0.4", "")),
        "main.sp",
    );
    let d = r.sink.all().iter().find(|d| d.code == code).unwrap();
    assert!(d.in_package.is_empty());
    assert_eq!(d.loc.as_ref().unwrap().file, "main.sp");
}

#[test]
fn parameter_errors_say_which_type_is_required() {
    for (body, expected) in [
        (r#"(0,0) @CIRCLE(radius="bad")"#, "a number"),
        (r#"(0,0) @TEXT(1)"#, "a string"),
        (r#"(0,0) @TEXT("x",fix_rotate=1)"#, "true or false"),
        (r#"$D = @DEFINE(roles=true) {}"#, "an array"),
        (r#"(0,0) @TEXT("x",textcolor=1)"#, "a colour string"),
        (r#"$S = @DEFINE-SYMBOL { (0,0) @PIN(terminal=true) }"#, "a terminal name"),
    ] {
        let r = read(&MemoryProvider::new().with("main.sp", &source("0.4", "", body)), "main.sp");
        let d = r
            .sink
            .all()
            .iter()
            .find(|d| d.code == codes::PARAM_WRONG_TYPE)
            .unwrap_or_else(|| panic!("{body}: {:?}", r.diagnostics()));
        assert!(d.message.contains(expected), "{}", d.message);
        assert_eq!(d.loc.as_ref().unwrap().line, 4);
    }
}

#[test]
fn header_and_content_parameters_share_the_type_description() {
    for (header, body) in [("@USE-PACKAGE(name=1)", ""), ("", "@USE-PACKAGE(name=1)")] {
        let r = read(&MemoryProvider::new().with("main.sp", &source("0.4", header, body)), "main.sp");
        let d = r.sink.all().iter().find(|d| d.code == codes::PARAM_WRONG_TYPE).unwrap();
        assert_eq!(d.message, "the name of @USE-PACKAGE must be a string (5.3)");
    }
}

#[test]
fn scoped_parameter_messages_point_to_the_writing_file() {
    let main="* main\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n.include child.sp\n* @RNL(scope=\"child\") { (0,0) @CIRCLE(radius=\"bad\") }\n";
    let child = "* child\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n.subckt child a\nR1 a 0 1k\n.ends child\n";
    let r = read(&MemoryProvider::new().with("main.sp", main).with("child.sp", child), "main.sp");
    let d = r.sink.all().iter().find(|d| d.code == codes::PARAM_WRONG_TYPE).unwrap();
    assert!(d.message.contains("must be a number"), "{}", d.message);
    assert_eq!(d.loc.as_ref().unwrap().file, "main.sp");
}
