use crate::json::{Node, Result};
use rnl_runtime::{
    diag::Loc,
    interp::obj::{Lit, Priority, WStr},
    junction::JunctionConfig,
    markup::{Run, RunStyle},
    problem::SolverSetting,
};
use serde_json::{json, Value};

pub(crate) fn strings(n: Node<'_>) -> Result<Vec<String>> {
    n.list(|n| Ok(n.string()?.to_owned()))
}
pub(crate) fn indices(n: Node<'_>) -> Result<Vec<usize>> {
    n.list(|n| n.index())
}
pub(crate) fn optional_string(n: Node<'_>) -> Result<Option<String>> {
    n.nullable(|n| Ok(n.string()?.to_owned()))
}
pub(crate) fn priority(n: Node<'_>) -> Result<Priority> {
    Priority::parse(n.string()?).ok_or_else(|| n.error("unknown priority"))
}
pub(crate) fn loc_value(l: &Loc) -> Value {
    json!({"file":l.file,"line":l.line,"col":l.col})
}
pub(crate) fn loc(n: Node<'_>) -> Result<Loc> {
    Ok(Loc { file: n.field("file")?.string()?.to_owned(), line: n.field("line")?.u32()?, col: n.field("col")?.u32()? })
}

pub(crate) fn lit_value(v: &Lit) -> Value {
    match v {
        Lit::Num(value) => json!({"kind":"number","value":value}),
        Lit::Str(value) => json!({"kind":"string","value":value.as_str(),"unit":value.unit}),
        Lit::Bool(value) => json!({"kind":"boolean","value":value}),
        Lit::Arr(values) => json!({"kind":"array","value":values.iter().map(lit_value).collect::<Vec<_>>()}),
    }
}
pub(crate) fn lit(n: Node<'_>, depth: usize) -> Result<Lit> {
    if depth > 64 {
        return Err(n.error("literal nesting exceeds 64 levels"));
    }
    Ok(match n.field("kind")?.string()? {
        "number" => Lit::Num(n.field("value")?.number()?),
        "string" => Lit::Str(WStr::new(n.field("value")?.string()?, n.field("unit")?.nullable(|n| n.index())?)),
        "boolean" => Lit::Bool(n.field("value")?.boolean()?),
        "array" => Lit::Arr(n.field("value")?.list(|n| lit(n, depth + 1))?),
        _ => return Err(n.field("kind")?.error("unknown literal kind")),
    })
}
pub(crate) fn pairs_value(values: &[(String, Lit)]) -> Value {
    json!(values.iter().map(|(key, value)| json!({"key":key,"value":lit_value(value)})).collect::<Vec<_>>())
}
pub(crate) fn pairs(n: Node<'_>) -> Result<Vec<(String, Lit)>> {
    n.list(|n| Ok((n.field("key")?.string()?.to_owned(), lit(n.field("value")?, 0)?)))
}

pub(crate) fn setting_value(s: &SolverSetting) -> Value {
    json!({"key":s.key,"value":lit_value(&s.value),"source":s.source.as_ref().map(loc_value),"package":s.package})
}
pub(crate) fn setting(n: Node<'_>) -> Result<SolverSetting> {
    Ok(SolverSetting {
        key: n.field("key")?.string()?.to_owned(),
        value: lit(n.field("value")?, 0)?,
        source: n.field("source")?.nullable(loc)?,
        package: n.field("package")?.string()?.to_owned(),
    })
}
pub(crate) fn junction_value(j: &JunctionConfig) -> Value {
    json!({"definition":j.definition,"invalid":j.invalid.as_ref().map(setting_value),"tool_override":j.tool_override.as_deref().map(junction_value)})
}
pub(crate) fn junction(n: Node<'_>, depth: usize) -> Result<JunctionConfig> {
    if depth > 64 {
        return Err(n.error("junction override nesting exceeds 64 levels"));
    }
    Ok(JunctionConfig {
        definition: optional_string(n.field("definition")?)?,
        invalid: n.field("invalid")?.nullable(setting)?,
        tool_override: n.field("tool_override")?.nullable(|n| Ok(Box::new(junction(n, depth + 1)?)))?,
    })
}

fn style_value(s: RunStyle) -> Value {
    json!({"level":s.level,"bold":s.bold,"italic":s.italic,"upright":s.upright,"overline":s.overline})
}
fn style(n: Node<'_>) -> Result<RunStyle> {
    let level = n.field("level")?;
    Ok(RunStyle {
        level: i8::try_from(level.integer()?).map_err(|_| level.error("script level exceeds i8"))?,
        bold: n.field("bold")?.boolean()?,
        italic: n.field("italic")?.boolean()?,
        upright: n.field("upright")?.boolean()?,
        overline: n.field("overline")?.boolean()?,
    })
}
pub(crate) fn run_value(r: &Run) -> Value {
    match r {
        Run::Text { text, style } => json!({"kind":"text","text":text,"style":style_value(*style)}),
        Run::Sqrt { radicand, style } => {
            json!({"kind":"sqrt","radicand":radicand.iter().map(run_value).collect::<Vec<_>>(),"style":style_value(*style)})
        }
        Run::Frac { num, den, style } => {
            json!({"kind":"frac","num":num.iter().map(run_value).collect::<Vec<_>>(),"den":den.iter().map(run_value).collect::<Vec<_>>(),"style":style_value(*style)})
        }
    }
}
pub(crate) fn run(n: Node<'_>, depth: usize) -> Result<Run> {
    if depth > 64 {
        return Err(n.error("markup nesting exceeds 64 levels"));
    }
    let style = style(n.field("style")?)?;
    Ok(match n.field("kind")?.string()? {
        "text" => Run::Text { text: n.field("text")?.string()?.to_owned(), style },
        "sqrt" => Run::Sqrt { radicand: n.field("radicand")?.list(|n| run(n, depth + 1))?, style },
        "frac" => Run::Frac {
            num: n.field("num")?.list(|n| run(n, depth + 1))?,
            den: n.field("den")?.list(|n| run(n, depth + 1))?,
            style,
        },
        _ => return Err(n.field("kind")?.error("unknown markup run kind")),
    })
}
