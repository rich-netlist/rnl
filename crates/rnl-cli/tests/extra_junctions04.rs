//! Additional junctions keep existing point identities (13.2).
use rnl_cli::interpret_file;
use rnl_runtime::{
    diag::{codes, Severity},
    problem::{Filler, PointRef, Reference},
    solution::{write_record, Decision, Solution},
    source::MemoryProvider,
    Interpretation, Options,
};
use std::path::Path;

const DOT: &str = r#"$DOT = @DEFINE(roles=[["p","point"]]) {};"#;
fn source(version: &str, setting: &str, body: &str) -> String {
    format!("* extra dots\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\"); {setting} }}\nR1 a b 1k\n* @RNL {{ {DOT} {body} }}\n")
}
fn text(source: &str) -> Interpretation {
    interpret_file(
        Path::new("main.sp"),
        None,
        &MemoryProvider::new().with("main.sp", source),
        &Options { std: false, problems: true, ..Default::default() },
    )
    .unwrap()
}
fn read(version: &str, setting: &str, body: &str) -> Interpretation {
    text(&source(version, setting, body))
}
fn clean(r: &Interpretation) {
    assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", r.diagnostics());
}
fn object(r: &Interpretation, name: &str) -> usize {
    r.problems[0].objects.iter().position(|o| o.reference == Reference::Name(name.into())).unwrap()
}
const ON: &str = r#"@SOLVER-CONFIG(junction="$DOT")"#;

#[test]
fn existing_points_and_terminals_receive_dots_without_new_points_or_wires() {
    let body = r#"$S= @DEFINE-SYMBOL { (0,0) @PIN(terminal=1) }; #R1 @SET-SYMBOL $S; %p=(3,0); &a @WIRE {#R1:1 %p}"#;
    let r = read("0.4", ON, body);
    clean(&r);
    let s = Solution { junctions: r.problems[0].wires[0].anchors.clone(), ..Default::default() };
    let w = write_record(&r.problems[0], &s, "test", "hash");
    assert!(w.unsolved.is_empty(), "{:?}", w.unsolved);
    assert!(w.record.contains("$DOT @ATTACH #R1:p"), "{}", w.record);
    assert!(w.record.contains("$DOT @ATTACH %p"), "{}", w.record);
    assert!(!w.record.contains("@WIRE") && !w.record.contains("%rt"));
    let back = source("0.4", ON, body) + &w.record.lines().map(|s| format!("* {s}\n")).collect::<String>();
    let again = text(&back);
    clean(&again);
    assert!(again.model.levels[0].points.iter().find(|p| p.id == "%p").unwrap().junction);
}

#[test]
fn no_configuration_and_an_empty_value_disable_extra_dots() {
    for setting in ["", r#"@SOLVER-CONFIG(junction="")"#] {
        let r = read("0.4", setting, "%p=(0,0)");
        clean(&r);
        let s = Solution { junctions: vec![PointRef::Origin(object(&r, "%p"))], ..Default::default() };
        let w = write_record(&r.problems[0], &s, "test", "hash");
        assert!(!w.record.contains("@ATTACH"));
        assert!(w.unsolved.is_empty() && w.warnings.is_empty());
    }
    let r = read("0.4", r#"@SOLVER-CONFIG(junction="$MISSING")"#, "%p=(0,0)");
    let all = r.diagnostics();
    let errors: Vec<_> = all.iter().filter(|d| d.severity == Severity::Error).collect();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, codes::SOLVER_CONFIG_JUNCTION_NOT_TAKEN);
    let w = write_record(
        &r.problems[0],
        &Solution { junctions: vec![PointRef::Origin(0)], ..Default::default() },
        "test",
        "hash",
    );
    assert!(!w.record.contains("@ATTACH"));
    assert!(w.warnings.is_empty());
}

#[test]
fn missing_and_unwritable_points_report_the_specific_request() {
    let mut r = read("0.4", ON, "%p=(0,0)");
    clean(&r);
    let index = object(&r, "%p");
    r.problems[0].objects[index].reference = Reference::Anonymous;
    let s = Solution {
        junctions: vec![PointRef::Origin(usize::MAX), PointRef::Centre(vec![]), PointRef::Origin(index)],
        ..Default::default()
    };
    let w = write_record(&r.problems[0], &s, "test", "hash");
    assert_eq!(w.unsolved.len(), 3);
    for (index, u) in w.unsolved.iter().enumerate() {
        assert_eq!(u.decision, Decision::Junction { index });
    }
    assert_eq!(w.unsolved[0].code, codes::UNSOLVED_NOT_IN_PROBLEM);
    assert_eq!(w.unsolved[1].code, codes::UNSOLVED_JUNCTION_CENTRE);
    assert!(!w.record.contains("@ATTACH"));
}

#[test]
fn equal_coordinates_do_not_change_the_identity_of_the_requested_dot() {
    let r = read("0.4", ON, "%a=(0,0); %b=(0,0)");
    clean(&r);
    let s = Solution { junctions: vec![PointRef::Origin(object(&r, "%a"))], ..Default::default() };
    let w = write_record(&r.problems[0], &s, "test", "hash");
    assert!(w.record.contains("$DOT @ATTACH %a"));
    assert!(!w.record.contains("%b") && !w.record.contains("@WIRE"));
}

#[test]
fn independent_pins_and_pin_positions_keep_their_written_reference() {
    let r = read(
        "0.4",
        ON,
        r#"$S= @DEFINE-SYMBOL { %first=(0,0) @PIN(terminal=1); %last=(2,0) @PIN(terminal=1) }; #R1 @SET-SYMBOL $S; &a @WIRE {#R1::first #R1::last::position}"#,
    );
    clean(&r);
    let s = Solution { junctions: r.problems[0].wires[0].anchors.clone(), ..Default::default() };
    let w = write_record(&r.problems[0], &s, "test", "hash");
    assert!(w.unsolved.is_empty(), "{:?}", w.unsolved);
    assert!(w.record.contains("$DOT @ATTACH #R1::first"), "{}", w.record);
    assert!(w.record.contains("$DOT @ATTACH #R1::last::position"), "{}", w.record);
    assert!(!w.record.contains("#R1:1"));
}

#[test]
fn generated_owners_are_written_before_their_dot() {
    let r = read("0.4", ON, r#"$D= @DEFINE(roles=["r"]) { %p=(1,0) }; # @SELECT-INSTANCE => @AS $D"#);
    clean(&r);
    let p = &r.problems[0];
    let index = p
        .objects
        .iter()
        .position(|o| matches!(&o.reference,Reference::Generated{path,..} if path.concat()=="::p"))
        .unwrap();
    let w =
        write_record(p, &Solution { junctions: vec![PointRef::Origin(index)], ..Default::default() }, "test", "hash");
    assert!(w.unsolved.is_empty(), "{:?}", w.unsolved);
    let definition = w.record.find("@ATTACH #R1").unwrap();
    let dot = w.record.find("$DOT @ATTACH").unwrap();
    assert!(definition < dot, "{}", w.record);
    assert!(w.record[dot..].contains("::p"), "{}", w.record);
}

#[test]
fn released_records_ignore_the_new_optional_interface() {
    for version in ["0.2", "0.3"] {
        let r = read(version, "", "%p=(0,0)");
        clean(&r);
        let a = write_record(&r.problems[0], &Solution::default(), "test", "hash");
        let b = write_record(
            &r.problems[0],
            &Solution { junctions: vec![PointRef::Origin(object(&r, "%p"))], ..Default::default() },
            "test",
            "hash",
        );
        assert_eq!(a, b);
    }
}

#[test]
fn an_unnamed_authored_point_gets_a_numbering_check_on_writeback() {
    let input = source("0.4", ON, "(2,3)");
    let r = text(&input);
    clean(&r);
    let p = &r.problems[0];
    let index = p.objects.iter().position(|o| matches!(o.reference, Reference::Number { .. })).unwrap();
    let w =
        write_record(p, &Solution { junctions: vec![PointRef::Origin(index)], ..Default::default() }, "test", "hash");
    assert!(w.unsolved.is_empty());
    assert!(w.record.contains("$DOT @ATTACH %#"), "{}", w.record);
    let files = MemoryProvider::new().with("main.sp", &input);
    let record = rnl_runtime::solution::finish_record(
        &r.doc,
        &files,
        &Options { std: false, ..Default::default() },
        p,
        &w.record,
    );
    assert!(record.contains("numbering="), "{record}");
    let again = text(&(input + &record.lines().map(|s| format!("* {s}\n")).collect::<String>()));
    clean(&again);
    let back = &again.problems[0];
    let dot = back.usages.iter().find(|u| u.definition == "$DOT").unwrap();
    let point = back.objects.iter().position(|o| o.reference == p.objects[index].reference).unwrap();
    assert_eq!(dot.roles[0].members, [Filler::Object(point)]);
}

#[test]
fn an_invalid_pin_index_does_not_write_a_different_terminal() {
    let r = read("0.4", ON, r#"$S= @DEFINE-SYMBOL { (0,0) @PIN(terminal=1) }; #R1 @SET-SYMBOL $S"#);
    clean(&r);
    let point = PointRef::Pin { object: object(&r, "#R1"), pin: 99, terminal: "1".into() };
    let w = write_record(&r.problems[0], &Solution { junctions: vec![point], ..Default::default() }, "test", "hash");
    assert_eq!(w.unsolved.len(), 1);
    assert_eq!(w.unsolved[0].code, codes::UNSOLVED_NOT_IN_PROBLEM);
    assert!(!w.record.contains("@ATTACH"));
}
