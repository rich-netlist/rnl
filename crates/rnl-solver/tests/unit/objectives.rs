use super::*;
use crate::{Layer, SolverOptions};

#[test]
fn cached_view_scores_match_reference_for_all_orientations_and_parameter_variants() {
    use crate::resolve::tests::{instance, point, problem_of};
    use crate::routing_geometry::{GeometryPointId, GeometrySource, GeometryVertex, GeometryWire, OutputWireId};
    use rnl_runtime::problem::{PointRef, ProblemWire, Reference, RouteIntent};
    let pin = |object, pin| PointRef::Pin { object, pin, terminal: format!("p{pin}") };
    for (mode, case) in ["straight", "horizontal-vertical", "manhattan", "auto"]
        .into_iter()
        .flat_map(|mode| (0..4).map(move |case| (mode, case)))
    {
        let mut p = problem_of(vec![instance(), instance(), point(), point()], Vec::new());
        p.wires.push(ProblemWire {
            label: "%w".into(),
            reference: Reference::Anonymous,
            nets: vec!["a".into()],
            bus: false,
            anchors: vec![
                pin(0, 0),
                PointRef::Centre(vec![PointRef::Origin(2), PointRef::Origin(3)]),
                PointRef::Origin(2),
                PointRef::Origin(3),
                pin(1, 1),
            ],
            mode: mode.into(),
            generated: false,
        });
        p.routes.push(RouteIntent {
            net: "b".into(),
            mode: "manhattan".into(),
            style: Vec::new(),
            pins: vec![pin(0, 1), pin(1, 0), pin(1, 0)],
            wires: Vec::new(),
        });
        if case == 0 {
            p.wires.clear();
        }
        if case == 1 {
            p.routes.clear();
        }
        if case == 3 {
            p.routes[0].net = "a".into();
            p.routes[0].wires = vec![0];
        }
        p.routes.push(RouteIntent {
            net: "empty".into(),
            mode: "manhattan".into(),
            style: Vec::new(),
            pins: Vec::new(),
            wires: Vec::new(),
        });
        let mut options = SolverOptions::without_time_limit();
        options.objectives.layers.push(Layer {
            items: vec![
                Term { ratio: frac(3, 7), ..Term::new("aspect") },
                Term { ratio: frac(9, 2), ..Term::new("aspect") },
                Term { scope: ObjectiveScope::Global, ..Term::new("symbol_alignment") },
                Term { scope: ObjectiveScope::Global, ..Term::new("symbol_distribution") },
                Term {
                    direction: Some(crate::Direction::Left),
                    symbols: vec![p.symbols[0].name.clone()],
                    ..Term::new("symbol_direction")
                },
                Term {
                    direction: Some(crate::Direction::Right),
                    symbols: vec![p.symbols[0].name.clone()],
                    ..Term::new("symbol_direction")
                },
            ],
            tolerance: frac(1, 13),
        });
        let resolved = crate::resolve(&p, &options);
        let mut s =
            crate::structure::analyse(&p, &resolved, &options, crate::options::CAPS, &mut crate::Work::default());
        s.scopes = vec![vec![0, 1]];
        s.labels.insert(0, vec![[int(-2), int(1), int(2), int(3)]; 8]);
        let at = vec![[int(0), int(0)], [frac(3, 2), int(1)], [int(4), int(2)], [int(4), int(2)]];
        let mut old_scores = Vec::new();
        let mut new_scores = Vec::new();
        for orientation in Orient::ALL {
            let orient = vec![orientation, orientation.then(Orient::MY), Orient::R0, Orient::R0];
            let view = View { s: &s, at: &at, orient: &orient };
            let g = crate::resolve::Geometry { origin: at.clone(), orient: orient.clone() };
            let mut routing = crate::route::Routing::default();
            routing
                .geometry
                .wires
                .extend((0..p.wires.len()).map(|i| crate::routing_geometry::authored_wire(&p, &g, i).unwrap()));
            let id = OutputWireId::Generated(0);
            routing.geometry.wires.push(GeometryWire {
                id,
                source: GeometrySource::Generated,
                nets: ["b".into()].into_iter().collect(),
                vertices: [[-4, 0], [4, 0], [4, 3]]
                    .into_iter()
                    .enumerate()
                    .map(|(index, xy)| GeometryVertex {
                        at: xy.map(int),
                        point: GeometryPointId::Bend { wire: id, index },
                        physical_pin: None,
                    })
                    .collect(),
            });
            let routed = crate::score::drawn(&p, &s, &g, &routing);
            for geometry in [None, Some(&routed)] {
                let scoring = Scoring::new(view, geometry);
                for term in s.active.terms() {
                    assert_eq!(
                        scoring.term(term),
                        score_term(term, &view, geometry),
                        "{mode}/{orientation:?}/{} {:?}",
                        term.kind,
                        term.parameters()
                    );
                }
                let old: Vec<Q> = s
                    .active
                    .layers
                    .iter()
                    .map(|l| l.terms.iter().map(|t| score_term(t, &view, geometry) * &t.weight).sum())
                    .collect();
                let new = s.active.score(&view, geometry);
                assert_eq!(new, old);
                old_scores.push((old, vec![u32::from(crate::orient::index(orientation))]));
                new_scores.push((new, vec![u32::from(crate::orient::index(orientation))]));
                let report = s.active.report(&view, geometry, &options);
                for (l, actual) in s.active.layers.iter().zip(report.levels) {
                    for (t, actual) in l.terms.iter().zip(actual.terms) {
                        assert_eq!(actual.raw, crate::rational::exact_text(&score_term(t, &view, geometry)));
                    }
                }
            }
        }
        assert_eq!(ranked(&s.active, &new_scores), ranked(&s.active, &old_scores));
    }
}

#[test]
fn cached_symbol_overlap_preserves_zero_width_zero_height_and_point_boxes() {
    use crate::resolve::tests::{instance, problem_of};
    for bbox in [[0.0, 0.0, 0.0, 1.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 0.0]] {
        let mut p = problem_of(vec![instance(), instance()], Vec::new());
        let mut enclosing = p.symbols[0].clone();
        enclosing.bbox = [-5.0, -5.0, 5.0, 5.0];
        p.symbols.push(enclosing);
        p.objects[1].symbol = Some(1);
        p.symbols[0].bbox = bbox;
        let options = SolverOptions::without_time_limit();
        let resolved = crate::resolve(&p, &options);
        let s = crate::structure::analyse(&p, &resolved, &options, crate::options::CAPS, &mut crate::Work::default());
        let at = vec![[int(0), int(0)]; 2];
        for orientation in Orient::ALL {
            let orient = vec![orientation, Orient::R0];
            let view = View { s: &s, at: &at, orient: &orient };
            let term = Term::new("symbol_overlaps");
            let old = score_term(&term, &view, None);
            assert_eq!(old, int(0));
            assert_eq!(Scoring::new(view, None).term(&term), old, "bbox={bbox:?}, orientation={orientation:?}");
        }
    }
}

#[test]
fn configured_terms_remain_in_reports_when_their_inputs_are_inactive() {
    let options = SolverOptions::without_time_limit();
    let active = Active::new(&options.objectives, &Inputs::default());
    assert_eq!(active.layers.len(), 13);
    assert!(active.inert.iter().any(|s| s.starts_with("current_direction")));
    assert!(active.is_on(Objective::SymbolOverlaps));
    assert_eq!(Objective::from_name("wire_turns"), Some(Objective::Turns));
    assert_eq!(Objective::from_name("ground_direction"), None);
}

fn ranked(active: &Active, entries: &[(Vec<Q>, Vec<u32>)]) -> Vec<usize> {
    let refs: Vec<(&[Q], &[u32])> = entries.iter().map(|(v, c)| (v.as_slice(), c.as_slice())).collect();
    active.rank(&refs)
}

#[test]
fn layers_rank_lexicographically_within_their_tolerances() {
    let objectives = Objectives {
        layers: vec![
            Layer { items: vec![crate::Term::new("area")], tolerance: frac(1, 2) },
            Layer { items: vec![crate::Term { weight: int(2), ..crate::Term::new("aspect") }], tolerance: int(0) },
        ],
    };
    let active = Active::new(&objectives, &Inputs::default());
    assert_eq!(active.layers[0].tolerance, frac(1, 2));
    let e = |a: Q, b: i64, code: u32| (vec![a, int(b)], vec![code]);
    // 1 and 2 are further apart than the tolerance: the first layer decides
    assert_eq!(ranked(&active, &[e(int(2), 0, 0), e(int(1), 5, 1)]), vec![1, 0]);
    // within the tolerance of the first layer, the second decides
    assert_eq!(ranked(&active, &[e(int(1), 5, 0), e(frac(3, 2), 0, 1)]), vec![1, 0]);
    // equal in every layer: the smaller code, then the earlier entry
    assert_eq!(ranked(&active, &[e(int(1), 5, 3), e(int(1), 5, 2), e(int(1), 5, 2)]), vec![1, 2, 0]);
}

/// Within a tolerance "equal" is not transitive: a pairwise comparison would put A before C,
/// C before B and B before A. The rank is still one order, whatever the order of the entries.
#[test]
fn a_rank_is_well_defined_where_tolerances_are_not_transitive() {
    let objectives = Objectives {
        layers: vec![
            Layer { items: vec![crate::Term::new("area")], tolerance: int(1) },
            Layer { items: vec![crate::Term::new("aspect")], tolerance: int(0) },
        ],
    };
    let active = Active::new(&objectives, &Inputs::default());
    let (a, b, c) =
        ((vec![int(0), int(5)], vec![0]), (vec![frac(3, 5), int(3)], vec![1]), (vec![frac(6, 5), int(1)], vec![2]));
    // the least of the first layer is 0: A and B are within 1 of it, and B is better in the
    // second; then A, the least of what is left; then C
    let entries = [a.clone(), b.clone(), c.clone()];
    let codes = |order: Vec<usize>, entries: &[(Vec<Q>, Vec<u32>)]| -> Vec<u32> {
        order.into_iter().map(|i| entries[i].1[0]).collect()
    };
    assert_eq!(codes(ranked(&active, &entries), &entries), vec![1, 0, 2]);
    for permutation in
        [[c.clone(), b.clone(), a.clone()], [b.clone(), c.clone(), a.clone()], [c.clone(), a.clone(), b.clone()]]
    {
        assert_eq!(codes(ranked(&active, &permutation), &permutation), vec![1, 0, 2]);
    }
    // with no tolerance it is the lexicographic order
    let exact =
        Active::new(&Objectives { layers: vec![Layer::single("area"), Layer::single("aspect")] }, &Inputs::default());
    assert_eq!(codes(ranked(&exact, &entries), &entries), vec![0, 1, 2]);
    // and the layer-by-layer choice agrees with the sort when every tolerance is 0
    let many: Vec<(Vec<Q>, Vec<u32>)> =
        (0..40u32).map(|i| (vec![int(i64::from(i % 5)), int(i64::from((i * 7) % 3))], vec![(i * 13) % 11])).collect();
    let mut by_hand: Vec<usize> = (0..many.len()).collect();
    by_hand.sort_by(|&x, &y| many[x].0.cmp(&many[y].0).then(many[x].1.cmp(&many[y].1)).then(x.cmp(&y)));
    assert_eq!(ranked(&exact, &many), by_hand);
    let tiny = Active::new(
        &Objectives {
            layers: vec![
                Layer { items: vec![crate::Term::new("area")], tolerance: crate::rational::frac(1, 1_000_000_000) },
                Layer::single("aspect"),
            ],
        },
        &Inputs::default(),
    );
    assert_eq!(ranked(&tiny, &many), by_hand);
}

#[test]
fn gap_kinds_count_kinds_and_prefer_one_dominant_spacing() {
    let q = |v: &[i64]| v.iter().map(|x| int(*x)).collect::<Vec<Q>>();
    // one gap kind: 0
    assert_eq!(gap_kinds(q(&[0, 2, 4, 6]).iter()), int(0));
    // repeats count once, fewer than two coordinates cost nothing
    assert_eq!(gap_kinds(q(&[3, 3, 3]).iter()), int(0));
    // two kinds 2, 2, 5: 1 + 1 − (4/9 + 1/9) = 13/9
    assert_eq!(gap_kinds(q(&[0, 2, 4, 9]).iter()), int(1));
    // two kinds 2, 5: 1 + 1 − (1/4 + 1/4) = 3/2, worse than one dominant kind
    assert_eq!(gap_kinds(q(&[0, 2, 7]).iter()), int(1));
}

#[test]
fn spanning_trees_bends_and_crossings() {
    let p = |x: i64, y: i64| [int(x), int(y)];
    let points = [p(0, 0), p(10, 0), p(1, 1), p(10, 5)];
    // (1, 0) is as near to (0, 0) as to (1, 1): it hangs from the earlier
    assert_eq!(spanning_tree(&points), vec![(0, 2), (0, 1), (1, 3)]);
    assert_eq!(bends(&[p(0, 0), p(5, 0), p(5, 5), p(5, 9), p(7, 9)]), 2);
    assert_eq!(bends(&[p(0, 0), p(5, 0)]), 0);
    // a plus crosses, a T and a shared end do not
    assert!(proper_crossing(&p(0, 0), &p(4, 0), &p(2, -2), &p(2, 2)));
    assert!(!proper_crossing(&p(0, 0), &p(4, 0), &p(2, 0), &p(2, 2)));
    assert!(!proper_crossing(&p(0, 0), &p(4, 0), &p(4, 0), &p(4, 2)));
    let segments = vec![(0, p(0, 0), p(4, 0)), (1, p(2, -2), p(2, 2)), (0, p(3, -1), p(3, 1))];
    // the two of net 0 cross each other but count only against net 1
    assert_eq!(crossings(&segments), int(1));
    // oblique segments: against all the others, each pair once
    let segments = vec![(0, p(0, 0), p(4, 4)), (1, p(0, 4), p(4, 0)), (2, p(2, -1), p(2, 5)), (3, p(-1, 2), p(5, 2))];
    // the two diagonals cross each other at (2, 2), where the plus also crosses: every pair
    assert_eq!(crossings(&segments), int(6));
    // touching at an end is no crossing
    assert_eq!(crossings(&[(0, p(0, 0), p(4, 0)), (1, p(4, 0), p(4, 3))]), int(0));
    assert_eq!(aspect(&int(16), &int(9), &frac(16, 9)), int(0));
    assert_eq!(aspect(&int(4), &int(0), &frac(16, 9)), int(1));
}

/// The turns of a connection as the router draws it (quality review SC-1), with T1's pins: the
/// source's pin leaving up, the resistor's leaving left towards it.
#[test]
fn a_connection_counts_the_turns_of_a_jog_an_l_and_a_straight_line() {
    let p = |x: i64, y: i64| [int(x), int(y)];
    let (up, down, left, right) = (Some((0, 1)), Some((0, -1)), Some((-1, 0)), Some((1, 0)));
    // the jog: on one height, the wire goes up, across and down again
    assert_eq!(connection_turns(&p(-7, 6), up, &p(-3, 6), left), 3);
    assert_eq!(connection_turns(&p(-3, 6), left, &p(-7, 6), up), 3);
    // less than a lead above is a jog too, off the grid
    assert_eq!(connection_turns(&p(0, 0), up, &[int(4), frac(1, 2)], left), 3);
    // the L: the resistor's pin above the end of the source's lead
    assert_eq!(connection_turns(&p(-7, 6), up, &p(-3, 7), left), 1);
    assert_eq!(connection_turns(&p(-7, 6), up, &p(-3, 9), left), 1);
    // the straight line: two pins facing each other, or a pin and a point on its way
    assert_eq!(connection_turns(&p(0, 0), right, &p(5, 0), left), 0);
    assert_eq!(connection_turns(&p(0, 0), up, &p(0, 4), down), 0);
    assert_eq!(connection_turns(&p(0, 0), right, &p(5, 0), None), 0);
    // a U: both leave up on one height; an S: one up, one down
    assert_eq!(connection_turns(&p(0, 0), up, &p(5, 0), up), 2);
    assert_eq!(connection_turns(&p(0, 0), up, &p(5, 0), down), 4);
    // a U too: both leave the same way, off an axis, the wire out past the farther and back
    // to it, (0, 0) (0, 2) (4, 2) (4, 1) and (0, 0) (-1, 0) (-1, 3) (5, 3)
    assert_eq!(connection_turns(&p(0, 0), up, &p(4, 1), up), 2);
    assert_eq!(connection_turns(&p(4, 1), up, &p(0, 0), up), 2);
    assert_eq!(connection_turns(&p(0, 0), up, &p(4, 5), up), 2);
    assert_eq!(connection_turns(&p(0, 0), down, &[int(4), frac(1, 2)], down), 2);
    assert_eq!(connection_turns(&p(0, 0), left, &p(5, 3), left), 2);
    // an end that leaves away from the other: two more
    assert_eq!(connection_turns(&p(0, 0), left, &p(5, 0), left), 2);
    assert_eq!(connection_turns(&p(0, 0), up, &p(5, 0), right), 3);
    assert_eq!(connection_turns(&p(0, 0), up, &p(5, 3), right), 3);
    // points that are no pins: an L when not along an axis
    assert_eq!(connection_turns(&p(0, 0), None, &p(5, 3), None), 1);
    assert_eq!(connection_turns(&p(0, 0), None, &p(5, 0), None), 0);
}

#[test]
fn symbol_direction_counts_real_pinless_attachments_and_all_same_named_symbols() {
    use rnl_runtime::source::MemoryProvider;
    const SOURCE: &str = include_str!("../data/pinless-instance-attachment.sp");
    let mut options = SolverOptions::from_toml(include_str!("../data/pinless-symbol-direction.toml")).unwrap();
    options.time_limit = None;
    for (host_orientation, extra_orientation, marker, expected) in [
        ("R0", None, false, 1),
        ("R180", None, false, 1),
        ("R0", Some("R0"), false, 2),
        ("R0", Some("R180"), false, 1),
        ("R0", None, true, 1),
    ] {
        let mut source = SOURCE
            .replace("@PLACE(0,0) => @ORIENT(\"R0\")", &format!("@PLACE(0,0) => @ORIENT(\"{host_orientation}\")"));
        if let Some(orientation) = extra_orientation {
            source = source.replace("*   &n @ROUTE",
                &format!("*   %second_badge = $BADGE @ATTACH #XA => @SET-SYMBOL $BADGE_SYMBOL => @ORIENT(\"{orientation}\");\n*   &n @ROUTE"));
        }
        if marker {
            source =
                source.replace("%badge = $BADGE @ATTACH #XH", "%marker_at = (0,0); %badge = $BADGE @ATTACH %marker_at");
        }
        let files = MemoryProvider::new().with("direction.sp", &source);
        let input = rnl_view::interpret("direction.sp", None, &files, &rnl_runtime::Options::solver()).unwrap();
        assert!(!input.diagnostics().iter().any(|d| d.severity == rnl_runtime::diag::Severity::Error));
        let p = input.problems.iter().find(|p| p.kind == "file").unwrap();
        let badge = p.objects.iter().position(|o| o.label == "%badge").unwrap();
        let host = p.objects.iter().position(|o| o.label == "#XH").unwrap();
        let main = if marker { p.objects.iter().position(|o| o.label == "%marker_at").unwrap() } else { host };
        assert_eq!(
            p.objects[badge].position,
            rnl_runtime::problem::Position::At(rnl_runtime::problem::PointRef::Origin(main))
        );
        assert_eq!(
            p.objects[main].kind,
            if marker { rnl_runtime::problem::ObjectKind::Point } else { rnl_runtime::problem::ObjectKind::Instance }
        );
        let resolved = crate::resolve(p, &options);
        assert_eq!(resolved.kept.len(), p.facts.len());
        let s = crate::structure::analyse(p, &resolved, &options, crate::options::CAPS, &mut crate::Work::default());
        assert!(s.pins(badge).is_empty() && !s.objects[badge].left_free);
        assert!(!s.components().any(|object| object == badge), "search classification is unchanged");
        let g = &resolved.witness.geometry;
        assert_eq!(g.orient[badge], Orient::R0, "an instance-origin attachment has its own fixed orientation");
        assert_eq!(g.orient[host], if host_orientation == "R0" { Orient::R0 } else { Orient::R180 });
        let at = s.origins(p, g);
        let view = View { s: &s, at: &at, orient: &g.orient };
        let term = &options.objectives.layers[0].items[0];
        assert_eq!(symbol_term(term, &view), int(expected));
        assert_eq!(term_status(term, &view), (false, Vec::new()));
        let report = s.active.report(&view, None, &options);
        assert_eq!(report.levels.len(), 1);
        assert_eq!(report.levels[0].terms.len(), 1);
        assert_eq!(report.levels[0].terms[0].raw, expected.to_string());
        assert!(!report.levels[0].terms[0].inactive);
        assert!(report.levels[0].terms[0].diagnostics.is_empty());
    }
}

#[test]
fn symbol_direction_eligibility_and_reference_status_agree_on_boundary_inputs() {
    use rnl_runtime::source::MemoryProvider;
    let source = include_str!("../data/pinless-instance-attachment.sp");
    let files = MemoryProvider::new().with("direction.sp", source);
    let input = rnl_view::interpret("direction.sp", None, &files, &rnl_runtime::Options::solver()).unwrap();
    assert!(!input.diagnostics().iter().any(|d| d.severity == rnl_runtime::diag::Severity::Error));
    let p = input.problems.iter().find(|p| p.kind == "file").unwrap();
    let badge = p.objects.iter().position(|o| o.label == "%badge").unwrap();
    let mut options = SolverOptions::from_toml(include_str!("../data/pinless-symbol-direction.toml")).unwrap();
    options.time_limit = None;
    let resolved = crate::resolve(p, &options);
    let mut s = crate::structure::analyse(p, &resolved, &options, crate::options::CAPS, &mut crate::Work::default());
    let g = &resolved.witness.geometry;
    let at = s.origins(p, g);
    let original = options.objectives.layers[0].items[0].clone();
    let check = |case: &str, s: &Structure, term: &Term, expected: i64, inactive: bool, diagnostics: &[&str]| {
        let view = View { s, at: &at, orient: &g.orient };
        assert_eq!(symbol_term(term, &view), int(expected), "{case}: raw score");
        let status = term_status(term, &view);
        assert_eq!(status.0, inactive, "{case}: inactive");
        let mut actual_diagnostics: Vec<&str> = status.1.iter().map(String::as_str).collect();
        actual_diagnostics.sort_unstable();
        let mut expected_diagnostics = diagnostics.to_vec();
        expected_diagnostics.sort_unstable();
        assert_eq!(actual_diagnostics, expected_diagnostics, "{case}: diagnostic identities and reasons");
    };
    check("common reference", &s, &original, 1, false, &[]);
    let mut per_name = original.clone();
    per_name.references.insert("$BADGE_SYMBOL".into(), Direction::Down);
    check("per-symbol overrides common reference", &s, &per_name, 0, false, &[]);
    per_name.reference = None;
    per_name.references.insert("$BADGE_SYMBOL".into(), Direction::Up);
    check("per-symbol reference without common reference", &s, &per_name, 1, false, &[]);
    let mut missing_reference = original.clone();
    missing_reference.reference = None;
    check(
        "pinless missing reference",
        &s,
        &missing_reference,
        0,
        true,
        &["symbol %badge requires a cardinal reference direction", "this term has no eligible objects in this level"],
    );
    let mut absent = original.clone();
    absent.symbols = vec!["$MISSING_SYMBOL".into()];
    check("configured symbol absent", &s, &absent, 0, true, &["this term has no eligible objects in this level"]);
    let mut natural_zero = missing_reference;
    natural_zero.symbols = vec!["$PIN_SYMBOL".into()];
    check(
        "three centre-pin symbols without reference",
        &s,
        &natural_zero,
        0,
        true,
        &[
            "symbol #XH requires a cardinal reference direction",
            "symbol #XA requires a cardinal reference direction",
            "symbol #XB requires a cardinal reference direction",
            "this term has no eligible objects in this level",
        ],
    );
    s.objects[badge].left_free = true;
    check(
        "left-free symbol excluded from score and status",
        &s,
        &original,
        0,
        true,
        &["this term has no eligible objects in this level"],
    );
}
