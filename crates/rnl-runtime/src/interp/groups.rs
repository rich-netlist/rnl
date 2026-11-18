//! Effective GROUP members, after symbols and final set changes (11.5).
use std::collections::HashSet;

use super::obj::{ObjData, ObjId, SceneId};
use super::scene::GroupFact;
use super::Interp;

impl Interp<'_> {
    pub(super) fn group_disabled(&mut self, scene: SceneId, group: &GroupFact) -> bool {
        let (handle, priority) = match group.table {
            Some(table) => match self.o(table) {
                ObjData::Constraint(data) => (table, data.priority_param),
                _ => return true,
            },
            None => (group.set, None),
        };
        self.eff_priority(scene, priority, &group.chain, Some(handle)) == super::obj::Priority::Disabled
            || (group.table.is_some()
                && self.scenes[scene].prioritys.iter().any(|fact| fact.target == group.set)
                && self.eff_priority(scene, None, &group.chain, Some(group.set)) == super::obj::Priority::Disabled)
    }

    pub(super) fn failed_group(&self, group: &GroupFact) -> bool {
        if let Some(table) = group.table {
            let ObjData::Constraint(data) = self.o(table) else { return true };
            if data.failed
                || self.is_failed(table)
                || data.status == Some("ignored")
                || data.table.as_ref().is_none_or(|rows| rows.points().len() < 2)
            {
                return true;
            }
        }
        self.failed_body(&group.site) || self.is_failed(group.set)
    }

    pub(super) fn group_members(&mut self, scene: SceneId, group: &GroupFact) -> Vec<ObjId> {
        let mut out = Vec::new();
        let mut active = HashSet::new();
        let mut stack = vec![(group.set, false)];
        while let Some((written, leaving)) = stack.pop() {
            let object = self.scene_value(written);
            if leaving {
                active.remove(&object);
                continue;
            }
            if self.is_failed(written) || self.is_failed(object) {
                continue;
            }
            let children = if self.is_set(object) {
                vec![object]
            } else if let ObjData::Usage(usage) = self.o(object) {
                if self.scenes[scene].symbol.contains_key(&object) {
                    out.push(object);
                    continue;
                }
                usage.roles.iter().map(|(_, filler)| *filler).collect()
            } else {
                out.push(object);
                continue;
            };
            // Guard only the current path: another occurrence of a usage still contributes
            // its roles, in their original order, including repeated members (5.2).
            if !active.insert(object) {
                continue;
            }
            stack.push((object, true));
            let mut members = Vec::new();
            for child in children {
                for member in self.scene_input_members(scene, child, Some(&group.site)) {
                    if !member.dependencies.iter().any(|dependency| self.is_failed(*dependency)) {
                        members.push((member.object, false));
                    }
                }
            }
            stack.extend(members.into_iter().rev());
        }
        out
    }
}
