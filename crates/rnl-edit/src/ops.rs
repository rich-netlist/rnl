//! The ops of an edit transaction (editor interface draft, 4.1), read from JSON one at a time, so
//! that a refusal can say which op it is.

use serde::Deserialize;
use serde_json::{Map, Value};

/// A point in scene units, y up.
pub type Point = [f64; 2];

/// One op, with the level it writes into: `subckt:NAME`, or `file:PATH` for the file level. The
/// `level` of a `use-package` is not used: the package goes into the header record.
#[derive(Debug, Clone, PartialEq)]
pub struct LevelOp {
    pub level: Option<String>,
    pub op: Op,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    /// Facts about a host object: its symbol, position and orientation.
    Set { object: String, symbol: Option<String>, at: Option<Point>, orient: Option<String> },
    /// A usage of a definition (9.4), attached to objects or points, by role in order.
    AddUsage {
        name: Option<String>,
        definition: String,
        attach: Vec<Attach>,
        symbol: Option<String>,
        at: Option<Point>,
        orient: Option<String>,
    },
    /// A named point, with a junction dot when `junction`.
    AddPoint { name: String, at: Point, junction: bool },
    /// A wire of nets through vertices.
    AddWire {
        name: Option<String>,
        nets: Vec<String>,
        via: Vec<Vertex>,
        mode: Option<String>,
        style: Vec<(String, Literal)>,
    },
    /// A label (attached to an object) or an annotation (at a point).
    AddText {
        name: Option<String>,
        attach: Option<String>,
        key: Option<String>,
        text: TextContent,
        at: Option<Point>,
        offset: Option<Point>,
        halign: Option<String>,
        valign: Option<String>,
        rotate: Option<f64>,
        size: Option<f64>,
        style: Vec<(String, Literal)>,
    },
    /// ADD-TO or REMOVE-FROM a set (10.5).
    Membership { add: bool, items: Vec<String>, set: String },
    /// A package at the root, in the header record (16.3).
    UsePackage { name: String, version: Option<String>, path: Option<String> },
}

impl Op {
    pub fn name(&self) -> &'static str {
        match self {
            Op::Set { .. } => "set",
            Op::AddUsage { .. } => "add-usage",
            Op::AddPoint { .. } => "add-point",
            Op::AddWire { .. } => "add-wire",
            Op::AddText { .. } => "add-text",
            Op::Membership { add: true, .. } => "add-to",
            Op::Membership { add: false, .. } => "remove-from",
            Op::UsePackage { .. } => "use-package",
        }
    }
}

/// What a usage attaches to in one role: an object by its id, or a point.
#[derive(Debug, Clone, PartialEq)]
pub enum Attach {
    Id(String),
    At(Point),
}

/// A vertex of a wire.
#[derive(Debug, Clone, PartialEq)]
pub enum Vertex {
    /// A pin: `#M1:1`.
    Terminal(String),
    /// A named point: `%j1`.
    Point(String),
    /// A role of a usage: `%gnd1:p`.
    Usage { usage: String, role: String },
    /// Coordinates.
    At(Point),
}

/// A style property's value.
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Str(String),
    Num(f64),
    Bool(bool),
}

/// The content of a text: markup as written (13.6), or lines of runs as the Document gives them.
#[derive(Debug, Clone, PartialEq)]
pub enum TextContent {
    Markup(String),
    Lines(Vec<Vec<Run>>),
}

/// A run of a line of text, as the Document's `Text.lines` holds it.
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub enum Run {
    Text {
        text: String,
        #[serde(default)]
        style: RunStyle,
    },
    Frac {
        num: Vec<Run>,
        den: Vec<Run>,
        #[serde(default)]
        style: RunStyle,
    },
}

/// The style of a run: its script level, and whether it is bold, italic and overlined.
#[derive(Deserialize, Debug, Clone, Copy, Default, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct RunStyle {
    pub level: i32,
    pub bold: bool,
    pub italic: bool,
    pub overline: bool,
}

/// Read op `k` of a transaction.
pub fn parse(k: usize, value: &Value) -> Result<LevelOp, String> {
    let fail = |op: &str, why: String| format!("op {k} ({op}): {why}");
    let Some(obj) = value.as_object() else { return Err(format!("op {k}: not an object")) };
    let kind = obj.get("op").and_then(Value::as_str).ok_or_else(|| format!("op {k}: no \"op\""))?;
    // fields of the draft that this writer does not write yet: refused rather than left out
    let later: &[&str] = match kind {
        "set" | "add-usage" => &["style", "layer"],
        "add-wire" => &["layer"],
        _ => &[],
    };
    if let Some(f) = later.iter().find(|f| obj.contains_key(**f)) {
        return Err(fail(kind, format!("`{f}` is not written yet")));
    }
    let allowed: &[&str] = match kind {
        "set" => &["object", "symbol", "at", "orient"],
        "add-usage" => &["name", "definition", "attach", "symbol", "at", "orient"],
        "add-point" => &["name", "at", "junction"],
        "add-wire" => &["name", "nets", "via", "mode", "style"],
        "add-text" => {
            &["name", "attach", "key", "markup", "lines", "at", "offset", "halign", "valign", "rotate", "size", "style"]
        }
        "add-to" | "remove-from" => &["items", "set"],
        "use-package" => &["name", "version", "path"],
        "unset" | "add-shape" | "remove" => return Err(fail(kind, "not written yet".into())),
        _ => return Err(format!("op {k}: unknown op \"{kind}\"")),
    };
    if let Some(f) = obj.keys().find(|f| *f != "op" && *f != "level" && !allowed.contains(&f.as_str())) {
        return Err(fail(kind, format!("unknown field `{f}`")));
    }
    let level = match obj.get("level") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => Some(s.clone()),
        Some(_) => return Err(fail(kind, "`level` is not a string".into())),
    };
    if level.is_none() && kind != "use-package" {
        return Err(fail(kind, "no `level`".into()));
    }
    let r = Reader { obj, kind, k };
    let op = match kind {
        "set" => {
            let op = Op::Set {
                object: r.string("object")?,
                symbol: r.opt_string("symbol")?,
                at: r.opt_point("at")?,
                orient: r.opt_string("orient")?,
            };
            if let Op::Set { symbol: None, at: None, orient: None, .. } = op {
                return Err(fail(kind, "nothing to set: no `symbol`, `at` or `orient`".into()));
            }
            op
        }
        "add-usage" => Op::AddUsage {
            name: r.opt_string("name")?,
            definition: r.string("definition")?,
            attach: r.attach()?,
            symbol: r.opt_string("symbol")?,
            at: r.opt_point("at")?,
            orient: r.opt_string("orient")?,
        },
        "add-point" => Op::AddPoint {
            name: r.string("name")?,
            at: r.point("at")?,
            junction: r.opt_bool("junction")?.unwrap_or(false),
        },
        "add-wire" => Op::AddWire {
            name: r.opt_string("name")?,
            nets: r.strings("nets")?,
            via: r.via()?,
            mode: r.opt_string("mode")?,
            style: r.style()?,
        },
        "add-text" => {
            let text = match (obj.get("markup"), obj.get("lines")) {
                (Some(_), Some(_)) => return Err(fail(kind, "both `markup` and `lines`; give one".into())),
                (Some(_), None) => TextContent::Markup(r.string("markup")?),
                (None, Some(lines)) => TextContent::Lines(
                    serde_json::from_value(lines.clone()).map_err(|e| fail(kind, format!("`lines`: {e}")))?,
                ),
                (None, None) => return Err(fail(kind, "no `markup` or `lines`".into())),
            };
            let (attach, at) = (r.opt_string("attach")?, r.opt_point("at")?);
            if attach.is_none() && at.is_none() {
                return Err(fail(kind, "neither `attach` nor `at`: the text has no place".into()));
            }
            Op::AddText {
                name: r.opt_string("name")?,
                attach,
                key: r.opt_string("key")?,
                text,
                at,
                offset: r.opt_point("offset")?,
                halign: r.opt_string("halign")?,
                valign: r.opt_string("valign")?,
                rotate: r.opt_num("rotate")?,
                size: r.opt_num("size")?,
                style: r.style()?,
            }
        }
        "add-to" | "remove-from" => {
            Op::Membership { add: kind == "add-to", items: r.strings("items")?, set: r.string("set")? }
        }
        "use-package" => {
            Op::UsePackage { name: r.string("name")?, version: r.opt_string("version")?, path: r.opt_string("path")? }
        }
        _ => unreachable!(),
    };
    Ok(LevelOp { level, op })
}

/// The fields of one op, with refusals that name the op.
struct Reader<'a> {
    obj: &'a Map<String, Value>,
    kind: &'a str,
    k: usize,
}

impl Reader<'_> {
    fn fail(&self, field: &str, what: &str) -> String {
        format!("op {} ({}): `{field}` {what}", self.k, self.kind)
    }

    fn opt_string(&self, f: &str) -> Result<Option<String>, String> {
        match self.obj.get(f) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(s)) => Ok(Some(s.clone())),
            Some(_) => Err(self.fail(f, "is not a string")),
        }
    }

    fn string(&self, f: &str) -> Result<String, String> {
        self.opt_string(f)?.ok_or_else(|| self.fail(f, "is missing"))
    }

    fn strings(&self, f: &str) -> Result<Vec<String>, String> {
        let list = self.obj.get(f).and_then(Value::as_array).ok_or_else(|| self.fail(f, "is not a list of strings"))?;
        let out: Option<Vec<String>> = list.iter().map(|v| v.as_str().map(String::from)).collect();
        match out {
            Some(v) if !v.is_empty() => Ok(v),
            Some(_) => Err(self.fail(f, "is empty")),
            None => Err(self.fail(f, "is not a list of strings")),
        }
    }

    fn opt_num(&self, f: &str) -> Result<Option<f64>, String> {
        match self.obj.get(f) {
            None | Some(Value::Null) => Ok(None),
            Some(v) => v.as_f64().map(Some).ok_or_else(|| self.fail(f, "is not a number")),
        }
    }

    fn opt_bool(&self, f: &str) -> Result<Option<bool>, String> {
        match self.obj.get(f) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::Bool(b)) => Ok(Some(*b)),
            Some(_) => Err(self.fail(f, "is not true or false")),
        }
    }

    fn opt_point(&self, f: &str) -> Result<Option<Point>, String> {
        match self.obj.get(f) {
            None | Some(Value::Null) => Ok(None),
            Some(v) => as_point(v).map(Some).ok_or_else(|| self.fail(f, "is not a point [x, y]")),
        }
    }

    fn point(&self, f: &str) -> Result<Point, String> {
        self.opt_point(f)?.ok_or_else(|| self.fail(f, "is missing"))
    }

    fn attach(&self) -> Result<Vec<Attach>, String> {
        let map = self
            .obj
            .get("attach")
            .and_then(Value::as_object)
            .ok_or_else(|| self.fail("attach", "is not an object of roles"))?;
        if map.is_empty() {
            return Err(self.fail("attach", "has no roles"));
        }
        map.iter()
            .map(|(role, v)| match v {
                Value::String(s) => Ok(Attach::Id(s.clone())),
                _ => as_point(v)
                    .map(Attach::At)
                    .ok_or_else(|| self.fail("attach", &format!("role `{role}` is not an id or a point"))),
            })
            .collect()
    }

    fn via(&self) -> Result<Vec<Vertex>, String> {
        let list = self
            .obj
            .get("via")
            .and_then(Value::as_array)
            .ok_or_else(|| self.fail("via", "is not a list of vertices"))?;
        if list.len() < 2 {
            return Err(self.fail("via", "has fewer than two vertices"));
        }
        list.iter()
            .enumerate()
            .map(|(n, v)| {
                vertex(v).ok_or_else(|| {
                    self.fail(
                        "via",
                        &format!("vertex {n} is not one of {{terminal}}, {{point}}, {{usage, role}}, {{at}}"),
                    )
                })
            })
            .collect()
    }

    fn style(&self) -> Result<Vec<(String, Literal)>, String> {
        let Some(v) = self.obj.get("style") else { return Ok(Vec::new()) };
        let map = v.as_object().ok_or_else(|| self.fail("style", "is not an object"))?;
        map.iter()
            .map(|(name, v)| {
                let lit = match v {
                    Value::String(s) => Literal::Str(s.clone()),
                    Value::Bool(b) => Literal::Bool(*b),
                    Value::Number(n) => Literal::Num(n.as_f64().unwrap_or(f64::NAN)),
                    _ => return Err(self.fail("style", &format!("`{name}` is not a string, number or boolean"))),
                };
                Ok((name.clone(), lit))
            })
            .collect()
    }
}

fn as_point(v: &Value) -> Option<Point> {
    let a = v.as_array()?;
    if a.len() != 2 {
        return None;
    }
    let (x, y) = (a[0].as_f64()?, a[1].as_f64()?);
    (x.is_finite() && y.is_finite()).then_some([x, y])
}

fn vertex(v: &Value) -> Option<Vertex> {
    let o = v.as_object()?;
    let only = |keys: &[&str]| o.len() == keys.len() && keys.iter().all(|k| o.contains_key(*k));
    let s = |k: &str| o.get(k)?.as_str().map(String::from);
    if only(&["terminal"]) {
        return s("terminal").map(Vertex::Terminal);
    }
    if only(&["point"]) {
        return s("point").map(Vertex::Point);
    }
    if only(&["usage", "role"]) {
        return Some(Vertex::Usage { usage: s("usage")?, role: s("role")? });
    }
    if only(&["at"]) {
        return as_point(o.get("at")?).map(Vertex::At);
    }
    None
}
