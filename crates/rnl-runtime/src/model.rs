//! The effective model: what each drawing level looks like after the RNL takes effect.
//!
//! Renderers, viewers and a future solver read this and nothing of the interpreter's internals
//! (architecture 4.5). Coordinates are scene units, x to the right and y up. Styles hold only the
//! properties some RNL gave; a renderer fills in its own defaults (9.3, principle 2.7).

use std::collections::BTreeMap;

use crate::geom::{Orient, Pt};
use crate::interp::obj::{Lit, PrimParams, Shape};
use crate::markup::Line;

pub type Style = BTreeMap<String, Lit>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Layer {
    Bottom,
    #[default]
    Middle,
    Top,
}

impl Layer {
    pub fn parse(s: &str) -> Layer {
        match s {
            "bottom" => Layer::Bottom,
            "top" => Layer::Top,
            _ => Layer::Middle,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Layer::Bottom => "bottom",
            Layer::Middle => "middle",
            Layer::Top => "top",
        }
    }
}

/// A graphic of a symbol in local coordinates.
#[derive(Clone, Debug)]
pub struct SymbolPrim {
    pub shape: Shape,
    pub points: Vec<Pt>,
    pub params: PrimParams,
    pub style: Style,
    /// For a fixed text: its alignment and rotation; the content is per item (`ItemView::texts`).
    pub halign: Option<String>,
    pub valign: Option<String>,
    pub rotate: Option<f64>,
}

#[derive(Clone, Debug)]
pub struct PinView {
    pub pos: Pt,
    pub label: Option<String>,
    /// Exit direction in degrees: 0 right, 90 up, 180 left, 270 down.
    pub dir: Option<u16>,
    pub terminals: Vec<String>,
}

/// A symbol drawn inside another (9.2.7).
#[derive(Clone, Debug)]
pub struct NestedView {
    pub symbol: usize,
    pub at: Pt,
    pub orient: Orient,
}

#[derive(Clone, Debug)]
pub struct SymbolView {
    pub name: String,
    pub prims: Vec<SymbolPrim>,
    pub pins: Vec<PinView>,
    pub nested: Vec<NestedView>,
    /// The symbol's box (9.2.3): x1, y1, x2, y2 in local coordinates. When the symbol writes no
    /// `box=`, the extent of all its graphics, text excluded.
    pub bbox: [f64; 4],
    /// The symbol writes `box=`.
    pub box_written: bool,

    /// The viewer's own stand-in for an instance with no symbol (9.2.6).
    pub default: bool,
}

/// An instance, bundle or usage drawn with a symbol.
#[derive(Clone, Debug)]
pub struct ItemView {
    pub id: String,
    pub kind: &'static str,
    pub symbol: usize,
    pub at: Pt,
    pub orient: Orient,
    pub style: Style,
    pub layer: Layer,
    pub order: Vec<f64>,
    /// The fixed texts of the symbol with their `${…}` values, by path: "3" is prim 3, "1/3" is
    /// prim 3 of nested symbol 1.
    pub texts: BTreeMap<String, Vec<Line>>,
}

#[derive(Clone, Debug)]
pub struct WireView {
    pub id: String,
    pub nets: Vec<String>,
    /// The vertices after the routing mode (13.1).
    pub points: Vec<Pt>,
    pub mode: String,
    pub style: Style,
    pub layer: Layer,
    pub order: Vec<f64>,
}

/// A graphic written in the scene: an annotation (13.4).
#[derive(Clone, Debug)]
pub struct ShapeView {
    pub shape: Shape,
    pub points: Vec<Pt>,
    pub params: PrimParams,
    pub style: Style,
    pub layer: Layer,
    pub order: Vec<f64>,
}

#[derive(Clone, Debug)]
pub struct TextView {
    pub id: String,
    pub anchor: Pt,
    pub lines: Vec<Line>,
    pub halign: String,
    pub valign: String,
    pub rotate: f64,
    /// A text written in a symbol turns with its instance (9.2.5); scene texts do not.
    pub orient: Orient,
    pub style: Style,
    pub layer: Layer,
    pub order: Vec<f64>,
}

#[derive(Clone, Debug)]
pub struct ConstraintView {
    pub label: String,
    pub kind: String,
    /// ok, dropped, needs-solver, unsolved, disabled, failed.
    pub status: String,
    pub priority: String,
}

#[derive(Clone, Debug)]
pub struct LevelView {
    pub title: String,
    pub kind: &'static str,
    pub name: String,
    pub file: String,
    pub line: u32,
    pub symbols: Vec<SymbolView>,
    pub items: Vec<ItemView>,
    pub wires: Vec<WireView>,
    pub shapes: Vec<ShapeView>,
    pub texts: Vec<TextView>,
    /// Lines a viewer may draw for nets that a ROUTE asks to wire (13.7); not wires.
    pub flylines: Vec<(Pt, Pt)>,
    pub constraints: Vec<ConstraintView>,
    /// Drawn objects without a position, left to a solver.
    pub unplaced: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct Model {
    pub levels: Vec<LevelView>,
    /// Set when the whole interpretation failed (1.5 rule 3, 7.6).
    pub failed: Option<String>,
}
