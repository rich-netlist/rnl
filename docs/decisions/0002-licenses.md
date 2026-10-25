# 0002. Licenses: MIT for code, CC-BY-4.0 for specification and documentation, CC0-1.0 for data

- Date: 2026-09-24
- Status: accepted

## Context

RNL is a personal project of its maintainer, who holds the copyright in the documents migrated from Analog Canvas and may relicense them. The goal is an open format that independent and commercial tools can implement and embed. Standard library content, such as symbol definitions, is copied into users' netlists, so it must not carry attribution or share-alike obligations.

## Decision

| Content | Paths | License |
| --- | --- | --- |
| Everything else, including code, configuration, and top-level files | everything not listed below | MIT |
| Specification text and project documentation | `spec/`, `docs/` | CC-BY-4.0 |
| Data: standard libraries, test corpora, and examples | `libraries/`, `tests/`, `examples/` | CC0-1.0 |

- `REUSE.toml` records the mapping. `LICENSES/` holds the full texts, taken verbatim from the SPDX license list. The root `LICENSE` carries the MIT text with the copyright line.
- The migrated documents are published here under CC-BY-4.0. Their frozen copies in Analog Canvas remain under AGPL-3.0.
- Published packages declare the license of their contents: crates and TypeScript packages `MIT`, the standard library data package `CC0-1.0`.

## Consequences

- Any tool, including a commercial one, may implement RNL and embed its code if it keeps the MIT notice.
- Copying or adapting the specification text requires attribution. CC-BY-4.0 grants no patent or trademark rights.
- Examples meant to be pasted into netlists belong in `examples/` (CC0-1.0). Examples inside the specification text follow CC-BY-4.0 like the rest of that text.
- Code from Analog Canvas, which is AGPL-3.0, can be added here under MIT only after its copyright holder relicenses it.
- Contribution terms are set in [0004](0004-contribution-terms.md).

Note (2026-10-25): the top-level `examples/` held only its README and was removed. Examples are now in `site/examples/`, `skills/rnl/examples/` and the [rnl-gallery](https://github.com/rich-netlist/rnl-gallery) repository; the table above and the line on `examples/` are kept as decided.

Note (2026-10-25): the symbols of the standard packages of the series 0.3 are generated from Analog Canvas's component definitions; the maintainers have confirmed that these definitions may be published here as part of the standard packages under CC0-1.0. Analog Canvas's code itself remains AGPL-3.0 and has not been copied here.
