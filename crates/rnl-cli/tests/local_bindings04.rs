//! Local names retain their first valid binding, including when reached forwards (6.6, 6.9).
use rnl_runtime::{
    diag::{codes, Code, Severity},
    source::MemoryProvider,
    Interpretation, Options,
};
use std::path::Path;

fn read(version: &str, roles: &str, body: &str, calls: &str) -> Interpretation {
    let calls = calls.replace("$D", "$D => @PLACE(0,0)");
    let source=format!("* local names\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\nR2 a b 2k\n* @RNL {{ #R1 @PLACE(1,0); #R2 @PLACE(8,0); $D = @DEFINE(roles={roles},parameters=[[\"v\",1]]) {{ {body} }}; {calls} }}\n");
    rnl_cli::interpret_file(
        Path::new("names.sp"),
        None,
        &MemoryProvider::new().with("names.sp", &source),
        &Options { std: false, problems: true, ..Default::default() },
    )
    .unwrap()
}
fn errors(result: &Interpretation) -> Vec<Code> {
    result.diagnostics().iter().filter(|d| d.severity == Severity::Error).map(|d| d.code).collect()
}
fn shapes(result: &Interpretation) -> Vec<Vec<(f64, f64)>> {
    result.model.levels[0].shapes.iter().map(|s| s.points.clone()).collect()
}

#[test]
fn repeated_role_or_local_references_keep_the_same_handle() {
    for body in ["%x=?r; %x=?r", "%p=?r; %x=%p; %x=%p", "%x=?r => @PLACE(x=?v); %x=?r"] {
        for calls in ["", "#R1 @AS $D"] {
            let r = read("0.4", r#"["r"]"#, body, calls);
            assert!(errors(&r).is_empty(), "{body}: {:?}", r.diagnostics());
        }
    }
}

#[test]
fn different_roles_are_compared_per_application() {
    let body = "%x=?a; %x=?b; %x::position @CIRCLE(radius=1)";
    for (calls, count) in [("", 0), ("#R1 #R1 @AS $D", 0), ("#R1 #R2 @AS $D", 1)] {
        let r = read("0.4", r#"["a","b"]"#, body, calls);
        assert_eq!(errors(&r), vec![codes::DEFINE_LOCAL_NAME_TWICE; count], "{:?}", r.diagnostics());
        if !calls.is_empty() {
            assert_eq!(shapes(&r), [vec![(1., 0.)]]);
        }
    }
}

#[test]
fn failed_first_bindings_do_not_hold_the_local_name() {
    for bad in ["(0,0) @CIRCLE(radius=\"bad\")", "(?v,0)"] {
        for forward in [false, true] {
            let bind = format!("%x={bad}; %x=(4,0)");
            let body =
                if forward { format!("%x @CIRCLE(radius=1); {bind}") } else { format!("{bind}; %x @CIRCLE(radius=1)") };
            let r = read("0.4", r#"["r"]"#, &body, "%u=#R1 @AS(v=\"bad\") $D; %u::x @PLACE(4,0)");
            assert!(!errors(&r).contains(&codes::DEFINE_LOCAL_NAME_TWICE), "{:?}", r.diagnostics());
            assert_eq!(errors(&r).len(), 1, "{:?}", r.diagnostics());
            assert_eq!(shapes(&r), [vec![(4., 0.)]], "{:?}", r.diagnostics());
        }
    }
}

#[test]
fn a_failed_later_binding_keeps_the_first_valid_one() {
    let r = read("0.4", r#"["r"]"#, "%x=(1,0); %x=(?v,0); %x @CIRCLE(radius=1)", "#R1 @AS(v=\"bad\") $D");
    assert_eq!(errors(&r).len(), 1, "{:?}", r.diagnostics());
    assert!(!errors(&r).contains(&codes::DEFINE_LOCAL_NAME_TWICE));
    assert_eq!(shapes(&r), [vec![(1., 0.)]]);
}

#[test]
fn forward_evaluation_does_not_change_the_written_binding_order() {
    for body in
        ["%y=%x; %x=?a; %x=?b; %y::position @CIRCLE(radius=1)", "%x=%y; %x=(8,0); %y=(1,0); %x @CIRCLE(radius=1)"]
    {
        let r = read("0.4", r#"["a","b"]"#, body, "#R1 #R2 @AS $D");
        assert_eq!(errors(&r), [codes::DEFINE_LOCAL_NAME_TWICE], "{:?}", r.diagnostics());
        assert_eq!(shapes(&r), [vec![(1., 0.)]], "{:?}", r.diagnostics());
    }
}

#[test]
fn distinct_constant_points_are_static_errors_even_when_unused() {
    for calls in ["", "#R1 @AS $D"] {
        let r = read("0.4", r#"["r"]"#, "%x=(1,0); %x=(1,0) @CIRCLE(radius=1)", calls);
        assert_eq!(errors(&r), [codes::DEFINE_LOCAL_NAME_TWICE], "{:?}", r.diagnostics());
        assert!(shapes(&r).is_empty());
    }
}

#[test]
fn released_definitions_keep_their_duplicate_name_diagnostic() {
    for version in ["0.2", "0.3"] {
        let r = read(version, r#"["r"]"#, "%x=?r; %x=?r", "#R1 @AS $D");
        assert!(errors(&r).contains(&codes::DEFINE_LOCAL_NAME_TWICE), "{:?}", r.diagnostics());
    }
}

#[test]
fn a_statically_failed_member_reserves_none_of_its_names() {
    let r = read("0.4", r#"["r"]"#, "%a=(1,0); %x=(3,0) => %a; %x=(4,0); %x @CIRCLE(radius=1)", "#R1 @AS $D");
    assert_eq!(errors(&r), [codes::DEFINE_LOCAL_NAME_TWICE], "{:?}", r.diagnostics());
    assert_eq!(shapes(&r), [vec![(4., 0.)]], "{:?}", r.diagnostics());
}

#[test]
fn failed_modifier_bindings_keep_the_input_handle() {
    let r = read(
        "0.4",
        r#"["r"]"#,
        "%x=?r => @PLACE(x=?v); %x=?r; %x::position @CIRCLE(radius=1)",
        "#R1 @AS(v=\"bad\") $D",
    );
    assert_eq!(errors(&r).len(), 1, "{:?}", r.diagnostics());
    assert!(!errors(&r).contains(&codes::DEFINE_LOCAL_NAME_TWICE));
    assert_eq!(shapes(&r), [vec![(1., 0.)]]);
}

#[test]
fn repeated_names_in_a_pipe_compare_each_bound_handle() {
    let same = read("0.4", r#"["r"]"#, "?r => %x => @PLACE(x=1) => %x; %x::position @CIRCLE(radius=1)", "#R1 @AS $D");
    assert!(errors(&same).is_empty(), "{:?}", same.diagnostics());
    assert_eq!(shapes(&same), [vec![(1., 0.)]]);
    let different =
        read("0.4", r#"["r"]"#, "?r::position => %x => @CIRCLE(radius=1) => %x; %x @CIRCLE(radius=2)", "#R1 @AS $D");
    assert_eq!(errors(&different), [codes::DEFINE_LOCAL_NAME_TWICE], "{:?}", different.diagnostics());
    assert_eq!(shapes(&different), [vec![(1., 0.)], vec![(1., 0.)]]);
}

#[test]
fn two_set_literals_make_different_objects_even_if_their_members_match() {
    let r = read("0.4", r#"["r"]"#, "%x={?r}; %x={?r}", "");
    assert_eq!(errors(&r), [codes::DEFINE_LOCAL_NAME_TWICE], "{:?}", r.diagnostics());
}

#[test]
fn known_local_aliases_share_the_original_handle_in_static_checks() {
    for calls in ["", "#R1 @AS $D"] {
        let same = read("0.4", r#"["r"]"#, "%p=(1,0); %x=%p; %x=%p", calls);
        assert!(errors(&same).is_empty(), "{:?}", same.diagnostics());
        let different = read("0.4", r#"["r"]"#, "%p=(1,0); %x=%p; %x=(4,0)", calls);
        assert_eq!(errors(&different), [codes::DEFINE_LOCAL_NAME_TWICE], "{:?}", different.diagnostics());
    }
}

#[test]
fn dynamic_creation_parameters_cannot_prove_a_static_binding_conflict() {
    for first in [
        "(0,0) @CIRCLE(radius=1,linewidth=?v)",
        "(0,0) @RECTANGLE(width=1,height=2,linewidth=?v)",
        "(0,0) @ELLIPSE(width=1,height=2,linewidth=?v)",
    ] {
        let body = format!("%x={first}; %x=(4,0); %x::position @CIRCLE(radius=2)");
        let unused = read("0.4", r#"["r"]"#, &body, "");
        assert!(errors(&unused).is_empty(), "{first}: {:?}", unused.diagnostics());
        let failed = read("0.4", r#"["r"]"#, &body, "#R1 @AS(v=\"bad\") $D");
        assert_eq!(errors(&failed).len(), 1, "{first}: {:?}", failed.diagnostics());
        assert!(!errors(&failed).contains(&codes::DEFINE_LOCAL_NAME_TWICE));
        assert_eq!(shapes(&failed), [vec![(4., 0.)]]);
        let valid = read("0.4", r#"["r"]"#, &body, "#R1 @AS(v=1) $D");
        assert_eq!(errors(&valid), [codes::DEFINE_LOCAL_NAME_TWICE], "{first}: {:?}", valid.diagnostics());
    }
}

#[test]
fn cyclic_invalid_bindings_leave_room_for_a_valid_later_binding() {
    for body in ["%x=%x; %x=?r; %x::position @CIRCLE(radius=1)", "%x=%y; %y=%x; %x=?r; %x::position @CIRCLE(radius=1)"]
    {
        let r = read("0.4", r#"["r"]"#, body, "#R1 @AS $D");
        assert!(errors(&r).contains(&codes::REF_NAME_CYCLE), "{body}: {:?}", r.diagnostics());
        assert!(!errors(&r).contains(&codes::DEFINE_LOCAL_NAME_TWICE), "{body}: {:?}", r.diagnostics());
        assert_eq!(shapes(&r), [vec![(1., 0.)]], "{body}: {:?}", r.diagnostics());
    }
}

#[test]
fn prefix_and_pipe_bindings_keep_the_first_written_name() {
    let r = read("0.4", r#"["r"]"#, "%x = ?r::position => %x => @CIRCLE(radius=1); %x @CIRCLE(radius=2)", "#R1 @AS $D");
    assert_eq!(
        errors(&r),
        [codes::DEFINE_LOCAL_NAME_TWICE, codes::KEYWORD_GRAPHIC_NOT_POINTS],
        "{:?}",
        r.diagnostics()
    );
    // The prefix names the graphic, so treating it as a point must fail.
    assert_eq!(shapes(&r).len(), 1, "{:?}", r.diagnostics());
}

#[test]
fn unique_forward_aliases_are_checked_without_applying_the_definition() {
    for body in ["%x=%p; %p=(1,0); %x=(4,0)", "%x=%p; %p=%q; %q=(1,0); %x=(4,0)", "%x=%p; %p={?r}; %x={?r}"] {
        for calls in ["", "#R1 @AS $D"] {
            let r = read("0.4", r#"["r"]"#, body, calls);
            assert_eq!(errors(&r), [codes::DEFINE_LOCAL_NAME_TWICE], "{body}: {:?}", r.diagnostics());
        }
    }
}

#[test]
fn a_creation_that_fails_after_symbols_keeps_its_local_name() {
    let r = read(
        "0.4",
        r#"["r"]"#,
        "%x = ?r::missing @AS $P; %x = ?r; %x::position @CIRCLE(radius=1)",
        "$P = @DEFINE(roles=[[\"p\",\"pin\"]]) {}; #R1 @AS $D",
    );
    assert!(errors(&r).contains(&codes::DEFINE_LOCAL_NAME_TWICE), "{:?}", r.diagnostics());
    assert!(errors(&r).contains(&codes::REF_NO_SUBOBJECT), "{:?}", r.diagnostics());
    assert!(shapes(&r).is_empty(), "{:?}", r.diagnostics());
}

#[test]
fn two_names_of_one_symbol_pin_are_different_early_handles() {
    let r = read(
        "0.4",
        r#"["r"]"#,
        "%x = ?r::p; %x = ?r::q",
        "$S = @DEFINE-SYMBOL { %p = (0,0) @PIN(terminal=1) => %q }; #R1 @SET-SYMBOL $S; #R1 @AS $D",
    );
    assert_eq!(errors(&r), [codes::DEFINE_LOCAL_NAME_TWICE], "{:?}", r.diagnostics());
}

#[test]
fn known_alias_kinds_support_creation_checks_without_application() {
    for body in [
        "%p=(1,0); %x=%p @CIRCLE(radius=1); %x=(4,0)",
        "%x=%p @CIRCLE(radius=1); %p=(1,0); %x=(4,0)",
        "%p={?r}; %x=%p @UNION; %x={?r}",
        "%p=(1,0); %x=%p @TEXT(\"a\"); %x=(4,0)",
    ] {
        let r = read("0.4", r#"["r"]"#, body, "");
        assert_eq!(errors(&r), [codes::DEFINE_LOCAL_NAME_TWICE], "{body}: {:?}", r.diagnostics());
    }
}

#[test]
fn wire_creation_does_not_inherit_the_identity_of_invalid_input() {
    let r = read("0.4", r#"["r"]"#, "%x=(0,0) @WIRE (1,0); %x=(4,0); %x @CIRCLE(radius=1)", "#R1 @AS $D");
    assert_eq!(errors(&r).len(), 1, "{:?}", r.diagnostics());
    assert!(!errors(&r).contains(&codes::DEFINE_LOCAL_NAME_TWICE));
    assert_eq!(shapes(&r), [vec![(4., 0.)]], "{:?}", r.diagnostics());
}

#[test]
fn a_static_pipe_conflict_points_to_the_later_written_binding() {
    let line = "* @RNL { $D = @DEFINE(roles=[\"r\"]) { %x = (0,0) => %x => @CIRCLE(radius=1) } }";
    let source = format!("* static pipe\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\n{line}\n");
    let r = rnl_cli::interpret_file(
        Path::new("pipe.sp"),
        None,
        &MemoryProvider::new().with("pipe.sp", &source),
        &Options { std: false, ..Default::default() },
    )
    .unwrap();
    assert_eq!(errors(&r), [codes::DEFINE_LOCAL_NAME_TWICE], "{:?}", r.diagnostics());
    let diagnostic = r.diagnostics().into_iter().find(|d| d.code == codes::DEFINE_LOCAL_NAME_TWICE).unwrap();
    let loc = diagnostic.loc.as_ref().unwrap();
    assert_eq!(loc.line, 3);
    assert_eq!(loc.col as usize, line.find("=> %x").unwrap() + 4);
}

#[test]
fn an_invalid_label_does_not_reserve_a_creation_name() {
    let r = read("0.4", r#"["r"]"#, "%x=(0,0)(bad) @CIRCLE(radius=1); %x=(4,0); %x @CIRCLE(radius=1)", "#R1 @AS $D");
    assert_eq!(errors(&r), [codes::KEYWORD_UNKNOWN_POSITION], "{:?}", r.diagnostics());
    assert_eq!(shapes(&r), [vec![(4., 0.)]], "{:?}", r.diagnostics());
}

#[test]
fn argument_independent_signature_errors_are_checked_once_at_the_definition() {
    let cases = [
        ("?r(bad) @PLACE(x=1)", codes::KEYWORD_UNKNOWN_POSITION),
        ("?r(items) {} @ADD-TO", codes::ROLE_LABELS_MIXED),
        ("?r(items) ?r(items) @ADD-TO", codes::KEYWORD_POSITION_LABELLED_TWICE),
        ("?r(items) @ADD-TO", codes::KEYWORD_POSITION_NOT_FILLED),
        ("@CIRCLE(radius=1)", codes::KEYWORD_NO_OBJECT),
        ("?r {} {} @ADD-TO", codes::KEYWORD_SLOT_COUNT),
        ("(0,0) @ADD-TO", codes::KEYWORD_SLOT_COUNT),
        ("?r => @ADD-TO {}(target)", codes::SYNTAX_ROLE_AFTER_PIPE),
    ];
    for (bad, code) in cases {
        for calls in ["", "#R1 @AS $D; #R2 @AS $D"] {
            let r = read("0.4", r#"["r"]"#, &format!("{bad}; (5,0) @CIRCLE(radius=1)"), calls);
            assert_eq!(errors(&r), [code], "{bad}: {:?}", r.diagnostics());
            assert_eq!(shapes(&r).len(), if calls.is_empty() { 0 } else { 2 }, "{bad}: {:?}", r.diagnostics());
            let diag = r.diagnostics().into_iter().find(|d| d.code == code).unwrap();
            assert!(diag.origin.is_empty(), "{bad}: {diag:?}");
        }
    }
}

#[test]
fn the_size_of_a_sole_untyped_role_is_not_a_static_signature_error() {
    let unused = read("0.4", r#"["r"]"#, "?r @ADD-TO", "");
    assert!(errors(&unused).is_empty(), "{:?}", unused.diagnostics());
    let applied = read("0.4", r#"["r"]"#, "?r @ADD-TO", "#R1 @AS $D");
    assert_eq!(errors(&applied), [codes::KEYWORD_SLOT_COUNT], "{:?}", applied.diagnostics());
}

#[test]
fn a_declared_single_object_role_has_a_known_signature_count() {
    for roles in [r#"[["r","instance"]]"#, r#"[["r",["instance","point"]]]"#] {
        for calls in ["", "#R1 @AS $D; #R2 @AS $D"] {
            let r = read("0.4", roles, "?r @ADD-TO; (5,0) @CIRCLE(radius=1)", calls);
            assert_eq!(errors(&r), [codes::KEYWORD_SLOT_COUNT], "{roles}: {:?}", r.diagnostics());
            assert_eq!(shapes(&r).len(), if calls.is_empty() { 0 } else { 2 });
        }
    }
    let set = read("0.4", r#"[["{r}","instance"]]"#, "?r @ADD-TO", "");
    assert!(errors(&set).is_empty(), "{:?}", set.diagnostics());
}

#[test]
fn literal_multi_point_creations_have_fresh_handles_without_application() {
    for graphic in [
        "{(0,0) (1,0)} @LINE",
        "(0,0) (1,0) @LINE",
        "{(0,0) (1,0)} @CIRCLE",
        "{(0,0) (1,0)} @RECTANGLE",
        "{(0,0) (1,0)} @ELLIPSE",
        "{(0,0) (1,1) (2,0)} @ARC",
        "{(0,0) (1,1) (2,1) (3,0)} @BEZIER",
    ] {
        for calls in ["", "#R1 @AS $D; #R2 @AS $D"] {
            let r = read("0.4", r#"["r"]"#, &format!("%x={graphic}; %x=(4,0)"), calls);
            assert_eq!(errors(&r), [codes::DEFINE_LOCAL_NAME_TWICE], "{graphic}: {:?}", r.diagnostics());
            assert_eq!(shapes(&r).len(), if calls.is_empty() { 0 } else { 2 });
        }
    }
}

#[test]
fn an_invalid_literal_graphic_count_does_not_reserve_its_name() {
    for graphic in ["{(0,0)} @LINE", "{(0,0)} @CIRCLE", "{(0,0) (1,0)} @ARC", "{} @BEZIER"] {
        for calls in ["", "#R1 @AS $D; #R2 @AS $D"] {
            let r = read("0.4", r#"["r"]"#, &format!("%x={graphic}; %x=(4,0); %x @CIRCLE(radius=1)"), calls);
            assert_eq!(errors(&r), [codes::KEYWORD_GRAPHIC_POINT_COUNT], "{graphic}: {:?}", r.diagnostics());
            assert_eq!(shapes(&r), vec![vec![(4., 0.)]; if calls.is_empty() { 0 } else { 2 }]);
        }
    }
}

#[test]
fn constant_definition_creations_have_distinct_handles_before_application() {
    for creation in ["@DEFINE(roles=[]) {}", "@DEFINE(roles=[\"a\"],parameters=[[\"p\",1]]) {}", "@DEFINE-SYMBOL {}"] {
        for calls in ["", "#R1 @AS $D; #R2 @AS $D"] {
            let r = read("0.4", r#"["r"]"#, &format!("%x = {creation}; %x=(4,0); (5,0) @CIRCLE(radius=1)"), calls);
            assert_eq!(errors(&r), [codes::DEFINE_LOCAL_NAME_TWICE], "{creation}: {:?}", r.diagnostics());
            assert_eq!(shapes(&r).len(), if calls.is_empty() { 0 } else { 2 });
        }
    }
}

#[test]
fn malformed_nested_declarations_are_static_and_do_not_reserve_names() {
    for (parameters, code) in [
        (r#"roles=["a","a"]"#, codes::DEFINE_ROLE_NAME_INVALID),
        (r#"roles=[["a","not-a-kind"]]"#, codes::DEFINE_ROLE_UNKNOWN_KIND),
        (r#"roles=[42]"#, codes::DEFINE_ROLE_MALFORMED),
        (r#"roles=[],parameters=[42]"#, codes::DEFINE_PARAM_MALFORMED),
        (r#"roles=[],parameters=["p","p"]"#, codes::DEFINE_PARAM_NAME_INVALID),
        (r#"roles=["r"],parameters=["r"]"#, codes::DEFINE_PARAM_NAME_INVALID),
    ] {
        for calls in ["", "#R1 @AS $D; #R2 @AS $D"] {
            let r = read(
                "0.4",
                r#"["r"]"#,
                &format!("%x = @DEFINE({parameters}) {{}}; %x=(4,0); %x @CIRCLE(radius=1)"),
                calls,
            );
            assert_eq!(errors(&r), [code], "{parameters}: {:?}", r.diagnostics());
            assert_eq!(shapes(&r), vec![vec![(4., 0.)]; if calls.is_empty() { 0 } else { 2 }]);
            assert!(r.diagnostics().into_iter().find(|d| d.code == code).unwrap().origin.is_empty());
        }
    }
}

#[test]
fn dynamic_declaration_validity_is_checked_per_application() {
    let body = "%x = @DEFINE(roles=?v) {}; %x=(4,0)";
    let unused = read("0.4", r#"["r"]"#, body, "");
    assert!(errors(&unused).is_empty(), "{:?}", unused.diagnostics());
    let bad = read("0.4", r#"["r"]"#, body, r#"#R1 @AS(v="bad") $D"#);
    assert_eq!(errors(&bad), [codes::PARAM_WRONG_TYPE], "{:?}", bad.diagnostics());
    let good = read("0.4", r#"["r"]"#, body, r#"#R1 @AS(v=["a"]) $D"#);
    assert_eq!(errors(&good), [codes::DEFINE_LOCAL_NAME_TWICE], "{:?}", good.diagnostics());
}

#[test]
fn required_definition_slots_are_checked_without_resolving_the_definition() {
    for (bad, code) in [
        ("@SET-SYMBOL", codes::KEYWORD_DEFINITION_MISSING),
        ("?r => @SET-SYMBOL", codes::KEYWORD_DEFINITION_PIPED),
        ("?r @SET-SYMBOL", codes::KEYWORD_NO_OBJECT),
        ("?r %s(bad) @SET-SYMBOL", codes::KEYWORD_DEFINITION_HAS_ROLE),
        ("?r(bad) %s @SET-SYMBOL", codes::KEYWORD_UNKNOWN_POSITION),
    ] {
        for calls in ["", "#R1 @AS $D; #R2 @AS $D"] {
            let r =
                read("0.4", r#"["r"]"#, &format!("%s = @DEFINE-SYMBOL {{}}; {bad}; (5,0) @CIRCLE(radius=1)"), calls);
            assert_eq!(errors(&r), [code], "{bad}: {:?}", r.diagnostics());
            assert_eq!(shapes(&r).len(), if calls.is_empty() { 0 } else { 2 });
            assert!(r.diagnostics().into_iter().find(|d| d.code == code).unwrap().origin.is_empty());
        }
    }
}

#[test]
fn definitions_require_a_written_body_in_modern_files() {
    for keyword in ["@DEFINE(roles=[])", "@DEFINE-SYMBOL", "@DEFINE-STYLE"] {
        for version in ["0.3", "0.4"] {
            for body in ["", " {}"] {
                let source = format!("* bodies\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\n* @RNL {{ $S = {keyword}{body} }}\n");
                let r = rnl_cli::interpret_file(
                    Path::new("body.sp"),
                    None,
                    &MemoryProvider::new().with("body.sp", &source),
                    &Options { std: false, ..Default::default() },
                )
                .unwrap();
                let expected =
                    if version == "0.4" && body.is_empty() { vec![codes::DEFINE_BODY_NOT_ONE_SET] } else { vec![] };
                assert_eq!(errors(&r), expected, "{keyword} {version} {body}: {:?}", r.diagnostics());
            }
        }
    }
}

#[test]
fn malformed_nested_definition_bodies_are_static_and_leave_names_available() {
    for keyword in ["@DEFINE(roles=[])", "@DEFINE-SYMBOL", "@DEFINE-STYLE"] {
        for body in ["", " ?r", " (0,0)", " {} {}"] {
            for calls in ["", "#R1 @AS $D; #R2 @AS $D"] {
                let r =
                    read("0.4", r#"["r"]"#, &format!("%x = {keyword}{body}; %x=(4,0); %x @CIRCLE(radius=1)"), calls);
                assert_eq!(errors(&r), [codes::DEFINE_BODY_NOT_ONE_SET], "{keyword}{body}: {:?}", r.diagnostics());
                assert_eq!(shapes(&r), vec![vec![(4., 0.)]; if calls.is_empty() { 0 } else { 2 }]);
                assert!(r
                    .diagnostics()
                    .into_iter()
                    .find(|d| d.code == codes::DEFINE_BODY_NOT_ONE_SET)
                    .unwrap()
                    .origin
                    .is_empty());
            }
        }
    }
}

#[test]
fn a_required_definition_prevents_implicit_grouping_of_written_objects() {
    for bad in ["?r ?r @SET-SYMBOL %s", "?r => @SET-SYMBOL ?r %s"] {
        for calls in ["", "#R1 @AS $D; #R2 @AS $D"] {
            let r =
                read("0.4", r#"["r"]"#, &format!("%s = @DEFINE-SYMBOL {{}}; {bad}; (5,0) @CIRCLE(radius=1)"), calls);
            assert_eq!(errors(&r), [codes::KEYWORD_SLOT_COUNT_WITH_DEFINITION], "{bad}: {:?}", r.diagnostics());
            assert_eq!(shapes(&r).len(), if calls.is_empty() { 0 } else { 2 });
            assert!(r
                .diagnostics()
                .into_iter()
                .find(|d| d.code == codes::KEYWORD_SLOT_COUNT_WITH_DEFINITION)
                .unwrap()
                .origin
                .is_empty());
        }
    }
}

#[test]
fn a_written_set_remains_one_slot_next_to_a_definition() {
    for good in ["{?r ?r} @SET-SYMBOL %s", "{?r ?r} => @SET-SYMBOL %s"] {
        for calls in ["", "#R1 @AS $D; #R2 @AS $D"] {
            let r =
                read("0.4", r#"["r"]"#, &format!("%s = @DEFINE-SYMBOL {{}}; {good}; (5,0) @CIRCLE(radius=1)"), calls);
            assert!(errors(&r).is_empty(), "{good}: {:?}", r.diagnostics());
            assert_eq!(shapes(&r).len(), if calls.is_empty() { 0 } else { 2 });
        }
    }
}

#[test]
fn impossible_declared_slot_kinds_are_static_errors() {
    for (kind, bad, filler, code) in [
        ("net", "?r @PLACE(1,2)", "&a", codes::PLACE_NOT_TAKEN),
        ("point", "?r @ORIENT(\"R90\")", "(0,0)", codes::PLACE_ORIENT_NOT_TAKEN),
        ("instance", "?r @CIRCLE(radius=1)", "#R1", codes::KEYWORD_GRAPHIC_NOT_POINTS),
        ("net", "?r (1,0) @DISTANCE(direction=\"left-to-right\",min=1)", "&a", codes::CONSTRAINT_MEMBER_NOT_LOCATABLE),
        ("point", "{} ?r @ADD-TO", "(0,0)", codes::SET_EDIT_TARGET_NOT_SET),
        ("instance", "?r {(0,0) (1,0)} @WIRE", "#R1", codes::WIRE_NET_WRONG_KIND),
        ("instance", "& {?r (1,0)} @WIRE", "#R1", codes::WIRE_POINT_WRONG_KIND),
    ] {
        let roles = format!("[[\"r\",\"{kind}\"]]");
        for calls in [String::new(), format!("{filler} @AS $D; {filler} @AS $D")] {
            let r = read("0.4", &roles, &format!("{bad}; (5,0) @CIRCLE(radius=1)"), &calls);
            assert_eq!(errors(&r), [code], "{kind} {bad}: {:?}", r.diagnostics());
            assert_eq!(shapes(&r).len(), if calls.is_empty() { 0 } else { 2 });
            assert!(r.diagnostics().into_iter().find(|d| d.code == code).unwrap().origin.is_empty());
        }
    }
}

#[test]
fn possible_conversions_and_unknown_collection_members_stay_deferred() {
    for (roles, body) in [
        (r#"[["r",["net","point"]]]"#, "?r @PLACE(1,2)"),
        (r#"[["r","pin"]]"#, "?r @CIRCLE(radius=1)"),
        (r#"[["r","terminal"]]"#, "?r @CIRCLE(radius=1)"),
        (r#"[["r","pin"]]"#, "?r {(0,0) (1,0)} @WIRE"),
        (r#"[["r","terminal"]]"#, "?r {(0,0) (1,0)} @WIRE"),
        (r#"[["r","instance"]]"#, "?r (1,0) @DISTANCE(direction=\"left-to-right\",min=1)"),
        (r#"[["r","annotation"]]"#, "?r (1,0) @DISTANCE(direction=\"left-to-right\",min=1)"),
        (r#"[["{r}","net"]]"#, "?r @PLACE(1,2)"),
        (r#"["r"]"#, "?r @PLACE(1,2)"),
    ] {
        let r = read("0.4", roles, body, "");
        assert!(errors(&r).is_empty(), "{roles} {body}: {:?}", r.diagnostics());
    }
}

#[test]
fn literal_object_kinds_are_checked_in_definition_slots() {
    for (bad, code) in [
        ("(0,0) @ORIENT(\"R90\")", codes::PLACE_ORIENT_NOT_TAKEN),
        ("{} (0,0) @ADD-TO", codes::SET_EDIT_TARGET_NOT_SET),
        ("?r @SET-SYMBOL (0,0)", codes::KEYWORD_SET_SYMBOL_NOT_A_SYMBOL),
        ("?r @SET-STYLE (0,0)", codes::STYLE_SET_STYLE_NOT_A_STYLE),
    ] {
        let r = read("0.4", r#"["r"]"#, bad, "");
        assert_eq!(errors(&r), [code], "{bad}: {:?}", r.diagnostics());
    }
}

#[test]
fn open_solver_setting_signature_errors_are_checked_once() {
    for (bad, code) in [
        ("@SOLVER-CONFIG(1)", codes::PARAM_POSITIONAL_NOT_ALLOWED),
        ("@SOLVER-CONFIG(custom=1,custom=2)", codes::PARAM_WRITTEN_TWICE),
        ("@SOLVER-CONFIG(custom=[[1]])", codes::SOLVER_CONFIG_VALUE_NOT_LITERAL),
        ("@SOLVER-CONFIG(custom=[[?v]])", codes::SOLVER_CONFIG_VALUE_NOT_LITERAL),
        ("@SOLVER-CONFIG(custom=[?v,[1]])", codes::SOLVER_CONFIG_VALUE_NOT_LITERAL),
    ] {
        for calls in ["", "#R1 @AS $D; #R2 @AS $D"] {
            let r = read("0.4", r#"["r"]"#, &format!("{bad}; (5,0) @CIRCLE(radius=1)"), calls);
            assert_eq!(errors(&r), [code], "{bad}: {:?}", r.diagnostics());
            assert_eq!(shapes(&r).len(), if calls.is_empty() { 0 } else { 2 });
            assert!(r.diagnostics().into_iter().find(|d| d.code == code).unwrap().origin.is_empty());
        }
    }
}

#[test]
fn unknown_solver_keys_and_parameter_dependent_values_stay_open() {
    let body = "@SOLVER-CONFIG(custom=?v); (5,0) @CIRCLE(radius=1)";
    let unused = read("0.4", r#"["r"]"#, body, "");
    assert!(errors(&unused).is_empty(), "{:?}", unused.diagnostics());
    let good = read("0.4", r#"["r"]"#, body, "#R1 @AS(v=[1,2]) $D");
    assert!(errors(&good).is_empty(), "{:?}", good.diagnostics());
    let mixed = read("0.4", r#"["r"]"#, body, "#R1 @AS(v=[[1]]) $D; #R2 @AS(v=[1,2]) $D");
    assert_eq!(errors(&mixed), [codes::SOLVER_CONFIG_VALUE_NOT_LITERAL], "{:?}", mixed.diagnostics());
    assert_eq!(shapes(&mixed).len(), 2);
}

#[test]
fn solver_setting_aliases_share_the_static_duplicate_key_check() {
    let source = r#"* settings
* @RNL(type="header") { @VERSION("0.4"); @ALIAS("horizontal_gap", "spacing_x") }
* @RNL { $D = @DEFINE(roles=[]) { @SOLVER-CONFIG(horizontal_gap=1,spacing_x=2); (5,0) @CIRCLE(radius=1) }; $D @ATTACH => @PLACE(0,0); $D @ATTACH => @PLACE(0,0) }
"#;
    let r = rnl_cli::interpret_file(
        Path::new("settings.sp"),
        None,
        &MemoryProvider::new().with("settings.sp", source),
        &Options { std: false, ..Default::default() },
    )
    .unwrap();
    assert_eq!(errors(&r), [codes::PARAM_WRITTEN_TWICE], "{:?}", r.diagnostics());
    assert_eq!(shapes(&r).len(), 2);
}

#[test]
fn application_definition_positions_are_checked_before_lookup() {
    for (bad, code) in [
        ("@AS", codes::USAGE_DEFINITION_PIPED),
        ("@ATTACH", codes::USAGE_DEFINITION_PIPED),
        ("?r => @AS", codes::USAGE_DEFINITION_PIPED),
        ("?r => @ATTACH", codes::USAGE_DEFINITION_PIPED),
        ("?r @AS (0,0)", codes::USAGE_NOT_A_DEFINITION),
        ("@ATTACH (0,0) ?r", codes::USAGE_NOT_A_DEFINITION),
        ("?r @AS %inner(bad)", codes::USAGE_DEFINITION_HAS_ROLE),
        ("@ATTACH %inner(bad) ?r", codes::USAGE_DEFINITION_HAS_ROLE),
    ] {
        for calls in ["", "#R1 @AS $D; #R2 @AS $D"] {
            let body = format!("%inner = @DEFINE(roles=[\"a\"]) {{}}; {bad}; (5,0) @CIRCLE(radius=1)");
            let r = read("0.4", r#"["r"]"#, &body, calls);
            assert_eq!(errors(&r), [code], "{bad}: {:?}", r.diagnostics());
            assert_eq!(shapes(&r).len(), if calls.is_empty() { 0 } else { 2 });
            assert!(r.diagnostics().into_iter().find(|d| d.code == code).unwrap().origin.is_empty());
        }
    }
}

#[test]
fn application_definition_slots_reject_declared_scene_objects_and_defer_untyped_roles() {
    for body in ["@AS ?r", "@ATTACH ?r"] {
        for (roles, bad) in [(r#"[["r","instance"]]"#, true), (r#"[["r",["point","net"]]]"#, true), (r#"["r"]"#, false)]
        {
            let r = read("0.4", roles, body, "");
            assert_eq!(
                errors(&r),
                if bad { vec![codes::USAGE_NOT_A_DEFINITION] } else { vec![] },
                "{roles} {body}: {:?}",
                r.diagnostics()
            );
        }
    }
}

#[test]
fn literal_selection_and_assertion_conditions_fail_once_at_definition() {
    for (bad, code) in [
        ("# @SELECT-INSTANCE(terminals=-1)", codes::ASSERT_TERMINALS_NOT_WHOLE),
        ("# @SELECT-INSTANCE(terminals=1.5)", codes::ASSERT_TERMINALS_NOT_WHOLE),
        ("?r @ASSERT-INSTANCE(terminals=-1)", codes::ASSERT_TERMINALS_NOT_WHOLE),
        ("?r @ASSERT-INSTANCE(same=\"model\")", codes::ASSERT_LIST_WRITTEN_AS_STRING),
        ("?r @ASSERT-INSTANCE(has=[\"\"])", codes::ASSERT_LIST_ITEM_NOT_NAME),
        ("?r @ASSERT-TERMINAL(same=[\"model\"])", codes::ASSERT_ATTRIBUTE_NOT_OF_KIND),
        ("?r @ASSERT-NET(different=[\"model\"])", codes::ASSERT_ATTRIBUTE_NOT_OF_KIND),
        ("?r @ASSERT-USAGE(same=[\"model\"]) %inner", codes::ASSERT_ATTRIBUTE_NOT_OF_KIND),
        ("?r @ASSERT-NET(same=[?v,\"model\"])", codes::ASSERT_ATTRIBUTE_NOT_OF_KIND),
        ("?r @ASSERT-INSTANCE(has=[?v,\"\"])", codes::ASSERT_LIST_ITEM_NOT_NAME),
        ("?r @ASSERT-INSTANCE(has=[[?v]])", codes::ASSERT_LIST_ITEM_NOT_NAME),
        ("?r @ASSERT-NET(same=[?v,[?v]])", codes::ASSERT_LIST_ITEM_NOT_NAME),
    ] {
        for calls in ["", "#R1 @AS $D; #R2 @AS $D"] {
            let body = format!("%inner = @DEFINE(roles=[]) {{}}; {bad}; (5,0) @CIRCLE(radius=1)");
            let r = read("0.4", r#"["r"]"#, &body, calls);
            assert_eq!(errors(&r), [code], "{bad}: {:?}", r.diagnostics());
            assert_eq!(shapes(&r).len(), if calls.is_empty() { 0 } else { 2 });
            assert!(r.diagnostics().into_iter().find(|d| d.code == code).unwrap().origin.is_empty());
        }
    }
}

#[test]
fn dynamic_selection_and_assertion_conditions_are_checked_per_application() {
    for (body, code) in [
        ("# @SELECT-INSTANCE(terminals=?v)", codes::ASSERT_TERMINALS_NOT_WHOLE),
        ("?r @ASSERT-INSTANCE(terminals=?v)", codes::ASSERT_TERMINALS_NOT_WHOLE),
        ("?r @ASSERT-INSTANCE(has=[?v])", codes::ASSERT_LIST_ITEM_NOT_NAME),
    ] {
        let unused = read("0.4", r#"["r"]"#, body, "");
        assert!(errors(&unused).is_empty(), "{body}: {:?}", unused.diagnostics());
        let (bad, good) = if body.contains("has=") { ("\"\"", "\"model\"") } else { ("-1", "2") };
        let r = read(
            "0.4",
            r#"["r"]"#,
            &format!("{body}; (5,0) @CIRCLE(radius=1)"),
            &format!("#R1 @AS(v={bad}) $D; #R2 @AS(v={good}) $D"),
        );
        assert_eq!(errors(&r), [code], "{body}: {:?}", r.diagnostics());
        assert_eq!(shapes(&r).len(), 2);
    }
}

#[test]
fn assertion_and_selection_slots_check_known_object_kinds() {
    for (bad, code) in [
        ("?r @ASSERT-IN (0,0)", codes::SET_EDIT_TARGET_NOT_SET),
        ("?r @ASSERT-USAGE (0,0)", codes::ASSERT_MEMBER_WRONG_KIND),
        ("?r @SELECT-ANCESTOR-USAGE (0,0)", codes::SELECT_DEFINITION_NOT_DEFINITION),
    ] {
        let r = read("0.4", r#"["r"]"#, bad, "");
        assert_eq!(errors(&r), [code], "{bad}: {:?}", r.diagnostics());
    }
}

#[test]
fn optional_definitions_leave_known_scene_objects_in_the_member_slot() {
    for body in ["?r @ASSERT-USAGE", "?r (0,0) @ASSERT-USAGE", "?r => @ASSERT-USAGE (0,0)"] {
        let r = read("0.4", r#"[["r","instance"]]"#, body, "");
        assert_eq!(errors(&r), [codes::ASSERT_MEMBER_WRONG_KIND], "{body}: {:?}", r.diagnostics());
    }
    for body in ["?r @SELECT-USAGE", "?r (0,0) @SELECT-USAGE", "?r @ASSERT-INSTANCE"] {
        let r = read("0.4", r#"[["r","instance"]]"#, body, "");
        assert!(errors(&r).is_empty(), "{body}: {:?}", r.diagnostics());
    }
    for (kind, body) in [
        ("pin", "?r @ASSERT-NET"),
        ("terminal", "?r @ASSERT-NET"),
        ("pin", "?r @ASSERT-TERMINAL"),
        ("bundle", "?r @ASSERT-INSTANCE"),
    ] {
        let r = read("0.4", &format!("[[\"r\",\"{kind}\"]]"), body, "");
        assert!(errors(&r).is_empty(), "{kind} {body}: {:?}", r.diagnostics());
    }
}

#[test]
fn declared_role_paths_keep_their_kind_at_consuming_slots() {
    for (kind, bad, code) in [
        ("pin", "?r::net @PLACE(1,2)", codes::PLACE_NOT_TAKEN),
        ("terminal", "?r::net @CIRCLE(radius=1)", codes::KEYWORD_GRAPHIC_NOT_POINTS),
        ("instance", "?r::position @ORIENT(\"R90\")", codes::PLACE_ORIENT_NOT_TAKEN),
        ("instance", "{} @ADD-TO ?r::position", codes::SET_EDIT_TARGET_NOT_SET),
        ("instance", "?r::position @ASSERT-INSTANCE", codes::ASSERT_MEMBER_WRONG_KIND),
        ("instance", "?r @AS ?r::position", codes::USAGE_NOT_A_DEFINITION),
        ("instance", "?r @SELECT-ANCESTOR-USAGE ?r::position", codes::SELECT_DEFINITION_NOT_DEFINITION),
        ("instance", "?r:1::net @PLACE(1,2)", codes::PLACE_NOT_TAKEN),
    ] {
        let r = read("0.4", &format!("[[\"r\",\"{kind}\"]]"), bad, "");
        assert_eq!(errors(&r), [code], "{kind} {bad}: {:?}", r.diagnostics());
    }
}

#[test]
fn declared_role_paths_preserve_valid_conversions_and_unknown_subobjects() {
    for (kind, body) in [
        ("pin", "?r::net @ASSERT-NET"),
        ("terminal", "?r::position @CIRCLE(radius=1)"),
        ("instance", "?r::position @PLACE(1,2)"),
        ("instance", "?r:1 @ASSERT-TERMINAL"),
        ("instance", "?r:1::net @ASSERT-NET"),
        ("usage", "?r::local @ORIENT(\"R90\")"),
        ("instance", "?r::named_pin @ORIENT(\"R90\")"),
    ] {
        let r = read("0.4", &format!("[[\"r\",\"{kind}\"]]"), body, "");
        assert!(errors(&r).is_empty(), "{kind} {body}: {:?}", r.diagnostics());
    }
}
