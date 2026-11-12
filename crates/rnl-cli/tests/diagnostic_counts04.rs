//! Counted diagnostic subjects retain released wording and use singular grammar in 0.4.
use rnl_runtime::{
    diag::{codes, Code},
    source::MemoryProvider,
    Interpretation, Options,
};
use std::path::Path;

fn read(version: &str, body: &str, second: bool) -> Interpretation {
    let extra = if second { "R2 a b 2k\n" } else { "" };
    let source = format!(
        "* counts\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\n{extra}* @RNL {{ {body} }}\n"
    );
    rnl_cli::interpret_file(
        Path::new("counts.sp"),
        None,
        &MemoryProvider::new().with("counts.sp", &source),
        &Options { std: false, ..Default::default() },
    )
    .unwrap()
}
fn message(r: &Interpretation, code: Code) -> String {
    r.diagnostics().into_iter().find(|d| d.code == code).unwrap().message.clone()
}

#[test]
fn a_single_unplaced_object_uses_its_levels_version() {
    for version in ["0.2", "0.3", "0.4"] {
        let r = read(version, "", false);
        let expected = if version == "0.4" {
            "1 object has no position and is left to a solver; it is not drawn"
        } else {
            "1 objects have no position and are left to a solver; they are not drawn"
        };
        assert!(message(&r, codes::UNPLACED_OBJECTS).contains(expected));
    }
    assert!(message(&read("0.4", "", true), codes::UNPLACED_OBJECTS).contains("2 objects have no position"));
}

#[test]
fn role_and_object_counts_keep_released_arity_messages() {
    for version in ["0.2", "0.3", "0.4"] {
        for (body, modern, legacy) in [
            (
                "$D = @DEFINE(roles=[\"r\"]) {}; #R1 #R1 @AS $D",
                "has 1 role, 2 objects are written",
                "has 1 roles, 2 objects are written",
            ),
            (
                "$D = @DEFINE(roles=[]) {}; #R1 @AS $D",
                "has 0 roles, 1 object is written",
                "has 0 roles, 1 objects are written",
            ),
        ] {
            assert!(message(&read(version, body, false), codes::USAGE_TOO_MANY_OBJECTS)
                .contains(if version == "0.4" { modern } else { legacy }));
        }
    }
}

#[test]
fn unused_body_arity_checks_share_the_same_count_formatting() {
    let r = read("0.4", "$ONE = @DEFINE(roles=[\"r\"]) {}; $D = @DEFINE(roles=[]) { (0,0) (1,1) @AS $ONE }", false);
    let errors: Vec<_> = r.diagnostics().into_iter().filter(|d| d.code == codes::USAGE_TOO_MANY_OBJECTS).collect();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("has 1 role, 2 objects are written"));
    assert!(errors[0].origin.is_empty());
}

#[test]
fn unplaced_summaries_follow_each_included_levels_version() {
    let top = "* counts\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\n.include child.inc\nR0 a b 1k\n";
    let child = "* child\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n.subckt child a b\nR1 a b 1k\n.ends child\n";
    let r = rnl_cli::interpret_file(
        Path::new("top.sp"),
        None,
        &MemoryProvider::new().with("top.sp", top).with("child.inc", child),
        &Options { std: false, ..Default::default() },
    )
    .unwrap();
    let messages: Vec<_> = r.diagnostics().into_iter().filter(|d| d.code == codes::UNPLACED_OBJECTS).collect();
    assert!(messages
        .iter()
        .any(|d| d.loc.as_ref().is_some_and(|l| l.file.ends_with("child.inc")) && d.message.contains("1 object has")));
    assert!(messages
        .iter()
        .any(|d| d.loc.as_ref().is_some_and(|l| l.file.ends_with("top.sp")) && d.message.contains("1 objects have")));
}
