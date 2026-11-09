# 0017. Isolate language 0.4 packages from earlier languages

- Date: 2026-11-09
- Status: accepted

## Context

Language 0.4 changes the meaning of definitions, sets and positions. The interaction between a
file written in 0.4 and packages written in earlier languages is not defined. Each 0.x language
version is a major version of its own (16.1).

The published 0.2 and 0.3 behavior already supports using each other's packages. In particular,
`rnl solve` raises a 0.2 file to 0.3 and pins the 0.2 standard series when writing its result.
Changing that behavior during development would change existing files' drawings.

## Decision

A file written in 0.4 may use only packages written in 0.4. A file written in 0.2 or 0.3 may not
use a package written in 0.4. This applies to direct and indirect references, references by path,
explicit package versions, alias packages and the standard package selected by the tool.
The language version is the package file's `VERSION`, independently of its `PACKAGE` version.

A reference that crosses this boundary fails with `RNL2308`. Statements using its names fail
as missing names under 16.2. Other names and statements remain available.

Files of different language versions may still be included together. Their package environments
remain separate across the 0.4 boundary. All published behavior between 0.2 and 0.3 stays,
including automatic version raising by `rnl solve` and existing package selection rules.

## Consequences

No compatibility semantics between 0.4 and earlier packages are needed during development.
Tests must cover the boundary in both directions and preserve the existing 0.2/0.3 corpus and
solver outputs. The 0.6.0 switch will remove the 0.2/0.3 implementation together.
