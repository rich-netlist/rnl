//! What a package file defines, as the runtime reads it when the package is checked on its own
//! (2.7), for a tool that lists the packages it has, such as the catalog of `rnl-view`: the
//! package's own `$` names that are symbols, ordinary definitions or sets, in the order of its
//! file, and its rules that draw every usage of a definition with a symbol. A symbol comes as the
//! model gives the symbols it draws ([`SymbolView`]: its box, pins and graphics), so that a list of
//! them matches the drawings, with its labels (13.3).

use std::collections::HashMap;

use crate::geom::Pt;
use crate::model::SymbolView;
use crate::source::Provider;
use crate::syntax::{Arg, Member, ObjExpr, Params};

use super::obj::{Lit, ObjData, ObjId, TermKey, UnitId};
use super::unit::Binding;
use super::{Interp, Memo, Options};

/// What a package file defines (2.7, 16.2).
#[derive(Clone, Debug)]
pub struct PackageContents {
    /// As its PACKAGE says.
    pub name: String,
    pub version: String,
    /// The language version its file is read in: the one its header record declares, or the tool's
    /// default when it declares none (16.1).
    pub language: String,
    /// The packages its header record references with USE-PACKAGE, by name, in the order written
    /// (16.2).
    pub references: Vec<String>,
    /// Its own symbols, in the order of its file.
    pub symbols: Vec<PackageSymbol>,
    /// Its own ordinary definitions, in the order of its file.
    pub definitions: Vec<PackageDefinition>,
    /// The `$` names of its own sets, such as `$GROUND_NET_SET`, in the order of its file.
    pub sets: Vec<String>,
    /// Its rules written `# @SELECT-USAGE $DEF => @SET-SYMBOL $SYM`, which draw every usage of a
    /// definition with a symbol, in the order of its file.
    pub usage_symbols: Vec<UsageSymbol>,
}

/// A symbol of a package (9.2).
#[derive(Clone, Debug)]
pub struct PackageSymbol {
    /// Such as `$RESISTOR`.
    pub name: String,
    /// What it is made of, as the model gives a symbol it draws: its box, the one it writes or the
    /// extent of its graphics (9.2.3), its pins (9.2.4) and its graphics (9.2.5). The symbols drawn
    /// in it (9.2.7) count in its box, but are not listed: `nested` is empty.
    pub view: SymbolView,
    /// The terminals of each pin of `view`, as the symbol writes them: by number or by name.
    pub terminals: Vec<Vec<TermKey>>,
    /// Its labels: the texts written on `?` with a key (13.3), in the order written.
    pub labels: Vec<SymbolLabel>,
}

/// A label of a symbol: a text written on `?` with a key (13.3).
#[derive(Clone, Debug, PartialEq)]
pub struct SymbolLabel {
    pub key: String,
    /// The text as written, before `${…}` is read (13.6); None when it writes none.
    pub text: Option<String>,
    /// Where the symbol puts it, in the symbol's coordinates: its PLACE in the body, or its offset
    /// from the registration point; None when the symbol does not.
    pub at: Option<Pt>,
    /// The alignment it writes, if any.
    pub halign: Option<String>,
    pub valign: Option<String>,
}

/// An ordinary definition of a package (9.4).
#[derive(Clone, Debug)]
pub struct PackageDefinition {
    /// Such as `$GROUND`.
    pub name: String,
    /// Its roles, each as `roles=` writes it: a name, `"{name}"` for a set role, or `[name, kinds]`.
    pub roles: Vec<Lit>,
    /// Its parameters, each with its default, if any.
    pub parameters: Vec<(String, Option<Lit>)>,
}

/// A `$` name with the package that defines it, as the runtime finds it from where it is written
/// (16.2).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DefinedName {
    /// Such as `$GROUND`.
    pub name: String,
    pub package: String,
    pub version: String,
}

/// A rule `# @SELECT-USAGE $DEF => @SET-SYMBOL $SYM` (10.1, 11.3): every usage of the definition
/// is drawn with the symbol.
#[derive(Clone, Debug, PartialEq)]
pub struct UsageSymbol {
    pub definition: DefinedName,
    pub symbol: DefinedName,
}

/// The order of the versions of a package, as the runtime takes the newest of them (16.2):
/// "major.minor" by its two numbers, any other text before every such version.
pub fn version_key(version: &str) -> (i64, i64) {
    super::unit::version_key(version)
}

/// What the package file at `path` defines, read as it is when it is checked on its own with
/// `options` ([`crate::check_package`], 2.7), the packages it references and the standard package
/// of its language version brought in. None when the file is not a package.
pub fn package_contents(path: &str, provider: &dyn Provider, options: &Options) -> Option<PackageContents> {
    let mut it = Interp::new_package(path, provider, options).settled(|_| Interp::new_package(path, provider, options));
    it.package_contents()
}

impl Interp<'_> {
    /// What the package checked on its own defines, once its statements were read (`run`).
    fn package_contents(&mut self) -> Option<PackageContents> {
        let pu = self.package_main?;
        if self.units[pu].pkg_name.is_empty() {
            return None;
        }
        let mut out = PackageContents {
            name: self.units[pu].pkg_name.clone(),
            version: self.pkg_ver(pu),
            language: self.version_of(Some(pu)),
            references: self.units[pu].refs.iter().map(|r| r.name.clone()).collect(),
            symbols: Vec::new(),
            definitions: Vec::new(),
            sets: Vec::new(),
            usage_symbols: Vec::new(),
        };
        for key in self.units[pu].name_order.clone() {
            if key.0 != '$' {
                continue; // `%` names are the package's own business
            }
            let b = self.units[pu].names[&key].clone();
            let Some(obj) = self.package_binding(&b) else { continue };
            let name = format!("${}", b.name);
            match self.o(obj) {
                ObjData::Symbol(_) => {
                    let symbol = self.package_symbol(name, obj);
                    out.symbols.push(symbol);
                }
                ObjData::Definition(d) => out.definitions.push(PackageDefinition {
                    name,
                    roles: d.roles.iter().map(|r| r.written.clone()).collect(),
                    parameters: d.params.clone(),
                }),
                ObjData::Set(_) => out.sets.push(name),
                _ => {}
            }
        }
        out.usage_symbols = self.usage_symbols(pu);
        Some(out)
    }

    /// The object a `$` name of a package is bound to, as the package's statements were read
    /// without a level; None when it was not read, or failed.
    fn package_binding(&self, b: &Binding) -> Option<ObjId> {
        let key = b.key();
        let Some(Memo::Val(v)) = self.memo.get(&(key.clone(), None)) else { return None };
        let v = self.bound_value(&key, None, b.prefix, &b.name, *v);
        (!self.is_failed(v)).then_some(v)
    }

    fn package_symbol(&mut self, name: String, sym: ObjId) -> PackageSymbol {
        let mut views = Vec::new();
        let k = self.symbol_view(sym, &mut HashMap::new(), &mut views);
        let mut view = views.swap_remove(k);
        // the symbols drawn in it go by their index in `views`, which is not kept
        view.nested.clear();
        let sd = self.symbol(sym);
        let labels = sd
            .labels
            .iter()
            .filter_map(|l| {
                Some(SymbolLabel {
                    key: l.key.clone()?,
                    text: l.items.text.as_ref().map(|t| t.as_str().to_string()),
                    at: l.position,
                    halign: l.items.halign.clone(),
                    valign: l.items.valign.clone(),
                })
            })
            .collect();
        PackageSymbol { name, view, terminals: sd.pins.iter().map(|p| p.terms.clone()).collect(), labels }
    }

    /// The rules of a package, in the order of its file, that are written
    /// `# @SELECT-USAGE $DEF => @SET-SYMBOL $SYM`, read as rules (10.3), with a definition and a
    /// symbol found where they are written (16.2).
    fn usage_symbols(&mut self, pu: UnitId) -> Vec<UsageSymbol> {
        let mut out = Vec::new();
        for m in self.units[pu].members.clone() {
            if !self.rules.contains(&m.key(pu)) {
                continue;
            }
            let Some((definition, symbol)) = self.usage_symbol_form(pu, &m.m) else { continue };
            let definition = self.defined(pu, &definition, |o| matches!(o, ObjData::Definition(_)));
            let symbol = self.defined(pu, &symbol, |o| matches!(o, ObjData::Symbol(_)));
            if let (Some(definition), Some(symbol)) = (definition, symbol) {
                out.push(UsageSymbol { definition, symbol });
            }
        }
        out
    }

    /// The names of the definition and the symbol of a member written
    /// `# @SELECT-USAGE $DEF => @SET-SYMBOL $SYM`, its keywords read through the aliases of the
    /// unit's file (16.5): the selection of every usage of the definition, without conditions.
    fn usage_symbol_form(&self, u: UnitId, m: &Member) -> Option<(String, String)> {
        let [select, set] = &m.segments[..] else { return None };
        let (sc, ss) = (select.call.as_ref()?, set.call.as_ref()?);
        let plain = |p: &Option<Params>| p.as_ref().is_none_or(|p| p.positional.is_empty() && p.named.is_empty());
        if !m.binds.is_empty()
            || self.kw(&sc.kw, u) != "SELECT-USAGE"
            || self.kw(&ss.kw, u) != "SET-SYMBOL"
            || !plain(&sc.params)
            || !plain(&ss.params)
        {
            return None;
        }
        let ([scope, definition], [symbol]) = (&select.args[..], &set.args[..]) else { return None };
        let level = matches!(&scope.obj, ObjExpr::Ref(r) if r.is_bare_level()) && scope.role.is_none();
        if !level || scope.start > sc.start {
            return None;
        }
        Some((dollar_after(definition, sc.start)?, dollar_after(symbol, ss.start)?))
    }

    /// `$name` as the package `pu` finds it (16.2), when it is bound to an object that `kind` accepts.
    fn defined(&mut self, pu: UnitId, name: &str, kind: fn(&ObjData) -> bool) -> Option<DefinedName> {
        let b = self.lookup_unit('$', name, pu, false, None)?;
        let obj = self.package_binding(&b)?;
        if !kind(self.o(obj)) {
            return None;
        }
        let owner = b.owner;
        Some(DefinedName {
            name: format!("${}", b.name),
            package: self.units[owner].pkg_name.clone(),
            version: self.pkg_ver(owner),
        })
    }
}

/// The name of a plain `$NAME` written after the keyword at `call`, without a role, a path or a
/// subscript.
fn dollar_after(a: &Arg, call: usize) -> Option<String> {
    match &a.obj {
        ObjExpr::Ref(r)
            if r.prefix == '$'
                && a.role.is_none()
                && a.start > call
                && r.sub.is_none()
                && r.path.is_empty()
                && r.number.is_none() =>
        {
            r.name.clone()
        }
        _ => None,
    }
}
