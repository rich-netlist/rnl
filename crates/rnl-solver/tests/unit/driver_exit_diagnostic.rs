/// Collect every historical fixed-frame exit violation without a first-panic blind spot.
/// This ignored acceptance diagnostic does not relax or replace the strict regression above.
#[test]
#[ignore = "acceptance evidence only; a completed diagnostic is not a regression pass"]
fn fixed_case_exit_diagnostic_collects_all_layers() {
    let output = std::env::var("RNL_EXIT_DIAGNOSTIC").expect("absolute RNL_EXIT_DIAGNOSTIC output path");
    let mut levels = Vec::new();
    let mut count = 0;
    for file in netlists("tests/integration/solver", "T") {
        for original in problems(&file) {
            let (p, solved, e) = fixed_legacy_case_level(file.file_name().unwrap().to_str().unwrap(), &original.title);
            let mut violations = Vec::new();
            for v in violated_exits(&p, &solved, &e) {
                let b = e.structure.box_at(&e.placed.origins, &e.placed.geometry.orient, v.pin.0);
                let a = &v.start;
                let z = &v.next;
                let geometry = b.as_ref().map(|b| {
                    let inside = |q: &[Q; 2]| b[0] < q[0] && q[0] < b[2] && b[1] < q[1] && q[1] < b[3];
                    let corner = (a[0] == b[0] || a[0] == b[2]) && (a[1] == b[1] || a[1] == b[3]);
                    let horizontal_outward = a[1] == z[1]
                        && ((a[0] == b[0] && z[0] < a[0]) || (a[0] == b[2] && z[0] > a[0]));
                    let vertical_outward = a[0] == z[0]
                        && ((a[1] == b[1] && z[1] < a[1]) || (a[1] == b[3] && z[1] > a[1]));
                    // Positive-length intersection with the actual bounded edge, not its infinite line.
                    let overlap = |axis: usize| {
                        a[axis].clone().min(z[axis].clone()).max(b[axis].clone())
                            < a[axis].clone().max(z[axis].clone()).min(b[axis + 2].clone())
                    };
                    let along = (a[0] == z[0] && (a[0] == b[0] || a[0] == b[2]) && overlap(1))
                        || (a[1] == z[1] && (a[1] == b[1] || a[1] == b[3]) && overlap(0));
                    serde_json::json!({
                        "start_inside": inside(a), "next_inside": inside(z),
                        "through": crate::route::meets_inside(b, a, z), "along": along,
                        "corner": corner, "horizontal_outward": horizontal_outward,
                        "vertical_outward": vertical_outward, "corner_outward": corner && (horizontal_outward || vertical_outward),
                    })
                });
                violations.push(serde_json::json!({
                    "object": v.pin.0, "pin": v.pin.1, "label": p.objects[v.pin.0].label,
                    "net": v.net, "terminal": v.terminal,
                    "exit": e.structure.exit(v.pin.0, v.pin.1, e.placed.geometry.orient[v.pin.0]),
                    "start": a.iter().map(ToString::to_string).collect::<Vec<_>>(),
                    "next": z.iter().map(ToString::to_string).collect::<Vec<_>>(),
                    "bbox": b.map(|b| b.iter().map(ToString::to_string).collect::<Vec<_>>()),
                    "geometry": geometry, "description": v.description,
                }));
            }
            count += violations.len();
            levels.push(serde_json::json!({
                "file": file.file_name().unwrap().to_string_lossy(), "level": p.title, "kind": p.kind,
                "originally_excluded_from_strict_exit_regression": file.ends_with("T4-miller-conflicts.sp"),
                "kept_facts": solved.report.kept.len(), "total_facts": p.facts.len(),
                "routing_fallbacks": format!("{:?}", solved.report.routing_fallbacks), "violations": violations,
            }));
        }
    }
    let report = serde_json::json!({
        "diagnostic_only": true, "strict_regression_unchanged": true,
        "level_count": levels.len(), "violation_count": count, "levels": levels,
    });
    std::fs::write(&output, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    println!("fixed exit diagnostic: {} levels, {count} violations; output={output}", report["level_count"]);
}

/// One solve of the failing tiny-budget T4 layer, with no deterministic second solve or checker.
#[test]
#[ignore = "test-only phase/owner evidence, not a weakened tiny-budget regression"]
fn tiny_t4_route_phase_and_owner_exemption() {
    use rnl_runtime::solution::WirePoint;
    let output = std::env::var("RNL_TINY_DIAGNOSTIC").expect("absolute RNL_TINY_DIAGNOSTIC output path");
    let p = problems(&repo().join("tests/integration/solver/T4-miller-conflicts.sp"))
        .into_iter()
        .find(|p| p.title == "T4-miller-conflicts.sp")
        .unwrap();
    let options =
        SolverOptions::without_time_limit().with_test_terms("budget=candidates:0,astar_expansions:12").unwrap();
    let caps = Caps { nodes: 2, pivots: 30 };
    let (solved, e) = solve_exact_with(&p, &options, &StoppedClock, caps);
    let vb = object(&p, "#VB");
    let boxes: Vec<_> = e
        .structure
        .components()
        .filter_map(|k| e.structure.box_at(&e.placed.origins, &e.placed.geometry.orient, k).map(|b| (k, b)))
        .collect();
    let vb_box = e.structure.box_at(&e.placed.origins, &e.placed.geometry.orient, vb).unwrap();
    let inside = |b: &[Q; 4], q: &[Q; 2]| b[0] < q[0] && q[0] < b[2] && b[1] < q[1] && q[1] < b[3];
    let at = |r: &PointRef| -> Option<[Q; 2]> {
        Some([point_value(&p, &e.placed.geometry, r, 0, 0)?, point_value(&p, &e.placed.geometry, r, 1, 0)?])
    };
    let net = &p.routes[4].net;
    let mut refs = Vec::new();
    for route in p.routes.iter().filter(|r| r.net == *net) {
        refs.extend(route.pins.iter().cloned());
        for &wire in &route.wires {
            refs.extend(p.wires[wire].anchors.iter().cloned());
        }
    }
    let mut keys = std::collections::BTreeSet::new();
    let mut exempt = std::collections::BTreeSet::new();
    let mut anchors = Vec::new();
    for reference in refs {
        let key = match &reference {
            PointRef::Pin { object, pin, .. } => (*object, Some(*pin)),
            PointRef::Origin(k) if p.objects[*k].kind == ObjectKind::Point => (*k, None),
            _ => continue,
        };
        if !keys.insert(key) {
            continue;
        }
        let owner = key.1.map(|_| key.0);
        let xy = at(&reference);
        let owner_box = owner.and_then(|owner| boxes.iter().find(|(k, _)| *k == owner).map(|(_, b)| b));
        let strict_inside_owner = owner_box.zip(xy.as_ref()).is_some_and(|(b, q)| inside(b, q));
        if strict_inside_owner {
            exempt.insert(owner.unwrap());
        }
        anchors.push(serde_json::json!({
            "reference": format!("{reference:?}"), "object": key.0, "pin": key.1,
            "owner": owner, "owner_label": owner.map(|k| &p.objects[k].label),
            "at": xy.map(|q| q.iter().map(ToString::to_string).collect::<Vec<_>>()),
            "owner_box": owner_box.map(|b| b.iter().map(ToString::to_string).collect::<Vec<_>>()),
            "strict_inside_owner": strict_inside_owner,
        }));
    }
    let mut wires = Vec::new();
    for (wire, path) in solved.solution.wires.iter().zip(&e.routing.paths).filter(|(w, _)| w.route == 4) {
        let endpoint_own_inside_vb = [wire.points.first(), wire.points.last()].into_iter().flatten().any(|end| {
            matches!(end, WirePoint::Anchor(r @ PointRef::Pin { object, .. }) if *object == vb && at(r).is_some_and(|q| inside(&vb_box, &q)))
        });
        wires.push(serde_json::json!({
            "route": wire.route, "points": format!("{:?}", wire.points),
            "path": path.iter().map(|q| q.iter().map(ToString::to_string).collect::<Vec<_>>()).collect::<Vec<_>>(),
            "endpoint_own_inside_vb": endpoint_own_inside_vb,
            "segments_through_vb": path.windows(2).map(|p| crate::route::meets_inside(&vb_box, &p[0], &p[1])).collect::<Vec<_>>(),
        }));
    }
    let report = serde_json::json!({
        "diagnostic_only": true, "solve_count": 1, "file": "T4-miller-conflicts.sp", "level": p.title,
        "caps_nodes": 2, "caps_pivots": 30, "budget": format!("{:?}", options.budget),
        "work": {"resolve_nodes": solved.report.work.resolve_nodes, "pivots": solved.report.work.pivots, "candidates": solved.report.work.candidates, "astar_expansions": solved.report.work.astar_expansions},
        "routing_expansions": e.routing.expansions, "candidate": format!("{:?}", e.candidate),
        "report": format!("{:?}", solved.report), "fallbacks": format!("{:?}", solved.report.routing_fallbacks),
        "route": 4, "net": net, "anchors": anchors, "route4_wires": wires,
        "vb": {"object": vb, "structure_object": format!("{:?}", e.structure.objects[vb]), "is_component": boxes.iter().any(|(k, _)| *k == vb), "bbox": vb_box.iter().map(ToString::to_string).collect::<Vec<_>>()},
        "grid_rule_exempt_owners": exempt, "grid_rule_exempt_labels": exempt.iter().map(|&k| p.objects[k].label.clone()).collect::<Vec<_>>(),
        "summary": summary(&p, &solved, &e),
    });
    std::fs::write(&output, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    println!("tiny T4 diagnostic written to {output}; solve_count=1");
}
