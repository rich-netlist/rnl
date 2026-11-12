//! Stretch whole groups in ordered bands without cutting through component extents.

use crate::rational::{int, Q};
pub(super) fn cuts(parts: &[[Q; 4]], corners: &[Q], a: usize) -> Vec<Q> {
    let mut extents: Vec<(Q, Q)> = parts.iter().map(|b| (b[a].clone(), b[a + 2].clone())).collect();
    extents.sort();
    let mut out = Vec::new();
    let mut top: Option<Q> = None;
    for (lo, hi) in extents {
        if let Some(t) = top.as_ref().filter(|t| lo > **t) {
            let mut at: Vec<Q> = corners.iter().filter(|c| *c > t && **c < lo).cloned().collect();
            at.push(t.clone());
            at.push(lo.clone());
            at.sort();
            at.dedup();
            let mut best = 0;
            for i in 1..at.len() - 1 {
                if &at[i + 1] - &at[i] > &at[best + 1] - &at[best] {
                    best = i;
                }
            }
            out.push((&at[best] + &at[best + 1]) / int(2));
        }
        if top.as_ref().is_none_or(|t| hi > *t) {
            top = Some(hi);
        }
    }
    out
}
pub(super) fn band(cuts: &[Q], v: &Q) -> usize {
    cuts.iter().filter(|c| *c < v).count()
}
pub(super) fn crossed<'a>(cuts: &[Q], paths: impl Iterator<Item = &'a [[Q; 2]]>, a: usize) -> Vec<usize> {
    let mut out = vec![0; cuts.len()];
    for path in paths {
        for w in path.windows(2) {
            let (i, j) = (band(cuts, &w[0][a]), band(cuts, &w[1][a]));
            for n in out.iter_mut().take(i.max(j)).skip(i.min(j)) {
                *n += 1;
            }
        }
    }
    out
}
#[derive(Clone, Debug, Default)]
pub(super) struct Bands {
    pub total: usize,
    of: Vec<[Vec<usize>; 2]>,
    item: Vec<usize>,
}

impl Bands {
    pub(super) fn new(counts: &[[usize; 2]]) -> Bands {
        let n = counts.len();
        let mut bands = Bands { total: n, of: vec![[Vec::new(), Vec::new()]; n], item: (0..n).collect() };
        for (u, count) in counts.iter().enumerate() {
            for (a, &count) in count.iter().enumerate() {
                if count < 2 {
                    continue;
                }
                bands.of[u][a].push(u);
                for _ in 1..count {
                    bands.of[u][a].push(bands.total);
                    bands.item.push(u);
                    bands.total += 1;
                }
            }
        }
        bands
    }
    pub(super) fn unit(&self, u: usize, a: usize, j: usize) -> usize {
        self.of[u][a].get(j).copied().unwrap_or(u)
    }
    pub(super) fn last(&self, u: usize, a: usize) -> usize {
        self.of[u][a].last().copied().unwrap_or(u)
    }
    pub(super) fn item(&self, unit: usize) -> usize {
        self.item[unit]
    }
    pub(super) fn count(&self, u: usize, a: usize) -> usize {
        self.of[u][a].len().max(1)
    }
    pub(super) fn chains(&self) -> Vec<(usize, usize, usize)> {
        let mut out = Vec::new();
        for of in &self.of {
            for (a, units) in of.iter().enumerate() {
                out.extend(units.windows(2).map(|w| (a, w[0], w[1])));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(x0: i64, y0: i64, x1: i64, y1: i64) -> [Q; 4] {
        [int(x0), int(y0), int(x1), int(y1)]
    }

    #[test]
    fn cuts_fall_between_parts_and_off_the_corners() {
        let parts = [b(0, 0, 4, 2), b(6, 1, 8, 5), b(0, 4, 4, 6), b(0, 10, 4, 12)];
        let corners = [int(7), int(9)];
        let cuts = cuts(&parts, &corners, 1);
        assert_eq!(cuts, vec![int(8)]);
        assert_eq!(band(&cuts, &int(5)), 0);
        assert_eq!(band(&cuts, &int(9)), 1);
        let cuts = [int(3), int(8)];
        let paths: [Vec<[Q; 2]>; 2] =
            [vec![[int(0), int(1)], [int(0), int(9)]], vec![[int(0), int(5)], [int(4), int(5)]]];
        assert_eq!(crossed(&cuts, paths.iter().map(|p| p.as_slice()), 1), vec![1, 1]);
    }

    #[test]
    fn bands_number_after_the_items() {
        let bands = Bands::new(&[[1, 1], [1, 3], [2, 1]]);
        assert_eq!(bands.total, 6);
        assert_eq!((bands.unit(1, 1, 0), bands.unit(1, 1, 1), bands.unit(1, 1, 2)), (1, 3, 4));
        assert_eq!(bands.unit(1, 0, 5), 1);
        assert_eq!((bands.last(1, 1), bands.last(0, 1), bands.last(2, 0)), (4, 0, 5));
        assert_eq!((bands.item(4), bands.item(5), bands.item(2)), (1, 2, 2));
        assert_eq!(bands.chains(), vec![(1, 1, 3), (1, 3, 4), (0, 2, 5)]);
        assert_eq!((bands.count(1, 1), bands.count(1, 0)), (3, 1));
    }
}
