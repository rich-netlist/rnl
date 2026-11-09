# Pins keep their own identity in language 0.4

Status: accepted; implementation in steps.

## Context

In language 0.4, each PIN creates a distinct object of its symbol (9.2.4).
Several pins may bind one terminal, and a pin may bind none. A local pin name
also names a sub-object of each instance or usage drawn with that symbol (6.5).
The existing representation discards earlier bindings and uses a terminal
reference when writing a solver's pin endpoint. That cannot identify every pin.

## Decision

- Keep each pin in declaration order in a symbol of 0.4. Keep the established
  replacement behavior for symbols written in 0.2 or 0.3.
- Resolve a terminal used for drawing to its last matching pin. Match names by
  the owner's terminal rules; usages bind named roles and never numbered roles.
- Preserve local pin names through symbol inclusion. Resolve named sub-objects
  after symbol assignment, checking collisions with names found earlier by `::`.
- Carry owner and pin identity through geometric facts, wire anchors and nets.
  Carry an explicit reference for writing a named pin, a reachable terminal, or
  an unreferenceable pin; never substitute another pin of the same terminal.
- Treat a pin's direction as a vector in its owner's frame. Orientation facts
  constrain that vector; relations between pin directions must not be mistaken
  for relations between the owners' full orientations.
- Keep earlier language output and problem digests unchanged. Keep the Viewer
  editing interface separate until its corresponding pin update is implemented.

## Implementation steps

The first change preserves every pin and its bindings, permits an omitted
`terminal=`, and exports all connected pins to the solver. A terminal still gives
its last pin; an earlier bus pin may use another terminal for which it is last.
Otherwise `PointRef::SymbolPin` carries its identity and an anonymous reference,
which the solution writer reports as unsolved. The solver handles both forms by
owner and pin index. The constraint view and its TypeScript declaration carry
the explicit reference form as well.

Named pin objects and `::` lookup, local pin-name validation, `SET-SYMBOL` terminal
maps, pin direction facts, selections and the pin editing interface remain later
steps. Until named lookup is implemented, naming a local point used by PIN does
not make that pin reachable from the scene.

## Consequences

The runtime, problem serializer, solver and solution writer must agree on pin
identity and references. Tests must cover bus pins, repeated terminal bindings,
unbound pins, named references, usage roles, direction constraints, and repeated
solve/write/read behavior. The implementation may be delivered in independently
tested changes; this record does not claim those changes are complete.

Upgrade rewrites are maintained separately. Each implementation pull request
records the affected 0.3 and 0.4 forms in its `Upgrade rewrite needed` section.
