//! The table of the solver's settings as JSON, for a tool that draws a page of them from data:
//! `rnl solver-defaults --json` prints it, and the solver's WebAssembly module gives it.
//!
//! It is [`rnl_solver::settings_table`] with the stages of a solve's progress
//! ([`crate::progress::STAGES`]). Fields are only ever added; a field that does not apply to a
//! setting is left out.

use serde_json::{json, Map, Value};

/// The table: `version`, `groups`, `settings`, `objectives`, `default_objectives` and `stages`.
pub fn settings_table_json() -> Value {
    let t = rnl_solver::settings_table();
    let when = |w: rnl_solver::When| -> Value {
        let mut map = Map::new();
        for (key, values) in w {
            map.insert((*key).to_string(), json!(values));
        }
        Value::Object(map)
    };
    let settings: Vec<Value> = t
        .settings
        .iter()
        .map(|s| {
            let mut row = Map::new();
            row.insert("key".into(), json!(s.key));
            row.insert("group".into(), json!(s.group));
            row.insert("type".into(), json!(s.kind.name()));
            if !s.choices.is_empty() {
                let choices: Vec<Value> = s.choices.iter().map(|c| json!({"value": c.value, "what": c.what})).collect();
                row.insert("choices".into(), Value::Array(choices));
            }
            for (name, bound) in [("min", s.min), ("min_exclusive", s.min_exclusive), ("max", s.max)] {
                if let Some(bound) = bound {
                    row.insert(name.into(), number(bound));
                }
            }
            if let Some(default) = &s.default {
                row.insert("default".into(), crate::solve::setting_json(default));
            }
            row.insert("in_file".into(), json!(s.in_file));
            row.insert("what".into(), json!(s.what));
            if let Some(what) = s.false_what {
                row.insert("false_what".into(), json!(what));
            }
            for (name, w) in [("applies", s.applies), ("shown_when", s.shown_when), ("required_when", s.required_when)]
            {
                if !w.is_empty() {
                    row.insert(name.into(), when(w));
                }
            }
            Value::Object(row)
        })
        .collect();
    let objectives: Vec<Value> =
        t.objectives.iter().map(|o| json!({"name": o.name, "what": o.what, "implemented": o.implemented})).collect();
    let layers: Vec<Value> = t
        .default_objectives
        .iter()
        .map(|l| {
            let items: Vec<Value> =
                l.items.iter().map(|(name, weight)| json!({"name": name, "weight": weight})).collect();
            json!({"items": items, "tolerance": number(l.tolerance)})
        })
        .collect();
    let groups: Vec<Value> = t.groups.iter().map(|g| json!({"name": g.name, "what": g.what})).collect();
    let stages: Vec<Value> =
        crate::progress::STAGES.iter().map(|(name, what)| json!({"name": name, "what": what})).collect();
    json!({
        "version": t.version,
        "groups": groups,
        "settings": settings,
        "objectives": objectives,
        "default_objectives": layers,
        "stages": stages,
    })
}

/// A whole number as an integer, as a page reads it, and another as it is.
fn number(n: f64) -> Value {
    crate::solve::setting_json(&rnl_runtime::interp::obj::Lit::Num(n))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_as_json() {
        let t = settings_table_json();
        assert_eq!(t["version"], 1);
        let setting = |key: &str| t["settings"].as_array().unwrap().iter().find(|s| s["key"] == key).unwrap().clone();
        assert_eq!(setting("layout")["default"], "lines");
        assert_eq!(setting("layout")["type"], "choice");
        assert_eq!(setting("layout")["choices"].as_array().unwrap().len(), 3);
        assert_eq!(setting("spacing_x")["applies"], json!({"layout": ["lines"]}));
        assert_eq!((setting("spacing_x")["min"].clone(), setting("spacing_x")["max"].clone()), (json!(-1), json!(20)));
        assert_eq!(setting("grid")["type"], "number-or-false");
        assert_eq!(setting("grid")["min_exclusive"], 0);
        assert!(setting("grid")["false_what"].is_string());
        assert_eq!(setting("work")["required_when"], json!({"limit": ["work"]}));
        assert!(setting("work").get("default").is_none());
        assert!(setting("junction").get("default").is_none());
        assert_eq!(setting("seconds")["default"], 10);
        assert_eq!(setting("seconds")["in_file"], false);
        assert_eq!(setting("beam")["in_file"], true);
        assert_eq!(setting("objectives")["default"], rnl_solver::DEFAULT_LAYERS.join(","));
        assert_eq!(t["objectives"].as_array().unwrap().len(), rnl_solver::OBJECTIVES.len());
        assert_eq!(
            t["default_objectives"][0],
            json!({"items": [{"name": "current_direction", "weight": 1}], "tolerance": 0})
        );
        let stages: Vec<&str> = t["stages"].as_array().unwrap().iter().map(|s| s["name"].as_str().unwrap()).collect();
        assert_eq!(stages, crate::progress::STAGES.iter().map(|(n, _)| *n).collect::<Vec<_>>());
        let groups: Vec<&str> = t["groups"].as_array().unwrap().iter().map(|g| g["name"].as_str().unwrap()).collect();
        assert!(t["settings"].as_array().unwrap().iter().all(|s| groups.contains(&s["group"].as_str().unwrap())));
    }
}
