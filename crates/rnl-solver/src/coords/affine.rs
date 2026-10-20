//! Exact placement-only projection. Public-to-the-crate variable IDs always name original
//! source variables; the simplex's variables are private to `Axis`.
use std::collections::BTreeSet;

use num_traits::{One, Zero};

use crate::rational::Q;
use crate::simplex::{Failure, LinExpr, Rel, Simplex, SourceBound, Var};

#[derive(Clone, Debug)]
pub(crate) struct AffineMap {
    /// Original coordinates as expressions over the reduced simplex's structural variables.
    pub(crate) images: Vec<LinExpr>,
    /// Projection P: the retained original coordinates, in reduced-variable order.
    pub(crate) retained: Vec<Var>,
}

impl AffineMap {
    pub(crate) fn identity(n: usize) -> Self {
        Self { images: (0..n).map(LinExpr::var).collect(), retained: (0..n).collect() }
    }

    pub(crate) fn map_expr(&self, e: &LinExpr) -> LinExpr {
        substitute(e, &self.images)
    }

    pub(crate) fn eval_original(&self, v: Var, value: impl FnMut(Var) -> Q) -> Q {
        self.images[v].eval(value)
    }
}

pub(crate) struct Compiled<T> {
    pub(crate) map: AffineMap,
    pub(crate) bounds: Vec<SourceBound<T>>,
    /// E_elim in original coordinates. All other equalities stay in `bounds` (E_rem).
    pub(crate) eliminated: Vec<SourceBound<T>>,
    pub(crate) support: Vec<T>,
}

fn substitute(e: &LinExpr, images: &[LinExpr]) -> LinExpr {
    let mut out = LinExpr::constant(e.constant_part().clone());
    for (v, a) in e.terms() {
        out.add_scaled(&images[*v], a);
    }
    out
}

fn replace(e: &LinExpr, v: Var, by: &LinExpr) -> LinExpr {
    let Some((_, a)) = e.terms().iter().find(|(w, _)| *w == v) else { return e.clone() };
    let mut out = LinExpr::from_terms(e.terms().iter().filter(|(w, _)| *w != v).cloned(), e.constant_part().clone());
    out.add_scaled(by, a);
    out
}

fn equation<T>(b: &SourceBound<T>) -> Option<&Q> {
    match (&b.lower, &b.upper) {
        (Some((lo, _)), Some((hi, _))) if lo == hi => Some(lo),
        _ => None,
    }
}

fn tags<T: Clone>(b: &SourceBound<T>) -> impl Iterator<Item = T> + '_ {
    b.lower.iter().chain(&b.upper).map(|(_, tag)| tag.clone())
}

/// Deterministic sparse elimination. Each pivot removes an exact equation and substitutes one
/// variable only when the total nonzero count of the remaining bounds *and lift map* does not
/// increase. Incidence includes the lift map, so pin aliases cost O(their actual users), not a
/// scan of the historical tableau. No time or product work budget controls representation.
pub(crate) fn compile<T: Clone + Ord>(
    n: usize,
    mut bounds: Vec<SourceBound<T>>,
    priority: &[u8],
) -> Result<Compiled<T>, Failure<T>> {
    bounds.sort_by(|a, b| {
        (a.expr.terms(), a.expr.constant_part(), &a.lower, &a.upper)
            .cmp(&(b.expr.terms(), b.expr.constant_part(), &b.lower, &b.upper))
    });
    let original = bounds.clone();
    let m = bounds.len();
    let mut expressions: Vec<LinExpr> = bounds.iter().map(|b| b.expr.clone()).collect();
    expressions.extend((0..n).map(LinExpr::var));
    let mut users = vec![BTreeSet::new(); n];
    for (i, e) in expressions.iter().enumerate() {
        for (v, _) in e.terms() {
            users[*v].insert(i);
        }
    }
    let mut removed = vec![false; m];
    let mut eliminated = Vec::new();
    let mut support = BTreeSet::new();
    let mut free = vec![true; n];
    loop {
        let mut progress = false;
        // Fixed coordinates and difference aliases first; general rational equations afterwards.
        let mut equations: Vec<usize> = (0..m).filter(|&i| !removed[i] && equation(&bounds[i]).is_some()).collect();
        equations.sort_by_key(|&i| {
            let e = &expressions[i];
            let kind = match e.terms() {
                [_] => 0,
                [(_, a), (_, b)] if a == &-b => 1,
                _ => 2,
            };
            (kind, i)
        });
        for i in equations {
            let e = &expressions[i];
            let rhs = equation(&bounds[i]).expect("an exact equation");
            if e.is_constant() {
                if e.constant_part() != rhs {
                    let mut why = support.clone();
                    why.extend(tags(&bounds[i]));
                    return Err(Failure::Conflict(why.into_iter().collect()));
                }
                continue;
            }
            let mut best: Option<((isize, u8, Var), LinExpr)> = None;
            for (v, a) in e.terms() {
                let mut by = LinExpr::constant((rhs - e.constant_part()) / a);
                for (w, b) in e.terms() {
                    if w != v {
                        by.add_term(*w, &(-b / a));
                    }
                }
                let mut fill = -(e.terms().len() as isize);
                for &j in &users[*v] {
                    if j != i {
                        fill += replace(&expressions[j], *v, &by).terms().len() as isize
                            - expressions[j].terms().len() as isize;
                    }
                }
                let key = (fill, priority.get(*v).copied().unwrap_or(1), *v);
                if fill <= 0 && best.as_ref().is_none_or(|(old, _)| key < *old) {
                    best = Some((key, by));
                }
            }
            let Some(((_, _, v), by)) = best else { continue };
            let affected: Vec<usize> = users[v].iter().copied().collect();
            for j in affected {
                let next = if j == i { LinExpr::constant(rhs.clone()) } else { replace(&expressions[j], v, &by) };
                for (w, _) in expressions[j].terms() {
                    users[*w].remove(&j);
                }
                for (w, _) in next.terms() {
                    users[*w].insert(j);
                }
                expressions[j] = next;
            }
            free[v] = false;
            removed[i] = true;
            eliminated.push(original[i].clone());
            support.extend(tags(&bounds[i]));
            progress = true;
        }
        if !progress {
            break;
        }
    }
    let retained: Vec<Var> = (0..n).filter(|v| free[*v]).collect();
    let mut renumber = vec![LinExpr::default(); n];
    for (z, v) in retained.iter().enumerate() {
        renumber[*v] = LinExpr::var(z);
    }
    let map = AffineMap {
        images: expressions[m..].iter().map(|e| substitute(e, &renumber)).collect(),
        retained,
    };
    let mut remaining = Vec::new();
    for (i, mut b) in bounds.drain(..).enumerate() {
        if removed[i] {
            continue;
        }
        b.expr = substitute(&expressions[i], &renumber);
        if b.expr.is_constant() {
            let value = b.expr.constant_part();
            if b.lower.as_ref().is_some_and(|(lo, _)| value < lo)
                || b.upper.as_ref().is_some_and(|(hi, _)| value > hi)
            {
                let mut why = support.clone();
                why.extend(tags(&b));
                return Err(Failure::Conflict(why.into_iter().collect()));
            }
        } else {
            remaining.push(b);
        }
    }
    Ok(Compiled { map, bounds: remaining, eliminated, support: support.into_iter().collect() })
}

/// Numerical facade. `value/assert/minimize` take original IDs; source forms, reduced IDs and
/// the current basis stay private. Checkpoints always belong to this representation.
pub(crate) struct Axis<T> {
    sx: Simplex<T>,
    map: AffineMap,
    support: Vec<T>,
}

impl<T: Clone + Ord> Axis<T> {
    pub(crate) fn new() -> Self {
        Self { sx: Simplex::new(), map: AffineMap::identity(0), support: Vec::new() }
    }

    pub(crate) fn new_var(&mut self) -> Var {
        self.new_var_at(Q::zero())
    }

    pub(crate) fn new_var_at(&mut self, value: Q) -> Var {
        let original = self.map.images.len();
        let v = self.sx.new_var_at(value);
        self.map.images.push(LinExpr::var(v));
        original
    }

    /// Build a new core-only numeric state. The caller must replay mutable bounds ABOVE its
    /// new base and must provide support in the original fact/literal domain.
    pub(crate) fn from_compiled(
        compiled: Compiled<T>,
        values: &[Q],
        used: u64,
        cap: Option<u64>,
        extra_support: impl IntoIterator<Item = T>,
    ) -> Result<Self, Failure<T>> {
        let mut sx = Simplex::new();
        sx.carry_pivots(used);
        sx.set_pivot_limit(cap);
        for &v in &compiled.map.retained {
            sx.new_var_at(values[v].clone());
        }
        let mut support: BTreeSet<T> = compiled.support.into_iter().collect();
        support.extend(extra_support);
        let mut out = Self { sx, map: compiled.map, support: support.into_iter().collect() };
        for b in compiled.bounds {
            if let Some((v, tag)) = b.lower {
                out.sx.assert(&b.expr, Rel::Ge, &v, tag).map_err(|f| out.explain(f))?;
            }
            if let Some((v, tag)) = b.upper {
                out.sx.assert(&b.expr, Rel::Le, &v, tag).map_err(|f| out.explain(f))?;
            }
        }
        Ok(out)
    }

    fn explain(&self, failure: Failure<T>) -> Failure<T> {
        match failure {
            Failure::Conflict(tags) => {
                let mut all: BTreeSet<T> = tags.into_iter().collect();
                all.extend(self.support.iter().cloned());
                Failure::Conflict(all.into_iter().collect())
            }
            f => f,
        }
    }

    pub(crate) fn assert(&mut self, e: &LinExpr, rel: Rel, c: &Q, tag: T) -> Result<(), Failure<T>> {
        let e = self.map.map_expr(e);
        self.sx.assert(&e, rel, c, tag).map_err(|f| self.explain(f))
    }

    pub(crate) fn value(&self, v: Var) -> Q {
        self.map.eval_original(v, |z| self.sx.value(z).clone())
    }

    pub(crate) fn eval(&self, e: &LinExpr) -> Q {
        self.sx.eval(&self.map.map_expr(e))
    }

    pub(crate) fn check(&mut self) -> Result<(), Failure<T>> {
        self.sx.check().map_err(|f| self.explain(f))
    }

    pub(crate) fn check_until(&mut self, out: &dyn Fn() -> bool) -> Result<(), Failure<T>> {
        self.sx.check_until(out).map_err(|f| self.explain(f))
    }

    pub(crate) fn minimize(&mut self, e: &LinExpr) -> Result<Q, Failure<T>> {
        self.sx.minimize(&self.map.map_expr(e)).map_err(|f| self.explain(f))
    }

    pub(crate) fn keep_optimum(&mut self, e: &LinExpr, tag: T) -> Result<(), Failure<T>> {
        self.sx.keep_optimum(&self.map.map_expr(e), tag).map_err(|f| self.explain(f))
    }

    pub(crate) fn checkpoint(&self) -> usize { self.sx.checkpoint() }
    pub(crate) fn backtrack(&mut self, cp: usize) { self.sx.backtrack(cp); }
    pub(crate) fn pivots(&self) -> u64 { self.sx.pivots() }
    pub(crate) fn carry_pivots(&mut self, used: u64) { self.sx.carry_pivots(used); }
    pub(crate) fn set_pivot_limit(&mut self, limit: Option<u64>) { self.sx.set_pivot_limit(limit); }
    pub(crate) fn pivot_limit(&self) -> Option<u64> { self.sx.pivot_limit() }
    pub(crate) fn profile_tableau(&self) { self.sx.profile_tableau(); }

    /// Compile this axis's current active constraints; no history, old optimum or grid pin is
    /// inherited unless currently bounded. Call BEFORE adding this pass's layers/grid pins.
    pub(crate) fn compact(&self) -> Result<Self, Failure<T>> {
        let _phase = crate::profile::work_span("affine_phase", "consumer_compile");
        let source = self.sx.active_bounds_in_source_variables();
        let n = source.variables.len();
        let before_bounds = source.bounds.len();
        let before_nnz: usize = source.bounds.iter().map(|b| b.expr.terms().len()).sum();
        let compiled = compile(n, source.bounds, &[]).map_err(|f| self.explain(f))?;
        if crate::profile::work_enabled() {
            crate::profile::work_set("affine_source_vars", n as u64);
            crate::profile::work_set("affine_reduced_vars", compiled.map.retained.len() as u64);
            crate::profile::work_set("affine_source_bounds", before_bounds as u64);
            crate::profile::work_set("affine_reduced_bounds", compiled.bounds.len() as u64);
            crate::profile::work_set("affine_source_nnz", before_nnz as u64);
            crate::profile::work_set("affine_reduced_nnz", compiled.bounds.iter().map(|b| b.expr.terms().len() as u64).sum());
            crate::profile::work_set("affine_eliminated_rank", compiled.eliminated.len() as u64);
            crate::profile::work_set("affine_retained_equalities", compiled.bounds.iter().filter(|b| equation(b).is_some()).count() as u64);
        }
        let values: Vec<Q> = source.variables.iter().map(|v| self.sx.value(*v).clone()).collect();
        // Check x=T(Px)+d at the existing source point. Failure is a compiler invariant error,
        // never an ordinary UNSAT outcome or permission to retry a second cold solve.
        for (v, value) in values.iter().enumerate() {
            assert_eq!(*value, compiled.map.eval_original(v, |z| values[compiled.map.retained[z]].clone()), "invalid affine lift");
        }
        let old_images: Vec<LinExpr> = self.map.images.iter().map(|e| substitute(e, &source.forms)).collect();
        let images: Vec<LinExpr> = old_images.iter().map(|e| compiled.map.map_expr(e)).collect();
        let mut out = Self::from_compiled(compiled, &values, self.pivots(), self.pivot_limit(), self.support.clone())?;
        out.map.images = images;
        out.check()?;
        Ok(out)
    }
}

#[cfg(test)]
mod tests;
