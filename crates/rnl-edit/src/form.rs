//! The standard form of the statements an op becomes (4.3): the object first, positional
//! parameters before named ones, the shortest numbers, and no parameter that would not change the
//! result.

use crate::ops::{Attach, Literal, Op, Point, Run, RunStyle, TextContent, Vertex};

/// A number as written: rounded to a millionth, in the shortest form that reads back the same, as
/// JavaScript's `String(number)` writes it; never `-0`.
pub fn num(v: f64) -> String {
    let r = (v * 1e6).round() / 1e6;
    if r == 0.0 {
        "0".into()
    } else {
        format!("{r}")
    }
}

pub fn point([x, y]: Point) -> String {
    format!("({}, {})", num(x), num(y))
}

/// A string literal (3.7): in double quotes, a quote doubled. A line break would end the comment
/// that carries the record, so it becomes a space.
pub fn quote(s: &str) -> String {
    let body: String = s.chars().map(|c| if c == '\n' || c == '\r' { ' ' } else { c }).collect();
    format!("\"{}\"", body.replace('"', "\"\""))
}

/// Text markup (13.6) from lines of runs, which reads back as the same runs: each line's runs in
/// turn, lines apart by `\\`.
pub fn markup(lines: &[Vec<Run>]) -> String {
    lines.iter().map(|l| runs(l, RunStyle::default())).collect::<Vec<_>>().join("\\\\")
}

/// Runs inside a group whose style is `outer`: each run says only how it differs from it, as the
/// markup reader adds a group's style to what is inside it.
fn runs(list: &[Run], outer: RunStyle) -> String {
    list.iter()
        .map(|run| {
            let (mut body, style) = match run {
                Run::Text { text, style } => (escape(text), *style),
                Run::Frac { num, den, style } => {
                    (format!("\\frac{{{}}}{{{}}}", runs(num, *style), runs(den, *style)), *style)
                }
            };
            if style.italic && !outer.italic {
                body = format!("\\textit{{{body}}}");
            }
            if style.bold && !outer.bold {
                body = format!("\\textbf{{{body}}}");
            }
            if style.overline && !outer.overline {
                body = format!("\\overline{{{body}}}");
            }
            for _ in outer.level..style.level {
                body = format!("^{{{body}}}");
            }
            for _ in style.level..outer.level {
                body = format!("_{{{body}}}");
            }
            body
        })
        .collect()
}

/// Plain text as markup: the characters the reader takes as markup escaped, a backslash as
/// `\textbackslash{}`, whose braces keep a space after it.
fn escape(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\textbackslash{}"),
            '_' | '{' | '}' | '$' | '%' | '&' | '#' | '^' => {
                out.push('\\');
                out.push(c);
            }
            '\0' => {}
            c => out.push(c),
        }
    }
    out
}

fn literal(v: &Literal) -> String {
    match v {
        Literal::Str(s) => quote(s),
        Literal::Num(n) => num(*n),
        Literal::Bool(b) => b.to_string(),
    }
}

/// The facts of a position: `@PLACE` and `@ORIENT`, the default orientation R0 left out.
fn placed(at: &Option<Point>, orient: &Option<String>) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(p) = at {
        out.push(format!("@PLACE{}", point(*p)));
    }
    if let Some(o) = orient.as_deref().filter(|o| *o != "R0") {
        out.push(format!("@ORIENT({})", quote(o)));
    }
    out
}

/// The statements of a level op, in order (`use-package` writes none: it goes into the header).
pub fn statements(op: &Op) -> Vec<String> {
    match op {
        Op::Set { object, symbol, at, orient } => {
            let mut facts: Vec<String> = symbol.iter().map(|s| format!("@SET-SYMBOL {s}")).collect();
            facts.extend(placed(at, orient));
            vec![format!("{object} {}", facts.join(" => "))]
        }
        Op::AddUsage { name, definition, attach, symbol, at, orient } => {
            let roles: Vec<String> = attach
                .iter()
                .map(|a| match a {
                    Attach::Id(id) => id.clone(),
                    Attach::At(p) => point(*p),
                })
                .collect();
            let mut parts = vec![format!("{definition} @ATTACH {}", roles.join(" "))];
            parts.extend(symbol.iter().map(|s| format!("@SET-SYMBOL {s}")));
            parts.extend(placed(at, orient));
            parts.extend(name.iter().cloned());
            vec![parts.join(" => ")]
        }
        Op::AddPoint { name, at, junction } => {
            let mut out = vec![format!("{name} = {}", point(*at))];
            if *junction {
                out.push(format!("$JUNCTION @ATTACH {name}"));
            }
            out
        }
        Op::AddWire { name, nets, via, mode, style } => {
            let mut params: Vec<String> =
                mode.iter().filter(|m| *m != "auto").map(|m| format!("mode={}", quote(m))).collect();
            params.extend(style.iter().map(|(k, v)| format!("{k}={}", literal(v))));
            let params = if params.is_empty() { String::new() } else { format!("({})", params.join(", ")) };
            let vertices: Vec<String> = via
                .iter()
                .map(|v| match v {
                    Vertex::Terminal(t) => t.clone(),
                    Vertex::Point(p) => p.clone(),
                    Vertex::Usage { usage, role } => format!("{usage}:{role}"),
                    Vertex::At(p) => point(*p),
                })
                .collect();
            let wire = format!("{} @WIRE{params} {{{}}}", nets.join(" "), vertices.join(" "));
            vec![match name {
                Some(n) => format!("{wire} => {n}"),
                None => wire,
            }]
        }
        Op::AddText { name, attach, key, text, at, offset, halign, valign, rotate, size, style } => {
            let content = match text {
                TextContent::Markup(m) => m.clone(),
                TextContent::Lines(lines) => markup(lines),
            };
            let mut params = vec![quote(&content)];
            params.extend(key.iter().map(|k| format!("key={}", quote(k))));
            if let Some([dx, dy]) = offset {
                params.push(format!("dx={}, dy={}", num(*dx), num(*dy)));
            }
            params.extend(halign.iter().filter(|h| *h != "start").map(|h| format!("halign={}", quote(h))));
            params.extend(valign.iter().filter(|v| *v != "baseline").map(|v| format!("valign={}", quote(v))));
            params.extend(rotate.iter().filter(|r| **r != 0.0).map(|r| format!("rotate={}", num(*r))));
            params.extend(size.iter().map(|s| format!("size={}", num(*s))));
            params.extend(style.iter().map(|(k, v)| format!("{k}={}", literal(v))));
            let text = format!("@TEXT({})", params.join(", "));
            let mut out = match (attach, at) {
                (Some(o), Some(p)) => format!("{o} {text} => @PLACE{}", point(*p)),
                (Some(o), None) => format!("{o} {text}"),
                (None, Some(p)) => format!("{} {text}", point(*p)),
                (None, None) => unreachable!("an add-text without a place is refused when read"),
            };
            if let Some(n) = name {
                out = format!("{out} => {n}");
            }
            vec![out]
        }
        Op::Membership { add, items, set } => {
            vec![format!("{} @{} {set}", items.join(" "), if *add { "ADD-TO" } else { "REMOVE-FROM" })]
        }
        Op::UsePackage { .. } => Vec::new(),
    }
}

/// The USE-PACKAGE of a header record.
pub fn use_package(name: &str, version: Option<&str>, path: Option<&str>) -> String {
    let mut params = vec![format!("name={}", quote(name))];
    params.extend(version.map(|v| format!("version={}", quote(v))));
    params.extend(path.map(|p| format!("path={}", quote(p))));
    format!("@USE-PACKAGE({})", params.join(", "))
}

/// A statement as lines of at most `width` characters, broken only at spaces outside strings and
/// brackets; the lines after the first are indented by `indent` more. A stretch with no such
/// space stays longer.
pub fn wrap(statement: &str, width: usize, indent: usize) -> Vec<String> {
    let chars: Vec<char> = statement.chars().collect();
    // the places a line may break: spaces outside strings and outside ( ) and [ ]
    let mut breaks = Vec::new();
    let (mut in_string, mut depth) = (false, 0i32);
    for (k, &c) in chars.iter().enumerate() {
        match c {
            '"' => in_string = !in_string, // a doubled quote toggles twice
            '(' | '[' if !in_string => depth += 1,
            ')' | ']' if !in_string => depth -= 1,
            ' ' if !in_string && depth == 0 => breaks.push(k),
            _ => {}
        }
    }
    let mut lines = Vec::new();
    let mut start = 0;
    loop {
        let room = if lines.is_empty() { width } else { width.saturating_sub(indent) };
        if chars.len() - start <= room {
            break;
        }
        // the last break that fits, else the first one after
        let fit = breaks.iter().copied().rev().find(|&b| b > start && b - start <= room);
        let Some(at) = fit.or_else(|| breaks.iter().copied().find(|&b| b > start)) else { break };
        lines.push(chars[start..at].iter().collect::<String>());
        start = at + 1;
    }
    lines.push(chars[start..].iter().collect::<String>());
    let pad = " ".repeat(indent);
    lines.into_iter().enumerate().map(|(k, l)| if k == 0 { l } else { format!("{pad}{l}") }).collect()
}
