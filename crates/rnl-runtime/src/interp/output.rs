//! From scenes to the effective model the renderer reads (crate::model).

use std::collections::{BTreeMap, HashMap};

use crate::geom::{Orient, Pt};
use crate::host::LevelKind;
use crate::markup;
use crate::model::*;

use super::obj::*;
use super::unit::Place;
use super::Interp;

fn flat(p: &Pos) -> Vec<f64> {
    p.iter()
        .map(|e| match e {
            PosElem::Int(k) => *k as f64,
            PosElem::Tag(t) => match *t {
                "b" => 1e9 + 1.0,
                "e" => 1e9 + 2.0,
                "g" => 1e9 + 3.0,
                "p" => 1e9 + 4.0,
                "pkg" => 1e9 + 5.0,
                "s" => 1e9 + 6.0,
                _ => 1e9,
            },
        })
        .collect()
}

/// Is a fact's position that of a package's statement? A package at the root is placed first (0);
/// one used in a level carries the tag `pkg` (`position_of`).
fn from_package(p: &Pos) -> bool {
    matches!(p.first(), Some(PosElem::Int(0))) || p.contains(&PosElem::Tag("pkg"))
}

/// The unit vector of an exit direction in degrees, for the four directions pins have.
fn dir_vector(d: u16) -> Option<(f64, f64)> {
    match d {
        0 => Some((1.0, 0.0)),
        90 => Some((0.0, 1.0)),
        180 => Some((-1.0, 0.0)),
        270 => Some((0.0, -1.0)),
        _ => None,
    }
}

/// An exit direction turned with an object's orientation.
fn turned(d: Option<u16>, o: Orient) -> Option<u16> {
    let (x, y) = o.apply(dir_vector(d?)?);
    match (x.round() as i32, y.round() as i32) {
        (1, 0) => Some(0),
        (0, 1) => Some(90),
        (-1, 0) => Some(180),
        (0, -1) => Some(270),
        _ => None,
    }
}

impl Interp<'_> {
    pub(crate) fn build_model(&mut self) -> Model {
        let mut levels = Vec::new();
        for s in 0..self.scenes.len() {
            levels.push(self.level_view(s));
        }
        let language = self.units[self.level_unit[0]].version.clone();
        let mut packages: Vec<(String, String)> = Vec::new();
        for s in 0..self.scenes.len() {
            let lv = self.scenes[s].level;
            let eff = self.effective_packages(lv);
            for e in &eff.entries {
                let u = &self.units[e.unit];
                let p = (u.pkg_name.clone(), u.pkg_version.clone());
                if !packages.contains(&p) {
                    packages.push(p);
                }
            }
        }
        // this version of the language has no conflict that makes the whole interpretation fail (7.5)
        Model { levels, failed: None, language, packages }
    }

    // ---- ids for the editor interface (rnl-dev notes/rnl-editor-interface, 3.2)

    /// Where the statement that made an object is, `@file:line:col`, for the objects that keep it.
    fn site_id(&self, o: ObjId) -> Option<String> {
        let (site, off) = match self.o(o) {
            ObjData::Usage(u) => (&u.site, u.off),
            ObjData::Wire { site, off, .. }
            | ObjData::Text { site, off, .. }
            | ObjData::KeyText { site, off, .. }
            | ObjData::Prim { site, off, .. } => (site, *off),
            ObjData::Bundle { site, .. } => (site, 0),
            ObjData::Constraint(c) => (&c.site, c.off),
            _ => return None,
        };
        let loc = self.rec_loc(site.as_ref()?.rec, off)?;
        Some(format!("@{}:{}:{}", loc.file, loc.line, loc.col))
    }

    /// Is the object's `%` name one the level's records bind to it? Not so for a name bound in the
    /// body of a definition, which each expansion binds anew.
    fn level_named(&self, s: SceneId, o: ObjId) -> bool {
        let name = &self.objs[o.0 as usize].name;
        if name.is_empty() {
            return false;
        }
        let u = self.level_unit[self.scenes[s].level];
        let Some(b) = self.units[u].names.get(&('%', name.clone())) else { return false };
        let key = b.key();
        [Some(s), None].into_iter().any(|sc| {
            let k = (key.clone(), sc);
            matches!(self.memo.get(&k), Some(super::Memo::Val(v)) if *v == o)
                || self
                    .bound_memo
                    .get(&k)
                    .is_some_and(|bound| bound.iter().any(|((p, n), v)| *p == '%' && n == name && *v == o))
        })
    }

    /// An object's id before the level's other objects are known: its name in the level, or
    /// where it was made.
    fn base_id(&self, s: SceneId, o: ObjId) -> String {
        match self.o(o) {
            ObjData::Inst { .. } => format!("#{}", quoted(&self.inst_name(o))),
            ObjData::Net { display, .. } => format!("&{}", quoted(display)),
            ObjData::LevelObj { .. } => "#".into(),
            _ if self.level_named(s, o) => format!("%{}", quoted(&self.objs[o.0 as usize].name)),
            _ => self.site_id(o).unwrap_or_else(|| format!("@{}", o.0)),
        }
    }

    /// A symbol's name as a reference writes it: `$NMOS`, or `%amp` for one named in a level.
    fn symbol_name(&self, sym: ObjId) -> String {
        let name = &self.objs[sym.0 as usize].name;
        if name.is_empty() || name.starts_with('$') {
            self.label(sym)
        } else {
            format!("%{}", quoted(name))
        }
    }

    /// Ids unique among `objs`. A name that several have, as each expansion of a definition makes
    /// its own, is not the level's: those go by where they were made. Several made by one
    /// statement are numbered after it, `#1`, `#2`, in order.
    fn unique_ids(&self, s: SceneId, objs: &[ObjId], taken: &HashMap<ObjId, String>) -> HashMap<ObjId, String> {
        let mut ids: Vec<String> = objs.iter().map(|o| self.base_id(s, *o)).collect();
        let count = |ids: &[String]| {
            let mut n: HashMap<String, usize> = taken.values().map(|v| (v.clone(), 1)).collect();
            for i in ids {
                *n.entry(i.clone()).or_default() += 1;
            }
            n
        };
        let n = count(&ids);
        for (k, o) in objs.iter().enumerate() {
            if ids[k].starts_with('%') && n[&ids[k]] > 1 {
                ids[k] = self.site_id(*o).unwrap_or_else(|| format!("@{}", o.0));
            }
        }
        let n = count(&ids);
        let mut seen: HashMap<String, usize> = HashMap::new();
        for id in ids.iter_mut() {
            if n[id.as_str()] > 1 {
                let k = seen.entry(id.clone()).or_default();
                *k += 1;
                *id = format!("{id}#{k}");
            }
        }
        objs.iter().copied().zip(ids).collect()
    }

    /// The id of an object in a level whose objects have `ids`: theirs, or for a net, a terminal
    /// or a point, as a reference writes it.
    fn ref_id(&self, s: SceneId, o: ObjId, ids: &HashMap<ObjId, String>) -> String {
        if let Some(id) = ids.get(&o) {
            return id.clone();
        }
        match self.o(o) {
            // an instance's terminals by number, as the netlist orders its nodes; a port by name
            ObjData::Term { owner, index, name } => {
                let base = self.ref_id(s, *owner, ids);
                match name {
                    Some(n) if !matches!(self.o(*owner), ObjData::Inst { .. } | ObjData::Bundle { .. }) => {
                        format!("{base}:{}", quoted(n))
                    }
                    _ => format!("{base}:{index}"),
                }
            }
            ObjData::KeyText { attach, key, .. } => format!("{}::{key}", self.ref_id(s, *attach, ids)),
            _ => self.base_id(s, o),
        }
    }

    /// Where a symbol is defined: the unit whose binding made it.
    fn symbol_origin(&self, sym: ObjId) -> SymbolOrigin {
        self.symbol_origin_version(sym).0
    }

    fn symbol_origin_version(&self, sym: ObjId) -> (SymbolOrigin, Option<String>) {
        if self.symbol(sym).default {
            return (SymbolOrigin::None, None);
        }
        match self.symbol(sym).unit.map(|u| &self.units[u]) {
            Some(u) if u.kind == super::unit::UnitKind::Package => {
                (SymbolOrigin::Package(u.pkg_name.clone()), Some(u.pkg_version.clone()))
            }
            Some(u) => (u.level.map(SymbolOrigin::Level).unwrap_or_default(), None),
            None => (SymbolOrigin::None, None),
        }
    }

    /// A vertex of a wire as the statement gives it.
    fn vertex(&mut self, s: SceneId, v: ObjId, ids: &HashMap<ObjId, String>) -> Option<Vertex> {
        match self.o(v).clone() {
            ObjData::Term { owner, index, name } if matches!(self.o(owner), ObjData::Usage(_)) => Some(Vertex::Usage {
                usage: self.ref_id(s, owner, ids),
                role: name.unwrap_or_else(|| index.to_string()),
            }),
            ObjData::Term { .. } => Some(Vertex::Terminal(self.ref_id(s, v, ids))),
            ObjData::Point { .. } if ids.contains_key(&v) => Some(Vertex::Point(ids[&v].clone())),
            _ => self.point_of(s, v).map(Vertex::At),
        }
    }

    /// What a role of a usage is attached to: for a set, its members, as a set role holds them (9.4).
    fn attached(&mut self, s: SceneId, o: ObjId, ids: &HashMap<ObjId, String>) -> Attach {
        if self.is_set(o) {
            let members = self.flatten(s, o, 7, None);
            return Attach::Set(members.into_iter().map(|m| self.attached(s, m, ids)).collect());
        }
        match self.o(o) {
            ObjData::Point { .. } if !ids.contains_key(&o) => match self.point_of(s, o) {
                Some(p) => Attach::At(p),
                None => Attach::Object(self.label(o)),
            },
            _ => Attach::Object(self.ref_id(s, o, ids)),
        }
    }

    /// The instances, bundles and usages of a scene, drawn or not, and its named points.
    fn objects_of(
        &mut self,
        s: SceneId,
        objs: &[ObjId],
        points: &[ObjId],
        ids: &HashMap<ObjId, String>,
    ) -> (Vec<ObjectInfo>, Vec<PointInfo>) {
        let mut out = Vec::new();
        for &o in objs {
            let (kind, host) = match self.o(o).clone() {
                ObjData::Inst { .. } => ("instance", vec![ids[&o].clone()]),
                ObjData::Bundle { members, .. } => {
                    ("bundle", members.iter().map(|m| self.ref_id(s, *m, ids)).collect())
                }
                _ => ("usage", Vec::new()),
            };
            let (definition, attach) = match self.o(o).clone() {
                ObjData::Usage(u) => {
                    let roles = u.roles.iter().map(|(name, r)| (name.clone(), self.attached(s, *r, ids))).collect();
                    (Some(self.label(u.definition)), roles)
                }
                _ => (None, Vec::new()),
            };
            let sym = self.scenes[s].symbol.get(&o).copied();
            let symbol = sym.map(|sym| SymbolInfo {
                name: self.symbol_name(sym),
                origin: self.symbol_origin(sym),
                default: self.symbol(sym).default,
            });
            let at = self.xy(s, o);
            let orient = self.orientation(s, o);
            let mut pins = Vec::new();
            if let (Some(sym), Some(at)) = (sym, at) {
                let sd = self.symbol(sym).clone();
                pins = sd
                    .pins
                    .iter()
                    .map(|p| {
                        let d = orient.apply(p.pos);
                        PinPlace {
                            terminals: Vec::new(),
                            at: (at.0 + d.0, at.1 + d.1),
                            dir: turned(p.dir, orient),
                            nets: Vec::new(),
                        }
                    })
                    .collect();
                for t in self.terms_of(s, o) {
                    let Some(k) = self.pin_for(s, o, t) else { continue };
                    let ObjData::Term { owner, index, name } = self.o(t).clone() else { continue };
                    let term = match self.o(owner) {
                        ObjData::Usage(_) => name.unwrap_or_else(|| index.to_string()),
                        _ => format!("{}:{index}", self.ref_id(s, owner, ids)),
                    };
                    if !pins[k].terminals.contains(&term) {
                        pins[k].terminals.push(term);
                    }
                    for n in self.term_nets(s, t) {
                        let net = self.ref_id(s, n, ids);
                        if !pins[k].nets.contains(&net) {
                            pins[k].nets.push(net);
                        }
                    }
                }
            }
            out.push(ObjectInfo {
                id: ids[&o].clone(),
                kind,
                host,
                definition,
                attach,
                symbol,
                at,
                orient,
                pins,
                symbol_view: None,
            });
        }
        let junction_at: Vec<ObjId> = self.scenes[s]
            .usages
            .iter()
            .filter(|a| self.label(self.usage(**a).definition) == "$JUNCTION")
            .flat_map(|a| self.usage(*a).roles.iter().map(|(_, r)| *r).collect::<Vec<_>>())
            .collect();
        let points = points
            .iter()
            .map(|p| PointInfo { id: ids[p].clone(), at: self.xy(s, *p), junction: junction_at.contains(p) })
            .collect();
        (out, points)
    }

    fn style_of(&self, s: SceneId, o: ObjId) -> Style {
        self.scenes[s]
            .style
            .get(&o)
            .map(|m| m.iter().map(|(k, (v, _))| (k.clone(), v.clone())).collect())
            .unwrap_or_default()
    }

    /// The style properties the document's own records give an object: those that won from a
    /// package, such as the standard package's defaults, left out. A package at the root is
    /// placed first (0); one used in a level carries the tag `pkg` (`position_of`).
    fn own_style_of(&self, s: SceneId, o: ObjId) -> Style {
        self.scenes[s]
            .style
            .get(&o)
            .map(|m| {
                m.iter().filter(|(_, (_, p))| !from_package(p)).map(|(k, (v, _))| (k.clone(), v.clone())).collect()
            })
            .unwrap_or_default()
    }

    fn layer_of(&self, s: SceneId, o: ObjId) -> Layer {
        self.scenes[s].layer.get(&o).map(|(l, _)| Layer::parse(l)).unwrap_or_default()
    }

    /// Where the latest symbol of each object of a scene is given, in the order of writing (5.5).
    fn last_symbol_pos(&self, s: SceneId) -> HashMap<ObjId, Pos> {
        let mut out: HashMap<ObjId, Pos> = HashMap::new();
        for f in &self.scenes[s].appear {
            let order = f.order();
            if out.get(&f.target).is_none_or(|p| order > *p) {
                out.insert(f.target, order);
            }
        }
        out
    }

    /// The key that orders an object among those drawn; `last_symbol` is what `last_symbol_pos` gives.
    fn key_of(&self, s: SceneId, o: ObjId, last_symbol: &HashMap<ObjId, Pos>) -> Vec<f64> {
        let sc = &self.scenes[s];
        match self.o(o) {
            ObjData::Inst { .. } => vec![0.0, self.inst_of(o).order as f64],
            ObjData::Bundle { members, .. } => vec![0.0, self.inst_of(members[0]).order as f64],
            ObjData::Usage(_) => {
                let p = last_symbol
                    .get(&o)
                    .cloned()
                    .or_else(|| sc.created_order.get(&o).cloned())
                    .unwrap_or_else(|| pos(&[9]));
                let mut k = vec![1.0];
                k.extend(flat(&p));
                k
            }
            _ => {
                let mut k = vec![1.0];
                k.extend(flat(&sc.created_order.get(&o).cloned().unwrap_or_else(|| pos(&[9]))));
                k
            }
        }
    }

    pub(crate) fn symbol_view(
        &mut self,
        sym: ObjId,
        table: &mut HashMap<ObjId, usize>,
        out: &mut Vec<SymbolView>,
    ) -> usize {
        if let Some(&k) = table.get(&sym) {
            return k;
        }
        let k = out.len();
        table.insert(sym, k);
        let sd = self.symbol(sym).clone();
        let bbox = match sd.bbox {
            Some(b) => b,
            None => self.symbol_extent(sym, 0),
        };
        let (origin, package_version) = self.symbol_origin_version(sym);
        out.push(SymbolView {
            id: sym.0,
            source: sd.source.clone(),
            labels: sd
                .labels
                .iter()
                .map(|l| SymbolLabel {
                    key: l.key.clone(),
                    text: l.items.text.as_ref().map(|t| t.as_str().to_string()).unwrap_or_default(),
                    at: l.position,
                    halign: l.items.halign.clone(),
                    valign: l.items.valign.clone(),
                    rotate: l.items.rotate.unwrap_or(0.0),
                    style: l.style.clone(),
                })
                .collect(),
            name: self.label(sym),
            prims: Vec::new(),
            pins: Vec::new(),
            nested: Vec::new(),
            bbox,
            box_written: sd.bbox.is_some(),
            default: sd.default,
            origin,
            package_version,
        });

        let prims = sd
            .prims
            .iter()
            .map(|p| SymbolPrim {
                text: p.text.as_ref().and_then(|t| t.text.as_ref().map(|s| s.as_str().to_string())),
                shape: p.shape,
                points: p.points.clone(),
                params: p.params.clone(),
                style: p.style.clone(),
                halign: p.text.as_ref().and_then(|t| t.halign.clone()),
                valign: p.text.as_ref().and_then(|t| t.valign.clone()),
                rotate: p.text.as_ref().and_then(|t| t.rotate),
            })
            .collect();
        let pins = sd
            .pins
            .iter()
            .map(|p| PinView {
                pos: p.pos,
                label: p.label.clone(),
                dir: p.dir,
                terminals: p
                    .terms
                    .iter()
                    .map(|t| match t {
                        TermKey::Num(n) => n.to_string(),
                        TermKey::Name(s) => s.clone(),
                    })
                    .collect(),
            })
            .collect();
        let mut nested = Vec::new();
        for a in &sd.usages {
            let u = self.usage(*a).clone();
            let Some(inner) = u.sym_symbol else { continue };
            if !matches!(self.o(inner), ObjData::Symbol(_)) {
                continue;
            }
            let Some((_, pt)) = u.roles.first() else { continue };
            let ObjData::Point { x: Some(x), y: Some(y), .. } = self.o(*pt).clone() else { continue };
            let ik = self.symbol_view(inner, table, out);
            nested.push(NestedView { symbol: ik, at: (x, y), orient: u.sym_orient });
        }
        out[k].prims = prims;
        out[k].pins = pins;
        out[k].nested = nested;
        k
    }

    /// The extent of a symbol's graphics, text excluded, with the symbols drawn in it (9.2.3).
    pub(crate) fn symbol_extent(&self, sym: ObjId, depth: usize) -> [f64; 4] {
        let mut b = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
        fn add(b: &mut [f64; 4], p: (f64, f64)) {
            b[0] = b[0].min(p.0);
            b[1] = b[1].min(p.1);
            b[2] = b[2].max(p.0);
            b[3] = b[3].max(p.1);
        }
        let sd = self.symbol(sym);
        for p in &sd.prims {
            for q in crate::geom::shape_extent_points(
                p.shape.as_str(),
                &p.points,
                p.params.width,
                p.params.height,
                p.params.radius,
            ) {
                add(&mut b, q);
            }
        }
        if depth < 8 {
            for a in &sd.usages {
                let u = self.usage(*a);
                let (Some(inner), Some((_, pt))) = (u.sym_symbol, u.roles.first()) else { continue };
                if !matches!(self.o(inner), ObjData::Symbol(_)) {
                    continue;
                }
                let ObjData::Point { x: Some(x), y: Some(y), .. } = self.o(*pt) else { continue };
                let ib = self.symbol_extent(inner, depth + 1);
                if ib[0] > ib[2] {
                    continue;
                }
                for c in [(ib[0], ib[1]), (ib[0], ib[3]), (ib[2], ib[1]), (ib[2], ib[3])] {
                    let d = u.sym_orient.apply(c);
                    add(&mut b, (x + d.0, y + d.1));
                }
            }
        }
        if b[0] > b[2] {
            for pin in &sd.pins {
                add(&mut b, pin.pos);
            }
        }
        if b[0] > b[2] {
            return [0.0; 4];
        }
        b
    }

    /// The fixed texts of a symbol drawn for `owner`, with `${…}` read from the owner.
    fn symbol_texts(
        &mut self,
        s: SceneId,
        sym: ObjId,
        owner: ObjId,
        prefix: &str,
        out: &mut BTreeMap<String, Vec<markup::Line>>,
        depth: usize,
    ) {
        if depth > 8 {
            return;
        }
        let sd = self.symbol(sym).clone();
        for (k, p) in sd.prims.iter().enumerate() {
            if p.shape != Shape::Text {
                continue;
            }
            let Some(t) = p.text.as_ref().and_then(|t| t.text.clone()) else { continue };
            let expanded = self.expand_text(s, &t, owner);
            out.insert(format!("{prefix}{k}"), markup::parse(&expanded));
        }
        let mut n = 0;
        for a in &sd.usages {
            let u = self.usage(*a).clone();
            let Some(inner) = u.sym_symbol else { continue };
            if !matches!(self.o(inner), ObjData::Symbol(_)) {
                continue;
            }
            self.symbol_texts(s, inner, owner, &format!("{prefix}{n}/"), out, depth + 1);
            n += 1;
        }
    }

    /// An object drawn with the symbol `sym` at `at`: the item of a level, with the nets at the pins
    /// of its symbol. `table` and `symbols` hold the symbols of the items made so far.
    #[allow(clippy::too_many_arguments)]
    fn item_view(
        &mut self,
        s: SceneId,
        o: ObjId,
        sym: ObjId,
        at: Pt,
        table: &mut HashMap<ObjId, usize>,
        symbols: &mut Vec<SymbolView>,
        last_symbol: &HashMap<ObjId, Pos>,
    ) -> ItemView {
        let si = self.symbol_view(sym, table, symbols);
        let mut texts = BTreeMap::new();
        self.symbol_texts(s, sym, o, "", &mut texts, 0);
        let (kind, host) = match self.o(o).clone() {
            ObjData::Inst { idx, .. } => ("instance", vec![idx]),
            ObjData::Bundle { members, .. } => (
                "bundle",
                members
                    .iter()
                    .filter_map(|m| match self.o(*m) {
                        ObjData::Inst { idx, .. } => Some(*idx),
                        _ => None,
                    })
                    .collect(),
            ),
            _ => ("usage", Vec::new()),
        };
        let mut pin_nets = vec![Vec::new(); self.symbol(sym).pins.len()];
        for t in self.terms_of(s, o) {
            if let Some(k) = self.pin_for(s, o, t) {
                for n in self.term_nets(s, t) {
                    let d = self.net_display(n);
                    if !pin_nets[k].contains(&d) {
                        pin_nets[k].push(d);
                    }
                }
            }
        }
        ItemView {
            id: self.label(o),
            object_id: String::new(),
            kind,
            symbol: si,
            at,
            orient: self.orientation(s, o),
            style: self.style_of(s, o),
            layer: self.layer_of(s, o),
            order: self.key_of(s, o, last_symbol),
            texts,
            host,
            pin_nets,
        }
    }

    /// The devices the level does not draw, since no symbol can be chosen for them.
    fn undrawn_views(&mut self, s: SceneId) -> Vec<UndrawnView> {
        let mut out = Vec::new();
        for (o, reason) in self.scenes[s].undrawn.clone() {
            let host = match self.o(o).clone() {
                ObjData::Inst { idx, .. } => vec![idx],
                ObjData::Bundle { members, .. } => members
                    .iter()
                    .filter_map(|m| match self.o(*m) {
                        ObjData::Inst { idx, .. } => Some(*idx),
                        _ => None,
                    })
                    .collect(),
                _ => Vec::new(),
            };
            out.push(UndrawnView { id: self.label(o), host, reason });
        }
        out
    }

    fn level_view(&mut self, s: SceneId) -> LevelView {
        let lv = self.scenes[s].level;
        let level = self.doc.levels[lv].clone();
        let mut table: HashMap<ObjId, usize> = HashMap::new();
        let mut symbols: Vec<SymbolView> = Vec::new();
        let mut items = Vec::new();
        let last_symbol = self.last_symbol_pos(s);

        // the objects of the editor interface, drawn or not, with ids unique in the level
        let mut objs: Vec<ObjId> = self.inst_like(s).into_iter().filter(|o| !self.bundle_empty(*o)).collect();
        objs.extend(self.scenes[s].usages.iter().copied().filter(|a| !self.usage(*a).in_symbol));
        let named_points: Vec<ObjId> =
            self.scenes[s].points.iter().copied().filter(|p| !self.objs[p.0 as usize].name.is_empty()).collect();
        let wire_objs: Vec<ObjId> = self.scenes[s].wires.iter().copied().filter(|w| !self.is_obj_failed(*w)).collect();
        let all: Vec<ObjId> = objs.iter().chain(&named_points).chain(&wire_objs).copied().collect();
        let mut ids = self.unique_ids(s, &all, &HashMap::new());
        // points whose name the level does not give them alone are left out
        let named_points: Vec<ObjId> = named_points.into_iter().filter(|p| ids[p].starts_with('%')).collect();
        let plain_texts: Vec<ObjId> =
            self.scenes[s].texts.iter().copied().filter(|t| !matches!(self.o(*t), ObjData::KeyText { .. })).collect();
        let text_ids = self.unique_ids(s, &plain_texts, &ids);
        ids.extend(text_ids);
        let prims = self.scenes[s].prims.clone();
        let prim_ids = self.unique_ids(s, &prims, &ids);
        ids.extend(prim_ids);
        let (objects, points) = self.objects_of(s, &objs, &named_points, &ids);
        let mut terminal_names = vec![Vec::new(); level.instances.len()];
        for i in self.scenes[s].insts.clone() {
            let ObjData::Inst { idx, .. } = *self.o(i) else { continue };
            terminal_names[idx] = self.owner_terms(s, i).into_iter().map(|(_, n)| n).collect();
        }
        let mut drawn: Vec<ObjId> = self.inst_like(s).into_iter().filter(|o| !self.bundle_empty(*o)).collect();
        for &a in &self.scenes[s].usages {
            let u = self.usage(a);
            if !u.in_symbol && self.scenes[s].symbol.contains_key(&a) {
                drawn.push(a);
            }
        }
        // an object without a position is not drawn: it is listed with its symbol at the origin
        let mut unplaced_table: HashMap<ObjId, usize> = HashMap::new();
        let mut unplaced_symbols: Vec<SymbolView> = Vec::new();
        let mut unplaced_items = Vec::new();
        // where each object that is drawn is: for the texts attached to it
        let mut item_of: HashMap<ObjId, usize> = HashMap::new();
        let mut wire_of: HashMap<ObjId, usize> = HashMap::new();
        for o in drawn {
            let Some(&sym) = self.scenes[s].symbol.get(&o) else { continue };
            match self.xy(s, o) {
                Some(at) => {
                    item_of.insert(o, items.len());
                    let mut item = self.item_view(s, o, sym, at, &mut table, &mut symbols, &last_symbol);
                    item.object_id = ids.get(&o).cloned().unwrap_or_default();
                    items.push(item);
                }
                None => unplaced_items.push(self.item_view(
                    s,
                    o,
                    sym,
                    (0.0, 0.0),
                    &mut unplaced_table,
                    &mut unplaced_symbols,
                    &last_symbol,
                )),
            }
        }
        // the symbols of the drawn objects, as the items have them
        let mut objects = objects;
        for (info, &o) in objects.iter_mut().zip(&objs) {
            info.symbol_view = self.scenes[s].symbol.get(&o).and_then(|sym| table.get(sym).copied());
        }
        // a wire some of whose points have no position yet is drawn in its runs of points that
        // have one, each a WireView of the wire, with flylines over the gaps (the draft 0.3,
        // section 9); a rewritten wire is drawn by its rewrite (13.1)
        let mut wires = Vec::new();
        let mut flylines = Vec::new();
        for w in self.scenes[s].wires.clone() {
            if self.is_obj_failed(w) {
                continue;
            }
            let ObjData::Wire { nets, .. } = self.o(w).clone() else { continue };
            let Some(shape) = self.wire_shape(s, w) else { continue };
            // the vertices as written: the rewrite's, when one holds (13.1)
            let via: Vec<Vertex> = shape.points.iter().filter_map(|v| self.vertex(s, *v, &ids)).collect();
            // every point of the wire has a position: it is drawn whole, as one run
            let whole = shape.points.iter().all(|&q| self.point_of(s, q).is_some());
            let (runs, gaps) = self.wire_drawing(s, w);
            flylines.extend(gaps);
            for points in runs {
                wire_of.entry(w).or_insert(wires.len());
                wires.push(WireView {
                    id: self.label(w),
                    object_id: self.ref_id(s, w, &ids),
                    nets: nets.iter().map(|n| self.net_display(*n)).collect(),
                    points,
                    via: via.clone(),
                    mode: shape.mode.clone(),
                    style: self.style_of(s, w),
                    own_style: self.own_style_of(s, w),
                    layer: self.layer_of(s, w),
                    order: self.key_of(s, w, &last_symbol),
                    whole,
                });
            }
        }
        let routes = self.scenes[s].routes.clone();
        if !routes.is_empty() {
            let terms = self.route_terms(s);
            for r in routes {
                flylines.extend(self.flywires(s, r, &terms));
            }
        }
        let mut shapes = Vec::new();
        for p in prims {
            let ObjData::Prim { shape, points, params, .. } = self.o(p).clone() else { continue };
            let mut pts = Vec::new();
            let mut ok = true;
            for q in points {
                match self.point_of(s, q) {
                    Some(xy) => pts.push(xy),
                    None => {
                        ok = false;
                        break;
                    }
                }
            }
            if !ok {
                continue;
            }
            shapes.push(ShapeView {
                object_id: ids[&p].clone(),
                shape,
                points: pts,
                params,
                style: self.style_of(s, p),
                own_style: self.own_style_of(s, p),
                layer: self.layer_of(s, p),
                order: self.key_of(s, p, &last_symbol),
            });
        }
        let mut texts = Vec::new();
        for t in self.scenes[s].texts.clone() {
            if self.is_obj_failed(t) {
                continue;
            }
            // a text whose attached object is drawn is drawn too (13.3; the draft 0.3, section 9)
            let Some((anchor, from_symbol)) = self.text_drawn_at(s, t) else { continue };
            let it = self.text_items(t);
            let Some(text) = it.text.clone() else { continue };
            let att = self.text_attach(t);
            let drawable_att = matches!(self.o(att), ObjData::Inst { .. } | ObjData::Usage(_) | ObjData::Bundle { .. });
            let orient = if from_symbol { self.orientation(s, att) } else { Orient::R0 };
            let expanded = self.expand_text(s, &text, att);
            let mut layer = self.layer_of(s, t);
            if drawable_att && layer == Layer::Middle {
                layer = self.layer_of(s, att);
            }
            let target = match self.o(att) {
                ObjData::Net { .. } => Some(TextTarget::Net(self.net_display(att))),
                _ => item_of
                    .get(&att)
                    .map(|&i| TextTarget::Item(i))
                    .or_else(|| wire_of.get(&att).map(|&i| TextTarget::Wire(i))),
            };
            let (key, attach) = match self.o(t) {
                ObjData::KeyText { key, .. } => (Some(key.clone()), Some(self.ref_id(s, att, &ids))),
                _ if matches!(self.o(att), ObjData::Point { .. } | ObjData::LevelObj { .. }) => (None, None),
                _ => (None, Some(self.ref_id(s, att, &ids))),
            };
            // a label the symbol gives has its layer (at -1) and a package's until the document
            // writes one of its own
            let written = match self.o(t) {
                ObjData::KeyText { layers, .. } => {
                    layers.iter().any(|(p, _, _)| !from_package(p) && p.first() != Some(&PosElem::Int(-1)))
                }
                ObjData::Text { from_symbol, .. } => !from_symbol,
                _ => true,
            };
            texts.push(TextView {
                id: self.label(t),
                object_id: self.ref_id(s, t, &ids),
                attach,
                target,
                key,
                markup: text.as_str().to_string(),
                written,
                anchor,
                lines: markup::parse(&expanded),
                halign: it.halign.clone().unwrap_or_else(|| "start".into()),
                valign: it.valign.clone().unwrap_or_else(|| "baseline".into()),
                rotate: it.rotate.unwrap_or(0.0),
                orient,
                style: self.style_of(s, t),
                own_style: self.own_style_of(s, t),
                layer,
                order: self.key_of(s, t, &last_symbol),
            });
        }
        let mut constraints = Vec::new();
        for c in self.scenes[s].constraints.clone() {
            let ObjData::Constraint(cd) = self.o(c).clone() else { continue };
            let status = if cd.failed { "failed" } else { cd.status.unwrap_or("unchecked") };
            constraints.push(ConstraintView {
                label: self.label(c),
                kind: cd.kind.as_str().to_string(),
                status: status.to_string(),
                priority: cd.eff.unwrap_or(Priority::Normal).as_str().to_string(),
            });
        }
        let packages = self.level_packages(lv);
        LevelView {
            level: lv,
            title: level.title(),
            kind: if level.kind == LevelKind::Subckt { "subckt" } else { "file" },
            name: level.name.clone(),
            file: level.file.clone(),
            line: level.line,
            symbols,
            items,
            wires,
            shapes,
            texts,
            flylines,
            constraints,
            unplaced: self.scenes[s].unplaced.clone(),
            undrawn: self.undrawn_views(s),
            unplaced_items,
            unplaced_symbols,
            objects,
            points,
            terminal_names,
            block_symbol: self.block_symbol_of(lv),
            packages,
        }
    }

    /// The packages in effect at a level in the order of their places (8.3, 16.2, 16.3): at the
    /// root, then at the USE-PACKAGEs of the levels from the top down.
    fn level_packages(&mut self, lv: usize) -> Vec<LevelPackage> {
        let mut chain = self.doc.ancestors(lv);
        chain.reverse();
        chain.push(lv);
        let eff = self.effective_packages(lv);
        let mut out: Vec<(_, LevelPackage)> = eff
            .entries
            .iter()
            .map(|e| {
                let (at, key) = match e.place {
                    Place::Root(k, j) => (None, (0, k, (0, 0), j)),
                    Place::Level(l, p, j) => (Some(l), (1 + chain.iter().position(|c| *c == l).unwrap_or(0), 0, p, j)),
                };
                let via = e.decl.as_ref().and_then(|d| Some((d.name.clone(), self.rec_loc(d.rec, d.offset)?)));
                let u = &self.units[e.unit];
                (key, LevelPackage { name: u.pkg_name.clone(), version: self.pkg_ver(e.unit), at, via })
            })
            .collect();
        out.sort_by_key(|(key, _)| *key);
        out.into_iter().map(|(_, p)| p).collect()
    }

    /// The symbol a subcircuit gives itself with `# @SET-SYMBOL`, the later one when it is written
    /// more than once (9.2.6, 8.3).
    fn block_symbol_of(&mut self, lv: usize) -> Option<BlockSymbol> {
        let level = &self.doc.levels[lv];
        if level.kind != LevelKind::Subckt {
            return None;
        }
        let key = self.doc.key(&level.name);
        let sym = self.block_symbols.get(&key)?.values().max_by(|a, b| a.0.cmp(&b.0))?.1;
        let mut views = Vec::new();
        let k = self.symbol_view(sym, &mut HashMap::new(), &mut views);
        let mut view = views[k].clone();
        // the symbols drawn in it go by their index in `views`, which is not kept
        view.nested.clear();
        Some(BlockSymbol {
            info: SymbolInfo {
                name: self.symbol_name(sym),
                origin: self.symbol_origin(sym),
                default: self.symbol(sym).default,
            },
            view,
            symbols: views,
        })
    }
}
