use super::*;
use crate::rational::{frac, int};
use num_traits::One;

fn eq(e: LinExpr, rhs: Q, tag: usize) -> SourceBound<usize> {
    SourceBound { expr: e, lower: Some((rhs.clone(), tag)), upper: Some((rhs, tag)) }
}

fn holds(b: &SourceBound<usize>, x: &[Q]) -> bool {
    let value = b.expr.eval(|v| x[v].clone());
    b.lower.as_ref().is_none_or(|(lo, _)| value >= *lo) && b.upper.as_ref().is_none_or(|(hi, _)| value <= *hi)
}

/// This checker uses dense multiplication in the original coordinates, not sparse substitution
/// or the compiler's elimination trace. It checks E_elim*T=0, E_elim*d=f and P*T=I,P*d=0.
#[allow(clippy::needless_range_loop)] // conventional dense matrix oracle, independent of sparse code
fn check_map(c: &Compiled<usize>) {
    let n = c.map.images.len();
    let k = c.map.retained.len();
    let mut t = vec![vec![Q::zero(); k]; n];
    let mut d = vec![Q::zero(); n];
    for i in 0..n {
        d[i] = c.map.images[i].constant_part().clone();
        for (v, a) in c.map.images[i].terms() {
            t[i][*v] = a.clone();
        }
    }
    for (z, v) in c.map.retained.iter().enumerate() {
        assert!(d[*v].is_zero());
        for j in 0..k {
            assert_eq!(t[*v][j], if j == z { Q::one() } else { Q::zero() });
        }
    }
    for e in &c.eliminated {
        let mut a = vec![Q::zero(); n];
        for (v, q) in e.expr.terms() {
            a[*v] = q.clone();
        }
        let constant = (0..n).fold(e.expr.constant_part().clone(), |s, i| s + &a[i] * &d[i]);
        assert_eq!(constant, e.lower.as_ref().unwrap().0);
        for j in 0..k {
            assert!((0..n).fold(Q::zero(), |s, i| s + &a[i] * &t[i][j]).is_zero());
        }
    }
}

#[test]
fn fixed_difference_average_and_redundant_equations_have_a_bidirectional_lift() {
    let bounds = vec![
        eq(LinExpr::var(0), frac(1, 3), 0),
        eq(LinExpr::diff(1, 0), frac(2, 5), 1),
        eq(LinExpr::from_terms([(2, int(2)), (1, int(-1)), (3, int(-1))], int(0)), frac(3, 7), 2),
        eq(LinExpr::diff(1, 0), frac(2, 5), 3),
        SourceBound { expr: LinExpr::var(3), lower: Some((int(-2), 4)), upper: Some((int(4), 4)) },
    ];
    let c = compile(4, bounds.clone(), &[]).unwrap();
    check_map(&c);
    // Independently chosen original feasible points, including a nonintegral one. Projection
    // followed by lift must reproduce EACH point, rather than just one successful witness.
    for x3 in [int(-2), frac(4, 9), int(4)] {
        let x = vec![frac(1, 3), frac(11, 15), (frac(11, 15) + &x3 + frac(3, 7)) / int(2), x3];
        assert!(bounds.iter().all(|b| holds(b, &x)));
        let z: Vec<Q> = c.map.retained.iter().map(|v| x[*v].clone()).collect();
        assert!(c.bounds.iter().all(|b| holds(b, &z)));
        let lifted: Vec<Q> = c.map.images.iter().map(|e| e.eval(|v| z[v].clone())).collect();
        assert_eq!(x, lifted);
    }
}

#[test]
fn dense_equation_is_retained_and_tolerance_is_not_an_equation() {
    let mut bounds = vec![
        eq(LinExpr::from_terms([(0, int(1)), (1, int(1)), (2, int(1)), (4, int(1))], int(0)), int(0), 0),
        eq(LinExpr::var(4), int(1), 3),
    ];
    for v in 0..3 {
        for w in 3..5 {
            for sign in [-1, 1] {
                bounds.push(SourceBound {
                    expr: LinExpr::from_terms([(v, int(1)), (w, int(sign))], int(0)),
                    lower: Some((int(-10), 1)),
                    upper: None,
                });
            }
        }
    }
    bounds.push(SourceBound { expr: LinExpr::var(3), lower: Some((frac(-1, 10), 2)), upper: Some((frac(1, 10), 2)) });
    let c = compile(5, bounds.clone(), &[]).unwrap();
    assert_eq!(c.eliminated.len(), 1, "only the fixed coordinate can be removed without fill");
    assert_eq!(c.bounds.len(), bounds.len() - 1);
    assert!(
        c.bounds.iter().any(|b| equation(b).is_some() && b.expr.constant_part() == &int(1)),
        "E_rem must retain its transformed nonzero offset"
    );
    check_map(&c);
    for x in [vec![int(1), int(-1), int(-1), frac(1, 20), int(1)], vec![int(1); 5]] {
        let z: Vec<Q> = c.map.retained.iter().map(|v| x[*v].clone()).collect();
        assert_eq!(bounds.iter().all(|b| holds(b, &x)), c.bounds.iter().all(|b| holds(b, &z)));
    }
}

#[test]
fn contradictory_equations_keep_the_original_tags() {
    let err = compile(2, vec![eq(LinExpr::diff(0, 1), int(0), 4), eq(LinExpr::diff(0, 1), int(1), 9)], &[]);
    assert!(matches!(err, Err(Failure::Conflict(tags)) if tags == vec![4, 9]));
}

#[test]
fn source_form_dag_keeps_an_unbounded_dependency_and_discards_inactive_bounds() {
    let mut sx = Simplex::<usize>::new();
    let x = sx.new_var();
    let y = sx.new_var();
    let old = sx.checkpoint();
    let s = sx.add_row(&LinExpr::from_terms([(x, int(2)), (y, int(1))], int(0)));
    sx.assert(&LinExpr::var(s), Rel::Ge, &int(-20), 1).unwrap();
    sx.backtrack(old);
    sx.assert(&LinExpr::from_terms([(s, int(1)), (x, int(1))], int(0)), Rel::Eq, &int(9), 2).unwrap();
    let active = sx.active_bounds_in_source_variables();
    assert_eq!(active.variables, vec![x, y]);
    assert_eq!(active.bounds.len(), 1);
    assert_eq!(active.bounds[0].expr, LinExpr::from_terms([(0, int(3)), (1, int(1))], int(0)));
    assert_eq!(active.bounds[0].lower, Some((int(9), 2)));
}

/// Exact dense Gaussian elimination for the oracle's candidate vertices. It neither uses the
/// sparse compiler nor the production simplex, and rejects rank-deficient candidate systems.
#[allow(clippy::needless_range_loop)]
fn dense_solve(mut a: Vec<Vec<Q>>, mut b: Vec<Q>, n: usize) -> Option<Vec<Q>> {
    for col in 0..n {
        let row = (col..a.len()).find(|r| !a[*r][col].is_zero())?;
        a.swap(col, row);
        b.swap(col, row);
        let pivot = a[col][col].clone();
        for v in &mut a[col] {
            *v /= &pivot;
        }
        b[col] /= pivot;
        let basis = a[col].clone();
        let rhs = b[col].clone();
        for row in 0..a.len() {
            if row == col {
                continue;
            }
            let factor = a[row][col].clone();
            for j in 0..n {
                a[row][j] -= &factor * &basis[j];
            }
            b[row] -= &factor * &rhs;
        }
    }
    if (n..a.len()).any(|r| !b[r].is_zero()) {
        return None;
    }
    Some(b[..n].to_vec())
}

fn vertices(n: usize, bounds: &[SourceBound<usize>]) -> Vec<Vec<Q>> {
    let mut planes = Vec::new();
    for b in bounds {
        let mut row = vec![Q::zero(); n];
        for (v, a) in b.expr.terms() {
            row[*v] = a.clone();
        }
        if let Some((v, _)) = &b.lower {
            planes.push((row.clone(), v - b.expr.constant_part()));
        }
        if b.upper.as_ref().map(|(v, _)| v) != b.lower.as_ref().map(|(v, _)| v) {
            if let Some((v, _)) = &b.upper {
                planes.push((row, v - b.expr.constant_part()));
            }
        }
    }
    fn choose(
        n: usize,
        start: usize,
        planes: &[(Vec<Q>, Q)],
        selected: &mut Vec<usize>,
        bounds: &[SourceBound<usize>],
        out: &mut Vec<Vec<Q>>,
    ) {
        if selected.len() == n {
            let a = selected.iter().map(|i| planes[*i].0.clone()).collect();
            let b = selected.iter().map(|i| planes[*i].1.clone()).collect();
            if let Some(x) = dense_solve(a, b, n) {
                if bounds.iter().all(|b| holds(b, &x)) && !out.contains(&x) {
                    out.push(x);
                }
            }
            return;
        }
        for i in start..planes.len() {
            selected.push(i);
            choose(n, i + 1, planes, selected, bounds, out);
            selected.pop();
        }
    }
    let mut out = Vec::new();
    choose(n, 0, &planes, &mut Vec::new(), bounds, &mut out);
    out
}

fn lex_vertices(mut points: Vec<Vec<Q>>, objectives: &[LinExpr]) -> Vec<Q> {
    assert!(!points.is_empty(), "only bounded pointed fixtures use vertex enumeration");
    let mut scores = Vec::new();
    for e in objectives {
        let best = points.iter().map(|x| e.eval(|v| x[v].clone())).min().unwrap();
        points.retain(|x| e.eval(|v| x[v].clone()) == best);
        scores.push(best);
    }
    scores
}

#[test]
fn original_and_reduced_four_layer_optima_match_an_independent_dense_vertex_oracle() {
    let mut bounds = vec![
        eq(LinExpr::from_terms([(0, int(1)), (1, int(1))], int(0)), int(4), 0),
        eq(LinExpr::from_terms([(2, int(1)), (0, frac(-1, 2))], int(0)), frac(1, 3), 1),
    ];
    for (v, lo, hi) in [(0, int(2), int(4)), (1, int(0), int(4)), (3, int(0), int(10)), (4, int(0), int(10))] {
        bounds.push(SourceBound { expr: LinExpr::var(v), lower: Some((lo, 2)), upper: Some((hi, 2)) });
    }
    // Two distinct origin-distance items have exactly the same expression. Their multiplicity
    // is visible at the nonzero optimum: 2*(4/3-1)=2/3, not 1/3.
    for d in [3, 4] {
        bounds.push(SourceBound { expr: LinExpr::diff(d, 2), lower: Some((int(-1), 3)), upper: None });
        bounds.push(SourceBound {
            expr: LinExpr::from_terms([(d, int(1)), (2, int(1))], int(0)),
            lower: Some((int(1), 3)),
            upper: None,
        });
    }
    let objectives = vec![
        LinExpr::from_terms([(0, int(1)), (1, int(1))], int(0)),
        LinExpr::from_terms([(3, int(1)), (4, int(1))], int(0)),
        LinExpr::var(0),
        LinExpr::var(1),
    ];
    let c = compile(5, bounds.clone(), &[]).unwrap();
    check_map(&c);
    let original_vertices = vertices(5, &bounds);
    let reduced_vertices = vertices(c.map.retained.len(), &c.bounds);
    for z in &reduced_vertices {
        let x: Vec<Q> = c.map.images.iter().map(|e| e.eval(|v| z[v].clone())).collect();
        assert!(bounds.iter().all(|b| holds(b, &x)));
    }
    for x in &original_vertices {
        let z: Vec<Q> = c.map.retained.iter().map(|v| x[*v].clone()).collect();
        assert!(c.bounds.iter().all(|b| holds(b, &z)));
        assert_eq!(*x, c.map.images.iter().map(|e| e.eval(|v| z[v].clone())).collect::<Vec<_>>());
    }
    let expected = vec![int(4), frac(2, 3), int(2), int(2)];
    assert_eq!(lex_vertices(original_vertices.clone(), &objectives), expected);
    let mapped: Vec<LinExpr> = objectives.iter().map(|e| c.map.map_expr(e)).collect();
    assert_eq!(lex_vertices(reduced_vertices, &mapped), expected);
    let seed = &original_vertices[0];
    let mut axis = Axis::from_compiled(c, seed, 0, None, []).unwrap();
    axis.check().unwrap();
    let mut actual = Vec::new();
    for e in &objectives {
        actual.push(axis.minimize(e).unwrap());
        axis.keep_optimum(e, 10).unwrap();
    }
    assert_eq!(actual, expected);
}

#[test]
fn affine_subspace_without_vertices_is_feasible_and_its_objective_is_unbounded() {
    let c = compile(2, vec![eq(LinExpr::diff(0, 1), frac(1, 3), 0)], &[]).unwrap();
    let mut axis = Axis::from_compiled(c, &[frac(1, 3), int(0)], 0, None, []).unwrap();
    axis.check().unwrap();
    assert_eq!(axis.value(0) - axis.value(1), frac(1, 3));
    assert_eq!(axis.minimize(&LinExpr::var(0)), Err(Failure::Unbounded));
}

#[test]
fn compact_axis_carries_used_cap_and_does_not_read_a_clock() {
    let mut original = Axis::<usize>::new();
    let x = original.new_var_at(frac(1, 2));
    original.assert(&LinExpr::var(x), Rel::Ge, &int(0), 0).unwrap();
    original.assert(&LinExpr::var(x), Rel::Le, &int(1), 0).unwrap();
    original.carry_pivots(7);
    original.set_pivot_limit(Some(8));
    let mut reduced = original.compact().unwrap();
    assert_eq!(reduced.pivots(), 7);
    reduced.minimize(&LinExpr::var(x)).unwrap();
    assert_eq!(reduced.pivots(), 8, "the nonbasic bound flip spends the existing axis cap");
    assert_eq!(reduced.minimize(&LinExpr::term(x, int(-1))), Err(Failure::Budget));
    original.carry_pivots(reduced.pivots());
    let mut next_pass = original.compact().unwrap();
    assert_eq!(next_pass.pivots(), 8);
    assert_eq!(next_pass.minimize(&LinExpr::var(x)), Err(Failure::Budget));
}

#[test]
fn consumer_composes_with_an_existing_take_projection_and_keeps_original_ids() {
    let core = compile(3, vec![eq(LinExpr::diff(1, 0), int(2), 0)], &[]).unwrap();
    let mut take = Axis::from_compiled(core, &[int(1), int(3), frac(11, 6)], 0, None, []).unwrap();
    take.assert(&LinExpr::var(0), Rel::Ge, &int(0), 1).unwrap();
    take.assert(&LinExpr::var(0), Rel::Le, &int(3), 1).unwrap();
    take.assert(&LinExpr::from_terms([(2, int(1)), (1, frac(-1, 2))], int(0)), Rel::Eq, &frac(1, 3), 2).unwrap();
    take.check().unwrap();
    let mut consumer = take.compact().unwrap();
    consumer.minimize(&LinExpr::var(2)).unwrap();
    assert_eq!(consumer.value(0), int(0));
    assert_eq!(consumer.value(1), int(2));
    assert_eq!(consumer.value(2), frac(4, 3));
    let helper = consumer.new_var_at(frac(4, 3));
    assert_eq!(helper, 3, "helper virtual IDs must follow the original Model variable domain");
    consumer.assert(&LinExpr::diff(helper, 2), Rel::Ge, &int(0), 3).unwrap();
    consumer.check().unwrap();
    assert_eq!(consumer.value(helper), frac(4, 3));
}

#[test]
fn all_fixed_zero_dimensional_projection_and_canonical_insertion_are_deterministic() {
    let mut bounds = vec![eq(LinExpr::var(0), frac(2, 3), 0), eq(LinExpr::diff(1, 0), frac(1, 5), 1)];
    let a = compile(2, bounds.clone(), &[]).unwrap();
    bounds.reverse();
    let b = compile(2, bounds, &[]).unwrap();
    assert!(a.map.retained.is_empty() && a.bounds.is_empty());
    assert_eq!(a.map.images, b.map.images);
    assert_eq!(a.support, b.support);
    check_map(&a);
    let mut axis = Axis::from_compiled(a, &[frac(2, 3), frac(13, 15)], 9, Some(9), []).unwrap();
    assert_eq!(axis.minimize(&LinExpr::diff(1, 0)), Ok(frac(1, 5)));
    assert_eq!(axis.pivots(), 9, "a constant optimum needs no fresh allowance");
}
