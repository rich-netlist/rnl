//! An incremental bounded simplex over exact rationals, after Dutertre and de Moura, "A Fast
//! Linear-Arithmetic Solver for DPLL(T)" (CAV 2006).
//!
//! The resolution of 8.5 takes the facts in one at a time and asks, after each, whether the facts
//! kept so far and the new one still have a solution together; a fact that does not fit is
//! discarded, with the facts it conflicts with. The coordinates are then the optimum of a series
//! of LPs over what was kept. Both work on one tableau:
//!
//! - **Variables**: the structural ones a caller makes ([`Simplex::new_var`]), such as an origin
//!   coordinate on one axis, and one slack variable per **row**, which names a linear form,
//!   `s = Σ aᵢ xᵢ` ([`Simplex::add_row`]). Rows are only ever added; the same form gives the same
//!   row, so a form that several facts bound, such as `x_B − x_A`, is one row.
//! - **Bounds**: each variable has an optional lower and upper bound, each with the tag of what
//!   asserted it: a unit of 8.5, a literal of the search. `x = v` is both bounds of `x`, and
//!   `x_B − x_A ≥ m` the lower bound of the row `x_B − x_A` ([`Simplex::assert`]). A bound no
//!   stronger than the one in place changes nothing, so the tag in place is the earlier one: the
//!   explanations lean towards what came first.
//! - **Check** ([`Simplex::check`]): brings every variable within its bounds by pivoting. It fixes
//!   the smallest basic variable out of its bounds, and moves the variable of its row whose pivot
//!   costs least: one with no bound in the way first, then the one in the fewest rows. Rows added
//!   after the rest, such as `t ≥ |xᵢ − xⱼ|` for the wire length of the coordinate LP, are then
//!   fixed by one pivot each on their own `t`, where the smallest variable would be an old one
//!   and each pivot would spread through the tableau. Once variables have left the basis again
//!   [`BLAND_AFTER`] times within one check, it takes Bland's rule (always the smallest variable
//!   that qualifies), which terminates. Either way the same input gives the same pivots. When a
//!   row cannot be brought within its bounds, because every variable of it that could help is at
//!   a bound, the bounds that hold it are the **explanation**: those bounds alone have no
//!   solution, a Farkas certificate read off the row.
//! - **Undo** ([`Simplex::checkpoint`], [`Simplex::backtrack`]): bounds are undone in the reverse
//!   order, and the rows stay, bounding nothing. Undoing only loosens bounds, so the assignment
//!   stays as valid as the tableau needs: no pivot is undone.
//! - **Optimum** ([`Simplex::minimize`]): the primal simplex from the current solution over the
//!   same tableau, with Bland's rule, for the layers of the lexicographic LP of the coordinates.
//!   The objective is a row like any form, so its reduced costs are read off that row, which the
//!   pivots keep up to date.
//!
//! Every number is a [`Q`]: no decision compares floating point, and the result is the same on
//! every platform. The pivots are counted, and with a limit ([`Simplex::set_pivot_limit`])
//! [`Simplex::check`] and [`Simplex::minimize`] stop with [`Failure::Budget`] instead of going on,
//! the caps of the solver's own work (plan 5.9). The resolution of 8.5 is bounded by time instead
//! (the draft 0.3, 8.5): [`Simplex::check_until`] asks the caller every [`PIVOTS_PER_READ`] pivots
//! whether its time is out, and stops with [`Failure::Time`] when it is.

// Used by `resolve`; what only the coordinate LP uses (minimisation) waits for stage S3.
#![cfg_attr(not(test), allow(dead_code))]

use std::collections::BTreeMap;

use num_traits::{One, Signed, Zero};

use crate::rational::Q;

/// A variable of the tableau, by its index: structural and slack variables share the numbering,
/// which is also the order of Bland's rule and of the ties of [`Simplex::check`].
pub(crate) type Var = usize;

/// A linear form with a constant, `Σ aᵢ xᵢ + c`: the terms sorted by variable, none with a zero
/// coefficient.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct LinExpr {
    terms: Vec<(Var, Q)>,
    constant: Q,
}

impl LinExpr {
    /// The constant `c`.
    pub(crate) fn constant(c: Q) -> LinExpr {
        LinExpr { terms: Vec::new(), constant: c }
    }

    /// The variable `v`.
    pub(crate) fn var(v: Var) -> LinExpr {
        LinExpr::term(v, Q::one())
    }

    /// `a v`.
    pub(crate) fn term(v: Var, a: Q) -> LinExpr {
        let mut e = LinExpr::default();
        e.add_term(v, &a);
        e
    }

    /// `to − from`, the difference of two variables.
    pub(crate) fn diff(to: Var, from: Var) -> LinExpr {
        let mut e = LinExpr::var(to);
        e.add_term(from, &-Q::one());
        e
    }

    /// The sum of the terms, in any order and with repeats, and a constant.
    pub(crate) fn from_terms(terms: impl IntoIterator<Item = (Var, Q)>, constant: Q) -> LinExpr {
        let mut e = LinExpr::constant(constant);
        for (v, a) in terms {
            e.add_term(v, &a);
        }
        e
    }

    /// Adds `a v`.
    pub(crate) fn add_term(&mut self, v: Var, a: &Q) {
        axpy(&mut self.terms, &[(v, Q::one())], a);
    }

    /// Adds `factor` times `other`, its constant included.
    pub(crate) fn add_scaled(&mut self, other: &LinExpr, factor: &Q) {
        axpy(&mut self.terms, &other.terms, factor);
        self.constant += &other.constant * factor;
    }

    /// Adds a constant.
    pub(crate) fn add_constant(&mut self, c: &Q) {
        self.constant += c;
    }

    /// The form times `factor`.
    pub(crate) fn scaled(&self, factor: &Q) -> LinExpr {
        let mut e = LinExpr::default();
        e.add_scaled(self, factor);
        e
    }

    /// The terms, sorted by variable, none zero.
    pub(crate) fn terms(&self) -> &[(Var, Q)] {
        &self.terms
    }

    /// The constant.
    pub(crate) fn constant_part(&self) -> &Q {
        &self.constant
    }

    /// Whether it has no variable.
    pub(crate) fn is_constant(&self) -> bool {
        self.terms.is_empty()
    }

    /// Its value with each variable given the value `value` gives it.
    pub(crate) fn eval(&self, mut value: impl FnMut(Var) -> Q) -> Q {
        let mut sum = self.constant.clone();
        for (v, a) in &self.terms {
            sum += a * value(*v);
        }
        sum
    }
}

/// How a form compares with a constant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Rel {
    /// `≤`
    Le,
    /// `≥`
    Ge,
    /// `=`
    Eq,
}

/// Why [`Simplex::check`] or [`Simplex::minimize`] gave no result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Failure<T> {
    /// The bounds with these tags have no solution together: sorted, without repeats. The set
    /// is valid but not always minimal; the resolution of 8.5 makes it minimal (plan 5.3).
    Conflict(Vec<T>),
    /// The objective of [`Simplex::minimize`] has no lower bound.
    Unbounded,
    /// The limit on pivots was reached first ([`Simplex::set_pivot_limit`]).
    Budget,
    /// The caller's time ran out first ([`Simplex::check_until`]).
    Time,
}

#[derive(Clone, Debug)]
struct Bound<T> {
    value: Q,
    tag: T,
}

#[derive(Clone, Debug)]
struct VarState<T> {
    value: Q,
    lower: Option<Bound<T>>,
    upper: Option<Bound<T>>,
    /// The row whose basic variable it is.
    row: Option<usize>,
}

impl<T> VarState<T> {
    fn below(&self) -> bool {
        self.lower.as_ref().is_some_and(|l| self.value < l.value)
    }

    fn above(&self) -> bool {
        self.upper.as_ref().is_some_and(|u| self.value > u.value)
    }

    fn can_increase(&self) -> bool {
        self.upper.as_ref().is_none_or(|u| self.value < u.value)
    }

    fn can_decrease(&self) -> bool {
        self.lower.as_ref().is_none_or(|l| self.value > l.value)
    }
}

/// A row of the tableau: `basic = Σ aⱼ xⱼ` over nonbasic variables, sorted, none zero.
#[derive(Clone, Debug)]
struct Row {
    basic: Var,
    terms: Vec<(Var, Q)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    Lower,
    Upper,
}

/// A bound replaced, to put back on undo.
#[derive(Clone, Debug)]
struct Undo<T> {
    var: Var,
    side: Side,
    old: Option<Bound<T>>,
}

/// The tableau, the bounds with their tags, and the undo stack.
#[derive(Clone, Debug)]
pub(crate) struct Simplex<T> {
    vars: Vec<VarState<T>>,
    rows: Vec<Row>,
    /// The slack variable of each form a row was made for, so one form makes one row.
    forms: BTreeMap<Vec<(Var, Q)>, Var>,
    trail: Vec<Undo<T>>,
    /// How many rows each variable occurs in, the basic variables of the rows not counted: what
    /// a pivot on it costs, which [`Simplex::check`] weighs.
    occurrences: Vec<u32>,
    pivots: u64,
    pivot_limit: Option<u64>,
    /// [`Simplex::check`] chooses by Bland's rule once variables have left the basis again this
    /// many times within one check ([`BLAND_AFTER`]).
    bland_after: u64,
}

/// How many times variables may leave the basis again within one [`Simplex::check`] before it
/// chooses by Bland's rule. Every pivot makes one variable leave the basis, so a check that has
/// not finished after this many pivots more than there are variables chooses by Bland's rule,
/// which terminates.
pub(crate) const BLAND_AFTER: u64 = 1000;

/// How many pivots of one [`Simplex::check_until`] pass between two questions to the caller
/// whether its time is out: a pivot over exact rationals costs far more than reading a clock, so
/// the question costs little, and a check that runs long is stopped within a few pivots.
pub(crate) const PIVOTS_PER_READ: u64 = 16;

impl<T: Clone + Ord> Default for Simplex<T> {
    fn default() -> Self {
        Simplex::new()
    }
}

impl<T: Clone + Ord> Simplex<T> {
    pub(crate) fn new() -> Simplex<T> {
        Simplex {
            vars: Vec::new(),
            rows: Vec::new(),
            forms: BTreeMap::new(),
            trail: Vec::new(),
            occurrences: Vec::new(),
            pivots: 0,
            pivot_limit: None,
            bland_after: BLAND_AFTER,
        }
    }

    /// A new structural variable, with no bound and the value 0.
    pub(crate) fn new_var(&mut self) -> Var {
        self.vars.push(VarState { value: Q::zero(), lower: None, upper: None, row: None });
        self.occurrences.push(0);
        self.vars.len() - 1
    }

    /// How many variables there are, structural and slack.
    pub(crate) fn var_count(&self) -> usize {
        self.vars.len()
    }

    /// How many rows there are.
    pub(crate) fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// How many pivots have been made, a move of a nonbasic variable from one bound to the other
    /// in [`Simplex::minimize`] counting as one.
    pub(crate) fn pivots(&self) -> u64 {
        self.pivots
    }

    /// Stop [`Simplex::check`] and [`Simplex::minimize`] once the count of pivots reaches
    /// `limit`; none for no limit.
    pub(crate) fn set_pivot_limit(&mut self, limit: Option<u64>) {
        self.pivot_limit = limit;
    }

    /// Let [`Simplex::check`] choose by Bland's rule once variables have left the basis again
    /// `repeats` times within one check; 0 for Bland's rule throughout. For tests.
    #[cfg(test)]
    pub(crate) fn set_bland_after(&mut self, repeats: u64) {
        self.bland_after = repeats;
    }

    /// The current value of a variable: after a successful [`Simplex::check`], a solution of
    /// all the bounds.
    pub(crate) fn value(&self, v: Var) -> &Q {
        &self.vars[v].value
    }

    /// The value of a form under the current values.
    pub(crate) fn eval(&self, e: &LinExpr) -> Q {
        e.eval(|v| self.vars[v].value.clone())
    }

    /// The lower bound of a variable, with its tag.
    pub(crate) fn lower(&self, v: Var) -> Option<(&Q, &T)> {
        self.vars[v].lower.as_ref().map(|b| (&b.value, &b.tag))
    }

    /// The upper bound of a variable, with its tag.
    pub(crate) fn upper(&self, v: Var) -> Option<(&Q, &T)> {
        self.vars[v].upper.as_ref().map(|b| (&b.value, &b.tag))
    }

    /// The variable equal to the form's terms, its constant left out: the variable itself for a
    /// single term with coefficient 1, otherwise the slack variable of a row, made the first
    /// time the form is seen.
    pub(crate) fn add_row(&mut self, e: &LinExpr) -> Var {
        let terms = &e.terms;
        if let [(v, a)] = terms.as_slice() {
            if a.is_one() {
                return *v;
            }
        }
        if let Some(&s) = self.forms.get(terms) {
            return s;
        }
        // the row over the nonbasic variables: the basic ones replaced by their rows
        let mut row: Vec<(Var, Q)> = Vec::new();
        for (v, a) in terms {
            match self.vars[*v].row {
                Some(r) => axpy(&mut row, &self.rows[r].terms, a),
                None => axpy(&mut row, &[(*v, Q::one())], a),
            }
        }
        let value = row.iter().fold(Q::zero(), |sum, (v, a)| sum + a * &self.vars[*v].value);
        let s = self.vars.len();
        self.vars.push(VarState { value, lower: None, upper: None, row: Some(self.rows.len()) });
        self.occurrences.push(0);
        count(&mut self.occurrences, &row, true);
        self.rows.push(Row { basic: s, terms: row });
        self.forms.insert(terms.clone(), s);
        s
    }

    /// Asserts `e rel c` with the tag `tag`: a bound on the form's variable. The form is first
    /// divided by its first coefficient, so that forms that differ by a factor share one row.
    /// A form without variables is a constant, and holds or conflicts alone.
    ///
    /// A conflict with the opposite bound in place is found at once and changes nothing; other
    /// conflicts are found by [`Simplex::check`].
    pub(crate) fn assert(&mut self, e: &LinExpr, rel: Rel, c: &Q, tag: T) -> Result<(), Failure<T>> {
        let rhs = c - &e.constant;
        let Some((_, lead)) = e.terms.first() else {
            let holds = match rel {
                Rel::Le => rhs >= Q::zero(),
                Rel::Ge => rhs <= Q::zero(),
                Rel::Eq => rhs.is_zero(),
            };
            return if holds { Ok(()) } else { Err(Failure::Conflict(vec![tag])) };
        };
        let lead = lead.clone();
        let form = LinExpr { terms: e.terms.iter().map(|(v, a)| (*v, a / &lead)).collect(), constant: Q::zero() };
        let bound = rhs / &lead;
        let rel = match rel {
            Rel::Le if lead.is_negative() => Rel::Ge,
            Rel::Ge if lead.is_negative() => Rel::Le,
            r => r,
        };
        let v = self.add_row(&form);
        match rel {
            Rel::Le => self.assert_bounds(v, None, Some(bound), tag),
            Rel::Ge => self.assert_bounds(v, Some(bound), None, tag),
            Rel::Eq => self.assert_bounds(v, Some(bound.clone()), Some(bound), tag),
        }
    }

    /// Asserts `lower ≤ v ≤ upper`, either side optional, with one tag. When either side
    /// conflicts with a bound in place, nothing changes.
    pub(crate) fn assert_bounds(
        &mut self,
        v: Var,
        lower: Option<Q>,
        upper: Option<Q>,
        tag: T,
    ) -> Result<(), Failure<T>> {
        let s = &self.vars[v];
        if let (Some(l), Some(u)) = (&lower, &upper) {
            if l > u {
                return Err(Failure::Conflict(vec![tag]));
            }
        }
        if let (Some(l), Some(u)) = (&lower, &s.upper) {
            if *l > u.value {
                return Err(Failure::Conflict(sorted(vec![u.tag.clone(), tag])));
            }
        }
        if let (Some(u), Some(l)) = (&upper, &s.lower) {
            if *u < l.value {
                return Err(Failure::Conflict(sorted(vec![l.tag.clone(), tag])));
            }
        }
        if let Some(l) = lower {
            if self.vars[v].lower.as_ref().is_none_or(|old| l > old.value) {
                let old = self.vars[v].lower.replace(Bound { value: l.clone(), tag: tag.clone() });
                self.trail.push(Undo { var: v, side: Side::Lower, old });
                if self.vars[v].row.is_none() && self.vars[v].value < l {
                    self.update(v, l);
                }
            }
        }
        if let Some(u) = upper {
            if self.vars[v].upper.as_ref().is_none_or(|old| u < old.value) {
                let old = self.vars[v].upper.replace(Bound { value: u.clone(), tag });
                self.trail.push(Undo { var: v, side: Side::Upper, old });
                if self.vars[v].row.is_none() && self.vars[v].value > u {
                    self.update(v, u);
                }
            }
        }
        Ok(())
    }

    /// A point to come back to with [`Simplex::backtrack`].
    pub(crate) fn checkpoint(&self) -> usize {
        self.trail.len()
    }

    /// Undoes the bounds asserted since `checkpoint`. The rows made since stay.
    pub(crate) fn backtrack(&mut self, checkpoint: usize) {
        while self.trail.len() > checkpoint {
            let Some(u) = self.trail.pop() else { break };
            let s = &mut self.vars[u.var];
            match u.side {
                Side::Lower => s.lower = u.old,
                Side::Upper => s.upper = u.old,
            }
        }
    }

    /// Brings every variable within its bounds, or explains why it cannot (see the module).
    pub(crate) fn check(&mut self) -> Result<(), Failure<T>> {
        self.check_until(&|| false)
    }

    /// [`Simplex::check`], asking `out_of_time` after every [`PIVOTS_PER_READ`] pivots of this
    /// check whether to stop, and stopping with [`Failure::Time`] when it says so: the time limit
    /// of the resolution of 8.5 (the draft 0.3, 8.5). A check stopped leaves a valid tableau, its
    /// variables not all within their bounds; a later check goes on from there.
    pub(crate) fn check_until(&mut self, out_of_time: &dyn Fn() -> bool) -> Result<(), Failure<T>> {
        // the variables that have left the basis in this check, and how many times one left again
        let mut left = vec![false; self.vars.len()];
        let mut repeats: u64 = 0;
        let mut made: u64 = 0;
        loop {
            // the smallest basic variable out of its bounds
            let mut pick: Option<(Var, bool)> = None;
            for row in &self.rows {
                let s = &self.vars[row.basic];
                let below = s.below();
                if (below || s.above()) && pick.is_none_or(|(p, _)| row.basic < p) {
                    pick = Some((row.basic, below));
                }
            }
            let Some((xi, below)) = pick else { return Ok(()) };
            let r = self.vars[xi].row.expect("a basic variable has a row");
            let Some(xj) = self.entering(r, below, repeats >= self.bland_after) else {
                return Err(Failure::Conflict(self.explain(r, below)));
            };
            if self.out_of_budget() {
                return Err(Failure::Budget);
            }
            if made > 0 && made % PIVOTS_PER_READ == 0 && out_of_time() {
                return Err(Failure::Time);
            }
            let s = &self.vars[xi];
            let target = if below { &s.lower } else { &s.upper };
            let target = target.as_ref().expect("the violated bound").value.clone();
            self.pivot_and_update(xi, xj, target);
            made += 1;
            if std::mem::replace(&mut left[xi], true) {
                repeats += 1;
            }
        }
    }

    /// The nonbasic variable of row `r` that [`Simplex::check`] moves to bring the row's basic
    /// variable up to its lower bound (`below`) or down to its upper bound: one that can move the
    /// way that needs. By Bland's rule, the smallest. Otherwise the one whose pivot costs least:
    /// one without a bound on the side it moves towards first, since it cannot overshoot when it
    /// becomes basic, then the one in the fewest rows, since the pivot rewrites each of them, then
    /// the smallest. A variable made after the rest and bounding one row, such as the `t` of
    /// `t ≥ |xᵢ − xⱼ|`, so fixes its row in one pivot. None when no variable can move: the row
    /// explains the conflict.
    fn entering(&self, r: usize, below: bool, bland: bool) -> Option<Var> {
        let mut best: Option<((bool, u32), Var)> = None;
        for (xj, a) in &self.rows[r].terms {
            let s = &self.vars[*xj];
            let (can_move, bounded) = if below == a.is_positive() {
                (s.can_increase(), s.upper.is_some())
            } else {
                (s.can_decrease(), s.lower.is_some())
            };
            if !can_move {
                continue;
            }
            if bland {
                return Some(*xj);
            }
            // the terms are sorted, so of equal keys the first is the smallest variable
            let key = (bounded, self.occurrences[*xj]);
            if best.as_ref().is_none_or(|(k, _)| key < *k) {
                best = Some((key, *xj));
            }
        }
        best.map(|(_, v)| v)
    }

    /// Minimises a form over the bounds, from the current solution: its optimum, the variables
    /// left at an optimal solution. [`Simplex::check`] runs first.
    ///
    /// The form, divided by its first coefficient as [`Simplex::assert`] divides it, is a row,
    /// made the first time and kept as rows are: its row over the nonbasic variables gives the
    /// reduced costs, and the pivots keep it up to date. A layer of the lexicographic LP that
    /// bounds the form by its optimum then bounds that same row.
    pub(crate) fn minimize(&mut self, objective: &LinExpr) -> Result<Q, Failure<T>> {
        self.check()?;
        let Some((_, lead)) = objective.terms.first() else { return Ok(objective.constant.clone()) };
        let lead = lead.clone();
        let form =
            LinExpr { terms: objective.terms.iter().map(|(v, a)| (*v, a / &lead)).collect(), constant: Q::zero() };
        // the objective is `lead · s` and a constant
        let s = self.add_row(&form);
        let flip = lead.is_negative();
        loop {
            // Bland: the smallest variable whose move lowers the objective; its reduced cost is
            // `lead` times its coefficient in the row of `s`, or `lead` for `s` when nonbasic
            let lowers = |v: Var, d: &Q| {
                let x = &self.vars[v];
                let negative = d.is_negative() != flip;
                (negative && x.can_increase()) || (!negative && x.can_decrease())
            };
            let entering = match self.vars[s].row {
                Some(r) => self.rows[r].terms.iter().find(|(v, d)| lowers(*v, d)).map(|(v, d)| (*v, d.is_negative())),
                None => lowers(s, &Q::one()).then_some((s, false)),
            };
            let Some((xk, negative)) = entering else { return Ok(self.eval(objective)) };
            let increase = negative != flip;
            // the ratio test: how far it can move before a variable reaches a bound; of equal
            // steps, the smallest variable (Bland), itself included
            let x = &self.vars[xk];
            let mut step: Option<(Q, Var)> = if increase {
                x.upper.as_ref().map(|u| (&u.value - &x.value, xk))
            } else {
                x.lower.as_ref().map(|l| (&x.value - &l.value, xk))
            };
            for row in &self.rows {
                let Some(a) = coefficient(&row.terms, xk) else { continue };
                let rate = if increase { a.clone() } else { -a };
                let b = &self.vars[row.basic];
                let limit = if rate.is_positive() {
                    b.upper.as_ref().map(|u| (&u.value - &b.value) / &rate)
                } else {
                    b.lower.as_ref().map(|l| (&l.value - &b.value) / &rate)
                };
                if let Some(t) = limit {
                    if step.as_ref().is_none_or(|(best, var)| t < *best || (t == *best && row.basic < *var)) {
                        step = Some((t, row.basic));
                    }
                }
            }
            let Some((theta, xl)) = step else { return Err(Failure::Unbounded) };
            if self.out_of_budget() {
                return Err(Failure::Budget);
            }
            if xl == xk {
                let value = &self.vars[xk].value;
                let target = if increase { value + &theta } else { value - &theta };
                self.update(xk, target);
                self.pivots += 1;
            } else {
                let rate = coefficient(&self.rows[self.vars[xl].row.expect("basic")].terms, xk).expect("in the row");
                let up = rate.is_positive() == increase;
                let b = &self.vars[xl];
                let target = if up { &b.upper } else { &b.lower };
                let target = target.as_ref().expect("the bound reached").value.clone();
                self.pivot_and_update(xl, xk, target);
            }
        }
    }

    /// Keeps the optimum that [`Simplex::minimize`] just found for `objective`, for the next
    /// layer of a lexicographic LP, without adding a row: every nonbasic variable of the
    /// objective's row is fixed at its value, with the tag `tag`. At an optimum those are the
    /// variables with a reduced cost other than 0, each at the bound that blocks it, so the
    /// solutions left are exactly the optimal ones: the objective is its optimum plus the sum of
    /// those variables' moves times their reduced costs, each move a cost. A bound `objective ≤
    /// optimum` would say the same with a row over all its variables, which every later pivot
    /// would have to rewrite.
    pub(crate) fn keep_optimum(&mut self, objective: &LinExpr, tag: T) -> Result<(), Failure<T>> {
        let Some((_, lead)) = objective.terms.first() else { return Ok(()) };
        let lead = lead.clone();
        let form =
            LinExpr { terms: objective.terms.iter().map(|(v, a)| (*v, a / &lead)).collect(), constant: Q::zero() };
        let s = self.add_row(&form);
        let fixed: Vec<Var> = match self.vars[s].row {
            Some(r) => self.rows[r].terms.iter().map(|(v, _)| *v).collect(),
            None => vec![s],
        };
        for v in fixed {
            let value = self.vars[v].value.clone();
            self.assert_bounds(v, Some(value.clone()), Some(value), tag.clone())?;
        }
        Ok(())
    }

    fn out_of_budget(&self) -> bool {
        self.pivot_limit.is_some_and(|limit| self.pivots >= limit)
    }

    /// The tags of the bounds that keep the basic variable of row `r` out of its bounds: its own
    /// violated bound, and the bounds each nonbasic variable of the row is held at.
    fn explain(&self, r: usize, below: bool) -> Vec<T> {
        let row = &self.rows[r];
        let s = &self.vars[row.basic];
        let own = if below { &s.lower } else { &s.upper };
        let mut tags = vec![own.as_ref().expect("the violated bound").tag.clone()];
        for (xj, a) in &row.terms {
            let s = &self.vars[*xj];
            // below: the row is at its largest, each term at the bound that makes it so
            let at_upper = a.is_positive() == below;
            let bound = if at_upper { &s.upper } else { &s.lower };
            tags.push(bound.as_ref().expect("a variable that cannot move is at a bound").tag.clone());
        }
        sorted(tags)
    }

    /// Sets a nonbasic variable to `value`, and the basic variables with it.
    fn update(&mut self, v: Var, value: Q) {
        let delta = &value - &self.vars[v].value;
        for row in &self.rows {
            if let Some(a) = coefficient(&row.terms, v) {
                self.vars[row.basic].value += a * &delta;
            }
        }
        self.vars[v].value = value;
    }

    /// Gives the basic variable `xi` the value `value` by moving the nonbasic `xj`, then
    /// exchanges them: `xj` becomes basic in `xi`'s row.
    fn pivot_and_update(&mut self, xi: Var, xj: Var, value: Q) {
        let r = self.vars[xi].row.expect("basic");
        let a = coefficient(&self.rows[r].terms, xj).expect("in the row").clone();
        let theta = (&value - &self.vars[xi].value) / &a;
        self.vars[xi].value = value;
        self.vars[xj].value += &theta;
        for (k, row) in self.rows.iter().enumerate() {
            if k != r {
                if let Some(c) = coefficient(&row.terms, xj) {
                    self.vars[row.basic].value += c * &theta;
                }
            }
        }
        // xi = a xj + Σ c x  becomes  xj = xi / a − Σ (c / a) x
        let inv = a.recip();
        let mut def: Vec<(Var, Q)> =
            self.rows[r].terms.iter().filter(|(v, _)| *v != xj).map(|(v, c)| (*v, -(c * &inv))).collect();
        let at = def.partition_point(|(v, _)| *v < xi);
        def.insert(at, (xi, inv));
        let occurrences = &mut self.occurrences;
        for (k, row) in self.rows.iter_mut().enumerate() {
            if k == r {
                continue;
            }
            if let Ok(i) = row.terms.binary_search_by_key(&xj, |(v, _)| *v) {
                count(occurrences, &row.terms, false);
                let (_, c) = row.terms.remove(i);
                axpy(&mut row.terms, &def, &c);
                count(occurrences, &row.terms, true);
            }
        }
        count(occurrences, &self.rows[r].terms, false);
        count(occurrences, &def, true);
        self.rows[r] = Row { basic: xj, terms: def };
        self.vars[xi].row = None;
        self.vars[xj].row = Some(r);
        self.pivots += 1;
    }

    /// Checks the invariants: every row holds under the current values, nonbasic variables are
    /// within their bounds, and the slack of every form equals the form. For tests.
    #[cfg(test)]
    pub(crate) fn assert_invariants(&self) {
        for (r, row) in self.rows.iter().enumerate() {
            assert_eq!(self.vars[row.basic].row, Some(r));
            let sum = row.terms.iter().fold(Q::zero(), |sum, (v, a)| sum + a * &self.vars[*v].value);
            assert_eq!(sum, self.vars[row.basic].value, "row {r}");
            for (v, a) in &row.terms {
                assert!(self.vars[*v].row.is_none() && !a.is_zero());
            }
            assert!(row.terms.windows(2).all(|w| w[0].0 < w[1].0));
        }
        for s in &self.vars {
            if s.row.is_none() {
                assert!(!s.below() && !s.above());
            }
        }
        let mut occurrences = vec![0; self.vars.len()];
        for row in &self.rows {
            count(&mut occurrences, &row.terms, true);
        }
        assert_eq!(occurrences, self.occurrences);
        for (form, s) in &self.forms {
            let sum = form.iter().fold(Q::zero(), |sum, (v, a)| sum + a * &self.vars[*v].value);
            assert_eq!(sum, self.vars[*s].value);
        }
    }

    /// Whether every variable is within its bounds. For tests.
    #[cfg(test)]
    pub(crate) fn satisfied(&self) -> bool {
        self.vars.iter().all(|s| !s.below() && !s.above())
    }
}

/// Counts the variables of a row's terms in `occurrences`, or with `add` false uncounts them.
fn count(occurrences: &mut [u32], terms: &[(Var, Q)], add: bool) {
    for (v, _) in terms {
        if add {
            occurrences[*v] += 1;
        } else {
            occurrences[*v] -= 1;
        }
    }
}

/// The coefficient of `v` in sorted terms.
fn coefficient(terms: &[(Var, Q)], v: Var) -> Option<&Q> {
    terms.binary_search_by_key(&v, |(w, _)| *w).ok().map(|i| &terms[i].1)
}

/// `acc += a x` over sorted terms, dropping the terms that cancel.
fn axpy(acc: &mut Vec<(Var, Q)>, x: &[(Var, Q)], a: &Q) {
    if a.is_zero() || x.is_empty() {
        return;
    }
    let old = std::mem::take(acc);
    acc.reserve(old.len() + x.len());
    let mut old = old.into_iter().peekable();
    let mut x = x.iter().peekable();
    loop {
        match (old.peek(), x.peek()) {
            (Some((u, _)), Some((v, _))) if u < v => acc.extend(old.next()),
            (Some((u, _)), Some((v, _))) if u == v => {
                let (u, c) = old.next().expect("peeked");
                let (_, d) = x.next().expect("peeked");
                let sum = c + a * d;
                if !sum.is_zero() {
                    acc.push((u, sum));
                }
            }
            (_, Some(_)) => {
                let (v, d) = x.next().expect("peeked");
                acc.push((*v, a * d));
            }
            (Some(_), None) => acc.extend(old.next()),
            (None, None) => break,
        }
    }
}

fn sorted<T: Ord>(mut tags: Vec<T>) -> Vec<T> {
    tags.sort();
    tags.dedup();
    tags
}

#[cfg(test)]
mod tests;
