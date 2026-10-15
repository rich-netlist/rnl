//! Orientation classes: a union-find whose edges carry an element of the group of the eight
//! orientations, built from ORIENT (11.2) and ORIENT-RELATION (14.9), with the facts behind each
//! edge for explanations (plan 5.3).
//!
//! An ORIENT-RELATION says that the orientation of `to` is the orientation of `from` followed by
//! the relation, `ori_to = ori_from.then(relation)` (14.9), and that the members of each side
//! share one orientation. So the objects fall into classes, each with one free orientation, the
//! orientation of its root: every node keeps `ori(node) = ori(parent).then(label)`, and the
//! orientation of any member follows from the root's by the labels on its path. An ORIENT fixes
//! the value of a class. A fact that contradicts what the class says already is a conflict, and
//! the facts on the path that closes the cycle explain it: the edges between the two nodes, up to
//! their lowest common ancestor, and the ORIENTs that fix the values involved. Such a set is
//! minimal, since a cycle without any one of its edges is a tree, which any orientations satisfy.
//!
//! There is no path compression, so each edge stays the fact that made it; union by size keeps the
//! paths short. The classes are cloned to undo a tentative fact.

use rnl_runtime::geom::Orient;

/// The index of an orientation in [`Orient::ALL`].
pub(crate) fn index(o: Orient) -> u8 {
    Orient::ALL.iter().position(|&a| a == o).unwrap_or(0) as u8
}

/// The orientation of an index in [`Orient::ALL`].
pub(crate) fn from_index(i: u8) -> Orient {
    Orient::ALL[usize::from(i) % 8]
}

/// The inverse in the group: `o.then(inverse(o))` is R0.
pub(crate) fn inverse(o: Orient) -> Orient {
    Orient::ALL.into_iter().find(|&i| o.then(i) == Orient::R0).unwrap_or(Orient::R0)
}

/// The value an ORIENT gives a class: the orientation of its root, and the ORIENT behind it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Fixed {
    /// The orientation of the root.
    pub value: Orient,
    /// The object the ORIENT is on.
    pub node: usize,
    /// The ORIENT, by its index among the units.
    pub fact: usize,
}

/// The orientation classes of the objects of a problem, by object index.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct OrientClasses {
    parent: Vec<usize>,
    /// `ori(node) = ori(parent).then(label)`; R0 for a root.
    label: Vec<Orient>,
    /// The fact of the edge to the parent; none for a root.
    edge: Vec<Option<usize>>,
    size: Vec<usize>,
    /// For a root: the value an ORIENT gives its class.
    fixed: Vec<Option<Fixed>>,
}

impl OrientClasses {
    /// Each of `n` objects in a class of its own, free.
    pub(crate) fn new(n: usize) -> OrientClasses {
        OrientClasses {
            parent: (0..n).collect(),
            label: vec![Orient::R0; n],
            edge: vec![None; n],
            size: vec![1; n],
            fixed: vec![None; n],
        }
    }

    /// The root of the class of `k`, and the label of `k` from it: `ori(k) = ori(root).then(label)`.
    pub(crate) fn find(&self, k: usize) -> (usize, Orient) {
        let mut label = Orient::R0;
        let mut node = k;
        // ori(k) = ori(p1).then(l_k); ori(p1) = ori(p2).then(l_p1); ... so the label from the root
        // is l_pn.then(...).then(l_p1).then(l_k): collect upwards, compose from the top
        let mut chain = Vec::new();
        while self.parent[node] != node {
            chain.push(self.label[node]);
            node = self.parent[node];
        }
        for l in chain.into_iter().rev() {
            label = label.then(l);
        }
        (node, label)
    }

    /// The value an ORIENT gives the class rooted at `root`.
    pub(crate) fn fixed(&self, root: usize) -> Option<Fixed> {
        self.fixed[root]
    }

    /// The orientation of `k` when its class is fixed. For tests.
    #[cfg(test)]
    pub(crate) fn value_of(&self, k: usize) -> Option<Orient> {
        let (r, g) = self.find(k);
        self.fixed[r].map(|f| f.value.then(g))
    }

    /// The facts on the edges from `k` up to its root.
    pub(crate) fn path_to_root(&self, k: usize) -> Vec<usize> {
        let mut out = Vec::new();
        let mut node = k;
        while self.parent[node] != node {
            out.extend(self.edge[node]);
            node = self.parent[node];
        }
        sorted(out)
    }

    /// The facts on the path between `a` and `b`, which are in one class: the edges up to their
    /// lowest common ancestor.
    pub(crate) fn path(&self, a: usize, b: usize) -> Vec<usize> {
        let mut up_a = vec![a];
        let mut top = a;
        while self.parent[top] != top {
            top = self.parent[top];
            up_a.push(top);
        }
        let mut out = Vec::new();
        let mut node = b;
        let meet = loop {
            if let Some(i) = up_a.iter().position(|&n| n == node) {
                break i;
            }
            if self.parent[node] == node {
                // not in one class: nothing connects them
                break up_a.len() - 1;
            }
            out.extend(self.edge[node]);
            node = self.parent[node];
        };
        for &n in &up_a[..meet] {
            out.extend(self.edge[n]);
        }
        sorted(out)
    }

    /// The facts that explain the value of the class of `k`: the ORIENT that fixes it and the path
    /// from its object to `k`. Empty when the class is free.
    pub(crate) fn fixed_reason(&self, k: usize) -> Vec<usize> {
        let (r, _) = self.find(k);
        match self.fixed[r] {
            Some(f) => {
                let mut out = self.path(k, f.node);
                out.push(f.fact);
                sorted(out)
            }
            None => Vec::new(),
        }
    }

    /// ORIENT (11.2): `k` has the orientation `code`, by the unit `fact`. A conflict gives the
    /// facts it conflicts with.
    pub(crate) fn fix(&mut self, k: usize, code: Orient, fact: usize) -> Result<(), Vec<usize>> {
        let (r, g) = self.find(k);
        // ori(r).then(g) = code
        let value = code.then(inverse(g));
        match self.fixed[r] {
            Some(f) if f.value != value => Err(self.fixed_reason(k)),
            Some(_) => Ok(()),
            None => {
                self.fixed[r] = Some(Fixed { value, node: k, fact });
                Ok(())
            }
        }
    }

    /// ORIENT-RELATION (14.9): `ori(b) = ori(a).then(relation)`, by the unit `fact`. A conflict
    /// gives the facts it conflicts with.
    ///
    /// The edge goes between `a` and `b` themselves, not between their roots (a proof forest): the
    /// smaller tree is first rerooted at its end of the edge, so that every edge is one fact that
    /// relates its two ends, and a path between two members explains how they relate.
    pub(crate) fn relate(&mut self, a: usize, b: usize, relation: Orient, fact: usize) -> Result<(), Vec<usize>> {
        let (ra, ga) = self.find(a);
        let (rb, gb) = self.find(b);
        if ra == rb {
            return if gb == ga.then(relation) { Ok(()) } else { Err(self.path(a, b)) };
        }
        // ori(rb).then(gb) = ori(ra).then(ga).then(relation), so ori(rb) = ori(ra).then(l)
        let l = ga.then(relation).then(inverse(gb));
        if let (Some(fa), Some(fb)) = (self.fixed[ra], self.fixed[rb]) {
            if fa.value.then(l) != fb.value {
                let mut out = self.fixed_reason(a);
                out.extend(self.fixed_reason(b));
                return Err(sorted(out));
            }
        }
        // the smaller tree hangs from the other end; of equal sizes, b's from a
        let (x, y, label) = if self.size[rb] <= self.size[ra] { (b, a, relation) } else { (a, b, inverse(relation)) };
        let (ry, gy) = self.find(y);
        self.reroot(x);
        // ori(x) = ori(y).then(label)
        self.parent[x] = y;
        self.label[x] = label;
        self.edge[x] = Some(fact);
        self.size[ry] += self.size[x];
        if let Some(f) = self.fixed[x].take() {
            if self.fixed[ry].is_none() {
                // ori(x) = ori(ry).then(gy).then(label) = f.value
                self.fixed[ry] = Some(Fixed { value: f.value.then(inverse(label)).then(inverse(gy)), ..f });
            }
        }
        Ok(())
    }

    /// Makes `x` the root of its tree, the edges on its path to the old root turned round, and
    /// the value of the class expressed for it.
    fn reroot(&mut self, x: usize) {
        let (r, gx) = self.find(x);
        let mut path = vec![x];
        while let Some(&n) = path.last() {
            if self.parent[n] == n {
                break;
            }
            path.push(self.parent[n]);
        }
        let old: Vec<(Orient, Option<usize>)> = path.iter().map(|&n| (self.label[n], self.edge[n])).collect();
        // child → parent with ori(child) = ori(parent).then(g) becomes parent → child with the inverse
        for i in 0..path.len() - 1 {
            let (child, parent) = (path[i], path[i + 1]);
            self.parent[parent] = child;
            self.label[parent] = inverse(old[i].0);
            self.edge[parent] = old[i].1;
        }
        self.parent[x] = x;
        self.label[x] = Orient::R0;
        self.edge[x] = None;
        self.size[x] = self.size[r];
        // ori(x) = ori(r).then(gx)
        self.fixed[x] = self.fixed[r].take().map(|f| Fixed { value: f.value.then(gx), ..f });
    }

    /// For each object, in order, the root of its class and its label from it ([`Self::find`]).
    /// For tests.
    #[cfg(test)]
    pub(crate) fn roots(&self) -> Vec<(usize, Orient)> {
        (0..self.parent.len()).map(|k| self.find(k)).collect()
    }
}

fn sorted(mut v: Vec<usize>) -> Vec<usize> {
    v.sort_unstable();
    v.dedup();
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The orientation of every object, given the value of each root: what the classes allow.
    fn orientations(c: &OrientClasses, root_value: impl Fn(usize) -> Orient) -> Vec<Orient> {
        c.roots().into_iter().map(|(r, g)| root_value(r).then(g)).collect()
    }

    #[test]
    fn the_group_and_its_inverses() {
        for o in Orient::ALL {
            assert_eq!(o.then(inverse(o)), Orient::R0);
            assert_eq!(inverse(o).then(o), Orient::R0);
            assert_eq!(from_index(index(o)), o);
        }
        // (a.then(b)).then(c) = a.then(b.then(c))
        for a in Orient::ALL {
            for b in Orient::ALL {
                for c in Orient::ALL {
                    assert_eq!(a.then(b).then(c), a.then(b.then(c)));
                }
            }
        }
    }

    #[test]
    fn relations_hold_from_whatever_value_the_root_takes() {
        let mut c = OrientClasses::new(5);
        c.relate(0, 1, Orient::MY, 10).unwrap();
        c.relate(2, 1, Orient::R90, 11).unwrap();
        c.relate(3, 4, Orient::MX, 12).unwrap();
        c.relate(4, 0, Orient::R0, 13).unwrap();
        for v in Orient::ALL {
            let o = orientations(&c, |_| v);
            assert_eq!(o[1], o[0].then(Orient::MY));
            assert_eq!(o[1], o[2].then(Orient::R90));
            assert_eq!(o[4], o[3].then(Orient::MX));
            assert_eq!(o[0], o[4]);
        }
        // consistent again: no change; inconsistent: the path that closes the cycle
        assert_eq!(c.relate(0, 1, Orient::MY, 20), Ok(()));
        assert_eq!(c.relate(0, 2, Orient::R0, 21), Err(vec![10, 11]));
        assert_eq!(c.relate(3, 1, Orient::MY, 22), Err(vec![10, 12, 13]));
    }

    #[test]
    fn orient_fixes_a_class_and_conflicts_name_the_cycle() {
        let mut c = OrientClasses::new(4);
        c.relate(0, 1, Orient::MY, 1).unwrap();
        assert_eq!(c.fix(0, Orient::R0, 2), Ok(()));
        assert_eq!(c.value_of(1), Some(Orient::MY));
        assert_eq!(c.fix(1, Orient::MY, 3), Ok(()));
        assert_eq!(c.fix(1, Orient::R0, 4), Err(vec![1, 2]));
        // two fixed classes joined inconsistently
        c.fix(2, Orient::R90, 5).unwrap();
        c.relate(3, 2, Orient::R0, 6).unwrap();
        assert_eq!(c.relate(1, 3, Orient::R0, 7), Err(vec![1, 2, 5, 6]));
        // and consistently: the merged class keeps one value
        // ori(3) = R90 = MY.then(MYR90)
        assert_eq!(c.relate(1, 3, Orient::MYR90, 8), Ok(()));
        assert_eq!(c.value_of(0), Some(Orient::R0));
        assert_eq!(c.value_of(2), Some(Orient::R90));
        assert_eq!(c.value_of(3), Some(Orient::R90));
    }

    #[test]
    fn a_fixed_value_survives_the_union_on_either_side() {
        for (first, second) in [(0, 1), (1, 0)] {
            let mut c = OrientClasses::new(3);
            c.relate(1, 2, Orient::R0, 1).unwrap();
            c.fix(first, Orient::MX, 2).unwrap();
            c.relate(first, second, Orient::R90, 3).unwrap();
            assert_eq!(c.value_of(first), Some(Orient::MX));
            assert_eq!(c.value_of(second), Some(Orient::MX.then(Orient::R90)));
            assert_eq!(c.fixed_reason(second), vec![2, 3]);
        }
    }

    /// One ORIENT or ORIENT-RELATION of a random test.
    #[derive(Clone, Copy, Debug)]
    enum Op {
        Fix(usize, Orient),
        Relate(usize, usize, Orient),
    }

    fn apply(c: &mut OrientClasses, op: Op, fact: usize) -> Result<(), Vec<usize>> {
        match op {
            Op::Fix(k, o) => c.fix(k, o, fact),
            Op::Relate(a, b, o) => c.relate(a, b, o, fact),
        }
    }

    /// Whether some ops, by index, hold together.
    fn consistent(ops: &[Op], which: &[usize]) -> bool {
        let mut c = OrientClasses::new(6);
        which.iter().all(|&i| apply(&mut c, ops[i], i).is_ok())
    }

    #[test]
    fn explanations_are_minimal_on_random_facts() {
        let mut seed: u64 = 0x2545_f491_4f6c_dd1d;
        let mut next = move |n: u64| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % n) as usize
        };
        for _ in 0..300 {
            let mut c = OrientClasses::new(6);
            let mut ops = Vec::new();
            let mut kept = Vec::new();
            for i in 0..12 {
                let op = if next(4) == 0 {
                    Op::Fix(next(6), Orient::ALL[next(8)])
                } else {
                    Op::Relate(next(6), next(6), Orient::ALL[next(8)])
                };
                ops.push(op);
                match apply(&mut c, op, i) {
                    Ok(()) => kept.push(i),
                    Err(set) => {
                        assert!(set.iter().all(|f| kept.contains(f)), "{set:?} {kept:?}");
                        let with = |set: &[usize]| set.iter().copied().chain([i]).collect::<Vec<_>>();
                        assert!(!consistent(&ops, &with(&set)), "{ops:?}: {set:?} does not conflict with {i}");
                        for f in &set {
                            let rest: Vec<usize> = set.iter().copied().filter(|g| g != f).collect();
                            assert!(consistent(&ops, &with(&rest)), "{ops:?}: {set:?} without {f}");
                        }
                    }
                }
                // every kept fact holds of the orientations the classes give
                for v in Orient::ALL {
                    let o = orientations(&c, |r| c.fixed(r).map_or(v, |f| f.value));
                    for &k in &kept {
                        match ops[k] {
                            Op::Fix(n, code) => assert_eq!(o[n], code),
                            Op::Relate(a, b, rel) => assert_eq!(o[b], o[a].then(rel)),
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn the_path_between_two_members_stops_at_their_common_ancestor() {
        let mut c = OrientClasses::new(6);
        c.relate(0, 1, Orient::R0, 1).unwrap();
        c.relate(2, 3, Orient::R0, 2).unwrap();
        c.relate(0, 2, Orient::R0, 3).unwrap();
        c.relate(4, 5, Orient::R0, 4).unwrap();
        c.relate(0, 4, Orient::R0, 5).unwrap();
        assert_eq!(c.path(1, 0), vec![1]);
        assert_eq!(c.path(3, 2), vec![2]);
        assert_eq!(c.path(1, 3), vec![1, 2, 3]);
        assert_eq!(c.path(5, 1), vec![1, 4, 5]);
        assert_eq!(c.path(4, 4), Vec::<usize>::new());
    }
}
