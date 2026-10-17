//! The catalog of the packages the core has: the symbols, ordinary definitions and sets of the
//! bundled packages, the standard series and `analog-canvas-extensions`, and of the frontend's own
//! packages, for a frontend that maps its parts onto them, as Analog Canvas does. Each package file
//! is read through the runtime as `rnl check PACKAGE.rnl` reads it, checked on its own (2.7), so a
//! symbol's pins and box are those the drawings have.

use std::collections::HashMap;

use rnl_runtime::host::file_name;
use rnl_runtime::interp::catalog::{package_contents, version_key, DefinedName, PackageContents, PackageSymbol};
use rnl_runtime::interp::obj::TermKey;
use rnl_runtime::interp::unit::scan_package_decl;
use rnl_runtime::source::Provider;
use rnl_runtime::Options;
use serde::Serialize;
use serde_json::Value;

use crate::snapshot::lit;
use crate::Workspace;

/// Which packages a catalog lists, and how they are read.
#[derive(Clone, Debug, Default)]
pub struct CatalogOptions {
    /// Only the packages of these names; every package when None.
    pub packages: Option<Vec<String>>,
    /// Every version of each package, not only the newest.
    pub all_versions: bool,
    /// The runtime's options each package file is checked with, as `rnl check` checks it: with the
    /// standard package of its language version unless `std` is false (16.3).
    pub runtime: Options,
}

/// The catalog of the packages a workspace has, the bundled ones among them: their symbols,
/// ordinary definitions and sets.
#[derive(Serialize, Debug, Clone, Default, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct Catalog {
    /// The packages listed, by name, then by version.
    pub packages: Vec<CatalogPackage>,
    /// Their symbols, package by package as `packages` lists them, each package's in the order of
    /// its file; so are the definitions and the sets.
    pub symbols: Vec<CatalogSymbol>,
    pub definitions: Vec<CatalogDefinition>,
    pub sets: Vec<CatalogSet>,
}

/// A package file (2.7).
#[derive(Serialize, Debug, Clone, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct CatalogPackage {
    /// As its PACKAGE says.
    pub name: String,
    pub version: String,
    /// The language version its file declares, or the one a file without it is read as (16.1).
    pub language: String,
    /// The name of its file, such as `rnl-std-basic.rnl`.
    pub file: String,
    /// The packages its header record references with USE-PACKAGE, by name, in order (16.2).
    pub references: Vec<String>,
    /// A package of the standard series: `rnl-std` or `rnl-std-*` (16.3).
    pub series: bool,
    /// The standard package, `rnl-std`, used implicitly at the root (16.3).
    pub implicit: bool,
}

/// A symbol of a package (9.2), in its own coordinates, y up.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct CatalogSymbol {
    /// Such as `$RESISTOR`.
    pub symbol: String,
    /// The package that defines it, its version and the name of its file.
    pub package: String,
    pub version: String,
    pub file: String,
    /// The definition whose usages a rule of the packages listed draws with it, a rule written
    /// `# @SELECT-USAGE $DEF => @SET-SYMBOL $SYM`, such as `$GROUND` for `$GROUND_SYMBOL`; null when
    /// none does.
    pub definition: Option<String>,
    /// Its pins, one for each terminal, in terminal order: those by number, then those by name in
    /// the order written.
    pub pins: Vec<CatalogPin>,
    /// x1, y1, x2, y2: the box the symbol writes, or the extent of its graphics (9.2.3), as the
    /// drawings have it.
    #[serde(rename = "box")]
    pub bbox: [f64; 4],
    /// Its labels, the texts it writes with a key (13.3), in the order written.
    pub labels: Vec<CatalogLabel>,
}

/// The pin of a terminal (9.2.4).
#[derive(Serialize, Debug, Clone, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct CatalogPin {
    /// As the symbol writes it: a number, or a name.
    pub terminal: Terminal,
    pub at: [f64; 2],
    /// The exit direction: "right", "up", "left" or "down"; null when the pin writes none.
    #[cfg_attr(test, ts(type = "\"right\" | \"up\" | \"left\" | \"down\" | null"))]
    pub direction: Option<&'static str>,
}

/// A terminal as a pin writes it (9.2.4): a number, or a name.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(untagged)]
pub enum Terminal {
    Number(u32),
    Name(String),
}

/// A text a symbol writes with a key (13.3).
#[derive(Serialize, Debug, Clone, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct CatalogLabel {
    /// Such as "name" or "value".
    pub key: String,
    /// As written, before `${…}` is read (13.6), such as "${name}"; null when the label writes none.
    pub text: Option<String>,
    /// Where the symbol puts it; null when it does not.
    pub at: Option<[f64; 2]>,
    /// As written, or as the model draws a label that writes none: "start" (13.3).
    pub halign: String,
    /// As written, or "baseline".
    pub valign: String,
}

/// An ordinary definition of a package (9.4).
#[derive(Serialize, Debug, Clone, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct CatalogDefinition {
    /// Such as `$GROUND`.
    pub definition: String,
    pub package: String,
    pub version: String,
    pub file: String,
    /// Its roles as `roles=` writes them: a name, `"{name}"` for a set role, or `[name, kinds]`,
    /// the kinds a name or a list of names.
    #[cfg_attr(test, ts(type = "Array<string | [string, string | Array<string>]>"))]
    pub roles: Vec<Value>,
    /// Its parameters, each `[name, default]`; the default is null when it has none.
    #[cfg_attr(test, ts(type = "Array<[string, string | number | boolean | Array<unknown> | null]>"))]
    pub parameters: Vec<(String, Value)>,
    /// The symbol a rule of the packages listed draws its usages with, the other side of a
    /// symbol's `definition`, such as `$GROUND_SYMBOL` for `$GROUND`; null when none does.
    pub symbol: Option<String>,
}

/// A set of a package (5.2), such as one the fill edits (10.5).
#[derive(Serialize, Debug, Clone, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct CatalogSet {
    /// Such as `$GROUND_NET_SET`.
    pub set: String,
    pub package: String,
    pub version: String,
    pub file: String,
}

/// The catalog of the packages of a workspace, which has the bundled ones.
pub(crate) fn catalog(ws: &Workspace, options: &CatalogOptions) -> Catalog {
    let mut read: Vec<(PackageContents, String)> = listed(ws, options)
        .iter()
        .filter_map(|path| Some((package_contents(path, ws, &options.runtime)?, file_name(path).to_string())))
        .collect();
    read.sort_by(|(a, _), (b, _)| {
        let key = |c: &PackageContents| (c.name.clone(), version_key(&c.version), c.version.clone());
        key(a).cmp(&key(b))
    });
    // what the rules draw with what, across the packages listed; of several rules, the first
    let mut definition_of: HashMap<&DefinedName, &str> = HashMap::new();
    let mut symbol_of: HashMap<&DefinedName, &str> = HashMap::new();
    for (c, _) in &read {
        for rule in &c.usage_symbols {
            definition_of.entry(&rule.symbol).or_insert(&rule.definition.name);
            symbol_of.entry(&rule.definition).or_insert(&rule.symbol.name);
        }
    }
    let mut out = Catalog::default();
    for (c, file) in &read {
        let named =
            |name: &str| DefinedName { name: name.to_string(), package: c.name.clone(), version: c.version.clone() };
        out.packages.push(CatalogPackage {
            name: c.name.clone(),
            version: c.version.clone(),
            language: c.language.clone(),
            file: file.clone(),
            references: c.references.clone(),
            series: c.name == "rnl-std" || c.name.starts_with("rnl-std-"),
            implicit: c.name == "rnl-std",
        });
        for s in &c.symbols {
            out.symbols.push(CatalogSymbol {
                symbol: s.name.clone(),
                package: c.name.clone(),
                version: c.version.clone(),
                file: file.clone(),
                definition: definition_of.get(&named(&s.name)).map(|d| d.to_string()),
                pins: pins(s),
                bbox: s.view.bbox,
                labels: s
                    .labels
                    .iter()
                    .map(|l| CatalogLabel {
                        key: l.key.clone(),
                        text: l.text.clone(),
                        at: l.at.map(|p| [p.0, p.1]),
                        halign: l.halign.clone().unwrap_or_else(|| "start".into()),
                        valign: l.valign.clone().unwrap_or_else(|| "baseline".into()),
                    })
                    .collect(),
            });
        }
        for d in &c.definitions {
            out.definitions.push(CatalogDefinition {
                definition: d.name.clone(),
                package: c.name.clone(),
                version: c.version.clone(),
                file: file.clone(),
                roles: d.roles.iter().map(lit).collect(),
                parameters: d
                    .parameters
                    .iter()
                    .map(|(n, v)| (n.clone(), v.as_ref().map_or(Value::Null, lit)))
                    .collect(),
                symbol: symbol_of.get(&named(&d.name)).map(|s| s.to_string()),
            });
        }
        for s in &c.sets {
            out.sets.push(CatalogSet {
                set: s.clone(),
                package: c.name.clone(),
                version: c.version.clone(),
                file: file.clone(),
            });
        }
    }
    out
}

/// The package files a catalog lists: of each name and version the first the package directories
/// have, the frontend's before the bundled ones, as the runtime finds a package by name (16.2); of
/// each name the newest version, by the runtime's order of versions, or every version with
/// `all_versions`; and only the names `packages` gives, when it gives them.
fn listed(ws: &Workspace, options: &CatalogOptions) -> Vec<String> {
    let mut found: Vec<(String, String, String)> = Vec::new();
    for path in ws.package_files() {
        let Some((name, version)) = ws.read(&path).as_deref().and_then(scan_package_decl) else { continue };
        if options.packages.as_ref().is_some_and(|names| !names.contains(&name)) {
            continue;
        }
        if !found.iter().any(|(n, v, _)| *n == name && *v == version) {
            found.push((name, version, path));
        }
    }
    if !options.all_versions {
        let newest = |name: &str| found.iter().filter(|f| f.0 == name).map(|f| version_key(&f.1)).max();
        // of versions equally new, the first found
        let mut kept: Vec<&(String, String, String)> = Vec::new();
        for f in &found {
            if Some(version_key(&f.1)) == newest(&f.0) && !kept.iter().any(|k| k.0 == f.0) {
                kept.push(f);
            }
        }
        return kept.into_iter().map(|f| f.2.clone()).collect();
    }
    found.into_iter().map(|f| f.2).collect()
}

/// The pins of a symbol, one for each terminal, in terminal order: by number, then by name in the
/// order written.
fn pins(s: &PackageSymbol) -> Vec<CatalogPin> {
    let mut out = Vec::new();
    for (pin, terminals) in s.view.pins.iter().zip(&s.terminals) {
        for t in terminals {
            let terminal = match t {
                TermKey::Num(n) => Terminal::Number(*n),
                TermKey::Name(n) => Terminal::Name(n.clone()),
            };
            out.push(CatalogPin { terminal, at: [pin.pos.0, pin.pos.1], direction: direction(pin.dir) });
        }
    }
    out.sort_by_key(|p| match p.terminal {
        Terminal::Number(n) => (0, n),
        Terminal::Name(_) => (1, 0),
    });
    out
}

/// An exit direction in degrees as a pin writes it.
fn direction(degrees: Option<u16>) -> Option<&'static str> {
    Some(match degrees? {
        0 => "right",
        90 => "up",
        180 => "left",
        270 => "down",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MemoryProvider;
    use serde_json::json;

    fn default() -> Catalog {
        crate::catalog(&MemoryProvider::new(), &CatalogOptions::default())
    }

    fn every_version() -> Catalog {
        crate::catalog(&MemoryProvider::new(), &CatalogOptions { all_versions: true, ..CatalogOptions::default() })
    }

    fn symbol<'a>(c: &'a Catalog, name: &str) -> &'a CatalogSymbol {
        c.symbols.iter().find(|s| s.symbol == name).unwrap_or_else(|| panic!("{name} is not listed"))
    }

    fn definition<'a>(c: &'a Catalog, name: &str) -> &'a CatalogDefinition {
        c.definitions.iter().find(|d| d.definition == name).unwrap_or_else(|| panic!("{name} is not listed"))
    }

    /// The names and versions of the packages listed.
    fn packages(c: &Catalog) -> Vec<(&str, &str)> {
        c.packages.iter().map(|p| (p.name.as_str(), p.version.as_str())).collect()
    }

    fn keys(v: &Value) -> Vec<&str> {
        v.as_object().unwrap().keys().map(String::as_str).collect()
    }

    #[test]
    fn a_symbol_comes_with_its_pins_box_and_labels() {
        let c = default();
        let r = symbol(&c, "$RESISTOR");
        assert_eq!(
            (r.package.as_str(), r.version.as_str(), r.file.as_str()),
            ("rnl-std-basic", "0.3", "rnl-std-basic.rnl")
        );
        let pins: Vec<(Terminal, [f64; 2], Option<&str>)> =
            r.pins.iter().map(|p| (p.terminal.clone(), p.at, p.direction)).collect();
        assert_eq!(
            pins,
            [(Terminal::Number(1), [0.0, 2.0], Some("up")), (Terminal::Number(2), [0.0, -2.0], Some("down"))]
        );
        assert_eq!(r.bbox, [-1.0, -2.4, 1.0, 2.4]);
        let keys: Vec<&str> = r.labels.iter().map(|l| l.key.as_str()).collect();
        assert_eq!(keys, ["name", "value"]);
        // no rule draws the usages of a definition with it
        assert_eq!(r.definition, None);
        // as JSON, a terminal by number is a number
        let v = serde_json::to_value(r).unwrap();
        assert_eq!(v["pins"][1], json!({"terminal": 2, "at": [0.0, -2.0], "direction": "down"}));
        assert_eq!(v["box"], json!([-1.0, -2.4, 1.0, 2.4]));
        assert_eq!(
            v["labels"][0],
            json!({"key": "name", "text": "${name}", "at": [0.95, 0.25], "halign": "start", "valign": "baseline"})
        );
        // the pins in terminal order, not in the order written: $TRANSFORMER writes 2, 1, 4, 3
        let t = symbol(&c, "$TRANSFORMER");
        let order: Vec<Terminal> = t.pins.iter().map(|p| p.terminal.clone()).collect();
        assert_eq!(order, [1, 2, 3, 4].map(Terminal::Number));
        assert_eq!(t.pins[0].at, [2.0, 1.0]);
    }

    #[test]
    fn a_symbol_is_what_the_drawings_have() {
        // the box and the pins of the resistor an instance is drawn with
        let text = "* t\n* @RNL(type=\"header\") { @VERSION(\"0.3\")}\nR1 a 0 1k\n* @RNL { #R1 @PLACE(0, 0) }\n.end\n";
        let files = MemoryProvider::new().with("/w/t.sp", text);
        let r = crate::interpret("/w/t.sp", None, &files, &Options::default()).unwrap();
        let drawn = r.model.levels[0].symbols.iter().find(|s| s.name == "$RESISTOR").unwrap();
        let c = default();
        let listed = symbol(&c, "$RESISTOR");
        assert_eq!(listed.bbox, drawn.bbox);
        let pins: Vec<([f64; 2], Option<&str>)> =
            drawn.pins.iter().map(|p| ([p.pos.0, p.pos.1], direction(p.dir))).collect();
        assert_eq!(listed.pins.iter().map(|p| (p.at, p.direction)).collect::<Vec<_>>(), pins);
    }

    #[test]
    fn a_rule_links_a_definition_and_its_symbol() {
        let c = default();
        assert_eq!(symbol(&c, "$GROUND_SYMBOL").definition.as_deref(), Some("$GROUND"));
        let ground = definition(&c, "$GROUND");
        assert_eq!((ground.package.as_str(), ground.version.as_str()), ("rnl-std-basic", "0.3"));
        assert_eq!(ground.symbol.as_deref(), Some("$GROUND_SYMBOL"));
        // the roles as written
        assert_eq!(serde_json::to_value(&ground.roles).unwrap(), json!([["p", ["net", "terminal"]]]));
        assert_eq!(serde_json::to_value(&definition(&c, "$NO_CONNECT").roles).unwrap(), json!([["t", "terminal"]]));
        assert_eq!(serde_json::to_value(&definition(&c, "$MATCHED").roles).unwrap(), json!(["{members}"]));
        // the parameters with their defaults; no rule draws a stub
        let stub = definition(&c, "$GROUND_STUB");
        assert_eq!(serde_json::to_value(&stub.parameters).unwrap(), json!([["gap", 2.0]]));
        assert_eq!(stub.symbol, None);
        // a rule of another form draws $NAMED_GROUND_SYMBOL, through the nets of a set
        assert_eq!(symbol(&c, "$NAMED_GROUND_SYMBOL").definition, None);
        // the sets of the fill
        let fill: Vec<&str> = c.sets.iter().filter(|s| s.package == "rnl-std-fill").map(|s| s.set.as_str()).collect();
        assert_eq!(fill[..3], ["$GROUND_NET_SET", "$SUPPLY_NET_SET", "$PORT_FILL_SET"]);
    }

    #[test]
    fn the_inputs_of_the_gates_with_four_inputs() {
        let c = default();
        let inputs = |name: &str| -> Vec<[f64; 2]> {
            let s = symbol(&c, name);
            s.pins.iter().filter(|p| matches!(p.terminal, Terminal::Number(1..=4))).map(|p| p.at).collect()
        };
        assert_eq!(symbol(&c, "$AND_4INPUT").package, "rnl-std-logic");
        assert_eq!(inputs("$AND_4INPUT"), [[-3.0, 2.0], [-3.0, 1.0], [-3.0, -1.0], [-3.0, -2.0]]);
        // Analog Canvas's, off the integer points
        let canvas = symbol(&c, "$CANVAS_AND_4INPUT");
        assert_eq!((canvas.package.as_str(), canvas.version.as_str()), ("analog-canvas-extensions", "0.1"));
        assert_eq!(inputs("$CANVAS_AND_4INPUT"), [[-3.0, 1.2], [-3.0, 0.4], [-3.0, -0.4], [-3.0, -1.2]]);
    }

    #[test]
    fn of_each_name_the_newest_version_or_every_version() {
        let c = default();
        let listed = packages(&c);
        assert!(listed.contains(&("rnl-std", "0.3")) && !listed.contains(&("rnl-std", "0.2")), "{listed:?}");
        // one version of each name, rnl-zh's 0.3; a name only the series 0.2 has keeps its version
        let mut names: Vec<&str> = listed.iter().map(|p| p.0).collect();
        names.dedup();
        assert_eq!(names.len(), listed.len(), "{listed:?}");
        assert!(listed.contains(&("rnl-zh", "0.3")) && listed.contains(&("rnl-std-virtual", "0.2")), "{listed:?}");
        assert!(listed.contains(&("analog-canvas-extensions", "0.1")), "{listed:?}");
        // the 16 packages of the series 0.3 and rnl-zh, rnl-std-symbols and rnl-std-virtual, Analog Canvas's
        assert_eq!(listed.len(), 17 + 2 + 1);
        // by name, then by version
        let mut sorted = listed.clone();
        sorted.sort();
        assert_eq!(listed, sorted);
        let std = c.packages.iter().find(|p| p.name == "rnl-std").unwrap();
        let want = CatalogPackage {
            name: "rnl-std".into(),
            version: "0.3".into(),
            language: "0.3".into(),
            file: "rnl-std.rnl".into(),
            references: ["rnl-std-core", "rnl-std-fill", "rnl-std-semantics", "rnl-std-layout"]
                .map(String::from)
                .to_vec(),
            series: true,
            implicit: true,
        };
        assert_eq!(*std, want);
        let canvas = c.packages.iter().find(|p| p.name == "analog-canvas-extensions").unwrap();
        assert_eq!((canvas.series, canvas.implicit, canvas.language.as_str()), (false, false, "0.3"));

        // every version: both series
        let all = every_version();
        let listed = packages(&all);
        assert_eq!(listed.len(), crate::BUNDLED.len());
        let at = |p: (&str, &str)| listed.iter().position(|q| *q == p).unwrap();
        assert_eq!(at(("rnl-std", "0.2")) + 1, at(("rnl-std", "0.3")));
        assert_eq!(all.packages[at(("rnl-std", "0.2"))].language, "0.2");
        // a name of the series 0.2 for a symbol, and one of the series 0.3 for a definition
        let ground: Vec<(&str, &str)> = all
            .symbols
            .iter()
            .filter(|s| s.symbol == "$GROUND")
            .map(|s| (s.package.as_str(), s.version.as_str()))
            .collect();
        assert_eq!(ground, [("rnl-std-virtual", "0.2")]);
        assert_eq!(definition(&all, "$GROUND").version, "0.3");
        // the rules of each series link its own: $GND of the series 0.2 is drawn with its $GROUND
        assert_eq!(symbol(&all, "$GROUND").definition.as_deref(), Some("$GND"));
        assert_eq!(symbol(&all, "$GROUND_SYMBOL").definition.as_deref(), Some("$GROUND"));
    }

    #[test]
    fn packages_keeps_only_the_names_it_gives() {
        let names = ["analog-canvas-extensions", "rnl-std"].map(String::from).to_vec();
        let options = CatalogOptions { packages: Some(names), ..CatalogOptions::default() };
        let c = crate::catalog(&MemoryProvider::new(), &options);
        assert_eq!(packages(&c), [("analog-canvas-extensions", "0.1"), ("rnl-std", "0.3")]);
        assert!(c.symbols.iter().all(|s| s.package == "analog-canvas-extensions"));
        assert_eq!(c.symbols.len(), 29);
        assert!(c.definitions.is_empty() && c.sets.is_empty());
        let none = CatalogOptions { packages: Some(vec!["no-such-package".into()]), ..CatalogOptions::default() };
        assert_eq!(crate::catalog(&MemoryProvider::new(), &none), Catalog::default());
    }

    const ACME: &str = r#"@RNL(type="header") {
  @VERSION("0.3");
  @PACKAGE(name="acme", version="1.0");
  @USE-PACKAGE(name="rnl-std-basic", version="0.3")
}
@RNL {
  $BLOCK = @DEFINE-SYMBOL(box=[-2, -2, 2, 2]) {
    (2, 0) @PIN(terminal="out", direction="right");
    (-2, 1) @PIN(terminal=2, direction="left");
    (-2, -1) @PIN(terminal=1, direction="left");
    (0, -2) @PIN(terminal="vss");
    @RECTANGLE {(-1.5, -1.5) (1.5, 1.5)};
    ? @TEXT("fixed");
    ? @TEXT("${name}", key="name", halign="middle") => @PLACE(0, 2.5)
  };
  $TAP = @DEFINE(roles=[["t", "terminal"], "{rest}"], parameters=["size", ["gap", 2]]) {};
  $TAP_SYMBOL = @DEFINE-SYMBOL {
    (0, 0) @CIRCLE(radius=0.5)
  };
  # @SELECT-USAGE $TAP => @SET-SYMBOL $TAP_SYMBOL;
  $TAPS = {};
  $TAPS @SELECT-ANCESTOR-USAGE $TAP => @SET-SYMBOL $BLOCK
}
"#;

    #[test]
    fn a_package_of_the_frontends_package_directories_is_listed() {
        let mut files = MemoryProvider::new().with("/w/packages/acme.rnl", ACME);
        let c = crate::catalog(&files, &CatalogOptions::default());
        assert!(c.packages.iter().all(|p| p.name != "acme"), "only the package directories are read");
        files.package_dirs.push("/w/packages".into());
        let c = crate::catalog(&files, &CatalogOptions::default());
        let acme = c.packages.iter().find(|p| p.name == "acme").unwrap();
        assert_eq!(
            serde_json::to_value(acme).unwrap(),
            json!({"name": "acme", "version": "1.0", "language": "0.3", "file": "acme.rnl",
                   "references": ["rnl-std-basic"], "series": false, "implicit": false})
        );
        // the terminals by number first, then by name as written; a pin without a direction; a
        // text without a key is no label
        let block = serde_json::to_value(symbol(&c, "$BLOCK")).unwrap();
        assert_eq!(
            block["pins"],
            json!([
                {"terminal": 1, "at": [-2.0, -1.0], "direction": "left"},
                {"terminal": 2, "at": [-2.0, 1.0], "direction": "left"},
                {"terminal": "out", "at": [2.0, 0.0], "direction": "right"},
                {"terminal": "vss", "at": [0.0, -2.0], "direction": null}
            ])
        );
        assert_eq!(
            block["labels"],
            json!([{"key": "name", "text": "${name}", "at": [0.0, 2.5], "halign": "middle", "valign": "baseline"}])
        );
        // a rule of another form draws $BLOCK
        assert_eq!(block["definition"], Value::Null);
        assert_eq!(
            serde_json::to_value(definition(&c, "$TAP")).unwrap(),
            json!({"definition": "$TAP", "package": "acme", "version": "1.0", "file": "acme.rnl",
                   "roles": [["t", "terminal"], "{rest}"], "parameters": [["size", null], ["gap", 2.0]],
                   "symbol": "$TAP_SYMBOL"})
        );
        let tap = symbol(&c, "$TAP_SYMBOL");
        assert_eq!((tap.definition.as_deref(), tap.bbox), (Some("$TAP"), [-0.5, -0.5, 0.5, 0.5]));
        let sets: Vec<&str> = c.sets.iter().filter(|s| s.package == "acme").map(|s| s.set.as_str()).collect();
        assert_eq!(sets, ["$TAPS"]);
        // in the order of the file
        let acme: Vec<&str> = c.symbols.iter().filter(|s| s.package == "acme").map(|s| s.symbol.as_str()).collect();
        assert_eq!(acme, ["$BLOCK", "$TAP_SYMBOL"]);
    }

    #[test]
    fn of_one_name_and_version_the_first_package_directory_has_it() {
        // a frontend's own rnl-std-basic 0.3 hides the bundled one, as the runtime finds it (16.2)
        let own = "@RNL(type=\"header\") { @VERSION(\"0.3\"); @PACKAGE(name=\"rnl-std-basic\", version=\"0.3\")}\n\
                   @RNL { $ONLY = @DEFINE-SYMBOL { (0, 0) @PIN(terminal=1) } }\n";
        let mut files = MemoryProvider::new().with("/p/basic.rnl", own);
        files.package_dirs.push("/p".into());
        let c = crate::catalog(&files, &CatalogOptions::default());
        let basic: Vec<&str> =
            c.symbols.iter().filter(|s| s.package == "rnl-std-basic").map(|s| s.symbol.as_str()).collect();
        assert_eq!(basic, ["$ONLY"]);
        assert_eq!(c.packages.iter().find(|p| p.name == "rnl-std-basic").unwrap().file, "basic.rnl");
    }

    #[test]
    fn the_catalog_keeps_the_field_names_frontends_read() {
        let v = serde_json::to_value(default()).unwrap();
        assert_eq!(keys(&v), ["packages", "symbols", "definitions", "sets"]);
        assert_eq!(
            keys(&v["packages"][0]),
            ["name", "version", "language", "file", "references", "series", "implicit"]
        );
        assert_eq!(
            keys(&v["symbols"][0]),
            ["symbol", "package", "version", "file", "definition", "pins", "box", "labels"]
        );
        assert_eq!(keys(&v["symbols"][0]["pins"][0]), ["terminal", "at", "direction"]);
        assert_eq!(
            keys(&v["definitions"][0]),
            ["definition", "package", "version", "file", "roles", "parameters", "symbol"]
        );
        assert_eq!(keys(&v["sets"][0]), ["set", "package", "version", "file"]);
        // every label of the bundled symbols has its text and its place, and every pin its direction
        let c = every_version();
        assert!(c.symbols.iter().flat_map(|s| &s.labels).all(|l| l.at.is_some() && l.text.is_some()));
        assert!(c.symbols.iter().flat_map(|s| &s.pins).all(|p| p.direction.is_some()));
    }
}
