use super::*;
use crate::rational::int;
use crate::resolve::tests::{fact, instance, problem_of, Ticks};
use crate::{RunLimit, StoppedClock};
use rnl_runtime::problem::{Axis, FactKind, PointRef, Reference};

#[test]
fn simplification_keeps_electrical_anchors_and_branches_at_coincident_coordinates() {
    let pin = WirePoint::Anchor(PointRef::SymbolPin { object: 0, pin: 0, reference: Reference::Anonymous });
    let points = vec![
        (pin.clone(), [int(0), int(0)]),
        (WirePoint::Bend((0., 0.)), [int(0), int(0)]),
        (WirePoint::Branch(0), [int(0), int(0)]),
        (WirePoint::Bend((0., 1.)), [int(0), int(1)]),
        (WirePoint::Anchor(PointRef::Origin(1)), [int(0), int(2)]),
    ];
    let result = simplified(points);
    assert_eq!(
        result.iter().map(|p| p.0.clone()).collect::<Vec<_>>(),
        vec![pin, WirePoint::Branch(0), WirePoint::Anchor(PointRef::Origin(1))]
    );
}

#[test]
fn strict_compaction_repairs_distribution_and_rejects_unfinished_repairs() {
    let mut p = problem_of(
        vec![instance(), instance(), instance()],
        vec![fact(FactKind::Distribute {
            axis: Axis::X,
            sense: rnl_runtime::problem::Sense::Increasing,
            members: (0..3).map(PointRef::Origin).collect(),
        })],
    );
    p.version = "0.4".into();
    let options = SolverOptions::without_time_limit();
    let resolved = crate::resolve::resolve(&p, &options, &StoppedClock);
    let s = crate::structure::analyse(&p, &resolved, &options, crate::options::CAPS, &mut Work::default());
    let mut geometry = s.witness.clone();
    for (k, x) in [0, 20, 50].into_iter().enumerate() {
        geometry.origin[k] = [int(x), int(0)];
    }
    let placed = Placed {
        origins: s.origins(&p, &geometry),
        geometry,
        dropped: vec![],
        overlaps: vec![],
        optimised: true,
        undecided: false,
    };
    let routing = Routing::default();
    assert!(read::read(&p, &s, &placed.geometry, &routing, &|_| true, false, false, None, false).is_none());
    let run = || compact_strict(&p, &s, &placed, &routing, &options, &mut Work::default(), None, &[]);
    let full = run();
    assert!(!full.failed && !full.interrupted, "{:?}", full.notes);
    let equal = |made: &Placed| &made.origins[1][0] - &made.origins[0][0] == &made.origins[2][0] - &made.origins[1][0];
    assert!(equal(&full.placed));
    let clock = Ticks::default();
    let (measured, _) = crate::budget::with(&clock, RunLimit::Time(f64::MAX), None, run);
    assert_eq!(measured, full);
    let end = clock.reads();
    let mut rejected = 0;
    let mut accepted = 0;
    // Include late cuts through output validation, not only skeleton preparation.
    for seconds in (0..160).chain(end.saturating_sub(12)..=end) {
        let (timed, used) = crate::budget::with(&Ticks::default(), RunLimit::Time(seconds as f64), None, run);
        let (replayed, _) = crate::budget::with(&StoppedClock, RunLimit::Work(used.work), None, run);
        assert_eq!(timed, replayed, "cutoff {seconds}, work {}", used.work);
        if timed.failed || timed.interrupted {
            rejected += 1;
            assert_eq!(timed.placed, placed, "a failed repair must not expose partial coordinates");
        } else {
            accepted += 1;
            assert!(equal(&timed.placed), "cutoff {seconds}");
        }
    }
    assert!(rejected > 0);
    assert!(accepted > 0);
}

#[test]
fn compaction_does_not_drop_an_unsatisfied_disjunctive_fact() {
    let mut p = problem_of(
        vec![instance(), instance()],
        vec![fact(FactKind::Distance {
            axis: Axis::X,
            sense: rnl_runtime::problem::Sense::Either,
            min: Some(10.),
            max: None,
            from: vec![PointRef::Origin(0)],
            to: vec![PointRef::Origin(1)],
        })],
    );
    p.version = "0.4".into();
    let options = SolverOptions::without_time_limit();
    let resolved = crate::resolve::resolve(&p, &options, &StoppedClock);
    let s = crate::structure::analyse(&p, &resolved, &options, crate::options::CAPS, &mut Work::default());
    assert_eq!(s.kept.len(), 1);
    assert!(!s.model.units[0].choices.is_empty());
    let mut geometry = s.witness.clone();
    geometry.origin[0] = [int(0), int(0)];
    geometry.origin[1] = [int(1), int(0)];
    let placed = Placed {
        origins: s.origins(&p, &geometry),
        geometry,
        dropped: vec![],
        overlaps: vec![],
        optimised: true,
        undecided: false,
    };
    let made = compact_strict(&p, &s, &placed, &Routing::default(), &options, &mut Work::default(), None, &[]);
    assert!(made.failed);
    assert_eq!(made.placed, placed);
}

#[test]
fn compaction_keeps_distribution_and_replays_every_interrupted_pass() {
    let mut p = problem_of(
        vec![instance(), instance(), instance()],
        vec![fact(FactKind::Distribute {
            axis: Axis::X,
            sense: rnl_runtime::problem::Sense::Increasing,
            members: (0..3).map(PointRef::Origin).collect(),
        })],
    );
    p.version = "0.4".into();
    let options = SolverOptions::without_time_limit();
    let resolved = crate::resolve::resolve(&p, &options, &StoppedClock);
    let s = crate::structure::analyse(&p, &resolved, &options, crate::options::CAPS, &mut Work::default());
    let mut g = s.witness.clone();
    for (k, origin) in g.origin[..3].iter_mut().enumerate() {
        *origin = [int(20 * k as i64), int(0)];
    }
    let placed = Placed {
        origins: s.origins(&p, &g),
        geometry: g,
        dropped: vec![],
        overlaps: vec![],
        optimised: true,
        undecided: false,
    };
    let routing = Routing::default();
    for strict in [false, true] {
        let run = || {
            if strict {
                let made = compact_strict(&p, &s, &placed, &routing, &options, &mut Work::default(), None, &[]);
                let mut notes = made.notes;
                notes.push(format!("failed={}, interrupted={}", made.failed, made.interrupted));
                (made.placed, made.routing, notes)
            } else {
                compact(&p, &s, &placed, &routing, &|_| true, &options, &mut Work::default())
            }
        };
        let full = run();
        assert!(full.0.origins[2][0].clone() - &full.0.origins[0][0] < int(40), "{:?}", full.2);
        for seconds in 0..100 {
            let (timed, used) = crate::budget::with(&Ticks::default(), RunLimit::Time(seconds as f64), None, run);
            let (replayed, _) = crate::budget::with(&StoppedClock, RunLimit::Work(used.work), None, run);
            assert_eq!(timed, replayed, "cutoff {seconds}, work {}", used.work);
            let xs: Vec<_> = timed.0.origins.iter().map(|q| q[0].clone()).collect();
            assert_eq!(&xs[1] - &xs[0], &xs[2] - &xs[1], "cutoff {seconds}");
            assert_eq!(timed.0.geometry.orient, placed.geometry.orient);
            assert_eq!(timed.1.wires.len(), routing.wires.len());
        }
    }
}

#[test]
fn centre_positioned_pin_keeps_its_fact_and_matches_the_written_wire() {
    use rnl_runtime::geom::Orient;
    let mut objects = vec![instance(), instance(), instance()];
    objects[2].position = Position::At(PointRef::Centre(vec![PointRef::Origin(0), PointRef::Origin(1)]));
    let pin = PointRef::SymbolPin { object: 2, pin: 0, reference: Reference::Anonymous };
    let mut facts: Vec<_> = (0..3).map(|object| fact(FactKind::Orient { object, code: Orient::R0 })).collect();
    facts.extend([Axis::X, Axis::Y].map(|axis| fact(FactKind::Place { at: pin.clone(), axis, value: 8. })));
    let mut p = problem_of(objects, facts);
    p.version = "0.4".into();
    let options = SolverOptions::without_time_limit();
    let resolved = crate::resolve::resolve(&p, &options, &StoppedClock);
    let s = crate::structure::analyse(&p, &resolved, &options, crate::options::CAPS, &mut Work::default());
    assert_eq!(s.kept.len(), p.facts.len());
    let mut g = s.witness.clone();
    g.origin[0] = [int(0), int(0)];
    g.origin[1] = [int(22), int(16)];
    let placed = Placed {
        origins: s.origins(&p, &g),
        geometry: g,
        dropped: vec![],
        overlaps: vec![],
        optimised: true,
        undecided: false,
    };
    let mut routing = Routing::default();
    routing
        .wires
        .push(RoutedWire { route: 0, points: vec![WirePoint::Anchor(pin.clone()), WirePoint::Bend((0., 8.))] });
    routing.paths.push(vec![[int(8), int(8)], [int(0), int(8)]]);
    routing.drawn.push(0);
    let (made, wires, notes) = compact(&p, &s, &placed, &routing, &|_| true, &options, &mut Work::default());
    assert!(notes.iter().any(|n| n.contains("x pass")), "{notes:?}");
    for (axis, expected) in wires.paths[0][0].iter().enumerate() {
        let actual = crate::resolve::point_value(&p, &made.geometry, &pin, axis, 0).unwrap();
        assert_eq!(actual, int(8), "axis {axis}: {notes:?}");
        assert_eq!(&actual, expected);
    }
}

#[test]
fn group_spacing_applies_when_the_level_spacing_is_zero() {
    use rnl_runtime::problem::ProblemGroup;
    let mut p = problem_of(vec![instance(), instance(), instance()], vec![]);
    p.version = "0.4".into();
    p.groups.push(ProblemGroup {
        identity: 0,
        first: true,
        direct: true,
        spacing_x: Some(5.),
        spacing_y: None,
        label: "inner".into(),
        members: vec![0, 1],
    });
    let options = SolverOptions::without_time_limit();
    let resolved = crate::resolve::resolve(&p, &options, &StoppedClock);
    let s = crate::structure::analyse(&p, &resolved, &options, crate::options::CAPS, &mut Work::default());
    let mut g = s.witness.clone();
    for (k, x) in [0, 5, 40].into_iter().enumerate() {
        g.origin[k] = [int(x), int(0)];
    }
    let placed = Placed {
        origins: s.origins(&p, &g),
        geometry: g,
        dropped: vec![],
        overlaps: vec![],
        optimised: true,
        undecided: false,
    };
    let routing = Routing::default();
    let run = || {
        let made = space::space(&p, &s, &placed, &routing, [0, 0], &options, &mut Work::default());
        (made.placed, made.routing, made.notes, made.moved)
    };
    let full = run();
    assert_eq!(&full.0.origins[1][0] - &full.0.origins[0][0], int(10), "{:?}", full.2);
    assert_eq!(full.0.origins.iter().map(|p| p[1].clone()).collect::<Vec<_>>(), vec![int(0); 3]);
    for seconds in 0..80 {
        let (timed, used) = crate::budget::with(&Ticks::default(), RunLimit::Time(seconds as f64), None, run);
        let (replayed, _) = crate::budget::with(&StoppedClock, RunLimit::Work(used.work), None, run);
        assert_eq!(timed, replayed, "cutoff {seconds}, work {}", used.work);
    }
}

#[test]
fn an_inner_zero_spacing_keeps_its_gap_while_the_outer_group_expands() {
    use rnl_runtime::problem::ProblemGroup;
    let mut p = problem_of(vec![instance(); 4], vec![]);
    p.version = "0.4".into();
    for (identity, members, extra) in [(0, vec![0, 1, 2, 3], 8.), (1, vec![0, 1], 0.)] {
        p.groups.push(ProblemGroup {
            identity,
            first: true,
            direct: true,
            spacing_x: Some(extra),
            spacing_y: None,
            label: format!("group {identity}"),
            members,
        });
    }
    let options = SolverOptions::without_time_limit();
    let resolved = crate::resolve::resolve(&p, &options, &StoppedClock);
    let s = crate::structure::analyse(&p, &resolved, &options, crate::options::CAPS, &mut Work::default());
    let mut g = s.witness.clone();
    for (k, x) in [0, 5, 40, 45].into_iter().enumerate() {
        g.origin[k] = [int(x), int(0)];
    }
    let placed = Placed {
        origins: s.origins(&p, &g),
        geometry: g,
        dropped: vec![],
        overlaps: vec![],
        optimised: true,
        undecided: false,
    };
    let made = space::space(&p, &s, &placed, &Routing::default(), [0, 0], &options, &mut Work::default());
    assert_eq!(&made.placed.origins[1][0] - &made.placed.origins[0][0], int(5), "{:?}", made.notes);
    assert_eq!(&made.placed.origins[3][0] - &made.placed.origins[2][0], int(13), "{:?}", made.notes);
}

#[test]
fn nested_anonymous_definition_coordinates_remain_relative_after_each_compaction() {
    use num_traits::Zero;
    use rnl_runtime::{diag::Severity, source::MemoryProvider, Options};
    let text = r#"* nested relative coordinates
* @RNL(type="header") { @VERSION("0.4") }
R1 a b 1k
R2 b c 1k
* @RNL {
* $S = @DEFINE-SYMBOL { (-1,-1) (1,1) @RECTANGLE };
* {#R1 #R2} @SET-SYMBOL $S;
* $INNER = @DEFINE(roles=["a"]) { ?a @PLACE(2,3); %p = (-1,4) };
* $OUTER = @DEFINE(roles=["a"]) { ?a @AS $INNER => @PLACE(5,-6) };
* %u = #R1 @AS $OUTER;
* #R2 @AS $OUTER;
* %u @PLACE(10,20)
* }
.end
"#;
    let files = MemoryProvider::new().with("nested.sp", text);
    let interpreted =
        rnl_view::interpret("nested.sp", None, &files, &Options { std: false, ..Options::solver() }).unwrap();
    assert!(interpreted.diagnostics().iter().all(|d| d.severity != Severity::Error), "{:?}", interpreted.diagnostics());
    let p = &interpreted.problems[0];
    assert_eq!(p.usages.iter().filter(|u| u.parent.is_some()).count(), 2);
    assert!(p.facts.iter().filter(|f| !f.chain.is_empty()).count() >= 12);
    for layout in [crate::Layout::Rows, crate::Layout::Tree] {
        let mut options = SolverOptions::without_time_limit();
        options.settings.layout = layout;
        let (solved, exact) = crate::driver::solve_exact(p, &options);
        assert_eq!(solved.report.kept.len(), p.facts.len());
        let s = &exact.structure;
        let compacted = compact(p, s, &exact.placed, &exact.routing, &|_| true, &options, &mut Work::default());
        assert!(compacted.2.iter().any(|n| n.contains("x pass")), "{:?}", compacted.2);
        let strict = compact_strict(p, s, &exact.placed, &exact.routing, &options, &mut Work::default(), None, &[]);
        assert!(!strict.failed, "{:?}", strict.notes);
        let spaced = space::space(p, s, &exact.placed, &exact.routing, [4, 2], &options, &mut Work::default());
        assert!(!spaced.notes.is_empty());
        for made in [compacted.0, strict.placed, spaced.placed] {
            let r1 = p.objects.iter().position(|o| o.label == "#R1").unwrap();
            assert_eq!(made.origins[r1], [int(17), int(17)]);
            for &f in &s.kept {
                let at = |point: &PointRef, axis| {
                    crate::resolve::point_value(p, &made.geometry, point, usize::from(axis == Axis::Y), 0)
                };
                assert_eq!(
                    crate::holds::check(
                        &p.facts[f].kind,
                        &at,
                        &|k| Some(made.geometry.orient[k]),
                        &Q::zero(),
                        &Q::zero()
                    ),
                    Some(true),
                    "{layout:?}: {:?}",
                    p.facts[f]
                );
            }
        }
    }
}

#[test]
fn strict_swaps_make_45_degree_crosses_or_keep_the_routed_shape_when_facts_prevent_them() {
    use crate::resolve::tests::point;
    use rnl_runtime::problem::RouteIntent;
    let positions = [[-10, 0], [10, 6], [-10, 8], [10, 2]];
    for fixed in [false, true] {
        let facts = if fixed {
            positions
                .iter()
                .enumerate()
                .flat_map(|(k, xy)| {
                    [Axis::X, Axis::Y].into_iter().enumerate().map(move |(a, axis)| {
                        fact(FactKind::Place { at: PointRef::Origin(k), axis, value: xy[a] as f64 })
                    })
                })
                .collect()
        } else {
            Vec::new()
        };
        let mut p = problem_of(vec![point(); 4], facts);
        p.version = "0.4".into();
        for (i, pins) in [[0, 1], [2, 3]].into_iter().enumerate() {
            p.routes.push(RouteIntent {
                tree: true,
                net: format!("net{i}"),
                mode: "manhattan".into(),
                style: vec![],
                pins: pins.map(PointRef::Origin).to_vec(),
                wires: vec![],
            });
        }
        let options = SolverOptions::without_time_limit();
        let resolved = crate::resolve::resolve(&p, &options, &StoppedClock);
        let s = crate::structure::analyse(&p, &resolved, &options, crate::options::CAPS, &mut Work::default());
        let mut g = s.witness.clone();
        for (k, xy) in positions.into_iter().enumerate() {
            g.origin[k] = xy.map(int);
        }
        let placed = Placed {
            origins: s.origins(&p, &g),
            geometry: g,
            dropped: vec![],
            overlaps: vec![],
            optimised: true,
            undecided: false,
        };
        let mut routing = Routing::default();
        for (i, (objects, x)) in [([0, 1], -2), ([2, 3], 2)].into_iter().enumerate() {
            let a = positions[objects[0]];
            let b = positions[objects[1]];
            let path = [a, [x, a[1]], [x, b[1]], b];
            routing.paths.push(path.map(|xy| xy.map(int)).to_vec());
            routing.wires.push(RoutedWire {
                route: i,
                points: vec![
                    WirePoint::Anchor(PointRef::Origin(objects[0])),
                    WirePoint::Bend((x as f64, a[1] as f64)),
                    WirePoint::Bend((x as f64, b[1] as f64)),
                    WirePoint::Anchor(PointRef::Origin(objects[1])),
                ],
            });
            routing.drawn.push(i);
        }
        assert_eq!(cross::find(&p, &s, &placed.geometry, &routing, None).len(), 1);
        if !fixed {
            // A text belonging to a point (not a symbol component) occupies the swap box.
            let mut covered = s.clone();
            covered.offset_labels.insert(0, vec![[int(9), int(3), int(11), int(5)]]);
            assert!(cross::find(&p, &covered, &placed.geometry, &routing, None).is_empty());
            use rnl_runtime::problem::{Filler, Slot, TextInfo};
            let mut with_text = p.clone();
            with_text.objects.push(instance());
            let mut text = crate::resolve::tests::label(Some(Axis::X));
            text.position = Position::InSymbol { owner: 4, local: (20., 0.), replaced: Some((Axis::X, Slot::Free)) };
            text.text = Some(TextInfo {
                attach: Filler::Object(4),
                key: Some("name".into()),
                lines: vec![vec![rnl_runtime::markup::Run::Text { text: "X".into(), style: Default::default() }]],
                halign: "center".into(),
                valign: "center".into(),
                rotate: 0.,
                fix_rotate: false,
                upright: true,
                baseline_mirrors: true,
                column_position: None,
                style: vec![],
            });
            with_text.objects.push(text);
            for (object, axis, value) in [(4, Axis::X, -20.), (4, Axis::Y, 4.), (5, Axis::X, 0.)] {
                with_text.facts.push(fact(FactKind::Place { at: PointRef::Origin(object), axis, value }));
            }
            let resolved = crate::resolve::resolve(&with_text, &options, &StoppedClock);
            let text_structure =
                crate::structure::analyse(&with_text, &resolved, &options, crate::options::CAPS, &mut Work::default());
            let mut geometry = text_structure.witness.clone();
            for (k, xy) in positions.into_iter().enumerate() {
                geometry.origin[k] = xy.map(int);
            }
            assert!(!text_structure.text_offsets(&with_text, &geometry, 5).is_empty());
            assert!(cross::find(&with_text, &text_structure, &geometry, &routing, None).is_empty());
            let sk = read::read(&with_text, &text_structure, &geometry, &routing, &|_| true, false, false, None, false)
                .unwrap();
            assert!(
                sk.features.iter().any(|f| f.kind == read::Kind::Text(5)),
                "replaced-axis text must constrain compaction too"
            );
        }
        let run = || compact_strict(&p, &s, &placed, &routing, &options, &mut Work::default(), None, &[]);
        let made = run();
        assert!(!made.failed, "fixed={fixed}: {:?}", made.notes);
        let mut spaced_options = options.clone();
        spaced_options.settings.spacing = [3, 2];
        let mut spacing_notes = Vec::new();
        let final_drawing = spaced(&p, &s, made.clone(), &spaced_options, &mut Work::default(), &mut spacing_notes);
        assert!(!spacing_notes.is_empty());
        for (tree, objects) in [[0, 1], [2, 3]].into_iter().enumerate() {
            for object in objects {
                let before = positions[object].map(int);
                let after = &final_drawing.placed.origins[object];
                assert!(
                    final_drawing.moved.iter().any(|(t, pre, post)| *t == tree && *pre == before && post == after),
                    "tree={tree}, object={object}: {:?}",
                    final_drawing.moved
                );
            }
        }
        let diagonals: Vec<_> = made
            .routing
            .paths
            .iter()
            .flat_map(|p| p.windows(2))
            .filter(|p| p[0][0] != p[1][0] && p[0][1] != p[1][1])
            .collect();
        if fixed {
            assert!(diagonals.is_empty(), "{:?}", made.notes);
            assert_eq!(made.placed.origins, placed.origins);
            assert!(made.notes.iter().any(|n| n.contains("swaps drawn as routed")), "{:?}", made.notes);
        } else {
            assert_eq!(diagonals.len(), 2, "{:?}", made.notes);
            for pair in diagonals {
                assert_eq!((&pair[1][0] - &pair[0][0]).abs(), (&pair[1][1] - &pair[0][1]).abs());
            }
        }
        for seconds in [1., 4., 16., 64., 128., 256., 512., 1024.] {
            let (timed, used) = crate::budget::with(&Ticks::default(), RunLimit::Time(seconds), None, run);
            let (replayed, _) = crate::budget::with(&StoppedClock, RunLimit::Work(used.work), None, run);
            assert_eq!(timed, replayed, "fixed={fixed}, cutoff={seconds}");
            if fixed {
                assert_eq!(timed.placed.origins, placed.origins);
            }
            for (wire, path) in timed.routing.wires.iter().zip(&timed.routing.paths) {
                for (point, coordinates) in wire.points.iter().zip(path) {
                    if let WirePoint::Anchor(reference) = point {
                        for (axis, coordinate) in coordinates.iter().enumerate() {
                            assert_eq!(
                                Some(coordinate.clone()),
                                crate::resolve::point_value(&p, &timed.placed.geometry, reference, axis, 0)
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn spacing_shrinks_to_its_floor_and_grows_only_as_far_as_retained_facts_allow() {
    use rnl_runtime::problem::Sense;
    for fixed in [false, true] {
        let mut facts = vec![fact(FactKind::Distribute {
            axis: Axis::X,
            sense: Sense::Increasing,
            members: (0..3).map(PointRef::Origin).collect(),
        })];
        if fixed {
            facts.push(fact(FactKind::Distance {
                axis: Axis::X,
                sense: Sense::Increasing,
                min: Some(8.),
                max: Some(8.),
                from: vec![PointRef::Origin(0)],
                to: vec![PointRef::Origin(1)],
            }));
        }
        let mut p = problem_of(vec![instance(); 3], facts);
        p.version = "0.4".into();
        let options = SolverOptions::without_time_limit();
        let resolved = crate::resolve::resolve(&p, &options, &StoppedClock);
        let s = crate::structure::analyse(&p, &resolved, &options, crate::options::CAPS, &mut Work::default());
        assert_eq!(s.kept.len(), p.facts.len());
        let mut g = s.witness.clone();
        for (k, origin) in g.origin[..3].iter_mut().enumerate() {
            *origin = [int(8 * k as i64), int(0)];
        }
        let placed = Placed {
            origins: s.origins(&p, &g),
            geometry: g,
            dropped: vec![],
            overlaps: vec![],
            optimised: true,
            undecided: false,
        };
        let made = space::space(
            &p,
            &s,
            &placed,
            &Routing::default(),
            [if fixed { 8 } else { -1 }, 0],
            &options,
            &mut Work::default(),
        );
        let expected = int(if fixed { 8 } else { 4 });
        for pair in made.placed.origins.windows(2) {
            assert_eq!(&pair[1][0] - &pair[0][0], expected, "fixed={fixed}: {:?}", made.notes);
        }
        if fixed {
            assert!(made.notes.iter().any(|n| n.contains("facts that held gaps")), "{:?}", made.notes);
        }
    }
}
