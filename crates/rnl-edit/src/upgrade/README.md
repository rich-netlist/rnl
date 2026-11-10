# Upgrading RNL to 0.4

The upgrade library reads a supported source document before making changes, edits only RNL records, and reads the proposed result with the same core. Its result contains rewritten texts and a structured report; it performs no filesystem writes. Without a write target, `rnl upgrade FILE` previews the migration and can save a JSON report with `--report`. With `-o DIR` or `--in-place`, the CLI saves the proposed texts using the same encoding-preserving writer as `rnl solve`.

An upgrade is successful only when all changes are supported, target interpretation succeeds, and comparison of objects, networks, positions and facts finds no differences. Unsupported statements retain their original text with an explanatory comment. A successful parse alone does not establish equivalence. Comparison includes unplaced symbol geometry, labels and pin networks; algebraically reversed single-point distance intervals use one canonical signed form. During development, the core may report target features that it has not implemented yet; these remain failures in the report.

## Optional approximations

`--approximate` is off by default. It opts into polarity-based differential-pair replacements, choosing an explicit axis direction from source positions, authored directed constraints or written order, and accepting the stronger combination semantics of held sets in ORDER. Each choice is reported in `approximations` with its original and proposed text and a `risk` explanation; the terminal summary counts it separately from exact edits. ALIGN/DISTRIBUTE on multi-member held sets remain manual. Approximation does not suppress target errors, assertion violations or comparison differences.

Verified directed collection minima and existing standard main-point ties no longer need review. Old explicit stub applications retain their default gap by writing it out. Additional main-point ties use ALIGN x/y, with fresh names only when needed. Assertion-only package changes are `notices`, and actual violations are also collected in `assertion_violations`. See [the definition audit](STANDARD-DEFINITIONS.md) for each standard definition and for the limits of each rule.

## Implementation status

The implementation changes language headers and standard package references, renames ROUTE, makes implicit recursive selections explicit when their rule has no mixed recursion, and separates references from adjacent sets. Standard usage terminal paths become named pins when package provenance and the binding are known; supported standard role renames update both arguments and references. Local overrides keep their own names. With a verified, uniform series 0.3 fill environment, removed routing sets become fresh file-owned sets with the same selection and exclusions, and automatic port tags get an explicit generation rule. Included references share that entry policy; aliases, inline overrides and mixed package environments are left for contextual migration. A terminal selection used directly by TEXT becomes electrical filtering followed by SELECT-PIN; selections that escape through bindings or feed electrical consumers remain explicitly unsupported. Nonpositive directed distance intervals between individual points reverse direction and bounds; zero single-point axis distances can supply an equivalent explicit direction. Coordinates are discardable facts and do not justify choosing a direction. Crossing intervals, set extents and other undirected constraints are retained with a specific explanation. The inventory below distinguishes implemented rewrites from constructs retained for review; the latter never count as successful conversions. Known unsupported constructs receive specific notes for absolute definition coordinates, nonequivalent held collection constraints, symbol pin overrides/inclusion, local names, fixed-text point results, numeric DEVICE names, selection-dependent creation and newly meaningful markup. Usage notes describe a lost main-role point relation or a removed/changed standard definition, rather than flagging every application; target diagnostics and structural differences also prevent success. Verified ordinal standard-symbol pins receive explicit terminal maps for host assignments, direct host sets, block symbols and instance selections, matched by unique local coordinates and directions. Simple local symbols receive fresh pin names, and old PIN/fixed-TEXT result names remain bound to their input points. Known named usage-role pins can then replace usage terminal references. Inherited or displaced pins, ambiguous scopes and unknown terminal bindings remain explicit review cases. The native writer preserves the supported UTF-8 (including BOM) and Latin-1 encodings.

Known series 0.2 names and merged packages migrate only with bundled provenance and no local name capture. A path-loaded package with a standard name but different contents remains an owned package: preserve its path and package version, and upgrade its language with the file group. Numeric host-port names and the earlier Spectre transformer terminal names use their verified host positions. Named collection dependencies are traced through aliases and membership edits, so unsupported set extents and selection-fed creation receive notes even when the set is held in a variable. Alias environments, inline overrides of merged packages, definition-local object paths and numeric PIN names remain explicit review cases.

## Rewrite inventory

Sections refer to RNL 0.4. Standard package changes refer to the bundled series 0.3 and 0.4.

| Source form | Target form / handling | Class | Basis |
| --- | --- | --- | --- |
| VERSION 0.2 or 0.3 | VERSION 0.4 | Mechanical | 16.1 |
| Implicit standard series | Explicit USE-PACKAGE for series 0.4 | Mechanical | 16.3 |
| Standard USE-PACKAGE / USE-ALIASES version | Available matching package of series 0.4; preserve inline overrides | Mechanical | 16.2, 16.3, 16.5 |
| Included files, own package files, inline packages | Apply rules using each source's effective language | Context | 2.8, 16.1, 16.2 |
| Reference immediately followed by a separate set | Insert whitespace so it cannot become member selection | Mechanical | 5.2.3 |
| ROUTE | ROUTE-NET with original parameters | Mechanical | 13.7 |
| Selection without recursive | Add recursive=true; report mixed false/implicit rule propagation | Context | 10.1, 10.4 |
| ORDER/DISTANCE with x or y; DISTRIBUTE axis | Only zero single-point distances gain a fixed direction; retain other axes and report the missing choice | Context | 14.3-14.5 |
| Negative DISTANCE interval | Reverse direction only when an equivalent nonnegative interval exists | Context | 14.5 |
| Nested/held sets in constraints | Retain and report centre/bounds semantics, including named held sets | Context | 5.2, 14.1 |
| Set ::position or set-wide PLACE | No direct equivalent; retain and report | Unsupported | 5.2, 6.5, 11.1 |
| ORIENT-RELATION with two sets | Retain and report: splitting changes conflict treatment | Context | 14.9 |
| Ordinary definition's absolute coordinates | Retain and report the required origin/priority decision | Context | 9.4, 12.1 |
| Usage terminal reference | Corresponding named symbol pin | Context | 6.4, 9.2.4, 12.1 |
| Custom symbol pin binding roles by number | Name simple pins; retain unresolved numeric usage-role references for review | Context | 9.2.4, 11.3 |
| Names bound to PIN or fixed TEXT results | Preserve separately named input points; give pins and text their own names | Context | 9.2.4, 9.2.5 |
| Reused local names or names inherited from symbols | Retain and report ambiguous/reused local identities | Context | 9.2.7 |
| Repeated PIN bindings in a symbol | Retain and report displaced bindings; do not guess which points remain live | Context | 9.2.4 |
| Symbol inheritance overriding a terminal's pin | Retain and report inclusion and displaced-binding ambiguity | Context | 9.2.7 |
| SELECT-TERMINAL wired | Electrical filter followed by SELECT-PIN for direct TEXT; retain electrical consumers for review | Context | 10.1, 10.2 |
| Other terminal selections | Rewrite direct TEXT; report expanded membership for electrical consumers and traversals | Context | 10.1 |
| Selection-fed WIRE | Retain and report the required topology/routing choice | Context | 13.1, 13.7 |
| Selection-fed BUNDLE | Retain and report selection dependencies, including named aliases | Context | 11.8 |
| Literal markup gaining meaning in 0.4 | Retain and report changed rendering intent | Context | 13.6 |
| Spectre transformer default terminal names | Verified default host names become stable terminal positions | Context | 6.12 |
| Numeric DEVICE/port names | Resolve host port positions; report invalid numeric/repeated DEVICE names | Context | 6.4, 6.12 |
| Series 0.2 names and merged packages | Provenance-checked target names and explicit destination packages; report removed names | Context | Standard series |
| Standard drawing-only role p | at for the matching standard definition | Mechanical | Standard series |
| Standard role names with underscores | Their matching definition's current role names | Context | Standard series |
| DIFFERENTIAL_PAIR | Retain and report changed constraints and the required N/P choice | Context | Standard series |
| MIRRORED roles a/b | left/right (or vertical variant when equivalent) | Context | Standard series |
| CHAIN, SUPPLY_HIGH, SUPPLY_LOW | No replacement definition; retain and report | Unsupported | Standard series |
| FLOW with a positional direction | Retain and report the changed standard definition body | Context | Standard series |
| WIRE_NET_SET modification / route | Explicit file-owned selection and exclusions, preserving user edits and ROUTE-NET | Context | Standard series, 13.7 |
| Automatically generated port tags | Explicit selection and generation, with exclusions for supplies, grounds and existing tags | Context | Standard series, 12.4 |
| Ground/supply attached to a terminal | Preserve a directly written host terminal attachment through its net and explicit ALIGN x/y; report unresolved contexts | Context | 12.1, 12.3 |
| Generated numbered references | Discard validated solver records; authored numbered references fail source validation | Context | 5.9 |
| New assertion violations or changed default fill | Report every target diagnostic and semantic difference | Verification | 7.6, 15.2-15.4 |

Additive language features (TABLE, orientation lists, assertions, solver settings, pin orientation, text rotation and math) require no rewrite unless an old spelling gains different meaning. Clarifications still participate in validation. Do not alter host netlist lines to hide a difference.

## Solver records

After validating the source, migration discards its solver-generated records. The report marks each affected file with `needs_solve: true`; this requests a new solve and does not count as a conversion failure. Verification compares the authored inputs on both sides, so discarded solver decisions do not create artificial differences.

## Report

Each change records its rule, file, original line, before and after text. Unconverted spans record a reason. Informational notices do not fail migration by themselves; approximation records remain separate from exact changes. Target diagnostics and comparison differences are separate lists. Unsupported or different output has exit status 1; unreadable input has exit status 2 and writes nothing.

## Regression coverage

The integration tests in `../../tests/upgrade_*.rs` cover headers and includes, record-only edits, direction intervals, standard provenance, local symbols, terminal maps, legacy names, host terminals, collection dependencies, drawing selections and manual review reasons. `rnl-cli/tests/upgrade.rs` covers writing, reports, exit codes and source preservation. All fixtures are small independently authored examples.
