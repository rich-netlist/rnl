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
    /// Facts about a host object: its symbol, style, position and orientation. The style is its
    /// properties (9.3), written as a DEFINE-STYLE that a SET-STYLE gives the object (11.3).
    Set {
        object: String,
        symbol: Option<String>,
        style: Vec<(String, Literal)>,
        at: Option<Point>,
        orient: Option<String>,
    },
    /// A usage of a definition (9.4), attached to objects or points: each role by its name, with
    /// what it is given, in the order the request writes them; and the parameters it writes, in the
    /// order given; and its style, as for `Set`.
    AddUsage {
        name: Option<String>,
        definition: String,
        attach: Vec<(String, Attach)>,
        params: Vec<(String, Literal)>,
        symbol: Option<String>,
        style: Vec<(String, Literal)>,
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
    /// A graphic in the scene (13.4), on a layer when given (11.6).
    AddShape { name: Option<String>, shape: ShapeOp, layer: Option<String> },
    /// A symbol (9.2): its box, pins and graphics, in its own coordinates.
    DefineSymbol { name: String, bbox: Option<[f64; 4]>, pins: Vec<PinOp>, prims: Vec<ShapeOp> },
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
            Op::AddShape { .. } => "add-shape",
            Op::DefineSymbol { .. } => "define-symbol",
        }
    }
}

/// A graphic (9.2.5): `line`, `rectangle`, `circle`, `ellipse`, `arc` or `bezier`, its points, and
/// its parameters and style properties in the order given. A rectangle, circle or ellipse is given
/// by its centre and its size, or by two points.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeOp {
    pub shape: String,
    pub points: Vec<Point>,
    pub params: Vec<(String, Literal)>,
    pub style: Vec<(String, Literal)>,
}

/// A pin of a symbol (9.2.4).
#[derive(Debug, Clone, PartialEq)]
pub struct PinOp {
    /// One terminal, or several for a bus pin, by number or by name.
    pub terminals: Vec<TermKey>,
    pub at: Point,
    /// "right", "up", "left" or "down".
    pub direction: Option<String>,
    /// The pin's name for display, in markup or as lines of runs.
    pub label: Option<TextContent>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TermKey {
    Num(u32),
    Name(String),
}

/// What a usage attaches to in one role: an object by its id, or a point; for a set role (9.4),
/// the list of its members, each an id or a point, which may be empty.
#[derive(Debug, Clone, PartialEq)]
pub enum Attach {
    Id(String),
    At(Point),
    Set(Vec<Attach>),
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

/// A literal (3.7): the value of a style property or of a graphic's parameter, a string, a number
/// or a truth value; the value of a usage's parameter may also be a list of them.
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Str(String),
    Num(f64),
    Bool(bool),
    List(Vec<Literal>),
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
        "set" | "add-usage" => &["layer"],
        "add-wire" => &["layer"],
        _ => &[],
    };
    if let Some(f) = later.iter().find(|f| obj.contains_key(**f)) {
        return Err(fail(kind, format!("`{f}` is not written yet")));
    }
    let allowed: &[&str] = match kind {
        "set" => &["object", "symbol", "style", "at", "orient"],
        "add-usage" => &["name", "definition", "attach", "params", "symbol", "style", "at", "orient"],
        "add-point" => &["name", "at", "junction"],
        "add-wire" => &["name", "nets", "via", "mode", "style"],
        "add-text" => {
            &["name", "attach", "key", "markup", "lines", "at", "offset", "halign", "valign", "rotate", "size", "style"]
        }
        "add-to" | "remove-from" => &["items", "set"],
        "use-package" => &["name", "version", "path"],
        "add-shape" => &["name", "shape", "points", "params", "style", "layer"],
        "define-symbol" => &["name", "box", "pins", "prims"],
        "unset" | "remove" => return Err(fail(kind, "not written yet".into())),
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
                style: r.style()?,
                at: r.opt_point("at")?,
                orient: r.opt_string("orient")?,
            };
            if let Op::Set { symbol: None, at: None, orient: None, style, .. } = &op {
                if style.is_empty() {
                    return Err(fail(kind, "nothing to set: no `symbol`, `style`, `at` or `orient`".into()));
                }
            }
            op
        }
        "add-usage" => Op::AddUsage {
            name: r.opt_string("name")?,
            definition: r.string("definition")?,
            attach: r.attach()?,
            params: r.params()?,
            symbol: r.opt_string("symbol")?,
            style: r.style()?,
            at: r.opt_point("at")?,
            orient: r.opt_string("orient")?,
        },
        "add-point" => Op::AddPoint {
            name: r.string("name")?,
            at: r.point("at")?,
            junction: r.opt_bool("junction")?.unwrap_or(false),
        },
        "add-shape" => Op::AddShape {
            name: r.opt_string("name")?,
            shape: shape_op(obj).map_err(|why| fail(kind, why))?,
            layer: match r.opt_string("layer")? {
                Some(l) if !["bottom", "middle", "top"].contains(&l.as_str()) => {
                    return Err(fail(kind, format!("`layer` \"{l}\" is not bottom, middle or top")))
                }
                l => l,
            },
        },
        "define-symbol" => {
            let name = r.string("name")?;
            if !(name.starts_with('$') || name.starts_with('%')) || name.len() < 2 {
                return Err(fail(kind, format!("`name` \"{name}\" is not a $ or % name")));
            }
            let bbox = match obj.get("box") {
                None | Some(Value::Null) => None,
                Some(v) => {
                    let b: Option<Vec<f64>> = v.as_array().and_then(|a| a.iter().map(Value::as_f64).collect());
                    match b.as_deref() {
                        Some(&[x1, y1, x2, y2]) => Some([x1, y1, x2, y2]),
                        _ => return Err(fail(kind, "`box` is not [x1, y1, x2, y2]".into())),
                    }
                }
            };
            let list = |f: &str| -> Result<Vec<Value>, String> {
                match obj.get(f) {
                    None | Some(Value::Null) => Ok(Vec::new()),
                    Some(Value::Array(a)) => Ok(a.clone()),
                    Some(_) => Err(fail(kind, format!("`{f}` is not a list"))),
                }
            };
            let pins = list("pins")?
                .iter()
                .enumerate()
                .map(|(n, p)| pin_op(p).map_err(|why| fail(kind, format!("pin {n}: {why}"))))
                .collect::<Result<_, _>>()?;
            let prims = list("prims")?
                .iter()
                .enumerate()
                .map(|(n, p)| {
                    let o = p.as_object().ok_or("not an object".to_string()).and_then(|o| {
                        match o.keys().find(|f| !["shape", "points", "params", "style"].contains(&f.as_str())) {
                            Some(f) => Err(format!("unknown field `{f}`")),
                            None => Ok(o),
                        }
                    });
                    o.and_then(shape_op).map_err(|why| fail(kind, format!("prim {n}: {why}")))
                })
                .collect::<Result<_, _>>()?;
            Op::DefineSymbol { name, bbox, pins, prims }
        }
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

    fn attach(&self) -> Result<Vec<(String, Attach)>, String> {
        let map = self
            .obj
            .get("attach")
            .and_then(Value::as_object)
            .ok_or_else(|| self.fail("attach", "is not an object of roles"))?;
        if map.is_empty() {
            return Err(self.fail("attach", "has no roles"));
        }
        map.iter()
            .map(|(role, v)| {
                // written as the role's name after what it is given, `#M1(ref)` (9.4)
                if role.is_empty() || role.chars().any(char::is_control) {
                    return Err(self.fail("attach", &format!("role {role:?} is not a name")));
                }
                let not = || self.fail("attach", &format!("role `{role}` is not an id, a point or a list of them"));
                let one = |m: &Value| match m {
                    Value::String(s) => Some(Attach::Id(s.clone())),
                    _ => as_point(m).map(Attach::At),
                };
                let given = match v {
                    // a set role's members; two numbers are a point
                    Value::Array(members) if as_point(v).is_none() => {
                        Attach::Set(members.iter().map(one).collect::<Option<Vec<_>>>().ok_or_else(not)?)
                    }
                    _ => one(v).ok_or_else(not)?,
                };
                Ok((role.clone(), given))
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

    /// A usage's parameters: each a string, a number, a truth value or a list of them, as the
    /// snapshot gives them back (`ObjectView.params`). A string with a line break is refused: it
    /// would end the comment that carries the record.
    fn params(&self) -> Result<Vec<(String, Literal)>, String> {
        fn value(v: &Value) -> Option<Literal> {
            Some(match v {
                Value::String(s) if !s.contains(['\n', '\r']) => Literal::Str(s.clone()),
                Value::Bool(b) => Literal::Bool(*b),
                Value::Number(n) => Literal::Num(n.as_f64()?),
                Value::Array(a) => Literal::List(a.iter().map(value).collect::<Option<_>>()?),
                _ => return None,
            })
        }
        let Some(v) = self.obj.get("params").filter(|v| !v.is_null()) else { return Ok(Vec::new()) };
        let map = v.as_object().ok_or_else(|| self.fail("params", "is not an object"))?;
        map.iter()
            .map(|(name, v)| match value(v) {
                Some(lit) => Ok((name.clone(), lit)),
                None => Err(self.fail(
                    "params",
                    &format!("`{name}` is not a string without a line break, a number, a boolean or a list of them"),
                )),
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

/// Literals by name, in order: a parameter list or a style.
fn literals(v: Option<&Value>, what: &str) -> Result<Vec<(String, Literal)>, String> {
    let Some(v) = v.filter(|v| !v.is_null()) else { return Ok(Vec::new()) };
    let map = v.as_object().ok_or_else(|| format!("`{what}` is not an object"))?;
    map.iter()
        .map(|(name, v)| {
            let lit = match v {
                Value::String(s) => Literal::Str(s.clone()),
                Value::Bool(b) => Literal::Bool(*b),
                Value::Number(n) => Literal::Num(n.as_f64().unwrap_or(f64::NAN)),
                _ => return Err(format!("`{what}`: `{name}` is not a string, number or boolean")),
            };
            Ok((name.clone(), lit))
        })
        .collect()
}

/// A graphic, from an add-shape op or a prim of define-symbol.
fn shape_op(obj: &Map<String, Value>) -> Result<ShapeOp, String> {
    let shape = obj.get("shape").and_then(Value::as_str).ok_or("no `shape`")?.to_string();
    let points: Vec<Point> = match obj.get("points").and_then(Value::as_array) {
        Some(a) => a.iter().map(as_point).collect::<Option<_>>().ok_or("`points` is not a list of [x, y]")?,
        None => return Err("no `points`".into()),
    };
    let params = literals(obj.get("params"), "params")?;
    if let Some((p, _)) =
        params.iter().find(|(p, _)| !["width", "height", "radius", "fill", "arrow"].contains(&p.as_str()))
    {
        return Err(format!("`params`: `{p}` is not width, height, radius, fill or arrow"));
    }
    let has = |p: &str| params.iter().any(|(q, _)| q == p);
    let n = points.len();
    let ok = match shape.as_str() {
        "line" => n >= 2,
        "arc" => n == 3,
        "bezier" => n >= 4 && (n - 1) % 3 == 0,
        // the centre and a size, or two points (9.2.5)
        "rectangle" | "ellipse" => {
            (n == 1 && has("width") && has("height")) || (n == 2 && !has("width") && !has("height"))
        }
        "circle" => (n == 1 && has("radius")) || (n == 2 && !has("radius")),
        _ => return Err(format!("`shape` \"{shape}\" is not line, rectangle, circle, ellipse, arc or bezier")),
    };
    if !ok {
        return Err(format!("{n} points with these params do not make the shape {shape} (9.2.5)"));
    }
    Ok(ShapeOp { shape, points, params, style: literals(obj.get("style"), "style")? })
}

/// A pin of define-symbol.
fn pin_op(v: &Value) -> Result<PinOp, String> {
    let o = v.as_object().ok_or("not an object")?;
    if let Some(f) = o.keys().find(|f| !["terminals", "at", "direction", "label"].contains(&f.as_str())) {
        return Err(format!("unknown field `{f}`"));
    }
    let terminals: Vec<TermKey> = o
        .get("terminals")
        .and_then(Value::as_array)
        .filter(|a| !a.is_empty())
        .and_then(|a| {
            a.iter()
                .map(|t| match t {
                    Value::String(s) => Some(TermKey::Name(s.clone())),
                    Value::Number(n) => n.as_u64().filter(|n| *n >= 1).map(|n| TermKey::Num(n as u32)),
                    _ => None,
                })
                .collect()
        })
        .ok_or("`terminals` is not a list of terminal numbers and names")?;
    let at = o.get("at").and_then(as_point).ok_or("`at` is not a point [x, y]")?;
    let direction = match o.get("direction") {
        None | Some(Value::Null) => None,
        Some(Value::String(d)) if ["right", "up", "left", "down"].contains(&d.as_str()) => Some(d.clone()),
        Some(_) => return Err("`direction` is not right, up, left or down".into()),
    };
    let label = match o.get("label") {
        None | Some(Value::Null) => None,
        Some(Value::Object(l)) => match (l.get("markup"), l.get("lines")) {
            (Some(Value::String(m)), None) => Some(TextContent::Markup(m.clone())),
            (None, Some(lines)) => {
                Some(TextContent::Lines(serde_json::from_value(lines.clone()).map_err(|e| format!("`label`: {e}"))?))
            }
            _ => return Err("`label` is not {markup} or {lines}".into()),
        },
        Some(_) => return Err("`label` is not {markup} or {lines}".into()),
    };
    Ok(PinOp { terminals, at, direction, label })
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
