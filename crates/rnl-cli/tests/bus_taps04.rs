//! Taps from one point of a bus (13.1, RNL7003 looks-joined): in a file of 0.4, wires that start
//! from a point where a wire that carries all their nets also passes or ends, such as a trunk that
//! ends at a junction and two narrower buses that leave it, are taps of that wire and do not look
//! joined to each other. Two wires of different nets that meet with no such wire there still look
//! joined, and a file of 0.3 reads them as it did.

use std::path::Path;

use rnl_runtime::diag::Severity;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::Options;
use rnl_view::hints::{interpretation_hints, Hint, Kind};

/// The nets `b<hi>` down to `b<lo>`, as a set of RNL.
fn bits(hi: u32, lo: u32) -> String {
    (lo..=hi).rev().map(|k| format!("&b{k}")).collect::<Vec<_>>().join(" ")
}

/// A file of `version` with a resistor on each of the nets b0 to b10, and these wires.
fn hints(version: &str, wires: &str) -> Vec<Hint> {
    let devices: String = (0..=10).map(|k| format!("R{k} b{k} 0 1k\n")).collect();
    let text = format!(
        "* t\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\n{devices}* @RNL {{\n{}\n* }}\n.end\n",
        wires.lines().map(|l| format!("*   {}", l.trim())).collect::<Vec<_>>().join("\n")
    );
    let files = MemoryProvider::new().with("/w/t.sp", &text);
    let r =
        rnl_cli::interpret_file(Path::new("/w/t.sp"), None, &files, &Options { problems: true, ..Options::default() })
            .unwrap();
    let errors: Vec<String> =
        r.diagnostics().iter().filter(|d| d.severity == Severity::Error).map(|d| r.sink.render(d)).collect();
    assert!(errors.is_empty(), "{errors:?}\n{text}");
    interpretation_hints(&r).into_iter().flat_map(|l| l.hints).collect()
}

fn joined(hs: &[Hint]) -> Vec<String> {
    hs.iter().filter(|h| h.kind == Kind::LooksJoined).map(|h| h.message.clone()).collect()
}

/// An 11-bit trunk from (0, 10) down to (0, 0), where it splits: b10 to b3 go left, twice, and b2
/// to b0 go right.
fn split() -> String {
    format!(
        "{{{all}}} @WIRE(mode=\"straight\") {{(0, 10) (0, 0)}};\n\
         {{{high}}} @WIRE(mode=\"straight\") {{(0, 0) (-10, 0)}};\n\
         {{{high}}} @WIRE(mode=\"straight\") {{(0, 0) (0, -10)}};\n\
         {{{low}}} @WIRE(mode=\"straight\") {{(0, 0) (10, 0)}}",
        all = bits(10, 0),
        high = bits(10, 3),
        low = bits(2, 0)
    )
}

#[test]
fn branches_that_leave_the_end_of_a_trunk_do_not_look_joined() {
    let hs = hints("0.4", &split());
    assert_eq!(joined(&hs), Vec::<String>::new(), "{hs:#?}");
}

#[test]
fn branches_from_a_point_inside_a_trunk_do_not_look_joined() {
    let wires = format!(
        "{{{all}}} @WIRE(mode=\"straight\") {{(0, 10) (0, 0) (0, -10)}};\n\
         {{{high}}} @WIRE(mode=\"straight\") {{(0, 0) (-10, 0)}};\n\
         {{{low}}} @WIRE(mode=\"straight\") {{(0, 0) (10, 0)}}",
        all = bits(10, 0),
        high = bits(10, 3),
        low = bits(2, 0)
    );
    assert_eq!(joined(&hints("0.4", &wires)), Vec::<String>::new());
}

#[test]
fn two_buses_that_meet_with_no_trunk_still_look_joined() {
    let wires = format!(
        "{{{high}}} @WIRE(mode=\"straight\") {{(0, 0) (-10, 0)}};\n{{{low}}} @WIRE(mode=\"straight\") {{(0, 0) (10, 0)}}",
        high = bits(10, 3),
        low = bits(2, 0)
    );
    let j = joined(&hints("0.4", &wires));
    assert_eq!(j.len(), 2, "{j:#?}");
    // a wire there whose nets are of one of them only is no trunk of both
    let wires = format!("{wires};\n{{{high}}} @WIRE(mode=\"straight\") {{(0, 0) (0, 10)}}", high = bits(10, 3));
    let j = joined(&hints("0.4", &wires));
    assert_eq!(j.len(), 4, "{j:#?}");
}

#[test]
fn a_trunk_that_lacks_a_net_of_a_branch_is_not_their_trunk() {
    // the trunk carries b10 to b1, not b0: the branches look joined, each to the other
    let wires = split().replacen(&format!("{{{}}}", bits(10, 0)), &format!("{{{}}}", bits(10, 1)), 1);
    let j = joined(&hints("0.4", &wires));
    assert_eq!(j.len(), 4, "{j:#?}");
    assert!(j.iter().all(|m| !m.contains("&b10 &b9 &b8 &b7 &b6 &b5 &b4 &b3 &b2 &b1 from")), "{j:#?}");
}

#[test]
fn a_trunk_and_a_branch_of_another_net_still_look_joined() {
    // b2 to b0 leave the end of a trunk of b10 to b3: they share no net
    let wires = format!(
        "{{{high}}} @WIRE(mode=\"straight\") {{(0, 10) (0, 0)}};\n{{{low}}} @WIRE(mode=\"straight\") {{(0, 0) (10, 0)}}",
        high = bits(10, 3),
        low = bits(2, 0)
    );
    assert_eq!(joined(&hints("0.4", &wires)).len(), 2);
}

#[test]
fn a_file_of_0_3_reads_them_as_it_did() {
    let j = joined(&hints("0.3", &split()));
    assert_eq!(j.len(), 4, "{j:#?}");
}
