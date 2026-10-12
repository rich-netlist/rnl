//! Cutting records out of carrier lines (spec-draft 2.3–2.5, 2.7).
//!
//! A host adapter decides which lines are carriers and strips the comment marker and the space
//! after it; what is left of a run of consecutive carrier lines is handed here. This part is the
//! same for every host: the record entry, braces, strings, comments and the disabling `/* */`.

use crate::diag::{Loc, Sink};
use crate::host::{Record, Seg};

/// One carrier line: its number, its content after the comment marker, and the 0-based column
/// of the content's first character in the host line.
#[derive(Clone, Debug)]
pub struct CarrierLine {
    pub line: u32,
    pub content: String,
    pub col: i64,
}

fn name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Cut the records out of a run of consecutive carrier lines of `file`.
pub fn scan_run(run: &[CarrierLine], file: &str, sink: &mut Sink) -> Vec<Record> {
    let mut segs = Vec::new();
    let mut chars: Vec<char> = Vec::new();
    for (k, cl) in run.iter().enumerate() {
        if k > 0 {
            chars.push('\n');
        }
        segs.push(Seg { off: chars.len(), file: file.to_string(), line: cl.line, col: cl.col });
        chars.extend(cl.content.chars());
    }
    let t = chars;
    let n = t.len();
    let starts = |at: usize, s: &str| -> bool { s.chars().enumerate().all(|(k, c)| t.get(at + k) == Some(&c)) };

    let find = |from: usize, s: &str| -> Option<usize> { (from..n).find(|&j| starts(j, s)) };
    let line_of = |pos: usize| -> Loc {
        let mut seg = &segs[0];
        for s in &segs {
            if s.off <= pos {
                seg = s;
            }
        }
        Loc::new(file, seg.line, (seg.col + pos as i64 - seg.off as i64 + 1).max(1) as u32)
    };
    let mut out = Vec::new();
    let mut i = 0usize;
    let mut disabled = false;
    let mut after_record = false;
    while i < n {
        if disabled {
            match find(i, "*/") {
                Some(j) => {
                    i = j + 2;
                    disabled = false;
                    continue;
                }
                None => return out,
            }
        }
        let c = t[i];
        if c == '\n' {
            after_record = false;
            i += 1;
            continue;
        }
        if starts(i, "/*") {
            // `/*` at the start of a line or right after a record disables what follows (2.5)
            let line_start = (0..i).rev().find(|&k| t[k] == '\n').map(|k| k + 1).unwrap_or(0);
            let before: String = t[line_start..i].iter().collect();
            if after_record || before.trim().is_empty() {
                disabled = true;
            }
            i += 2;
            continue;
        }
        if starts(i, "@RNL") {
            let glued = i + 4 < n && name_char(t[i + 4]);
            if i == 0 || t[i - 1] != ' ' || glued {
                if !glued {
                    sink.warning(
                        "@RNL is not preceded by a space, so it is not a record entry (2.3)",
                        Some(line_of(i)),
                    );
                }
                i += 4;
                continue;
            }
            match record_extent(&t, i) {
                Err((msg, pos)) => {
                    sink.error(msg, Some(line_of(pos)));
                    if pos > i {
                        i = if starts(pos, "@RNL") { pos } else { pos + 1 };
                    } else {
                        i += 4;
                    }
                    continue;
                }
                Ok(end) => {
                    for nl in i..end {
                        if t[nl] == '\n' {
                            let line: String = t[i..nl].iter().collect();
                            if line.trim_end_matches([' ', '\t']).ends_with('\\') {
                                sink.warning(
                                    "a line of RNL ends with `\\`: some hosts join the next line to this comment (2.2)",
                                    Some(line_of(nl - 1)),
                                );
                            }
                        }
                    }
                    let rsegs = rebase(&segs, i, end);
                    let lines = (rsegs.first().map(|s| s.line).unwrap_or(0), rsegs.last().map(|s| s.line).unwrap_or(0));
                    out.push(Record { text: t[i..end].iter().collect(), segs: rsegs, lines, file: file.to_string() });
                    i = end;
                    after_record = true;
                    // formal tokens after the record on the same line fail (2.4)
                    let mut j = i;
                    while j < n && t[j] != '\n' {
                        if starts(j, "@RNL") && j > 0 && t[j - 1] == ' ' {
                            break;
                        }
                        if starts(j, "/*") {
                            break;
                        }
                        let ch = t[j];
                        if !(ch.is_alphanumeric() || " \t_,.，。".contains(ch)) {
                            sink.error("formal tokens after the record on the same line fail (2.4)", Some(line_of(j)));
                            while j < n && t[j] != '\n' {
                                j += 1;
                            }
                            break;
                        }
                        j += 1;
                    }
                    i = j;
                    continue;
                }
            }
        }
        i += 1;
    }
    out
}

/// The end of the record starting at `start`, or why it is incomplete (2.4).
fn record_extent(t: &[char], start: usize) -> Result<usize, (String, usize)> {
    let n = t.len();
    let mut i = start + 4;
    let mut depth = 0i64;
    let mut paren = 0i64;
    let mut seen_brace = false;
    let starts = |at: usize, s: &str| -> bool { s.chars().enumerate().all(|(k, c)| t.get(at + k) == Some(&c)) };
    while i < n {
        let c = t[i];
        if c == '"' {
            let mut j = i + 1;
            loop {
                if j >= n || t[j] == '\n' {
                    return Err(("the string is not closed, so the record is incomplete (2.4, 3.7)".into(), i));
                }
                if t[j] == '"' {
                    if j + 1 < n && t[j + 1] == '"' {
                        j += 2;
                        continue;
                    }
                    break;
                }
                j += 1;
            }
            i = j + 1;
            continue;
        }
        if starts(i, "/*") {
            match (i + 2..n).find(|&j| starts(j, "*/")) {
                Some(j) => {
                    i = j + 2;
                    continue;
                }
                None => return Err(("the comment is not closed, so the record is incomplete (2.4)".into(), i)),
            }
        }
        if starts(i, "@RNL") && i > 0 && t[i - 1] == ' ' && !(i + 4 < n && name_char(t[i + 4])) {
            return Err(("incomplete record: a new @RNL comes before this one ends (2.4)".into(), i));
        }
        match c {
            '(' => paren += 1,
            ')' => paren = (paren - 1).max(0),
            '{' if paren == 0 => {
                depth += 1;
                seen_brace = true;
            }
            '}' if paren == 0 => {
                depth -= 1;
                if depth == 0 && seen_brace {
                    return Ok(i + 1);
                }
                if depth < 0 {
                    return Err(("the braces do not match: the whole record fails (7.4)".into(), i));
                }
            }
            _ => {}
        }
        i += 1;
    }
    Err(("incomplete record: cut off by a blank line, a host statement or the end of the file (2.4)".into(), n))
}

fn rebase(segs: &[Seg], start: usize, end: usize) -> Vec<Seg> {
    let mut out = Vec::new();
    for (k, s) in segs.iter().enumerate() {
        let next = segs.get(k + 1).map(|x| x.off).unwrap_or(usize::MAX);
        if next <= start || s.off >= end {
            continue;
        }
        if s.off <= start {
            out.push(Seg { off: 0, file: s.file.clone(), line: s.line, col: s.col + (start - s.off) as i64 });
        } else {
            out.push(Seg { off: s.off - start, file: s.file.clone(), line: s.line, col: s.col });
        }
    }
    out
}

/// The carrier lines of a package file (2.7): every line is a carrier, and the start of a line
/// counts as a space before `@`.
pub fn package_lines(text: &str) -> Vec<CarrierLine> {
    text.lines()
        .enumerate()
        .map(|(k, raw)| {
            let content = if raw.trim().is_empty() { String::new() } else { format!(" {raw}") };
            CarrierLine { line: k as u32 + 1, content, col: -1 }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(v: &[&str]) -> Vec<CarrierLine> {
        v.iter().enumerate().map(|(k, s)| CarrierLine { line: k as u32 + 2, content: s.to_string(), col: 1 }).collect()
    }

    #[test]
    fn records_across_lines() {
        let mut sink = Sink::new();
        let recs = scan_run(
            &lines(&[" @RNL {", "   #M1 @PLACE(1, 2);", "", "   #M2 @PLACE(3, 4)", " } @RNL {#M3}"]),
            "a.sp",
            &mut sink,
        );
        assert_eq!(recs.len(), 2);
        assert!(sink.all().is_empty(), "{:?}", sink.all());
        assert_eq!(recs[0].loc(0), Loc::new("a.sp", 2, 3));
        assert_eq!(recs[1].text, "@RNL {#M3}");
        assert_eq!(recs[0].lines, (2, 6));
    }

    #[test]
    fn incomplete_and_disabled() {
        let mut sink = Sink::new();
        let recs = scan_run(&lines(&[" @RNL {#M1", " /* @RNL {#M2} */", " @RNL {#M3}"]), "a.sp", &mut sink);
        // the first is cut off by the second entry, which is disabled; the third is fine
        assert_eq!(recs.len(), 1);
        assert_eq!(sink.all().len(), 1);
        assert!(sink.all()[0].message.contains("incomplete"));
    }

    #[test]
    fn tokens_after_record() {
        let mut sink = Sink::new();
        let recs = scan_run(&lines(&[" @RNL {#M1} #M2"]), "a.sp", &mut sink);
        assert_eq!(recs.len(), 1);
        assert!(sink.all()[0].message.contains("same line"));
    }
}
