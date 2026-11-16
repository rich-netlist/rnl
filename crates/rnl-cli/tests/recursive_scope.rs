//! Selection statements and named sets have distinct reach (10.1, 10.3, 10.4).
use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};
use std::path::Path;

fn read(version: &str, body: &str) -> Interpretation {
    let body = format!("$S = @DEFINE-SYMBOL {{ (0,0) @PIN(terminal=1) }}; # @SELECT-INSTANCE(name=\"R*\", recursive=true) => @SET-SYMBOL $S; {body}");
    let source = format!("* scope\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\n.subckt child p q\nR2 p q 1k\n.ends child\nR1 a b 1k\nX1 a b child\n* @RNL {{\n{}\n* }}\n.end\n", body.lines().map(|l| format!("* {l}")).collect::<Vec<_>>().join("\n"));
    let r = interpret_file(
        Path::new("t.sp"),
        None,
        &MemoryProvider::new().with("t.sp", &source),
        &Options { std: false, ..Options::solver() },
    )
    .unwrap();
    let errors: Vec<_> = r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).collect();
    assert!(errors.is_empty(), "{body}: {errors:?}");
    r
}

fn marked(r: &Interpretation) -> Vec<Vec<String>> {
    r.problems
        .iter()
        .map(|p| {
            p.objects
                .iter()
                .filter(|o| o.text.as_ref().is_some_and(|t| t.key.as_deref() == Some("mark")))
                .map(|o| o.label.clone())
                .collect()
        })
        .collect()
}

fn check(body: &str, child: bool) {
    let r = read("0.4", body);
    assert_eq!(
        marked(&r),
        vec![vec!["#R1::mark".to_string()], if child { vec!["#R2::mark".to_string()] } else { vec![] }],
        "{body}"
    );
}

#[test]
fn default_reach_changes_only_for_statements_written_in_0_4() {
    for version in ["0.2", "0.3", "0.4"] {
        let r = read(version, "# @SELECT-INSTANCE(name=\"R*\") => @TEXT(\"mark\",key=\"mark\")");
        assert_eq!(marked(&r)[1].len(), usize::from(version != "0.4"), "{version}");
    }
}

#[test]
fn an_explicit_flag_decides_the_whole_selection_pipe() {
    check("# @SELECT-INSTANCE(name=\"R*\",recursive=false) => @TEXT(\"mark\",key=\"mark\")", false);
    for pipe in [
        "# @SELECT-INSTANCE(name=\"R*\",recursive=true) => @SELECT-INSTANCE(recursive=false)",
        "# @SELECT-INSTANCE(name=\"R*\",recursive=false) => @SELECT-INSTANCE(recursive=true)",
        "{# @SELECT-INSTANCE(name=\"R*\",recursive=true)} @SELECT-INSTANCE",
    ] {
        check(&format!("{pipe} => @TEXT(\"mark\",key=\"mark\")"), true);
    }
}

#[test]
fn a_written_local_selection_does_not_borrow_reach_from_a_named_scope() {
    check(
        "$wide=# @SELECT-INSTANCE(name=\"R*\",recursive=true); $wide @SELECT-INSTANCE => @TEXT(\"mark\",key=\"mark\")",
        false,
    );
}

#[test]
fn mixed_sets_keep_the_members_that_reach_the_evaluated_level() {
    for expression in ["{$local $wide}", "{$wide $local}", "{$local $wide} @UNION"] {
        check(&format!("$local=# @SELECT-INSTANCE(name=\"R1\",recursive=false); $wide=# @SELECT-INSTANCE(name=\"R*\",recursive=true); $both={expression}; $both @TEXT(\"mark\",key=\"mark\")"),true);
    }
}

#[test]
fn a_recursive_statement_cannot_expand_a_named_local_set() {
    let r=read("0.4","$local=# @SELECT-INSTANCE(name=\"R*\"); $local @SELECT-INSTANCE(recursive=true) => @TEXT(\"mark\",key=\"mark\")");
    assert_eq!(marked(&r)[1], Vec::<String>::new());
}

#[test]
fn additions_follow_the_source_reach_and_removals_do_not_expand_it() {
    check(
        "$dest={}; # @SELECT-INSTANCE(name=\"R*\",recursive=true) => @ADD-TO $dest; $dest @TEXT(\"mark\",key=\"mark\")",
        true,
    );
    check("$dest={}; $src=# @SELECT-INSTANCE(name=\"R*\",recursive=true); $src @ADD-TO $dest; $dest @TEXT(\"mark\",key=\"mark\")",true);
    check("$dest=# @SELECT-INSTANCE(name=\"R*\"); # @SELECT-INSTANCE(name=\"none\",recursive=true) => @REMOVE-FROM $dest; $dest @TEXT(\"mark\",key=\"mark\")",false);
}

#[test]
fn an_addition_can_extend_a_local_selection_target() {
    check("$dest=# @SELECT-INSTANCE(name=\"R*\"); # @SELECT-INSTANCE(name=\"R*\",recursive=true) => @ADD-TO $dest; $dest @TEXT(\"mark\",key=\"mark\")", true);
}

#[test]
fn an_addition_cannot_borrow_reach_from_its_target() {
    let r=read("0.4", "$dest=# @SELECT-INSTANCE(name=\"none\",recursive=true); $src=# @SELECT-INSTANCE(name=\"R*\"); $src @ADD-TO $dest; $dest @TEXT(\"mark\",key=\"mark\")");
    assert_eq!(marked(&r), vec![vec!["#R1::mark".to_string()], vec![]]);
}

#[test]
fn selection_edits_of_nested_sets_remain_rule_inputs() {
    check("$dest={}; $nested={$dest}; # @SELECT-INSTANCE(name=\"R*\",recursive=true) => @ADD-TO $dest; $nested @TEXT(\"mark\",key=\"mark\")",true);
}

#[test]
fn later_edits_are_seen_by_earlier_rule_uses() {
    check(
        "$dest={}; $dest @TEXT(\"mark\",key=\"mark\"); # @SELECT-INSTANCE(name=\"R*\",recursive=true) => @ADD-TO $dest",
        true,
    );
}

#[test]
fn local_removal_does_not_remove_members_in_descendants() {
    check("$dest=# @SELECT-INSTANCE(name=\"R*\",recursive=true); # @SELECT-INSTANCE(name=\"none\") => @REMOVE-FROM $dest; $dest @TEXT(\"mark\",key=\"mark\")",true);
    let r=read("0.4", "$dest=# @SELECT-INSTANCE(name=\"R*\",recursive=true); # @SELECT-INSTANCE(name=\"R*\") => @REMOVE-FROM $dest; $dest @TEXT(\"mark\",key=\"mark\")");
    assert_eq!(marked(&r), vec![vec![], vec!["#R2::mark".to_string()]]);
}

#[test]
fn package_rule_origin_is_the_use_site() {
    for recursive in [false, true] {
        for in_header in [false, true] {
            for indirect in [false, true] {
                let usage = if indirect {
                    "@USE-PACKAGE(name=\"outer\",version=\"1.0\",path=\"outer.rnl\")"
                } else {
                    "@USE-PACKAGE(name=\"inner\",version=\"1.0\",path=\"inner.rnl\")"
                };
                let header = if in_header { format!("; {usage}") } else { String::new() };
                let content = if in_header { String::new() } else { format!("* @RNL {{ {usage} }}\n") };
                let source=format!("* scope\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\"){header} }}\n.subckt child p q\nR2 p q 1k\n.ends child\nR1 a b 1k\nX1 a b child\n{content}.end\n");
                let package=format!("@RNL(type=\"header\") {{ @VERSION(\"0.4\"); @PACKAGE(name=\"inner\",version=\"1.0\") }}\n@RNL {{ $S = @DEFINE-SYMBOL {{ (0,0) @PIN(terminal=1) }}; # @SELECT-INSTANCE(recursive=true) => @SET-SYMBOL $S; # @SELECT-INSTANCE(name=\"R*\",recursive={recursive}) => @TEXT(\"mark\",key=\"mark\") }}\n");
                let outer="@RNL(type=\"header\") { @VERSION(\"0.4\"); @PACKAGE(name=\"outer\",version=\"1.0\"); @USE-PACKAGE(name=\"inner\",version=\"1.0\",path=\"inner.rnl\") }\n";
                let files =
                    MemoryProvider::new().with("t.sp", &source).with("inner.rnl", &package).with("outer.rnl", outer);
                let r = interpret_file(Path::new("t.sp"), None, &files, &Options { std: false, ..Options::solver() })
                    .unwrap();
                assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", r.diagnostics());
                assert_eq!(
                    marked(&r),
                    vec![vec!["#R1::mark".to_string()], if recursive { vec!["#R2::mark".to_string()] } else { vec![] }],
                    "recursive={recursive}, header={in_header}, indirect={indirect}"
                );
            }
        }
    }
}

#[test]
fn recursive_generation_makes_no_usage_where_its_set_role_is_empty() {
    // the recursive selection reaches the child, where the local set has no members: a rule of set
    // roles only makes no usage there (9.4); the top level, where the set holds R1, gets its one
    for selection in ["$local", "$local @SELECT-INSTANCE(recursive=true)"] {
        let r = read(
            "0.4",
            &format!(
                "$D = @DEFINE(roles=[\"{{xs}}\"]) {{}}; $local=# @SELECT-INSTANCE(name=\"R*\"); {selection} => @AS $D"
            ),
        );
        let count = |p: &rnl_runtime::problem::LevelProblem| p.usages.iter().filter(|u| u.definition == "$D").count();
        assert_eq!(count(&r.problems[1]), 0, "{selection}");
        assert_eq!(r.problems.iter().map(count).sum::<usize>(), 1, "{selection}");
    }
}

#[test]
fn inline_local_selection_limits_a_set_containing_a_wide_reference() {
    check("$wide=# @SELECT-INSTANCE(name=\"R*\",recursive=true); $local={$wide; # @SELECT-INSTANCE(name=\"none\")}; $local @TEXT(\"mark\",key=\"mark\")",false);
    check("$wide=# @SELECT-INSTANCE(name=\"R*\",recursive=true); $local={$wide; # @SELECT-INSTANCE(name=\"none\")} @UNION; $local @SELECT-INSTANCE(recursive=true) => @TEXT(\"mark\",key=\"mark\")",false);
}

#[test]
fn a_scoped_record_uses_the_target_level_as_its_origin() {
    let source = r#"* scope
* @RNL(type="header") { @VERSION("0.4") }
.subckt child p q
R2 p q 1k
.ends child
R1 a b 1k
X1 a b child
* @RNL(scope="child") { $S = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1) }; # @SELECT-INSTANCE => @SET-SYMBOL $S; # @SELECT-INSTANCE => @TEXT("mark",key="mark") }
.end
"#;
    let r = interpret_file(
        Path::new("t.sp"),
        None,
        &MemoryProvider::new().with("t.sp", source),
        &Options { std: false, ..Options::solver() },
    )
    .unwrap();
    assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", r.diagnostics());
    assert_eq!(marked(&r), vec![vec![], vec!["#R2::mark".to_string()]]);
}

#[test]
fn generated_sets_keep_the_reach_of_their_creation_rule() {
    for (flag, expected) in [("", 0), (",recursive=true", 1)] {
        let r=read("0.4",&format!("$ONE = @DEFINE(roles=[\"x\"]) {{}}; $ALL = @DEFINE(roles=[\"{{xs}}\"]) {{}}; $g=# @SELECT-INSTANCE(name=\"R*\"{flag}) => @AS $ONE; $g @AS $ALL"));
        assert_eq!(r.problems[1].usages.iter().filter(|u| u.definition == "$ALL").count(), expected, "{flag}");
    }
}

#[test]
fn later_edits_propagate_through_addition_sources() {
    check("$dest={}; $src={}; $src @ADD-TO $dest; # @SELECT-INSTANCE(name=\"R*\",recursive=true) => @ADD-TO $src; $dest @TEXT(\"mark\",key=\"mark\")",true);
}

#[test]
fn the_reach_of_either_set_argument_can_enable_a_rule() {
    let r=read("0.4", "$D = @DEFINE(roles=[\"{local}\",\"{wide}\"]) {}; $local=# @SELECT-INSTANCE(name=\"none\"); $wide=# @SELECT-INSTANCE(name=\"R*\",recursive=true); $local $wide @AS $D");
    assert_eq!(r.problems[1].usages.iter().filter(|u| u.definition == "$D").count(), 1);
}

#[test]
fn scoped_rules_follow_the_source_file_version_in_mixed_includes() {
    for (host_version, rule_version, grand) in [("0.3", "0.4", false), ("0.4", "0.3", true)] {
        let source = format!(
            r#"* scope
* @RNL(type="header") {{ @VERSION("{rule_version}") }}
.include "cells.sp"
R1 a b 1k
X1 a b parent
* @RNL(scope="parent") {{ $S = @DEFINE-SYMBOL {{ (0,0) @PIN(terminal=1) }}; # @SELECT-INSTANCE(recursive=true) => @SET-SYMBOL $S; # @SELECT-INSTANCE(name="R*") => @TEXT("mark",key="mark") }}
.end
"#
        );
        let cells = format!(
            r#"* cells
* @RNL(type="header") {{ @VERSION("{host_version}") }}
.subckt parent p q
.subckt grand p q
R3 p q 1k
.ends grand
R2 p q 1k
X2 p q grand
.ends parent
"#
        );
        let files = MemoryProvider::new().with("t.sp", &source).with("cells.sp", &cells);
        let r = interpret_file(Path::new("t.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap();
        assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", r.diagnostics());
        let counts: Vec<_> = r
            .problems
            .iter()
            .map(|p| {
                (
                    p.name.as_str(),
                    p.objects
                        .iter()
                        .filter(|o| o.text.as_ref().is_some_and(|t| t.key.as_deref() == Some("mark")))
                        .count(),
                )
            })
            .collect();
        assert_eq!(counts.iter().find(|(name, _)| *name == "parent").unwrap().1, 1);
        assert_eq!(
            counts.iter().find(|(name, _)| name.ends_with("grand")).unwrap().1,
            usize::from(grand),
            "host={host_version}, rule={rule_version}, {counts:?}"
        );
    }
}

#[test]
fn a_collection_rule_still_expands_an_ordinary_set_in_a_single_role() {
    let r = read(
        "0.4",
        "$D = @DEFINE(roles=[\"x\",\"{ys}\"]) {}; $selected=# @SELECT-INSTANCE; {#R1 #R1} $selected @AS $D",
    );
    let usages: Vec<_> = r.problems[0].usages.iter().filter(|u| u.definition == "$D").collect();
    assert_eq!(usages.len(), 2);
    assert!(usages
        .iter()
        .all(|usage| usage.generated && usage.roles.iter().find(|role| role.role == "x").unwrap().members.len() == 1));
}
