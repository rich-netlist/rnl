//! Formula markup is interpreted by its source language, including symbol and pin labels (13.6).

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_render::{primitives, Kind, RenderOptions};
use rnl_runtime::diag::{codes, Severity};
use rnl_runtime::markup::{plain, Run};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn file(version: &str, body: &str) -> String {
    format!("* formula\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\n* @RNL {{ {body} }}\n.end\n")
}

fn read(files: &MemoryProvider) -> Interpretation {
    let r = interpret_file(Path::new("t.sp"), None, files, &Options { std: false, ..Options::solver() }).unwrap();
    assert!(r.diagnostics().iter().all(|d| d.severity != Severity::Error), "{:?}", r.diagnostics());
    r
}

#[test]
fn formulas_and_upright_overrides_are_in_model_and_rendering() {
    let source = file("0.4", r#"(0,0) @TEXT("\(g_m=2\mathrm{V}\)", italic=true)"#);
    let r = read(&MemoryProvider::new().with("t.sp", &source));
    let text = &r.model.levels[0].texts[0];
    assert_eq!(plain(&text.lines[0]), "gm=2V");
    let drawing = primitives(&r.model.levels[0], &RenderOptions::default());
    let t = drawing
        .iter()
        .find_map(|p| match &p.kind {
            Kind::Text(t) => Some(t),
            _ => None,
        })
        .unwrap();
    assert!(t.runs.iter().filter(|run| run.text.contains('g') || run.text == "m").all(|run| run.italic));
    assert!(t.runs.iter().filter(|run| run.text.contains('2') || run.text == "V").all(|run| !run.italic));
    assert!(!t.runs.iter().any(|run| run.text.contains('(')));
}

#[test]
fn stacked_formulas_contribute_bars_and_bounds_to_solver_texts() {
    let source = file("0.4", r#"(0,0) @TEXT("\(\frac{\sqrt{x^2+1}}{\sqrt{y_1}}\)")"#);
    let r = read(&MemoryProvider::new().with("t.sp", &source));
    let drawing = primitives(&r.model.levels[0], &RenderOptions::default());
    let (primitive, t) = drawing
        .iter()
        .find_map(|p| match &p.kind {
            Kind::Text(t) => Some((p, t)),
            _ => None,
        })
        .unwrap();
    assert_eq!(t.bars.len(), 9);
    assert_eq!(t.bars.iter().filter(|bar| bar.a.1 != bar.b.1).count(), 6);
    assert!(t.runs.iter().all(|r| r.at.0.is_finite() && r.at.1.is_finite() && r.size > 0.0));
    assert!(primitive.bbox[2] > primitive.bbox[0]);
    assert!(primitive.bbox[3] > primitive.bbox[1]);
    assert!(matches!(&r.model.levels[0].texts[0].lines[0][0], Run::Frac { .. }));
    assert!(!r.problems[0].objects.is_empty());
}

#[test]
fn attribute_values_inside_formulas_are_literal_and_upright() {
    let source = file(
        "0.4",
        r#"$S = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1) }; #R1 @SET-SYMBOL $S => @PLACE(0,0); #R1 @TEXT("\(x+${name}+${value}\)", italic=true)"#,
    );
    let r = read(&MemoryProvider::new().with("t.sp", &source));
    let drawing = primitives(&r.model.levels[0], &RenderOptions::default());
    let lines = drawing
        .iter()
        .find_map(|p| match &p.kind {
            Kind::Text(t) if plain(&t.lines[0]).contains("R1") => Some(&t.lines),
            _ => None,
        })
        .unwrap();
    assert_eq!(plain(&lines[0]), "x+R1+1k");
    assert!(lines[0]
        .iter()
        .any(|run| matches!(run, Run::Text { text, style } if text.contains("R1") && style.upright)));
}

#[test]
fn formulas_in_pin_labels_and_symbol_text_are_read() {
    let source = file(
        "0.4",
        r#"$S = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1, direction="left", label="\(V_1\)"); (2,0) @TEXT("\(I_2\)") }; #R1 @SET-SYMBOL $S => @PLACE(0,0)"#,
    );
    let r = read(&MemoryProvider::new().with("t.sp", &source));
    let drawing = primitives(&r.model.levels[0], &RenderOptions::default());
    let labels: Vec<_> = drawing
        .iter()
        .filter_map(|p| match &p.kind {
            Kind::Text(t) => Some(t.runs.iter().map(|run| run.text.as_str()).collect::<String>()),
            _ => None,
        })
        .collect();
    for label in ["V1", "I2"] {
        assert!(labels.iter().any(|text| text == label), "{labels:?}");
    }
}

#[test]
fn unsupported_commands_warn_without_failing_the_text() {
    let source = file("0.4", r#"(0,0) @TEXT("\(\textbf{x}+\unknown{y}\)"); (2,0) @TEXT("after")"#);
    let r = read(&MemoryProvider::new().with("t.sp", &source));
    assert_eq!(r.sink.all().iter().filter(|d| d.code == codes::MARKUP_UNKNOWN_COMMAND).count(), 2);
    assert_eq!(plain(&r.model.levels[0].texts[0].lines[0]), r"\textbf{x}+\unknown{y}");
    assert_eq!(plain(&r.model.levels[0].texts[1].lines[0]), "after");
}

#[test]
fn scoped_text_in_an_included_subcircuit_is_read() {
    let source = file("0.4", "").replace("R1 a b 1k", "R1 a b 1k\n.include child.sp").replace(
        ".end",
        r#"* @RNL(scope="cell") { (4,0) @TEXT("\(M_1\)") }
.end"#,
    );
    let included = "* child\n.subckt cell a b\nR1 a b 1k\n* @RNL { (0,0) @TEXT(\"\\(C_2\\)\") }\n.ends cell\n";
    let r = read(&MemoryProvider::new().with("t.sp", &source).with("child.sp", included));
    let level = r.model.levels.iter().find(|level| level.title == "cell").unwrap();
    let labels: Vec<_> = level.texts.iter().map(|text| plain(&text.lines[0])).collect();
    assert!(labels.contains(&"M1".into()), "{labels:?}");
    assert!(labels.contains(&"C2".into()), "{labels:?}");
}

#[test]
fn formula_structure_and_upright_style_are_in_the_solver_digest() {
    let mut values = Vec::new();
    for content in ["1", r"\(1\)", r"\(\sqrt{1}\)"] {
        let source = file("0.4", &format!("(0,0) @TEXT(\"{content}\", italic=true)"));
        let r = read(&MemoryProvider::new().with("t.sp", &source));
        let canonical = rnl_runtime::problem::canonical(&r.problems[0]);
        assert_eq!(canonical.contains("upright"), content != "1");
        assert_eq!(canonical.contains("sqrt"), content.contains("sqrt"));
        values.push(rnl_runtime::problem::input_digest(&r.problems[0]));
    }
    assert!(values[0] != values[1] && values[1] != values[2] && values[0] != values[2]);
}
