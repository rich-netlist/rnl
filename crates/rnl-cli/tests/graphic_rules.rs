//! Graphics generated from the shared selection snapshot (9.2.5, 10.4–10.5).
use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Severity};
use rnl_runtime::interp::obj::Shape;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn read(version: &str, body: &str) -> Interpretation {
    let text = format!("* graphic rules\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\nR2 c d 2k\n* @RNL {{ $S = @DEFINE-SYMBOL {{ (0,0) @PIN(terminal=1); (4,0) @PIN(terminal=2) }}; {{#R1 #R2}} @SET-SYMBOL $S; #R1 @PLACE(0,0); #R2 @PLACE(10,0); {} }}\n.end\n", body.replace('\n', " "));
    interpret_file(
        Path::new("t.sp"),
        None,
        &MemoryProvider::new().with("t.sp", &text),
        &Options { std: false, ..Options::solver() },
    )
    .unwrap()
}

fn ok(r: &Interpretation) {
    let errors: Vec<_> =
        r.diagnostics().iter().filter(|d| d.severity == Severity::Error).map(|d| r.sink.render(d)).collect();
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn sized_shapes_are_made_once_per_pin_and_the_result_accepts_style() {
    for (call, shape) in [
        ("CIRCLE(radius=1)", Shape::Circle),
        ("RECTANGLE(width=2,height=3)", Shape::Rectangle),
        ("ELLIPSE(width=2,height=3)", Shape::Ellipse),
    ] {
        let r=read("0.4",&format!("$Red = @DEFINE-STYLE(linecolor=\"#ff0000\") {{}}; %drawn = #R1 @SELECT-PIN => @{call}; %drawn @SET-STYLE $Red"));
        ok(&r);
        let shapes = &r.model.levels[0].shapes;
        assert_eq!(shapes.len(), 2);
        assert!(shapes
            .iter()
            .all(|s| s.shape == shape && s.style.get("linecolor").and_then(|x| x.str()) == Some("#ff0000")));
        assert_eq!(shapes.iter().map(|s| s.points[0]).collect::<Vec<_>>(), [(0.0, 0.0), (4.0, 0.0)]);
    }
}

#[test]
fn a_line_uses_the_whole_selection_in_scope_order() {
    let r = read("0.4", "{#R2 #R1} @SELECT-PIN => @LINE");
    ok(&r);
    assert_eq!(r.model.levels[0].shapes.len(), 1);
    assert_eq!(r.model.levels[0].shapes[0].points, [(10.0, 0.0), (14.0, 0.0), (0.0, 0.0), (4.0, 0.0)]);
}

#[test]
fn arcs_and_beziers_check_the_selected_point_count() {
    let r = read("0.4", "# @SELECT-PIN => @BEZIER");
    ok(&r);
    assert_eq!(r.model.levels[0].shapes[0].shape, Shape::Bezier);
    let r = read("0.4", "# @SELECT-PIN => @ARC");
    assert!(r.diagnostics().iter().any(|d| d.code == codes::KEYWORD_GRAPHIC_POINT_COUNT));
    assert!(r.model.levels[0].shapes.is_empty());
    let r = read("0.4", "{%one #R2:1} @ARC; %one = #R1 @SELECT-PIN");
    ok(&r);
    assert_eq!(r.model.levels[0].shapes[0].shape, Shape::Arc);
    assert_eq!(r.model.levels[0].shapes[0].points.len(), 3);
}

#[test]
fn two_point_closed_shapes_still_reject_selections() {
    for call in ["CIRCLE", "RECTANGLE", "ELLIPSE"] {
        let r = read("0.4", &format!("#R1 @SELECT-PIN => @{call}"));
        assert!(r.diagnostics().iter().any(|d| d.code == codes::KEYWORD_GRAPHIC_POINTS_SELECTION));
        assert!(r.model.levels[0].shapes.is_empty());
    }
}

#[test]
fn generation_does_not_see_generated_graphics_but_late_styles_do() {
    let r = read(
        "0.4",
        r##"$D = @DEFINE(roles=["x"]) {}; $Red = @DEFINE-STYLE(linecolor="#ff0000") {}; #R1 @SELECT-PIN => @CIRCLE(radius=1); #R1 @SELECT-PIN => @CIRCLE(radius=1); # @SELECT-ANNOTATION => @AS $D; # @SELECT-ANNOTATION => @SET-STYLE $Red"##,
    );
    ok(&r);
    assert_eq!(r.model.levels[0].shapes.len(), 4);
    assert!(r.problems[0].usages.is_empty());
    assert!(r.model.levels[0].shapes.iter().all(|s| s.style.get("linecolor").and_then(|x| x.str()) == Some("#ff0000")));
}

#[test]
fn an_empty_selection_draws_nothing_without_point_count_errors() {
    for call in ["LINE", "ARC", "BEZIER", "CIRCLE(radius=1)"] {
        let r = read("0.4", &format!("# @SELECT-INSTANCE(name=\"missing\") => @SELECT-PIN => @{call}"));
        ok(&r);
        assert!(r.model.levels[0].shapes.is_empty());
    }
}

#[test]
fn invalid_size_fails_even_when_selection_is_empty() {
    let r = read("0.4", "# @SELECT-INSTANCE(name=\"missing\") => @SELECT-PIN => @RECTANGLE(width=2)");
    assert!(r.diagnostics().iter().any(|d| d.code == codes::PARAM_GRAPHIC_WIDTH_HEIGHT));
}

#[test]
fn a_bad_member_does_not_discard_other_centred_graphics() {
    let r = read("0.4", "%pins = #R1 @SELECT-PIN; {%pins #R2} @CIRCLE(radius=1)");
    assert_eq!(r.diagnostics().iter().filter(|d| d.code == codes::KEYWORD_GRAPHIC_NOT_POINTS).count(), 1);
    assert_eq!(r.model.levels[0].shapes.len(), 2);
}

#[test]
fn rules_in_explicit_usages_run_but_generation_cannot_recurse() {
    let definition = "$D = @DEFINE(roles=[\"x\"]) { ?x @SELECT-PIN => @CIRCLE(radius=1) }; ";
    let r = read("0.4", &(definition.to_string() + "#R1 @AS $D"));
    ok(&r);
    assert_eq!(r.model.levels[0].shapes.len(), 2);
    let r = read("0.4", &(definition.to_string() + "# @SELECT-INSTANCE(name=\"R1\") => @AS $D"));
    assert!(r.diagnostics().iter().any(|d| d.code == codes::USAGE_GENERATED_RULE));
    assert!(r.model.levels[0].shapes.is_empty());
}

#[test]
fn recursive_graphic_sets_are_generated_independently_in_each_level() {
    for recursive in [false, true] {
        let source = format!(
            r##"* recursive graphics
* @RNL(type="header") {{ @VERSION("0.4") }}
.subckt child p q
R2 p q 1k
.ends child
R1 a b 1k
X1 a b child
* @RNL {{ $S = @DEFINE-SYMBOL {{ (0,0) @PIN(terminal=1) }}; $Red = @DEFINE-STYLE(linecolor="#ff0000") {{}}; # @SELECT-INSTANCE(name="R*",recursive=true) => @SET-SYMBOL $S => @PLACE(0,0); %circles = # @SELECT-INSTANCE(name="R*",recursive={recursive}) => @SELECT-PIN => @CIRCLE(radius=1); %circles @SET-STYLE $Red }}
.end
"##
        );
        let r = interpret_file(
            Path::new("t.sp"),
            None,
            &MemoryProvider::new().with("t.sp", &source),
            &Options { std: false, ..Options::solver() },
        )
        .unwrap();
        ok(&r);
        assert_eq!(r.model.levels[0].shapes.len(), 1);
        assert_eq!(r.model.levels.iter().find(|lv| lv.title == "child").unwrap().shapes.len(), usize::from(recursive));
        assert!(r.model.levels.iter().flat_map(|lv| &lv.shapes).all(|s| s
            .style
            .get("linecolor")
            .and_then(|x| x.str())
            == Some("#ff0000")));
    }
}
