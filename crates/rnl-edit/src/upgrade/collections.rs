//! Trace named collection dependencies for operations whose meaning changed (5.2, 10.4).
use rnl_runtime::source::normalize;
use rnl_runtime::syntax::{parse_record_features, Member, ObjExpr, RefOrHole, Sep};
use rnl_runtime::Interpretation;
use std::collections::{BTreeMap, BTreeSet};
type Key = (usize, String);
#[derive(Clone, Default)]
struct Set {
    known: bool,
    selected: bool,
    refs: BTreeSet<String>,
}
#[derive(Default)]
pub(super) struct Collections {
    sets: BTreeMap<Key, Set>,
}

fn selection(member: &Member) -> bool {
    member.segments.iter().any(|s| {
        s.call.as_ref().is_some_and(|c| c.kw.starts_with("SELECT-"))
            || s.args.iter().any(|a| matches!(&a.obj, ObjExpr::Set(set) if set.members.iter().any(|m| selection(m))))
    })
}
fn references(member: &Member) -> BTreeSet<String> {
    let mut refs = Vec::new();
    rnl_runtime::syntax::walk_refs(member, &mut refs);
    refs.into_iter()
        .filter_map(|r| match r {
            RefOrHole::Ref(r) if matches!(r.prefix, '%' | '$') && r.path.is_empty() => {
                r.name.as_ref().map(|n| format!("{}{n}", r.prefix))
            }
            _ => None,
        })
        .collect()
}
impl Collections {
    pub(super) fn new(input: &Interpretation, readings: &BTreeMap<(String, u32, u32), Vec<usize>>) -> Self {
        let mut out = Self::default();
        let mut counts = BTreeMap::<Key, usize>::new();
        let mut edits = Vec::new();
        for level in &input.doc.levels {
            for record in &level.records {
                let Some(levels) = readings.get(&(normalize(&level.path), record.lines.0, record.lines.1)) else {
                    continue;
                };
                let Some(body) = parse_record_features(&record.text, true, false).body else { continue };
                for member in &body.members {
                    let refs = references(member);
                    for bind in &member.binds {
                        if !matches!(bind.prefix, '%' | '$') {
                            continue;
                        }
                        let name = format!("{}{}", bind.prefix, bind.name);
                        let Some(segment) = member.segments.get(bind.after) else { continue };
                        let known = segment.call.as_ref().is_some_and(|c| c.kw.starts_with("SELECT-"))
                            || (segment.call.is_none()
                                && matches!(segment.args.as_slice(),[a] if matches!(a.obj,ObjExpr::Set(_))));
                        let alias = segment.call.is_none()
                            && matches!(segment.args.as_slice(),[a] if matches!(a.obj,ObjExpr::Ref(_)));
                        for level in levels {
                            let key = (*level, name.clone());
                            *counts.entry(key.clone()).or_default() += 1;
                            if known || alias {
                                out.sets.insert(key, Set { known, selected: selection(member), refs: refs.clone() });
                            }
                        }
                    }
                    for segment in &member.segments {
                        if segment.call.as_ref().is_some_and(|c| matches!(c.kw.as_str(), "ADD-TO" | "REMOVE-FROM")) {
                            for arg in &segment.args {
                                if let ObjExpr::Ref(r) = &arg.obj {
                                    if let Some(name) =
                                        r.name.as_ref().filter(|_| matches!(r.prefix, '%' | '$') && r.path.is_empty())
                                    {
                                        for level in levels {
                                            edits.push((
                                                (*level, format!("{}{name}", r.prefix)),
                                                refs.clone(),
                                                selection(member),
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        out.sets.retain(|k, _| counts.get(k) == Some(&1));
        for (key, refs, selected) in edits {
            if let Some(set) = out.sets.get_mut(&key) {
                set.refs.extend(refs);
                set.selected |= selected;
            }
        }
        for _ in 0..out.sets.len() {
            let old = out.sets.clone();
            let mut changed = false;
            for ((level, _), set) in &mut out.sets {
                for name in &set.refs {
                    if let Some(parent) = old.get(&(*level, name.clone())) {
                        changed |= (!set.known && parent.known) || (!set.selected && parent.selected);
                        set.known |= parent.known;
                        set.selected |= parent.selected;
                    }
                }
            }
            if !changed {
                break;
            }
        }
        out
    }
    fn named(&self, readings: &[usize], object: &ObjExpr, selected: bool) -> bool {
        let ObjExpr::Ref(r) = object else { return false };
        let Some(name) = &r.name else { return false };
        readings.iter().any(|level| {
            self.sets
                .get(&(*level, format!("{}{name}", r.prefix)))
                .is_some_and(|s| s.known && (!selected || s.selected))
        })
    }
    fn dependent(&self, readings: &[usize], object: &ObjExpr) -> bool {
        self.named(readings, object, true)
            || matches!(object,ObjExpr::Set(s) if s.members.iter().any(|m| selection(m)||m.segments.iter().flat_map(|s|&s.args).any(|a|self.dependent(readings,&a.obj))))
    }
    fn direct_set(&self, readings: &[usize], object: &ObjExpr) -> bool {
        matches!(object, ObjExpr::Set(_))
            || matches!(object, ObjExpr::Ref(r) if r.path.is_empty() && self.named(readings, object, false))
    }
    pub(super) fn reasons(&self, readings: &[usize], member: &Member) -> Vec<super::manual::Reason> {
        let mut reasons = Vec::new();
        for segment in &member.segments {
            if segment.args.iter().any(|a|matches!(&a.obj,ObjExpr::Ref(r) if r.path.first().is_some_and(|p|p.sep==Sep::Sub&&p.name=="position")&&self.named(readings,&a.obj,false))) {
                reasons.push(("set-position","sets have no position in 0.4; choose an explicit point or usage that preserves the old set centre (5.2, 6.5)"));
            }
            let Some(call) = &segment.call else { continue };
            let sets = segment.args.iter().any(|a| self.direct_set(readings, &a.obj));
            if call.kw == "PLACE" && sets {
                reasons.push(("set-placement","placing a set now places every member; preserve the old collective placement with explicit points or usages (11.1)"));
            }
            if call.kw == "DISTANCE" && sets {
                reasons.push(("distance-sets","distance between set extents becomes pairwise point distances; the old range is not a single point (14.5)"));
            }
            if call.kw == "ORIENT-RELATION"
                && segment.args.len() == 2
                && segment.args.iter().all(|a| self.direct_set(readings, &a.obj))
            {
                reasons.push(("orientation-sets", "two collection slots no longer form one orientation fact; splitting changes conflict resolution (14.9)"));
            }
            let two_points = matches!(call.kw.as_str(), "RECTANGLE" | "CIRCLE" | "ELLIPSE")
                && call
                    .params
                    .as_ref()
                    .is_none_or(|p| ["width", "height", "radius", "rx", "ry"].iter().all(|k| p.get(k).is_none()));
            if (matches!(call.kw.as_str(), "WIRE" | "BUNDLE") || two_points)
                && (selection(member) || segment.args.iter().any(|a| self.dependent(readings, &a.obj)))
            {
                reasons.push(("selection-dependent-creation","this operation depends on a selection; choose explicit members or a routing rule with equivalent topology before migration (10.4, 11.8, 13.1, 13.4)"));
            }
        }
        reasons
    }
}
