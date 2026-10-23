# Host-region classification and safe writeback

This document records the supported host boundaries for RNL 0.3 §2.2. The adapters
classify these boundaries before collecting comment carriers. Excluded text cannot
add records, instances, includes, subcircuits, model cards or language switches.
A diagnostic identifies each excluded `@RNL` line. The original text is preserved.
This is structural classification, not a simulator or preprocessor implementation.

## Supported classifications and sources

| Host syntax | Reading and writing policy | Simulator documentation |
| --- | --- | --- |
| SPICE `.control` / `.endc` | Exclude the script through the exact closing keyword. Also applies in Spectre's SPICE mode. | [ngspice manual](https://ngspice.sourceforge.io/docs/ngspice-manual.pdf), §12.4.3 (control section); [WRspice Reference Manual 4.3.22](https://wrcad.com/manual/wrsmanual/node71.html), §2.10.1. |
| WRspice `.exec`, `.postrun` / `.endc` | Exclude these scripts as well. Prefixes such as `.endc_extra` do not close a script. | [WRspice 4.3.22 §2.10.1](https://wrcad.com/manual/wrsmanual/node71.html). |
| WRspice `.verilog` / `.endv` | Exclude the embedded Verilog block; netlist-looking statements inside it have no host effect. | [WRspice 4.3.22 manual](https://www.wrcad.com/manual/wrsmanual.pdf), §2.11.1, p. 67 ([HTML section](https://wrcad.com/manual/wrsmanual/node74.html)). |
| WRspice `.newjob` | The next physical line in a directly read main file is a title and cannot carry RNL. The insertion point before that title is forbidden. `.newjob` in include/library files does not introduce a title. | [WRspice 4.3.22 manual](https://www.wrcad.com/manual/wrsmanual.pdf), §2.3.5, pp. 26–27 ([HTML section](https://wrcad.com/manual/wrsmanual/node23.html)). |
| WRspice `.if`, `.elif`/`.elseif`, `.else`, `.endif` | Track nested preprocessing conditions. Retain a provably selected literal branch, exclude inactive branches, and diagnose unresolved conditions. New records go outside the whole conditional. | [WRspice 4.3.22 §2.5.3](https://wrcad.com/manual/wrsmanual/node40.html): these directives filter source before circuit parsing, may enclose scripts/Verilog, and can depend on parameters or shell state. |
| Spectre `name reliability ... { ... }` | Exclude the complete block, with nested braces and quoted strings/C comments handled. An opening brace on a later line and a continued header are recognized. The shorthand `reliability{...}` is also shielded conservatively; the documented simulator form includes a name. | Cadence *Virtuoso Spectre Circuit Simulator Reference*, 11.1, September 2011, “Reliability Analysis”, pp. 257–264 ([manual mirror](https://studylib.net/doc/8301508/virtuoso-spectre-circuit-simulator-reference)); RNL §2.2 explicitly excludes this language region. |
| Spectre `ahdl_include` | Do not load the referenced Verilog-A file as a netlist. | The same Cadence reference, “Verilog-A”, describes `ahdl_include` as loading separately written Verilog-A modules. |
| Spectre encrypted envelopes | Exclude from `//pragma protect begin_protected` through `//pragma protect end_protected`, allowing whitespace after `//`, including an envelope beginning on line 1. Plaintext `protect`/`prot` … `unprotect`/`unprot`, and SPICE `.protect`/`.prot` … `.unprotect`/`.unprot`, remain readable with warnings and unwritable. | Cadence *Spectre Circuit Simulator User Guide*, 5.1.41, July 2004, “Encryption”, p. 246 ([manual mirror](https://web.eecs.utk.edu/~dbouldin/protected/spectreuser.pdf)); Cadence reference 11.1, “encryption”, p. 332. |
| C preprocessing directives | Exclude directive lines and their backslash continuations. Track `#if`/`#elif`/`#else`/`#endif`; diagnose unresolved `#ifdef`/`#ifndef` and expressions. Ordinary netlist statements outside those regions remain readable. | Cadence reference 11.1, command options `-E`, `-D`, `-U`, `-I`, p. 21; [Cadence explanation of `-E`](https://community.cadence.com/cadence_technology_forums/f/custom-ic-design/12181/how-to-use-the--e-command-line-argument); GNU CPP manual, [initial processing](https://gcc.gnu.org/onlinedocs/cpp/Initial-processing.html) and [conditional syntax](https://gcc.gnu.org/onlinedocs/cpp/Conditional-Syntax.html). |
| Unselected libraries | Exclude unselected SPICE `.lib` and Spectre `section` bodies. Writing a selected section stays before its closing `.endl`/`endsection`, rather than at file EOF. | [WRspice 4.3.22 §2.3.7](https://wrcad.com/manual/wrsmanual/node25.html); RNL §2.2 and §2.8. |

The manuals identify simulator syntax. RNL §2.2 supplies the separate rule that these
regions cannot carry RNL and that tools must not write there. The implementation was
not tested by running the commercial simulators. Archived Cadence manuals are cited
with their editions; they are not evidence of every later release's syntax.

## Explicit limits and conservative cases

The generic `spice` adapter does not select one simulator/version. It recognizes the
listed WRspice forms in both SPICE readers as excluded regions; this does not assert
that Spectre itself accepts WRspice-specific statements. Dot conditionals are distinct
from Spectre's ordinary structural `if` syntax, which this change does not reinterpret.

Literal conditions are deliberately narrow: a finite decimal numeric literal, optionally
parenthesized, or equality of two such literals for dot conditionals; a decimal integer
literal for CPP. Unrepresentable numeric values remain unresolved. Parameters, macro
expansion, include-path definitions, command-line definitions and shell variables are
not evaluated. An unknown condition excludes its uncertain branches with a warning,
as §2.2 requires. It does not choose a branch from a guessed simulator environment.
Even known active branches remain unsuitable for new output that should survive a
later change of simulation conditions. A subcircuit entirely inside a conditional
therefore has no unconditional insertion place, and an editor request for it is refused
with the region's line range.

No external preprocessing command is executed. Files using more elaborate macros,
conditional expressions, preprocessor directives inside script languages, or a different
encryption envelope require a dialect-specific extension or an already preprocessed
netlist. These syntaxes are **not claimed as supported**. Existing `#` shell comments
inside control scripts are kept inside the script; they do not start CPP state.

A missing `.endc`, `.endv`, closing reliability brace, encryption terminator or `#endif`
keeps the region excluded through EOF and emits a warning. This preserves uncertainty
instead of treating the remaining text as ordinary carriers. There is no automatic
repair of malformed host syntax. `.newjob` title classification does not add a separate
multi-job circuit model; users should read separate circuit decks for independent jobs.
The rule excluding everything after the main `.end` continues to follow RNL §2.2.

## Implementation and regression evidence

[`Regions`](../crates/rnl-host-spice/src/regions.rs) owns host syntax and is used by
both adapters' `Reader::read_file`. The dialect-neutral runtime receives only line
ranges in [`HostDocument::excluded`](../crates/rnl-runtime/src/host.rs).
`HostDocument::unwritable_at` combines those ranges with readable protection regions.
Unselected library spans are stored separately with the file level that skipped them;
`unwritable_in` adds only that reading's spans. Reading a library both whole and through
a selected section therefore does not make the selected section unwritable. Actual
script, conditional and protection ranges still apply in every reading. A requested
section that does not exist is diagnosed and refuses record insertion rather than
placing output outside the selected text. Header movement keeps the target reading
through destination selection and exclusion checks, so a whole-file include cannot
move a selected section's header before its opening delimiter.
[`writeback::record_line` / `free_place`](../crates/rnl-runtime/src/writeback.rs)
find a legal insertion place in the same level; append, generated replacement,
header creation/movement, and version insertion consult the same exclusion data.
No place in that level means no generated record, with a diagnostic from the solver,
or a refused editor request. A header may move above an excluded region, or after a
closed envelope that starts on the first line, while remaining the first RNL record.

[`host_regions.rs`](../crates/rnl-cli/tests/host_regions.rs) covers script/Verilog
extraction in both readers, exact terminators, foreign-language include isolation,
reliability comments/quotes/nesting/continuations, main versus included `.newjob`,
selected/inactive/unknown conditional branches, CPP continuations, encrypted envelopes,
selected library insertion and commented section boundaries in both readers, LF/CRLF
preservation, missing final newline, and rereading
appended/generated output. `selected_library_headers_move_before_subcircuits_with_real_reader_boundaries`
checks late headers in selected-only and whole-plus-selected readings, both include
orders, all three host paths, and readable protection regions; it covers the solver
header prepass and combined generated writeback, rereading and repeated writes. The editor's
[`editor_records_and_headers_stay_outside_unterminated_host_regions`](../crates/rnl-edit/src/tests.rs)
and `the_editor_explains_when_a_conditional_subcircuit_has_no_safe_write_place` exercise
its public write path. Existing `protected_regions` tests retain the distinction
between readable plaintext protection and encrypted text.

Focused commands: `cargo test -p rnl-cli --test host_regions` and
`cargo test -p rnl-edit`. Repository-wide validation is recorded in the
[supported profile](supported-profile-0.3.md#validation).
