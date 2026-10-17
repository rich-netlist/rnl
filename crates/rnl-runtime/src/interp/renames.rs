//! The names of the standard series 0.2 that the series 0.3 renamed, moved or left out, and the
//! roles it renamed (16.3), for the diagnostics of a file that writes one where the series 0.3 is
//! in effect: they name what the series 0.3 has instead, as the migration table of
//! `libraries/standard` does. A hint is given only when the reading finds a name of the series 0.3,
//! so a file of the series 0.2 is never told about 0.3.

use super::ctx::Ctx;
use super::Interp;

/// A name of the series 0.2, without its `$`, and the name the series 0.3 gives the same thing.
const RENAMED: [(&str, &str); 20] = [
    // symbols
    ("RES", "RESISTOR"),
    ("CAP", "CAPACITOR"),
    ("IND", "INDUCTOR"),
    ("VSRC", "VOLTAGE_SOURCE"),
    ("ISRC", "CURRENT_SOURCE"),
    ("NMOS4", "NMOS_4TERM"),
    ("PMOS4", "PMOS_4TERM"),
    ("SUPPLY_BAR", "SUPPLY_SYMBOL"),
    ("PORT_TAG", "IO_PORT_SYMBOL"),
    ("NC_CROSS", "NO_CONNECT_SYMBOL"),
    ("JUNCTION_DOT", "JUNCTION_SYMBOL"),
    ("CURRENT_ARROW_SYM", "CURRENT_ARROW_SYMBOL"),
    ("ARROW_HEAD", "ARROW_SYMBOL"),
    // definitions and sets
    ("GND", "GROUND"),
    ("IOPORT", "IO_PORT"),
    ("NC", "NO_CONNECT"),
    ("DIFFPAIR", "DIFFERENTIAL_PAIR"),
    ("GND_STUB", "GROUND_STUB"),
    ("GND_FILL_SET", "GROUND_FILL_SET"),
    ("LABEL_TERM_SET", "LABEL_TERMINAL_SET"),
];

/// A symbol of the series 0.2 whose name the series 0.3 gives to a definition, and the name the
/// series 0.3 gives the symbol.
const SYMBOL_NOW_DEFINITION: [(&str, &str); 1] = [("GROUND", "GROUND_SYMBOL")];

/// A name of the series 0.2 that the series 0.3 keeps, in a package that rnl-std does not bring
/// in and rnl-std-full does, and that package.
const MOVED: [(&str, &str); 1] = [("OPAMP", "rnl-std-functional")];

/// A name of the series 0.3 that the series 0.2 does not have: found, it says that the series 0.3
/// is in effect.
const ONLY_IN_0_3: &str = "RESISTOR";

/// A definition of the series 0.2 that the series 0.3 left out, and what to write instead, given
/// when `$DIFFERENTIAL_PAIR` of the series 0.3 is found: the tail of a differential pair is a role of
/// it, which may be left empty, and the standard series puts it under the middle of the pair.
const LEFT_OUT: [(&str, &str); 3] = [
    ("DIFFPAIR_TAIL", "the tail is the role tail of $DIFFERENTIAL_PAIR in the standard series 0.3, which may be left empty; write that"),
    (
        "INPUT_STAGE",
        "the standard series 0.3 has no input stage; write the tail into the role tail of the differential pair, as in #M1(in_p) #M2(in_n) #M5(tail) @AS $DIFFERENTIAL_PAIR",
    ),
    (
        "TAIL_CENTER",
        "the standard series 0.3 puts the tail of each differential pair under the middle of the pair itself, as a low default (14.8); for other objects write a definition of your own",
    ),
];

/// A role of a definition of the series 0.2 that the series 0.3 renamed: the definition, without
/// its `$`, the role in the series 0.2 and the role in the series 0.3.
const ROLE_RENAMED: [(&str, &str, &str); 1] = [("CHAIN", "series", "chain")];

impl Interp<'_> {
    /// Why `$name` cannot be found, when it is a name of the series 0.2 and the series 0.3 is in
    /// effect here: the name the series 0.3 gives it, or the package that has it.
    pub(crate) fn renamed_in_0_3(&mut self, name: &str, ctx: &Ctx) -> Option<String> {
        if let Some(&(_, new)) = RENAMED.iter().find(|(old, _)| *old == name) {
            if self.lookup('$', new, ctx).is_some() {
                return Some(format!(
                    "the standard series 0.3 calls it ${new}; write that, or pin the series 0.2 with @USE-PACKAGE(name=\"rnl-std\", version=\"0.2\") in the header record (16.3)"
                ));
            }
        }
        if let Some(&(_, package)) = MOVED.iter().find(|(old, _)| *old == name) {
            if self.lookup('$', ONLY_IN_0_3, ctx).is_some() {
                return Some(format!(
                    "the standard series 0.3 has it in {package}, which rnl-std does not bring in: name {package} or rnl-std-full with @USE-PACKAGE in the header record, or pin the series 0.2 (16.3)"
                ));
            }
        }
        if let Some(&(_, instead)) = LEFT_OUT.iter().find(|(old, _)| *old == name) {
            if self.lookup('$', "DIFFERENTIAL_PAIR", ctx).is_some() {
                return Some(format!(
                    "{instead}, or pin the series 0.2 with @USE-PACKAGE(name=\"rnl-std\", version=\"0.2\") in the header record (16.3)"
                ));
            }
        }
        None
    }

    /// Why the ordinary definition `definition` has no role `role`, when the series 0.3 renamed
    /// that role of the definition and the definition has the new name among its `roles`.
    pub(crate) fn role_renamed_in_0_3(definition: &str, role: &str, roles: &[&str]) -> Option<String> {
        let &(_, _, new) =
            ROLE_RENAMED.iter().find(|(d, old, _)| definition.strip_prefix('$') == Some(*d) && *old == role)?;
        roles.contains(&new).then(|| format!("; the standard series 0.3 calls it {new} (16.3)"))
    }

    /// Why `$name` is not the symbol it was in the series 0.2, when the series 0.3 is in effect
    /// here and gives that name to a definition.
    pub(crate) fn no_longer_a_symbol(&mut self, name: &str, ctx: &Ctx) -> Option<String> {
        let &(_, symbol) = SYMBOL_NOW_DEFINITION.iter().find(|(old, _)| *old == name)?;
        self.lookup('$', symbol, ctx)?;
        Some(format!(
            "in the standard series 0.3 ${name} is a definition, and ${symbol} the symbol that the series 0.2 called ${name} (16.3)"
        ))
    }
}
