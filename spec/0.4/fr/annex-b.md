<!-- translated from en/annex-b.md sha256:f5f5cbcacdf4 -->
## Annexe B Phrases standards (informative)

Une phrase standard est une instruction sous forme standard à laquelle du texte libre est ajouté entre ses jetons, afin que l’instruction se lise comme une phrase. Le texte libre n’en modifie pas le sens (3.3) ; les règles de formulation se trouvent en 4.5. Dans les tableaux, `…` au début d’une phrase représente les segments qui la précèdent dans un chaînage, et `{…}` ou un `…` final représente du contenu omis.

### B.1 Placement et orientation

| Mot-clé | Cas | Phrase |
|---|---|---|
| PLACE | deux axes | `#M1 is at @PLACE(10, 20)` |
| | un axe | `the x of #M1 is at @PLACE(x=10)` |
| | écrit par un éditeur | `the x of #X2 is at @PLACE(x=3, priority="high")` |
| | dans un chaînage | `… => it is at @PLACE(40, 0)` |
| ORIENT | pivoté | `#M2 @ORIENT("R90")` |
| | en miroir | `#M2 @ORIENT("MX"), i.e., flipped upside down` |
| | | `#M2 @ORIENT("MY"), i.e., flipped left-right` |
| | | `#M3 @ORIENT("MXR90"), i.e., flipped upside down, then rotated 90°` |
| | | `#M3 @ORIENT("MYR90"), i.e., flipped left-right, then rotated 90°` |
| | l’un ou l’autre de deux | `#M5 @ORIENT(["R0", "MY"]), i.e., not turned or flipped left-right` |
| | dans un chaînage | `… => they @ORIENT("R90")` |
| ORIENT-RELATION | deux objets | `from #M1 to #M2, they have @ORIENT-RELATION("MY"), i.e., flipped left-right` |
| | l’un ou l’autre de deux | `from #M3 to #M4, they have @ORIENT-RELATION(["R0", "MY"]), i.e., the same or flipped left-right` |
| | la même orientation | `from #M5 to {#M7 #M8}, they have @ORIENT-RELATION("R0"), i.e., the same orientation` |
| LAYER | | `#X is on @LAYER("bottom")` |
| | dans un chaînage | `… => they are on @LAYER("top")` |
| GROUP | | `{#M1 #M2 #M5} as a @GROUP` |
| | nommé | `%s1_set = {#M1 #M2 #M5} as a @GROUP` |
| | espacement | `{#M3 #M4} as a @GROUP(spacing_x=-1), i.e., one grid unit tighter in x` |

### B.2 Contraintes de disposition

| Mot-clé | Cas | Phrase |
|---|---|---|
| ALIGN | même y | `{#M1 #M2} @ALIGN("y"), i.e., same y` |
| | même x | `{#M1 #C1} @ALIGN("x"), i.e., same x` |
| | basse priorité | `{#M7 #M5} @ALIGN("y", priority="low"), i.e., same y` |
| | dans un chaînage | `… => they @ALIGN("y")` |
| ORDER | avec une direction | `{#M1 #M2 #M3} are in @ORDER("left-to-right")` |
| | dans un chaînage | `… => they are in @ORDER("bottom-to-top")` |
| DISTRIBUTE | | `{#R1 #R2 #R3} are in @DISTRIBUTE("left-to-right"), i.e., in equal steps` |
| | dans un chaînage | `… => they are in @DISTRIBUTE("bottom-to-top"), i.e., in equal steps` |
| DISTANCE | avec une direction | `from #M1 to #M2, they have @DISTANCE("bottom-to-top", min=4)` |
| | depuis un ensemble | `from {#M6 #M7 #CL} to %pout, they have @DISTANCE("left-to-right", min=2)` |
| TABLE | | `@TABLE layout: {{#M1 #M2 #M3} {#M4 {#M5 #M6} #M7}}` |
| | en colonnes | `@TABLE(transpose=true) layout: {{#M1 #M2} {#M3 #M4}}` |
| PRIORITY | | `%c1 has @PRIORITY("disabled")` |
| | dans un chaînage | `… => they have @PRIORITY("low")` |
| SOLVER-CONFIG | | `@SOLVER-CONFIG(spacing_x=2, spacing_y=1)` |

- `the x of` (ou `the y of`) ne s’écrit qu’avant un PLACE qui donne un seul axe (B.1) ; ALIGN prend à la place la glose `, i.e., same x`.
- Le x est celui de la position de chaque membre : pour une instance, le point que fixe PLACE (l’origine de son symbole), et non le centre de son contour ; un ensemble vaut pour chacun de ses membres (14.1).

### B.3 Apparence

| Mot-clé | Cas | Phrase |
|---|---|---|
| SET-SYMBOL | un objet | `#M1 @SET-SYMBOL to $NMOS` |
| | un ensemble | `{#M1 #M2} @SET-SYMBOL to $NMOS` |
| | le symbole de bloc d’un sous-circuit | `this subcircuit # @SET-SYMBOL to $NAND2_SYM` |
| | dans un chaînage | `… => they @SET-SYMBOL to $PMOS` |
| SET-STYLE | | `{#CC #RZ} @SET-STYLE to $orange` |
| | dans un chaînage | `… => they @SET-STYLE to $grey` |
| DEFINE-STYLE | | `$orange = @DEFINE-STYLE(linecolor="#e07000", textcolor="#e07000") {}` |
| | fusionnant un autre style | `$wide = @DEFINE-STYLE(linewidth=2) based on {$orange}` |

### B.4 Symboles et graphismes

| Mot-clé | Cas | Phrase |
|---|---|---|
| DEFINE-SYMBOL | | `$TRI = @DEFINE-SYMBOL {…}` |
| PIN | | `at (-4, 0) there is a @PIN(terminal="in", direction="left")` |
| LINE | | `through {(0, 0) (4, 0)} runs a @LINE` |
| ARC | | `through {(0, 2) (2, 0) (0, -2)} runs an @ARC` |
| BEZIER | | `the control points {(0, 0) (1, 1) (2, 1) (3, 0)} make a @BEZIER` |
| CIRCLE | | `centered at (0, 0), a @CIRCLE(radius=1)` |
| RECTANGLE | centre et taille | `centered at (0, 0), a @RECTANGLE(width=4, height=2)` |
| | deux coins | `with corners {(0, 0) (4, 2)}, a @RECTANGLE` |
| ELLIPSE | centre et taille | `centered at (0, 0), an @ELLIPSE(width=4, height=2)` |
| | coins de sa boîte | `in the box with corners {(0, 0) (4, 2)}, an @ELLIPSE` |
| TEXT | dans un symbole | `on ? there is a @TEXT("${name}", key="name") => it is at @PLACE(-2, 2.5)` |
| ATTACH | dans un symbole | `$ARROW is @ATTACH to (0, -1.5) => …` |

- Les graphismes écrits dans la scène, en tant qu’annotations (13.4), se lisent de même.

### B.5 Fils et texte

| Mot-clé | Cas | Phrase |
|---|---|---|
| WIRE | | `net &n1, through {#R1:2 %j #R2:1}, has a @WIRE` |
| | un bus | `net &D<3:0>, through {%p1 %p2}, has a @WIRE, which is a bus` |
| | un mode de routage | `net &in, through {#VIN:1 #R1:1}, has a @WIRE(mode="vertical-horizontal")` |
| ROUTE-NET | | `net &n1 is drawn with @ROUTE-NET` |
| | un ensemble | `nets {&a &b} are drawn with @ROUTE-NET` |
| | dans un chaînage | `… => it is drawn with @ROUTE-NET` |
| ROUTE-OBJECT | | `the pins of {#M1 #M2 #M5} are joined with @ROUTE-OBJECT` |
| UNROUTE-NET | | `net &vdd is left out with @UNROUTE-NET(priority="high")` |
| UNROUTE-OBJECT | | `the pins of #M3 are left out with @UNROUTE-OBJECT` |
| TEXT | sur un objet | `on #M1 there is a @TEXT("W/L = ${w}/${l}", key="wl", dy=-3)` |
| | en un point | `at (_, _) there is a @TEXT("RC low-pass") => named %title` |
| | dans un chaînage | `… => on them there is a @TEXT("${w}", key="size")` |

- Nommer un point, tel que `%j = (_, _)`, n’ajoute aucun mot.

### B.6 Définitions et applications

| Mot-clé | Cas | Phrase |
|---|---|---|
| DEFINE | | `$PAIR = a definition, @DEFINE(roles=["a", "b"], parameters=[["gap", 2]]) {…}` |
| AS | trois rôles | `#M1(left), #M2(right) and #M5(tail) are treated @AS $DIFFERENTIAL_PAIR_N` |
| | un rôle qui prend un ensemble | `{#M5 #M7 #M8} are treated @AS $MATCHED` |
| | un objet | `#M3 is treated @AS $DIODE_CONNECTED` |
| | avec une priorité | `#M1(left), #M2(right) and %ax(axis) are treated @AS(priority="high") $MIRRORED_LAYOUT` |
| | un paramètre de la définition | `{#R1 #R2} are treated @AS(gap=6) $SPACED` |
| | une utilisation dans un rôle | `%dp(pair) and %cm(load) are treated @AS $OTA` |
| | nommé | `%dp = #M1(left), #M2(right) and #M5(tail) treated @AS $DIFFERENTIAL_PAIR_N` |
| | nommé dans un chaînage, avec un autre objet | `#M1(left), #M2(right) and #M5(tail) are treated @AS $DIFFERENTIAL_PAIR_N => named %dp => it and %cm are treated @AS $OTA` |
| ATTACH | sur un net | `$GROUND is @ATTACH to &vss` |
| | sur une borne | `$NO_CONNECT is @ATTACH to #M1:4` |
| | sur un point | `$JUNCTION is @ATTACH to %j` |
| | deux rôles | `$DIFF_PORT is @ATTACH to &inp(a) and &inn(b)` |
| | suivi d’autres segments, nommé | `$GROUND is @ATTACH to &vss => it is at @PLACE(24, 38) => named %g0` |
| | sur chacun de plusieurs objets | `$SIZE_LABEL is @ATTACH to {#M1 #M2 #M3}` |
| | sur un ensemble | `$GROUND_STUB is @ATTACH to $GROUND_FILL_SET` |
| | la complétion, dans un chaînage | `among the instances at each level in # and below, @SELECT-INSTANCE(recursive=true) => among the pins of them, @SELECT-PIN(net="vss", wired=false) => $GROUND_STUB is @ATTACH(gap=3) to them` |

- AS est utilisé lorsque les objets forment ensemble la chose, telle qu’une structure de circuit, un placement ou une façon locale de dessiner telle que `$RC_STAGE` ; ATTACH lorsque la chose est ajoutée sur les objets, telle qu’une masse, une étiquette de port, une marque de non-connexion, un point de jonction, une étiquette pour chaque instance ou la complétion. Les deux mots-clés font la même chose ; seule la place de la définition diffère (9.4).
- `gap` est un paramètre de `$GROUND_STUB`, la distance minimale depuis la broche vers le bas jusqu’au symbole de masse (1 par défaut). Comme les noms de rôles `left` et `axis`, c’est un nom donné dans une définition, et non un mot standard.

### B.7 Sélections

Une sélection s’écrit comme un choix parmi des candidats : `among <candidates>, @SELECT-…(conditions)`. La formulation des candidats dépend du genre de la portée et du genre sélectionné :

| Portée | Sélection | Candidats |
|---|---|---|
| `#` | tout genre | `the <kind> in # only` |
| `#`, dans une instruction qui atteint les niveaux inférieurs (10.1) | tout genre | `the <kind> at each level in # and below` |
| un ensemble du genre sélectionné (un filtre) | ce genre | `{…}` |
| net | instances | `the instances connected to &en` |
| net | bornes | `the terminals connected to &en` |
| net | fils | `the wires on &en` |
| instance | bornes | `the terminals of #R1` |
| instance, faisceau, utilisation avec symbole | broches | `the pins of #M1` |
| borne | broches | `the pins of #X1:vdd` |
| net | broches | `the pins on &en` |
| le résultat d’un chaînage | broches | `the pins of them` |
| instance | nets | `the nets connected to #M1` |
| borne | instances | `the instances owning #M1:g` |
| borne | nets | `the nets connected to #M1:g` |
| fil | nets | `the nets of %w13` |
| instance, borne, fil, net, élément graphique | étiquettes | `the labels attached to #M1` |
| utilisation (sans utilisations parmi ses membres) | tout genre | `the members of %dp` |
| utilisation (avec utilisations parmi ses membres, ou non connu) | tout genre | `all members of %st1 at any depth` |
| utilisation | utilisations (SELECT-USAGE) | `%st1 itself and the usages inside it` |
| tout objet | les utilisations qui le contiennent (SELECT-ANCESTOR-USAGE) | `the usages containing #M1` |
| un ensemble de plusieurs genres (genre non connu) | | écrire `for %mixed_set, @SELECT-…` à la place |

- `<kind>` suit le mot-clé : instances, terminals, pins, nets, wires, labels, annotations, usages.
- Au milieu d’un chaînage, la portée dans les candidats s’écrit `them`, comme `=> among all members of them at any depth,`. On ignore au moment où la phrase est écrite si les utilisations sélectionnées précédemment ont des utilisations parmi leurs membres, c’est pourquoi `at any depth` est utilisé.
- Pour les membres d’utilisations, la formulation est `at any depth`, non `at any level` : un niveau est un niveau du circuit, comme dans `at each level in # and below`.
- Une règle dans un paquet compte comme écrite dans le niveau de son USE-PACKAGE (10.4) ; avec `recursive=true`, un paquet écrit `at every level that uses this package, #` à la place de `at each level in # and below`.

| Mot-clé | Cas | Phrase |
|---|---|---|
| SELECT-INSTANCE | ce niveau et au-dessous | `among the instances at each level in # and below, @SELECT-INSTANCE(model="nch", recursive=true) => they @SET-SYMBOL to $NMOS` |
| | ce niveau seulement | `among the instances in # only, @SELECT-INSTANCE(type="pmos") => …` |
| | un filtre | `among {#M1 #M2 #R1}, @SELECT-INSTANCE(name="M*") => they @ORIENT("MY")` |
| | sur un net | `among the instances connected to &en, @SELECT-INSTANCE => they @SET-STYLE to $grey` |
| | possédant une borne | `among the instances owning #M1:g, @SELECT-INSTANCE => they are on @LAYER("top")` |
| | membres d’une utilisation | `among all members of %st1 at any depth, @SELECT-INSTANCE(role="pair")` |
| | membres d’une utilisation sans utilisations imbriquées | `among the members of %dp, @SELECT-INSTANCE` |
| | au milieu d’un chaînage | `among the usages at each level in # and below, @SELECT-USAGE(recursive=true) of $DIFFERENTIAL_PAIR_N => among all members of them at any depth, @SELECT-INSTANCE => they @SET-STYLE to $signal` |
| SELECT-TERMINAL | ce niveau et au-dessous | `among the terminals at each level in # and below, @SELECT-TERMINAL(net="vss", recursive=true) => they have @ASSERT-TERMINAL(same=["net"])` |
| | d’une instance | `among the terminals of #R1, @SELECT-TERMINAL => on them there is a @TEXT("${net}")` |
| | sur un net | `among the terminals connected to $GROUND_NET_SET, @SELECT-TERMINAL => among the pins of them, @SELECT-PIN(wired=false) => $GROUND_STUB is @ATTACH to them` |
| SELECT-PIN | dans le niveau | `among the pins in # only, @SELECT-PIN(wired=false) => on them there is a @TEXT("${net}", key="netname")` |
| | de bornes | `among the pins of {#M1:s #M2:s}, @SELECT-PIN(wired=false) => $GROUND_STUB is @ATTACH to them` |
| | d’une borne | `among the pins of #X1:vdd, @SELECT-PIN => they @SET-STYLE to $red` |
| SELECT-NET | ce niveau et au-dessous | `among the nets at each level in # and below, @SELECT-NET(port=true, recursive=true) => $IO_PORT is @ATTACH to them` |
| | d’une instance | `among the nets connected to #M1, @SELECT-NET => named %m1_net_set` |
| | d’une borne | `among the nets connected to #M1:g, @SELECT-NET => they are drawn with @ROUTE-NET` |
| | d’un fil | `among the nets of %w13, @SELECT-NET => they are drawn with @ROUTE-NET` |
| | un filtre | `among {&a &b}, @SELECT-NET(wired=false) => they are drawn with @ROUTE-NET` |
| SELECT-WIRE | ce niveau et au-dessous | `among the wires at each level in # and below, @SELECT-WIRE(bus=true, recursive=true) => they @SET-STYLE to $bus` |
| | sur un net | `among the wires on &clk, @SELECT-WIRE => they @SET-STYLE to $red` |
| SELECT-LABEL | ce niveau et au-dessous | `among the labels at each level in # and below, @SELECT-LABEL(key="wl", recursive=true) => they @SET-STYLE to $grey` |
| | attachée à un objet | `among the labels attached to #M1, @SELECT-LABEL => they @SET-STYLE to $grey` |
| SELECT-ANNOTATION | ce niveau et au-dessous | `among the annotations at each level in # and below, @SELECT-ANNOTATION(recursive=true) => they @SET-STYLE to $base` |
| | un filtre | `among {%n1 %f1 #M1}, @SELECT-ANNOTATION` |
| SELECT-USAGE | ce niveau et au-dessous | `among the usages at each level in # and below, @SELECT-USAGE(recursive=true) of $DIFFERENTIAL_PAIR_N => they have @PRIORITY("disabled")` |
| | sans définition | `among the usages at each level in # and below, @SELECT-USAGE(recursive=true) => they @SET-STYLE to $base` |
| | une utilisation et les utilisations qu’elle contient | `among %st1 itself and the usages inside it, @SELECT-USAGE of $DIFFERENTIAL_PAIR_N` |
| SELECT-ANCESTOR-USAGE | | `among the usages containing #M1, @SELECT-ANCESTOR-USAGE of $DIFFERENTIAL_PAIR_N => named %dp_set` |
| toute sélection | genre de la portée non connu | `for %mixed_set, @SELECT-TERMINAL => …` |

### B.8 Ensembles et autres mots-clés

| Mot-clé | Cas | Phrase |
|---|---|---|
| ADD-TO | | `&vdd @ADD-TO $SUPPLY_NET_SET` |
| | dans un chaînage | `… => they @ADD-TO $SUPPLY_NET_SET` |
| REMOVE-FROM | | `&agnd @REMOVE-FROM $GROUND_NET_SET` |
| UNION | | `%u_set = %a_set and %b_set @UNION` |
| BUNDLE | | `#XI<3:0> is drawn as a @BUNDLE => named %xi` |
| CURRENT | entrant | `#M5:1 has @CURRENT("in")` |
| | sortant | `#:iout has @CURRENT("out")` |
| | non spécifié | `#M5:2 has @CURRENT("unspecified")` |
| ASSERT-INSTANCE | | `{#M1 #M2} have @ASSERT-INSTANCE(same=["model", "w", "l"])` |
| ASSERT-TERMINAL | | `{#M1:s #M2:s} have @ASSERT-TERMINAL(same=["net"])` |
| ASSERT-NET | | `{&inp &inn} have @ASSERT-NET(port=true)` |
| WIRE sans net | | `between {%sub1::y %int1::a}, on no net &, there is a @WIRE` |
| ANNOTATION | | `at (40, 20) there is an @ANNOTATION(rotate=30) of $ARROW_SYMBOL` |
| ASSERT-USAGE | | `%dp1 is, by @ASSERT-USAGE, a usage of $DIFFERENTIAL_PAIR_N` |
| ASSERT-IN | | `{&vss} is, by @ASSERT-IN, in $GROUND_NET_SET` |
| ASSERT-TOPOLOGY | série | `{#R1 #R2 #R3} are connected as @ASSERT-TOPOLOGY("series")` |
| | parallèle | `{#M3 #M4} are connected as @ASSERT-TOPOLOGY("parallel")` |
| | étoile | `{#L1 #C1 #L2} are connected as @ASSERT-TOPOLOGY("star")` |
| | boucle | `{#C1 #L1 #C2} are connected as @ASSERT-TOPOLOGY("loop")` |

### B.9 Enregistrement d’en-tête et paquets

Aucun mot n’est ajouté. L’enregistrement d’en-tête est lu dans l’orthographe standard et n’utilise pas d’alias (16.5).

| Mot-clé | Cas | Phrase |
|---|---|---|
| VERSION | | `@VERSION("0.4")` |
| ALIAS | | `@ALIAS("@位于", "@PLACE")` |
| USE-ALIASES | | `@USE-ALIASES(name="alias-zh", version="0.4")` |
| DEVICE | | `@DEVICE(…)` |
| PACKAGE | | `@PACKAGE(name="acme-style", version="1.2")` |
| USE-PACKAGE | dans un enregistrement d’en-tête ; également une référence dans l’enregistrement d’en-tête d’un fichier de paquet | `@USE-PACKAGE(name="rnl-std-all", version="0.4")` |
| | dans un niveau | `@USE-PACKAGE(name="acme-style", version="1.2")` |
