//! Increasing-radius row neighbourhoods, ordered by troubled objects and then by estimate.
//! Enumeration is interruptible before each recursive step. The driver restarts at radius
//! one when a better candidate is found.
use super::*;

pub(crate) const BATCH: usize = 128;

const TROUBLE_ORDERED: usize = 1 << 20;

struct Moved {
    rows: Vec<Vec<usize>>,
    changes: usize,
    shifted: Vec<usize>,
}

type Slot = (usize, usize);
type VisitChoices<'a> = dyn FnMut(&[(usize, usize)], bool, bool) -> bool + 'a;

fn each_arrangement(start: &[Vec<usize>], k: usize, visit: &mut dyn FnMut(Moved) -> bool) -> bool {
    let elements: Vec<(usize, usize)> =
        start.iter().enumerate().flat_map(|(r, row)| row.iter().map(move |&e| (e, r))).collect();

    #[allow(clippy::too_many_arguments)]
    fn order(
        rows: &[Vec<usize>],
        at: usize,
        left: &mut Vec<usize>,
        made: &mut Vec<Vec<usize>>,
        changes: usize,
        k: usize,
        shifted: &mut Vec<usize>,
        visit: &mut dyn FnMut(Moved) -> bool,
    ) -> bool {
        if !crate::budget::permit(crate::budget::Step::Structure, 1) {
            return false;
        }
        if left.is_empty() {
            let (next, more) = (at + 1, at + 1 < rows.len());
            if !more {
                let rows = made.iter().filter(|r| !r.is_empty()).cloned().collect();
                return visit(Moved { rows, changes, shifted: shifted.clone() });
            }
            let mut left = rows[next].clone();
            made.push(Vec::new());
            let go = order(rows, next, &mut left, made, changes, k, shifted, visit);
            made.pop();
            return go;
        }
        for j in 0..left.len() {
            let cost = usize::from(j > 0);
            if changes + cost > k {
                break;
            }
            let e = left.remove(j);
            made.last_mut().expect("a row is being made").push(e);
            if cost > 0 {
                shifted.push(e);
            }
            let go = order(rows, at, left, made, changes + cost, k, shifted, visit);
            if cost > 0 {
                shifted.pop();
            }
            made.last_mut().expect("a row is being made").pop();
            left.insert(j, e);
            if !go {
                return false;
            }
        }
        true
    }

    #[allow(clippy::too_many_arguments)]
    fn assign(
        elements: &[(usize, usize)],
        rows: usize,
        at: usize,
        slots: &mut BTreeMap<Slot, Vec<usize>>,
        changes: usize,
        k: usize,
        shifted: &mut Vec<usize>,
        visit: &mut dyn FnMut(Moved) -> bool,
    ) -> bool {
        if !crate::budget::permit(crate::budget::Step::Structure, 1) {
            return false;
        }
        let Some(&(e, home)) = elements.get(at) else {
            let list: Vec<Vec<usize>> = slots.values().cloned().collect();
            let mut made = vec![Vec::new()];
            let mut left = list.first().cloned().unwrap_or_default();
            return order(&list, 0, &mut left, &mut made, changes, k, shifted, visit);
        };
        let mut places: Vec<Slot> = vec![(2 * home + 1, 0)];
        places.extend((0..rows).filter(|&r| r != home).map(|r| (2 * r + 1, 0)));
        for g in 0..=rows {
            let used = slots.keys().filter(|s| s.0 == 2 * g).count();
            places.push((2 * g, used));
        }
        for (i, place) in places.into_iter().enumerate() {
            let cost = usize::from(i > 0);
            if changes + cost > k {
                break;
            }
            slots.entry(place).or_default().push(e);
            if cost > 0 {
                shifted.push(e);
            }
            let go = assign(elements, rows, at + 1, slots, changes + cost, k, shifted, visit);
            if cost > 0 {
                shifted.pop();
            }
            let list = slots.get_mut(&place).expect("just put there");
            list.pop();
            if list.is_empty() {
                slots.remove(&place);
            }
            if !go {
                return false;
            }
        }
        true
    }

    assign(&elements, start.len(), 0, &mut BTreeMap::new(), 0, k, &mut Vec::new(), visit)
}

pub(super) fn varied_values(s: &Structure, class: &FreeClass) -> Vec<Orient> {
    let (root, label) = match class.members.as_slice() {
        [(m, label)] if s.objects[*m].role == Role::Port => (*m, *label),
        _ => return domain(s, class),
    };
    if !s.objects[root].fact_touched {
        return Vec::new();
    }
    let mut values = domain(s, class);
    values.sort_by_key(|v| preference(*v));
    let mut out: Vec<Orient> = Vec::new();
    for way in [(1, 0), (-1, 0)] {
        out.extend(values.iter().copied().find(|v| s.exit(root, 0, v.then(label)) == Some(way)));
    }
    if out.is_empty() {
        values
    } else {
        out
    }
}

struct Pick {
    rows: Vec<Vec<usize>>,
    moved: bool,
    values: Vec<(usize, usize)>,
    even: bool,
    no_columns: bool,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn neighbours(
    problem: &LevelProblem,
    s: &Structure,
    options: &SolverOptions,
    work: &mut Work,
    start: &Candidate,
    k: usize,
    skip: usize,
    trouble: Option<&[u64]>,
) -> (Vec<Candidate>, bool, bool) {
    let free: Vec<(usize, usize)> = s
        .open
        .classes
        .iter()
        .enumerate()
        .map(|(c, class)| (c, varied_values(s, class).len()))
        .chain(s.open.ways.iter().enumerate().map(|(i, w)| (s.open.classes.len() + i, w.count)))
        .filter(|(_, n)| *n > 1)
        .collect();
    fn others(
        free: &[(usize, usize)],
        at: usize,
        left: usize,
        switches: &[bool; 2],
        picked: &mut Vec<(usize, usize)>,
        visit: &mut VisitChoices<'_>,
    ) -> bool {
        if !crate::budget::permit(crate::budget::Step::Structure, 1) {
            return false;
        }
        if at == free.len() {
            return match left {
                0 => visit(picked, false, false),
                1 => (!switches[0] || visit(picked, true, false)) && (!switches[1] || visit(picked, false, true)),
                2 if switches[0] && switches[1] => visit(picked, true, true),
                _ => true,
            };
        }
        if !others(free, at + 1, left, switches, picked, visit) {
            return false;
        }
        if left > 0 {
            for j in 0..free[at].1 - 1 {
                picked.push((free[at].0, j));
                let go = others(free, at + 1, left - 1, switches, picked, visit);
                picked.pop();
                if !go {
                    return false;
                }
            }
        }
        true
    }
    let either = true;
    let switches_of = |rows: &[Vec<usize>]| {
        [s.has(Source::EvenRows) && !evenly(s, rows).is_empty() && (either || !start.even), either || start.columns]
    };
    let wanted: Option<Vec<u64>> = trouble.and_then(|rank| {
        let touched = |shifted: &[usize], values: &[(usize, usize)]| -> u64 {
            let by_rows = shifted.iter().map(|e| rank[*e]).max().unwrap_or(0);
            let by_values = values
                .iter()
                .filter_map(|(c, _)| s.open.classes.get(*c))
                .flat_map(|class| class.members.iter().map(|(m, _)| rank[*m]))
                .max()
                .unwrap_or(0);
            by_rows.max(by_values)
        };
        let mut keys: Vec<(std::cmp::Reverse<u64>, u64)> = Vec::new();
        let mut count = 0u64;
        let mut stood: BTreeSet<(Vec<Vec<usize>>, usize)> = BTreeSet::new();
        let finished = each_arrangement(&start.rows, k, &mut |m: Moved| {
            if !stood.insert((m.rows.clone(), m.changes)) {
                return true;
            }
            let switches = switches_of(&m.rows);
            let shifted = m.shifted;
            others(&free, 0, k - m.changes, &switches, &mut Vec::new(), &mut |values, _, _| {
                keys.push((std::cmp::Reverse(touched(&shifted, values)), count));
                count += 1;
                keys.len() < TROUBLE_ORDERED
            })
        });
        if !finished {
            return None;
        }
        keys.sort_unstable();
        Some(keys.into_iter().map(|(_, n)| n).collect())
    });
    let chosen: Option<Vec<u64>> = wanted.as_ref().map(|w| w.iter().skip(skip).take(BATCH).copied().collect());
    let mut places: BTreeMap<u64, usize> = BTreeMap::new();
    if let Some(chosen) = &chosen {
        places.extend(chosen.iter().enumerate().map(|(i, n)| (*n, i)));
    }
    let mut slotted: Vec<Option<Pick>> = chosen.as_ref().map_or_else(Vec::new, |c| c.iter().map(|_| None).collect());
    let mut seen = 0;
    let mut any = false;
    let mut more = wanted.as_ref().is_some_and(|w| w.len() > skip + BATCH);
    let mut picks: Vec<Pick> = Vec::new();
    let mut stood: BTreeSet<(Vec<Vec<usize>>, usize)> = BTreeSet::new();
    let ordered = chosen.is_some();
    each_arrangement(&start.rows, k, &mut |m: Moved| {
        if !stood.insert((m.rows.clone(), m.changes)) {
            return true;
        }
        let moved = m.changes > 0;
        let switches = switches_of(&m.rows);
        let rows = m.rows;
        others(&free, 0, k - m.changes, &switches, &mut Vec::new(), &mut |values, even, no_columns| {
            any = true;
            seen += 1;
            if ordered {
                if let Some(&i) = places.get(&(seen as u64 - 1)) {
                    slotted[i] = Some(Pick { rows: rows.clone(), moved, values: values.to_vec(), even, no_columns });
                }
                return true;
            }
            if seen <= skip {
                return true;
            }
            if picks.len() == BATCH {
                more = true;
                return false;
            }
            picks.push(Pick { rows: rows.clone(), moved, values: values.to_vec(), even, no_columns });
            true
        })
    });
    picks.extend(slotted.into_iter().flatten());
    let mut search = Search { problem, s, options, made: 0, work };
    let mut scored: Vec<(Vec<Q>, Vec<u32>, Candidate)> = Vec::new();
    for pick in picks {
        if !crate::budget::permit(crate::budget::Step::Candidate, s.elements.len()) {
            more = true;
            break;
        }
        let apart = start
            .apart
            .iter()
            .filter(|(left, right)| {
                pick.rows.iter().all(|row| {
                    let at = |set: &Vec<usize>| row.iter().position(|e| set.contains(e));
                    let last_left = row.iter().rposition(|e| left.contains(e));
                    last_left.is_none() || at(right).is_none() || last_left < at(right)
                })
            })
            .cloned()
            .collect();
        let mut plan = Plan {
            order: pick.rows.concat(),
            values: start.values.clone(),
            items: BTreeMap::new(),
            flipped: BTreeSet::new(),
            ways: start.ways.clone(),
            moved: Some((pick.rows, apart)),
        };
        if pick.moved {
            let classes: Vec<usize> = (0..s.open.classes.len()).collect();
            plan = search.orient(&plan, &classes).0;
        }
        for (c, j) in pick.values {
            if let Some(i) = c.checked_sub(s.open.classes.len()) {
                let now = plan.ways[i];
                let others: Vec<usize> = (0..s.open.ways[i].count).filter(|v| *v != now).collect();
                if let Some(v) = others.get(j) {
                    plan.ways[i] = *v;
                }
                continue;
            }
            let others: Vec<Orient> =
                varied_values(s, &s.open.classes[c]).into_iter().filter(|v| *v != plan.values[c]).collect();
            if let Some(v) = others.get(j) {
                plan.values[c] = *v;
            }
        }
        let Some((e, mut c)) = search.estimate(&plan) else {
            more = true;
            break;
        };
        c.even = start.even != pick.even;
        c.columns = start.columns != pick.no_columns;
        c.origin = "search";
        let code = c.code(s);
        scored.push((e, code, c));
    }
    let refs: Vec<(&[Q], &[u32])> = scored.iter().map(|(e, code, _)| (e.as_slice(), code.as_slice())).collect();
    let ranked = s.active.rank(&refs);
    let made = search.made;
    search.work.candidates += made;
    let mut slots: Vec<Option<Candidate>> = scored.into_iter().map(|(_, _, c)| Some(c)).collect();
    let out = ranked.into_iter().filter_map(|i| slots[i].take()).collect();
    (out, more, any)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::tests::{instance, problem_of};

    #[test]
    fn recursive_row_enumeration_stops_before_materializing_arrangements() {
        let rows = vec![(0..100).collect::<Vec<_>>()];
        let mut visited = 0;
        let (finished, report) = crate::budget::with(&crate::StoppedClock, crate::RunLimit::Work(32), None, || {
            each_arrangement(&rows, 2, &mut |_| {
                visited += 1;
                true
            })
        });
        assert!(!finished);
        assert_eq!(visited, 0);
        assert_eq!(report.work, 32);
        assert_eq!(report.reason, crate::StopReason::Work);
    }

    #[test]
    fn rows_refine_under_work_limit_but_seed_stops_after_initial_candidates() {
        let mut p = problem_of(vec![instance(), instance()], vec![]);
        p.version = "0.4".into();
        let options = SolverOptions::default().with_overrides("layout=rows;objectives=overlaps,crowding,area").unwrap();
        let seed = crate::solve(&p, &SolverOptions { run_limit: Some(crate::RunLimit::Seed), ..options.clone() });
        let refined = crate::solve(&p, &SolverOptions { run_limit: Some(crate::RunLimit::Work(2_000_000)), ..options });
        assert!(!seed.report.notes.iter().any(|n| n.starts_with("row search:")));
        assert!(refined.report.notes.iter().any(|n| n.starts_with("row search:") && !n.contains(": 0 ")));
        assert!(refined.report.work.candidates > seed.report.work.candidates);
    }
}
