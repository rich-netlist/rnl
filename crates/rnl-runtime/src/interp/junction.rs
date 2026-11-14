//! Resolve a junction setting in the target level's generated-record name environment (13.2).
use super::{
    obj::{ObjData, ObjId, SceneId, UnitId},
    Interp,
};
use crate::{junction::JunctionConfig, problem::SolverSetting};

impl Interp<'_> {
    pub(super) fn junction_with_tools_for(
        &mut self,
        scene: SceneId,
        settings: &[SolverSetting],
    ) -> (JunctionConfig, Option<ObjId>) {
        let (mut file, object) = self.junction_for(scene, settings);
        if !self.follows_0_4(self.level_unit[self.scenes[scene].level]) {
            return (file, object);
        }
        let setting = self.tool_solver_config.iter().rev().find(|s| s.key == "junction").cloned();
        if let Some(setting) = setting {
            let (tool, object) = self.junction_for(scene, &[setting]);
            file.tool_override = Some(Box::new(tool));
            (file, object)
        } else {
            (file, object)
        }
    }

    pub(super) fn junction_for(
        &mut self,
        scene: SceneId,
        settings: &[SolverSetting],
    ) -> (JunctionConfig, Option<ObjId>) {
        let unit = self.level_unit[self.scenes[scene].level];
        if !self.follows_0_4(unit) {
            return (JunctionConfig::default(), None);
        }
        // the last value that names a definition here is in effect; a later one that does not is
        // kept for a warning, and an earlier one stands (14.10)
        let mut invalid = None;
        for setting in settings.iter().rev().filter(|s| s.key == "junction") {
            if setting.value.str() == Some("") {
                return (JunctionConfig { definition: None, invalid, tool_override: None }, None);
            }
            match self.junction_definition(scene, unit, setting) {
                Some((name, object)) => {
                    return (JunctionConfig { definition: Some(name), invalid, tool_override: None }, Some(object))
                }
                None => {
                    invalid.get_or_insert_with(|| setting.clone());
                }
            }
        }
        (JunctionConfig { definition: None, invalid, tool_override: None }, None)
    }

    /// The definition that a value of `junction` names in the level, and its name as a usage
    /// writes it; None when it names none whose first role accepts a point (13.2).
    fn junction_definition(
        &mut self,
        scene: SceneId,
        unit: UnitId,
        setting: &SolverSetting,
    ) -> Option<(String, ObjId)> {
        // A local name is meaningful only when the setting was written in this level,
        // not inherited from a file/package header or supplied by a tool (14.10).
        let local = self.scenes[scene]
            .solver_config
            .iter()
            .any(|fact| !self.failed_body(&fact.site) && fact.values.contains(setting));
        let prefix = if setting.value.str().is_some_and(|name| name.starts_with('%')) && local { '%' } else { '$' };
        setting.value.str().and_then(|text| crate::syntax::definition_name(text, prefix)).and_then(|(name, _quoted)| {
            let name = self.word(&format!("{prefix}{name}"), unit).strip_prefix(prefix)?.to_string();
            let binding = self.lookup_unit(prefix, &name, unit, false, Some(scene))?;
            let home = self.home_scene(binding.owner);
            let target = if self.places.rules.contains(&binding.key()) { Some(scene) } else { home };
            let object = if self.modern_binding(&binding) {
                self.evaluated_binding(&binding, target)?
            } else {
                let value = self
                    .eval_member_at(
                        binding.owner,
                        binding.rec,
                        binding.index,
                        binding.embedded,
                        &binding.member,
                        target,
                        binding.header.clone(),
                        None,
                        "",
                    )
                    .ok()?;
                self.bound_value(&binding.key(), target, binding.prefix, &binding.name, value)
            };
            let ObjData::Definition(definition) = self.o(object) else { return None };
            let role = definition.roles.first()?;
            if role.kinds.as_ref().is_some_and(|k| !k.iter().any(|k| k == "point")) {
                return None;
            }
            Some((format!("{prefix}{}", crate::model::quoted(&name)), object))
        })
    }
}
