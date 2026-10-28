//! The text format: a file that is not a netlist, such as a Verilog-A model, kept as text and not
//! read. It claims the files whose suffix marks a hardware description language, so that one opened
//! or included without a format set for it is not read as SPICE; a frontend may set it for any file.

use crate::diag::Loc;
use crate::host::format::{FileRead, Format, Reading};
use crate::host::{file_name, Dialect};

/// The name of the text format, as settings and requests give it.
pub const NAME: &str = "text";

/// The suffixes of the hardware description languages, lower case and without the dot.
const SUFFIXES: [&str; 8] = ["va", "vams", "v", "vh", "sv", "svh", "vhd", "vhdl"];

/// The format of the files that are not netlists.
pub struct Text;

/// The hardware description language that the suffix of `path` marks, if any.
pub fn language(path: &str) -> Option<&'static str> {
    let name = file_name(path).to_lowercase();
    let (stem, suffix) = name.rsplit_once('.')?;
    if stem.is_empty() {
        return None;
    }
    Some(match suffix {
        "va" => "Verilog-A",
        "vams" => "Verilog-AMS",
        "v" | "vh" => "Verilog",
        "sv" | "svh" => "SystemVerilog",
        "vhd" | "vhdl" => "VHDL",
        _ => return None,
    })
}

impl Format for Text {
    fn name(&self) -> &'static str {
        NAME
    }

    fn title(&self) -> &'static str {
        "Text"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &SUFFIXES
    }

    fn dialect(&self) -> Dialect {
        Dialect { name: NAME.into(), case_sensitive: false, ground_aliases: Vec::new() }
    }

    fn claims(&self, path: &str, _text: &str) -> bool {
        language(path).is_some()
    }

    /// Keep the file's text, and note at its include, or at its first line when it is opened, that
    /// it is not read.
    fn read_file(&self, cx: &mut Reading, file: &FileRead) {
        let Some(text) = cx.provider.read(file.path) else {
            return;
        };
        let fname = cx.display(file.path);
        if !cx.doc.sources.iter().any(|(f, _)| *f == fname) {
            cx.doc.sources.push((fname.clone(), text));
        }
        let shown = file_name(&fname);
        let message = match language(file.path) {
            Some(l) => format!("`{shown}` is not a netlist ({l}); it is not read"),
            None => format!("`{shown}` is read as text, not as a netlist; it is not read"),
        };
        let loc = cx.doc.levels[file.level].include_site.clone().unwrap_or_else(|| Loc::line(&fname, 1));
        cx.sink.hint(message, Some(loc));
    }

    fn finish(&self, _cx: &mut Reading) {}
}
