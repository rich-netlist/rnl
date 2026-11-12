//! LAYER validates drawable targets and fixed body errors before applications.
use rnl_runtime::{
    diag::{codes, Severity},
    source::MemoryProvider,
    Interpretation, Options,
};
use std::path::Path;
fn read(version: &str, body: &str) -> Interpretation {
    let source =
        format!("title\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\n* @RNL {{ {body} }}\n");
    rnl_cli::interpret_file(
        Path::new("layer.sp"),
        None,
        &MemoryProvider::new().with("layer.sp", &source),
        &Options { std: false, ..Default::default() },
    )
    .unwrap()
}
fn layer_errors(r: &Interpretation) -> usize {
    r.diagnostics().iter().filter(|d| d.severity == Severity::Error && d.code == codes::LAYER_NOT_TAKEN).count()
}
#[test]
fn definitions_positions_and_pins_do_not_silently_accept_layers() {
    for body in [
        "$S = @DEFINE-SYMBOL {}; $S @LAYER(\"top\")",
        "$S = @DEFINE-STYLE {}; $S @LAYER(\"top\")",
        "$D = @DEFINE(roles=[]) {}; $D @LAYER(\"top\")",
        "#R1::position @LAYER(\"top\")",
        "$S = @DEFINE-SYMBOL { %p=(0,0) @PIN(terminal=1) }; #R1 @SET-SYMBOL $S; #R1::p @LAYER(\"top\")",
    ] {
        let r = read("0.4", body);
        assert_eq!(layer_errors(&r), 1, "{body}: {:?}", r.diagnostics());
    }
}
#[test]
fn fixed_layer_errors_are_reported_once_in_unused_or_repeated_bodies() {
    for body in ["(0,0) @LAYER(\"top\")", "%p=(0,0); %p @LAYER(\"top\")"] {
        for calls in ["", "$D @ATTACH; $D @ATTACH"] {
            let r = read("0.4", &format!("$D = @DEFINE(roles=[]) {{ {body} }}; {calls}"));
            assert_eq!(layer_errors(&r), 1, "{body}, {calls}: {:?}", r.diagnostics());
            let error = r.diagnostics().into_iter().find(|d| d.code == codes::LAYER_NOT_TAKEN).unwrap();
            assert!(error.origin.is_empty());
        }
    }
}
#[test]
fn declared_role_kinds_are_checked_without_applying_the_definition() {
    for kind in ["point", "net", "pin", "terminal"] {
        let r = read("0.4", &format!("$D = @DEFINE(roles=[[\"r\",\"{kind}\"]]) {{ ?r @LAYER(\"top\") }}"));
        assert_eq!(layer_errors(&r), 1, "{kind}: {:?}", r.diagnostics());
    }
}
#[test]
fn graphics_text_and_symbol_less_usages_keep_their_valid_layer_behavior() {
    let r=read("0.4","(0,0) @CIRCLE(radius=1) => @LAYER(\"bottom\"); (0,0) @TEXT(\"label\") => @LAYER(\"top\"); $D = @DEFINE(roles=[]) {}; $D @ATTACH => @LAYER(\"top\")");
    assert_eq!(layer_errors(&r), 0, "{:?}", r.diagnostics());
    assert_eq!(r.model.levels[0].shapes.len(), 1);
}
#[test]
fn released_definition_and_position_targets_remain_unchanged() {
    for version in ["0.2", "0.3"] {
        let r = read(version, "$S = @DEFINE-SYMBOL {}; $S @LAYER(\"top\"); #R1::position @LAYER(\"top\")");
        assert_eq!(layer_errors(&r), 0, "{:?}", r.diagnostics());
    }
}

#[test]
fn fixed_local_aliases_and_forward_references_report_at_the_definition() {
    for body in [
        "%p @LAYER(\"top\"); %p=(0,0)",
        "%q=%p; %p=(0,0); %q @LAYER(\"top\")",
        "%p=(0,0) => @LAYER(\"top\")",
        "%p=(0,0) => @PLACE(x=2) => @LAYER(\"top\")",
    ] {
        let r = read("0.4", &format!("$D = @DEFINE(roles=[]) {{ {body} }}"));
        assert_eq!(layer_errors(&r), 1, "{body}: {:?}", r.diagnostics());
    }
}

#[test]
fn a_fixed_layer_error_omits_its_whole_statement_but_not_siblings() {
    let r=read("0.4","$D = @DEFINE(roles=[]) { (0,0) @LAYER(\"top\") => @CIRCLE(radius=1); (2,0) @CIRCLE(radius=1) }; $D @ATTACH => @PLACE(0,0)");
    assert_eq!(layer_errors(&r), 1, "{:?}", r.diagnostics());
    assert_eq!(r.model.levels[0].shapes.len(), 1);
}

#[test]
fn local_graphics_and_texts_are_valid_even_when_reached_forwards() {
    for body in [
        "%g @LAYER(\"top\"); %g=(0,0) @CIRCLE(radius=1)",
        "%t=(0,0) @TEXT(\"label\"); %t @LAYER(\"top\")",
        "%g=(0,0) @CIRCLE(radius=1); {%p=(1,0); %g @LAYER(\"top\")}",
    ] {
        let r = read("0.4", &format!("$D = @DEFINE(roles=[]) {{ {body} }}; $D @ATTACH => @PLACE(0,0)"));
        assert_eq!(layer_errors(&r), 0, "{body}: {:?}", r.diagnostics());
    }
}

#[test]
fn untyped_roles_still_check_each_actual_application() {
    let r = read("0.4", "$D = @DEFINE(roles=[\"r\"]) { ?r @LAYER(\"top\") }; #R1 @AS $D; (0,0) @AS $D; (1,0) @AS $D");
    assert_eq!(layer_errors(&r), 2, "{:?}", r.diagnostics());
    assert!(r.diagnostics().into_iter().filter(|d| d.code == codes::LAYER_NOT_TAKEN).all(|d| !d.origin.is_empty()));
}

#[test]
fn fixed_external_and_local_definitions_cannot_take_layers() {
    for body in [
        "$S = @DEFINE-SYMBOL {}; $D = @DEFINE(roles=[]) { $S @LAYER(\"top\") }",
        "$D = @DEFINE(roles=[]) { %S = @DEFINE-SYMBOL {}; %S @LAYER(\"top\") }",
        "$D = @DEFINE(roles=[]) { %S = @DEFINE-STYLE {}; %S @LAYER(\"top\") }",
    ] {
        let r = read("0.4", body);
        assert_eq!(layer_errors(&r), 1, "{body}: {:?}", r.diagnostics());
    }
}

#[test]
fn a_nested_set_local_name_does_not_use_the_outer_points_kind() {
    let r=read("0.4","$D = @DEFINE(roles=[]) { %p=(0,0); {%p=(0,0) @CIRCLE(radius=1); %p @LAYER(\"top\")} }; $D @ATTACH => @PLACE(0,0)");
    assert_eq!(layer_errors(&r), 0, "{:?}", r.diagnostics());
}

#[test]
fn multiple_argument_dependent_bindings_are_not_given_a_guessed_kind() {
    let r = read("0.4", "$D = @DEFINE(roles=[\"a\",\"b\"]) { %p=?a; %p=?b; %p @LAYER(\"top\") }");
    assert_eq!(layer_errors(&r), 0, "{:?}", r.diagnostics());
}

#[test]
fn the_definitions_source_version_controls_unused_body_checks() {
    for (main, child, expected) in [("0.4", "0.3", 0), ("0.3", "0.4", 1)] {
        let top = format!("title\n* @RNL(type=\"header\") {{ @VERSION(\"{main}\") }}\n.include child.inc\n");
        let child=format!("title\n* @RNL(type=\"header\") {{ @VERSION(\"{child}\") }}\n* @RNL {{ $D = @DEFINE(roles=[]) {{ (0,0) @LAYER(\"top\") }} }}\n");
        let r = rnl_cli::interpret_file(
            Path::new("top.sp"),
            None,
            &MemoryProvider::new().with("top.sp", &top).with("child.inc", &child),
            &Options { std: false, ..Default::default() },
        )
        .unwrap();
        assert_eq!(layer_errors(&r), expected, "{:?}", r.diagnostics());
    }
}
