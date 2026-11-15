//! A definition body that binds a local name to a selection of its roles (6.9, 10.4). Its rules
//! are evaluated in the usage's level and in each level below. Below, the selection reaches
//! nothing of that level, so the rule gives that level nothing. Before, the name was taken as
//! failed there, and every statement naming it was an error, RNL3203, in any file with a
//! subcircuit, even one no instance used.

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::problem::{FactKind, ProblemFact};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

/// The body of `$F`: the source and the drain of its device, aligned.
const BODY: &str = r#"%c = ?device @SELECT-TERMINAL(terminal="s"); ?device @SELECT-TERMINAL(terminal="d") => @ADD-TO %c; %c @ALIGN("y", priority="low")"#;

/// A file with a transistor M1 that `$F` is applied to, on line 8, with the subcircuit `buf`
/// when `subckt` holds, which no instance uses unless `used` holds too.
fn interpret(body: &str, subckt: bool, used: bool) -> Interpretation {
    let cell = if subckt { ".subckt buf a y\nR9 a y 1k\n.ends buf\n" } else { "" };
    let instance = if used { "X1 a b buf\n" } else { "" };
    let net = format!(
        "* t\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\n.model nch nmos level=1\n{cell}M1 a g b 0 nch\n{instance}\
         * @RNL {{\n*   $F = @DEFINE(roles=[[\"device\", \"instance\"]]) {{ {body} }}; #M1 @AS $F;\n* }}\n.end\n"
    );
    let files = MemoryProvider::new().with("t.sp", &net);
    let options = Options { std: false, problems: true, ..Options::default() };
    interpret_file(Path::new("t.sp"), None, &files, &options).unwrap()
}

fn loud(r: &Interpretation) -> Vec<String> {
    r.diagnostics()
        .into_iter()
        .filter(|d| d.severity != Severity::Hint)
        .map(|d| format!("{}: {}", d.code, d.message))
        .collect()
}

/// The members of the ALIGN facts that the level `title` has.
fn aligned(r: &Interpretation, title: &str) -> Vec<usize> {
    let p = r.problems.iter().find(|p| p.title == title).unwrap();
    let members = |f: &ProblemFact| match &f.kind {
        FactKind::Align { members, .. } => Some(members.len()),
        _ => None,
    };
    p.facts.iter().filter_map(members).collect()
}

#[test]
fn a_local_name_bound_to_a_selection_holds_in_a_file_with_a_subcircuit() {
    for (subckt, used) in [(false, false), (true, false), (true, true)] {
        let r = interpret(BODY, subckt, used);
        assert_eq!(loud(&r), Vec::<String>::new(), "subckt {subckt}, used {used}");
        // the source and the drain of M1 are aligned in the level of the usage
        assert_eq!(aligned(&r, "t.sp"), [2], "subckt {subckt}, used {used}");
        // and the subcircuit, where the selection reaches nothing, has no alignment
        if subckt {
            assert_eq!(aligned(&r, "buf"), Vec::<usize>::new(), "used {used}");
        }
    }
}

#[test]
fn the_name_alone_and_a_set_that_names_the_terminals_hold_too() {
    let alone = r#"%c = ?device @SELECT-TERMINAL(terminal="s"); %c @ALIGN("y", priority="low")"#;
    let set = r#"%c = {?device:s}; ?device:d => @ADD-TO %c; %c @ALIGN("y", priority="low")"#;
    for body in [alone, set] {
        let r = interpret(body, true, false);
        assert_eq!(loud(&r), Vec::<String>::new(), "{body}");
        assert_eq!(aligned(&r, "t.sp").len(), 1, "{body}");
    }
}
