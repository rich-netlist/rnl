//! The solver's options: the tolerance (11.7), the grid, the layered objectives, the beam and the
//! budget, and their canonical text, which the options digest of `sig1` covers (chapter 17).
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
//! tolerance=0.000001;grid=1;objectives=current_direction,turns,…;beam=4;budget=resolve_nodes:10000,…
//! ```
//!
//! - `objectives` lists the layers in order, separated by `,`; the objectives of a layer are
//!   separated by `+`, each with `*` and its weight when the weight is not 1, and a layer with a
//!   tolerance other than 0 ends with `~` and the tolerance: `area+wire_length*2~0.5`. A layer is
//!   a sum, so the order of its objectives means nothing: the text writes them sorted by name,
//!   and `wire_length*2+area` reads as the same layer. `none` is no objective at all.
//! - `grid` is a number, or `none` to leave free coordinates where the LP puts them.
//! - `budget` lists `name:count` pairs separated by `,`; an override may give some of them.

use rnl_runtime::geom::{fmt_num, TOL};

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

/// How much work the solver may do, counted in steps rather than time so that the result does not
/// depend on the machine (plan 5.9). When a count runs out, the solver keeps the best it has: a
/// fact it could not decide is discarded and reported as undecided, a search stops with the best
/// candidate so far.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Budget {
    /// The nodes of the search for the literals of one fact, in the resolution of 8.5.
    pub resolve_nodes: u64,
    /// The pivots of each simplex the solver builds: one per axis for the resolution of 8.5, for
    /// each test of a conflict set, for each analysis of the rows, and for each candidate taken in
    /// and placed. A simplex that reaches it stops: a fact it was deciding is discarded undecided,
    /// a relation of the solver's own is dropped, an LP gives up and the candidate stays where its
    /// resolution put it, and the report says so. The pivots of a level are the sum over all of
    /// them ([`crate::Work::pivots`]), which this does not bound. A pivot rewrites the rows it
    /// touches with exact rationals, so its cost grows with the size of the tableau and of its
    /// numbers: the count bounds the work, not the time, and the default of 1,000,000 is far
    /// more than a level of a few hundred objects needs, a backstop rather than a limit on time.
    pub pivots: u64,
    /// The candidates the search makes over the whole level.
    pub candidates: u64,
    /// The cells the router expands for one net.
    pub astar_expansions: u64,
}

impl Default for Budget {
    fn default() -> Self {
        Budget { resolve_nodes: 10_000, pivots: 1_000_000, candidates: 256, astar_expansions: 200_000 }
    }
}

impl Budget {
    const NAMES: [&'static str; 4] = ["resolve_nodes", "pivots", "candidates", "astar_expansions"];

    fn get(&mut self, name: &str) -> Option<&mut u64> {
        Some(match name {
            "resolve_nodes" => &mut self.resolve_nodes,
            "pivots" => &mut self.pivots,
            "candidates" => &mut self.candidates,
            "astar_expansions" => &mut self.astar_expansions,
            _ => return None,
        })
    }

    fn canonical(&self) -> String {
        let Budget { resolve_nodes, pivots, candidates, astar_expansions } = self;
        format!(
            "resolve_nodes:{resolve_nodes},pivots:{pivots},candidates:{candidates},astar_expansions:{astar_expansions}"
        )
    }

    /// Reads `name:count` pairs over this budget.
    fn read(&mut self, text: &str) -> Result<(), String> {
        let mut given: Vec<&str> = Vec::new();
        for pair in text.split(',') {
            let Some((name, count)) = pair.split_once(':') else {
                return Err(format!("a budget is `name:count`, not `{}`", pair.trim()));
            };
            let name = name.trim();
            let slot = self
                .get(name)
                .ok_or_else(|| format!("the budget has no `{name}`; it has {}", Budget::NAMES.join(", ")))?;
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
}

impl Default for SolverOptions {
    fn default() -> Self {
        SolverOptions {
            tolerance: TOL,
            grid: Some(1.0),
            objectives: Objectives::default(),
            beam: 4,
            budget: Budget::default(),
        }
    }
}

const KEYS: [&str; 5] = ["tolerance", "grid", "objectives", "beam", "budget"];

impl SolverOptions {
    /// The options as one string, every setting that changes the result, in a fixed order with
    /// numbers by `fmt_num`: what the options digest of `sig1` hashes (chapter 17).
    pub fn canonical(&self) -> String {
        let grid = self.grid.map_or_else(|| "none".to_string(), fmt_num);
        format!(
            "tolerance={};grid={grid};objectives={};beam={};budget={}",
            fmt_num(self.tolerance),
            self.objectives.canonical(),
            self.beam,
            self.budget.canonical()
        )
    }

    /// Options from their text: the defaults, with the settings the text gives.
    pub fn parse(text: &str) -> Result<SolverOptions, String> {
        SolverOptions::default().with_overrides(text)
    }

    /// These options with the settings of `text` over them: `key=value` pairs separated by `;`,
    /// as [`SolverOptions::canonical`] writes them. `objectives` replaces the whole list, `budget`
    /// only the counts it names. An unknown key, a key given twice, or a value that does not
    /// read is an error.
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
                _ => options.budget.read(value)?,
            }
        }
        options.validate()?;
        Ok(options)
    }

    /// Whether the options make sense: a tolerance not below 0, a grid above 0, objectives that
    /// [`Objectives::validate`] accepts, a beam of at least 1.
    pub fn validate(&self) -> Result<(), String> {
        if !(self.tolerance.is_finite() && self.tolerance >= 0.0) {
            return Err(format!("the tolerance must be a number not below 0, not {}", self.tolerance));
        }
        if let Some(grid) = self.grid {
            if !(grid.is_finite() && grid > 0.0) {
                return Err(format!("the grid must be a number above 0, not {grid}"));
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
        budget=resolve_nodes:10000,pivots:1000000,candidates:256,astar_expansions:200000";

    #[test]
    fn the_defaults_in_their_canonical_text() {
        let options = SolverOptions::default();
        assert_eq!(options.canonical(), DEFAULT);
        assert_eq!(options.tolerance, TOL);
        assert_eq!(options.objectives.layers.len(), 11);
        assert!(options
            .objectives
            .layers
            .iter()
            .all(|l| l.items.len() == 1 && l.items[0].1 == 1 && l.tolerance == 0.0));
        assert_eq!(options.objectives.layer_of("turns"), Some(1));
        assert!(!options.objectives.is_on("signal_flow"));
        assert_eq!(options.validate(), Ok(()));
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
            budget: Budget { resolve_nodes: 5, pivots: 6, candidates: 7, astar_expansions: 8 },
        };
        let text = options.canonical();
        assert_eq!(
            text,
            "tolerance=0.001;grid=none;objectives=signal_flow,area+wire_length*2~0.5,crossings*3;beam=7;\
             budget=resolve_nodes:5,pivots:6,candidates:7,astar_expansions:8"
        );
        assert_eq!(SolverOptions::parse(&text), Ok(options.clone()));
        options.objectives = Objectives::none();
        options.grid = Some(0.5);
        assert!(options.canonical().contains(";grid=0.5;objectives=none;"));
        assert_eq!(SolverOptions::parse(&options.canonical()), Ok(options));
        assert_eq!(SolverOptions::parse(DEFAULT), Ok(SolverOptions::default()));
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
        let options = SolverOptions::parse(" beam = 2 ; budget=pivots:500 ;grid=none;").unwrap();
        assert_eq!(options.beam, 2);
        assert_eq!(options.grid, None);
        assert_eq!(options.budget, Budget { pivots: 500, ..Budget::default() });
        assert_eq!(options.objectives, Objectives::default());
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
            "budget=pivots",
            "budget=speed:3",
            "budget=pivots:-3",
            "budget=pivots:1,pivots:2",
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
