//! Deterministic obstacle-free orthogonal routing for `auto` wires (13.1).
//!
//! The input is a whole wire, not a collection of independent routes: endpoint directions apply
//! only at the ends, and a waypoint cannot reverse the incoming leg. The result keeps one path per
//! input segment so every written waypoint retains its identity even if consecutive legs align.

use super::Pt;
use std::cmp::Ordering;

/// The outward direction of a pin in scene coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Direction {
    Right,
    Up,
    Left,
    Down,
}
impl Direction {
    /// A transformed pin direction in degrees counterclockwise from right, or None when absent
    /// or not one of the four pin directions. A point must always be passed as None.
    pub fn from_degrees(degrees: u16) -> Option<Self> {
        match degrees {
            0 => Some(Self::Right),
            90 => Some(Self::Up),
            180 => Some(Self::Left),
            270 => Some(Self::Down),
            _ => None,
        }
    }
    fn vector(self) -> (i8, i8) {
        match self {
            Self::Right => (1, 0),
            Self::Up => (0, 1),
            Self::Left => (-1, 0),
            Self::Down => (0, -1),
        }
    }
}

type Dir = (i8, i8);
// Numerically equal signed zeros are ties, as they are in geometry and cost comparison.
fn cmp_number(a: f64, b: f64) -> Ordering {
    if a == b {
        Ordering::Equal
    } else {
        a.total_cmp(&b)
    }
}
#[derive(Clone, Debug)]
struct Candidate {
    points: Vec<Pt>,
    first: Dir,
    last: Dir,
    bends: usize,
    length: f64,
    deviation: f64,
    horizontal: u8,
    side: f64,
}
impl Candidate {
    fn cmp_key(&self, other: &Self) -> Ordering {
        self.bends
            .cmp(&other.bends)
            .then_with(|| cmp_number(self.length, other.length))
            .then_with(|| cmp_number(self.deviation, other.deviation))
            .then(self.horizontal.cmp(&other.horizontal))
            .then_with(|| cmp_number(self.side, other.side))
    }
}
fn sign(x: f64) -> i8 {
    if x > 0.0 {
        1
    } else if x < 0.0 {
        -1
    } else {
        0
    }
}
fn direction(a: Pt, b: Pt) -> Dir {
    (sign(b.0 - a.0), sign(b.1 - a.1))
}
fn opposite(a: Dir, b: Dir) -> bool {
    a.0 == -b.0 && a.1 == -b.1
}
fn between(v: f64, a: f64, b: f64) -> bool {
    v >= a.min(b) && v <= a.max(b)
}
fn on(p: Pt, a: Pt, b: Pt) -> bool {
    (a.0 == b.0 && p.0 == a.0 && between(p.1, a.1, b.1)) || (a.1 == b.1 && p.1 == a.1 && between(p.0, a.0, b.0))
}
fn overlap(a: Pt, b: Pt, c: Pt, d: Pt) -> bool {
    if a.0 == b.0 && b.0 == c.0 && c.0 == d.0 {
        a.1.min(b.1).max(c.1.min(d.1)) < a.1.max(b.1).min(c.1.max(d.1))
    } else if a.1 == b.1 && b.1 == c.1 && c.1 == d.1 {
        a.0.min(b.0).max(c.0.min(d.0)) < a.0.max(b.0).min(c.0.max(d.0))
    } else {
        false
    }
}
fn valid(pts: &[Pt]) -> bool {
    let segments: Vec<_> = pts.windows(2).map(|p| (p[0], p[1])).collect();
    let mut last: Option<Dir> = None;
    for (i, &(a, b)) in segments.iter().enumerate() {
        let d = direction(a, b);
        if d == (0, 0) || d.0 != 0 && d.1 != 0 || last.is_some_and(|v| v == d || opposite(v, d)) {
            return false;
        }
        if (i > 0 && on(pts[0], a, b)) || (i + 1 < segments.len() && on(*pts.last().unwrap(), a, b)) {
            return false;
        }
        for &(c, e) in segments.iter().take(i.saturating_sub(1)) {
            if overlap(a, b, c, e) {
                return false;
            }
        }
        last = Some(d);
    }
    true
}
fn midpoint(a: f64, b: f64) -> f64 {
    let delta = b - a;
    if delta.is_finite() {
        a + delta / 2.0
    } else {
        a / 2.0 + b / 2.0
    }
}
/// Scaled signed difference, preserving close large values when their difference is finite.
fn scaled_delta(a: f64, b: f64, scale: f64) -> f64 {
    let delta = a - b;
    if delta.is_finite() {
        delta / scale
    } else {
        a / scale - b / scale
    }
}
fn coords(a: f64, b: f64) -> Vec<f64> {
    let mut v = vec![a, b, a + 1.0, a - 1.0, b + 1.0, b - 1.0, midpoint(a, b)];
    v.retain(|v| v.is_finite());
    v.sort_by(f64::total_cmp);
    v.dedup_by(|a, b| *a == *b);
    v
}
fn candidates(a: Pt, da: Option<Direction>, b: Pt, db: Option<Direction>, single: bool, scale: f64) -> Vec<Candidate> {
    let xs = coords(a.0, b.0);
    let ys = coords(a.1, b.1);
    let mid = (midpoint(a.0, b.0), midpoint(a.1, b.1));
    let ccw = da.or(db).map(|d| {
        let v = d.vector();
        (-v.1, v.0)
    });
    let mut all = Vec::new();
    let mut min_bends = None;
    for bends in 0..=4 {
        let start_len = all.len();
        for first_h in [true, false] {
            let mut combos = vec![Vec::<f64>::new()];
            for k in 0..bends {
                // The final bend must share the unchanged axis with B. Enumerating all other
                // values only to discard them afterward multiplies the hot path by up to seven.
                let fixed = [if (k % 2 == 0) == first_h { b.0 } else { b.1 }];
                let vals: &[f64] = if k + 1 == bends {
                    &fixed
                } else if (k % 2 == 0) == first_h {
                    &xs
                } else {
                    &ys
                };
                let mut next = Vec::with_capacity(combos.len() * vals.len());
                for c in &combos {
                    for &v in vals {
                        let mut c = c.clone();
                        c.push(v);
                        next.push(c);
                    }
                }
                combos = next;
            }
            for combo in combos {
                let mut x = a.0;
                let mut y = a.1;
                let mut pts = vec![a];
                for (k, &v) in combo.iter().enumerate() {
                    if (k % 2 == 0) == first_h {
                        x = v
                    } else {
                        y = v
                    };
                    pts.push((x, y));
                }
                let last_h = (bends % 2 == 0) == first_h;
                if (last_h && y != b.1) || (!last_h && x != b.0) {
                    continue;
                }
                pts.push(b);
                if !valid(&pts) {
                    continue;
                }
                let first = direction(pts[0], pts[1]);
                let last = direction(pts[pts.len() - 2], b);
                if da.is_some_and(|d| d.vector() != first) || db.is_some_and(|d| !opposite(d.vector(), last)) {
                    continue;
                }
                let length = pts
                    .windows(2)
                    .map(|v| scaled_delta(v[1].0, v[0].0, scale).abs() + scaled_delta(v[1].1, v[0].1, scale).abs())
                    .sum::<f64>();
                let inner = &pts[1..pts.len() - 1];
                let deviation = inner
                    .iter()
                    .map(|q| scaled_delta(q.0, mid.0, scale).abs().min(scaled_delta(q.1, mid.1, scale).abs()))
                    .sum::<f64>();
                let side = ccw.map_or(0.0, |d| {
                    -inner.iter().map(|q| (q.0 / scale) * f64::from(d.0) + (q.1 / scale) * f64::from(d.1)).sum::<f64>()
                });
                if !length.is_finite() || !deviation.is_finite() || !side.is_finite() {
                    continue;
                }
                all.push(Candidate {
                    points: pts,
                    first,
                    last,
                    bends,
                    length,
                    deviation,
                    horizontal: u8::from(first.1 != 0),
                    side,
                });
            }
        }
        if all.len() > start_len && min_bends.is_none() {
            min_bends = Some(bends);
        }
        if single && min_bends.is_some() || min_bends.is_some_and(|n| bends >= n + 2) {
            break;
        }
    }
    // Only the best path per pair of end directions can improve a global solution.
    let mut best: Vec<Candidate> = Vec::new();
    for c in all {
        if let Some(i) = best.iter().position(|p| p.first == c.first && p.last == c.last) {
            if c.cmp_key(&best[i]) == Ordering::Less {
                best[i] = c;
            }
        } else {
            best.push(c);
        }
    }
    best
}
#[derive(Clone)]
struct State {
    bends: usize,
    length: f64,
    last: Dir,
    key: Candidate,
    path: usize,
}
struct Link {
    points: Vec<Pt>,
    prev: Option<usize>,
}
fn state_cmp(a: &State, b: &State) -> Ordering {
    a.bends.cmp(&b.bends).then_with(|| cmp_number(a.length, b.length)).then_with(|| a.key.cmp_key(&b.key))
}

/// Route a complete wire. `first` and `last` are outward scene directions of the endpoint pins;
/// None denotes an undirected endpoint. Returns one point-list per consecutive input pair, or None
/// for non-finite coordinates, unrepresentable excursions, or no valid path. Consecutive coincident
/// input points are preserved as single-point segments; constraints apply to the first/last nonzero
/// segment. This is geometry only: coincident coordinates never merge handles or nets. With fewer
/// than two points an empty segment list is returned.
pub fn auto_wire(points: &[Pt], first: Option<Direction>, last: Option<Direction>) -> Option<Vec<Vec<Pt>>> {
    if points.iter().any(|p| !p.0.is_finite() || !p.1.is_finite()) {
        return None;
    }
    if points.len() < 2 {
        return Some(Vec::new());
    }
    // One positive scale for the entire wire keeps dynamic-programming path lengths comparable.
    // Leave ordinary coordinates unscaled, so their existing comparison and 54 reference paths
    // are unchanged. The bound also accounts for the accumulated number of segments.
    let max_abs = points.iter().fold(1.0_f64, |m, p| m.max(p.0.abs()).max(p.1.abs()));
    let bound = (points.len() as f64) * 16.0;
    let scale = if max_abs > f64::MAX / bound { max_abs } else { 1.0 };
    let active: Vec<_> = points.windows(2).enumerate().filter(|(_, p)| p[0] != p[1]).map(|(i, _)| i).collect();
    if active.is_empty() {
        return Some(points.windows(2).map(|p| vec![p[0]]).collect());
    }
    let mut states: Vec<State> = Vec::new();
    let mut links: Vec<Link> = Vec::new();
    for (k, &i) in active.iter().enumerate() {
        let cs = candidates(
            points[i],
            if k == 0 { first } else { None },
            points[i + 1],
            if k + 1 == active.len() { last } else { None },
            active.len() == 1,
            scale,
        );
        if cs.is_empty() {
            return None;
        }
        let mut next: Vec<State> = Vec::new();
        if k == 0 {
            for c in cs {
                let path = links.len();
                links.push(Link { points: c.points.clone(), prev: None });
                let s = State { bends: c.bends, length: c.length, last: c.last, path, key: c };
                if let Some(j) = next.iter().position(|x| x.last == s.last) {
                    if state_cmp(&s, &next[j]) == Ordering::Less {
                        next[j] = s;
                    }
                } else {
                    next.push(s);
                }
            }
        } else {
            for old in &states {
                for c in &cs {
                    if opposite(old.last, c.first) {
                        continue;
                    }
                    let path = links.len();
                    links.push(Link { points: c.points.clone(), prev: Some(old.path) });
                    let s = State {
                        bends: old.bends + c.bends + usize::from(old.last != c.first),
                        length: old.length + c.length,
                        last: c.last,
                        key: c.clone(),
                        path,
                    };
                    if !s.length.is_finite() {
                        continue;
                    }
                    if let Some(j) = next.iter().position(|x| x.last == s.last) {
                        if state_cmp(&s, &next[j]) == Ordering::Less {
                            next[j] = s;
                        }
                    } else {
                        next.push(s);
                    }
                }
            }
        }
        states = next;
        if states.is_empty() {
            return None;
        }
    }
    let mut iter = states.into_iter();
    let mut best = iter.next()?;
    for state in iter {
        if state_cmp(&state, &best) == Ordering::Less {
            best = state;
        }
    }
    let mut reverse = Vec::with_capacity(active.len());
    let mut link = Some(best.path);
    while let Some(i) = link {
        reverse.push(links[i].points.clone());
        link = links[i].prev;
    }
    reverse.reverse();
    let mut paths = reverse.into_iter();
    Some(points.windows(2).map(|p| if p[0] == p[1] { vec![p[0]] } else { paths.next().unwrap() }).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reference_54_paths() {
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (4.0, 0.0)], Some(Direction::Right), Some(Direction::Left)).unwrap(),
            vec![vec![(0.0, 0.0), (4.0, 0.0)]],
            "directed_left_1"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (4.0, 3.0)], Some(Direction::Right), Some(Direction::Left)).unwrap(),
            vec![vec![(0.0, 0.0), (2.0, 0.0), (2.0, 3.0), (4.0, 3.0)]],
            "directed_left_2"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (0.0, 3.0)], Some(Direction::Right), Some(Direction::Left)).unwrap(),
            vec![vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.5), (-1.0, 1.5), (-1.0, 3.0), (0.0, 3.0)]],
            "directed_left_3"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (-4.0, 3.0)], Some(Direction::Right), Some(Direction::Left)).unwrap(),
            vec![vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.5), (-5.0, 1.5), (-5.0, 3.0), (-4.0, 3.0)]],
            "directed_left_4"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (-4.0, 0.0)], Some(Direction::Right), Some(Direction::Left)).unwrap(),
            vec![vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (-5.0, 1.0), (-5.0, 0.0), (-4.0, 0.0)]],
            "directed_left_5"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (-4.0, -3.0)], Some(Direction::Right), Some(Direction::Left)).unwrap(),
            vec![vec![(0.0, 0.0), (1.0, 0.0), (1.0, -1.5), (-5.0, -1.5), (-5.0, -3.0), (-4.0, -3.0)]],
            "directed_left_6"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (0.0, -3.0)], Some(Direction::Right), Some(Direction::Left)).unwrap(),
            vec![vec![(0.0, 0.0), (1.0, 0.0), (1.0, -1.5), (-1.0, -1.5), (-1.0, -3.0), (0.0, -3.0)]],
            "directed_left_7"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (4.0, -3.0)], Some(Direction::Right), Some(Direction::Left)).unwrap(),
            vec![vec![(0.0, 0.0), (2.0, 0.0), (2.0, -3.0), (4.0, -3.0)]],
            "directed_left_8"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (4.0, 0.0)], Some(Direction::Right), Some(Direction::Right)).unwrap(),
            vec![vec![(0.0, 0.0), (2.0, 0.0), (2.0, 1.0), (5.0, 1.0), (5.0, 0.0), (4.0, 0.0)]],
            "directed_right_1"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (4.0, 3.0)], Some(Direction::Right), Some(Direction::Right)).unwrap(),
            vec![vec![(0.0, 0.0), (5.0, 0.0), (5.0, 3.0), (4.0, 3.0)]],
            "directed_right_2"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (0.0, 3.0)], Some(Direction::Right), Some(Direction::Right)).unwrap(),
            vec![vec![(0.0, 0.0), (1.0, 0.0), (1.0, 3.0), (0.0, 3.0)]],
            "directed_right_3"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (-4.0, 3.0)], Some(Direction::Right), Some(Direction::Right)).unwrap(),
            vec![vec![(0.0, 0.0), (1.0, 0.0), (1.0, 3.0), (-4.0, 3.0)]],
            "directed_right_4"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (-4.0, 0.0)], Some(Direction::Right), Some(Direction::Right)).unwrap(),
            vec![vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (-2.0, 1.0), (-2.0, 0.0), (-4.0, 0.0)]],
            "directed_right_5"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (-4.0, -3.0)], Some(Direction::Right), Some(Direction::Right)).unwrap(),
            vec![vec![(0.0, 0.0), (1.0, 0.0), (1.0, -3.0), (-4.0, -3.0)]],
            "directed_right_6"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (0.0, -3.0)], Some(Direction::Right), Some(Direction::Right)).unwrap(),
            vec![vec![(0.0, 0.0), (1.0, 0.0), (1.0, -3.0), (0.0, -3.0)]],
            "directed_right_7"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (4.0, -3.0)], Some(Direction::Right), Some(Direction::Right)).unwrap(),
            vec![vec![(0.0, 0.0), (5.0, 0.0), (5.0, -3.0), (4.0, -3.0)]],
            "directed_right_8"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (4.0, 0.0)], Some(Direction::Right), Some(Direction::Up)).unwrap(),
            vec![vec![(0.0, 0.0), (2.0, 0.0), (2.0, 1.0), (4.0, 1.0), (4.0, 0.0)]],
            "directed_up_1"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (4.0, 3.0)], Some(Direction::Right), Some(Direction::Up)).unwrap(),
            vec![vec![(0.0, 0.0), (2.0, 0.0), (2.0, 4.0), (4.0, 4.0), (4.0, 3.0)]],
            "directed_up_2"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (0.0, 3.0)], Some(Direction::Right), Some(Direction::Up)).unwrap(),
            vec![vec![(0.0, 0.0), (1.0, 0.0), (1.0, 4.0), (0.0, 4.0), (0.0, 3.0)]],
            "directed_up_3"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (-4.0, 3.0)], Some(Direction::Right), Some(Direction::Up)).unwrap(),
            vec![vec![(0.0, 0.0), (1.0, 0.0), (1.0, 4.0), (-4.0, 4.0), (-4.0, 3.0)]],
            "directed_up_4"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (-4.0, 0.0)], Some(Direction::Right), Some(Direction::Up)).unwrap(),
            vec![vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (-4.0, 1.0), (-4.0, 0.0)]],
            "directed_up_5"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (-4.0, -3.0)], Some(Direction::Right), Some(Direction::Up)).unwrap(),
            vec![vec![(0.0, 0.0), (1.0, 0.0), (1.0, -1.5), (-4.0, -1.5), (-4.0, -3.0)]],
            "directed_up_6"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (0.0, -3.0)], Some(Direction::Right), Some(Direction::Up)).unwrap(),
            vec![vec![(0.0, 0.0), (1.0, 0.0), (1.0, -1.5), (0.0, -1.5), (0.0, -3.0)]],
            "directed_up_7"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (4.0, -3.0)], Some(Direction::Right), Some(Direction::Up)).unwrap(),
            vec![vec![(0.0, 0.0), (4.0, 0.0), (4.0, -3.0)]],
            "directed_up_8"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (4.0, 0.0)], Some(Direction::Right), Some(Direction::Down)).unwrap(),
            vec![vec![(0.0, 0.0), (2.0, 0.0), (2.0, -1.0), (4.0, -1.0), (4.0, 0.0)]],
            "directed_down_1"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (4.0, 3.0)], Some(Direction::Right), Some(Direction::Down)).unwrap(),
            vec![vec![(0.0, 0.0), (4.0, 0.0), (4.0, 3.0)]],
            "directed_down_2"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (0.0, 3.0)], Some(Direction::Right), Some(Direction::Down)).unwrap(),
            vec![vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.5), (0.0, 1.5), (0.0, 3.0)]],
            "directed_down_3"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (-4.0, 3.0)], Some(Direction::Right), Some(Direction::Down)).unwrap(),
            vec![vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.5), (-4.0, 1.5), (-4.0, 3.0)]],
            "directed_down_4"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (-4.0, 0.0)], Some(Direction::Right), Some(Direction::Down)).unwrap(),
            vec![vec![(0.0, 0.0), (1.0, 0.0), (1.0, -1.0), (-4.0, -1.0), (-4.0, 0.0)]],
            "directed_down_5"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (-4.0, -3.0)], Some(Direction::Right), Some(Direction::Down)).unwrap(),
            vec![vec![(0.0, 0.0), (1.0, 0.0), (1.0, -4.0), (-4.0, -4.0), (-4.0, -3.0)]],
            "directed_down_6"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (0.0, -3.0)], Some(Direction::Right), Some(Direction::Down)).unwrap(),
            vec![vec![(0.0, 0.0), (1.0, 0.0), (1.0, -4.0), (0.0, -4.0), (0.0, -3.0)]],
            "directed_down_7"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (4.0, -3.0)], Some(Direction::Right), Some(Direction::Down)).unwrap(),
            vec![vec![(0.0, 0.0), (2.0, 0.0), (2.0, -4.0), (4.0, -4.0), (4.0, -3.0)]],
            "directed_down_8"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (4.0, 0.0)], Some(Direction::Right), None).unwrap(),
            vec![vec![(0.0, 0.0), (4.0, 0.0)]],
            "directed_free_1"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (4.0, 3.0)], Some(Direction::Right), None).unwrap(),
            vec![vec![(0.0, 0.0), (4.0, 0.0), (4.0, 3.0)]],
            "directed_free_2"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (0.0, 3.0)], Some(Direction::Right), None).unwrap(),
            vec![vec![(0.0, 0.0), (1.0, 0.0), (1.0, 3.0), (0.0, 3.0)]],
            "directed_free_3"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (-4.0, 3.0)], Some(Direction::Right), None).unwrap(),
            vec![vec![(0.0, 0.0), (1.0, 0.0), (1.0, 3.0), (-4.0, 3.0)]],
            "directed_free_4"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (-4.0, 0.0)], Some(Direction::Right), None).unwrap(),
            vec![vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (-4.0, 1.0), (-4.0, 0.0)]],
            "directed_free_5"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (-4.0, -3.0)], Some(Direction::Right), None).unwrap(),
            vec![vec![(0.0, 0.0), (1.0, 0.0), (1.0, -3.0), (-4.0, -3.0)]],
            "directed_free_6"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (0.0, -3.0)], Some(Direction::Right), None).unwrap(),
            vec![vec![(0.0, 0.0), (1.0, 0.0), (1.0, -3.0), (0.0, -3.0)]],
            "directed_free_7"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (4.0, -3.0)], Some(Direction::Right), None).unwrap(),
            vec![vec![(0.0, 0.0), (4.0, 0.0), (4.0, -3.0)]],
            "directed_free_8"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (4.0, 0.0)], None, None).unwrap(),
            vec![vec![(0.0, 0.0), (4.0, 0.0)]],
            "free_free_1"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (4.0, 3.0)], None, None).unwrap(),
            vec![vec![(0.0, 0.0), (4.0, 0.0), (4.0, 3.0)]],
            "free_free_2"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (0.0, 3.0)], None, None).unwrap(),
            vec![vec![(0.0, 0.0), (0.0, 3.0)]],
            "free_free_3"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (-4.0, 3.0)], None, None).unwrap(),
            vec![vec![(0.0, 0.0), (-4.0, 0.0), (-4.0, 3.0)]],
            "free_free_4"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (-4.0, 0.0)], None, None).unwrap(),
            vec![vec![(0.0, 0.0), (-4.0, 0.0)]],
            "free_free_5"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (-4.0, -3.0)], None, None).unwrap(),
            vec![vec![(0.0, 0.0), (-4.0, 0.0), (-4.0, -3.0)]],
            "free_free_6"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (0.0, -3.0)], None, None).unwrap(),
            vec![vec![(0.0, 0.0), (0.0, -3.0)]],
            "free_free_7"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (4.0, -3.0)], None, None).unwrap(),
            vec![vec![(0.0, 0.0), (4.0, 0.0), (4.0, -3.0)]],
            "free_free_8"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (4.0, 4.0), (4.0, 0.0)], None, None).unwrap(),
            vec![vec![(0.0, 0.0), (0.0, 4.0), (4.0, 4.0)], vec![(4.0, 4.0), (4.0, 0.0)]],
            "chain_1"
        );
        assert_eq!(
            auto_wire(&[(4.0, 0.0), (0.0, 0.0), (4.0, 4.0)], None, None).unwrap(),
            vec![vec![(4.0, 0.0), (0.0, 0.0)], vec![(0.0, 0.0), (0.0, 4.0), (4.0, 4.0)]],
            "chain_2"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (3.0, 4.0), (8.0, 1.0)], Some(Direction::Right), Some(Direction::Left)).unwrap(),
            vec![vec![(0.0, 0.0), (1.5, 0.0), (1.5, 4.0), (3.0, 4.0)], vec![(3.0, 4.0), (3.0, 1.0), (8.0, 1.0)]],
            "chain_3"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (4.0, 2.0), (7.0, -2.0)], Some(Direction::Up), None).unwrap(),
            vec![vec![(0.0, 0.0), (0.0, 2.0), (4.0, 2.0)], vec![(4.0, 2.0), (7.0, 2.0), (7.0, -2.0)]],
            "chain_4"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (-4.0, 0.0), (-4.0, 4.0)], Some(Direction::Right), None).unwrap(),
            vec![vec![(0.0, 0.0), (1.0, 0.0), (1.0, -1.0), (-4.0, -1.0), (-4.0, 0.0)], vec![(-4.0, 0.0), (-4.0, 4.0)]],
            "chain_5"
        );
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (3.0, 3.0), (6.0, 0.0), (9.0, 3.0)], Some(Direction::Right), Some(Direction::Left))
                .unwrap(),
            vec![
                vec![(0.0, 0.0), (1.5, 0.0), (1.5, 3.0), (3.0, 3.0)],
                vec![(3.0, 3.0), (3.0, 0.0), (6.0, 0.0)],
                vec![(6.0, 0.0), (6.0, 3.0), (9.0, 3.0)]
            ],
            "chain_6"
        );
    }
    #[test]
    fn coincident_and_nonfinite_inputs_are_safe() {
        assert_eq!(auto_wire(&[(0.0, 0.0), (0.0, 0.0)], Some(Direction::Right), None), Some(vec![vec![(0.0, 0.0)]]));
        assert_eq!(
            auto_wire(&[(0.0, 0.0), (0.0, 0.0), (3.0, 2.0)], Some(Direction::Right), None).unwrap()[0],
            vec![(0.0, 0.0)]
        );
        assert!(auto_wire(&[(f64::NAN, 0.0), (1.0, 0.0)], None, None).is_none());
    }
    #[test]
    fn directed_cases_rotate_with_their_pin_directions() {
        fn turn(p: Pt) -> Pt {
            (-p.1, p.0)
        }
        fn turn_d(d: Direction) -> Direction {
            match d {
                Direction::Right => Direction::Up,
                Direction::Up => Direction::Left,
                Direction::Left => Direction::Down,
                Direction::Down => Direction::Right,
            }
        }
        let positions = [(4., 0.), (4., 3.), (0., 3.), (-4., 3.), (-4., 0.), (-4., -3.), (0., -3.), (4., -3.)];
        for d in [None, Some(Direction::Right), Some(Direction::Up), Some(Direction::Left), Some(Direction::Down)] {
            for end in positions {
                let base = auto_wire(&[(0., 0.), end], Some(Direction::Right), d).unwrap();
                let mut transformed = base.clone();
                let mut finish = end;
                let mut first = Direction::Right;
                let mut last = d;
                for _ in 0..3 {
                    for path in &mut transformed {
                        for p in path {
                            *p = turn(*p);
                        }
                    }
                    finish = turn(finish);
                    first = turn_d(first);
                    last = last.map(turn_d);
                    assert_eq!(auto_wire(&[(0., 0.), finish], Some(first), last).unwrap(), transformed);
                }
            }
        }
    }
    #[test]
    fn repeated_waypoints_keep_segment_slots_and_direction() {
        assert_eq!(auto_wire(&[], None, None), Some(vec![]));
        assert_eq!(auto_wire(&[(1., 2.)], None, None), Some(vec![]));
        let pts = [(0., 0.), (0., 0.), (4., 4.), (4., 4.), (4., 0.), (4., 0.)];
        let result = auto_wire(&pts, None, None).unwrap();
        assert_eq!(result.len(), 5);
        assert_eq!(result[0], vec![(0., 0.)]);
        assert_eq!(result[2], vec![(4., 4.)]);
        assert_eq!(result[4], vec![(4., 0.)]);
        let prev = direction(result[1][result[1].len() - 2], *result[1].last().unwrap());
        let next = direction(result[3][0], result[3][1]);
        assert!(!opposite(prev, next));
    }
    #[test]
    fn directions_constrain_nonzero_legs_across_coincident_waypoints() {
        let p = [(0., 0.), (0., 0.), (4., 3.), (4., 3.)];
        let a = auto_wire(&p, Some(Direction::Right), Some(Direction::Left)).unwrap();
        assert_eq!(a[0], vec![p[0]]);
        assert_eq!(a[2], vec![p[3]]);
        assert_eq!(direction(a[1][0], a[1][1]), Direction::Right.vector());
        assert_eq!(direction(a[1][a[1].len() - 2], *a[1].last().unwrap()), Direction::Right.vector());
        assert_eq!(Direction::from_degrees(90), Some(Direction::Up));
        assert_eq!(Direction::from_degrees(45), None);
    }
    #[test]
    fn fractional_and_long_wires_are_repeatable() {
        let points: Vec<Pt> = (0..300).map(|i| (f64::from(i) * 0.25, if i % 2 == 0 { 0.25 } else { 1.75 })).collect();
        let a = auto_wire(&points, Some(Direction::Right), None).unwrap();
        let b = auto_wire(&points, Some(Direction::Right), None).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.len(), points.len() - 1);
        for (i, segment) in a.iter().enumerate() {
            assert_eq!(segment.first(), Some(&points[i]));
            assert_eq!(segment.last(), Some(&points[i + 1]));
        }
    }
    #[test]
    fn finite_extreme_endpoints_do_not_fail_on_length_overflow() {
        let m = f64::MAX;
        assert_eq!(auto_wire(&[(-m, 0.0), (m, 0.0)], None, None), Some(vec![vec![(-m, 0.0), (m, 0.0)]]));
        assert_eq!(
            auto_wire(&[(-m, 0.0), (0.0, 0.0), (m, 0.0)], None, None),
            Some(vec![vec![(-m, 0.0), (0.0, 0.0)], vec![(0.0, 0.0), (m, 0.0)]])
        );
        let below = f64::from_bits(m.to_bits() - 1);
        assert_eq!(auto_wire(&[(below, 0.0), (m, 0.0)], None, None), Some(vec![vec![(below, 0.0), (m, 0.0)]]));
        assert!(
            auto_wire(&[(m, 0.0), (m, 4.0)], Some(Direction::Right), None).is_none(),
            "an unrepresentable outward stub is not invented"
        );
    }
    #[test]
    fn common_scale_compares_global_lengths_without_overflow() {
        let m = f64::MAX;
        let p = [(-m, 0.0), (0.0, 3.0), (m, 0.0)];
        let a = auto_wire(&p, None, None).unwrap();
        assert_eq!(a.len(), 2);
        for (i, s) in a.iter().enumerate() {
            assert_eq!(s.first(), Some(&p[i]));
            assert_eq!(s.last(), Some(&p[i + 1]));
            assert!(s.iter().all(|q| q.0.is_finite() && q.1.is_finite()));
        }
        let e1 = direction(a[0][a[0].len() - 2], *a[0].last().unwrap());
        let e2 = direction(a[1][0], a[1][1]);
        assert!(!opposite(e1, e2));
        assert_eq!(a, auto_wire(&p, None, None).unwrap());
    }
}
