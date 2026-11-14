//! Literal solver settings: every value of each key, in the order of 14.10.
use super::ctx::Ctx;
use super::eval::{ArgV, R};
use super::obj::{Lit, ObjId, Pos, RecId, SceneId, UnitId, WStr, NOTHING};
use super::unit::UnitKind;
use super::Interp;
use crate::diag::{codes, Code, Severity};
use crate::host::language::Languages;
use crate::problem::SolverSetting;
use crate::syntax::{Named, Params, Segment, Value};
use std::collections::HashSet;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct ConfigFact {
    pub values: Vec<SolverSetting>,
    pub site: Rc<Ctx>,
    pub pos: Pos,
}

fn entries(params: Option<&Params>) -> Result<Vec<&Named>, (Code, String, usize)> {
    let Some(params) = params else { return Ok(Vec::new()) };
    if !params.positional.is_empty() {
        return Err((
            codes::PARAM_POSITIONAL_NOT_ALLOWED,
            "SOLVER-CONFIG takes named parameters, as key=value (14.10)".into(),
            params.start,
        ));
    }
    let mut seen = HashSet::new();
    for item in &params.named {
        if !seen.insert(&item.name) {
            return Err((
                codes::PARAM_WRITTEN_TWICE,
                format!("parameter {} of @SOLVER-CONFIG is written twice (4.2 rule 9)", item.name),
                item.off,
            ));
        }
    }
    Ok(params.named.iter().collect())
}

fn flat(value: &Lit) -> bool {
    match value {
        Lit::Arr(items) => items.iter().all(|v| !matches!(v, Lit::Arr(_))),
        _ => true,
    }
}

fn literal(value: &Value, unit: UnitId) -> Option<Lit> {
    Some(match value {
        Value::Num { value, .. } => Lit::Num(*value),
        Value::Str(value) => Lit::Str(WStr::new(value, Some(unit))),
        Value::Bool(value) => Lit::Bool(*value),
        Value::Arr(values) => Lit::Arr(values.iter().map(|v| literal(v, unit)).collect::<Option<Vec<_>>>()?),
        _ => return None,
    })
}

fn set_unit(value: &mut Lit, unit: UnitId) {
    match value {
        Lit::Str(value) => {
            value.unit.get_or_insert(unit);
        }
        Lit::Arr(values) => {
            for value in values {
                set_unit(value, unit)
            }
        }
        _ => {}
    }
}

const VALUE: &str = "a SOLVER-CONFIG value is a number, string, boolean, or a flat array of these (14.10)";

/// Whether the core takes a value of `spacing_x` or `spacing_y`: a finite whole number of grid
/// units (11.5). A value of another key is the solver's to judge, and `junction` is judged in each
/// level, as it names a definition there ([`Interp::solver_config_for`]).
fn spacing_taken(key: &str, value: &Lit) -> bool {
    !matches!(key, "spacing_x" | "spacing_y") || matches!(value, Lit::Num(n) if n.is_finite() && n.fract() == 0.0)
}

/// The error of a value of `spacing_x` or `spacing_y` that the core does not take: it fails its
/// key alone, and the statement's other keys apply (14.10).
fn spacing_error(key: &str) -> String {
    format!(
        "the {key} of @SOLVER-CONFIG must be a finite whole number of grid units (11.5); the value is left out, \
         and the one before it stays in effect (14.10)"
    )
}
const CONTEXT: &str="SOLVER-CONFIG has no effect in this context; write it in an unqualified file or package header, or a level record (14.10)";

impl Interp<'_> {
    fn solver_config_key(&self, item: &Named, unit: UnitId) -> String {
        let translated = self.word(&item.name, unit);
        if !item.quoted && matches!(translated.as_str(), "spacing_x" | "spacing_y" | "junction") {
            translated
        } else {
            item.name.clone()
        }
    }

    /// Check the open signature without substituting application parameters (6.9).
    pub(super) fn check_solver_config_at_definition(&mut self, seg: &Segment, ctx: &Ctx) -> R<()> {
        fn has_parameter(value: &Value) -> bool {
            match value {
                Value::Hole(_) => true,
                Value::Arr(items) => items.iter().any(has_parameter),
                _ => false,
            }
        }
        let items = match entries(seg.call.as_ref().and_then(|call| call.params.as_ref())) {
            Ok(items) => items,
            Err((code, message, off)) => return self.fail(ctx, code, message, off),
        };
        let mut seen = HashSet::new();
        for item in items {
            let key = self.solver_config_key(item, ctx.unit);
            if !seen.insert(key.clone()) {
                return self.fail(
                    ctx,
                    codes::PARAM_WRITTEN_TWICE,
                    format!("parameter {key} of @SOLVER-CONFIG is written twice (4.2 rule 9)"),
                    item.off,
                );
            }
            if matches!(&item.value, Value::Arr(values) if values.iter().any(|value| matches!(value, Value::Arr(_)))) {
                return self.fail(ctx, codes::SOLVER_CONFIG_VALUE_NOT_LITERAL, VALUE, item.off);
            }
            if !has_parameter(&item.value) {
                if let Some(value) = self.value(&item.value, ctx, item.off)? {
                    if !flat(&value) {
                        return self.fail(ctx, codes::SOLVER_CONFIG_VALUE_NOT_LITERAL, VALUE, item.off);
                    }
                }
            }
        }
        Ok(())
    }

    pub(super) fn header_solver_config(&mut self, unit: UnitId, rec: RecId, seg: &Segment) {
        let package = self.units[unit].pkg().to_string();
        if self.rec_header[&rec].languages != Languages::ALL || self.units[unit].embedded {
            self.diag_in(
                Severity::Warning,
                codes::SOLVER_CONFIG_CONTEXT,
                CONTEXT,
                Some(self.recs[rec].loc(seg.start)),
                &package,
            );
            return;
        }
        if !seg.args.is_empty() {
            self.diag_in(
                Severity::Error,
                codes::HEADER_STATEMENT_TAKES_NO_OBJECTS,
                "@SOLVER-CONFIG takes no objects (14.10)",
                Some(self.recs[rec].loc(seg.start)),
                &package,
            );
            return;
        }
        let items = match entries(seg.call.as_ref().and_then(|c| c.params.as_ref())) {
            Ok(items) => items,
            Err((code, message, off)) => {
                self.diag_in(Severity::Error, code, message, Some(self.recs[rec].loc(off)), &package);
                return;
            }
        };
        let mut values = Vec::new();
        for item in items {
            let value = literal(&item.value, unit).filter(flat);
            let Some(value) = value else {
                self.diag_in(
                    Severity::Error,
                    codes::SOLVER_CONFIG_VALUE_NOT_LITERAL,
                    VALUE,
                    Some(self.recs[rec].loc(item.off)),
                    &package,
                );
                return;
            };
            if !spacing_taken(&item.name, &value) {
                let at = Some(self.recs[rec].loc(item.off));
                self.diag_in(
                    Severity::Error,
                    codes::SOLVER_CONFIG_SPACING_NOT_WHOLE,
                    spacing_error(&item.name),
                    at,
                    &package,
                );
                continue;
            }
            values.push(SolverSetting {
                key: item.name.clone(),
                value,
                source: Some(self.recs[rec].loc(item.off)),
                package: package.clone(),
            });
        }
        self.units[unit].solver_config.extend(values);
    }

    pub(super) fn kw_solver_config(&mut self, args: &[ArgV], seg: &Segment, ctx: &Ctx) -> R<(ObjId, bool)> {
        let unit = self.source_unit(ctx);
        if self.units[unit].kind == UnitKind::Package
            || self.units[ctx.unit].kind == UnitKind::Package
            || ctx.header.languages != Languages::ALL
            || ctx.header.type_ == "generated"
        {
            self.warn(ctx, codes::SOLVER_CONFIG_CONTEXT, CONTEXT, seg.start);
            return Ok((NOTHING, false));
        }
        self.map_args("SOLVER-CONFIG", args, seg, ctx, &[])?;
        let Some(scene) = ctx.scene else { return self.scene_only(ctx, seg.start) };
        let items = match entries(seg.call.as_ref().and_then(|c| c.params.as_ref())) {
            Ok(items) => items,
            Err((code, message, off)) => return self.fail(ctx, code, message, off),
        };
        let mut values = Vec::new();
        let mut seen = HashSet::new();
        for item in items {
            let key = self.solver_config_key(item, ctx.unit);
            if !seen.insert(key.clone()) {
                return self.fail(
                    ctx,
                    codes::PARAM_WRITTEN_TWICE,
                    format!("parameter {key} of @SOLVER-CONFIG is written twice (4.2 rule 9)"),
                    item.off,
                );
            }
            let Some(mut value) = self.value(&item.value, ctx, item.off)? else { continue };
            if !flat(&value) {
                return self.fail(ctx, codes::SOLVER_CONFIG_VALUE_NOT_LITERAL, VALUE, item.off);
            }
            set_unit(&mut value, unit);
            if !spacing_taken(&key, &value) {
                self.err(ctx, codes::SOLVER_CONFIG_SPACING_NOT_WHOLE, spacing_error(&key), item.off);
                continue;
            }
            values.push(SolverSetting {
                key,
                value,
                source: self.rec_loc(ctx.rec, item.off),
                package: self.units[unit].pkg().to_string(),
            });
        }
        self.scenes[scene].solver_config.push(ConfigFact { values, site: Rc::new(ctx.clone()), pos: ctx.pos.clone() });
        Ok((NOTHING, false))
    }

    /// Every value of every key that applies to the level (14.10), by key, and the values of a
    /// key in the order of 14.10: the packages in their effective order, the file's header, the
    /// level's records. A later value does not replace an earlier one here: the solver takes per
    /// key the last value it accepts, so an earlier one stands when a later one is refused. The
    /// values of the keys this standard names that the core does not take are left out, with an
    /// error: of `spacing_x` and `spacing_y` when they are read, of `junction` here, a value that
    /// names no definition whose first role accepts a point in this level (13.2).
    pub(super) fn solver_config_for(&mut self, scene: SceneId) -> Vec<SolverSetting> {
        let level = self.scenes[scene].level;
        let effective = self.effective_packages(level);
        let mut packages: Vec<_> = effective
            .entries
            .iter()
            .filter(|e| e.solver_config)
            .map(|e| (self.position_of(scene, e.unit, None, None), e.unit))
            .collect();
        packages.sort_by(|a, b| a.0.cmp(&b.0));
        let mut settings = Vec::new();
        for (_, unit) in packages {
            settings.extend(self.units[unit].solver_config.iter().cloned());
        }
        let file = self.doc.file_level(level);
        settings.extend(self.units[self.level_unit[file]].solver_config.iter().cloned());
        let mut facts = self.scenes[scene].solver_config.clone();
        facts.sort_by(|a, b| a.pos.cmp(&b.pos));
        for fact in facts {
            if self.failed_body(&fact.site) {
                continue;
            }
            settings.extend(fact.values);
        }
        // by key, a stable sort keeping each key's values in their order
        settings.sort_by(|a, b| a.key.cmp(&b.key));
        let unit = self.level_unit[level];
        if self.follows_0_4(unit) {
            let mut taken = Vec::with_capacity(settings.len());
            for s in settings {
                if s.key == "junction"
                    && s.value.str() != Some("")
                    && self.junction_definition(scene, unit, &s).is_none()
                {
                    let message = format!(
                        "junction={} must name a definition whose first role accepts a point in the level {}, or be \
                         empty (13.2); the value is left out, and the one before it stays in effect (14.10)",
                        s.value.show(),
                        self.doc.levels[level].title()
                    );
                    self.diag_in(
                        Severity::Error,
                        codes::SOLVER_CONFIG_JUNCTION_NOT_TAKEN,
                        message,
                        s.source.clone(),
                        &s.package,
                    );
                    continue;
                }
                taken.push(s);
            }
            settings = taken;
        }
        settings
    }
}
