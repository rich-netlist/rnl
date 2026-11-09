//! Character-based record edits mapped back to the original carrier text.

use rnl_runtime::host::Record;

use super::Change;

#[derive(Clone, Debug)]
pub(super) struct Edit {
    pub start: usize,
    pub end: usize,
    pub replacement: String,
    pub change: Change,
}

fn byte_at(text: &str, line: u32, col: u32) -> Result<usize, String> {
    let mut start = 0;
    for _ in 1..line {
        start += text[start..].find('\n').ok_or("record line is outside its source")? + 1;
    }
    let row = text[start..].split('\n').next().unwrap_or("");
    let offset = col.saturating_sub(1) as usize;
    let at = row.char_indices().map(|(i, _)| i).chain(std::iter::once(row.len())).nth(offset);
    at.map(|at| start + at).ok_or_else(|| "record column is outside its source".into())
}

pub(super) fn record_edit(
    source: &str,
    record: &Record,
    range: std::ops::Range<usize>,
    replacement: String,
    rule: &str,
) -> Result<Edit, String> {
    let first = record.loc(range.start);
    let last = record.loc(range.end);
    let start = byte_at(source, first.line, first.col)?;
    let end = byte_at(source, last.line, last.col)?;
    let before = source.get(start..end).ok_or("invalid record edit range")?.to_string();
    Ok(Edit {
        start,
        end,
        replacement: replacement.clone(),
        change: Change { rule: rule.into(), line: first.line, before, after: replacement },
    })
}

pub(super) fn apply(source: &str, edits: &mut Vec<Edit>) -> Result<String, String> {
    edits.sort_by_key(|e| (e.start, e.end));
    edits.dedup_by(|b, a| a.start == b.start && a.end == b.end && a.replacement == b.replacement);
    for pair in edits.windows(2) {
        if pair[0].end > pair[1].start {
            return Err("overlapping upgrade edits; the source was not written".into());
        }
    }
    let mut text = source.to_string();
    for edit in edits.iter().rev() {
        text.replace_range(edit.start..edit.end, &edit.replacement);
    }
    Ok(text)
}

/// Remove only the solver record, retaining line boundaries and surrounding host text.
pub(super) fn discard_generated(source: &str, record: &Record) -> Result<Edit, String> {
    let mut edit = record_edit(source, record, 0..record.text.chars().count(), String::new(), "discard-solver-record")?;
    edit.replacement = edit.change.before.chars().filter(|c| matches!(c, '\r' | '\n')).collect();
    edit.change.after = edit.replacement.clone();
    Ok(edit)
}
