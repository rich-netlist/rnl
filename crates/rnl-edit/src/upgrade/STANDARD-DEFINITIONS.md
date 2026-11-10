# Standard definition migration audit

This table compares every ordinary definition in bundled series 0.3 with series 0.4. Symbol spelling and pin maps are covered separately by the rewrite inventory. Classification applies only with verified bundled provenance; local overrides and alias environments keep their own meaning. Body-based recognizers check the actual tokens before applying an exact transformation. Update this table and its regression tests whenever either package series changes.

| Definition | Change in series 0.4 | Upgrade handling |
| --- | --- | --- |
| GROUND | `p` accepts a net rather than net or terminal; body stays empty | For a directly written host terminal, attach its net and align the usage point with the original terminal in x and y. Net attachment has no lost main point. |
| SUPPLY | Same role narrowing as GROUND; body stays empty | Same exact attachment/point treatment. |
| IO_PORT | Same role narrowing, plus `ASSERT-NET(port=true)` | Exact attachment/point treatment; assertion-only change is a notice. A non-port net is a target violation, separately listed; do not change connectivity or disguise an internal net as a port. |
| NO_CONNECT | Adds ALIGN x/y of `?` with `?t` | Already preserves the old main point; no additional alignment or review. |
| JUNCTION | Adds ALIGN x/y of `?` with `?p` | Already preserves the old main point. |
| CURRENT_ARROW | Adds ALIGN x/y of `?` with `?p` | Already preserves the old main point. |
| ARROW | Role `p` becomes `at`; adds ALIGN x/y | Rename the role; target body preserves the old main point. |
| SPDT_SWITCH | Role `p` becomes `at`; adds ALIGN x/y | Same exact treatment. |
| FIGURE | Role `p` becomes `at`; adds ALIGN x/y | Same exact treatment. |
| DELAY_CELL | Role `p` becomes `at`; adds ALIGN x/y | Same exact treatment. |
| INTEGRATOR | Role `p` becomes `at`; adds ALIGN x/y | Same exact treatment. |
| DISCRETE_TIME_INTEGRATOR | Role `p` becomes `at`; adds ALIGN x/y | Same exact treatment. |
| QUANTIZER | Role `p` becomes `at`; adds ALIGN x/y | Same exact treatment. |
| UNIT_DELAY | Role `p` becomes `at`; adds ALIGN x/y | Same exact treatment. |
| GROUND_STUB | `gap` default 2 → 1; role `t` becomes a pin; internal ground terminal reference becomes named `::pin` | Explicit `gap=2` on old applications without a value; retain authored values. Pin binding and target diagnostics remain independently checked. |
| SUPPLY_STUB | Same default/pin changes as GROUND_STUB | Same exact default treatment. |
| DIFFERENTIAL_PAIR | Split into N/P, input roles become left/right; adds ordering, routing, grouping, pin orientations and assertions | Only with `--approximate`, choose by both resolved input-device polarities. Unknown or mismatched polarities stay manual. Model-card and Spectre polarity use the validated host type; DEVICE `type` is a 0.4 addition, not a supported 0.3 source declaration. A source the old reader rejects is never bypassed or guessed from its model name. No equivalence claim. |
| CURRENT_MIRROR | Formerly empty; adds ALIGN y, orientation relations, ROUTE-OBJECT and model/topology assertions | Manual: these are actual drawing facts as well as assertions. Keeping or disabling each new fact requires a design choice. |
| MATCHED | Empty body remains; role is narrowed to instances/usages | No body rewrite. Target role validation remains authoritative. |
| BRANCH | Adds SERIES (routing/topology) and top-to-bottom ORDER to the old ALIGN x | Manual: new drawing/routing facts. |
| CHAIN | Removed | Manual: decide whether to retain the original source/chain/load ordering explicitly or choose another definition. |
| CASCADE | Four roles become one stages role; empty body | Manual role mapping, since feedback and load membership cannot be inferred from spelling. |
| SUPPLY_HIGH | Removed | Manual: electrical meaning alone does not determine placement or a replacement symbol. |
| SUPPLY_LOW | Removed | Same limitation. |
| MIRRORED | Axis-only distribution plus endpoint ORDER becomes directed distribution; axis becomes a collection role | Manual: a multi-member axis no longer means its centre. Even a singleton axis changes minimum spacing: old endpoint ORDER allowed a half-unit distribution step; directed DISTRIBUTE requires a unit step. Conflict grouping also differs. |
| FLOW | Removes the low-priority ORDER controlled by `direction` | Manual: retaining the usage loses an ordering fact; decide whether to keep that explicit order or adopt the new semantic-only usage. |

Series 0.2 names and merged packages retain the provenance-checked renames in `legacy.rs`; the same actual-body checks apply when their bodies match these transformations. Unrecognized body changes remain manual rather than inheriting a classification just from a familiar name.

## Set-distance proof and limits

For a directed distance and nonnegative lower bound d, `min(B)-max(A) >= d` is equivalent to every `b-a >= d`. The reverse directions use the reversed signed projection. Empty sets are vacuous on both sides. This covers literal, named and selected collections without evaluating membership. A maximum or exact bound does not distribute this way, even if just one side is a singleton: from A={0,10} to B={12}, the old gap is 2, while the pairwise distances are 12 and 2. Only two proven singleton slots with an already nonnegative old range are accepted. A maximum without a lower bound also gains the new implicit lower bound zero, so is not silently accepted.

## Other contextual categories

| Category | Exact support | Opt-in approximation | Why remaining cases require a person |
| --- | --- | --- | --- |
| usage-pin | Proven unique named pins, including local symbol bindings | None | Displaced, inherited or unnamed pins need a specific retained identity. |
| alias-context | No additional expansion-based conversion | None | The same spelling can denote a different operation/definition in its alias environment. |
| standard-name-removed | Verified legacy renames and preserved routing/fill selections | Differential pair only when both model polarities agree | Removed policies and definitions have no uniquely determined replacement. |
| distance-interval | Algebraic reversal of supported nonpositive single-point ranges | Axis choice only | Crossing signed ranges and extent maxima cannot be represented by one nonnegative directed range. |
| pin-selection | Direct drawing consumers become SELECT-PIN | None | Electrical consumers need terminal identities; pin membership alone is insufficient. |
| terminal-selection | Direct TEXT uses explicit pin selection | None | Expanded electrical membership can change constraints and topology. |
| symbol-pin-inheritance | Simple local symbol identities are preserved separately | None | Inclusion can retain displaced pins and introduce name conflicts; the intended live pin must be identified. |
| constraint-items | Singleton literal groups need no centre choice | ORDER alone keeps text and reports stronger combination semantics | ALIGN/DISTRIBUTE would constrain every member, destroying the old collective centre behavior. |

## Reserved follow-up for future package revisions

Re-audit OPAMP_5TERM/COMPARATOR_5TERM supply pins, net-label text keys, and four-terminal MOS bulk-pin fill when those package changes land. Do not apply prospective mappings before their target definitions exist. Default fill rules and their selected membership also remain subject to target comparison; an explicit application default does not silently override an imported generation rule.
