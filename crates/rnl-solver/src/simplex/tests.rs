//! Tests of the simplex: small cases written by hand, then systems made by a fixed pseudo-random
//! sequence and compared with complete deciders: Bellman–Ford for difference constraints,
//! Gaussian elimination for equalities, Fourier–Motzkin elimination for small systems of any
//! linear constraints and for the optimum of a linear objective.

use num_traits::{Signed, Zero};

use super::*;
use crate::rational::{frac, int};

/// The reference never enters canonical_mul, and the scope restores a nested/panicking caller.
fn with_reference_axpy<R>(f: impl FnOnce() -> R) -> R {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            REFERENCE_AXPY_MUL.with(|reference| reference.set(self.0));
        }
    }
    let old = REFERENCE_AXPY_MUL.with(|reference| reference.replace(true));
    let _restore = Restore(old);
    f()
}

fn same_canonical_parts(actual: &Q, expected: &Q) {
    assert_eq!(actual.numer(), expected.numer());
    assert_eq!(actual.denom(), expected.denom());
    assert!(actual.denom().is_positive());
    assert!(actual.numer().gcd(actual.denom()).is_one());
    if actual.is_zero() {
        assert!(actual.denom().is_one());
    }
}

fn same_canonical_simplex(a: &Simplex<usize>, b: &Simplex<usize>) {
    same_indexed_and_linear(a, b);
    let numbers = |s: &Simplex<usize>| {
        let mut out = Vec::new();
        for v in &s.vars {
            out.push(v.value.clone());
            out.extend(v.lower.iter().chain(&v.upper).map(|bound| bound.value.clone()));
        }
        out.extend(s.rows.iter().flat_map(|row| row.terms.iter().map(|(_, q)| q.clone())));
        out.extend(s.forms.keys().flat_map(|terms| terms.iter().map(|(_, q)| q.clone())));
        out.extend(s.trail.iter().flat_map(|undo| undo.old.iter().map(|bound| bound.value.clone())));
        for pivot in s.trace.iter().flatten() {
            out.extend(pivot.values.iter().cloned());
            out.extend(pivot.rows.iter().flat_map(|row| row.terms.iter().map(|(_, q)| q.clone())));
        }
        out
    };
    let (a, b) = (numbers(a), numbers(b));
    assert_eq!(a.len(), b.len());
    for (a, b) in a.iter().zip(&b) {
        same_canonical_parts(a, b);
    }
}

#[test]
fn canonical_mul_matches_original_ratio_components_for_signs_and_large_cross_factors() {
    use num_bigint::BigInt;
    let p = (BigInt::one() << 521usize) - BigInt::one();
    let q = BigInt::one() << 256usize;
    let mut cases = vec![
        int(0),
        int(1),
        int(-1),
        int(7),
        int(-11),
        frac(2, 3),
        frac(-7, 11),
        Q::new(p.clone(), q.clone()),
        Q::new(q.clone(), p.clone()),
        Q::new(-p.clone(), q.clone()),
        Q::new(q.clone(), -p.clone()),
        Q::new(BigInt::zero(), -p.clone()),
        Q::new(&p * &p, &q * &q),
    ];
    let mut rng = Rng(0x54b9_e235_78c1_06ad);
    for _ in 0..24 {
        let numerator = (BigInt::from(rng.int(-23, 23)) << rng.below(300) as usize) + BigInt::from(rng.int(-3, 3));
        let denominator = (BigInt::from(rng.int(1, 23)) << rng.below(300) as usize) + BigInt::one();
        cases.push(Q::new(numerator, denominator));
    }
    for a in &cases {
        for b in &cases {
            // This remains the original Ratio implementation, not the new helper or dispatcher.
            same_canonical_parts(&canonical_mul(a, b), &(a * b));
        }
    }
    // Chaining results tests the canonical closure as well as isolated inputs.
    let (mut actual, mut reference) = (frac(-5, 7), frac(-5, 7));
    for b in cases.iter().filter(|q| !q.is_zero()).cycle().take(48) {
        actual = canonical_mul(&actual, b);
        reference = &reference * b;
        same_canonical_parts(&actual, &reference);
    }
}

#[test]
fn canonical_axpy_matches_map_oracle_cancellation_order_and_linexpr_reference() {
    let acc = vec![(1, frac(-6, 35)), (4, frac(-15, 77)), (7, frac(9, 13))];
    let x = vec![(0, int(-1)), (1, frac(2, 5)), (3, frac(-2, 3)), (4, frac(5, 11)), (8, int(1))];
    for a in [int(0), int(1), int(-1), frac(3, 7), frac(-7, 3)] {
        for x in [x.as_slice(), &[]] {
            // Independent map oracle: only the original Ratio product/addition is used.
            let mut expected: BTreeMap<_, _> = acc.iter().cloned().collect();
            let mut expected_changes = Vec::new();
            if !a.is_zero() {
                for (v, d) in x {
                    let product = &a * d;
                    if let Some(c) = expected.get(v) {
                        let sum = c + product;
                        if sum.is_zero() {
                            expected.remove(v);
                            expected_changes.push((*v, false));
                        } else {
                            expected.insert(*v, sum);
                        }
                    } else {
                        expected.insert(*v, product);
                        expected_changes.push((*v, true));
                    }
                }
            }
            let mut actual = acc.clone();
            let mut changes = Vec::new();
            axpy_changed(&mut actual, x, &a, None, |v, add| changes.push((v, add)));
            assert_eq!(changes, expected_changes);
            assert_eq!(actual.len(), expected.len());
            for ((v, value), (want_v, want)) in actual.iter().zip(&expected) {
                assert_eq!(v, want_v);
                same_canonical_parts(value, want);
            }
            let build = || {
                let mut e = LinExpr::from_terms(acc.iter().cloned(), frac(1, 5));
                e.add_scaled(&LinExpr::from_terms(x.iter().cloned(), frac(-2, 7)), &a);
                e
            };
            let (actual, reference) = (build(), with_reference_axpy(build));
            assert_eq!(actual.terms.len(), reference.terms.len());
            for ((v, value), (want_v, want)) in actual.terms.iter().zip(&reference.terms) {
                assert_eq!(v, want_v);
                same_canonical_parts(value, want);
            }
            same_canonical_parts(&actual.constant, &reference.constant);
        }
    }
}

#[test]
fn canonical_mul_solver_trajectories_match_original_ratio_arithmetic() {
    let mut rng = Rng(0x53a7_e02b_98d4_10fc);
    for _ in 0..16 {
        let (mut a, vars) = simplex(3);
        a.trace = Some(Vec::new());
        let mut b = a.clone();
        for &v in &vars {
            assert_eq!(
                a.assert_bounds(v, Some(int(-8)), Some(int(8)), v),
                with_reference_axpy(|| b.assert_bounds(v, Some(int(-8)), Some(int(8)), v))
            );
        }
        for tag in 3..11 {
            let terms: Vec<_> = vars.iter().map(|&v| (v, frac(rng.int(-3, 3), rng.int(1, 5)))).collect();
            let e = LinExpr::from_terms(terms.iter().cloned(), frac(2, 7));
            let original = with_reference_axpy(|| LinExpr::from_terms(terms, frac(2, 7)));
            let value = original.eval(|v| int(v as i64 - 1));
            let relation = [Rel::Le, Rel::Ge, Rel::Eq][rng.below(3) as usize];
            assert_eq!(
                a.assert(&e, relation, &value, tag),
                with_reference_axpy(|| b.assert(&original, relation, &value, tag))
            );
            assert_eq!(a.check(), with_reference_axpy(|| b.check()));
            same_canonical_simplex(&a, &b);
        }
        let objective = LinExpr::from_terms([(vars[0], frac(-3, 5)), (vars[2], frac(7, 11))], frac(1, 3));
        let original =
            with_reference_axpy(|| LinExpr::from_terms([(vars[0], frac(-3, 5)), (vars[2], frac(7, 11))], frac(1, 3)));
        let (left, right) = (a.minimize(&objective), with_reference_axpy(|| b.minimize(&original)));
        assert_eq!(left, right);
        same_canonical_parts(left.as_ref().unwrap(), right.as_ref().unwrap());
        same_canonical_simplex(&a, &b);
        let (cp_a, cp_b) = (a.checkpoint(), b.checkpoint());
        assert_eq!(a.keep_optimum(&objective, 12), with_reference_axpy(|| b.keep_optimum(&original, 12)));
        assert_eq!(
            a.assert_bounds(vars[0], Some(int(9)), None, 13),
            with_reference_axpy(|| b.assert_bounds(vars[0], Some(int(9)), None, 13))
        );
        a.backtrack(cp_a);
        with_reference_axpy(|| b.backtrack(cp_b));
        same_canonical_simplex(&a, &b);
        let (mut a, mut b) = (a.clone(), b.clone());
        assert_eq!(a.check(), with_reference_axpy(|| b.check()));
        same_canonical_simplex(&a, &b);
    }
}

#[test]
fn canonical_mul_preserves_controlled_time_budget_resume_and_pivot_trace() {
    use std::cell::RefCell;
    let mut a = chain_of_links();
    let mut b = with_reference_axpy(chain_of_links);
    a.trace = Some(Vec::new());
    b.trace = Some(Vec::new());
    let (reads_a, reads_b) = (RefCell::new(Vec::new()), RefCell::new(Vec::new()));
    let run = |sx: &mut Simplex<usize>, reads: &RefCell<Vec<bool>>| {
        sx.check_until(&|| {
            let mut reads = reads.borrow_mut();
            let expired = !reads.is_empty();
            reads.push(expired);
            expired
        })
    };
    assert_eq!(run(&mut a, &reads_a), Err(Failure::Time));
    assert_eq!(with_reference_axpy(|| run(&mut b, &reads_b)), Err(Failure::Time));
    assert_eq!(*reads_a.borrow(), [false, true]);
    assert_eq!(*reads_a.borrow(), *reads_b.borrow());
    same_canonical_simplex(&a, &b);
    assert_eq!(a.pivots(), 2 * PIVOTS_PER_READ);
    a.set_pivot_limit(Some(a.pivots()));
    b.set_pivot_limit(Some(b.pivots()));
    assert_eq!(a.check(), Err(Failure::Budget));
    assert_eq!(with_reference_axpy(|| b.check()), Err(Failure::Budget));
    same_canonical_simplex(&a, &b);
    a.set_pivot_limit(None);
    b.set_pivot_limit(None);
    assert_eq!(a.check(), Ok(()));
    assert_eq!(with_reference_axpy(|| b.check()), Ok(()));
    same_canonical_simplex(&a, &b);
}

#[test]
fn work_accounting_clone_rollback_and_drop_match_small_oracles() {
    use crate::profile::{self, ProfileEvent, ProfileObserver};
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Default)]
    struct Observer(RefCell<Vec<ProfileEvent>>);
    impl crate::Clock for Observer {
        fn elapsed(&self) -> f64 {
            panic!("work accounting must not read a clock");
        }
    }
    impl ProfileObserver for Observer {
        fn record(&self, e: ProfileEvent) {
            self.0.borrow_mut().push(e);
        }
        fn work_diagnostics(&self) -> bool {
            true
        }
    }

    let observer = Rc::new(Observer::default());
    let mut expected_update_rows = 0;
    let mut expected_update_hits = 0;
    let mut expected_pivot_rows = 0;
    let mut expected_pivot_hits = 0;
    profile::with(Some(observer.clone()), || {
        let mut sx = Simplex::<usize>::new();
        let x = sx.new_var();
        let y = sx.new_var();
        sx.add_row(&LinExpr::diff(x, y));
        sx.add_row(&LinExpr::from_terms([(x, int(1)), (y, int(1))], int(0)));
        sx.add_row(&LinExpr::term(y, int(2)));
        let oracle_nnz = |s: &Simplex<usize>| s.rows.iter().map(|r| r.terms.len() as u64).sum::<u64>();
        assert_eq!(sx.work.nnz(), Some(oracle_nnz(&sx)));
        let hits = sx.rows.iter().filter(|r| r.terms.iter().any(|(v, _)| *v == x)).count() as u64;
        expected_update_rows += hits;
        expected_update_hits += hits;
        sx.assert_bounds(x, Some(int(2)), None, 0).unwrap();
        // Flush the source first: its executed work must not be inherited by the clone.
        profile::work::flush();
        {
            let mut copy = sx.clone();
            assert_eq!(copy.work.nnz(), Some(oracle_nnz(&copy)));
            let hits = copy.rows.iter().filter(|r| r.terms.iter().any(|(v, _)| *v == x)).count() as u64;
            expected_update_rows += hits;
            expected_update_hits += hits;
            copy.assert_bounds(x, Some(int(3)), None, 1).unwrap();
            let cp = copy.checkpoint();
            let r = copy.add_row(&LinExpr::from_terms([(x, int(1)), (y, int(2))], int(0)));
            copy.assert_bounds(r, Some(int(8)), None, 2).unwrap();
            assert!(matches!(copy.assert_bounds(x, None, Some(int(1)), 3), Err(Failure::Conflict(_))));
            copy.backtrack(cp);
            // Bounds roll back; the new row and its nnz remain, as the algorithm specifies.
            assert_eq!(copy.work.nnz(), Some(oracle_nnz(&copy)));
            assert_eq!(copy.rows.len(), 4);
            // Drop before any coarse boundary: all this short-lived work must still be reported.
        }
        assert_eq!(sx.work.nnz(), Some(oracle_nnz(&sx)));
        expected_pivot_hits = sx.rows.iter().skip(1).filter(|r| r.terms.iter().any(|(v, _)| *v == x)).count() as u64;
        expected_pivot_rows = expected_pivot_hits;
        let basic = sx.rows[0].basic;
        sx.pivot_and_update(basic, x, int(5));
        sx.assert_invariants();
        assert_eq!(sx.work.nnz(), Some(oracle_nnz(&sx)));
        profile::work::flush();
        profile::work::flush(); // A second boundary cannot count the same work again.
    });
    let sum = |name| {
        observer
            .0
            .borrow()
            .iter()
            .filter_map(|e| match e {
                ProfileEvent::Work { counters, .. } => Some(counters),
                _ => None,
            })
            .flat_map(|c| c.iter())
            .filter(|(n, _)| *n == name)
            .map(|(_, v)| *v)
            .sum::<u64>()
    };
    assert_eq!(sum("work_tableaux_created"), 1);
    assert_eq!(sum("work_tableaux_cloned"), 1);
    assert_eq!(sum("work_clone_rows"), 3);
    assert_eq!(sum("work_clone_terms"), 5);
    assert_eq!(sum("work_new_rows"), 4);
    assert_eq!(sum("work_update_rows_visited"), expected_update_rows);
    assert_eq!(sum("work_update_rows_hit"), expected_update_hits);
    assert_eq!(sum("work_update_rows_missed"), expected_update_rows - expected_update_hits);
    for (visited, hit) in [
        ("work_pivot_value_rows_visited", "work_pivot_value_rows_hit"),
        ("work_pivot_rewrite_rows_visited", "work_pivot_rewrite_rows_hit"),
    ] {
        assert_eq!(sum(visited), expected_pivot_rows);
        assert_eq!(sum(hit), expected_pivot_hits);
    }
    let (mut disabled, vars) = simplex(2);
    disabled.add_row(&LinExpr::diff(vars[0], vars[1]));
    assert_eq!(disabled.work.nnz(), None);
}

/// Compare complete semantic state and every pivot, excluding diagnostic account identity and
/// the test-only access policy. Membership is independently rebuilt from stored row terms.
fn same_indexed_and_linear(a: &Simplex<usize>, b: &Simplex<usize>) {
    assert_eq!(a.vars, b.vars);
    assert_eq!(a.rows, b.rows);
    assert_eq!(a.forms, b.forms);
    assert_eq!(a.trail, b.trail);
    assert_eq!(a.occurrences, b.occurrences);
    assert_eq!(a.violated, b.violated);
    assert_eq!(a.pivots, b.pivots);
    assert_eq!(a.trace, b.trace);
    a.assert_invariants();
    b.assert_invariants();
}

fn indexed_and_linear(n: usize) -> (Simplex<usize>, Simplex<usize>, Vec<Var>) {
    let (mut a, v) = simplex(n);
    a.trace = Some(Vec::new());
    let mut b = a.clone();
    b.linear_scans = true;
    (a, b, v)
}

#[test]
fn incidence_transfer_cancellation_zero_step_and_later_rows_match_linear_access() {
    let (mut a, mut b, v) = indexed_and_linear(3);
    let sum = LinExpr::from_terms([(v[0], int(1)), (v[1], int(1))], int(0));
    let extended = LinExpr::from_terms([(v[0], int(1)), (v[1], int(1)), (v[2], int(1))], int(0));
    let first = a.add_row(&sum);
    assert_eq!(first, b.add_row(&sum));
    let second = a.add_row(&extended);
    assert_eq!(second, b.add_row(&extended));
    // An unrelated row must not be visited. Extra rows also cross multiple incidence blocks.
    for i in 2..135 {
        let e = LinExpr::from_terms([(v[1], int(i)), (v[2], int(1))], int(0));
        assert_eq!(a.add_row(&e), b.add_row(&e));
    }
    same_indexed_and_linear(&a, &b);
    // theta = 0 still exchanges the basis. y cancels from the second row; the whole x column
    // becomes the first slack's column, including the pivot row, without duplicate xi updates.
    a.pivot_and_update(first, v[0], int(0));
    b.pivot_and_update(first, v[0], int(0));
    same_indexed_and_linear(&a, &b);
    assert!(coefficient(&a.rows[a.vars[second].row.unwrap()].terms, v[1]).is_none());
    assert!(a.incidence[v[0]].is_empty());
    assert_eq!(a.incidence[first].iter().collect::<Vec<_>>(), [0, 1]);

    let e = LinExpr::from_terms([(v[0], int(2)), (v[2], int(-1))], frac(1, 3));
    assert_eq!(a.add_row(&e), b.add_row(&e));
    let cp = a.checkpoint();
    assert_eq!(cp, b.checkpoint());
    assert_eq!(a.assert_bounds(v[0], Some(int(2)), None, 7), b.assert_bounds(v[0], Some(int(2)), None, 7));
    same_indexed_and_linear(&a, &b);
    assert_eq!(a.check(), b.check());
    same_indexed_and_linear(&a, &b);
    a.backtrack(cp);
    b.backtrack(cp);
    same_indexed_and_linear(&a, &b);
    // Neither rows nor the basis is rolled back. A clone owns independent, matching columns.
    let (mut ca, mut cb) = (a.clone(), b.clone());
    assert_eq!(ca.assert_bounds(v[2], None, Some(int(-2)), 8), cb.assert_bounds(v[2], None, Some(int(-2)), 8));
    assert_eq!(ca.check(), cb.check());
    same_indexed_and_linear(&ca, &cb);
    same_indexed_and_linear(&a, &b);
}

#[test]
fn indexed_ratio_preserves_basic_id_ties_and_exact_random_trajectories() {
    let (mut a, mut b, v) = indexed_and_linear(2);
    for sx in [&mut a, &mut b] {
        sx.assert_bounds(v[0], Some(int(0)), None, 0).unwrap();
        sx.assert_bounds(v[1], Some(int(0)), Some(int(4)), 1).unwrap();
        sx.assert(&LinExpr::from_terms([(v[0], int(1)), (v[1], int(1))], int(0)), Rel::Le, &int(4), 2).unwrap();
        sx.assert(&LinExpr::from_terms([(v[0], int(1)), (v[1], int(2))], int(0)), Rel::Le, &int(4), 3).unwrap();
    }
    let objective = LinExpr::term(v[0], int(-1));
    assert_eq!(a.minimize(&objective), Ok(int(-4)));
    assert_eq!(b.minimize(&objective), Ok(int(-4)));
    same_indexed_and_linear(&a, &b);
    assert_eq!((a.trace.as_ref().unwrap()[0].leaving, a.trace.as_ref().unwrap()[0].entering), (2, v[0]));

    let mut rng = Rng(0x1234_5678_9abc_def1);
    for _ in 0..24 {
        let (mut a, mut b, v) = indexed_and_linear(4);
        for &x in &v {
            assert_eq!(
                a.assert_bounds(x, Some(int(-10)), Some(int(10)), x),
                b.assert_bounds(x, Some(int(-10)), Some(int(10)), x)
            );
        }
        for tag in 4..16 {
            let constant = frac(rng.int(-2, 2), 3);
            let e = LinExpr::from_terms(v.iter().map(|&x| (x, frac(rng.int(-3, 3), rng.int(1, 3)))), constant);
            // Every constraint contains this known witness, but the starting geometry need not.
            let c = e.eval(|x| int(x as i64 - 2));
            let rel = match rng.below(3) {
                0 => Rel::Le,
                1 => Rel::Ge,
                _ => Rel::Eq,
            };
            assert_eq!(a.assert(&e, rel, &c, tag), b.assert(&e, rel, &c, tag));
            same_indexed_and_linear(&a, &b);
            assert_eq!(a.check(), b.check());
            same_indexed_and_linear(&a, &b);
        }
        let cp = a.checkpoint();
        assert_eq!(cp, b.checkpoint());
        assert_eq!(a.assert_bounds(v[0], Some(int(20)), None, 20), b.assert_bounds(v[0], Some(int(20)), None, 20));
        same_indexed_and_linear(&a, &b); // immediate conflict must leave the same bounds/tags
        a.backtrack(cp);
        b.backtrack(cp);
        let objective = LinExpr::from_terms([(v[0], frac(2, 3)), (v[3], frac(-1, 2))], int(1));
        assert_eq!(a.minimize(&objective), b.minimize(&objective));
        same_indexed_and_linear(&a, &b);
        assert_eq!(a.keep_optimum(&objective, 21), b.keep_optimum(&objective, 21));
        same_indexed_and_linear(&a, &b);
        assert_eq!(a.minimize(&LinExpr::var(v[2])), b.minimize(&LinExpr::var(v[2])));
        same_indexed_and_linear(&a, &b);
    }
}

#[test]
fn indexed_access_preserves_controlled_time_budget_and_resume() {
    use std::cell::RefCell;
    let mut a = chain_of_links();
    a.trace = Some(Vec::new());
    let mut b = a.clone();
    b.linear_scans = true;
    let (reads_a, reads_b) = (RefCell::new(Vec::new()), RefCell::new(Vec::new()));
    let run = |sx: &mut Simplex<usize>, reads: &RefCell<Vec<bool>>| {
        sx.check_until(&|| {
            let mut r = reads.borrow_mut();
            let passed = !r.is_empty();
            r.push(passed);
            passed
        })
    };
    assert_eq!(run(&mut a, &reads_a), Err(Failure::Time));
    assert_eq!(run(&mut b, &reads_b), Err(Failure::Time));
    assert_eq!(*reads_a.borrow(), [false, true]);
    assert_eq!(*reads_a.borrow(), *reads_b.borrow());
    same_indexed_and_linear(&a, &b);
    assert_eq!(a.pivots(), 2 * PIVOTS_PER_READ);
    a.set_pivot_limit(Some(a.pivots()));
    b.set_pivot_limit(Some(b.pivots()));
    assert_eq!(a.check(), Err(Failure::Budget));
    assert_eq!(b.check(), Err(Failure::Budget));
    same_indexed_and_linear(&a, &b);
    a.set_pivot_limit(None);
    b.set_pivot_limit(None);
    assert_eq!(a.check(), Ok(()));
    assert_eq!(b.check(), Ok(()));
    same_indexed_and_linear(&a, &b);
}

#[test]
fn violated_basics_use_variable_order_and_survive_basis_changes_and_undo() {
    let (mut a, mut b, v) = indexed_and_linear(2);
    let sum = LinExpr::from_terms([(v[0], int(1)), (v[1], int(1))], int(0));
    let other = LinExpr::from_terms([(v[0], int(2)), (v[1], int(1))], int(0));
    let s = a.add_row(&sum);
    assert_eq!(s, b.add_row(&sum));
    let t = a.add_row(&other);
    assert_eq!(t, b.add_row(&other));
    let cp = a.checkpoint();
    assert_eq!(cp, b.checkpoint());
    for sx in [&mut a, &mut b] {
        sx.assert_bounds(v[0], None, Some(int(0)), 1).unwrap();
        sx.assert_bounds(s, None, Some(int(1)), 2).unwrap();
        // A pivot may put its new basic outside its own bounds; update both old/new basics.
        sx.pivot_and_update(t, v[0], int(6));
    }
    same_indexed_and_linear(&a, &b);
    assert_eq!(a.violated.iter().copied().collect::<Vec<_>>(), [v[0], s]);
    assert_eq!(a.rows[0].basic, s);
    assert_eq!(a.rows[1].basic, v[0]);
    assert_eq!(a.violated_pick(None), Some((v[0], false)));
    assert_eq!(b.violated_pick(None), Some((v[0], false)));
    // An immediate conflict must leave the cached set, basis, tags and bounds unchanged.
    assert_eq!(a.assert_bounds(v[0], Some(int(4)), None, 3), Err(Failure::Conflict(vec![1, 3])));
    assert_eq!(b.assert_bounds(v[0], Some(int(4)), None, 3), Err(Failure::Conflict(vec![1, 3])));
    same_indexed_and_linear(&a, &b);
    let (mut ca, mut cb) = (a.clone(), b.clone());
    assert_eq!(ca.check(), Ok(()));
    assert_eq!(cb.check(), Ok(()));
    same_indexed_and_linear(&ca, &cb);
    // Undo acts on the current basis, not the basis at the checkpoint. Retained rows stay.
    ca.backtrack(cp);
    cb.backtrack(cp);
    assert!(ca.violated.is_empty());
    same_indexed_and_linear(&ca, &cb);
    same_indexed_and_linear(&a, &b);
    a.backtrack(cp);
    b.backtrack(cp);
    assert!(a.violated.is_empty());
    same_indexed_and_linear(&a, &b);
}

#[test]
fn a_minimum_bound_flip_and_optimum_bounds_keep_the_violation_cache_exact() {
    let (mut a, mut b, v) = indexed_and_linear(1);
    for sx in [&mut a, &mut b] {
        sx.assert_bounds(v[0], Some(int(0)), Some(int(3)), 0).unwrap();
    }
    let objective = LinExpr::term(v[0], int(-1));
    assert_eq!(a.minimize(&objective), Ok(int(-3)));
    assert_eq!(b.minimize(&objective), Ok(int(-3)));
    same_indexed_and_linear(&a, &b);
    assert_eq!(a.pivots(), 1);
    assert_eq!(a.trace.as_ref().unwrap()[0].entering, a.trace.as_ref().unwrap()[0].leaving);
    assert!(a.violated.is_empty());
    assert_eq!(a.keep_optimum(&objective, 1), Ok(()));
    assert_eq!(b.keep_optimum(&objective, 1), Ok(()));
    same_indexed_and_linear(&a, &b);
}

#[test]
fn violation_work_counts_selection_separately_from_incremental_refresh() {
    use crate::profile::{self, ProfileEvent, ProfileObserver};
    use std::cell::RefCell;
    use std::rc::Rc;
    #[derive(Default)]
    struct Observer(RefCell<Vec<ProfileEvent>>);
    impl crate::Clock for Observer {
        fn elapsed(&self) -> f64 {
            panic!("violation maintenance must not read a clock");
        }
    }
    impl ProfileObserver for Observer {
        fn record(&self, event: ProfileEvent) {
            self.0.borrow_mut().push(event);
        }
        fn work_diagnostics(&self) -> bool {
            true
        }
    }
    let observer = Rc::new(Observer::default());
    profile::with(Some(observer.clone()), || {
        let (mut sx, v) = simplex(2);
        let s = sx.add_row(&LinExpr::from_terms([(v[0], int(1)), (v[1], int(1))], int(0)));
        sx.add_row(&LinExpr::term(v[1], int(2))); // not involved in the chosen pivot
        sx.assert_bounds(s, Some(int(2)), None, 1).unwrap();
        assert_eq!(sx.check(), Ok(()));
        sx.assert_invariants();
    });
    let sum = |name| {
        observer
            .0
            .borrow()
            .iter()
            .filter_map(|e| match e {
                ProfileEvent::Work { counters, .. } => Some(counters),
                _ => None,
            })
            .flat_map(|c| c.iter())
            .filter(|(n, _)| *n == name)
            .map(|(_, v)| v)
            .sum::<u64>()
    };
    assert_eq!(sum("work_check_passes"), 2);
    assert_eq!(sum("work_check_rows_examined"), 1);
    assert_eq!(sum("work_check_violations"), 1);
    assert_eq!(sum("work_violation_refreshes"), 3);
    assert_eq!(sum("work_violation_basic_checks"), 2);
    assert_eq!(sum("work_violation_membership_changes"), 2);
}

/// A fixed xorshift sequence, so every run tests the same systems.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    /// An integer from `lo` to `hi`, both included.
    fn int(&mut self, lo: i64, hi: i64) -> i64 {
        lo + self.below((hi - lo + 1) as u64) as i64
    }
}

/// One constraint of a test: `Σ a x rel c`, the variables by their index among the test's.
#[derive(Clone, Debug)]
struct Constraint {
    terms: Vec<(usize, i64)>,
    rel: Rel,
    c: Q,
}

impl Constraint {
    fn expr(&self, vars: &[Var]) -> LinExpr {
        LinExpr::from_terms(self.terms.iter().map(|&(i, a)| (vars[i], int(a))), Q::zero())
    }

    fn holds(&self, value: impl Fn(usize) -> Q) -> bool {
        let lhs = self.terms.iter().fold(Q::zero(), |s, &(i, a)| s + int(a) * value(i));
        match self.rel {
            Rel::Le => lhs <= self.c,
            Rel::Ge => lhs >= self.c,
            Rel::Eq => lhs == self.c,
        }
    }

    /// As `Σ a x ≤ b` rows: one, or two for an equality.
    fn as_le(&self) -> Vec<(Vec<Q>, Q)> {
        let n = self.terms.iter().map(|&(i, _)| i + 1).max().unwrap_or(0);
        let mut row = vec![Q::zero(); n];
        for &(i, a) in &self.terms {
            row[i] += int(a);
        }
        let neg = |r: &Vec<Q>| r.iter().map(|a| -a).collect::<Vec<Q>>();
        match self.rel {
            Rel::Le => vec![(row, self.c.clone())],
            Rel::Ge => vec![(neg(&row), -self.c.clone())],
            Rel::Eq => vec![(neg(&row), -self.c.clone()), (row, self.c.clone())],
        }
    }
}

fn simplex(n: usize) -> (Simplex<usize>, Vec<Var>) {
    let mut sx = Simplex::new();
    let vars = (0..n).map(|_| sx.new_var()).collect();
    (sx, vars)
}

fn le(terms: &[(usize, i64)], c: i64) -> Constraint {
    Constraint { terms: terms.to_vec(), rel: Rel::Le, c: int(c) }
}

fn ge(terms: &[(usize, i64)], c: i64) -> Constraint {
    Constraint { terms: terms.to_vec(), rel: Rel::Ge, c: int(c) }
}

fn eq(terms: &[(usize, i64)], c: i64) -> Constraint {
    Constraint { terms: terms.to_vec(), rel: Rel::Eq, c: int(c) }
}

/// What became of each constraint: kept, or discarded with the explanation of the conflict.
type Outcomes = Vec<Result<(), Vec<usize>>>;

/// Takes the constraints in one at a time as the resolution of 8.5 does: each that fits with
/// those kept is kept, each that does not is undone. Gives the result of each, and checks the
/// invariants and that the values solve what is kept after each step.
fn resolve(cs: &[Constraint], n: usize) -> (Simplex<usize>, Vec<Var>, Outcomes) {
    resolve_with(cs, n, BLAND_AFTER)
}

/// [`resolve`] with Bland's rule after `bland_after` repeats ([`Simplex::set_bland_after`]).
fn resolve_with(cs: &[Constraint], n: usize, bland_after: u64) -> (Simplex<usize>, Vec<Var>, Outcomes) {
    let (mut sx, vars) = simplex(n);
    sx.set_bland_after(bland_after);
    let mut kept: Vec<usize> = Vec::new();
    let mut results = Vec::new();
    for (i, c) in cs.iter().enumerate() {
        let cp = sx.checkpoint();
        let r = sx.assert(&c.expr(&vars), c.rel, &c.c, i).and_then(|()| sx.check());
        sx.assert_invariants();
        match r {
            Ok(()) => {
                kept.push(i);
                for &k in &kept {
                    assert!(cs[k].holds(|j| sx.value(vars[j]).clone()), "constraint {k} fails after {i}");
                }
                results.push(Ok(()));
            }
            Err(Failure::Conflict(tags)) => {
                assert!(tags.contains(&i), "the explanation {tags:?} of {i} leaves it out");
                assert!(tags.iter().all(|t| *t == i || kept.contains(t)), "{tags:?} is not among {kept:?} and {i}");
                sx.backtrack(cp);
                assert_eq!(sx.check(), Ok(()), "what was kept before {i} has a solution");
                sx.assert_invariants();
                results.push(Err(tags));
            }
            Err(e) => panic!("{e:?}"),
        }
    }
    (sx, vars, results)
}

// ------------------------------------------------------------------------------ by hand

#[test]
fn linear_forms_merge_sort_and_cancel() {
    let e = LinExpr::from_terms([(3, int(2)), (1, int(1)), (3, int(-2)), (0, frac(1, 2)), (1, int(1))], int(4));
    assert_eq!(e.terms(), &[(0, frac(1, 2)), (1, int(2))]);
    assert_eq!(e.constant_part(), &int(4));
    assert!(!e.is_constant());
    let mut f = LinExpr::diff(1, 0);
    f.add_scaled(&e, &int(-2));
    // (x1 − x0) − 2 (x0 / 2 + 2 x1 + 4) = −2 x0 − 3 x1 − 8
    assert_eq!(f.terms(), &[(0, int(-2)), (1, int(-3))]);
    assert_eq!(f.constant_part(), &int(-8));
    assert_eq!(f.eval(|v| int(v as i64 + 1)), int(-2 - 6 - 8));
    f.add_scaled(&f.clone(), &int(-1));
    assert!(f.is_constant() && f.constant_part().is_zero());
    assert_eq!(LinExpr::term(5, int(0)), LinExpr::default());
    let (mut sx, v) = simplex(2);
    assert_eq!(sx.var_count(), 2);
    sx.add_row(&LinExpr::from_terms([(v[0], int(1)), (v[1], int(1))], int(0)));
    assert_eq!((sx.var_count(), sx.row_count()), (3, 1));
}

#[test]
fn opposite_bounds_conflict_at_once_and_change_nothing() {
    let (mut sx, v) = simplex(1);
    assert_eq!(sx.assert(&LinExpr::var(v[0]), Rel::Ge, &int(1), 7), Ok(()));
    assert_eq!(sx.assert(&LinExpr::var(v[0]), Rel::Le, &int(0), 3), Err(Failure::Conflict(vec![3, 7])));
    assert!(sx.upper(v[0]).is_none());
    assert_eq!(sx.check(), Ok(()));
    assert_eq!(*sx.value(v[0]), int(1));
    // both sides of one assertion conflict with each other alone
    assert_eq!(sx.assert_bounds(v[0], Some(int(5)), Some(int(4)), 9), Err(Failure::Conflict(vec![9])));
}

#[test]
fn a_cycle_of_distances_is_explained_by_all_of_it() {
    // DISTANCE(x, A→B, min 10), DISTANCE(x, B→C, min 10), DISTANCE(x, A→C, max 15)
    let (a, b, c) = (0, 1, 2);
    let cs = [ge(&[(b, 1), (a, -1)], 10), ge(&[(c, 1), (b, -1)], 10), le(&[(c, 1), (a, -1)], 15)];
    let (_, _, results) = resolve(&cs, 3);
    assert_eq!(results, vec![Ok(()), Ok(()), Err(vec![0, 1, 2])]);
}

#[test]
fn three_variable_equalities_are_exact() {
    // DISTRIBUTE(x: A, B, C), DISTANCE(x, A→B, exact 10), DISTANCE(x, B→C, exact 15)
    let (a, b, c) = (0, 1, 2);
    let cs = [eq(&[(a, 1), (b, -2), (c, 1)], 0), eq(&[(b, 1), (a, -1)], 10), eq(&[(c, 1), (b, -1)], 15)];
    let (_, _, results) = resolve(&cs, 3);
    assert_eq!(results, vec![Ok(()), Ok(()), Err(vec![0, 1, 2])]);
    // and with exact 10 twice, C is 10 beyond B
    let cs = [eq(&[(a, 1), (b, -2), (c, 1)], 0), eq(&[(b, 1), (a, -1)], 10), eq(&[(a, 1)], 3)];
    let (sx, v, results) = resolve(&cs, 3);
    assert!(results.iter().all(Result::is_ok));
    assert_eq!((sx.value(v[0]), sx.value(v[1]), sx.value(v[2])), (&int(3), &int(13), &int(23)));
}

#[test]
fn distribute_with_distances_conflicts() {
    // DISTRIBUTE(x: A, B, C), DISTANCE(x, A→C, min 30), DISTANCE(x, A→B, max 10)
    let (a, b, c) = (0, 1, 2);
    let cs = [eq(&[(a, 1), (b, -2), (c, 1)], 0), ge(&[(c, 1), (a, -1)], 30), le(&[(b, 1), (a, -1)], 10)];
    let (_, _, results) = resolve(&cs, 3);
    assert_eq!(results, vec![Ok(()), Ok(()), Err(vec![0, 1, 2])]);
}

#[test]
fn a_conflict_leaves_out_what_it_does_not_need() {
    let (a, b, c) = (0, 1, 2);
    // the bound on C is no part of the conflict of the others
    let cs = [ge(&[(c, 1)], 100), ge(&[(b, 1), (a, -1)], 5), le(&[(b, 1), (a, -1)], 4)];
    let (_, _, results) = resolve(&cs, 3);
    assert_eq!(results[2], Err(vec![1, 2]));
}

#[test]
fn fractions_stay_exact() {
    // 3 x = 1 and y = x: y is 1/3, not a float near it
    let (x, y) = (0, 1);
    let cs = [eq(&[(x, 3)], 1), eq(&[(y, 1), (x, -1)], 0)];
    let (sx, v, results) = resolve(&cs, 2);
    assert!(results.iter().all(Result::is_ok));
    assert_eq!(*sx.value(v[1]), frac(1, 3));
}

#[test]
fn undo_puts_the_earlier_bounds_back() {
    let (mut sx, v) = simplex(2);
    let d = LinExpr::diff(v[1], v[0]);
    sx.assert(&d, Rel::Ge, &int(5), 1).unwrap();
    assert_eq!(sx.check(), Ok(()));
    let cp = sx.checkpoint();
    sx.assert(&d.scaled(&int(-1)), Rel::Ge, &int(0), 2).unwrap_err();
    sx.assert(&LinExpr::var(v[0]), Rel::Ge, &int(10), 3).unwrap();
    sx.assert(&LinExpr::var(v[1]), Rel::Le, &int(12), 4).unwrap();
    assert_eq!(sx.check(), Err(Failure::Conflict(vec![1, 3, 4])));
    sx.backtrack(cp);
    assert!(sx.lower(v[0]).is_none() && sx.upper(v[1]).is_none());
    assert_eq!(sx.check(), Ok(()));
    sx.assert(&LinExpr::var(v[1]), Rel::Le, &int(12), 5).unwrap();
    assert_eq!(sx.check(), Ok(()));
    assert!(sx.value(v[1]) - sx.value(v[0]) >= int(5));
    sx.assert_invariants();
}

#[test]
fn a_stronger_bound_replaces_and_an_equal_one_keeps_the_first_tag() {
    let (mut sx, v) = simplex(1);
    let x = LinExpr::var(v[0]);
    sx.assert(&x, Rel::Le, &int(5), 1).unwrap();
    let cp = sx.checkpoint();
    sx.assert(&x, Rel::Le, &int(5), 2).unwrap();
    assert_eq!(sx.upper(v[0]), Some((&int(5), &1)));
    sx.assert(&x, Rel::Le, &int(7), 3).unwrap();
    assert_eq!(sx.upper(v[0]), Some((&int(5), &1)));
    sx.assert(&x, Rel::Le, &int(3), 4).unwrap();
    assert_eq!(sx.upper(v[0]), Some((&int(3), &4)));
    sx.backtrack(cp);
    assert_eq!(sx.upper(v[0]), Some((&int(5), &1)));
}

#[test]
fn one_form_is_one_row_whatever_its_factor() {
    let (mut sx, v) = simplex(2);
    let d = LinExpr::diff(v[1], v[0]);
    let s = sx.add_row(&d);
    assert_eq!(sx.add_row(&d), s);
    assert_eq!(sx.add_row(&LinExpr::var(v[0])), v[0]);
    assert_eq!(sx.row_count(), 1);
    // assert divides by the first coefficient, so 2 (b − a) ≥ 4, −(b − a) ≥ −3 and
    // b − a + 1 ≤ 3 all bound one row, a − b: −3 ≤ a − b ≤ −2, then −2 ≤ a − b
    sx.assert(&d.scaled(&int(2)), Rel::Ge, &int(4), 1).unwrap();
    sx.assert(&d.scaled(&int(-1)), Rel::Ge, &int(-3), 2).unwrap();
    let mut shifted = d.clone();
    shifted.add_constant(&int(1));
    sx.assert(&shifted, Rel::Le, &int(3), 3).unwrap();
    assert_eq!(sx.row_count(), 2);
    let r = sx.add_row(&LinExpr::diff(v[0], v[1]));
    assert_eq!(sx.upper(r), Some((&int(-2), &1)));
    assert_eq!(sx.lower(r), Some((&int(-2), &3)));
    assert_eq!(sx.check(), Ok(()));
    assert_eq!(sx.eval(&d), int(2));
    assert_eq!(*sx.value(s), int(2));
}

#[test]
fn a_form_without_variables_holds_or_conflicts_alone() {
    let (mut sx, _) = simplex(0);
    assert_eq!(sx.assert(&LinExpr::constant(int(3)), Rel::Le, &int(5), 1), Ok(()));
    assert_eq!(sx.assert(&LinExpr::constant(int(3)), Rel::Ge, &int(5), 2), Err(Failure::Conflict(vec![2])));
    assert_eq!(sx.assert(&LinExpr::constant(int(3)), Rel::Eq, &int(3), 3), Ok(()));
}

#[test]
fn the_optimum_of_a_form() {
    let (mut sx, v) = simplex(2);
    let (x, y) = (v[0], v[1]);
    sx.assert(&LinExpr::var(x), Rel::Ge, &int(1), 1).unwrap();
    sx.assert(&LinExpr::diff(y, x), Rel::Ge, &int(2), 2).unwrap();
    assert_eq!(sx.minimize(&LinExpr::var(y)), Ok(int(3)));
    assert_eq!(*sx.value(y), int(3));
    let sum = LinExpr::from_terms([(x, int(1)), (y, int(1))], int(10));
    assert_eq!(sx.minimize(&sum), Ok(int(14)));
    assert_eq!(sx.minimize(&LinExpr::term(y, int(-1))), Err(Failure::Unbounded));
    sx.assert(&LinExpr::var(y), Rel::Le, &int(10), 3).unwrap();
    assert_eq!(sx.minimize(&LinExpr::term(y, int(-1))), Ok(int(-10)));
    assert_eq!(*sx.value(y), int(10));
    // fixing the optimum keeps it for the next layer: x is then as large as it can be
    sx.assert(&LinExpr::var(y), Rel::Eq, &int(10), 4).unwrap();
    assert_eq!(sx.minimize(&LinExpr::term(x, int(-1))), Ok(int(-8)));
    sx.assert_invariants();
    assert!(sx.satisfied());
}

#[test]
fn the_optimum_of_an_absolute_value() {
    // |x − y| through t ≥ x − y and t ≥ y − x, with x = 3 and y ≥ 7: 4
    let (mut sx, v) = simplex(3);
    let (x, y, t) = (v[0], v[1], v[2]);
    sx.assert(&LinExpr::from_terms([(t, int(1)), (x, int(-1)), (y, int(1))], int(0)), Rel::Ge, &int(0), 1).unwrap();
    sx.assert(&LinExpr::from_terms([(t, int(1)), (x, int(1)), (y, int(-1))], int(0)), Rel::Ge, &int(0), 2).unwrap();
    sx.assert(&LinExpr::var(x), Rel::Eq, &int(3), 3).unwrap();
    sx.assert(&LinExpr::var(y), Rel::Ge, &int(7), 4).unwrap();
    assert_eq!(sx.minimize(&LinExpr::var(t)), Ok(int(4)));
    assert_eq!((sx.value(x), sx.value(y)), (&int(3), &int(7)));
}

#[test]
fn an_objective_is_the_row_of_its_form() {
    let (mut sx, v) = simplex(2);
    let (x, y) = (v[0], v[1]);
    let sum = LinExpr::from_terms([(x, int(1)), (y, int(1))], int(0));
    sx.assert(&LinExpr::var(x), Rel::Ge, &int(0), 1).unwrap();
    sx.assert(&LinExpr::var(y), Rel::Ge, &int(0), 2).unwrap();
    sx.assert(&sum, Rel::Ge, &int(3), 3).unwrap();
    assert_eq!(sx.check(), Ok(()));
    let s = sx.add_row(&sum);
    assert_eq!(sx.row_count(), 1);
    // the check made the row of x + y nonbasic at its bound: the objective is that variable
    assert_eq!(*sx.value(s), int(3));
    // 2 (x + y) + 1, and −(x + y), are the same row, whatever their factor and constant
    let mut twice = sum.scaled(&int(2));
    twice.add_constant(&int(1));
    assert_eq!(sx.minimize(&twice), Ok(int(7)));
    assert_eq!(sx.minimize(&sum.scaled(&int(-1))), Err(Failure::Unbounded));
    assert_eq!(sx.row_count(), 1);
    sx.assert(&LinExpr::var(x), Rel::Le, &int(5), 4).unwrap();
    sx.assert(&LinExpr::var(y), Rel::Le, &int(4), 5).unwrap();
    assert_eq!(sx.minimize(&sum.scaled(&int(-1))), Ok(int(-9)));
    assert_eq!((sx.value(x), sx.value(y)), (&int(5), &int(4)));
    // a layer bounds the form by its optimum, on the same row; the next layer works within it
    sx.assert(&sum, Rel::Ge, &int(9), 6).unwrap();
    assert_eq!(sx.minimize(&LinExpr::var(x)), Ok(int(5)));
    assert_eq!(sx.minimize(&twice), Ok(int(19)));
    // an objective without variables is its constant, and makes no row
    assert_eq!(sx.minimize(&LinExpr::constant(int(-2))), Ok(int(-2)));
    assert_eq!(sx.row_count(), 1);
    sx.assert_invariants();
}

#[test]
fn an_infeasible_minimum_is_a_conflict() {
    // b − a ≥ 1, c − b ≥ 1, a − c ≥ 1: three rows, no solution
    let (mut sx, v) = simplex(3);
    sx.assert(&LinExpr::diff(v[1], v[0]), Rel::Ge, &int(1), 1).unwrap();
    sx.assert(&LinExpr::diff(v[2], v[1]), Rel::Ge, &int(1), 2).unwrap();
    sx.assert(&LinExpr::diff(v[0], v[2]), Rel::Ge, &int(1), 3).unwrap();
    assert_eq!(sx.minimize(&LinExpr::var(v[0])), Err(Failure::Conflict(vec![1, 2, 3])));
}

#[test]
fn the_budget_stops_the_pivots() {
    let cs = [ge(&[(1, 1), (0, -1)], 10), ge(&[(2, 1), (1, -1)], 10), le(&[(2, 1), (0, -1)], 25), eq(&[(0, 1)], 5)];
    let (mut sx, v) = simplex(3);
    sx.set_pivot_limit(Some(0));
    for (i, c) in cs.iter().enumerate() {
        sx.assert(&c.expr(&v), c.rel, &c.c, i).unwrap();
    }
    assert_eq!(sx.check(), Err(Failure::Budget));
    sx.set_pivot_limit(None);
    assert_eq!(sx.check(), Ok(()));
    let used = sx.pivots();
    assert!(used > 0);
    sx.set_pivot_limit(Some(used));
    assert_eq!(sx.minimize(&LinExpr::term(v[2], int(-1))), Err(Failure::Budget));
    sx.set_pivot_limit(None);
    assert_eq!(sx.minimize(&LinExpr::term(v[2], int(-1))), Ok(int(-30)));
}

/// A chain `xᵢ₊₁ − xᵢ ≥ 1` of 60 variables, all at 0: its check pivots once for each link or more.
fn chain_of_links() -> Simplex<usize> {
    let n = 60;
    let (mut sx, v) = simplex(n);
    for i in 0..n - 1 {
        sx.assert(&LinExpr::diff(v[i + 1], v[i]), Rel::Ge, &int(1), i).unwrap();
    }
    sx
}

#[test]
fn a_check_stops_when_its_time_is_out_and_goes_on_after() {
    // the time limit of the resolution (the draft 0.3, 8.5): asked every PIVOTS_PER_READ pivots
    let mut sx = chain_of_links();
    let reads = std::cell::Cell::new(0);
    let out = || {
        reads.set(reads.get() + 1);
        reads.get() >= 2
    };
    assert_eq!(sx.check_until(&out), Err(Failure::Time));
    assert_eq!((reads.get(), sx.pivots()), (2, 2 * PIVOTS_PER_READ));
    sx.assert_invariants();
    assert!(!sx.satisfied());
    // a check after it goes on from where it stopped
    assert_eq!(sx.check(), Ok(()));
    assert!(sx.satisfied());
    sx.assert_invariants();
    // a time that is never out changes nothing: the same pivots as a check without it
    let (mut a, mut b) = (chain_of_links(), chain_of_links());
    let asked = std::cell::Cell::new(0);
    assert_eq!(
        a.check_until(&|| {
            asked.set(asked.get() + 1);
            false
        }),
        Ok(())
    );
    assert_eq!(b.check(), Ok(()));
    assert_eq!(a.pivots(), b.pivots());
    assert!(a.pivots() >= 59);
    assert_eq!(asked.get(), (a.pivots() - 1) / PIVOTS_PER_READ);
}

// ------------------------------------------------------------------------------ deciders

/// Whether difference constraints, `x_j − x_i rel c` or `x_i rel c`, have a solution: no
/// negative cycle in their graph (Bellman–Ford). Variable `n` is the zero the bounds are
/// differences from.
fn bellman_ford(cs: &[&Constraint], n: usize) -> bool {
    let mut edges: Vec<(usize, usize, Q)> = Vec::new();
    for c in cs {
        // to − from rel c
        let (to, from) = match c.terms.as_slice() {
            [(i, 1)] => (*i, n),
            [(i, -1)] => (n, *i),
            [(j, 1), (i, -1)] | [(i, -1), (j, 1)] => (*j, *i),
            other => panic!("not a difference: {other:?}"),
        };
        if matches!(c.rel, Rel::Le | Rel::Eq) {
            edges.push((from, to, c.c.clone()));
        }
        if matches!(c.rel, Rel::Ge | Rel::Eq) {
            edges.push((to, from, -c.c.clone()));
        }
    }
    let mut dist = vec![Q::zero(); n + 1];
    for _ in 0..=n + 1 {
        let mut changed = false;
        for (u, v, w) in &edges {
            let d = &dist[*u] + w;
            if d < dist[*v] {
                dist[*v] = d;
                changed = true;
            }
        }
        if !changed {
            return true;
        }
    }
    false
}

/// Whether equalities have a solution (Gaussian elimination over Q).
fn gauss(cs: &[&Constraint], n: usize) -> bool {
    let mut rows: Vec<Vec<Q>> = cs
        .iter()
        .map(|c| {
            assert_eq!(c.rel, Rel::Eq);
            let mut row = vec![Q::zero(); n + 1];
            for &(i, a) in &c.terms {
                row[i] += int(a);
            }
            row[n] = c.c.clone();
            row
        })
        .collect();
    let mut rank = 0;
    for col in 0..n {
        let Some(p) = (rank..rows.len()).find(|&r| !rows[r][col].is_zero()) else { continue };
        rows.swap(rank, p);
        let pivot = rows[rank].clone();
        for (r, row) in rows.iter_mut().enumerate() {
            if r != rank && !row[col].is_zero() {
                let f = &row[col] / &pivot[col];
                for k in col..=n {
                    let d = &f * &pivot[k];
                    row[k] -= d;
                }
            }
        }
        rank += 1;
    }
    rows[rank..].iter().all(|row| row[n].is_zero())
}

/// Rows `Σ a x ≤ b` over `n` variables, then Fourier–Motzkin elimination of variables `from..n`
/// (all when `from` is 0): the rows left over the first `from` variables, or none when the
/// system has no solution.
fn fourier_motzkin(cs: &[&Constraint], n: usize, extra: &[(Vec<Q>, Q)], keep: usize) -> Option<Vec<(Vec<Q>, Q)>> {
    let pad = |mut a: Vec<Q>| {
        a.resize(n, Q::zero());
        a
    };
    let mut rows: Vec<(Vec<Q>, Q)> =
        cs.iter().flat_map(|c| c.as_le()).map(|(a, b)| (pad(a), b)).chain(extra.iter().cloned()).collect();
    for k in (keep..n).rev() {
        let (mut pos, mut neg, mut rest) = (Vec::new(), Vec::new(), Vec::new());
        for (a, b) in rows {
            if a[k].is_positive() {
                pos.push((a, b));
            } else if a[k].is_negative() {
                neg.push((a, b));
            } else {
                rest.push((a, b));
            }
        }
        for (p, pb) in &pos {
            for (q, qb) in &neg {
                // p / p_k + q / |q_k| has no x_k
                let (fp, fq) = (p[k].recip(), -q[k].recip());
                let a: Vec<Q> = p.iter().zip(q).map(|(x, y)| x * &fp + y * &fq).collect();
                rest.push((a, pb * &fp + qb * &fq));
            }
        }
        // normalise and drop repeats and rows without variables that hold
        let mut seen: BTreeMap<Vec<Q>, Q> = BTreeMap::new();
        for (a, b) in rest {
            let scale = a.iter().find(|x| !x.is_zero()).map(|x| x.abs());
            match scale {
                None if b.is_negative() => return None,
                None => {}
                Some(s) => {
                    let a: Vec<Q> = a.iter().map(|x| x / &s).collect();
                    let b = b / &s;
                    let e = seen.entry(a).or_insert(b.clone());
                    if b < *e {
                        *e = b;
                    }
                }
            }
        }
        rows = seen.into_iter().collect();
    }
    if rows.iter().any(|(a, b)| a.iter().all(Zero::is_zero) && b.is_negative()) {
        return None;
    }
    Some(rows)
}

/// The minimum of `Σ cᵢ xᵢ` over the constraints by Fourier–Motzkin: z = objective, every x
/// eliminated, the largest lower bound left on z. Err(()) when infeasible, Ok(None) when
/// unbounded.
fn fm_minimum(cs: &[&Constraint], n: usize, objective: &[i64]) -> Result<Option<Q>, ()> {
    // variables: z, then x_0 … x_{n-1}; the constraints shifted by one
    let shifted: Vec<Constraint> = cs
        .iter()
        .map(|c| Constraint { terms: c.terms.iter().map(|&(i, a)| (i + 1, a)).collect(), rel: c.rel, c: c.c.clone() })
        .collect();
    let refs: Vec<&Constraint> = shifted.iter().collect();
    let mut row: Vec<Q> = vec![int(1)];
    row.extend(objective.iter().map(|&c| int(-c)));
    let neg: Vec<Q> = row.iter().map(|x| -x).collect();
    let rows = fourier_motzkin(&refs, n + 1, &[(row, Q::zero()), (neg, Q::zero())], 1).ok_or(())?;
    // a z ≤ b with a < 0 is z ≥ b / a
    Ok(rows.iter().filter(|(a, _)| a[0].is_negative()).map(|(a, b)| b / &a[0]).max())
}

// ------------------------------------------------------------------------------ against them

/// A random difference constraint over `n` variables, constants from −10 to 10, a few halves.
fn random_difference(rng: &mut Rng, n: usize) -> Constraint {
    let i = rng.below(n as u64) as usize;
    let j = (i + 1 + rng.below(n as u64 - 1) as usize) % n;
    let terms =
        if rng.below(4) == 0 { vec![(i, if rng.below(2) == 0 { 1 } else { -1 })] } else { vec![(j, 1), (i, -1)] };
    let rel = [Rel::Le, Rel::Ge, Rel::Eq, Rel::Ge][rng.below(4) as usize];
    let c = if rng.below(5) == 0 { frac(rng.int(-20, 20), 2) } else { int(rng.int(-10, 10)) };
    Constraint { terms, rel, c }
}

#[test]
fn difference_constraints_agree_with_bellman_ford() {
    let mut rng = Rng(0x5eed_0001);
    let (mut kept_total, mut discarded_total) = (0, 0);
    for _ in 0..1000 {
        let n = 2 + rng.below(6) as usize;
        let m = 1 + rng.below(18) as usize;
        let cs: Vec<Constraint> = (0..m).map(|_| random_difference(&mut rng, n)).collect();
        let (_, _, results) = resolve(&cs, n);
        let mut kept: Vec<&Constraint> = Vec::new();
        for (i, r) in results.iter().enumerate() {
            let mut with = kept.clone();
            with.push(&cs[i]);
            let feasible = bellman_ford(&with, n);
            match r {
                Ok(()) => {
                    assert!(feasible, "kept {i} of {cs:?}");
                    kept.push(&cs[i]);
                    kept_total += 1;
                }
                Err(tags) => {
                    assert!(!feasible, "discarded {i} of {cs:?}");
                    let subset: Vec<&Constraint> = tags.iter().map(|&t| &cs[t]).collect();
                    assert!(!bellman_ford(&subset, n), "the explanation {tags:?} of {i} has a solution: {cs:?}");
                    discarded_total += 1;
                }
            }
        }
    }
    // the systems are neither all feasible nor all conflicting
    assert!(kept_total > 2000 && discarded_total > 1000, "{kept_total} kept, {discarded_total} discarded");
}

/// A random equality of the kinds the facts make: a DISTRIBUTE step, a difference, a fixed
/// coordinate, or any three variables with small coefficients.
fn random_equality(rng: &mut Rng, n: usize) -> Constraint {
    let kind = rng.below(4);
    let mut pick = |k: usize| {
        let mut vs: Vec<usize> = Vec::new();
        while vs.len() < k {
            let v = rng.below(n as u64) as usize;
            if !vs.contains(&v) {
                vs.push(v);
            }
        }
        vs
    };
    let terms = match kind {
        0 => {
            let v = pick(3);
            vec![(v[0], 1), (v[1], -2), (v[2], 1)]
        }
        1 => {
            let v = pick(2);
            vec![(v[0], 1), (v[1], -1)]
        }
        2 => vec![(pick(1)[0], 1)],
        _ => {
            let v = pick(3);
            let mut coefficient = || {
                let c = rng.int(-3, 2);
                if c >= 0 {
                    c + 1
                } else {
                    c
                }
            };
            vec![(v[0], coefficient()), (v[1], coefficient()), (v[2], coefficient())]
        }
    };
    let c = if rng.below(3) == 0 { int(0) } else { int(rng.int(-12, 12)) };
    Constraint { terms, rel: Rel::Eq, c }
}

#[test]
fn equalities_of_three_variables_agree_with_gaussian_elimination() {
    let mut rng = Rng(0x5eed_0002);
    let (mut kept_total, mut discarded_total) = (0, 0);
    for _ in 0..800 {
        let n = 3 + rng.below(4) as usize;
        let m = 1 + rng.below(9) as usize;
        let cs: Vec<Constraint> = (0..m).map(|_| random_equality(&mut rng, n)).collect();
        let (_, _, results) = resolve(&cs, n);
        let mut kept: Vec<&Constraint> = Vec::new();
        for (i, r) in results.iter().enumerate() {
            let mut with = kept.clone();
            with.push(&cs[i]);
            let feasible = gauss(&with, n);
            match r {
                Ok(()) => {
                    assert!(feasible, "kept {i} of {cs:?}");
                    kept.push(&cs[i]);
                    kept_total += 1;
                }
                Err(tags) => {
                    assert!(!feasible, "discarded {i} of {cs:?}");
                    let subset: Vec<&Constraint> = tags.iter().map(|&t| &cs[t]).collect();
                    assert!(!gauss(&subset, n), "the explanation {tags:?} of {i} has a solution: {cs:?}");
                    discarded_total += 1;
                }
            }
        }
    }
    assert!(kept_total > 1500 && discarded_total > 500, "{kept_total} kept, {discarded_total} discarded");
}

/// A random constraint over one to three of `n` variables, coefficients from −3 to 3.
fn random_linear(rng: &mut Rng, n: usize) -> Constraint {
    let k = 1 + rng.below(n.min(3) as u64) as usize;
    let mut terms: Vec<(usize, i64)> = Vec::new();
    while terms.len() < k {
        let v = rng.below(n as u64) as usize;
        let a = rng.int(-3, 3);
        if a != 0 && terms.iter().all(|&(w, _)| w != v) {
            terms.push((v, a));
        }
    }
    let rel = [Rel::Le, Rel::Ge, Rel::Eq, Rel::Le, Rel::Ge][rng.below(5) as usize];
    Constraint { terms, rel, c: int(rng.int(-8, 8)) }
}

#[test]
fn linear_constraints_and_optima_agree_with_fourier_motzkin() {
    let mut rng = Rng(0x5eed_0003);
    let (mut kept_total, mut discarded_total, mut optima, mut unbounded) = (0, 0, 0, 0);
    for _ in 0..800 {
        let n = 2 + rng.below(2) as usize;
        let m = 1 + rng.below(7) as usize;
        let cs: Vec<Constraint> = (0..m).map(|_| random_linear(&mut rng, n)).collect();
        let (mut sx, vars, results) = resolve(&cs, n);
        let mut kept: Vec<&Constraint> = Vec::new();
        for (i, r) in results.iter().enumerate() {
            let mut with = kept.clone();
            with.push(&cs[i]);
            let feasible = fourier_motzkin(&with, n, &[], 0).is_some();
            match r {
                Ok(()) => {
                    assert!(feasible, "kept {i} of {cs:?}");
                    kept.push(&cs[i]);
                    kept_total += 1;
                }
                Err(tags) => {
                    assert!(!feasible, "discarded {i} of {cs:?}");
                    let subset: Vec<&Constraint> = tags.iter().map(|&t| &cs[t]).collect();
                    assert!(fourier_motzkin(&subset, n, &[], 0).is_none(), "the explanation {tags:?} has a solution");
                    discarded_total += 1;
                }
            }
        }
        // the optimum of a few objectives over what was kept, one after the other on one tableau
        for _ in 0..3 {
            let objective: Vec<i64> = (0..n).map(|_| rng.int(-2, 2)).collect();
            let form = LinExpr::from_terms(objective.iter().enumerate().map(|(i, &c)| (vars[i], int(c))), int(0));
            let expected = fm_minimum(&kept, n, &objective).expect("what was kept has a solution");
            match (sx.minimize(&form), expected) {
                (Ok(v), Some(w)) => {
                    assert_eq!(v, w, "{objective:?} over {kept:?}");
                    assert_eq!(sx.eval(&form), v);
                    for c in &kept {
                        assert!(c.holds(|j| sx.value(vars[j]).clone()));
                    }
                    optima += 1;
                }
                (Err(Failure::Unbounded), None) => unbounded += 1,
                (got, want) => panic!("{got:?} against {want:?} for {objective:?} over {kept:?}"),
            }
            sx.assert_invariants();
        }
    }
    assert!(kept_total > 1000 && discarded_total > 300, "{kept_total} kept, {discarded_total} discarded");
    assert!(optima > 800 && unbounded > 200, "{optima} optima, {unbounded} unbounded");
}

#[test]
fn undoing_and_asserting_again_equals_building_afresh() {
    let mut rng = Rng(0x5eed_0004);
    for _ in 0..500 {
        let n = 2 + rng.below(4) as usize;
        let m = 2 + rng.below(10) as usize;
        let cs: Vec<Constraint> = (0..m)
            .map(|_| match rng.below(3) {
                0 => random_difference(&mut rng, n),
                1 => random_linear(&mut rng, n),
                _ => random_equality(&mut rng, n.max(3)),
            })
            .map(|c| Constraint { terms: c.terms.into_iter().filter(|&(i, _)| i < n).collect(), ..c })
            .filter(|c| !c.terms.is_empty())
            .collect();
        // one tableau, with junk asserted, checked and undone between the constraints
        let (mut sx, vars) = simplex(n);
        let mut kept: Vec<usize> = Vec::new();
        for (i, c) in cs.iter().enumerate() {
            if rng.below(2) == 0 {
                let cp = sx.checkpoint();
                for _ in 0..1 + rng.below(3) {
                    let junk = random_linear(&mut rng, n);
                    if sx.assert(&junk.expr(&vars), junk.rel, &junk.c, 1000).is_err() {
                        break;
                    }
                }
                let _ = sx.check();
                if rng.below(2) == 0 {
                    let _ = sx.minimize(&LinExpr::term(vars[0], int(rng.int(-1, 1))));
                }
                sx.backtrack(cp);
            }
            let cp = sx.checkpoint();
            match sx.assert(&c.expr(&vars), c.rel, &c.c, i).and_then(|()| sx.check()) {
                Ok(()) => kept.push(i),
                Err(Failure::Conflict(_)) => sx.backtrack(cp),
                Err(e) => panic!("{e:?}"),
            }
            sx.assert_invariants();
        }
        assert_eq!(sx.check(), Ok(()));
        // the same constraints on a fresh tableau, without the junk, keep the same ones
        let (fresh_sx, fresh_vars, results) = resolve(&cs, n);
        let fresh_kept: Vec<usize> = (0..cs.len()).filter(|&i| results[i].is_ok()).collect();
        assert_eq!(kept, fresh_kept, "{cs:?}");
        // and give the same optima and the same answers to more constraints
        let mut fresh = fresh_sx;
        for _ in 0..3 {
            let objective: Vec<(usize, i64)> = (0..n).map(|i| (i, rng.int(-2, 2))).collect();
            let form = |vars: &[Var]| LinExpr::from_terms(objective.iter().map(|&(i, c)| (vars[i], int(c))), int(0));
            assert_eq!(sx.minimize(&form(&vars)), fresh.minimize(&form(&fresh_vars)), "{cs:?}");
            let more = random_linear(&mut rng, n);
            let (a, b) = (sx.checkpoint(), fresh.checkpoint());
            let ra = sx.assert(&more.expr(&vars), more.rel, &more.c, 999).and_then(|()| sx.check()).is_ok();
            let rb = fresh.assert(&more.expr(&fresh_vars), more.rel, &more.c, 999).and_then(|()| fresh.check()).is_ok();
            assert_eq!(ra, rb, "{more:?} after {cs:?}");
            sx.backtrack(a);
            fresh.backtrack(b);
            assert_eq!(sx.check(), Ok(()));
            assert_eq!(fresh.check(), Ok(()));
        }
    }
}

#[test]
fn the_same_input_gives_the_same_tableau() {
    let mut rng = Rng(0x5eed_0005);
    let cs: Vec<Constraint> = (0..30).map(|_| random_linear(&mut rng, 5)).collect();
    let (a, va, ra) = resolve(&cs, 5);
    let (b, vb, rb) = resolve(&cs, 5);
    assert_eq!(ra, rb);
    assert_eq!(a.pivots(), b.pivots());
    for (x, y) in va.iter().zip(&vb) {
        assert_eq!(a.value(*x), b.value(*y));
    }
}

#[test]
fn blands_rule_throughout_or_from_midway_decides_alike() {
    // what a check decides does not depend on how it pivots: the same constraints are kept, and
    // every explanation has no solution, whether Bland's rule chooses from the start, after one
    // or a few repeats, or only as the fallback
    let mut rng = Rng(0x5eed_0006);
    for round in 0..600 {
        let n = 2 + rng.below(2) as usize;
        let m = 1 + rng.below(10) as usize;
        let difference = round % 2 == 0;
        let cs: Vec<Constraint> = (0..m)
            .map(|_| if difference { random_difference(&mut rng, n) } else { random_linear(&mut rng, n) })
            .collect();
        let (_, _, expected) = resolve(&cs, n);
        let kept = |results: &Outcomes| results.iter().map(Result::is_ok).collect::<Vec<bool>>();
        for bland_after in [0, 1, 3] {
            let (_, _, results) = resolve_with(&cs, n, bland_after);
            assert_eq!(kept(&results), kept(&expected), "Bland after {bland_after}: {cs:?}");
            for tags in results.iter().filter_map(|r| r.as_ref().err()) {
                let subset: Vec<&Constraint> = tags.iter().map(|&t| &cs[t]).collect();
                let feasible =
                    if difference { bellman_ford(&subset, n) } else { fourier_motzkin(&subset, n, &[], 0).is_some() };
                assert!(!feasible, "Bland after {bland_after}: the explanation {tags:?} has a solution: {cs:?}");
            }
        }
    }
}

// ------------------------------------------------------------------------------ at scale

/// One axis of a level as the coordinate LP of stage S3 builds it (plan 5.1, 5.6): per object an
/// origin and a pin at a fixed offset; facts taken in one at a time as the resolution of 8.5 takes
/// them (ORDER, ALIGN, DISTANCE, DISTRIBUTE, fixed coordinates), each that conflicts undone; then
/// the wire-length rows, per connected pair of pins a new variable `t` with `t ≥ pᵢ − pⱼ` and
/// `t ≥ pⱼ − pᵢ`, and one check. The `t`s are made after the rows of the facts, as the LP adds
/// them, or with `t_first` before them. Gives the tableau, the `t`s, and the pivots of the last
/// check.
fn level_axis(seed: u64, objects: usize, t_first: bool) -> (Simplex<usize>, Vec<Var>, u64) {
    let mut rng = Rng(seed);
    let mut sx: Simplex<usize> = Simplex::new();
    let mut ts: Vec<Var> = if t_first { (0..objects).map(|_| sx.new_var()).collect() } else { Vec::new() };
    let origins: Vec<Var> = (0..objects).map(|_| sx.new_var()).collect();
    let pins: Vec<Var> = (0..objects).map(|_| sx.new_var()).collect();
    let mut tag = 0;
    let mut take = |sx: &mut Simplex<usize>, e: LinExpr, rel: Rel, c: Q| {
        tag += 1;
        let cp = sx.checkpoint();
        match sx.assert(&e, rel, &c, tag).and_then(|()| sx.check()) {
            Ok(()) => {}
            Err(Failure::Conflict(_)) => sx.backtrack(cp),
            Err(e) => panic!("{e:?}"),
        }
    };
    for k in 0..objects {
        take(&mut sx, LinExpr::diff(pins[k], origins[k]), Rel::Eq, int(rng.int(-2, 2)));
    }
    let n = objects as u64;
    for _ in 0..3 * objects {
        let a = rng.below(n) as usize;
        let b = (a + 1 + rng.below(n - 1) as usize) % objects;
        let (a, b) = (a.min(b), a.max(b));
        match rng.below(6) {
            0 | 1 => take(&mut sx, LinExpr::diff(origins[b], origins[a]), Rel::Ge, int(1)),
            2 => take(&mut sx, LinExpr::diff(pins[b], pins[a]), Rel::Eq, int(0)),
            3 => take(&mut sx, LinExpr::diff(origins[b], origins[a]), Rel::Ge, int(rng.int(1, 10))),
            4 if b + 1 < objects => {
                let c = b + 1 + rng.below((objects - b - 1) as u64) as usize;
                let e =
                    LinExpr::from_terms([(origins[a], int(1)), (origins[b], int(-2)), (origins[c], int(1))], int(0));
                take(&mut sx, e, Rel::Eq, int(0))
            }
            _ => take(&mut sx, LinExpr::var(origins[a]), Rel::Eq, int(4 * a as i64 + rng.int(0, 20))),
        }
    }
    let before = sx.pivots();
    for k in 0..objects {
        let i = rng.below(n) as usize;
        let j = (i + 1 + rng.below(n - 1) as usize) % objects;
        let t = if t_first { ts[k] } else { sx.new_var() };
        ts.push(t);
        tag += 1;
        let up = LinExpr::from_terms([(t, int(1)), (pins[i], int(-1)), (pins[j], int(1))], int(0));
        let down = LinExpr::from_terms([(t, int(1)), (pins[i], int(1)), (pins[j], int(-1))], int(0));
        sx.assert(&up, Rel::Ge, &int(0), tag).unwrap();
        sx.assert(&down, Rel::Ge, &int(0), tag).unwrap();
    }
    ts.truncate(objects);
    assert_eq!(sx.check(), Ok(()));
    let pivots = sx.pivots() - before;
    (sx, ts, pivots)
}

#[test]
fn wire_length_rows_made_after_the_facts_take_about_one_pivot_each() {
    // Bland's rule alone would move an old variable of each row, the smallest, and the pivots
    // would spread: about 1000 to 1350 pivots for these 150 rows, and minutes of rationals
    for seed in [0x7e57_0002, 0x7e57_0003] {
        let (mut late, ts, pivots) = level_axis(seed, 150, false);
        let wires = ts.len() as u64;
        assert!(pivots <= wires + wires / 10, "seed {seed:x}: {pivots} pivots for {wires} wires");
        assert!(late.satisfied());
        late.assert_invariants();
        // made before the facts or after them, the `t`s give the same optimum
        let (mut early, early_ts, early_pivots) = level_axis(seed, 150, true);
        assert!(early_pivots <= wires + wires / 10, "seed {seed:x}: {early_pivots} pivots for {wires} wires");
        let sum = |ts: &[Var]| LinExpr::from_terms(ts.iter().map(|&t| (t, int(1))), int(0));
        let optimum = late.minimize(&sum(&ts));
        assert!(optimum.is_ok(), "{optimum:?}");
        assert_eq!(optimum, early.minimize(&sum(&early_ts)));
    }
}

#[test]
fn keeping_an_optimum_by_its_variables_equals_bounding_the_objective() {
    // two layers on one tableau: the second optimum is the same whether the first is kept by
    // fixing the variables its row holds at a bound, or by a bound on the objective
    let mut rng = Rng(0x5eed_0004);
    let mut layered = 0;
    for _ in 0..600 {
        let n = 2 + rng.below(3) as usize;
        let m = 2 + rng.below(6) as usize;
        let cs: Vec<Constraint> = (0..m).map(|_| random_linear(&mut rng, n)).collect();
        let objectives: [Vec<i64>; 2] =
            [(0..n).map(|_| rng.int(-2, 2)).collect(), (0..n).map(|_| rng.int(-2, 2)).collect()];
        let form = |vars: &[Var], o: &[i64]| {
            LinExpr::from_terms(o.iter().enumerate().map(|(i, &c)| (vars[i], int(c))), int(0))
        };
        let (mut a, va, results) = resolve(&cs, n);
        let (mut b, vb, _) = resolve(&cs, n);
        let kept: Vec<&Constraint> = cs.iter().zip(&results).filter(|(_, r)| r.is_ok()).map(|(c, _)| c).collect();
        let (f1a, f1b) = (form(&va, &objectives[0]), form(&vb, &objectives[0]));
        let (Ok(o1), Ok(o1b)) = (a.minimize(&f1a), b.minimize(&f1b)) else { continue };
        assert_eq!(o1, o1b);
        a.keep_optimum(&f1a, 100).unwrap();
        b.assert(&f1b, Rel::Le, &o1b, 100).unwrap();
        let (f2a, f2b) = (form(&va, &objectives[1]), form(&vb, &objectives[1]));
        match (a.minimize(&f2a), b.minimize(&f2b)) {
            (Ok(x), Ok(y)) => {
                assert_eq!(x, y, "{objectives:?} over {kept:?}");
                // and the first optimum still holds, with every constraint kept
                assert_eq!(a.eval(&f1a), o1);
                for c in &kept {
                    assert!(c.holds(|j| a.value(va[j]).clone()));
                }
                layered += 1;
            }
            (Err(Failure::Unbounded), Err(Failure::Unbounded)) => {}
            (x, y) => panic!("{x:?} against {y:?} for {objectives:?} over {kept:?}"),
        }
        a.assert_invariants();
    }
    // the others are empty or unbounded at the first layer
    assert!(layered > 150, "{layered}");
}
