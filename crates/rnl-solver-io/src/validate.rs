use crate::json::{Error, Result};
use rnl_runtime::problem::*;
use std::collections::{BTreeSet, VecDeque};

fn error(path: &str, message: impl Into<String>) -> Error {
    Error { path: path.into(), message: message.into() }
}
fn index(i: usize, len: usize, path: &str) -> Result<()> {
    if i < len {
        Ok(())
    } else {
        Err(error(path, format!("index {i} is out of range (length {len})")))
    }
}
fn indices(values: &[usize], len: usize, path: &str) -> Result<()> {
    for (i, &value) in values.iter().enumerate() {
        index(value, len, &format!("{path}[{i}]"))?;
    }
    Ok(())
}
fn reference(r: &Reference, p: &LevelProblem, path: &str) -> Result<()> {
    crate::solution::validate_reference(r, p, path)
}
fn point(r: &PointRef, p: &LevelProblem, path: &str, depth: usize) -> Result<()> {
    if depth > 64 {
        return Err(error(path, "point nesting exceeds 64 levels"));
    }
    if let PointRef::Centre(members) = r {
        for (i, r) in members.iter().enumerate() {
            point(r, p, &format!("{path}.members[{i}]"), depth + 1)?;
        }
        Ok(())
    } else {
        crate::solution::validate_point(r, p, path)
    }
}
fn points(rs: &[PointRef], p: &LevelProblem, path: &str) -> Result<()> {
    for (i, r) in rs.iter().enumerate() {
        point(r, p, &format!("{path}[{i}]"), 0)?;
    }
    Ok(())
}
fn slot(s: &Slot, p: &LevelProblem, path: &str) -> Result<()> {
    if let Slot::Fixed { fact, .. } = s {
        index(*fact, p.facts.len(), path)?;
    }
    Ok(())
}
fn directed(d: DirectedPin, p: &LevelProblem, path: &str) -> Result<()> {
    point(&PointRef::SymbolPin { object: d.object, pin: d.pin, reference: Reference::Anonymous }, p, path, 0)?;
    if ![0, 90, 180, 270].contains(&d.direction) {
        return Err(error(path, "pin direction must be a quarter turn"));
    }
    Ok(())
}
fn filler(f: &Filler, p: &LevelProblem, path: &str) -> Result<()> {
    match f {
        Filler::Object(k) => index(*k, p.objects.len(), path),
        Filler::Pin { object, pin } | Filler::Terminal { object, pin: Some(pin), .. } => {
            point(&PointRef::SymbolPin { object: *object, pin: *pin, reference: Reference::Anonymous }, p, path, 0)
        }
        Filler::Terminal { object, pin: None, .. } => index(*object, p.objects.len(), path),
        Filler::Usage(k) => index(*k, p.usages.len(), path),
        Filler::Wire(k) => index(*k, p.wires.len(), path),
        Filler::Net(_) | Filler::Other(_) => Ok(()),
    }
}
fn table(t: &TableCell<PointRef>, p: &LevelProblem, path: &str, depth: usize) -> Result<()> {
    if depth > 64 {
        return Err(error(path, "TABLE nesting exceeds 64 levels"));
    }
    match t {
        TableCell::Point(r) => point(r, p, path, 0),
        TableCell::List(items) => {
            for (i, t) in items.iter().enumerate() {
                table(t, p, &format!("{path}.items[{i}]"), depth + 1)?;
            }
            Ok(())
        }
    }
}
fn object_dependencies(r: &PointRef, out: &mut Vec<usize>) {
    match r {
        PointRef::Origin(k)
        | PointRef::Pin { object: k, .. }
        | PointRef::SymbolPin { object: k, .. }
        | PointRef::PinPosition { object: k, .. } => out.push(*k),
        PointRef::Centre(members) => {
            for r in members {
                object_dependencies(r, out);
            }
        }
    }
}
fn acyclic(dependencies: &[Vec<usize>], path: &str) -> Result<()> {
    let mut degrees: Vec<_> = dependencies.iter().map(Vec::len).collect();
    let mut dependents = vec![Vec::new(); dependencies.len()];
    for (k, ds) in dependencies.iter().enumerate() {
        for &d in ds {
            dependents[d].push(k);
        }
    }
    let mut ready: VecDeque<_> = degrees.iter().enumerate().filter_map(|(k, &n)| (n == 0).then_some(k)).collect();
    let mut count = 0;
    while let Some(k) = ready.pop_front() {
        count += 1;
        for &next in &dependents[k] {
            degrees[next] -= 1;
            if degrees[next] == 0 {
                ready.push_back(next);
            }
        }
    }
    if count == dependencies.len() {
        Ok(())
    } else {
        Err(error(path, "cyclic dependency"))
    }
}

pub(crate) fn problem(p: &LevelProblem) -> Result<()> {
    if !["0.2", "0.3", "0.4"].contains(&p.version.as_str()) {
        return Err(error("$.version", "unsupported RNL language version"));
    }
    let mut positions = vec![Vec::new(); p.objects.len()];
    for (k, o) in p.objects.iter().enumerate() {
        let at = format!("$.objects[{k}]");
        reference(&o.reference, p, &format!("{at}.reference"))?;
        if let Some(s) = o.symbol {
            index(s, p.symbols.len(), &format!("{at}.symbol"))?;
        }
        if let Some(u) = o.usage {
            index(u, p.usages.len(), &format!("{at}.usage"))?;
        }
        indices(&o.nets, p.nets.len(), &format!("{at}.nets"))?;
        if let Some(OrientSlot::Fixed { fact, .. }) = &o.orient {
            index(*fact, p.facts.len(), &format!("{at}.orient.fact"))?;
        }
        match &o.position {
            Position::Own { x, y } => {
                slot(x, p, &format!("{at}.position.x"))?;
                slot(y, p, &format!("{at}.position.y"))?;
            }
            Position::At(r) => {
                point(r, p, &format!("{at}.position.point"), 0)?;
                object_dependencies(r, &mut positions[k]);
            }
            Position::InSymbol { owner, replaced, .. } => {
                index(*owner, p.objects.len(), &format!("{at}.position.owner"))?;
                positions[k].push(*owner);
                if let Some((_, s)) = replaced {
                    slot(s, p, &format!("{at}.position.replaced"))?;
                }
            }
        }
        if let Some(t) = &o.text {
            filler(&t.attach, p, &format!("{at}.text.attach"))?;
        }
    }
    acyclic(&positions, "$.objects.position")?;
    for (i, f) in p.facts.iter().enumerate() {
        let at = format!("$.facts[{i}]");
        if let Some(r) = &f.reference {
            reference(r, p, &format!("{at}.reference"))?;
        }
        indices(&f.chain, p.usages.len(), &format!("{at}.chain"))?;
        match &f.kind {
            FactKind::Place { at: r, .. } => point(r, p, &at, 0)?,
            FactKind::PointCoordinate { point: k, .. } | FactKind::Orient { object: k, .. } => {
                index(*k, p.objects.len(), &at)?
            }
            FactKind::Offset { text, from, .. } => {
                index(*text, p.objects.len(), &at)?;
                point(from, p, &at, 0)?;
            }
            FactKind::PinOrient { pin, .. } => directed(*pin, p, &at)?,
            FactKind::PinOrientRelation { from, to, .. } => {
                directed(*from, p, &at)?;
                directed(*to, p, &at)?;
            }
            FactKind::Align { members, .. }
            | FactKind::Order { members, .. }
            | FactKind::Distribute { members, .. } => points(members, p, &at)?,
            FactKind::Distance { from, to, .. } => {
                points(from, p, &at)?;
                points(to, p, &at)?;
            }
            FactKind::Table { rows, .. } => table(rows, p, &at, 0)?,
            FactKind::OrientRelation { from, to, .. } => {
                indices(from, p.objects.len(), &at)?;
                indices(to, p.objects.len(), &at)?;
            }
        }
    }
    for (i, w) in p.wires.iter().enumerate() {
        let at = format!("$.wires[{i}]");
        reference(&w.reference, p, &at)?;
        points(&w.anchors, p, &at)?;
    }
    for (i, r) in p.routes.iter().enumerate() {
        let at = format!("$.routes[{i}]");
        points(&r.pins, p, &at)?;
        indices(&r.wires, p.wires.len(), &at)?;
    }
    for (i, c) in p.currents.iter().enumerate() {
        let at = format!("$.currents[{i}]");
        point(&c.pin, p, &at, 0)?;
        if !["in", "out"].contains(&c.direction.as_str()) {
            return Err(error(&at, "unknown current direction"));
        }
    }
    for (i, g) in p.groups.iter().enumerate() {
        indices(&g.members, p.objects.len(), &format!("$.groups[{i}].members"))?;
    }
    let mut usages = vec![Vec::new(); p.usages.len()];
    for (i, u) in p.usages.iter().enumerate() {
        let at = format!("$.usages[{i}]");
        reference(&u.reference, p, &at)?;
        if let Some(parent) = u.parent {
            index(parent, i, &format!("{at}.parent"))?;
        }
        if let Some(k) = u.object {
            index(k, p.objects.len(), &format!("{at}.object"))?;
        }
        for (j, r) in u.roles.iter().enumerate() {
            for f in &r.members {
                filler(f, p, &format!("{at}.roles[{j}]"))?;
                if let Filler::Usage(k) = f {
                    usages[i].push(*k);
                }
            }
        }
    }
    acyclic(&usages, "$.usages.roles")?;
    for (i, n) in p.nets.iter().enumerate() {
        let at = format!("$.nets[{i}]");
        reference(&n.reference, p, &at)?;
        points(&n.pins, p, &at)?;
        indices(&n.wires, p.wires.len(), &at)?;
        indices(&n.routes, p.routes.len(), &at)?;
    }
    let mut orient_facts = BTreeSet::new();
    for (i, (f, codes)) in p.orient_lists.iter().enumerate() {
        let at = format!("$.orient_lists[{i}]");
        index(*f, p.facts.len(), &at)?;
        if !orient_facts.insert(*f)
            || codes.is_empty()
            || !matches!(p.facts[*f].kind, FactKind::Orient { .. } | FactKind::OrientRelation { .. })
        {
            return Err(error(&at, "invalid orientation-list fact"));
        }
    }
    Ok(())
}
