//! Scope-local usage inspection, without constructing the solver's geometric problem.
use std::collections::{BTreeSet, HashMap, HashSet};

use rnl_runtime::model::{quoted, Attach, GroupInfo, LevelView, ObjectInfo};
use rnl_runtime::Interpretation;
use serde::Serialize;

use super::{Index, Level, Span};

#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct Usage {
    /// Shared frame and label anchors in SVG pixels; absent when no member is drawn.
    pub frame: Option<crate::UsageFrame>,
    pub object: usize,
    /// "usage", or "group" for a set marked with GROUP (11.5), whose one role, `members`, holds
    /// its members.
    pub kind: &'static str,
    pub id: String,
    /// Its definition; empty for a group.
    pub definition: String,
    /// Whether the usage itself is a virtual part with its own symbol.
    pub has_symbol: bool,
    /// Also includes virtual symbols/wires/annotations created inside the usage.
    pub has_drawing: bool,
    pub file: Option<String>,
    pub first_line: u32,
    pub source: String,
    pub roles: Vec<UsageRole>,
    /// "in-effect"; "overridden", a fact its statement writes lost a conflict to one of higher
    /// priority (8.5); "disabled", its priority, or that of the usage it is made in, is
    /// "disabled" (14.8). A group is only "in-effect" or "disabled" (11.5). As
    /// `MemberOf::state`; an application that failed makes no usage and has no row.
    pub state: &'static str,
    /// Why it was overridden: the message of the diagnostic.
    pub reason: Option<String>,
}

#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct UsageRole {
    pub name: String,
    /// References as written by the runtime, including unavailable members.
    pub members: Vec<String>,
    pub objects: Vec<usize>,
    /// SVG groups to keep at full opacity while focusing this role.
    pub groups: Vec<String>,
    /// Visible extent in SVG pixels, y down; absent when no member can be displayed.
    pub bounds: Option<[f64; 4]>,
}

#[derive(Default, Clone)]
struct Geometry {
    objects: BTreeSet<usize>,
    groups: BTreeSet<String>,
    bounds: Option<[f64; 4]>,
}
impl Geometry {
    fn add(&mut self, other: &Self) {
        self.objects.extend(other.objects.iter().copied());
        self.groups.extend(other.groups.iter().cloned());
        if let Some(b) = other.bounds {
            self.bounds = Some(match self.bounds {
                Some(a) => [a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])],
                None => b,
            });
        }
    }
}

pub(super) fn inspect_all(r: &Interpretation, levels: &mut [Level], ix: &mut Index<'_>) {
    let source_lines: HashMap<_, Vec<_>> = r.sources.iter().map(|(p, s)| (p.as_str(), s.lines().collect())).collect();
    for (lv, level) in r.model.levels.iter().zip(levels) {
        level.usages = inspect(r, lv, level, ix, &source_lines);
    }
}
/// A group (11.5) as a usage of kind "group", with one role, `members`.
fn group_object(g: &GroupInfo) -> ObjectInfo {
    ObjectInfo {
        usage_has_drawing: false,
        source: g.source.clone(),
        id: g.id.clone(),
        kind: "group",
        host: Vec::new(),
        definition: None,
        attach: vec![("members".into(), Attach::Set(g.members.clone()))],
        params: Vec::new(),
        own_style: Default::default(),
        symbol: None,
        at: None,
        orient: Default::default(),
        pins: Vec::new(),
        symbol_view: None,
        origin: g.origin.clone(),
        parent: g.parent.clone(),
        disabled: g.disabled,
    }
}

/// The usages of a level, then its groups as usages of kind "group".
pub(super) fn listed(lv: &LevelView) -> Vec<ObjectInfo> {
    let usages = lv.objects.iter().filter(|o| o.kind == "usage").cloned();
    usages.chain(lv.groups.iter().map(group_object)).collect()
}

fn inspect(
    r: &Interpretation,
    lv: &LevelView,
    level: &Level,
    ix: &mut Index<'_>,
    source_lines: &HashMap<&str, Vec<&str>>,
) -> Vec<Usage> {
    let listed = listed(lv);
    if listed.is_empty() {
        return Vec::new();
    }
    let instances: HashMap<_, _> = ix.doc.levels[lv.level]
        .instances
        .iter()
        .enumerate()
        .filter_map(|(i, inst)| ix.insts.get(&(lv.level, i)).map(|&k| (format!("#{}", quoted(&inst.name)), k)))
        .collect();
    let mut references = HashMap::new();
    for o in lv.objects.iter().filter(|o| o.kind != "usage") {
        references.insert(o.id.clone(), o.host.iter().filter_map(|id| instances.get(id).copied()).collect());
    }
    for o in &listed {
        references.insert(o.id.clone(), vec![ix.usage_of_kind(lv.level, &o.id, o.kind)]);
    }
    // Nets may not have appeared on a drawing yet, but must still be inspectable as roles.
    for o in &listed {
        for (_, a) in &o.attach {
            add_nets(a, lv.level, ix, &mut references);
        }
    }
    let mut by_document: HashMap<usize, Geometry> = HashMap::new();
    for h in &level.hits {
        let mut g = Geometry::default();
        for p in &h.points {
            g.add(&Geometry { bounds: Some([p[0], p[1], p[0], p[1]]), ..Default::default() });
        }
        // Group ids are derived from object membership by the frontend. Do not repeat a
        // large net's complete group list for every ground/port usage attached to that net.
        for &o in &h.objects {
            by_document.entry(o).or_default().add(&g);
        }
    }
    let mut direct = HashMap::new();
    for (id, objects) in &references {
        let mut g = Geometry { objects: objects.iter().copied().collect(), ..Default::default() };
        for o in objects {
            if let Some(b) = by_document.get(o) {
                g.add(b);
            }
        }
        direct.insert(id.clone(), g);
    }
    // Terminal roles enclose the pin, while retaining the owning part's ink during focus.
    for o in &lv.objects {
        for pin in &o.pins {
            for terminal in &pin.terminals {
                let id = if o.kind == "usage" { format!("{}:{}", o.id, quoted(terminal)) } else { terminal.clone() };
                let mut g = direct.get(&o.id).cloned().unwrap_or_default();
                g.bounds = Some([
                    pin.at.0 * level.scale,
                    -pin.at.1 * level.scale,
                    pin.at.0 * level.scale,
                    -pin.at.1 * level.scale,
                ]);
                direct.insert(id, g);
            }
        }
    }
    for p in &lv.points {
        if let Some(at) = p.at {
            direct.insert(p.id.clone(), point(at, level.scale));
        }
    }
    for (k, w) in lv.wires.iter().enumerate() {
        let mut g = Geometry::default();
        for p in &w.points {
            g.add(&point(*p, level.scale));
        }
        g.groups.insert(format!("wire-{k}"));
        direct.entry(w.object_id.clone()).or_default().add(&g);
    }
    let by_id: HashMap<_, _> = lv.objects.iter().map(|o| (o.id.as_str(), o)).collect();
    let mut memo = HashMap::new();
    listed
        .iter()
        .map(|o| {
            let object = references[&o.id][0];
            let mut source = String::new();
            let mut file = None;
            let mut first_line = 0;
            if let Some((a, b)) = &o.source {
                file = Some(a.file.clone());
                first_line = a.line;
                if let Some(lines) = source_lines.get(a.file.as_str()) {
                    let mut parts: Vec<_> = (a.line..=b.line)
                        .filter_map(|n| {
                            let line = lines.get(n.saturating_sub(1) as usize)?;
                            let from = if n == a.line { a.col.saturating_sub(1) as usize } else { 0 };
                            let to = if n == b.line { b.col as usize } else { line.chars().count() };
                            Some((n, line.chars().skip(from).take(to.saturating_sub(from)).collect::<String>()))
                        })
                        .collect();
                    let blank = |s: &str| s.trim().trim_start_matches('*').trim_start_matches("//").trim().is_empty();
                    while parts.first().is_some_and(|(_, s)| blank(s)) {
                        parts.remove(0);
                    }
                    while parts.last().is_some_and(|(_, s)| blank(s)) {
                        parts.pop();
                    }
                    first_line = parts.first().map_or(a.line, |(n, _)| *n);
                    let last_line = parts.last().map_or(b.line, |(n, _)| *n);
                    source = parts.into_iter().map(|(_, s)| s).collect::<Vec<_>>().join("\n");
                    if let Some(&f) = ix.files.get(&a.file) {
                        ix.objects[object].lines = Some([f as u32, first_line, last_line]);
                        // Bind only the usage name, so clicking a role still selects its own object.
                        for n in first_line..=last_line {
                            if let Some(line) = lines.get(n.saturating_sub(1) as usize) {
                                let from = if n == a.line { a.col.saturating_sub(1) as usize } else { 0 };
                                let to = if n == b.line { b.col as usize } else { line.chars().count() };
                                let statement: String = line.chars().skip(from).take(to.saturating_sub(from)).collect();
                                let matched = statement.match_indices(&o.id).find(|(byte, _)| {
                                    statement[byte + o.id.len()..]
                                        .chars()
                                        .next()
                                        .is_none_or(|c| !c.is_alphanumeric() && c != '_' && c != ':')
                                });
                                if let Some((byte, _)) = matched {
                                    ix.spans.push(Span {
                                        object,
                                        file: f,
                                        line: n,
                                        col: (from + statement[..byte].chars().count()) as u32 + 1,
                                        len: o.id.chars().count() as u32,
                                        kind: "ref",
                                    });
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            let roles: Vec<UsageRole> = o
                .attach
                .iter()
                .map(|(name, a)| {
                    let mut members = Vec::new();
                    names(a, &mut members);
                    let g = geometry(a, level.scale, &by_id, &direct, &mut HashSet::new(), &mut memo);
                    UsageRole {
                        name: name.clone(),
                        members,
                        objects: g.objects.into_iter().collect(),
                        groups: g.groups.into_iter().collect(),
                        bounds: g.bounds,
                    }
                })
                .collect();
            let local = o.symbol.is_some()
                && roles.iter().all(|r| {
                    r.bounds.is_none_or(|b| {
                        (!r.objects.is_empty() && r.objects.iter().all(|i| ix.objects[*i].kind == "net"))
                            || (b[0] == b[2] && b[1] == b[3])
                    })
                });
            let (state, reason) = super::member_of::state(r, o);
            Usage {
                frame: crate::overlays::usage_frame(
                    object,
                    local,
                    &roles,
                    &level.hits,
                    &format!(
                        "{}: {}",
                        if o.kind == "group" { "@GROUP" } else { o.definition.as_deref().unwrap_or_default() },
                        o.id
                    ),
                ),
                object,
                kind: o.kind,
                id: o.id.clone(),
                definition: o.definition.clone().unwrap_or_default(),
                has_symbol: o.symbol.is_some(),
                has_drawing: o.usage_has_drawing,
                file,
                first_line,
                source,
                roles,
                state,
                reason,
            }
        })
        .collect()
}

fn add_nets(a: &Attach, level: usize, ix: &mut Index<'_>, refs: &mut HashMap<String, Vec<usize>>) {
    match a {
        Attach::Object(id) if id.starts_with('&') => {
            let written = &id[1..];
            let name =
                written.strip_prefix('"').and_then(|s| s.strip_suffix('"')).unwrap_or(written).replace("\"\"", "\"");
            refs.entry(id.clone()).or_insert_with(|| vec![ix.net(level, &name)]);
        }
        Attach::Set(members) => {
            for a in members {
                add_nets(a, level, ix, refs);
            }
        }
        _ => (),
    }
}
fn names(a: &Attach, out: &mut Vec<String>) {
    match a {
        Attach::Object(id) => out.push(id.clone()),
        Attach::At((x, y)) => out.push(format!("({x}, {y})")),
        Attach::Set(members) => {
            for a in members {
                names(a, out);
            }
        }
    }
}
fn point((x, y): (f64, f64), scale: f64) -> Geometry {
    Geometry { bounds: Some([x * scale, -y * scale, x * scale, -y * scale]), ..Default::default() }
}
fn geometry(
    a: &Attach,
    scale: f64,
    objects: &HashMap<&str, &ObjectInfo>,
    direct: &HashMap<String, Geometry>,
    seen: &mut HashSet<String>,
    memo: &mut HashMap<String, Geometry>,
) -> Geometry {
    match a {
        Attach::At(p) => point(*p, scale),
        Attach::Set(members) => {
            let mut g = Geometry::default();
            for a in members {
                g.add(&geometry(a, scale, objects, direct, seen, memo));
            }
            g
        }
        Attach::Object(id) => {
            if let Some(g) = memo.get(id) {
                return g.clone();
            }
            let mut g = direct.get(id).cloned().unwrap_or_default();
            if g.bounds.is_none() && seen.insert(id.clone()) {
                if let Some(o) = objects.get(id.as_str()).filter(|o| o.kind == "usage" && o.symbol.is_none()) {
                    for (_, a) in &o.attach {
                        g.add(&geometry(a, scale, objects, direct, seen, memo));
                    }
                }
                seen.remove(id);
            }
            memo.insert(id.clone(), g.clone());
            g
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{BuildOptions, MemoryProvider};

    #[test]
    fn usages_keep_their_scope_source_identity_and_visible_roles() {
        let p =
            MemoryProvider::new().with("usages.sp", include_str!("../../../../tests/viewer/inputs/usage-inspector.sp"));
        let d = crate::build("usages.sp", &p, &BuildOptions::default()).unwrap();
        assert!(!d.diagnostics.iter().any(|d| d.severity == "error"), "{:?}", d.diagnostics);
        let lv = &d.levels[0];
        let pair = lv.usages.iter().find(|u| u.id == "%pair1").unwrap();
        assert!(!pair.has_symbol);
        assert!(!pair.has_drawing);
        assert!(pair.source.contains("%pair1 = #R1(left) #R2(right) @AS $PAIR"), "{}", pair.source);
        assert!(d.objects[pair.object].lines.is_some());
        assert!(d.spans.iter().any(|s| s.object == pair.object));
        assert!(pair.roles.iter().all(|r| r.bounds.is_some() && !r.objects.is_empty()));
        assert!(pair.roles[0].objects.iter().any(|&o| d.objects[o].name == "R1"));
        let unseen = lv.usages.iter().find(|u| u.id == "%unseen").unwrap();
        assert!(unseen.roles.iter().all(|r| r.bounds.is_none()));
        let ground = lv.usages.iter().find(|u| u.id == "%ground").unwrap();
        assert!(ground.has_symbol);
        assert!(ground.has_drawing);
        assert!(lv.hits.iter().any(|h| h.kind == "box" && h.objects[0] == ground.object));
        let nested = lv.usages.iter().find(|u| u.id == "%nested").unwrap();
        assert_eq!(
            nested.roles[0].bounds,
            pair.roles.iter().map(|r| r.bounds.unwrap()).reduce(|a, b| [
                a[0].min(b[0]),
                a[1].min(b[1]),
                a[2].max(b[2]),
                a[3].max(b[3])
            ])
        );
        assert!(lv.usages.iter().all(|u| u.id != "%other"));
        let other = d.levels.iter().find(|l| l.title == "other").unwrap();
        assert!(other.usages.iter().any(|u| u.id == "%other"));
        assert!(other.usages.iter().all(|u| u.id != "%pair1"));
        let pin = lv.usages.iter().find(|u| u.id == "%pin1").unwrap().roles[0].bounds.unwrap();
        assert_eq!(pin[0], pin[2]);
        assert_eq!(pin[1], pin[3]);
        assert!(lv.usages.iter().find(|u| u.id == "%set1").unwrap().roles[0].members.len() == 2);
    }

    #[test]
    fn drawing_wrappers_are_distinct_from_pure_semantic_usages() {
        let p = MemoryProvider::new().with(
            "wrappers.sp",
            "* wrappers\nR1 a 0 1k\n* @RNL { #R1 @PLACE(0, 0); %stub = $GROUND_STUB @ATTACH #R1:2; $ROUTER = @DEFINE(roles=[[\"signal\", \"net\"]]) { ?signal @ROUTE }; %r = &a @AS $ROUTER }\n.end\n",
        );
        let d = crate::build("wrappers.sp", &p, &BuildOptions::default()).unwrap();
        assert!(!d.diagnostics.iter().any(|d| d.severity == "error"), "{:?}", d.diagnostics);
        let stub = d.levels[0].usages.iter().find(|u| u.id == "%stub").unwrap();
        assert!(!stub.has_symbol);
        assert!(stub.has_drawing);
        let route = d.levels[0].usages.iter().find(|u| u.id == "%r").unwrap();
        assert!(!route.has_symbol);
        assert!(route.has_drawing);
    }

    #[test]
    fn usage_name_spans_stay_inside_their_own_statement() {
        let line = "* @RNL { $LEAF = @DEFINE(roles=[\"part\"]) {}; %u10 = #R1 @AS $LEAF; %u1 = #R2 @AS $LEAF }";
        let text = format!("* prefixes\nR1 a b 1k\nR2 b c 1k\n{line}\n.end\n");
        let p = MemoryProvider::new().with("names.sp", &text);
        let d = crate::build("names.sp", &p, &BuildOptions::default()).unwrap();
        let u = d.levels[0].usages.iter().find(|u| u.id == "%u1").unwrap();
        let span = d.spans.iter().find(|s| s.object == u.object).unwrap();
        assert_eq!(span.col, line.find("%u1 =").unwrap() as u32 + 1);
    }

    #[test]
    fn wire_fragments_and_quoted_virtual_pins_keep_all_visible_geometry() {
        use rnl_runtime::model::Attach;
        let p =
            MemoryProvider::new().with("usages.sp", include_str!("../../../../tests/viewer/inputs/usage-inspector.sp"));
        let mut r = crate::interpret("usages.sp", None, &p, &Default::default()).unwrap();
        let lv = &mut r.model.levels[0];
        let k = lv.wires.iter().position(|w| w.object_id == "%wire").unwrap();
        lv.wires[k].points = vec![(0., 0.), (1., 0.)];
        let mut next = lv.wires[k].clone();
        next.points = vec![(10., 0.), (11., 0.)];
        lv.wires.push(next);
        let ground = lv.objects.iter_mut().find(|o| o.id == "%ground").unwrap();
        ground.pins[0].terminals = vec!["pin-name".into()];
        let pin = ground.pins[0].at;
        lv.objects.iter_mut().find(|o| o.id == "%pin1").unwrap().attach =
            vec![("pin".into(), Attach::Object("%ground:\"pin-name\"".into()))];
        lv.objects.iter_mut().find(|o| o.id == "%signal").unwrap().attach =
            vec![("signal".into(), Attach::Object("%wire".into()))];
        let d = crate::document::document("usages.sp", &r, Vec::new(), &BuildOptions::default(), false);
        let lv = &d.levels[0];
        let wire = &lv.usages.iter().find(|u| u.id == "%signal").unwrap().roles[0];
        assert_eq!(wire.bounds, Some([0., 0., 11. * lv.scale, 0.]));
        assert_eq!(wire.groups.len(), 2);
        let got = lv.usages.iter().find(|u| u.id == "%pin1").unwrap().roles[0].bounds;
        assert_eq!(got, Some([pin.0 * lv.scale, -pin.1 * lv.scale, pin.0 * lv.scale, -pin.1 * lv.scale]));
        let files = std::collections::HashMap::new();
        let mut ix = crate::document::Index::new(&r.doc, &files);
        let mut refs = std::collections::HashMap::new();
        let id = format!("&{}", rnl_runtime::model::quoted("\"rail\""));
        super::add_nets(&Attach::Object(id.clone()), 0, &mut ix, &mut refs);
        assert_eq!(ix.objects[refs[&id][0]].name, "\"rail\"");
    }

    #[test]
    fn shared_nested_usages_resolve_once_and_keep_unique_members() {
        let mut text = String::from("* shared usage DAG\nR1 a b 1k\n* @RNL {\n* $LEAF = @DEFINE(roles=[\"part\"]) {};\n* $BRANCH = @DEFINE(roles=[\"left\", \"right\"]) {};\n* #R1 @PLACE(0, 0);\n* %u0 = #R1 @AS $LEAF;\n");
        for i in 1..=25 {
            text.push_str(&format!("* %u{i} = %u{}(left) %u{}(right) @AS $BRANCH;\n", i - 1, i - 1));
        }
        text.push_str("* }\n.end\n");
        let p = MemoryProvider::new().with("dag.sp", &text);
        let d = crate::build("dag.sp", &p, &BuildOptions::default()).unwrap();
        assert!(!d.diagnostics.iter().any(|d| d.severity == "error"));
        let u = d.levels[0].usages.iter().find(|u| u.id == "%u25").unwrap();
        assert!(u.roles.iter().all(|r| r.bounds.is_some() && r.objects.len() == 26));
    }
}
