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
    /// A version may replace the old parameter type.
    pub type_since: Option<(String, ParamType)>,
    pub required: bool,
    /// The last version requiring this parameter; later versions still accept it as optional.
    pub required_until: Option<String>,
    /// The version that adds it, none for 0.2.
    pub since: Option<String>,
    /// The last version that has it, none while it is kept.
    pub until: Option<String>,
    /// Whether it takes a list of its values as well as one, such as ORIENT's codes in 0.4 (11.2).
    pub list: bool,
    /// Of an enumeration, the values a version adds or removes: each with the version that adds it
    /// and the last that has it, as `values_since` and `values_until` write them.
    pub value_versions: Vec<(String, Option<String>, Option<String>)>,
    /// The version from which it takes a list, as `list_since` writes it.
    pub list_since: Option<String>,
}

impl ParamSpec {
    /// An optional parameter of every version.
    pub fn new(name: impl Into<String>, ty: ParamType) -> ParamSpec {
        ParamSpec {
            name: name.into(),
            ty,
            type_since: None,
            required: false,
            required_until: None,
            since: None,
            until: None,
            list: false,
            value_versions: Vec::new(),
            list_since: None,
        }
    }

    /// Whether a file of the language version `version` has this parameter.
    pub fn in_version(&self, version: &str) -> bool {
        in_version(&self.since, &self.until, version)
    }

    /// The parameter as a file of the language version `version` has it: the values of an
    /// enumeration of that version, and whether it takes a list there.
    pub fn as_in(&self, version: &str) -> ParamSpec {
        let mut p = self.clone();
        p.required = self.required && in_version(&None, &self.required_until, version);
        if let Some((since, ty)) = &self.type_since {
            if in_version(&Some(since.clone()), &None, version) {
                p.ty = ty.clone();
            }
        }
        if let ParamType::Enum(vals) = &mut p.ty {
            vals.retain(|v| match v {
                Literal::Str(s) => self
                    .value_versions
                    .iter()
                    .find(|(w, _, _)| w == s)
                    .is_none_or(|(_, since, until)| in_version(since, until, version)),
                Literal::Num(_) => true,
            });
        }
        p.list = self.list_since.as_deref().is_some_and(|s| in_version(&Some(s.to_string()), &None, version));
        p
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

impl Takes {
    fn parse(value: &str) -> Takes {
        match value {
            "one" => Takes::One,
            "set" => Takes::Set,
            "shape" => Takes::Shape,
            "any" => Takes::Any,
            "body" => Takes::Body,
            other => other.strip_prefix("def:").map_or(Takes::Any, |kind| Takes::Def(kind.to_string())),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PositionSpec {
    pub name: String,
    pub takes: Takes,
    /// Changes of the slot's cardinality, keyed by the version that introduces each change.
    pub takes_versions: Vec<(String, Takes)>,
    pub kinds: Vec<String>,
    /// The versions that add or remove accepted kinds, as `kinds_since` and `kinds_until` write them.
    pub kind_versions: Vec<(String, Option<String>, Option<String>)>,
    /// Versions that change the declared ordering of accepted kinds.
    pub kind_orders: Vec<(String, Vec<String>)>,
    pub optional: bool,
}

impl PositionSpec {
    pub fn takes_in(&self, version: &str) -> &Takes {
        let key = crate::interp::unit::version_key;
        self.takes_versions
            .iter()
            .filter(|(since, _)| key(since) <= key(version))
            .max_by_key(|(since, _)| key(since))
            .map_or(&self.takes, |(_, takes)| takes)
    }

    /// The kinds accepted by this position in a file of the given language version.
    pub fn kinds_in(&self, version: &str) -> Vec<String> {
        let key = crate::interp::unit::version_key;
        self.kind_orders
            .iter()
            .filter(|(since, _)| key(since) <= key(version))
            .max_by_key(|(since, _)| key(since))
            .map_or(&self.kinds, |(_, kinds)| kinds)
            .iter()
            .filter(|kind| {
                self.kind_versions
                    .iter()
                    .find(|(name, _, _)| name == *kind)
                    .is_none_or(|(_, since, until)| in_version(since, until, version))
            })
            .cloned()
            .collect()
    }
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
    /// Takes named parameters declared by the source definition (13.5) or usage (9.4).
    pub definition_params: bool,
    /// The parameters that may be written by position, in order (4.2 rule 9).
    pub positional: Vec<String>,
    /// Groups of parameters of which each needs at least one written (4.2), such as DISTANCE's
    /// `min`, `max` and `exact`.
    pub at_least_one: Vec<Vec<String>>,
    /// The first version requiring these parameter groups.
    pub at_least_one_since: Option<String>,
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
    pub fn at_least_one_in(&self, version: &str) -> &[Vec<String>] {
        if in_version(&self.at_least_one_since, &None, version) {
            &self.at_least_one
        } else {
            &[]
        }
    }

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
        self.param_list().iter().filter(|p| p.in_version(version)).map(|p| p.as_in(version)).collect()
    }

    /// The parameters that may be written by position in a file of the language version
    /// `version`: those of `positional` that the version has, in order.
    pub fn positional_in(&self, version: &str) -> Vec<String> {
        let params = self.params_in(version);
        self.positional.iter().filter(|n| params.iter().any(|p| &p.name == *n)).cloned().collect()
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
    pub since: Option<String>,
}

impl StyleProp {
    pub fn in_version(&self, version: &str) -> bool {
        in_version(&self.since, &None, version)
    }
}

#[derive(Debug)]
pub struct Words {
    pub language_version: String,
    pub header_fields: Vec<HeaderField>,
    pub kinds: Vec<String>,
    pub kind_versions: Vec<(String, Option<String>, Option<String>)>,
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
        let kind_versions = word_versions(&v, "kinds");
        let subobjects = strings("subobjects")?;
        let attributes = strings("attributes")?;
        let mut style_props = Vec::new();
        for (name, spec) in obj.get("style_props").and_then(Value::as_object).ok_or("style_props")? {
            style_props.push(StyleProp {
                name: name.clone(),
                ty: param_type(spec),
                since: spec.get("since").and_then(Value::as_str).map(String::from),
            });
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
            kind_versions,
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

    /// The object kinds available to signatures and role declarations in this language version.
    pub fn kinds_in(&self, version: &str) -> Vec<String> {
        self.kinds
            .iter()
            .filter(|kind| {
                self.kind_versions
                    .iter()
                    .find(|(name, _, _)| name == *kind)
                    .is_none_or(|(_, since, until)| in_version(since, until, version))
            })
            .cloned()
            .collect()
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
        for s in self.style_props.iter().filter(|s| version.is_none_or(|v| s.in_version(v))) {
            out.insert(s.name.clone());
            if let ParamType::Enum(vals) = &s.ty {
                for v in vals {
                    if let Literal::Str(x) = v {
                        out.insert(x.clone());
                    }
                }
            }
        }
        out.extend(version.map_or_else(|| self.kinds.clone(), |version| self.kinds_in(version)));
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

/// The values of an enumeration that a version adds or removes: `values_since` and `values_until`
/// map a value to the version that adds it or the last that has it.
fn word_versions(spec: &Value, field: &str) -> Vec<(String, Option<String>, Option<String>)> {
    let mut out: Vec<(String, Option<String>, Option<String>)> = Vec::new();
    for (suffix, since) in [("since", true), ("until", false)] {
        for (word, v) in spec.get(format!("{field}_{suffix}")).and_then(Value::as_object).into_iter().flatten() {
            let v = v.as_str().map(String::from);
            match out.iter_mut().find(|(w, _, _)| w == word) {
                Some(e) if since => e.1 = v,
                Some(e) => e.2 = v,
                None if since => out.push((word.clone(), v, None)),
                None => out.push((word.clone(), None, v)),
            }
        }
    }
    out
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
                    let takes = Takes::parse(p.get("takes").and_then(Value::as_str).unwrap_or("one"));
                    PositionSpec {
                        name: str_of(p.get("name")).unwrap_or_default(),
                        takes,
                        takes_versions: p
                            .get("takes_since")
                            .and_then(Value::as_object)
                            .into_iter()
                            .flatten()
                            .filter_map(|(version, takes)| Some((version.clone(), Takes::parse(takes.as_str()?))))
                            .collect(),
                        kinds: p
                            .get("kinds")
                            .and_then(Value::as_array)
                            .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                            .unwrap_or_default(),
                        optional: p.get("optional").and_then(Value::as_bool).unwrap_or(false),
                        kind_versions: word_versions(p, "kinds"),
                        kind_orders: p
                            .get("kinds_order")
                            .and_then(Value::as_object)
                            .into_iter()
                            .flat_map(|orders| orders.iter())
                            .map(|(since, kinds)| {
                                (
                                    since.clone(),
                                    kinds
                                        .as_array()
                                        .into_iter()
                                        .flatten()
                                        .filter_map(Value::as_str)
                                        .map(String::from)
                                        .collect(),
                                )
                            })
                            .collect(),
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
                    type_since: p.get("type_since").and_then(Value::as_object).and_then(|v| v.iter().next()).map(
                        |(version, ty)| {
                            let spec = serde_json::json!({"type": ty});
                            (version.clone(), param_type(&spec))
                        },
                    ),
                    required: p.get("required").and_then(Value::as_bool).unwrap_or(false),
                    required_until: str_of(p.get("required_until")),
                    since: str_of(p.get("since")),
                    until: str_of(p.get("until")),
                    list: false,
                    value_versions: word_versions(p, "values"),
                    list_since: str_of(p.get("list_since")),
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
        definition_params: spec
            .get("definition_params")
            .and_then(Value::as_bool)
            .unwrap_or(matches!(params, ParamsSpec::Definition)),
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
        at_least_one_since: str_of(spec.get("at_least_one_since")),
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

/// Record fields added by the drafts through 0.4 (2.8).
pub fn draft_0_4_header_fields() -> &'static [HeaderField] {
    static F: OnceLock<Vec<HeaderField>> = OnceLock::new();
    F.get_or_init(|| {
        let mut fields = draft_header_fields().to_vec();
        fields.push(HeaderField {
            name: "language".into(),
            values: Some(vec!["spice".into(), "spectre".into()]),
            affects: true,
        });
        fields
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
    fn embedded_list_has_46_keywords_of_0_2_and_0_3() {
        let w = words();
        assert_eq!(w.language_version, "0.2");
        assert_eq!(w.keywords_in("0.2").count(), 46);
        assert_eq!(w.keywords_in("0.3").count(), 46);
        // Annotations, pin selection, routing, settings, tables and assertions added by 0.4.
        let added: Vec<_> = w.keywords.iter().filter(|k| !k.in_version("0.3")).map(|k| k.name.as_str()).collect();
        assert_eq!(
            added,
            [
                "ANNOTATION",
                "SELECT-PIN",
                "ROUTE-NET",
                "ROUTE-OBJECT",
                "UNROUTE-NET",
                "UNROUTE-OBJECT",
                "SOLVER-CONFIG",
                "TABLE",
                "ASSERT-INSTANCE",
                "ASSERT-TERMINAL",
                "ASSERT-NET",
                "ASSERT-TOPOLOGY",
                "ASSERT-USAGE",
                "ASSERT-IN"
            ]
        );
        assert!(w.keyword_in("ROUTE", "0.3").is_some());
        assert!(w.keyword_in("ROUTE", "0.4").is_none());
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
    fn style_words_follow_the_source_language_version() {
        let w = words();
        for version in ["0.2", "0.3"] {
            assert!(!w.all_words_in(version).contains("textvisible"));
            assert!(w.all_words_in(version).contains("textcolor"));
        }
        assert!(w.all_words_in("0.4").contains("textvisible"));
        assert!(w.all_words().contains("textvisible"));
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
