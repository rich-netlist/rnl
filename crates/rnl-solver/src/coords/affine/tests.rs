use super::*;
use crate::rational::{frac, int};

fn eq(e: LinExpr, rhs: Q, tag: usize) -> SourceBound<usize> {
    SourceBound { expr: e, lower: Some((rhs.clone(), tag)), upper: Some((rhs, tag)) }
}

fn holds(b: &SourceBound<usize>, x: &[Q]) -> bool {
    let value = b.expr.eval(|v| x[v].clone());
    b.lower.as_ref().is_none_or(|(lo, _)| value >= *lo) && b.upper.as_ref().is_none_or(|(hi, _)| value <= *hi)
}

/// This checker uses dense multiplication in the original coordinates, not sparse substitution
/// or the compiler's elimination trace. It checks E_elim*T=0, E_elim*d=f and P*T=I,P*d=0.
fn check_map(c: &Compiled<usize>) {
    let n = c.map.images.len();
    let k = c.map.retained.len();
    let mut t = vec![vec![Q::zero(); k]; n];
    let mut d = vec![Q::zero(); n];
    for i in 0..n {
        d[i] = c.map.images[i].constant_part().clone();
        for (v, a) in c.map.images[i].terms() { t[i][*v] = a.clone(); }
    }
    for (z, v) in c.map.retained.iter().enumerate() {
        assert!(d[*v].is_zero());
        for j in 0..k { assert_eq!(t[*v][j], if j == z { Q::one() } else { Q::zero() }); }
    }
    for e in &c.eliminated {
        let mut a = vec![Q::zero(); n];
        for (v, q) in e.expr.terms() { a[*v] = q.clone(); }
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
    let mut bounds = vec![eq(LinExpr::from_terms([(0, int(1)), (1, int(1)), (2, int(1))], int(0)), int(0), 0)];
    for v in 0..3 {
        for w in 3..5 {
            for sign in [-1, 1] {
                bounds.push(SourceBound {
                    expr: LinExpr::from_terms([(v, int(1)), (w, int(sign))], int(0)),
                    lower: Some((int(-10), 1)), upper: None,
                });
            }
        }
    }
    bounds.push(SourceBound { expr: LinExpr::var(4), lower: Some((frac(-1, 10), 2)), upper: Some((frac(1, 10), 2)) });
    let c = compile(5, bounds.clone(), &[]).unwrap();
    assert!(c.eliminated.is_empty(), "this equation must be kept because every substitution adds fill");
    assert_eq!(c.bounds.len(), bounds.len());
    check_map(&c);
    for x in [vec![int(1), int(-1), int(0), int(0), frac(1, 20)], vec![int(1); 5]] {
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
