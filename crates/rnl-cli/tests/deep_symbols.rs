//! Deep accepted symbols keep identical geometry, text paths and inferred bounds.

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_render::{primitives, Kind, RenderOptions};
use rnl_runtime::diag::Severity;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{markup, Interpretation, Options};

fn read(body: &str) -> Interpretation {
    let text = format!(
        "* deep symbols\n* @RNL(type=\"header\") {{ @VERSION(\"0.3\") }}\nR1 a b 1k\nR2 c d 1k\n* @RNL {{\n{}\n* }}\n.end\n",
        body.lines().map(|line| format!("* {line}")).collect::<Vec<_>>().join("\n")
    );
    let files = MemoryProvider::new().with("deep.sp", &text);
    interpret_file(Path::new("deep.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap()
}

fn chain(wrappers: usize) -> String {
    let mut body = String::from(
        "$MARK = @DEFINE(roles=[\"at\"]) {};\n$S0 = @DEFINE-SYMBOL { @RECTANGLE {(2,3) (6,5)}; (3,4) @TEXT(\"deep ${name}\") };\n",
    );
    for n in 1..=wrappers {
        body.push_str(&format!(
            "$S{n} = @DEFINE-SYMBOL {{ $MARK @ATTACH (1,2) => @SET-SYMBOL $S{} => @ORIENT(\"R90\") }};\n",
            n - 1
        ));
    }
    body
}

#[test]
fn deep_transformed_graphics_and_fixed_text_survive_with_computed_or_explicit_boxes() {
    let mut body = chain(13);
    body.push_str(
        "$EXPLICIT = @DEFINE-SYMBOL(box=[-10,-11,10,11]) { $S13 };\n#R1 @SET-SYMBOL $S13 => @PLACE(20,30);\n#R2 @SET-SYMBOL $EXPLICIT => @PLACE(50,60)",
    );
    let r = read(&body);
    let errors: Vec<_> = r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).collect();
    assert!(errors.is_empty(), "{:?}", errors.iter().map(|d| r.sink.render(d)).collect::<Vec<_>>());
    let lv = &r.model.levels[0];
    let computed = lv.items.iter().find(|i| i.id == "#R1").unwrap();
    let explicit = lv.items.iter().find(|i| i.id == "#R2").unwrap();
    // Four quarter-turn wrappers cancel both rotation and translation; thirteen leave one.
    assert_eq!(lv.symbols[computed.symbol].bbox, [-4.0, 4.0, -2.0, 8.0]);
    assert!(!lv.symbols[computed.symbol].box_written);
    assert_eq!(lv.symbols[explicit.symbol].bbox, [-10.0, -11.0, 10.0, 11.0]);
    assert!(lv.symbols[explicit.symbol].box_written);
    assert_eq!(computed.texts.len(), 1);
    assert_eq!(explicit.texts.len(), 1);
    let prims = primitives(lv, &RenderOptions::default());
    for (name, dx, dy) in [("#R1", 20.0, 30.0), ("#R2", 50.0, 60.0)] {
        let rects: Vec<_> = prims
            .iter()
            .filter(|p| p.owner == name)
            .filter_map(|p| match &p.kind {
                Kind::Rect { min, max, .. } => Some((*min, *max)),
                _ => None,
            })
            .collect();
        assert_eq!(rects, [((dx - 4.0, dy + 4.0), (dx - 2.0, dy + 8.0))]);
        let texts: Vec<_> = prims
            .iter()
            .filter(|p| p.owner == name)
            .filter_map(|p| match &p.kind {
                Kind::Text(t) => Some(t),
                _ => None,
            })
            .collect();
        assert_eq!(texts.len(), 1);
        assert_eq!(texts[0].anchor, (dx - 3.0, dy + 5.0));
        assert_eq!(markup::plain(&texts[0].lines[0]), format!("deep {}", &name[1..]));
    }
    let symbols = &r.problems[0].symbols;
    assert_eq!(symbols.iter().find(|s| s.name == "$S13").unwrap().bbox, [-4.0, 4.0, -2.0, 8.0]);
    assert_eq!(symbols.iter().find(|s| s.name == "$EXPLICIT").unwrap().bbox, [-10.0, -11.0, 10.0, 11.0]);
}

#[test]
fn the_nesting_limit_fails_explicitly_and_keeps_independent_content() {
    let mut body = chain(64);
    body.push_str(
        "#R1 @SET-SYMBOL $S63 => @PLACE(0,0);\n#R2 @SET-SYMBOL $S64 => @PLACE(20,0);\n(100,100) @TEXT(\"independent\")",
    );
    let r = read(&body);
    let errors: Vec<_> =
        r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).map(|d| r.sink.render(d)).collect();
    assert!(errors.iter().any(|e| e.contains("limit of 64 symbol levels")), "{errors:?}");
    let lv = &r.model.levels[0];
    assert!(lv.texts.iter().any(|t| t.markup == "independent"));
    let accepted = lv.items.iter().find(|i| i.id == "#R1").unwrap();
    assert_eq!(accepted.texts.len(), 1);
    assert_eq!(lv.symbols[accepted.symbol].bbox, [1.0, -5.0, 3.0, -1.0]);
    let drawn = primitives(lv, &RenderOptions::default());
    assert!(drawn
        .iter()
        .any(|p| p.owner == "#R1" && matches!(&p.kind, Kind::Text(t) if markup::plain(&t.lines[0]) == "deep R1")));
    assert!(!lv.symbols.iter().any(|s| s.name == "$S64"));
}

#[test]
fn direct_forward_expansion_obeys_the_limit_and_restores_independent_definitions() {
    for levels in [64, 65, 256, 1000] {
        // Use the outermost name first and declare all wrappers before their dependencies:
        // neither classify() nor an earlier use can prewarm the nested definitions.
        let definitions = chain(levels - 1);
        let mut body = format!("#R1 @SET-SYMBOL $S{} => @PLACE(0,0);\n", levels - 1);
        body.push_str(&definitions.lines().rev().collect::<Vec<_>>().join("\n"));
        body.push_str(
            "\n$SAFE = @DEFINE-SYMBOL { @RECTANGLE {(0,0) (2,2)}; (1,1) @TEXT(\"safe\") };\n#R2 @SET-SYMBOL $SAFE => @PLACE(20,0);\n(100,100) @TEXT(\"independent\")",
        );
        let r = read(&body);
        let errors: Vec<_> =
            r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).map(|d| r.sink.render(d)).collect();
        let lv = &r.model.levels[0];
        let outer = format!("$S{}", levels - 1);
        if levels == 64 {
            assert!(errors.is_empty(), "{levels}: {errors:?}");
            let accepted = lv.items.iter().find(|i| i.id == "#R1").unwrap();
            assert_eq!(lv.symbols[accepted.symbol].name, outer);
            assert_eq!(accepted.texts.len(), 1);
            assert_eq!(lv.symbols[accepted.symbol].bbox, [1.0, -5.0, 3.0, -1.0]);
        } else {
            assert!(errors.iter().any(|e| e.contains("limit of 64 active symbol bodies")), "{levels}: {errors:?}");
            assert!(!lv.symbols.iter().any(|s| s.name == outer), "a truncated {levels}-level symbol escaped");
        }
        assert!(lv.texts.iter().any(|t| t.markup == "independent"));
        let safe = lv.items.iter().find(|i| i.id == "#R2").unwrap();
        assert_eq!(lv.symbols[safe.symbol].name, "$SAFE");
        assert_eq!(lv.symbols[safe.symbol].bbox, [0.0, 0.0, 2.0, 2.0]);
        let drawn = primitives(lv, &RenderOptions::default());
        assert!(drawn
            .iter()
            .any(|p| p.owner == "#R2" && matches!(&p.kind, Kind::Text(t) if markup::plain(&t.lines[0]) == "safe")));
    }
}
