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
fn literal_points_justify_each_axis_direction() {
    for (points, axis, direction) in [
        ("{(0, 0) (2, 0) (4, 0)}", "x", "left-to-right"),
        ("{(4, 0) (2, 0) (0, 0)}", "x", "right-to-left"),
        ("{(0, 0) (0, 2) (0, 4)}", "y", "bottom-to-top"),
        ("{(0, 4) (0, 2) (0, 0)}", "y", "top-to-bottom"),
    ] {
        let r = convert(&format!("{points} @DISTRIBUTE(axis=\"{axis}\")"));
        assert!(r.files["t.sp"].contains(&format!("@DISTRIBUTE(\"{direction}\")")), "{}", r.files["t.sp"]);
        assert!(r.report.files[0].unconverted.is_empty());
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
