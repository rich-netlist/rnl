//! Fixed line-search cost weights. The shared budget owns every admitted cost.

use crate::Work;

const UNIT_NS: u64 = 1050;

const CHOICE_NS: u64 = 24039;
const RELAXATION_NS: u64 = 51;
const ROUTE_NS: (u64, u64) = (24879, 6887);
const EXPANSION_NS: u64 = 119;
const MERGE_NS: u64 = 583090;
const MERGE_PIVOT_ROW_NS: u64 = 139;
const MERGE_RELAXATION_NS: u64 = 51;
const LISTING_NS: u64 = 393;

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
    Listing,
}

#[derive(Debug)]
pub(super) struct Tally {
    counts: [u64; 8],
    weights: [u64; 8],
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
            weight(LISTING_NS),
        ];
        Tally { counts: [0; 8], weights, charged: 0 }
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
