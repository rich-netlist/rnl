//! Per-assignment terminal mappings in language 0.4 (6.4, 11.3).

use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Severity};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};
use std::path::Path;

fn read(path: &str, source: &str) -> Interpretation {
    let files = MemoryProvider::new().with(path, source);
    interpret_file(Path::new(path), None, &files, &Options { std: false, ..Options::solver() }).unwrap()
}

fn spice(version: &str, body: &str) -> Interpretation {
    read("map.sp", &format!("* maps\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\nR2 c d 1k\n* @RNL {{ {body}; #R1 @PLACE(0,0); #R2 @PLACE(10,0) }}\n.end\n"))
}

fn no_errors(r: &Interpretation) {
    let errors: Vec<_> = r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).collect();
    assert!(errors.is_empty(), "{errors:?}");
}

fn nets(r: &Interpretation, id: &str) -> Vec<Vec<String>> {
    r.model.levels[0]
        .objects
        .iter()
        .find(|object| object.id == id)
        .unwrap()
        .pins
        .iter()
        .map(|pin| pin.nets.iter().map(|net| net.trim_start_matches('&').to_string()).collect())
        .collect()
}

const PAIR: &str = "$S = @DEFINE-SYMBOL { (-2,0) @PIN(terminal=\"a\"); (2,0) @PIN(terminal=\"b\") };";

#[test]
fn an_order_maps_each_owner_without_changing_the_reusable_symbol() {
    let r = spice("0.4", &format!("{PAIR} #R1 @SET-SYMBOL(terminals=[\"b\",\"a\"]) $S; #R2 @SET-SYMBOL(terminals=[\"a\",\"b\"]) $S; #R1 @PLACE(0,0); &a @WIRE(mode=\"straight\") {{#R1:1 (5,0)}}"));
    no_errors(&r);
    assert_eq!(nets(&r, "#R1"), [vec!["b"], vec!["a"]]);
    assert_eq!(nets(&r, "#R2"), [vec!["c"], vec!["d"]]);
    assert_eq!(r.model.levels[0].wires[0].points[0], (2.0, 0.0));
    assert_eq!(r.problems[0].wires[0].anchors[0].pin_key(), Some((0, 1)));
}

#[test]
fn pairs_match_by_object_name_and_leave_unlisted_symbol_names_alone() {
    let r = spice("0.4", "$S = @DEFINE-SYMBOL { (-2,0) @PIN(terminal=\"p\"); (2,0) @PIN(terminal=\"extra\"); (4,0) @PIN(terminal=\"n\") }; #R1 @SET-SYMBOL(terminals=[[\"p\",\"extra\"]]) $S");
    no_errors(&r);
    assert_eq!(nets(&r, "#R1"), [vec!["a"], vec!["a"], vec!["b"]]);
}

#[test]
fn a_bus_and_repeated_pins_keep_all_of_their_mapped_bindings() {
    let r = spice("0.4", "$S = @DEFINE-SYMBOL { (-2,0) @PIN(terminal=[\"a\",\"b\"]); (2,0) @PIN(terminal=\"a\") }; #R1 @SET-SYMBOL(terminals=[\"b\",\"a\"]) $S => @PLACE(0,0); &b @WIRE(mode=\"straight\") {#R1:2 (5,0)}");
    no_errors(&r);
    assert_eq!(nets(&r, "#R1"), [vec!["a", "b"], vec!["b"]]);
    assert_eq!(r.problems[0].nets.iter().find(|n| n.name == "b").unwrap().pins.len(), 2);
    assert_eq!(r.model.levels[0].wires[0].points[0], (2.0, 0.0));
}

#[test]
fn a_later_assignment_without_a_map_does_not_keep_the_previous_map() {
    let r = spice("0.4", "$S = @DEFINE-SYMBOL { (-2,0) @PIN(terminal=\"p\"); (2,0) @PIN(terminal=\"n\") }; #R1 @SET-SYMBOL(terminals=[\"n\",\"p\"]) $S; #R1 @SET-SYMBOL $S");
    no_errors(&r);
    assert_eq!(nets(&r, "#R1"), [vec!["a"], vec!["b"]]);
}

#[test]
fn malformed_or_duplicate_maps_leave_a_prior_valid_assignment() {
    for map in [
        "[true]",
        "[0]",
        "[1.5]",
        "[4294967296]",
        "[\"0\"]",
        "[\"\"]",
        "[\"a\",[\"b\",\"c\"]]",
        "[[\"p\"]]",
        "[\"a\",\"a\"]",
        "[[\"p\",\"a\"],[\"p\",\"b\"]]",
        "[[\"p\",\"a\"],[\"n\",\"a\"]]",
        "[[\"p\",\"a\"],[1,\"b\"]]",
        "[1,\"01\"]",
        "[\"a\",\"A\"]",
    ] {
        let r = spice(
            "0.4",
            &format!("{PAIR} #R1 @SET-SYMBOL(terminals=[\"a\",\"b\"]) $S; #R1 @SET-SYMBOL(terminals={map}) $S"),
        );
        assert!(
            r.diagnostics().iter().any(|d| d.code == codes::PARAM_SYMBOL_TERMINALS_INVALID),
            "{map}: {:?}",
            r.diagnostics()
        );
        assert_eq!(nets(&r, "#R1"), [vec!["a"], vec!["b"]], "{map}");
    }
}

#[test]
fn usage_mapping_uses_role_order_and_role_case_in_a_spice_file() {
    let r = spice("0.4", "$D = @DEFINE(roles=[[\"P\",\"net\"],[\"p\",\"net\"]]) {}; $S = @DEFINE-SYMBOL { (-2,0) @PIN(terminal=\"A\"); (2,0) @PIN(terminal=\"a\") }; %u = &a &b @AS $D => @SET-SYMBOL(terminals=[\"a\",\"A\"]) $S => @PLACE(0,0); %v = &a &b @AS $D => @SET-SYMBOL(terminals=[[\"p\",\"A\"],[\"P\",\"a\"]]) $S => @PLACE(10,0)");
    no_errors(&r);
    assert_eq!(nets(&r, "%u"), [vec!["b"], vec!["a"]]);
    assert_eq!(nets(&r, "%v"), [vec!["b"], vec!["a"]]);
}

#[test]
fn block_symbol_maps_apply_to_subcircuit_instances() {
    let r = read("blocks.sp", "* blocks\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n.subckt cell p n\n* @RNL { $S = @DEFINE-SYMBOL { (-2,0) @PIN(terminal=\"a\"); (2,0) @PIN(terminal=\"b\") }; # @SET-SYMBOL(terminals=[\"b\",\"a\"]) $S }\n.ends\nX1 a b cell\n* @RNL { #X1 @PLACE(0,0) }\n.end\n");
    no_errors(&r);
    assert_eq!(nets(&r, "#X1"), [vec!["b"], vec!["a"]]);
}

#[test]
fn quoted_terminal_numbers_match_numbers_in_both_pin_and_map() {
    let r = spice("0.4", "$S = @DEFINE-SYMBOL { (-2,0) @PIN(terminal=\"01\"); (2,0) @PIN(terminal=2) }; #R1 @SET-SYMBOL(terminals=[\"2\",1]) $S");
    no_errors(&r);
    assert_eq!(nets(&r, "#R1"), [vec!["b"], vec!["a"]]);
}

#[test]
fn earlier_files_reject_the_new_parameter_and_keep_their_assignment() {
    for version in ["0.2", "0.3"] {
        let r = spice(version, "$S = @DEFINE-SYMBOL { (-2,0) @PIN(terminal=1); (2,0) @PIN(terminal=2) }; #R1 @SET-SYMBOL $S; #R1 @SET-SYMBOL(terminals=[2,1]) $S");
        assert!(r.diagnostics().iter().any(|d| d.code == codes::PARAM_UNKNOWN));
        assert_eq!(nets(&r, "#R1"), [vec!["a"], vec!["b"]]);
    }
}

#[test]
fn short_and_empty_orders_leave_unlisted_symbol_terminals_unbound() {
    let r = spice("0.4", &format!("{PAIR} #R1 @SET-SYMBOL(terminals=[\"b\"]) $S; #R2 @SET-SYMBOL(terminals=[]) $S"));
    no_errors(&r);
    assert_eq!(nets(&r, "#R1"), [Vec::<String>::new(), vec!["a".to_string()]]);
    assert_eq!(nets(&r, "#R2"), [Vec::<String>::new(), Vec::<String>::new()]);
}

#[test]
fn a_direct_mapping_wins_over_a_later_selection_mapping() {
    let r = spice("0.4", &format!("{PAIR} #R1 @SET-SYMBOL(terminals=[\"b\",\"a\"]) $S; # @SELECT-INSTANCE => @SET-SYMBOL(terminals=[\"a\",\"b\"]) $S"));
    no_errors(&r);
    assert_eq!(nets(&r, "#R1"), [vec!["b"], vec!["a"]]);
    assert_eq!(nets(&r, "#R2"), [vec!["c"], vec!["d"]]);
}

#[test]
fn case_distinct_symbol_names_map_separately_in_spectre() {
    let r = read("map.scs", "simulator lang=spectre\n// @RNL(type=\"header\") { @VERSION(\"0.4\") }\nI0 (upper lower) resistor r=1k\n// @RNL { $S = @DEFINE-SYMBOL { (-2,0) @PIN(terminal=\"P\"); (2,0) @PIN(terminal=\"p\") }; #I0 @SET-SYMBOL(terminals=[\"p\",\"P\"]) $S => @PLACE(0,0) }\n");
    no_errors(&r);
    assert_eq!(nets(&r, "#I0"), [vec!["lower"], vec!["upper"]]);
}

#[test]
fn invalid_quoted_terminal_numbers_fail_only_the_pin_that_writes_them() {
    for value in ["0", "4294967296"] {
        let r = spice("0.4", &format!("$S = @DEFINE-SYMBOL {{ (0,0) @PIN(terminal=\"{value}\"); (2,0) @PIN(terminal=1) }}; #R1 @SET-SYMBOL $S"));
        assert!(r.diagnostics().iter().any(|d| d.code == codes::PARAM_PIN_TERMINAL_NUMBER));
        assert_eq!(nets(&r, "#R1"), [vec!["a"]]);
    }
}

#[test]
fn generation_receives_its_mapping_in_the_second_symbol_queue() {
    let r = spice("0.4", "$D = @DEFINE(roles=[[\"node\",\"net\"]]) {}; $S = @DEFINE-SYMBOL { (0,0) @PIN(terminal=\"wire\") }; # @SELECT-USAGE $D => @SET-SYMBOL(terminals=[\"wire\"]) $S; # @SELECT-NET(name=\"a\") => @AS $D => @PLACE(0,5)");
    no_errors(&r);
    let usage = r.model.levels[0].items.iter().find(|item| item.kind == "usage").unwrap();
    assert_eq!(usage.pin_nets, [vec!["a"]]);
}

#[test]
fn scoped_mapping_uses_the_version_of_the_source_statement() {
    for (outer, inner) in [("0.4", "0.3"), ("0.3", "0.4")] {
        let files = MemoryProvider::new()
            .with("outer.sp", &format!("* outer\n* @RNL(type=\"header\") {{ @VERSION(\"{outer}\") }}\n.include \"inner.sp\"\n* @RNL(scope=\"child\") {{ $S = @DEFINE-SYMBOL {{ (-2,0) @PIN(terminal=\"a\"); (2,0) @PIN(terminal=\"b\") }}; #R1 @SET-SYMBOL(terminals=[\"b\",\"a\"]) $S; #R1 @PLACE(0,0) }}\n.end\n"))
            .with("inner.sp", &format!("* inner\n* @RNL(type=\"header\") {{ @VERSION(\"{inner}\") }}\n.subckt child a b\nR1 a b 1k\n.ends\n"));
        let r =
            interpret_file(Path::new("outer.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap();
        let child = r.model.levels.iter().find(|lv| lv.items.iter().any(|item| item.id == "#R1")).unwrap();
        if outer == "0.4" {
            no_errors(&r);
            assert_eq!(child.items[0].pin_nets, [vec!["b"], vec!["a"]]);
        } else {
            assert!(r.diagnostics().iter().any(|d| d.code == codes::PARAM_UNKNOWN));
            assert_eq!(child.items[0].pin_nets, [vec!["a"], vec!["b"]]);
        }
    }
}

#[test]
fn a_definition_parameter_can_supply_a_map_or_leave_it_unwritten() {
    let r = spice("0.4", "$S = @DEFINE-SYMBOL { (-2,0) @PIN(terminal=\"p\"); (2,0) @PIN(terminal=\"n\") }; $D = @DEFINE(roles=[[\"part\",\"instance\"]], parameters=[\"map\"]) { ?part @SET-SYMBOL(terminals=?map) $S }; #R1 @AS(map=[\"n\",\"p\"]) $D; #R2 @AS $D");
    no_errors(&r);
    assert_eq!(nets(&r, "#R1"), [vec!["b"], vec!["a"]]);
    assert_eq!(nets(&r, "#R2"), [vec!["c"], vec!["d"]]);
}

#[test]
fn a_mapping_failure_for_one_set_member_does_not_stop_later_members() {
    let r = spice("0.4", "$OLD = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1) }; #R1 @SET-SYMBOL $OLD; $D = @DEFINE(roles=[[\"P\",\"net\"],[\"p\",\"net\"]]) {}; %u = &a &b @AS $D; $S = @DEFINE-SYMBOL { (-2,0) @PIN(terminal=\"A\"); (2,0) @PIN(terminal=\"a\") }; %targets = {#R1 %u}; %mapped = %targets @SET-SYMBOL(terminals=[\"A\",\"a\"]) $S => @PLACE(2,3)");
    assert_eq!(r.diagnostics().iter().filter(|d| d.code == codes::PARAM_SYMBOL_TERMINALS_INVALID).count(), 1);
    assert_eq!(nets(&r, "#R1"), [vec!["a"]]);
    assert_eq!(nets(&r, "%u"), [vec!["a"], vec!["b"]]);
    assert_eq!(r.model.levels[0].items.iter().find(|item| item.id == "%u").unwrap().at, (2.0, 3.0));
}
