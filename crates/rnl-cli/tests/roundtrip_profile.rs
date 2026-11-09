//! Dedicated 0.3 solver-record roundtrips. The corpus and reviewed expected files live in
//! tests/roundtrip; RNL_BLESS_ROUNDTRIP=1 updates only this corpus.

use std::path::Path;

use rnl_cli::{dump, interpret_file};
use rnl_runtime::diag::Severity;
use rnl_runtime::geom::{Orient, Pt};
use rnl_runtime::problem::{Filler, LevelProblem, PointRef, Position, Slot};
use rnl_runtime::solution::{write_record, Placement, Solution, WirePoint, WireRoute};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::writeback::{replace_generated, strip_generated};
use rnl_runtime::{Interpretation, Options};

fn read(file: &str, text: &str, solver: bool) -> Interpretation {
    let options = Options { std: false, problems: true, generated: !solver, ..Options::default() };
    let r = interpret_file(Path::new(file), None, &MemoryProvider::new().with(file, text), &options).unwrap();
    let errors: Vec<_> =
        r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).map(|d| r.sink.render(d)).collect();
    assert!(errors.is_empty(), "{file}: {errors:?}");
    r
}

/// A deterministic solution supplied to the writer, independent of solver search and timeouts.
/// The snapshots check its exact geometry, text and priorities after interpretation.
fn solution(p: &LevelProblem) -> Solution {
    let mut placements = Vec::new();
    for (object, o) in p.objects.iter().enumerate() {
        if let Position::Own { x, y } = &o.position {
            let fixed = |s: &Slot, fallback| match s {
                Slot::Fixed { value, .. } => *value,
                _ => fallback,
            };
            let at = if o.generated { (0.0, -6.0) } else { (object as f64 * 10.0, object as f64 * 4.0) };
            placements.push(Placement {
                object,
                x: Some(fixed(x, at.0)),
                y: Some(fixed(y, at.1)),
                orient: o.orient.as_ref().map(|_| Orient::R0),
            });
        }
    }
    // Keep the fixture's keyed label attached on x; the record must write only its free y.
    for (object, o) in p.objects.iter().enumerate() {
        if let Some(t) = &o.text {
            if t.key.as_deref() == Some("note") {
                if let Filler::Object(owner) = t.attach {
                    let x = placements.iter().find(|pl| pl.object == owner).unwrap().x.unwrap() + 1.0;
                    placements.iter_mut().find(|pl| pl.object == object).unwrap().x = Some(x);
                }
            }
        }
    }
    let mut s = Solution { placements, ..Solution::default() };
    for (wire, w) in p.wires.iter().enumerate().filter(|(_, w)| w.mode == "auto") {
        let mut points = vec![WirePoint::Anchor(w.anchors[0].clone())];
        for pair in w.anchors.windows(2) {
            let (a, b) = (point(p, &s, &pair[0]), point(p, &s, &pair[1]));
            points.push(WirePoint::Bend((a.0, b.1)));
            points.push(WirePoint::Anchor(pair[1].clone()));
        }
        s.wire_routes.push(WireRoute { wire, points });
    }
    s
}

fn point(p: &LevelProblem, s: &Solution, q: &PointRef) -> Pt {
    let at = |object| s.placements.iter().find(|pl| pl.object == object).unwrap().at().unwrap();
    match q {
        PointRef::Origin(object) => at(*object),
        PointRef::Pin { object, pin, .. } | PointRef::SymbolPin { object, pin, .. } => {
            let a = at(*object);
            let b = p.symbols[p.objects[*object].symbol.unwrap()].pins[*pin].pos;
            (a.0 + b.0, a.1 + b.1)
        }
        PointRef::Centre(_) => panic!("this corpus does not route through set centres"),
    }
}

fn priorities(r: &Interpretation, level: usize) -> Vec<(String, String)> {
    let mut v: Vec<_> = r
        .model
        .levels
        .iter()
        .find(|l| l.level == level)
        .unwrap()
        .constraints
        .iter()
        .map(|c| (c.kind.clone(), c.priority.clone()))
        .collect();
    v.sort();
    v
}

fn expected(path: &Path, actual: &str) {
    if std::env::var_os("RNL_BLESS_ROUNDTRIP").is_some() {
        std::fs::write(path, actual).unwrap();
    } else {
        let want = std::fs::read_to_string(path)
            .unwrap_or_else(|_| panic!("{} missing; run with RNL_BLESS_ROUNDTRIP=1", path.display()));
        assert_eq!(want.replace("\r\n", "\n"), actual, "{}", path.display());
    }
}

#[test]
fn dedicated_cases_preserve_the_written_result_on_reread_and_replacement() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/roundtrip");
    let mut cases: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "sp" || e == "scs"))
        .collect();
    cases.sort();
    assert_eq!(cases.len(), 6);
    for path in cases {
        let file = path.file_name().unwrap().to_str().unwrap();
        let source = std::fs::read_to_string(&path).unwrap().replace("\r\n", "\n");
        for (eol, final_newline) in [("\n", true), ("\r\n", true), ("\n", false)] {
            let text = if final_newline { source.clone() } else { source.trim_end_matches('\n').to_string() }
                .replace('\n', eol);
            let before = read(file, &text, true);
            let p = before.problems.iter().find(|p| !p.wires.is_empty()).unwrap();
            let s = solution(p);
            let written = write_record(p, &s, "roundtrip", &rnl_runtime::problem::sig2(p, "roundtrip-v1"));
            assert!(written.unsolved.is_empty(), "{file}: {:?}", written.unsolved);
            assert!(written.notes.is_empty(), "{file}: {:?}", written.notes);
            let files = MemoryProvider::new().with(file, &text);
            let options = Options { std: false, ..Options::default() };
            let record = rnl_view::finish_record(&before.doc, &files, &options, p, &written.record);
            assert!(record.contains("numbering=\""), "{file}: {record}");
            assert!(record.contains("@WIRE(mode=\"straight\")"), "{file}: {record}");
            let output = replace_generated(&before.doc, p.level, &text, &record);
            let after = read(file, &output, false);
            assert_eq!(
                priorities(&before, p.level),
                priorities(&after, p.level),
                "{file}: usage body priorities changed"
            );
            assert_eq!(
                strip_generated(&before.doc, file, &text),
                strip_generated(&after.doc, file, &output),
                "{file}: authored text changed"
            );
            if eol == "\r\n" {
                assert!(!output.replace("\r\n", "").contains('\n'), "{file}: mixed line endings");
            }
            let hints = rnl_runtime::previous::previous(p, &after).unwrap();
            assert_eq!(hints.numbering, Some(true), "{file}: stale numbering");
            assert_eq!(hints.same_input, Some(true), "{file}: stale input hash");
            for route in &s.wire_routes {
                let want: Vec<_> = route
                    .points
                    .iter()
                    .map(|q| match q {
                        WirePoint::Anchor(a) => point(p, &s, a),
                        WirePoint::Bend(b) => *b,
                        WirePoint::Branch(_) => unreachable!(),
                    })
                    .collect();
                let got = hints.wire_routes.iter().find(|r| r.wire == route.wire).unwrap();
                assert_eq!(got.points, want, "{file}: lost a route");
            }
            // Read again as a solver, excluding its previous output; rewrite from that fresh
            // problem. Byte idempotence and the entire effective model must both hold.
            let fresh = read(file, &output, true);
            let q = fresh.problems.iter().find(|q| q.level == p.level).unwrap();
            let next = write_record(q, &solution(q), "roundtrip", &rnl_runtime::problem::sig2(q, "roundtrip-v1"));
            assert!(next.unsolved.is_empty() && next.notes.is_empty());
            let files = MemoryProvider::new().with(file, &output);
            let next = rnl_view::finish_record(&fresh.doc, &files, &options, q, &next.record);
            assert_eq!(record, next, "{file}: unstable record");
            let twice = replace_generated(&fresh.doc, q.level, &output, &next);
            assert_eq!(output, twice, "{file}: replacement not idempotent");
            assert_eq!(
                dump::conformance(&after),
                dump::conformance(&read(file, &twice, false)),
                "{file}: effective model changed"
            );
            if eol == "\n" && final_newline {
                expected(&path.with_extension("record.rnl"), &(record + "\n"));
                expected(
                    &path.with_extension("expected.json"),
                    &(serde_json::to_string_pretty(&dump::conformance(&after)).unwrap() + "\n"),
                );
            }
        }
    }
}
