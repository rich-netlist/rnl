# Toolkit support for RNL 0.3, revision 2

The initial audit covered the toolkit at base commit `20c93440c314a8838e17cd8ae76b78fb9ae06ef0`
plus the audit fixes, against the published English standard tagged
[`spec-0.3-r2`](https://github.com/rich-netlist/rnl/tree/spec-0.3-r2/spec/0.3/en)
(`0a308e55b74a7994202ff8cb061800ea04dcd2e8`), dated 2026-10-18.
The English chapter files and `spec/rnl-words-0.3.json` in the audited checkout are
identical to that tag. No standard text is changed.

The toolkit implements all **46 standard keywords and their 98 fixed parameters**,
as well as the record fields, style properties and other standard words. This does
**not** make it fully conformant: the host-region, error-recovery,
encoding and drawing limitations below remain. Language 0.2 is also read for compatibility;
new solver/editor output uses 0.3, with a standard-package pin when upgrading 0.2.

## How to read the evidence

| Audit | Coverage |
| --- | --- |
| [Foundations and file boundaries](profile-foundations-audit.md) | Every subsection of chapters 1–8 and 16–17, plus annexes: normative effect, implementation file/function, existing test or explicit coverage gap. |
| [Definitions, drawing and constraints](profile-semantics-audit.md) | Every subsection of chapters 9–15, with implementation and test anchors, limitations and policy choices. |
| [Complete vocabulary inventory](profile-vocabulary-audit.md) | Every keyword/parameter, record field, style property, kind, slot category, sub-object, attribute and truth value. |
| [Dedicated roundtrip corpus](../tests/roundtrip/README.md) | Six fixture-backed cases, reviewed records and effective models; 18 runs across LF, CRLF and missing-final-newline inputs. |

“Implemented” in the matrices means the rule has an inspected implementation and the
listed evidence; it does not assert exhaustive testing of every combination. “Partial”
identifies a known missing behavior or boundary. “Choice” is behavior the standard leaves
to implementations. “Informative” is explanatory text, examples or future work rather
than another feature obligation. A test for a whole keyword is not a test of every one
of its optional values. The vocabulary table says where only source evidence was found.
The old conformance corpus explicitly reads 0.2; it is useful shared-code evidence and
is distinguished from the 24 cases of 0.3 and dedicated 0.3 regression tests.

## Supported paths and boundaries

- Parsing, names, sets/selections, definitions/packages/aliases, numbered references,
  appearance and constraint evaluation have implemented paths, with the exceptions in
  the matrices. SPICE and Spectre adapters are partial netlist readers; they are not
  complete simulator parsers.
- The renderer/view layer displays explicit positions, symbols, wires, text and diagnostics.
  Rendering defaults, font metrics and placement of text left free are implementation
  policy; a view is not a proof that a solver has satisfied all constraints.
- The native solver resolves ranked facts, reports discarded/undecided ones, and writes
  its decisions as generated records. The writeback interface can express numbered
  placements, keyed text and wire rewrites even where the built-in solver does not choose
  those decisions yet. Automatic rewriting of authored auto wires, free-label optimization
  and warm-start consumption remain outside the current solver's implemented feature set.
- `rnl-edit` writes new netlists without existing RNL records and explicitly refuses
  merging edits into records already present. The solver's generated-record replacement
  is a separate supported path. This audit does not claim a complete editor.
- UTF-8 (including BOM) and Latin-1 input are supported by the CLI provider. Arbitrary host
  encodings and arbitrary precision numbers are not. Symbols support up to 64 symbol levels, including the outer symbol; deeper graphs fail with an explicit diagnostic before rendering or solver export. Uncached definition expansion is separately limited to 64 active symbol bodies, including fragment dependencies; hitting that limit fails the active chain and leaves subsequent independent definitions usable.
  Numeric conversion uses binary64; overflow and underflow to zero now fail explicitly.

### File-version boundaries (16.1)

The first correctly placed header of each file (including each selected library
section) determines its explicit language version. Otherwise the file inherits the
nearest declared version; a main file defaults to 0.3. A subcircuit follows its own
source file. An explicitly supported include is interpreted even when its parent
uses an unsupported version; its unversioned includes then inherit its version.

An unsupported file contributes no names, aliases, DEVICE declarations, scopes,
rules or package imports, including imports written before VERSION. It remains a
host hierarchy link: visible declarations and package pins from supported ancestors
still reach supported descendants. Thus an unread file cannot shadow a visible name
or pin a package, and does not hide an earlier supported ancestor. Implicit standard
packages follow each supported file's effective version. The solver writes only
supported files and preserves unsupported source text.

## Corrections and regression evidence

| Requirement | Correction | Regression evidence |
| --- | --- | --- |
| 2.8 (F-02) | Repeated known record-head fields are diagnosed once and treated as absent; the body and other fields survive. Ordinary keyword duplicates still fail their member. Classification and writeback use the same absent-field rule. | `header.rs::{repeated_record_head_fields_are_absent_without_losing_the_body,a_repeated_head_field_keeps_other_fields_and_keyword_duplicates_still_fail}` |
| F-05, 5.3, Annex A | CI compares actual runtime vocabulary with the published list; handler defaults and structured rules are checked by execution, and unhandled schema fields fail. | [`vocabulary_parity.rs`](../crates/rnl-cli/tests/vocabulary_parity.rs); detailed scope in the vocabulary appendix |
| 16.1 (F-03) | Supported explicit include versions restart inheritance below unsupported files. Unread files contribute no RNL declarations or packages; supported ancestors remain visible. | [`include_versions.rs`](../crates/rnl-cli/tests/include_versions.rs), including selected library sections and child-only writeback |
| 2.4, 3.5 | A record head separated from `@RNL` by whitespace/comment fails the whole record. | `syntax.rs::tests::a_record_head_separated_from_the_entry_fails_the_whole_record` |
| 3.7 | Out-of-range numeric conversion reports an implementation limit instead of infinity or silently becoming zero. | `syntax.rs::tests::out_of_range_numbers_fail_the_member_without_becoming_infinite_or_zero` |
| 5.3, 6.4, 9.2.4 | PIN validates every bus member and integral positive terminal index; SELECT-TERMINAL rejects invalid terminal types. | [`profile_validation.rs`](../crates/rnl-cli/tests/profile_validation.rs) |
| 5.3, 6.12 | Header/package parameter types are checked before defaults; DEVICE rejects invalid terminal-name array entries. | `profile_validation.rs::header_parameters_reject_wrong_types_before_using_their_defaults`, `malformed_device_names_do_not_shift_or_rename_terminals` |
| 16.4 | Keywords with no parameters reject supplied parameters before applying their effects. | `profile_validation.rs::parameterless_keywords_reject_parameters_before_changing_objects` |
| 3.6, 6.4, 6.12 | Terminal selections respect Spectre case and each SPICE-mode instance's case rule. | [`profile_semantics.rs`](../crates/rnl-cli/tests/profile_semantics.rs) |
| 9.2.5 | A Bezier arrow uses the next distinct control point when the adjacent control coincides with its endpoint. | `rnl-render/src/lib.rs::tests::bezier_arrows_skip_coincident_control_points` |
| 11.8 | Shorter members of a heterogeneous bundle no longer cause out-of-bounds node access. Existing positional correspondence is retained; see the open name-correspondence issue below. | `profile_semantics.rs::a_shorter_bundle_member_contributes_only_its_existing_terminals` |

## Subsequent focused corrections

- **Symbol declaration order (9.2.1, 9.2.7, 11.6):** graphics, fixed text and nested symbols now carry one declaration position through inclusion and rendering. The [regressions](../crates/rnl-cli/tests/symbol_drawing_order.rs) check repeated fragment inclusion, transformed symbols, SVG order and more than 100 primitives.
- **Deep nested symbols (9.2.3, 9.2.7, 13.3):** geometry, inferred bounds and fixed text traverse the same accepted nested usages. The silent depth-8 cutoffs are removed. Definition validation reports cycles and the implementation limit of 64 symbol levels; this limit includes the outer symbol and applies equally with explicit and inferred boxes. [Regressions](../crates/rnl-cli/tests/deep_symbols.rs) cover 13 transformed wrappers, expanded fixed text, model/solver bounds, explicit boxes, the accepted boundary and error recovery beyond it. A separate pre-expansion guard bounds uncached forward references; reverse-declared chains of 64, 65, 256 and 1,000 levels check the boundary, direct outermost use and subsequent independent definitions.
- **Named symbol pins (3.6, 6.4, 9.2.4, 9.2.7):** reusable symbols preserve exact terminal spelling. The owning instance decides matching: Spectre and RNL roles are case sensitive; SPICE, including SPICE mode inside a Spectre file, merges case variants with the later pin winning. A cached host-specific pin view preserves other bus members and the original symbol identity. Subcircuit block-symbol metadata uses its host level case rule; owner-free package catalogs retain exact definition keys. [Regressions](../crates/rnl-cli/tests/symbol_pin_case.rs) verify separate nets/wire endpoints, partial bus overrides and reuse of one SPICE-defined symbol by both host modes.

## Remaining issues, in priority order

These are open work, not silently accepted extensions. The [roadmap](roadmap.md) remains
the central work list. P1 denotes wrong interpretation or file-preservation risk; P2
is a bounded semantic/geometry defect; P3 is advisory behavior or assurance coverage.
The linked matrices contain the exact source anchors.

| Priority | Remaining issue | Evidence / practical limit |
| --- | --- | --- |
| P1 | **Excluded host regions (F-01, 2.2).** RNL inside Spectre `reliability {}` or SPICE `.verilog` is read; `.newjob` titles and preprocessing are not comprehensively classified. | CLI probes drew text inside both excluded regions. Host adapters need context-aware shielding and write-placement policy. |
| P1 | **Encoding preservation (F-04, 3.1).** CLI writing can silently change Latin-1 to UTF-8 when new characters do not fit; strip-generated directly writes decoded text. | `solve.rs::encode_as` has an explicit UTF-8 fallback (even tested as current behavior). Desktop encoding checks differ. A consistent refusal/conversion policy needs an API decision. |
| P2 | **Bundle terminal correspondence (11.8).** Heterogeneous members use first-member terminal ordinals, not name-based correspondence when names/orders differ. | The crash is fixed; the broader mapping remains partial and needs regression cases for differing names/orders. |
| P3 | **Advisory diagnostics (F-08).** Non-ASCII simulator validation, some forward-name typo hints and Spectre SPICE-mode post-`.end` warnings are incomplete. | SHOULD/advisory behavior; see the foundations matrix, distinct from mandatory interpretation failures. |
| P3 | **Optional-value coverage.** Several optional values and negative combinations still lack dedicated behavioral assertions. | F-05 vocabulary parity is now maintained in CI; it does not establish complete semantic conformance or exhaust the cross product of parameters and contexts. |

## Decisions and standard questions

1. **Generated objects (10.5, 14.8):** standard fills exclude objects written explicitly;
   arbitrary custom generators may generate another object on reread. The standard already
   describes that result. Decide whether the solver should detect such cases and leave
   their decisions unsolved, rather than promising a general idempotent writeback. Current
   roundtrip fixtures intentionally use a wire that suppresses regeneration.
2. **Preservation (16.4 versus 17):** clarify whether preserving unknown/failing statements
   when rewriting owned records also applies when the solver replaces generated records
   as a whole. Authored records and host text outside that replacement must be preserved.
3. **Bundle mapping and computed boxes:** specify the desired behavior for absent/differently
   named terminals in heterogeneous bundles, and whether conservative curve boxes satisfy
   the computed-box wording. Keep these questions separate from the confirmed panic and
   nested-order defects.
4. **Prose/vocabulary alignment:** WIRE accepts a wire for rewriting in 13.1 while the
   signature's net slot is typed as net; PIN's terminal type also accepts arrays as specified
   by 9.2.4. Implementations must consult the normative prose as well as the JSON. No change
   to the published standard is proposed by this PR.

## Validation

The initial audit checks passed on 2026-10-18 (follow-up PRs add their own
regressions and run the Rust CI gates):

- `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings`.
- `cargo test --workspace`: 716 tests passed, none failed or ignored. This includes the
  existing conformance corpus, all new regressions and the six roundtrip fixtures with
  all 18 line-ending variants.
- `node bindings/runtime-wasm/tests/core.mjs target/wasm-test`, after building with
  `--features test-panic`: all 16 tests passed, including panic and out-of-memory recovery.
- `node bindings/runtime-wasm/tests/equality.mjs` and its `--snapshot` mode, after a
  normal Wasm build and `cargo build -p rnl-cli`: all 84 files and the complete catalog
  matched native output in both modes (90 and 174 Wasm builds respectively).
- `reuse lint` and `git diff --check`; all relative audit/roundtrip Markdown links resolve.

All cargo operations run serially with `CARGO_BUILD_JOBS=4`, without `--release`, after
checking for other cargo/rustc processes and at least 8 GiB of free physical memory.
The WebAssembly builds use the repository's documented `wasm` profile. Browser tests
are outside this change; no Playwright/browser installation is needed for these checks.
