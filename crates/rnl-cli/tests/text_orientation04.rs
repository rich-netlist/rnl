//! Text boxes turn with their anchors while their glyphs remain readable (9.2.5).
use rnl_runtime::{diag::Severity, geom::Orient, source::MemoryProvider, Options};
use std::path::Path;

fn drawing(
    version: &str,
    orientation: Orient,
    rotate: u16,
    fixed: bool,
    keyed: bool,
) -> (rnl_runtime::Interpretation, rnl_render::Primitive) {
    let attach = if keyed { "?" } else { "(0,0)" };
    let key = if keyed { ",key=\"label\"" } else { "" };
    let place = if keyed { " => @PLACE(0,0)" } else { "" };
    let fixed = if fixed { ",fix_rotate=true" } else { "" };
    let source = format!("* text\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\n* @RNL {{ $S = @DEFINE-SYMBOL {{ {attach} @TEXT(\"ABC\",rotate={rotate}{key}{fixed}){place} }}; #R1 @SET-SYMBOL $S => @PLACE(0,0) => @ORIENT(\"{}\") }}\n", orientation.as_str());
    let result = rnl_cli::interpret_file(
        Path::new("text.sp"),
        None,
        &MemoryProvider::new().with("text.sp", &source),
        &Options { std: false, problems: true, ..Default::default() },
    )
    .unwrap();
    assert!(!result.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", result.diagnostics());
    let primitive = rnl_render::primitives(&result.model.levels[0], &Default::default())
        .into_iter()
        .find(|p| matches!(p.kind, rnl_render::Kind::Text(_)))
        .unwrap();
    (result, primitive)
}

fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-8, "{a} != {b}");
}

#[test]
fn baseline_boxes_reflect_for_all_symbol_orientations_and_text_rotations() {
    for keyed in [false, true] {
        let base = drawing("0.4", Orient::R0, 0, false, keyed).1.bbox;
        for (rotate, rotation) in [(0, Orient::R0), (90, Orient::R90), (180, Orient::R180), (270, Orient::R270)] {
            for orientation in Orient::ALL {
                let (result, primitive) = drawing("0.4", orientation, rotate, false, keyed);
                let transform = rotation.then(orientation);
                let a = transform.apply((base[0], base[1]));
                let b = transform.apply((base[2], base[3]));
                let want = [a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1)];
                for (a, b) in primitive.bbox.into_iter().zip(want) {
                    close(a, b);
                }
                let rnl_render::Kind::Text(text) = primitive.kind else { unreachable!() };
                assert_eq!(text.runs.iter().map(|r| r.text.as_str()).collect::<String>(), "ABC");
                if keyed {
                    assert!(result.problems[0]
                        .objects
                        .iter()
                        .filter_map(|o| o.text.as_ref())
                        .all(|t| t.baseline_mirrors));
                }
            }
        }
    }
}

#[test]
fn fixed_rotation_preserves_its_own_box_when_the_owner_has_no_turn() {
    for keyed in [false, true] {
        for rotate in [0, 90, 180, 270] {
            let normal = drawing("0.4", Orient::R0, rotate, false, keyed).1;
            let fixed = drawing("0.4", Orient::R0, rotate, true, keyed).1;
            assert_eq!(fixed, normal);
        }
    }
}

#[test]
fn released_baselines_keep_their_original_side() {
    for keyed in [false, true] {
        let a = drawing("0.3", Orient::R0, 0, false, keyed).1.bbox;
        let (result, b) = drawing("0.3", Orient::MX, 0, false, keyed);
        close(a[1], b.bbox[1]);
        close(a[3], b.bbox[3]);
        assert!(result.problems[0].objects.iter().filter_map(|o| o.text.as_ref()).all(|t| !t.baseline_mirrors));
    }
}

#[test]
fn reflected_multiline_formulas_keep_their_reading_order_and_bars() {
    for markup in [r"ABC\\DEF", r"\frac{ABC}{DEF}", r"\sqrt{ABC}"] {
        for vertical in [false, true] {
            let mut original = rnl_render::TextPrim {
                anchor: (3.0, 7.0),
                lines: rnl_runtime::markup::parse(markup),
                size: 1.0,
                color: String::new(),
                font: String::new(),
                bold: false,
                italic: false,
                halign: "start".into(),
                valign: "baseline".into(),
                vertical,
                transform: None,
                runs: Vec::new(),
                bars: Vec::new(),
            };
            let mut reflected = original.clone();
            let before = rnl_render::lay_out(&mut original, 10.0);
            let after = rnl_render::lay_out_reflected_baseline(&mut reflected, 10.0);
            let axis = if vertical { 0 } else { 1 };
            let anchor = if vertical { 3.0 } else { 7.0 };
            close(after[axis], 2.0 * anchor - before[axis + 2]);
            close(after[axis + 2], 2.0 * anchor - before[axis]);
            assert_eq!(original.anchor, reflected.anchor);
            assert_eq!(
                original.runs.iter().map(|r| &r.text).collect::<Vec<_>>(),
                reflected.runs.iter().map(|r| &r.text).collect::<Vec<_>>()
            );
            assert_eq!(original.bars.len(), reflected.bars.len());
            assert!(reflected.runs.iter().all(|r| r.size > 0.0));
        }
    }
}

fn columns(version: &str, orientation: Orient, body: &str, scene: &str) -> rnl_runtime::Interpretation {
    let body = body.replace('\n', " ");
    let source = format!("* columns\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\n* @RNL {{ $S = @DEFINE-SYMBOL {{ {body} }}; #R1 @SET-SYMBOL $S => @PLACE(10,20) => @ORIENT(\"{}\"); {scene} }}\n", orientation.as_str());
    let result = rnl_cli::interpret_file(
        Path::new("column.sp"),
        None,
        &MemoryProvider::new().with("column.sp", &source),
        &Options { std: false, problems: true, ..Default::default() },
    )
    .unwrap();
    assert!(!result.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", result.diagnostics());
    result
}

fn anchor(result: &rnl_runtime::Interpretation, key: &str) -> (f64, f64) {
    result.model.levels[0].texts.iter().find(|t| t.key.as_deref() == Some(key)).unwrap().anchor
}

const COLUMN: &str = r#"? @TEXT("TOP",key="a") => @PLACE(2,3); ? @TEXT("BOTTOM",key="b") => @PLACE(2,-2)"#;

#[test]
fn columns_keep_label_identities_and_reverse_positions_in_four_orientations() {
    for version in ["0.3", "0.4"] {
        for rotate in [0, 90, 180, 270] {
            let body = COLUMN.replace("key=", &format!("rotate={rotate},key="));
            for turn in Orient::ALL {
                let result = columns(version, turn, &body, "");
                let reverse =
                    version == "0.4" && matches!(turn, Orient::MX | Orient::R180 | Orient::R270 | Orient::MXR90);
                for (key, y) in [("a", if reverse { -2.0 } else { 3.0 }), ("b", if reverse { 3.0 } else { -2.0 })] {
                    let offset = turn.apply((2.0, y));
                    assert_eq!(anchor(&result, key), (10.0 + offset.0, 20.0 + offset.1));
                }
                let a = result.model.levels[0].texts.iter().find(|t| t.key.as_deref() == Some("a")).unwrap();
                assert_eq!(rnl_runtime::markup::plain(&a.lines[0]), "TOP");
                let problem = &result.problems[0];
                let a = problem
                    .objects
                    .iter()
                    .find(|o| o.text.as_ref().is_some_and(|t| t.key.as_deref() == Some("a")))
                    .unwrap();
                assert_eq!(a.text.as_ref().unwrap().column_position, (version == "0.4").then_some((2.0, -2.0)));
            }
        }
    }
}

#[test]
fn columns_exclude_empty_content_other_alignments_and_scene_positions() {
    let extras = r#";
        ? @TEXT("",key="empty") => @PLACE(2,10);
        ? @TEXT("MIDDLE",key="middle",valign="middle") => @PLACE(2,8);
        ? @TEXT("END",key="end",halign="end") => @PLACE(2,7);
        ? @TEXT("ROTATED",key="rot",rotate=90) => @PLACE(2,6);
        ? @TEXT("FIXED",key="fixed",fix_rotate=true) => @PLACE(2,11);
        ? @TEXT("OTHER",key="other") => @PLACE(3,5);
        ? @TEXT("SCENE",key="scene") => @PLACE(2,4);
        ? @TEXT("OFFSET",key="offset") => @PLACE(2,9)
    "#;
    let result = columns(
        "0.4",
        Orient::MX,
        &format!("{COLUMN}{extras}"),
        r#"#R1::scene @PLACE(x=90); #R1 @TEXT("OFFSET",key="offset",dx=4)"#,
    );
    assert_eq!(anchor(&result, "a"), (12.0, 22.0));
    assert_eq!(anchor(&result, "b"), (12.0, 17.0));
    for o in &result.problems[0].objects {
        if let Some(t) = &o.text {
            if !matches!(t.key.as_deref(), Some("a" | "b")) {
                assert_eq!(t.column_position, None, "{:?}", t.key);
            }
        }
    }
    let result = columns("0.4", Orient::MX, COLUMN, r#"#R1 @TEXT("",key="b")"#);
    assert_eq!(anchor(&result, "a"), (12.0, 17.0), "expanded empty labels do not occupy a column slot");
}

#[test]
fn disabled_scene_position_keeps_a_label_in_its_column() {
    let result = columns("0.4", Orient::MX, COLUMN, r#"#R1::a @PLACE(90,90,priority="disabled")"#);
    assert_eq!(anchor(&result, "a"), (12.0, 22.0));
}

#[test]
fn solver_links_and_writeback_use_the_same_reversed_column() {
    let result = columns("0.4", Orient::MX, COLUMN, r#"{#R1::a (12,22)} @ALIGN("y"); {#R1::b (12,17)} @ALIGN("y")"#);
    let problem = &result.problems[0];
    let solved = rnl_solver::solve(problem, &Default::default());
    assert!(solved.report.failed.is_empty(), "{:?}", solved.report);
    assert!(solved.report.discarded.is_empty(), "{:?}", solved.report);
    let record = rnl_runtime::solution::write_record(problem, &solved.solution, "test", "test").record;
    assert!(!record.contains("::a"), "{record}");
    assert!(!record.contains("::b"), "{record}");
}

#[test]
fn unplaced_owner_is_solved_from_reversed_labels_without_freezing_them() {
    let source = format!("* columns\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\nR1 a b 1k\n* @RNL {{ $S = @DEFINE-SYMBOL {{ {COLUMN}; ? @TEXT(\"MID\",key=\"mid\") => @PLACE(2,1) }}; #R1 @SET-SYMBOL $S => @ORIENT(\"MX\"); {{#R1::a (12,22)}} @ALIGN(\"x\"); {{#R1::a (12,22)}} @ALIGN(\"y\"); {{#R1::b (12,17)}} @ALIGN(\"y\") }}\n");
    let read = |source: &str| {
        rnl_cli::interpret_file(
            Path::new("free.sp"),
            None,
            &MemoryProvider::new().with("free.sp", source),
            &Options { std: false, problems: true, ..Default::default() },
        )
        .unwrap()
    };
    let result = read(&source);
    assert!(!result.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", result.diagnostics());
    let p = &result.problems[0];
    let solved = rnl_solver::solve(p, &Default::default());
    assert!(solved.report.discarded.is_empty(), "{:?}", solved.report);
    assert!(solved.report.failed.is_empty(), "{:?}", solved.report);
    let written = rnl_runtime::solution::write_record(p, &solved.solution, "test", "test");
    assert!(!written.record.contains("::a"), "{:?}", written);
    assert!(!written.record.contains("::b"), "{:?}", written);
    assert!(!written.record.contains("::mid"), "{:?}", written);
    let body = written.record.lines().map(|s| format!("* {s}\n")).collect::<String>();
    let reread = read(&format!("{source}{body}"));
    assert_eq!(anchor(&reread, "a"), (12.0, 22.0));
    assert_eq!(anchor(&reread, "mid"), (12.0, 19.0));
    assert_eq!(anchor(&reread, "b"), (12.0, 17.0));
}

#[test]
fn fixed_columns_reorder_even_when_their_glyphs_stay_horizontal() {
    let body = COLUMN.replace("key=", "fix_rotate=true,key=");
    for turn in Orient::ALL {
        let result = columns("0.4", turn, &body, "");
        let reverse = matches!(turn, Orient::MX | Orient::R180 | Orient::R270 | Orient::MXR90);
        for (key, y) in [("a", if reverse { -2.0 } else { 3.0 }), ("b", if reverse { 3.0 } else { -2.0 })] {
            let offset = turn.apply((2.0, y));
            assert_eq!(anchor(&result, key), (10.0 + offset.0, 20.0 + offset.1));
        }
        assert!(rnl_render::primitives(&result.model.levels[0], &Default::default())
            .iter()
            .all(|p| { !matches!(&p.kind, rnl_render::Kind::Text(t) if t.vertical) }));
    }
}
