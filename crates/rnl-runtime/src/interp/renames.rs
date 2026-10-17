//! The names of the standard series 0.2 that the series 0.3 renamed or moved (16.3), for the
//! diagnostics of a file that writes one where the series 0.3 is in effect: they name what the
//! series 0.3 calls it, as the migration table of `libraries/standard` does. A hint is given only
//! when the reading finds the new name, so a file of the series 0.2 is never told about 0.3.

use super::ctx::Ctx;
use super::Interp;

/// A name of the series 0.2, without its `$`, and the name the series 0.3 gives the same thing.
const RENAMED: [(&str, &str); 21] = [
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
    ("DIFFPAIR_TAIL", "DIFFERENTIAL_PAIR_WITH_TAIL"),
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
        None
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
