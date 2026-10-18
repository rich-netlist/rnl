//! TOML parsing with raw numeric spans: no configuration number passes through f64.
use crate::options::{Direction, Layer, ObjectiveScope, Objectives, SolverOptions, Term};
use crate::rational::{parse_decimal, Q};
use num_traits::{Signed, ToPrimitive};
use std::collections::BTreeSet;
use std::fmt;
use toml_edit::{Document, Item, Table, Value};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigError {
    pub line: usize,
    pub column: usize,
    pub message: String,
}
impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}, column {}: {}", self.line, self.column, self.message)
    }
}
impl std::error::Error for ConfigError {}
struct Reader<'a> {
    text: &'a str,
}
impl Reader<'_> {
    fn at(&self, offset: usize, message: impl Into<String>) -> ConfigError {
        let prefix = &self.text[..offset.min(self.text.len())];
        ConfigError {
            line: prefix.bytes().filter(|b| *b == b'\n').count() + 1,
            column: prefix.rsplit_once('\n').map_or(prefix, |(_, last)| last).chars().count() + 1,
            message: message.into(),
        }
    }
    fn err(&self, item: &Item, message: impl Into<String>) -> ConfigError {
        self.at(item.span().map_or(0, |s| s.start), message)
    }
    fn val_err(&self, value: &Value, message: impl Into<String>) -> ConfigError {
        self.at(value.span().map_or(0, |s| s.start), message)
    }
    fn table<'a>(&self, item: &'a Item) -> Result<&'a Table, ConfigError> {
        item.as_table().ok_or_else(|| self.err(item, "expected a table"))
    }
    fn string<'a>(&self, item: &'a Item) -> Result<&'a str, ConfigError> {
        item.as_str().ok_or_else(|| self.err(item, "expected a string"))
    }
    fn boolean(&self, item: &Item) -> Result<bool, ConfigError> {
        item.as_bool().ok_or_else(|| self.err(item, "expected true or false"))
    }
    fn number_value(&self, value: &Value) -> Result<Q, ConfigError> {
        if !matches!(value, Value::Integer(_) | Value::Float(_)) {
            return Err(self.val_err(value, "expected a decimal number"));
        }
        let raw = value
            .span()
            .and_then(|s| self.text.get(s))
            .ok_or_else(|| self.val_err(value, "numeric source span is missing"))?;
        parse_decimal(&raw.replace('_', ""))
            .ok_or_else(|| self.val_err(value, "expected a finite exact decimal number"))
    }
    fn number(&self, item: &Item) -> Result<Q, ConfigError> {
        self.number_value(item.as_value().ok_or_else(|| self.err(item, "expected a decimal number"))?)
    }
    fn count(&self, item: &Item) -> Result<u64, ConfigError> {
        let q = self.number(item)?;
        if !q.is_integer() || q.is_negative() {
            return Err(self.err(item, "expected a nonnegative integer count"));
        }
        q.to_integer().to_u64().ok_or_else(|| self.err(item, "integer count is too large"))
    }
    fn direction(&self, item: &Item) -> Result<Direction, ConfigError> {
        Direction::parse(self.string(item)?).ok_or_else(|| self.err(item, "direction must be up, down, left or right"))
    }
    fn keys(&self, table: &Table, allowed: &[&str]) -> Result<(), ConfigError> {
        for (key, item) in table.iter() {
            if !allowed.contains(&key) {
                return Err(self.err(item, format!("unknown configuration key `{key}`")));
            }
        }
        Ok(())
    }
    fn rules(&self, table: &Table, out: &mut SolverOptions) -> Result<(), ConfigError> {
        self.keys(
            table,
            &[
                "grid",
                "tolerance",
                "separate_symbols",
                "separate_gap",
                "route_clearance",
                "beam",
                "route_beam",
                "time_limit",
                "texts_at_anchor",
                "budget",
            ],
        )?;
        for (key, item) in table.iter() {
            match key {
                "grid" | "time_limit" => {
                    let number = if item.as_str() == Some("none") { None } else { Some(self.number(item)?) };
                    if number.as_ref().is_some_and(|q| if key == "grid" { !q.is_positive() } else { q.is_negative() }) {
                        return Err(self.err(item, format!("{key} is out of range")));
                    }
                    if key == "grid" {
                        out.grid = number
                    } else {
                        out.time_limit = number
                    }
                }
                "tolerance" => {
                    let q = self.number(item)?;
                    if !q.is_positive() {
                        return Err(self.err(item, "tolerance must be above 0 (11.7)"));
                    }
                    out.tolerance = q;
                }
                "separate_gap" | "route_clearance" => {
                    let q = self.number(item)?;
                    if q.is_negative() {
                        return Err(self.err(item, format!("{key} must not be negative")));
                    }
                    if key == "separate_gap" {
                        out.separate_gap = q
                    } else {
                        out.route_clearance = q
                    }
                }
                "separate_symbols" => out.separate_symbols = self.boolean(item)?,
                "texts_at_anchor" => {
                    out.texts_at_anchor = self.boolean(item)?;
                    if !out.texts_at_anchor {
                        return Err(self.err(item, "texts_at_anchor is required in this version"));
                    }
                }
                "beam" | "route_beam" => {
                    let n = self.count(item)?;
                    let n = usize::try_from(n).map_err(|_| self.err(item, "count is too large"))?;
                    if n == 0 {
                        return Err(self.err(item, format!("{key} must be positive")));
                    }
                    if key == "beam" {
                        out.beam = n
                    } else {
                        out.route_beam = n
                    }
                }
                "budget" => {
                    for (name, count) in self.table(item)?.iter() {
                        out.budget.set(name, self.count(count)?).map_err(|e| self.err(count, e))?
                    }
                }
                _ => unreachable!(),
            }
        }
        if table.contains_key("beam") && !table.contains_key("route_beam") {
            out.route_beam = out.beam;
        }
        if let Some(b) = table.get("budget").and_then(Item::as_table) {
            if b.contains_key("astar_expansions") && !b.contains_key("soft_expansions") {
                out.budget.soft_expansions = out.budget.astar_expansions;
            }
        }
        Ok(())
    }
    fn term(&self, table: &Table) -> Result<Option<Term>, ConfigError> {
        let kind = table
            .get("kind")
            .ok_or_else(|| self.at(table.span().map_or(0, |s| s.start), "an objective term requires kind"))?;
        let mut term = Term::new(self.string(kind)?);
        let mut allowed = vec!["kind", "weight", "enabled"];
        match term.kind.as_str() {
            "current_direction" => allowed.push("direction"),
            "symbol_direction" => allowed.extend(["direction", "symbols", "reference", "references"]),
            "symbol_alignment" | "symbol_distribution" | "symbol_symmetry" => allowed.push("scope"),
            "pin_wire_alignment" => allowed.extend(["scope", "by_direction"]),
            "area" => allowed.push("include_wires"),
            "aspect" => allowed.extend(["include_wires", "ratio"]),
            _ => {}
        }
        self.keys(table, &allowed)?;
        for (key, item) in table.iter() {
            match key {
                "kind" | "enabled" => {}
                "weight" => term.weight = self.number(item)?,
                "direction" => term.direction = Some(self.direction(item)?),
                "reference" => term.reference = Some(self.direction(item)?),
                "scope" => {
                    term.scope = match self.string(item)? {
                        "group" => ObjectiveScope::Group,
                        "global" => ObjectiveScope::Global,
                        _ => return Err(self.err(item, "scope must be group or global")),
                    }
                }
                "include_wires" => term.include_wires = self.boolean(item)?,
                "by_direction" => term.by_direction = self.boolean(item)?,
                "symbols" => {
                    let a = item.as_array().ok_or_else(|| self.err(item, "symbols must be an array of names"))?;
                    for v in a.iter() {
                        term.symbols.push(
                            v.as_str().ok_or_else(|| self.val_err(v, "a symbol name must be a string"))?.to_owned(),
                        );
                    }
                }
                "references" => {
                    if let Some(t) = item.as_inline_table() {
                        for (name, v) in t.iter() {
                            let d = v
                                .as_str()
                                .and_then(Direction::parse)
                                .ok_or_else(|| self.val_err(v, "a reference must be a cardinal direction"))?;
                            term.references.insert(name.into(), d);
                        }
                    } else if let Some(t) = item.as_table() {
                        for (name, v) in t.iter() {
                            term.references.insert(name.into(), self.direction(v)?);
                        }
                    } else {
                        return Err(self.err(item, "references must be a table"));
                    }
                }
                "ratio" => {
                    let a = item
                        .as_array()
                        .filter(|a| a.len() == 2)
                        .ok_or_else(|| self.err(item, "ratio must be two positive integers"))?;
                    let n = self.number_value(a.get(0).expect("ratio length checked"))?;
                    let d = self.number_value(a.get(1).expect("ratio length checked"))?;
                    if !n.is_integer() || !d.is_integer() || !n.is_positive() || !d.is_positive() {
                        return Err(self.err(item, "ratio must be two positive integers"));
                    }
                    term.ratio = n / d;
                }
                _ => unreachable!(),
            }
        }
        term.symbols.sort();
        term.symbols.dedup();
        if term.references.keys().any(|s| !term.symbols.contains(s)) {
            return Err(self.err(kind, "references contains a symbol outside symbols"));
        }
        term.validate().map_err(|e| self.err(kind, e))?;
        let enabled = table.get("enabled").map(|i| self.boolean(i)).transpose()?.unwrap_or(true);
        Ok(enabled.then_some(term))
    }
}

pub(crate) fn parse(text: &str, mut out: SolverOptions) -> Result<SolverOptions, ConfigError> {
    let r = Reader { text };
    let doc = Document::parse(text).map_err(|e| r.at(e.span().map_or(0, |s| s.start), e.message()))?;
    let root = doc.as_table();
    r.keys(root, &["format", "rules", "level"])?;
    if let Some(format) = root.get("format") {
        if r.string(format)? != "rnl-solver-config/1" {
            return Err(r.err(format, "unsupported solver config format"));
        }
    }
    if let Some(rules) = root.get("rules") {
        r.rules(r.table(rules)?, &mut out)?;
    }
    if let Some(levels) = root.get("level") {
        let mut layers = Vec::new();
        let mut seen = BTreeSet::new();
        if levels.as_array().is_some_and(|a| a.is_empty()) {
            out.objectives = Objectives::none();
            return Ok(out);
        }
        let tables =
            levels.as_array_of_tables().ok_or_else(|| r.err(levels, "level must be [[level]] tables or []"))?;
        for table in tables.iter() {
            r.keys(table, &["term", "tolerance"])?;
            let tolerance = table.get("tolerance").map(|i| r.number(i)).transpose()?.unwrap_or_default();
            if tolerance.is_negative() {
                return Err(r.err(&table["tolerance"], "a level tolerance must not be negative"));
            }
            let terms = table
                .get("term")
                .and_then(Item::as_array_of_tables)
                .ok_or_else(|| r.at(table.span().map_or(0, |s| s.start), "a level requires [[level.term]] tables"))?;
            let mut items = Vec::new();
            for t in terms.iter() {
                if let Some(term) = r.term(t)? {
                    if !seen.insert(term.key()) {
                        return Err(r.at(
                            t.span().map_or(0, |s| s.start),
                            format!("objective `{}` with identical parameters is given twice", term.kind),
                        ));
                    }
                    items.push(term);
                }
            }
            if items.is_empty() {
                continue;
            }
            if items.len() > 1 && items.iter().any(|t| !t.integer()) {
                return Err(
                    r.at(table.span().map_or(0, |s| s.start), "a non-integer objective must occupy a level by itself")
                );
            }
            layers.push(Layer { items, tolerance });
        }
        out.objectives = Objectives { layers };
    }
    out.objectives.normalize();
    out.validate().map_err(|e| r.at(0, e))?;
    Ok(out)
}
