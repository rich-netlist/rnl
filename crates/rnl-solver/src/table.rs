//! The table of the solver's settings (14.10), for a tool that draws a page of them from data: each
//! key with the kind of its value, its bounds, its default, whether a file may give it, and a line
//! that says what it does; the objectives, each with what it prefers; and the default objectives.
//!
//! The defaults are those of a 0.4 level, read from [`SolverOptions::defaults_config`] and
//! [`DEFAULT_LAYERS`], so that the table cannot say another default than the solver takes. A level
//! of an earlier version is laid out in rows and reads none of the file's settings; the table does
//! not repeat that, as the check of a level says what it takes.
//!
//! Each setting, objective and stage says the layouts it applies to (`applies`), none when it
//! applies to every layout; and the table says how the line layout ranks its drawings, an order
//! no setting changes ([`LINES_ORDER`]), for a page to show.
//!
//! Rows are only ever added, and so are fields: a tool takes a kind it does not know for text.
//! The lines are English; their translations are kept apart from the solver.

use rnl_runtime::interp::obj::{Lit, WStr};

use crate::{Layer, SolverOptions, DEFAULT_LAYERS, OBJECTIVES};

/// The version of the table's form. It changes only if a field changes its meaning.
pub const SETTINGS_TABLE_VERSION: u32 = 1;

/// The kind of a setting's value: how a page draws its field and reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingKind {
    /// A whole number, within `min` and `max` where they are given.
    Integer,
    /// A finite number, within the bounds given.
    Number,
    /// A number within the bounds given, or `false`, which `false_what` says the meaning of.
    NumberOrFalse,
    Boolean,
    String,
    /// One of `choices`, a string.
    Choice,
    /// The layered objectives as a string, as `objectives` takes them.
    Objectives,
    /// An array of `length` whole numbers, each within `min` and `max`, named by `labels`.
    Integers,
}

impl SettingKind {
    /// Its name in the table: `integer`, `number`, `number-or-false`, `boolean`, `string`,
    /// `choice`, `objectives` or `integers`.
    pub fn name(self) -> &'static str {
        match self {
            SettingKind::Integer => "integer",
            SettingKind::Number => "number",
            SettingKind::NumberOrFalse => "number-or-false",
            SettingKind::Boolean => "boolean",
            SettingKind::String => "string",
            SettingKind::Choice => "choice",
            SettingKind::Objectives => "objectives",
            SettingKind::Integers => "integers",
        }
    }
}

/// A value a setting of [`SettingKind::Choice`] may take.
#[derive(Clone, Debug, PartialEq)]
pub struct SettingChoice {
    pub value: &'static str,
    pub what: &'static str,
}

/// A condition on other settings: each key, and the values of it under which the condition holds.
/// All must hold. Values are strings, as the settings it names are choices.
pub type When = &'static [(&'static str, &'static [&'static str])];

/// One setting.
#[derive(Clone, Debug, PartialEq)]
pub struct SettingRow {
    pub key: &'static str,
    /// The name of its group ([`SettingsTable::groups`]).
    pub group: &'static str,
    pub kind: SettingKind,
    /// For a choice, the values it may take.
    pub choices: &'static [SettingChoice],
    pub min: Option<f64>,
    /// A bound the value must be above, not reach.
    pub min_exclusive: Option<f64>,
    pub max: Option<f64>,
    /// Its value when nothing gives it; none when it has none, such as `work`.
    pub default: Option<Lit>,
    /// Whether a file may give it, in SOLVER-CONFIG; a run setting is the tool's alone.
    pub in_file: bool,
    pub what: &'static str,
    /// For [`SettingKind::NumberOrFalse`], what `false` means.
    pub false_what: Option<&'static str>,
    /// Where the solver uses it: a page may grey it out elsewhere. Empty when everywhere.
    pub applies: When,
    /// Where a page shows it. Empty when always.
    pub shown_when: When,
    /// Where it must be given.
    pub required_when: When,
    /// For [`SettingKind::Integers`], how many numbers it holds.
    pub length: Option<usize>,
    /// For [`SettingKind::Integers`], what each number is, in English, for a page to name its box.
    pub labels: &'static [&'static str],
}

/// A group of settings, in the order a page shows them.
#[derive(Clone, Debug, PartialEq)]
pub struct SettingGroup {
    pub name: &'static str,
    pub what: &'static str,
}

/// An objective (plan 5.4).
#[derive(Clone, Debug, PartialEq)]
pub struct ObjectiveRow {
    pub name: &'static str,
    pub what: &'static str,
    /// Whether this version computes it: one it does not is inert when given.
    pub implemented: bool,
    /// The layouts that take it, as [`SettingRow::applies`]: empty when every layout does. The
    /// line layout takes only overlaps, crowding, misleading and wire_over_symbol, and another
    /// objective only in a layer with one of them, which it takes whole.
    pub applies: When,
}

/// A rank of the line layout's fixed order ([`LINES_ORDER`]).
#[derive(Clone, Debug, PartialEq)]
pub struct OrderRow {
    pub name: &'static str,
    pub what: &'static str,
}

/// The table: what [`settings_table`] gives.
#[derive(Clone, Debug, PartialEq)]
pub struct SettingsTable {
    pub version: u32,
    pub groups: Vec<SettingGroup>,
    pub settings: Vec<SettingRow>,
    /// Every objective, in the order of [`OBJECTIVES`].
    pub objectives: Vec<ObjectiveRow>,
    /// The objectives a solve takes when none are given, layer by layer.
    pub default_objectives: Vec<Layer>,
    /// How the line layout ranks its drawings ([`LINES_ORDER_WHAT`]), first rank first.
    pub lines_order: Vec<OrderRow>,
}

const GROUPS: [SettingGroup; 5] = [
    SettingGroup { name: "layout", what: "Layout" },
    SettingGroup { name: "search", what: "Search and preferences" },
    SettingGroup { name: "run", what: "Limit of each level's solve" },
    SettingGroup { name: "caps", what: "Caps on the search and the router" },
    SettingGroup { name: "report", what: "Report" },
];

const LAYOUTS: [SettingChoice; 3] = [
    SettingChoice {
        value: "rows",
        what: "Components in rows by how deep they are along the current, with fixed gaps.",
    },
    SettingChoice { value: "tree", what: "Groups arranged in nested rows and columns, the wires routed between them." },
    SettingChoice {
        value: "lines",
        what: "Components placed along the lines of their wires, then the drawing compacted on the grid.",
    },
];

const LIMITS: [SettingChoice; 3] = [
    SettingChoice { value: "time", what: "Each level stops after the given seconds." },
    SettingChoice { value: "work", what: "Each level stops after the given work: the same result on every machine." },
    SettingChoice {
        value: "seed",
        what: "Each level takes the layout's first pass, with its own quick improvements, and searches no further.",
    },
];

const LINES: When = &[("layout", &["lines"])];
const ROWS: When = &[("layout", &["rows"])];
const TREE: When = &[("layout", &["tree"])];
/// What the line layout leaves out: an objective it does not take.
const NOT_LINES: When = &[("layout", &["rows", "tree"])];

/// What the line layout's order is, for a page to show beside it.
pub const LINES_ORDER_WHAT: &str = "The lines layout ranks the drawings it makes in a fixed order, the first rank \
     deciding first, which no setting changes but the objectives it takes: overlaps, crowding, misleading and \
     wire_over_symbol, with the layers that hold them, in the order of those layers. The same order ranks the \
     group-tree layout it starts from and chooses between the two drawings.";

/// How the line layout ranks its drawings, first rank first: the outcomes of its search
/// (`lines::settle`), then the line-layout order (`lines::quality::LabOrder`), by which it also
/// weighs its drawing against the group-tree layout it starts from.
pub const LINES_ORDER: [OrderRow; 19] = [
    OrderRow { name: "complete", what: "A drawing of the whole level made." },
    OrderRow { name: "facts", what: "Fewer kept facts broken." },
    OrderRow { name: "false_joins", what: "Fewer places where wires read as joined where they are not." },
    OrderRow { name: "low_facts", what: "Fewer kept facts of priority low broken." },
    OrderRow { name: "groups", what: "Fewer groups dissolved or stretched." },
    OrderRow { name: "routed", what: "Fewer parts of the nets left unrouted." },
    OrderRow { name: "merge", what: "Fewer conflicts where the drawings of the groups are joined." },
    OrderRow { name: "annotations", what: "Fewer overlaps of annotations, then fewer wires through them." },
    OrderRow { name: "rules", what: "The objectives it takes, layer by layer, as configured." },
    OrderRow { name: "current", what: "Fewer pins whose wire leaves other than along their CURRENT." },
    OrderRow { name: "crossings_and_bends", what: "Fewer crossings plus four times the bends of the wires." },
    OrderRow { name: "hidden_ports", what: "Fewer ports whose way out a symbol or another net's wire shuts in." },
    OrderRow { name: "rail_heights", what: "The grounds and the supplies at fewer heights." },
    OrderRow { name: "mirror_pairs", what: "Fewer mirror pairs off the axis they share." },
    OrderRow { name: "dots_near_pins", what: "Fewer junction dots less than two from a pin." },
    OrderRow { name: "length", what: "Shorter wires." },
    OrderRow { name: "wire_lines", what: "The wires on fewer lines." },
    OrderRow { name: "symbol_lines", what: "The symbols on fewer rows and columns." },
    OrderRow { name: "area", what: "A smaller drawing." },
];

/// The layouts that pass through a stage of a solve's progress, by its name, as
/// [`SettingRow::applies`]: the search, the coordinates, the routing and the scoring of candidates
/// are those of rows, and each of the group tree and the line layout has a stage of its own. The
/// others, empty, are not of one layout: every solve's, and `respace`, that of a respace whatever
/// the layout.
pub fn stage_applies(name: &str) -> When {
    match name {
        "search" | "coords" | "route" | "score" => ROWS,
        "tree" => TREE,
        "lines" => LINES,
        _ => &[],
    }
}

/// The largest count a setting may give: the integers a number of RNL holds exactly.
const MOST: f64 = 9_007_199_254_740_992.0;

/// The objectives' lines, in the order of [`OBJECTIVES`].
const OBJECTIVE_WHAT: [&str; 18] = [
    "Transistors with their source or emitter toward the ground, the current running down.",
    "Ground symbols below their pins.",
    "Inputs on the left and outputs on the right.",
    "Components sharing rows and columns.",
    "Even gaps between neighbouring components of a group.",
    "Mirror pairs drawn symmetric.",
    "Wires with fewer bends.",
    "Fewer crossings of wires of different nets.",
    "Shorter wires.",
    "A smaller drawing.",
    "A drawing near 16:9.",
    "Wires leaving terminals in line.",
    "Labels clear of the drawing.",
    "Bends of different wires in line.",
    "No symbols overlapping.",
    "Nothing drawn too close, and no text over another symbol.",
    "No wires that seem to connect what they do not.",
    "No wire through a symbol.",
];

/// The table of the solver's settings and objectives.
pub fn settings_table() -> SettingsTable {
    let defaults = rnl_runtime::solver_settings::parse_overrides(&SolverOptions::defaults_config()).unwrap_or_default();
    let default = |key: &str| defaults.iter().find(|s| s.key == key).map(|s| s.value.clone());
    let objectives = DEFAULT_LAYERS.join(",");
    let row = |key: &'static str, group: &'static str, kind: SettingKind, what: &'static str| SettingRow {
        key,
        group,
        kind,
        choices: &[],
        min: None,
        min_exclusive: None,
        max: None,
        default: default(key),
        in_file: !crate::is_run_setting(key),
        what,
        false_what: None,
        applies: &[],
        shown_when: &[],
        required_when: &[],
        length: None,
        labels: &[],
    };
    let spacing = |key, what| SettingRow {
        min: Some(-1.0),
        max: Some(20.0),
        applies: LINES,
        ..row(key, "layout", SettingKind::Integer, what)
    };
    let settings = vec![
        SettingRow { choices: &LAYOUTS, ..row("layout", "layout", SettingKind::Choice, "How the level is laid out.") },
        spacing("spacing_x", "Grid units added to the gaps between columns of the line layout, -1 to close them up; respacing uses it in every layout."),
        spacing("spacing_y", "Grid units added to the gaps between rows of the line layout, -1 to close them up; respacing uses it in every layout."),
        SettingRow {
            min: Some(0.0),
            max: Some(crate::LinesWeights::MAX as f64),
            applies: LINES,
            length: Some(2),
            labels: &["crossing", "bend"],
            ..row(
                "lines_weights",
                "layout",
                SettingKind::Integers,
                "The weights of a crossing and of a bend in the line layout, as [crossing, bend], not both 0: \
                 its drawings rank by crossing × crossings + bend × bends, after the facts, the false joins, \
                 the annotations, the objectives it takes and the pins against their CURRENT.",
            )
        },
        row(
            "join_trees",
            "layout",
            SettingKind::Boolean,
            "Route the wire trees of each net as one, so that a net that several structures wire is drawn \
             connected rather than in pieces joined by its name.",
        ),
        row(
            "junction",
            "layout",
            SettingKind::String,
            "The definition drawn where wires meet in a junction dot, by its name; an empty string draws none.",
        ),
        SettingRow {
            min: Some(1.0),
            max: Some(1024.0),
            applies: ROWS,
            ..row(
                "beam",
                "search",
                SettingKind::Integer,
                "How many partial candidates the row search keeps at each step: more searches wider and slower.",
            )
        },
        SettingRow {
            default: Some(Lit::Str(WStr::new(&objectives, None))),
            ..row(
                "objectives",
                "search",
                SettingKind::Objectives,
                "What the solver prefers among layouts that keep every fact, the most important first: layers \
                 separated by commas, the objectives of a layer joined by +, a weight as name*2, a tolerance of a \
                 layer as ~0.5; none for no preference.",
            )
        },
        SettingRow {
            min_exclusive: Some(0.0),
            false_what: Some("Positions are not rounded to a grid."),
            ..row("grid", "search", SettingKind::NumberOrFalse, "The step the positions are rounded to.")
        },
        SettingRow {
            min: Some(0.0),
            ..row(
                "tolerance",
                "search",
                SettingKind::Number,
                "How far a coordinate may miss a fixed value of a fact and the fact still hold.",
            )
        },
        SettingRow {
            choices: &LIMITS,
            ..row("limit", "run", SettingKind::Choice, "What ends the solve of each level.")
        },
        SettingRow {
            min: Some(0.0),
            shown_when: &[("limit", &["time"])],
            ..row("seconds", "run", SettingKind::Number, "The seconds each level's solve may take.")
        },
        SettingRow {
            min: Some(0.0),
            max: Some(MOST),
            shown_when: &[("limit", &["work"])],
            required_when: &[("limit", &["work"])],
            ..row(
                "work",
                "run",
                SettingKind::Integer,
                "The work each level's solve may do; a solve reports the work it did, to be run again alike.",
            )
        },
        SettingRow {
            min: Some(0.0),
            shown_when: &[("limit", &["work", "seed"])],
            ..row(
                "safety_seconds",
                "run",
                SettingKind::Number,
                "The seconds after which a solve limited by work or to the seed stops all the same.",
            )
        },
        SettingRow {
            min: Some(0.0),
            max: Some(MOST),
            applies: ROWS,
            ..row("candidates", "caps", SettingKind::Integer, "The most candidates the row search makes over a level.")
        },
        SettingRow {
            min: Some(0.0),
            max: Some(MOST),
            ..row("astar_expansions", "caps", SettingKind::Integer, "The most cells the router expands for one net.")
        },
        SettingRow {
            min: Some(0.0),
            max: Some(MOST),
            false_what: Some("The group-tree start refines until its search ends or the budget runs out."),
            applies: LINES,
            shown_when: &[("limit", &["time", "work"])],
            ..row(
                "tree_work",
                "caps",
                SettingKind::NumberOrFalse,
                "The work of the level at which the lines layout's group-tree start stops refining its \
                 arrangement. Left out, it is the lesser of the solver's estimate of 5 seconds and 0.3 of the \
                 level's limit. A solve reports it with its work, so that a run is replayed alike.",
            )
        },
        SettingRow {
            default: Some(Lit::Bool(false)),
            ..row(
                "report_hints",
                "report",
                SettingKind::Boolean,
                "Give every conflict of a discarded fact of priority low in the report, which can take much longer.",
            )
        },
    ];
    let objectives = OBJECTIVES
        .iter()
        .zip(OBJECTIVE_WHAT)
        .zip(crate::objectives::TABLE.iter())
        .map(|((name, what), entry)| {
            let lines = crate::objectives::Objective::from_name(name)
                .is_some_and(|o| crate::config::LINES_OBJECTIVES.contains(&o));
            ObjectiveRow { name, what, implemented: entry.implemented, applies: if lines { &[] } else { NOT_LINES } }
        })
        .collect();
    SettingsTable {
        version: SETTINGS_TABLE_VERSION,
        groups: GROUPS.to_vec(),
        settings,
        objectives,
        default_objectives: crate::Objectives::default().layers,
        lines_order: LINES_ORDER.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_key_the_solver_takes_is_in_the_table_once_in_a_group() {
        let t = settings_table();
        let keys: Vec<&str> = t.settings.iter().map(|s| s.key).collect();
        for key in [
            "layout",
            "spacing_x",
            "spacing_y",
            "junction",
            "tolerance",
            "grid",
            "beam",
            "objectives",
            "limit",
            "seconds",
            "work",
            "safety_seconds",
            "candidates",
            "astar_expansions",
            "tree_work",
            "report_hints",
        ] {
            assert_eq!(keys.iter().filter(|k| **k == key).count(), 1, "{key}");
        }
        // the old spelling of the time limit is not offered
        assert!(!keys.contains(&"time_limit"));
        for s in &t.settings {
            assert!(t.groups.iter().any(|g| g.name == s.group), "{}", s.key);
            assert_eq!(s.in_file, !crate::is_run_setting(s.key), "{}", s.key);
            assert!(!s.what.is_empty() && s.what.ends_with('.'), "{}", s.key);
            assert_eq!(s.kind == SettingKind::Choice, !s.choices.is_empty(), "{}", s.key);
            for (key, values) in s.applies.iter().chain(s.shown_when).chain(s.required_when) {
                let other = t.settings.iter().find(|o| o.key == *key).unwrap();
                assert!(values.iter().all(|v| other.choices.iter().any(|c| c.value == *v)), "{} {key}", s.key);
            }
        }
    }

    #[test]
    fn the_defaults_are_the_solvers() {
        let t = settings_table();
        // every default given, as a tool would give it, is taken, and makes the default options
        let text = t
            .settings
            .iter()
            .filter_map(|s| Some(format!("{}={}", s.key, s.default.as_ref()?.show())))
            .collect::<Vec<_>>()
            .join(",");
        let o = SolverOptions::default().with_config(&text).unwrap();
        let d = SolverOptions::default();
        assert_eq!(o.settings.layout, d.settings.layout);
        assert_eq!(o.settings.layout, crate::Layout::Lines);
        assert_eq!(
            (o.settings.spacing, o.beam, o.grid, o.tolerance),
            (d.settings.spacing, d.beam, d.grid, d.tolerance)
        );
        assert_eq!(
            (o.budget, o.objectives.clone(), o.hints_for_low, o.safety_seconds),
            (d.budget, d.objectives, d.hints_for_low, d.safety_seconds)
        );
        // the run settings are the tool's, taken together when a solve begins: none of them refused
        let tool = rnl_runtime::solver_settings::parse_overrides(&text).unwrap();
        let tool: Vec<_> = tool.into_iter().map(|s| (s, false)).collect();
        let check = SolverOptions::check_settings(&tool, None);
        assert!(check.errors.is_empty(), "{:?}", check.errors);
        // none for what has no default
        for key in ["junction", "work"] {
            assert_eq!(t.settings.iter().find(|s| s.key == key).unwrap().default, None, "{key}");
        }
        // the bounds of a number are those the solver checks
        let refused = |text: &str| SolverOptions::default().with_config(text).is_err();
        // a value of a setting: of an array of integers, the value first and the rest 1
        let at = |s: &SettingRow, v: f64| match s.length {
            Some(n) => format!("{}=[{v}{}]", s.key, ", 1".repeat(n - 1)),
            None => format!("{}={v}", s.key),
        };
        for s in &t.settings {
            if let Some(min) = s.min {
                assert!(refused(&at(s, min - 1.0)), "{} below {min}", s.key);
                assert!(!refused(&at(s, min)) || s.key == "work", "{} at {min}", s.key);
            }
            if let Some(max) = s.max.filter(|m| *m < MOST) {
                assert!(refused(&at(s, max + 1.0)), "{} above {max}", s.key);
                assert!(!refused(&at(s, max)), "{} at {max}", s.key);
            }
            if let Some(min) = s.min_exclusive {
                assert!(refused(&format!("{}={min}", s.key)), "{} at {min}", s.key);
            }
            if s.kind == SettingKind::NumberOrFalse {
                assert!(!refused(&format!("{}=false", s.key)), "{} false", s.key);
            }
            for c in s.choices {
                assert!(!refused(&format!("{}=\"{}\"", s.key, c.value)) || s.key == "limit", "{} {}", s.key, c.value);
            }
        }
    }

    #[test]
    fn every_objective_has_a_line_and_says_whether_it_is_computed() {
        let t = settings_table();
        assert_eq!(t.objectives.iter().map(|o| o.name).collect::<Vec<_>>(), OBJECTIVES);
        let off: Vec<&str> = t.objectives.iter().filter(|o| !o.implemented).map(|o| o.name).collect();
        assert_eq!(off, ["terminal_wire_alignment", "labels", "bend_alignment"]);
        assert!(t.objectives.iter().all(|o| o.what.ends_with('.')));
        let names: Vec<&str> =
            t.default_objectives.iter().flat_map(|l| l.items.iter().map(|(n, _)| n.as_str())).collect();
        assert_eq!(names, DEFAULT_LAYERS);
    }

    /// The line layout takes four objectives, which every layout takes, and none of the others;
    /// its own order is said, a line for each rank.
    #[test]
    fn the_objectives_say_the_layouts_that_take_them() {
        let t = settings_table();
        let every: Vec<&str> = t.objectives.iter().filter(|o| o.applies.is_empty()).map(|o| o.name).collect();
        assert_eq!(every, ["overlaps", "crowding", "misleading", "wire_over_symbol"]);
        assert!(t.objectives.iter().all(|o| o.applies.is_empty() || o.applies == NOT_LINES));
        assert_eq!(t.lines_order.len(), LINES_ORDER.len());
        let mut names: Vec<&str> = t.lines_order.iter().map(|r| r.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), LINES_ORDER.len());
        assert!(t.lines_order.iter().all(|r| r.what.ends_with('.')));
        assert_eq!(stage_applies("search"), ROWS);
        assert_eq!(stage_applies("lines"), LINES);
        assert!(stage_applies("resolve").is_empty());
    }
}
