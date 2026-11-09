# Upgrading RNL to 0.4

The upgrade library reads a supported source document before making changes, edits only RNL records, and reads the proposed result with the same core. Its result contains rewritten texts and a structured report; it performs no filesystem writes. The CLI saves those texts using the same encoding-preserving writer as `rnl solve`.

An upgrade is successful only when all changes are supported, target interpretation succeeds, and comparison of objects, networks, positions and facts finds no differences. Unsupported statements retain their original text with an explanatory comment. A successful parse alone does not establish equivalence. During development, the core may report target features that it has not implemented yet; these remain failures in the report.

## Implementation status

The implementation changes language headers and standard package references, renames ROUTE, makes implicit recursive selections explicit when their rule has no mixed recursion, and separates references from adjacent sets. Standard usage terminal paths become named pins when package provenance and the binding are known; supported standard role renames update both arguments and references. Local overrides keep their own names. With a verified, uniform series 0.3 fill environment, removed routing sets become fresh file-owned sets with the same selection and exclusions, and automatic port tags get an explicit generation rule. Included references share that entry policy; aliases, inline overrides and mixed package environments are left for contextual migration. A wired terminal selection used directly by TEXT becomes electrical filtering followed by SELECT-PIN; selections that escape through bindings or feed electrical consumers remain explicitly unsupported. Distance intervals that cannot be carried across directly are retained with a specific explanation. The inventory below is the remaining migration plan, not a claim that every rule is implemented. Known unsupported constructs receive notes and report entries; target diagnostics and structural differences also prevent success. The native writer preserves the supported UTF-8 (including BOM) and Latin-1 encodings.

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
| ORDER/DISTANCE with x or y; DISTRIBUTE axis | Explicit direction justified by positions; report an undetermined direction | Context | 14.3-14.5 |
| Negative DISTANCE interval | Reverse direction only when an equivalent nonnegative interval exists | Context | 14.5 |
| Nested/held sets in constraints | Preserve old flat membership where possible; report centre/bounds semantics | Context | 5.2, 14.1 |
| Set ::position or set-wide PLACE | No direct equivalent; retain and report | Unsupported | 5.2, 6.5, 11.1 |
| ORIENT-RELATION with two sets | Pairwise facts when original semantics and conflict treatment are preserved | Context | 14.9 |
| Ordinary definition's absolute coordinates | Lift absolute facts to each usage when its expansion and priority can be preserved | Context | 9.4, 12.1 |
| Usage terminal reference | Corresponding named symbol pin | Context | 6.4, 9.2.4, 12.1 |
| Custom symbol pin binding roles by number | Name the pin and bind its corresponding role | Context | 9.2.4, 11.3 |
| Names bound to PIN or fixed TEXT results | Preserve separately named input points; give pins and text their own names | Context | 9.2.4, 9.2.5 |
| Reused local names or names inherited from symbols | Rename distinct results and references before conflicting inclusion | Context | 9.2.7 |
| Repeated PIN bindings in a symbol | Remove displaced bindings from earlier pins; retain points still referenced | Context | 9.2.4 |
| Symbol inheritance overriding a terminal's pin | Materialize equivalent nonduplicating pins, otherwise report | Context | 9.2.7 |
| SELECT-TERMINAL wired | SELECT-PIN, preserving electrical consumers where applicable | Context | 10.1, 10.2 |
| Other terminal selections | Check the effect of selecting all electrical terminals | Context | 10.1 |
| Selection-fed WIRE | ROUTE-OBJECT only when topology and styling can be preserved | Context | 13.1, 13.7 |
| Selection-fed BUNDLE | Explicit resolved members when independent of future selection results | Context | 11.8 |
| Literal markup gaining meaning in 0.4 | Preserve text intent or report changed rendering | Context | 13.6 |
| Spectre transformer default terminal names | p_plus/p_minus/s_plus/s_minus to p1/n1/p2/n2 | Context | 6.12 |
| Numeric DEVICE/port names | Resolve positions and report impossible name preservation | Context | 6.4, 6.12 |
| Standard drawing-only role p | at for the matching standard definition | Mechanical | Standard series |
| Standard role names with underscores | Their matching definition's current role names | Context | Standard series |
| DIFFERENTIAL_PAIR | DIFFERENTIAL_PAIR_N or DIFFERENTIAL_PAIR_P with left/right/tail | Context | Standard series |
| MIRRORED roles a/b | left/right (or vertical variant when equivalent) | Context | Standard series |
| CHAIN, SUPPLY_HIGH, SUPPLY_LOW | No replacement definition; retain and report | Unsupported | Standard series |
| FLOW with a positional direction | Preserve meaning and add corresponding explicit positional facts | Context | Standard series |
| WIRE_NET_SET modification / route | Explicit file-owned selection and exclusions, preserving user edits and ROUTE-NET | Context | Standard series, 13.7 |
| Automatically generated port tags | Explicit selection and generation, with exclusions for supplies, grounds and existing tags | Context | Standard series, 12.4 |
| Ground/supply attached to a terminal | Explicit constraints that preserve its original point | Context | 12.1, 12.3 |
| Generated numbered references | Resolve old object identity; report when it cannot be carried across | Context | 5.9 |
| New assertion violations or changed default fill | Report every target diagnostic and semantic difference | Verification | 7.6, 15.2-15.4 |

Additive language features (TABLE, orientation lists, assertions, solver settings, pin orientation, text rotation and math) require no rewrite unless an old spelling gains different meaning. Clarifications still participate in validation. Do not alter host netlist lines to hide a difference.

## Solver records

After validating the source, migration discards its solver-generated records. The report marks each affected file with `needs_solve: true`; this requests a new solve and does not count as a conversion failure. Verification compares the authored inputs on both sides, so discarded solver decisions do not create artificial differences.

## Report

Each change records its rule, file, original line, before and after text. Unconverted spans record a reason. Target diagnostics and comparison differences are separate lists. Unsupported or different output has exit status 1; unreadable input has exit status 2 and writes nothing.
