//! Text markup (spec-draft 13.6): a subset of LaTeX text mode, and `${…}` attribute references.

/// Marks text taken from an attribute: shown as written, never read as markup.
pub const LITERAL: char = '\u{0}';

/// One run of text in a line.
#[derive(Clone, Debug, PartialEq)]
pub enum Run {
    Text {
        text: String,
        style: RunStyle,
    },
    /// `\frac{…}{…}`: numerator over denominator.
    Frac {
        num: Vec<Run>,
        den: Vec<Run>,
        style: RunStyle,
    },
}

/// Level: 0 normal, 1 superscript, -1 subscript (nested levels add up).
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct RunStyle {
    pub level: i8,
    pub bold: bool,
    pub italic: bool,
    pub overline: bool,
}

pub type Line = Vec<Run>;

const GREEK: [(&str, char); 40] = [
    ("alpha", 'α'),
    ("beta", 'β'),
    ("gamma", 'γ'),
    ("delta", 'δ'),
    ("epsilon", 'ε'),
    ("zeta", 'ζ'),
    ("eta", 'η'),
    ("theta", 'θ'),
    ("iota", 'ι'),
    ("kappa", 'κ'),
    ("lambda", 'λ'),
    ("mu", 'μ'),
    ("nu", 'ν'),
    ("xi", 'ξ'),
    ("pi", 'π'),
    ("rho", 'ρ'),
    ("sigma", 'σ'),
    ("tau", 'τ'),
    ("upsilon", 'υ'),
    ("phi", 'φ'),
    ("chi", 'χ'),
    ("psi", 'ψ'),
    ("omega", 'ω'),
    ("varepsilon", 'ε'),
    ("vartheta", 'ϑ'),
    ("varpi", 'ϖ'),
    ("varrho", 'ϱ'),
    ("varsigma", 'ς'),
    ("varphi", 'φ'),
    ("Gamma", 'Γ'),
    ("Delta", 'Δ'),
    ("Theta", 'Θ'),
    ("Lambda", 'Λ'),
    ("Xi", 'Ξ'),
    ("Pi", 'Π'),
    ("Sigma", 'Σ'),
    ("Upsilon", 'Υ'),
    ("Phi", 'Φ'),
    ("Psi", 'Ψ'),
    ("Omega", 'Ω'),
];

const SYMS: [(&str, char); 18] = [
    ("pm", '±'),
    ("times", '×'),
    ("cdot", '·'),
    ("infty", '∞'),
    ("approx", '≈'),
    ("neq", '≠'),
    ("leq", '≤'),
    ("geq", '≥'),
    ("ll", '≪'),
    ("gg", '≫'),
    ("propto", '∝'),
    ("parallel", '∥'),
    ("angle", '∠'),
    ("circ", '∘'),
    ("to", '→'),
    ("uparrow", '↑'),
    ("downarrow", '↓'),
    ("textbackslash", '\\'),
];

fn symbol(cmd: &str) -> Option<char> {
    GREEK.iter().chain(SYMS.iter()).find(|(n, _)| *n == cmd).map(|(_, c)| *c)
}

fn known(cmd: &str) -> bool {
    symbol(cmd).is_some() || matches!(cmd, "textbf" | "textit" | "overline" | "frac")
}

struct P<'a> {
    t: &'a [char],
    lines: Vec<Line>,
}

impl P<'_> {
    fn emit(&mut self, buf: &mut String, style: RunStyle) {
        if !buf.is_empty() {
            self.lines.last_mut().unwrap().push(Run::Text { text: std::mem::take(buf), style });
        }
    }

    fn parse(&mut self, mut i: usize, style: RunStyle, until_brace: bool) -> usize {
        let t = self.t;
        let n = t.len();
        let mut buf = String::new();
        while i < n {
            let c = t[i];
            if c == LITERAL {
                let j = (i + 1..n).find(|&j| t[j] == LITERAL).unwrap_or(n);
                buf.extend(&t[i + 1..j]);
                i = j + 1;
                continue;
            }
            if until_brace && c == '}' {
                self.emit(&mut buf, style);
                return i + 1;
            }
            if c == '\\' {
                if t.get(i + 1) == Some(&'\\') {
                    self.emit(&mut buf, style);
                    self.lines.push(Vec::new());
                    i += 2;
                    continue;
                }
                if let Some(&d) = t.get(i + 1) {
                    if "_{}$%&#^".contains(d) {
                        buf.push(d);
                        i += 2;
                        continue;
                    }
                }
                let mut j = i + 1;
                while j < n && t[j].is_ascii_alphabetic() {
                    j += 1;
                }
                if j > i + 1 {
                    let cmd: String = t[i + 1..j].iter().collect();
                    if cmd == "frac" && t.get(j) == Some(&'{') {
                        // \frac{numerator}{denominator}: one run holding both parts
                        self.emit(&mut buf, style);
                        let outer = std::mem::take(self.lines.last_mut().unwrap());
                        i = self.parse(j + 1, style, true);
                        let num = std::mem::take(self.lines.last_mut().unwrap());
                        let mut den = Vec::new();
                        if t.get(i) == Some(&'{') {
                            i = self.parse(i + 1, style, true);
                            den = std::mem::take(self.lines.last_mut().unwrap());
                        }
                        *self.lines.last_mut().unwrap() = outer;
                        self.lines.last_mut().unwrap().push(Run::Frac { num, den, style });
                        continue;
                    }
                    if matches!(cmd.as_str(), "textbf" | "textit" | "overline") && t.get(j) == Some(&'{') {
                        self.emit(&mut buf, style);
                        let mut st = style;
                        match cmd.as_str() {
                            "textbf" => st.bold = true,
                            "textit" => st.italic = true,
                            _ => st.overline = true,
                        }
                        i = self.parse(j + 1, st, true);
                        continue;
                    }
                    if let Some(ch) = symbol(&cmd) {
                        buf.push(ch);
                        i = after_command(t, j);
                        continue;
                    }
                }
                buf.push('\\');
                i += 1;
                continue;
            }
            if (c == '_' || c == '^') && t.get(i + 1) == Some(&'{') {
                self.emit(&mut buf, style);
                let mut st = style;
                st.level += if c == '^' { 1 } else { -1 };
                i = self.parse(i + 2, st, true);
                continue;
            }
            buf.push(c);
            i += 1;
        }
        self.emit(&mut buf, style);
        i
    }
}

/// LaTeX habit (13.6): spaces after a command name are swallowed, then one empty `{}`.
fn after_command(t: &[char], mut j: usize) -> usize {
    while j < t.len() && (t[j] == ' ' || t[j] == '\t') {
        j += 1;
    }
    if t.get(j) == Some(&'{') && t.get(j + 1) == Some(&'}') {
        j += 2;
    }
    j
}

/// Lines of runs for a text whose attribute values are already wrapped in [`LITERAL`].
pub fn parse(text: &str) -> Vec<Line> {
    let chars: Vec<char> = text.chars().collect();
    let mut p = P { t: &chars, lines: vec![Vec::new()] };
    p.parse(0, RunStyle::default(), false);
    p.lines
}

/// The plain characters of a run list, for width estimates and searching.
pub fn plain(runs: &[Run]) -> String {
    let mut s = String::new();
    for r in runs {
        match r {
            Run::Text { text, .. } => s.push_str(text),
            Run::Frac { num, den, .. } => {
                s.push_str(&plain(num));
                s.push('/');
                s.push_str(&plain(den));
            }
        }
    }
    s
}

/// Remove `${…}` and `\${…}` references.
fn strip_refs(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let start = if chars[i] == '\\' && chars.get(i + 1) == Some(&'$') && chars.get(i + 2) == Some(&'{') {
            Some(i + 3)
        } else if chars[i] == '$' && chars.get(i + 1) == Some(&'{') {
            Some(i + 2)
        } else {
            None
        };
        if let Some(s) = start {
            if let Some(e) = (s..chars.len()).find(|&j| chars[j] == '}') {
                i = e + 1;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// 13.6: unknown commands and unbalanced braces are shown as written, with a warning.
pub fn problems(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let t: Vec<char> = strip_refs(text).replace("\\\\", "\n").chars().collect();
    let mut i = 0;
    while i < t.len() {
        if t[i] == '\\' {
            let mut j = i + 1;
            while j < t.len() && t[j].is_ascii_alphabetic() {
                j += 1;
            }
            if j > i + 1 {
                let cmd: String = t[i + 1..j].iter().collect();
                if !known(&cmd) {
                    out.push(format!("unknown markup \\{cmd} is shown as written"));
                } else if cmd == "frac" {
                    let mut k = j;
                    while k < t.len() && t[k].is_whitespace() {
                        k += 1;
                    }
                    if t.get(k) != Some(&'{') {
                        out.push("\\frac is followed by two braces, the numerator and the denominator".into());
                    }
                }
                i = j;
                continue;
            }
        }
        i += 1;
    }
    let mut depth = 0i64;
    let mut i = 0;
    while i < t.len() {
        let c = t[i];
        if c == '\\' && t.get(i + 1).is_some_and(|d| "{}\\".contains(*d)) {
            i += 2;
            continue;
        }
        if c == '{' {
            depth += 1;
        } else if c == '}' {
            depth -= 1;
            if depth < 0 {
                break;
            }
        }
        i += 1;
    }
    if depth != 0 {
        out.push("unbalanced braces are shown as written".into());
    }
    out
}

/// The paths of the `${…}` references in a text, not the literal `\${…}`.
pub fn attr_refs(text: &str) -> Vec<String> {
    let t: Vec<char> = text.replace("\\${", "").chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i + 1 < t.len() {
        if t[i] == '$' && t[i + 1] == '{' {
            if let Some(e) = (i + 2..t.len()).find(|&j| t[j] == '}') {
                out.push(t[i + 2..e].iter().collect());
                i = e + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Replace each `${path}` by its value wrapped in [`LITERAL`]; a literal `\${` stays `${`.
pub fn expand(text: &str, mut value: impl FnMut(&str) -> String) -> String {
    let t: Vec<char> = text.replace("\\${", "\u{1}").chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < t.len() {
        if t[i] == '$' && t.get(i + 1) == Some(&'{') {
            if let Some(e) = (i + 2..t.len()).find(|&j| t[j] == '}') {
                let path: String = t[i + 2..e].iter().collect();
                out.push(LITERAL);
                out.push_str(&value(&path));
                out.push(LITERAL);
                i = e + 1;
                continue;
            }
        }
        out.push(t[i]);
        i += 1;
    }
    out.replace('\u{1}', "${")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(l: &Line) -> Vec<(String, i8)> {
        l.iter()
            .map(|r| match r {
                Run::Text { text, style } => (text.clone(), style.level),
                Run::Frac { .. } => ("<frac>".into(), 0),
            })
            .collect()
    }

    #[test]
    fn scripts_and_commands() {
        let l = parse("V_{DD} 10 \\mu A\\\\x");
        assert_eq!(l.len(), 2);
        assert_eq!(texts(&l[0]), vec![("V".into(), 0), ("DD".into(), -1), (" 10 μA".into(), 0)]);
        let l = parse("\\frac{W}{L}");
        assert!(matches!(&l[0][0], Run::Frac { num, den, .. } if plain(num) == "W" && plain(den) == "L"));
        assert_eq!(texts(&parse("g_m")[0]), vec![("g_m".into(), 0)]);
    }

    #[test]
    fn problems_and_refs() {
        assert_eq!(problems("C:\\temp").len(), 1);
        assert!(problems("\\${x} and ${y}").is_empty());
        assert_eq!(problems("a}").len(), 1);
        assert_eq!(attr_refs("W/L=${w}/${l} \\${n}"), vec!["w", "l"]);
        let e = expand("${name}_{x} \\${k}", |p| p.to_uppercase());
        assert_eq!(e, "\u{0}NAME\u{0}_{x} ${k}");
        assert_eq!(texts(&parse(&e)[0]), vec![("NAME".into(), 0), ("x".into(), -1), (" ${k}".into(), 0)]);
    }
}
