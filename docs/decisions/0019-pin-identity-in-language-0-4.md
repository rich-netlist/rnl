# Pins keep their own identity in language 0.4

Status: proposed; implementation in progress.

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

## Consequences

The runtime, problem serializer, solver and solution writer must agree on pin
identity and references. Tests must cover bus pins, repeated terminal bindings,
unbound pins, named references, usage roles, direction constraints, and repeated
solve/write/read behavior. The implementation may be delivered in independently
tested changes; this record does not claim those changes are complete.

Upgrade rewrites are maintained separately. Each implementation pull request
records the affected 0.3 and 0.4 forms in its `Upgrade rewrite needed` section.
