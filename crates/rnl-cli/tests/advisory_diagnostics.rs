//! Advisories from the 0.3 profile: comment compatibility, visible-name typos and trailing RNL.

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn read(path: &str, text: &str, extra: &[(&str, &str)]) -> Interpretation {
    let mut files = MemoryProvider::new().with(path, text);
    for (name, body) in extra {
        files = files.with(name, body);
    }
    let r = interpret_file(Path::new(path), None, &files, &Options { std: false, ..Options::default() }).unwrap();
    let errors: Vec<_> =
        r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).map(|d| r.sink.render(d)).collect();
    assert!(errors.is_empty(), "{errors:?}");
    r
}

fn typo_words(r: &Interpretation) -> Vec<String> {
    r.diagnostics()
        .into_iter()
        .filter_map(|d| {
            d.message.strip_prefix("the free-text word ").map(|m| m.split_whitespace().next().unwrap().to_string())
        })
        .collect()
}

#[test]
fn non_ascii_comment_advice_is_located_once_per_file_and_keeps_the_drawing() {
    for (file, text, line) in [
        ("t.sp", "* title\n* ordinary 注释\nR1 a b 1k ; 第二条注释\n* @RNL { #R1 @PLACE(2,3) }\n.end\n", 2),
        ("t.scs", "// title\nsimulator lang=spectre\nR1 (a b) resistor r=1k // 注释\n// 更多注释\n// @RNL { #R1 @PLACE(2,3) }\n", 3),
        ("mixed.scs", "// title\nsimulator lang=spice\nR1 a b 1k $ 注释\n* @RNL { #R1 @PLACE(2,3) }\n.end\n", 3),
        ("continued.scs", "// title\nsimulator lang=spectre\nR1 (a b) resistor \\\n+ r=1k // 注释\n// @RNL { #R1 @PLACE(2,3) }\n", 4),
    ] {
        let r = read(file, text, &[]);
        let hints: Vec<_> = r.diagnostics().into_iter().filter(|d| d.message.contains("non-ASCII characters in host comments")).collect();
        assert_eq!(hints.len(), 1, "{file}");
        assert_eq!(hints[0].severity, Severity::Hint);
        let loc = hints[0].loc.as_ref().unwrap();
        assert_eq!(loc.file, file);
        assert_eq!(loc.line, line);
        assert_eq!(loc.col as usize, text.lines().nth(line as usize - 1).unwrap().chars().position(|c| !c.is_ascii()).unwrap() + 1);
        assert_eq!(r.model.levels[0].items.iter().find(|i| i.id == "#R1").unwrap().at, (2.0,3.0));
    }
}

#[test]
fn each_included_file_gets_its_own_comment_compatibility_hint() {
    let r = read(
        "t.sp",
        "* title\n* 主文件\n.include child.inc\n.include child.inc\n.end\n",
        &[("child.inc", "* 子文件\n* 注释\n")],
    );
    let mut files: Vec<_> = r
        .diagnostics()
        .into_iter()
        .filter(|d| d.message.contains("non-ASCII characters in host comments"))
        .map(|d| d.loc.as_ref().unwrap().file.clone())
        .collect();
    files.sort();
    assert_eq!(files, ["child.inc", "t.sp"]);
}

#[test]
fn spectre_spice_mode_warns_about_ignored_rnl_after_the_main_end() {
    let text = "// title\nsimulator lang=spice\nR1 a b 1k\n* @RNL { #R1 @PLACE(2,3); (0,0) @TEXT(\"kept\") }\n.end\n* @RNL { (0,0) @TEXT(\"ignored\") }\nsimulator lang=spectre\n// @RNL { (0,0) @TEXT(\"also ignored\") }\n";
    let r = read("t.scs", text, &[]);
    let lines: Vec<_> = r
        .diagnostics()
        .into_iter()
        .filter(|d| d.message == "RNL after `.end` of the main file is not read (2.2)")
        .map(|d| {
            assert_eq!(d.severity, Severity::Warning);
            d.loc.as_ref().unwrap().line
        })
        .collect();
    assert_eq!(lines, [6, 8]);
    assert_eq!(r.doc.levels[0].end_line, 5);
    // Check the authored markers independently of the resistor symbol's own labels.
    let texts: Vec<_> = r.model.levels.iter().flat_map(|l| l.texts.iter().map(|t| t.markup.as_str())).collect();
    assert!(texts.contains(&"kept"), "pre-.end text missing: {texts:?}");
    for ignored in ["ignored", "also ignored"] {
        assert!(!texts.contains(&ignored), "post-.end text was drawn: {ignored}: {texts:?}");
    }
    assert_eq!(r.model.levels[0].items.iter().find(|i| i.id == "#R1").unwrap().at, (2.0, 3.0));
    // .end in an included file does not terminate the main file or suppress its remaining records.
    let r = read(
        "t.scs",
        "// title\nsimulator lang=spice\n.include child.inc\nR1 a b 1k\nX1 a b child\n.end\n",
        &[("child.inc", "* child\n.subckt child a b\n.end\n* @RNL { (0,0) @TEXT(\"included\") }\n.ends child\n")],
    );
    assert!(r.model.levels.iter().any(|l| l.texts.iter().any(|t| t.markup == "included")));
    assert!(!r.diagnostics().iter().any(|d| d.message.contains("RNL after `.end`")));
}

#[test]
fn free_words_see_forward_declarations_across_records_and_definition_bodies() {
    let text = "* title\nR1 a b 1k\n* @RNL { later (0,0) @TEXT(\"first\") }\n* @RNL { %later=(1,2); $D = @DEFINE(roles=[\"r\"],parameters=[[\"gap\",1]]) { local gap r (0,0) @TEXT(\"body\"); %local=(1,1) }; #R1 @AS $D; local (2,2) @TEXT(\"outside\") }\n.end\n";
    let r = read("t.sp", text, &[]);
    let mut words = typo_words(&r);
    words.sort();
    assert_eq!(words, ["gap", "later", "local", "r"]);
    let later = r.diagnostics().into_iter().find(|d| d.message.starts_with("the free-text word later ")).unwrap();
    assert_eq!(later.loc.as_ref().unwrap().line, 3);
    assert_eq!(r.model.levels[0].texts.len(), 3);
}

#[test]
fn free_words_use_inherited_and_scope_published_names_without_leaking_private_names() {
    let text = "* title\n.subckt cell a b\nR1 a b 1k\n* @RNL { Future Scoped Private (0,0) @TEXT(\"child\") }\n.ends cell\n* @RNL { $Future = @DEFINE-STYLE {}; %Private=(1,1) }\n* @RNL(scope=\"cell\") { %Scoped=(2,2) }\nX1 a b cell\n.end\n";
    let r = read("t.sp", text, &[]);
    let mut words = typo_words(&r);
    words.sort();
    assert_eq!(words, ["Future", "Scoped"]);
    assert!(r.model.levels.iter().find(|l| l.title == "cell").unwrap().texts.iter().any(|t| t.markup == "child"));
}

#[test]
fn package_names_are_visible_but_package_advice_and_private_names_stay_local() {
    let package = "@RNL(type=\"header\") { @VERSION(\"0.3\"); @PACKAGE(name=\"p\",version=\"1.0\") }\n@RNL { $Drawing = @DEFINE-SYMBOL { Forward (0,0) @TEXT(\"inside\") }; $Forward = @DEFINE-STYLE {}; $Visible = @DEFINE-STYLE {}; %Hidden=(0,0) }\n";
    let text = "* title\n* @RNL(type=\"header\") { @VERSION(\"0.3\"); @USE-PACKAGE(name=\"p\",path=\"p.rnl\") }\n* @RNL { Visible Hidden (0,0) @TEXT(\"outside\") }\n.end\n";
    let r = read("t.sp", text, &[("p.rnl", package)]);
    assert_eq!(typo_words(&r), ["Visible"]);
    let p = read("p.rnl", package, &[]);
    assert_eq!(typo_words(&p), ["Forward"]);
}

#[test]
fn a_definitions_own_name_is_not_visible_inside_its_body() {
    let r = read("t.sp", "* title\n* @RNL { $Self = @DEFINE-SYMBOL { Self (0,0) @CIRCLE(radius=1) } }\n.end\n", &[]);
    assert!(typo_words(&r).is_empty());
}
