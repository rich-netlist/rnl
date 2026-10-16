//! The solver's options: the tolerance (11.7), the grid, the layered objectives, the beam, the
//! budget and the time limit of the resolution (the draft 0.3, 8.5), and their canonical text,
//! which the options digest of `sig1` covers (chapter 17).
//!
//! The objectives are configured in **layers**: the objectives of a layer add up with their
//! weights, layers rank lexicographically, the first layer first, and a value of a layer within its
//! tolerance of the least value of that layer counts as equal to it. An objective left out is off:
//! it is not computed, does not order candidates and breaks no ties. The default order of the
//! layers is the prototype's, one objective per layer with weight 1 ([`DEFAULT_LAYERS`]).
//!
//! [`SolverOptions::canonical`] writes every setting that changes the result in a fixed order,
//! numbers by `fmt_num`, so the same options give the same text, and so the same hash, on every
//! platform. [`SolverOptions::parse`] reads that text back, and [`SolverOptions::with_overrides`]
//! reads some of its settings over other options, as `rnl solve --solver-options` does:
//!
//! ```text
//! tolerance=0.000001;grid=1;objectives=current_direction,turns,…;beam=4;budget=candidates:256,…;time_limit=5
//! ```
//!
//! - `objectives` lists the layers in order, separated by `,`; the objectives of a layer are
//!   separated by `+`, each with `*` and its weight when the weight is not 1, and a layer with a
//!   tolerance other than 0 ends with `~` and the tolerance: `area+wire_length*2~0.5`. A layer is
//!   a sum, so the order of its objectives means nothing: the text writes them sorted by name,
//!   and `wire_length*2+area` reads as the same layer. `none` is no objective at all.
//! - `grid` is a number, or `none` to leave free coordinates where the LP puts them.
//! - `budget` lists `name:count` pairs separated by `,`; an override may give some of them.
//! - `time_limit` is a number of seconds, or `none` for no limit.

use rnl_runtime::geom::{fmt_num, TOL};

/// The time limit of the toolkit, in seconds: the standard does not fix one (the draft 0.3, 8.5).
pub const DEFAULT_TIME_LIMIT: f64 = 5.0;

/// The objectives the solver knows, by the names of the prototype (plan 5.4).
pub const OBJECTIVES: [&str; 14] = [
    "current_direction",
    "ground_direction",
    "signal_flow",
    "component_alignment",
    "distribution",
    "symmetry",
    "turns",
    "crossings",
    "wire_length",
    "area",
    "aspect",
    "terminal_wire_alignment",
    "labels",
    "bend_alignment",
];

/// The default layers, one objective each with weight 1, in this order: the prototype's order,
/// which the user kept on 2026-10-16 (plan 5.4).
pub const DEFAULT_LAYERS: [&str; 11] = [
    "current_direction",
    "turns",
    "ground_direction",
    "component_alignment",
    "distribution",
    "symmetry",
    "terminal_wire_alignment",
    "area",
    "crossings",
    "wire_length",
    "aspect",
];

/// One layer of objectives: their weighted sum is one component of a candidate's score.
#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    /// The objectives, by name ([`OBJECTIVES`]), with their weights. Their order means nothing:
    /// [`SolverOptions::parse`] gives them sorted by name, and the canonical text writes them so.
    pub items: Vec<(String, u32)>,
    /// Where candidates are ranked, a value of this layer no more than this above the least value
    /// of the layer counts as equal to it, and the next layer decides among those (plan 5.8); 0 by
    /// default. Each layer is held against its least value, never one candidate against another,
    /// so the ranking is well defined whatever the tolerances. The coordinate LP of a candidate
    /// has layers of its own, linear stand-ins (plan 5.6), and keeps each of their optima
    /// exactly: this tolerance does not reach it.
    pub tolerance: f64,
}

impl Layer {
    /// A layer of one objective with weight 1 and no tolerance.
    pub fn single(name: &str) -> Layer {
        Layer { items: vec![(name.to_string(), 1)], tolerance: 0.0 }
    }
}

/// The objectives, layer by layer, the most important first.
#[derive(Clone, Debug, PartialEq)]
pub struct Objectives {
    pub layers: Vec<Layer>,
}

impl Default for Objectives {
    fn default() -> Self {
        Objectives { layers: DEFAULT_LAYERS.iter().map(|name| Layer::single(name)).collect() }
    }
}

impl Objectives {
    /// No objective: the solver only satisfies the facts.
    pub fn none() -> Objectives {
        Objectives { layers: Vec::new() }
    }

    /// The layer of an objective, none when it is off.
    pub fn layer_of(&self, name: &str) -> Option<usize> {
        self.layers.iter().position(|layer| layer.items.iter().any(|(n, _)| n == name))
    }

    /// Whether an objective is on.
    pub fn is_on(&self, name: &str) -> bool {
        self.layer_of(name).is_some()
    }

    fn canonical(&self) -> String {
        if self.layers.is_empty() {
            return "none".to_string();
        }
        let layer = |l: &Layer| {
            // a layer is a sum: its objectives by name, so that equal layers are written alike
            let mut sorted: Vec<&(String, u32)> = l.items.iter().collect();
            sorted.sort();
            let items: Vec<String> = sorted
                .into_iter()
                .map(|(name, weight)| if *weight == 1 { name.clone() } else { format!("{name}*{weight}") })
                .collect();
            let mut text = items.join("+");
            if l.tolerance != 0.0 {
                text.push('~');
                text.push_str(&fmt_num(l.tolerance));
            }
            text
        };
        self.layers.iter().map(layer).collect::<Vec<_>>().join(",")
    }

    fn parse(text: &str) -> Result<Objectives, String> {
        if text.trim() == "none" {
            return Ok(Objectives::none());
        }
        let mut layers = Vec::new();
        for layer in text.split(',') {
            let (items, tolerance) = match layer.split_once('~') {
                Some((items, t)) => (items, number("the tolerance of a layer", t)?),
                None => (layer, 0.0),
            };
            let mut parsed = Vec::new();
            for item in items.split('+') {
                let (name, weight) = match item.split_once('*') {
                    Some((name, w)) => {
                        let w = w.trim();
                        (
                            name.trim(),
                            w.parse::<u32>()
                                .map_err(|_| format!("the weight of `{}` is not a count: `{w}`", name.trim()))?,
                        )
                    }
                    None => (item.trim(), 1),
                };
                parsed.push((name.to_string(), weight));
            }
            parsed.sort();
            layers.push(Layer { items: parsed, tolerance });
        }
        let objectives = Objectives { layers };
        objectives.validate()?;
        Ok(objectives)
    }

    /// Every name known, none twice, every weight at least 1, every tolerance finite and not
    /// negative, no layer empty.
    pub fn validate(&self) -> Result<(), String> {
        let mut seen: Vec<&str> = Vec::new();
        for layer in &self.layers {
            if layer.items.is_empty() {
                return Err("a layer of objectives is empty".to_string());
            }
            if !(layer.tolerance.is_finite() && layer.tolerance >= 0.0) {
                return Err(format!("the tolerance of a layer must be a number not below 0, not {}", layer.tolerance));
            }
            for (name, weight) in &layer.items {
                if !OBJECTIVES.contains(&name.as_str()) {
                    return Err(format!(
                        "no objective is called `{name}`; the objectives are {}",
                        OBJECTIVES.join(", ")
                    ));
                }
                if seen.contains(&name.as_str()) {
                    return Err(format!("the objective `{name}` is given twice"));
                }
                if *weight == 0 {
                    return Err(format!("the weight of `{name}` is 0; leave the objective out to turn it off"));
                }
                seen.push(name);
            }
        }
        Ok(())
    }
}

/// How much work the search and the router may do, counted in steps rather than time so that
/// their result does not depend on the machine (plan 5.9). When a count runs out, the solver keeps
/// the best it has: the search stops with the best candidate so far, a net is left in parts.
///
/// The resolution of 8.5 has no budget of work: only the time limit bounds it
/// ([`SolverOptions::time_limit`], the draft 0.3, 8.5). The witness taken as written and the
/// solver's own uses of the resolution, for the relations of a candidate and the coordinate LP,
/// stay counted, by caps that the implementation fixes rather than options (`Caps`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Budget {
    /// The candidates the search makes over the whole level.
    pub candidates: u64,
    /// The cells the router expands for one net.
    pub astar_expansions: u64,
}

impl Default for Budget {
    fn default() -> Self {
        Budget { candidates: 256, astar_expansions: 200_000 }
    }
}

/// The work that the uses of the resolution other than the resolution of 8.5 itself may do: the
/// witness taken as written (`crate::resolve`), the relations of a candidate taken in after the
/// facts kept, the analysis of the rows, and the coordinate LP. They are not the checks of 8.5,
/// which only the time limit bounds ([`SolverOptions::time_limit`]); they are counted, so that
/// their result does not depend on the machine, by caps that the implementation fixes ([`CAPS`]),
/// backstops far above what a level of a few hundred objects needs. A search that reaches its cap
/// drops its relation without a conflict, a simplex that reaches its cap stops and the LP leaves
/// the candidate where its resolution put it, the witness is the resolution's, within the
/// tolerance, when it cannot be taken as written; the report says so in a note each time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Caps {
    /// The nodes of the search for the literals of one relation.
    pub nodes: u64,
    /// The pivots of each simplex. A pivot rewrites the rows it touches with exact rationals, so
    /// its cost grows with the size of the tableau and of its numbers: the count bounds the work,
    /// not the time. The pivots of a level are the sum over every simplex ([`crate::Work::pivots`]),
    /// which this does not bound.
    pub pivots: u64,
}

/// The caps of the solver's own uses of the resolution: those of the budget before the time
/// limit replaced it for the resolution of 8.5.
pub(crate) const CAPS: Caps = Caps { nodes: 10_000, pivots: 1_000_000 };

impl Budget {
    const NAMES: [&'static str; 2] = ["candidates", "astar_expansions"];

    fn get(&mut self, name: &str) -> Option<&mut u64> {
        Some(match name {
            "candidates" => &mut self.candidates,
            "astar_expansions" => &mut self.astar_expansions,
            _ => return None,
        })
    }

    fn canonical(&self) -> String {
        let Budget { candidates, astar_expansions } = self;
        format!("candidates:{candidates},astar_expansions:{astar_expansions}")
    }

    /// Reads `name:count` pairs over this budget.
    fn read(&mut self, text: &str) -> Result<(), String> {
        let mut given: Vec<&str> = Vec::new();
        for pair in text.split(',') {
            let Some((name, count)) = pair.split_once(':') else {
                return Err(format!("a budget is `name:count`, not `{}`", pair.trim()));
            };
            let name = name.trim();
            let slot = self.get(name).ok_or_else(|| {
                let mut e = format!("the budget has no `{name}`; it has {}", Budget::NAMES.join(", "));
                if matches!(name, "resolve_nodes" | "pivots") {
                    e.push_str("; the resolution of 8.5 is bounded by `time_limit` instead");
                }
                e
            })?;
            *slot = count
                .trim()
                .parse::<u64>()
                .map_err(|_| format!("the budget `{name}` is not a count: `{}`", count.trim()))?;
            if given.contains(&name) {
                return Err(format!("the budget `{name}` is given twice"));
            }
            given.push(name);
        }
        Ok(())
    }
}

/// The options of the solver: every setting that changes its result.
#[derive(Clone, Debug, PartialEq)]
pub struct SolverOptions {
    /// Differences within it count as satisfied (11.7): the constant bounds of the facts are
    /// widened by it, never the equalities between variables (plan section 4).
    pub tolerance: f64,
    /// The grid free coordinates are rounded to when the facts allow it; none to leave them as
    /// the LP gives them. Rounding is the solver's own rule, ranked after all descriptions (11.7).
    pub grid: Option<f64>,
    /// The layered objectives.
    pub objectives: Objectives,
    /// How many variants of each group the search keeps, and how many complete candidates it
    /// routes and scores.
    pub beam: usize,
    pub budget: Budget,
    /// How many seconds the resolution of a level may spend on checking, none for no limit (the
    /// draft 0.3, 8.5): the checks of the facts, each on its own first and then in order, and the
    /// search for all their minimal conflict groups, nothing else, on the clock the caller gives
    /// (`crate::solve_with_clock`). When the check of a fact does not finish within it, the fact
    /// is discarded undecided, with a warning (7.6), and so is every later fact that needs a
    /// check; a later fact that the placement reached by then already satisfies is kept. A fact
    /// whose check on its own does not finish is undecided too, not failed. The result may then
    /// differ from that of 8.5, and from one run to the next; without a limit, and whenever no
    /// check reaches it, it is that of 8.5. The placement is then taken from the facts kept with
    /// their constants as written all the same, which is not timed, so that a fact kept before the
    /// time ran out puts its objects where it says. When the groups are not all found within it,
    /// those found are given, each minimal, and the hint says that its list is not complete
    /// (`crate::Discarded::complete`). [`DEFAULT_TIME_LIMIT`] by default.
    pub time_limit: Option<f64>,
    /// Whether a fact of priority low that is discarded gets every minimal conflict group it has
    /// and its place in the hints by statement (the draft 0.3, 8.5). Discarding such a fact is
    /// expected, and its hints may be left out (8.5, 7.6); so, false by default, it gets only its
    /// first group, which the deletion filter makes of the explanation of its conflict, with
    /// `complete` false (`crate::Discarded::complete`), it is in no hint by statement, and a note
    /// says that the groups of such facts are listed only with `rnl solve --report`, which sets
    /// this. Enumerating every group of a low fact can cost far more than the rest of the
    /// resolution. It changes no fact kept and no placement, only the report, so it is not in the
    /// canonical text, and not in the hash (chapter 17), nor an option `--solver-options` reads.
    pub hints_for_low: bool,
}

impl Default for SolverOptions {
    fn default() -> Self {
        SolverOptions {
            tolerance: TOL,
            grid: Some(1.0),
            objectives: Objectives::default(),
            beam: 4,
            budget: Budget::default(),
            time_limit: Some(DEFAULT_TIME_LIMIT),
            hints_for_low: false,
        }
    }
}

const KEYS: [&str; 6] = ["tolerance", "grid", "objectives", "beam", "budget", "time_limit"];

impl SolverOptions {
    /// The defaults without a time limit: the resolution is exactly that of 8.5, the same on every
    /// run and every machine, however long it takes. What the tests use.
    pub fn without_time_limit() -> SolverOptions {
        SolverOptions { time_limit: None, ..SolverOptions::default() }
    }

    /// The options as one string, every setting that changes the result, in a fixed order with
    /// numbers by `fmt_num`: what the options digest of `sig1` hashes (chapter 17). The time limit
    /// is among them, as it may change which facts are kept (8.5); `hints_for_low`, which changes
    /// only the report, is not.
    pub fn canonical(&self) -> String {
        let none = |v: Option<f64>| v.map_or_else(|| "none".to_string(), fmt_num);
        format!(
            "tolerance={};grid={};objectives={};beam={};budget={};time_limit={}",
            fmt_num(self.tolerance),
            none(self.grid),
            self.objectives.canonical(),
            self.beam,
            self.budget.canonical(),
            none(self.time_limit)
        )
    }

    /// Options from their text: the defaults, with the settings the text gives.
    pub fn parse(text: &str) -> Result<SolverOptions, String> {
        SolverOptions::default().with_overrides(text)
    }

    /// These options with the settings of `text` over them: `key=value` pairs separated by `;`,
    /// as [`SolverOptions::canonical`] writes them. `objectives` replaces the whole list, `budget`
    /// only the counts it names; `grid` and `time_limit` take `none` as well as a number. An
    /// unknown key, a key given twice, or a value that does not read is an error.
    pub fn with_overrides(&self, text: &str) -> Result<SolverOptions, String> {
        let mut options = self.clone();
        let mut given: Vec<&str> = Vec::new();
        for setting in text.split(';') {
            if setting.trim().is_empty() {
                continue;
            }
            let Some((key, value)) = setting.split_once('=') else {
                return Err(format!("a setting is `key=value`, not `{}`", setting.trim()));
            };
            let (key, value) = (key.trim(), value.trim());
            if !KEYS.contains(&key) {
                return Err(format!("the solver has no option `{key}`; it has {}", KEYS.join(", ")));
            }
            if given.contains(&key) {
                return Err(format!("the option `{key}` is given twice"));
            }
            given.push(key);
            match key {
                "tolerance" => options.tolerance = number("the tolerance", value)?,
                "grid" if value == "none" => options.grid = None,
                "grid" => options.grid = Some(number("the grid", value)?),
                "objectives" => options.objectives = Objectives::parse(value)?,
                "beam" => {
                    options.beam = value.parse::<usize>().map_err(|_| format!("the beam is not a count: `{value}`"))?
                }
                "time_limit" if value == "none" => options.time_limit = None,
                "time_limit" => options.time_limit = Some(number("the time limit", value)?),
                _ => options.budget.read(value)?,
            }
        }
        options.validate()?;
        Ok(options)
    }

    /// Whether the options make sense: a tolerance not below 0, a grid above 0, objectives that
    /// [`Objectives::validate`] accepts, a beam of at least 1, a time limit not below 0.
    pub fn validate(&self) -> Result<(), String> {
        if !(self.tolerance.is_finite() && self.tolerance >= 0.0) {
            return Err(format!("the tolerance must be a number not below 0, not {}", self.tolerance));
        }
        if let Some(grid) = self.grid {
            if !(grid.is_finite() && grid > 0.0) {
                return Err(format!("the grid must be a number above 0, not {grid}"));
            }
        }
        if let Some(limit) = self.time_limit {
            if !(limit.is_finite() && limit >= 0.0) {
                return Err(format!("the time limit must be a number of seconds not below 0, or none, not {limit}"));
            }
        }
        if self.beam == 0 {
            return Err("the beam must be at least 1".to_string());
        }
        self.objectives.validate()
    }
}

/// A finite number.
fn number(what: &str, text: &str) -> Result<f64, String> {
    let text = text.trim();
    match text.parse::<f64>() {
        Ok(v) if v.is_finite() => Ok(v),
        _ => Err(format!("{what} is not a number: `{text}`")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEFAULT: &str = "tolerance=0.000001;grid=1;objectives=current_direction,turns,ground_direction,\
        component_alignment,distribution,symmetry,terminal_wire_alignment,area,crossings,wire_length,aspect;beam=4;\
        budget=candidates:256,astar_expansions:200000;time_limit=5";

    #[test]
    fn the_defaults_in_their_canonical_text() {
        let options = SolverOptions::default();
        assert_eq!(options.canonical(), DEFAULT);
        assert_eq!(options.tolerance, TOL);
        // the toolkit's time limit of the resolution: 5 seconds (the draft 0.3, 8.5)
        assert_eq!(options.time_limit, Some(5.0));
        let unlimited = SolverOptions::without_time_limit();
        assert_eq!(unlimited, SolverOptions { time_limit: None, ..SolverOptions::default() });
        assert_eq!(unlimited.canonical(), DEFAULT.replace(";time_limit=5", ";time_limit=none"));
        assert_eq!(options.objectives.layers.len(), 11);
        assert!(options
            .objectives
            .layers
            .iter()
            .all(|l| l.items.len() == 1 && l.items[0].1 == 1 && l.tolerance == 0.0));
        assert_eq!(options.objectives.layer_of("turns"), Some(1));
        assert!(!options.objectives.is_on("signal_flow"));
        assert_eq!(options.validate(), Ok(()));
        // the groups of the facts of priority low are not all sought by default; asking for them
        // changes only the report, so not the canonical text, and no setting reads it (8.5, 7.6)
        assert!(!options.hints_for_low);
        let low = SolverOptions { hints_for_low: true, ..SolverOptions::default() };
        assert_eq!(low.canonical(), DEFAULT);
        assert!(SolverOptions::parse("hints_for_low=true").unwrap_err().contains("no option `hints_for_low`"));
    }

    #[test]
    fn the_canonical_text_reads_back_as_the_same_options() {
        let mut options = SolverOptions {
            tolerance: 0.001,
            grid: None,
            objectives: Objectives {
                layers: vec![
                    Layer::single("signal_flow"),
                    Layer { items: vec![("area".to_string(), 1), ("wire_length".to_string(), 2)], tolerance: 0.5 },
                    Layer { items: vec![("crossings".to_string(), 3)], tolerance: 0.0 },
                ],
            },
            beam: 7,
            budget: Budget { candidates: 7, astar_expansions: 8 },
            time_limit: None,
            hints_for_low: false,
        };
        let text = options.canonical();
        assert_eq!(
            text,
            "tolerance=0.001;grid=none;objectives=signal_flow,area+wire_length*2~0.5,crossings*3;beam=7;\
             budget=candidates:7,astar_expansions:8;time_limit=none"
        );
        assert_eq!(SolverOptions::parse(&text), Ok(options.clone()));
        options.objectives = Objectives::none();
        options.grid = Some(0.5);
        options.time_limit = Some(0.25);
        assert!(options.canonical().contains(";grid=0.5;objectives=none;"));
        assert!(options.canonical().ends_with(";time_limit=0.25"));
        assert_eq!(SolverOptions::parse(&options.canonical()), Ok(options));
        assert_eq!(SolverOptions::parse(DEFAULT), Ok(SolverOptions::default()));
    }

    #[test]
    fn the_time_limit_reads_back_as_a_number_or_none() {
        for (text, limit) in [("time_limit=5", Some(5.0)), ("time_limit=none", None), ("time_limit=0", Some(0.0))] {
            let options = SolverOptions::parse(text).unwrap();
            assert_eq!(options.time_limit, limit, "{text}");
            assert!(options.canonical().ends_with(&format!(";{text}")), "{}", options.canonical());
            assert_eq!(SolverOptions::parse(&options.canonical()), Ok(options));
        }
        // over options without a limit, a number sets one again
        let five = SolverOptions::without_time_limit().with_overrides("time_limit=5").unwrap();
        assert_eq!(five, SolverOptions::default());
        // the limit is part of the options the hash covers
        assert_ne!(SolverOptions::default().canonical(), SolverOptions::without_time_limit().canonical());
        // the counts of work that the resolution had before are gone, and the error says why
        for gone in ["budget=resolve_nodes:10", "budget=pivots:10"] {
            let e = SolverOptions::parse(gone).unwrap_err();
            assert!(e.contains("it has candidates, astar_expansions") && e.contains("`time_limit`"), "{e}");
        }
    }

    #[test]
    fn the_order_within_a_layer_means_nothing() {
        let a = SolverOptions::parse("objectives=area+wire_length*2~0.5,turns").unwrap();
        let b = SolverOptions::parse("objectives=wire_length*2+area~0.5,turns").unwrap();
        assert_eq!(a, b);
        assert_eq!(a.canonical(), b.canonical());
        assert!(a.canonical().contains(";objectives=area+wire_length*2~0.5,turns;"));
        // options made in code with the items in another order are written alike too
        let mut c = a.clone();
        c.objectives.layers[0].items.reverse();
        assert_eq!(c.canonical(), a.canonical());
        // the order of the layers does mean something
        let d = SolverOptions::parse("objectives=turns,area+wire_length*2~0.5").unwrap();
        assert_ne!(d.canonical(), a.canonical());
    }

    #[test]
    fn overrides_change_only_what_they_give() {
        let options = SolverOptions::parse(" beam = 2 ; budget=candidates:500 ;grid=none;").unwrap();
        assert_eq!(options.beam, 2);
        assert_eq!(options.grid, None);
        assert_eq!(options.budget, Budget { candidates: 500, ..Budget::default() });
        assert_eq!(options.objectives, Objectives::default());
        assert_eq!(options.time_limit, Some(DEFAULT_TIME_LIMIT));
        let options = options.with_overrides("objectives=turns + wire_length*3, area ~ 1e-3; tolerance=1e-4").unwrap();
        assert_eq!(options.tolerance, 0.0001);
        assert_eq!(options.objectives.layers[0].items, vec![("turns".to_string(), 1), ("wire_length".to_string(), 3)]);
        assert_eq!(options.objectives.layers[1].tolerance, 0.001);
        assert_eq!(options.beam, 2);
        assert_eq!(SolverOptions::parse(""), Ok(SolverOptions::default()));
    }

    #[test]
    fn what_does_not_read_is_an_error() {
        let bad = [
            "beam",
            "speed=3",
            "beam=2;beam=3",
            "beam=0",
            "beam=-1",
            "tolerance=-1",
            "tolerance=NaN",
            "tolerance=inf",
            "grid=0",
            "grid=x",
            "objectives=",
            "objectives=beauty",
            "objectives=turns,turns",
            "objectives=turns*0",
            "objectives=turns*x",
            "objectives=turns~-1",
            "objectives=turns+",
            "budget=candidates",
            "budget=speed:3",
            "budget=candidates:-3",
            "budget=candidates:1,candidates:2",
            "budget=pivots:1000000",
            "budget=resolve_nodes:10000",
            "time_limit=",
            "time_limit=-1",
            "time_limit=inf",
            "time_limit=NaN",
            "time_limit=5s",
            "time_limit=5;time_limit=none",
        ];
        for text in bad {
            let result = SolverOptions::parse(text);
            assert!(result.is_err(), "{text}: {result:?}");
        }
        let mut options = SolverOptions::default();
        options.objectives.layers.push(Layer { items: Vec::new(), tolerance: 0.0 });
        assert!(options.validate().is_err());
    }

    #[test]
    fn every_objective_has_a_name_and_the_defaults_are_among_them() {
        assert!(DEFAULT_LAYERS.iter().all(|name| OBJECTIVES.contains(name)));
        let mut sorted = OBJECTIVES.to_vec();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), OBJECTIVES.len());
        // the names need no quoting in the canonical text
        assert!(OBJECTIVES.iter().all(|n| n.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')));
    }
}
