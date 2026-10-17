# 0003. npm packages use the @rich-netlist scope

- Date: 2026-09-24
- Status: accepted

## Context

The architecture proposal used `@rnl/*` as placeholder package names. The unscoped npm name `rnl` is held by an unrelated placeholder package from 2022, and whether the `@rnl` scope is free could not be checked without creating it.

## Decision

- The maintainer has created the npm organization `rich-netlist`. All npm packages are published under the `@rich-netlist/` scope.
- No placeholder packages are published. The organization already reserves every name inside the scope.
- The unscoped name `rich-netlist` is not claimed now. It becomes worth claiming only if a package, such as a CLI run with `npx rich-netlist`, should have a short unscoped name.
- Crate names are claimed with the first real crate release, because crates.io has no namespaces.

## Consequences

- Scoped packages are private by default, so each package sets `"publishConfig": {"access": "public"}`.
- The first publish of each package is made manually with two-factor authentication. Later releases use npm trusted publishing from GitHub Actions. npm generates provenance statements only for packages published from a public repository.
- Note, 2026-10-17: `@rich-netlist/runtime` was first published by its workflow, `publish-runtime.yml`, with a one-off npm token instead, so that its first previews carry provenance too. The token was revoked once the package's trusted publisher named the workflow, which publishes with no token since.
