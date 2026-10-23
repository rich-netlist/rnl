# Native output encoding (§3.1)

The CLI and desktop share [`rnl-cli::codec`](../crates/rnl-cli/src/codec.rs).
They read valid UTF-8 (removing one encoding BOM from the in-memory text), otherwise
Latin-1. The default output choice, `preserve`, writes UTF-8 with its original BOM
state or Latin-1 as appropriate. A source created in memory uses UTF-8. If unmarked
UTF-8 output would begin with a text U+FEFF, the codec adds a separate encoding BOM
so reopening cannot mistake that character for the BOM and remove it. This necessary
exception also applies when an edit introduces leading U+FEFF; other new UTF-8 sources
have no BOM.
Other legacy encodings are not detected or claimed as supported; an ASCII-only
source is indistinguishable from UTF-8 and is treated as UTF-8.

If a new character cannot be represented in Latin-1, saving fails with a message
naming the character and asking for explicit UTF-8 conversion. There is no silent
fallback. A named disk source that has disappeared or cannot be read also fails in
preserve mode: its encoding cannot be established. A clipboard source is tracked
separately and uses UTF-8; an explicit UTF-8 choice can recreate a missing source.
All rewritten files in one solve/editor save are encoded before any is
written, so this error leaves every destination unchanged. IO errors during the
subsequent writes can still leave a partial batch, which the existing save result
reports.

## Choosing UTF-8

CLI examples:

```sh
rnl solve top.sp -o solved --encoding utf8
rnl solve top.sp --in-place --encoding=utf8
rnl strip-generated top.sp -o authored.sp --encoding utf8
```

`--encoding preserve` is the default; `utf-8` is also accepted as a spelling of
`utf8`. The option is accepted only by `solve` and `strip-generated`. UTF-8 conversion
normally adds no encoding BOM and preserves the decoded text and its line endings.
When the text begins with U+FEFF, it adds a separate encoding BOM to protect that
character on reopening. An original single encoding BOM is therefore removed; an
encoding BOM followed by a text U+FEFF remains two encoded U+FEFF values.
`strip-generated` snapshots the actual entry bytes before parsing. A readable disk
file takes precedence over an exact bundled-package lookup; the captured disk text
is parsed through its absolute physical path so a relative `<bundled>/...` filename
cannot be redirected into the shared reader's virtual package namespace. Relative
includes still resolve from the entered directory. Virtual packages use their UTF-8
text, independent of the adapter's relative display names.
Includes copied unchanged to a solved folder retain their original bytes: conversion
applies to the files actually rewritten, not to every dependency.

In the desktop, ordinary **File › Save**, **File › Save As**, and **Solve › Save
Solved Copy As** retain the source encoding. The explicitly named **File › Save
Changes Using UTF-8**, **File › Save As UTF-8**, and **Solve › Save Solved Copy As UTF-8**
commands choose conversion. Save Changes Using UTF-8 saves only files with unsaved edits, including
included files (use Save As UTF-8 to convert an unchanged netlist), and retains the existing changed-on-disk overwrite/reload checks.
Save As writes the opened netlist and uses the original source's encoding, even if
its destination already exists in a different encoding. A cancelled save dialog
writes nothing. An explicit conversion choice is retained if the user subsequently
chooses Overwrite in the changed-on-disk dialog.

The public `write` / `save_copy` functions retain the default-preserve behavior;
`write_with_encoding` / `save_copy_with_encoding` accept `OutputEncoding`. Callers
saving a purely in-memory solve select `Utf8`; the desktop does this automatically
only for its tracked clipboard source. The old
`solve::encode_as` convenience function now returns `Result<Vec<u8>, String>` so a
caller must handle an unrepresentable character. Desktop IPC accepts the optional
`encoding` value `preserve` or `utf8`; omission means preserve.

## Regression evidence

- [`codec.rs`](../crates/rnl-cli/src/codec.rs): byte roundtrips for Latin-1, UTF-8,
  BOM, a leading text U+FEFF distinct from the encoding BOM, CRLF and missing final
  newline; Latin-1 rejection and explicit conversion. Explicit conversion and new or
  edited unmarked UTF-8 sources preserve leading text U+FEFF when reopened.
- [`tests/encoding.rs`](../crates/rnl-cli/tests/encoding.rs): multi-file refusal
  before any write; single-file and folder solved-copy refusal and conversion;
  untouched include bytes; real `strip-generated` CLI runs across UTF-8/Latin-1 and single/double-BOM inputs,
  LF/CRLF and final-newline variants; bundled-package lookup and physical-file
  precedence (exact virtual-path shadowing where the filesystem allows it); real
  `solve` CLI with both option spellings.
- [`desktop/src/files.rs`](../apps/desktop/src/files.rs): batch preflight, explicit
  UTF-8, source-based Save As, refusal without overwriting an existing destination,
  and preservation of the source BOM.
- [`edit.spec.js`](../tests/viewer/specs/edit.spec.js) and
  [`solve.spec.js`](../tests/viewer/specs/solve.spec.js): explicit UI commands pass
  the encoding choice to the shell, including the overwrite retry. Native menu
  tests check that these choices appear in the desktop menu.

Focused verification commands (run serially with `CARGO_BUILD_JOBS=4`):

```sh
cargo test -p rnl-cli --test encoding
cargo test -p rnl-cli codec
cargo test --manifest-path apps/desktop/Cargo.toml
# From tests/viewer; its setup builds the CLI, so it must share the same build lock:
npx playwright test specs/edit.spec.js specs/solve.spec.js specs/solve-settings.spec.js --project chromium
```

The profile's [validation record](supported-profile-0.3.md#validation) describes the
repository-wide checks. These cases establish the stated UTF-8/Latin-1 policy, not
support for arbitrary host encodings or atomic filesystem transactions.
