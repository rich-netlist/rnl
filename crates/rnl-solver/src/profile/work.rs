//! Opt-in work accounting. No arithmetic decisions, deadlines or per-row observer calls.
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

macro_rules! counters {
    ($($key:ident => $name:literal),+ $(,)?) => {
        #[derive(Clone, Copy)]
        pub(crate) enum Key { $($key),+ }
        const NAMES: &[&str] = &[$($name),+];
    };
}

counters! {
    Tableaux => "work_tableaux_created",
    Clones => "work_tableaux_cloned",
    CloneRows => "work_clone_rows",
    CloneTerms => "work_clone_terms",
    NewRows => "work_new_rows",
    ReusedForms => "work_reused_forms",
    UnitForms => "work_unit_forms",
    AxpyCalls => "work_axpy_calls",
    AxpyLeft => "work_axpy_left_terms",
    AxpyRight => "work_axpy_right_terms",
    AxpyResult => "work_axpy_result_terms",
    ReserveRequests => "work_reserve_growth_requests",
    CapacityChanges => "work_capacity_changes",
    RatioCalls => "work_ratio_calls",
    RatioRows => "work_ratio_rows_visited",
    RatioHits => "work_ratio_rows_hit",
    RatioBounded => "work_ratio_bounded",
    UpdateCalls => "work_update_calls",
    UpdateRows => "work_update_rows_visited",
    UpdateHits => "work_update_rows_hit",
    PivotCalls => "work_pivot_calls",
    PivotValueRows => "work_pivot_value_rows_visited",
    PivotValueHits => "work_pivot_value_rows_hit",
    PivotRewriteRows => "work_pivot_rewrite_rows_visited",
    PivotRewriteHits => "work_pivot_rewrite_rows_hit",
    RewriteOldTerms => "work_rewrite_old_terms",
    RewriteNewTerms => "work_rewrite_new_terms",
    CheckPasses => "work_check_passes",
    CheckRows => "work_check_rows_examined",
    CheckViolations => "work_check_violations",
    CheckFeasible => "work_check_feasible_passes",
    ViolationRefreshes => "work_violation_refreshes",
    ViolationBasicChecks => "work_violation_basic_checks",
    ViolationChanges => "work_violation_membership_changes",
    EnteringTerms => "work_entering_terms_visited",
    Resolvers => "work_resolvers_created",
    Rebuilds => "work_rebuild_calls",
    FixedReasserts => "work_fixed_reassertions",
    HeldReasserts => "work_held_reassertions",
    Kept => "work_outcomes_kept",
    Conflict => "work_outcomes_conflict",
    Undecided => "work_outcomes_undecided",
}

#[derive(Clone)]
struct Counts {
    values: [u64; NAMES.len()],
    overflow: bool,
}

impl Default for Counts {
    fn default() -> Self {
        Self { values: [0; NAMES.len()], overflow: false }
    }
}

fn plus(to: &mut u64, value: u64, overflow: &mut bool) {
    let (sum, over) = to.overflowing_add(value);
    *to = if over { u64::MAX } else { sum };
    *overflow |= over;
}

impl Counts {
    fn add(&mut self, key: Key, value: u64) {
        plus(&mut self.values[key as usize], value, &mut self.overflow);
    }

    fn merge(&mut self, other: &Self) {
        for (to, value) in self.values.iter_mut().zip(other.values) {
            plus(to, value, &mut self.overflow);
        }
        self.overflow |= other.overflow;
    }
}

#[derive(Default)]
struct Ledger {
    locals: Vec<Weak<Account>>,
    pending: Counts,
    max_nnz: u64,
    max_rows: u64,
    max_vars: u64,
    call: u64,
}

struct Account {
    ledger: Rc<RefCell<Ledger>>,
    pending: RefCell<Counts>,
    nnz: Cell<u64>,
    rows: Cell<u64>,
    vars: Cell<u64>,
    max_nnz: Cell<u64>,
    max_rows: Cell<u64>,
    max_vars: Cell<u64>,
}

impl Account {
    fn drain_into(&self, ledger: &mut Ledger) {
        ledger.pending.merge(&std::mem::take(&mut *self.pending.borrow_mut()));
        ledger.max_nnz = ledger.max_nnz.max(self.max_nnz.replace(self.nnz.get()));
        ledger.max_rows = ledger.max_rows.max(self.max_rows.replace(self.rows.get()));
        ledger.max_vars = ledger.max_vars.max(self.max_vars.replace(self.vars.get()));
    }
}

impl Drop for Account {
    fn drop(&mut self) {
        // Short-lived clones and failed branches remain in the run's ledger after destruction.
        self.drain_into(&mut self.ledger.borrow_mut());
    }
}

thread_local! {
    static ACTIVE: RefCell<Option<Rc<RefCell<Ledger>>>> = const { RefCell::new(None) };
}

pub(super) struct Scope(Option<Rc<RefCell<Ledger>>>);

impl Scope {
    pub(super) fn new(enabled: bool) -> Self {
        let ledger = enabled.then(|| Rc::new(RefCell::new(Ledger::default())));
        Self(ACTIVE.with(|a| a.replace(ledger)))
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        flush();
        ACTIVE.with(|a| *a.borrow_mut() = self.0.take());
    }
}

/// One actual object's work. Clone copies the gauge, never the source's cumulative work.
#[derive(Default)]
pub(crate) struct Local(Option<Rc<Account>>);

impl std::fmt::Debug for Local {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Diagnostic state must not enter a tableau's semantic/debug comparison.
        f.write_str("WorkAccount")
    }
}

impl Local {
    fn make(ledger: Rc<RefCell<Ledger>>, nnz: u64, rows: u64, vars: u64) -> Self {
        let account = Rc::new(Account {
            ledger: ledger.clone(),
            pending: RefCell::new(Counts::default()),
            nnz: Cell::new(nnz),
            rows: Cell::new(rows),
            vars: Cell::new(vars),
            max_nnz: Cell::new(nnz),
            max_rows: Cell::new(rows),
            max_vars: Cell::new(vars),
        });
        ledger.borrow_mut().locals.push(Rc::downgrade(&account));
        Self(Some(account))
    }

    pub(crate) fn new(key: Key) -> Self {
        let local = ACTIVE.with(|a| a.borrow().clone()).map(|l| Self::make(l, 0, 0, 0)).unwrap_or_default();
        if let Some(mut b) = local.block() {
            b.add(key, 1);
        }
        local
    }

    pub(crate) fn block(&self) -> Option<Block> {
        self.0.as_ref().map(|a| Block { account: a.clone(), counts: Counts::default() })
    }

    pub(crate) fn dimensions(&self, rows: usize, vars: usize) {
        if let Some(a) = &self.0 {
            a.rows.set(rows as u64);
            a.vars.set(vars as u64);
            a.max_rows.set(a.max_rows.get().max(rows as u64));
            a.max_vars.set(a.max_vars.get().max(vars as u64));
        }
    }

    #[cfg(test)]
    pub(crate) fn nnz(&self) -> Option<u64> {
        self.0.as_ref().map(|a| a.nnz.get())
    }
}

impl Clone for Local {
    fn clone(&self) -> Self {
        let Some(a) = &self.0 else { return Self::default() };
        let copy = Self::make(a.ledger.clone(), a.nnz.get(), a.rows.get(), a.vars.get());
        if let Some(mut b) = copy.block() {
            b.add(Key::Clones, 1);
            b.add(Key::CloneRows, a.rows.get());
            b.add(Key::CloneTerms, a.nnz.get());
        }
        copy
    }
}

/// Stack-local aggregation; merging happens once per function, never once per row/term.
pub(crate) struct Block {
    account: Rc<Account>,
    counts: Counts,
}

impl Block {
    pub(crate) fn add(&mut self, key: Key, value: u64) {
        self.counts.add(key, value);
    }

    pub(crate) fn row_change(&mut self, old: usize, new: usize) {
        let current = self.account.nnz.get();
        let Some(rest) = current.checked_sub(old as u64) else {
            self.counts.overflow = true;
            return;
        };
        let mut next = rest;
        plus(&mut next, new as u64, &mut self.counts.overflow);
        self.account.nnz.set(next);
        self.account.max_nnz.set(self.account.max_nnz.get().max(next));
    }
}

impl Drop for Block {
    fn drop(&mut self) {
        self.account.pending.borrow_mut().merge(&self.counts);
    }
}

pub(crate) fn next_call() -> Option<u64> {
    ACTIVE.with(|a| {
        a.borrow().as_ref().map(|l| {
            let mut l = l.borrow_mut();
            let id = l.call;
            if let Some(next) = id.checked_add(1) {
                l.call = next;
            } else {
                l.pending.overflow = true;
            }
            id
        })
    })
}

pub(crate) fn flush() {
    let Some(ledger) = ACTIVE.with(|a| a.borrow().clone()) else { return };
    let (counts, maxima, gauges) = {
        let mut l = ledger.borrow_mut();
        let live: Vec<_> = l.locals.iter().filter_map(Weak::upgrade).collect();
        l.locals.retain(|a| a.strong_count() > 0);
        let mut live_nnz = 0;
        for a in &live {
            a.drain_into(&mut l);
            plus(&mut live_nnz, a.nnz.get(), &mut l.pending.overflow);
        }
        let counts = std::mem::take(&mut l.pending);
        let maxima = vec![
            ("work_max_tableau_nnz", std::mem::take(&mut l.max_nnz)),
            ("work_max_tableau_rows", std::mem::take(&mut l.max_rows)),
            ("work_max_tableau_vars", std::mem::take(&mut l.max_vars)),
            ("work_counter_overflow", u64::from(counts.overflow)),
        ];
        let gauges = vec![("work_live_nnz", live_nnz), ("work_live_accounts", live.len() as u64)];
        // Release temporary strong references after releasing the ledger borrow.
        (counts, maxima, (gauges, live))
    };
    let (gauges, live) = gauges;
    drop(live);
    let mut counters: Vec<_> = NAMES.iter().zip(counts.values).map(|(&name, value)| (name, value)).collect();
    for (name, visits, hits) in [
        ("work_ratio_rows_missed", Key::RatioRows, Key::RatioHits),
        ("work_update_rows_missed", Key::UpdateRows, Key::UpdateHits),
        ("work_pivot_value_rows_missed", Key::PivotValueRows, Key::PivotValueHits),
        ("work_pivot_rewrite_rows_missed", Key::PivotRewriteRows, Key::PivotRewriteHits),
    ] {
        counters.push((name, counts.values[visits as usize].saturating_sub(counts.values[hits as usize])));
    }
    super::emit(super::ProfileEvent::Work { counters, maxima, gauges });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_saturate_and_flag_overflow() {
        let mut counts = Counts::default();
        counts.add(Key::RatioRows, u64::MAX);
        counts.add(Key::RatioRows, 1);
        assert_eq!(counts.values[Key::RatioRows as usize], u64::MAX);
        assert!(counts.overflow);
        let mut merged = Counts::default();
        merged.merge(&counts);
        assert_eq!(merged.values[Key::RatioRows as usize], u64::MAX);
        assert!(merged.overflow);
    }
}
