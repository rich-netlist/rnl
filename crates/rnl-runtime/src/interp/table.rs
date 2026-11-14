//! Nested TABLE is one geometric fact and one group (14.11).
use std::collections::HashSet;

use crate::diag::{codes, Code, DiagId, Severity};
use crate::problem::{Axis, TableCell};

use super::ctx::Ctx;
use super::obj::*;
use super::Interp;

const MAX_DEPTH: usize = 64;
const MAX_ORDERS: usize = 4096;

struct Failure {
    cause: Option<DiagId>,
    code: Code,
    message: String,
}

impl Interp<'_> {
    pub(super) fn prepare_table(&mut self, scene: SceneId, object: ObjId, data: &ConstraintData) -> bool {
        let site = data.site.as_ref().expect("TABLE has a statement");
        let mut cells = 0;
        let result = self.table_cell(scene, data.args[0], site, 0, &mut cells);
        let rows = match result {
            Ok(rows) => rows,
            Err(failure) => {
                if failure.cause.is_some() {
                    self.err_cause(site, failure.code, failure.message, data.off, failure.cause);
                } else {
                    self.diag_site(Severity::Error, failure.code, &data.site, data.off, failure.message);
                }
                self.constraint_mut(object).failed = true;
                return false;
            }
        };
        let leaves: Vec<_> = rows.points().into_iter().copied().collect();
        let mut seen = HashSet::new();
        for leaf in leaves {
            let invalid = if matches!(
                self.o(leaf),
                ObjData::Prim { .. }
                    | ObjData::Definition(_)
                    | ObjData::Symbol(_)
                    | ObjData::Style(_)
                    | ObjData::LevelObj { .. }
                    | ObjData::Marker
            ) {
                Some(format!("{} cannot be located", self.label(leaf)))
            } else {
                self.locatable(scene, leaf)
            };
            if let Some(why) = invalid {
                self.diag_site(
                    Severity::Error,
                    codes::TABLE_MEMBER_NOT_LOCATABLE,
                    &data.site,
                    data.off,
                    format!("a member of {}: {why}; the whole TABLE fails (14.1, 14.11)", self.label(object)),
                );
                self.constraint_mut(object).failed = true;
                return false;
            }
            let mut key = leaf;
            while let ObjData::Pos { parent } = *self.o(key) {
                if self.is_set(parent) {
                    break;
                }
                key = self.scene_value(parent);
            }
            if let Some((owner, index, _)) = self.direction_pin(scene, key) {
                key = self.scene_pin_object(scene, owner, index);
            }
            if !seen.insert(key) {
                self.diag_site(Severity::Error, codes::TABLE_OBJECT_IN_TWO_CELLS, &data.site, data.off,
                    format!("{} is in two cells of {}, and would have to lie strictly before itself; the TABLE fails (8.5, 14.11)", self.label(leaf), self.label(object)));
                self.constraint_mut(object).failed = true;
                return false;
            }
        }
        let mut orders = 0;
        if !rows.visit_orders(Axis::of(data.params.axis.unwrap_or('y')), &mut |_, _, _| {
            orders += 1;
            orders <= MAX_ORDERS
        }) {
            self.diag_site(
                Severity::Error,
                codes::TABLE_TOO_MANY_ORDERS,
                &data.site,
                data.off,
                Self::table_limit().message,
            );
            self.constraint_mut(object).failed = true;
            return false;
        }
        self.constraint_mut(object).table = Some(rows);
        true
    }

    fn table_limit() -> Failure {
        Failure { cause: None, code: codes::TABLE_TOO_MANY_ORDERS,
            message: format!("TABLE requires more than {MAX_ORDERS} point orders, a limit of this implementation; write a smaller table (3.7, 14.11)") }
    }

    fn table_cell(
        &mut self,
        scene: SceneId,
        written: ObjId,
        site: &Ctx,
        depth: usize,
        cells: &mut usize,
    ) -> Result<TableCell<ObjId>, Failure> {
        let object = self.scene_value(written);
        if self.is_failed(written) || self.is_failed(object) {
            return Err(Failure {
                cause: self.failed_obj(written).or_else(|| self.failed_obj(object)).flatten(),
                code: codes::TABLE_MEMBER_FAILED,
                message: "a member of TABLE failed, so the whole TABLE fails (7.3, 14.11)".into(),
            });
        }
        if !self.is_set(object) {
            *cells += 1;
            // A nonempty table of N cells needs at least N-1 adjacent-point orders.
            if *cells > MAX_ORDERS + 1 {
                return Err(Self::table_limit());
            }
            return Ok(TableCell::Point(object));
        }
        if depth >= MAX_DEPTH {
            return Err(Failure { cause: None, code: codes::TABLE_TOO_DEEP,
                message: format!("the lists of TABLE nest more than {MAX_DEPTH} deep, a limit of this implementation; the TABLE fails (3.7, 14.11)") });
        }
        let mut items = Vec::new();
        for member in self.written_items_with_failures(scene, object, 7, site, true) {
            if member.dependencies.iter().any(|dependency| self.is_failed(*dependency)) {
                return Err(Failure {
                    cause: member.dependencies.iter().find_map(|dependency| self.failed_obj(*dependency).flatten()),
                    code: codes::TABLE_MEMBER_FAILED,
                    message: "a member of TABLE failed, so the whole TABLE fails (7.3, 14.11)".into(),
                });
            }
            let cell = self.table_cell(scene, member.object, site, depth + 1, cells)?;
            if !cell.points().is_empty() {
                items.push(cell);
            }
        }
        Ok(TableCell::List(items))
    }
}
