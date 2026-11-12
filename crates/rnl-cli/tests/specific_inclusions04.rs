//! Inclusion cycles discard only the closing edge, independently for each root (9.1).

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_render::{primitives, Kind, RenderOptions};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn read(version: &str, body: &str) -> Interpretation {
    let text = format!("* inclusions\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\nR2 a b 2k\n* @RNL {{ {body}; #R1 @PLACE(0,0); #R2 @PLACE(8,0) }}\n");
    let provider = MemoryProvider::new().with("inclusions.sp", &text);
    interpret_file(Path::new("inclusions.sp"), None, &provider, &Options { std: false, ..Options::solver() }).unwrap()
}

fn radii(result: &Interpretation, owner: &str) -> Vec<f64> {
    primitives(&result.model.levels[0], &RenderOptions::default())
        .iter()
        .filter(|primitive| primitive.owner == owner)
        .filter_map(|primitive| match primitive.kind {
            Kind::Circle { r, .. } => Some(r),
            _ => None,
        })
        .collect()
}

#[test]
fn each_symbol_is_expanded_from_itself_in_either_declaration_order() {
    let a = "$A = @DEFINE-SYMBOL { $B; (0,0) @CIRCLE(radius=1) }";
    let b = "$B = @DEFINE-SYMBOL { $A; (0,0) @CIRCLE(radius=2) }";
    for declarations in [format!("{a}; {b}"), format!("{b}; {a}")] {
        let result = read("0.4", &format!("{declarations}; #R1 @SET-SYMBOL $A; #R2 @SET-SYMBOL $B"));
        assert_eq!(radii(&result, "#R1"), [2.0, 1.0]);
        assert_eq!(radii(&result, "#R2"), [1.0, 2.0]);
        let diagnostics = result.diagnostics();
        let messages: Vec<_> = diagnostics.iter().map(|diagnostic| result.sink.render(diagnostic)).collect();
        assert!(messages.iter().any(|message| message.contains("$A -> $B -> $A")), "{messages:?}");
        assert!(messages.iter().any(|message| message.contains("$B -> $A -> $B")), "{messages:?}");
        assert!(!messages.iter().any(|message| message.contains("ref-name-cycle")), "{messages:?}");
    }
}

#[test]
fn released_symbol_cycle_keeps_its_published_result() {
    let result = read("0.3", "$A = @DEFINE-SYMBOL { $B; (0,0) @CIRCLE(radius=1) }; $B = @DEFINE-SYMBOL { $A; (0,0) @CIRCLE(radius=2) }; #R1 @SET-SYMBOL $A; #R2 @SET-SYMBOL $B");
    assert_eq!(radii(&result, "#R1"), [2.0, 1.0]);
    assert_eq!(radii(&result, "#R2"), [2.0]);
}

#[test]
fn style_cycles_merge_properties_independently_and_keep_local_overrides() {
    let a = "$A = @DEFINE-STYLE(linecolor=\"#ff0000\") { $B }";
    let b = "$B = @DEFINE-STYLE(linewidth=0.3) { $A }";
    for declarations in [format!("{a}; {b}"), format!("{b}; {a}")] {
        let result = read("0.4", &format!("{declarations}; #R1 @SET-STYLE $A; #R2 @SET-STYLE $B"));
        for item in &result.model.levels[0].items {
            assert_eq!(item.style.get("linecolor").and_then(|value| value.str()), Some("#ff0000"));
            assert_eq!(item.style.get("linewidth").and_then(|value| value.num()), Some(0.3));
        }
        assert!(result.diagnostics().iter().all(|d| d.code != rnl_runtime::diag::codes::REF_NAME_CYCLE));
    }
    let result = read("0.4", "$A = @DEFINE-STYLE(linewidth=0.2) { $B }; $B = @DEFINE-STYLE(linewidth=0.3) { $A }; #R1 @SET-STYLE $A; #R2 @SET-STYLE $B");
    for (name, width) in [("#R1", 0.2), ("#R2", 0.3)] {
        let item = result.model.levels[0].items.iter().find(|item| item.id == name).unwrap();
        assert_eq!(item.style.get("linewidth").and_then(|value| value.num()), Some(width));
    }
}

#[test]
fn aliases_and_forward_first_use_do_not_cache_partial_bodies() {
    let result = read("0.4", "$ALIAS = $A; #R1 @SET-SYMBOL $ALIAS; #R2 @SET-SYMBOL $B; $A = @DEFINE-SYMBOL { $B; (0,0) @CIRCLE(radius=1) }; $B = @DEFINE-SYMBOL { $ALIAS; (0,0) @CIRCLE(radius=2) }");
    assert_eq!(radii(&result, "#R1"), [2.0, 1.0]);
    assert_eq!(radii(&result, "#R2"), [1.0, 2.0]);
}

#[test]
fn cycles_through_drawn_usages_preserve_the_rest_of_both_symbols() {
    let result = read("0.4", "$D = @DEFINE(roles=[[\"p\",\"point\"]]) {}; $A = @DEFINE-SYMBOL { $D @ATTACH (1,0) => @SET-SYMBOL $B; (0,0) @CIRCLE(radius=1) }; $B = @DEFINE-SYMBOL { $A; (0,0) @CIRCLE(radius=2) }; #R1 @SET-SYMBOL $A; #R2 @SET-SYMBOL $B");
    assert_eq!(radii(&result, "#R1"), [2.0, 1.0]);
    assert_eq!(radii(&result, "#R2"), [1.0, 2.0]);
    let messages: Vec<_> = result.diagnostics().iter().map(|d| result.sink.render(d)).collect();
    assert!(messages.iter().any(|m| m.contains("$A -> $B -> $A")), "{messages:?}");
    assert!(messages.iter().any(|m| m.contains("$B -> $A -> $B")), "{messages:?}");
}

#[test]
fn a_shared_dependency_is_expanded_for_each_distinct_path() {
    // C closes the cycle on A -> C, but not on A -> B -> C until it reaches A.
    // Repeating B is an inclusion, not handle deduplication.
    let result = read("0.4", "$A = @DEFINE-SYMBOL { $C; $B; $B; (0,0) @CIRCLE(radius=1) }; $B = @DEFINE-SYMBOL { $C; (0,0) @CIRCLE(radius=2) }; $C = @DEFINE-SYMBOL { $A; (0,0) @CIRCLE(radius=3) }; #R1 @SET-SYMBOL $A; #R2 @SET-SYMBOL $C");
    assert_eq!(radii(&result, "#R1"), [3.0, 3.0, 2.0, 3.0, 2.0, 1.0]);
    assert_eq!(radii(&result, "#R2"), [2.0, 2.0, 1.0, 3.0]);
}

#[test]
fn included_pin_names_are_available_to_later_body_statements() {
    let result = read("0.4", "$A = @DEFINE-SYMBOL { %a = (1,0) @PIN(terminal=1); $B; @LINE {%b (0,0)} }; $B = @DEFINE-SYMBOL { %b = (2,0) @PIN(terminal=2); $A; @LINE {%a (0,0)} }; #R1 @SET-SYMBOL $A; #R2 @SET-SYMBOL $B");
    let level = &result.model.levels[0];
    for name in ["#R1", "#R2"] {
        let item = level.items.iter().find(|item| item.id == name).unwrap();
        assert_eq!(level.symbols[item.symbol].pins.len(), 2);
    }
    let prims = primitives(level, &RenderOptions::default());
    assert_eq!(prims.iter().filter(|p| p.owner == "#R1" && matches!(p.kind, Kind::Path { .. })).count(), 1);
    assert_eq!(prims.iter().filter(|p| p.owner == "#R2" && matches!(p.kind, Kind::Path { .. })).count(), 1);
}

#[test]
fn a_pin_name_conflict_rejects_the_whole_inclusion_after_cycle_recovery() {
    let result = read("0.4", "$A = @DEFINE-SYMBOL { %p = (1,0) @PIN(terminal=1); $B; (0,0) @CIRCLE(radius=1) }; $B = @DEFINE-SYMBOL { %p = (2,0) @PIN(terminal=2); $A; (0,0) @CIRCLE(radius=2) }; #R1 @SET-SYMBOL $A; #R2 @SET-SYMBOL $B");
    assert_eq!(radii(&result, "#R1"), [1.0]);
    assert_eq!(radii(&result, "#R2"), [2.0]);
    let level = &result.model.levels[0];
    for item in &level.items {
        assert_eq!(level.symbols[item.symbol].pins.len(), 1);
    }
    assert!(result.diagnostics().iter().any(|d| d.code == rnl_runtime::diag::codes::PIN_NAME_TWICE_INCLUDED));
}

#[test]
fn the_own_name_still_means_the_outer_definition() {
    let result = read("0.4", "$A = @DEFINE-SYMBOL { $A; (0,0) @CIRCLE(radius=1) }; #R1 @SET-SYMBOL $A");
    assert_eq!(radii(&result, "#R1"), [1.0]);
    assert!(result.diagnostics().iter().any(|d| d.code == rnl_runtime::diag::codes::REF_DEFINE_SELF_NO_OUTER));
    assert!(!result.diagnostics().iter().any(|d| d.code == rnl_runtime::diag::codes::DEFINE_INCLUSION_CYCLE));
}

#[test]
fn deep_forward_inclusions_fail_explicitly_and_leave_independent_symbols() {
    for depth in [64, 65] {
        let mut body = String::from("#R1 @SET-SYMBOL $S0;");
        for index in 0..depth - 1 {
            body.push_str(&format!("$S{index} = @DEFINE-SYMBOL {{ $S{} }};", index + 1));
        }
        body.push_str(&format!("$S{} = @DEFINE-SYMBOL {{ (0,0) @CIRCLE(radius=1) }}; $SAFE = @DEFINE-SYMBOL {{ (0,0) @CIRCLE(radius=2) }}; #R2 @SET-SYMBOL $SAFE", depth - 1));
        let result = read("0.4", &body);
        assert_eq!(radii(&result, "#R2"), [2.0]);
        if depth == 64 {
            assert_eq!(radii(&result, "#R1"), [1.0]);
            assert!(
                !result.diagnostics().iter().any(|d| d.severity == rnl_runtime::diag::Severity::Error),
                "{:?}",
                result.diagnostics()
            );
        } else {
            assert!(result
                .diagnostics()
                .iter()
                .any(|d| d.code == rnl_runtime::diag::codes::DEFINE_SYMBOL_EXPANSION_LIMIT));
            assert!(radii(&result, "#R1").is_empty());
        }
    }
}

#[test]
fn repeated_forward_style_dependencies_do_not_expand_exponentially() {
    let mut body = String::from("#R1 @SET-STYLE $S0;");
    for index in 0..30 {
        body.push_str(&format!("$S{index} = @DEFINE-STYLE {{ $S{}; $S{} }};", index + 1, index + 1));
    }
    body.push_str("$S30 = @DEFINE-STYLE(linewidth=0.25) {}");
    let result = read("0.4", &body);
    let item = result.model.levels[0].items.iter().find(|item| item.id == "#R1").unwrap();
    assert_eq!(item.style.get("linewidth").and_then(|value| value.num()), Some(0.25));
    assert!(!result.diagnostics().iter().any(|d| d.severity == rnl_runtime::diag::Severity::Error));
}

#[test]
fn an_over_limit_definition_does_not_win_a_first_valid_name() {
    let mut body = String::from("#R1 @SET-SYMBOL $S0;");
    for index in 0..64 {
        body.push_str(&format!("$S{index} = @DEFINE-SYMBOL {{ $S{} }};", index + 1));
    }
    body.push_str(
        "$S64 = @DEFINE-SYMBOL { (0,0) @CIRCLE(radius=1) }; $S0 = @DEFINE-SYMBOL { (0,0) @CIRCLE(radius=2) }",
    );
    let result = read("0.4", &body);
    assert_eq!(radii(&result, "#R1"), [2.0]);
    let result = read("0.4", &body.replace("#R1 @SET-SYMBOL $S0;", "$ALIAS = $S0; #R1 @SET-SYMBOL $ALIAS;"));
    assert_eq!(radii(&result, "#R1"), [2.0]);
}

#[test]
fn a_deep_style_chain_has_an_explicit_limit_and_keeps_independent_styles() {
    let mut body = String::from("#R1 @SET-STYLE $S0;");
    for index in 0..64 {
        body.push_str(&format!("$S{index} = @DEFINE-STYLE {{ $S{} }};", index + 1));
    }
    body.push_str(
        "$S64 = @DEFINE-STYLE(linewidth=0.1) {}; $SAFE = @DEFINE-STYLE(linewidth=0.3) {}; #R2 @SET-STYLE $SAFE",
    );
    let result = read("0.4", &body);
    let item = result.model.levels[0].items.iter().find(|item| item.id == "#R2").unwrap();
    assert_eq!(item.style.get("linewidth").and_then(|value| value.num()), Some(0.3));
    assert!(result.diagnostics().iter().any(|d| d.code == rnl_runtime::diag::codes::DEFINE_STYLE_EXPANSION_LIMIT));
}
