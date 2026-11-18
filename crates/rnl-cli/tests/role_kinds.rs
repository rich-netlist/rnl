//! The kinds a role of a definition declares (9.4): every kind name of the word list (5.3), not
//! only those of the host and the scene. "text" takes every TEXT; a TEXT attached to an object is a
//! label and one attached to a point an annotation (5.1); "bundle" takes bundles, which "instance"
//! takes too (11.8). Analog Canvas's extension package attaches a usage to the label of an
//! instance, and could not say that its role takes a label.

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::Options;

/// The errors of declaring and applying the definitions below, in a file with a label `#R1::name`,
/// a text `%t` at a point, a bundle `%b`, and the `extra` statements. The label comes from the
/// standard packages of symbols and their host rules; any description of R2 and R3 that fails on
/// its own, as they are in a bundle (11.8), is left out.
fn errors(extra: &str) -> Vec<String> {
    let net = format!(
        "* t\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\"); @USE-PACKAGE(name=\"rnl-std-basic\", version=\"0.4\"); @USE-PACKAGE(name=\"rnl-std-semiconductors\", version=\"0.4\"); @USE-PACKAGE(name=\"rnl-std-semantics\", version=\"0.4\"); @USE-PACKAGE(name=\"rnl-std-spice\", version=\"0.4\") }}\nR1 a 0 1k\nR2 a b 1k\nR3 b 0 1k\n* @RNL {{\n\
         *   $TEXT_OF = @DEFINE(roles=[[\"t\", \"text\"]]) {{}};\n\
         *   $LABEL_OF = @DEFINE(roles=[[\"t\", \"label\"]]) {{}};\n\
         *   $NOTE_OF = @DEFINE(roles=[[\"t\", \"annotation\"]]) {{}};\n\
         *   $GROUP_OF = @DEFINE(roles=[[\"b\", \"bundle\"]]) {{}};\n\
         *   $PART_OF = @DEFINE(roles=[[\"i\", \"instance\"]]) {{}};\n\
         *   #R1 @PLACE(0, 0);\n*   (0, 6) @TEXT(\"note\") => %t;\n*   %b = {{#R2 #R3}} @BUNDLE{extra}\n* }}\n.end\n"
    );
    let files = MemoryProvider::new().with("t.sp", &net);
    let r = interpret_file(Path::new("t.sp"), None, &files, &Options::default()).unwrap();
    let errors = r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error);
    errors.map(|d| r.sink.render(d)).filter(|e| !e.contains("is in a bundle")).collect()
}

#[test]
fn every_kind_name_can_be_declared() {
    assert!(errors("").is_empty(), "{:?}", errors(""));
}

#[test]
fn a_role_takes_the_objects_of_its_kind() {
    for statement in [
        "$TEXT_OF @ATTACH #R1::name",
        "$TEXT_OF @ATTACH %t",
        "$LABEL_OF @ATTACH #R1::name",
        "$NOTE_OF @ATTACH %t",
        "$GROUP_OF @ATTACH %b",
        "$PART_OF @ATTACH %b",
    ] {
        let e = errors(&format!(";\n*   {statement}"));
        assert!(e.is_empty(), "{statement}: {e:?}");
    }
}

#[test]
fn a_role_refuses_the_objects_of_another_kind() {
    // the error names the kind the role takes (9.4)
    for (statement, takes) in [
        ("$LABEL_OF @ATTACH %t", "takes label"),
        ("$NOTE_OF @ATTACH #R1::name", "takes annotation"),
        ("$TEXT_OF @ATTACH #R1", "takes text"),
        ("$GROUP_OF @ATTACH #R1", "takes bundle"),
    ] {
        let e = errors(&format!(";\n*   {statement}"));
        assert!(e.iter().any(|e| e.contains("usage-role-wrong-kind") && e.contains(takes)), "{statement}: {e:?}");
    }
}
