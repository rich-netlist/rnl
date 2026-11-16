//! The table of the solver's settings as JSON, for a tool that draws a page of them from data:
//! `rnl solver-defaults --json` prints it, and the solver's WebAssembly module gives it.
//!
//! It is [`rnl_solver::settings_table`] with the stages of a solve's progress
//! ([`crate::progress::STAGES`]), each with the layouts it applies to. Fields are only ever added;
//! a field that does not apply to a row is left out, `applies` among them where a row applies to
//! every layout.

use serde_json::{json, Map, Value};

/// The table: `version`, `groups`, `settings`, `objectives`, `default_objectives`, `stages` and
/// `lines_order`.
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
            // an array of whole numbers: how many, and what each is
            if let Some(length) = s.length {
                row.insert("length".into(), json!(length));
            }
            if !s.labels.is_empty() {
                row.insert("labels".into(), json!(s.labels));
            }
            // layered objectives of names other than the objectives, and without tolerances
            if !s.items.is_empty() {
                let items: Vec<Value> = s.items.iter().map(|r| json!({"name": r.name, "what": r.what})).collect();
                row.insert("items".into(), Value::Array(items));
            }
            if !s.tolerance {
                row.insert("tolerance".into(), json!(false));
            }
            Value::Object(row)
        })
        .collect();
    let applies = |row: &mut Map<String, Value>, w: rnl_solver::When| {
        if !w.is_empty() {
            row.insert("applies".into(), when(w));
        }
    };
    let objectives: Vec<Value> = t
        .objectives
        .iter()
        .map(|o| {
            let mut row = Map::new();
            row.insert("name".into(), json!(o.name));
            row.insert("what".into(), json!(o.what));
            row.insert("implemented".into(), json!(o.implemented));
            applies(&mut row, o.applies);
            Value::Object(row)
        })
        .collect();
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
    let stages: Vec<Value> = crate::progress::STAGES
        .iter()
        .map(|(name, what)| {
            let mut row = Map::new();
            row.insert("name".into(), json!(name));
            row.insert("what".into(), json!(what));
            applies(&mut row, rnl_solver::stage_applies(name));
            Value::Object(row)
        })
        .collect();
    let ranks: Vec<Value> =
        t.lines_order.iter().map(|r| json!({"name": r.name, "what": r.what, "fixed": r.fixed})).collect();
    json!({
        "version": t.version,
        "groups": groups,
        "settings": settings,
        "objectives": objectives,
        "default_objectives": layers,
        "stages": stages,
        "lines_order": {"what": rnl_solver::LINES_ORDER_WHAT, "ranks": ranks},
    })
}

/// The layouts, as the table's `layout` setting lists them, its default first.
pub fn layouts() -> Vec<&'static str> {
    let t = rnl_solver::settings_table();
    let row = t.settings.iter().find(|s| s.key == "layout").expect("the table has a layout setting");
    let default = match &row.default {
        Some(rnl_runtime::interp::obj::Lit::Str(s)) => s.as_str().to_string(),
        _ => String::new(),
    };
    let mut out: Vec<&'static str> = row.choices.iter().map(|c| c.value).collect();
    out.sort_by_key(|l| *l != default);
    out
}

/// Whether a setting with the key `key` is used, the value of each setting being `value`: its
/// row's `applies` and `shown_when` each hold for it, every key they name having one of the
/// values listed. A key the table does not list is taken as used.
pub fn used(key: &str, value: &dyn Fn(&str) -> Option<String>) -> bool {
    let t = rnl_solver::settings_table();
    let Some(row) = t.settings.iter().find(|s| s.key == key) else { return true };
    [row.applies, row.shown_when]
        .iter()
        .all(|w| w.iter().all(|(k, values)| value(k).is_none_or(|v| values.contains(&v.as_str()))))
}

/// A setting's value as `applies` and `shown_when` name it: a string as it is, another as JSON.
pub fn shown_value(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// The defaults of the solver that the layout `layout` uses, as RNL named parameters, its layout
/// first: those of [`rnl_solver::SolverOptions::defaults_config`], without the ones the table says
/// apply to other layouts or are shown only for other values of the defaults, such as
/// `safety_seconds` under the default `limit`.
pub fn defaults_for(layout: &str) -> String {
    let defaults = rnl_runtime::solver_settings::parse_overrides(&rnl_solver::SolverOptions::defaults_config())
        .unwrap_or_default();
    let value = |key: &str| -> Option<String> {
        if key == "layout" {
            return Some(layout.to_string());
        }
        defaults.iter().find(|s| s.key == key).map(|s| shown_value(&crate::solve::setting_json(&s.value)))
    };
    let mut items = vec![format!("layout=\"{layout}\"")];
    for s in defaults.iter().filter(|s| s.key != "layout" && used(&s.key, &value)) {
        items.push(format!("{}={}", s.key, s.value.show()));
    }
    items.join(", ")
}

/// What `rnl solver-defaults` prints: for each layout, the default first, or for `layout` alone,
/// a line of the defaults it uses ([`defaults_for`]).
pub fn defaults_text(layout: Option<&str>) -> String {
    match layout {
        Some(layout) => defaults_for(layout),
        None => layouts().into_iter().map(defaults_for).collect::<Vec<_>>().join(
            "
",
        ),
    }
}

/// The table as [`settings_table_json`] gives it, for `layout` alone when one is given: its
/// settings, objectives and stages that apply to that layout, the order of the lines layout only
/// for it, and `layout`, the layout it is for.
pub fn settings_table_json_for(layout: Option<&str>) -> Value {
    let mut t = settings_table_json();
    let Some(layout) = layout else { return t };
    let applies = |row: &Value| {
        row.get("applies")
            .and_then(|a| a.get("layout"))
            .and_then(Value::as_array)
            .is_none_or(|layouts| layouts.iter().any(|l| l == layout))
    };
    for list in ["settings", "objectives", "stages"] {
        if let Some(rows) = t[list].as_array_mut() {
            rows.retain(applies);
        }
    }
    let map = t.as_object_mut().unwrap();
    if layout != "lines" {
        map.remove("lines_order");
    }
    map.insert("layout".into(), json!(layout));
    t
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
        assert_eq!(
            (setting("join_trees")["type"].clone(), setting("join_trees")["default"].clone()),
            (json!("boolean"), json!(false))
        );
        assert_eq!(setting("join_trees")["in_file"], true);
        assert_eq!(setting("objectives")["default"], rnl_solver::DEFAULT_LAYERS.join(","));
        // an array of whole numbers says how many, and what each is
        let weights = setting("lines_weights");
        assert_eq!((weights["type"].clone(), weights["default"].clone()), (json!("integers"), json!([1, 4])));
        assert_eq!((weights["length"].clone(), weights["labels"].clone()), (json!(2), json!(["crossing", "bend"])));
        assert_eq!((weights["min"].clone(), weights["max"].clone()), (json!(0), json!(1000)));
        assert_eq!(weights["applies"], json!({"layout": ["lines"]}));
        assert!(setting("beam").get("length").is_none() && setting("beam").get("labels").is_none());
        assert_eq!(t["objectives"].as_array().unwrap().len(), rnl_solver::OBJECTIVES.len());
        assert_eq!(
            t["default_objectives"][0],
            json!({"items": [{"name": "current_direction", "weight": 1}], "tolerance": 0})
        );
        let objective =
            |name: &str| t["objectives"].as_array().unwrap().iter().find(|o| o["name"] == name).unwrap().clone();
        assert!(objective("crowding").get("applies").is_none());
        assert_eq!(objective("turns")["applies"], json!({"layout": ["rows", "tree"]}));
        let stage = |name: &str| t["stages"].as_array().unwrap().iter().find(|s| s["name"] == name).unwrap().clone();
        assert_eq!(stage("search")["applies"], json!({"layout": ["rows"]}));
        assert_eq!(stage("lines")["applies"], json!({"layout": ["lines"]}));
        assert!(stage("resolve").get("applies").is_none());
        assert_eq!(t["lines_order"]["ranks"].as_array().unwrap().len(), rnl_solver::LINES_ORDER.len());
        assert_eq!(
            t["lines_order"]["ranks"][0],
            json!({"name": "complete", "what": rnl_solver::LINES_ORDER[0].what, "fixed": true})
        );
        // the setting lines_order offers the ranks not fixed, in their order, and no tolerance
        let order = setting("lines_order");
        assert_eq!(order["type"], json!("objectives"));
        assert_eq!(order["default"], json!(rnl_solver::SolverOptions::default().settings.lines_order.canonical()));
        assert_eq!(order["tolerance"], json!(false));
        assert_eq!(order["applies"], json!({"layout": ["lines"]}));
        let movable: Vec<Value> = t["lines_order"]["ranks"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["fixed"] == false)
            .map(|r| json!({"name": r["name"], "what": r["what"]}))
            .collect();
        assert_eq!(order["items"], Value::Array(movable));
        assert!(setting("objectives").get("items").is_none() && setting("objectives").get("tolerance").is_none());
        assert!(t["lines_order"]["what"].is_string());
        let stages: Vec<&str> = t["stages"].as_array().unwrap().iter().map(|s| s["name"].as_str().unwrap()).collect();
        assert_eq!(stages, crate::progress::STAGES.iter().map(|(n, _)| *n).collect::<Vec<_>>());
        let groups: Vec<&str> = t["groups"].as_array().unwrap().iter().map(|g| g["name"].as_str().unwrap()).collect();
        assert!(t["settings"].as_array().unwrap().iter().all(|s| groups.contains(&s["group"].as_str().unwrap())));
    }
}
