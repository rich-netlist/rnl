//! Name formats of 0.4 (13.6), read by the version of the file that wrote the text.

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_render::{primitives, Kind, RenderOptions};
use rnl_runtime::diag::{codes, Severity};
use rnl_runtime::markup::{plain, Line, Run};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn file(version: &str, body: &str) -> String {
    format!(
        "* name formats\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR12 a b 1k\n* @RNL {{\n{}\n* }}\n.end\n",
        body.lines().map(|line| format!("* {line}")).collect::<Vec<_>>().join("\n")
    )
}

fn read(files: &MemoryProvider) -> Interpretation {
    let r =
        interpret_file(Path::new("t.sp"), None, files, &Options { std: false, problems: true, ..Options::default() })
            .unwrap();
    let errors: Vec<_> =
        r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).map(|d| r.sink.render(d)).collect();
    assert!(errors.is_empty(), "{errors:?}");
    r
}

fn runs(line: &Line) -> Vec<(&str, i8)> {
    line.iter()
        .map(|r| match r {
            Run::Text { text, style } => (text.as_str(), style.level),
            Run::Frac { .. } => panic!("unexpected fraction"),
        })
        .collect()
}

fn unknowns(r: &Interpretation) -> usize {
    r.sink.all().iter().filter(|d| d.code == codes::MARKUP_UNKNOWN_COMMAND).count()
}

#[test]
fn text_formats_are_versioned_in_the_drawing_and_solver_input() {
    for version in ["0.2", "0.3", "0.4"] {
        let source = file(version, r#"(0,0) @TEXT("\subdigits{R12}"); (10,0) @TEXT("\subrest{VDD}")"#);
        let r = read(&MemoryProvider::new().with("t.sp", &source));
        assert_eq!(unknowns(&r), if version == "0.4" { 0 } else { 2 });
        let texts = &r.model.levels[0].texts;
        assert_eq!(texts.len(), 2);
        for (text, expected) in texts.iter().zip([vec![("R", 0), ("12", -1)], vec![("V", 0), ("DD", -1)]]) {
            if version == "0.4" {
                assert_eq!(runs(&text.lines[0]), expected);
            } else {
                assert_eq!(runs(&text.lines[0]), vec![(text.markup.as_str(), 0)]);
            }
            let solver_text =
                r.problems[0].objects.iter().filter_map(|o| o.text.as_ref()).find(|t| t.lines == text.lines);
            assert!(solver_text.is_some(), "text must reach the solver with its formatting");
        }
    }
}

#[test]
fn literal_attributes_are_split_after_expansion_without_becoming_markup() {
    let source = file(
        "0.4",
        r#"
        $S = @DEFINE-SYMBOL { @LINE {(0,0) (1,1)} };
        #R12 @SET-SYMBOL $S => @PLACE(0,0);
        #R12 @TEXT("\textit{\subdigits{${name}}}", key="name")
    "#,
    );
    let r = read(&MemoryProvider::new().with("t.sp", &source));
    assert_eq!(unknowns(&r), 0);
    let text = r.model.levels[0].texts.iter().find(|t| t.key.as_deref() == Some("name")).unwrap();
    assert_eq!(runs(&text.lines[0]), vec![("R", 0), ("12", -1)]);
    assert!(text.lines[0].iter().all(|r| matches!(r, Run::Text { style, .. } if style.italic)));
}

#[test]
fn package_symbols_keep_their_own_text_and_pin_label_version() {
    for (main, included) in [("0.4", "0.3"), ("0.3", "0.4")] {
        let source = file(main, r#"@USE-PACKAGE(name="s", path="s.rnl"); #R12 @SET-SYMBOL $S => @PLACE(0,0)"#);
        let symbol = format!(
            "@RNL(type=\"header\") {{ @VERSION(\"{included}\"); @PACKAGE(name=\"s\", version=\"1.0\") }}\n@RNL {{\n\
             $S = @DEFINE-SYMBOL {{\n\
             (0,0) @PIN(terminal=1, direction=\"left\", label=\"\\subdigits{{Vout1}}\");\n\
             (1,0) @TEXT(\"\\subrest{{${{name}}}}\");\n\
             ? @TEXT(\"\\subdigits{{${{name}}}}\", key=\"name\")\n\
             }}\n}}\n"
        );
        let r = read(&MemoryProvider::new().with("t.sp", &source).with("s.rnl", &symbol));
        assert_eq!(unknowns(&r), if included == "0.4" { 0 } else { 3 });
        let level = &r.model.levels[0];
        let symbol = &level.symbols[level.items[0].symbol];
        assert_eq!(symbol.pins[0].label.as_deref(), Some(r"\subdigits{Vout1}"));
        assert_eq!(symbol.pins[0].label_name_formats, included == "0.4");
        let drawn = primitives(level, &RenderOptions::default());
        let texts: Vec<_> = drawn
            .iter()
            .filter_map(|p| match &p.kind {
                Kind::Text(t) => Some(t),
                _ => None,
            })
            .collect();
        assert_eq!(texts.len(), 3);
        if included == "0.4" {
            assert!(texts.iter().any(|t| runs(&t.lines[0]) == vec![("Vout", 0), ("1", -1)]));
            assert!(texts
                .iter()
                .filter(|t| plain(&t.lines[0]) == "R12")
                .all(|t| runs(&t.lines[0]) == vec![("R", 0), ("12", -1)]));
        } else {
            assert!(texts.iter().all(|t| plain(&t.lines[0]).starts_with(r"\sub")));
        }
    }
}

#[test]
fn a_text_parameter_keeps_the_version_of_its_caller() {
    for (main, included) in [("0.4", "0.3"), ("0.3", "0.4")] {
        let source = file(main, r#"@USE-PACKAGE(name="d", path="d.rnl"); $D @ATTACH(caption="\subrest{VDD}") (0,0)"#);
        let definition = format!(
            "@RNL(type=\"header\") {{ @VERSION(\"{included}\"); @PACKAGE(name=\"d\", version=\"1.0\") }}\n\
             @RNL {{ $D = @DEFINE(roles=[\"p\"], parameters=[\"caption\"]) {{ ?p @TEXT(?caption) }} }}\n"
        );
        let r = read(&MemoryProvider::new().with("t.sp", &source).with("d.rnl", &definition));
        assert_eq!(unknowns(&r), if main == "0.4" { 0 } else { 1 });
        let text = &r.model.levels[0].texts[0];
        assert_eq!(
            runs(&text.lines[0]),
            if main == "0.4" { vec![("V", 0), ("DD", -1)] } else { vec![(r"\subrest{VDD}", 0)] }
        );
    }
}

#[test]
fn included_levels_and_scoped_records_read_their_source_file_version() {
    for (main, included) in [("0.4", "0.3"), ("0.3", "0.4")] {
        let source = file(main, r#"(20,20) @TEXT("main")"#)
            .replace("R12 a b 1k", "R12 a b 1k\n.include \"cell.inc\"")
            .replace(".end\n", "* @RNL(scope=\"cell\") { (10,0) @TEXT(\"\\subdigits{R12}\") }\n.end\n");
        let child = format!(
            "* cell\n* @RNL(type=\"header\") {{ @VERSION(\"{included}\") }}\n\
             .subckt cell a b\nR1 a b 1k\n* @RNL {{ (0,0) @TEXT(\"\\subrest{{VDD}}\") }}\n.ends cell\n"
        );
        let r = read(&MemoryProvider::new().with("t.sp", &source).with("cell.inc", &child));
        let level = r.model.levels.iter().find(|l| l.title == "cell").unwrap();
        assert_eq!(level.texts.len(), 2);
        for (command, version, formatted) in [
            (r"\subrest{VDD}", included, vec![("V", 0), ("DD", -1)]),
            (r"\subdigits{R12}", main, vec![("R", 0), ("12", -1)]),
        ] {
            let text = level.texts.iter().find(|t| t.markup == command).unwrap();
            assert_eq!(runs(&text.lines[0]), if version == "0.4" { formatted } else { vec![(command, 0)] });
        }
        assert_eq!(unknowns(&r), 1);
    }
}

#[test]
fn pin_label_version_reaches_json_and_only_extends_new_digests() {
    for version in ["0.2", "0.3", "0.4"] {
        let source = file(
            version,
            r#"
            $S = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1, direction="left", label="\subrest{VDD}") };
            #R12 @SET-SYMBOL $S => @PLACE(0,0)
        "#,
        );
        let r = read(&MemoryProvider::new().with("t.sp", &source));
        let dump = rnl_cli::dump::level(&r.model.levels[0]);
        let problem = rnl_cli::problem::level(&r.problems[0], "");
        for json in [&dump, &problem] {
            let pin = &json["symbols"][0]["pins"][0];
            assert_eq!(pin["label"], r"\subrest{VDD}");
            assert_eq!(pin.get("label_name_formats"), (version == "0.4").then_some(&serde_json::Value::Bool(true)));
        }
        let canonical = rnl_runtime::problem::canonical(&r.problems[0]);
        assert_eq!(canonical.contains("label-name-formats"), version == "0.4");
    }
}
