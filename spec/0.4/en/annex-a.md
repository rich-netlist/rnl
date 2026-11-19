## Annex A Keyword signatures (normative)

This annex is generated from the word list `rnl-words-0.4.json` (5.3); where the two differ, it is an erratum. It lists what each keyword takes and gives; what it means is in the section named in the last column.

- **Slots** are listed in the standard order, with their names (which can be written as roles, 5.3) and what they take: *one*: a single object; given a set, the keyword is applied to each member (5.2.1), except that PRIORITY takes a set marked with GROUP as the group itself (14.8); *set*: a set; loose objects written for the slot form one set (5.2.1); *any*: one object or a set, used as the keyword says (for DISTANCE, a set stands for each of its members, 14.5); *shape*: a point, or a set of points (two forms of a graphic, 9.2.5); *body*: statements in braces, enclosed (5.4); *definition*: a definition: a `$` name, a `%` name bound to a definition, or an anonymous definition (5.6, 6.6). Absorbed input is strict; enclosed input is tolerant (5.4, 7.3).
- **Kinds** of slots are kind names (`instance`, `terminal`, `pin`, `net`, `point`, `wire`, `usage`, `bundle`, `text`, `annotation`, `label`) or: *any*, any object; *level*, `#` alone: the level itself; *owner*, `?` alone: in a symbol body, the object drawn with the symbol; in the body of a general definition, the usage being expanded (9.4); *position*, the sub-object `::position` of an object (its origin); *locatable*, an object with a position: instance, bundle, pin, terminal with a pin, point, text, the centre of a graphic written with a centre, a usage by its own point; a set given for it stands for each of its members (14.1); *orientable*, an object with an orientation: instance, bundle, usage with a symbol; *constraint*, an object made by a constraint keyword; *set*, a set, written as a name; in ASSERT-TOPOLOGY also an item that is a `{…}` written out (15.4); *empty node*, `&` alone: no net; taken by roles of kind `net` and by the net slot of WIRE, and elsewhere as an empty set of nets (6.3).
- **Parameters** marked *position 1*, *position 2* can be given by position, in that order; all others only by name (4.2). `priority` always takes `"disabled"`, `"low"`, `"normal"` or `"high"`; without it the priority of the record head applies, except that AS and ATTACH take only `"disabled"` from it (14.8).
- **Result**: the object the statement gives (5.5) and how its facts override (5.8). Without an override, facts accumulate.
- Unless noted, a keyword can be written in any record of a level; *symbol bodies* means it can also be written in a DEFINE-SYMBOL body.

### A.1 The record

`@RNL(…) {…}` commits its statements to the level (5.4). Record head fields (2.8), all given by name:

| Field | Values | Default | Affects interpretation |
|---|---|---|---|
| `type` | `"authored"`, `"generated"`, `"header"` | `"authored"` | yes |
| `scope` | string | — | yes |
| `priority` | `"disabled"`, `"low"`, `"normal"`, `"high"` | `"normal"` | yes |
| `author` | string | — | no |
| `hash` | string | — | no |
| `numbering` | string | — | yes |
| `modified` | string | — | no |
| `language` | `"spice"`, `"spectre"` | — | yes |

### A.2 Header records and packages

| Keyword | Slots | Parameters | Result | Section |
|---|---|---|---|---|
| `@VERSION` (header records only) | — | `version` (position 1; required) | — | 16.1 |
| `@PACKAGE` (header record of a package file only) | — | `name` (required), `version` (required) | — | 2.7 |
| `@USE-PACKAGE` (also in header records) | `body`: body, optional | `name` (required), `version`, `path` | — | 16.2 |
| `@ALIAS` (header records only) | — | `alias` (position 1; required), `original` (position 2; required), `definition` | — | 16.5 |
| `@USE-ALIASES` (header records only) | — | `name` (required), `version`, `path` | — | 16.5 |
| `@DEVICE` (header records only) | — | `name`, `model`, `terminals` (array), `type` (string); at least one of `name`, `model`; at least one of `terminals`, `type` | — | 6.12 |

### A.3 Definitions

| Keyword | Slots | Parameters | Result | Section |
|---|---|---|---|---|
| `@DEFINE-SYMBOL` (also in symbol bodies) | `body`: body | `box` (array) | a definition (symbol) | 9.2 |
| `@DEFINE-STYLE` | `extends`: body (a style) | style properties (9.3) | a definition (style) | 9.3 |
| `@DEFINE` | `body`: body | `roles` (required; array), `parameters` (array) | a definition (general) | 9.4 |

### A.4 Symbol content and graphics

| Keyword | Slots | Parameters | Result | Section |
|---|---|---|---|---|
| `@PIN` (symbol bodies only) | `position`: one (point) | `terminal` (terminal, or a non-empty array of terminals; may be left out), `label`, `direction` (`"right"`, `"up"`, `"left"`, `"down"`) | a new pin | 9.2.4 |
| `@LINE` (also in symbol bodies) | `points`: set (point, pin, terminal) | `arrow` (`"none"`, `"start"`, `"end"`, `"both"`; default `"none"`), `fill` (`"none"`, `"linecolor"`, `"fillcolor"`; default `"none"`), line style properties | a graphic of the symbol; in the scene, a new annotation (13.4) | 9.2.5 |
| `@RECTANGLE` (also in symbol bodies) | `points`: shape (point, pin, terminal) | `width` (number), `height` (number), `fill` (`"none"`, `"linecolor"`, `"fillcolor"`; default `"none"`), area style properties; `width` and `height` together or not at all | a graphic of the symbol; in the scene, a new annotation (13.4) | 9.2.5 |
| `@CIRCLE` (also in symbol bodies) | `points`: shape (point, pin, terminal) | `radius` (number), `fill` (`"none"`, `"linecolor"`, `"fillcolor"`; default `"none"`), area style properties | a graphic of the symbol; in the scene, a new annotation (13.4) | 9.2.5 |
| `@ELLIPSE` (also in symbol bodies) | `points`: shape (point, pin, terminal) | `width` (number), `height` (number), `fill` (`"none"`, `"linecolor"`, `"fillcolor"`; default `"none"`), area style properties; `width` and `height` together or not at all | a graphic of the symbol; in the scene, a new annotation (13.4) | 9.2.5 |
| `@ARC` (also in symbol bodies) | `points`: set (point, pin, terminal) | `arrow` (`"none"`, `"start"`, `"end"`, `"both"`; default `"none"`), `fill` (`"none"`, `"linecolor"`, `"fillcolor"`; default `"none"`), line style properties | a graphic of the symbol; in the scene, a new annotation (13.4) | 9.2.5 |
| `@BEZIER` (also in symbol bodies) | `points`: set (point, pin, terminal) | `arrow` (`"none"`, `"start"`, `"end"`, `"both"`; default `"none"`), `fill` (`"none"`, `"linecolor"`, `"fillcolor"`; default `"none"`), line style properties | a graphic of the symbol; in the scene, a new annotation (13.4) | 9.2.5 |
| `@ANNOTATION` | `at`: one (locatable); `source`: one (a symbol, a general definition or a usage), written last | `rotate` (number, degrees counterclockwise; default `0`), `mirror` (bool; default `false`), style properties, and the parameters of a symbol or general definition given as `source`, by name (9.4) | a new annotation | 13.5 |
| `@TEXT` (also in symbol bodies) | `attach`: one (point, instance, usage, bundle, pin, terminal, wire, annotation, net, `?`) | `text` (position 1; required), `key`, `dx` (number), `dy` (number), `halign` (`"start"`, `"middle"`, `"end"`; default `"start"`), `valign` (`"baseline"`, `"top"`, `"middle"`, `"bottom"`; default `"baseline"`), `rotate` (number, degrees counterclockwise; default `0`), `fix_rotate` (bool; default `false`), `upright` (bool; default `true`), text style properties | a new text; with `key`, a sub-object of the attached object; overrides by key (key: attached object and key, when key is given) | 13.3 |

### A.5 Placement and appearance

| Keyword | Slots | Parameters | Result | Section |
|---|---|---|---|---|
| `@PLACE` (also in symbol bodies) | `target`: one (instance, usage, bundle, point, text, `::position`) | `x` (position 1; number), `y` (position 2; number), `priority`; at least one of `x`, `y` | the input object; geometric fact (key: object and axis) | 11.1 |
| `@ORIENT` (also in symbol bodies) | `target`: one (instance, usage, bundle, pin) | `code` (position 1; required; `"R0"`, `"R90"`, `"R180"`, `"R270"`, `"MX"`, `"MY"`, `"MXR90"`, `"MYR90"`, or the words `"left"`, `"down"`, `"right"`, `"up"` for the first four; or a non-empty array of them), `priority` | the input object; geometric fact (key: object, or pin) | 11.2 |
| `@LAYER` | `target`: one (instance, usage, bundle, wire, annotation, text) | `layer` (position 1; required; `"bottom"`, `"middle"`, `"top"`) | the input object; overrides by key (key: object) | 11.6 |
| `@SET-SYMBOL` (also in symbol bodies) | `target`: one (instance, usage, bundle, `#`); `symbol`: definition (a symbol) | `terminals` (array of terminal names, or of pairs) | the input object; appearance queue | 11.3 |
| `@SET-STYLE` | `target`: one (instance, usage, bundle, wire, annotation, text); `style`: definition (a style) | — | the input object; appearance queue | 11.3 |

### A.6 Selections

| Keyword | Slots | Parameters | Result | Section |
|---|---|---|---|---|
| `@SELECT-INSTANCE` | `scope`: any (`#`, any object) | `name`, `model`, `type`, `terminals` (number, whole), `symbol` (string: a `$` name, with `*`), `drawn` (bool), `role`, `recursive` (bool; default `false`) | a new set of instances | 10.1 |
| `@SELECT-TERMINAL` | `scope`: any (`#`, any object) | `name`, `model`, `type`, `terminal` (terminal), `net`, `current` (`"in"`, `"out"`, `"unspecified"`), `role`, `recursive` (bool; default `false`) | a new set of terminals | 10.1 |
| `@SELECT-PIN` | `scope`: any (`#`, any object) | `name`, `model`, `type`, `terminal` (terminal), `net`, `wired` (bool), `role`, `recursive` (bool; default `false`) | a new set of pins | 10.1 |
| `@SELECT-NET` | `scope`: any (`#`, any object) | `name`, `port` (bool), `wired` (bool), `global` (bool), `role`, `recursive` (bool; default `false`) | a new set of nets | 10.1 |
| `@SELECT-WIRE` | `scope`: any (`#`, any object) | `net`, `bus` (bool), `role`, `recursive` (bool; default `false`) | a new set of wires | 10.1 |
| `@SELECT-LABEL` | `scope`: any (`#`, any object) | `key`, `role`, `recursive` (bool; default `false`) | a new set of labels | 10.1 |
| `@SELECT-ANNOTATION` | `scope`: any (`#`, any object) | `role`, `recursive` (bool; default `false`) | a new set of annotations | 10.1 |
| `@SELECT-USAGE` | `scope`: any (`#`, any object); `definition`: definition (a general definition), optional | `symbol` (string: a `$` name, with `*`), `drawn` (bool), `recursive` (bool; default `false`) | a new set of usages | 10.1 |
| `@SELECT-ANCESTOR-USAGE` | `scope`: any (any object); `definition`: definition (a general definition) | `recursive` (bool; default `false`) | a new set of usages | 10.1 |

### A.7 Applying definitions

| Keyword | Slots | Parameters | Result | Section |
|---|---|---|---|---|
| `@AS` (also in symbol bodies) | the roles of the definition, then the definition (`definition`, written last) | the parameters of the definition (by position in the order of its parameter list, or by name; 9.4), `priority` | a new usage | 9.4 |
| `@ATTACH` (also in symbol bodies) | the definition, then its roles (`definition`, written first) | the parameters of the definition (by position in the order of its parameter list, or by name; 9.4), `priority` | a new usage | 9.4 |

### A.8 Wires

| Keyword | Slots | Parameters | Result | Section |
|---|---|---|---|---|
| `@WIRE` | `net`: any (net, or the empty node); `points`: set (point, pin; a terminal stands for its pin, 5.2.4) | `mode` (`"auto"`, `"straight"`, `"horizontal-vertical"`, `"vertical-horizontal"`, `"manhattan"`, `"manhattan45"`; default `"auto"`), wire style properties | a new wire; given a wire, rewrites it, overriding by key (key: that wire) | 13.1 |
| `@ROUTE-NET` | `net`: one (net) | `mode` (`"auto"`, `"straight"`, `"horizontal-vertical"`, `"vertical-horizontal"`, `"manhattan"`, `"manhattan45"`; default `"auto"`), wire style properties, `priority` | a new route | 13.7 |
| `@ROUTE-OBJECT` | `objects`: set (any object) | `mode` (as ROUTE-NET), wire style properties, `priority` | the input object | 13.7 |
| `@UNROUTE-NET` | `net`: one (net) | `priority` | the input object | 13.7 |
| `@UNROUTE-OBJECT` | `objects`: set (any object) | `priority` | the input object | 13.7 |

### A.9 Sets, groups and bundles

| Keyword | Slots | Parameters | Result | Section |
|---|---|---|---|---|
| `@BUNDLE` | `members`: set (instance) | — | a new bundle | 11.8 |
| `@GROUP` | `members`: set (any object) | `spacing_x` (number, whole), `spacing_y` (number, whole) | the input object | 11.5 |
| `@ADD-TO` | `items`: any (any object); `target`: any (set) | — | the target set | 5.2.2 |
| `@REMOVE-FROM` | `items`: any (any object); `target`: any (set) | — | the target set | 5.2.2 |
| `@UNION` | `sets`: set (any object) | — | a new set | 5.2.2 |

### A.10 Constraints and priority

| Keyword | Slots | Parameters | Result | Section |
|---|---|---|---|---|
| `@ALIGN` | `members`: set (locatable) | `axis` (position 1; required; `"x"`, `"y"`), `priority` | a new constraint; geometric fact | 14.2 |
| `@ORDER` | `members`: set (locatable) | `direction` (position 1; required; `"left-to-right"`, `"right-to-left"`, `"bottom-to-top"`, `"top-to-bottom"`), `priority` | a new constraint; geometric fact | 14.3 |
| `@DISTRIBUTE` | `members`: set (locatable) | `direction` (position 1; required; `"left-to-right"`, `"right-to-left"`, `"bottom-to-top"`, `"top-to-bottom"`), `priority` | a new constraint; geometric fact | 14.4 |
| `@DISTANCE` | `from`: any (locatable); `to`: any (locatable) | `direction` (position 1; required; `"left-to-right"`, `"right-to-left"`, `"bottom-to-top"`, `"top-to-bottom"`), `min` (number), `max` (number), `exact` (number), `priority`; at least one of `min`, `max`, `exact`; `exact` not with `min`; `exact` not with `max` | a new constraint; geometric fact | 14.5 |
| `@ORIENT-RELATION` | `from`: one (orientable, or pin); `to`: one (orientable, or pin, as `from`) | `relation` (position 1; required; the eight codes of ORIENT, not its words, or a non-empty array of them), `priority` | a new constraint; geometric fact | 14.9 |
| `@TABLE` | `rows`: set (locatable; nested sets are the lists of the table) | `transpose` (bool; default `false`), `priority` | a new constraint; geometric fact | 14.11 |
| `@SOLVER-CONFIG` (header record without `language`, header record of a package file, and level) | — | named parameters of the solver: numbers, strings, truth values and arrays of these; `spacing_x`, `spacing_y` (number, whole) and `junction` (string: a `$` name; in a level record also a `%` name of the level; or empty) are named by the standard; the other keys are those of the solver | no object; settings of the solver | 14.10 |
| `@PRIORITY` | `target`: one (constraint, usage, set) | `priority` (position 1; required) | the input object; overrides by key (key: object) | 14.8 |

### A.11 Electrical semantics

| Keyword | Slots | Parameters | Result | Section |
|---|---|---|---|---|
| `@CURRENT` | `terminal`: one (terminal) | `direction` (position 1; required; `"in"`, `"out"`, `"unspecified"`) | the input object; overrides by key (key: terminal) | 15.2 |
| `@ASSERT-INSTANCE` | `members`: set (instance) | the conditions of SELECT-INSTANCE but `role` and `recursive`, `has` (array of attribute names), `same` (array of attribute names), `different` (array of attribute names), `message` | no object; a check | 15.3 |
| `@ASSERT-TERMINAL` | `members`: set (terminal) | the conditions of SELECT-TERMINAL but `role` and `recursive`, `same`, `different`, `message` | no object; a check | 15.3 |
| `@ASSERT-NET` | `members`: set (net) | the conditions of SELECT-NET but `role` and `recursive`, `same`, `different`, `message` | no object; a check | 15.3 |
| `@ASSERT-USAGE` | `members`: set (usage); `definition`: definition (a general definition), optional | `symbol`, `drawn` (bool), `same`, `different` (arrays; only `"definition"`), `message` | no object; a check | 15.3 |
| `@ASSERT-IN` | `members`: set (any object); `set`: any (set) | `message` | no object; a check | 15.3 |
| `@ASSERT-TOPOLOGY` | `members`: set (instance, bundle, usage, set) | `topology` (position 1; required; `"series"`, `"parallel"`, `"star"`, `"loop"`), `exclude` (array of pairs: type, terminal), `message` | no object; a check | 15.4 |

### A.12 Other standard words

- Kinds: `instance`, `terminal`, `pin`, `net`, `point`, `wire`, `usage`, `bundle`, `text`, `annotation`, `label` (5.3, 9.4).
- Sub-objects after `::`: `position`, `net` (6.5).
- Attributes read by `${…}`: `name`, `value`, `model`, `type`, `net`, `x`, `y` (13.6).
- Truth values: `true`, `false` (3.7).

Style properties (9.3):

| Property | Type and values | Default |
|---|---|---|
| `linecolor` | color | — |
| `linestyle` | `"solid"`, `"dashed"`, `"dotted"`, `"dash-dot"`, `"double"`, `"bold"`, `"none"` | `"solid"` |
| `linewidth` | number | `0` |
| `cap` | `"butt"`, `"round"`, `"square"` | — |
| `join` | `"miter"`, `"round"`, `"bevel"` | — |
| `fillcolor` | color | — |
| `textcolor` | color | — |
| `size` | number | — |
| `font` | string | — |
| `bold` | bool | — |
| `italic` | bool | — |
| `arrowsize` | number | — |
| `textvisible` | bool | `true` |

Style properties that keywords take as parameters (9.2.5): *line*: `linecolor`, `linestyle`, `linewidth`, `cap`, `join`, `fillcolor`, `arrowsize`; *area*: `linecolor`, `linestyle`, `linewidth`, `cap`, `join`, `fillcolor`; *wire*: `linecolor`, `linestyle`, `linewidth`, `cap`, `join`; *text*: `textcolor`, `size`, `font`, `bold`, `italic`, `textvisible`.
