//! `rnl`: check, draw and solve Rich Netlist documents.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;

use rnl_cli::codec::{decode, encode, OutputEncoding};
use rnl_cli::solve::Target;
use rnl_cli::{dump, FsProvider};
use rnl_runtime::diag::{codes, Code, Severity, Sink};
use rnl_runtime::source::{Layered, MemoryProvider, Provider};
use rnl_runtime::{Interpretation, Options};
use rnl_view::BuildOptions;

mod solver_args;
mod upgrade_args;

const USAGE: &str = concat!(
    "usage:
  rnl --version                   print the core version and build commit
  rnl check FILE [options]          print the diagnostics and the drawing's geometry hints; exit 1 on errors or violations
  rnl render FILE -o DIR [options]  write one SVG per drawing level, or one PNG image with --png
  rnl dump FILE [options]           print the effective model and diagnostics as JSON
  rnl view FILE [options]           print the Document a viewer shows, as JSON
  rnl explain FILE REF [options]    print the usages and groups the object REF is in, such as #M1, &vdd,
                                    #M5:d or %dp1, written as RNL refers to it
  rnl problem FILE [options]        print what a solver reads of each level, with its hash, as JSON
  rnl solve FILE -o DIR [options]   solve each drawing level and write its record of the solver back
  rnl solver-defaults              print 0.4 solver defaults as RNL named parameters
  rnl upgrade FILE [-o DIR | --in-place] [--report REPORT.json] [--approximate]  preview migration to 0.4, or save it
  rnl preview FILE [options]        print the solve as the desktop viewer previews it, as JSON; writes nothing
  rnl snapshot FILE [options]       print the editor interface's snapshot, as JSON
  rnl catalog [options]             print the symbols, definitions and sets of the packages, as JSON
  rnl formats                       print the formats a file can be read with, as JSON
  rnl strip-generated FILE -o OUT [options] write FILE without its type=\"generated\" records
  rnl write REQUEST.json -o OUT     write an edit transaction's ops into its netlist
  rnl replace-records FILE --level TITLE --records RECORDS (-o DIR | --in-place) [options]
                                    replace the RNL records of one level with the records in RECORDS

FILE is a SPICE netlist, a Spectre netlist (.scs) or a package file (.rnl), which is checked on its
own; catalog and formats take none. A package file is one of the format rnl: a .rnl file, or a file
of another suffix whose first record, at the start of a line, is a header record with @PACKAGE.
A file whose suffix marks a hardware description language (.va, .vams, .v, .vh, .sv, .svh, .vhd,
.vhdl) is not a netlist: its format is text, and it is not read, unless a format is set for it.
options:
  --path DIR          search packages and bare include filenames here; repeatable
  --package-dir DIR   look for packages in DIR as well (16.2), its .rnl files; repeatable
  --no-std            do not use the standard package at the root (16.3)
  --include-std       add the standard package to a draft 0.4 file (off by default)
  --std-version V     use version V of the file language's standard collection at the root
                      to read a file of 0.3 with the series 0.2; a version pinned in a header
                      record still comes first (16.3), and V not found is an error only for the
                      files that do not pin one. rnl solve does not take it
  --no-generated      read the netlist without its type=\"generated\" records, as a solver does (2.8)
  --dialect NAME      the format of FILE, spice, spectre, rnl or text, instead of guessing from it
  --format PATH=NAME  read the file PATH with the format NAME, spice, spectre, rnl or text; repeatable.
                      PATH is relative to the current directory or absolute, as FILE may be; one
                      that the netlist does not read is warned of. A file given none is read by its own
                      suffix and first lines, a .scs file or one with simulator lang=spectre as
                      Spectre, a package file as rnl, a .va file and the like as text, and FILE by
                      --dialect before them. A file set to rnl is a package that USE-PACKAGE finds
                      by name, before those of --package-dir, whatever its suffix; FILE set to rnl
                      is checked as a package file, an error when its header has no @PACKAGE
upgrade options:
  --approximate      opt into documented choices that may change meaning; off by default
                      report each choice and its risk separately from exact edits
view options:
  --primitives        give each level's primitives, in scene units with y up
  --no-svg            leave out each level's SVG
  --flylines          draw the flylines (13.3, 13.7) of every net in the SVG and the primitives, to
                      check what is not wired; view, snapshot and render take it. Each is a group of
                      class \"flyline\" in the SVG
  --hints             give each level its geometry hints, as check prints them; experimental
  --status            give each level its solve status: fixed, unsolved, solved or stale
  --constraints       give each level its constraints after expansion, what a solver takes in, tied
                      to the Document's objects; experimental, not promised to stay as it is
  --bind SUBCKT       show the parameters of the subcircuit SUBCKT with the values --param gives
  --param NAME=VALUE  a value for a parameter of the subcircuit --bind names, as text; repeatable.
                      Only the Document's lists of parameters change; nothing drawn does
render options:
  --png               write a PNG image of each level instead of its SVG, drawn by the command
                      itself with the fonts it holds, so that it needs no font of the computer;
                      CJK text is drawn in it as empty boxes
  --scale N           the pixels of the PNG image for a unit of the SVG, 2 by default; at most 16
render options for inspection (all disabled by default):
  --usages MODE       usage frames: semantic (no drawing of their own) or all
  --currents          show in/out terminal currents, including explicit subcircuit port symbols
  --usage-roles       show role labels on enabled usage frames
  --overlay-style FILE  JSON: palette, width, dashes, current_color; independent of RNL styles
",
    solver_args::usage!(),
    "saving options (solve, strip-generated and replace-records only):
  --encoding MODE     preserve (default) or utf8; UTF-8 adds a BOM only to protect leading U+FEFF
                      copied includes that are not rewritten retain their original bytes
explain options:
  --level TITLE       look for REF only in the level with this title: a subcircuit name, or the
                      file name; otherwise in every level, each that has it in turn
  --json              print the usages as JSON
  one line for each usage: the roles the object fills · the usage · its definition · where it is
  written · where it comes from (written, a rule, a definition's body, written back by a solver) ·
  in effect, failed, overridden or disabled, with the reason; the usages that contain it follow,
  indented. A failed application, which made no usage, is listed too
snapshot options:
  --parts LIST        the parts to give, of drawing, host and model; host,model by default
catalog options (the packages are those of --package-dir and the bundled ones, each read as rnl
check reads a package file, with --no-std and --std-version):
  --package NAME      only the packages of this name; repeatable
  --all-versions      every version of each package, not only the newest
write:
  REQUEST.json is {\"dialect\", \"netlist\", \"author\", \"ops\"}, the ops as the editor interface
  draft writes them; --dialect (spice by default) and --author NAME give the fields it leaves out
replace-records options (FILE may have RNL records already; -o DIR or --in-place is needed, as for solve):
  --level TITLE       the level whose records are replaced, a subcircuit name or the file name; needed
  --records FILE      the new records, RNL source without comment markers: one or more @RNL(...) {...},
                      the level's whole new drawing; needed
  every record of the level in the file that holds it goes, authored or type=\"generated\"; the header
  record, the records of other levels and every other line stay as they are. The new records are
  written at the end of the level's own text, behind the comment marker of the file. It refuses,
  writing nothing, with a message that begins with the kind of the refusal, and exits with 1, where no
  level has the title or several do, where the level has no place for a record (in a region written
  to be encrypted, for one), where the records would come before the header record of the file, where
  the file declares a language version that is not interpreted, and where the records given do not
  read: the netlist is read again with them, and an error in their lines refuses"
);

struct Args {
    overlays: rnl_view::OverlayOptions,
    command: String,
    file: PathBuf,
    out: Option<PathBuf>,
    package_dirs: Vec<PathBuf>,
    search_paths: Vec<PathBuf>,
    std: bool,
    include_std: bool,
    std_version: Option<String>,
    generated: bool,
    dialect: Option<String>,
    /// `--format`: the formats set for files by path.
    formats: BTreeMap<String, String>,
    svg: bool,
    primitives: bool,
    /// `--flylines`.
    flylines: bool,
    /// `render`'s `--png`, and its `--scale`.
    png: bool,
    scale: f32,
    /// `view`'s `--hints`.
    hints: bool,
    status: bool,
    /// `view`'s `--constraints`.
    constraints: bool,
    level: Option<String>,
    /// The options of `problem`, `solve` and `preview`.
    solver: solver_args::SolverArgs,
    upgrade_report: Option<PathBuf>,
    upgrade_approximate: bool,
    author: Option<String>,
    /// `replace-records`' `--records`: the file of the new records.
    records: Option<PathBuf>,
    parts: Option<String>,
    /// `--package`: the names `catalog` lists; every name when there are none.
    packages: Vec<String>,
    all_versions: bool,
    encoding: OutputEncoding,
    /// `explain`'s REF, and `--json`.
    reference: Option<String>,
    json: bool,
    /// `view`'s `--bind` and `--param`.
    bind: Option<rnl_view::Bind>,
}

/// What the `rnl` command says of its own, printed as `error: RNL8010 cli-option-needs-value: …`:
/// its kind, and its text. A message of a library the command calls has no kind yet and is printed
/// without one.
pub(crate) struct Said {
    code: Option<Code>,
    text: String,
}

impl Said {
    pub(crate) fn new(code: Code, text: impl Into<String>) -> Said {
        Said { code: Some(code), text: text.into() }
    }
}

impl From<String> for Said {
    fn from(text: String) -> Said {
        Said { code: None, text }
    }
}

impl From<(Code, String)> for Said {
    fn from((code, text): (Code, String)) -> Said {
        Said { code: Some(code), text }
    }
}

impl From<&str> for Said {
    fn from(text: &str) -> Said {
        Said { code: None, text: text.into() }
    }
}

impl std::fmt::Display for Said {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.code {
            Some(code) => write!(f, "{code}: {}", self.text),
            None => f.write_str(&self.text),
        }
    }
}

/// An option that is last on the command line, or whose value has the wrong shape.
pub(crate) fn needs(text: &str) -> Said {
    Said::new(codes::CLI_OPTION_NEEDS_VALUE, text)
}

/// An `--encoding` that is neither preserve nor utf8.
fn unknown_encoding(text: String) -> Said {
    Said::new(codes::CLI_ENCODING_UNKNOWN, text)
}

/// The commands, as `run` takes them.
const COMMANDS: [&str; 16] = [
    "solver-defaults",
    "check",
    "dump",
    "problem",
    "view",
    "explain",
    "snapshot",
    "formats",
    "catalog",
    "render",
    "solve",
    "upgrade",
    "preview",
    "strip-generated",
    "write",
    "replace-records",
];

fn parse_args() -> Result<Args, Said> {
    let mut it = std::env::args().skip(1);
    let command = it.next().ok_or_else(|| Said::new(codes::CLI_USAGE, USAGE))?;
    if !COMMANDS.contains(&command.as_str()) && !command.starts_with('-') {
        return Err(Said::new(codes::CLI_UNKNOWN_COMMAND, format!("unknown command {command}\n{USAGE}")));
    }
    let mut positional = Vec::new();
    let mut written_options = Vec::new();
    let mut json = false;
    let mut constraints = false;
    let mut bind_subckt: Option<String> = None;
    let mut bind_params: BTreeMap<String, String> = BTreeMap::new();
    let mut out = None;
    let mut package_dirs = Vec::new();
    let mut search_paths = Vec::new();
    let mut std = true;
    let mut include_std = false;
    let mut std_version = None;
    let mut generated = true;
    let mut dialect = None;
    let mut formats = BTreeMap::new();
    let (mut svg, mut primitives, mut status) = (true, false, false);
    let (mut flylines, mut hints) = (false, false);
    let (mut png, mut scale) = (false, None);
    let mut level = None;
    let mut solver = solver_args::SolverArgs::default();
    let mut upgrade_report = None;
    let mut upgrade_approximate = false;
    let mut author = None;
    let mut records = None;
    let mut parts = None;
    let mut packages = Vec::new();
    let mut all_versions = false;
    let mut overlays = rnl_view::OverlayOptions::default();
    let mut overlay_args = false;
    let mut encoding = None;
    while let Some(a) = it.next() {
        if a.starts_with('-') {
            written_options.push(a.clone());
        }
        match a.as_str() {
            "--usages" => {
                overlay_args = true;
                overlays.usages = match it.next().as_deref() {
                    Some("all") => rnl_view::UsageMode::All,
                    Some("semantic") => rnl_view::UsageMode::Semantic,
                    _ => return Err(needs("--usages needs all or semantic")),
                };
            }
            "--currents" => {
                overlay_args = true;
                overlays.currents = true;
            }
            "--usage-roles" => {
                overlay_args = true;
                overlays.roles = true;
            }
            "--overlay-style" => {
                overlay_args = true;
                let path = it.next().ok_or_else(|| needs("--overlay-style needs a JSON file"))?;
                let text = std::fs::read_to_string(&path)
                    .map_err(|e| Said::new(codes::CLI_OVERLAY_STYLE_UNREADABLE, format!("{path}: {e}")))?;
                overlays.style = serde_json::from_str(&text)
                    .map_err(|e| Said::new(codes::CLI_OVERLAY_STYLE_UNREADABLE, format!("{path}: {e}")))?;
            }
            "-o" | "--out" => out = Some(PathBuf::from(it.next().ok_or_else(|| needs("-o needs a path"))?)),
            "--package-dir" => {
                package_dirs.push(PathBuf::from(it.next().ok_or_else(|| needs("--package-dir needs a directory"))?))
            }
            "--path" => {
                let dir = PathBuf::from(it.next().ok_or_else(|| needs("--path needs a directory"))?);
                package_dirs.push(dir.clone());
                search_paths.push(dir);
            }
            "--no-std" => {
                std = false;
                include_std = false;
            }
            "--include-std" => {
                std = true;
                include_std = true;
            }
            "--std-version" => {
                std_version = Some(it.next().ok_or_else(|| needs("--std-version needs a version, such as 0.2"))?)
            }
            "--no-generated" => generated = false,
            "--dialect" => {
                dialect = Some(it.next().ok_or_else(|| needs("--dialect needs spice, spectre, rnl or text"))?)
            }
            "--format" => {
                let set = it.next().ok_or_else(|| needs("--format needs PATH=NAME, such as models.inc=spectre"))?;
                let (path, name) = set
                    .rsplit_once('=')
                    .ok_or_else(|| needs("--format needs PATH=NAME, such as models.inc=spectre"))?;
                formats.insert(path.to_string(), name.to_string());
            }
            "--no-svg" => svg = false,
            "--primitives" => primitives = true,
            "--flylines" => flylines = true,
            "--png" => png = true,
            "--scale" => {
                let n = it.next().ok_or_else(|| needs("--scale needs a number, such as 2"))?;
                scale = match n.parse::<f32>() {
                    Ok(n) if n > 0.0 && n <= 16.0 => Some(n),
                    _ => return Err(needs("--scale needs a number above 0 and at most 16, such as 2")),
                };
            }
            "--hints" => hints = true,
            "--status" => status = true,
            "--constraints" => constraints = true,
            "--level" => level = Some(it.next().ok_or_else(|| needs("--level needs a title"))?),
            "--author" => author = Some(it.next().ok_or_else(|| needs("--author needs a name"))?),
            "--records" => records = Some(PathBuf::from(it.next().ok_or_else(|| needs("--records needs a file"))?)),
            "--parts" => parts = Some(it.next().ok_or_else(|| needs("--parts needs a list, such as host,model"))?),
            "--package" => {
                packages.push(it.next().ok_or_else(|| needs("--package needs a name, such as rnl-std-basic"))?)
            }
            "--all-versions" => all_versions = true,
            "--json" => json = true,
            "--bind" => bind_subckt = Some(it.next().ok_or_else(|| needs("--bind needs a subcircuit name"))?),
            "--param" => {
                let set = it.next().ok_or_else(|| needs("--param needs NAME=VALUE, such as w=2u"))?;
                let (name, value) =
                    set.split_once('=').ok_or_else(|| needs("--param needs NAME=VALUE, such as w=2u"))?;
                bind_params.insert(name.to_string(), value.to_string());
            }
            "--encoding" => {
                encoding = Some(
                    it.next()
                        .ok_or_else(|| needs("--encoding needs preserve or utf8"))?
                        .parse()
                        .map_err(unknown_encoding)?,
                )
            }
            _ if a.starts_with("--encoding=") => encoding = Some(a[11..].parse().map_err(unknown_encoding)?),
            "--approximate" if command == "upgrade" => upgrade_approximate = true,
            "--report" if command == "upgrade" => {
                upgrade_report =
                    Some(PathBuf::from(it.next().ok_or_else(|| needs("upgrade --report needs a JSON path"))?));
            }
            _ if solver.take(&a, &mut it)? => {}
            _ if a.starts_with('-') => {
                return Err(Said::new(codes::CLI_UNKNOWN_OPTION, format!("unknown option {a}\n{USAGE}")))
            }
            _ => positional.push(a),
        }
    }
    for option in &written_options {
        // Upgrade owns these existing whole-source validation errors.
        if command == "upgrade" && matches!(option.as_str(), "--level" | "--solver-options" | "--previous") {
            continue;
        }
        let commands: &[&str] = match option.as_str() {
            "--level" => &["problem", "solve", "preview", "explain", "replace-records"],
            "--parts" => &["snapshot"],
            "--author" => &["write"],
            "--report" => &["solve", "upgrade"],
            "--in-place" => &["solve", "upgrade", "replace-records"],
            "--solver-options" | "--solver-config" => &["problem", "solve", "preview"],
            "--progress" | "--progress=json" | "--external-solver" | "--external-arg" | "--external-timeout" => {
                &["solve"]
            }
            "--previous" | "--solver-input" => &["problem"],
            "--png" | "--scale" => &["render"],
            "--primitives" | "--no-svg" | "--status" => &["view", "snapshot"],
            "--flylines" => &["view", "snapshot", "render"],
            "--hints" => &["view"],
            "--package" | "--all-versions" => &["catalog"],
            "-o" | "--out" => &["render", "solve", "upgrade", "strip-generated", "write", "replace-records"],
            _ => continue,
        };
        if !commands.contains(&command.as_str()) {
            return Err(Said::new(
                codes::CLI_OPTION_WRONG_COMMAND,
                format!("{option} is not an option of {command}; accepted by {}", commands.join(", ")),
            ));
        }
    }
    if let Some(name) = &dialect {
        rnl_view::validate_format(name, "--dialect")
            .map_err(|message| Said::new(codes::VIEW_FORMAT_UNKNOWN, message))?;
    }
    // explain takes FILE and REF; another command FILE, the last one given
    let reference = if command == "explain" {
        if positional.len() != 2 {
            let text = format!("rnl explain takes FILE and REF, such as rnl explain top.sp '#M1'\n{USAGE}");
            return Err(Said::new(codes::CLI_EXPLAIN_ARGS, text));
        }
        positional.pop()
    } else {
        None
    };
    if command == "upgrade" && positional.len() != 1 {
        return Err(needs("upgrade takes exactly one input FILE"));
    }
    let file = positional.pop().map(PathBuf::from);
    if json && command != "explain" {
        return Err(Said::new(codes::CLI_OPTION_WRONG_COMMAND, "--json is an option of explain"));
    }
    if constraints && command != "view" {
        let text = "--constraints is an option of view; rnl problem prints what a solver reads";
        return Err(Said::new(codes::CLI_OPTION_WRONG_COMMAND, text));
    }
    if (bind_subckt.is_some() || !bind_params.is_empty()) && command != "view" {
        return Err(Said::new(codes::CLI_OPTION_WRONG_COMMAND, "--bind and --param are options of view"));
    }
    if !bind_params.is_empty() && bind_subckt.is_none() {
        let text = "--param needs --bind SUBCKT, the subcircuit whose parameters it gives";
        return Err(Said::new(codes::CLI_PARAM_NEEDS_BIND, text));
    }
    let bind = bind_subckt.map(|subckt| rnl_view::Bind { subckt, params: bind_params });
    solver.check(&command)?;
    if overlay_args && command != "render" {
        return Err(Said::new(codes::CLI_OPTION_WRONG_COMMAND, "overlay options are options of render"));
    }
    overlays.validate_coded()?;
    if encoding.is_some() && !matches!(command.as_str(), "solve" | "strip-generated" | "replace-records") {
        let text = "--encoding is only for solve, strip-generated and replace-records";
        return Err(Said::new(codes::CLI_OPTION_WRONG_COMMAND, text));
    }
    if records.is_some() && command != "replace-records" {
        return Err(Said::new(codes::CLI_OPTION_WRONG_COMMAND, "--records is an option of replace-records"));
    }
    // the catalog reads the package directories and the bundled packages, not a file
    let file = match (command.as_str(), file) {
        ("catalog" | "formats" | "solver-defaults", None) => PathBuf::new(),
        (c @ ("catalog" | "formats" | "solver-defaults"), Some(f)) => {
            let text = format!("rnl {c} takes no FILE, not {}\n{USAGE}", f.display());
            return Err(Said::new(codes::CLI_COMMAND_TAKES_NO_FILE, text));
        }
        (_, file) => file.ok_or_else(|| Said::new(codes::CLI_USAGE, USAGE))?,
    };
    let formats = formats.into_iter().map(|(path, name)| (format_path(&path, &file), name)).collect();
    Ok(Args {
        upgrade_report,
        upgrade_approximate,
        overlays,
        command,
        file,
        out,
        package_dirs,
        search_paths,
        std,
        include_std,
        std_version,
        generated,
        dialect,
        formats,
        svg,
        primitives,
        flylines,
        png,
        scale: scale.unwrap_or(2.0),
        hints,
        status,
        constraints,
        level,
        solver,
        author,
        records,
        parts,
        packages,
        all_versions,
        encoding: encoding.unwrap_or_default(),
        reference,
        json,
        bind,
    })
}

fn main() -> ExitCode {
    let argv: Vec<_> = std::env::args().skip(1).collect();
    if argv == ["--version"] || argv == ["-V"] {
        println!("rnl {} ({})", rnl_cli::CORE_VERSION, rnl_cli::BUILD_COMMIT);
        return ExitCode::SUCCESS;
    }
    // asked for, the usage is the answer, not an error
    if argv.first().is_some_and(|a| a == "-h" || a == "--help") {
        print!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    match parse_args().and_then(|args| run(&args)) {
        Ok(code) => code,
        Err(said) => {
            eprintln!("error: {said}");
            ExitCode::from(2)
        }
    }
}

/// The runtime's options the command line gives.
fn options(args: &Args) -> Options {
    Options {
        std: args.std,
        include_std: args.include_std,
        std_version: args.std_version.clone(),
        generated: args.generated,
        ..Options::default()
    }
}

fn interpret(args: &Args, provider: &FsProvider) -> Result<Interpretation, String> {
    rnl_cli::interpret_file_with(&args.file, args.dialect.as_deref(), &args.formats, provider, &options(args))
}

/// The path of a `--format` as the reading names the files of `file`: an include resolves against
/// the directory of `file`, relative when `file` is and absolute when it is, so the path is made
/// absolute for an absolute `file`, and relative to the current directory, when it is in it, for a
/// relative one.
fn format_path(path: &str, file: &std::path::Path) -> String {
    let p = PathBuf::from(path);
    let p = if file.is_absolute() {
        std::path::absolute(&p).unwrap_or(p)
    } else {
        // the current directory compared as the file system compares paths, by case or not
        let key = |p: &std::path::Path| rnl_cli::FsProvider::new(&[]).path_key(&rnl_cli::path_str(p));
        let cwd = std::env::current_dir().map(|d| format!("{}/", key(&d).trim_end_matches('/')));
        match cwd {
            Ok(dir) if key(&p).starts_with(&dir) => {
                let skip = rnl_cli::path_str(&p).chars().take(dir.chars().count()).map(char::len_utf8).sum::<usize>();
                PathBuf::from(&rnl_cli::path_str(&p)[skip..])
            }
            _ => p,
        }
    };
    rnl_cli::path_str(&p)
}

/// Check the `--format`s before a command reads FILE: an unknown format name is an error, and each
/// path that FILE's netlist does not read is warned of, as it sets nothing, but for one set to `rnl`,
/// a package that USE-PACKAGE finds by name. A package file FILE reads no other file.
fn check_formats(args: &Args, provider: &FsProvider) -> Result<(), Said> {
    if args.formats.is_empty() || matches!(args.command.as_str(), "catalog" | "formats" | "write") {
        return Ok(());
    }
    for (path, name) in &args.formats {
        rnl_view::validate_format(name, path).map_err(|message| Said::new(codes::VIEW_FORMAT_UNKNOWN, message))?;
    }
    let format =
        rnl_view::entry_format(&args.file.to_string_lossy(), args.dialect.as_deref(), &args.formats, provider)?;
    let read: Vec<String> = if format == rnl_runtime::host::package::NAME {
        Vec::new()
    } else {
        let mut sink = Sink::new();
        let doc = rnl_view::read_host_with(
            &args.file.to_string_lossy(),
            args.dialect.as_deref(),
            &args.formats,
            provider,
            &mut sink,
        )?;
        doc.levels
            .iter()
            .filter(|l| l.kind == rnl_runtime::host::LevelKind::File)
            .map(|l| provider.path_key(&rnl_runtime::source::normalize(&l.path)))
            .collect()
    };
    for (path, name) in args.formats.iter().filter(|(_, name)| *name != rnl_runtime::host::package::NAME) {
        let key = provider.path_key(&rnl_runtime::source::normalize(path));
        if !read.contains(&key) {
            let code = codes::CLI_FORMAT_PATH_UNUSED;
            eprintln!("warning: {code}: --format {path}={name}: {} reads no such file", args.file.display());
        }
    }
    Ok(())
}

/// An error of the operating system writing a file or a directory.
fn io(e: std::io::Error) -> Said {
    Said::new(codes::CLI_IO_ERROR_BARE, e.to_string())
}

fn diagnostic_summary(
    r: &rnl_runtime::Interpretation,
    errors: usize,
    violations: usize,
    warnings: usize,
    hints: usize,
) -> String {
    let modern =
        r.problems.iter().find(|problem| problem.level == 0).map(|problem| problem.version == "0.4").unwrap_or_else(
            || {
                r.sources
                    .iter()
                    .find(|(file, _)| file == &r.doc.main().file)
                    .and_then(|(_, text)| rnl_runtime::interp::unit::scan_language_decl(text))
                    .as_deref()
                    == Some("0.4")
            },
        );
    if modern {
        let count = |n, noun| if n == 1 { format!("1 {noun}") } else { format!("{n} {noun}s") };
        format!(
            "{}, {}, {}, {}",
            count(errors, "error"),
            count(violations, "violation"),
            count(warnings, "warning"),
            count(hints, "hint")
        )
    } else {
        format!("{errors} errors, {violations} violations, {warnings} warnings, {hints} hints")
    }
}

fn run(args: &Args) -> Result<ExitCode, Said> {
    let provider = FsProvider::new(&args.package_dirs).with_search_paths(&args.search_paths);
    check_formats(args, &provider)?;
    match args.command.as_str() {
        "solver-defaults" => {
            println!("{}", rnl_solver::SolverOptions::defaults_config());
            Ok(ExitCode::SUCCESS)
        }
        "check" => {
            // read with the problems too, for the lines of the records that place the objects the
            // geometry hints name
            let read = Options { problems: true, ..options(args) };
            let r = rnl_cli::interpret_file_with(&args.file, args.dialect.as_deref(), &args.formats, &provider, &read)?;
            let shown = r.diagnostics();
            for d in &shown {
                println!("{}", r.sink.render(d));
            }
            // the geometry hints of the drawing, read-only: they never block
            let geometry: Vec<rnl_view::hints::Hint> =
                rnl_view::hints::interpretation_hints(&r).into_iter().flat_map(|l| l.hints).collect();
            for g in &geometry {
                println!("{}", g.render());
            }
            let count = |sev| {
                shown.iter().filter(|d| d.severity == sev).count()
                    + geometry.iter().filter(|g| g.severity == sev).count()
            };
            let (e, v) = (count(Severity::Error), count(Severity::Violation));
            let (w, h) = (count(Severity::Warning), count(Severity::Hint));
            for line in rnl_cli::status_lines(&r) {
                println!("{line}");
            }
            println!("{}", diagnostic_summary(&r, e, v, w, h));
            // a violation fails the check as an error does: an assertion is written to be checked (7.6)
            Ok(if e > 0 || v > 0 { ExitCode::from(1) } else { ExitCode::SUCCESS })
        }
        "dump" => {
            let r = interpret(args, &provider)?;
            println!("{}", serde_json::to_string_pretty(&dump::interpretation(&r)).unwrap());
            Ok(ExitCode::SUCCESS)
        }
        "problem" => solver_args::problem(args, &provider),
        "view" => {
            let options = BuildOptions {
                dialect: args.dialect.clone(),
                formats: args.formats.clone(),
                runtime: options(args),
                svg: args.svg,
                primitives: args.primitives,
                flylines: args.flylines,
                hints: args.hints,
                status: args.status,
                constraints: args.constraints,
                bind: args.bind.clone(),
            };
            let doc = rnl_view::build(&args.file.to_string_lossy(), &provider, &options)?;
            println!("{}", serde_json::to_string_pretty(&doc).unwrap());
            Ok(ExitCode::SUCCESS)
        }
        "explain" => {
            let options = BuildOptions {
                dialect: args.dialect.clone(),
                formats: args.formats.clone(),
                runtime: options(args),
                svg: false,
                primitives: false,
                flylines: true,
                hints: false,
                status: false,
                constraints: false,
                bind: None,
            };
            let doc = rnl_view::build(&args.file.to_string_lossy(), &provider, &options)?;
            let reference = args.reference.as_deref().unwrap_or_default();
            let found = rnl_cli::explain::find(&doc, reference, args.level.as_deref())?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&rnl_cli::explain::json(&doc, &found)).unwrap());
            } else {
                for line in rnl_cli::explain::lines(&doc, &found) {
                    println!("{line}");
                }
            }
            Ok(ExitCode::SUCCESS)
        }
        "snapshot" => {
            let list = args.parts.as_deref().unwrap_or("host,model");
            let mut parts = rnl_view::Parts { drawing: false, host: false, model: false };
            for p in list.split(',').map(str::trim).filter(|p| !p.is_empty()) {
                match p {
                    "drawing" => parts.drawing = true,
                    "host" => parts.host = true,
                    "model" => parts.model = true,
                    _ => {
                        let text = format!("--parts: unknown part {p}; the parts are drawing, host and model");
                        return Err(Said::new(codes::CLI_PARTS_UNKNOWN, text));
                    }
                }
            }
            let options = BuildOptions {
                dialect: args.dialect.clone(),
                formats: args.formats.clone(),
                runtime: options(args),
                svg: args.svg,
                primitives: args.primitives,
                flylines: args.flylines,
                hints: false,
                status: args.status,
                constraints: false,
                bind: None,
            };
            let snapshot = rnl_view::snapshot(&args.file.to_string_lossy(), &provider, &options, parts)?;
            println!("{}", serde_json::to_string_pretty(&snapshot).unwrap());
            Ok(ExitCode::SUCCESS)
        }
        "formats" => {
            println!("{}", serde_json::to_string_pretty(&rnl_view::formats()).unwrap());
            Ok(ExitCode::SUCCESS)
        }
        "catalog" => {
            let options = rnl_view::CatalogOptions {
                packages: (!args.packages.is_empty()).then(|| args.packages.clone()),
                all_versions: args.all_versions,
                runtime: options(args),
            };
            println!("{}", serde_json::to_string_pretty(&rnl_view::catalog(&provider, &options)).unwrap());
            Ok(ExitCode::SUCCESS)
        }
        "render" => {
            let r = interpret(args, &provider)?;
            let overlays = if args.overlays.usages != rnl_view::UsageMode::None || args.overlays.currents {
                Some(rnl_view::build(
                    &args.file.to_string_lossy(),
                    &provider,
                    &BuildOptions {
                        dialect: args.dialect.clone(),
                        formats: args.formats.clone(),
                        runtime: options(args),
                        flylines: args.flylines,
                        ..BuildOptions::default()
                    },
                )?)
            } else {
                None
            };
            let out = args.out.clone().ok_or_else(|| Said::new(codes::CLI_NEEDS_OUT, "render needs -o DIR"))?;
            std::fs::create_dir_all(&out).map_err(io)?;
            let stem = args.file.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "out".into());
            for (k, lv) in r.model.levels.iter().enumerate() {
                let svg = match &overlays {
                    Some(d) => rnl_view::overlays::svg(&d.levels[k], &args.overlays)?,
                    None => rnl_render::svg(
                        lv,
                        &rnl_render::RenderOptions { flylines: args.flylines, ..rnl_render::RenderOptions::default() },
                    ),
                };
                let suffix = if args.png { "png" } else { "svg" };
                let name = if k == 0 {
                    format!("{stem}.{suffix}")
                } else {
                    format!("{stem}.{}.{suffix}", sanitize(&lv.title))
                };
                if args.png {
                    let image = rnl_cli::png::png(&svg, args.scale).map_err(|e| {
                        Said::new(codes::CLI_RENDER_PNG_FAILED, format!("{name}: no PNG image is drawn: {e}"))
                    })?;
                    std::fs::write(out.join(&name), image).map_err(io)?;
                } else {
                    std::fs::write(out.join(&name), svg).map_err(io)?;
                }
                println!("{}", out.join(&name).display());
            }
            let errors = r.diagnostics().iter().filter(|d| d.severity == Severity::Error).count();
            if errors > 0 {
                eprintln!("warning: {}: {errors} errors; run `rnl check` to see them", codes::CLI_RENDER_HAS_ERRORS);
            }
            Ok(ExitCode::SUCCESS)
        }
        "solve" => solver_args::solve(args, &provider),
        "upgrade" => upgrade_args::run(args, &provider),
        "preview" => solver_args::preview(args, &provider),
        "replace-records" => replace_records(args, &provider),
        "strip-generated" => {
            let out =
                args.out.clone().ok_or_else(|| Said::new(codes::CLI_NEEDS_OUT, "strip-generated needs -o OUT"))?;
            // Preserve the origin of the text actually read, not the adapter's display name
            // (which can be a basename). Disk wins over an exact virtual-package lookup.
            let entry = rnl_cli::path_str(&args.file);
            let (original, input) = match std::fs::read(&args.file) {
                Ok(bytes) => {
                    let text = decode(&bytes);
                    (Some(bytes), text)
                }
                Err(e) => {
                    let text = if args.file.exists() { None } else { rnl_cli::bundled().read(&entry) };
                    let cannot =
                        || Said::new(codes::CLI_CANNOT_READ, format!("cannot read {}: {e}", args.file.display()));
                    let text = text.ok_or_else(cannot)?;
                    (None, text)
                }
            };
            // A real file can have the virtual package's relative name on Unix. The shared
            // reader reserves the <bundled> prefix, so use a physical absolute path for disk
            // sources. Do not canonicalize: relative includes retain the entered directory.
            let read_path = if original.is_some() {
                std::path::absolute(&args.file).map_err(|e| {
                    Said::new(codes::CLI_CANNOT_RESOLVE_PATH, format!("cannot resolve {}: {e}", args.file.display()))
                })?
            } else {
                args.file.clone()
            };
            // Read the captured entry, so a file changed or removed during parsing cannot change
            // which encoding belongs to its text. Includes still use the normal disk provider.
            let held = MemoryProvider::new().with(&rnl_cli::path_str(&read_path), &input);
            let files = Layered { layers: vec![&held, &provider] };
            let mut sink = Sink::new();
            let read = read_path.to_string_lossy();
            // the entry read by its absolute path, the formats' paths so too
            let formats: BTreeMap<String, String> =
                args.formats.iter().map(|(p, f)| (format_path(p, &read_path), f.clone())).collect();
            let doc = rnl_view::read_host_with(&read, args.dialect.as_deref(), &formats, &files, &mut sink)?;
            let main = doc.main().file.clone();
            let cannot = || Said::new(codes::VIEW_CANNOT_READ, format!("cannot read {}", args.file.display()));
            let text = doc.source(&main).ok_or_else(cannot)?;
            let stripped = rnl_runtime::writeback::strip_generated(&doc, &main, text);
            let bytes = encode(original.as_deref(), &stripped, args.encoding)
                .map_err(|e| Said::new(codes::CODEC_LATIN1_UNREPRESENTABLE, format!("{}: {e}", args.file.display())))?;
            std::fs::write(&out, bytes).map_err(io)?;
            Ok(ExitCode::SUCCESS)
        }
        "write" => {
            let out = args.out.clone().ok_or_else(|| Said::new(codes::CLI_NEEDS_OUT, "write needs -o OUT"))?;
            let text = std::fs::read_to_string(&args.file)
                .map_err(|e| Said::new(codes::CLI_CANNOT_READ, format!("cannot read {}: {e}", args.file.display())))?;
            let mut request: serde_json::Value = serde_json::from_str(&text)
                .map_err(|e| Said::new(codes::CLI_REQUEST_NOT_JSON, format!("{}: {e}", args.file.display())))?;
            let fields = request
                .as_object_mut()
                .ok_or_else(|| Said::new(codes::CLI_REQUEST_NOT_JSON, "the request is not a JSON object"))?;
            if let Some(d) = &args.dialect {
                fields.insert("dialect".into(), d.clone().into());
            }
            if let Some(a) = &args.author {
                fields.insert("author".into(), a.clone().into());
            }
            fields.entry("dialect").or_insert_with(|| "spice".into());
            // the netlist is saved as OUT, so it is read under that name unless the request names one
            if let Some(name) = out.file_name().and_then(|n| n.to_str()) {
                fields.entry("file").or_insert_with(|| name.into());
            }
            let request: rnl_edit::WriteRequest = serde_json::from_value(request)
                .map_err(|e| Said::new(codes::CLI_REQUEST_INVALID, format!("the request is not valid: {e}")))?;
            let written = if request.language_version.as_deref() == Some("0.4") {
                rnl_edit::write_with_options_diagnosed(&request, &provider, &options(args))
                    .map(|w| w.text)
                    .map_err(|e| Said::new(e.code, e.message))?
            } else {
                rnl_edit::write_with_options(&request, &provider, &options(args))?
            };
            std::fs::write(&out, written).map_err(io)?;
            Ok(ExitCode::SUCCESS)
        }
        _ => Err(Said::new(codes::CLI_UNKNOWN_COMMAND, format!("unknown command {}\n{USAGE}", args.command))),
    }
}

/// `rnl replace-records`: the records of one level replaced in the file that holds it, written as
/// `rnl solve` writes (`-o DIR` or `--in-place`). Exits with 1, writing nothing, when it refuses; with
/// 2 when a file cannot be read or written or the arguments do not read.
fn replace_records(args: &Args, provider: &FsProvider) -> Result<ExitCode, Said> {
    let target = match (&args.out, args.solver.in_place) {
        (Some(_), true) => {
            let text = "replace-records writes under -o DIR or --in-place, not both";
            return Err(Said::new(codes::CLI_REPLACE_TARGET_BOTH, text));
        }
        (Some(dir), false) => Target::Dir(dir.clone()),
        (None, true) => Target::InPlace,
        (None, false) => {
            let text = "replace-records needs -o DIR, to write the file under DIR, or --in-place, to write it over the file read";
            return Err(Said::new(codes::CLI_REPLACE_TARGET_MISSING, text));
        }
    };
    let command = "replace-records";
    let Some(level) = &args.level else {
        let text = format!("{command} needs --level TITLE, the level whose records are replaced");
        return Err(Said::new(codes::CLI_COMMAND_NEEDS_OPTION, text));
    };
    let Some(records) = &args.records else {
        let text = format!("{command} needs --records FILE, the RNL source of the new records");
        return Err(Said::new(codes::CLI_COMMAND_NEEDS_OPTION, text));
    };
    let bytes = std::fs::read(records)
        .map_err(|e| Said::new(codes::CLI_CANNOT_READ, format!("cannot read {}: {e}", records.display())))?;
    let options = rnl_cli::replace::ReplaceOptions {
        dialect: args.dialect.clone(),
        formats: args.formats.clone(),
        runtime: options(args),
    };
    let done = match rnl_cli::replace::replace_records(&args.file, provider, &options, level, &decode(&bytes)) {
        Ok(done) => done,
        Err(rnl_edit::replace::Failure::Refused(refused)) => {
            eprintln!("error: {refused}");
            return Ok(ExitCode::from(1));
        }
        Err(rnl_edit::replace::Failure::Unreadable(why)) => return Err(why.into()),
    };
    let writing = rnl_cli::solve::write_with_encoding(&done.run, &target, args.encoding);
    if let Some(path) = writing.written.first() {
        let was = &done.run.files[0].path;
        let line = format!("replaced the records of {level} in {was}: {} removed, {} added", done.removed, done.added);
        match target {
            Target::InPlace => println!("{line}"),
            Target::Dir(_) => println!("{line}; wrote {}", rnl_cli::path_str(path)),
        }
    }
    for path in &writing.copied {
        println!("copied {}", rnl_cli::path_str(path));
    }
    if let Target::Dir(dir) = &target {
        for path in &writing.outside {
            eprintln!(
                "warning: {}: {} reads {path}, which is outside its directory and is not copied under {}",
                codes::CLI_SOLVE_OUTSIDE_NOT_COPIED,
                done.run.entry,
                rnl_cli::path_str(dir)
            );
        }
    }
    match writing.error {
        Some(e) => Err(e.into()),
        None => Ok(ExitCode::SUCCESS),
    }
}

fn sanitize(s: &str) -> String {
    s.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '_' }).collect()
}

#[cfg(test)]
mod count_tests {
    use super::diagnostic_summary;

    #[test]
    fn check_summary_counts_follow_the_main_language_version() {
        for version in ["0.2", "0.3", "0.4"] {
            for problems in [false, true] {
                let source = format!("* counts\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\n");
                let provider = rnl_runtime::source::MemoryProvider::new().with("counts.sp", &source);
                let r = rnl_cli::interpret_file(
                    std::path::Path::new("counts.sp"),
                    None,
                    &provider,
                    &rnl_runtime::Options { std: false, problems, ..Default::default() },
                )
                .unwrap();
                let expected = if version == "0.4" {
                    "1 error, 1 violation, 1 warning, 1 hint"
                } else {
                    "1 errors, 1 violations, 1 warnings, 1 hints"
                };
                assert_eq!(diagnostic_summary(&r, 1, 1, 1, 1), expected);
                assert_eq!(diagnostic_summary(&r, 0, 2, 2, 0), "0 errors, 2 violations, 2 warnings, 0 hints");
            }
        }
    }
}
