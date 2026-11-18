# Guide succinct de RNL 0.4

RNL s’écrit dans les commentaires d’une netlist SPICE ou Spectre pour décrire comment le circuit est dessiné et ce que signifient ses éléments ; il ne modifie jamais le circuit. Langage `"0.4"`, série de paquets standards `"0.4"` ; les numéros tels que (14.8) renvoient aux sections de la norme. Les exemples constituent des modèles autonomes, et non un programme unique : utilisez des objets existant dans leur niveau et insérez ces extraits dans des enregistrements RNL. Les commentaires `/* */` en fournissent l’explication. Le catalogue en fin de document répertorie chaque nom sous forme de signature, et non comme du code RNL à copier tel quel.

## Un fichier

```spice
* Two resistors                                   /* line 1 is the title, never read */
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4");}
R1 a b 1k
R2 b 0 1k
* @RNL {
*   {#R1 #R2} @ALIGN("x"); /* every statement ends with ; */
*   &b @WIRE {#R1:n #R2:p};
* }
.end
```

- Les enregistrements s’écrivent dans des commentaires sur ligne entière, avec `*` en colonne 1 suivi d’une espace ; il en va de même pour `//` en mode Spectre (dans un fichier `.scs`, ou après `simulator lang=spectre`). L’enregistrement d’en-tête vient en premier. Un enregistrement placé à l’intérieur de `.subckt … .ends` appartient à ce sous-circuit.
- Sans USE-PACKAGE, aucun nom standard n’est connu. Laissez tel quel tout enregistrement d’en-tête qui vous est fourni : ses lignes `@DEVICE` donnent aux composants de PDK leurs noms de bornes et leurs types (6.12), et vous n’avez ni à les écrire ni à les modifier.
- Placez une espace ASCII avant chaque `@` ; accolez les parenthèses de paramètres au mot-clé et les parenthèses de rôle à l’objet. Les paramètres positionnels précèdent les paramètres nommés : `@PLACE(10, y=20)`. Les nombres sont sans unités et il n’existe pas d’expressions. Les noms et mots-clés RNL sont sensibles à la casse ; les noms de la netlist suivent les règles de l’hôte.

## Références

```rnl
#M1; /* an instance of this level; # alone is the level */
&vdd; &"vdd!"; /* nets; quote other characters */
#M1:d; #M1:1; /* a terminal, by name or number: the same one */
#M1::d; /* the pin of M1's symbol that draws terminal d */
#:inp; /* a port of this subcircuit */
%dp1; $GROUND; /* a name of this level; a definition, symbol, style or shared set */
%g::pin; %dp1::left; /* a part: the pin of a ground, the left role of a pair */
(10, 20); (0, _); /* points; _ leaves an axis free */
{#M1 #M2 #M3}; /* a set, in order */
&D<3:0>; /* D<3> D<2> D<1> D<0> */
%row_set{1}; %row_set{end}; %row_set{2:end}; /* members of a written set by place */
```

- Noms de bornes : M `d g s b` ; Q `c b e s` ; J Z `d g s` ; D `a k` ; R C L V I B F H W `p n` ; E G S `p n cp cn` ; T O `p1 n1 p2 n2` ; U `a b ref` ; et pour X, les ports du sous-circuit. Là où une broche est nécessaire, une borne représente sa broche.
- Chaque nom ne se lie qu’une seule fois par niveau et peut être utilisé avant sa définition. Pour déplacer `%p`, écrivez `%p @PLACE(...)`, et non un second `%p = ...`. Les noms en `%` sont locaux au niveau ; les définitions en `$` sont héritées par les niveaux inférieurs, où une autre définition peut les masquer.
- Un mot sans préfixe constitue du texte libre (ce qui produit un avertissement s’il désigne un nom connu) : `#M1 #M2 M3 @ALIGN("y");` n’aligne ainsi que M1 et M2.

## Instructions et chaînages

```rnl
#M1 @PLACE(10, 20); /* objects, one keyword, its parameters */
$GROUND @ATTACH &vss => @PLACE(20, -4); /* => passes the result on: the ground, not the net */
%g = $GROUND @ATTACH &vss; /* %x = names the result */
$GROUND @ATTACH &vdd => %g2 => @PLACE(20, 4); /* => %x names the segment before it */
#M1 #M2 @ALIGN("y"); /* loose objects are one set */
{#D1 #D2} @ORIENT("MX"); /* a one-object slot given a set does each member */
{#M1 #M2} @SET-SYMBOL $NMOS; /* two slots: exactly two objects */
{#R1 {#R2 #C2} #M1} @ORDER("left-to-right"); /* a set as one item stands for each member: R2 and C2 each between */
```

Objet transmis par chaque segment : PLACE, ORIENT, SET-SYMBOL, SET-STYLE, GROUP et CURRENT transmettent leur entrée ; AS et ATTACH, la nouvelle utilisation (ou un ensemble d’utilisations créées une par une) ; ALIGN, ORDER, DISTRIBUTE, DISTANCE, TABLE et ORIENT-RELATION, la nouvelle contrainte ; TEXT, le nouveau texte ; et SELECT-…, l’ensemble sélectionné.

## Définitions, rôles et utilisations

```rnl
$SIDE = @DEFINE(roles=["a", "b"], parameters=[["gap", 4]]) {
  {?a ?b} @ALIGN("y"); /* ?a: a role in the body */
  ?a ?b @DISTANCE("left-to-right", min=?gap); /* ?gap: a parameter */
};
#M3(a) #M4(b) @AS(gap=6) $SIDE; /* apply with roles marked, and a parameter */
#M3 #M4 @AS $SIDE; /* mark every role or none; without marks, use role order */
%dp1 = #M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N; /* every one-object role is filled: tail too */
{#M5 #M3} @AS $MATCHED; /* a set role takes the whole set: one usage */
{#M1 #M2} @AS $DIODE_CONNECTED; /* a one-object role given a set: one usage each */
$KIND = @DEFINE(roles=[["p", "net"], ["{at}", "point"]]) {}; /* common role kinds: instance terminal pin net point usage */
{%dp1::left #M9} @ALIGN("y"); /* place a structure through a member */
%g = $GROUND @ATTACH &vss; {%g::pin #M5::s} @ALIGN("x"); /* a virtual part is drawn at its own point; place it by its pin */
```

- Un `{role}` reçoit l’ensemble entier (ou un objet unique traité comme un ensemble à un seul élément) et peut être omis ; lorsque les arguments ne portent aucune marque de rôle, seuls les rôles d’ensemble en fin de liste peuvent être omis. Tout rôle à objet unique est obligatoire. Ne marquez chaque rôle qu’une fois au plus ; réunissez plusieurs membres dans un même ensemble : `{#R1 #R2}(members)`.
- Dans un corps de définition, `?` seul désigne le point propre de l’utilisation, les coordonnées sont des décalages par rapport à celui-ci, et `%mid = (_, _);` crée un objet local accessible de l’extérieur sous la forme `%u::mid`.
- Les structures standards qui placent lient le point propre de leur utilisation : une rangée ou une colonne de membres l’a sur sa ligne, deux côtés en miroir au centre entre eux, de sorte qu’une contrainte sur `%u::position` déplace la structure. La colonne *Dessine* du catalogue indique où : `$BRANCH` ne partage avec lui que le x des membres, `$BRANCH_HORIZONTAL` uniquement leur y, `$PARALLEL_HORIZONTAL` et `$MIRRORED_LAYOUT` le tiennent en leur centre. Toute autre structure le laisse libre, et une contrainte sur son `::position` ne modifie rien de dessiné.
- Privilégiez les structures qui expriment à la fois ce que sont les composants et la manière de les dessiner. Un `$BRANCH` est une structure série empilée verticalement, ses membres ordonnés de haut en bas ; son emploi habituel est une branche CC, telle qu’un chemin d’une alimentation à la masse. Écrivez chaque chemin de ce type comme un seul `$BRANCH`, et toute paire en parallèle qui s’y trouve avec une variante qui place : `{#R1 #M1 #R2} @AS $BRANCH; {#R2 #C2} @AS $PARALLEL_HORIZONTAL;`. `$PARALLEL` seul ne fait que router. Une branche ne dit rien du courant, que `@CURRENT` indique séparément. Un chemin de signal à travers des interrupteurs et des condensateurs, comme dans un échantillonneur à capacités commutées, se dessine au moyen de faits atomiques, tels qu’un ALIGN et un ORDER sur les broches de chaque rangée : `{#S1:s #S1:d #C1:p #C1:n} @ALIGN("y"); {#S1:s #S1:d #C1:p #C1:n} @ORDER("left-to-right");`. Une structure à *signification seule* ne dessine rien : employez-la là où la propriété est vraie, jamais à la place d’une structure qui place. Ne marquez un `$SIGNAL_FLOW` que sur un chemin de signal net dont vous êtes sûr ; omettez-le là où le signal emprunte plusieurs chemins, reboucle en rétroaction ou circule dans les deux sens à travers une grande partie du circuit, quel que soit le nombre de membres du chemin.

## Sélections et règles

```rnl
# @SELECT-INSTANCE(model="nch", recursive=true) => @SET-STYLE $grey; /* a rule: here and in the levels below */
# @SELECT-INSTANCE(type="pmos") => @ORIENT("MY"); /* without recursive=true: this level only */
&en @SELECT-TERMINAL(terminal="g"); /* the gate terminals on net en */
# @SELECT-PIN(wired=false); /* pins without wires */
# @SELECT-NET(port=true); # @SELECT-NET(global=true); # @SELECT-NET(wired=false);
# @SELECT-USAGE $DIFFERENTIAL_PAIR_N => @SELECT-INSTANCE(role="left"); /* the left device of every pair */
&vss @ADD-TO $GROUND_NET_SET; /* edit a shared set of the package */
&agnd @REMOVE-FROM $GROUND_NET_SET;
```

- `name`, `model`, `net` et `key` acceptent le joker `*`. `type` désigne le type issu de la carte `.model`, ou celui déclaré par un DEVICE.
- Un résultat de sélection, ainsi que tout ensemble construit à partir d’un tel résultat, n’accepte pas les places de membres (`%sel{1}`) et ne peut pas être passé à WIRE ; nommez l’objet, ou utilisez ROUTE-OBJECT.

## Faits de placement

```rnl
#M1 @PLACE(10, 20); #M1 @PLACE(x=10); /* origin at (x, y); x right, y up, grid units */
#M2 @ORIENT("MY"); #M5 @ORIENT(["R0", "MY"]); /* an object takes a code; a list lets the solver choose */
#M1 #M2 @ORIENT-RELATION("MY"); /* M2 is M1 flipped left-right */
{#M1 #M2} @ALIGN("y"); /* one row; "x" is one column */
{#M1 #M2 #M3} @ORDER("left-to-right"); /* one after the other, at least 1 apart */
{#R1 #R2 #R3} @DISTRIBUTE("top-to-bottom"); /* in order, equal steps */
#M3 #M1 @DISTANCE("top-to-bottom", min=4); /* from the first to the second: min, max or exact, none negative */
{#M1 #M2 #M5} @GROUP; /* groups the set for the solver; no own origin or rigid transform */
@TABLE layout: {{#M1 #M2 #M3} {#M4 {#M5 #M6} #M7}}; /* rows top down, cells left to right, a cell column inside; one group; no alignment */
@TABLE(transpose=true) layout: {{#M1 #M2} {#M3 #M4}}; /* columns left to right */
```

- Directions : `"left-to-right"`, `"right-to-left"`, `"bottom-to-top"`, `"top-to-bottom"`. Codes : `R0`, `R90`, `R180`, `R270` tournent dans le sens antihoraire ; `MY` inverse gauche et droite, `MX` inverse haut et bas ; `MYR90` et `MXR90` appliquent d’abord la symétrie miroir, puis la rotation.
- Les membres interviennent par l’origine pour les instances, par la broche pour les bornes et broches, par eux-mêmes pour les points et les textes, et par leur point propre pour les utilisations avec symbole. Une utilisation sans symbole représente ses membres jusqu’aux plus bas, comme dans GROUP ; son point propre est `%u::position`. Les ensembles n’ont pas d’origine : PLACE positionne chaque membre individuellement, même après un GROUP. PLACE ne place ni les broches ni les contraintes. Il n’existe pas de position « le plus à gauche » : exprimez-la par rapport à d’autres composants.
- Le solveur décide de ce qui reste libre et ne devine rien sur la nature d’un composant : écrivez tout ce qui doit être satisfait. Privilégiez le sens et les contraintes en laissant le solveur calculer le placement ; lorsque la tâche exige des coordonnées, écrivez plutôt PLACE, ORIENT et WIRE pour chaque élément. Ne mélangez pas les deux approches : optez soit pour le sens et les contraintes destinés au solveur avec peu ou pas de PLACE, soit pour des positions entièrement écrites, sans résolution.
- Ordonnez les grands blocs plutôt que les composants isolés : `{#V2 #C1 %bias %stage #C3 #R5} @ORDER("left-to-right");` dispose des colonnes entières côte à côte. Une structure représente ses membres, de sorte que chaque composant de `%stage` se trouve entre chaque composant de `%bias` et C3 ; un composant partagé par deux structures n’est pas ordonné par rapport à lui-même. Pour contraindre plutôt le centre d’une structure, écrivez son point propre : `{%dp::position #X1} @ALIGN("y");` place X1 à la hauteur du centre de la paire, là où `{%dp #X1} @ALIGN("y");` placerait les trois transistors à la hauteur de X1 (la colonne *Dessine* du catalogue indique où une structure lie son point).
- Les éléments autour du circuit, tels que les alimentations et les sources de signal, se tiennent en général d’un côté de celui-ci, légèrement à l’écart du reste ; un DISTANCE l’exprime : `#VDD {#VIN #XU1 #CL} @DISTANCE("left-to-right", min=4);` maintient l’alimentation au moins 4 unités à gauche de chacun des autres.
- Donnez au dessin un arbre de groupes clair : chaque bloc fonctionnel forme un groupe, les groupes s’imbriquent dans de plus grands, avec environ cinq ou six éléments directement dans chaque nœud. De nombreuses structures sont déjà des groupes (colonne *Groupe*) ; une structure susceptible de se trouver en partie à l’intérieur d’une autre, telle qu’un miroir de courant, n’en est pas un. Encadrez vous-même des structures entières, ou un étage avec ses ports : `%amp %p_inp %p_inn %p_out @GROUP;`. Les groupes ne se croisent pas : entre deux groupes qui partagent des membres sans imbrication, le solveur n’en conserve qu’un seul d’un bloc.
- Ne fixez les orientations que là où le lecteur en attend une que rien d’autre n’impose, en général pour la source de signal, les sources d’alimentation et la charge : `{#V1 #V2 #R5} @ORIENT("R0");`. Laissez le reste au solveur ; les structures et le courant en déterminent la majeure partie.

## Broches et directions

```rnl
#M1::d @ORIENT("up"); /* a pin takes a direction: left down right up */
#M1::g @ORIENT("left"); /* drain up and gate left: M1 is R0, nothing else */
#M3::s @ORIENT("up"); /* a PMOS upright */
#M1::g #M2::g @ORIENT-RELATION("R180"); /* two pins face opposite ways */
{#M3::d #M1::d} @ALIGN("x"); /* stack: the drains in one column... */
&x @WIRE {#M3:d #M1:d}; /* ...and wired, or the fill labels them */
%t = $IO_PORT @ATTACH &inp; %t::pin @ORIENT("right"); {%t::pin #M1::g} @ALIGN("y"); &inp @WIRE {%t::pin #M1::g};
$IO_PORT_STUB @ATTACH #M1:g; /* the same tag, put on the pin by the package */
```

- Une seule broche fixée laisse deux orientations possibles sur huit : avec la grille seule, droit ou tête en bas ; avec le drain seul, grille à gauche ou à droite. Fixez deux broches.
- Broches standards en orientation R0 : R, C, L et sources p en haut, n en bas ; `$DIODE` a en haut, k en bas ; `$NMOS` d en haut, g à gauche, s en bas, b à droite (4TERM) ; `$PMOS` s en haut, g à gauche, d en bas ; `$NPN` c en haut, b à gauche, e en bas ; `$PNP` e en haut, b à gauche, c en bas.

## Courant

```rnl
#M1:d @CURRENT("in"); #M1:s @CURRENT("out"); /* the static current at a terminal: in, out, unspecified */
{#M1:d #M2:d #M5:d} @CURRENT("in"); /* a set does each member */
#:out @CURRENT("in"); /* at a port: current enters the subcircuit */
#M1::d @CURRENT("in"); /* a pin stands for its terminal */
```

- Cette instruction ne dessine rien ; elle indique ce que fait le circuit afin que le solveur dispose les chemins de courant en conséquence. Écrivez-la sur les chemins principaux.
- Ne l’écrivez que là où un courant statique circule et où son sens est certain : les condensateurs, les grilles et les sources dépourvues de valeur CC n’en portent aucun, omettez-les donc, ainsi que toute borne en cas de doute. Laissez également les sources d’alimentation sans marque : à l’intérieur, le courant monte de n vers p, et un solveur qui privilégie un courant descendant les retournerait.
- Un net dont toutes les bornes et tous les ports orientent le courant dans un même sens produit une violation ; les nets globaux tels que 0 ne sont pas vérifiés.

## Priorité

```rnl
#M5 @PLACE(x=3, priority="low"); /* disabled, low, normal (default), high */
%dp1 = #M1(left) #M2(right) #M5(tail) @AS(priority="high") $DIFFERENTIAL_PAIR_N; /* covers every fact of the body */
%dp1 @PRIORITY("disabled"); /* turns the default drawing of the pair off */
```

- En cas de conflit, la priorité la plus basse cède en premier et, à égalité, le fait le plus ancien ; tout fait retenu est strict. Le paquet standard dessine à la priorité `"low"`. Le champ `priority=` d’un en-tête d’enregistrement sert de valeur par défaut, mais AS et ATTACH n’en reprennent que `"disabled"`.
- Une structure n’a pas besoin de priorité : n’en écrivez une que pour trancher un conflit révélé par une vérification ou une résolution.

## Fils et routage

```rnl
%j = (_, _);
&b @WIRE {#R1:n %j #R2:p}; /* wires join only at shared point objects and terminals */
&b @WIRE(mode="manhattan") {%j #R3:p}; /* auto straight horizontal-vertical vertical-horizontal manhattan manhattan45 */
$JUNCTION @ATTACH %j; /* the dot is written */
&a @ROUTE-NET; /* the solver draws the whole net */
{#R3 #R4} @ROUTE-OBJECT; /* the connections among these objects' pins */
#R3:p @UNROUTE-OBJECT; &x @UNROUTE-NET; /* take pins out of trees; on one pin the higher priority, then the later, wins */
&vss @ROUTE-NET(priority="high"); $GROUND @ATTACH &vss; /* ground and supply nets are unrouted at high: wire them so, and attach a ground */
&D<1:0> @WIRE {%a %b}; /* several nets: a bus */
%all = # @SELECT-NET; %all @ROUTE-NET; /* every net of this level as wires; the fill keeps the ground, supply and bias nets unrouted at high, so they keep their stubs */
```

- Des coordonnées égales ne connectent pas des broches. Écrivez un WIRE entre elles, même de longueur nulle, pour les marquer câblées et éviter une complétion superflue.
- Les structures dessinent leurs fils internes à priorité low. Deux structures distinctes sont reliées par une étiquette de net, sauf si vous écrivez un ROUTE.
- Ne câblez par un fil que ce qui gagne en lisibilité, et non tout le circuit : le solveur ne dessine que ce que demandent WIRE, ROUTE-NET et ROUTE-OBJECT, et la complétion donne à toute autre broche non câblée un raccord de masse, d’alimentation ou d’étiquette de net. Routez le chemin principal du signal ainsi que les liaisons internes aux structures et entre structures voisines ; laissez aux étiquettes les connexions longues ou encombrées, telles que les lignes de polarisation et les nets partagés à travers le dessin.
- Dans un petit circuit, tous les nets à l’exception des rails se lisent en général bien sous forme de fils : routez-les comme à la dernière ligne ci-dessus, ou avec `# @SELECT-NET => @ROUTE-NET;`, qui dessine la même chose. Dans les deux cas, la complétion écarte du routage les nets de masse, d’alimentation et de polarisation à priorité high, et chaque rail reçoit une indication (RNL9403) signalant que son arbre a perdu toutes ses broches ; cette indication est attendue.
- Un net routé vers un port de son niveau n’atteint le port que si l’étiquette de port est écrite : `$IO_PORT @ATTACH &x;`, ou `$IO_PORT_STUB` sur la broche qu’il attaque. La complétion ajoute ses étiquettes de port après avoir décidé quelles broches sont câblées ; un net qui n’a qu’une broche à lui, comme une entrée qui attaque une seule grille, forme donc un arbre d’un seul point : cette broche reçoit une étiquette de net, et aucun fil n’atteint le port.

## Textes

```rnl
#M1 @TEXT("${w}/${l}", key="wl", dy=-3); /* a label that moves with M1; #M1::wl */
#M1 @TEXT("M_{in}", key="name"); /* the same key replaces the name the symbol shows */
(10, 10) @TEXT("bias", size=0.8); /* on a point: an annotation */
#M1 @TEXT("\overline{EN} \mu A \frac{W}{L}", halign="middle", valign="top", rotate=90);
```

- `${…}` : `name`, `value`, `model`, `type`, les paramètres de la netlist tels que `w` ; `net` ; les rôles ; et les chemins tels que `${pair::left::w}`.
- `dx` et `dy` sont des décalages par rapport à l’objet porteur du texte. `#M1::wl @PLACE(5, 0);` place l’étiquette en (5, 0) du dessin ; sur chaque axe, un PLACE et un décalage se remplacent mutuellement, le dernier écrit l’emportant. Dans un corps de symbole, tous deux s’expriment dans les coordonnées propres du symbole.

## Symboles et styles

```rnl
$TRI = @DEFINE-SYMBOL {
  %in = (-4, 0) @PIN(terminal="in", direction="left"); /* a named pin: where, which terminal, which way */
  %out = (4, 0) @PIN(terminal="out", direction="right");
  @LINE {(-2, 2) (2, 0) (-2, -2) (-2, 2)};
  (0, 0) @RECTANGLE(width=4, height=2); (0, 0) @CIRCLE(radius=1);
  ? @TEXT("${name}", key="name") => @PLACE(-2, 2.5); /* ? is the object drawn */
};
#X1 @SET-SYMBOL $TRI; /* terminal names match by name, numbers by place */
#X2 @SET-SYMBOL(terminals=["inp", "inn", "out"]) $OPAMP; /* names differ: the symbol's names in node order */
# @SELECT-INSTANCE(model="nch", recursive=true) => @SET-SYMBOL $NMOS; /* by rule; a direct SET-SYMBOL wins */
%blk = @DEFINE-SYMBOL { %i = (-3, 0) @PIN(terminal="in", direction="left"); (0, 0) @RECTANGLE(width=6, height=3); };
# @SET-SYMBOL %blk; /* inside a .subckt: its block symbol, for its instances above */
$orange = @DEFINE-STYLE(linecolor="#e07000", textcolor="#e07000") {};
# @SELECT-INSTANCE => @SET-STYLE $grey; #M1 @SET-STYLE $orange; /* M1 is orange: direct beats rule, per property */
# @SELECT-USAGE(recursive=true) $GROUND => @SET-STYLE $grey; /* virtual parts are objects of their own */
```

- Propriétés de style : `linecolor linestyle linewidth fillcolor textcolor size font bold italic textvisible` ; les couleurs s’écrivent `"#RRGGBB"`. Un nom en `%` peut aussi désigner une définition locale. Propriété par propriété, les réglages directs l’emportent sur les règles ; au sein d’un même palier, le dernier réglage l’emporte.

## Vérifications

Les structures standards vérifient ce qu’elles affirment : des moitiés de paire d’un même modèle et de même taille, une mise en série réellement en série, un rôle pmos occupé par un PMOS, une étiquette de port sur un port. Une vérification non satisfaite produit une **violation** : il s’agit d’un rapport ; aucune instruction n’échoue et le dessin reste inchangé. Corrigez la structure ou les rôles, ou choisissez une structure qui en affirme moins.

## Le paquet standard

```rnl
@USE-PACKAGE(name="rnl-std-all", version="0.4"); /* everything: styles, symbols, rules for SPICE and Spectre, fill, structures, the blocks */
@USE-PACKAGE(name="rnl-std-no-labels", version="0.4"); /* optional, besides rnl-std-all: hides the labels symbols draw (name, value, model, net names); a label you write shows only with textvisible=true */
@USE-PACKAGE(name="rnl-std-45deg", version="0.4"); /* optional: symbols at 45 degrees */
@USE-PACKAGE(name="rnl-std-fill") {}; /* an empty package drops that part */
&vdd @ADD-TO $SUPPLY_NET_SET; &vss @ADD-TO $GROUND_NET_SET; /* supply and ground stubs on their pins; 0 is a ground already */
$PORT_LIST @ATTACH; /* a port tag on every port net, in one column */
$FIGURE @ATTACH %p => @SET-SYMBOL $MY_FIG; /* no netlist instance: use a symbol with unbound pins and no ${name}/${value}; not $RESISTOR */
```

- Symboles par défaut : R `$RESISTOR`, C `$CAPACITOR`, L `$INDUCTOR`, V `$VOLTAGE_SOURCE`, I `$CURRENT_SOURCE`, D `$DIODE`, E `$VCVS_4TERM`, G `$VCCS_4TERM`, F `$CONTROLLED_CURRENT_SOURCE`, H `$CONTROLLED_VOLTAGE_SOURCE`, B `$BEHAVIORAL_SOURCE`, S `$VOLTAGE_CONTROLLED_SWITCH`, W `$SWITCH`, T et O `$TRANSMISSION_LINE`, U `$RC_LINE` ; transistors selon leur type : nmos `$NMOS_4TERM`, pmos `$PMOS_4TERM`, npn `$NPN`, pnp `$PNP`, njf `$NJFET`, pjf `$PJFET` ; un NMOS dont le substrat est sur un net de masse et un PMOS dont le substrat est sur un net d’alimentation reçoivent à la place les `$NMOS` et `$PMOS` à trois broches, sans aucun SET-SYMBOL de votre part. Sans type connu (ni `.model`, ni DEVICE), un transistor n’a pas de symbole de composant et est dessiné comme une boîte avec une broche par nœud (9.2.8).
- Placez chaque net d’alimentation dans `$SUPPLY_NET_SET` et chaque masse autre que 0 dans `$GROUND_NET_SET`, comme ci-dessus : leurs broches non câblées reçoivent des raccords et ces nets restent non routés. La complétion ignore les broches portant des fils ou des marques de non-connexion ; toute autre broche non câblée, d’un composant ou d’un composant virtuel, reçoit un `$GROUND_STUB` sur un net de `$GROUND_NET_SET` (net 0 par défaut), un `$SUPPLY_STUB` sur un net de `$SUPPLY_NET_SET` (vide par défaut), un `$BIAS_STUB` sur un net de `$BIAS_NET_SET` (vide par défaut), et un `$NET_LABEL_STUB` ailleurs ; la broche de substrat d’un MOS à quatre bornes reçoit un `$NET_LABEL_STUB` sur n’importe quel net. Chaque net de port reçoit un `$IO_PORT` — nets d’alimentation et de masse compris — en une colonne à au moins 6 unités à gauche des instances du niveau, chacun à R0 ; retirez un net avec `&vdd @REMOVE-FROM $PORT_FILL_SET;`, ou écrivez `$IO_PORT @ATTACH &out;` pour placer vous-même une étiquette. Une étiquette de port affiche elle-même le nom de son net sous la forme de son texte `netname` : n’y ajoutez aucun TEXT portant ce nom, et déplacez-le avec `%t::netname @PLACE(x, y);`. Pour un nom de net dans un banc de test, utilisez `$NET_LABEL`. Une broche qui possède déjà un `$IO_PORT_STUB` ou un `$NET_LABEL_STUB` ne reçoit pas non plus d’étiquette de net, bien que leurs pattes soient dessinées par le symbole et non par un WIRE. Les ensembles `*_FILL_SET` sont réservés aux raccords et étiquettes propres à la complétion : lors d’une résolution, ces éléments sont réécrits dans le fichier, après quoi ces ensembles ne contiennent plus les broches ni les nets concernés ; ne construisez donc aucune autre règle sur eux, telle qu’un TEXT sur leurs broches, et sélectionnez plutôt les broches vous-même. Pour modifier les broches qui reçoivent un raccord, ajoutez des broches à ces ensembles ou retirez-en ; tout raccord attaché par l’une de vos règles échappe à la complétion, qui ajoute aussi le sien.

## Enregistrements du solveur

```spice fragment
* @RNL(type="generated", priority="high", author="rnl-solver") { #R1 @PLACE(0, 4); #R2 @PLACE(0, -4);} /* never edit; replaced on each solve */
* @RNL(author="editor") { #R2 @PLACE(y=-6, priority="high");} /* to fix a position: high, in a later record */
```

Les numéros tels que `%#3` n’apparaissent que là ; nommez plutôt les objets.

## Autres mots-clés

Produits par les paquets et les outils ; sachez les lire, vous aurez rarement à les écrire :

```rnl
@VERSION("0.4"); @PACKAGE(name="my-pkg", version="1.0"); /* header records */
@DEVICE(model="sky130_fd_pr__nfet_01v8*", terminals=["d", "g", "s", "b"], type="nmos"); /* names and types of devices */
@ALIAS("@at", "@PLACE"); @USE-ALIASES(name="alias-zh"); /* other spellings */
@SOLVER-CONFIG(spacing_x=2, junction="$JUNCTION"); /* settings of the solver */
#XI<3:0> @BUNDLE; /* an array of instances drawn as one */
#X @LAYER("bottom"); /* drawing layer */
%u_set = %a_set %b_set @UNION; /* a set without repeats */
(0, 0) @ELLIPSE(width=4, height=2); @ARC {(0, 2) (2, 0) (0, -2)}; @BEZIER {(0, 0) (1, 1) (2, 1) (3, 0)}; /* more graphics */
&n1 @SELECT-WIRE(bus=true); #M1 @SELECT-LABEL(key="wl"); # @SELECT-ANNOTATION; #M1 @SELECT-ANCESTOR-USAGE $CURRENT_MIRROR_N;
{#M1 #M2} @ASSERT-INSTANCE(same=["model", "w", "l"]); {#M1:s #M2:s} @ASSERT-TERMINAL(same=["net"]);
{&inp &inn} @ASSERT-NET(port=true); {#R1 #R2 #R3} @ASSERT-TOPOLOGY("series"); /* the checks inside structures */
```

## Erreurs fréquentes

La colonne de gauche est invalide ou trompeuse ; ne reprenez que la correction de droite, avec les noms de votre fichier.

|Erreur|Correction|
|---|---|
|`#M1 @PLACE (10, 20);`|`#M1 @PLACE(10, 20);`|
|`#M1 (left) #M2 (right) @AS $CROSS_COUPLED_PAIR;`|`#M1(left) #M2(right) @AS $CROSS_COUPLED_PAIR;`|
|`#M1 #M2 @SET-SYMBOL $NMOS;`|`{#M1 #M2} @SET-SYMBOL $NMOS;`|
|`#M5 #M3 @AS $MATCHED;`|`{#M5 #M3} @AS $MATCHED;`|
|`{#R1 #R2} @ORDER("x");`|`{#R1 #R2} @ORDER("left-to-right");`|
|`#R1 @PLACE(x=2u);`|`#R1 @PLACE(x=2);`|
|en version 0.3 : noms `$DIFFERENTIAL_PAIR`, rôles `in_p/in_n`|`#M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N;`|
|`#R1 @SET-SYMBOL $GROUND;`|`$GROUND @ATTACH &n;` pour une masse ; gardez le symbole de la résistance|
|`%t #M1 @ALIGN("y");` pour aligner leurs broches|`{%t::pin #M1::g} @ALIGN("y");`|
|`$IO_PORT @ATTACH &n;` sur un net qui n’est pas un port|`$NET_LABEL @ATTACH &n;`|
|`&vss @ROUTE-NET;` quand vss figure dans l’ensemble de masse|`&vss @ROUTE-NET(priority="high");`|
|`%#3 @PLACE(1, 2);` dans du texte rédigé à la main|Nommez cet objet, puis utilisez son nom|
|`#M1.d; M3; &vdd!;`|`#M1:d; #M3; &"vdd!";`|
|`{@VERSION("0.4");}`|`{ @VERSION("0.4");}`|

Un fil vers le substrat de `$NMOS` échoue : utilisez `$NMOS_4TERM`. Les types de transistors inconnus nécessitent une carte de modèle, un DEVICE ou un symbole explicite. Les règles du niveau supérieur exigent `recursive=true` pour atteindre les sous-circuits.

## Un exemple complet

```spice
* Five-transistor OTA
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4");}
.model nch nmos level=1
.model pch pmos level=1
.subckt ota5t inp inn out vdd vss vbias
M1 x    inp   tail vss nch W=2u L=0.5u
M2 out  inn   tail vss nch W=2u L=0.5u
M3 x    x     vdd  vdd pch W=4u L=0.5u
M4 out  x     vdd  vdd pch W=4u L=0.5u
M5 tail vbias vss  vss nch W=4u L=1u
* @RNL {
*   &vss @ADD-TO $GROUND_NET_SET; /* grounds and supplies instead of labels */
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   %dp1 = #M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N; /* mirrored about the tail, tail below */
*   %cm1 = #M3(ref) #M4(out) @AS $CURRENT_MIRROR_P;
*   {%dp1 %cm1} @ROUTE-OBJECT; /* the nets between them as wires */
*   {#M1::d #M2::d #M5::d} @ORIENT("up"); /* NMOS upright */
*   #M1::g @ORIENT("left"); /* gates outwards */
*   {#M3::s #M4::s} @ORIENT("up"); /* PMOS upright */
*   {#M3::d #M1::d} @ALIGN("x"); /* each load over its input device */
*   {#M4::d #M2::d} @ALIGN("x");
*   #M3 #M1 @DISTANCE("top-to-bottom", min=4);
*   {#M1:d #M2:d #M5:d #M3:s #M4:s} @CURRENT("in"); /* current down each branch */
*   {#M1:s #M2:s #M5:s #M3:d #M4:d} @CURRENT("out");
*   $PORT_LIST @ATTACH; /* the ports in one column */
*   {#M1 #M2} @TEXT("${w}/${l}", key="wl", dy=-3);
* }
.ends ota5t
.end
```

Avant de remettre un fichier : en-tête en premier avec VERSION et le paquet ; chaque nom existe dans son niveau ; uniquement des noms 0.4 (le catalogue) ; une espace avant chaque `@` et `;` après chaque instruction ; directions renseignées ; les rôles d’ensemble reçoivent des ensembles ; masses et alimentations dans leurs ensembles ; orientations fixées seulement là où elles sont nécessaires, et courant indiqué seulement là où son sens est certain ; broches alignées câblées ; aucune modification dans les enregistrements générés ; aucune violation que vous n’ayez voulue.

<!-- lists:begin -->
## Catalogue

Une référence, et non du code RNL à copier : gardez le `$` de chaque nom. Appliquez les définitions avec AS ou ATTACH, attribuez les symboles avec SET-SYMBOL et les styles avec SET-STYLE. Les rôles figurent dans leur ordre de déclaration, chacun avec les sortes qu’il accepte (un rôle sans sorte accepte des objets de toute sorte) ; `{role}` reçoit un ensemble entier et peut être omis ; les paramètres suivent `;`. Ainsi, `$MATCHED({members}: instance/usage)` s’emploie comme `{#M1 #M2} @AS $MATCHED;`.

### Paquets

Chacun est déclaré dans l’en-tête avec `@USE-PACKAGE(name="...", version="0.4");`.

|Paquet|Contenu|
|---|---|
|rnl-std-all|L’unique collection de paquets de la série 0.4, sans contenu propre : elle apporte les styles, les symboles et définitions de base et de semi-conducteurs, la complétion automatique, la sémantique de circuit, les quatre paquets de domaine ainsi que le paquet de règles de chaque dialecte hôte via un enregistrement d’en-tête propre à ce langage (2.8, 16.3).|
|rnl-std-style|Les styles (9.3) et les règles de style par défaut.|
|rnl-std-basic|Les symboles de composants passifs, sources, interrupteurs, lignes de transmission, composants virtuels et marques ; les définitions de composants virtuels, marques, figures et raccords, chacune avec la règle qui dessine ses utilisations ; et les ensembles de nets de masse, d’alimentation et de polarisation.|
|rnl-std-semiconductors|Les symboles de diodes et de transistors MOS, bipolaires, JFET et unijonction (UJT, PUT).|
|rnl-std-spice|Le paquet de règles de SPICE : attribue des symboles aux instances par lettre d’élément, par type de modèle et par net de substrat ou de commande négative (16.3).|
|rnl-std-spectre|Le paquet de règles de Spectre : attribue des symboles aux instances par maître, par type de modèle et par net de substrat ou de commande négative (16.3).|
|rnl-std-fill|Les ensembles et règles de la complétion automatique : attache des raccords de masse, d’alimentation, de polarisation et d’étiquette de net aux broches non câblées, y compris celles des composants virtuels, ainsi qu’une étiquette de port à chaque net de port de `$PORT_FILL_SET` (10.5, 12.4, 16.3).|
|rnl-std-semantics|Les définitions sémantiques des structures de circuit et de leurs dessins par défaut.|
|rnl-std-functional|Modèles comportementaux : amplificateurs opérationnels, comparateurs, amplificateurs, transconducteurs, CAN, CNA et modulateur hacheur, avec des variantes *_SUPPLY dotées des broches vp (en haut) et vn (en bas) sur des points de grille entiers.|
|rnl-std-rf|Composants RF : l’inductance à prise médiane et le T-coil.|
|rnl-std-logic|Portes logiques, tampon, inverseur, cellule de retard et bascules D.|
|rnl-std-signal-flow|Composants virtuels idéaux de flux de signal et leurs symboles : additionneur, soustracteur, multiplieur, intégrateurs à temps continu et à temps discret, quantificateur et retard unitaire.|
|rnl-std-no-labels|Masque les étiquettes par défaut des symboles standards, pour les fichiers qui écrivent les leurs (16.3).|
|rnl-std-no-instance-labels|Masque uniquement les étiquettes par défaut des instances ; les noms de nets que dessinent les masses, alimentations, marques de polarisation, étiquettes de port et étiquettes de net demeurent visibles (16.3).|
|rnl-std-45deg|Symboles à 45 degrés pour ponts, paires croisées, enroulements diagonaux et fils à 45 degrés.|

### Définitions

Mentions de la colonne *Dessine* pour les structures de rnl-std-semantics : *signification seule* ne place ni ne route rien ; *place* pose des faits de position sur ses membres ; *route* relie les nets que partagent ses membres. Leurs faits de position et routages ont la priorité low. Colonne *Groupe*, lue dans le corps de chaque définition : *oui* avec les rôles dont elle réunit les membres en un groupe, *via* `$X` avec les rôles qu’elle transmet à une définition `$X` qui forme le groupe, et rien lorsqu’elle n’en forme aucun ; une structure susceptible de se trouver en partie à l’intérieur d’une autre, telle qu’un miroir de courant, n’en forme aucun (voir l’arbre de groupes sous *Faits de placement*). Une utilisation sans symbole représente ses membres dans ALIGN, ORDER, DISTRIBUTE et DISTANCE ; son point propre, `%u::position` (14.1), ne suit la structure que là où la mention *point propre* le relie à ses membres : seule une telle utilisation se déplace comme un seul élément par son `::position`, ou comme membre d’une autre structure qui place ; pour les autres, contraignez leurs membres ou un ensemble nommé de ceux-ci. Les autres paquets définissent les composants et marques qui sont dessinés.

|Signature|Dessine|Groupe|Signification|
|---|---|---|---|
|**rnl-std-basic**||||
|`$GROUND(p: net)`|—||Une masse sur un net, dessinée en un point qui lui est propre : sur le net 0 ce sont les trois barres seules, et sur tout autre net elle affiche le nom de ce net.|
|`$SUPPLY(p: net)`|—||Une alimentation sur un net, dessinée en un point qui lui est propre.|
|`$BIAS(p: net)`|—||Une marque de tension de polarisation sur un net, dessinée en un point qui lui est propre.|
|`$IO_PORT(p: net)`|—||Une étiquette de port sur un net, dessinée en un point qui lui est propre ; son net doit être un port de son niveau.|
|`$NET_LABEL(p: net/pin)`|—||Une étiquette de net : le nom d’un net sur une patte courte, dessinée en un point qui lui est propre ; sur une broche de bus, telle qu’une broche d’un faisceau (11.8), elle retient la broche et forme une seule étiquette pour tous ses nets en affichant leurs noms (13.6).|
|`$NO_CONNECT(t: terminal/pin)`|—||Une marque de non-connexion sur une borne ou une broche laissée déconnectée volontairement ; une borne est retenue telle quelle afin qu’une marque sur un bit de bus consigne quel bit est ouvert (12.6).|
|`$JUNCTION(p: point/terminal)`|—||Un point de jonction sur un point ou une borne.|
|`$CURRENT_ARROW(p: terminal/point)`|—||Une flèche de courant sur une borne ou un point.|
|`$ARROW(at: point)`|—||Une pointe de flèche dessinée sur un point dans un symbole ou dans la scène (9.2.7).|
|`$SPDT_SWITCH(at: point)`|—||Un commutateur unipolaire à deux directions (SPDT) dessiné comme une figure sur un point, sans ligne dans la netlist.|
|`$FIGURE(at: point)`|—||Une figure dessinée en un point sans ligne dans la netlist, dotée de son propre symbole avec SET-SYMBOL.|
|`$PORT_LIST()`|—||Une étiquette de port sur chaque net de port d’un niveau ne possédant pas déjà de `$IO_PORT`, en une colonne, ordonnée de haut en bas et regroupée, à au moins 6 unités à gauche des instances du niveau, chacune orientée en R0 avec sa broche à droite, toutes avec la priorité low, comme les dessine rnl-std-fill ; la complétion n’en ajoute alors aucune dans ce niveau (12.4).|
|`$COMMENT({target}; text="")`|—||Un commentaire non dessiné attaché à des objets quelconques, ou au niveau lui-même lorsque target est omis.|
|`$GROUND_STUB(t: pin; gap=3)`|—||Une masse près d’une broche, à l’endroit, avec son propre fil : à la hauteur de la broche ou au-dessous, à au plus gap unités verticalement (priorité low).|
|`$SUPPLY_STUB(t: pin; gap=3)`|—||Une alimentation près d’une broche, à l’endroit, avec son propre fil : à la hauteur de la broche ou au-dessus, à au plus gap unités verticalement (priorité low).|
|`$BIAS_STUB(t: pin)`|—||Une marque de tension de polarisation directement sur une broche, tournée à l’opposé de la broche comme un raccord de port ; sa barre s’incline en / au-dessus, au-dessous et à gauche de la broche, et en \ à sa droite.|
|`$IO_PORT_STUB(t: pin)`|—||Une étiquette de port directement sur une broche, sa patte servant de court fil au raccord ; la broche et l’étiquette restent non routées, comme avec `$NET_LABEL_STUB`.|
|`$NET_LABEL_STUB(t: pin)`|—||Une étiquette de net directement sur une broche, sa patte d’une unité servant de court fil au raccord ; sur une broche de bus, une seule étiquette pour le bus ; la broche et l’étiquette restent non routées.|
|**rnl-std-semantics**||||
|`$MATCHED({members}: instance/usage)`|signification seule||Composants ou sous-structures appariés par leur type ou leur définition et par leur placement.|
|`$MATCHED_MOS({members}: instance)`|signification seule||MOSFET appariés en type, modèle et longueur de canal.|
|`$MATCHED_BJT({members}: instance)`|signification seule||Transistors bipolaires appariés en type, modèle et géométrie d’émetteur unitaire.|
|`$MATCHED_RESISTOR({members}: instance)`|signification seule||Résistances appariées en type et modèle.|
|`$MATCHED_CAPACITOR({members}: instance)`|signification seule||Condensateurs appariés en type et modèle.|
|`$MIRRORED_LAYOUT(left: instance/usage, right: instance/usage, axis: point/instance/usage)`|place ; point propre au centre||Deux objets symétriques à la même hauteur, images miroir l’un de l’autre par rapport à un axe : un placement uniquement, qui ne dit rien sur le circuit.|
|`$MIRRORED_VERTICAL_LAYOUT(top: instance/usage, bottom: instance/usage, axis: point/instance/usage)`|place ; point propre au centre||Deux objets symétriques dans une même colonne, images miroir l’un de l’autre par rapport à un axe horizontal : un placement uniquement.|
|`$BRANCH({members}: instance/usage)`|place, route ; point propre en x||Une structure série empilée verticalement : membres en série dans une colonne, de haut en bas ; en général une branche CC, telle qu’un chemin d’une alimentation à la masse.|
|`$BRANCH_HORIZONTAL({members}: instance/usage)`|place, route ; point propre en y||Une structure série dessinée en rangée : membres en série à la même hauteur, de gauche à droite.|
|`$CASCADE({stages}: instance/usage)`|signification seule||Une cascade d’étages le long d’un chemin de signal.|
|`$SERIES({members}: instance/usage)`|route||Membres connectés bout à bout en série le long d’un même chemin de courant.|
|`$PARALLEL({members}: instance/usage)`|route||Membres connectés en parallèle entre les deux mêmes nets.|
|`$PARALLEL_HORIZONTAL({members}: instance/usage)`|place, route ; point propre au centre||Membres en parallèle côte à côte à la même hauteur, occupant une seule place dans une colonne.|
|`$PARALLEL_VERTICAL({members}: instance/usage)`|place, route ; point propre au centre||Membres en parallèle l’un au-dessus de l’autre à un même x, occupant une seule place dans une rangée.|
|`$SIGNAL_FLOW({path}: instance/usage, {source}: instance/usage/terminal/net, {load}: instance/usage/terminal/net)`|signification seule||Un chemin de signal unidirectionnel : depuis ses sources à travers les étages de son chemin, dans l’ordre écrit, jusqu’à ses charges.|
|`$POWER_FLOW({path}: instance/usage, {source}: instance/usage/terminal/net, {load}: instance/usage/terminal/net)`|signification seule||Un chemin de puissance : depuis ses sources, telles qu’une alimentation ou une pile, à travers les étages de son chemin qui la convertissent ou la commutent, dans l’ordre écrit, jusqu’aux charges qui l’utilisent.|
|`$BIAS_CIRCUIT({members}: instance/usage)`|signification seule||Un circuit de polarisation : les composants ou structures qui génèrent la polarisation d’autres parties, comme une référence de courant.|
|`$FEEDBACK_LOOP({forward}: instance/usage, {feedback}: instance/usage)`|signification seule||Une boucle de rétroaction : une chaîne directe et un réseau de rétroaction.|
|`$DIFFERENTIAL_PAIR_N(left: instance, right: instance, tail: instance/usage)`|place, route ; point propre au centre|oui : left, right, tail|Une paire différentielle de transistors de type n avec son pied au-dessous.|
|`$DIFFERENTIAL_PAIR_P(left: instance, right: instance, tail: instance/usage)`|place, route ; point propre au centre|oui : left, right, tail|Une paire différentielle de transistors de type p avec son pied au-dessus.|
|`$COMPLEMENTARY_DIFFERENTIAL_PAIR(pairup: usage, pairdown: usage)`|route|oui : pairup, pairdown|Un étage d’entrée rail-à-rail : une paire différentielle supérieure et une paire inférieure sur les mêmes entrées.|
|`$PSEUDO_DIFFERENTIAL_PAIR(left: instance, right: instance)`|place ; point propre au centre||Une paire pseudo-différentielle : deux transistors d’entrée appariés sans pied, dessinés en miroir.|
|`$DEGENERATED_DIFFERENTIAL_PAIR_TYPE_H(left: instance, right: instance, tailleft: instance/usage, tailright: instance/usage, bridge: instance/usage)`|place, route ; point propre au centre|oui : left, right, tailleft, tailright, bridge|Une paire différentielle dégénérée par un pont entre ses deux pieds, en forme de H.|
|`$DEGENERATED_DIFFERENTIAL_PAIR_TYPE_T_N(left: instance, right: instance, degleft: instance/usage, degright: instance/usage, tail: instance/usage)`|place, route ; point propre au centre|oui : left, right, degleft, degright, tail|Une paire différentielle de transistors de type n dégénérée par un élément sous chaque entrée, en forme de T, avec son pied au-dessous.|
|`$DEGENERATED_DIFFERENTIAL_PAIR_TYPE_T_P(left: instance, right: instance, degleft: instance/usage, degright: instance/usage, tail: instance/usage)`|place, route ; point propre au centre|oui : left, right, degleft, degright, tail|Une paire différentielle de transistors de type p dégénérée par un élément au-dessus de chaque entrée, en forme de T renversé, avec son pied au-dessus.|
|`$CROSS_COUPLED_PAIR(left: instance, right: instance)`|place, route ; point propre au centre|oui : left, right|Une paire croisée : deux transistors appariés en couplage croisé avec réaction positive, dessinés en miroir.|
|`$CURRENT_MIRROR_N({ref}: instance, {out}: instance)`|place ; point propre en y||Un miroir de courant côté bas de transistors de type n, avec sources ou émetteurs pointant vers le bas.|
|`$CURRENT_MIRROR_P({ref}: instance, {out}: instance)`|place ; point propre en y||Un miroir de courant côté haut de transistors de type p, avec sources ou émetteurs pointant vers le haut.|
|`$CASCODE_CURRENT_MIRROR_N({ref}: instance, {out}: instance, {refcascode}: instance, {outcascode}: instance)`|place ; point propre en y||Un miroir de courant cascode côté bas : une rangée de miroir en bas avec une rangée de cascode au-dessus.|
|`$CASCODE_CURRENT_MIRROR_P({ref}: instance, {out}: instance, {refcascode}: instance, {outcascode}: instance)`|place ; point propre en y||Un miroir de courant cascode côté haut : une rangée de miroir en haut avec une rangée de cascode au-dessous.|
|`$TRANS_LINEAR_LOOP({a}: instance/usage, {b}: instance/usage)`|place, route ; point propre en x|oui : a, b|Une boucle translinéaire de deux groupes de transistors, dessinée en une colonne.|
|`$TRANS_LINEAR_LOOP_HORIZONTAL({a}: instance/usage, {b}: instance/usage)`|place, route ; point propre en y|oui : a, b|Une boucle translinéaire de deux groupes de transistors, dessinée en une rangée.|
|`$DIODE_CONNECTED(device: instance)`|route||Un transistor monté en diode avec sa grille reliée vers son drain, ou sa base vers son collecteur.|
|`$COMMON_SOURCE(device: instance)`|signification seule||Un étage en source commune d’un seul FET.|
|`$COMMON_GATE(device: instance)`|signification seule||Un étage en grille commune d’un seul FET.|
|`$COMMON_DRAIN(device: instance)`|signification seule||Un suiveur de source (un étage en drain commun) d’un seul FET.|
|`$COMMON_EMITTER(device: instance)`|signification seule||Un étage en émetteur commun d’un seul transistor bipolaire.|
|`$COMMON_BASE(device: instance)`|signification seule||Un étage en base commune d’un seul transistor bipolaire.|
|`$COMMON_COLLECTOR(device: instance)`|signification seule||Un suiveur d’émetteur (un étage en collecteur commun) d’un seul transistor bipolaire.|
|`$FOLLOWER(device: instance)`|signification seule||Un étage suiveur de tension d’un seul transistor.|
|`$CASCODE(source: instance/usage, cascode: instance)`|route||Un cascode : un composant source ou une sous-structure en série avec un transistor en grille commune.|
|`$FOLDED_CASCODE(source: instance/usage, cascode: instance, bias: instance/usage)`|route||Un cascode replié : un composant d’entrée replié dans un transistor en grille commune avec sa polarisation.|
|`$COMMON_SOURCE_AMPLIFIER(input: instance, load: instance/usage)`|route|via `$TRANSCONDUCTOR_Z_AMPLIFIER` : input, load|Un amplificateur en source commune : un transistor d’entrée FET et sa charge.|
|`$COMMON_EMITTER_AMPLIFIER(input: instance, load: instance/usage)`|route|via `$TRANSCONDUCTOR_Z_AMPLIFIER` : input, load|Un amplificateur en émetteur commun : un transistor d’entrée bipolaire et sa charge.|
|`$DEGENERATED_COMMON_SOURCE_AMPLIFIER(input: instance, load: instance/usage, degenerator: instance/usage)`|route|via `$DEGENERATED_TRANSCONDUCTOR_Z_AMPLIFIER` : input, load, degenerator|Un amplificateur en source commune dégénéré : un transistor d’entrée FET, sa charge et sa dégénérescence.|
|`$DEGENERATED_COMMON_EMITTER_AMPLIFIER(input: instance, load: instance/usage, degenerator: instance/usage)`|route|via `$DEGENERATED_TRANSCONDUCTOR_Z_AMPLIFIER` : input, load, degenerator|Un amplificateur en émetteur commun dégénéré : un transistor d’entrée bipolaire, sa charge et sa dégénérescence.|
|`$MILLER_AMPLIFIER(stage: instance/usage, compensation: instance/usage)`|route|oui : stage, compensation|Un amplificateur à compensation Miller : un étage de gain et son réseau de compensation.|
|`$PUSH_PULL_STAGE(pullup: instance/usage, pulldown: instance/usage)`|route|oui : pullup, pulldown|Un étage de sortie push-pull : les seuls transistors de sortie, un tirage haut en série avec un tirage bas ; leur polarisation et l’étage qui les attaque n’en font pas partie.|
|`$TRANSCONDUCTOR_Z_AMPLIFIER(transconductor: instance/usage, load: instance/usage)`|route|oui : transconductor, load|Un étage amplificateur : un transconducteur et sa charge.|
|`$DEGENERATED_TRANSCONDUCTOR_Z_AMPLIFIER(transconductor: instance/usage, load: instance/usage, degenerator: instance/usage)`|route|oui : transconductor, load, degenerator|Un étage amplificateur dégénéré : un transconducteur, sa charge et sa dégénérescence.|
|`$PASS_SWITCH(device: instance/usage)`|signification seule||Un interrupteur qui laisse passer ou bloque un signal entre ses deux côtés.|
|`$RESET_SWITCH(device: instance)`|signification seule||Un transistor de précharge, d’égalisation ou de réinitialisation.|
|`$GROUND_SWITCH(device: instance)`|signification seule||Un interrupteur côté bas connecté à un net de masse.|
|`$POWER_SWITCH(device: instance)`|signification seule||Un interrupteur côté haut connecté à un net d’alimentation.|
|`$TRANSMISSION_GATE(pmos: instance/usage, nmos: instance/usage)`|place, route ; point propre au centre|oui : pmos, nmos|Une porte de transmission CMOS, le PMOS au-dessus du NMOS.|
|`$TRANSMISSION_GATE_HORIZONTAL(pmos: instance/usage, nmos: instance/usage)`|place, route ; point propre au centre|oui : pmos, nmos|Une porte de transmission CMOS dessinée côte à côte, le PMOS à gauche.|
|`$MULTI_POLE_SWITCH({throws}: instance/usage)`|route|oui : throws|Un commutateur à plusieurs directions partageant un même pôle.|
|`$BOOTSTRAPPED_SWITCH(device: instance/usage)`|signification seule||Un interrupteur d’échantillonnage à bootstrap.|
|`$SC_SAMPLER({switch}: instance/usage, {capacitor}: instance/usage)`|route||Un échantillonneur à capacités commutées : interrupteurs d’échantillonnage et condensateurs d’échantillonnage.|
|`$CHARGE_SHARING(a: instance/usage, switch: instance/usage, b: instance/usage)`|route||Un réseau à partage de charge : deux condensateurs ou bancs de condensateurs échangeant leur charge via un interrupteur.|
|`$CHOPPER({straight}: instance/usage, {crossed}: instance/usage)`|route||Un hacheur : un pont de quatre interrupteurs qui inverse la polarité d’un signal.|
|`$FLYING_CAPACITOR(capacitor: instance/usage)`|signification seule||Un condensateur flottant commuté entre phases d’horloge.|
|`$CMOS_INVERTER(pmos: instance, nmos: instance)`|place, route ; point propre au centre|via `$CMOS_GATE` : pmos, nmos|Un inverseur CMOS, le PMOS au-dessus du NMOS.|
|`$CMOS_INVERTER_HORIZONTAL(pmos: instance, nmos: instance)`|place, route ; point propre au centre|via `$CMOS_GATE` : pmos, nmos|Un inverseur CMOS dessiné côte à côte, le PMOS à gauche.|
|`$CMOS_GATE({pullup}: instance/usage, {pulldown}: instance/usage)`|route|oui : pullup, pulldown|Une porte logique CMOS statique : un réseau de tirage haut et un réseau de tirage bas se rejoignant en série à la sortie.|
|`$LATCH({members}: instance/usage)`|route|oui : members|Un verrou : un cœur bistable accompagné des composants qui l’entourent.|
|`$INVERTER_LATCH(a: instance/usage, b: instance/usage)`|route|via `$LATCH` : a, b|Un verrou constitué de deux inverseurs en couplage croisé.|
|`$SHIFT_REGISTER({stages}: instance/usage)`|signification seule||Un registre à décalage : une chaîne de bascules ou de verrous, souvent dessinée avec chevauchement et dont les horloges sont rarement câblées.|
|`$DIVIDER(upper: instance/usage, lower: instance/usage)`|route||Un diviseur formé de deux branches en série, supérieure et inférieure.|
|`$RESISTOR_LADDER({resistors}: instance)`|place, route ; point propre en x|oui : resistors|Une échelle de résistances en série dans une colonne, de haut en bas.|
|`$RESISTOR_LADDER_HORIZONTAL({resistors}: instance)`|place, route ; point propre en y|oui : resistors|Une échelle de résistances en série dessinée en rangée.|
|`$PI_NETWORK(shunt1: instance/usage, series: instance/usage, shunt2: instance/usage)`|route||Un réseau en pi : une branche parallèle, une branche série et une seconde branche parallèle.|
|`$T_NETWORK(series1: instance/usage, shunt: instance/usage, series2: instance/usage)`|route||Un réseau en T : une branche série, une branche parallèle et une seconde branche série.|
|`$LC_TANK_PARALLEL(inductor: instance/usage, capacitor: instance/usage)`|route|oui : inductor, capacitor|Un circuit résonant LC parallèle.|
|`$RLC_TANK_PARALLEL(resistor: instance/usage, inductor: instance/usage, capacitor: instance/usage)`|route|oui : resistor, inductor, capacitor|Un circuit résonant RLC parallèle.|
|`$LC_TANK_SERIES(inductor: instance/usage, capacitor: instance/usage)`|route|oui : inductor, capacitor|Un circuit résonant LC série.|
|`$RLC_TANK_SERIES(resistor: instance/usage, inductor: instance/usage, capacitor: instance/usage)`|route|oui : resistor, inductor, capacitor|Un circuit résonant RLC série.|
|`$DC_FEED(device: instance/usage)`|signification seule||Une bobine d’arrêt RF ou résistance de polarisation alimentant un nœud en CC ou en polarisation.|
|`$DECAP(device: instance)`|place ; point propre en x||Un condensateur de découplage sur un nœud d’alimentation ou de polarisation.|
|`$AC_COUPLE(device: instance/usage)`|signification seule||Un condensateur de liaison série qui laisse passer le signal CA et bloque la composante CC.|
|`$OSCILLATOR({members}: instance/usage)`|route|oui : members|Un oscillateur formé de ses éléments actifs et de ses éléments résonants ou à retard.|
|**rnl-std-signal-flow**||||
|`$ADDER(in1: net, in2: net, out: net)`|—||Un additionneur idéal de flux de signal (out = in1 + in2), dessiné avec `$ADDER_SYMBOL`.|
|`$SUBTRACTOR(in1: net, in2: net, out: net)`|—||Un soustracteur idéal de flux de signal (out = in1 - in2), dessiné avec `$SUBTRACTOR_SYMBOL`.|
|`$MULTIPLIER(in1: net, in2: net, out: net)`|—||Un multiplieur idéal de flux de signal (out = in1 * in2), dessiné avec `$MULTIPLIER_SYMBOL`.|
|`$INTEGRATOR(in: net, out: net)`|—||Un intégrateur idéal à temps continu, dessiné avec `$INTEGRATOR_SYMBOL`.|
|`$DISCRETE_TIME_INTEGRATOR(in: net, out: net)`|—||Un intégrateur idéal à temps discret, dessiné avec `$DISCRETE_TIME_INTEGRATOR_SYMBOL`.|
|`$QUANTIZER(in: net, out: net)`|—||Un quantificateur idéal, dessiné avec `$QUANTIZER_SYMBOL`.|
|`$UNIT_DELAY(in: net, out: net)`|—||Un retard unitaire idéal, dessiné avec `$UNIT_DELAY_SYMBOL`.|
|**rnl-std-45deg**||||
|`$ARROW_45(at: point)`|—||Une pointe de flèche à 45 degrés dessinée sur un point dans un symbole ou sur un fil à 45 degrés dans la scène.|
|`$CURRENT_ARROW_45(p: terminal/point)`|—||Une flèche de courant à 45 degrés sur une borne ou un point le long d’une branche à 45 degrés.|

### Symboles

Les broches sont énumérées selon la borne ou le rôle qu’elles lient ; `pin` (`p`) est la broche nommée `pin`, liée au rôle `p` ; `pin` (nothing) ne lie rien.

|Symbole|Broches|Signification|
|---|---|---|
|**rnl-std-basic**|||
|`$RESISTOR`|`p` `n`|Une résistance.|
|`$VARIABLE_RESISTOR`|`p` `n`|Une résistance variable : une résistance traversée par une flèche.|
|`$CAPACITOR`|`p` `n`|Un condensateur.|
|`$VARIABLE_CAPACITOR`|`p` `n`|Un condensateur variable : un condensateur traversé par une flèche.|
|`$INDUCTOR`|`p` `n`|Une inductance.|
|`$VARIABLE_INDUCTOR`|`p` `n`|Une inductance variable : une inductance traversée par une flèche.|
|`$TRANSFORMER`|`p1` `n1` `p2` `n2`|Un transformateur : deux enroulements couplés placés verticalement côte à côte, leurs points de polarité en haut.|
|`$POTENTIOMETER`|`n1` `nw` `n2`|Un potentiomètre à trois bornes, le curseur au milieu.|
|`$PHOTORESISTOR`|`p` `n`|Une photorésistance (résistance dépendante de la lumière).|
|`$POLARIZED_CAPACITOR`|`p` `n`|Un condensateur polarisé (électrolytique), dont l’armature positive est sur p.|
|`$CRYSTAL`|`n1` `n2`|Un résonateur à quartz.|
|`$SPEAKER`|`p` `n`|Un haut-parleur.|
|`$VOLTAGE_SOURCE`|`p` `n`|Une source de tension indépendante.|
|`$CURRENT_SOURCE`|`p` `n`|Une source de courant indépendante, sa flèche dirigée de p vers n.|
|`$BATTERY`|`p` `n`|Une pile ou batterie.|
|`$CONTROLLED_VOLTAGE_SOURCE`|`p` `n`|Une source de tension commandée dessinée sous forme de losange avec ses seules broches de sortie.|
|`$CONTROLLED_CURRENT_SOURCE`|`p` `n`|Une source de courant commandée dessinée sous forme de losange avec ses seules broches de sortie.|
|`$VCVS_4TERM`|`p` `n` `cp` `cn`|Une source de tension commandée en tension (E) avec ses broches de commande à gauche.|
|`$VCCS_4TERM`|`p` `n` `cp` `cn`|Une source de courant commandée en tension (G) avec ses broches de commande à gauche.|
|`$BEHAVIORAL_SOURCE`|`p` `n`|Une source comportementale (B) : un cercle marqué B.|
|`$RF_PORT`|`p` `n`|Un port RF : une source sinusoïdale en série avec sa résistance interne, pour le port de Spectre.|
|`$SINE_VOLTAGE_SOURCE`|`p` `n`|Une source de tension sinusoïdale, + sur p.|
|`$PULSE_VOLTAGE_SOURCE`|`p` `n`|Une source de tension impulsionnelle, + sur p.|
|`$MICROPHONE`|`p` `n`|Un microphone, + sur p.|
|`$AMMETER`|`p` `n`|Un ampèremètre, le courant mesuré de p vers n.|
|`$SWITCH`|`p` `n`|Un interrupteur dessiné ouvert, pour un interrupteur commandé en courant (W) ou le composant switch de Spectre.|
|`$CLOSED_SWITCH`|`p` `n`|Un interrupteur dessiné fermé.|
|`$EXTERNALLY_CONTROLLED_SWITCH`|`p` `n` `cp`|Un interrupteur muni d’une seule broche de commande sur le côté.|
|`$VOLTAGE_CONTROLLED_SWITCH`|`p` `n` `cp` `cn`|Un interrupteur commandé en tension (S), ses broches de commande différentielles au-dessous.|
|`$SPDT_SWITCH_SYMBOL`|`com` (nothing) `a` (nothing) `b` (nothing)|Le symbole de `$SPDT_SWITCH` : un commutateur unipolaire à deux directions dont les broches com, a et b ne lient rien et sont câblées par nom (9.2.4).|
|`$TRANSMISSION_LINE`|`p1` `n1` `p2` `n2`|Une ligne de transmission (T ou O), port 1 à gauche et port 2 à droite.|
|`$RC_LINE`|`a` `b` `ref`|Une ligne RC répartie uniforme (U).|
|`$GROUND_SYMBOL`|`pin` (`p`)|Le symbole de `$GROUND` sur le net 0 : une masse à trois barres, sa broche en haut.|
|`$NAMED_GROUND_SYMBOL`|`pin` (`p`)|Le symbole de `$GROUND` sur tout net autre que 0 : une masse à trois barres avec le nom de son net au-dessous.|
|`$SUPPLY_SYMBOL`|`pin` (`p`)|Le symbole de `$SUPPLY` : une barre horizontale avec le nom de son net, sa broche en bas.|
|`$BIAS_SYMBOL`|`pin` (`p`)|Le symbole de `$BIAS` : une barre oblique avec le nom de son net, sa broche en bas.|
|`$IO_PORT_SYMBOL`|`pin` (`p`)|Le symbole de `$IO_PORT` : un petit cercle avec le nom de son net, sa broche à droite.|
|`$NET_LABEL_SYMBOL`|`pin` (`p`)|Le symbole de `$NET_LABEL` : le nom de son net, ou les noms des nets d’une broche de bus, à gauche d’une patte d’une unité, sa broche pointant vers la droite comme `$IO_PORT_SYMBOL`.|
|`$ARROW_SYMBOL`|—|Une pointe de flèche pleine pointant vers la droite, le symbole de `$ARROW` ; `$CURRENT_ARROW` en dessine une de moitié moins grande.|
|`$NO_CONNECT_SYMBOL`|—|Le symbole de `$NO_CONNECT` : une croix.|
|`$JUNCTION_SYMBOL`|—|Le symbole de `$JUNCTION` : un point plein.|
|**rnl-std-semiconductors**|||
|`$DIODE`|`a` `k`|Une diode.|
|`$ZENER_DIODE`|`a` `k`|Une diode Zener.|
|`$LED`|`a` `k`|Une diode électroluminescente (LED).|
|`$SCHOTTKY_DIODE`|`a` `k`|Une diode à barrière Schottky.|
|`$SHOCKLEY_DIODE`|`a` `k`|Une diode Shockley à quatre couches.|
|`$PUT`|`a` `g` `k`|Un transistor unijonction programmable (PUT).|
|`$NMOS`|`d` `g` `s`|Un transistor NMOS à trois broches, son substrat non dessiné.|
|`$NMOS_4TERM`|`d` `g` `s` `b`|Un transistor NMOS avec sa broche de substrat alignée sur drain et source et sa flèche de substrat sur le canal.|
|`$PMOS`|`d` `g` `s`|Un transistor PMOS à trois broches, son substrat non dessiné.|
|`$PMOS_4TERM`|`d` `g` `s` `b`|Un transistor PMOS avec sa broche de substrat alignée sur drain et source.|
|`$NMOS_LOGIC`|`d` `g` `s`|Un transistor NMOS logique à trois broches sans flèche de source.|
|`$PMOS_LOGIC`|`d` `g` `s`|Un transistor PMOS logique à trois broches sans flèche de source, avec une bulle d’inversion sur sa grille.|
|`$DEPLETION_NMOS`|`d` `g` `s`|Un transistor NMOS à déplétion à trois broches, son substrat non dessiné.|
|`$DEPLETION_NMOS_4TERM`|`d` `g` `s` `b`|Un transistor NMOS à déplétion avec sa broche de substrat alignée sur drain et source.|
|`$DEPLETION_PMOS`|`d` `g` `s`|Un transistor PMOS à déplétion à trois broches, son substrat non dessiné.|
|`$DEPLETION_PMOS_4TERM`|`d` `g` `s` `b`|Un transistor PMOS à déplétion avec sa broche de substrat alignée sur drain et source.|
|`$NDMOS`|`d` `g` `s`|Un transistor DMOS de type N à trois broches, son substrat non dessiné.|
|`$NDMOS_4TERM`|`d` `g` `s` `b`|Un transistor DMOS de type N avec sa broche de substrat alignée sur drain et source.|
|`$PDMOS`|`d` `g` `s`|Un transistor DMOS de type P à trois broches, son substrat non dessiné.|
|`$PDMOS_4TERM`|`d` `g` `s` `b`|Un transistor DMOS de type P avec sa broche de substrat alignée sur drain et source.|
|`$NPN`|`c` `b` `e`|Un transistor bipolaire NPN, son substrat non dessiné.|
|`$PNP`|`c` `b` `e`|Un transistor bipolaire PNP, son substrat non dessiné.|
|`$NJFET`|`d` `g` `s`|Un JFET à canal N, sa flèche de grille pointant vers le canal.|
|`$PJFET`|`d` `g` `s`|Un JFET à canal P, sa flèche de grille pointant à l’opposé du canal.|
|`$UJT`|`e` `b1` `b2`|Un transistor unijonction (UJT), b2 en haut et b1 en bas.|
|**rnl-std-functional**|||
|`$OPAMP`|`inp` `inn` `out`|Un amplificateur opérationnel à trois bornes : inp, inn, out.|
|`$OPAMP_SUPPLY`|`inp` `inn` `out` `vp` `vn`|Un amplificateur opérationnel avec broches d’alimentation : inp, inn, out, vp, vn.|
|`$OPAMP_INVERTED_SUPPLY`|`inp` `inn` `out` `vp` `vn`|Un amplificateur opérationnel avec broches d’alimentation, ses entrées échangées (inn en haut, inp en bas) et ses alimentations maintenues (vp en haut, vn en bas), ce que ne permet pas `$OPAMP_SUPPLY` retourné en MX : inp, inn, out, vp, vn.|
|`$DIFFERENTIAL_OPAMP`|`inp` `inn` `outp` `outn`|Un amplificateur opérationnel entièrement différentiel : inp, inn, outp, outn.|
|`$DIFFERENTIAL_OPAMP_SUPPLY`|`inp` `inn` `outp` `outn` `vp` `vn`|Un amplificateur opérationnel entièrement différentiel avec broches d’alimentation : inp, inn, outp, outn, vp, vn.|
|`$DIFFERENTIAL_OPAMP_INVERTED_SUPPLY`|`inp` `inn` `outp` `outn` `vp` `vn`|Un amplificateur opérationnel entièrement différentiel avec broches d’alimentation, ses entrées et ses sorties échangées (inn et outn en haut, inp et outp en bas) et ses alimentations maintenues (vp en haut, vn en bas), ce que ne permet pas `$DIFFERENTIAL_OPAMP_SUPPLY` retourné en MX : inp, inn, outp, outn, vp, vn.|
|`$DIFFERENTIAL_OPAMP_TWISTED`|`inp` `inn` `outp` `outn`|Un amplificateur opérationnel entièrement différentiel avec polarité de sortie inversée (outn en haut, outp en bas) : inp, inn, outp, outn.|
|`$DIFFERENTIAL_OPAMP_TWISTED_SUPPLY`|`inp` `inn` `outp` `outn` `vp` `vn`|Un amplificateur opérationnel entièrement différentiel à sorties croisées avec broches d’alimentation : inp, inn, outp, outn, vp, vn.|
|`$DIFFERENTIAL_OPAMP_TWISTED_INVERTED_SUPPLY`|`inp` `inn` `outp` `outn` `vp` `vn`|Un amplificateur opérationnel entièrement différentiel avec broches d’alimentation, ses entrées échangées (inn en haut, inp en bas), ses sorties comme sur `$DIFFERENTIAL_OPAMP_SUPPLY` (outp en haut, outn en bas) et ses alimentations maintenues (vp en haut, vn en bas), ce que ne permet pas `$DIFFERENTIAL_OPAMP_TWISTED_SUPPLY` retourné en MX : inp, inn, outp, outn, vp, vn.|
|`$COMPARATOR`|`inp` `inn` `out`|Un comparateur à trois bornes : inp, inn, out.|
|`$COMPARATOR_SUPPLY`|`inp` `inn` `out` `vp` `vn`|Un comparateur avec broches d’alimentation : inp, inn, out, vp, vn.|
|`$COMPARATOR_INVERTED_SUPPLY`|`inp` `inn` `out` `vp` `vn`|Un comparateur avec broches d’alimentation, ses entrées échangées (inn en haut, inp en bas) et ses alimentations maintenues (vp en haut, vn en bas), ce que ne permet pas `$COMPARATOR_SUPPLY` retourné en MX ; son échelon se lit comme sur `$COMPARATOR_SUPPLY` : inp, inn, out, vp, vn.|
|`$VOLTAGE_AMPLIFIER`|`in` `out`|Un amplificateur de tension, avec un texte centré facultatif de clé label : in, out.|
|`$VOLTAGE_AMPLIFIER_SUPPLY`|`in` `out` `vp` `vn`|Un amplificateur de tension avec broches d’alimentation : in, out, vp, vn.|
|`$TRANSCONDUCTOR`|`in` `out`|Un transconducteur (une cellule gm), avec un texte centré de clé label, g_m par défaut : in, out.|
|`$TRANSCONDUCTOR_SUPPLY`|`in` `out` `vp` `vn`|Un transconducteur avec broches d’alimentation et un texte centré de clé label, g_m par défaut : in, out, vp, vn.|
|`$DIFFERENTIAL_TRANSCONDUCTOR`|`inp` `inn` `out`|Un transconducteur à entrée différentielle et un texte de clé label (g_m par défaut) : inp, inn, out.|
|`$DIFFERENTIAL_TRANSCONDUCTOR_SUPPLY`|`inp` `inn` `out` `vp` `vn`|Un transconducteur à entrée différentielle avec broches d’alimentation et un texte de clé label (g_m par défaut) : inp, inn, out, vp, vn.|
|`$DIFFERENTIAL_TRANSCONDUCTOR_INVERTED_SUPPLY`|`inp` `inn` `out` `vp` `vn`|Un transconducteur à entrée différentielle avec broches d’alimentation et un texte de clé label (g_m par défaut), ses entrées échangées (inn en haut, inp en bas) et ses alimentations maintenues (vp en haut, vn en bas), ce que ne permet pas `$DIFFERENTIAL_TRANSCONDUCTOR_SUPPLY` retourné en MX : inp, inn, out, vp, vn.|
|`$ADC`|`in` `out`|Un convertisseur analogique-numérique : in, out.|
|`$ADC_SUPPLY`|`in` `out` `vp` `vn`|Un convertisseur analogique-numérique avec broches d’alimentation : in, out, vp, vn.|
|`$DAC`|`in` `out`|Un convertisseur numérique-analogique : in, out.|
|`$DAC_SUPPLY`|`in` `out` `vp` `vn`|Un convertisseur numérique-analogique avec broches d’alimentation : in, out, vp, vn.|
|`$CHOPPER_SYMBOL`|`inp` `inn` `outp` `outn`|Un bloc modulateur hacheur (distinct de la structure en pont d’interrupteurs `$CHOPPER`) : inp, inn, outp, outn.|
|**rnl-std-rf**|||
|`$CENTER_TAP_INDUCTOR`|`n1` `n2` `tap`|Une inductance à prise médiane.|
|`$T_COIL`|`n1` `n2` `tap`|Un T-coil : deux bobines couplées avec une prise médiane et un condensateur de pontage.|
|**rnl-std-logic**|||
|`$AND_2INPUT`|`a` `b` `y`|Une porte ET à deux entrées : a, b, y.|
|`$AND_3INPUT`|`a` `b` `c` `y`|Une porte ET à trois entrées : a, b, c, y.|
|`$NAND_2INPUT`|`a` `b` `y`|Une porte NON-ET à deux entrées : a, b, y.|
|`$NAND_3INPUT`|`a` `b` `c` `y`|Une porte NON-ET à trois entrées : a, b, c, y.|
|`$OR_2INPUT`|`a` `b` `y`|Une porte OU à deux entrées : a, b, y.|
|`$OR_3INPUT`|`a` `b` `c` `y`|Une porte OU à trois entrées : a, b, c, y.|
|`$NOR_2INPUT`|`a` `b` `y`|Une porte NON-OU à deux entrées : a, b, y.|
|`$NOR_3INPUT`|`a` `b` `c` `y`|Une porte NON-OU à trois entrées : a, b, c, y.|
|`$XOR_2INPUT`|`a` `b` `y`|Une porte OU exclusif à deux entrées : a, b, y.|
|`$XOR_3INPUT`|`a` `b` `c` `y`|Une porte OU exclusif à trois entrées : a, b, c, y.|
|`$XNOR_2INPUT`|`a` `b` `y`|Une porte NON-OU exclusif à deux entrées : a, b, y.|
|`$XNOR_3INPUT`|`a` `b` `c` `y`|Une porte NON-OU exclusif à trois entrées : a, b, c, y.|
|`$AND_4INPUT`|`a` `b` `c` `d` `y`|Une porte ET à quatre entrées sur des points de grille entiers : a, b, c, d, y.|
|`$NAND_4INPUT`|`a` `b` `c` `d` `y`|Une porte NON-ET à quatre entrées sur des points de grille entiers : a, b, c, d, y.|
|`$OR_4INPUT`|`a` `b` `c` `d` `y`|Une porte OU à quatre entrées sur des points de grille entiers : a, b, c, d, y.|
|`$NOR_4INPUT`|`a` `b` `c` `d` `y`|Une porte NON-OU à quatre entrées sur des points de grille entiers : a, b, c, d, y.|
|`$XOR_4INPUT`|`a` `b` `c` `d` `y`|Une porte OU exclusif à quatre entrées sur des points de grille entiers : a, b, c, d, y.|
|`$XNOR_4INPUT`|`a` `b` `c` `d` `y`|Une porte NON-OU exclusif à quatre entrées sur des points de grille entiers : a, b, c, d, y.|
|`$BUFFER`|`a` `y`|Un tampon : a, y.|
|`$INVERTER`|`a` `y`|Un inverseur (une porte NON) : a, y.|
|`$DELAY_CELL`|`a` `y`|Une cellule de retard : a, y.|
|`$D_FLIP_FLOP`|`d` `ck` `q` `qb`|Une bascule D avec la sortie Q et son complément : d, ck, q, qb.|
|`$D_FLIP_FLOP_Q_ONLY`|`d` `ck` `q`|Une bascule D avec la sortie Q uniquement : d, ck, q.|
|`$D_FLIP_FLOP_WITH_RESET`|`d` `ck` `rst` `q` `qb`|Une bascule D avec réinitialisation : d, ck, rst, q, qb.|
|`$AND_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|Une porte ET à 2 entrées avec broches d’alimentation : a, b, y, vp, vn.|
|`$AND_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|Une porte ET à 3 entrées avec broches d’alimentation : a, b, c, y, vp, vn.|
|`$AND_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|Une porte ET à 4 entrées avec broches d’alimentation : a, b, c, d, y, vp, vn.|
|`$NAND_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|Une porte NON-ET à 2 entrées avec broches d’alimentation : a, b, y, vp, vn.|
|`$NAND_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|Une porte NON-ET à 3 entrées avec broches d’alimentation : a, b, c, y, vp, vn.|
|`$NAND_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|Une porte NON-ET à 4 entrées avec broches d’alimentation : a, b, c, d, y, vp, vn.|
|`$OR_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|Une porte OU à 2 entrées avec broches d’alimentation : a, b, y, vp, vn.|
|`$OR_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|Une porte OU à 3 entrées avec broches d’alimentation : a, b, c, y, vp, vn.|
|`$OR_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|Une porte OU à 4 entrées avec broches d’alimentation : a, b, c, d, y, vp, vn.|
|`$NOR_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|Une porte NON-OU à 2 entrées avec broches d’alimentation : a, b, y, vp, vn.|
|`$NOR_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|Une porte NON-OU à 3 entrées avec broches d’alimentation : a, b, c, y, vp, vn.|
|`$NOR_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|Une porte NON-OU à 4 entrées avec broches d’alimentation : a, b, c, d, y, vp, vn.|
|`$XOR_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|Une porte OU exclusif à 2 entrées avec broches d’alimentation : a, b, y, vp, vn.|
|`$XOR_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|Une porte OU exclusif à 3 entrées avec broches d’alimentation : a, b, c, y, vp, vn.|
|`$XOR_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|Une porte OU exclusif à 4 entrées avec broches d’alimentation : a, b, c, d, y, vp, vn.|
|`$XNOR_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|Une porte NON-OU exclusif à 2 entrées avec broches d’alimentation : a, b, y, vp, vn.|
|`$XNOR_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|Une porte NON-OU exclusif à 3 entrées avec broches d’alimentation : a, b, c, y, vp, vn.|
|`$XNOR_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|Une porte NON-OU exclusif à 4 entrées avec broches d’alimentation : a, b, c, d, y, vp, vn.|
|`$BUFFER_SUPPLY`|`a` `y` `vp` `vn`|Un tampon avec broches d’alimentation : a, y, vp, vn.|
|`$INVERTER_SUPPLY`|`a` `y` `vp` `vn`|Un inverseur avec broches d’alimentation : a, y, vp, vn.|
|`$DELAY_CELL_SUPPLY`|`a` `y` `vp` `vn`|Une cellule de retard avec broches d’alimentation : a, y, vp, vn.|
|`$D_FLIP_FLOP_SUPPLY`|`d` `ck` `q` `qb` `vp` `vn`|Une bascule D avec broches d’alimentation : d, ck, q, qb, vp, vn.|
|`$D_FLIP_FLOP_Q_ONLY_SUPPLY`|`d` `ck` `q` `vp` `vn`|Une bascule D avec Q uniquement et broches d’alimentation : d, ck, q, vp, vn.|
|`$D_FLIP_FLOP_WITH_RESET_SUPPLY`|`d` `ck` `rst` `q` `qb` `vp` `vn`|Une bascule D avec réinitialisation et broches d’alimentation (vp et vn alignées en x = 0) : d, ck, rst, q, qb, vp, vn.|
|**rnl-std-signal-flow**|||
|`$ADDER_SYMBOL`|`a` (`in1`) `b` (`in2`) `y` (`out`)|Le symbole de `$ADDER` : un cercle sommateur avec pointes de flèche sur ses pattes d’entrée a (in1) et b (in2), et sortie y (out).|
|`$SUBTRACTOR_SYMBOL`|`a` (`in1`) `b` (`in2`) `y` (`out`)|Le symbole de `$SUBTRACTOR` : un cercle sommateur avec + au centre et - à l’extérieur près de son entrée négative b (in2), entrées a (in1) et b (in2), et sortie y (out).|
|`$MULTIPLIER_SYMBOL`|`a` (`in1`) `b` (`in2`) `y` (`out`)|Le symbole de `$MULTIPLIER` : un cercle multiplieur avec pointes de flèche sur ses pattes d’entrée a (in1) et b (in2), et sortie y (out).|
|`$INTEGRATOR_SYMBOL`|`a` (`in`) `y` (`out`)|Le symbole de `$INTEGRATOR` : un bloc marqué 1/s avec entrée a (in) et sortie y (out).|
|`$DISCRETE_TIME_INTEGRATOR_SYMBOL`|`a` (`in`) `y` (`out`)|Le symbole de `$DISCRETE_TIME_INTEGRATOR` : un bloc portant sa fonction de transfert dans le domaine z, entrée a (in) et sortie y (out).|
|`$QUANTIZER_SYMBOL`|`a` (`in`) `y` (`out`)|Le symbole de `$QUANTIZER` : un bloc comportant une caractéristique en escalier, entrée a (in) et sortie y (out).|
|`$UNIT_DELAY_SYMBOL`|`a` (`in`) `y` (`out`)|Le symbole de `$UNIT_DELAY` : un bloc marqué z puissance moins un, entrée a (in) et sortie y (out).|
|**rnl-std-45deg**|||
|`$RESISTOR_45`|`p` `n`|Une résistance à 45 degrés.|
|`$CAPACITOR_45`|`p` `n`|Un condensateur à 45 degrés.|
|`$INDUCTOR_45`|`p` `n`|Une inductance à 45 degrés (sert aussi aux enroulements diagonaux de transformateur ou de balun).|
|`$SWITCH_45`|`p` `n`|Un interrupteur ouvert à 45 degrés.|
|`$VOLTAGE_SOURCE_45`|`p` `n`|Une source de tension indépendante à 45 degrés, avec + près de p et - près de n maintenus droits.|
|`$CURRENT_SOURCE_45`|`p` `n`|Une source de courant indépendante à 45 degrés, la flèche dirigée de p vers n.|
|`$DIODE_45`|`a` `k`|Une diode à 45 degrés.|
|`$ZENER_DIODE_45`|`a` `k`|Une diode Zener à 45 degrés.|
|`$SCHOTTKY_DIODE_45`|`a` `k`|Une diode Schottky à 45 degrés.|
|`$LED_45`|`a` `k`|Une diode électroluminescente (LED) à 45 degrés.|
|`$NMOS_45`|`d` `g` `s`|Un transistor NMOS à trois broches à 45 degrés.|
|`$NMOS_4TERM_45`|`d` `g` `s` `b`|Un transistor NMOS à quatre broches à 45 degrés, substrat en (0, 0).|
|`$PMOS_45`|`d` `g` `s`|Un transistor PMOS à trois broches à 45 degrés.|
|`$PMOS_4TERM_45`|`d` `g` `s` `b`|Un transistor PMOS à quatre broches à 45 degrés, substrat en (0, 0).|
|`$NPN_45`|`c` `b` `e`|Un transistor bipolaire NPN à 45 degrés.|
|`$PNP_45`|`c` `b` `e`|Un transistor bipolaire PNP à 45 degrés.|
|`$ARROW_45_SYMBOL`|—|Une pointe de flèche pleine orientée à -45 degrés (bas-droite, le long de p->n / a->k des symboles à 45 degrés sous R90), tournant vers les quatre diagonales sous R0/R90/R180/R270.|

### Ensembles et styles

|Nom|Paquet|Signification|
|---|---|---|
|`$GROUND_NET_SET`|rnl-std-basic|Les nets de masse (0 par défaut), qu’un fichier modifie avec ADD-TO et REMOVE-FROM ; rnl-std-fill donne à leurs broches non câblées un raccord de masse et les laisse non routés. La manière dont une masse est dessinée n’en dépend pas.|
|`$SUPPLY_NET_SET`|rnl-std-basic|Les nets d’alimentation (aucun par défaut), qu’un fichier modifie avec ADD-TO et REMOVE-FROM.|
|`$BIAS_NET_SET`|rnl-std-basic|Les nets de polarisation (aucun par défaut), qu’un fichier modifie avec ADD-TO et REMOVE-FROM.|
|`$PORT_FILL_SET`|rnl-std-basic|Les nets de port auxquels rnl-std-fill attribue une étiquette de port : chaque net de port de chaque niveau — y compris les nets d’alimentation et de masse — à l’exception de ceux qui possèdent un `$IO_PORT` explicite et de ceux d’un niveau doté d’un `$PORT_LIST`.|
|`$GROUND_FILL_SET`|rnl-std-fill|Les broches non câblées sur des nets de masse, auxquelles la complétion attribue un raccord de masse.|
|`$SUPPLY_FILL_SET`|rnl-std-fill|Les broches non câblées sur des nets d’alimentation, auxquelles la complétion attribue un raccord d’alimentation.|
|`$BIAS_FILL_SET`|rnl-std-fill|Les broches non câblées sur des nets de polarisation, auxquelles la complétion attribue un raccord de polarisation.|
|`$LABEL_FILL_SET`|rnl-std-fill|Les autres broches non câblées sur des nets, auxquelles la complétion attribue un raccord d’étiquette de net ; une broche de bus en reçoit un seul.|
|`$base`|rnl-std-style|Le style de base de chaque objet : traits, textes et remplissages tels qu’Analog Canvas les dessine.|
|`$bus`|rnl-std-style|Le style d’un bus : ses traits en gras.|
|`$signal`|rnl-std-style|Un style pour les fils de signal : bleu et en gras.|
|`$note`|rnl-std-style|Un style pour les notes : texte gris, plus petit.|
|`$grey`|rnl-std-style|Un style gris pour les traits et les textes.|

<!-- lists:end -->
