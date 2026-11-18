//! Preserve ordinal standard-symbol bindings with explicit terminal maps (11.3).
use std::collections::{BTreeMap, BTreeSet};

use rnl_runtime::host::Record;
use rnl_runtime::interp::unit::scan_package_decl;
use rnl_runtime::syntax::{parse_record_features, Coord, Member, ObjExpr, Value};
use rnl_runtime::Interpretation;

use super::edit::{record_edit, Edit};

#[derive(Clone, PartialEq)]
struct Pin {
    x: f64,
    y: f64,
    direction: Option<String>,
    terminal: Value,
}
struct RawSymbol {
    pins: Vec<Pin>,
    parents: Vec<String>,
    valid: bool,
}

type Symbols = BTreeMap<(String, String), Vec<Pin>>;
type RawSymbols = BTreeMap<(String, String), RawSymbol>;

fn symbols(version: &str) -> Symbols {
    let mut raw: RawSymbols = BTreeMap::new();
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
                let Some(binding) = member.binds.iter().find(|b| b.prefix == '$' || b.prefix == '%') else {
                    continue;
                };
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
                let mut parents = Vec::new();
                let mut valid = true;
                for statement in &body.members {
                    for segment in &statement.segments {
                        let Some(call) = &segment.call else {
                            match segment.args.as_slice() {
                                [a] if !segment.piped => match &a.obj {
                                    ObjExpr::Ref(r)
                                        if (r.prefix == '$' || r.prefix == '%')
                                            && r.path.is_empty()
                                            && r.sub.is_none()
                                            && r.places.is_none() =>
                                    {
                                        if let Some(name) = &r.name {
                                            parents.push(format!("{}{name}", r.prefix));
                                        } else {
                                            valid = false;
                                        }
                                    }
                                    _ => valid = false,
                                },
                                _ => valid = false,
                            }
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
                raw.insert(
                    (package.clone(), format!("{}{}", binding.prefix, binding.name)),
                    RawSymbol { pins, parents, valid },
                );
            }
        }
    }
    fn resolve(key: &(String, String), raw: &RawSymbols, depth: usize) -> Option<Vec<Pin>> {
        if depth > 16 {
            return None;
        }
        let symbol = raw.get(key)?;
        if !symbol.valid {
            return None;
        }
        let mut all = Vec::new();
        for parent in &symbol.parents {
            all.extend(resolve(&(key.0.clone(), parent.clone()), raw, depth + 1)?);
        }
        all.extend(symbol.pins.iter().cloned());
        let mut seen = BTreeSet::new();
        for p in &all {
            if !seen.insert(super::rewrite::value(&p.terminal)) {
                return None;
            }
        }
        Some(all)
    }
    let mut out = Symbols::new();
    for key in raw.keys() {
        if !key.1.starts_with('$') {
            continue;
        }
        if let Some(pins) = resolve(key, &raw, 0).filter(|p| !p.is_empty()) {
            out.insert(key.clone(), pins);
        }
    }
    out
}

fn mapping(before: &[Pin], after: &[Pin], allow_direction_fallback: bool) -> Option<String> {
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
        let mut found: Vec<_> =
            after.iter().filter(|p| p.x == old.x && p.y == old.y && p.direction == old.direction).collect();
        if found.is_empty()
            && allow_direction_fallback
            && old.direction.is_some()
            && before.iter().filter(|p| p.direction == old.direction).count() == 1
        {
            found = after.iter().filter(|p| p.direction == old.direction).collect();
        }
        let [new] = found.as_slice() else { return None };
        let Value::Str(name) = &new.terminal else { return None };
        if name.is_empty() || name.chars().all(|c| c.is_ascii_digit()) || !names.insert(name.to_lowercase()) {
            return None;
        }
        pairs.push(format!("[{number}, {}]", rnl_runtime::problem::written_string(name)));
    }
    Some(format!("@SET-SYMBOL(terminals=[{}])", pairs.join(", ")))
}

/// The rule of the terminal maps given to instances whose terminals a DEVICE renames.
const RENAMED_RULE: &str = "device-terminal-map";

/// An instance that the reading of 0.3 draws with a symbol of a package while a DEVICE
/// gives its terminals names other than the adapter's (6.12). The pins of 0.3 bind terminal
/// numbers, so the names did not matter; those of 0.4 bind the adapter's names, which the instance
/// no longer has, so they would bind nothing (11.3). With the verified terminal map of its symbol,
/// the instance gets that symbol with the map, written after the records of its level; where a
/// statement of the file gives it the symbol, that statement gets the same map (`standard-terminal-map`).
/// Without a verified map, a notice says what is missing.
struct Renamed {
    level: usize,
    line: u32,
    /// `#Z1`, as a record writes it.
    instance: String,
    /// The symbol of 0.4.
    symbol: String,
    /// `#Z1 @SET-SYMBOL(terminals=[...]) $NJFET`, where the map is verified.
    statement: Option<String>,
}

fn renamed(input: &Interpretation, maps: &BTreeMap<(usize, String), String>, symbols: &Symbols) -> Vec<Renamed> {
    let doc = &input.doc;
    let mut out = Vec::new();
    for level in &input.model.levels {
        if level.language.as_str() >= "0.4" {
            continue;
        }
        let Some(lv) = doc.levels.get(level.level) else { continue };
        for object in &level.objects {
            let Some(symbol) = object.symbol.as_ref().filter(|s| {
                object.kind == "instance" && matches!(s.origin, rnl_runtime::model::SymbolOrigin::Package(_))
            }) else {
                continue;
            };
            let name = object.id.trim_start_matches('#');
            let Some(index) = lv.instances.iter().position(|i| doc.key(&i.name) == doc.key(name)) else { continue };
            let instance = &lv.instances[index];
            let Some(names) = level.terminal_names.get(index) else { continue };
            let target = super::legacy::renamed_03_symbol(symbol.name.trim_start_matches('$'))
                .map_or_else(|| symbol.name.clone(), |s| format!("${s}"));
            // the pins of the symbol of 0.4, each by the terminal name it binds
            let same = |a: &str, b: &str| if instance.case_sensitive { a == b } else { a.eq_ignore_ascii_case(b) };
            let binds = |pins: &Vec<Pin>| {
                pins.iter().all(|pin| match &pin.terminal {
                    Value::Str(name) => names.iter().flatten().any(|n| same(n, name)),
                    _ => true,
                })
            };
            let pins: Vec<_> = symbols.iter().filter(|((_, n), _)| *n == target).map(|(_, pins)| pins).collect();
            if pins.is_empty() || pins.iter().any(|p| binds(p)) {
                continue;
            }
            let reference = format!("#{}", rnl_runtime::model::quoted(&instance.name));
            let statement =
                maps.get(&(level.level, symbol.name.clone())).map(|map| format!("{reference} {map} {target}"));
            out.push(Renamed {
                level: level.level,
                line: instance.line,
                instance: reference,
                symbol: target,
                statement,
            });
        }
    }
    out
}

#[derive(Default)]
pub(super) struct Terminals {
    maps: BTreeMap<(usize, String), String>,
    renamed: Vec<Renamed>,
}
impl Terminals {
    pub(super) fn new(input: &Interpretation) -> Self {
        let mut maps = BTreeMap::new();
        let target = symbols("0.4");
        let (shadowed, aliases) = super::standard::shadows(input);
        if aliases {
            return Self { renamed: renamed(input, &maps, &target), maps };
        }
        let bundled = rnl_view::bundled();
        for version in ["0.2", "0.3"] {
            for ((package, name), before) in symbols(version) {
                let target_name = if version == "0.2" {
                    super::legacy::renamed(name.trim_start_matches('$'))
                        .map(|s| format!("${s}"))
                        .unwrap_or_else(|| name.clone())
                } else {
                    super::legacy::renamed_03_symbol(name.trim_start_matches('$'))
                        .map(|s| format!("${s}"))
                        .unwrap_or_else(|| name.clone())
                };
                if shadowed.contains(&name) || shadowed.contains(&target_name) {
                    continue;
                }
                let (after, before) = if version == "0.2" && package == "rnl-std-symbols" && name == "$OPAMP" {
                    let Some(after) = target.get(&("rnl-std-functional".into(), target_name)) else { continue };
                    let mirrored = before.into_iter().map(|p| Pin { y: -p.y, ..p }).collect::<Vec<_>>();
                    (after, mirrored)
                } else {
                    let Some(after) = target.get(&(package.clone(), target_name)) else { continue };
                    (after, before)
                };
                let allow_direction_fallback = version == "0.3" || (version == "0.2" && name == "$OPAMP");
                let Some(replacement) = mapping(&before, after, allow_direction_fallback) else { continue };
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
        Self { renamed: renamed(input, &maps, &target), maps }
    }

    /// For the levels written in the file `path`: a record, before the end of each, that gives the
    /// instances whose terminals a DEVICE renames their symbol with its terminal map; a notice for
    /// each such instance with no verified map.
    pub(super) fn renamed_edits(
        &self,
        doc: &rnl_runtime::host::HostDocument,
        path: &str,
        source: &str,
        notices: &mut Vec<super::Unconverted>,
    ) -> Vec<Edit> {
        let mut statements: BTreeMap<usize, Vec<&str>> = BTreeMap::new();
        for renamed in self.renamed.iter().filter(|r| doc.levels.get(r.level).is_some_and(|l| l.path == path)) {
            match &renamed.statement {
                Some(statement) => statements.entry(renamed.level).or_default().push(statement),
                None => notices.push(super::Unconverted {
                    rule: RENAMED_RULE.into(),
                    line: renamed.line,
                    before: String::new(),
                    reason: format!(
                        "a DEVICE renames the terminals of {}, and the pins of {} in 0.4 bind the adapter's default terminal names: give {} @SET-SYMBOL(terminals=[...]) {} that maps its pins to its terminals (6.12, 11.3)",
                        renamed.instance, renamed.symbol, renamed.instance, renamed.symbol
                    ),
                }),
            }
        }
        statements
            .into_iter()
            .filter_map(|(level, statements)| {
                let record = format!("@RNL {{\n  {}\n}}", statements.join(";\n  "));
                super::default_boxes::at_end(doc, level, path, source, &record, RENAMED_RULE)
            })
            .collect()
    }

    pub(super) fn edit(
        &self,
        source: &str,
        record: &Record,
        member: &Member,
        index: usize,
        readings: &[usize],
    ) -> Result<Option<Edit>, String> {
        set_symbol_edit(&self.maps, source, record, member, index, readings, "standard-terminal-map")
    }
}

/// The `SET-SYMBOL` of instances, given as host references or selected by `SELECT-INSTANCE`, with
/// the terminal map of its symbol in `maps` by the level it is read in, put in place of the bare
/// call, under the rule `rule`. A call with parameters of its own, a symbol drawn on other objects,
/// and a symbol the readings map differently are left alone.
pub(super) fn set_symbol_edit(
    maps: &BTreeMap<(usize, String), String>,
    source: &str,
    record: &Record,
    member: &Member,
    index: usize,
    readings: &[usize],
    rule: &str,
) -> Result<Option<Edit>, String> {
    {
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
        let mut replacements = readings.iter().map(|i| maps.get(&(*i, symbol.clone())));
        let Some(Some(replacement)) = replacements.next() else { return Ok(None) };
        if !replacements.all(|r| r == Some(replacement)) {
            return Ok(None);
        }
        record_edit(source, record, call.start..call.end, replacement.clone(), rule).map(Some)
    }
}
