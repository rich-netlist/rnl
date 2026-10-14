//! The standard word list (standard 5.3, Z1): every keyword's signature, the record header
//! fields, kind names, sub-object names, attribute names and style properties.
//!
//! It is a data file released with each language version. Checking, the reference documents and
//! the writing brief are derived from it, so adding or changing a keyword means editing the file
//! and adding a handler, not editing tables in code.

use std::collections::{BTreeSet, HashMap};
use std::sync::OnceLock;

use serde_json::Value;

/// The word list of language 0.2, embedded.
pub const WORDS_0_2: &str = include_str!("../data/words-0.2.json");

/// A literal in a list of allowed values: strings, or numbers such as TEXT's `rotate`.
#[derive(Clone, Debug, PartialEq)]
pub enum Literal {
    Str(String),
    Num(f64),
}

#[derive(Clone, Debug, PartialEq)]
pub enum ParamType {
    Number,
    String,
    Bool,
    Array,
    /// A terminal: a number, a name, or an array of them (PIN's bus pins, 9.2.4).
    Term,
    Color,
    Enum(Vec<Literal>),
    /// Anything, for parameters whose check is done by the keyword (a definition's own).
    Any,
}

#[derive(Clone, Debug)]
pub struct ParamSpec {
    pub name: String,
    pub ty: ParamType,
    pub required: bool,
}

/// What an object position takes (5.2.1).
#[derive(Clone, Debug, PartialEq)]
pub enum Takes {
    /// A single object; a set is done member by member.
    One,
    /// A set, taken whole.
    Set,
    /// A point (centre form) or a set of points (two-point form) of a shape (9.2.5).
    Shape,
    /// An object or a set, taken whole.
    Any,
    /// A definition body or a package body (encapsulated, 5.4).
    Body,
    /// A definition of the given kind: "symbol", "style" or "definition".
    Def(String),
}

#[derive(Clone, Debug)]
pub struct PositionSpec {
    pub name: String,
    pub takes: Takes,
    pub kinds: Vec<String>,
    pub optional: bool,
}

#[derive(Clone, Debug)]
pub enum Positions {
    List(Vec<PositionSpec>),
    /// AS, ATTACH: the roles of the definition they apply (9.4).
    Definition,
}

#[derive(Clone, Debug)]
pub enum ParamsSpec {
    List(Vec<ParamSpec>),
    /// DEFINE-STYLE: the style properties (9.3).
    Style,
    /// AS, ATTACH: the parameters of the definition, plus `priority` (9.4, 14.8).
    Definition,
}

#[derive(Clone, Debug)]
pub struct KeywordSig {
    pub name: String,
    pub where_: Vec<String>,
    pub positions: Positions,
    /// AS: "last"; ATTACH: "first" (4.2 rule 7).
    pub definition_at: Option<String>,
    pub params: ParamsSpec,
    /// The parameters that may be written by position, in order (4.2 rule 9).
    pub positional: Vec<String>,
    pub at_least_one: Vec<String>,
    /// The group of style properties written as parameters: line_style, area_style, wire_style, text_style.
    pub style: Option<String>,
    pub output: String,
    pub override_: Option<String>,
    pub phase: Option<u8>,
    pub section: String,
}

impl KeywordSig {
    pub fn param_list(&self) -> &[ParamSpec] {
        match &self.params {
            ParamsSpec::List(v) => v,
            _ => &[],
        }
    }

    pub fn position_list(&self) -> &[PositionSpec] {
        match &self.positions {
            Positions::List(v) => v,
            Positions::Definition => &[],
        }
    }
}

#[derive(Clone, Debug)]
pub struct HeaderField {
    pub name: String,
    pub values: Option<Vec<String>>,
    pub affects: bool,
}

#[derive(Clone, Debug)]
pub struct StyleProp {
    pub name: String,
    pub ty: ParamType,
}

#[derive(Debug)]
pub struct Words {
    pub language_version: String,
    pub header_fields: Vec<HeaderField>,
    pub kinds: Vec<String>,
    pub subobjects: Vec<String>,
    pub attributes: Vec<String>,
    pub style_props: Vec<StyleProp>,
    pub groups: HashMap<String, Vec<String>>,
    pub keywords: Vec<KeywordSig>,
    by_name: HashMap<String, usize>,
    all: BTreeSet<String>,
}

impl Words {
    pub fn parse(text: &str) -> Result<Words, String> {
        let v: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let obj = v.as_object().ok_or("the word list is not an object")?;
        let language_version = str_of(obj.get("language_version")).ok_or("language_version")?;
        let mut header_fields = Vec::new();
        for (name, spec) in obj.get("header_fields").and_then(Value::as_object).ok_or("header_fields")? {
            header_fields.push(HeaderField {
                name: name.clone(),
                values: spec
                    .get("values")
                    .and_then(Value::as_array)
                    .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()),
                affects: spec.get("affects").and_then(Value::as_bool).unwrap_or(false),
            });
        }
        let strings = |key: &str| -> Result<Vec<String>, String> {
            Ok(obj
                .get(key)
                .and_then(Value::as_array)
                .ok_or_else(|| key.to_string())?
                .iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect())
        };
        let kinds = strings("kinds")?;
        let subobjects = strings("subobjects")?;
        let attributes = strings("attributes")?;
        let mut style_props = Vec::new();
        for (name, spec) in obj.get("style_props").and_then(Value::as_object).ok_or("style_props")? {
            style_props.push(StyleProp { name: name.clone(), ty: param_type(spec) });
        }
        let mut groups = HashMap::new();
        for g in ["line_style", "area_style", "wire_style", "text_style"] {
            groups.insert(g.to_string(), strings(g)?);
        }
        let mut keywords = Vec::new();
        for (name, spec) in obj.get("keywords").and_then(Value::as_object).ok_or("keywords")? {
            keywords.push(keyword(name, spec)?);
        }
        let by_name = keywords.iter().enumerate().map(|(i, k)| (k.name.clone(), i)).collect();
        let mut w = Words {
            language_version,
            header_fields,
            kinds,
            subobjects,
            attributes,
            style_props,
            groups,
            keywords,
            by_name,
            all: BTreeSet::new(),
        };
        w.all = w.collect_all();
        Ok(w)
    }

    pub fn keyword(&self, name: &str) -> Option<&KeywordSig> {
        self.by_name.get(name).map(|&i| &self.keywords[i])
    }

    pub fn is_keyword(&self, name: &str) -> bool {
        self.by_name.contains_key(name)
    }

    pub fn header_field(&self, name: &str) -> Option<&HeaderField> {
        self.header_fields.iter().find(|h| h.name == name)
    }

    pub fn style_prop(&self, name: &str) -> Option<&StyleProp> {
        self.style_props.iter().find(|s| s.name == name)
    }

    pub fn is_style_prop(&self, name: &str) -> bool {
        self.style_prop(name).is_some()
    }

    pub fn group(&self, name: &str) -> &[String] {
        self.groups.get(name).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn is_subobject(&self, name: &str) -> bool {
        self.subobjects.iter().any(|s| s == name)
    }

    /// Every standard word other than the keywords (16.5): parameter names and values, position
    /// names, style properties, kinds, sub-object names, attribute names, `true` and `false`.
    pub fn all_words(&self) -> &BTreeSet<String> {
        &self.all
    }

    fn collect_all(&self) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        for k in &self.keywords {
            for p in k.param_list() {
                out.insert(p.name.clone());
                if let ParamType::Enum(vals) = &p.ty {
                    for v in vals {
                        if let Literal::Str(s) = v {
                            out.insert(s.clone());
                        }
                    }
                }
            }
            for p in k.position_list() {
                out.insert(p.name.clone());
            }
        }
        for s in &self.style_props {
            out.insert(s.name.clone());
            if let ParamType::Enum(vals) = &s.ty {
                for v in vals {
                    if let Literal::Str(x) = v {
                        out.insert(x.clone());
                    }
                }
            }
        }
        out.extend(self.kinds.iter().cloned());
        out.extend(self.subobjects.iter().cloned());
        out.extend(self.attributes.iter().cloned());
        out.insert("true".into());
        out.insert("false".into());
        out
    }
}

fn str_of(v: Option<&Value>) -> Option<String> {
    v.and_then(Value::as_str).map(String::from)
}

fn param_type(spec: &Value) -> ParamType {
    if let Some(vals) = spec.get("values").and_then(Value::as_array) {
        return ParamType::Enum(
            vals.iter()
                .filter_map(|x| match x {
                    Value::String(s) => Some(Literal::Str(s.clone())),
                    Value::Number(n) => n.as_f64().map(Literal::Num),
                    _ => None,
                })
                .collect(),
        );
    }
    match spec.get("type").and_then(Value::as_str) {
        Some("number") => ParamType::Number,
        Some("string") => ParamType::String,
        Some("bool") => ParamType::Bool,
        Some("array") => ParamType::Array,
        Some("term") => ParamType::Term,
        Some("color") => ParamType::Color,
        _ => ParamType::Any,
    }
}

fn keyword(name: &str, spec: &Value) -> Result<KeywordSig, String> {
    let positions = match spec.get("positions") {
        Some(Value::String(s)) if s == "definition" => Positions::Definition,
        Some(Value::Array(items)) => Positions::List(
            items
                .iter()
                .map(|p| {
                    let takes = match p.get("takes").and_then(Value::as_str).unwrap_or("one") {
                        "one" => Takes::One,
                        "set" => Takes::Set,
                        "shape" => Takes::Shape,
                        "any" => Takes::Any,
                        "body" => Takes::Body,
                        other => match other.strip_prefix("def:") {
                            Some(kind) => Takes::Def(kind.to_string()),
                            None => Takes::Any,
                        },
                    };
                    PositionSpec {
                        name: str_of(p.get("name")).unwrap_or_default(),
                        takes,
                        kinds: p
                            .get("kinds")
                            .and_then(Value::as_array)
                            .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                            .unwrap_or_default(),
                        optional: p.get("optional").and_then(Value::as_bool).unwrap_or(false),
                    }
                })
                .collect(),
        ),
        _ => return Err(format!("{name}: positions")),
    };
    let params = match spec.get("params") {
        Some(Value::String(s)) if s == "style" => ParamsSpec::Style,
        Some(Value::String(s)) if s == "definition" => ParamsSpec::Definition,
        Some(Value::Array(items)) => ParamsSpec::List(
            items
                .iter()
                .map(|p| ParamSpec {
                    name: str_of(p.get("name")).unwrap_or_default(),
                    ty: param_type(p),
                    required: p.get("required").and_then(Value::as_bool).unwrap_or(false),
                })
                .collect(),
        ),
        _ => return Err(format!("{name}: params")),
    };
    let list = |key: &str| -> Vec<String> {
        spec.get(key)
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default()
    };
    Ok(KeywordSig {
        name: name.to_string(),
        where_: list("where"),
        positions,
        definition_at: str_of(spec.get("definition_at")),
        params,
        positional: list("positional"),
        at_least_one: list("at_least_one"),
        style: str_of(spec.get("style")),
        output: str_of(spec.get("output")).unwrap_or_default(),
        override_: str_of(spec.get("override")),
        phase: spec.get("phase").and_then(Value::as_u64).map(|p| p as u8),
        section: str_of(spec.get("section")).unwrap_or_default(),
    })
}

/// The word list of the language version this runtime supports.
pub fn words() -> &'static Words {
    static W: OnceLock<Words> = OnceLock::new();
    W.get_or_init(|| Words::parse(WORDS_0_2).expect("the embedded word list is valid"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_list_has_46_keywords() {
        let w = words();
        assert_eq!(w.language_version, "0.2");
        assert_eq!(w.keywords.len(), 46);
        let place = w.keyword("PLACE").unwrap();
        assert_eq!(place.positional, vec!["x", "y"]);
        assert!(matches!(w.keyword("AS").unwrap().positions, Positions::Definition));
        assert!(w.all_words().contains("left-to-right"));
        assert!(w.all_words().contains("position"));
        assert!(!w.all_words().contains("PLACE"));
    }
}
