//! The TypeScript declarations of the Document in `types/document.d.ts`, of the snapshot in
//! `types/snapshot.d.ts` and of the catalog in `types/catalog.d.ts`, generated from the Rust types
//! by ts-rs, which the types derive in the tests only. The test fails when a file is not what the
//! types give; run it with `RNL_BLESS=1` to write the files after a change that is meant.

use std::path::Path;

use ts_rs::{Config, TS};

use crate::{
    Bar, Cap, Catalog, CatalogDefinition, CatalogLabel, CatalogPackage, CatalogPin, CatalogSet, CatalogSymbol, Child,
    Color, Diagnostic, Document, FileRef, HAlign, Hit, HostInstance, HostLevel, HostNet, HostTerminal, Join, Layer,
    Level, LevelModel, LevelSnapshot, Object, ObjectView, PackageUse, PinLabel, PinView, PointView, Primitive, Run,
    RunStyle, Segment, Shape, ShapeView, Slot, Snapshot, SourceFile, Span, Stroke, SymbolDefinition, SymbolPin,
    SymbolPrim, SymbolRef, Terminal, Text, TextRun, TextView, Unit, VAlign, Vertex, WireView,
};

const CATALOG_HEADER: &str = "\
// The catalog of the packages the core has (crates/rnl-view/src/catalog.rs) as TypeScript
// declarations, generated from its Rust types by ts-rs. Do not edit: after changing the types, run
//   RNL_BLESS=1 cargo test -p rnl-view types
// The catalog is plain JSON; these are its shapes, with nothing to load at run time.
";

/// The documentation and the declaration of the catalog and of everything in it.
fn catalog_all() -> Vec<(String, String)> {
    fn one<T: TS>(cfg: &Config) -> (String, String) {
        (T::docs().unwrap_or_default(), T::decl(cfg))
    }
    let cfg = Config::new();
    vec![
        one::<Catalog>(&cfg),
        one::<CatalogPackage>(&cfg),
        one::<CatalogSymbol>(&cfg),
        one::<CatalogPin>(&cfg),
        one::<Terminal>(&cfg),
        one::<CatalogLabel>(&cfg),
        one::<CatalogDefinition>(&cfg),
        one::<CatalogSet>(&cfg),
    ]
}

/// The text of `types/catalog.d.ts`.
fn catalog_declarations() -> String {
    let all: Vec<String> =
        catalog_all().iter().map(|(docs, decl)| format!("{docs}export {}\n", layout(decl))).collect();
    format!("{CATALOG_HEADER}\n{}", all.join("\n"))
}

const SNAPSHOT_HEADER: &str = "\
// The snapshot of the editor interface (crates/rnl-view/src/snapshot.rs) as TypeScript
// declarations, generated from its Rust types by ts-rs. Do not edit: after changing the types, run
//   RNL_BLESS=1 cargo test -p rnl-view types
// The snapshot is plain JSON; these are its shapes, with nothing to load at run time.

import type { Diagnostic, Document, Run } from \"./document.js\";
";

/// The documentation and the declaration of the snapshot and of everything in it.
fn snapshot_all() -> Vec<(String, String)> {
    fn one<T: TS>(cfg: &Config) -> (String, String) {
        (T::docs().unwrap_or_default(), T::decl(cfg))
    }
    let cfg = Config::new();
    vec![
        one::<Snapshot>(&cfg),
        one::<PackageUse>(&cfg),
        one::<FileRef>(&cfg),
        one::<LevelSnapshot>(&cfg),
        one::<HostLevel>(&cfg),
        one::<HostInstance>(&cfg),
        one::<HostTerminal>(&cfg),
        one::<HostNet>(&cfg),
        one::<LevelModel>(&cfg),
        one::<SymbolRef>(&cfg),
        one::<SymbolDefinition>(&cfg),
        one::<SymbolPin>(&cfg),
        one::<PinLabel>(&cfg),
        one::<SymbolPrim>(&cfg),
        one::<Slot<String>>(&cfg),
        one::<ObjectView>(&cfg),
        one::<PinView>(&cfg),
        one::<PointView>(&cfg),
        one::<WireView>(&cfg),
        one::<Vertex>(&cfg),
        one::<TextView>(&cfg),
        one::<ShapeView>(&cfg),
    ]
}

/// The text of `types/snapshot.d.ts`.
fn snapshot_declarations() -> String {
    let all: Vec<String> =
        snapshot_all().iter().map(|(docs, decl)| format!("{docs}export {}\n", layout(decl))).collect();
    format!("{SNAPSHOT_HEADER}\n{}", all.join("\n"))
}

const HEADER: &str = "\
// The Document of rnl-view (crates/rnl-view/README.md) as TypeScript declarations, generated from
// its Rust types by ts-rs. Do not edit: after changing the types, run
//   RNL_BLESS=1 cargo test -p rnl-view types
// The Document is plain JSON; these are its shapes, with nothing to load at run time.
";

/// The documentation and the declaration of the Document and of everything in it, as ts-rs gives
/// them: the Document and its parts, then the primitives.
fn all() -> Vec<(String, String)> {
    fn one<T: TS>(cfg: &Config) -> (String, String) {
        // ts-rs writes the shape of a flattened field in place; a primitive refers to it by name
        let decl = T::decl(cfg).replace(&Shape::inline_flattened(cfg), "Shape");
        (T::docs().unwrap_or_default(), decl)
    }
    let cfg = Config::new();
    vec![
        one::<Document>(&cfg),
        one::<Level>(&cfg),
        one::<Hit>(&cfg),
        one::<Child>(&cfg),
        one::<SourceFile>(&cfg),
        one::<Object>(&cfg),
        one::<Span>(&cfg),
        one::<Diagnostic>(&cfg),
        one::<Primitive>(&cfg),
        one::<Layer>(&cfg),
        one::<Shape>(&cfg),
        one::<Segment>(&cfg),
        one::<Color>(&cfg),
        one::<Stroke>(&cfg),
        one::<Unit>(&cfg),
        one::<Cap>(&cfg),
        one::<Join>(&cfg),
        one::<Text>(&cfg),
        one::<TextRun>(&cfg),
        one::<Bar>(&cfg),
        one::<HAlign>(&cfg),
        one::<VAlign>(&cfg),
        one::<Run>(&cfg),
        one::<RunStyle>(&cfg),
    ]
}

/// The text of `types/document.d.ts`.
fn declarations() -> String {
    let all: Vec<String> = all().iter().map(|(docs, decl)| format!("{docs}export {}\n", layout(decl))).collect();
    format!("{HEADER}\n{}", all.join("\n"))
}

/// A declaration laid out to be read: an object type with documented members one member a line,
/// other objects on one line, and a long union one alternative a line.
fn layout(decl: &str) -> String {
    let (head, body) = decl.split_once(" = ").unwrap();
    let tokens = tokens(body.strip_suffix(';').unwrap());
    let one_line = format!("{head} = {};", typescript(&tokens, 0));
    let alternatives = split(&tokens, '|');
    if alternatives.len() > 1 && one_line.len() > 100 {
        let lines: String = alternatives.iter().map(|a| format!("\n  | {}", typescript(a, 2))).collect();
        format!("{head} ={lines};")
    } else {
        one_line
    }
}

/// A piece of ts-rs's output.
#[derive(Clone, Debug, PartialEq)]
enum Token {
    /// A documentation comment, `/** … */`.
    Doc(String),
    /// A name or a string.
    Word(String),
    Mark(char),
}

const MARKS: &str = "{}[]()<>,:|&;=";

fn tokens(text: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut rest = text.trim_start();
    while let Some(c) = rest.chars().next() {
        let (n, token) = if rest.starts_with("/**") {
            let n = rest.find("*/").unwrap() + 2;
            (n, Token::Doc(rest[..n].to_string()))
        } else if c == '"' {
            let n = rest[1..].find('"').unwrap() + 2;
            (n, Token::Word(rest[..n].to_string()))
        } else if MARKS.contains(c) {
            (1, Token::Mark(c))
        } else {
            let n = rest.find(|x: char| x.is_whitespace() || MARKS.contains(x)).unwrap_or(rest.len());
            (n, Token::Word(rest[..n].to_string()))
        };
        out.push(token);
        rest = rest[n..].trim_start();
    }
    out
}

/// The tokens between the marks `at` that are not inside brackets.
fn split(tokens: &[Token], at: char) -> Vec<&[Token]> {
    let mut parts = Vec::new();
    let (mut depth, mut start) = (0, 0);
    for (k, t) in tokens.iter().enumerate() {
        match t {
            Token::Mark('{' | '[' | '(' | '<') => depth += 1,
            Token::Mark('}' | ']' | ')' | '>') => depth -= 1,
            Token::Mark(c) if *c == at && depth == 0 => {
                parts.push(&tokens[start..k]);
                start = k + 1;
            }
            _ => {}
        }
    }
    parts.push(&tokens[start..]);
    parts
}

/// The tokens as TypeScript, `indent` spaces in. An object type with a doc comment among its
/// members is written one member a line, with the comments; other objects on one line.
fn typescript(tokens: &[Token], indent: usize) -> String {
    let mut out = String::new();
    let mut k = 0;
    while k < tokens.len() {
        match &tokens[k] {
            Token::Mark('{') => {
                let mut depth = 0;
                let end = k + tokens[k..]
                    .iter()
                    .position(|t| {
                        depth += match t {
                            Token::Mark('{') => 1,
                            Token::Mark('}') => -1,
                            _ => 0,
                        };
                        depth == 0
                    })
                    .unwrap();
                let members: Vec<&[Token]> =
                    split(&tokens[k + 1..end], ',').into_iter().filter(|m| !m.is_empty()).collect();
                if members.iter().any(|m| matches!(m.first(), Some(Token::Doc(_)))) {
                    let pad = " ".repeat(indent + 2);
                    out.push_str("{\n");
                    for m in members {
                        let docs = m.iter().take_while(|t| matches!(t, Token::Doc(_))).count();
                        for doc in &m[..docs] {
                            let Token::Doc(doc) = doc else { unreachable!() };
                            for line in doc.lines().map(str::trim).filter(|l| !l.is_empty()) {
                                let space = if line.starts_with('*') { " " } else { "" };
                                out.push_str(&format!("{pad}{space}{line}\n"));
                            }
                        }
                        out.push_str(&format!("{pad}{},\n", typescript(&m[docs..], indent + 2)));
                    }
                    out.push_str(&" ".repeat(indent));
                    out.push('}');
                } else {
                    let members: Vec<String> = members.iter().map(|m| typescript(m, indent)).collect();
                    out.push_str(&format!("{{ {} }}", members.join(", ")));
                }
                k = end + 1;
                continue;
            }
            Token::Mark(',') => out.push_str(", "),
            Token::Mark(':') => out.push_str(": "),
            Token::Mark(c @ ('|' | '&' | '=')) => out.push_str(&format!(" {c} ")),
            Token::Mark(c) => out.push(*c),
            Token::Word(w) | Token::Doc(w) => out.push_str(w),
        }
        k += 1;
    }
    out
}

#[test]
fn the_type_declarations_are_those_of_the_rust_types() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("types/document.d.ts");
    let text = declarations();
    if std::env::var("RNL_BLESS").is_ok() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &text).unwrap();
        return;
    }
    let want = std::fs::read_to_string(&path).unwrap_or_default();
    // line endings may differ
    assert!(want.replace("\r\n", "\n") == text, "types/document.d.ts is out of date; run with RNL_BLESS=1");
}

#[test]
fn the_snapshot_declarations_are_those_of_the_rust_types() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("types/snapshot.d.ts");
    let text = snapshot_declarations();
    if std::env::var("RNL_BLESS").is_ok() {
        std::fs::write(&path, &text).unwrap();
        return;
    }
    let want = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(want.replace("\r\n", "\n") == text, "types/snapshot.d.ts is out of date; run with RNL_BLESS=1");
}

#[test]
fn the_catalog_declarations_are_those_of_the_rust_types() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("types/catalog.d.ts");
    let text = catalog_declarations();
    if std::env::var("RNL_BLESS").is_ok() {
        std::fs::write(&path, &text).unwrap();
        return;
    }
    let want = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(want.replace("\r\n", "\n") == text, "types/catalog.d.ts is out of date; run with RNL_BLESS=1");
}

#[test]
fn the_layout_changes_only_the_spacing() {
    // and the comma after the last member of an object on one line, and the `|` before the first
    // alternative of a union on several lines
    let words = |text: &str| -> Vec<Token> {
        let doc = |d: &str| d.lines().map(str::trim).filter(|l| !l.is_empty()).collect::<Vec<_>>().join("\n");
        let mut out: Vec<Token> = Vec::new();
        for t in tokens(text) {
            match t {
                Token::Mark(',') => {}
                Token::Mark('|') if out.last() == Some(&Token::Mark('=')) => {}
                Token::Doc(d) => out.push(Token::Doc(doc(&d))),
                t => out.push(t),
            }
        }
        out
    };
    let all = all();
    for (_, decl) in all.iter().chain(&catalog_all()) {
        assert_eq!(words(&layout(decl)), words(decl), "{decl}");
    }
    // a primitive's shape is named, not written in place
    assert!(all.iter().any(|(_, decl)| decl.starts_with("type Primitive = {") && decl.ends_with("} & Shape;")));
}
