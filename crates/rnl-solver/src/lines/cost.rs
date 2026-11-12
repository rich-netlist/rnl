//! Fixed line-search cost weights. The shared budget owns every admitted cost.

use crate::Work;

const UNIT_NS: u64 = 1050;

// Rounded coefficients from the optimized-build calibration in solver-lines.md.
// These constants are part of the standalone solver 0.4.0 replay policy.
const CHOICE_NS: u64 = 12_434;
const RELAXATION_NS: u64 = 102;
const ROUTE_NS: (u64, u64) = (0, 7_907);
const EXPANSION_NS: u64 = 217;
const MERGE_NS: u64 = 378_800;
const MERGE_PIVOT_ROW_NS: u64 = 180;
const MERGE_RELAXATION_NS: u64 = 102;
const LISTING_NS: u64 = 2_908;

/// Admit each orientation trial before constructing its geometry. Listing used
/// to defer this charge until the next search batch, leaving interrupted lists
/// undercharged. Both time and work limits now stop at this same checkpoint.
pub(super) fn listing(objects: usize) -> bool {
    let n = objects.max(1) as u64;
    let cost = n.saturating_mul(weight(LISTING_NS)).div_ceil(1000).saturating_add(n.saturating_mul(4));
    let admitted = crate::budget::permit_weighted(crate::budget::Step::Line, cost);
    if admitted {
        crate::profile::add("listed", n);
    }
    admitted
}

fn weight(ns: u64) -> u64 {
    ns * 1000 / UNIT_NS
}

#[derive(Clone, Copy, Debug)]
pub(super) enum Step {
    Choice,
    Relaxation,
    Route,
    Expansion,
    Merge,
    MergePivotRow,
    MergeRelaxation,
}

#[derive(Debug)]
pub(super) struct Tally {
    counts: [u64; 7],
    weights: [u64; 7],
    charged: u64,
}

impl Tally {
    pub(super) fn new(objects: usize) -> Tally {
        let n = objects as u64;
        let weights = [
            weight(CHOICE_NS),
            weight(RELAXATION_NS),
            weight(ROUTE_NS.0 + ROUTE_NS.1 * n),
            weight(EXPANSION_NS),
            weight(MERGE_NS),
            weight(MERGE_PIVOT_ROW_NS),
            weight(MERGE_RELAXATION_NS),
        ];
        Tally { counts: [0; 7], weights, charged: 0 }
    }

    pub(super) fn charge(&mut self, _work: &mut Work, step: Step, n: u64) {
        if n == 0 {
            return;
        }
        self.counts[step as usize] = self.counts[step as usize].saturating_add(n);
        let total: u64 =
            self.counts.iter().zip(self.weights).fold(0u64, |sum, (c, w)| sum.saturating_add(c.saturating_mul(w)))
                / 1000;
        crate::budget::permit_weighted(crate::budget::Step::Line, total.saturating_sub(self.charged).max(1));
        self.charged = total;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RunLimit, StopReason, StoppedClock};

    #[test]
    fn calibrated_line_work_policy_is_pinned() {
        // One thousand operations exposes the fixed integer coefficient rather
        // than hiding changes behind rounding of a single small operation.
        for (step, expected) in [
            (Step::Choice, 11_841),
            (Step::Relaxation, 97),
            (Step::Route, 120_487),
            (Step::Expansion, 206),
            (Step::Merge, 360_761),
            (Step::MergePivotRow, 171),
            (Step::MergeRelaxation, 97),
        ] {
            let (_, report) = crate::budget::with(&StoppedClock, RunLimit::Work(u64::MAX), None, || {
                Tally::new(16).charge(&mut Work::default(), step, 1000);
            });
            assert_eq!(report.work, expected, "{step:?}");
            assert_eq!(report.reason, StopReason::Complete);
        }
    }

    #[test]
    fn listing_is_paid_atomically_before_the_trial() {
        let (admitted, report) = crate::budget::with(&StoppedClock, RunLimit::Work(324), None, || listing(48));
        assert!(!admitted);
        assert_eq!(report.work, 0);
        let (admitted, report) = crate::budget::with(&StoppedClock, RunLimit::Work(325), None, || {
            let first = listing(48);
            assert!(!listing(1));
            first
        });
        assert!(admitted);
        assert_eq!(report.work, 325);
        assert_eq!(report.reason, StopReason::Work);
    }
}
