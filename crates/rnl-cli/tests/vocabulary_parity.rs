//! Published 0.3 vocabulary versus the signatures actually used by the interpreter.
//! Handler-owned defaults and cross-parameter rules are checked through execution below.
//! This is vocabulary parity, not exhaustive semantic conformance.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::interp::unit::READ_VERSIONS;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::words::{draft_header_fields, words, Literal, ParamSpec, ParamType, ParamsSpec, Positions, Takes};
use rnl_runtime::{Interpretation, Options};
use serde_json::{json, Value};

fn published() -> Value {
    serde_json::from_str(include_str!("../../../spec/rnl-words-0.3.json")).unwrap()
}

// Every field is accounted for. These are the only prose-only exemptions: descriptions,
// human-readable form/sentence templates (with ellipses), and explanations of keys/array items.
// They cannot be compared as machine signatures; their rules have separate behavior regressions.
// A new structured field must acquire a comparison or executable check, not disappear here.
fn keys(value: &Value, allowed: &[&str]) {
    for key in value.as_object().unwrap().keys() {
        assert!(allowed.contains(&key.as_str()), "unreviewed vocabulary field {key}: {value}");
    }
}

fn names(items: &Value) -> BTreeSet<String> {
    items.as_array().unwrap().iter().map(|v| v["name"].as_str().unwrap().to_string()).collect()
}

fn strings(items: &[String]) -> BTreeSet<String> {
    items.iter().cloned().collect()
}

fn ty(ty: &ParamType) -> (String, Value) {
    let name = match ty {
        ParamType::Number => "number",
        ParamType::String => "string",
        ParamType::Bool => "bool",
        ParamType::Array => "array",
        ParamType::Term => "terminal",
        ParamType::Color => "color",
        ParamType::Any => "any",
        ParamType::Enum(values) => {
            return (
                "enum".into(),
                Value::Array(
                    values
                        .iter()
                        .map(|v| match v {
                            Literal::Str(s) => json!(s),
                            Literal::Num(n) => json!(n),
                        })
                        .collect(),
                ),
            );
        }
    };
    (name.into(), Value::Null)
}

fn number_normalized(value: &Value) -> Value {
    // The runtime stores enum numbers as f64; JSON distinguishes 0 from 0.0 syntactically.
    match value {
        Value::Number(n) => json!(n.as_f64().unwrap()),
        Value::Array(a) => Value::Array(a.iter().map(number_normalized).collect()),
        _ => value.clone(),
    }
}

fn parameter(actual: &ParamSpec, expected: &Value) {
    keys(expected, &["name", "type", "values", "required", "default", "items", "since"]);
    assert_eq!(actual.name, expected["name"].as_str().unwrap());
    let (kind, values) = ty(&actual.ty);
    assert_eq!(kind, expected["type"].as_str().unwrap(), "{} type", actual.name);
    assert_eq!(values, number_normalized(&expected["values"]), "{} enum", actual.name);
    assert_eq!(actual.required, expected["required"].as_bool().unwrap_or(false), "{} required", actual.name);
    // All fixed parameter words belong to the embedded 0.2 vocabulary. The supported 0.3
    // additions are the header supplement below, not a silently ignored `since` field.
    assert_eq!(expected["since"], words().language_version, "{} version", actual.name);
}

#[test]
fn runtime_signatures_match_every_published_word_and_structured_field() {
    let p = published();
    keys(
        &p,
        &[
            "$schema",
            "language",
            "version",
            "description",
            "record",
            "kinds",
            "pseudo_kinds",
            "takes",
            "subobjects",
            "attributes",
            "truth_values",
            "style_properties",
            "style_groups",
            "reserved",
            "keywords",
        ],
    );
    assert_eq!(p["$schema"], "rnl-words.schema.json");
    assert_eq!(p["language"], "RNL");
    assert!(READ_VERSIONS.contains(&p["version"].as_str().unwrap()));
    let w = words();
    for (category, actual) in [("kinds", &w.kinds), ("subobjects", &w.subobjects), ("attributes", &w.attributes)] {
        assert_eq!(strings(actual), names(&p[category]), "{category}");
        for item in p[category].as_array().unwrap() {
            keys(item, &["name", "since"]);
            assert_eq!(item["since"], w.language_version, "{category}: {item}");
        }
    }
    let record = &p["record"];
    keys(record, &["marker", "section", "fields"]);
    // Exercise the marker as syntax, not a duplicated expected string.
    assert!(rnl_runtime::syntax::parse_record(&format!("{} {{}}", record["marker"].as_str().unwrap())).error.is_none());
    assert_eq!(record["section"], "2.8");
    let headers: BTreeMap<_, _> =
        w.header_fields.iter().chain(draft_header_fields()).map(|f| (f.name.as_str(), f)).collect();
    assert_eq!(headers.len(), record["fields"].as_array().unwrap().len());
    for field in record["fields"].as_array().unwrap() {
        keys(field, &["name", "type", "values", "default", "affects_interpretation", "since"]);
        let name = field["name"].as_str().unwrap();
        let actual = headers.get(name).unwrap_or_else(|| panic!("missing header field {name}"));
        assert_eq!(field["type"], if actual.values.is_some() { "enum" } else { "string" });
        assert_eq!(field["values"], json!(actual.values));
        assert_eq!(field["affects_interpretation"], actual.affects);
        let since = if w.header_field(name).is_some() { &w.language_version } else { p["version"].as_str().unwrap() };
        assert_eq!(field["since"], since);
    }
    assert_eq!(w.style_props.len(), p["style_properties"].as_array().unwrap().len());
    for prop in p["style_properties"].as_array().unwrap() {
        let actual = w.style_prop(prop["name"].as_str().unwrap()).unwrap();
        parameter(&ParamSpec { name: actual.name.clone(), ty: actual.ty.clone(), required: false }, prop);
    }
    let groups: BTreeMap<_, _> = w.groups.iter().map(|(k, v)| (k.strip_suffix("_style").unwrap(), v)).collect();
    assert_eq!(json!(groups), p["style_groups"]);
    assert_eq!(w.keywords.len(), p["keywords"].as_array().unwrap().len());
    let mut pseudo = BTreeSet::new();
    let mut takes = BTreeSet::new();
    let mut all_words = BTreeSet::new();
    for k in p["keywords"].as_array().unwrap() {
        keys(
            k,
            &[
                "keyword",
                "since",
                "group",
                "summary",
                "section",
                "where",
                "slots",
                "role_slots",
                "params",
                "definition_params",
                "style",
                "positional",
                "at_least_one",
                "together",
                "exclusive",
                "shape_params",
                "output",
                "output_kind",
                "rewrites",
                "selects",
                "key_subobject",
                "override",
                "key",
                "form",
                "sentence",
            ],
        );
        let name = k["keyword"].as_str().unwrap();
        let a = w.keyword(name).unwrap_or_else(|| panic!("missing keyword {name}"));
        assert_eq!(k["since"], w.language_version, "{name} since");
        assert_eq!(k["section"], a.section, "{name} section");
        assert_eq!(k["where"], json!(a.where_), "{name} contexts");
        assert_eq!(k.get("positional").cloned().unwrap_or(json!([])), json!(a.positional), "{name} positional");
        assert_eq!(k.get("at_least_one").cloned().unwrap_or(json!([])), json!(a.at_least_one), "{name} at_least_one");
        assert_eq!(k["override"], json!(a.override_), "{name} override");
        let dynamic = matches!(a.params, ParamsSpec::Definition);
        assert_eq!(k["definition_params"].as_bool().unwrap_or(false), dynamic, "{name} definition params");
        let params = if dynamic { w.usage_params() } else { a.param_list().to_vec() };
        assert_eq!(params.len(), k["params"].as_array().unwrap().len(), "{name} params");
        for e in k["params"].as_array().unwrap() {
            parameter(params.iter().find(|v| e["name"] == v.name).unwrap(), e);
            all_words.insert(e["name"].as_str().unwrap().to_string());
            for v in e["values"].as_array().into_iter().flatten().filter_map(Value::as_str) {
                all_words.insert(v.into());
            }
        }
        let style = if matches!(a.params, ParamsSpec::Style) {
            Some("all")
        } else {
            a.style.as_deref().map(|s| s.strip_suffix("_style").unwrap())
        };
        assert_eq!(k["style"], json!(style), "{name} style");
        let mut slots = Vec::new();
        if matches!(a.positions, Positions::Definition) {
            slots.push(json!({"name":"definition", "takes":"definition", "kinds":["general"], "mode":"absorb", "at":a.definition_at}));
        } else {
            assert!(a.definition_at.is_none());
            for s in a.position_list() {
                let (take, kinds) = match &s.takes {
                    Takes::One => ("one", s.kinds.clone()),
                    Takes::Set => ("set", s.kinds.clone()),
                    Takes::Any => ("any", s.kinds.clone()),
                    Takes::Shape => ("shape", s.kinds.clone()),
                    Takes::Body => ("body", s.kinds.clone()),
                    Takes::Def(kind) => {
                        ("definition", vec![if kind == "definition" { "general".into() } else { kind.clone() }])
                    }
                };
                let mut slot =
                    json!({"name":s.name, "takes":take, "mode":if take == "body" {"enclose"} else {"absorb"}});
                if !kinds.is_empty() {
                    slot["kinds"] = json!(kinds);
                }
                if s.optional {
                    slot["optional"] = json!(true);
                }
                slots.push(slot);
            }
        }
        assert_eq!(k["role_slots"].as_bool().unwrap_or(false), matches!(a.positions, Positions::Definition));
        assert_eq!(json!(slots), k["slots"], "{name} slots");
        for slot in &slots {
            keys(slot, &["name", "takes", "kinds", "mode", "optional", "at"]);
            takes.insert(slot["takes"].as_str().unwrap().to_string());
            all_words.insert(slot["name"].as_str().unwrap().to_string());
            if slot["takes"] != "definition" {
                for kind in slot["kinds"].as_array().into_iter().flatten().filter_map(Value::as_str) {
                    if !w.kinds.iter().any(|k| k == kind) && kind != "style" {
                        pseudo.insert(kind.to_string());
                    }
                }
            }
        }
        let output = match k["output"].as_str().unwrap() {
            "none" => "none".to_string(),
            "same" => "pass".into(),
            "target" => "pass-target".into(),
            "definition" => format!(
                "def:{}",
                if k["output_kind"] == "general" { "definition" } else { k["output_kind"].as_str().unwrap() }
            ),
            "new" if k.get("selects").is_some() => {
                assert_eq!(k["output_kind"], "set");
                format!("new:select:{}", k["selects"].as_str().unwrap())
            }
            "new" => format!("new:{}", k["output_kind"].as_str().unwrap()),
            other => panic!("unhandled output {other}"),
        };
        assert_eq!(a.output, output, "{name} output");
    }
    assert_eq!(pseudo, names(&p["pseudo_kinds"]));
    assert_eq!(takes, names(&p["takes"]));
    for list in ["pseudo_kinds", "takes"] {
        for item in p[list].as_array().unwrap() {
            keys(item, &["name", "meaning"]);
        }
    }
    for category in ["kinds", "subobjects", "attributes", "style_properties"] {
        all_words.extend(names(&p[category]));
        for item in p[category].as_array().unwrap() {
            for v in item["values"].as_array().into_iter().flatten().filter_map(Value::as_str) {
                all_words.insert(v.into());
            }
        }
    }
    for truth in p["truth_values"].as_array().unwrap() {
        let name = truth.as_str().unwrap();
        let ast = rnl_runtime::syntax::parse_record(&format!("@RNL {{ @TEXT(text={name}) }}"));
        let body = ast.body.unwrap();
        let value = &body.members[0].segments[0].call.as_ref().unwrap().params.as_ref().unwrap().named[0].value;
        assert!(matches!(value, rnl_runtime::syntax::Value::Bool(_)), "{truth} must parse as a boolean");
        all_words.insert(name.into());
    }
    all_words.extend(p["reserved"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()));
    assert_eq!(&all_words, w.all_words(), "all standard/reserved words");
}

fn interpret(body: &str, head: &str) -> Interpretation {
    let text = format!("* vocabulary parity\n* @RNL(type=\"header\") {{ @VERSION(\"0.3\") }}\nR1 a b 1k\n* @RNL{head} {{ {body} }}\n.end\n");
    read(&text)
}

fn read(text: &str) -> Interpretation {
    let files = MemoryProvider::new().with("v.sp", text);
    interpret_file(Path::new("v.sp"), None, &files, &Options { std: false, problems: true, ..Options::default() })
        .unwrap()
}

fn no_errors(r: &Interpretation) {
    let errors: Vec<_> =
        r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).map(|d| r.sink.render(d)).collect();
    assert!(errors.is_empty(), "{errors:?}");
}

fn view(r: &Interpretation) -> String {
    no_errors(r);
    // Explicit default style properties and omitted properties differ in source/model metadata,
    // so compare their rendered result, plus nonvisual wire/route/priority semantics.
    format!(
        "{:?}|{:?}|{:?}|{:?}",
        r.model.levels.iter().map(|l| rnl_render::primitives(l, &Default::default())).collect::<Vec<_>>(),
        r.model.levels.iter().flat_map(|l| l.wires.iter().map(|w| &w.mode)).collect::<Vec<_>>(),
        r.problems.iter().flat_map(|p| p.routes.iter().map(|r| &r.mode)).collect::<Vec<_>>(),
        r.model.levels.iter().flat_map(|l| l.constraints.iter().map(|c| (&c.kind, &c.priority))).collect::<Vec<_>>()
    )
}

fn graphic(keyword: &str, params: &str) -> String {
    let points = match keyword {
        "LINE" => "{(0,0) (4,3)}",
        "ARC" => "{(0,0) (2,3) (4,0)}",
        "BEZIER" => "{(0,0) (1,3) (3,3) (4,0)}",
        "RECTANGLE" | "ELLIPSE" | "CIRCLE" => "{(0,0) (4,3)}",
        _ => panic!("add a default/shape probe for {keyword}"),
    };
    format!("{points} @{keyword}({params})")
}

fn default_body(keyword: &str, param: &str) -> String {
    match keyword {
        "LINE" | "RECTANGLE" | "CIRCLE" | "ELLIPSE" | "ARC" | "BEZIER" => graphic(keyword, param),
        "TEXT" => format!("(2,3) @TEXT(text=\"wide text\"{comma}{param})", comma=if param.is_empty() {""} else {","}),
        "WIRE" => format!("&a @WIRE({param}) {{(0,0) (5,3)}}"),
        "ROUTE" => format!("$S = @DEFINE-SYMBOL {{ (0,0) @PIN(terminal=1); (2,2) @PIN(terminal=2) }}; #R1 @SET-SYMBOL $S => @PLACE(0,0); &a @ROUTE({param})"),
        _ => panic!("unhandled default on {keyword}"),
    }
}

fn selection(keyword: &str, parameter: &str) -> Interpretation {
    // Give the created usage its symbol directly. Its main object is #R1, so it is
    // anchored there and must not receive an independent PLACE (12.1).
    // Ancestor selection uses a recursive inner selection: a direct #R1 reference
    // resolves only in its own level and would mask the recursive=false behavior.
    let definitions = "$S = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1); (2,2) @PIN(terminal=2) }; $BOX = @DEFINE-SYMBOL { (0,0) @RECTANGLE(width=2,height=2) }; $U = @DEFINE(roles=[\"r\"]) {}";
    let setup = "#R1 @SET-SYMBOL $S => @PLACE(0,0); %u = #R1 @AS $U => @SET-SYMBOL $BOX; #R1 @TEXT(\"label\",key=\"label\"); (2,2) @CIRCLE(radius=1); &a @WIRE {#R1:1 (5,0)}";
    let (scope, definition, action) = match keyword {
        "SELECT-INSTANCE" | "SELECT-TERMINAL" | "SELECT-NET" | "SELECT-WIRE" => {
            ("#", "", "@TEXT(\"selected\",key=\"selected\")")
        }
        "SELECT-LABEL" | "SELECT-ANNOTATION" => ("#", "", "@LAYER(\"top\")"),
        "SELECT-USAGE" => ("#", "$U", "@LAYER(\"top\")"),
        "SELECT-ANCESTOR-USAGE" => ("{# @SELECT-INSTANCE}", "$U", "@LAYER(\"top\")"),
        _ => panic!("unhandled recursive default {keyword}"),
    };
    read(&format!("* recursive\n* @RNL(type=\"header\") {{ @VERSION(\"0.3\") }}\n* @RNL {{ {definitions} }}\n.subckt cell a b\nR1 a b 1k\n* @RNL {{ {setup} }}\n.ends cell\nR1 a b 1k\nX1 a b cell\n* @RNL {{ {setup}; {scope} @{keyword}({parameter}) {definition} => {action} }}\n.end\n"))
}

#[test]
fn every_published_default_matches_omission_in_the_running_toolkit() {
    let p = published();
    for keyword in p["keywords"].as_array().unwrap() {
        let name = keyword["keyword"].as_str().unwrap();
        for param in keyword["params"].as_array().unwrap() {
            let Some(default) = param.get("default") else { continue };
            let key = param["name"].as_str().unwrap();
            let explicit = format!("{key}={default}");
            if key == "recursive" {
                let omitted = view(&selection(name, ""));
                assert_eq!(omitted, view(&selection(name, &explicit)), "{name}.{key} default");
                let opposite = format!("{key}={}", !default.as_bool().unwrap());
                assert_ne!(
                    omitted,
                    view(&selection(name, &opposite)),
                    "{name}: probe must observe recursive inheritance"
                );
            } else {
                assert_eq!(
                    view(&interpret(&default_body(name, ""), "")),
                    view(&interpret(&default_body(name, &explicit), "")),
                    "{name}.{key} default"
                );
            }
        }
    }
    for prop in p["style_properties"].as_array().unwrap() {
        if let Some(default) = prop.get("default") {
            let explicit = format!("{}={default}", prop["name"].as_str().unwrap());
            assert_eq!(
                view(&interpret(&graphic("LINE", ""), "")),
                view(&interpret(&graphic("LINE", &explicit), "")),
                "style default {prop}"
            );
        }
    }
    for field in p["record"]["fields"].as_array().unwrap() {
        if let Some(default) = field.get("default") {
            let head = format!("({}={default})", field["name"].as_str().unwrap());
            let body = "#R1 @PLACE(2,3); %p = (4,5); %p #R1 @ALIGN(\"x\")";
            assert_eq!(view(&interpret(body, "")), view(&interpret(body, &head)), "record default {field}");
        }
    }
}

#[test]
fn published_shape_rules_and_keyed_outputs_are_executed_by_the_handlers() {
    for keyword in published()["keywords"].as_array().unwrap() {
        let name = keyword["keyword"].as_str().unwrap();
        if let Some(shape) = keyword.get("shape_params") {
            let params: Vec<_> =
                shape.as_array().unwrap().iter().map(|p| format!("{}=2", p.as_str().unwrap())).collect();
            no_errors(&interpret(&format!("(0,0) @{name}({})", params.join(",")), ""));
            no_errors(&interpret(&graphic(name, ""), ""));
            assert!(
                interpret(&format!("(0,0) @{name}"), "").diagnostics().iter().any(|d| d.severity == Severity::Error),
                "{name}: centre needs dimensions"
            );
        }
        for group in keyword["together"].as_array().into_iter().flatten() {
            for parameter in group.as_array().unwrap() {
                let r = interpret(&format!("(0,0) @{name}({}=2)", parameter.as_str().unwrap()), "");
                assert!(r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{name}: {group}");
            }
        }
        for pair in keyword["exclusive"].as_array().into_iter().flatten() {
            let params: Vec<_> =
                pair.as_array().unwrap().iter().map(|v| format!("{}=2", v.as_str().unwrap())).collect();
            let r = interpret(&format!("(0,0) (4,0) @{name}(direction=\"x\",{})", params.join(",")), "");
            assert!(r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{name}: {pair}");
        }
        if let Some(kind) = keyword.get("rewrites") {
            assert_eq!(kind, "wire", "add a rewrite probe for {name}");
            let r =
                interpret(&format!("%w = &a @WIRE {{(0,0) (4,0)}}; %w @{name}(mode=\"straight\") {{(0,0) (4,3)}}"), "");
            no_errors(&r);
            assert_eq!(r.model.levels[0].wires.len(), 1);
            assert_eq!(r.model.levels[0].wires[0].points, [(0.0, 0.0), (4.0, 3.0)]);
        }
        if let Some(keyed) = keyword.get("key_subobject") {
            assert_eq!(keyed, true, "add a key-subobject probe for {name}");
            let r = interpret(&format!("#R1 @PLACE(0,0); #R1 @{name}(\"old\",key=\"note\"); #R1 @{name}(\"new\",key=\"note\"); #R1::note @PLACE(2,3)"), "");
            no_errors(&r);
            // The default resistor symbol also supplies a name label. Count the key under
            // test rather than treating unrelated symbol labels as duplicate keyed output.
            let notes: Vec<_> =
                r.model.levels[0].texts.iter().filter(|text| text.key.as_deref() == Some("note")).collect();
            assert_eq!(notes.len(), 1);
            assert_eq!(notes[0].object_id, "#R1::note");
            assert_eq!(notes[0].markup, "new");
            assert_eq!(notes[0].anchor, (2.0, 3.0));
            assert!(!r.model.levels[0].texts.iter().any(|text| text.markup == "old"));
        }
    }
}

#[test]
fn corrected_signature_metadata_preserves_existing_dispatch() {
    for version in ["0.2", "0.3"] {
        let interpret = |body: &str| {
            read(&format!("* vocabulary parity\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\n* @RNL {{ {body} }}\n.end\n"))
        };
        // SELECT-USAGE accepts a set of member objects, not only a level/usage. Kinds are
        // descriptive metadata; select_usages already implements this scope in both versions.
        let r = interpret(
            "$D = @DEFINE(roles=[\"r\"]) {}; %u = #R1 @AS $D; {#R1} @SELECT-USAGE $D => @PRIORITY(\"disabled\")",
        );
        no_errors(&r);
        // DEFINE-STYLE has always checked that its enclosed statements produce styles.
        let good = interpret("$base = @DEFINE-STYLE(linecolor=\"#ff0000\") {}; $s = @DEFINE-STYLE {$base}");
        no_errors(&good);
        let bad = interpret("$s = @DEFINE-STYLE {(0,0)}");
        assert!(bad.diagnostics().iter().any(|d| d.severity == Severity::Error), "{version}: non-style body accepted");
    }
}
