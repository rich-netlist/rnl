//! Resolve a junction setting in the target level's generated-record name environment (13.2).
use super::{
    obj::{ObjData, ObjId, SceneId},
    Interp,
};
use crate::{junction::JunctionConfig, problem::SolverSetting};

impl Interp<'_> {
    pub(super) fn junction_for(
        &mut self,
        scene: SceneId,
        settings: &[SolverSetting],
    ) -> (JunctionConfig, Option<ObjId>) {
        let unit = self.level_unit[self.scenes[scene].level];
        if !self.follows_0_4(unit) {
            return (JunctionConfig::default(), None);
        }
        let Some(setting) = settings.iter().find(|s| s.key == "junction") else {
            return (JunctionConfig::default(), None);
        };
        if setting.value.str() == Some("") {
            return (JunctionConfig::default(), None);
        }
        let found = setting.value.str().and_then(crate::syntax::definition_name).and_then(|(name, _quoted)| {
            let name = self.word(&format!("${name}"), unit).strip_prefix('$')?.to_string();
            let binding = self.lookup_unit('$', &name, unit, false, Some(scene))?;
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
            Some((format!("${}", crate::model::quoted(&name)), object))
        });
        match found {
            Some((name, object)) => (JunctionConfig { definition: Some(name), invalid: None }, Some(object)),
            None => (JunctionConfig { definition: None, invalid: Some(setting.clone()) }, None),
        }
    }
}
