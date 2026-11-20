//! Verilog-A files that a netlist brings in (2.9): Spectre's `ahdl_include` and HSPICE's `.hdl`.
//!
//! The adapters only note where a file is brought in ([`Reading::include_hdl`]); once the language
//! version of every file is known, the files that a file of version 0.4 brings in are read here, so
//! that a file of an earlier version reads as it always has. The text outside the modules is the
//! file level of the Verilog-A file, below the level whose text brings it in; each `module …
//! endmodule` is a level below it, read as a subcircuit with no circuit: its terminals are its
//! ports, a vector port one terminal for each bit, and its `# @SET-SYMBOL` gives the block symbol
//! of the instances of it (9.2.6). RNL is read from whole-line `//` comments in column 1, line 1
//! included; other comments, and the text between `` `ifdef `` or `` `ifndef `` and its
//! `` `endif ``, are not read. The instances of modules that a module writes are noted for the
//! outline ([`Level::module_instances`]); they make no circuit.

use crate::diag::{codes, Code, Loc};
use crate::host::format::Reading;
use crate::host::{file_name, Dialect, FileFormat, Instance, Level, LevelId, LevelKind};
use crate::records::{scan_run_0_4, CarrierLine};
use crate::source::resolve_include;

/// The name of the format of Verilog-A files, as the document gives it.
pub const NAME: &str = "veriloga";

/// The rules of Verilog-A names: they compare by case (2.9).
pub fn dialect() -> Dialect {
    Dialect { name: NAME.into(), case_sensitive: true, ground_aliases: Vec::new() }
}

/// A Verilog-A file brought in by the line `no` of the file `fname` (path `path`), whose text is of
/// the level `cur`: its name as written, and the name `-master` gives its module.
#[derive(Clone, Debug)]
pub struct HdlInclude {
    pub target: String,
    pub master: Option<String>,
    pub no: u32,
    pub fname: String,
    pub path: String,
    pub cur: LevelId,
}

/// A module of a Verilog-A file: its name, the lines of its `module` and `endmodule`, its
/// terminals in the order of its port list, and the instances of modules it writes.
#[derive(Clone, Debug, PartialEq)]
pub struct Module {
    pub name: String,
    pub line: u32,
    pub end_line: u32,
    pub terminals: Vec<String>,
    pub instances: Vec<ModuleInstance>,
}

/// An instance of a module that a module writes, `amp #(.g(2)) u1 (a, b);`: its name, the module
/// it names and the line of its name.
#[derive(Clone, Debug, PartialEq)]
pub struct ModuleInstance {
    pub name: String,
    pub master: String,
    pub line: u32,
}

/// What a Verilog-A file holds for RNL: its modules, its runs of carrier lines in the order of the
/// text, and the lines whose `@RNL` is not read, with why.
#[derive(Clone, Debug, Default)]
pub struct Parsed {
    pub modules: Vec<Module>,
    pub runs: Vec<Vec<CarrierLine>>,
    pub unread: Vec<(Code, &'static str, u32)>,
}

const NO_SPACE: &str =
    "no space after the comment marker: this line is not a carrier and its @RNL is not a record entry (2.2)";
const HOST_TEXT: &str = "@RNL in an end-of-line comment or a host statement is host text (2.2)";
const BRANCH: &str = "RNL in an inactive or unresolved preprocessing branch is not read (2.2)";

/// Read the Verilog-A file that `h` brings in, into the document of `cx`: a file level below the
/// level that brings it in, and a level for each of its modules, known by its name, or by the name
/// of `-master` when the file has one module. A compiled model, an `.osdi` file, has no text to
/// read; a file that cannot be found gives a warning, and its modules no block symbol.
pub fn read(cx: &mut Reading, h: &HdlInclude) {
    let at = Loc::line(&h.fname, h.no);
    if file_name(&h.target).to_lowercase().ends_with(".osdi") {
        return;
    }
    let full = resolve_include(cx.provider, &h.path, &h.target, cx.sink, at.clone());
    let Some(text) = cx.provider.read(&full) else {
        cx.sink.warning(
            codes::HOST_VERILOGA_NOT_FOUND,
            format!("the Verilog-A file {} cannot be found; its modules give no block symbol (2.9)", h.target),
            Some(at),
        );
        return;
    };
    let display = cx.display(&full);
    if !cx.doc.sources.iter().any(|(f, _)| *f == display) {
        cx.doc.sources.push((display.clone(), text.clone()));
    }
    if !cx.doc.file_formats.iter().any(|f| f.file == display) {
        cx.doc.file_formats.push(FileFormat { file: display.clone(), format: NAME.into(), default: NAME.into() });
    }
    cx.doc.file_dialects.entry(display.clone()).or_insert_with(dialect);
    let parsed = parse(&text);
    let mut file = Level::new(LevelKind::File, file_name(&display), &display, &full);
    file.line = h.no;
    file.include_site = Some(at);
    file.case_sensitive = true;
    file.end_line = text.lines().count() as u32 + 1;
    let file = cx.doc.add_level(file, Some(h.cur));
    let renamed = h.master.as_ref().filter(|_| parsed.modules.len() == 1);
    let mut modules = Vec::new();
    for m in &parsed.modules {
        let mut lv = Level::new(LevelKind::Subckt, renamed.unwrap_or(&m.name), &display, &full);
        lv.line = m.line;
        lv.end_line = m.end_line;
        lv.ports = m.terminals.clone();
        lv.case_sensitive = true;
        lv.module_instances = m
            .instances
            .iter()
            .enumerate()
            .map(|(order, i)| Instance {
                name: i.name.clone(),
                model: Some(i.master.clone()),
                file: display.clone(),
                line: i.line,
                order,
                is_subckt: true,
                case_sensitive: true,
                ..Instance::default()
            })
            .collect();
        let id = cx.doc.add_level(lv, Some(file));
        cx.doc.add_subckt(id);
        modules.push((m.line, m.end_line, id));
    }
    for (code, message, line) in parsed.unread {
        cx.sink.warning(code, message, Some(Loc::line(&display, line)));
    }
    for run in &parsed.runs {
        let first = run[0].line;
        let level = modules.iter().find(|(a, b, _)| *a <= first && first <= *b).map_or(file, |m| m.2);
        let records = scan_run_0_4(run, &display, cx.sink);
        cx.doc.levels[level].records.extend(records);
    }
}

/// The modules and the RNL carriers of the text of a Verilog-A file.
pub fn parse(text: &str) -> Parsed {
    let mut parsed = Parsed { modules: modules(&tokens(text)), ..Parsed::default() };
    let mut run: Vec<CarrierLine> = Vec::new();
    let mut in_comment = false;
    let mut branches = 0usize;
    let mut inactive = std::collections::HashSet::new();
    for (k, line) in text.lines().enumerate() {
        let no = k as u32 + 1;
        if branches > 0 {
            inactive.insert(no);
        }
        if !in_comment && branches == 0 {
            if let Some(rest) = line.strip_prefix("//") {
                if rest.trim().is_empty() || rest.starts_with(' ') {
                    run.push(CarrierLine { line: no, content: rest.to_string(), col: 2 });
                } else {
                    flush(&mut parsed.runs, &mut run);
                    if line.contains("@RNL") {
                        parsed.unread.push((codes::HOST_CARRIER_NO_SPACE, NO_SPACE, no));
                    }
                }
                continue;
            }
        }
        flush(&mut parsed.runs, &mut run);
        let directive = (!in_comment).then(|| line.trim_start().strip_prefix('`')).flatten();
        let word: String =
            directive.unwrap_or("").chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
        let opens = matches!(word.as_str(), "ifdef" | "ifndef");
        if opens {
            branches += 1;
        }
        if line.contains("@RNL") {
            if branches > 0 {
                parsed.unread.push((codes::HOST_RNL_IN_INACTIVE_BRANCH, BRANCH, no));
            } else {
                parsed.unread.push((codes::HOST_RNL_IN_HOST_STATEMENT, HOST_TEXT, no));
            }
        }
        if word == "endif" {
            branches = branches.saturating_sub(1);
        }
        in_comment = comment_open_after(line, in_comment);
    }
    flush(&mut parsed.runs, &mut run);
    // an instance the preprocessor may leave out is not one
    for m in &mut parsed.modules {
        m.instances.retain(|i| !inactive.contains(&i.line));
    }
    parsed
}

fn flush(runs: &mut Vec<Vec<CarrierLine>>, run: &mut Vec<CarrierLine>) {
    if !run.is_empty() {
        runs.push(std::mem::take(run));
    }
}

/// Whether a `/* */` comment is open at the end of `line`, given whether one is open at its start.
fn comment_open_after(line: &str, mut open: bool) -> bool {
    let c: Vec<char> = line.chars().collect();
    let mut i = 0;
    let mut string = false;
    while i < c.len() {
        if open {
            if c[i] == '*' && c.get(i + 1) == Some(&'/') {
                open = false;
                i += 1;
            }
        } else if string {
            if c[i] == '\\' {
                i += 1;
            } else if c[i] == '"' {
                string = false;
            }
        } else if c[i] == '"' {
            string = true;
        } else if c[i] == '/' && c.get(i + 1) == Some(&'/') {
            break;
        } else if c[i] == '/' && c.get(i + 1) == Some(&'*') {
            open = true;
            i += 1;
        }
        i += 1;
    }
    open
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Ident(String),
    Num(String),
    Punct(char),
}

/// The tokens of Verilog-A text, each with its line, without comments, strings, attributes and
/// compiler directives (a `` `define `` with its continued lines).
fn tokens(text: &str) -> Vec<(Tok, u32)> {
    let c: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut line = 1u32;
    let mut i = 0;
    let ident_char = |ch: char| ch.is_ascii_alphanumeric() || ch == '_' || ch == '$';
    while i < c.len() {
        let ch = c[i];
        if ch == '\n' {
            line += 1;
            i += 1;
        } else if ch.is_whitespace() {
            i += 1;
        } else if ch == '/' && c.get(i + 1) == Some(&'/') {
            while i < c.len() && c[i] != '\n' {
                i += 1;
            }
        } else if ch == '/' && c.get(i + 1) == Some(&'*') {
            i += 2;
            while i < c.len() && !(c[i] == '*' && c.get(i + 1) == Some(&'/')) {
                if c[i] == '\n' {
                    line += 1;
                }
                i += 1;
            }
            i += 2;
        } else if ch == '(' && c.get(i + 1) == Some(&'*') && c.get(i + 2) != Some(&')') {
            i += 2;
            while i < c.len() && !(c[i] == '*' && c.get(i + 1) == Some(&')')) {
                if c[i] == '\n' {
                    line += 1;
                }
                i += 1;
            }
            i += 2;
        } else if ch == '"' {
            i += 1;
            while i < c.len() && c[i] != '"' && c[i] != '\n' {
                if c[i] == '\\' {
                    i += 1;
                }
                i += 1;
            }
            i += 1;
        } else if ch == '`' {
            let start = i + 1;
            i += 1;
            while i < c.len() && ident_char(c[i]) {
                i += 1;
            }
            let word: String = c[start..i].iter().collect();
            if matches!(word.as_str(), "ifdef" | "ifndef" | "elsif" | "undef") {
                // the name of the macro is part of the directive
                while i < c.len() && (c[i] == ' ' || c[i] == '\t') {
                    i += 1;
                }
                while i < c.len() && ident_char(c[i]) {
                    i += 1;
                }
            } else if word == "define" {
                // to the end of the line, and the lines its `\` continues
                while i < c.len() && c[i] != '\n' {
                    if c[i] == '\\' && c.get(i + 1) == Some(&'\n') {
                        line += 1;
                        i += 1;
                    }
                    i += 1;
                }
            }
        } else if ch == '\\' {
            let start = i + 1;
            i += 1;
            while i < c.len() && !c[i].is_whitespace() {
                i += 1;
            }
            out.push((Tok::Ident(c[start..i].iter().collect()), line));
        } else if ch.is_ascii_alphabetic() || ch == '_' {
            let start = i;
            while i < c.len() && ident_char(c[i]) {
                i += 1;
            }
            out.push((Tok::Ident(c[start..i].iter().collect()), line));
        } else if ch.is_ascii_digit() {
            let start = i;
            while i < c.len() && (c[i].is_ascii_alphanumeric() || matches!(c[i], '_' | '.' | '\'')) {
                i += 1;
            }
            out.push((Tok::Num(c[start..i].iter().collect()), line));
        } else {
            out.push((Tok::Punct(ch), line));
            i += 1;
        }
    }
    out
}

fn is_ident(t: Option<&(Tok, u32)>, word: &str) -> bool {
    matches!(t, Some((Tok::Ident(w), _)) if w == word)
}

fn is_punct(t: Option<&(Tok, u32)>, p: char) -> bool {
    matches!(t, Some((Tok::Punct(q), _)) if *q == p)
}

/// The index past the bracket that closes the one at `open`.
fn past_close(toks: &[(Tok, u32)], open: usize) -> usize {
    let mut depth = 0i32;
    let mut i = open;
    while i < toks.len() {
        match &toks[i].0 {
            Tok::Punct('(' | '[' | '{') => depth += 1,
            Tok::Punct(')' | ']' | '}') => {
                depth -= 1;
                if depth == 0 {
                    return i + 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    toks.len()
}

/// The parts of `toks` between the commas outside brackets.
fn split_commas(toks: &[(Tok, u32)]) -> Vec<&[(Tok, u32)]> {
    let mut parts = Vec::new();
    let (mut depth, mut start) = (0i32, 0);
    for (i, (t, _)) in toks.iter().enumerate() {
        match t {
            Tok::Punct('(' | '[' | '{') => depth += 1,
            Tok::Punct(')' | ']' | '}') => depth -= 1,
            Tok::Punct(',') if depth == 0 => {
                parts.push(&toks[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&toks[start..]);
    parts
}

/// The first range `[a:b]` of integer bounds in `toks`.
fn range(toks: &[(Tok, u32)]) -> Option<(i64, i64)> {
    toks.windows(5).find_map(|w| match (&w[0].0, &w[1].0, &w[2].0, &w[3].0, &w[4].0) {
        (Tok::Punct('['), Tok::Num(a), Tok::Punct(':'), Tok::Num(b), Tok::Punct(']')) => {
            Some((a.replace('_', "").parse().ok()?, b.replace('_', "").parse().ok()?))
        }
        _ => None,
    })
}

/// The last name of a declaration's part outside brackets: the port or net it declares.
fn declared_name(part: &[(Tok, u32)]) -> Option<String> {
    let mut depth = 0i32;
    let mut name = None;
    for (t, _) in part {
        match t {
            Tok::Punct('(' | '[' | '{') => depth += 1,
            Tok::Punct(')' | ']' | '}') => depth -= 1,
            Tok::Ident(w) if depth == 0 => name = Some(w.clone()),
            _ => {}
        }
    }
    name
}

const DIRECTIONS: [&str; 3] = ["input", "output", "inout"];

/// The words that begin a statement of a module that is not an instance.
const NOT_MODULES: [&str; 31] = [
    "analog",
    "initial",
    "always",
    "parameter",
    "localparam",
    "aliasparam",
    "defparam",
    "real",
    "integer",
    "string",
    "genvar",
    "branch",
    "wire",
    "wreal",
    "reg",
    "ground",
    "supply0",
    "supply1",
    "assign",
    "if",
    "else",
    "for",
    "while",
    "repeat",
    "case",
    "casex",
    "casez",
    "default",
    "generate",
    "task",
    "function",
];

/// The instances a statement of a module writes, `amp #(…) u1 [range] (…), u2 (…)`: none when it
/// is not an instance of a module.
fn instances_in(statement: &[(Tok, u32)]) -> Vec<ModuleInstance> {
    let Some((Tok::Ident(master), _)) = statement.first() else { return Vec::new() };
    if NOT_MODULES.contains(&master.as_str()) || DIRECTIONS.contains(&master.as_str()) {
        return Vec::new();
    }
    let mut i = 1;
    if is_punct(statement.get(i), '#') && is_punct(statement.get(i + 1), '(') {
        i = past_close(statement, i + 1);
    }
    let mut out = Vec::new();
    for part in split_commas(&statement[i.min(statement.len())..]) {
        let Some((Tok::Ident(name), line)) = part.first() else { return Vec::new() };
        let mut k = 1;
        if is_punct(part.get(k), '[') {
            k = past_close(part, k);
        }
        if !is_punct(part.get(k), '(') || past_close(part, k) != part.len() {
            return Vec::new();
        }
        out.push(ModuleInstance { name: name.clone(), master: master.clone(), line: *line });
    }
    out
}

/// The modules of a Verilog-A file, with their terminals.
fn modules(toks: &[(Tok, u32)]) -> Vec<Module> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < toks.len() {
        if !(is_ident(toks.get(i), "module") || is_ident(toks.get(i), "macromodule")) {
            i += 1;
            continue;
        }
        let line = toks[i].1;
        let Some((Tok::Ident(name), _)) = toks.get(i + 1) else {
            i += 1;
            continue;
        };
        let name = name.clone();
        i += 2;
        if is_punct(toks.get(i), '#') && is_punct(toks.get(i + 1), '(') {
            i = past_close(toks, i + 1);
        }
        // the ports in order, each with the range written with it
        let mut ports: Vec<(String, Option<(i64, i64)>)> = Vec::new();
        if is_punct(toks.get(i), '(') {
            let close = past_close(toks, i);
            let inner = &toks[i + 1..close.saturating_sub(1).max(i + 1)];
            let ansi = inner.iter().any(|(t, _)| matches!(t, Tok::Ident(w) if DIRECTIONS.contains(&w.as_str())));
            let mut last = None;
            for part in split_commas(inner) {
                let Some(port) = declared_name(part) else { continue };
                let declares = part.len() > 1;
                let r = if ansi && !declares { last } else { range(part) };
                if declares {
                    last = r;
                }
                ports.push((port, r));
            }
            i = close;
        }
        // the body, to `endmodule`; the ranges of the declarations of the ports in it
        let end = (i..toks.len()).find(|&k| is_ident(toks.get(k), "endmodule")).unwrap_or(toks.len());
        let end_line = toks.get(end).map_or_else(|| toks.last().map_or(line, |t| t.1), |t| t.1);
        let mut k = i;
        let mut in_function = false;
        let mut instances = Vec::new();
        while k < end {
            let start = k;
            while k < end && !is_punct(toks.get(k), ';') {
                k += 1;
            }
            let mut statement = &toks[start..k];
            k += 1;
            // the words that end a block may come before a declaration, with no `;` after them
            while let Some((Tok::Ident(w), _)) = statement.first() {
                if w.starts_with("end") || w == "begin" {
                    in_function &= w != "endfunction";
                    statement = &statement[1..];
                } else {
                    break;
                }
            }
            if statement.iter().any(|(t, _)| matches!(t, Tok::Ident(w) if w == "function")) {
                in_function = true;
            }
            if in_function {
                if statement.iter().any(|(t, _)| matches!(t, Tok::Ident(w) if w == "endfunction")) {
                    in_function = false;
                }
                continue;
            }
            let written = instances_in(statement);
            if !written.is_empty() {
                instances.extend(written);
                continue;
            }
            let Some((Tok::Ident(first), _)) = statement.first() else { continue };
            let direction = DIRECTIONS.contains(&first.as_str());
            // a direction, or a discipline or net type followed by a range
            if !direction && !is_punct(statement.get(1), '[') {
                continue;
            }
            let Some(r) = range(statement) else { continue };
            for part in split_commas(statement) {
                let Some(n) = declared_name(part) else { continue };
                if let Some(port) = ports.iter_mut().find(|(p, _)| *p == n) {
                    if direction || port.1.is_none() {
                        port.1 = Some(r);
                    }
                }
            }
        }
        let terminals = ports
            .into_iter()
            .flat_map(|(p, r)| match r {
                None => vec![p],
                Some((a, b)) => {
                    let bits: Vec<i64> = if a >= b { (b..=a).rev().collect() } else { (a..=b).collect() };
                    bits.into_iter().map(|k| format!("{p}[{k}]")).collect()
                }
            })
            .collect();
        out.push(Module { name, line, end_line, terminals, instances });
        i = end + 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn terminals(text: &str) -> Vec<(String, Vec<String>)> {
        parse(text).modules.into_iter().map(|m| (m.name, m.terminals)).collect()
    }

    #[test]
    fn ports_in_order_with_vectors_from_left_to_right_bound() {
        let text = "`include \"disciplines.vams\"\nmodule m(a, d, e, out);\n  inout a, out;\n  inout [3:0] d;\n  input [0:2] e;\n  electrical a, out;\n  electrical [3:0] d;\n  electrical [0:2] e;\n  analog begin\n    V(out) <+ V(a);\n  end\nendmodule\n";
        assert_eq!(
            terminals(text),
            [(
                "m".to_string(),
                ["a", "d[3]", "d[2]", "d[1]", "d[0]", "e[0]", "e[1]", "e[2]", "out"].map(String::from).to_vec()
            )]
        );
    }

    #[test]
    fn ansi_ports_inherit_the_declaration_before_them() {
        let text = "module m #(parameter real g = 1) (inout electrical [1:0] p, q, output electrical o);\nendmodule\nmacromodule n;\nendmodule\n";
        assert_eq!(
            terminals(text),
            [
                ("m".to_string(), ["p[1]", "p[0]", "q[1]", "q[0]", "o"].map(String::from).to_vec()),
                ("n".to_string(), Vec::new())
            ]
        );
    }

    #[test]
    fn a_range_written_only_on_the_discipline_counts() {
        let text = "module m(d);\n  inout d;\n  electrical [2:1] d;\n  analog function real f;\n    input [7:0] d;\n  endfunction\nendmodule\n";
        assert_eq!(terminals(text), [("m".to_string(), ["d[2]", "d[1]"].map(String::from).to_vec())]);
    }

    #[test]
    fn carriers_are_whole_line_comments_in_column_1_outside_conditionals() {
        let text = "// @RNL(type=\"header\") { @VERSION(\"0.4\")}\nmodule m(a);\n  // @RNL { indented }\n/* @RNL { block } */\n/*\n// @RNL { in a block }\n*/\n`ifdef X\n// @RNL { conditional }\n`endif\n//@RNL { tight }\n// @RNL {\n//   # @SET-SYMBOL $R\n// }\nendmodule\n";
        let parsed = parse(text);
        let firsts: Vec<u32> = parsed.runs.iter().map(|r| r[0].line).collect();
        assert_eq!(firsts, [1, 12]);
        assert_eq!(parsed.runs[1].len(), 3);
        let unread: Vec<u32> = parsed.unread.iter().map(|u| u.2).collect();
        assert_eq!(unread, [3, 4, 6, 9, 11]);
        assert_eq!(parsed.unread[3].0, codes::HOST_RNL_IN_INACTIVE_BRANCH);
        assert_eq!(parsed.unread[4].0, codes::HOST_CARRIER_NO_SPACE);
        assert_eq!((parsed.modules[0].line, parsed.modules[0].end_line), (2, 15));
    }

    #[test]
    fn the_instances_a_module_writes_by_name_module_and_line() {
        let text = "module top(x, y);
  inout x, y;
  electrical x, y, mid;
  electrical [1:0] v;
  parameter real g = 1 from (0:inf);
  gain #(.k(g), .n(2)) u1 (x, mid);
  gain u2 (.a(mid), .b(y)), u3 (mid, y);
  bank u4 [1:0] (v, x);
`ifdef DEBUG
  probe p1 (x);
`else
  probe p2 (y);
`endif
`ifndef QUIET
`endif
  gain u5 (y, x);
  analog begin
    V(mid) <+ 0;
    if (g > 1) V(y) <+ g * V(x);
  end
  analog function real f;
    input a;
    f = a;
  endfunction
endmodule
module gain(a, b);
endmodule
";
        let parsed = parse(text);
        let written: Vec<(&str, &str, u32)> =
            parsed.modules[0].instances.iter().map(|i| (i.name.as_str(), i.master.as_str(), i.line)).collect();
        // those the preprocessor may leave out are not among them
        assert_eq!(
            written,
            [("u1", "gain", 6), ("u2", "gain", 7), ("u3", "gain", 7), ("u4", "bank", 8), ("u5", "gain", 16)]
        );
        assert!(parsed.modules[1].instances.is_empty());
        // the terminals are as before
        assert_eq!(parsed.modules[0].terminals, ["x", "y"]);
    }
}
