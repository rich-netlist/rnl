//! A solver's result written back as RNL (standard 10.5, 13.3, 13.7, 14.8, 17).
//!
//! A solver gives its result for one level as a [`Solution`]: the position and orientation of
//! the objects of its [`LevelProblem`], and the wires it routed for ROUTE. [`write_record`] turns
//! it into the one record the solver writes at the end of the level's own text,
//! `@RNL(type="generated", priority="high", author="…", hash="…") {…}`, in standard form;
//! [`crate::writeback::replace_generated`] puts it in place of the previous one.
//!
//! - Objects made by rules have no handle of their own (5.7, 10.5): their generating usage is
//!   written out first as an explicit statement with a name [`generated_names`] gives, and the
//!   objects are reached through it, as in `%gnd_stub_M1_s = $GND_STUB @ATTACH(…) #M1:s;
//!   %gnd_stub_M1_s::g @PLACE(0, -4)`. The explicit usage then takes the place of the generated
//!   one when the file is read again.
//! - A routed wire is a WIRE with `mode="straight"` and the style of its ROUTE, through its bends;
//!   the points where wires branch are named points, with `$JUNCTION` where the solver wants a
//!   dot (13.2, 13.7).
//! - Numbers are written by [`fmt_num`], which reads back as the same value (11.7).
//! - What cannot be referred to, such as a free point `(_, _)` with no name or a text without a
//!   key or a name, is left out and reported (5.7, 13.3).

use crate::geom::{fmt_num, Orient, Pt};
use crate::interp::obj::Priority;
use crate::problem::*;

/// Where the solver puts one object of the problem.
#[derive(Clone, Debug, PartialEq)]
pub struct Placement {
    /// The object, by its index in [`LevelProblem::objects`].
    pub object: usize,
    /// Its position; none for a virtual part drawn at the point of its main object, which has no
    /// position of its own (12.1), or when only the orientation is given.
    pub at: Option<Pt>,
    /// Its orientation, for the objects that have one.
    pub orient: Option<Orient>,
}

/// A point where wires routed for ROUTE branch (13.7).
#[derive(Clone, Debug, PartialEq)]
pub struct Branch {
    pub at: Pt,
    /// Draw a dot there (13.2).
    pub junction: bool,
}

/// A point a routed wire passes.
#[derive(Clone, Debug, PartialEq)]
pub enum WirePoint {
    /// A pin, or a point object of the problem.
    Anchor(PointRef),
    /// A branch point of the solution, by its index in [`Solution::branches`].
    Branch(usize),
    /// A bend.
    Bend(Pt),
}

/// A wire routed for a ROUTE.
#[derive(Clone, Debug, PartialEq)]
pub struct RoutedWire {
    /// The ROUTE, by its index in [`LevelProblem::routes`]: the net and the style.
    pub route: usize,
    pub points: Vec<WirePoint>,
}

/// A solver's result for one level.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Solution {
    pub placements: Vec<Placement>,
    pub branches: Vec<Branch>,
    pub wires: Vec<RoutedWire>,
}

/// A record written from a solution.
#[derive(Clone, Debug, PartialEq)]
pub struct Written {
    /// The whole record, from `@RNL` to the closing brace, lines joined by `\n` without the
    /// comment marker of the host: what [`crate::writeback::replace_generated`] takes.
    pub record: String,
    /// What was left out, and why: placements and wires whose objects cannot be referred to.
    pub skipped: Vec<String>,
}

/// A name for RNL made of any text: ASCII letters, digits and `_`, without runs of `_`.
fn slug(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        let c = if c.is_ascii_alphanumeric() { c } else { '_' };
        if c == '_' && (out.is_empty() || out.ends_with('_')) {
            continue;
        }
        out.push(c);
    }
    out.trim_end_matches('_').to_string()
}

/// The names that the usages made by rules take when they are written out (10.5), by usage
/// index: the definition and what fills the first role, such as `gnd_stub_M1_s` for
/// `$GND_STUB @ATTACH #M1:s`, made unique within the level. The same problem gives the same
/// names, so the names stay as they were from one solve to the next while the netlist stays.
/// None for the other usages, and for those that cannot be written: a role that cannot be
/// referred to, or a definition without a `$` name.
pub fn generated_names(p: &LevelProblem) -> Vec<Option<String>> {
    let mut taken: Vec<String> = p.names.clone();
    let mut out = Vec::new();
    for u in &p.usages {
        let writable = u.rule_made && u.definition.starts_with('$') && u.roles.iter().all(|r| r.written.is_some());
        if !writable {
            out.push(None);
            continue;
        }
        let def = slug(&u.definition[1..].to_ascii_lowercase());
        let first = u.roles.first().and_then(|r| r.written.as_deref()).map(slug).unwrap_or_default();
        let base = if first.is_empty() { def } else { format!("{def}_{first}") };
        let mut name = base.clone();
        let mut n = 2;
        while taken.contains(&name) {
            name = format!("{base}_{n}");
            n += 1;
        }
        taken.push(name.clone());
        out.push(Some(name));
    }
    out
}

/// How the level's text writes a reference, given the names of the usages written out.
fn written(r: &Reference, names: &[Option<String>]) -> Option<String> {
    match r {
        Reference::Name(n) => Some(n.clone()),
        Reference::Generated { usage, path } => {
            let base = names.get(*usage)?.as_ref()?;
            Some(format!("%{}{}", written_name(base), path.concat()))
        }
        // not written yet: only the record of the solver of a file of the draft 0.3 may (5.9)
        Reference::Number { .. } | Reference::GeneratedNumber { .. } | Reference::Anonymous => None,
    }
}

/// The usage made by a rule that a reference goes through, if any.
fn root_of(r: &Reference) -> Option<usize> {
    match r {
        Reference::Generated { usage, .. } => Some(*usage),
        _ => None,
    }
}

/// How a routed wire writes one of its points, and the usage written out it goes through.
fn wire_point(
    p: &LevelProblem,
    names: &[Option<String>],
    q: &WirePoint,
    branch: &[String],
) -> Option<(String, Option<usize>)> {
    match q {
        WirePoint::Anchor(PointRef::Pin { object, terminal, .. }) => {
            let r = &p.objects.get(*object)?.reference;
            Some((format!("{}:{}", written(r, names)?, written_name(terminal)), root_of(r)))
        }
        WirePoint::Anchor(PointRef::Origin(k)) => {
            let o = p.objects.get(*k)?;
            if o.kind != ObjectKind::Point {
                return None;
            }
            Some((written(&o.reference, names)?, root_of(&o.reference)))
        }
        WirePoint::Anchor(PointRef::Centre(_)) => None,
        WirePoint::Branch(i) => Some((format!("%{}", branch.get(*i)?), None)),
        WirePoint::Bend((x, y)) => Some((format!("({}, {})", fmt_num(*x), fmt_num(*y)), None)),
    }
}

/// The statement that writes out a usage made by a rule (10.5): its definition, its roles in
/// order, its parameters and its own priority, so that what it expands to keeps the priority it
/// had in place of the record's `"high"` (14.8).
fn usage_statement(u: &UsageEntry, name: &str) -> String {
    let mut params: Vec<String> = u.params.iter().map(|(k, v)| format!("{}={}", k, written_lit(v))).collect();
    params.push(format!("priority=\"{}\"", u.priority.unwrap_or(Priority::Normal).as_str()));
    let roles: Vec<&str> = u.roles.iter().filter_map(|r| r.written.as_deref()).collect();
    format!(
        "%{} = ${} @ATTACH({}) {}",
        written_name(name),
        written_name(&u.definition[1..]),
        params.join(", "),
        roles.join(" ")
    )
}

/// The solver's record for a level (14.8, 17): the usages made by rules that the result reaches,
/// written out; the branch points of routed wires; the positions and orientations; the wires;
/// the junctions. `author` and `hash` go in the record head, the hash being what [`sig1`] gives
/// for the problem and the solver's options.
pub fn write_record(p: &LevelProblem, solution: &Solution, author: &str, hash: &str) -> Written {
    let names = generated_names(p);
    let mut skipped = Vec::new();
    let mut needed = vec![false; p.usages.len()];
    let mut placements: Vec<(usize, String)> = Vec::new();
    let mut sorted: Vec<&Placement> = solution.placements.iter().collect();
    sorted.sort_by_key(|pl| pl.object);
    for pl in sorted {
        let Some(o) = p.objects.get(pl.object) else {
            skipped.push(format!("object {}: not in the problem", pl.object));
            continue;
        };
        let Some(r) = written(&o.reference, &names) else {
            let why = match o.reference {
                Reference::Generated { .. } => "the usage made by a rule that it belongs to cannot be written out",
                _ => "it cannot be referred to (5.7)",
            };
            skipped.push(format!("{}: {why}", o.label));
            continue;
        };
        let at = match (&o.position, pl.at) {
            (Position::At(_), _) | (_, None) => None,
            (_, Some(xy)) => Some(xy),
        };
        let orient = if o.orient.is_some() { pl.orient } else { None };
        let mut parts = Vec::new();
        if let Some((x, y)) = at {
            parts.push(format!("@PLACE({}, {})", fmt_num(x), fmt_num(y)));
        }
        if let Some(code) = orient {
            parts.push(format!("@ORIENT(\"{}\")", code.as_str()));
        }
        if parts.is_empty() {
            continue;
        }
        if let Some(u) = root_of(&o.reference) {
            needed[u] = true;
        }
        placements.push((pl.object, format!("{r} {}", parts.join(" => "))));
    }

    let mut taken: Vec<String> = p.names.iter().chain(names.iter().flatten()).cloned().collect();
    let mut branch_names = Vec::new();
    for _ in &solution.branches {
        let mut n = branch_names.len() + 1;
        while taken.contains(&format!("rt{n}")) {
            n += 1;
        }
        taken.push(format!("rt{n}"));
        branch_names.push(format!("rt{n}"));
    }

    let mut wires = Vec::new();
    'wire: for w in &solution.wires {
        let Some(route) = p.routes.get(w.route) else {
            skipped.push(format!("route {}: not in the problem", w.route));
            continue;
        };
        let mut pts = Vec::new();
        let mut roots = Vec::new();
        for q in &w.points {
            match wire_point(p, &names, q, &branch_names) {
                Some((text, root)) => {
                    pts.push(text);
                    roots.extend(root);
                }
                None => {
                    skipped.push(format!(
                        "a wire of the ROUTE of &{}: a point of it cannot be referred to (5.7)",
                        route.net
                    ));
                    continue 'wire;
                }
            }
        }
        for u in roots {
            needed[u] = true;
        }
        let mut params = vec!["mode=\"straight\"".to_string()];
        params.extend(route.style.iter().map(|(k, v)| format!("{}={}", k, written_lit(v))));
        wires.push(format!("&{} @WIRE({}) {{{}}}", written_name(&route.net), params.join(", "), pts.join(" ")));
    }

    let mut statements = Vec::new();
    for (k, u) in p.usages.iter().enumerate() {
        if needed[k] {
            if let Some(name) = &names[k] {
                statements.push(usage_statement(u, name));
            }
        }
    }
    for (b, name) in solution.branches.iter().zip(&branch_names) {
        statements.push(format!("%{name} = ({}, {})", fmt_num(b.at.0), fmt_num(b.at.1)));
    }
    statements.extend(placements.into_iter().map(|(_, s)| s));
    statements.extend(wires);
    for (b, name) in solution.branches.iter().zip(&branch_names) {
        if b.junction {
            statements.push(format!("$JUNCTION @ATTACH %{name}"));
        }
    }

    let head = format!(
        "@RNL(type=\"generated\", priority=\"high\", author={}, hash={})",
        written_string(author),
        written_string(hash)
    );
    let record = if statements.is_empty() {
        format!("{head} {{}}")
    } else {
        format!("{head} {{\n  {}\n}}", statements.join(";\n  "))
    };
    Written { record, skipped }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs() {
        assert_eq!(slug("#M1:s"), "M1_s");
        assert_eq!(slug("&\"D<3>\""), "D_3");
        assert_eq!(slug("gnd_stub"), "gnd_stub");
        assert_eq!(slug("{#M1 #M2}"), "M1_M2");
    }
}
