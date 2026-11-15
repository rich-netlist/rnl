//! Files that use Analog Canvas's package analog-canvas-extensions 0.1–0.6, of the language 0.3.
//!
//! Its symbols that the standard series 0.4 has take their standard names; the others keep their
//! names in its version for the language 0.4, [`TARGET_VERSION`]. Each pin of the earlier versions
//! binds a terminal number, and each pin of a standard symbol and of the package for 0.4 binds a
//! terminal name, so the `SET-SYMBOL` of an instance gets the terminal map of [`TABLE`]. The
//! package is known by its name and version; only when a file gives its `path=` are its bytes
//! compared with those published, and a package that differs changes nothing.
use std::collections::{BTreeMap, BTreeSet};

use rnl_runtime::host::Record;
use rnl_runtime::interp::unit::scan_package_decl;
use rnl_runtime::syntax::{parse_record_features, Member, ObjExpr, Ref, Segment, Value};
use rnl_runtime::Interpretation;

use super::edit::{record_edit, Edit};

/// The package's name.
pub const PACKAGE: &str = "analog-canvas-extensions";

/// The version of the package for the language 0.4 that an upgraded file uses: the published one.
pub const TARGET_VERSION: &str = "0.7";

/// The published versions for the language 0.3, with the SHA-256 of each file.
pub const PUBLISHED: [(&str, &str); 6] = [
    ("0.1", "3738e1ce0c5d6fca3e13ad44faac8bd54f5eeb70577a22e412f5858f21f86711"),
    ("0.2", "b9da7bd01b85ec1108546770cb0a845a85627a2b4879de8fc7351006f19cde7e"),
    ("0.3", "d000fb9bfafae4b53c28e2293f40b6ab0c1bdc15e2b17d53ee502877262fc914"),
    ("0.4", "40843a19ccbe02f81e7cbb4b5d7acb351bf2d3a4e737cb9be1f7b5fe6f83ac4a"),
    ("0.5", "4b902f72322d88c74bbd92a760d16b0bc38232e413b9140cb7e039efab8be3a5"),
    ("0.6", "6b8af6bbc4538cc5ea845b45c4511027fd61865917839a37612c326e19905ead"),
];

/// A symbol or a definition of the package: its name in 0.1–0.6, its name in 0.4, the standard
/// package that has it (None: the package itself, version [`TARGET_VERSION`]), and the terminal
/// each drawn pin binds, by its number in 0.1–0.6 and by its name in 0.4.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Row {
    pub old: &'static str,
    pub new: &'static str,
    pub standard: Option<&'static str>,
    pub terminals: &'static [(u32, &'static str)],
}

const fn row(
    old: &'static str,
    new: &'static str,
    standard: Option<&'static str>,
    terminals: &'static [(u32, &'static str)],
) -> Row {
    Row { old, new, standard, terminals }
}

const PN: &[(u32, &str)] = &[(1, "p"), (2, "n")];
const AK: &[(u32, &str)] = &[(1, "a"), (2, "k")];
const DIFFERENTIAL: &[(u32, &str)] = &[(1, "inp"), (2, "inn"), (3, "outp"), (4, "outn")];
const OPAMP: &[(u32, &str)] = &[(1, "inp"), (2, "inn"), (5, "out")];
const GATE: &[(u32, &str)] = &[(1, "a"), (2, "b"), (3, "c"), (4, "d"), (5, "y")];
const BASIC: Option<&str> = Some("rnl-std-basic");
const SEMICONDUCTORS: Option<&str> = Some("rnl-std-semiconductors");
const FUNCTIONAL: Option<&str> = Some("rnl-std-functional");

/// Every name of the package in 0.1–0.6 (rnl#481). Each symbol that moves keeps the place and the
/// direction of each pin, at R0 with no shift; only its outline is the standard one. A terminal
/// that no pin draws, such as the controlling pair of an E or G card, stays undrawn.
pub const TABLE: [Row; 56] = [
    row("CANVAS_AMMETER", "AMMETER", BASIC, PN),
    row("CANVAS_CRYSTAL", "CRYSTAL", BASIC, &[(1, "n1"), (2, "n2")]),
    row("CANVAS_LED", "LED", SEMICONDUCTORS, AK),
    row("CANVAS_MICROPHONE", "MICROPHONE", BASIC, PN),
    row("CANVAS_PHOTORESISTOR", "PHOTORESISTOR", BASIC, PN),
    row("CANVAS_POLARIZED_CAPACITOR", "POLARIZED_CAPACITOR", BASIC, PN),
    row("CANVAS_POTENTIOMETER", "POTENTIOMETER", BASIC, &[(1, "n1"), (2, "nw"), (3, "n2")]),
    row("CANVAS_PULSE_VOLTAGE_SOURCE", "PULSE_VOLTAGE_SOURCE", BASIC, PN),
    row("CANVAS_PUT", "PUT", SEMICONDUCTORS, &[(1, "a"), (2, "g"), (3, "k")]),
    row("CANVAS_SCHOTTKY_DIODE", "SCHOTTKY_DIODE", SEMICONDUCTORS, AK),
    row("CANVAS_SHOCKLEY_DIODE", "SHOCKLEY_DIODE", SEMICONDUCTORS, AK),
    row("CANVAS_SINE_VOLTAGE_SOURCE", "SINE_VOLTAGE_SOURCE", BASIC, PN),
    row("CANVAS_SPEAKER", "SPEAKER", BASIC, PN),
    row("CANVAS_UJT", "UJT", SEMICONDUCTORS, &[(1, "e"), (2, "b1"), (3, "b2")]),
    row("CANVAS_CHOPPER", "CHOPPER_SYMBOL", FUNCTIONAL, DIFFERENTIAL),
    row("CANVAS_OPAMP_DIFFERENTIAL_CROSSED", "DIFFERENTIAL_OPAMP_TWISTED", FUNCTIONAL, DIFFERENTIAL),
    row("CANVAS_VCVS_DIAMOND", "CONTROLLED_VOLTAGE_SOURCE", BASIC, PN),
    row("CANVAS_CCVS_DIAMOND", "CONTROLLED_VOLTAGE_SOURCE", BASIC, PN),
    row("CANVAS_VCCS_DIAMOND", "CONTROLLED_CURRENT_SOURCE", BASIC, PN),
    row("CANVAS_CCCS_DIAMOND", "CONTROLLED_CURRENT_SOURCE", BASIC, PN),
    row("CANVAS_CURRENT_SOURCE_DIAMOND", "CONTROLLED_CURRENT_SOURCE", BASIC, PN),
    row("CANVAS_VCVS_CIRCLE", "VOLTAGE_SOURCE", BASIC, PN),
    row("CANVAS_CCVS_CIRCLE", "VOLTAGE_SOURCE", BASIC, PN),
    row("CANVAS_VCCS_CIRCLE", "CURRENT_SOURCE", BASIC, PN),
    row("CANVAS_CCCS_CIRCLE", "CURRENT_SOURCE", BASIC, PN),
    row("CANVAS_OPAMP_LETTERED", "CANVAS_OPAMP_LETTERED", None, OPAMP),
    row("CANVAS_OPAMP_WIDE", "CANVAS_OPAMP_WIDE", None, OPAMP),
    row("CANVAS_OPAMP_WIDE_LETTERED", "CANVAS_OPAMP_WIDE_LETTERED", None, OPAMP),
    row("CANVAS_COMPARATOR_UNMARKED", "CANVAS_COMPARATOR_UNMARKED", None, &[(1, "inp"), (2, "inn"), (3, "out")]),
    row("CANVAS_OPAMP_DIFFERENTIAL_LETTERED", "CANVAS_OPAMP_DIFFERENTIAL_LETTERED", None, DIFFERENTIAL),
    row("CANVAS_OPAMP_DIFFERENTIAL_CROSSED_LETTERED", "CANVAS_OPAMP_DIFFERENTIAL_CROSSED_LETTERED", None, DIFFERENTIAL),
    row("CANVAS_OPAMP_DIFFERENTIAL_WIDE", "CANVAS_OPAMP_DIFFERENTIAL_WIDE", None, DIFFERENTIAL),
    row("CANVAS_OPAMP_DIFFERENTIAL_WIDE_CROSSED", "CANVAS_OPAMP_DIFFERENTIAL_WIDE_CROSSED", None, DIFFERENTIAL),
    row("CANVAS_OPAMP_DIFFERENTIAL_WIDE_LETTERED", "CANVAS_OPAMP_DIFFERENTIAL_WIDE_LETTERED", None, DIFFERENTIAL),
    row(
        "CANVAS_OPAMP_DIFFERENTIAL_WIDE_CROSSED_LETTERED",
        "CANVAS_OPAMP_DIFFERENTIAL_WIDE_CROSSED_LETTERED",
        None,
        DIFFERENTIAL,
    ),
    row("CANVAS_VOLTAGE_AMPLIFIER_LETTERED", "CANVAS_VOLTAGE_AMPLIFIER_LETTERED", None, &[(1, "in"), (2, "out")]),
    row("CANVAS_INDUCTOR_LARGE", "CANVAS_INDUCTOR_LARGE", None, PN),
    row("CANVAS_SIMPLE_SWITCH", "CANVAS_SIMPLE_SWITCH", None, PN),
    // a figure in 0.7, as the standard switch is: its pins bind no terminal
    row("CANVAS_SIMPLE_SPDT_SWITCH", "CANVAS_SIMPLE_SPDT_SWITCH", None, &[]),
    // drawn on a usage of $IO_PORT, whose role p its pin binds by name in 0.7 and by number before
    row("CANVAS_PORT_FILLED", "CANVAS_PORT_FILLED", None, &[]),
    row("CANVAS_AND_4INPUT", "CANVAS_AND_4INPUT", None, GATE),
    row("CANVAS_NAND_4INPUT", "CANVAS_NAND_4INPUT", None, GATE),
    row("CANVAS_OR_4INPUT", "CANVAS_OR_4INPUT", None, GATE),
    row("CANVAS_NOR_4INPUT", "CANVAS_NOR_4INPUT", None, GATE),
    row("CANVAS_XOR_4INPUT", "CANVAS_XOR_4INPUT", None, GATE),
    row("CANVAS_XNOR_4INPUT", "CANVAS_XNOR_4INPUT", None, GATE),
    row("CANVAS_NAME", "CANVAS_NAME", None, &[]),
    row("CANVAS_LABEL", "CANVAS_LABEL", None, &[]),
    row("CANVAS_TYPESETTING", "CANVAS_TYPESETTING", None, &[]),
    row("CANVAS_PARAMETER_TEXT", "CANVAS_PARAMETER_TEXT", None, &[]),
    row("CANVAS_TYPOGRAPHY", "CANVAS_TYPOGRAPHY", None, &[]),
    row("CANVAS_PARAMETER", "CANVAS_PARAMETER", None, &[]),
    row("CANVAS_UNNAMED", "CANVAS_UNNAMED", None, &[]),
    row("CANVAS_VARIANT", "CANVAS_VARIANT", None, &[]),
    row("CANVAS_CELL_NAME", "CANVAS_CELL_NAME", None, &[]),
    row("CANVAS_TOP", "CANVAS_TOP", None, &[]),
];

/// The table as Markdown, one row a name, for the upgrade's README.
pub fn table_markdown() -> String {
    let mut out = String::from("| 0.1–0.6 | 0.4 | Package | Terminals |\n|---|---|---|---|\n");
    for r in TABLE {
        let package = r.standard.map_or_else(|| format!("{PACKAGE} {TARGET_VERSION}"), str::to_string);
        let terminals = if r.terminals.is_empty() {
            "as before".to_string()
        } else {
            r.terminals.iter().map(|(n, name)| format!("{n}→{name}")).collect::<Vec<_>>().join(", ")
        };
        out.push_str(&format!("| `${}` | `${}` | {package} | {terminals} |\n", r.old, r.new));
    }
    out
}

fn known(version: &str) -> bool {
    PUBLISHED.iter().any(|(v, _)| *v == version)
}

/// Whether the file group gives the package by `path=` in a USE-PACKAGE of a known version.
fn by_path(input: &Interpretation) -> bool {
    input.doc.levels.iter().flat_map(|l| &l.records).any(|record| {
        let Some(body) = parse_record_features(&record.text, true, false).body else { return false };
        let mut found = false;
        visit(&body.members, &mut |m| {
            found |= m.segments.iter().any(|s| package_use(s).is_some_and(|(_, path)| path));
        });
        found
    })
}

fn visit(members: &[std::rc::Rc<Member>], f: &mut impl FnMut(&Member)) {
    for member in members {
        f(member);
        for segment in &member.segments {
            for arg in &segment.args {
                if let ObjExpr::Set(set) = &arg.obj {
                    visit(&set.members, f);
                }
            }
        }
    }
}

/// A USE-PACKAGE of the package in a known version, not inline: its version, and whether it
/// gives a `path=`.
fn package_use(segment: &Segment) -> Option<(String, bool)> {
    let call = segment.call.as_ref().filter(|c| c.kw == "USE-PACKAGE")?;
    if segment.args.iter().any(|a| matches!(a.obj, ObjExpr::Set(_))) {
        return None;
    }
    let params = call.params.as_ref()?;
    let (Some(Value::Str(name)), Some(Value::Str(version))) = (params.get("name"), params.get("version")) else {
        return None;
    };
    (name == PACKAGE && known(version)).then(|| (version.clone(), params.get("path").is_some()))
}

/// Whether the package's files the file group read are those published: always when it names
/// the package alone, which a package directory gives; by their bytes when it gives a `path=`.
pub(super) fn recognized(input: &Interpretation) -> bool {
    if !by_path(input) {
        return true;
    }
    input.sources.iter().all(|(_, text)| match scan_package_decl(text) {
        Some((name, version)) if name == PACKAGE => {
            let hash = rnl_runtime::numbering::hex(&rnl_runtime::numbering::sha256(text.as_bytes()));
            PUBLISHED.iter().any(|(v, h)| *v == version && *h == hash)
        }
        _ => true,
    })
}

#[derive(Default)]
pub(super) struct Canvas {
    /// By the level and the name without `$`: its name in 0.4, and the standard package it moves to.
    names: BTreeMap<(usize, String), (&'static str, Option<&'static str>)>,
    /// By the level and the `$` name: the `SET-SYMBOL` with its terminal map.
    maps: BTreeMap<(usize, String), String>,
    /// The levels that use the package in a version of 0.1–0.6.
    levels: BTreeSet<usize>,
    /// The package was given by a path to a file other than the published one.
    changed: bool,
}

impl Canvas {
    pub(super) fn new(input: &Interpretation) -> Self {
        let mut out = Self::default();
        let levels: BTreeSet<usize> = input
            .model
            .levels
            .iter()
            .filter(|l| l.packages.iter().any(|p| p.name == PACKAGE && known(&p.version)))
            .map(|l| l.level)
            .collect();
        if levels.is_empty() {
            return out;
        }
        if !recognized(input) {
            out.changed = true;
            return out;
        }
        // a name the file group defines itself, outside the package and the standard ones, is its own
        let bundled = rnl_view::bundled();
        let mut own = BTreeSet::new();
        let mut aliases = !input.imported_aliases.is_empty();
        for (path, text) in &input.sources {
            if bundled.files.values().any(|b| b == text) || scan_package_decl(text).is_some_and(|(n, _)| n == PACKAGE) {
                continue;
            }
            for record in rnl_runtime::records::scan_run(
                &rnl_runtime::records::package_lines(text),
                path,
                &mut rnl_runtime::diag::Sink::default(),
            ) {
                if let Some(body) = parse_record_features(&record.text, true, false).body {
                    for member in &body.members {
                        aliases |= super::standard::bound(member, &mut own);
                    }
                }
            }
        }
        // aliases can change what a name stands for: such a file needs a migration aware of them
        if aliases {
            return out;
        }
        for &level in &levels {
            for r in TABLE {
                if own.contains(&format!("${}", r.old)) || own.contains(&format!("${}", r.new)) {
                    continue;
                }
                if r.old != r.new {
                    out.names.insert((level, r.old.to_string()), (r.new, r.standard));
                }
                if !r.terminals.is_empty() {
                    let pairs: Vec<String> = r
                        .terminals
                        .iter()
                        .map(|(n, name)| format!("[{n}, {}]", rnl_runtime::problem::written_string(name)))
                        .collect();
                    out.maps.insert(
                        (level, format!("${}", r.old)),
                        format!("@SET-SYMBOL(terminals=[{}])", pairs.join(", ")),
                    );
                }
            }
        }
        out.levels = levels;
        out
    }

    /// Whether the file gives the package by a path to a file that is not the published one.
    pub(super) fn changed(&self) -> bool {
        self.changed
    }

    pub(super) fn reference(&self, readings: &[usize], r: &Ref) -> Option<(&'static str, Option<&'static str>)> {
        if r.prefix != '$' || !r.path.is_empty() || r.sub.is_some() || r.places.is_some() {
            return None;
        }
        let name = r.name.as_ref()?;
        let mut values = readings.iter().map(|level| self.names.get(&(*level, name.clone())).copied());
        let first = values.next()??;
        values.all(|v| v == Some(first)).then_some(first)
    }

    pub(super) fn rename(
        &self,
        source: &str,
        record: &Record,
        readings: &[usize],
        r: &Ref,
    ) -> Result<Option<Edit>, String> {
        let Some((new, _)) = self.reference(readings, r) else { return Ok(None) };
        record_edit(
            source,
            record,
            r.start..r.end,
            format!("${}", rnl_runtime::problem::written_name(new)),
            "canvas-name",
        )
        .map(Some)
    }

    /// The pins that a symbol of the package draws on a usage, by role and by the pin's name in
    /// [`TARGET_VERSION`]: the role p of `$IO_PORT` on `$CANVAS_PORT_FILLED`, at its pin `%pin`.
    pub(super) fn usage_pins(
        &self,
        level: usize,
        package: &str,
        symbol: &str,
    ) -> Option<&'static [(&'static str, &'static str)]> {
        (package == PACKAGE && self.levels.contains(&level) && symbol == "$CANVAS_PORT_FILLED")
            .then_some(&[("p", "pin")])
    }

    pub(super) fn maps(&self) -> &BTreeMap<(usize, String), String> {
        &self.maps
    }

    /// The USE-PACKAGE of the package in a level that uses it: the version for 0.4, without the
    /// path to a file of 0.1–0.6. The second is whether a path was left out.
    pub(super) fn package(
        &self,
        source: &str,
        record: &Record,
        segment: &Segment,
        readings: &[usize],
    ) -> Result<Option<(Edit, bool)>, String> {
        if readings.is_empty() || !readings.iter().all(|l| self.levels.contains(l)) {
            return Ok(None);
        }
        let Some((_, path)) = package_use(segment) else { return Ok(None) };
        let call = segment.call.as_ref().expect("a USE-PACKAGE");
        let params = call.params.as_ref().expect("its parameters");
        let mut values = params.positional.iter().map(super::rewrite::value).collect::<Vec<_>>();
        for n in &params.named {
            let key = if n.quoted { rnl_runtime::problem::written_string(&n.name) } else { n.name.clone() };
            match n.name.as_str() {
                "version" => values.push(format!("version={}", rnl_runtime::problem::written_string(TARGET_VERSION))),
                "path" => {}
                _ => values.push(format!("{key}={}", super::rewrite::value(&n.value))),
            }
        }
        let edit = record_edit(
            source,
            record,
            call.start..call.end,
            format!("@USE-PACKAGE({})", values.join(", ")),
            "canvas-package",
        )?;
        Ok(Some((edit, path)))
    }

    /// The standard packages that the names a record renames move to.
    pub(super) fn standard_packages(&self, readings: &[usize], record: &Record) -> BTreeSet<&'static str> {
        let mut out = BTreeSet::new();
        let Some(body) = parse_record_features(&record.text, true, false).body else { return out };
        for member in &body.members {
            let mut refs = Vec::new();
            rnl_runtime::syntax::walk_refs(member, &mut refs);
            for r in refs {
                if let rnl_runtime::syntax::RefOrHole::Ref(r) = r {
                    if let Some((_, Some(package))) = self.reference(readings, r) {
                        out.insert(package);
                    }
                }
            }
        }
        out
    }
}

/// Whether a member names the package by a path to a file that is not the published one.
pub(super) fn names_changed_package(member: &Member) -> bool {
    member.segments.iter().any(|s| package_use(s).is_some_and(|(_, path)| path))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The table of the README of the upgrade is [`table_markdown`]; `RNL_BLESS=1` writes it.
    #[test]
    fn the_readme_has_the_table() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/upgrade/README.md");
        let text = std::fs::read_to_string(&path).unwrap().replace("\r\n", "\n");
        let (begin, end) = ("<!-- canvas-table:begin -->\n", "<!-- canvas-table:end -->");
        let start = text.find(begin).expect("begin marker") + begin.len();
        let stop = text.find(end).expect("end marker");
        let table = table_markdown();
        if std::env::var_os("RNL_BLESS").is_some() {
            std::fs::write(&path, format!("{}{table}{}", &text[..start], &text[stop..])).unwrap();
        } else {
            assert_eq!(&text[start..stop], table, "run with RNL_BLESS=1 to write the table");
        }
    }
}
