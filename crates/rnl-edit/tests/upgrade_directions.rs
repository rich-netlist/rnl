use rnl_edit::upgrade::{upgrade, UpgradeOptions};
use rnl_runtime::source::MemoryProvider;
fn convert(statement: &str) -> rnl_edit::upgrade::Upgrade {
    let source = format!("* title\n* @RNL(type=\"header\") {{ @VERSION(\"0.3\") }}\n* @RNL {{ {statement} }}\n.end\n");
    upgrade(
        "t.sp",
        &MemoryProvider::new().with("t.sp", &source),
        &UpgradeOptions { std: false, ..UpgradeOptions::default() },
    )
    .unwrap()
}
#[test]
fn negative_directed_intervals_reverse_both_bounds() {
    let r = convert("(0, 0) (-3, 0) @DISTANCE(\"left-to-right\", min=-5, max=-2, priority=\"high\")");
    assert!(r.files["t.sp"].contains("@DISTANCE(\"right-to-left\", priority=\"high\", min=2, max=5)"));
    assert!(r.report.files[0].unconverted.is_empty());
    assert!(r.report.successful(), "{:?}", r.report);
}
#[test]
fn negative_exact_and_unbounded_lower_intervals_reverse() {
    let r = convert("(0, 0) (0, 3) @DISTANCE(\"top-to-bottom\", exact=-3)");
    assert!(r.files["t.sp"].contains("@DISTANCE(\"bottom-to-top\", exact=3)"));
    let r = convert("(0, 0) (-3, 0) @DISTANCE(\"left-to-right\", max=-2)");
    assert!(r.files["t.sp"].contains("@DISTANCE(\"right-to-left\", min=2)"));
}
#[test]
fn a_crossing_directed_interval_is_retained() {
    let r = convert("(0, 0) (3, 0) @DISTANCE(\"left-to-right\", min=-2, max=5)");
    assert!(r.files["t.sp"].contains("@DISTANCE(\"left-to-right\", min=-2, max=5)"));
    assert!(r.report.files[0].unconverted.iter().any(|u| u.rule == "distance-interval"));
}
#[test]
fn literal_coordinates_do_not_choose_a_discardable_direction() {
    for statement in [
        "{(0, 0) (2, 0) (4, 0)} @DISTRIBUTE(axis=\"x\")",
        "{(0, 4) (0, 2) (0, 0)} @DISTRIBUTE(axis=\"y\")",
        "(0, 0) (2, 0) @ORDER(\"x\")",
        "(0, 0) (2, 0) @DISTANCE(\"x\", min=1)",
    ] {
        let r = convert(statement);
        assert!(r.files["t.sp"].contains(statement));
        assert!(r.report.files[0].unconverted.iter().any(|u| u.rule == "direction"));
    }
}
#[test]
fn zero_axis_distance_has_an_equivalent_direction() {
    for (axis, direction) in [("x", "left-to-right"), ("y", "bottom-to-top")] {
        let r = convert(&format!("(_, _) (_, _) @DISTANCE(\"{axis}\", exact=0)"));
        assert!(r.files["t.sp"].contains(&format!("@DISTANCE(\"{direction}\", exact=0)")));
        assert!(r.report.successful(), "{:?}", r.report);
    }
}
#[test]
fn free_or_coincident_points_do_not_determine_a_strict_distribution() {
    for points in ["{(0, 0) (_, 0) (4, 0)}", "{(0, 0) (0, 0) (0, 0)}"] {
        let r = convert(&format!("{points} @DISTRIBUTE(\"x\")"));
        assert!(r.files["t.sp"].contains("@DISTRIBUTE(\"x\")"));
        assert!(r.report.files[0].unconverted.iter().any(|u| u.rule == "direction"));
    }
}

#[test]
fn set_extents_are_not_negated_as_single_point_distances() {
    let r = convert("{(0, 0) (2, 0)} {(-4, 0) (-3, 0)} @DISTANCE(\"left-to-right\", min=-8, max=-1)");
    assert!(r.files["t.sp"].contains("@DISTANCE(\"left-to-right\", min=-8, max=-1)"));
    assert!(r.report.files[0].unconverted.iter().any(|u| u.rule == "distance-interval"));
}

#[test]
fn zero_boundary_keeps_a_canonical_zero_in_comparison() {
    let r = convert("(0, 0) (-3, 0) @DISTANCE(\"left-to-right\", min=-5, max=0)");
    assert!(r.files["t.sp"].contains("@DISTANCE(\"right-to-left\", min=0, max=5)"));
    assert!(r.report.successful(), "{:?}", r.report);
}
