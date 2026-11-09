//! Pin names must remain reachable before text keys in language 0.4 (6.5).

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Code, Severity};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn read_files(files: &MemoryProvider) -> Interpretation {
    interpret_file(Path::new("names.sp"), None, files, &Options { std: false, ..Options::solver() }).unwrap()
}

fn read(version: &str, header: &str, body: &str) -> Interpretation {
    read_files(&MemoryProvider::new().with("names.sp", &format!("* pin names\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\"); {header} }}\nR1 a b 1k\n* @RNL {{ {body}; #R1 @PLACE(0,0) }}\n.end\n")))
}

fn count(r: &Interpretation, code: Code) -> usize {
    r.diagnostics().iter().filter(|d| d.code == code).count()
}

fn no_errors(r: &Interpretation) {
    let errors: Vec<_> = r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).collect();
    assert!(errors.is_empty(), "{errors:?}");
}

fn pins(r: &Interpretation, name: &str) -> Vec<(f64, f64)> {
    let level = &r.model.levels[0];
    let object = level.objects.iter().find(|o| o.id == name).unwrap();
    let item = level.items.iter().find(|i| i.object_id == object.id).unwrap();
    level.symbols[item.symbol].pins.iter().map(|p| p.pos).collect()
}

const GOOD: &str = "$GOOD = @DEFINE-SYMBOL { (-2,0) @PIN(terminal=1) };";

#[test]
fn standard_names_fail_the_assignment_and_keep_an_earlier_valid_symbol() {
    for name in ["position", "net"] {
        let r = read("0.4", "", &format!("{GOOD} $BAD = @DEFINE-SYMBOL {{ %{name} = (3,0) @PIN(terminal=1) }}; #R1 @SET-SYMBOL $GOOD; #R1 @SET-SYMBOL $BAD"));
        assert_eq!(count(&r, codes::PIN_NAME_IS_SUBOBJECT), 1, "{:?}", r.diagnostics());
        assert_eq!(pins(&r, "#R1"), [(-2.0, 0.0)]);
    }
}

#[test]
fn an_unused_symbol_may_contain_names_that_are_invalid_on_assignment() {
    let r = read("0.4", "", "$S = @DEFINE-SYMBOL { %position = (3,0) @PIN(terminal=1); @LINE {%position (0,0)} }");
    no_errors(&r);
}

#[test]
fn standard_aliases_use_the_file_of_the_assignment() {
    let r = read("0.4", "@ALIAS(\"origin\",\"position\")", &format!("{GOOD} $BAD = @DEFINE-SYMBOL {{ %origin = (3,0) @PIN(terminal=1) }}; #R1 @SET-SYMBOL $GOOD; #R1 @SET-SYMBOL $BAD"));
    assert_eq!(count(&r, codes::PIN_NAME_IS_SUBOBJECT), 1, "{:?}", r.diagnostics());
    assert_eq!(pins(&r, "#R1"), [(-2.0, 0.0)]);
    let r = read("0.4", "", "$S = @DEFINE-SYMBOL { %origin = (3,0) @PIN(terminal=1) }; #R1 @SET-SYMBOL $S");
    no_errors(&r);
}

#[test]
fn usage_roles_locals_and_definition_aliases_precede_pin_names() {
    for name in ["p", "inside", "role_alias", "local_alias"] {
        let r = read("0.4", "@ALIAS(\"role_alias\",\"p\",definition=\"$D\"); @ALIAS(\"local_alias\",\"inside\",definition=\"$D\")", &format!("$D = @DEFINE(roles=[\"p\"]) {{ %inside = (0,0) }}; $GOOD = @DEFINE-SYMBOL {{ (-2,0) @PIN(terminal=\"p\") }}; $BAD = @DEFINE-SYMBOL {{ %{name} = (3,0) @PIN(terminal=\"p\") }}; %u = &a @AS $D => @PLACE(0,0); %u @SET-SYMBOL $GOOD; %u @SET-SYMBOL $BAD"));
        assert_eq!(count(&r, codes::PIN_NAME_IS_USAGE_NAME), 1, "{name}: {:?}", r.diagnostics());
        assert_eq!(pins(&r, "%u"), [(-2.0, 0.0)]);
    }
}

#[test]
fn local_names_written_later_in_the_body_are_already_reserved() {
    let r = read("0.4", "", "$S = @DEFINE-SYMBOL { %inside = (3,0) @PIN(terminal=\"p\") }; $D = @DEFINE(roles=[\"p\"]) { ? @SET-SYMBOL $S; %inside = (0,0) }; %u = &a @AS $D => @PLACE(0,0)");
    assert_eq!(count(&r, codes::PIN_NAME_IS_USAGE_NAME), 1, "{:?}", r.diagnostics());
    assert!(!r.model.levels[0].items.iter().any(|i| i.object_id == "%u"));
}

#[test]
fn parameter_names_and_case_distinct_names_are_not_subobject_clashes() {
    let r = read("0.4", "", "$D = @DEFINE(roles=[\"p\"],parameters=[\"gain\"]) {}; $S = @DEFINE-SYMBOL { %gain = (-2,0) @PIN(terminal=\"p\"); %P = (2,0) @PIN(terminal=\"p\"); %Position = (4,0) @PIN(terminal=\"p\") }; %u = &a @AS(gain=1) $D => @SET-SYMBOL $S => @PLACE(0,0)");
    no_errors(&r);
    assert_eq!(pins(&r, "%u").len(), 3);
}

#[test]
fn every_alias_and_included_pin_name_is_checked() {
    let r = read("0.4", "", &format!("{GOOD} $PART = @DEFINE-SYMBOL {{ %p = (3,0) @PIN(terminal=1) => %net }}; $BAD = @DEFINE-SYMBOL {{ $PART }}; #R1 @SET-SYMBOL $GOOD; #R1 @SET-SYMBOL $BAD"));
    assert_eq!(count(&r, codes::PIN_NAME_IS_SUBOBJECT), 1);
    assert_eq!(pins(&r, "#R1"), [(-2.0, 0.0)]);
}

#[test]
fn a_failed_member_keeps_its_previous_symbol_and_other_members_continue() {
    let r = read("0.4", "", "$D = @DEFINE(roles=[\"p\"]) {}; $S = @DEFINE-SYMBOL { %p = (3,0) @PIN(terminal=\"p\") }; $GOOD = @DEFINE-SYMBOL { (-2,0) @PIN(terminal=\"p\") }; %u = &a @AS $D => @SET-SYMBOL $GOOD => @PLACE(0,0); {%u #R1} @SET-SYMBOL $S");
    assert_eq!(count(&r, codes::PIN_NAME_IS_USAGE_NAME), 1);
    assert_eq!(pins(&r, "%u"), [(-2.0, 0.0)]);
    assert_eq!(pins(&r, "#R1"), [(3.0, 0.0)]);
}

#[test]
fn generated_usages_check_names_in_the_second_symbol_queue() {
    let r = read("0.4", "", "$S = @DEFINE-SYMBOL { %p = (3,0) @PIN(terminal=\"p\") }; $GOOD = @DEFINE-SYMBOL { (-2,0) @PIN(terminal=\"p\") }; $D = @DEFINE(roles=[\"p\"]) { ? @SET-SYMBOL $GOOD; ? @SET-SYMBOL $S }; # @SELECT-NET(name=\"a\") => @AS $D => @PLACE(0,0)");
    assert_eq!(count(&r, codes::PIN_NAME_IS_USAGE_NAME), 1, "{:?}", r.diagnostics());
    assert!(r.model.levels[0]
        .items
        .iter()
        .any(|item| r.model.levels[0].symbols[item.symbol].pins.iter().any(|p| p.pos == (-2.0, 0.0))));
}

#[test]
fn symbol_text_keys_fail_after_all_pin_names_are_known() {
    for body in [
        "? @TEXT(\"bad\",key=\"p\"); %p = (3,0) @PIN(terminal=1)",
        "%p = (3,0) @PIN(terminal=1); ? @TEXT(\"bad\",key=\"p\")",
    ] {
        let r = read(
            "0.4",
            "",
            &format!("$S = @DEFINE-SYMBOL {{ {body}; ? @TEXT(\"good\",key=\"ok\") }}; #R1 @SET-SYMBOL $S"),
        );
        assert_eq!(count(&r, codes::PIN_NAME_IS_TEXT_KEY_SYMBOL), 1, "{:?}", r.diagnostics());
        assert_eq!(r.model.levels[0].texts.len(), 1);
        assert_eq!(pins(&r, "#R1"), [(3.0, 0.0)]);
    }
}

#[test]
fn scene_texts_check_the_effective_symbol_in_either_written_order() {
    for body in ["#R1 @TEXT(\"bad\",key=\"p\"); #R1 @SET-SYMBOL $S", "#R1 @SET-SYMBOL $S; #R1 @TEXT(\"bad\",key=\"p\")"]
    {
        let r = read(
            "0.4",
            "",
            &format!("$S = @DEFINE-SYMBOL {{ %p = (3,0) @PIN(terminal=1) }}; {body}; #R1 @TEXT(\"good\",key=\"ok\")"),
        );
        assert_eq!(count(&r, codes::PIN_NAME_IS_TEXT_KEY_SCENE), 1, "{:?}", r.diagnostics());
        assert_eq!(r.model.levels[0].texts.len(), 1);
        assert_eq!(pins(&r, "#R1"), [(3.0, 0.0)]);
    }
}

#[test]
fn a_losing_symbol_does_not_reserve_its_pin_names() {
    let r = read("0.4", "", &format!("{GOOD} $S = @DEFINE-SYMBOL {{ %p = (3,0) @PIN(terminal=1) }}; #R1 @SET-SYMBOL $S; #R1 @SET-SYMBOL $GOOD; #R1 @TEXT(\"kept\",key=\"p\")"));
    no_errors(&r);
    assert_eq!(r.model.levels[0].texts.len(), 1);
}

#[test]
fn pin_labels_and_case_distinct_keys_are_not_pin_names() {
    for key in ["p", "Q"] {
        let r = read("0.4", "", &format!("$S = @DEFINE-SYMBOL {{ %q = (3,0) @PIN(terminal=1,label=\"p\") }}; #R1 @SET-SYMBOL $S; #R1 @TEXT(\"kept\",key=\"{key}\")"));
        no_errors(&r);
        assert_eq!(r.model.levels[0].texts.len(), 1);
    }
}

#[test]
fn old_files_keep_pin_local_names_as_points_and_keep_their_texts() {
    for version in ["0.2", "0.3"] {
        let r = read(version, "", "$S = @DEFINE-SYMBOL { %position = (-2,0) @PIN(terminal=1); %p = (2,0) @PIN(terminal=2); ? @TEXT(\"old\",key=\"p\") }; #R1 @SET-SYMBOL $S; #R1 @TEXT(\"still valid\",key=\"p\")");
        no_errors(&r);
        assert_eq!(r.model.levels[0].texts.len(), 1);
        assert_eq!(pins(&r, "#R1"), [(-2.0, 0.0), (2.0, 0.0)]);
    }
}

#[test]
fn mixed_version_text_layers_keep_the_old_layer_in_either_source_order() {
    for (outer, inner) in [("0.4", "0.3"), ("0.3", "0.4")] {
        let symbol = "$S = @DEFINE-SYMBOL { %p = (3,0) @PIN(terminal=1) }; #R1 @SET-SYMBOL $S;";
        let (outer_body, inner_body) = if outer == "0.4" {
            (format!("{symbol} #R1 @TEXT(\"bad\",key=\"p\")"), "#R1 @TEXT(\"old\",key=\"p\")".to_string())
        } else {
            ("#R1 @TEXT(\"old\",key=\"p\")".to_string(), format!("{symbol} #R1 @TEXT(\"bad\",key=\"p\")"))
        };
        let files = MemoryProvider::new()
            .with("names.sp", &format!("* outer\n* @RNL(type=\"header\") {{ @VERSION(\"{outer}\") }}\n.include \"inner.sp\"\n* @RNL(scope=\"child\") {{ {outer_body}; #R1 @PLACE(0,0) }}\n.end\n"))
            .with("inner.sp", &format!("* inner\n* @RNL(type=\"header\") {{ @VERSION(\"{inner}\") }}\n.subckt child a b\nR1 a b 1k\n* @RNL {{ {inner_body} }}\n.ends\n"));
        let r = read_files(&files);
        assert_eq!(count(&r, codes::PIN_NAME_IS_TEXT_KEY_SCENE), 1, "{:?}", r.diagnostics());
        let texts: Vec<_> = r.model.levels.iter().flat_map(|level| &level.texts).collect();
        assert_eq!(texts.len(), 1, "{texts:?}");
        assert_eq!(texts[0].markup, "old");
    }
}

#[test]
fn block_symbol_name_failure_keeps_the_earlier_block_assignment() {
    let files = MemoryProvider::new().with("names.sp", "* blocks\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n.subckt child p n\n* @RNL { $GOOD = @DEFINE-SYMBOL { (-2,0) @PIN(terminal=1) }; $BAD = @DEFINE-SYMBOL { %position = (3,0) @PIN(terminal=1) }; # @SET-SYMBOL $GOOD; # @SET-SYMBOL $BAD }\n.ends\nX1 a b child\n* @RNL { #X1 @PLACE(0,0) }\n.end\n");
    let r = read_files(&files);
    assert_eq!(count(&r, codes::PIN_NAME_IS_SUBOBJECT), 1, "{:?}", r.diagnostics());
    assert_eq!(pins(&r, "#X1"), [(-2.0, 0.0)]);
}

#[test]
fn drawing_usages_inside_symbols_validate_their_pin_names() {
    let r = read("0.4", "", "$D = @DEFINE(roles=[[\"anchor\",\"point\"]]) {}; $GOOD = @DEFINE-SYMBOL { (0,0) @CIRCLE(radius=1) }; $BAD = @DEFINE-SYMBOL { %anchor = (3,0) @PIN }; $OUTER = @DEFINE-SYMBOL { %u = (0,0) @AS $D => @SET-SYMBOL $GOOD; %u @SET-SYMBOL $BAD }; #R1 @SET-SYMBOL $OUTER");
    assert_eq!(count(&r, codes::PIN_NAME_IS_USAGE_NAME), 1, "{:?}", r.diagnostics());
    assert!(r.model.levels[0].symbols.iter().any(|s| !s.prims.is_empty()));
}

#[test]
fn generated_text_waits_for_the_generated_owners_symbol() {
    let r = read("0.4", "", "$D = @DEFINE(roles=[\"p\"]) { ? @TEXT(\"bad\",key=\"q\") }; $S = @DEFINE-SYMBOL { %q = (3,0) @PIN(terminal=\"p\") }; # @SELECT-NET(name=\"a\") => @AS $D => @PLACE(0,0); # @SELECT-USAGE $D => @SET-SYMBOL $S");
    assert_eq!(count(&r, codes::PIN_NAME_IS_TEXT_KEY_SCENE), 1, "{:?}", r.diagnostics());
    assert!(
        r.model.levels[0].texts.iter().all(|text| text.key.as_deref() != Some("q")),
        "{:?}",
        r.model.levels[0].texts
    );
    assert!(r.model.levels[0].items.iter().any(|item| item.kind == "usage"));
}

#[test]
fn a_rejected_assignment_still_passes_and_names_its_input_handle() {
    let r = read("0.4", "", &format!("{GOOD} $BAD = @DEFINE-SYMBOL {{ %position = (3,0) @PIN(terminal=1) }}; #R1 @SET-SYMBOL $GOOD; %saved = #R1 @SET-SYMBOL $BAD => @ORIENT(\"R90\"); %saved @TEXT(\"still valid\",key=\"ok\")"));
    let errors: Vec<_> = r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).collect();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].code, codes::PIN_NAME_IS_SUBOBJECT);
    assert_eq!(r.model.levels[0].items[0].orient, rnl_runtime::geom::Orient::R90);
    assert!(r.model.levels[0].texts.iter().any(|text| text.markup == "still valid"));
}

#[test]
fn a_rejected_member_does_not_change_the_set_handle_or_its_members() {
    let r = read("0.4", "", "$D = @DEFINE(roles=[\"p\"]) {}; $S = @DEFINE-SYMBOL { %p = (3,0) @PIN(terminal=\"p\") }; $GOOD = @DEFINE-SYMBOL { (-2,0) @PIN(terminal=\"p\") }; %u = &a @AS $D => @SET-SYMBOL $GOOD => @PLACE(0,0); %targets = {%u #R1}; %out = %targets @SET-SYMBOL $S; %targets = %out; %out @ORIENT(\"R90\")");
    let errors: Vec<_> = r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).collect();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].code, codes::PIN_NAME_IS_USAGE_NAME);
    assert!(r.model.levels[0].items.iter().all(|item| item.orient == rnl_runtime::geom::Orient::R90));
    assert_eq!(pins(&r, "%u"), [(-2.0, 0.0)]);
    assert_eq!(pins(&r, "#R1"), [(3.0, 0.0)]);
}
