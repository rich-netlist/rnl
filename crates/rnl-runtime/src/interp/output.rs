//! From scenes to the effective model the renderer reads (crate::model).

use std::collections::{BTreeMap, HashMap};

use crate::geom::Orient;
use crate::host::LevelKind;
use crate::markup;
use crate::model::*;

use super::obj::*;
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

impl Interp<'_> {
    pub(crate) fn build_model(&mut self) -> Model {
        let mut levels = Vec::new();
        for s in 0..self.scenes.len() {
            levels.push(self.level_view(s));
        }
        // this version of the language has no conflict that makes the whole interpretation fail (7.5)
        Model { levels, failed: None }
    }

    fn style_of(&self, s: SceneId, o: ObjId) -> Style {
        self.scenes[s]
            .style
            .get(&o)
            .map(|m| m.iter().map(|(k, (v, _))| (k.clone(), v.clone())).collect())
            .unwrap_or_default()
    }

    fn layer_of(&self, s: SceneId, o: ObjId) -> Layer {
        self.scenes[s].layer.get(&o).map(|(l, _)| Layer::parse(l)).unwrap_or_default()
    }

    /// Where the latest symbol of each object of a scene is given.
    fn last_symbol_pos(&self, s: SceneId) -> HashMap<ObjId, Pos> {
        let mut out: HashMap<ObjId, Pos> = HashMap::new();
        for f in &self.scenes[s].appear {
            if out.get(&f.target).is_none_or(|p| f.pos > *p) {
                out.insert(f.target, f.pos.clone());
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
                let p =
                    last_symbol.get(&o).cloned().or_else(|| sc.created.get(&o).cloned()).unwrap_or_else(|| pos(&[9]));
                let mut k = vec![1.0];
                k.extend(flat(&p));
                k
            }
            _ => {
                let mut k = vec![1.0];
                k.extend(flat(&sc.created.get(&o).cloned().unwrap_or_else(|| pos(&[9]))));
                k
            }
        }
    }

    fn symbol_view(&mut self, sym: ObjId, table: &mut HashMap<ObjId, usize>, out: &mut Vec<SymbolView>) -> usize {
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
        out.push(SymbolView {
            name: self.label(sym),
            prims: Vec::new(),
            pins: Vec::new(),
            nested: Vec::new(),
            bbox,
            box_written: sd.bbox.is_some(),
            default: sd.default,
        });

        let prims = sd
            .prims
            .iter()
            .map(|p| SymbolPrim {
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

    fn level_view(&mut self, s: SceneId) -> LevelView {
        let lv = self.scenes[s].level;
        let level = self.doc.levels[lv].clone();
        let mut table: HashMap<ObjId, usize> = HashMap::new();
        let mut symbols: Vec<SymbolView> = Vec::new();
        let mut items = Vec::new();
        let last_symbol = self.last_symbol_pos(s);
        let mut drawn: Vec<ObjId> = self.inst_like(s).into_iter().filter(|o| !self.bundle_empty(*o)).collect();
        for &a in &self.scenes[s].usages {
            let u = self.usage(a);
            if !u.in_symbol && self.scenes[s].symbol.contains_key(&a) {
                drawn.push(a);
            }
        }
        for o in drawn {
            let Some(&sym) = self.scenes[s].symbol.get(&o) else { continue };
            let Some(at) = self.xy(s, o) else { continue };
            let si = self.symbol_view(sym, &mut table, &mut symbols);
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
            items.push(ItemView {
                id: self.label(o),
                kind,
                symbol: si,
                at,
                orient: self.orientation(s, o),
                style: self.style_of(s, o),
                layer: self.layer_of(s, o),
                order: self.key_of(s, o, &last_symbol),
                texts,
                host,
                pin_nets,
            });
        }
        let mut wires = Vec::new();
        for w in self.scenes[s].wires.clone() {
            if self.is_obj_failed(w) {
                continue;
            }
            let Some(points) = self.wire_points(s, w) else { continue };
            let ObjData::Wire { nets, mode, .. } = self.o(w).clone() else { continue };
            wires.push(WireView {
                id: self.label(w),
                nets: nets.iter().map(|n| self.net_display(*n)).collect(),
                points,
                mode,
                style: self.style_of(s, w),
                layer: self.layer_of(s, w),
                order: self.key_of(s, w, &last_symbol),
            });
        }
        let mut flylines = Vec::new();
        let routes = self.scenes[s].routes.clone();
        if !routes.is_empty() {
            let terms = self.route_terms(s);
            for r in routes {
                flylines.extend(self.flywires(s, r, &terms));
            }
        }
        let mut shapes = Vec::new();
        for p in self.scenes[s].prims.clone() {
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
                shape,
                points: pts,
                params,
                style: self.style_of(s, p),
                layer: self.layer_of(s, p),
                order: self.key_of(s, p, &last_symbol),
            });
        }
        let mut texts = Vec::new();
        for t in self.scenes[s].texts.clone() {
            if self.is_obj_failed(t) {
                continue;
            }
            let Some(anchor) = self.text_anchor(s, t) else { continue };
            let it = self.text_items(t);
            let Some(text) = it.text.clone() else { continue };
            let att = self.text_attach(t);
            let drawable_att = matches!(self.o(att), ObjData::Inst { .. } | ObjData::Usage(_) | ObjData::Bundle { .. });
            let explicit = self.xy(s, t).is_some();
            let orient = if self.text_sym_pos(t).is_some() && drawable_att && !explicit {
                self.orientation(s, att)
            } else {
                Orient::R0
            };
            let expanded = self.expand_text(s, &text, att);
            let mut layer = self.layer_of(s, t);
            if drawable_att && layer == Layer::Middle {
                layer = self.layer_of(s, att);
            }
            texts.push(TextView {
                id: self.label(t),
                anchor,
                lines: markup::parse(&expanded),
                halign: it.halign.clone().unwrap_or_else(|| "start".into()),
                valign: it.valign.clone().unwrap_or_else(|| "baseline".into()),
                rotate: it.rotate.unwrap_or(0.0),
                orient,
                style: self.style_of(s, t),
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
        }
    }
}
