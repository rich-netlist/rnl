//! The parameters of subcircuits, as a viewer shows them: what a subcircuit's definition writes,
//! what an instance gives it, and the numbers they come to. The language evaluates no parameter
//! (`${w}` shows what is written, 13.6) and nothing drawn depends on one, so this is for showing
//! only: a number is evaluated here, and a value it cannot evaluate has none.
//!
//! The evaluator takes numbers with their scale factors (SPICE's `1meg`, `2u`; Spectre's `1M`,
//! `2u`), the operators `+ - * / ^ **`, parentheses, and the common functions. Anything else, such
//! as a string, a condition or a function the netlist defines, gives no number and no diagnostic.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use rnl_runtime::diag::Loc;
use rnl_runtime::host::{HostDocument, Instance, LevelId, LevelKind};
use serde::{Deserialize, Serialize};

/// The parameters to build a subcircuit with ([`crate::BuildOptions::bind`]): its name, and values
/// for some of its parameters, as text, such as `"2u"`, `"1e-06"` or `"{2*l}"`. Only the Document's
/// parameter lists change; nothing is remembered.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct Bind {
    pub subckt: String,
    #[serde(default)]
    pub params: BTreeMap<String, String>,
}

/// A parameter of a subcircuit: its default, the value it has here, and the number that comes to.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct SubcktParam {
    pub name: String,
    /// The default its definition writes; none when it writes none.
    pub default: Option<String>,
    /// The value it has, as written: the one bound, the one the instance gives, or the default.
    pub value: Option<String>,
    /// That value evaluated; none when it cannot be.
    pub number: Option<f64>,
    /// Where the value comes from: "bound" ([`Bind`]), "instance", "default", or "unset" when it
    /// has none.
    pub from: &'static str,
}

/// A parameter an instance writes, and the number it comes to where the instance is.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ParamValue {
    pub name: String,
    pub value: String,
    pub number: Option<f64>,
}

#[derive(Clone, Debug)]
enum Entry {
    Text(String),
    Fixed(Option<f64>),
    Unset,
}

/// Values by name, the innermost scope first.
struct Env {
    frames: Vec<Vec<(String, Entry)>>,
    sensitive: bool,
    spectre: bool,
    memo: HashMap<(usize, usize), Option<f64>>,
    /// The values being evaluated, each one needed by the one before it.
    busy: Vec<(usize, usize)>,
    cycles: BTreeSet<String>,
}

impl Env {
    fn same(&self, a: &str, b: &str) -> bool {
        if self.sensitive {
            a == b
        } else {
            a.eq_ignore_ascii_case(b)
        }
    }

    /// The value `name` has, looked for from the scope `from` outwards; a later one of a scope
    /// hides an earlier one.
    fn lookup(&mut self, from: usize, name: &str) -> Option<f64> {
        let (f, k) = (from..self.frames.len())
            .find_map(|f| self.frames[f].iter().rposition(|(n, _)| self.same(n, name)).map(|k| (f, k)))?;
        self.get(f, k)
    }

    fn get(&mut self, f: usize, k: usize) -> Option<f64> {
        if let Some(v) = self.memo.get(&(f, k)) {
            return *v;
        }
        if let Some(start) = self.busy.iter().position(|b| *b == (f, k)) {
            let cycle: Vec<String> = self.busy[start..].iter().map(|(f, k)| self.frames[*f][*k].0.clone()).collect();
            self.cycles.extend(cycle);
            return None;
        }
        self.busy.push((f, k));
        let v = match self.frames[f][k].1.clone() {
            Entry::Text(t) => {
                let spectre = self.spectre;
                evaluate(&t, spectre, &mut |n| self.lookup(f, n))
            }
            Entry::Fixed(v) => v,
            Entry::Unset => None,
        };
        self.busy.pop();
        let v = v.filter(|x| x.is_finite());
        self.memo.insert((f, k), v);
        v
    }

    fn evaluate(&mut self, text: &str) -> Option<f64> {
        let spectre = self.spectre;
        evaluate(text, spectre, &mut |n| self.lookup(0, n)).filter(|x| x.is_finite())
    }
}

/// The parameters of a document's subcircuits and instances, with a [`Bind`].
pub(crate) struct Params<'d> {
    doc: &'d HostDocument,
    bound: Option<(LevelId, &'d BTreeMap<String, String>)>,
    globals: Vec<(String, String)>,
}

/// A diagnostic of the parameters: its severity, its message and where it is.
pub(crate) type Note = (&'static str, String, Option<Loc>);

impl<'d> Params<'d> {
    pub(crate) fn new(doc: &'d HostDocument, bind: Option<&'d Bind>) -> (Params<'d>, Vec<Note>) {
        let mut notes = Vec::new();
        // the values the files set outside subcircuits are seen everywhere
        let globals = doc
            .levels
            .iter()
            .filter(|l| l.kind == LevelKind::File)
            .flat_map(|l| l.param_statements.iter().cloned())
            .collect();
        let bound = bind.and_then(|b| {
            let Some(lv) = doc.subckt_for(&b.subckt, doc.dialect.case_sensitive) else {
                notes.push(("warning", format!("no subcircuit {} to bind parameters to", b.subckt), None));
                return None;
            };
            let level = &doc.levels[lv];
            for name in b.params.keys() {
                if !level.params.iter().any(|(n, _)| level.same_name(n, name)) {
                    let close = level
                        .params
                        .iter()
                        .map(|(n, _)| n)
                        .find(|n| distance(&n.to_lowercase(), &name.to_lowercase()) <= 2)
                        .map(|n| format!("; did you mean {n}?"))
                        .unwrap_or_default();
                    notes.push((
                        "warning",
                        format!(
                            "bound parameter {name} is not a parameter of subcircuit {}; it is ignored{close}",
                            level.name
                        ),
                        Some(Loc::line(&level.file, level.line)),
                    ));
                }
            }
            Some((lv, &b.params))
        });
        let params = Params { doc, bound, globals };
        // parameters whose defaults refer to each other have no number
        for (lv, level) in doc.levels.iter().enumerate().filter(|(_, l)| l.kind == LevelKind::Subckt) {
            let mut env = params.env(lv, None);
            for k in 0..level.params.len() {
                env.get(0, k);
            }
            if !env.cycles.is_empty() {
                let names: Vec<String> = env.cycles.iter().cloned().collect();
                notes.push((
                    "hint",
                    format!(
                        "the parameters {} of subcircuit {} refer to each other, so they have no number",
                        names.join(", "),
                        level.name
                    ),
                    Some(Loc::line(&level.file, level.line)),
                ));
            }
        }
        (params, notes)
    }

    /// The scopes of the level `lv`: its own parameters, `own` in place of those it gives, and the
    /// values it sets; those of the subcircuits it is written in; and the files' values.
    fn env(&self, lv: LevelId, own: Option<&[(String, Entry)]>) -> Env {
        let level = &self.doc.levels[lv];
        let mut frames = Vec::new();
        let mut at = Some(lv);
        while let Some(l) = at.filter(|l| self.doc.levels[*l].kind == LevelKind::Subckt) {
            let here = &self.doc.levels[l];
            let given = |name: &str| -> Option<Entry> {
                if l == lv {
                    if let Some(own) = own {
                        return own.iter().find(|(n, _)| here.same_name(n, name)).map(|(_, e)| e.clone());
                    }
                }
                let (b, values) = self.bound?;
                (b == l)
                    .then(|| values.iter().find(|(n, _)| here.same_name(n, name)).map(|(_, v)| Entry::Text(v.clone())))
                    .flatten()
            };
            let mut frame: Vec<(String, Entry)> = here
                .params
                .iter()
                .map(|(n, d)| {
                    let e = given(n).unwrap_or_else(|| default_entry(d));
                    (n.clone(), e)
                })
                .collect();
            frame.extend(here.param_statements.iter().map(|(n, v)| (n.clone(), Entry::Text(v.clone()))));
            frames.push(frame);
            at = here.parent;
        }
        frames.push(self.globals.iter().map(|(n, v)| (n.clone(), Entry::Text(v.clone()))).collect());
        Env {
            frames,
            sensitive: level.case_sensitive,
            spectre: level.case_sensitive,
            memo: HashMap::new(),
            busy: Vec::new(),
            cycles: BTreeSet::new(),
        }
    }

    /// The parameters of the subcircuit `lv`, with the values bound to it, or their defaults.
    pub(crate) fn level(&self, lv: LevelId) -> Vec<SubcktParam> {
        let level = &self.doc.levels[lv];
        let bound = self.bound.filter(|(b, _)| *b == lv).map(|(_, v)| v);
        let mut env = self.env(lv, None);
        level
            .params
            .iter()
            .enumerate()
            .map(|(k, (name, default))| {
                let given = bound.and_then(|v| v.iter().find(|(n, _)| level.same_name(n, name)).map(|(_, v)| v));
                let default = written(default);
                let (value, from) = match (given, &default) {
                    (Some(v), _) => (Some(v.clone()), "bound"),
                    (None, Some(d)) => (Some(d.clone()), "default"),
                    (None, None) => (None, "unset"),
                };
                SubcktParam { name: name.clone(), default, value, number: env.get(0, k), from }
            })
            .collect()
    }

    /// The parameters `inst` writes, evaluated in the level `parent` it is in.
    pub(crate) fn actuals(&self, parent: LevelId, inst: &Instance) -> Vec<ParamValue> {
        let mut env = self.env(parent, None);
        inst.params
            .iter()
            .map(|(name, value)| ParamValue { name: name.clone(), value: value.clone(), number: env.evaluate(value) })
            .collect()
    }

    /// The parameters of the subcircuit `child` for the instance `inst` of it in the level
    /// `parent`: those the instance writes, evaluated where it is, and the defaults of the others,
    /// evaluated with them.
    pub(crate) fn effective(&self, parent: LevelId, inst: &Instance, child: LevelId) -> Vec<SubcktParam> {
        let level = &self.doc.levels[child];
        let actuals = self.actuals(parent, inst);
        let given = |name: &str| actuals.iter().find(|a| level.same_name(&a.name, name));
        let own: Vec<(String, Entry)> = level
            .params
            .iter()
            .map(|(n, d)| match given(n) {
                Some(a) => (n.clone(), Entry::Fixed(a.number)),
                None => (n.clone(), default_entry(d)),
            })
            .collect();
        let mut env = self.env(child, Some(&own));
        level
            .params
            .iter()
            .enumerate()
            .map(|(k, (name, default))| {
                let default = written(default);
                let (value, from) = match (given(name), &default) {
                    (Some(a), _) => (Some(a.value.clone()), "instance"),
                    (None, Some(d)) => (Some(d.clone()), "default"),
                    (None, None) => (None, "unset"),
                };
                SubcktParam { name: name.clone(), default, value, number: env.get(0, k), from }
            })
            .collect()
    }
}

fn written(default: &str) -> Option<String> {
    (!default.trim().is_empty()).then(|| default.to_string())
}

fn default_entry(default: &str) -> Entry {
    match written(default) {
        Some(d) => Entry::Text(d),
        None => Entry::Unset,
    }
}

fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cur = row[j + 1];
            row[j + 1] = (prev + usize::from(ca != *cb)).min(row[j] + 1).min(cur + 1);
            prev = cur;
        }
    }
    row[b.len()]
}

/// The number `text` comes to, names looked up with `lookup`; none when it does not come to one.
/// `spectre` reads scale factors as Spectre does, by case (`M` is mega, `m` milli); otherwise as
/// SPICE does, without case (`meg` is mega, `m` milli).
pub(crate) fn evaluate(text: &str, spectre: bool, lookup: &mut dyn FnMut(&str) -> Option<f64>) -> Option<f64> {
    let mut t = text.trim();
    // a SPICE expression in braces or single quotes; a double-quoted text is a string
    loop {
        let inner = t
            .strip_prefix('{')
            .and_then(|s| s.strip_suffix('}'))
            .or_else(|| t.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')));
        match inner {
            Some(i) => t = i.trim(),
            None => break,
        }
    }
    let tokens = lex(t, spectre)?;
    let mut p = Parser { tokens, at: 0, lookup };
    let v = p.sum()?;
    (p.at == p.tokens.len()).then_some(v)
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Num(f64),
    Name(String),
    Op(char),
    Pow,
}

fn lex(s: &str, spectre: bool) -> Option<Vec<Tok>> {
    let chars: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut k = 0;
    while k < chars.len() {
        let c = chars[k];
        if c.is_whitespace() {
            k += 1;
        } else if c.is_ascii_digit() || (c == '.' && chars.get(k + 1).is_some_and(|d| d.is_ascii_digit())) {
            let start = k;
            while k < chars.len() && (chars[k].is_ascii_digit() || chars[k] == '.') {
                k += 1;
            }
            if k < chars.len() && matches!(chars[k], 'e' | 'E') {
                let mut j = k + 1;
                if j < chars.len() && matches!(chars[j], '+' | '-') {
                    j += 1;
                }
                if j < chars.len() && chars[j].is_ascii_digit() {
                    while j < chars.len() && chars[j].is_ascii_digit() {
                        j += 1;
                    }
                    k = j;
                }
            }
            let mantissa: f64 = chars[start..k].iter().collect::<String>().parse().ok()?;
            let unit_start = k;
            while k < chars.len() && (chars[k].is_ascii_alphabetic() || chars[k] == '_' || chars[k] == '%') {
                k += 1;
            }
            let unit: String = chars[unit_start..k].iter().collect();
            out.push(Tok::Num(mantissa * scale(&unit, spectre)));
        } else if c.is_alphabetic() || c == '_' {
            let start = k;
            while k < chars.len() && (chars[k].is_alphanumeric() || chars[k] == '_' || chars[k] == '.') {
                k += 1;
            }
            out.push(Tok::Name(chars[start..k].iter().collect()));
        } else if c == '*' && chars.get(k + 1) == Some(&'*') {
            out.push(Tok::Pow);
            k += 2;
        } else if c == '^' {
            out.push(Tok::Pow);
            k += 1;
        } else if "+-*/(),".contains(c) {
            out.push(Tok::Op(c));
            k += 1;
        } else {
            return None;
        }
    }
    Some(out)
}

/// The factor of a scale factor and unit after a number: `u` in `2u`, `meg` in `1meg`, `k` in
/// `10kohm`; the letters after the factor are a unit and count for nothing.
fn scale(unit: &str, spectre: bool) -> f64 {
    if spectre {
        return match unit.chars().next() {
            Some('T') => 1e12,
            Some('G') => 1e9,
            Some('M') => 1e6,
            Some('K' | 'k') => 1e3,
            Some('%' | 'c') => 1e-2,
            Some('m') => 1e-3,
            Some('u') => 1e-6,
            Some('n') => 1e-9,
            Some('p') => 1e-12,
            Some('f') => 1e-15,
            Some('a') => 1e-18,
            _ => 1.0,
        };
    }
    let low = unit.to_lowercase();
    if low.starts_with("meg") {
        return 1e6;
    }
    if low.starts_with("mil") {
        return 25.4e-6;
    }
    match low.chars().next() {
        Some('t') => 1e12,
        Some('g') => 1e9,
        Some('x') => 1e6,
        Some('k') => 1e3,
        Some('m') => 1e-3,
        Some('u') => 1e-6,
        Some('n') => 1e-9,
        Some('p') => 1e-12,
        Some('f') => 1e-15,
        Some('a') => 1e-18,
        _ => 1.0,
    }
}

struct Parser<'l> {
    tokens: Vec<Tok>,
    at: usize,
    lookup: &'l mut dyn FnMut(&str) -> Option<f64>,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Tok> {
        self.tokens.get(self.at)
    }

    fn eat(&mut self, t: &Tok) -> bool {
        if self.peek() == Some(t) {
            self.at += 1;
            true
        } else {
            false
        }
    }

    fn sum(&mut self) -> Option<f64> {
        let mut v = self.product()?;
        loop {
            if self.eat(&Tok::Op('+')) {
                v += self.product()?;
            } else if self.eat(&Tok::Op('-')) {
                v -= self.product()?;
            } else {
                return Some(v);
            }
        }
    }

    fn product(&mut self) -> Option<f64> {
        let mut v = self.unary()?;
        loop {
            if self.eat(&Tok::Op('*')) {
                v *= self.unary()?;
            } else if self.eat(&Tok::Op('/')) {
                v /= self.unary()?;
            } else {
                return Some(v);
            }
        }
    }

    fn unary(&mut self) -> Option<f64> {
        if self.eat(&Tok::Op('-')) {
            return Some(-self.unary()?);
        }
        if self.eat(&Tok::Op('+')) {
            return self.unary();
        }
        self.power()
    }

    fn power(&mut self) -> Option<f64> {
        let base = self.primary()?;
        if self.eat(&Tok::Pow) {
            // right to left: 2^3^2 is 2^9
            let exp = self.unary()?;
            return Some(base.powf(exp));
        }
        Some(base)
    }

    fn primary(&mut self) -> Option<f64> {
        match self.peek()?.clone() {
            Tok::Num(v) => {
                self.at += 1;
                Some(v)
            }
            Tok::Op('(') => {
                self.at += 1;
                let v = self.sum()?;
                self.eat(&Tok::Op(')')).then_some(v)
            }
            Tok::Name(name) => {
                self.at += 1;
                if self.eat(&Tok::Op('(')) {
                    let mut args = Vec::new();
                    if !self.eat(&Tok::Op(')')) {
                        loop {
                            args.push(self.sum()?);
                            if self.eat(&Tok::Op(')')) {
                                break;
                            }
                            if !self.eat(&Tok::Op(',')) {
                                return None;
                            }
                        }
                    }
                    return function(&name.to_lowercase(), &args);
                }
                (self.lookup)(&name)
            }
            _ => None,
        }
    }
}

fn function(name: &str, a: &[f64]) -> Option<f64> {
    let one = || (a.len() == 1).then(|| a[0]);
    let two = || (a.len() == 2).then(|| (a[0], a[1]));
    Some(match name {
        "sqrt" => one()?.sqrt(),
        "abs" => one()?.abs(),
        "exp" => one()?.exp(),
        "log" | "ln" => one()?.ln(),
        "log10" => one()?.log10(),
        "int" | "trunc" => one()?.trunc(),
        "floor" => one()?.floor(),
        "ceil" => one()?.ceil(),
        "sin" => one()?.sin(),
        "cos" => one()?.cos(),
        "tan" => one()?.tan(),
        "atan" => one()?.atan(),
        "sinh" => one()?.sinh(),
        "cosh" => one()?.cosh(),
        "tanh" => one()?.tanh(),
        "pow" => {
            let (x, y) = two()?;
            x.powf(y)
        }
        "pwr" => {
            let (x, y) = two()?;
            x.signum() * x.abs().powf(y)
        }
        "min" if !a.is_empty() => a.iter().copied().fold(f64::INFINITY, f64::min),
        "max" if !a.is_empty() => a.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(text: &str, spectre: bool) -> Option<f64> {
        evaluate(text, spectre, &mut |n| match n {
            "w" | "W" => Some(2e-6),
            "l" => Some(1e-7),
            _ => None,
        })
    }

    fn close(a: Option<f64>, b: f64) -> bool {
        a.is_some_and(|a| (a - b).abs() <= 1e-12 * b.abs().max(1e-30))
    }

    #[test]
    fn numbers_with_scale_factors_and_expressions() {
        assert!(close(eval("2u", false), 2e-6));
        assert!(close(eval("1MEG", false), 1e6));
        assert!(close(eval("1m", false), 1e-3));
        assert!(close(eval("10kohm", false), 1e4));
        assert!(close(eval("1M", true), 1e6));
        assert!(close(eval("1m", true), 1e-3));
        assert!(close(eval("1.5e-3", false), 1.5e-3));
        assert!(close(eval("{2*w}", false), 4e-6));
        assert!(close(eval("'w/l'", false), 20.0));
        assert!(close(eval("2^3^2", false), 512.0));
        assert!(close(eval("-2**2", false), -4.0));
        assert!(close(eval("max(1, 3, 2) + sqrt(4)", false), 5.0));
        assert!(close(eval("pwr(-8, 1/3)", false), -2.0));
        // what does not come to a number has none
        assert_eq!(eval("\"text\"", false), None);
        assert_eq!(eval("w > 1u ? 1 : 2", false), None);
        assert_eq!(eval("unknown*2", false), None);
        assert_eq!(eval("myfunc(1)", false), None);
        assert_eq!(eval("(1 + 2", false), None);
        assert_eq!(eval("", false), None);
    }
}
