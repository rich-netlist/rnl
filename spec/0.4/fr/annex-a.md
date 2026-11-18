## Annexe A Signatures des mots-clés (normative)

La présente annexe est générée à partir de la liste de mots `rnl-words-0.4.json` (5.3) ; en cas de divergence entre les deux, il s’agit d’un erratum. Elle énumère ce que chaque mot-clé reçoit et produit ; ce qu’il signifie figure dans la section nommée dans la dernière colonne.

- **Emplacements** : ils sont listés dans l’ordre standard, avec leurs noms (qui peuvent s’écrire comme des rôles, 5.3) et ce qu’ils prennent : *un* (*one*) : un objet unique ; recevant un ensemble, le mot-clé est appliqué à chaque membre (5.2.1), à ceci près que PRIORITY prend un ensemble marqué par GROUP comme le groupe lui-même (14.8) ; *ensemble* (*set*) : un ensemble ; des objets libres écrits pour l’emplacement forment un seul ensemble (5.2.1) ; *quelconque* (*any*) : un objet ou un ensemble, utilisé comme l’indique le mot-clé (pour DISTANCE, un ensemble représente chacun de ses membres, 14.5) ; *forme* (*shape*) : un point, ou un ensemble de points (deux formes d’un graphique, 9.2.5) ; *corps* (*body*) : des instructions entre accolades, en inclusion fermée (5.4) ; *définition* (*definition*) : une définition : un nom en `$`, un nom en `%` lié à une définition, ou une définition anonyme (5.6, 6.6). L’entrée absorbée est stricte ; l’entrée en inclusion fermée est tolérante (5.4, 7.3).
- **Sortes** des emplacements : ce sont des noms de sortes (`instance`, `terminal`, `pin`, `net`, `point`, `wire`, `usage`, `bundle`, `text`, `annotation`, `label`) ou : *tout* (*any*), tout objet ; *niveau* (*level*), `#` seul : le niveau lui-même ; *propriétaire* (*owner*), `?` seul : dans un corps de symbole, l’objet dessiné avec le symbole ; dans le corps d’une définition générale, l’utilisation en cours de développement (9.4) ; *position*, le sous-objet `::position` d’un objet (son origine) ; *localisable* (*locatable*), un objet pourvu d’une position : instance, faisceau, broche, borne pourvue d’une broche, point, texte, le centre d’un graphique écrit avec un centre, une utilisation par son point propre ; un ensemble donné pour cela représente chacun de ses membres (14.1) ; *orientable*, un objet pourvu d’une orientation : instance, faisceau, utilisation pourvue d’un symbole ; *contrainte* (*constraint*), un objet créé par un mot-clé de contrainte ; *ensemble* (*set*), un ensemble, écrit sous forme de nom ; dans ASSERT-TOPOLOGY également un élément qui est un `{…}` écrit explicitement (15.4) ; *nœud vide* (*empty node*), `&` seul : aucun net ; pris par les rôles de sorte `net` et par l’emplacement de net de WIRE, et ailleurs comme un ensemble vide de nets (6.3).
- **Paramètres** : ceux marqués *position 1*, *position 2* peuvent être donnés par position, dans cet ordre ; tous les autres uniquement par nom (4.2). `priority` prend toujours `"disabled"`, `"low"`, `"normal"` ou `"high"` ; sans lui la priorité de l’en-tête d’enregistrement s’applique, sauf qu’AS et ATTACH n’en prennent que `"disabled"` (14.8).
- **Résultat** : l’objet que produit l’instruction (5.5) et comment ses faits surchargent (5.8). Sans surcharge, les faits s’accumulent.
- Sauf mention contraire, un mot-clé peut être écrit dans n’importe quel enregistrement d’un niveau ; *corps de symboles* signifie qu’il peut aussi être écrit dans un corps de DEFINE-SYMBOL.

### A.1 L’enregistrement

`@RNL(…) {…}` verse ses instructions dans le niveau (5.4). Champs d’en-tête d’enregistrement (2.8), tous donnés par nom :

| Champ | Valeurs | Valeur par défaut | Affecte l’interprétation |
|---|---|---|---|
| `type` | `"authored"`, `"generated"`, `"header"` | `"authored"` | oui |
| `scope` | chaîne | — | oui |
| `priority` | `"disabled"`, `"low"`, `"normal"`, `"high"` | `"normal"` | oui |
| `author` | chaîne | — | non |
| `hash` | chaîne | — | non |
| `numbering` | chaîne | — | oui |
| `modified` | chaîne | — | non |
| `language` | `"spice"`, `"spectre"` | — | oui |

### A.2 Enregistrements d’en-tête et paquets

| Mot-clé | Emplacements | Paramètres | Résultat | Section |
|---|---|---|---|---|
| `@VERSION` (enregistrements d’en-tête uniquement) | — | `version` (position 1 ; requis) | — | 16.1 |
| `@PACKAGE` (enregistrement d’en-tête d’un fichier de paquet uniquement) | — | `name` (requis), `version` (requis) | — | 2.7 |
| `@USE-PACKAGE` (aussi dans les enregistrements d’en-tête) | `body` : corps, facultatif | `name` (requis), `version`, `path` | — | 16.2 |
| `@ALIAS` (enregistrements d’en-tête uniquement) | — | `alias` (position 1 ; requis), `original` (position 2 ; requis), `definition` | — | 16.5 |
| `@USE-ALIASES` (enregistrements d’en-tête uniquement) | — | `name` (requis), `version`, `path` | — | 16.5 |
| `@DEVICE` (enregistrements d’en-tête uniquement) | — | `name`, `model`, `terminals` (tableau), `type` (chaîne) ; au moins l’un de `name`, `model` ; au moins l’un de `terminals`, `type` | — | 6.12 |

### A.3 Définitions

| Mot-clé | Emplacements | Paramètres | Résultat | Section |
|---|---|---|---|---|
| `@DEFINE-SYMBOL` (aussi dans les corps de symboles) | `body` : corps | `box` (tableau) | une définition (symbole) | 9.2 |
| `@DEFINE-STYLE` | `extends` : corps (un style) | propriétés de style (9.3) | une définition (style) | 9.3 |
| `@DEFINE` | `body` : corps | `roles` (requis ; tableau), `parameters` (tableau) | une définition (générale) | 9.4 |

### A.4 Contenu des symboles et graphiques

| Mot-clé | Emplacements | Paramètres | Résultat | Section |
|---|---|---|---|---|
| `@PIN` (corps de symboles uniquement) | `position` : un (point) | `terminal` (borne, ou tableau non vide de bornes ; peut être omis), `label`, `direction` (`"right"`, `"up"`, `"left"`, `"down"`) | une nouvelle broche | 9.2.4 |
| `@LINE` (aussi dans les corps de symboles) | `points` : ensemble (point, broche, borne) | `arrow` (`"none"`, `"start"`, `"end"`, `"both"` ; par défaut `"none"`), `fill` (`"none"`, `"linecolor"`, `"fillcolor"` ; par défaut `"none"`), propriétés de style de trait | un graphique du symbole ; dans la scène, une nouvelle annotation (13.4) | 9.2.5 |
| `@RECTANGLE` (aussi dans les corps de symboles) | `points` : forme (point, broche, borne) | `width` (nombre), `height` (nombre), `fill` (`"none"`, `"linecolor"`, `"fillcolor"` ; par défaut `"none"`), propriétés de style de surface ; `width` et `height` ensemble ou pas du tout | un graphique du symbole ; dans la scène, une nouvelle annotation (13.4) | 9.2.5 |
| `@CIRCLE` (aussi dans les corps de symboles) | `points` : forme (point, broche, borne) | `radius` (nombre), `fill` (`"none"`, `"linecolor"`, `"fillcolor"` ; par défaut `"none"`), propriétés de style de surface | un graphique du symbole ; dans la scène, une nouvelle annotation (13.4) | 9.2.5 |
| `@ELLIPSE` (aussi dans les corps de symboles) | `points` : forme (point, broche, borne) | `width` (nombre), `height` (nombre), `fill` (`"none"`, `"linecolor"`, `"fillcolor"` ; par défaut `"none"`), propriétés de style de surface ; `width` et `height` ensemble ou pas du tout | un graphique du symbole ; dans la scène, une nouvelle annotation (13.4) | 9.2.5 |
| `@ARC` (aussi dans les corps de symboles) | `points` : ensemble (point, broche, borne) | `arrow` (`"none"`, `"start"`, `"end"`, `"both"` ; par défaut `"none"`), `fill` (`"none"`, `"linecolor"`, `"fillcolor"` ; par défaut `"none"`), propriétés de style de trait | un graphique du symbole ; dans la scène, une nouvelle annotation (13.4) | 9.2.5 |
| `@BEZIER` (aussi dans les corps de symboles) | `points` : ensemble (point, broche, borne) | `arrow` (`"none"`, `"start"`, `"end"`, `"both"` ; par défaut `"none"`), `fill` (`"none"`, `"linecolor"`, `"fillcolor"` ; par défaut `"none"`), propriétés de style de trait | un graphique du symbole ; dans la scène, une nouvelle annotation (13.4) | 9.2.5 |
| `@ANNOTATION` | `at` : un (localisable) ; `source` : un (un symbole, une définition générale ou une utilisation), écrit en dernier | `rotate` (nombre, degrés dans le sens antihoraire ; par défaut `0`), `mirror` (booléen ; par défaut `false`), propriétés de style, et les paramètres d’un symbole ou d’une définition générale donnés comme `source`, par nom (9.4) | une nouvelle annotation | 13.5 |
| `@TEXT` (aussi dans les corps de symboles) | `attach` : un (point, instance, utilisation, faisceau, broche, borne, fil, annotation, net, `?`) | `text` (position 1 ; requis), `key`, `dx` (nombre), `dy` (nombre), `halign` (`"start"`, `"middle"`, `"end"` ; par défaut `"start"`), `valign` (`"baseline"`, `"top"`, `"middle"`, `"bottom"` ; par défaut `"baseline"`), `rotate` (nombre, degrés dans le sens antihoraire ; par défaut `0`), `fix_rotate` (booléen ; par défaut `false`), `upright` (booléen ; par défaut `true`), propriétés de style de texte | un nouveau texte ; avec `key`, un sous-objet de l’objet attaché ; surcharge par clé (clé : objet attaché et clé, lorsqu’une clé est donnée) | 13.3 |

### A.5 Placement et apparence

| Mot-clé | Emplacements | Paramètres | Résultat | Section |
|---|---|---|---|---|
| `@PLACE` (aussi dans les corps de symboles) | `target` : un (instance, utilisation, faisceau, point, texte, `::position`) | `x` (position 1 ; nombre), `y` (position 2 ; nombre), `priority` ; au moins l’un de `x`, `y` | l’objet d’entrée ; fait géométrique (clé : objet et axe) | 11.1 |
| `@ORIENT` (aussi dans les corps de symboles) | `target` : un (instance, utilisation, faisceau, broche) | `code` (position 1 ; requis ; `"R0"`, `"R90"`, `"R180"`, `"R270"`, `"MX"`, `"MY"`, `"MXR90"`, `"MYR90"`, ou les mots `"left"`, `"down"`, `"right"`, `"up"` pour les quatre premiers ; ou un tableau non vide de ceux-ci), `priority` | l’objet d’entrée ; fait géométrique (clé : objet, ou broche) | 11.2 |
| `@LAYER` | `target` : un (instance, utilisation, faisceau, fil, annotation, texte) | `layer` (position 1 ; requis ; `"bottom"`, `"middle"`, `"top"`) | l’objet d’entrée ; surcharge par clé (clé : objet) | 11.6 |
| `@SET-SYMBOL` (aussi dans les corps de symboles) | `target` : un (instance, utilisation, faisceau, `#`) ; `symbol` : définition (un symbole) | `terminals` (tableau de noms de bornes, ou de paires) | l’objet d’entrée ; file d’apparence | 11.3 |
| `@SET-STYLE` | `target` : un (instance, utilisation, faisceau, fil, annotation, texte) ; `style` : définition (un style) | — | l’objet d’entrée ; file d’apparence | 11.3 |

### A.6 Sélections

| Mot-clé | Emplacements | Paramètres | Résultat | Section |
|---|---|---|---|---|
| `@SELECT-INSTANCE` | `scope` : quelconque (`#`, tout objet) | `name`, `model`, `type`, `terminals` (nombre, entier), `symbol` (chaîne : un nom en `$`, avec `*`), `drawn` (booléen), `role`, `recursive` (booléen ; par défaut `false`) | un nouvel ensemble d’instances | 10.1 |
| `@SELECT-TERMINAL` | `scope` : quelconque (`#`, tout objet) | `name`, `model`, `type`, `terminal` (borne), `net`, `current` (`"in"`, `"out"`, `"unspecified"`), `role`, `recursive` (booléen ; par défaut `false`) | un nouvel ensemble de bornes | 10.1 |
| `@SELECT-PIN` | `scope` : quelconque (`#`, tout objet) | `name`, `model`, `type`, `terminal` (borne), `net`, `wired` (booléen), `role`, `recursive` (booléen ; par défaut `false`) | un nouvel ensemble de broches | 10.1 |
| `@SELECT-NET` | `scope` : quelconque (`#`, tout objet) | `name`, `port` (booléen), `wired` (booléen), `global` (booléen), `role`, `recursive` (booléen ; par défaut `false`) | un nouvel ensemble de nets | 10.1 |
| `@SELECT-WIRE` | `scope` : quelconque (`#`, tout objet) | `net`, `bus` (booléen), `role`, `recursive` (booléen ; par défaut `false`) | un nouvel ensemble de fils | 10.1 |
| `@SELECT-LABEL` | `scope` : quelconque (`#`, tout objet) | `key`, `role`, `recursive` (booléen ; par défaut `false`) | un nouvel ensemble d’étiquettes | 10.1 |
| `@SELECT-ANNOTATION` | `scope` : quelconque (`#`, tout objet) | `role`, `recursive` (booléen ; par défaut `false`) | un nouvel ensemble d’annotations | 10.1 |
| `@SELECT-USAGE` | `scope` : quelconque (`#`, tout objet) ; `definition` : définition (une définition générale), facultatif | `symbol` (chaîne : un nom en `$`, avec `*`), `drawn` (booléen), `recursive` (booléen ; par défaut `false`) | un nouvel ensemble d’utilisations | 10.1 |
| `@SELECT-ANCESTOR-USAGE` | `scope` : quelconque (tout objet) ; `definition` : définition (une définition générale) | `recursive` (booléen ; par défaut `false`) | un nouvel ensemble d’utilisations | 10.1 |

### A.7 Application de définitions

| Mot-clé | Emplacements | Paramètres | Résultat | Section |
|---|---|---|---|---|
| `@AS` (aussi dans les corps de symboles) | les rôles de la définition, puis la définition (`definition`, écrite en dernier) | les paramètres de la définition (par position dans l’ordre de sa liste de paramètres, ou par nom ; 9.4), `priority` | une nouvelle utilisation | 9.4 |
| `@ATTACH` (aussi dans les corps de symboles) | la définition, puis ses rôles (`definition`, écrite en premier) | les paramètres de la définition (par position dans l’ordre de sa liste de paramètres, ou par nom ; 9.4), `priority` | une nouvelle utilisation | 9.4 |

### A.8 Fils

| Mot-clé | Emplacements | Paramètres | Résultat | Section |
|---|---|---|---|---|
| `@WIRE` | `net` : quelconque (net, ou le nœud vide) ; `points` : ensemble (point, broche ; une borne représente sa broche, 5.2.4) | `mode` (`"auto"`, `"straight"`, `"horizontal-vertical"`, `"vertical-horizontal"`, `"manhattan"`, `"manhattan45"` ; par défaut `"auto"`), propriétés de style de fil | un nouveau fil ; recevant un fil, le réécrit, surchargeant par clé (clé : ce fil) | 13.1 |
| `@ROUTE-NET` | `net` : un (net) | `mode` (`"auto"`, `"straight"`, `"horizontal-vertical"`, `"vertical-horizontal"`, `"manhattan"`, `"manhattan45"` ; par défaut `"auto"`), propriétés de style de fil, `priority` | un nouveau routage | 13.7 |
| `@ROUTE-OBJECT` | `objects` : ensemble (tout objet) | `mode` (comme ROUTE-NET), propriétés de style de fil, `priority` | l’objet d’entrée | 13.7 |
| `@UNROUTE-NET` | `net` : un (net) | `priority` | l’objet d’entrée | 13.7 |
| `@UNROUTE-OBJECT` | `objects` : ensemble (tout objet) | `priority` | l’objet d’entrée | 13.7 |

### A.9 Ensembles, groupes et faisceaux

| Mot-clé | Emplacements | Paramètres | Résultat | Section |
|---|---|---|---|---|
| `@BUNDLE` | `members` : ensemble (instance) | — | un nouveau faisceau | 11.8 |
| `@GROUP` | `members` : ensemble (tout objet) | `spacing_x` (nombre, entier), `spacing_y` (nombre, entier) | l’objet d’entrée | 11.5 |
| `@ADD-TO` | `items` : quelconque (tout objet) ; `target` : quelconque (ensemble) | — | l’ensemble cible | 5.2.2 |
| `@REMOVE-FROM` | `items` : quelconque (tout objet) ; `target` : quelconque (ensemble) | — | l’ensemble cible | 5.2.2 |
| `@UNION` | `sets` : ensemble (tout objet) | — | un nouvel ensemble | 5.2.2 |

### A.10 Contraintes et priorité

| Mot-clé | Emplacements | Paramètres | Résultat | Section |
|---|---|---|---|---|
| `@ALIGN` | `members` : ensemble (localisable) | `axis` (position 1 ; requis ; `"x"`, `"y"`), `priority` | une nouvelle contrainte ; fait géométrique | 14.2 |
| `@ORDER` | `members` : ensemble (localisable) | `direction` (position 1 ; requis ; `"left-to-right"`, `"right-to-left"`, `"bottom-to-top"`, `"top-to-bottom"`), `priority` | une nouvelle contrainte ; fait géométrique | 14.3 |
| `@DISTRIBUTE` | `members` : ensemble (localisable) | `direction` (position 1 ; requis ; `"left-to-right"`, `"right-to-left"`, `"bottom-to-top"`, `"top-to-bottom"`), `priority` | une nouvelle contrainte ; fait géométrique | 14.4 |
| `@DISTANCE` | `from` : quelconque (localisable) ; `to` : quelconque (localisable) | `direction` (position 1 ; requis ; `"left-to-right"`, `"right-to-left"`, `"bottom-to-top"`, `"top-to-bottom"`), `min` (nombre), `max` (nombre), `exact` (nombre), `priority` ; au moins l’un de `min`, `max`, `exact` ; `exact` non combiné avec `min` ; `exact` non combiné avec `max` | une nouvelle contrainte ; fait géométrique | 14.5 |
| `@ORIENT-RELATION` | `from` : un (orientable, ou broche) ; `to` : un (orientable, ou broche, comme `from`) | `relation` (position 1 ; requis ; les huit codes d’ORIENT, non ses mots, ou un tableau non vide de ceux-ci), `priority` | une nouvelle contrainte ; fait géométrique | 14.9 |
| `@TABLE` | `rows` : ensemble (localisable ; les ensembles imbriqués sont les listes de la table) | `transpose` (booléen ; par défaut `false`), `priority` | une nouvelle contrainte ; fait géométrique | 14.11 |
| `@SOLVER-CONFIG` (enregistrement d’en-tête sans `language`, enregistrement d’en-tête d’un fichier de paquet, et niveau) | — | paramètres nommés du solveur : nombres, chaînes, valeurs booléennes et tableaux de ceux-ci ; `spacing_x`, `spacing_y` (nombre, entier) et `junction` (chaîne : un nom en `$` ; dans un enregistrement de niveau aussi un nom en `%` du niveau ; ou vide) sont nommés par la norme ; les autres clés sont celles du solveur | aucun objet ; réglages du solveur | 14.10 |
| `@PRIORITY` | `target` : un (contrainte, utilisation, ensemble) | `priority` (position 1 ; requis) | l’objet d’entrée ; surcharge par clé (clé : objet) | 14.8 |

### A.11 Sémantique électrique

| Mot-clé | Emplacements | Paramètres | Résultat | Section |
|---|---|---|---|---|
| `@CURRENT` | `terminal` : un (borne) | `direction` (position 1 ; requis ; `"in"`, `"out"`, `"unspecified"`) | l’objet d’entrée ; surcharge par clé (clé : borne) | 15.2 |
| `@ASSERT-INSTANCE` | `members` : ensemble (instance) | les conditions de SELECT-INSTANCE sauf `role` et `recursive`, `has` (tableau de noms d’attributs), `same` (tableau de noms d’attributs), `different` (tableau de noms d’attributs), `message` | aucun objet ; une vérification | 15.3 |
| `@ASSERT-TERMINAL` | `members` : ensemble (borne) | les conditions de SELECT-TERMINAL sauf `role` et `recursive`, `same`, `different`, `message` | aucun objet ; une vérification | 15.3 |
| `@ASSERT-NET` | `members` : ensemble (net) | les conditions de SELECT-NET sauf `role` et `recursive`, `same`, `different`, `message` | aucun objet ; une vérification | 15.3 |
| `@ASSERT-USAGE` | `members` : ensemble (utilisation) ; `definition` : définition (une définition générale), facultatif | `symbol`, `drawn` (booléen), `same`, `different` (tableaux ; uniquement `"definition"`), `message` | aucun objet ; une vérification | 15.3 |
| `@ASSERT-IN` | `members` : ensemble (tout objet) ; `set` : quelconque (ensemble) | `message` | aucun objet ; une vérification | 15.3 |
| `@ASSERT-TOPOLOGY` | `members` : ensemble (instance, faisceau, utilisation, ensemble) | `topology` (position 1 ; requis ; `"series"`, `"parallel"`, `"star"`, `"loop"`), `exclude` (tableau de paires : type, borne), `message` | aucun objet ; une vérification | 15.4 |

### A.12 Autres mots standards

- Sortes : `instance`, `terminal`, `pin`, `net`, `point`, `wire`, `usage`, `bundle`, `text`, `annotation`, `label` (5.3, 9.4).
- Sous-objets après `::` : `position`, `net` (6.5).
- Attributs lus par `${…}` : `name`, `value`, `model`, `type`, `net`, `x`, `y` (13.6).
- Valeurs booléennes : `true`, `false` (3.7).

Propriétés de style (9.3) :

| Propriété | Type et valeurs | Valeur par défaut |
|---|---|---|
| `linecolor` | couleur | — |
| `linestyle` | `"solid"`, `"dashed"`, `"dotted"`, `"dash-dot"`, `"double"`, `"bold"`, `"none"` | `"solid"` |
| `linewidth` | nombre | `0` |
| `cap` | `"butt"`, `"round"`, `"square"` | — |
| `join` | `"miter"`, `"round"`, `"bevel"` | — |
| `fillcolor` | couleur | — |
| `textcolor` | couleur | — |
| `size` | nombre | — |
| `font` | chaîne | — |
| `bold` | booléen | — |
| `italic` | booléen | — |
| `arrowsize` | nombre | — |
| `textvisible` | booléen | `true` |

Propriétés de style que les mots-clés prennent comme paramètres (9.2.5) : *trait* (*line*) : `linecolor`, `linestyle`, `linewidth`, `cap`, `join`, `fillcolor`, `arrowsize` ; *surface* (*area*) : `linecolor`, `linestyle`, `linewidth`, `cap`, `join`, `fillcolor` ; *fil* (*wire*) : `linecolor`, `linestyle`, `linewidth`, `cap`, `join` ; *texte* (*text*) : `textcolor`, `size`, `font`, `bold`, `italic`, `textvisible`.
