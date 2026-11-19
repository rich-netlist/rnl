//! A reading of one file of a document ([`crate::Options::only_file`]) against the reading of the
//! whole document. Each level that a reading of one file builds must be the level the whole
//! reading builds: the same drawing, problem, numbers and references (6.1, 10.3, 5.9). What a
//! reading of one file reports must be among what the whole reading reports, and lie in the file
//! itself, in the files above it, in the subcircuits it instantiates or in a package; and the
//! readings of all the files together report all that the whole reading does.

use std::collections::{BTreeSet, HashSet};

use crate::diag::Diag;
use crate::host::{HostDocument, LevelId, LevelKind};
use crate::Interpretation;

/// The files of a document, by their names in it, in the order read: those a reading of one file
/// can be asked for. A package file read by the netlist is not one.
pub fn files(doc: &HostDocument) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for lv in &doc.levels {
        if lv.kind != LevelKind::Package && !doc.is_package(&lv.file) && !out.contains(&lv.file) {
            out.push(lv.file.clone());
        }
    }
    out
}

/// The levels written in `file`.
fn levels_of(doc: &HostDocument, file: &str) -> Vec<LevelId> {
    (0..doc.levels.len()).filter(|&lv| doc.levels[lv].file == file).collect()
}

/// The files a reading of `file` may report about: the file, the files above its levels, and the
/// files of the subcircuits its levels instantiate.
fn reported_files(doc: &HostDocument, file: &str) -> HashSet<String> {
    let mut out = HashSet::from([file.to_string()]);
    for lv in levels_of(doc, file) {
        out.extend(doc.ancestors(lv).into_iter().map(|a| doc.levels[a].file.clone()));
        for inst in &doc.levels[lv].instances {
            if let Some(child) = doc.subckt_of(inst) {
                out.insert(doc.levels[child].file.clone());
            }
        }
    }
    out
}

/// A diagnostic as a line of text, with its upstream cause.
fn diagnostic(r: &Interpretation, d: &Diag) -> String {
    r.sink.render(d)
}

/// What a reading reports: its diagnostics and the statements that do not take effect, each a
/// line of text.
pub fn reported(r: &Interpretation) -> BTreeSet<String> {
    let mut out: BTreeSet<String> = r.diagnostics().into_iter().map(|d| diagnostic(r, d)).collect();
    out.extend(r.failed.iter().map(|f| format!("failed {f:?}")));
    out
}

/// How the reading of `file` alone, `part`, differs from the whole reading, `whole`: each a line;
/// none when it reads as it must.
pub fn differences(whole: &Interpretation, part: &Interpretation, file: &str) -> Vec<String> {
    let mut out = Vec::new();
    let doc = &whole.doc;
    let mine: HashSet<LevelId> = levels_of(doc, file).into_iter().collect();
    if whole.model.failed != part.model.failed {
        out.push(format!("failed: {:?} against {:?}", part.model.failed, whole.model.failed));
    }

    // the levels, their problems, numbers and references; a symbol's identity is that of its
    // object in the reading, so it is counted in the order the symbols come
    let drawn = |r: &Interpretation| -> Vec<String> {
        let levels: Vec<String> =
            r.model.levels.iter().filter(|l| mine.contains(&l.level)).map(|l| format!("{l:?}")).collect();
        renumbered(&levels)
    };
    let (a, b) = (drawn(whole), drawn(part));
    if part.model.levels.len() != b.len() {
        let other: Vec<&str> =
            part.model.levels.iter().filter(|l| !mine.contains(&l.level)).map(|l| l.title.as_str()).collect();
        out.push(format!("levels of other files built: {other:?}"));
    }
    if a.len() != b.len() {
        out.push(format!("{} levels against {}", b.len(), a.len()));
    }
    for (k, (x, y)) in a.iter().zip(&b).enumerate() {
        if x != y {
            out.push(format!("level {k} of the file ({}) differs: {}", title(whole, &mine, k), first_difference(x, y)));
        }
    }
    let problems = |r: &Interpretation| -> Vec<String> {
        let problems: Vec<String> =
            r.problems.iter().filter(|p| mine.contains(&p.level)).map(|p| format!("{p:?}")).collect();
        renumbered(&problems)
    };
    let (a, b) = (problems(whole), problems(part));
    if a != b {
        out.push(format!(
            "problems differ: {}",
            first_difference(
                &a.join(
                    "
"
                ),
                &b.join(
                    "
"
                )
            )
        ));
    }
    // the input of the solver, which says whether a record of the solver is up to date (17)
    let digests = |r: &Interpretation| -> Vec<String> {
        r.problems.iter().filter(|p| mine.contains(&p.level)).map(crate::problem::input_digest).collect()
    };
    if digests(whole) != digests(part) {
        out.push(format!("input digests differ: {:?} against {:?}", digests(part), digests(whole)));
    }
    let numbers = |r: &Interpretation| -> Vec<String> {
        r.numbering.iter().filter(|n| mine.contains(&n.level)).map(|n| format!("{n:?}")).collect()
    };
    let (a, b) = (numbers(whole), numbers(part));
    if a != b {
        out.push(format!(
            "numbering differs: {}",
            first_difference(
                &a.join(
                    "
"
                ),
                &b.join(
                    "
"
                )
            )
        ));
    }
    let refs = |r: &Interpretation| -> BTreeSet<String> {
        r.refs.iter().filter(|h| mine.contains(&h.level)).map(|h| format!("{h:?}")).collect()
    };
    if refs(whole) != refs(part) {
        out.push("references differ".to_string());
    }
    let unsupported = |r: &Interpretation| -> Vec<String> {
        r.unsupported.iter().enumerate().filter(|(lv, _)| mine.contains(lv)).map(|(_, u)| format!("{u:?}")).collect()
    };
    if unsupported(whole) != unsupported(part) {
        out.push("unsupported files differ".to_string());
    }

    // what it reports: among the whole reading's, and about the files it reads
    let all = reported(whole);
    let allowed = reported_files(doc, file);
    let sources: HashSet<&str> = doc.sources.iter().map(|(name, _)| name.as_str()).collect();
    // what reading the document reports is the document's, about any file
    let read: HashSet<*const Diag> = part.sink.all()[..part.read].iter().map(|d| d as *const Diag).collect();
    for d in part.diagnostics() {
        let line = diagnostic(part, d);
        if !all.contains(&line) {
            out.push(format!("reported only alone: {line}"));
        }
        let elsewhere = !read.contains(&(d as *const Diag))
            && d.loc.as_ref().is_some_and(|l| {
                d.in_package.is_empty() && sources.contains(l.file.as_str()) && !allowed.contains(&l.file)
            });
        if elsewhere {
            out.push(format!("reported about another file: {line}"));
        }
    }
    for f in &part.failed {
        let line = format!("failed {f:?}");
        if !all.contains(&line) {
            out.push(format!("does not take effect only alone: {line}"));
        }
        if sources.contains(f.start.file.as_str()) && !allowed.contains(&f.start.file) {
            out.push(format!("does not take effect in another file: {line}"));
        }
    }
    out
}

/// Texts of the model with the numbers of the objects of the reading made the order in which they
/// first come in them: the identity of each symbol, `SymbolView { id: N`, and the id of an object
/// with no name and no place of its own, `"@N`. Two readings that make the same objects in another
/// order of numbers give the same texts.
pub fn renumbered(texts: &[String]) -> Vec<String> {
    let mut texts = texts.to_vec();
    for mark in ["SymbolView { id: ", "\"@"] {
        let mut ids: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        texts = texts
            .iter()
            .map(|text| {
                let mut out = String::with_capacity(text.len());
                let mut rest = text.as_str();
                while let Some(at) = rest.find(mark) {
                    out.push_str(&rest[..at + mark.len()]);
                    rest = &rest[at + mark.len()..];
                    let digits = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
                    if digits > 0 {
                        let next = ids.len();
                        let n = *ids.entry(rest[..digits].to_string()).or_insert(next);
                        out.push_str(&n.to_string());
                        rest = &rest[digits..];
                    }
                }
                out.push_str(rest);
                out
            })
            .collect();
    }
    texts
}

/// Where two texts first differ: a little of each from there.
fn first_difference(whole: &str, part: &str) -> String {
    let at = whole.chars().zip(part.chars()).take_while(|(a, b)| a == b).count();
    let from = at.saturating_sub(60);
    let piece = |s: &str| s.chars().skip(from).take(160).collect::<String>();
    format!("at character {at}, whole `{}` against alone `{}`", piece(whole), piece(part))
}

/// The title of the `k`th level of the file in the whole reading.
fn title(whole: &Interpretation, mine: &HashSet<LevelId>, k: usize) -> String {
    let levels = whole.model.levels.iter().filter(|l| mine.contains(&l.level));
    levels.clone().nth(k).map(|l| l.title.clone()).unwrap_or_default()
}

/// What the whole reading reports that none of the readings of one file does, given all that
/// they report ([`reported`]).
pub fn unreported(whole: &Interpretation, found: &HashSet<String>) -> Vec<String> {
    reported(whole).into_iter().filter(|line| !found.contains(line)).collect()
}
