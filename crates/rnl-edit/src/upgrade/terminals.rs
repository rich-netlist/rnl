//! Preserve ordinal standard-symbol bindings with explicit terminal maps (11.3).
use std::collections::{BTreeMap, BTreeSet};

use rnl_runtime::host::Record;
use rnl_runtime::interp::unit::scan_package_decl;
use rnl_runtime::syntax::{parse_record_features, Coord, Member, ObjExpr, Value};
use rnl_runtime::Interpretation;

use super::edit::{record_edit, Edit};

#[derive(PartialEq)]
struct Pin {
    x: f64,
    y: f64,
    direction: Option<String>,
    terminal: Value,
}
type Symbols = BTreeMap<(String, String), Vec<Pin>>;

fn symbols(version: &str) -> Symbols {
    let mut out = Symbols::new();
    for (path, text) in &rnl_view::bundled().files {
        if !path.starts_with(&format!("<bundled>/{version}/")) {
            continue;
        }
        let Some((package, _)) = scan_package_decl(text) else { continue };
        for record in rnl_runtime::records::scan_run(
            &rnl_runtime::records::package_lines(text),
            path,
            &mut rnl_runtime::diag::Sink::default(),
        ) {
            let Some(body) = parse_record_features(&record.text, true, true).body else { continue };
            for member in &body.members {
                let Some(binding) = member.binds.iter().find(|b| b.prefix == '$') else { continue };
                let Some(segment) =
                    member.segments.iter().find(|s| s.call.as_ref().is_some_and(|c| c.kw == "DEFINE-SYMBOL"))
                else {
                    continue;
                };
                let Some(body) = segment.args.iter().find_map(|a| match &a.obj {
                    ObjExpr::Set(s) => Some(s),
                    _ => None,
                }) else {
                    continue;
                };
                let mut pins = Vec::new();
                let mut valid = true;
                for statement in &body.members {
                    for segment in &statement.segments {
                        let Some(call) = &segment.call else {
                            // Inheritance and aliases can add or replace pins.
                            valid = false;
                            continue;
                        };
                        if call.kw != "PIN" {
                            continue;
                        }
                        let Some(params) = &call.params else {
                            valid = false;
                            continue;
                        };
                        let Some(terminal) = params.get("terminal") else {
                            valid = false;
                            continue;
                        };
                        let point = match segment.args.as_slice() {
                            [a] if !segment.piped => match &a.obj {
                                ObjExpr::Point(p) => Some(p),
                                _ => None,
                            },
                            _ => None,
                        };
                        let Some(point) = point else {
                            valid = false;
                            continue;
                        };
                        let (Coord::Num(x), Coord::Num(y)) = (&point.x, &point.y) else {
                            valid = false;
                            continue;
                        };
                        let direction = params.get("direction").map(super::rewrite::value);
                        pins.push(Pin { x: *x, y: *y, direction, terminal: terminal.clone() });
                    }
                }
                if valid && !pins.is_empty() {
                    out.insert((package.clone(), format!("${}", binding.name)), pins);
                }
            }
        }
    }
    out
}

fn mapping(before: &[Pin], after: &[Pin]) -> Option<String> {
    if before.len() != after.len() {
        return None;
    }
    let mut pairs = Vec::new();
    let mut ordinals = BTreeSet::new();
    let mut names = BTreeSet::new();
    for old in before {
        let Value::Num { value: number, .. } = &old.terminal else { return None };
        if *number < 1.0 || number.fract() != 0.0 || !ordinals.insert(number.to_string()) {
            return None;
        }
        let found: Vec<_> =
            after.iter().filter(|p| p.x == old.x && p.y == old.y && p.direction == old.direction).collect();
        let [new] = found.as_slice() else { return None };
        let Value::Str(name) = &new.terminal else { return None };
        if name.is_empty() || name.chars().all(|c| c.is_ascii_digit()) || !names.insert(name.to_lowercase()) {
            return None;
        }
        pairs.push(format!("[{number}, {}]", rnl_runtime::problem::written_string(name)));
    }
    Some(format!("@SET-SYMBOL(terminals=[{}])", pairs.join(", ")))
}

#[derive(Default)]
pub(super) struct Terminals {
    maps: BTreeMap<(usize, String), String>,
}
impl Terminals {
    pub(super) fn new(input: &Interpretation) -> Self {
        let mut maps = BTreeMap::new();
        let (shadowed, aliases) = super::standard::shadows(input);
        if aliases {
            return Self { maps };
        }
        let target = symbols("0.4");
        let bundled = rnl_view::bundled();
        for version in ["0.2", "0.3"] {
            for ((package, name), before) in symbols(version) {
                if shadowed.contains(&name) {
                    continue;
                }
                let Some(after) = target.get(&(package.clone(), name.clone())) else { continue };
                let Some(replacement) = mapping(&before, after) else { continue };
                let sources: Vec<_> = input
                    .sources
                    .iter()
                    .map(|(_, text)| text)
                    .filter(|s| scan_package_decl(s).is_some_and(|(p, _)| p == package))
                    .collect();
                if sources.is_empty() || sources.iter().any(|s| !bundled.files.values().any(|b| b == *s)) {
                    continue;
                }
                for level in &input.model.levels {
                    if level.packages.iter().any(|p| p.name == package && p.version == version) {
                        maps.insert((level.level, name.clone()), replacement.clone());
                    }
                }
            }
        }
        Self { maps }
    }

    pub(super) fn edit(
        &self,
        source: &str,
        record: &Record,
        member: &Member,
        index: usize,
        readings: &[usize],
    ) -> Result<Option<Edit>, String> {
        let segment = &member.segments[index];
        let Some(call) = segment.call.as_ref().filter(|c| {
            c.kw == "SET-SYMBOL" && c.params.as_ref().is_none_or(|p| p.named.is_empty() && p.positional.is_empty())
        }) else {
            return Ok(None);
        };
        let symbols: Vec<_> = segment
            .args
            .iter()
            .filter_map(|a| match &a.obj {
                ObjExpr::Ref(r) if r.prefix == '$' && r.path.is_empty() && r.sub.is_none() => {
                    r.name.as_ref().map(|s| format!("${s}"))
                }
                _ => None,
            })
            .collect();
        let [symbol] = symbols.as_slice() else { return Ok(None) };
        fn hosts(o: &ObjExpr) -> bool {
            match o {
                ObjExpr::Ref(r) => {
                    r.prefix == '#' && r.number.is_none() && r.path.is_empty() && r.sub.is_none() && r.places.is_none()
                }
                ObjExpr::Set(s) => {
                    !s.members.is_empty()
                        && s.members.iter().all(|m| {
                            m.binds.is_empty()
                                && m.segments.len() == 1
                                && m.segments[0].call.is_none()
                                && m.segments[0].args.iter().all(|a| hosts(&a.obj))
                        })
                }
                _ => false,
            }
        }
        let direct: Vec<_> =
            segment.args.iter().filter(|a| !matches!(&a.obj, ObjExpr::Ref(r) if r.prefix == '$')).collect();
        let known_hosts = if segment.piped {
            direct.is_empty()
                && index > 0
                && member.segments[index - 1].call.as_ref().is_some_and(|c| c.kw == "SELECT-INSTANCE")
        } else {
            !direct.is_empty() && direct.iter().all(|a| hosts(&a.obj))
        };
        if !known_hosts {
            return Ok(None);
        }
        let mut replacements = readings.iter().map(|i| self.maps.get(&(*i, symbol.clone())));
        let Some(Some(replacement)) = replacements.next() else { return Ok(None) };
        if !replacements.all(|r| r == Some(replacement)) {
            return Ok(None);
        }
        record_edit(source, record, call.start..call.end, replacement.clone(), "standard-terminal-map").map(Some)
    }
}
