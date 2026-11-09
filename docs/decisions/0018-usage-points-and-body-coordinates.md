# 0018. Give usages their own points and translate body coordinates

- Date: 2026-11-09
- Status: accepted

## Context

Language 0.4 gives every usage its own point, including usages without a symbol (12.1, 14.1).
The point is independent of the objects filling its roles. A general definition body can refer
to the usage as `?` and tie it to those objects with positional facts (9.4).

## Decision

The file creating a usage selects its point semantics, including records scoped into an included
file. Files of 0.2 and 0.3 keep their existing placement behavior. A symbol-less usage of 0.4 is
a point in the solver input, with no symbol, orientation or device collision box. Wires and
graphics reach it explicitly through `::position`; ORIENT still requires a symbol.

Coordinates written in a general definition body of 0.4 are offsets from that usage's point.
They translate in scene axes, independently of its symbol's orientation. Each nested usage has
its own base. Free axes stay free, and supplied parameters become offsets after substitution.
PLACE on the body's own usage point fails, including through `?::position`.

The runtime retains relative coordinates as facts, with their source, priority and expansion
chain. It resolves coordinate equalities in the order of 8.5 and propagates coordinates through
anchored components. An absolute PLACE on a role can therefore fix the usage in reverse, and
several compatible facts can hold together. Other positional constraints still require the
solver when fixed coordinates cannot settle them.

The public problem format represents each relative coordinate as an exact directed DISTANCE
between two origins, preserving the coordinate statement's label and rank. Negative offsets
reverse the direction and use a nonnegative bound. The solver record writes absolute scene
coordinates, as statements outside a definition body do. No new problem variant is required.

A constraint on a symbol-less usage gives `RNL3517` when its point has no positional connection
to a drawing. Connections follow effective facts transitively and separately on each axis;
roles, body ancestry, disabled facts and discarded facts do not establish a connection.

## Consequences

General bodies can define layouts around an independent point without drawing a symbol for it.
Their coordinates may change existing 0.4 previews. The 0.2/0.3 reference outputs stay unchanged.
Regression tests cover nesting, parameters, partial coordinates, conflict priorities, explicit
point references, source-file versions and solve/write/read stability.
