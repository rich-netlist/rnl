//! Host regions whose text is not ordinary netlist comments (standard 2.2).
//!
//! Shared by the SPICE adapter and Spectre's two language modes. Simulator syntax and the
//! conservative treatment of conditions are documented in docs/host-regions.md.

use rnl_runtime::diag::{Loc, Sink};

const CONDITIONAL: &str = "RNL in an inactive or unresolved preprocessing branch is not read (2.2)";
const PREPROCESSOR: &str = "RNL in a preprocessing directive is not read (2.2)";
const TITLE: &str = "the line after `.newjob` is a title and is not read as RNL (2.2)";
const RELIABILITY: &str = "RNL inside `reliability { ... }` is not read (2.2)";

struct Conditional {
    start: u32,
    cpp: bool,
    parent: bool,
    taken: Option<bool>,
    active: bool,
    otherwise: bool,
}

struct Block {
    start: u32,
    end: &'static str,
    warning: &'static str,
}

#[derive(Default)]
struct Braces {
    start: u32,
    depth: usize,
    comment: bool,
    quote: Option<char>,
    escape: bool,
}

impl Braces {
    // Ignore strings and both C comment forms. In particular an RNL record's braces on a
    // comment line cannot close a reliability block or hide its closing brace.
    fn line(&mut self, text: &str) -> bool {
        if !self.comment && self.quote.is_none() && text.trim_start().starts_with('*') {
            return false;
        }
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            if self.comment {
                if c == '*' && chars.peek() == Some(&'/') {
                    chars.next();
                    self.comment = false;
                }
            } else if let Some(q) = self.quote {
                if self.escape {
                    self.escape = false;
                } else if c == '\\' {
                    self.escape = true;
                } else if c == q {
                    self.quote = None;
                }
            } else {
                match c {
                    '/' if chars.peek() == Some(&'/') => break,
                    '/' if chars.peek() == Some(&'*') => {
                        chars.next();
                        self.comment = true;
                    }
                    '\'' | '"' => self.quote = Some(c),
                    '{' => {
                        self.depth += 1;
                    }
                    '}' if self.depth > 0 => {
                        self.depth -= 1;
                        if self.depth == 0 {
                            return true;
                        }
                    }
                    _ => {}
                }
            }
        }
        false
    }
}

/// Per-file classification state. `spans` holds positions where inserting a new comment is
/// unsafe: after the opening line through the closing line, inclusive, like HostDocument's
/// protected regions. Even a known active conditional branch gets no new records: changing the
/// simulator's condition later must not remove a tool's output.
#[derive(Default)]
pub struct Regions {
    block: Option<Block>,
    braces: Option<Braces>,
    conditionals: Vec<Conditional>,
    directive: Option<u32>,
    title: Option<u32>,
    encrypted: Option<u32>,
    spans: Vec<(u32, u32)>,
}

impl Regions {
    /// True if this physical line is not to be parsed as a circuit statement or RNL carrier.
    pub fn skip(&mut self, raw: &str, no: u32, spectre: bool, main: bool, file: &str, sink: &mut Sink) -> bool {
        let reason = self.classify(raw, no, spectre, main, file, sink);
        if let Some(warning) = reason {
            if raw.contains("@RNL") {
                sink.warning(warning, Some(Loc::line(file, no)));
            }
            true
        } else {
            false
        }
    }

    fn classify(
        &mut self,
        raw: &str,
        no: u32,
        spectre: bool,
        main: bool,
        file: &str,
        sink: &mut Sink,
    ) -> Option<&'static str> {
        if let Some(start) = self.title.take() {
            self.spans.push((start, no));
            return Some(TITLE);
        }
        let text = raw.trim();
        let low = text.to_ascii_lowercase();
        let first = low.split_whitespace().next().unwrap_or("");
        let pragma = text.strip_prefix("//").map(str::trim_start).unwrap_or("");
        let pragma = pragma.split_whitespace().collect::<Vec<_>>();
        let protection = pragma.starts_with(&["pragma", "protect"]);
        if let Some(start) = self.encrypted {
            if protection && pragma.get(2) == Some(&"end_protected") {
                self.spans.push((start, no));
                self.encrypted = None;
            }
            return Some("RNL in encrypted text is not read (2.2)");
        }
        if let Some(start) = self.directive {
            if !raw.ends_with('\\') {
                self.spans.push((start, no));
                self.directive = None;
            }
            return Some(PREPROCESSOR);
        }
        // CPP directives run before either host language. Dot conditionals are WRspice input
        // preprocessing, including around script/Verilog blocks (manual 2.5.3).
        let cpp = text.starts_with('#')
            && self.block.is_none()
            && self.braces.as_ref().is_none_or(|b| !b.comment && b.quote.is_none());
        let directive = if cpp { text[1..].trim_start() } else { text };
        let word = directive.split(|c: char| c.is_whitespace() || c == '(').next().unwrap_or("");
        let word = word.to_ascii_lowercase();
        let key = if cpp { word.as_str() } else { word.strip_prefix('.').unwrap_or("") };
        if (cpp || !spectre) && matches!(key, "if" | "ifdef" | "ifndef" | "elif" | "elseif" | "else" | "endif") {
            let expr = directive[word.len()..].trim();
            let selected = if matches!(key, "ifdef" | "ifndef") || raw.ends_with('\\') {
                None
            } else {
                literal_condition(expr, cpp)
            };
            match key {
                "if" | "ifdef" | "ifndef" => {
                    let parent = self.conditionals.iter().all(|c| c.active);
                    if parent && selected.is_none() {
                        sink.warning(
                            "the preprocessing condition cannot be resolved without simulator settings; its branches are not read (2.2)",
                            Some(Loc::line(file, no)),
                        );
                    }
                    self.conditionals.push(Conditional {
                        start: no,
                        cpp,
                        parent,
                        taken: selected,
                        active: parent && selected == Some(true),
                        otherwise: false,
                    });
                }
                "endif" => {
                    if self.conditionals.last().is_some_and(|c| c.cpp == cpp) {
                        let c = self.conditionals.pop().unwrap();
                        self.spans.push((c.start, no));
                    } else {
                        sink.warning(
                            "an unmatched preprocessing directive is not read (2.2)",
                            Some(Loc::line(file, no)),
                        );
                    }
                }
                _ => {
                    if let Some(c) = self.conditionals.last_mut().filter(|c| c.cpp == cpp) {
                        let branch = if key == "else" { Some(true) } else { selected };
                        c.active = c.parent && c.taken == Some(false) && branch == Some(true) && !c.otherwise;
                        if c.parent && c.taken == Some(false) && branch.is_none() {
                            sink.warning(
                                "the preprocessing condition cannot be resolved without simulator settings; its remaining branches are not read (2.2)",
                                Some(Loc::line(file, no)),
                            );
                        }
                        c.taken = match (c.taken, branch) {
                            (Some(true), _) | (Some(false), Some(true)) => Some(true),
                            (Some(false), Some(false)) => Some(false),
                            _ => None,
                        };
                        c.otherwise |= key == "else";
                    } else {
                        sink.warning(
                            "an unmatched preprocessing directive is not read (2.2)",
                            Some(Loc::line(file, no)),
                        );
                    }
                }
            }
            if raw.ends_with('\\') {
                self.directive = Some(no);
            }
            return Some(PREPROCESSOR);
        }
        if cpp {
            if raw.ends_with('\\') {
                self.directive = Some(no);
            }
            return Some(PREPROCESSOR);
        }
        if self.conditionals.iter().any(|c| !c.active) {
            return Some(CONDITIONAL);
        }
        if protection && pragma.get(2) == Some(&"begin_protected") && self.block.is_none() && self.braces.is_none() {
            self.encrypted = Some(no);
            return Some("RNL in encrypted text is not read (2.2)");
        }
        if let Some(block) = &self.block {
            let warning = block.warning;
            if first == block.end {
                self.spans.push((block.start, no));
                self.block = None;
            }
            return Some(warning);
        }
        if let Some(braces) = &mut self.braces {
            if braces.line(raw) {
                self.spans.push((braces.start, no));
                self.braces = None;
            }
            return Some(RELIABILITY);
        }
        if !spectre {
            let block = match first {
                ".control" => Some((".endc", "RNL inside `.control` is not read (2.2)")),
                ".exec" => Some((".endc", "RNL inside `.exec` is not read (2.2)")),
                ".postrun" => Some((".endc", "RNL inside `.postrun` is not read (2.2)")),
                ".verilog" => Some((".endv", "RNL inside `.verilog` is not read (2.2)")),
                _ => None,
            };
            if let Some((end, warning)) = block {
                self.block = Some(Block { start: no, end, warning });
                return Some(warning);
            }
            if first == ".newjob" && main {
                self.title = Some(no);
                return Some(TITLE);
            }
        } else if !text.starts_with("//") && !text.starts_with('*') && reliability_header(text) {
            let mut braces = Braces { start: no, ..Braces::default() };
            if braces.line(raw) {
                self.spans.push((no, no));
            } else {
                self.braces = Some(braces);
            }
            return Some(RELIABILITY);
        }
        None
    }

    /// Close unfinished regions at EOF, where appending text is still unsafe. A missing delimiter
    /// gets a warning even if no RNL was present, rather than silently swallowing following text.
    pub fn finish(mut self, file: &str, past: u32, sink: &mut Sink) -> Vec<(String, u32, u32)> {
        let mut unfinished = self.conditionals.iter().map(|c| c.start).collect::<Vec<_>>();
        unfinished.extend(self.block.map(|b| b.start));
        unfinished.extend(self.braces.map(|b| b.start));
        unfinished.extend(self.directive);
        unfinished.extend(self.title);
        unfinished.extend(self.encrypted);
        for start in unfinished {
            self.spans.push((start, past));
            sink.warning(
                "an unterminated host region is not writable through end of file (2.2)",
                Some(Loc::line(file, start)),
            );
        }
        self.spans.into_iter().map(|(a, b)| (file.to_string(), a, b)).collect()
    }
}

// Only a literal whose meaning is independent of simulator state is evaluated. Expressions,
// macro names, numeric suffixes and external definitions remain explicitly unresolved.
fn literal_condition(expr: &str, cpp: bool) -> Option<bool> {
    let expr = if cpp { expr.split("//").next()? } else { expr.split([';', '$']).next()? }.trim();
    let expr = expr.strip_prefix('(').and_then(|s| s.strip_suffix(')')).unwrap_or(expr).trim();
    if cpp {
        return expr.parse::<i64>().ok().map(|n| n != 0);
    }
    let number = |s: &str| {
        let s = s.trim();
        s.parse::<f64>().ok().filter(|n| {
            n.is_finite()
                && (*n != 0.0 || !s.split(['e', 'E']).next().unwrap_or("").chars().any(|c| matches!(c, '1'..='9')))
        })
    };
    if let Some((a, b)) = expr.split_once('=') {
        return Some(number(a)? == number(b.strip_prefix('=').unwrap_or(b))?);
    }
    Some(number(expr)? != 0.0)
}

// A keyword in an instance's name or node list is not an analysis header. A documented
// reliability header has only options between its keyword and its opening brace.
fn reliability_header(text: &str) -> bool {
    fn tail(text: &str) -> Option<&str> {
        let rest = text.strip_prefix("reliability")?;
        (rest.is_empty() || rest.starts_with('{') || rest.starts_with(char::is_whitespace)).then(|| rest.trim_start())
    }
    let rest = tail(text).or_else(|| tail(text.split_once(char::is_whitespace)?.1.trim_start()));
    rest.is_some_and(|rest| {
        rest.is_empty()
            || rest.starts_with('{')
            || rest == "\\"
            || rest.split_whitespace().next().is_some_and(|word| word.contains('='))
            || rest.split_whitespace().nth(1) == Some("=")
    })
}
