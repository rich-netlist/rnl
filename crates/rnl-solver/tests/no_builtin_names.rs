use std::fs;
use std::path::Path;

/// Scan Rust string tokens, skipping ordinary and nested block comments. Fixtures and the
/// include_str configuration data live outside src and are deliberately outside this rule.
fn literals(source: &str) -> Vec<String> {
    let bytes = source.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i..].starts_with(b"//") {
            i += 2;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
        } else if bytes[i..].starts_with(b"/*") {
            i += 2;
            let mut depth = 1;
            while i < bytes.len() && depth > 0 {
                if bytes[i..].starts_with(b"/*") {
                    depth += 1;
                    i += 2;
                } else if bytes[i..].starts_with(b"*/") {
                    depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
        } else if bytes[i] == b'r' {
            let mut opening = i + 1;
            while opening < bytes.len() && bytes[opening] == b'#' {
                opening += 1;
            }
            if opening < bytes.len() && bytes[opening] == b'"' {
                let end = format!("\"{}", "#".repeat(opening - i - 1));
                let start = opening + 1;
                let length = source[start..].find(&end).expect("Rust raw string terminates");
                out.push(source[start..start + length].into());
                i = start + length + end.len();
            } else {
                i += 1;
            }
        } else if bytes[i] == b'"' {
            i += 1;
            let mut literal = String::new();
            while i < bytes.len() && bytes[i] != b'"' {
                if bytes[i] == b'\\' {
                    i += 1;
                    if i + 2 < bytes.len() && bytes[i] == b'x' {
                        if let Ok(v) = u8::from_str_radix(&source[i + 1..i + 3], 16) {
                            literal.push(char::from(v));
                        }
                        i += 3;
                    } else if i + 1 < bytes.len() && bytes[i..].starts_with(b"u{") {
                        let start = i + 2;
                        let len = source[start..].find('}').expect("Rust unicode escape terminates");
                        if let Ok(v) = u32::from_str_radix(&source[start..start + len], 16) {
                            if let Some(c) = char::from_u32(v) {
                                literal.push(c);
                            }
                        }
                        i = start + len + 1;
                    } else if i < bytes.len() {
                        literal.push(char::from(bytes[i]));
                        i += 1;
                    }
                } else {
                    let c = source[i..].chars().next().unwrap();
                    literal.push(c);
                    i += c.len_utf8();
                }
            }
            out.push(literal);
            i += 1;
        } else {
            i += 1;
        }
    }
    out
}

fn inspect(path: &Path) {
    for entry in fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            inspect(&path);
        } else if path.extension().is_some_and(|e| e == "rs") {
            for literal in literals(&fs::read_to_string(&path).unwrap()) {
                assert!(
                    !literal
                        .as_bytes()
                        .windows(2)
                        .any(|p| p[0] == b'$' && (p[1].is_ascii_alphabetic() || p[1] == b'_')),
                    "solver source contains a built-in vocabulary name in {}: {literal}",
                    path.display()
                );
            }
        }
    }
}

#[test]
fn solver_source_string_literals_do_not_contain_vocabulary_names() {
    inspect(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"));
}

#[test]
fn token_scan_excludes_comments_and_preserves_raw_and_escaped_strings() {
    let values = literals(
        r###"// "$COMMENT"
/* "$IGNORED" /* nested */ */
let a = r#"$RAW"#;
let b = "\x24ESCAPED";
let c = "\u{24}UNICODE";
"###,
    );
    assert_eq!(values, ["$RAW", "$ESCAPED", "$UNICODE"]);
}
