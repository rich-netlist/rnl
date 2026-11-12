# Standard definition migration audit

This table compares every ordinary definition in bundled series 0.3 with series 0.4. Symbol spelling and pin maps are covered separately by the rewrite inventory. Classification applies only with verified bundled provenance; local overrides and alias environments keep their own meaning. Body-based recognizers check the actual tokens before applying an exact transformation. Update this table and its regression tests whenever either package series changes.

| Definition | Change in series 0.4 | Upgrade handling |
| --- | --- | --- |
| GROUND | `p` accepts a net rather than net or terminal; body stays empty | For a directly written host terminal, attach its net and align the usage point with the original terminal in x and y. Net attachment has no lost main point. |
| SUPPLY | Same role narrowing as GROUND; body stays empty | Same exact attachment/point treatment. |
| IO_PORT | Same role narrowing, plus `ASSERT-NET(port=true)` | In default mode, exact attachment/point treatment with an assertion notice (a non-port net remains a target violation). With `--approximate`, top-level non-port `$IO_PORT` usages without symbol overrides become `$NET_LABEL` and report pin-direction risk. |
| NO_CONNECT | Role `t` accepts a terminal or pin; adds ALIGN x/y of `?` with `?t` | Already preserves terminal attachment and the old main point; no additional alignment or review. |
| JUNCTION | Adds ALIGN x/y of `?` with `?p` | Already preserves the old main point. |
| CURRENT_ARROW | Adds ALIGN x/y of `?` with `?p`; symbol renamed to `ARROW_SYMBOL` | Already preserves the old main point. |
| ARROW | Role `p` becomes `at`; adds ALIGN x/y | Rename the role; target body preserves the old main point. |
| SPDT_SWITCH | Role `p` becomes `at`; adds ALIGN x/y | Same exact treatment. |
| FIGURE | Role `p` becomes `at`; adds ALIGN x/y | Same exact treatment. |
| DELAY_CELL | Point-attached definition removed (`$DELAY_CELL` is now the two-terminal logic symbol) | Manual: choose `$FIGURE` with `@SET-SYMBOL $DELAY_CELL` or a host instance. Old symbol references `$DELAY_CELL_SYMBOL` rename to `$DELAY_CELL`. |
| INTEGRATOR | Changed from point-attached figure (`p`) to semantic block (`in`, `out`) | Manual: choose `$FIGURE` with `$INTEGRATOR_SYMBOL` for a drawing-only figure or bind block roles. |
| DISCRETE_TIME_INTEGRATOR | Same figure-to-block role change as INTEGRATOR | Same manual review. |
| QUANTIZER | Same figure-to-block role change as INTEGRATOR | Same manual review. |
| UNIT_DELAY | Same figure-to-block role change as INTEGRATOR | Same manual review. |
| GROUND_STUB | `gap` default 2 → 1; role `t` becomes a pin; internal ground terminal reference becomes named `::pin` | Explicit `gap=2` on old applications without a value; retain authored values. Pin binding and target diagnostics remain independently checked. |
| SUPPLY_STUB | Same default/pin changes as GROUND_STUB | Same exact default treatment. |
| DIFFERENTIAL_PAIR | Split into N/P, input roles become left/right, `tail` becomes optional single object; adds ordering, routing, grouping, pin orientations and assertions | Only with `--approximate`, choose by both resolved input-device polarities when `tail` has at most one member (unwrapping `{x}(tail)` and dropping `{}(tail)`). Unknown/mismatched polarities or multi-member tails stay manual. |
| CURRENT_MIRROR | Split into `CURRENT_MIRROR_N` and `CURRENT_MIRROR_P`; adds ALIGN y, orientation relations, ROUTE-OBJECT and model/topology assertions | With `--approximate`, choose `_N`/`_P` when all resolved `ref` and `out` devices share one polarity and report the added facts; manual otherwise and in default mode. |
| MATCHED | Adds `ASSERT-INSTANCE(same=["type"])` and `ASSERT-USAGE(same=["definition"])`; still draws nothing | Assertion-only change is a notice; drawing facts are unchanged. |
| BRANCH | Adds SERIES (routing/topology) and top-to-bottom ORDER to the old ALIGN x | Manual: new drawing/routing facts. |
| CHAIN | Removed | Manual: decide whether to retain the original source/chain/load ordering explicitly or choose another definition. |
| CASCADE | Four roles become one stages role; adds left-to-right ORDER and ROUTE-OBJECT | Manual role mapping, since feedback and load membership cannot be inferred from spelling. |
| SUPPLY_HIGH | Removed | Manual: electrical meaning alone does not determine placement or a replacement symbol. |
| SUPPLY_LOW | Removed | Same limitation. |
| MIRRORED | Axis-only distribution plus endpoint ORDER becomes directed distribution; `axis` narrows to a single instance, usage or point | With `--approximate`, retain an empty or singleton non-net axis and resolve sides together with differential pairs using the source mirror ORDER, directed ORDER/DISTANCE and explicit x positions. Contradictory evidence or net axes need review. Minimum spacing and conflict grouping change even for a singleton axis. |
| FLOW | Removes the low-priority ORDER controlled by `direction` | Preserve the usage and restore its expanded ORDER with the effective direction and priority. Nested groups with more than one member require `--approximate` because the target orders combinations rather than centres. |

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

## Completed package-sync rules

`OPAMP_5TERM`/`COMPARATOR_5TERM` from 0.3 rename to 3-terminal `OPAMP`/`COMPARATOR` with explicit `[1, 2, 5]` terminal maps, net-displaying virtual parts rewrite `key="name"` to `key="netname"`, `TRANSCONDUCTOR` rewrites `key="formula"` to `key="label"`, and scenes that used 0.3 four-terminal MOS symbols retain their default symbol rules and ground/supply bulk-pin fill.
