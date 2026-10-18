//! Exact effective configuration, independent of any filesystem or host.
use crate::rational::{exact_text, int, parse_decimal, Q};
use num_traits::{One, Signed, Zero};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

pub const DEFAULT_CONFIG: &str = include_str!("../../../config/rnl-solver.toml");
pub const DEFAULT_TIME_LIMIT: f64 = 5.0;
pub const OBJECTIVES: [&str; 17] = [
    "symbol_overlaps",
    "wire_shorts",
    "wire_overlaps",
    "label_overlaps",
    "current_direction",
    "symbol_direction",
    "symbol_alignment",
    "pin_wire_alignment",
    "symbol_distribution",
    "symbol_symmetry",
    "wire_turns",
    "wire_crossings",
    "area",
    "aspect",
    "wire_length",
    "orientation_r0",
    "origin_distance",
];
pub const DEFAULT_LAYERS: [&str; 13] = [
    "symbol_overlaps",
    "wire_shorts",
    "wire_overlaps",
    "label_overlaps",
    "current_direction",
    "wire_turns",
    "symbol_direction",
    "symbol_alignment",
    "symbol_distribution",
    "area",
    "wire_crossings",
    "aspect",
    "wire_length",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}
impl Direction {
    pub fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "up" => Self::Up,
            "down" => Self::Down,
            "left" => Self::Left,
            "right" => Self::Right,
            _ => return None,
        })
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Down => "down",
            Self::Left => "left",
            Self::Right => "right",
        }
    }
    pub(crate) fn vector(self) -> (i64, i64) {
        match self {
            Self::Up => (0, 1),
            Self::Down => (0, -1),
            Self::Left => (-1, 0),
            Self::Right => (1, 0),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ObjectiveScope {
    Group,
    Global,
}
impl ObjectiveScope {
    pub fn name(self) -> &'static str {
        match self {
            Self::Group => "group",
            Self::Global => "global",
        }
    }
}

/// One configured measure; parameters are normalized before execution and hashing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Term {
    pub kind: String,
    pub weight: Q,
    pub direction: Option<Direction>,
    pub scope: ObjectiveScope,
    pub symbols: Vec<String>,
    pub reference: Option<Direction>,
    pub references: BTreeMap<String, Direction>,
    pub ratio: Q,
    pub include_wires: bool,
    pub by_direction: bool,
}
impl Term {
    pub fn new(kind: &str) -> Self {
        Self {
            kind: kind.into(),
            weight: int(1),
            direction: None,
            scope: ObjectiveScope::Group,
            symbols: Vec::new(),
            reference: None,
            references: BTreeMap::new(),
            ratio: Q::new(16.into(), 9.into()),
            include_wires: false,
            by_direction: true,
        }
    }
    pub fn integer(&self) -> bool {
        !matches!(self.kind.as_str(), "symbol_symmetry" | "area" | "aspect" | "wire_length" | "origin_distance")
    }
    pub fn parameters(&self) -> Value {
        let mut p = BTreeMap::new();
        match self.kind.as_str() {
            "current_direction" | "symbol_direction" => {
                if let Some(d) = self.direction {
                    p.insert("direction", json!(d.name()));
                }
                if self.kind == "symbol_direction" {
                    p.insert("symbols", json!(self.symbols));
                    if let Some(d) = self.reference {
                        p.insert("reference", json!(d.name()));
                    }
                    if !self.references.is_empty() {
                        p.insert(
                            "references",
                            json!(self.references.iter().map(|(k, v)| (k, v.name())).collect::<BTreeMap<_, _>>()),
                        );
                    }
                }
            }
            "symbol_alignment" | "symbol_distribution" | "symbol_symmetry" | "pin_wire_alignment" => {
                p.insert("scope", json!(self.scope.name()));
                if self.kind == "pin_wire_alignment" {
                    p.insert("by_direction", json!(self.by_direction));
                }
            }
            "area" | "aspect" => {
                p.insert("include_wires", json!(self.include_wires));
                if self.kind == "aspect" {
                    p.insert("ratio", json!(exact_text(&self.ratio)));
                }
            }
            _ => {}
        }
        json!(p)
    }
    pub fn key(&self) -> (String, String) {
        (self.kind.clone(), canonical_json(&self.parameters()))
    }
    pub fn validate(&self) -> Result<(), String> {
        if !OBJECTIVES.contains(&self.kind.as_str()) {
            return Err(format!("unknown objective `{}`", self.kind));
        }
        if matches!(self.kind.as_str(), "symbol_symmetry" | "pin_wire_alignment" | "orientation_r0" | "origin_distance")
        {
            return Err(format!("objective `{}` is not implemented in this version", self.kind));
        }
        if !self.weight.is_positive() {
            return Err(format!("the weight of `{}` must be above 0", self.kind));
        }
        if matches!(self.kind.as_str(), "current_direction" | "symbol_direction") && self.direction.is_none() {
            return Err(format!("`{}` requires direction", self.kind));
        }
        if self.kind == "symbol_direction" && self.symbols.is_empty() {
            return Err("symbol_direction requires a nonempty symbols list".into());
        }
        if !self.ratio.is_positive() {
            return Err("ratio must contain two positive integers".into());
        }
        if matches!(self.kind.as_str(), "area" | "aspect") && self.include_wires {
            return Err(format!("include_wires=true for `{}` is not implemented in this version", self.kind));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layer {
    pub items: Vec<Term>,
    pub tolerance: Q,
}
impl Layer {
    pub fn single(kind: &str) -> Self {
        Self { items: vec![Term::new(kind)], tolerance: Q::zero() }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Objectives {
    pub layers: Vec<Layer>,
}
impl Objectives {
    pub fn none() -> Self {
        Self::default()
    }
    pub fn layer_of(&self, kind: &str) -> Option<usize> {
        self.layers.iter().position(|l| l.items.iter().any(|t| t.kind == kind))
    }
    pub fn is_on(&self, kind: &str) -> bool {
        self.layer_of(kind).is_some()
    }
    pub fn validate(&self) -> Result<(), String> {
        let mut seen = BTreeSet::new();
        for l in &self.layers {
            if l.items.is_empty() {
                return Err("an objective level is empty".into());
            }
            if l.tolerance.is_negative() {
                return Err("a level tolerance must not be negative".into());
            }
            if l.items.len() > 1 && l.items.iter().any(|t| !t.integer()) {
                return Err("a non-integer objective must occupy a level by itself".into());
            }
            for t in &l.items {
                t.validate()?;
                if !seen.insert(t.key()) {
                    return Err(format!("objective `{}` with identical parameters is given twice", t.kind));
                }
            }
        }
        Ok(())
    }
    pub(crate) fn normalize(&mut self) {
        for l in &mut self.layers {
            for t in &mut l.items {
                t.symbols.sort();
                t.symbols.dedup();
            }
            l.items.sort_by_key(Term::key);
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Budget {
    /// Actual candidate attempts over one level, including P0. Zero permits only P0 (work=1).
    pub candidates: u64,
    pub astar_expansions: u64,
    pub soft_expansions: u64,
    pub symmetry_steps: u64,
}
impl Default for Budget {
    fn default() -> Self {
        Self { candidates: 256, astar_expansions: 200_000, soft_expansions: 200_000, symmetry_steps: 1_000_000 }
    }
}
impl Budget {
    pub(crate) fn set(&mut self, key: &str, value: u64) -> Result<(), String> {
        match key {
            "candidates" => self.candidates = value,
            "astar_expansions" => self.astar_expansions = value,
            "soft_expansions" => self.soft_expansions = value,
            "symmetry_steps" => self.symmetry_steps = value,
            _ => return Err(format!("unknown budget `{key}`")),
        };
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Caps {
    pub nodes: u64,
    pub pivots: u64,
}
pub(crate) const CAPS: Caps = Caps { nodes: 10_000, pivots: 1_000_000 };

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SolverOptions {
    pub tolerance: Q,
    pub grid: Option<Q>,
    pub objectives: Objectives,
    pub beam: usize,
    pub route_beam: usize,
    pub budget: Budget,
    pub time_limit: Option<Q>,
    pub separate_symbols: bool,
    pub separate_gap: Q,
    pub route_clearance: Q,
    pub texts_at_anchor: bool,
    pub hints_for_low: bool,
}
impl Default for SolverOptions {
    fn default() -> Self {
        Self::builtin()
    }
}
impl SolverOptions {
    pub(crate) fn base() -> Self {
        Self {
            tolerance: Q::new(1.into(), 1_000_000.into()),
            grid: Some(int(1)),
            objectives: Objectives::none(),
            beam: 4,
            route_beam: 4,
            budget: Budget::default(),
            time_limit: Some(int(5)),
            separate_symbols: true,
            separate_gap: int(1),
            route_clearance: int(1),
            texts_at_anchor: true,
            hints_for_low: false,
        }
    }
    pub fn builtin() -> Self {
        crate::config::parse(DEFAULT_CONFIG, Self::base()).expect("the bundled solver configuration is valid")
    }
    pub fn from_toml(text: &str) -> Result<Self, crate::config::ConfigError> {
        crate::config::parse(text, Self::builtin())
    }
    pub fn without_time_limit() -> Self {
        Self { time_limit: None, ..Self::builtin() }
    }
    pub fn parse(text: &str) -> Result<Self, String> {
        Self::builtin().with_overrides(text)
    }
    pub fn with_overrides(&self, text: &str) -> Result<Self, String> {
        let mut out = self.clone();
        let mut seen = BTreeSet::new();
        for part in text.split(';').filter(|s| !s.trim().is_empty()) {
            let (key, value) = part.split_once('=').ok_or_else(|| format!("a setting is key=value, not `{part}`"))?;
            let (key, value) = (key.trim(), value.trim());
            if !seen.insert(key) {
                return Err(format!("the option `{key}` is given twice"));
            }
            let number = || parse_decimal(value).ok_or_else(|| format!("`{key}` requires an exact decimal number"));
            let boolean = || value.parse::<bool>().map_err(|_| format!("`{key}` requires true or false"));
            match key {
                "tolerance" => out.tolerance = number()?,
                "grid" => out.grid = if value == "none" { None } else { Some(number()?) },
                "time_limit" => out.time_limit = if value == "none" { None } else { Some(number()?) },
                "separate_symbols" => out.separate_symbols = boolean()?,
                "separate_gap" => out.separate_gap = number()?,
                "route_clearance" => out.route_clearance = number()?,
                "texts_at_anchor" => out.texts_at_anchor = boolean()?,
                "beam" => out.beam = value.parse().map_err(|_| "beam must be a positive count")?,
                "route_beam" => out.route_beam = value.parse().map_err(|_| "route_beam must be a positive count")?,
                "budget" => {
                    for pair in value.split(',') {
                        let (name, count) = pair.split_once(':').ok_or("a budget entry is name:count")?;
                        out.budget
                            .set(name.trim(), count.trim().parse().map_err(|_| "a budget is a nonnegative count")?)?;
                    }
                }
                "objectives" => {
                    return Err(
                        "objectives are configured in a solver TOML file; --solver-options accepts rules only".into()
                    )
                }
                _ => return Err(format!("unknown solver rule `{key}`")),
            }
        }
        out.validate()?;
        Ok(out)
    }
    pub fn validate(&self) -> Result<(), String> {
        if !self.tolerance.is_positive() {
            return Err("tolerance must be above 0 (11.7)".into());
        }
        if self.grid.as_ref().is_some_and(|g| !g.is_positive()) {
            return Err("grid must be above 0 or none".into());
        }
        if self.time_limit.as_ref().is_some_and(Signed::is_negative) {
            return Err("time_limit must not be negative".into());
        }
        if self.separate_gap.is_negative() || self.route_clearance.is_negative() {
            return Err("separate_gap and route_clearance must not be negative".into());
        }
        if self.beam == 0 || self.route_beam == 0 {
            return Err("beam and route_beam must be positive counts".into());
        }
        if !self.texts_at_anchor {
            return Err("texts_at_anchor is a required mechanism in this version".into());
        }
        self.objectives.validate()
    }
    pub fn canonical(&self) -> String {
        let mut objectives = self.objectives.clone();
        objectives.normalize();
        let levels:Vec<Value>=objectives.layers.iter().map(|l| json!({"term":l.items.iter().map(|t|json!({"kind":t.kind,"parameters":t.parameters(),"weight":exact_text(&t.weight)})).collect::<Vec<_>>(),"tolerance":exact_text(&l.tolerance)})).collect();
        canonical_json(&json!({"format":"rnl-solver-config/1","level":levels,"rules":{
            "beam":self.beam,"route_beam":self.route_beam,"budget":{"candidates":self.budget.candidates,"astar_expansions":self.budget.astar_expansions,"soft_expansions":self.budget.soft_expansions,"symmetry_steps":self.budget.symmetry_steps},
            "grid":self.grid.as_ref().map(exact_text),"time_limit":self.time_limit.as_ref().map(exact_text),"tolerance":exact_text(&self.tolerance),"separate_symbols":self.separate_symbols,"separate_gap":exact_text(&self.separate_gap),"route_clearance":exact_text(&self.route_clearance),"texts_at_anchor":self.texts_at_anchor}}))
    }
    pub(crate) fn routing_weights(&self) -> RoutingWeights {
        let mut out = RoutingWeights::default();
        let relevant: Vec<_> = self
            .objectives
            .layers
            .iter()
            .filter(|l| {
                l.items.iter().any(|t| {
                    matches!(
                        t.kind.as_str(),
                        "wire_overlaps" | "wire_shorts" | "wire_crossings" | "wire_turns" | "wire_length"
                    )
                })
            })
            .collect();
        let mut scale = Q::one();
        for l in relevant.into_iter().rev() {
            for t in &l.items {
                let slot = match t.kind.as_str() {
                    "wire_overlaps" => &mut out.wire_overlaps,
                    "wire_shorts" => &mut out.wire_shorts,
                    "wire_crossings" => &mut out.wire_crossings,
                    "wire_turns" => &mut out.wire_turns,
                    "wire_length" => &mut out.wire_length,
                    _ => continue,
                };
                *slot += &t.weight * &scale;
            }
            scale *= int(16);
        }
        out
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RoutingWeights {
    pub wire_overlaps: Q,
    pub wire_shorts: Q,
    pub wire_crossings: Q,
    pub wire_turns: Q,
    pub wire_length: Q,
}
pub(crate) fn canonical_json(value: &Value) -> String {
    fn sorted(v: &Value) -> Value {
        match v {
            Value::Object(m) => Value::Object(
                m.iter().map(|(k, v)| (k.clone(), sorted(v))).collect::<BTreeMap<_, _>>().into_iter().collect(),
            ),
            Value::Array(a) => Value::Array(a.iter().map(sorted).collect()),
            v => v.clone(),
        }
    }
    serde_json::to_string(&sorted(value)).expect("configuration JSON consists of exact strings")
}
