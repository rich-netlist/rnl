//! Count mirror pairs away from their common horizontal axis.

use std::collections::BTreeSet;

use crate::coords::Placed;
use crate::rational::{int, Q};
use crate::structure::Structure;
pub(crate) fn off_axis(s: &Structure, placed: &Placed, within: Option<&BTreeSet<usize>>) -> usize {
    let g = &placed.geometry;
    let reads = |k: usize| within.is_none_or(|w| w.contains(&k));
    let centre = |k: usize| -> Option<[Q; 2]> {
        let b = s.box_at(&placed.origins, &g.orient, k)?;
        Some([(&b[0] + &b[2]) / int(2), (&b[1] + &b[3]) / int(2)])
    };
    let pairs: Vec<(Q, bool)> = s
        .pairs
        .iter()
        .filter(|p| reads(p.a) && reads(p.b))
        .filter_map(|p| {
            let (a, b) = (centre(p.a)?, centre(p.b)?);
            Some(((&a[0] + &b[0]) / int(2), a[1] == b[1]))
        })
        .collect();
    if pairs.len() < 2 {
        return 0;
    }
    let mut axis = &pairs[0].0;
    let mut most = 0;
    for (m, _) in &pairs {
        let n = pairs.iter().filter(|(x, _)| x == m).count();
        if n > most {
            (axis, most) = (m, n);
        }
    }
    pairs.iter().filter(|(m, level)| m != axis || !level).count()
}
