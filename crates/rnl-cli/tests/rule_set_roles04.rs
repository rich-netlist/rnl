//! A rule that fills only set roles makes no usage in a level where all of them are empty (9.4),
//! and position facts in the body of a generated usage take effect after generation (10.5).
use std::path::Path;
use std::process::Command;

use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Severity};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

const HIERARCHY: &str = "* t
* @RNL(type=\"header\") { @VERSION(\"0.4\") }
.subckt sub a b
R1 a b 1k
.ends sub
X1 n1 n2 sub
* @RNL { $D = @DEFINE(roles=[\"{xs}\"]) {}; %s = # @SELECT-INSTANCE(name=\"R*\", recursive=true); $D @ATTACH %s; WRITTEN }
.end
";

/// The usages of `definition` in each level, by title, with the members of their one role.
fn usages(netlist: &str, definition: &str) -> Vec<(String, Vec<serde_json::Value>)> {
    let dir = std::env::temp_dir().join(format!("rnl-rule-set-roles-{}-{}", std::process::id(), definition.len()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("t{}.sp", netlist.len()));
    std::fs::write(&path, netlist).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_rnl")).arg("problem").arg(&path).arg("--no-std").output().unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let mut out = Vec::new();
    for level in json["levels"].as_array().unwrap() {
        for usage in level["usages"].as_array().unwrap() {
            if usage["definition"] == definition {
                let members = usage["roles"][0]["members"].as_array().cloned().unwrap_or_default();
                out.push((level["title"].as_str().unwrap().to_string(), members));
            }
        }
    }
    out
}

#[test]
fn a_rule_of_set_roles_makes_no_usage_where_they_are_empty() {
    // the selection has R1 in sub and nothing at the top: one usage, in sub
    let made = usages(&HIERARCHY.replace("WRITTEN", ""), "$D");
    assert_eq!(made.len(), 1, "{made:?}");
    assert_eq!(made[0].0, "sub");
    assert_eq!(made[0].1.len(), 1);
}

#[test]
fn a_written_application_of_set_roles_is_made_when_they_are_empty() {
    // a statement, not a rule: the usage is made with its set role empty, as before
    let made = usages(&HIERARCHY.replace("WRITTEN", "$D @ATTACH;"), "$D");
    assert_eq!(made.len(), 2, "{made:?}");
    assert!(made.iter().any(|(level, members)| level != "sub" && members.is_empty()));
}

fn read(body: &str) -> Interpretation {
    let text = format!("* t\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\nR1 a b 1k\nR2 c d 2k\n* @RNL {{ $S = @DEFINE-SYMBOL {{ (0,0) @PIN(terminal=1); (4,0) @PIN(terminal=2) }}; {{#R1 #R2}} @SET-SYMBOL $S; #R1 @PLACE(0,0); #R2 @PLACE(10,0); {} }}\n.end\n", body);
    interpret_file(
        Path::new("t.sp"),
        None,
        &MemoryProvider::new().with("t.sp", &text),
        &Options { std: false, ..Options::solver() },
    )
    .unwrap()
}

#[test]
fn orient_in_a_generated_usage_takes_effect_after_generation() {
    // the usage is generated, its body orients what its set role got: no rule of step 5 or 6 (10.5)
    let r =
        read("$C = @DEFINE(roles=[\"{xs}\"]) { ?xs @ORIENT(\"R90\") }; # @SELECT-INSTANCE(name=\"R1\") => $C @ATTACH");
    let errors: Vec<_> =
        r.diagnostics().iter().filter(|d| d.severity == Severity::Error).map(|d| r.sink.render(d)).collect();
    assert!(errors.is_empty(), "{errors:?}");
    assert!(!r.diagnostics().iter().any(|d| d.code == codes::USAGE_GENERATED_RULE));
    let items = &r.model.levels[0].items;
    let r1 = items
        .iter()
        .find(|i| i.object_id == "#R1")
        .unwrap_or_else(|| panic!("{:?}", items.iter().map(|i| &i.object_id).collect::<Vec<_>>()));
    let r2 = items.iter().find(|i| i.object_id == "#R2").unwrap();
    assert_ne!(r1.orient, r2.orient, "R1 is turned, R2 is not");
}

#[test]
fn layer_in_a_generated_usage_takes_effect_in_the_last_step() {
    // LAYER runs in step 8, after the generation of step 6 that queues it (10.5)
    let r =
        read("$C = @DEFINE(roles=[\"{xs}\"]) { ?xs @LAYER(\"top\") }; # @SELECT-INSTANCE(name=\"R1\") => $C @ATTACH");
    let errors: Vec<_> =
        r.diagnostics().iter().filter(|d| d.severity == Severity::Error).map(|d| r.sink.render(d)).collect();
    assert!(errors.is_empty(), "{errors:?}");
    let items = &r.model.levels[0].items;
    let layer = |id: &str| items.iter().find(|i| i.object_id == id).map(|i| format!("{:?}", i.layer));
    assert_eq!(layer("#R1").as_deref(), Some("Top"));
    assert_eq!(layer("#R2").as_deref(), Some("Middle"));
}

#[test]
fn generation_in_a_generated_usage_still_fails() {
    // a rule of step 6 in the body of a generated usage fails as before (10.5)
    let r = read("$P = @DEFINE(roles=[\"{xs}\"]) { ?xs @SELECT-PIN => @CIRCLE(radius=1) }; # @SELECT-INSTANCE(name=\"R1\") => $P @ATTACH");
    assert!(r.diagnostics().iter().any(|d| d.code == codes::USAGE_GENERATED_RULE));
}
