//! Fixed text direction keeps the transformed anchor (13.3).
use std::path::Path;

use rnl_cli::interpret_file;
use rnl_render::{primitives, Kind, RenderOptions, TextPrim};
use rnl_runtime::diag::Severity;
use rnl_runtime::geom::Orient;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn read(version: &str, body: &str) -> Interpretation {
    let source = format!(
        "* text rotation\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\n* @RNL {{\n{}\n* }}\n.end\n",
        body.lines().map(|line| format!("* {line}")).collect::<Vec<_>>().join("\n")
    );
    interpret_file(
        Path::new("t.sp"),
        None,
        &MemoryProvider::new().with("t.sp", &source),
        &Options { std: false, ..Options::solver() },
    )
    .unwrap()
}

fn valid(body: &str) -> Interpretation {
    let r = read("0.4", body);
    let errors: Vec<_> = r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).collect();
    assert!(errors.is_empty(), "{body}: {errors:?}");
    r
}

fn ink(r: &Interpretation) -> Vec<TextPrim> {
    primitives(&r.model.levels[0], &RenderOptions::default())
        .into_iter()
        .filter_map(|p| match p.kind {
            Kind::Text(t) => Some(t),
            _ => None,
        })
        .collect()
}

#[test]
fn fixed_labels_and_fixed_symbol_text_keep_scene_direction_in_all_orientations() {
    for (turn, halign, valign) in [
        (Orient::R0, "end", "top"),
        (Orient::R90, "start", "top"),
        (Orient::R180, "start", "bottom"),
        (Orient::R270, "end", "bottom"),
        (Orient::MY, "start", "top"),
        (Orient::MX, "end", "bottom"),
        (Orient::MYR90, "start", "bottom"),
        (Orient::MXR90, "end", "top"),
    ] {
        for rotate in [0, 90, 180, 270] {
            let (halign, valign) = if rotate % 180 == 90 {
                match turn {
                    Orient::MY => ("end", "bottom"),
                    Orient::MX => ("start", "top"),
                    Orient::MYR90 => ("end", "top"),
                    Orient::MXR90 => ("start", "bottom"),
                    _ => (halign, valign),
                }
            } else {
                (halign, valign)
            };
            let q = turn.apply((2.0, 3.0));
            let at = (10.0 + q.0, 20.0 + q.1);
            let control = valid(&format!(
                "({}, {}) @TEXT(\"ink\", rotate={rotate}, halign=\"{halign}\", valign=\"{valign}\")",
                at.0, at.1
            ));
            let expected = ink(&control).pop().unwrap();
            for label in [false, true] {
                let attach = if label { "?" } else { "(2,3)" };
                let key = if label { ", key=\"name\"" } else { "" };
                let place = if label { " => @PLACE(2,3)" } else { "" };
                let r = valid(&format!("$S = @DEFINE-SYMBOL {{ {attach} @TEXT(\"ink\"{key}, fix_rotate=true, rotate={rotate}, halign=\"end\", valign=\"top\"){place} }}; #R1 @SET-SYMBOL $S => @PLACE(10,20) => @ORIENT(\"{}\")", turn.as_str()));
                assert_eq!(ink(&r).as_slice(), std::slice::from_ref(&expected), "{turn:?} {rotate} label={label}");
                if label {
                    let text = &r.model.levels[0].texts[0];
                    assert_eq!(text.anchor, at);
                    assert_eq!(text.orient, turn);
                    assert_eq!(text.fix_rotate, Some(true));
                    assert!(r.problems[0].objects.iter().filter_map(|o| o.text.as_ref()).all(|t| t.fix_rotate));
                }
            }
        }
    }
}

#[test]
fn key_overrides_preserve_then_reset_fixed_rotation_independently() {
    let base = "$S = @DEFINE-SYMBOL { ? @TEXT(\"old\", key=\"name\", fix_rotate=true, rotate=90) => @PLACE(2,3) }; #R1 @SET-SYMBOL $S => @PLACE(10,20) => @ORIENT(\"MXR90\");";
    for (extra, fixed, turn) in [
        ("#R1 @TEXT(\"new\", key=\"name\", halign=\"end\")", true, Orient::MXR90),
        ("#R1 @TEXT(\"new\", key=\"name\", fix_rotate=false)", false, Orient::MXR90),
    ] {
        let r = valid(&format!("{base} {extra}"));
        let t = &r.model.levels[0].texts[0];
        assert_eq!(t.fix_rotate, Some(fixed));
        assert_eq!(t.orient, turn);
        assert_eq!(t.rotate, 90.0);
        let local = Orient::MXR90.apply((2.0, 3.0));
        assert_eq!(t.anchor, (10.0 + local.0, 20.0 + local.1));
        assert_eq!(t.markup, "new");
    }
}

#[test]
fn scene_text_and_scene_replaced_positions_keep_the_existing_direction_rule() {
    let r = valid(
        r#"$S = @DEFINE-SYMBOL { ? @TEXT("label", key="name") => @PLACE(2,3) };
        #R1 @SET-SYMBOL $S => @PLACE(10,20) => @ORIENT("R90");
        #R1 @TEXT("label", key="name", fix_rotate=false, dx=4, dy=5);
        #R1 @TEXT("scene", fix_rotate=false, dx=6, dy=7)"#,
    );
    let texts = &r.model.levels[0].texts;
    assert_eq!(texts.len(), 2);
    assert_eq!(texts[0].anchor, (14.0, 25.0));
    assert_eq!(texts[1].anchor, (16.0, 27.0));
    assert!(texts.iter().all(|t| t.orient == Orient::R0));
}

#[test]
fn nested_symbols_transform_the_anchor_but_not_fixed_text_direction() {
    let r = valid(
        r#"$D = @DEFINE(roles=["at"]) {};
        $I = @DEFINE-SYMBOL { (2,3) @TEXT("ink", fix_rotate=true, rotate=90) };
        $S = @DEFINE-SYMBOL { $D @ATTACH (4,5) => @SET-SYMBOL $I => @ORIENT("MX") };
        #R1 @SET-SYMBOL $S => @PLACE(10,20) => @ORIENT("R90")"#,
    );
    let control = valid("(8,26) @TEXT(\"ink\", rotate=90, halign=\"end\", valign=\"top\")");
    assert_eq!(ink(&r), ink(&control));
}

#[test]
fn symbol_inclusion_keeps_fixed_text_and_label_settings() {
    let r = valid(
        r#"$I = @DEFINE-SYMBOL { (2,3) @TEXT("ink", fix_rotate=true); ? @TEXT("label", key="name", fix_rotate=true) => @PLACE(2,3) };
        $S = @DEFINE-SYMBOL { $I };
        #R1 @SET-SYMBOL $S => @PLACE(10,20) => @ORIENT("MY")"#,
    );
    assert!(ink(&r).iter().all(|t| t.anchor == (8.0, 23.0) && !t.vertical));
    assert_eq!(r.model.levels[0].texts[0].fix_rotate, Some(true));
}

#[test]
fn fixed_rotation_requires_a_boolean() {
    for value in ["1", "\"true\"", "[true]"] {
        let r = read("0.4", &format!("(0,0) @TEXT(\"bad\", fix_rotate={value})"));
        assert!(r.diagnostics().iter().any(|d| d.severity == Severity::Error));
    }
}

#[test]
fn fixed_rotation_changes_the_solver_digest_without_changing_the_anchor_constraint() {
    let r = |fixed| {
        valid(&format!("$S = @DEFINE-SYMBOL {{ ? @TEXT(\"ink\", key=\"name\", fix_rotate={fixed}) => @PLACE(2,3) }}; #R1 @SET-SYMBOL $S => @PLACE(10,20) => @ORIENT(\"R90\")"))
    };
    let a = r(false);
    let b = r(true);
    assert_ne!(rnl_runtime::problem::input_digest(&a.problems[0]), rnl_runtime::problem::input_digest(&b.problems[0]));
    let a = a.problems[0].objects.iter().find(|o| o.text.is_some()).unwrap();
    let b = b.problems[0].objects.iter().find(|o| o.text.is_some()).unwrap();
    assert_eq!(a.position, b.position);
}

#[test]
fn fixed_text_above_a_symbol_moves_to_the_correct_side() {
    for (turn, h, v) in [
        (Orient::R0, "middle", "bottom"),
        (Orient::R90, "end", "middle"),
        (Orient::R180, "middle", "top"),
        (Orient::R270, "start", "middle"),
        (Orient::MY, "middle", "bottom"),
        (Orient::MX, "middle", "top"),
        (Orient::MYR90, "end", "middle"),
        (Orient::MXR90, "start", "middle"),
    ] {
        for valign in ["bottom", "baseline"] {
            let r = valid(&format!(
                r#"$S = @DEFINE-SYMBOL {{ ? @TEXT("outside",key="name",fix_rotate=true,halign="middle",valign="{valign}") => @PLACE(0,2) }}; #R1 @SET-SYMBOL $S => @PLACE(0,0) => @ORIENT("{}")"#,
                turn.as_str()
            ));
            let text = ink(&r).pop().unwrap();
            assert_eq!(text.halign, h, "{turn:?} {valign}");
            assert_eq!(text.valign, if v == "bottom" { valign } else { v }, "{turn:?} {valign}");
            assert!(!text.vertical);
            assert_eq!(text.anchor, turn.apply((0.0, 2.0)));
        }
    }
}

#[test]
fn mirroring_a_quarter_turned_fixed_label_moves_its_box_across_the_anchor() {
    let control = valid(r#"(0,2) @TEXT("side",rotate=90,halign="middle",valign="top")"#);
    for keyed in [false, true] {
        let attach = if keyed { "?" } else { "(0,2)" };
        let key = if keyed { ",key=\"name\"" } else { "" };
        let at = if keyed { " => @PLACE(0,2)" } else { "" };
        let source = format!(
            r#"$S = @DEFINE-SYMBOL {{ {attach} @TEXT("side"{key},rotate=90,fix_rotate=true,halign="middle",valign="bottom"){at} }}; #R1 @SET-SYMBOL $S => @PLACE(0,0) => @ORIENT("MY")"#
        );
        assert_eq!(ink(&valid(&source)), ink(&control), "keyed={keyed}");
    }
}
