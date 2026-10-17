//! The standard form of the statements an op becomes (4.3): the object first, positional
//! parameters before named ones, the shortest numbers, and no parameter that would not change the
//! result.

use crate::ops::{Attach, Literal, Op, PinOp, Point, Run, RunStyle, ShapeOp, TermKey, TextContent, Vertex};

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

/// What fills a role: an id, a point, or a set role's members in braces, `{#M3 #M9}`, `{}` when
/// it is left empty.
fn given(a: &Attach) -> String {
    match a {
        Attach::Id(id) => id.clone(),
        Attach::At(p) => point(*p),
        Attach::Set(members) => format!("{{{}}}", members.iter().map(given).collect::<Vec<_>>().join(" ")),
    }
}

/// A role in role parentheses: an ASCII name as it is, any other in quotes, which is the name
/// itself and never an alias (3.6, 16.5).
fn role_name(role: &str) -> String {
    let plain = role.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && role.chars().next().is_some_and(|c| !c.is_ascii_digit());
    if plain {
        role.to_string()
    } else {
        quote(role)
    }
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

fn content(text: &TextContent) -> String {
    match text {
        TextContent::Markup(m) => m.clone(),
        TextContent::Lines(lines) => markup(lines),
    }
}

/// A graphic (9.2.5): a rectangle, circle or ellipse given by its centre is written on that point;
/// the others with their points. Its parameters, then its style properties, in the order given.
pub fn shape(s: &ShapeOp) -> String {
    let params: Vec<String> = s.params.iter().chain(&s.style).map(|(k, v)| format!("{k}={}", literal(v))).collect();
    let keyword = format!(
        "@{}{}",
        s.shape.to_uppercase(),
        if params.is_empty() { String::new() } else { format!("({})", params.join(", ")) }
    );
    if s.points.len() == 1 {
        format!("{} {keyword}", point(s.points[0]))
    } else {
        format!("{keyword} {{{}}}", s.points.iter().map(|p| point(*p)).collect::<Vec<_>>().join(" "))
    }
}

/// A pin (9.2.4): `terminal=` a number, a name, or a list of them for a bus pin.
fn pin(p: &PinOp) -> String {
    let key = |t: &TermKey| match t {
        TermKey::Num(n) => n.to_string(),
        TermKey::Name(s) => quote(s),
    };
    let terminal = match p.terminals.as_slice() {
        [one] => key(one),
        many => format!("[{}]", many.iter().map(key).collect::<Vec<_>>().join(", ")),
    };
    let mut params = vec![format!("terminal={terminal}")];
    params.extend(p.label.iter().map(|l| format!("label={}", quote(&content(l)))));
    params.extend(p.direction.iter().map(|d| format!("direction={}", quote(d))));
    format!("{} @PIN({})", point(p.at), params.join(", "))
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
            // each object with the role it fills, so that the roles are those of the request
            // whatever order it writes them in, and a role the definition lacks fails (9.4)
            let roles: Vec<String> =
                attach.iter().map(|(role, a)| format!("{}({})", given(a), role_name(role))).collect();
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
            // several nets are one set, which the net slot takes whole: a bus (5.2.1, 13.1)
            let nets = match nets.as_slice() {
                [one] => one.clone(),
                _ => format!("{{{}}}", nets.join(" ")),
            };
            let wire = format!("{nets} @WIRE{params} {{{}}}", vertices.join(" "));
            vec![match name {
                Some(n) => format!("{wire} => {n}"),
                None => wire,
            }]
        }
        Op::AddText { name, attach, key, text, at, offset, halign, valign, rotate, size, style } => {
            let mut params = vec![quote(&content(text))];
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
        // several items are one set, whose members are added or removed (5.2.2)
        Op::Membership { add, items, set } => {
            let what = match items.as_slice() {
                [one] => one.clone(),
                _ => format!("{{{}}}", items.join(" ")),
            };
            vec![format!("{what} @{} {set}", if *add { "ADD-TO" } else { "REMOVE-FROM" })]
        }
        Op::UsePackage { .. } => Vec::new(),
        Op::AddShape { name, shape: s, layer } => {
            let mut parts = vec![shape(s)];
            parts.extend(layer.iter().map(|l| format!("@LAYER({})", quote(l))));
            parts.extend(name.iter().cloned());
            vec![parts.join(" => ")]
        }
        // a statement of several lines: its body one statement a line, indented by two
        Op::DefineSymbol { name, bbox, pins, prims } => {
            let params = bbox.map(|b| format!("(box=[{}])", b.map(num).join(", "))).unwrap_or_default();
            let body: Vec<String> = pins.iter().map(pin).chain(prims.iter().map(shape)).collect();
            let mut lines = vec![format!("{name} = @DEFINE-SYMBOL{params} {{")];
            for (k, s) in body.iter().enumerate() {
                lines.push(format!("  {s}{}", if k + 1 < body.len() { ";" } else { "" }));
            }
            lines.push("}".into());
            vec![lines.join("\n")]
        }
    }
}

/// The USE-PACKAGE of a header record.
pub fn use_package(name: &str, version: Option<&str>, path: Option<&str>) -> String {
    let mut params = vec![format!("name={}", quote(name))];
    params.extend(version.map(|v| format!("version={}", quote(v))));
    params.extend(path.map(|p| format!("path={}", quote(p))));
    format!("@USE-PACKAGE({})", params.join(", "))
}

/// A statement as lines of at most `width` characters, broken only at spaces outside strings: best
/// outside brackets, else after a comma inside them, such as between the parameters of a long
/// TEXT; the lines after the first are indented by `indent` more. A stretch with no such space
/// stays longer.
pub fn wrap(statement: &str, width: usize, indent: usize) -> Vec<String> {
    let chars: Vec<char> = statement.chars().collect();
    // the places a line may break: spaces outside strings and outside ( ) and [ ], and second
    // best, spaces after a comma inside them
    let (mut outside, mut inside) = (Vec::new(), Vec::new());
    let (mut in_string, mut depth) = (false, 0i32);
    for (k, &c) in chars.iter().enumerate() {
        match c {
            '"' => in_string = !in_string, // a doubled quote toggles twice
            '(' | '[' if !in_string => depth += 1,
            ')' | ']' if !in_string => depth -= 1,
            ' ' if !in_string && depth == 0 => outside.push(k),
            ' ' if !in_string && k > 0 && chars[k - 1] == ',' => inside.push(k),
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
        // the last break that fits, outside brackets before inside; else the first one after
        let fits = |breaks: &[usize]| breaks.iter().copied().rev().find(|&b| b > start && b - start <= room);
        let after = |breaks: &[usize]| breaks.iter().copied().find(|&b| b > start);
        let Some(at) = fits(&outside).or_else(|| fits(&inside)).or_else(|| after(&outside)).or_else(|| after(&inside))
        else {
            break;
        };
        lines.push(chars[start..at].iter().collect::<String>());
        start = at + 1;
    }
    lines.push(chars[start..].iter().collect::<String>());
    let pad = " ".repeat(indent);
    lines.into_iter().enumerate().map(|(k, l)| if k == 0 { l } else { format!("{pad}{l}") }).collect()
}
