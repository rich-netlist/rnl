# Named sub-objects keep their written handles

Status: accepted. Implemented in #412 and released in core 0.5.6.

## Context

Named scene pins need both a reference handle and a geometric pin identity.
These answer different questions. This clarifies the named-reference part of
[0019](0019-pin-identity-in-language-0-4.md).

## Decision

- In language 0.4, `object::name` has the handle `(object, name)` (6.5).
  Two names of one pin produce different sub-object handles. Repeating the
  same parent and name produces the same handle.
- Sets and UNION compare handles (5.2, 5.2.2). They must not canonicalize two
  differently named sub-objects merely because both later resolve to one pin.
- Ordinary creation uses those written handles in step 3. Given a symbol
  whose pin has names `p` and `q`, `{#R1::p #R1::q} @UNION => @AS $D`, with
  a one-role definition `$D`, creates two usages. Repeating `#R1::p` instead
  creates one usage after UNION.
- Symbol-dependent conversions happen after symbol assignment (5.2.4).
  Resolving both roles to the same pin does not change the earlier set or
  recreate usages (8.2). Appearance rules can therefore select the step-3
  usages without depending on the symbol assignment they help determine.
- Geometric and electrical operations still identify the resolved pin by
  owner and pin index. Conflicting settings of that pin follow the ordinary
  conflict rules (1.5); a different written name does not create another pin.
- Preserve the behavior and output of earlier language versions.

## Consequences

Reference identity and resolved slot values must remain separate. Do not add
a global object redirect that changes UNION, REMOVE-FROM, member positions,
or creation counts when symbols become known. Export exact pin identities
and usable references without treating distinct named handles as one member.

Tests cover repeated names, different names of one pin, name aliases bound to
the same reference, removal by handle, member positions, and appearance rules
that observe the usages before symbol-dependent conversion. This record does
not claim that named scene-pin resolution is already implemented.
