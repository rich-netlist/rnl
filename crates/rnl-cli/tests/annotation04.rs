//! 0.4 ANNOTATION (§13.5): public interpretation and drawing regressions.
//! Private regressions for the 0.4 implementation and its public outputs.
use rnl_cli::interpret_file;
use rnl_render::{primitives, Kind, RenderOptions};
use rnl_runtime::diag::Severity;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};
use std::path::Path;

fn source(body: &str) -> String {
    format!(
        "* annotation\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\nR1 a b 1k\n* @RNL {{\n{}\n* }}\n.end\n",
        body.lines().map(|s| format!("* {s}")).collect::<Vec<_>>().join("\n")
    )
}
fn read(body: &str) -> Interpretation {
    let text = source(body);
    let files = MemoryProvider::new().with("annotation.sp", &text);
    interpret_file(Path::new("annotation.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap()
}
fn errors(r: &Interpretation) -> Vec<String> {
    r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).map(|d| r.sink.render(d)).collect()
}
fn require_clean(r: &Interpretation) {
    assert!(errors(r).is_empty(), "{:?}", errors(r));
}
fn rects(r: &Interpretation) -> Vec<[f64; 4]> {
    primitives(&r.model.levels[0], &RenderOptions::default())
        .iter()
        .filter_map(|p| match &p.kind {
            Kind::Rect { fill: Some(c), .. } | Kind::Path { fill: Some(c), .. } if c == "#ff0000" => {
                Some(p.bbox.map(|v| (v * 1e9).round() / 1e9))
            }
            _ => None,
        })
        .collect()
}
fn text(r: &Interpretation) -> Vec<String> {
    primitives(&r.model.levels[0], &RenderOptions::default())
        .iter()
        .filter_map(|p| match &p.kind {
            Kind::Text(t) => Some(t.runs.iter().map(|r| r.text.as_str()).collect()),
            _ => None,
        })
        .collect()
}
const SYM: &str = r##"$S = @DEFINE-SYMBOL { @RECTANGLE(fill="fillcolor",fillcolor="#ff0000") {(0,0) (2,1)} }"##;

#[test]
fn symbol_source_draws_one_annotation_and_ignores_symbol_pin() {
    let r = read(&format!(r#"{SYM}; $P = @DEFINE-SYMBOL {{ $S; (0,0) @PIN(terminal=1) }}; %a = (4,5) @ANNOTATION $P"#));
    require_clean(&r);
    assert_eq!(rects(&r), vec![[4.0, 5.0, 6.0, 6.0]]);
    assert!(r.model.levels[0].wires.is_empty());
    assert!(r.model.levels[0].items.iter().all(|i| i.id != "%a"), "an annotation is not an electrical item");
}

#[test]
fn definition_source_does_not_apply_definition_or_make_usage() {
    let r = read(
        r##"$D = @DEFINE(roles=["x"]) { @RECTANGLE(fill="fillcolor",fillcolor="#ff0000") {(0,0) (2,1)} }; %a = (4,5) @ANNOTATION $D"##,
    );
    require_clean(&r);
    assert_eq!(rects(&r), vec![[4.0, 5.0, 6.0, 6.0]]);
    assert!(
        r.model.levels[0].objects.iter().all(|o| o.definition.as_deref() != Some("$D")),
        "drawing a definition must not apply it"
    );
}

#[test]
fn usage_source_adds_symbol_and_direct_body_graphic_without_moving_original() {
    let r = read(&format!(
        r##"{SYM}; $D = @DEFINE(roles=["x"]) {{ @RECTANGLE(fill="fillcolor",fillcolor="#ff0000") {{(3,0) (4,1)}} }}; #R1(x) @AS $D => %u => @SET-SYMBOL $S => @PLACE(10,0); (20,0) @ANNOTATION %u"##
    ));
    require_clean(&r);
    let boxes = rects(&r);
    for expected in [[10., 0., 12., 1.], [13., 0., 14., 1.], [20., 0., 22., 1.], [23., 0., 24., 1.]] {
        assert!(boxes.contains(&expected), "{boxes:?}");
    }
    assert_eq!(boxes.len(), 4, "each source graphic appears once at each location");
}

#[test]
fn object_dependent_graphic_is_skipped_but_literal_graphic_remains() {
    let r = read(
        r##"$D = @DEFINE(roles=[["x","point"]]) { @RECTANGLE(fill="fillcolor",fillcolor="#ff0000") {(0,0) (2,1)}; ?x (1,0) @LINE }; (4,5) @ANNOTATION $D"##,
    );
    require_clean(&r);
    assert_eq!(rects(&r), vec![[4., 5., 6., 6.]]);
    assert!(
        r.diagnostics().iter().any(|d| d.severity == Severity::Hint),
        "skipped object-dependent graphic needs hint"
    );
}

#[test]
fn symbol_owner_text_sees_only_annotation_xy() {
    let r = read(
        r#"$S = @DEFINE-SYMBOL { (0,0) (1,0) @LINE; ? @TEXT("xy=${x},${y};name=${name};model=${model}") => @PLACE(0,1) }; (4,5) @ANNOTATION $S"#,
    );
    require_clean(&r);
    let labels = text(&r);
    assert!(labels.iter().any(|s| s == "xy=4,5;name=;model="), "{labels:?}");
}

#[test]
fn literal_anchor_moves_by_position_but_object_anchor_follows_object() {
    let r = read(&format!(
        r#"{SYM}; #R1 @PLACE(10,20); %a = (1,2) @ANNOTATION $S; %a::position @PLACE(4,5); #R1 @ANNOTATION $S"#
    ));
    require_clean(&r);
    let boxes = rects(&r);
    assert!(boxes.contains(&[4., 5., 6., 6.]), "{boxes:?}");
    assert!(boxes.contains(&[10., 20., 12., 21.]), "{boxes:?}");
}

#[test]
fn wrong_source_errors_and_empty_source_warns_without_ink() {
    let r =
        read(r#"$EMPTY = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1) }; (1,2) @ANNOTATION #R1; (3,4) @ANNOTATION $EMPTY"#);
    assert!(!errors(&r).is_empty(), "wrong source must error");
    assert!(r.diagnostics().iter().any(|d| d.severity == Severity::Warning), "no drawable graphics must warn");
    assert!(primitives(&r.model.levels[0], &RenderOptions::default()).is_empty());
}

#[test]
fn selection_sees_compound_annotation_once_and_can_style_it() {
    let r = read(
        r##"$S = @DEFINE-SYMBOL { @RECTANGLE(fill="fillcolor",fillcolor="#ff0000") {(0,0) (2,1)}; @RECTANGLE(fill="fillcolor",fillcolor="#ff0000") {(3,0) (4,1)} }; %a = (4,5) @ANNOTATION $S; # @SELECT-ANNOTATION => @LAYER("top")"##,
    );
    require_clean(&r);
    let drawing = primitives(&r.model.levels[0], &RenderOptions::default());
    let reds: Vec<_> = drawing.iter().filter(|p| matches!(&p.kind, Kind::Rect { fill: Some(c), .. } | Kind::Path { fill: Some(c), .. } if c == "#ff0000")).collect();
    assert_eq!(reds.len(), 2);
    assert!(reds.iter().all(|p| p.layer == rnl_runtime::model::Layer::Top));
    assert_eq!(reds[0].owner, reds[1].owner, "the two shapes belong to one annotation");
}

#[test]
fn mirror_precedes_ninety_degree_rotation_and_preserves_one_identity() {
    let r = read(&format!(r#"{SYM}; %a = (10,20) @ANNOTATION(rotate=90,mirror=true) $S"#));
    require_clean(&r);
    // Local rectangle x=[0,2], y=[0,1]: after MY then R90 it spans x=[-1,0], y=[-2,0].
    assert_eq!(rects(&r), vec![[9., 18., 10., 20.]]);
}

#[test]
fn arbitrary_angle_rotation_is_not_quantized_to_orthogonal_orientation() {
    let r = read(&format!(r#"{SYM}; (0,0) @ANNOTATION(rotate=30) $S"#));
    require_clean(&r);
    let boxes = rects(&r);
    assert_eq!(boxes.len(), 1);
    let b = boxes[0];
    let root3 = 3.0_f64.sqrt();
    for (actual, expected) in b.into_iter().zip([-0.5, 0.0, root3, 1.0 + root3 / 2.0]) {
        assert!((actual - expected).abs() < 1e-8, "{b:?}");
    }
}

#[test]
fn annotation_is_not_a_keyword_in_released_03() {
    let text = source(&format!(r#"{SYM}; (4,5) @ANNOTATION $S"#)).replace("@VERSION(\"0.4\")", "@VERSION(\"0.3\")");
    let files = MemoryProvider::new().with("annotation.sp", &text);
    let r =
        interpret_file(Path::new("annotation.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap();
    assert!(!errors(&r).is_empty(), "0.3 must not admit this 0.4 keyword");
}

#[test]
fn definition_body_annotation_accepts_graphic_and_text_style() {
    let r = read(&format!(
        r##"{SYM}; $D = @DEFINE(roles=["p"]) {{ ?p @ANNOTATION(linecolor="#ff0000",textvisible=false) $S }}"##
    ));
    require_clean(&r);
}

#[test]
fn definition_body_annotation_rejects_invalid_style_values() {
    let r = read(&format!(r##"{SYM}; $D = @DEFINE(roles=["p"]) {{ ?p @ANNOTATION(textvisible=3) $S }}"##));
    let found = errors(&r);
    assert!(!found.is_empty(), "invalid textvisible must be rejected");
    assert!(found.iter().any(|d| d.contains("textvisible")), "{found:?}");
}

#[test]
fn annotation_following_named_point_tracks_its_final_position() {
    let r = read(&format!(r#"{SYM}; %p = (1,2); %a = %p @ANNOTATION $S; %p @PLACE(7,8)"#));
    require_clean(&r);
    assert_eq!(rects(&r), vec![[7.0, 8.0, 9.0, 9.0]]);
    let a = &r.model.levels[0].annotations[0];
    assert_eq!(a.anchor_kind, "follow");
}

#[test]
fn hidden_annotation_text_is_not_a_pickable_primitive() {
    let r = read(
        r#"$S = @DEFINE-SYMBOL { ? @TEXT("secret"); (0,0) (1,0) @LINE }; %a = (2,3) @ANNOTATION(textvisible=false) $S"#,
    );
    require_clean(&r);
    assert!(!text(&r).iter().any(|t| t == "secret"));
}

#[test]
fn literal_anchor_is_exposed_for_position_editing() {
    let r = read(&format!(r#"{SYM}; %a = (1,2) @ANNOTATION $S; %a::position @PLACE(4,5)"#));
    require_clean(&r);
    let a = &r.model.levels[0].annotations[0];
    assert_eq!(a.object_id, "%a");
    assert_eq!(a.anchor_kind, "independent");
    assert_eq!(a.anchor, (4.0, 5.0));
}

#[test]
fn definition_source_parameters_use_values_defaults_and_literal_holes_without_usage() {
    let r = read(
        r##"$D = @DEFINE(roles=[],parameters=[["w",3], ["x",1]]) { (?x,0) @RECTANGLE(width=?w,height=2,fill="fillcolor",fillcolor="#ff0000") }; (10,0) @ANNOTATION(w=5,x=2) $D; (20,0) @ANNOTATION $D"##,
    );
    require_clean(&r);
    let boxes = rects(&r);
    assert!(boxes.contains(&[9.5, -1.0, 14.5, 1.0]), "{boxes:?}");
    assert!(boxes.contains(&[19.5, -1.0, 22.5, 1.0]), "{boxes:?}");
    assert!(r.model.levels[0].objects.iter().all(|i| i.definition.as_deref() != Some("$D")));
}

#[test]
fn annotation_own_parameter_shadows_source_and_usage_rejects_override() {
    let r = read(
        r##"$D = @DEFINE(roles=[],parameters=[["rotate",3],["w",2]]) { (0,0) @RECTANGLE(width=?w,height=1) }; (1,1) @ANNOTATION(rotate=90) $D; %u = $D @ATTACH(w=4); (3,3) @ANNOTATION(w=5) %u"##,
    );
    let diagnostics = r.diagnostics().into_iter().map(|d| r.sink.render(d)).collect::<Vec<_>>();
    assert!(diagnostics.iter().any(|d| d.contains("RNL4420")), "{diagnostics:?}");
    assert!(diagnostics.iter().any(|d| d.contains("w") && d.contains("error")), "{diagnostics:?}");
}

#[test]
fn piped_literal_graphic_in_definition_is_collected() {
    let r = read(r#"$D = @DEFINE(roles=[]) { {(0,0) (1,1)} => @LINE }; %a = (2,3) @ANNOTATION $D"#);
    require_clean(&r);
    assert_eq!(r.model.levels[0].annotations[0].graphics.len(), 1);
}

#[test]
fn invalid_anchor_reports_once_and_later_annotation_recovers() {
    let r = read(&format!(r#"{SYM}; &a @ANNOTATION $S; %ok = (2,3) @ANNOTATION $S"#));
    let errors = errors(&r);
    assert_eq!(errors.iter().filter(|e| e.contains("RNL4419")).count(), 1, "{errors:?}");
    assert!(r.model.levels[0].annotations.iter().any(|a| a.object_id == "%ok"));
}

#[test]
fn editor_set_position_writes_literal_annotation_anchor() {
    let op = rnl_edit::Op::Set {
        object: "%a::position".into(),
        symbol: None,
        terminals: vec![],
        style: vec![],
        at: Some([6.0, 7.0]),
        orient: None,
    };
    let statement = rnl_edit::form::statements(&op, None).join("; ");
    let r = read(&format!(r#"{SYM}; %a = (1,2) @ANNOTATION $S; {statement}"#));
    require_clean(&r);
    let a = &r.model.levels[0].annotations[0];
    assert_eq!(a.object_id, "%a");
    assert_eq!(a.anchor, (6.0, 7.0));
}

#[test]
fn viewer_snapshot_and_pick_preserve_annotation_identity() {
    let input = source(&format!(r#"{SYM}; %a = (1,2) @ANNOTATION $S"#));
    let files = MemoryProvider::new().with("annotation.sp", &input);
    let mut options = rnl_view::BuildOptions::default();
    options.runtime.std = false;
    let doc = rnl_view::build("annotation.sp", &files, &options).unwrap();
    let hits: Vec<_> = doc.levels[0].hits.iter().filter(|h| h.id.as_deref() == Some("%a")).collect();
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert_eq!(doc.objects[hits[0].objects[0]].kind, "annotation");
    let snapshot = rnl_view::snapshot(
        "annotation.sp",
        &files,
        &options,
        rnl_view::Parts { drawing: false, host: false, model: true },
    )
    .unwrap();
    let snapshot = serde_json::to_value(snapshot).unwrap();
    let entry = &snapshot["levels"][0]["model"]["annotations"][0];
    assert_eq!(entry["id"], "%a");
    assert_eq!(entry["source_kind"], "symbol");
    assert_eq!(entry["anchor_kind"], "independent");
}

#[test]
fn definition_body_checks_known_annotation_source_parameters() {
    let good = read(
        r##"$D = @DEFINE(roles=[],parameters=[["w",3]]) { (0,0) (1,1) @LINE }; $WRAP = @DEFINE(roles=["p"]) { ?p @ANNOTATION(w=5,linecolor="#000000") $D }"##,
    );
    require_clean(&good);
    let bad = read(
        r#"$D = @DEFINE(roles=[],parameters=[["w",3]]) { (0,0) (1,1) @LINE }; $WRAP = @DEFINE(roles=["p"]) { ?p @ANNOTATION(unknown=5) $D }"#,
    );
    let errors = errors(&bad);
    assert!(errors.iter().any(|e| e.contains("unknown")), "{errors:?}");
}

#[test]
fn missing_definition_parameter_makes_coordinate_free_and_omits_graphic() {
    let r = read(r#"$D = @DEFINE(roles=[],parameters=["x"]) { (?x,0) (1,0) @LINE }; (2,3) @ANNOTATION $D"#);
    require_clean(&r);
    assert!(r.model.levels[0].annotations[0].graphics.is_empty());
    let diagnostics = r.diagnostics().into_iter().map(|d| r.sink.render(d)).collect::<Vec<_>>();
    assert!(diagnostics.iter().any(|d| d.contains("RNL4417")), "{diagnostics:?}");
    assert!(diagnostics.iter().any(|d| d.contains("RNL4418")), "{diagnostics:?}");
}

#[test]
fn annotation_coordinate_tokens_respect_writing_unit_aliases() {
    let input = source(r#"$S = @DEFINE-SYMBOL { ? @TEXT("${xx}:${name}") }; (4,5) @ANNOTATION $S"#)
        .replace("@VERSION(\"0.4\")", "@VERSION(\"0.4\"); @ALIAS(\"xx\",\"x\")");
    let files = MemoryProvider::new().with("annotation.sp", &input);
    let r =
        interpret_file(Path::new("annotation.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap();
    require_clean(&r);
    let g = &r.model.levels[0].annotations[0].graphics[0];
    assert_eq!(
        g.text_refs,
        vec![rnl_runtime::model::AnnotationTextRef::X, rnl_runtime::model::AnnotationTextRef::Empty]
    );
}

#[test]
fn literal_annotation_anchor_has_one_numbered_solver_point() {
    use rnl_runtime::problem::{ObjectKind, Reference};
    let r = read(&format!(r#"{SYM}; (_,_) @ANNOTATION $S"#));
    require_clean(&r);
    let points: Vec<_> = r.problems[0].objects.iter().filter(|o| o.kind == ObjectKind::Point).collect();
    assert_eq!(points.len(), 1, "{points:?}");
    assert!(matches!(&points[0].reference, Reference::Number {number:1,path} if path.is_empty()));
    assert_eq!(r.problems[0].authored_numbers, 2);
    assert_eq!(r.numbering[0].authored.len(), 2);
}

#[test]
fn literal_anchor_position_edits_do_not_leave_a_free_point() {
    use rnl_runtime::problem::{ObjectKind, Position, Slot};
    for expression in ["%a = (_,_) @ANNOTATION $S", "(_,_) => @ANNOTATION $S => %a"] {
        let r = read(&format!(r#"{SYM}; {expression}; %a::position @PLACE(4,5)"#));
        require_clean(&r);
        let points: Vec<_> = r.problems[0].objects.iter().filter(|o| o.kind == ObjectKind::Point).collect();
        assert_eq!(points.len(), 1, "{expression}: {points:?}");
        assert!(matches!(
            points[0].position,
            Position::Own { x: Slot::Fixed { value: 4.0, .. }, y: Slot::Fixed { value: 5.0, .. } }
        ));
        assert_eq!(r.model.levels[0].annotations[0].anchor, (4.0, 5.0));
        assert_eq!(r.model.levels[0].annotations[0].anchor_kind, "independent");
    }
}

#[test]
fn previously_bound_piped_point_remains_a_followed_object() {
    use rnl_runtime::problem::ObjectKind;
    let r = read(&format!(r#"{SYM}; (_,_) => %p => @ANNOTATION $S => %a; %p @PLACE(8,9)"#));
    require_clean(&r);
    assert_eq!(r.problems[0].objects.iter().filter(|o| o.kind == ObjectKind::Point).count(), 1);
    assert_eq!(r.model.levels[0].annotations[0].anchor, (8.0, 9.0));
    assert_eq!(r.model.levels[0].annotations[0].anchor_kind, "follow");
}

#[test]
fn dump_and_conformance_expose_annotation_geometry_and_identity() {
    let r = read(
        r##"$S = @DEFINE-SYMBOL { (0,0) @CIRCLE(radius=2); ? @TEXT("at ${x}") }; %a = (4,5) @ANNOTATION(rotate=30,mirror=true,linecolor="#ff0000") $S"##,
    );
    require_clean(&r);
    let dump = rnl_cli::dump::interpretation(&r);
    let conformance = rnl_cli::dump::conformance(&r);
    assert_eq!(dump["levels"][0], conformance["levels"][0]);
    let a = &dump["levels"][0]["annotations"][0];
    assert_eq!(a["id"], "%a");
    assert_eq!(a["source_kind"], "symbol");
    assert_eq!(a["source"], "$S");
    assert_eq!(a["anchor_kind"], "independent");
    assert_eq!(a["anchor"], serde_json::json!([4.0, 5.0]));
    assert_eq!(a["rotate"], 30.0);
    assert_eq!(a["mirror"], true);
    assert_eq!(a["style"]["linecolor"], "#ff0000");
    assert!(a["order"].is_array());
    let graphics = a["graphics"].as_array().unwrap();
    assert!(graphics.iter().any(|g| g["shape"] == "CIRCLE" && g["radius"] == 2.0));
    let text = graphics.iter().find(|g| g["shape"] == "TEXT").unwrap();
    assert_eq!(text["text"], "at ${x}");
    assert_eq!(text["modern_text"], true);
    assert!(text["order"].is_number());
    assert_eq!(text["text_refs"], serde_json::json!(["x"]));
    assert!(text["lines"].to_string().contains("at 4"));
}

#[test]
fn dump_cli_writes_annotation_data_and_legacy_omits_new_key() {
    use std::process::Command;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("annotation.sp");
    std::fs::write(&path, source(&format!(r#"{SYM}; %a = (4,5) @ANNOTATION $S"#))).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_rnl")).arg("dump").arg(&path).arg("--no-std").output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["levels"][0]["annotations"][0]["id"], "%a");
    let legacy = source("(0,0) (1,1) @LINE").replace("@VERSION(\"0.4\")", "@VERSION(\"0.3\")");
    std::fs::write(&path, legacy).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_rnl")).arg("dump").arg(&path).arg("--no-std").output().unwrap();
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(value["levels"][0].get("annotations").is_none());
}

#[test]
fn place_rejects_bare_annotation_without_moving_anchor_or_owner() {
    for (setup, expected) in
        [("%a = (1,2) @ANNOTATION $S", (1.0, 2.0)), ("#R1 @PLACE(3,4); %a = #R1 @ANNOTATION $S", (3.0, 4.0))]
    {
        let r = read(&format!(r#"{SYM}; {setup}; %a @PLACE(8,9)"#));
        let errors = errors(&r);
        assert_eq!(errors.iter().filter(|e| e.contains("PLACE does not take")).count(), 1, "{errors:?}");
        assert_eq!(r.model.levels[0].annotations[0].anchor, expected);
        if setup.starts_with("#R1") {
            let item = r.model.levels[0].items.iter().find(|i| i.object_id == "#R1").unwrap();
            assert_eq!(item.at, expected);
        }
    }
}

#[test]
fn place_on_followed_annotation_position_keeps_existing_follow_semantics() {
    let r = read(&format!(r#"{SYM}; %a = #R1 @ANNOTATION $S; %a::position @PLACE(8,9)"#));
    require_clean(&r);
    assert_eq!(r.model.levels[0].annotations[0].anchor, (8.0, 9.0));
    assert_eq!(r.model.levels[0].annotations[0].anchor_kind, "follow");
    assert_eq!(r.model.levels[0].items.iter().find(|i| i.object_id == "#R1").unwrap().at, (8.0, 9.0));
}

#[test]
fn definition_static_check_rejects_place_on_annotation() {
    let r = read(&format!(r#"{SYM}; $D = @DEFINE(roles=["p"]) {{ %a = ?p @ANNOTATION $S; %a @PLACE(8,9) }}"#));
    let errors = errors(&r);
    assert!(errors.iter().any(|e| e.contains("PLACE") && e.contains("annotation")), "{errors:?}");
}

#[test]
fn definition_static_check_accepts_annotation_position_subobject() {
    let r =
        read(&format!(r#"{SYM}; $D = @DEFINE(roles=["p"]) {{ %a = ?p @ANNOTATION $S; %a::position @PLACE(8,9) }}"#));
    require_clean(&r);
}

fn annotation_draw_order(r: &Interpretation, id: &str) -> Vec<String> {
    primitives(&r.model.levels[0], &RenderOptions::default())
        .iter()
        .filter(|p| p.owner == id)
        .map(|p| match &p.kind {
            Kind::Text(t) => t.runs.iter().map(|r| r.text.as_str()).collect(),
            Kind::Path { fill: Some(_), .. } | Kind::Rect { fill: Some(_), .. } => "RECT".to_string(),
            other => panic!("unexpected annotation primitive: {other:?}"),
        })
        .collect()
}

#[test]
fn annotation_symbol_preserves_interleaved_label_graphic_order_and_layer() {
    let r = read(
        r##"$B = @DEFINE-SYMBOL { ? @TEXT("A"); @RECTANGLE(fill="fillcolor",fillcolor="#ff0000") {(0,0) (2,2)} }; $S = @DEFINE-SYMBOL { $B; ? @TEXT("B") }; %a = (3,4) @ANNOTATION $S => @LAYER("top")"##,
    );
    require_clean(&r);
    assert_eq!(annotation_draw_order(&r, "%a"), vec!["A", "RECT", "B"]);
    assert!(primitives(&r.model.levels[0], &RenderOptions::default())
        .iter()
        .filter(|p| p.owner == "%a")
        .all(|p| p.layer == rnl_runtime::model::Layer::Top));
}

#[test]
fn annotation_definition_preserves_interleaved_label_graphic_order() {
    let r = read(
        r##"$D = @DEFINE(roles=[]) { ? @TEXT("A"); @RECTANGLE(fill="fillcolor",fillcolor="#ff0000") {(0,0) (2,2)}; ? @TEXT("B") }; %a = (3,4) @ANNOTATION $D"##,
    );
    require_clean(&r);
    assert_eq!(annotation_draw_order(&r, "%a"), vec!["A", "RECT", "B"]);
}

#[test]
fn annotation_usage_preserves_symbol_then_body_interleaved_order() {
    let r = read(
        r##"$S = @DEFINE-SYMBOL { ? @TEXT("A"); @RECTANGLE(fill="fillcolor",fillcolor="#ff0000") {(0,0) (2,2)} }; $D = @DEFINE(roles=[]) { ? @TEXT("B"); @RECTANGLE(fill="fillcolor",fillcolor="#ff0000") {(3,0) (4,1)}; ? @TEXT("C") }; %u = $D @ATTACH => @SET-SYMBOL $S => @PLACE(0,0); %a = (10,0) @ANNOTATION %u"##,
    );
    require_clean(&r);
    assert_eq!(annotation_draw_order(&r, "%a"), vec!["A", "RECT", "B", "RECT", "C"]);
}

#[test]
fn annotation_collects_mixed_piped_points_and_pure_point_sets() {
    for body in ["(0,0) => @LINE (1,1)", "{(0,0) (1,1)} => @LINE", "@LINE {(0,0) (1,1)}"] {
        let r = read(&format!(r#"$D = @DEFINE(roles=[]) {{ {body} }}; %a = (2,3) @ANNOTATION $D"#));
        require_clean(&r);
        let a = &r.model.levels[0].annotations[0];
        assert_eq!(a.graphics.len(), 1, "{body}");
        assert_eq!(a.graphics[0].primitive.points, vec![(0.0, 0.0), (1.0, 1.0)], "{body}");
    }
}

#[test]
fn annotation_mixed_pipe_and_written_set_rejects_nested_point_set() {
    for body in ["{(0,0) (1,0)} => @LINE (2,0)", "(0,0) => @LINE {(1,1)}"] {
        let r = read(&format!(r#"$D = @DEFINE(roles=[]) {{ {body} }}; (2,3) @ANNOTATION $D"#));
        let errors = errors(&r);
        assert!(errors.iter().any(|e| e.contains("RNL3007")), "{body}: {errors:?}");
    }
}

#[test]
fn static_place_preserves_graphics_and_suppresses_duplicate_parameter_error() {
    for graphic in ["CIRCLE(radius=1)", "ARC { (0,0) (1,1) (2,0) }"] {
        let body = if graphic.starts_with("CIRCLE") { format!("(0,0) @{graphic}") } else { format!("@{graphic}") };
        let r = read(&format!(r#"$D = @DEFINE(roles=["r"]) {{ {body} => @PLACE(x=?r) }}"#));
        let errors = errors(&r);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].contains("RNL3205"), "{errors:?}");
    }
    let r = read(&format!(r#"{SYM}; $D = @DEFINE(roles=["r"]) {{ ?r @ANNOTATION $S => @PLACE(x=?r) }}"#));
    let errors = errors(&r);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("RNL3205"), "{errors:?}");
}
