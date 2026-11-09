//! The standard word list (standard 5.3, Z1): every keyword's signature, the record header
//! fields, kind names, sub-object names, attribute names and style properties.
//!
//! It is a data file released with each language version. Checking, the reference documents and
//! the writing brief are derived from it, so adding or changing a keyword means editing the file
//! and adding a handler, not editing tables in code.
//!
//! One list serves every version read (16.1): a keyword or a parameter that a version adds carries
//! `since`, one that a version removes carries `until`, the last version that has it, and the list
//! `removed` names each removed word with the section that replaces it. A word is in a version from
//! its `since` (0.2 when not written) through its `until`.

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
    /// The version that adds it, none for 0.2.
    pub since: Option<String>,
    /// The last version that has it, none while it is kept.
    pub until: Option<String>,
}

impl ParamSpec {
    /// An optional parameter of every version.
    pub fn new(name: impl Into<String>, ty: ParamType) -> ParamSpec {
        ParamSpec { name: name.into(), ty, required: false, since: None, until: None }
    }

    /// Whether a file of the language version `version` has this parameter.
    pub fn in_version(&self, version: &str) -> bool {
        in_version(&self.since, &self.until, version)
    }
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
    /// Groups of parameters of which each needs at least one written (4.2), such as DISTANCE's
    /// `min`, `max` and `exact`.
    pub at_least_one: Vec<Vec<String>>,
    /// Whether it takes parameters beyond its list, which the keyword checks itself (SOLVER-CONFIG).
    pub open_params: bool,
    /// The version that adds it, none for 0.2.
    pub since: Option<String>,
    /// The last version that has it, none while it is kept.
    pub until: Option<String>,
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

    /// Whether a file of the language version `version` has this keyword.
    pub fn in_version(&self, version: &str) -> bool {
        in_version(&self.since, &self.until, version)
    }

    /// The parameters a file of the language version `version` has.
    pub fn params_in(&self, version: &str) -> Vec<ParamSpec> {
        self.param_list().iter().filter(|p| p.in_version(version)).cloned().collect()
    }
}

/// A word that a version removes (16.1): the word, what it was, the last version that has it, and the
/// section that says what replaces it.
#[derive(Clone, Debug)]
pub struct Removed {
    pub word: String,
    pub what: String,
    pub until: String,
    pub see: String,
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
    pub removed: Vec<Removed>,
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
        let mut removed = Vec::new();
        for r in obj.get("removed").and_then(Value::as_array).into_iter().flatten() {
            let field = |key: &str| str_of(r.get(key)).ok_or_else(|| format!("removed: {key}"));
            removed.push(Removed {
                word: field("word")?,
                what: field("what")?,
                until: field("until")?,
                see: field("see")?,
            });
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
            removed,
            by_name,
            all: BTreeSet::new(),
        };
        w.all = w.collect_all(None);
        Ok(w)
    }

    /// The keyword `name` of any version read; [`Words::keyword_in`] for that of one version.
    pub fn keyword(&self, name: &str) -> Option<&KeywordSig> {
        self.by_name.get(name).map(|&i| &self.keywords[i])
    }

    /// The keyword `name` as a file of the language version `version` has it.
    pub fn keyword_in(&self, name: &str, version: &str) -> Option<&KeywordSig> {
        self.keyword(name).filter(|k| k.in_version(version))
    }

    /// Whether `name` is a keyword in a file of the language version `version`.
    pub fn is_keyword_in(&self, name: &str, version: &str) -> bool {
        self.keyword_in(name, version).is_some()
    }

    /// The keywords a file of the language version `version` has.
    pub fn keywords_in<'a>(&'a self, version: &'a str) -> impl Iterator<Item = &'a KeywordSig> + 'a {
        self.keywords.iter().filter(move |k| k.in_version(version))
    }

    /// The entries of `removed` for `word` that a file of the language version `version` no
    /// longer has, the word removed in that version or an earlier one after its own.
    pub fn removed_in<'a>(&'a self, word: &'a str, version: &'a str) -> impl Iterator<Item = &'a Removed> + 'a {
        self.removed.iter().filter(move |r| r.word == word && !in_version(&None, &Some(r.until.clone()), version))
    }

    /// Fixed parameters accepted by AS/ATTACH in addition to a definition's own parameters.
    pub fn usage_params(&self) -> Vec<ParamSpec> {
        let mut priority = self.keyword("PRIORITY").expect("PRIORITY signature").param_list()[0].clone();
        priority.required = false;
        vec![priority]
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
    /// Those of every version read; [`Words::all_words_in`] for those of one version.
    pub fn all_words(&self) -> &BTreeSet<String> {
        &self.all
    }

    /// The standard words other than the keywords of a file of the language version `version`.
    pub fn all_words_in(&self, version: &str) -> BTreeSet<String> {
        self.collect_all(Some(version))
    }

    fn collect_all(&self, version: Option<&str>) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        for k in self.keywords.iter().filter(|k| version.is_none_or(|v| k.in_version(v))) {
            for p in k.param_list().iter().filter(|p| version.is_none_or(|v| p.in_version(v))) {
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

/// Whether the language version `version` is within `since` (0.2 when none) and `until` (no end
/// when none), by the order of versions (16.1).
fn in_version(since: &Option<String>, until: &Option<String>, version: &str) -> bool {
    let key = crate::interp::unit::version_key;
    since.as_deref().is_none_or(|s| key(version) >= key(s)) && until.as_deref().is_none_or(|u| key(version) <= key(u))
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
                    since: str_of(p.get("since")),
                    until: str_of(p.get("until")),
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
        // one group written as a list of names, or several as a list of lists
        at_least_one: match spec.get("at_least_one").and_then(Value::as_array) {
            Some(a) if a.iter().all(Value::is_array) => a
                .iter()
                .map(|g| g.as_array().unwrap().iter().filter_map(|x| x.as_str().map(String::from)).collect())
                .collect(),
            Some(_) => vec![list("at_least_one")],
            None => Vec::new(),
        },
        open_params: spec.get("open_params").and_then(Value::as_bool).unwrap_or(false),
        since: str_of(spec.get("since")),
        until: str_of(spec.get("until")),
        style: str_of(spec.get("style")),
        output: str_of(spec.get("output")).unwrap_or_default(),
        override_: str_of(spec.get("override")),
        phase: spec.get("phase").and_then(Value::as_u64).map(|p| p as u8),
        section: str_of(spec.get("section")).unwrap_or_default(),
    })
}

/// The fields of the record head that the draft 0.3 adds (2.8), read only in a file that declares
/// it: `numbering`, the check of the numbers of the record of the solver, which affects the
/// interpretation (5.9). In a file of 0.2 it is an unknown field.
pub fn draft_header_fields() -> &'static [HeaderField] {
    static F: OnceLock<Vec<HeaderField>> = OnceLock::new();
    F.get_or_init(|| vec![HeaderField { name: "numbering".into(), values: None, affects: true }])
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
    fn embedded_list_has_46_keywords_of_0_2_and_0_3() {
        let w = words();
        assert_eq!(w.language_version, "0.2");
        assert_eq!(w.keywords_in("0.2").count(), 46);
        assert_eq!(w.keywords_in("0.3").count(), 46);
        // and the assertions of 0.4 (15.3)
        let added: Vec<_> = w.keywords.iter().filter(|k| !k.in_version("0.3")).map(|k| k.name.as_str()).collect();
        assert_eq!(added, ["ASSERT-INSTANCE", "ASSERT-TERMINAL", "ASSERT-NET"]);
        assert!(!w.all_words_in("0.3").contains("different") && w.all_words_in("0.4").contains("different"));
        let place = w.keyword("PLACE").unwrap();
        assert_eq!(place.positional, vec!["x", "y"]);
        assert!(matches!(w.keyword("AS").unwrap().positions, Positions::Definition));
        assert!(w.all_words().contains("left-to-right"));
        assert!(w.all_words().contains("position"));
        assert!(!w.all_words().contains("PLACE"));
        // DISTANCE's group as one list of names
        assert_eq!(w.keyword("DISTANCE").unwrap().at_least_one, vec![vec!["min", "max", "exact"]]);
    }

    #[test]
    fn a_word_is_in_the_versions_from_since_through_until() {
        let mut v: Value = serde_json::from_str(WORDS_0_2).unwrap();
        let kws = v["keywords"].as_object_mut().unwrap();
        let mut table = kws["GROUP"].clone();
        table["since"] = "0.4".into();
        table["at_least_one"] = serde_json::json!([["a", "b"], ["c"]]);
        table["open_params"] = true.into();
        kws.insert("TABLE".into(), table);
        kws["ROUTE"]["until"] = "0.3".into();
        kws["PRIORITY"]["params"][0]["since"] = "0.3".into();
        v["removed"] = serde_json::json!([{"word": "ROUTE", "what": "keyword", "until": "0.3", "see": "13.7"}]);
        let w = Words::parse(&v.to_string()).unwrap();
        assert!(!w.is_keyword_in("TABLE", "0.3") && w.is_keyword_in("TABLE", "0.4"));
        assert!(w.is_keyword_in("ROUTE", "0.3") && !w.is_keyword_in("ROUTE", "0.4"));
        // a keyword of any version read is still a standard word, which no alias may be spelled as
        assert!(w.is_keyword("TABLE") && w.is_keyword("ROUTE"));
        assert!(w.keywords_in("0.4").all(|k| k.name != "ROUTE"));
        assert_eq!(w.removed_in("ROUTE", "0.3").count(), 0);
        assert_eq!(w.removed_in("ROUTE", "0.4").map(|r| r.see.as_str()).collect::<Vec<_>>(), ["13.7"]);
        let table = w.keyword("TABLE").unwrap();
        assert_eq!(table.at_least_one, vec![vec!["a", "b"], vec!["c"]]);
        assert!(table.open_params && !w.keyword("GROUP").unwrap().open_params);
        let priority = w.keyword("PRIORITY").unwrap();
        assert!(priority.params_in("0.2").is_empty() && priority.params_in("0.3").len() == 1);
    }
}
