//! Attribute references `${a::b::c}` (standard 13.6): what a text shows of the object it is
//! attached to.

use crate::geom::fmt_num;
use crate::words::words;

use super::obj::*;
use super::Interp;

/// A value along an attribute path: an object, text, or a list of them.
#[derive(Clone, Debug)]
pub enum AVal {
    Obj(ObjId),
    Text(String),
    List(Vec<AVal>),
}

const INST_ATTRS: [&str; 4] = ["name", "value", "model", "type"];

impl Interp<'_> {
    /// Take attributes step by step; the last value is shown by its kind. `unit` is where the text
    /// is written, whose aliases the names are read with (16.5).
    pub(crate) fn eval_path(
        &mut self,
        s: SceneId,
        att: ObjId,
        path: &str,
        unit: Option<UnitId>,
    ) -> Result<String, String> {
        let mut v = AVal::Obj(att);
        for step in path.split("::") {
            let step = step.trim();
            if step.is_empty() {
                return Err(format!("`${{{path}}}` is not well formed"));
            }
            v = self.attr_get(s, v, step, unit)?;
        }
        self.display(s, &v)
    }

    fn attr_get(&mut self, s: SceneId, v: AVal, name: &str, unit: Option<UnitId>) -> Result<AVal, String> {
        let host_only = name.starts_with('#');
        let mut key = name.trim_start_matches('#').to_string();
        let obj = match v {
            AVal::Text(_) => return Err(format!("text has no attributes; `::{name}` reaches nothing")),
            AVal::List(items) => {
                let mut out = Vec::new();
                for x in items {
                    out.push(self.attr_get(s, x, name, unit)?);
                }
                return Ok(AVal::List(out));
            }
            AVal::Obj(o) => o,
        };
        if self.is_set(obj) {
            let mut out = Vec::new();
            for m in self.flatten(s, obj, 8, None) {
                out.push(self.attr_get(s, AVal::Obj(m), name, unit)?);
            }
            return Ok(AVal::List(out));
        }
        if let ObjData::Bundle { members, .. } = self.o(obj).clone() {
            let mut vals = Vec::new();
            for m in members {
                vals.push(self.attr_get(s, AVal::Obj(m), name, unit)?);
            }
            let mut texts = Vec::new();
            for v in &vals {
                texts.push(self.display(s, v)?);
            }
            if texts.iter().all(|t| *t == texts[0]) {
                return Ok(vals.swap_remove(0));
            }
            return Ok(AVal::Text(texts.join(",")));
        }
        if let (ObjData::Usage(u), false) = (self.o(obj).clone(), host_only) {
            // role and parameter names are RNL names: only the aliases scoped to the definition (16.5)
            let names: Vec<String> = self.def(u.definition).params.iter().map(|(n, _)| n.clone()).collect();
            if u.role(&key).is_none() && !names.contains(&key) {
                if let Some(un) = unit {
                    if let Some((_, o)) = self.scoped_for(un, u.definition).into_iter().find(|(a, _)| *a == key) {
                        key = o;
                    }
                }
            }
            if let Some(r) = u.role(&key) {
                return Ok(AVal::Obj(r));
            }
            if names.contains(&key) {
                let v = u.params.iter().find(|(n, _)| *n == key).and_then(|(_, v)| v.clone());
                return Ok(AVal::Text(match v {
                    None => String::new(),
                    Some(Lit::Num(n)) => fmt_num(n),
                    Some(Lit::Str(w)) => w.as_str().to_string(),
                    Some(other) => other.show(),
                }));
            }
            let l = self.label(obj);
            return Err(format!("{l} has no attribute {name}"));
        }
        if !host_only && !words().attributes.contains(&key) {
            if let Some(un) = unit {
                let std = self.word(&key, un); // an alias of a standard attribute name (13.6, 16.5)
                if words().attributes.contains(&std) {
                    key = std;
                }
            }
        }
        match self.o(obj).clone() {
            ObjData::Inst { .. } => {
                let inst = self.inst_of(obj).clone();
                if !host_only && INST_ATTRS.contains(&key.as_str()) {
                    // model: the name in the model position, the subcircuit name for X instances (13.6, TR1)
                    return Ok(AVal::Text(match key.as_str() {
                        "name" => inst.name.clone(),
                        "value" => inst.value.clone().unwrap_or_default(),
                        "model" => inst.model.clone().unwrap_or_default(),
                        _ => inst.mtype.clone().unwrap_or_default(),
                    }));
                }
                if let Some(v) = inst.param(&key) {
                    return Ok(AVal::Text(v.to_string()));
                }
                let l = self.label(obj);
                Err(format!("{l} has no attribute {name}"))
            }
            _ if host_only => {
                let l = self.label(obj);
                Err(format!("{l} is not a host instance and has no host parameter {name}"))
            }
            ObjData::Term { .. } if key == "net" => Ok(self.term_nets_val(s, obj)),
            ObjData::Wire { nets, .. } if key == "net" => {
                if nets.len() == 1 {
                    Ok(AVal::Obj(nets[0]))
                } else {
                    Ok(AVal::List(nets.into_iter().map(AVal::Obj).collect()))
                }
            }
            ObjData::Net { display, .. } if key == "name" => Ok(AVal::Text(display)),
            ObjData::Point { .. } | ObjData::Pos { .. } if key == "x" || key == "y" => {
                let p = self.point_of(s, obj);
                Ok(AVal::Text(match p {
                    None => String::new(),
                    Some(p) => fmt_num(if key == "x" { p.0 } else { p.1 }),
                }))
            }
            _ => {
                let l = self.label(obj);
                Err(format!("{l} has no attribute {name}"))
            }
        }
    }

    fn term_nets_val(&mut self, s: SceneId, t: ObjId) -> AVal {
        let nets = self.term_nets(s, t);
        if nets.len() == 1 {
            AVal::Obj(nets[0])
        } else {
            AVal::List(nets.into_iter().map(AVal::Obj).collect())
        }
    }

    /// The nets of a terminal: a usage's terminal takes the net of its role member.
    pub(crate) fn term_nets(&mut self, s: SceneId, t: ObjId) -> Vec<ObjId> {
        let ObjData::Term { owner, name, .. } = self.o(t).clone() else { return Vec::new() };
        if let ObjData::Usage(u) = self.o(owner).clone() {
            let role = name.as_ref().and_then(|n| u.role(n));
            return match role.map(|r| (r, self.o(r).clone())) {
                Some((r, ObjData::Net { .. })) => vec![r],
                Some((r, ObjData::Term { .. })) => self.term_nets(s, r),
                _ => Vec::new(),
            };
        }
        self.tnets(s, t)
    }

    /// How a value shows as text (13.6).
    pub(crate) fn display(&mut self, s: SceneId, v: &AVal) -> Result<String, String> {
        match v {
            AVal::Text(t) => Ok(t.clone()),
            AVal::List(items) => {
                let mut out = Vec::new();
                for x in items {
                    out.push(self.display(s, x)?);
                }
                Ok(out.join(","))
            }
            AVal::Obj(o) => {
                let o = *o;
                match self.o(o).clone() {
                    ObjData::Set(_) => {
                        let mut out = Vec::new();
                        for m in self.flatten(s, o, 8, None) {
                            out.push(self.display(s, &AVal::Obj(m))?);
                        }
                        Ok(out.join(","))
                    }
                    ObjData::Inst { .. } => Ok(self.inst_of(o).name.clone()),
                    ObjData::Bundle { members, .. } => {
                        Ok(members.iter().map(|m| self.inst_of(*m).name.clone()).collect::<Vec<_>>().join(","))
                    }
                    ObjData::Net { display, .. } => Ok(display),
                    ObjData::Term { .. } => {
                        let nets = self.term_nets(s, o);
                        let v = if nets.len() == 1 {
                            AVal::Obj(nets[0])
                        } else {
                            AVal::List(nets.into_iter().map(AVal::Obj).collect())
                        };
                        self.display(s, &v)
                    }
                    ObjData::Wire { nets, .. } => {
                        Ok(nets.iter().map(|n| self.net_display(*n)).collect::<Vec<_>>().join(","))
                    }
                    _ => {
                        let l = self.label(o);
                        Err(format!("{l} cannot be shown; take one of its attributes, such as `::x`"))
                    }
                }
            }
        }
    }

    /// The text of a TEXT with its `${…}` references replaced by their values (13.6).
    pub(crate) fn expand_text(&mut self, s: SceneId, text: &WStr, att: ObjId) -> String {
        let unit = text.unit;
        let mut values = Vec::new();
        for path in crate::markup::attr_refs(text.as_str()) {
            let v = self.eval_path(s, att, &path, unit).unwrap_or_default();
            values.push((path, v));
        }
        let mut k = 0;
        crate::markup::expand(text.as_str(), |_p| {
            let v = values.get(k).map(|(_, v)| v.clone()).unwrap_or_default();
            k += 1;
            v
        })
    }
}
