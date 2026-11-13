# Modern conformance examples

These are RNL 0.4 counterparts of the examples in `cases` and `cases-0.3`.
The `from-0.2` and `from-0.3` directories retain their original filenames so
each test's purpose and migration can be traced. The existing corpora continue
to test their declared language versions independently during development.

Each case has an effective-model reference (`.expected.json`) and an editor
snapshot reference (`.snapshot.json`). `RNL_BLESS_0_4=1` updates only these new
references. The existing blessing variables do not update them. The modern
SPICE/Spectre pairs live in `../equivalence-0.4` and must produce equal models
after removing only the same file-location fields as the original pairs.

The migration preserves host circuit records and the examples' purpose:

- Directed constraints replace axis-only forms. Collections in a constraint
  expand into combinations, and empty items are omitted; they have no centre.
- Usages have independent points and their body coordinates are relative.
  Explicit placements preserve the original device layouts where applicable.
- Named pins, terminal-to-pin conversion, generated record numbering, wire
  replacement, priority and failure recovery remain exercised.
- The formal standard collection supplies the new symbols and fill rules.
  Local fixture definitions preserve intentionally empty structural roles
  where a new standard definition would add unrelated assertions or facts.
- Unconnected top-level net annotations use net labels. Generated port
  selection is explicit, and overlapping coordinates alone do not connect pins.
- Deliberate errors and violations remain negative tests. The unsupported
  version case still declares `9.0`; it is not a valid 0.4 example.

The headerless-main example is retained in the original corpus until the tool's
default language switches to 0.4. Its modern counterpart must remain headerless
when it is enabled, to preserve the behavior being tested.

The examples use Manhattan wires, with explicit corners where needed to preserve
pin exit directions and avoid crossing symbols. The 0.4 standard base style uses
bold text; its current arrows have a compact shape independent of ordinary arrows.
The font-fallback and explicit-style examples retain their dedicated coverage.

Review PNGs use the complete, hash-pinned Noto Sans SC faces and the same renderer
metrics so Chinese text is visible. The core's metrics and historical references
are preserved. The review renderer is not part of this corpus.
