<!-- translated from en/front.md sha256:0cc8c8aaa953 -->
# Norme du langage RNL, version 0.4

RNL (Rich Netlist) s’écrit dans les commentaires d’une netlist de circuit. Il décrit comment le circuit est dessiné sous forme de schéma et ce que signifient ses parties, tandis que la netlist seule définit le circuit. Ce document est la norme de la version 0.4 du langage.

- Le texte anglais est normatif. Ses traductions, nommées `rnl-0.4.<lang>.md` d’après une étiquette de langue telle que `zh` ou `ja`, sont informatives ; en cas de divergence avec une traduction, le texte anglais prévaut.
- La liste de mots `rnl-words-0.4.json` et son schéma `rnl-words-0.4.schema.json` font partie de cette norme (5.3, Annexe A).
- Les principes de RNL figurent dans `rich-netlist.md` (en anglais, texte faisant foi ; traductions : `rich-netlist.<lang>.md`) ; la présente norme s’y réfère sous le nom de « Principes ».
- Les versions 0.x ne font aucune promesse de compatibilité entre elles (16.1).
- Ce texte révise la version 0.3 selon le projet de version 0.4 du 2026-11-19 ; l’Annexe D en énumère les modifications.

## Sommaire

- [1 Vue d’ensemble](#1-vue-densemble)
  - [1.1 Domaine d’application](#11-domaine-dapplication)
  - [1.2 Structure de la présente norme](#12-structure-de-la-présente-norme)
  - [1.3 D’un coup d’œil](#13-dun-coup-dœil)
  - [1.4 Termes et conventions](#14-termes-et-conventions)
  - [1.5 Principes fondamentaux](#15-principes-fondamentaux)
  - [1.6 Conformité](#16-conformité)
- [2 Incorporation dans l’hôte](#2-incorporation-dans-lhôte)
  - [2.1 Analyse en deux étapes](#21-analyse-en-deux-étapes)
  - [2.2 Porteurs](#22-porteurs)
  - [2.3 L’entrée](#23-lentrée)
  - [2.4 Étendue d’un enregistrement](#24-étendue-dun-enregistrement)
  - [2.5 Enregistrements désactivés et texte hôte](#25-enregistrements-désactivés-et-texte-hôte)
  - [2.6 Niveaux](#26-niveaux)
  - [2.7 Fichiers de paquet](#27-fichiers-de-paquet)
  - [2.8 En-tête d’enregistrement](#28-en-tête-denregistrement)
  - [2.9 Fichiers Verilog-A](#29-fichiers-verilog-a)
- [3 Structure lexicale](#3-structure-lexicale)
  - [3.1 Caractères](#31-caractères)
  - [3.2 Jetons](#32-jetons)
  - [3.3 Texte libre](#33-texte-libre)
  - [3.4 Commentaires](#34-commentaires)
  - [3.5 Espaces blancs et adjacence](#35-espaces-blancs-et-adjacence)
  - [3.6 Noms](#36-noms)
  - [3.7 Littéraux](#37-littéraux)
  - [3.8 Points](#38-points)
- [4 Syntaxe](#4-syntaxe)
  - [4.1 Grammaire](#41-grammaire)
  - [4.2 Contraintes hors de la grammaire](#42-contraintes-hors-de-la-grammaire)
  - [4.3 Ordre des mots](#43-ordre-des-mots)
  - [4.4 Exemples d’erreurs](#44-exemples-derreurs)
  - [4.5 Phrases standards](#45-phrases-standards)
- [5 Objets et instructions](#5-objets-et-instructions)
  - [5.1 Objets](#51-objets)
  - [5.2 Ensembles](#52-ensembles)
  - [5.3 Instructions et signatures](#53-instructions-et-signatures)
  - [5.4 Modes de consommation de l’entrée](#54-modes-de-consommation-de-lentrée)
  - [5.5 Chaînages et imbrication](#55-chaînages-et-imbrication)
  - [5.6 Définitions en tant qu’objets](#56-définitions-en-tant-quobjets)
  - [5.7 Poignées et noms](#57-poignées-et-noms)
  - [5.8 Clés des faits](#58-clés-des-faits)
  - [5.9 Numéros des objets sans nom](#59-numéros-des-objets-sans-nom)
- [6 Noms et portée](#6-noms-et-portée)
  - [6.1 Hiérarchie des niveaux](#61-hiérarchie-des-niveaux)
  - [6.2 Préfixes](#62-préfixes)
  - [6.3 Références hôtes](#63-références-hôtes)
  - [6.4 Bornes](#64-bornes)
  - [6.5 Sous-objets](#65-sous-objets)
  - [6.6 Nommage](#66-nommage)
  - [6.7 Portée des noms](#67-portée-des-noms)
  - [6.8 Héritage et masquage](#68-héritage-et-masquage)
  - [6.9 Noms dans les corps de définition](#69-noms-dans-les-corps-de-définition)
  - [6.10 Paramètres génériques](#610-paramètres-génériques)
  - [6.11 Indices](#611-indices)
  - [6.12 Noms de bornes : DEVICE](#612-noms-de-bornes-device)
- [7 Échecs et diagnostics](#7-échecs-et-diagnostics)
  - [7.1 Deux phases de vérification](#71-deux-phases-de-vérification)
  - [7.2 Objets en échec](#72-objets-en-échec)
  - [7.3 Entrées strictes et tolérantes](#73-entrées-strictes-et-tolérantes)
  - [7.4 Reprise sur erreur](#74-reprise-sur-erreur)
  - [7.5 Les conflits ne sont pas des échecs](#75-les-conflits-ne-sont-pas-des-échecs)
  - [7.6 Diagnostics](#76-diagnostics)
- [8 Prise d’effet](#8-prise-deffet)
  - [8.1 Un niveau prend effet après avoir été lu entièrement](#81-un-niveau-prend-effet-après-avoir-été-lu-entièrement)
  - [8.2 Phases](#82-phases)
  - [8.3 Files d’apparence](#83-files-dapparence)
  - [8.4 Surcharge par clé](#84-surcharge-par-clé)
  - [8.5 Conflits](#85-conflits)
- [9 Définitions](#9-définitions)
  - [9.1 Règles communes](#91-règles-communes)
  - [9.2 Symboles : DEFINE-SYMBOL](#92-symboles-define-symbol)
  - [9.3 Styles : DEFINE-STYLE](#93-styles-define-style)
  - [9.4 Définitions générales : DEFINE](#94-définitions-générales-define)
  - [9.5 Usages des définitions générales](#95-usages-des-définitions-générales)
- [10 Sélections, règles et génération un par un](#10-sélections-règles-et-génération-un-par-un)
  - [10.1 Mots-clés de sélection](#101-mots-clés-de-sélection)
  - [10.2 La condition `wired`](#102-la-condition-wired)
  - [10.3 Évaluation des sélections](#103-évaluation-des-sélections)
  - [10.4 Règles](#104-règles)
  - [10.5 Génération un par un](#105-génération-un-par-un)
- [11 Placement et apparence](#11-placement-et-apparence)
  - [11.1 Position : PLACE](#111-position-place)
  - [11.2 Orientation : ORIENT](#112-orientation-orient)
  - [11.3 Symboles et styles : SET-SYMBOL et SET-STYLE](#113-symboles-et-styles-set-symbol-et-set-style)
  - [11.4 Plusieurs dessins d’un même objet](#114-plusieurs-dessins-dun-même-objet)
  - [11.5 Groupes : GROUP](#115-groupes-group)
  - [11.6 Ordre de tracé et couches : LAYER](#116-ordre-de-tracé-et-couches-layer)
  - [11.7 Coordonnées et unités](#117-coordonnées-et-unités)
  - [11.8 Faisceaux : BUNDLE](#118-faisceaux-bundle)
- [12 Composants virtuels](#12-composants-virtuels)
  - [12.1 Concept](#121-concept)
  - [12.2 Création de composants virtuels](#122-création-de-composants-virtuels)
  - [12.3 Positions relatives à une borne](#123-positions-relatives-à-une-borne)
  - [12.4 Ports](#124-ports)
  - [12.5 Composants virtuels standards](#125-composants-virtuels-standards)
  - [12.6 Marques de non-connexion](#126-marques-de-non-connexion)
- [13 Fils et texte](#13-fils-et-texte)
  - [13.1 Fils : WIRE](#131-fils-wire)
  - [13.2 Points de jonction](#132-points-de-jonction)
  - [13.3 Texte : TEXT](#133-texte-text)
  - [13.4 Annotations : graphiques dans la scène](#134-annotations-graphiques-dans-la-scène)
  - [13.5 Symboles dessinés comme annotations : ANNOTATION](#135-symboles-dessinés-comme-annotations-annotation)
  - [13.6 Balisage de texte et références aux attributs](#136-balisage-de-texte-et-références-aux-attributs)
  - [13.7 Dessiner les connexions sous forme de fils : ROUTE-NET, ROUTE-OBJECT, UNROUTE-NET, UNROUTE-OBJECT](#137-dessiner-les-connexions-sous-forme-de-fils-route-net-route-object-unroute-net-unroute-object)
- [14 Intention de placement](#14-intention-de-placement)
  - [14.1 Règles communes](#141-règles-communes)
  - [14.2 ALIGN](#142-align)
  - [14.3 ORDER](#143-order)
  - [14.4 DISTRIBUTE](#144-distribute)
  - [14.5 DISTANCE](#145-distance)
  - [14.6 Flux de signal](#146-flux-de-signal)
  - [14.7 Symétrie](#147-symétrie)
  - [14.8 Priorité : PRIORITY](#148-priorité-priority)
  - [14.9 Relation d’orientation : ORIENT-RELATION](#149-relation-dorientation-orient-relation)
  - [14.10 Réglages du solveur : SOLVER-CONFIG](#1410-réglages-du-solveur-solver-config)
  - [14.11 Tables : TABLE](#1411-tables-table)
- [15 Sémantique électrique](#15-sémantique-électrique)
  - [15.1 Application des définitions sémantiques](#151-application-des-définitions-sémantiques)
  - [15.2 Sens du courant : CURRENT](#152-sens-du-courant-current)
  - [15.3 Assertions : ASSERT-INSTANCE, ASSERT-TERMINAL, ASSERT-NET, ASSERT-USAGE, ASSERT-IN](#153-assertions-assert-instance-assert-terminal-assert-net-assert-usage-assert-in)
  - [15.4 Assertions de topologie : ASSERT-TOPOLOGY](#154-assertions-de-topologie-assert-topology)
- [16 Fichiers, versions, paquets et alias](#16-fichiers-versions-paquets-et-alias)
  - [16.1 Versions](#161-versions)
  - [16.2 Paquets et USE-PACKAGE](#162-paquets-et-use-package)
  - [16.3 Le paquet standard](#163-le-paquet-standard)
  - [16.4 Extensions et contenu inconnu](#164-extensions-et-contenu-inconnu)
  - [16.5 Alias](#165-alias)
- [17 Modèle de modification](#17-modèle-de-modification)
- [Annexe A Signatures des mots-clés (normative)](#annexe-a-signatures-des-mots-clés-normative)
  - [A.1 L’enregistrement](#a1-lenregistrement)
  - [A.2 Enregistrements d’en-tête et paquets](#a2-enregistrements-den-tête-et-paquets)
  - [A.3 Définitions](#a3-définitions)
  - [A.4 Contenu des symboles et graphiques](#a4-contenu-des-symboles-et-graphiques)
  - [A.5 Placement et apparence](#a5-placement-et-apparence)
  - [A.6 Sélections](#a6-sélections)
  - [A.7 Application de définitions](#a7-application-de-définitions)
  - [A.8 Fils](#a8-fils)
  - [A.9 Ensembles, groupes et faisceaux](#a9-ensembles-groupes-et-faisceaux)
  - [A.10 Contraintes et priorité](#a10-contraintes-et-priorité)
  - [A.11 Sémantique électrique](#a11-sémantique-électrique)
  - [A.12 Autres mots standards](#a12-autres-mots-standards)
- [Annexe B Phrases standards (informative)](#annexe-b-phrases-standards-informative)
  - [B.1 Placement et orientation](#b1-placement-et-orientation)
  - [B.2 Contraintes de disposition](#b2-contraintes-de-disposition)
  - [B.3 Apparence](#b3-apparence)
  - [B.4 Symboles et graphismes](#b4-symboles-et-graphismes)
  - [B.5 Fils et texte](#b5-fils-et-texte)
  - [B.6 Définitions et applications](#b6-définitions-et-applications)
  - [B.7 Sélections](#b7-sélections)
  - [B.8 Ensembles et autres mots-clés](#b8-ensembles-et-autres-mots-clés)
  - [B.9 Enregistrement d’en-tête et paquets](#b9-enregistrement-den-tête-et-paquets)
- [Annexe C Réservé pour les versions ultérieures (informative)](#annexe-c-réservé-pour-les-versions-ultérieures-informative)
- [Annexe D Modifications par rapport à la version 0.3 (informative)](#annexe-d-modifications-par-rapport-à-la-version-03-informative)

<!-- translated from en/ch01.md sha256:e38c6540c72f -->
## 1 Vue d’ensemble

### 1.1 Domaine d’application

RNL (Rich Netlist) s’écrit dans les commentaires d’une netlist hôte. Il décrit comment le circuit que la netlist définit est compris et présenté sous forme de schéma : quels symboles sont utilisés, où vont les composants, comment ils sont orientés, alignés, câblés, étiquetés et stylés, et ce que signifient les groupes de composants (Principes 1).

Les composants, types, paramètres, connexions et interfaces de sous-circuits ne sont définis que par la netlist hôte. RNL s’y réfère et ne peut pas les modifier. RNL ne vérifie pas la signification électrique qu’un dessin suggère (Principes 2.2).

La présente norme définit :

- comment RNL est extrait des commentaires d’une netlist hôte, ainsi que des fichiers Verilog-A qu’elle importe (chapitre 2) ;
- la structure lexicale et la syntaxe du texte RNL (chapitres 3 et 4) ;
- quels objets le texte décrit, comment les noms sont résolus, comment les échecs sont traités et comment les descriptions prennent effet ensemble (chapitres 5–8) ;
- le vocabulaire de chaque sorte de description (chapitres 9–17) ;
- la liste de mots standard, publiée avec ce texte sous le nom de `rnl-words-0.4.json` (5.3, Annexe A).

La présente norme ne définit ni un algorithme de placement ni un rendu unique (Principes 1), et elle ne définit pas comment une netlist hôte est analysée (Principes 2.1). Le paquet standard `rnl-std-all` et les paquets d’alias tels que `alias-zh` sont écrits en RNL ; la présente norme définit le fonctionnement des paquets (16.2, 16.3, 16.5), et leur contenu est publié séparément.

Ce texte définit la version 0.4 du langage. Les versions 0.x ne font aucune promesse de compatibilité entre elles (16.1).

### 1.2 Structure de la présente norme

La norme se compose d’un modèle abstrait et d’une syntaxe textuelle. Le modèle abstrait recouvre les objets, les faits, les modes de consommation de l’entrée et les phases de prise d’effet (chapitres 5–8) ; la syntaxe textuelle est sa forme écrite (chapitres 2–4). Les outils échangent le texte.

L’Annexe A dresse la liste des signatures des mots-clés ; elle est générée à partir de la liste de mots et est normative. L’Annexe B donne les phrases standards, l’Annexe C énumère ce qui est réservé pour des versions ultérieures, et l’Annexe D énumère les modifications par rapport à la version 0.3 ; toutes trois sont informatives. Les notes et les exemples sont informatifs. Tout le reste est normatif.

### 1.3 D’un coup d’œil

```spice
* Five-transistor OTA, input stage
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4")}
.model nch nmos level=1
.subckt ota5t inp inn out vdd vss vbias
M1 x    inp   tail vss nch W=2u L=0.5u
M2 out  inn   tail vss nch W=2u L=0.5u
M5 tail vbias vss  vss nch W=4u L=1u
* @RNL {
*   %dp1 = #M1(left), #M2(right) and #M5(tail) treated @AS $DIFFERENTIAL_PAIR_N;
*   {#M1 #M2} @ALIGN("y"), i.e., same y;
*   #M5 is at @PLACE(20, 6) => it @ORIENT("R0");
*   %g0 = $GROUND is @ATTACH to &vss;
*   net &vss, through {#M5:3 %g0::pin}, has a @WIRE
* }
.ends ota5t
.end
```

- `* ` commence un commentaire SPICE. `@RNL {…}` est un enregistrement ; à l’intérieur des accolades figurent des membres séparés par `;` (chapitre 2). L’enregistrement d’en-tête au début du fichier déclare la version du langage (16.1).
- `#M1`, `&vss` et `#M5:3` désignent une instance, un net et une borne de l’hôte. `%dp1` et `%g0` sont des noms RNL de ce niveau. `$DIFFERENTIAL_PAIR_N` et `$GROUND` sont des définitions générales (9.4) issues du paquet standard, que l’enregistrement d’en-tête utilise (16.3). `%g0` est l’utilisation créée par l’application de `$GROUND` ; le paquet standard lui attribue un symbole de masse, de sorte qu’il est dessiné comme une masse (chapitre 12).
- `@AS`, `@ALIGN`, `@PLACE`, `@ORIENT`, `@ATTACH` et `@WIRE` sont des mots-clés. Les paramètres s’écrivent entre parenthèses accolées au mot-clé et ne contiennent que des littéraux. AS remplit les rôles de la définition écrite après lui avec les objets écrits avant lui ; ATTACH ajoute la définition écrite avant lui aux objets écrits après lui (9.4, chapitre 12).
- `(left)` marque un rôle. `%dp1 =` nomme le résultat d’une instruction. `=>` transmet le résultat d’un segment au suivant ; suivi uniquement d’un nom, il nomme le résultat du segment précédent (5.5).
- `and`, `treated`, `, i.e., same y`, `is at`, `it`, `is … to`, `net`, `, through`, `, has a` sont du texte libre et ne modifient pas le sens (3.3). Les phrases standards ajoutent de tels mots entre les jetons afin que les instructions se lisent comme des phrases (4.5, Annexe B). Sans eux, l’enregistrement s’écrit :

```rnl
%dp1 = #M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N;
{#M1 #M2} @ALIGN("y");
#M5 @PLACE(20, 6) => @ORIENT("R0");
%g0 = $GROUND @ATTACH &vss;
&vss {#M5:3 %g0::pin} @WIRE
```

### 1.4 Termes et conventions

#### 1.4.1 Mots d’exigence

- **doit** (**must**), **ne doit pas** (**must not**) : une exigence portant sur les documents ou les implémentations.
- **devrait** (**should**), **ne devrait pas** (**should not**) : une recommandation ; il peut exister des raisons d’agir autrement, mais celles-ci devraient d’abord être comprises.
- **peut** (**may**) : quelque chose qui est autorisé.

Le sens du langage lui-même est énoncé au présent : « l’instruction échoue », « le dernier l’emporte ».

#### 1.4.2 Échecs et diagnostics

- Un élément (un champ, une instruction, un enregistrement ou un fichier ; 7.6) qui **échoue** est sans effet ; une instruction qui échoue a pour valeur un objet en échec (7.2). Tout échec est une **erreur**, quelle qu’en soit la cause (7.6).
- Un **avertissement** signale un élément valide mais qui pourrait ne pas correspondre à l’intention de l’auteur ; il prend effet normalement, sauf lorsque la présente norme en dispose autrement, comme pour un fait que le solveur n’a pas pu décider dans sa limite de temps (8.5). Une **violation** signale qu’une instruction énoncée par le fichier au sujet du circuit n’est pas vérifiée ; rien n’échoue (15.3, 15.4). Une **indication** signale une surcharge, un masquage ou une résolution courant qui peut aussi être une erreur. Les cas sont énumérés en 7.6 ; là où la présente norme indique que quelque chose produit un avertissement ou une indication, les implémentations devraient le signaler.
- Lorsque **l’interprétation échoue**, rien n’est dessiné et une erreur est signalée (1.5 point 3, 7.6).

#### 1.4.3 Notation

- Dans le texte, les mots-clés sont écrits en majuscules sans `@` : PLACE, SET-SYMBOL. Dans la syntaxe et les exemples, ils sont écrits avec `@`.
- Les mots standards, les noms et les littéraux sont écrits en police de code : `priority`, `"high"`, `instance`, `$GROUND`.
- Les numéros de section entre parenthèses renvoient à la présente norme : (9.4). « Principes 2.4 » renvoie à la section 2.4 des principes de RNL, `rich-netlist.md`.
- Les exemples sont écrits sous la forme standard (4.3) et sans texte libre, sauf lorsque le texte libre est le sujet traité. Un exemple marqué comme erreur illustre une faute.
- Les exemples utilisent le paquet standard : un exemple qui ne montre aucun enregistrement d’en-tête se lit comme si son fichier principal contenait `@USE-PACKAGE(name="rnl-std-all", version="0.4")` dans son enregistrement d’en-tête (16.3).

#### 1.4.4 Termes

| Terme | Anglais | Signification |
|---|---|---|
| alias | alias | Une autre graphie d’un mot standard, ou d’un nom en `$` ainsi que des noms de rôles, de paramètres et locaux d’une définition, déclarée avec ALIAS ou chargée avec USE-ALIASES (16.5) |
| paquet d’alias | alias package | Un fichier de paquet qui ne déclare que des alias ; chargé avec USE-ALIASES (2.7, 16.5) |
| annotation | annotation | Un élément graphique ou un texte écrit dans la scène plutôt qu’attaché à un objet (13.4) |
| appliquer, application | apply, application | Ce que font AS et ATTACH : remplir les rôles d’une définition avec des objets. Chaque application crée une utilisation (9.4) |
| attacher | attach | Un TEXT est attaché au point ou à l’objet qu’il reçoit : il peut lire les attributs de l’objet, et il est placé à l’endroit ou à côté de l’objet sauf si une position est donnée (13.3). À ne pas confondre avec le mot-clé ATTACH, qui applique une définition générale (9.4) |
| corps | body | Les instructions comprises entre les accolades d’une définition ou d’un paquet en ligne (9.1, 16.2) |
| faisceau | bundle | Un objet créé avec BUNDLE : plusieurs instances hôtes dessinées comme une seule, à leur place. Ses bornes représentent les bornes de même nom de tous ses membres (11.8) |
| bus | bus | Un fil dont l’emplacement de net reçoit un ensemble de nets, même un ensemble d’un seul élément ; la netlist n’a pas de bus, un bus n’est qu’une manière de dessiner (13.1) |
| porteur | carrier | Un commentaire sur ligne entière de l’hôte qui porte du RNL : le caractère de commentaire (`*`, ou `//` en mode langage Spectre) en colonne 1, suivi de rien, uniquement d’espaces blancs, ou d’une seule espace et du contenu (2.2) |
| versement | commit | Ce qu’un enregistrement fait de ses instructions : chaque instruction d’un enregistrement devient partie intégrante de son niveau, qu’elle soit nommée ou non (5.4) |
| contrainte | constraint | Une condition portant sur les positions ou les orientations pour le solveur : ALIGN, ORDER, DISTRIBUTE, DISTANCE, ORIENT-RELATION, TABLE (chapitre 14) |
| définition | definition | Ce que créent DEFINE, DEFINE-SYMBOL ou DEFINE-STYLE, généralement nommé avec `$` (chapitre 9). Ce que crée DEFINE est une **définition générale** ; les symboles (DEFINE-SYMBOL) et les styles (DEFINE-STYLE) sont des **définitions spécifiques**. Le présent texte emploie « définition » pour une définition générale partout où les rôles, paramètres, applications ou utilisations rendent la sorte claire, et « définition générale » lorsqu’il faut la distinguer des symboles et des styles |
| fait | fact | Une assertion portée par une instruction, telle que « M1 est en (10, 20) » (5.1) |
| objet en échec | failed object | La valeur d’une instruction qui a échoué (7.2) |
| complétion | fill | Les masses, alimentations et étiquettes de nets que les règles du paquet standard ajoutent pour les bornes et les nets qui ne sont pas dessinés autrement (10.5, 16.3) |
| texte libre | free text | Texte placé entre les jetons d’une instruction ; il ne modifie pas le sens (3.3) |
| ensemble généré | generated set | L’ensemble que produit une règle de génération ; ses membres pris isolément ne peuvent pas être référencés (10.5) |
| groupe | group | Un ensemble marqué avec GROUP : ce n’est pas un objet ; le solveur dispose ses membres ensemble et maintient les autres hors de sa région, à titre de critère après chaque fait (11.5) |
| poignée | handle | La chose sur laquelle porte un fait : une instance, un net ou une borne de l’hôte, ou un point, fil, ensemble ou autre objet créé par RNL. Les faits concernant une même poignée prennent effet ensemble (5.1, 5.7) |
| enregistrement d’en-tête | header record | Un enregistrement portant `type="header"` au début d’un fichier : un sans `language`, et au plus un pour chaque langage (2.8) |
| masquage | hiding | Une définition d’un niveau inférieur, ou d’un paquet ultérieur, utilisée à la place d’une définition de même nom provenant du dessus (6.8, 16.2) |
| hôte, netlist hôte | host, host netlist | La netlist qui contient le RNL. L’adaptateur hôte la lit et fournit ses commentaires et ses objets à RNL (2.1) |
| style maison | house style | La manière générale de dessiner fixée par un paquet ou une implémentation : quels symboles, styles et définitions de placement sont utilisés, si les connexions sans fils sont représentées par des étiquettes ou par un chevelu, etc. Un paquet qui en fixe un est un paquet de style maison. Ce n’est pas un style au sens de DEFINE-STYLE |
| élément | item | Ce à quoi s’applique un échec : un champ d’un en-tête d’enregistrement, une instruction, un enregistrement, un fichier ou l’interprétation entière (7.6) |
| instance | instance | Un élément de la netlist hôte, y compris les instances de sous-circuit (lignes X). Une instance itérée telle que `XI<3:0>` représente plusieurs instances (6.11) |
| clé | key | Le nom d’un texte sur l’objet auquel il est attaché : un TEXT avec `key="wl"` sur M1 est `#M1::wl` (13.3) |
| mot-clé | keyword | `@` suivi d’un mot en majuscules, tel que `@PLACE`, ou d’un alias déclaré (16.5) ; au plus un par segment (4.1, 5.3) |
| étiquette | label | Un TEXT attaché à un objet ; il peut se référer aux attributs de l’objet, tels que son nom ou sa valeur (13.3) |
| niveau | level | Une étendue de texte : le niveau de fichier du fichier principal (le niveau supérieur du circuit), le niveau de fichier d’un fichier inclus, ou une définition de sous-circuit ; chaque paquet est aussi un niveau. Les niveaux forment un arbre par inclusion textuelle (2.6, 6.1) |
| objet local | local object | Un objet nommé dans le corps d’une définition générale ; chaque utilisation possède le sien, désigné de l’extérieur par `%c1::mid` (9.4) |
| objet principal | main object | L’objet qui remplit le premier rôle d’une utilisation ; les textes s’y réfèrent par l’intermédiaire du rôle (13.6). Une utilisation est dessinée à son point propre, et non à son objet principal (12.1) |
| membre, segment, instruction | member, segment, statement | Un membre est une partie d’un ensemble séparée par `;`. Un segment est une partie d’un membre séparée par `=>` (un nom seul après `=>` n’est pas un segment, 4.2). Une instruction est un segment comportant un mot-clé, ou un objet dans un segment qui n’en comporte pas (4.2, 5.3) |
| nom | name | `%x =`, `$X =`, `=> %x` ou `=> $X` donne un nom à un objet ; un nom ne crée aucun objet et n’est lié qu’une seule fois dans un niveau (6.6) |
| point propre | own point | Le point d’une utilisation : l’origine de son symbole, ou un point libre que le corps de sa définition attache à ses membres ; écrit `%u::position`. Une contrainte le prend pour une utilisation pourvue d’un symbole, PLACE pour toute utilisation (14.1, 6.5) |
| paquet | package | Un ensemble de définitions et de règles placé au-dessus d’un niveau par USE-PACKAGE ; écrit sous forme de fichier de paquet (2.7) ou en ligne dans le corps de USE-PACKAGE (16.2). Un paquet qui ne fait que référencer d’autres paquets est une **collection de paquets (package collection)**, telle que `rnl-std-all` (16.3) |
| paquets à la racine | packages at the root | Paquets placés avant tous les niveaux : les paquets de USE-PACKAGE dans les enregistrements d’en-tête, le paquet standard figurant en premier parmi eux lorsqu’il est écrit ou ajouté par un réglage de l’outil (16.3) |
| broche | pin | Un objet d’un symbole : un point où se connectent les fils, qui lie des bornes de l’objet dessiné avec le symbole, ou des rôles d’une utilisation, ou rien (9.2.4) |
| priorité | priority | Le rang d’un fait géométrique : `"disabled"`, `"low"`, `"normal"` ou `"high"`. En cas de conflit, la priorité est comparée en premier, puis l’endroit où le fait est écrit (8.5, 14.8) |
| enregistrement | record | Un `@RNL(…) {…}` dans les commentaires. Les champs entre parenthèses forment l’en-tête d’enregistrement et s’appliquent à chaque instruction de l’enregistrement (2.3, 2.8) |
| objet RNL | RNL object | Ce que crée une instruction comportant un mot-clé ou un `{…}` : une poignée et des faits (5.1) |
| rôle | role | L’emplacement qu’un objet remplit dans une instruction, marqué entre parenthèses juste après l’objet, comme dans `#M1(in_p)`. Les rôles d’une définition sont nommés dans sa liste de rôles et écrits `?name` dans son corps ; les rôles d’un mot-clé intégré sont les noms de ses emplacements (5.3, 9.4) |
| règle | rule | Une instruction qui reçoit le résultat d’une sélection. Une règle écrite dans un niveau est évaluée dans ce niveau et, avec `recursive=true`, une fois de plus dans chaque niveau inférieur, à chaque fois avec les objets de ce seul niveau (10.4) |
| scène | scene | Les objets d’un niveau, provenant de l’hôte et de RNL, avec les faits qui ont pris effet (chapitre 8) |
| sélection | selection | Le résultat d’un mot-clé SELECT-… : les objets d’une même sorte dans sa portée qui satisfont à ses conditions, évalué au moment de son utilisation (chapitre 10) |
| ensemble | set | Une collection ordonnée d’objets, écrite `{…}` ou trouvée par une sélection (5.2) |
| emplacement | slot | L’une des places où un mot-clé reçoit des objets, telles que `from` et `to` de DISTANCE. Chaque emplacement a un nom, qui peut être écrit comme un rôle (5.2.1, 5.3) |
| solveur | solver | L’outil qui calcule les positions et les orientations à partir du RNL et les écrit dans un enregistrement généré (14.8, chapitre 17) |
| forme standard | standard form | La façon unique d’écrire chaque mot-clé que produisent les outils : où se place le mot-clé, quels paramètres sont donnés par position, si les rôles sont marqués (4.3, 5.3) |
| paquet standard | standard package | La collection de paquets `rnl-std-all` de définitions, symboles, styles et règles, qu’un fichier utilise en l’écrivant dans son enregistrement d’en-tête, ou qu’un outil ajoute par un réglage (16.3) |
| phrase standard | standard sentence | Une instruction sous forme standard avec du texte libre ajouté entre les jetons afin qu’elle se lise comme une phrase (4.5, Annexe B) |
| mot standard | standard word | Un mot défini par la liste de mots : mots-clés, noms d’emplacements, noms et valeurs de paramètres, champs d’en-tête d’enregistrement, sortes, noms de sous-objets, noms d’attributs, propriétés de style, `true` et `false` (5.3) |
| style | style | Uniquement les propriétés définies avec DEFINE-STYLE : couleurs, style de trait, épaisseur de trait, taille de texte, etc. (9.3). Les préférences de placement ne sont pas des styles |
| indice | subscript | `<…>` ou `[…]` juste après un nom hôte ou un nom de borne, tel que `&D<3>` ou `&D<7:0>`, représentant un ou plusieurs noms (6.11) ; à ne pas confondre avec les indices du balisage de texte (13.6) |
| sous-objet | sub-object | Une partie d’un objet atteinte par `::`, telle que `#M1::position` ou `%dp::left` (6.5) |
| symbole | symbol | Ce que crée DEFINE-SYMBOL : éléments graphiques, broches et textes, dessinés pour une instance, un faisceau ou une utilisation (9.2) |
| prendre effet | take effect | Une fois qu’un niveau a été lu entièrement, ses faits deviennent la scène, phase par phase (chapitre 8) |
| jeton | token | Un mot-clé, une référence, un point, une parenthèse, un crochet, une accolade, `;`, `=>` ou `=` ; tout le reste est du texte libre ou des commentaires (3.2) |
| borne | terminal | Une connexion d’une instance, telle qu’elle figure sur sa ligne de netlist : `#M1:d`, ou `#M1:1` par numéro. `#:inp` est le port `inp` du niveau (6.4) |
| utilisation | usage | L’objet que crée une application. Elle consigne la définition et quels objets remplissent ses rôles ; lorsqu’elle possède un symbole, elle est dessinée (9.4, 9.5). Dans une contrainte, une utilisation sans symbole représente ses membres (14.1) |
| composant virtuel | virtual part | Une utilisation qui possède un symbole : dessinée, mais ne faisant pas partie de la netlist, telle qu’une masse, une alimentation, une étiquette de port ou un point de jonction (chapitre 12) |
| liste de mots | word list | La liste lisible par machine de tous les mots standards et signatures de mots-clés d’une version du langage, `rnl-words-0.4.json` pour la présente version (5.3, Annexe A) |

### 1.5 Principes fondamentaux

Les règles de la présente norme suivent ces principes. Les points 1–6 précisent Principes 2.9 ; le point 7 indique comment le vocabulaire est réparti.

1. **Tout est description.** Le texte RNL est un ensemble de descriptions. L’interpréteur les rassemble toutes avant de donner un résultat. Rien n’est exécuté, et un nom peut être utilisé avant l’instruction qui le lie.
2. **L’endroit où l’on écrit n’est qu’une donnée d’ordre.** L’endroit où une description est écrite (son niveau et sa place dans le texte) n’importe que dans trois cas : lorsque des descriptions se contredisent, la dernière est retenue (les faits géométriques comparent d’abord la priorité ; 8.3, 8.4, 8.5) ; l’ordre de tracé (11.6) ; et l’ordre des membres d’un ensemble (5.2). Sinon, le résultat ne dépend pas de l’ordre d’écriture. Chaque segment d’un chaînage a sa propre place : un segment ultérieur est écrit après un segment antérieur (5.5).
3. **Contradictions.**
   - Une description incompréhensible (erreur de syntaxe, nom introuvable, mauvaise sorte, etc.) ou qui se contredit elle-même échoue (Principes 2.4, 2.9 ; chapitre 7).
   - Les descriptions compréhensibles mais qui se contredisent entre elles sont résolues là où la présente norme donne une méthode définie, telle que la surcharge par clé (8.4), les files d’apparence (8.3) et la résolution des faits géométriques (8.5) ; une indication peut être donnée.
   - Là où il n’existe pas encore de méthode définie, l’interprétation entière échoue (7.6). La présente version ne comporte aucun cas de ce type (7.5). Une version ultérieure pourra définir une méthode et les rendre tolérants ; les fichiers qui fonctionnent aujourd’hui n’en sont pas affectés.
4. **Chaque description voit l’état avant qu’elle ne prenne effet.** Une description lit l’état tel qu’il était lorsque sa phase a commencé ; son propre effet et les effets des autres descriptions de la même phase ne sont pas inclus (8.2).
5. **Objets et noms.** Chaque instruction comportant un mot-clé et chaque `{…}` crée un nouvel objet, fait d’une poignée et de faits. Une référence ne crée aucun objet, sauf que prendre des membres par leurs places crée un nouvel ensemble (5.2.3). Un nom ne fait que désigner un objet et n’est lié qu’une seule fois dans un niveau. La surcharge s’applique aux faits, non aux noms (5.1, 6.6).
6. **Aucune limite au nom de la lisibilité.** Une forme difficile à lire mais ayant un sens défini est autorisée ; il appartient aux auteurs de se modérer. Seules les formes dépourvues de sens défini sont des erreurs.
7. **Mots-clés et définitions.** Ce qu’une implémentation doit comprendre elle-même, et ce qui ne peut pas être écrit à l’aide d’autres descriptions, est un mot-clé. Ce qui peut être écrit avec des descriptions existantes dans le corps d’une définition générale est une définition, et son nom n’est qu’une indication (9.5).

### 1.6 Conformité

**Documents.** Une netlist hôte est conforme à RNL 0.4 lorsque ses enregistrements, lus selon les chapitres 2–4 et interprétés selon les chapitres 5–17, ne produisent aucune erreur (7.6), y compris d’erreurs dans le contenu des paquets qu’elle utilise. Les avertissements, les indications et les violations n’affectent pas la conformité : une violation signale que le circuit ne corrobore pas ce qu’énonce le fichier, et non que le fichier est mal écrit. Un fichier de paquet est conforme de la même façon lorsqu’il est vérifié isolément (16.2). Un fichier dont l’enregistrement d’en-tête déclare une autre version du langage n’est pas régi par la présente version (16.1).

**Implémentations.** Ce sont les interpréteurs (un interpréteur utilisé uniquement pour la vérification est aussi appelé lint), visionneuses, solveurs, éditeurs et autres outils qui lisent ou écrivent du RNL. Une implémentation conforme :

1. extrait le RNL des commentaires de l’hôte comme le définit le chapitre 2, et le lit comme le définissent les chapitres 3 et 4 ;
2. donne à chaque description l’effet défini par les chapitres 5–17, avec les signatures de la liste de mots (5.3) ;
3. signale chaque erreur, avec son emplacement et sa cause (7.6) ; les erreurs ne peuvent être ni abaissées ni désactivées ;
4. devrait signaler les avertissements et indications énumérés par la présente norme, et doit signaler les violations (7.6) ;
5. ne peut ajouter ses propres vérifications qu’à titre d’avertissements ou d’indications : elle ne doit pas faire échouer un élément que la présente norme laisse prendre effet, ni laisser prendre effet un élément que la présente norme fait échouer ;
6. peut proposer un mode plus strict qui s’arrête ou rejette toute l’entrée, mais pas par défaut ;
7. permet à l’utilisateur de voir les diagnostics ; leur présentation est laissée à l’implémentation (7.6) ;
8. déclare les versions du langage qu’elle prend en charge, et traite un fichier d’une version qu’elle ne prend pas en charge comme le définit 16.1 ;
9. traite les mots-clés, paramètres et valeurs inconnus comme des erreurs (16.4) ;
10. place le paquet standard à la racine lorsqu’un fichier l’écrit, comme le définit 16.3 ;
11. lorsqu’elle écrit du RNL, écrit les nouvelles instructions sous la forme standard et ne réordonne pas les instructions écrites par les utilisateurs (4.3) ; écrit l’enregistrement d’en-tête et la version lorsqu’elle enregistre un fichier (16.1) ; laisse tels quels les instructions inconnues et en échec, le texte hôte, les commentaires ordinaires et les fichiers de versions non prises en charge (16.4) ; et ne modifie que les enregistrements que le chapitre 17 l’autorise à modifier.
12. lorsqu’elle résout, résout les conflits comme le définit 8.5, réécrit chaque décision qu’elle prend, et ne transmet ses résultats aux autres outils que sous une forme qu’un enregistrement du solveur peut contenir (14.8) ;
13. peut fixer des limites aux ressources qu’elle utilise, telles que la profondeur d’imbrication des symboles (9.2.7), des développements d’utilisations (9.4), des fichiers inclus et des sous-circuits, ou le nombre d’objets. Une entrée dépassant une telle limite produit une erreur indiquant qu’une limite de l’implémentation a été atteinte, avec son emplacement ; l’implémentation ne doit pas tronquer l’entrée en silence, ni dessiner ou résoudre un résultat autre que celui que définit la présente norme. Elle prend en charge au moins 64 niveaux d’imbrication de symboles. La limite sur la vérification des faits lors d’une résolution n’est pas une telle limite : c’est une option du solveur, et un fait qu’elle laisse indécis est écarté avec un avertissement (8.5).

La présente norme ne définit pas comment un solveur choisit parmi les positions qui satisfont aux contraintes, ni comment une visionneuse dessine ce que RNL laisse ouvert ; les endroits où ce choix leur est laissé sont précisés dans le texte (par exemple 13.1).

**Tests de conformité.** Ils vérifient les erreurs, avec leurs emplacements et la portée de chaque échec, ainsi que la scène résultante. Les avertissements peuvent être vérifiés comme attentes facultatives ; les indications ne sont pas vérifiées (7.6).

<!-- translated from en/ch02.md sha256:e9821c4aa800 -->
## 2 Incorporation dans l’hôte

### 2.1 Analyse en deux étapes

- L’adaptateur hôte :
  - reconnaît les commentaires ordinaires valides de l’hôte et décide quelles lignes sont des porteurs (2.2) ;
  - détermine à quel niveau appartient chaque commentaire, et de quel fichier et de quelle inclusion proviennent chaque enregistrement et chaque sous-circuit (6.1) ;
  - fournit les positions sources et l’ordre du texte ;
  - fournit les objets hôtes : instances, nets, bornes, ports de sous-circuit et types de modèle.
- Le cœur RNL ne recherche d’enregistrements que dans ces commentaires. Il ne doit pas parcourir toute la netlist à la recherche de `@RNL` sans tenir compte du contexte : les mêmes caractères dans des instructions hôtes ou des chaînes hôtes ne sont pas du RNL.
- L’interpréteur analyse entièrement la netlist hôte avant de traiter le RNL (Principes 2.9).

### 2.2 Porteurs

Les enregistrements s’écrivent dans des commentaires sur ligne entière de l’hôte ; ces lignes de commentaire sont des porteurs. L’adaptateur hôte décide ligne par ligne quelles lignes sont des porteurs, retire le caractère de commentaire et l’unique espace qui le suit, et transmet le reste au cœur RNL (2.1).

- **Commentaires sur ligne entière uniquement.** Les commentaires de fin de ligne (`$`, `;`, `//`, `!`, etc.) ne sont ni lus ni écrits, et les commentaires en bloc ne sont pas utilisés.
  - `@RNL` dans un commentaire de fin de ligne n’est que du texte hôte et produit un avertissement.
  - Note : le fait que la fin d’une ligne soit un commentaire dépend du dialecte, du mode et des options : en HSPICE, `$` peut suivre directement un nombre (`1k$x`), en mode `ps` de ngspice `$` est un caractère ordinaire, et Eldo peut le changer avec `.COMCHAR` ; en CDL, `$SUB=` et les formes semblables sont des données. Écrire en fin de ligne modifierait également la ligne d’une instruction hôte (Principes 2.1). SPICE et Spectre n’ont pas de commentaires en bloc (Spectre uniquement à l’intérieur de `reliability{}`).
- **Caractères de commentaire des hôtes.**

  | Hôte | Caractère de commentaire |
  |---|---|
  | La famille SPICE : SPICE3, ngspice, HSPICE, Xyce, LTspice, PSpice, WRspice, Eldo, et les netlists CDL | `*` |
  | Sections en mode langage Spectre : fichiers `.scs`, ou texte après `simulator lang=spectre` | `//` ou `*` |
  | Sections en mode langage SPICE de Spectre, y compris les fichiers inclus qui n’ont pas de suffixe `.scs` et ne commencent pas par `simulator lang=spectre` | `*` |

  - `*` est un commentaire dans les deux modes de langage de Spectre. `//` n’est documenté que pour le mode langage Spectre, et ne l’est ni par Eldo ni par CDL.
  - Les lignes d’un même enregistrement peuvent mélanger `//` et `*`. Un outil qui écrit un enregistrement doit suivre les enregistrements déjà présents dans le fichier ; lorsqu’il n’y en a aucun, il écrit `*` en mode SPICE et `//` en mode Spectre.
  - Les porteurs appartiennent à la forme externe, qui ne change pas (16.1). La prise en charge d’un nouvel hôte ajoute seulement des porteurs ; les porteurs existants ne sont pas modifiés.
- **Lignes porteuses.** Chaque ligne d’un enregistrement doit satisfaire à ces deux conditions :
  - Le caractère de commentaire est en colonne 1, sans espace blanc avant lui.
  - Après le caractère de commentaire se trouve soit rien, soit uniquement de l’espace blanc, ce qui forme une ligne de commentaire vide ; soit une espace ASCII suivie du contenu. Ainsi, `*@RNL`, `*#M1` et `//@RNL` ne sont pas des lignes porteuses.

    Note : HSPICE et PSpice exigent `*` en colonne 1, et les outils traitent les lignes indentées différemment. Du texte collé au caractère de commentaire a un sens pour d’autres outils : ngspice et WRspice exécutent les lignes `*#` comme des commandes, et WRspice également les lignes `*@` ; `*.` commence une directive CDL (telle que `*.PININFO`), `*|` commence des données DSPF, et `//pragma` est l’enveloppe de chiffrement de Spectre.
  - Une ligne de commentaire qui ne satisfait pas à ces conditions (telle que `*#M1 …`, `** …` ou une ligne indentée `* …`) interrompt un enregistrement qui n’est pas encore achevé : l’enregistrement entier échoue (2.4). En dehors d’un enregistrement, `@RNL` dans une telle ligne n’est pas une entrée et produit un avertissement.
  - Une ligne contenant du RNL ne devrait pas se terminer par `\`. Une telle ligne produit un avertissement, et les outils ne doivent pas en écrire. Le RNL est lu normalement et rien n’échoue ; ce qui est menacé est la netlist hôte. Note : WRspice joint la ligne suivante à un tel commentaire, et il peut en être de même pour Spectre exécuté avec `-E` (via le préprocesseur C) et Eldo avec `SLASHCONT` ; une instruction hôte sur la ligne suivante serait alors absorbée dans le commentaire.
- **Endroits où le RNL n’est pas lu.** `@RNL` à ces endroits produit un avertissement, et les outils ne doivent pas y écrire :
  - La ligne 1 de chaque fichier hôte, et les autres lignes que l’hôte traite comme des titres (telle que la ligne qui suit `.newjob` dans WRspice). La ligne 1 du fichier principal est son titre, mais ce n’est pas un titre lorsque le fichier est inclus ; elle n’est jamais lue, afin qu’un fichier se lise de la même manière en tant que fichier principal et en tant que fichier inclus. L’enregistrement d’en-tête peut donc être écrit au plus tôt à la ligne 2 (2.8).
  - Le texte après le `.end` du fichier principal. Note : après `.end`, ngspice continue la lecture, Xyce s’arrête, et HSPICE et PSpice peuvent prendre le texte pour le circuit suivant.
  - Les régions que l’hôte ne traite pas comme des commentaires de netlist : scripts de contrôle (ngspice exécute les lignes `*#` à l’intérieur de `.control … .endc`), blocs et fichiers d’autres langages (`.verilog` dans WRspice, `reliability{}` dans Spectre ; les fichiers Verilog-A importés avec `ahdl_include` ou `.hdl` ne sont lus que comme l’indique 2.9), texte chiffré, parties supprimées par le prétraitement, et sections `.lib` non sélectionnées. Une région que l’adaptateur ne peut pas classer avec certitude est traitée comme non lue, avec un avertissement.
- **Endroits lus avec un avertissement :**
  - Les lignes de commentaire entre une instruction et ses lignes de continuation (lignes `+`). Leur sens pour RNL est unique, elles sont donc lues ; mais les hôtes les traitent différemment (PSpice signale une erreur, SPICE 3f4 joint la ligne `+` suivante au commentaire). Elles produisent un avertissement, et les outils ne doivent pas y écrire.
  - Les régions chiffrées écrites en texte clair (`.prot … .unprot`, `protect … unprotect`). Elles sont lues, avec un avertissement : une fois la région chiffrée, les enregistrements auront disparu.
- **Le langage dans lequel un fichier commence** est une entrée fournie à l’outil. Par défaut le suffixe en décide, `.scs` pour le mode Spectre et tout autre fichier pour le mode SPICE, comme Spectre lui-même le fait ; l’utilisateur peut le modifier pour chaque fichier, y compris les fichiers inclus. Après le début, chaque ligne suit les règles ci-dessus (`simulator lang=`). Savoir quelles lignes sont en mode SPICE et lesquelles en mode Spectre détermine ce sur quoi agissent les enregistrements portant `language` (2.8) ; un outil lit donc chaque fichier dans le langage choisi par son utilisateur ; la manière dont un outil permet à l’utilisateur de choisir ne fait pas partie de la présente norme.
- **Recommandations d’écriture :**
  - RNL ne limite pas la longueur des lignes (2.4). Les générateurs devraient couper les lignes aux espaces blancs, avec au plus 132 caractères par ligne : la limite des anciennes versions de PSpice, la plus stricte parmi les hôtes courants (HSPICE autorise 1024, Eldo 2000). Un adaptateur qui connaît la limite de son hôte suit l’hôte. Lorsqu’un jeton unique ne tient pas sur une ligne, l’outil doit signaler une erreur et ne doit pas l’écrire (les chaînes ne peuvent pas continuer sur la ligne suivante, 3.7).
  - Pour l’encodage, voir 3.1.

### 2.3 L’entrée

- Un enregistrement commence par `@RNL`. L’entrée n’est reconnue qu’en majuscules.
- Tous les hôtes utilisent la même entrée, `@RNL` ; c’est la sous-balise de Principes 2.1.
- `@RNL` peut être écrit au milieu d’un commentaire ; le texte qui le précède sur la même ligne appartient au commentaire hôte.
- **L’espace avant `@`.** Le caractère juste avant le `@` de l’entrée `@RNL` et d’un mot-clé doit être une espace ASCII (U+0020) ; une tabulation, un saut de ligne ou un début de ligne ne compte pas. Sur une ligne porteuse, l’espace qui suit le caractère de commentaire compte comme l’espace avant `@`, comme dans `* @RNL`. Un `@` à l’intérieur d’une chaîne, d’un nom entre guillemets ou d’un commentaire `/* */` n’est pas soumis à cette restriction.
  - Ainsi, un mot-clé après `{`, `=>`, `=` ou `;` a également besoin d’une espace avant lui : `#M1 @SET-SYMBOL { @DEFINE-SYMBOL {…}}`, `%x = @DEFINE(…)`, `#M2 @PLACE(30, 20) => @ORIENT("MY")`.
  - Note : `@` est courant dans les commentaires hôtes, comme dans les adresses de courriel et `@param` ; avec cette règle, ce texte n’est pas pris pour du RNL. WRspice exécute également les lignes qui commencent par `*@` (2.2).
  - En dehors d’un enregistrement, `x@RNL` n’est pas une entrée, mais seulement du commentaire hôte. À l’intérieur d’un enregistrement, un `@` sans espace avant lui n’est ni un mot-clé ni un caractère de texte libre : c’est une erreur de syntaxe, et le membre dans lequel il se trouve échoue (7.4).

### 2.4 Étendue d’un enregistrement

- Un enregistrement s’écrit `@RNL(…) {…}`. L’en-tête d’enregistrement entre parenthèses peut être omis ; les accolades ne le peuvent pas, et une instruction unique en a également besoin. La parenthèse de l’en-tête d’enregistrement touche `@RNL`, comme les parenthèses de paramètres d’un mot-clé touchent le mot-clé (3.5) : `@RNL (type="header") {…}` est une erreur, et l’enregistrement entier échoue.
- Un enregistrement s’étend de `@RNL` jusqu’au `}` qui ferme son premier `{`. Les chaînes et les commentaires `/* */` sont ignorés lors de la recherche du `}` correspondant. Les parenthèses dans l’en-tête d’enregistrement sont également ignorées ; après le premier `{`, chaque accolade compte, y compris une accolade à l’intérieur des parenthèses de paramètres, afin qu’une parenthèse laissée ouverte ne fasse échouer que son membre (7.4). Une chaîne ne peut pas traverser une ligne (3.7) : lorsqu’elle n’est pas fermée sur sa ligne, elle s’y termine et l’enregistrement est incomplet (7.4).
- **Enregistrements sur plusieurs lignes.** Un enregistrement peut être écrit sur plusieurs lignes de commentaire consécutives (porteurs, 2.2). Le préfixe de commentaire de chaque ligne est retiré et les contenus sont joints ; un saut de ligne est un espace blanc (3.5).
  - Une ligne de commentaire vide, avec seulement `*` (ou `*` et des espaces blancs), est un espace blanc, et l’enregistrement continue.
  - Lorsqu’une ligne qui n’est pas un porteur est rencontrée avant le `}` fermant — une instruction hôte, une ligne telle que `.subckt`, `.ends` ou `.include`, une ligne entièrement vide, ou une ligne de commentaire qui n’est pas écrite comme un porteur (2.2) — ou que le fichier se termine, l’enregistrement est incomplet et échoue dans son ensemble. Ainsi, un enregistrement ne franchit jamais la frontière d’un sous-circuit ou d’une inclusion. Pour laisser une ligne vide à l’intérieur d’un enregistrement, écrire une ligne contenant uniquement `*`.
  - Lorsqu’une entrée `@RNL` apparaît hors des chaînes et commentaires avant la fin de l’enregistrement, l’enregistrement précédent est incomplet et échoue dans son ensemble, et la lecture reprend à la nouvelle entrée. Un `}` manquant ne fait ainsi perdre qu’un seul enregistrement.
  - Un enregistrement incomplet est une erreur, et le diagnostic indique que l’enregistrement est incomplet ainsi que l’endroit où il a été interrompu. Une ligne vide de trop fait échouer tout un enregistrement ; cela arrive facilement et ne doit pas passer sous silence (7.6).
  - Chaque ligne de commentaire à l’intérieur d’un enregistrement est du RNL. Une ligne de remarques ordinaires intercalée, telle que `* the next line is for M2`, est lue comme du texte libre, et c’est une erreur si elle contient des caractères qui ne peuvent pas être du texte libre. Les remarques à l’intérieur d’un enregistrement s’écrivent dans `/* */` (3.4).
  - RNL ne limite pas la longueur des lignes ; les limites de l’hôte relèvent de l’adaptateur. Une instruction longue peut être coupée sur plusieurs lignes aux espaces blancs ; les jetons et les chaînes ne peuvent pas être coupés (3.5). Les recommandations pour les générateurs figurent en 2.2.
- Un jeton sur la même ligne après la fin d’un enregistrement est une erreur : ces jetons échouent, et l’enregistrement lui-même prend effet normalement. Un nouvel `@RNL` fait exception, si bien qu’une même ligne peut contenir plusieurs enregistrements.
- Les porteurs de commentaire, l’entrée `@RNL` et l’en-tête d’enregistrement ne dépendent pas de la version du langage, et les versions ultérieures ne les modifient pas (16.1). Les accolades du corps sont appariées comme ci-dessus dans la présente version. Les champs de l’en-tête d’enregistrement figurent en 2.8.

### 2.5 Enregistrements désactivés et texte hôte

- Une ligne de commentaire hors des enregistrements qui ne contient aucun `@RNL` est un commentaire hôte et est ignorée dans son ensemble.
- **Enregistrements désactivés.** Dans une ligne de commentaire, un `/*` qui n’est précédé que d’espaces blancs ou de commentaires, ou qui vient juste après un enregistrement, commence un commentaire qui s’étend jusqu’à `*/`. Un `@RNL` à l’intérieur n’est pas un enregistrement.
- Un `/*` au milieu du texte d’une ligne de commentaire, comme dans `models/*.lib`, n’est que du texte hôte.
- **Fin d’un commentaire hôte.** Les porteurs sont des commentaires sur ligne entière (2.2) : un commentaire hôte se termine à la fin de sa ligne, et rien à l’intérieur de la ligne ne peut le terminer plus tôt, si bien que la fin d’un commentaire hôte n’entre pas en conflit avec le texte RNL.

### 2.6 Niveaux

- Les niveaux sont découpés par le texte ; leur hiérarchie figure en 6.1 :
  - Le niveau de fichier du fichier principal : ce qui est écrit dans le fichier principal en dehors de tout `.subckt … .ends`. C’est le niveau supérieur du circuit.
  - Le niveau de fichier de chaque fichier inclus. Les sections de `.lib` sont traitées de la même façon.
  - Chaque `.subckt … .ends`.
  - Le niveau de fichier de chaque fichier Verilog-A que la netlist importe, et chaque module qu’il contient (2.9).
- Un enregistrement appartient au niveau dans lequel il est écrit (Principes 2.5).
- **Ordre entre fichiers.** Un fichier inclus est un niveau à part entière, au-dessous du niveau qui l’inclut (6.1). L’endroit où la ligne d’inclusion est écrite n’affecte donc pas l’ordre, contrairement à USE-PACKAGE (16.2), et les fichiers inclus côte à côte ne s’affectent pas l’un l’autre. Au sein d’un même niveau, l’ordre est celui du texte.
- **scope.** Un enregistrement peut désigner un niveau cible par `@RNL(scope="name")`. Le nom est le nom d’un sous-circuit et désigne la définition de ce sous-circuit. Dans le fichier suivant, le dernier enregistrement est écrit dans le fichier principal et s’applique au sous-circuit `ota` :

  ```spice
  * A record written outside its subcircuit
  * @RNL(type="header") { @VERSION("0.4")}
  .model nch nmos level=1
  .subckt ota inp inn out tail vss
  M1 x inp tail vss nch
  M2 out inn tail vss nch
  .ends ota
  X1 vip vin vo vt 0 ota
  * @RNL(scope="ota") { #M1 @PLACE(10, 20)}
  .end
  ```

  - Chaque sous-circuit n’a qu’un seul schéma (6.1), donc scope désigne la définition, et non une instance.
  - scope ne descend que vers le bas : la cible doit être un sous-circuit que ce niveau contient dans son texte, c’est-à-dire défini dans ce fichier, ou dans un fichier que ce niveau inclut directement ou par des inclusions successives. scope ne s’écrit pas à l’intérieur d’un sous-circuit : le RNL d’un sous-circuit ne porte que sur lui-même. Un `scope` écrit dans un enregistrement à l’intérieur d’un sous-circuit est une erreur et est traité comme non écrit (2.8). Lorsque la cible est introuvable ou ne se situe pas au-dessous de ce niveau, c’est une erreur, et les instructions de l’enregistrement, qui n’ont aucun niveau dans lequel être résolues, échouent toutes (chapitre 7).
  - Toutes les références de l’enregistrement sont résolues dans le niveau cible. Ses instructions sont versées dans le niveau cible, et les noms qu’elles lient y sont publiés et partagés avec les enregistrements propres du niveau cible (6.7) ; lier un nom que le niveau cible lie déjà à un autre objet est une erreur, et le lier à nouveau au même objet est autorisé (6.6). scope n’affecte que son propre enregistrement et ne modifie pas la netlist hôte.
  - Ordre : les instructions que scope apporte dans un sous-circuit viennent après le propre texte de celui-ci ; lorsque plusieurs endroits ciblent le même sous-circuit par scope, les plus externes viennent plus tard (8.3, 8.5). Ce qui est écrit explicitement à l’extérieur supplante ce qui est écrit à l’intérieur, tout comme la priorité d’une utilisation externe supplante son corps (14.8).
  - La sortie du solveur et les modifications que les utilisateurs apportent dans un éditeur ne doivent pas utiliser scope ; elles sont écrites dans le texte propre du sous-circuit (chapitre 17). scope est uniquement destiné aux instructions que les auteurs choisissent d’écrire à l’extérieur.

### 2.7 Fichiers de paquet

Un paquet peut être écrit sous la forme d’un fichier de paquet distinct, que USE-PACKAGE recherche (16.2). Un fichier de paquet est un fichier RNL pur ; son extension recommandée est `.rnl`, et il est écrit en UTF-8. Ce n’est pas une netlist et il n’est pas chargé par `.include`.

- Un fichier de paquet est **lu** comme les porteurs d’une netlist (2.2, 2.4), à ceci près que chaque ligne est un porteur, et que le caractère de commentaire et l’espace qui le suit sont tous deux vides :
  - un `@` en début de ligne compte comme ayant une espace avant lui (2.3), de sorte que `@RNL` et les mots-clés peuvent commencer en colonne 1 ;
  - une ligne vide est une ligne de commentaire vide, et un enregistrement continue au-dessus d’elle ;
  - le texte en dehors des enregistrements est constitué de remarques et est ignoré, comme les commentaires hôtes (2.5) ; `/* */` fonctionne normalement ;
  - la ligne 1 est lue : un fichier de paquet n’est pas une netlist et n’a pas de ligne de titre (2.2).
- Il contient des enregistrements `@RNL(…) {…}`, analysés dans l’ordre de 16.1. Le premier enregistrement est l’enregistrement d’en-tête sans `language`, qui contient VERSION et PACKAGE ; un fichier de paquet peut également comporter un enregistrement d’en-tête pour chaque langage (2.8). Pour ce qu’un paquet peut contenir, y compris les références à d’autres paquets, voir 16.2 ; les paquets d’alias sont aussi des fichiers de paquet (16.5).

Voici le texte complet d’un fichier de paquet, sans préfixes de commentaire. La troisième ligne est en dehors des enregistrements et est une remarque. Le paquet utilise `$NMOS` sans référencer le paquet standard ; il le trouve donc dans le paquet standard en vigueur là où il est utilisé, ou, lorsqu’il est vérifié isolément, dans celui qu’ajoute un réglage de l’outil (16.2, 16.3) :

```rnl-file
@RNL(type="header") { @VERSION("0.4"); @PACKAGE(name="acme-style", version="1.2")}

ACME house style: NMOS symbol and its rule.
@RNL {
  $NMOS_ACME = @DEFINE-SYMBOL {
    $NMOS;
    ? @TEXT("${name}", key="name") => @PLACE(-2.5, 1.5)
  };
  # @SELECT-INSTANCE(type="nmos", recursive=true) => @SET-SYMBOL $NMOS_ACME
}
```

### 2.8 En-tête d’enregistrement

Les champs entre les parenthèses de `@RNL(…)` s’appliquent à chaque instruction de l’enregistrement, et tous sont donnés par nom. Les noms de champs et les valeurs de `type` s’écrivent dans l’orthographe standard ; ils sont sensibles à la casse et ne peuvent pas avoir d’alias (3.6, 16.5).

```rnl
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4")}    /* the first record of the file: read as version 0.4 (16.1) */
* @RNL { %dp1 = #M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N}    /* no type: "authored" */
* @RNL(type="generated", priority="high", author="rnl-solver 0.3", hash="sig1:9f3a62c1") {
*   #M1 @PLACE(12, 20); #M2 @PLACE(28, 20)
* }
* @RNL(author="rnl-editor 0.1") { #M2 @PLACE(x=30, priority="high")}
```

| Champ | Signification | Affecte l’interprétation |
|---|---|---|
| `type` | La sorte d’enregistrement : `"authored"` (par défaut), `"generated"`, `"header"` | oui |
| `scope` | Le sous-circuit cible (2.6) | oui |
| `priority` | La priorité par défaut des instructions dans l’enregistrement ; pour AS et ATTACH, uniquement `"disabled"` (14.8) | oui |
| `author` | L’auteur, l’outil et sa version, la version d’un paquet, etc. ; toute chaîne | non |
| `hash` | Une empreinte de l’entrée du solveur (chapitre 17) | non |
| `numbering` | La vérification des numéros qu’utilise l’enregistrement (5.9) ; uniquement avec `type="generated"` | oui |
| `modified` | L’heure de la dernière modification, une chaîne ISO 8601 | non |
| `language` | Le langage de netlist sur les sections duquel agit l’enregistrement : `"spice"` ou `"spectre"` (ci-dessous) | oui |

- **type** : les valeurs sont fixes, et chacune a un comportement défini.
  - `"authored"`, par défaut : écrit à la main, par un éditeur ou par une importation.
  - `"generated"` : le résultat du solveur. Lorsque le solveur résout, le niveau est interprété sans les enregistrements portant `type="generated"` : ils n’ajoutent ni objets, ni noms, ni faits, et ne contraignent pas le résultat. La façon dont le solveur les utilise par ailleurs dépend de lui. Ensuite il les remplace en bloc, quel qu’en soit l’auteur et qu’il comprenne ou non leurs instructions : la règle de 16.4 selon laquelle un outil garde tels quels les instructions qu’il ne comprend pas lorsqu’il réécrit ses propres enregistrements ne s’applique pas à ce remplacement. Ce qui doit être retenu est écrit dans un enregistrement d’un autre type. Les fils qu’un éditeur redessine pendant qu’un composant est déplacé sont également des résultats du solveur (14.8, chapitre 17). `type` n’implique aucune priorité : le solveur écrit `priority="high"` explicitement.
  - `"header"` : les réglages du fichier (16.1). L’enregistrement d’en-tête peut être omis. Lorsqu’il est écrit, il se trouve au niveau du fichier, non à l’intérieur de `.subckt` ou d’un module Verilog-A (2.9), et ne porte pas de `scope` ; les enregistrements d’en-tête sont les premiers enregistrements du fichier, au plus un sans `language` et un pour chaque langage (voir `language` ci-dessous). Il contient VERSION (16.1), les alias d’ALIAS et les paquets d’alias d’USE-ALIASES (16.5), les noms de bornes et types de DEVICE (6.12), les paquets à la racine d’USE-PACKAGE (16.3) et les réglages de SOLVER-CONFIG (14.10) ; un fichier de paquet y écrit également PACKAGE (2.7, 16.2). L’enregistrement d’en-tête lui-même est lu en orthographe standard, sans alias (16.5). Un enregistrement d’en-tête écrit après d’autres enregistrements, ou un deuxième sans `language` ou pour le même langage, est une erreur et est ignoré dans son ensemble ; les enregistrements déjà lus ne sont pas réexaminés. Une section de `.lib` qui est lue compte comme un fichier (2.6) : son enregistrement d’en-tête est le premier enregistrement de la section. Un outil écrit l’enregistrement d’en-tête avec un marqueur de commentaire du mode de langage dans lequel il l’écrit (2.2). Lorsque la première instruction d’un fichier vient après une ligne `simulator lang=`, l’enregistrement d’en-tête peut aller juste après cette ligne, dans le mode du reste du fichier, ou avant elle, par exemple à la ligne 2, dans le mode dans lequel commence le fichier ; dans les deux cas il est le premier enregistrement du fichier.
- **language** fait que l’enregistrement n’agit que sur les objets des sections de ce langage.
  - **Sections** : l’adaptateur divise chaque fichier en sections par mode de langage, d’après `simulator lang=` (2.2). Les valeurs sont `"spice"` pour la famille SPICE, HSPICE et les autres dialectes de SPICE compris, et `"spectre"` ; des versions ultérieures pourront ajouter un langage incompatible avec ceux-ci. Les valeurs sont sensibles à la casse.
  - **Portée nominale** : les niveaux qu’atteint un enregistrement sont les mêmes que sans `language` : un enregistrement écrit dans un niveau atteint ce niveau et les niveaux inférieurs, et un enregistrement dans un paquet atteint les niveaux qui utilisent le paquet (16.2). Parmi ce qu’il atteint, un enregistrement portant `language` n’agit que sur les objets situés dans les sections de son langage ; un enregistrement sans `language` agit sur toutes les sections, donc dans un fichier Spectre il agit aussi sur une section SPICE.
  - **Dans quelle section se trouve un objet** : une instance et ses bornes, la section de la ligne de l’instance ; un port d’un sous-circuit (`#:port`), la section de la ligne qui commence le sous-circuit ; un net, chaque section comportant une ligne qui l’écrit, de sorte qu’un net écrit dans les deux sortes de section est atteint par les enregistrements des deux langages ; un objet créé par RNL (une utilisation, un point, un graphique ou un texte), la section de l’objet hôte auquel il est attaché ou qui le remplit, telle qu’une masse attachée à `&vss` ; le niveau lui-même, et ce qui ne s’attache à aucun objet hôte, chaque langage qui possède une ligne dans le niveau.
  - **Instructions traversant des sections** : une instruction dont les objets se trouvent en partie hors des sections du langage de l’enregistrement, telle qu’une contrainte, un fil ou un GROUP, est sans effet dans son ensemble. Une référence à un objet situé hors de ces sections est sans effet et n’est pas une erreur, et ce qui en dépend est également sans effet ; un nom introuvable est une erreur comme d’habitude, lorsque le niveau comporte des sections de ce langage. Un enregistrement dont le langage n’a aucune section dans ce qu’il atteint n’agit sur rien et n’est pas vérifié.
  - **Les définitions ne sont pas divisées par section** : les noms en `$`, les symboles et les styles sont visibles dans la portée nominale comme d’habitude (6.8) ; seules les instructions qui agissent sur des objets sont filtrées par section.
  - **Enregistrements d’en-tête** : un fichier peut comporter un enregistrement d’en-tête sans `language` et, pour chaque langage, un enregistrement d’en-tête avec lui. Ils sont écrits en premier, l’un après l’autre, celui sans `language` en premier. VERSION, PACKAGE, ALIAS et USE-ALIASES ne s’écrivent que dans l’enregistrement d’en-tête sans `language` ; un enregistrement d’en-tête avec `language` ne contient que USE-PACKAGE et DEVICE, qui n’agissent alors que sur les sections de son langage (6.12, 16.2). Tous les enregistrements d’en-tête prennent effet ensemble. Le même paquet écrit dans deux enregistrements d’en-tête avec des versions différentes ou un contenu en ligne différent est une erreur, car un même niveau peut comporter des sections des deux langages et n’utilise un paquet d’un nom donné qu’une seule fois (16.2) : le USE-PACKAGE de l’enregistrement d’en-tête avec `language` échoue, et celui de l’enregistrement d’en-tête sans `language` prend effet pour chaque section ; entre deux enregistrements d’en-tête avec `language`, le plus tardif échoue.
  - Un enregistrement portant `type="generated"` ne peut pas avoir `language` : le solveur résout tout un niveau et remplace ses enregistrements en bloc. Écrit là, `language` est une erreur et est traité comme non écrit.
  - Une valeur que la présente version ne répertorie pas fait que l’enregistrement n’agit sur rien, avec un avertissement qui nomme la valeur la plus proche ; c’est une exception à la règle ci-dessous selon laquelle un champ connu mal écrit est traité comme non écrit, ce qui ferait agir l’enregistrement sur toutes les sections. Les outils qui écrivent `language` écrivent `@VERSION("0.4")` dans l’enregistrement d’en-tête sans `language` (16.1). Les enregistrements nouvellement écrits par des outils n’ont pas de `language` ; les enregistrements qui en ont un sont gardés tels quels (16.4).
- **Métadonnées** : `author`, `hash` et `modified` n’affectent pas l’interprétation ; le dessin est identique lorsqu’ils sont supprimés. Les outils les utilisent pour décider comment traiter un enregistrement (chapitre 17). Ni `type` ni `author` n’affectent l’ordre de résolution (8.3).
- **Erreurs** :
  - Un champ inconnu ne donne qu’un avertissement et est ignoré ; il n’affecte ni l’enregistrement ni les autres champs. Cela laisse place à des extensions ultérieures. Lorsque l’avertissement est émis, il doit nommer le nom de champ le plus proche. Note : en conséquence, un champ mal orthographié qui affecterait l’interprétation (tel que `priorty="low"`) ne reçoit qu’un avertissement, et l’enregistrement prend effet avec la priorité `"normal"` ; les champs de cette sorte ajoutés ultérieurement s’appuient sur la version (16.1) pour signaler aux anciens outils qu’ils sont dépassés.
  - Un champ connu mal écrit (une mauvaise valeur, `numbering` dans un enregistrement dont `type` n’est pas `"generated"` ou qui comporte `scope`, ou le champ écrit deux fois) est une erreur, et le champ est traité comme non écrit : `priority` revient à `"normal"`, `type` à `"authored"`, et les métadonnées sont écartées. L’exception est un `scope` dont la cible est introuvable : les instructions de l’enregistrement n’ont aucun niveau dans lequel être résolues et échouent toutes (2.6). Un champ donné par position est une erreur et est ignoré.
- Les enregistrements n’ont pas d’identifiants. Les outils retrouvent leurs propres enregistrements par `type`, `author` et les clés des instructions (5.8) ; un objet est référencé depuis un autre enregistrement par son nom, ou, depuis l’enregistrement du solveur, par son numéro (5.9). Les autres remarques s’écrivent en commentaires.
- Un résultat du solveur n’est pas une proposition qui devrait être adoptée : il prend effet directement, et un résultat périmé ne donne qu’une indication (chapitre 17).

### 2.9 Fichiers Verilog-A

Un sous-circuit donne à ses instances leur symbole de bloc (block symbol) (9.2.6). Un module Verilog-A que la netlist importe peut faire de même, à l’aide d’enregistrements écrits dans le fichier Verilog-A lui-même :

```verilog
// @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4")}
`include "disciplines.vams"
module vco(vin, out, vdd, vss);
  inout vin, out, vdd, vss;
  electrical vin, out, vdd, vss;
  parameter real f0 = 1e9;
// @RNL {
//   %vco = @DEFINE-SYMBOL(box=[-3, -3, 3, 3]) {
//     @RECTANGLE {(-2, -2) (2, 2)};
//     (-3, 0) @PIN(terminal="vin", direction="left");
//     (3, 0) @PIN(terminal="out", direction="right");
//     (0, 3) @PIN(terminal="vdd", direction="up");
//     (0, -3) @PIN(terminal="vss", direction="down");
//     ? @TEXT("${name}", key="name") => @PLACE(0, 2.5);
//     ? @TEXT("f0=${f0}", key="value") => @PLACE(0, -2.5)
//   };
//   # @SET-SYMBOL %vco
// }
  analog begin
    V(out) <+ 0;
  end
endmodule
```

Une netlist Spectre qui écrit `ahdl_include "vco.va"` et `I1 (ctl out vdd 0) vco f0=2e9` dessine I1 avec ce symbole, sans aucun RNL qui lui soit propre.

- **Quels fichiers.** Les fichiers sources Verilog-A que la netlist importe : avec `ahdl_include` dans Spectre, dans l’un ou l’autre mode de langage, et avec `.hdl` dans HSPICE. Un tel fichier est lu là où il est importé, comme l’est un fichier inclus (6.1). Avec `ahdl_include … -master name` de Spectre, le module est connu sous ce nom dans la netlist, et son symbole de bloc va aux instances de ce nom. Un modèle qu’un hôte importe déjà compilé, tel qu’un fichier `.osdi` de ngspice, n’a pas de texte à lire ; ses instances ont le symbole de bloc par défaut, avec une broche pour chaque nœud de leurs lignes (9.2.6), et un niveau supérieur peut leur donner un symbole par une règle.
- **Porteurs.** Commentaires sur ligne entière `//`, selon les règles de 2.2 : `//` en colonne 1, suivi de rien, uniquement d’espaces blancs, ou d’une espace et du contenu. Les commentaires de fin de ligne et les commentaires `/* */` ne sont pas lus, et `@RNL` à l’intérieur produit un avertissement. La ligne 1 est lue : un fichier Verilog-A n’a pas de ligne de titre. Le texte que le préprocesseur de Verilog-A peut omettre, entre `` `ifdef `` ou `` `ifndef `` et son `` `endif ``, n’est pas lu, et `@RNL` à cet endroit produit un avertissement, comme pour les régions de 2.2 que l’adaptateur ne peut pas classer.
- **Niveaux.** Le texte extérieur à tout `module … endmodule` (et `macromodule … endmodule`) est le niveau de fichier du fichier Verilog-A. Chaque module est un niveau à part entière, un niveau de module (module level), au-dessous de ce niveau de fichier, lequel se trouve au-dessous du niveau dont le texte importe le fichier (6.1).
  - Le niveau de fichier ne contient que les enregistrements d’en-tête (2.8), écrits avant le premier module. Tout autre enregistrement à cet endroit est une erreur et échoue.
  - Un niveau de module n’a ni circuit ni schéma : rien n’y est dessiné ni résolu, et il ne comporte aucun enregistrement du solveur.
- **Ce que contient un niveau de module.** DEFINE-SYMBOL et DEFINE-STYLE, ainsi qu’une instruction `# @SET-SYMBOL` avec un symbole, où `#` est le module lui-même. Ce SET-SYMBOL donne le symbole de bloc du module, avec tout ce que dit 9.2.6 du symbole de bloc d’un sous-circuit : il s’applique partout où le module est instancié, et sa place dans la file des symboles est la même (8.3). Tout autre élément écrit dans un niveau de module est une erreur et échoue : références hôtes, contraintes, PLACE, ORIENT, WIRE, un TEXT hors du corps d’un symbole, règles autres que `# @SET-SYMBOL`, AS et ATTACH, assertions, `scope`, DEVICE et SOLVER-CONFIG. USE-PACKAGE ne s’écrit que dans l’enregistrement d’en-tête du fichier.
- **Bornes.** Les bornes d’une instance d’un module sont les nœuds que l’hôte lit sur sa ligne, dans l’ordre des ports de la liste de ports du module, et elles portent le nom des ports. Verilog-A compare les noms en tenant compte de la casse, donc les noms des ports en tiennent compte également.
  - Un port vectoriel (vector port) représente une borne pour chacun de ses bits, de la borne gauche de sa plage déclarée jusqu’à sa borne droite, car un vecteur est connecté par position : le port `inout [3:0] d` reçoit les quatre nœuds suivants de la ligne, en tant que `d[3]`, `d[2]`, `d[1]` et `d[0]`, et `inout [0:2] e` en reçoit trois, en tant que `e[0]`, `e[1]` et `e[2]`. Ce sont leurs noms de borne, écrits avec un indice dans les références (6.11), comme `#I1:d[3:0]`, et dans `terminal=` de PIN sous forme de chaînes, comme une broche de bus avec `terminal=["d[3]", "d[2]", "d[1]", "d[0]"]` (9.2.4).
  - Un module ne donne aucun type à ses instances (6.12) ; DEVICE dans la netlist peut leur en donner un, comme pour d’autres instances.
- **Versions et paquets.** L’enregistrement d’en-tête d’un fichier Verilog-A est celui d’un fichier inclus : sans lui, ou sans VERSION, le fichier prend la version du fichier qui l’importe (16.1), et son USE-PACKAGE ne s’applique qu’au fichier et à ses modules (16.2).
- **Écriture.** Un outil écrit `//` dans un fichier Verilog-A, et uniquement l’enregistrement d’en-tête ainsi que les enregistrements des symboles de bloc ; il n’y écrit aucun résultat du solveur (chapitre 17).

<!-- translated from en/ch03.md sha256:9148069f1090 -->
## 3 Structure lexicale

### 3.1 Caractères

- Les jetons sont composés de caractères ASCII.
- Les caractères non ASCII n’apparaissent que dans les chaînes (y compris les noms entre guillemets), le texte libre, les commentaires et les alias, ainsi que dans les noms de rôles et de paramètres sans guillemets. En dehors du texte libre et des commentaires, un mot non ASCII sans guillemets n’apparaît qu’après `@` et à l’intérieur de parenthèses ou de crochets : après `@`, il doit s’agir d’un alias déclaré ; à l’intérieur des parenthèses, il s’agit d’un alias, ou d’un nom de rôle ou de paramètre lui-même (3.6, 16.5).
- **Encodage.** L’adaptateur hôte décode le texte selon les règles de l’hôte ; RNL travaille sur des caractères. Un outil qui écrit du RNL ne doit modifier ni l’encodage ni les fins de ligne du fichier hôte et ne doit pas ajouter d’indicateur d’ordre des octets (BOM) ; il ne doit pas écrire un caractère que l’encodage de l’hôte ne peut pas représenter, et émet un avertissement à la place. Les nouveaux fichiers devraient utiliser UTF-8.
- **Caractères non ASCII dans les commentaires hôtes.** Les adaptateurs doivent indiquer les hôtes pour lesquels les caractères non ASCII dans les commentaires ont été vérifiés ; pour les autres hôtes, un caractère non ASCII donne une indication.

  Note : la prise en charge varie selon les hôtes. ngspice (à partir de la version 42), WRspice (sauf dans le titre), Xyce et les versions récentes de LTspice acceptent les caractères non ASCII dans les commentaires ; la documentation de HSPICE, PSpice et Eldo ne le précise pas ; le manuel de Spectre décrit les netlists comme des fichiers ASCII ; QSPICE utilise Windows-1252. Une autre raison de ne pas ajouter d’indicateur d’ordre des octets : les anciennes versions de ngspice le collent au premier jeton d’un fichier inclus, et WRspice arrête la lecture lorsque la ligne 1 commence par un tel indicateur.

### 3.2 Jetons

| Jeton | Forme |
|---|---|
| Mot-clé | `@` suivi d’un nom en majuscules, tel que `@PLACE` ; ou un alias déclaré (16.5) |
| Référence | Un préfixe suivi d’un nom, éventuellement avec des bornes et des sous-objets, tel que `#M1`, `#M1:d`, `%dp1::left` (chapitre 6) ; ou `#`, `?` ou `&` seuls (6.2). Un nom hôte ou un nom de borne peut être suivi directement d’un indice, tel que `&D<3>`, `&D<7:0>` (6.11). Une référence à un ensemble peut être suivie directement de places de membres, telles que `%a{1}`, `?ref{2:end}` (5.2.3). Dans un enregistrement du solveur, également `%#` suivi d’un entier non signé, un numéro, tel que `%#3` (5.9) |
| Point | `(x, y)`, ne touchant ni un mot-clé ni un objet : après un espace blanc, un commentaire, `{`, `;`, `=>` ou `=` (3.8) |
| Parenthèses | Les `(` `)` des paramètres et des rôles, touchant le mot-clé ou l’objet (3.5) |
| Accolades | Les `{` `}` des ensembles |
| Séparateurs | `;` (sépare les membres), `=>` (chaînages et nommage), `=` (nommage en début de membre) |

À l’intérieur des parenthèses et crochets (les `()` de paramètres, rôles et points, et les `[]` de tableaux) figurent également des chaînes, des nombres, `,`, `=`, `_` et des noms (noms de paramètres, noms de rôles, `true`, `false`), ainsi que, dans le corps d’une définition générale, des paramètres génériques `?name` (6.10). À l’intérieur des `<>` et `[]` des indices figurent uniquement des entiers non signés, `,` et `:` (6.11) ; à l’intérieur des accolades des places de membres figurent uniquement des entiers non signés, `end`, `-`, `,` et `:` (5.2.3).

### 3.3 Texte libre

- En dehors des parenthèses et crochets, tout caractère peut être du texte libre à l’exception de ceux énumérés ci-dessous. Cela inclut tous les caractères non ASCII (tels que la ponctuation chinoise `：`, `、`, `；`, `（`, `）`) ainsi que `-`, `:`, `'`, `!`, `+`, `/`, `*` et `|`. Les exceptions sont :
  - `@`, `#`, `&`, `%`, `$`, `?`, `"`, `;`, `=`, `{`, `}`, `(`, `)` : en dehors des parenthèses et crochets, ils commencent des jetons ou leur appartiennent (3.2). `&` seul, sans nom après lui, est le nœud vide (empty node) (6.3). `?` commence un paramètre générique (6.10) ; un `?` isolé désigne, dans un corps de symbole, l’objet dessiné avec le symbole (13.3), et dans le corps d’une définition générale, l’utilisation en cours de développement (9.4) ;
  - `[`, `]`, `<`, `>` : les crochets des tableaux et des indices (3.7, 6.11) ;
  - `/*` : cela commence un commentaire (3.4).
- Des règles de position maintiennent l’unicité du sens :
  - Un `:` ou `::` juste après une référence et suivi d’un nom ou d’un guillemet est une borne ou un sous-objet, comme d’habitude (6.4, 6.5). Lorsqu’aucun nom ne suit, le `:` est du texte libre, comme dans `Input pair: %dp1 = …` et `#M1: the left one`.
  - Un `<` ou `[` juste après un nom est un indice (6.11).
  - Un `{` juste après une référence, sans espace blanc, commence des places de membres (5.2.3) ; après un espace blanc, il commence un ensemble.
  - Dans le nom d’un mot-clé, un `-` n’appartient au nom que lorsqu’une lettre le suit (mots-clés : 3.6). `@GROUP- the pair` est GROUP suivi de texte libre.
- Le texte libre peut être écrit entre deux jetons quelconques en dehors des parenthèses et crochets, ainsi qu’au début d’un membre.
- **Le texte libre fait partie de l’instruction.** Une référence qui y est écrite n’est pas une explication mais un objet de l’instruction : `Unlike #M2, #M1 @PLACE(x=0)` place également M2 en x = 0. Un texte destiné uniquement à expliquer s’écrit dans un commentaire (3.4), ou comme un membre distinct, séparé du membre suivant par `;` : un membre ne contenant que du texte libre est un membre vide et est ignoré (4.1).
- La norme ne vérifie pas le texte libre, et le texte libre ne modifie pas le sens (Principes 2.8) : `do not move #M2 @PLACE(x=30)` déplace tout de même M2. Une implémentation peut émettre un avertissement qui lui est propre pour des mots tels que des négations (7.6).
- De la même façon, dans un `#M1.d` erroné, le `.d` est du texte libre, et la référence est M1 lui-même, non son drain (un avertissement est émis, voir ci-dessous).
- Il n’y a pas de texte libre à l’intérieur des parenthèses et crochets.
- **Avertissements.** Les erreurs suivantes pourraient passer inaperçues ; elles produisent un avertissement (7.6) :
  - Un mot dans le texte libre qui est le nom d’une instance hôte de ce niveau, un nom RNL visible (sans son préfixe) ou le nom d’un mot-clé, tel que `M3` dans `#M1 #M2 M3 @ALIGN("y")` ou `WIRE` dans `WIRE {…}` : il manque probablement un `#`, `%`, `$` ou `@`. Les noms de nets sont souvent des mots courants (tels que in et out) et ne sont pas vérifiés.
  - Une référence suivie directement de `.` et d’un nom, telle que `#M1.d` : les bornes s’écrivent avec `:`.
  - Une référence suivie directement de `-`, `!`, `'`, `+`, `/`, `*`, `\`, `^`, `~` ou `|`, telle que `&vdd!` ou `#X1-a` : le nom contient probablement ce caractère et nécessite des guillemets, comme dans `&"vdd!"` (3.6) ; sans eux, la référence désigne silencieusement `vdd` ou `X1`.
  - `|` dans le texte libre : un chaînage s’écrit `=>` (4.2 point 3).
  - `:` ou `::` juste après une référence et suivi d’un mot non ASCII, tel que `%c1::中点` : un nom comportant des caractères non ASCII après `::` nécessite des guillemets, comme dans `%c1::"中点"` (3.6) ; sans eux, le `::` et le mot sont du texte libre, et la référence désigne `%c1`.
- La plupart des autres fautes de ce genre sont déjà des erreurs : il n’y a pas de texte libre entre parenthèses, donc `@ORIENT(MY)` est une erreur ; `#M1 @PLACE 1 2` ne correspond pas à la signature ; `#M1 (in_p)` est une erreur, car une parenthèse après un espace blanc commence un point et `(in_p)` n’est pas un point (3.5) ; et le `>` dans `R1 -> R2` ne peut pas être du texte libre.
- Les guides d’écriture à l’intention des auteurs sont générés à partir de la liste de mots (5.3). Les enregistrements écrits par des solveurs et des éditeurs ne doivent pas contenir de texte libre.

### 3.4 Commentaires

`/* … */` peut être écrit partout où un espace blanc est autorisé, y compris à l’intérieur des parenthèses et crochets. Les commentaires ne s’imbriquent pas, et tout caractère peut y être écrit. Une explication contenant des références, ou des caractères tels que `@`, `#` et `;`, s’écrit dans un commentaire (3.4).

### 3.5 Espaces blancs et adjacence

- L’espace blanc est l’espace, la tabulation et le saut de ligne. Les jetons ne peuvent pas être coupés d’une ligne à l’autre, et les chaînes ne peuvent pas s’étendre sur plusieurs lignes.
- Les espaces blancs et les commentaires ne sont pas autorisés : entre un préfixe et son nom, ni entre `%#` et son numéro ; de part et d’autre de `:` et `::` ; entre un nom et son indice (6.11) ; entre une référence et le `{` de ses places de membres (5.2.3) ; entre un mot-clé et ses parenthèses de paramètres, et entre `@RNL` et la parenthèse de son en-tête d’enregistrement (2.4) ; entre un objet et ses parenthèses de rôle.
- Un `(` après un espace blanc ou un commentaire commence un point (3.8). Ainsi `#M1 @PLACE (10, 20)` est PLACE suivi d’un point, et PLACE n’a pas de paramètres : une erreur. `#M1 (in_p)` est M1 suivi d’un point qui n’est pas écrit correctement : également une erreur. Le diagnostic suggère de supprimer l’espace blanc.
- Un `<` ou `[` juste après un nom, entre guillemets ou non, commence toujours un indice (6.11). Les points s’écrivent `(x, y)`, ce qui n’entre pas en conflit avec les indices ; pour écrire un point après un nom, séparez-les par un espace blanc, comme dans `&D (1, 2)`. Les espaces blancs et les commentaires peuvent s’écrire à l’intérieur d’un indice, comme dans `&D<1, 2:3>` (6.11).
- Un `.` juste après un mot-clé est du texte libre, tel que le point final dans `{#M1 #M2} @GROUP.` ; `@BUNDLE.LOCK` est BUNDLE suivi du texte libre `.LOCK` (16.4).

### 3.6 Noms

- **Les noms sans guillemets** sont composés d’une ou plusieurs lettres ASCII, chiffres et traits de soulignement, pris par correspondance la plus longue.
- **Les noms hôtes** (après `#` et `&`) et les noms de bornes suivent les règles du dialecte hôte (Principes 2.1) : quels noms sont valides, si la casse importe et si un nom peut commencer par un chiffre, comme dans `&0`. SPICE est insensible à la casse ; Spectre y est sensible. Un nom comportant d’autres caractères s’écrit entre guillemets, comme `#"M1<0>"`, `&"vdd!"`, `#XU1:"in+"` ; le texte entre guillemets s’écrit comme dans les chaînes (3.7). Un nom peut être suivi directement d’un indice : `#M1<0>` est `#"M1<0>"`, et `&D<7:0>` est une liste de nets (6.11). Un `<` ou `[` à l’intérieur de guillemets fait partie du nom et n’est pas analysé.
- **Noms RNL** (les noms après `%`, `$` et `?`, les noms de rôles, et les noms de paramètres des définitions générales) :
  - Ils sont sensibles à la casse : `$DIFFERENTIAL_PAIR` et `$differential_pair` sont deux noms différents.
  - Après `%`, `$` et `?`, un nom peut commencer par un chiffre, comme dans `%1st`. Aux endroits sans préfixe (`(role)` et `parameter=`), un nom sans guillemets se compose de lettres ASCII, chiffres, traits de soulignement et de tout caractère non ASCII, et ne commence pas par un chiffre ASCII, afin de ne pas ressembler à un nombre : `#M1(正输入)`, `@AS(间距=3) $X`. Ces deux endroits se trouvent à l’intérieur de parenthèses, où il n’y a pas de texte libre (3.3), de sorte qu’aucun autre mot n’est inclus dans le nom.
  - Ils peuvent être mis entre guillemets, avec le même sens : `$"a"` est `$a` ; les guillemets ne servent qu’à échapper. Les noms comportant d’autres caractères, ainsi que les noms non ASCII, s’écrivent entre guillemets, tels que `%"差分对"`, `%"input pair"`, `?"正输入"`, `%dp::"正输入"`. Aux places de rôle et de nom de paramètre, les noms non ASCII peuvent être sans guillemets (point ci-dessus), mais une espace ou une autre ponctuation ASCII nécessite toujours des guillemets, comme dans `#M1("input pair")`. Aux places de rôle et de nom de paramètre, un nom entre guillemets est toujours le nom lui-même, jamais un alias. Cela ne s’applique pas aux noms en `$` ni aux noms après `::`, dont les alias non ASCII s’écrivent de toute façon entre guillemets (16.5).
  - Ils sont comparés caractère par caractère tels qu’ils sont écrits, sans normalisation Unicode. Un nom vide est une erreur.
  - Ils ne prennent pas d’indice (6.11) : un `<` ou `[` juste après eux est une erreur, comme dans `%p<1, 2>`. Un point s’écrit `(1, 2)`, séparé du nom par un espace blanc.
- **Alias** (16.5) : ils sont déclarés dans l’enregistrement d’en-tête ou proviennent de paquets d’alias. Les alias de mots-clés et de mots standards peuvent contenir des lettres non ASCII ; ils s’écrivent sans guillemets uniquement après `@` et à l’intérieur des parenthèses. Un alias d’un nom en `$` s’écrit comme un nom en `$`, entre guillemets lorsqu’il est non ASCII, comme dans `$"接地补位集"`. Les noms sans guillemets après `#`, `&`, `%`, `$`, `?` et `::` sont uniquement ASCII, comme ci-dessus, si bien que `#M1和#M2` est `#M1`, le texte libre `和` et `#M2`.
- **Les mots-clés** sont uniquement en majuscules, avec les mots joints par `-`, tels que `@AS` et `@SET-SYMBOL` ; les majuscules les rendent aussi faciles à repérer dans le texte libre. `@at` est un mot-clé inconnu, à moins que l’enregistrement d’en-tête ne le déclare comme alias (16.5).
- **Les mots standards** sont sensibles à la casse et écrits comme la norme les orthographie : noms de paramètres, noms de champs de l’en-tête d’enregistrement, noms de sous-objets, noms de conditions de sélection et valeurs d’énumération (telles que `"x"`, `"low"`, `"MY"`). Les valeurs qui nomment des choses de l’hôte (noms dans les conditions de sélection, noms de modèles, noms de nets) suivent les règles de l’hôte. D’autres orthographes peuvent être acceptées au moyen d’alias (16.5).
- **`true` et `false`** sont réservés et uniquement en minuscules. Ils ne peuvent pas être des noms de rôles ni des noms de paramètres.

### 3.7 Littéraux

- **Les nombres** sont décimaux. Ils peuvent avoir un signe et des zéros non significatifs, omettre le zéro avant la virgule décimale ou les chiffres après elle, et avoir un exposant : `0`, `+1`, `01`, `.5`, `1.`, `-2.5E-4`. `01` vaut 1, non octal. Il n’y a ni unités ni suffixes d’ingénierie (`2u` et `10k` sont des erreurs), ni nombres hexadécimaux, ni `NaN` ni `Infinity`. Le nombre de chiffres n’est pas limité. Lorsqu’une implémentation ne peut pas contenir une valeur, l’instruction échoue dans cette implémentation, avec une erreur indiquant qu’il s’agit d’une limite de l’implémentation ; la valeur n’est pas modifiée en silence. Les coordonnées sont comparées avec une tolérance (11.7).
- **Les chaînes** sont écrites entre guillemets doubles et ne peuvent pas s’étendre sur plusieurs lignes. Les caractères entre guillemets sont pris tels quels ; seul `"` lui-même s’écrit `""`, comme dans `"say ""hi"""`. Il n’y a pas d’échappement par barre oblique inversée : `\` est un caractère ordinaire, et il n’y a ni `\n` ni `\uXXXX` ; les caractères non ASCII s’écrivent directement. Un saut de ligne ne peut pas s’écrire dans une chaîne ; un saut de ligne dans un texte s’écrit avec le balisage `\\` (13.6). TEXT possède ses propres commandes à barre oblique inversée (13.6) ; elles constituent la manière dont TEXT lit son texte, et non des échappements de la chaîne. Les chaînes dans l’en-tête d’enregistrement s’écrivent de la même manière ; cela appartient à la forme externe et ne change pas (16.1).
- **Les tableaux** s’écrivent `[value, value, …]`. Leurs éléments sont des littéraux, et les tableaux peuvent être imbriqués.
- **Les valeurs de vérité** sont `true` et `false`.
- Il n’y a pas d’autres littéraux. Les énumérations s’écrivent sous forme de chaînes, telles que `axis="x"` ; il n’y a ni `null`, ni tableau associatif ni valeur construite.
- Il n’y a pas d’expressions : les opérateurs sont des erreurs.

### 3.8 Points

- Un point s’écrit `(x, y)`, où x et y sont des nombres ou `_` (dans le corps d’une définition générale, également un paramètre générique, 6.10). `_` laisse cet axe libre, à décider par les contraintes ou le solveur.
- Écrire un point crée un objet point (5.1).
- Il n’y a pas d’arithmétique sur les points ; les positions relatives s’écrivent sous forme de contraintes (chapitre 14). Dans le corps d’une définition générale, les coordonnées littérales sont des décalages par rapport au point propre de l’utilisation (9.4).
- Le `(` d’un point est précédé d’un espace blanc ou d’un commentaire, ou vient juste après `{`, `;`, `=>` ou `=`. Les parenthèses qui touchent un mot-clé ou un objet contiennent ses paramètres ou son rôle (3.5). Un `<` juste après un nom est un indice dans tous les cas (6.11).

<!-- translated from en/ch04.md sha256:86b9e3072d96 -->
## 4 Syntaxe

### 4.1 Grammaire

La grammaire d’un enregistrement est donnée en EBNF. Chaque règle s’écrit `name = expression ;` ; `|` sépare les alternatives, `[ … ]` est facultatif, `{ … }` se répète zéro fois ou plus, `( … )` regroupe, le texte entre guillemets doubles s’écrit littéralement, et `(* … *)` est un commentaire.

```ebnf
record         = "@RNL" [ parameters ] set ;          (* the bracket touches "@RNL" *)
set            = "{" [ member ] { ";" [ member ] } "}" ;
member         = [ front-naming ] segment { naming } { "=>" segment { naming } } ;
front-naming   = ( "%" rnl-name | "$" rnl-name ) "=" ;
naming         = "=>" ( "%" rnl-name | "$" rnl-name ) ;
segment        = argument { argument } [ call { argument } ]
               | call { argument } ;
call           = keyword [ parameters ] ;          (* the bracket touches the keyword *)
keyword        = "@" keyword-name ;                (* upper case, or a declared alias *)
argument       = object [ "(" role-label ")" ] ;   (* the bracket touches the object *)
object         = reference | placeholder | point | set ;
reference      = "#" [ host-name [ subscript ] ] path
               | "&" host-name [ subscript ] path
               | "%" rnl-name [ places ] path
               | "%#" integer path                (* a number, in a record of the solver, 5.9 *)
               | "$" rnl-name [ places ] ;
placeholder    = "?" rnl-name [ places ] path
               | "?" path ;                        (* alone: in a symbol body, or in a general definition, 9.4; a path only in a general definition *)
places         = "{" place-item { "," place-item } "}" ;    (* touching the name, 5.2.3 *)
place-item     = place [ ":" place [ ":" integer ] ] ;
place          = integer | "end" [ "-" integer ] ;
path           = { ":" terminal-name [ subscript ] | "::" rnl-name } ;
subscript      = "<" subscript-item { "," subscript-item } ">"
               | "[" subscript-item { "," subscript-item } "]" ;
subscript-item = integer [ ":" integer [ ":" integer ] ] ;   (* unsigned, 6.11 *)
host-name      = name | string ;
rnl-name       = name | string ;
terminal-name  = name | string ;
role-label     = bracket-name | string ;
point          = "(" coordinate "," coordinate ")" ;   (* after white space, a comment, "{", ";", "=>" or "=", 3.8 *)
coordinate     = number | "_" | parameter-placeholder ;
parameters     = "(" [ value { "," value } [ "," named-value { "," named-value } ]
                     | named-value { "," named-value } ] ")" ;
named-value    = parameter-name "=" value ;
parameter-name = bracket-name | string ;
value          = number | string | "true" | "false"
               | "[" [ value { "," value } ] "]"
               | parameter-placeholder ;
parameter-placeholder = "?" rnl-name ;
```

- `name`, `bracket-name`, `keyword-name`, `string`, `number` et `integer` s’écrivent comme le définit le chapitre 3. `name` est un nom sans guillemets (3.6). `bracket-name` est un nom sans guillemets dans les parenthèses d’un rôle ou devant le `=` d’un paramètre : lettres ASCII, chiffres, `_` et tout caractère non ASCII, ne commençant pas par un chiffre ASCII (3.6). `keyword-name` est composé de lettres, chiffres et `_`, avec les mots joints par `-` ; un `-` n’y appartient que lorsqu’une lettre le suit (3.3). Un mot-clé standard est en majuscules (3.6) ; un alias est composé de lettres de toute écriture, de chiffres et de `_`, sans `-` (16.5). `string` et `number` figurent en 3.7 ; `integer` est un entier décimal non signé.
- `rnl-name`, `host-name`, `terminal-name`, `role-label` et `parameter-name` peuvent également être mis entre guillemets, avec le même sens, sauf qu’aux places de rôle et de nom de paramètre un nom entre guillemets n’est jamais un alias (3.6). Les mots-clés, noms de paramètres, noms d’emplacements dans les parenthèses de rôle, noms de sous-objets, `true` et `false` peuvent s’écrire sous forme d’alias déclarés ; les alias non ASCII sans guillemets ne sont utilisés qu’après `@` et à l’intérieur des parenthèses, et un alias après `::` est mis entre guillemets (16.5).
- Un indice suit directement un nom hôte ou un nom de borne, sans espace blanc entre eux ; des espaces blancs et des commentaires peuvent être écrits à l’intérieur d’un indice (3.5, 6.11). Quels noms prennent un indice : 4.2 point 10.
- Les parenthèses de paramètres touchent le mot-clé, et les parenthèses de rôle touchent l’objet ; un `(` après un espace blanc ou un commentaire commence un point (3.5).
- `=>` est un chaînage ou un nommage, selon ce qui le suit (4.2 point 3).
- Un paramètre générique portant un nom ne s’écrit que dans le corps d’une définition générale (6.10). À une place d’objet, c’est un rôle et il peut avoir un chemin ; à une place qui prend un littéral (une valeur de paramètre, un élément de tableau, une coordonnée de point), c’est un paramètre de la définition et il n’a pas de chemin (6.10, 9.4). Le `?` isolé s’écrit dans un corps de symbole (13.3) et dans le corps d’une définition générale (9.4) ; ce n’est que dans une définition générale qu’il peut avoir un chemin, tel que `?::position`.
- Le texte libre et les commentaires peuvent être écrits entre deux jetons quelconques hors des parenthèses et crochets et au début d’un membre (3.3, 3.4) ; les espaces blancs et commentaires peuvent aussi être écrits à l’intérieur des parenthèses et crochets. La grammaire ne les montre pas.
- Un membre vide (comme dans `;;`) est ignoré ; le `;` après le dernier membre peut être omis.

### 4.2 Contraintes hors de la grammaire

1. Un segment n’est pas vide.
2. Un segment comporte au plus un mot-clé. `@RNL` n’apparaît qu’au début d’un enregistrement.
3. Un `=>` suivi d’un nom en `%` ou `$`, après lequel viennent `=>`, `;`, `}` ou la fin du membre, est un nommage. Tout autre `=>` commence le segment suivant ; ce segment comporte exactement un mot-clé, et un nom qui y est écrit est un objet du segment. Le premier segment d’un chaînage peut ne comporter aucun mot-clé ; il possède alors exactement un objet. Ainsi, pour nommer un résultat et poursuivre le chaînage, écrivez `=> %g0 => @PLACE(1, 2)`. Dans `=> %g0 @PLACE(1, 2)`, `%g0` est un objet de PLACE ; habituellement il n’est pas lié, et l’instruction échoue parce que `%g0` est introuvable. Le diagnostic peut demander s’il manque un `=>`.
4. Dans un segment sans mot-clé, chaque argument est une instruction à part entière : `#M1 #M2` dans `{#M1 #M2}` représente deux instructions. Un tel segment, lorsqu’il est nommé (`%p =` ou `=> %p`), ne comporte qu’un seul objet : écrivez `%p = {#M1 #M2}`, et non `%p = #M1 #M2`.
5. Les rôles ne sont marqués que sur les arguments d’un segment qui comporte un mot-clé. Les segments après un chaînage ne marquent aucun rôle.
6. Dans un segment, soit tous les arguments sont marqués avec des rôles, soit aucun ne l’est. Le même rôle n’est pas marqué deux fois.
7. Pour les mots-clés qui prennent une définition, le mot-clé fixe l’emplacement de la définition : pour ATTACH c’est le premier objet écrit ; pour AS, SET-SYMBOL, SET-STYLE, SELECT-USAGE et SELECT-ANCESTOR-USAGE c’est le dernier objet. SELECT-USAGE peut omettre sa définition : son dernier objet ne remplit l’emplacement de définition que lorsqu’il s’agit d’une définition (10.1). La définition n’est pas marquée avec un rôle et ne compte pas pour la règle 6 (5.6).
8. Il existe deux manières de nommer, qui peuvent être utilisées ensemble. `name =` ne peut être que le premier jeton d’un membre (du texte libre peut le précéder), au plus une fois dans un membre. `=> name` suit un segment, à la fin du membre ou avant le `=>` du segment suivant, et un membre peut en comporter plusieurs. La règle 3 le distingue d’un chaînage.
9. Les paramètres donnés par position viennent en premier, puis ceux donnés par nom : après un `name=value`, tous les paramètres suivants sont donnés par nom. Seuls les paramètres pour lesquels la signature indique un ordre de position peuvent être donnés par position, dans cet ordre. Aucun paramètre n’est donné deux fois.
10. Un indice ne suit qu’un nom hôte après `#` ou `&`, ou un nom de borne ou un nom de port dans une référence en `#`. Les noms RNL, noms de rôles et noms de sous-objets ne prennent aucun indice (6.11).
11. Une même référence comporte au plus un indice qui se développe en une liste de noms (6.11).

### 4.3 Ordre des mots

- Le mot-clé, avec ses paramètres, peut se placer n’importe où parmi les objets ; l’ordre des objets importe. `#M1 #M2 @DISTANCE(…)`, `#M1 @DISTANCE(…) #M2` et `@DISTANCE(…) #M1 #M2` sont identiques.
- Chaque mot-clé a une forme standard unique, et les outils ne doivent écrire que cette forme. La forme standard est consignée dans la liste de mots (5.3) : où se place le mot-clé, quels paramètres sont donnés par position et lesquels par nom, et si les rôles sont marqués.
- Par défaut, la forme standard est : d’abord les objets, puis le mot-clé avec ses paramètres, puis la définition après le mot-clé. ATTACH écrit d’abord sa définition, puis le mot-clé, puis les objets : `$GROUND @ATTACH &vss`. TABLE écrit son mot-clé en premier, avant la table : `@TABLE layout: {…}` (14.11). Les paramètres pour lesquels la signature indique un ordre de position sont donnés par position, en premier ; les autres sont donnés par nom, dans l’ordre de la liste de mots : `@PLACE(10, 20)`, `@ORIENT("R90")`, `@DISTANCE("bottom-to-top", min=4)`, `@TEXT("W/L", key="wl")`. PLACE ne donne x et y par position que lorsque les deux sont écrits ; un seul axe s’écrit par nom, comme dans `@PLACE(x=10)`. Les paramètres qui peuvent être retirés sans changer le sens ne sont pas écrits, tels que `mode="auto"` de WIRE. Les mots-clés intégrés s’écrivent sans rôles ; AS et ATTACH marquent tous les rôles lorsque la définition générale comporte deux rôles ou plus.
- Mise en forme : une espace après chaque virgule entre parenthèses ou crochets ; aucune espace autour du `=` d’un paramètre ; une espace de part et d’autre du `=` d’un nommage et de `=>` ; le mot-clé touche sa parenthèse ; une espace entre `{` et un mot-clé, comme dans `{ @…` (2.3) ; les nombres s’écrivent sous la forme décimale la plus courte qui se relit comme la même valeur.
- **Forme standard du nommage.** Un membre sans chaînage est nommé au début, `%x = statement` ; un membre avec chaînage est nommé à la fin, `statement => … => %x`, le nom figurant après le segment qu’il nomme. Ainsi, un nom au début nomme toujours un membre entier, et les lecteurs n’ont pas à deviner quel segment d’un chaînage il nomme. La syntaxe accepte les deux formes.
- « Uniquement cette forme » s’applique aux instructions qu’un outil écrit nouvellement. Lorsqu’un outil réécrit des enregistrements existants, il ne doit pas réordonner les instructions écrites par les utilisateurs (16.4) ; il ne les met sous la forme standard que lorsque l’utilisateur demande le formatage.
- Les phrases standards figurent en 4.5.

### 4.4 Exemples d’erreurs

Chacun des exemples suivants est une erreur ; le commentaire en indique la raison.

```rnl error
#M1 @PLACE (10, 20)    /* a bracket after white space is a point, and PLACE has no parameters (3.5) */
```

```rnl error
#M1 (left) #M2 (right) #M5 (tail) @AS $DIFFERENTIAL_PAIR_N    /* role brackets touch the object; a separate bracket is a point (3.5) */
```

```rnl error
#M2 @PLACE(62, 40) (MY)    /* the second bracket is a point, and its content is not a point */
```

```rnl error
&vss @ATTACH $GROUND    /* the definition of ATTACH is written first: $GROUND @ATTACH &vss */
```

```rnl error
R1 -> R2 {#R1 #R2} @ORDER("left-to-right")    /* ">" cannot be free text (3.3) */
```

```rnl error
#M1 @PLACE(1, 2), #M2 @PLACE(3, 4)    /* a comma does not separate statements: two keywords in one segment */
```

```rnl error
#M1 #M2 @SET-SYMBOL $NMOS    /* three objects, and SET-SYMBOL has two slots (5.2.1); write {#M1 #M2} @SET-SYMBOL $NMOS */
```

```rnl error
#M1 => #M2    /* the segment after a pipe has no keyword */
```

```rnl error
#M1 #M2 #M3 @DISTANCE("left-to-right", min=1)    /* three objects for two slots */
```

```rnl error
#M1 @PLACE(x=10, 20)    /* a parameter given by position after one given by name (4.2 item 9) */
```

```rnl error
{#M1 #M2} @ORDER(axis="x", direction="increasing")
/* ORDER has no axis: the axis and the direction are one parameter, direction, as in @ORDER("left-to-right") (14.3) */
```

```rnl error
#M1 @SET-SYMBOL($NMOS)    /* brackets hold only literals */
```

```rnl error
{#M1 #M2} @ALIGN(axis=y)    /* enumerations are written as strings */
```

```rnl error
%ax = (_, _);
#M1(left) #M2 %ax @AS $MIRRORED_LAYOUT /* roles are marked on all objects or on none */
```

```rnl error
$NMOS(symbol) #M1(target) @SET-SYMBOL    /* the definition is written last and is not marked with a role */
```

```rnl error
#R1(source) #R2(chain) #R3(chain) #R4(load) @AS $CHAIN_ROW    /* the same role is marked twice */
```

```rnl error
(0, 0) @PIN(terminal=1)    /* a pin cannot be written in the scene (13.4) */
```

```rnl error
#M1 @CURRENT("in")    /* CURRENT takes only terminals (15.2) */
```

```rnl error
&D<1:0>[1:0]    /* a name has at most one subscript (6.11) */
```

```rnl error
#XI0<1:0>:D<1:0>    /* one reference has at most one expansion (6.11) */
```

```rnl error
&D<_, 1>    /* a "<" directly after a name starts a subscript, and its content is not a subscript (3.5) */
```

```rnl error
%p<1, 2>    /* RNL names take no subscript; a point is written (1, 2) (6.11) */
```

### 4.5 Phrases standards

Une phrase standard est une instruction sous la forme standard avec du texte libre ajouté entre ses jetons, afin qu’elle se lise comme une phrase. Les phrases standards sont utilisées dans les guides d’écriture et par les outils de formatage qui ajoutent ces mots aux instructions ou les retirent. Le texte libre ne modifie pas le sens (3.3) : les phrases ne sont que des suggestions, et les utilisateurs comme les modèles de langue peuvent les écrire, n’en écrire aucune ou écrire d’autres mots.

- **Des faits.** Une instruction énonce ce qui vaut dans le dessin, et non une opération à exécuter. Les objets viennent en premier en tant que sujet, le mot-clé sert de verbe ou de nom, et les paramètres sont ses valeurs. Seuls des mots de liaison sont ajoutés, tels que `is`, `are`, `has`, `is at` et `there is`. Quelques instructions peuvent se lire comme des opérations, telles que `#M1 @SET-SYMBOL to $NMOS`.
- **Les mots découlent de l’instruction.** Les mots qu’un outil génère ne dépendent que du mot-clé, des paramètres, du nombre et des sortes des objets ainsi que du niveau où l’instruction est écrite, et non du sens du circuit. Par exemple, `is` ou `are` suit le nombre d’objets ; `the x of` s’écrit lorsqu’un seul axe est donné ; la formulation des candidats d’une sélection suit la portée et la sorte sélectionnée (Annexe B). C’est ainsi que les phrases sont générées ; cela ne restreint pas les utilisateurs.
- **Aucun mot qui semble signifier quelque chose que l’instruction ne dit pas.** Des mots tels que `must`, `required` et `only` ne sont pas écrits, et un mot de direction n’est écrit que lorsque le paramètre possède cette direction. Les lecteurs croient aux mots d’une phrase, et les guides d’écriture devraient les en avertir.
- **Gloses.** Les lecteurs peuvent ignorer les codes d’orientation et les axes. Après le mot-clé, `, i.e., …` peut indiquer ce que le code lui-même signifie, comme dans `#M2 @ORIENT("MX"), i.e., flipped upside down`.
- **ORIENT-RELATION et DISTANCE**, qui ont deux emplacements, se lisent `from A to B`.
- **Les sélections** se lisent toujours comme un choix parmi des candidats : `among <candidates>, @SELECT-…`. Lorsque la portée est `#`, les candidats sont `the <kind> in # only`, et lorsque l’instruction atteint les niveaux inférieurs, `the <kind> at each level in # and below`, ce qui indique que la règle est évaluée dans chaque niveau (10.1, 10.4). Lorsque la sorte ne peut pas être déterminée, la phrase se lit `for X,`.
- **AS et ATTACH.** AS s’emploie lorsque les objets forment ensemble la chose : `#M1(left), #M2(right) and #M5(tail) are treated @AS $DIFFERENTIAL_PAIR_N`. ATTACH s’emploie lorsque la chose est ajoutée aux objets : `$GROUND is @ATTACH to &vss` (9.4).
- **Chaînages.** Le segment suivant se réfère au résultat du précédent. L’anglais emploie `it` (après AS, ATTACH ou un objet unique) ou `they` (après une sélection). Un nom dans un chaînage est précédé de `named`, comme dans `… => named %pout` ; il s’agit toujours d’un nommage (4.2 point 3).
- **Nommage.** Le nom d’une instruction isolée s’écrit au début (4.3). Lorsqu’une instruction AS est nommée, l’anglais omet `are` : `%dp = #M1(left), #M2(right) and #M5(tail) treated @AS $DIFFERENTIAL_PAIR_N`.
- **Noms d’ensembles.** Un nom qui désigne un ensemble devrait se terminer par `_SET` pour les noms en `$` et `_set` pour les noms en `%`, tels que `$GROUND_FILL_SET` et `%bias_set` ; le paquet standard suit cette règle.
- **Pas de `each`.** Des mots tels que `each` ne sont pas écrits. Le fait qu’un ensemble soit appliqué en bloc ou membre par membre dépend de la liste de rôles de la définition, et les phrases standards ne précisent pas lequel.
- **Autres langues.** La présente norme donne les phrases en anglais. Les phrases dans une autre langue utilisent les alias d’un paquet d’alias de cette langue, tel que `alias-zh` pour le chinois, et suivent sa documentation (16.5) ; après un signe de ponctuation de cette langue, une espace reste nécessaire avant `@` (2.3).

La phrase standard de chaque mot-clé figure dans l’Annexe B.

<!-- translated from en/ch05.md sha256:53b51cde1141 -->
## 5 Objets et instructions

### 5.1 Objets

- Chaque instruction comportant un mot-clé crée un nouvel objet RNL, fait d’une poignée et de faits. Un `{…}` crée également un nouvel objet, un ensemble (5.2).
- Une référence (`#M1`, `&vss`, `%x`, `$X`, une borne ou un sous-objet) ne crée aucun objet ; elle prend une poignée existante. Les places de membres qui donnent plusieurs membres créent un nouvel ensemble (5.2.3). Un `#M1` isolé est aussi une instruction : il donne M1 sans faits. Un `#` isolé donne ce niveau sans faits.
- Une poignée est la chose décrite : une instance, un net ou une borne de l’hôte, ou le niveau ; ou encore un point, fil, ensemble, utilisation, contrainte, etc. créé par RNL. Une poignée n’est jamais un autre objet RNL.
- L’objet d’une instruction possède soit une nouvelle poignée (l’instruction crée, comme WIRE, les graphiques, `{…}`, les sélections, AS, ATTACH et les contraintes), soit garde la poignée de son entrée en y ajoutant de nouveaux faits (l’instruction modifie, comme PLACE, ORIENT, SET-SYMBOL, SET-STYLE, GROUP et CURRENT ; ADD-TO et REMOVE-FROM gardent la poignée de l’ensemble cible). La signature indique lequel des deux (5.3) ; WIRE crée lorsqu’il reçoit un net et modifie lorsqu’il reçoit un fil (13.1).
- Deux instructions ayant le même contenu sont deux objets, comme deux `#M1 @PLACE(1, 2)`. Leurs faits ne se contredisent pas et prennent effet ensemble.
- Un fait est versé une seule fois, à l’instruction qui le crée (5.4). Lorsque le niveau prend effet, tous les faits concernant une même poignée sont combinés ; les contradictions sont traitées comme l’indique 1.5 point 3.
- Les propriétés sont des faits : la position, l’orientation, le symbole et le style sont des faits portant sur l’objet, et il n’existe pas de champs de propriété à écraser. Lorsque le niveau prend effet, les faits concernant une même poignée sont combinés en valeurs finales : x et y de PLACE en tant que faits géométriques (5.8, 8.5), les symboles et les styles par l’intermédiaire des files d’apparence (8.3).
- La sorte d’une poignée découle de sa provenance : les objets hôtes la reçoivent de l’hôte, les objets RNL de l’instruction qui les crée. La signature indique quelles sortes accepte chaque emplacement (Annexe A) ; une description qui reçoit un objet d’une mauvaise sorte échoue. Ce qui peut être vérifié à la lecture du texte est vérifié alors ; ce qui n’est connu que plus tard (si une utilisation a un symbole, les rôles d’une définition) est vérifié lorsque le niveau prend effet (7.1).

Sortes d’objets :

| Sorte | Créé par | Dessiné |
|---|---|---|
| niveau | l’hôte ; `#` | non |
| instance | l’hôte ; `#M1` | avec son symbole |
| borne | l’hôte ; `#M1:d`, `#:inp` (6.4). Les utilisations n’ont pas de bornes | aux broches qui la lient (9.2.4) |
| broche | un PIN du symbole d’un objet dessiné (9.2.4) ; `#X1::vdd_t` lorsque la broche porte un nom | comme un point du symbole |
| net | l’hôte ; `&vss` | non ; fils et étiquettes sont des objets à part entière |
| point | `(x, y)` | non ; un point de jonction est `$JUNCTION` appliqué à un point (13.2) |
| fil | WIRE recevant un net ; un fil dont l’emplacement de net reçoit un ensemble est un bus. Recevant un fil, WIRE le réécrit (13.1) | oui |
| étiquette | un TEXT attaché à un objet ; avec une clé, c’est le sous-objet `object::key` de cet objet (13.3) | oui |
| faisceau | BUNDLE (11.8) | une seule fois, à la place de ses membres |
| annotation | un graphique dans la scène (LINE, RECTANGLE, CIRCLE, ELLIPSE, ARC, BEZIER), un TEXT attaché à un point (13.4), ou un symbole dessiné par ANNOTATION (13.5) | oui |
| utilisation | AS, ATTACH (9.4) | uniquement lorsqu’elle a un symbole ; c’est alors un composant virtuel (chapitre 12) |
| contrainte | ALIGN, ORDER, DISTRIBUTE, DISTANCE, ORIENT-RELATION, TABLE (chapitre 14) | non |
| ensemble | `{…}`, sélections, UNION, résultat d’une génération un par un (5.2, chapitre 10) | pas lui-même ; ses membres sont dessinés chacun pour soi |
| définition | DEFINE-SYMBOL, DEFINE-STYLE, DEFINE (chapitre 9) | non ; n’agit que là où elle est utilisée |

Parmi ces sortes, instance, borne, broche, net, point, fil, étiquette, faisceau, annotation et utilisation possèdent des noms de sorte (5.3), tout comme `text`, qui couvre tout TEXT, qu’il s’agisse d’une étiquette ou d’une annotation ; l’Annexe A indique comment les signatures nomment les autres.

Il existe aussi des sous-objets (6.5), dont la poignée est un objet parent et un nom, ainsi que des objets en échec (chapitre 7).

### 5.2 Ensembles

- `{…}` crée un objet ensemble. Ses membres sont ordonnés. Un ensemble peut être nommé (`%pair_set = {#M1 #M2}`), et d’autres instructions peuvent ajouter des membres à un ensemble nommé ou en retirer (5.2.2).
- Un ensemble garde les membres répétés et l’ordre dans lequel ils sont écrits, tel que `{#M1 #M2 #M3 #M1}`. Le fait que deux membres soient le même membre est décidé par poignée : deux `{…}` écrits séparément sont deux poignées et ne sont pas le même membre, même lorsque leurs contenus sont égaux.
- **Une référence est dépliée d’un niveau** : une référence dans un ensemble qui désigne un ensemble met les membres de cet ensemble à sa place, si bien que `{%a_set %b_set}` est constitué des membres de a_set suivis de ceux de b_set. Il en va de même pour une instruction dont le résultat est un ensemble, telle qu’une sélection, et pour la liste de références en laquelle se développe un indice, telle que `&D<7:0>` (6.11). Pour supprimer les doublons, écrivez `%a_set %b_set @UNION` (5.2.2).
- **Un `{…}` écrit tel quel est retenu** : un `{…}` écrit directement dans un ensemble est retenu comme un seul membre, si bien que `{{%a_set} {%b_set}}` est un ensemble de deux ensembles.
- **Les objets libres sont dépliés aussi** : lorsqu’un mot-clé à un seul emplacement reçoit plusieurs objets écrits sans accolades, ils sont rassemblés exactement comme s’ils étaient écrits dans `{…}` (5.2.1 point 3), de sorte qu’un ensemble parmi eux est déplié. Après `%bias_set = {#R1 #R2 #R3}` et un texte `%lbl` :
  - `%bias_set %lbl @ALIGN("x")` équivaut à `{%bias_set %lbl} @ALIGN("x")` ;
  - un ensemble qui est le seul objet est utilisé tel quel : `%bias_set @ALIGN("x")` aligne R1, R2 et R3.
- **Éléments** : un ensemble garde aussi ses éléments tels qu’ils sont écrits, dans l’ordre : un objet écrit dedans, un `{…}` écrit tel quel, ou une référence à un ensemble. Là où la présente norme compte des éléments (les listes de membres d’ALIGN, ORDER et DISTRIBUTE, 14.1, les membres d’ASSERT-TOPOLOGY, 15.4, et le découpage de 5.2.1 point 2), un objet est un élément, et un `{…}` ou une référence à un ensemble en tant qu’élément représente chacun de ses membres les plus internes à cet élément ; dans ASSERT-TOPOLOGY il représente une boîte noire faite de ceux-ci. ADD-TO ajoute un élément à la fin, et REMOVE-FROM retire les objets de chaque élément (5.2.2) ; un ensemble créé par une sélection, par UNION ou par génération un par un comporte un élément par membre, tout comme la liste en laquelle se développe un indice (6.11) et l’ensemble que prennent les places de membres (5.2.3). Partout ailleurs, un ensemble désigne ses membres, dépliés d’un niveau comme ci-dessus. Ainsi `{%bias_set %lbl} @ALIGN("x")` possède deux éléments, R1–R3 comme premier et le texte comme second, et donne à chacun de R1, R2 et R3 le x du texte (14.1).
- Le contenu d’un ensemble est calculé dans la phase qui l’utilise, comme pour une sélection : son contenu initial, ainsi que les sélections dans les instructions qui ajoutent ou retirent des membres, sont évalués sur l’état avant le début de cette phase, et les ajouts et retraits sont appliqués dans l’ordre d’écriture (leur position, 8.5). Ainsi une utilisation écrite plus tôt voit également tous les ajouts et retraits (1.5 point 1), et un même ensemble peut différer d’une phase à l’autre : les règles qui génèrent des objets le voient sans les objets générés, tandis que les règles de style le voient avec eux (8.2).

  Note : le contenu n’est pas calculé une fois pour toutes à une seule étape, car nommer une sélection changerait alors son résultat : `%g_set = # @SELECT-USAGE $GROUND; %g_set @SET-STYLE $grey` manquerait les masses générées, que la même sélection écrite sans nom trouve ; un nom n’est qu’une autre façon de désigner un objet (1.5 point 5).

#### 5.2.1 Objets et emplacements

Les objets écrits dans une instruction sont mis en correspondance avec les emplacements de la signature comme suit.

1. **Marqués avec des rôles** : un objet marqué avec un rôle entre entier dans ce rôle et n’est pas scindé. Dans une même instruction, soit tous les objets sont marqués, soit aucun ne l’est.
2. **Scinder d’un niveau d’abord** : lorsqu’aucun rôle n’est marqué et que l’instruction entière comporte un seul objet, qui est un ensemble, cet ensemble est d’abord scindé d’un niveau en ses éléments (5.2). TABLE fait exception : son emplacement prend l’ensemble tel qu’il est écrit, comme liste externe de la table (14.11). Une définition et le résultat transmis par un chaînage comptent comme des objets. Ainsi `{#M1 #M2} @DISTANCE(…)` équivaut à `#M1 #M2 @DISTANCE(…)`, tandis que `{#M1 #M2} @SET-SYMBOL $X` comporte deux objets (l’ensemble et la définition) et n’est pas scindé.
3. **Correspondance selon la signature** :
   - Un mot-clé à plusieurs emplacements (une définition occupe aussi un emplacement) : le nombre d’objets doit égaler le nombre d’emplacements, et ils sont mis en correspondance un à un dans l’ordre ; sinon l’instruction échoue. Pour AS et ATTACH, les rôles d’ensemble à la fin de la liste de rôles peuvent être laissés sans objets (9.4).
   - Un mot-clé à un seul emplacement (tel que PLACE, ORIENT, ALIGN, ORDER, DISTRIBUTE, GROUP, les sélections autres que SELECT-USAGE et SELECT-ANCESTOR-USAGE qui prennent aussi une définition (5.6), et les mots-clés de définition dont l’emplacement est le corps) : un objet unique entre tel quel dans l’emplacement ; plusieurs objets sont réunis en un seul ensemble, dans lequel les ensembles référencés sont dépliés d’un niveau et chaque `{…}` écrit tel quel est retenu comme un membre. Lorsque les objets sont les éléments du seul ensemble scindé au point 2, l’emplacement reçoit cet ensemble lui-même.
   - Lorsque le seul objet est un ensemble vide :
     - Un emplacement qui prend un objet unique effectue son travail zéro fois : rien ne se passe et rien n’échoue. Un nom est lié à l’ensemble vide comme d’habitude, et les instructions qui utilisent ce nom ne font rien.
     - Un ensemble vide écrit tel quel pour un emplacement qui prend un objet unique produit un avertissement, comme `{} @PLACE(1, 2)`. Un ensemble vide provenant d’une sélection ou d’une règle n’en produit aucun : une règle avec `recursive=true` est également évaluée dans chaque niveau inférieur, et un niveau sans objets de cette sorte est normal. Lorsqu’une sélection écrite par l’utilisateur ne sélectionne rien dans aucun des niveaux où elle est évaluée, une implémentation peut émettre un avertissement ; la condition est probablement erronée.
     - Un emplacement qui prend un ensemble accepte un ensemble vide, tel que le corps vide de `@DEFINE(roles=["a"]) {}`, ou une contrainte sans membres, qui est trivialement satisfaite. Les exceptions sont BUNDLE et l’emplacement de net de WIRE : ils créent un nouvel objet à partir de l’ensemble, ce qui ne peut pas être créé à partir de rien, donc l’instruction échoue (11.8, 13.1).
     - Une instruction sans aucun objet, telle qu’un `@PLACE(1, 2)` isolé, ne correspond pas à la signature et échoue.
4. **Un ensemble dans un emplacement** :
   - Un emplacement qui prend un objet unique effectue son travail une fois pour chaque membre. Chaque membre constitue une description à part entière, et le résultat est l’ensemble des résultats. Un membre qui est lui-même un ensemble est scindé à nouveau et traité membre par membre. La seule exception est PRIORITY : recevant un ensemble marqué avec GROUP, il agit sur le groupe lui-même (11.5, 14.8).
   - Un emplacement qui prend un ensemble le prend entier, et prend un objet unique comme un ensemble de cet objet unique : `#M3 @ROUTE-OBJECT` équivaut à `{#M3} @ROUTE-OBJECT`.
   - Chacun des deux emplacements de DISTANCE prend un objet ou un ensemble ; un ensemble représente chacun de ses membres, et DISTANCE relie chaque membre d’un emplacement à chaque membre de l’autre, à raison d’une contrainte par paire (14.5). C’est le seul mot-clé dont les deux emplacements peuvent recevoir des ensembles simultanément.
   - Dans une même instruction, au plus un emplacement prenant un objet unique peut recevoir un ensemble de plus d’un membre. Lorsque plus d’un tel emplacement en reçoit un, l’instruction échoue : apparier les membres dans l’ordre ou prendre toutes les combinaisons auraient tous deux du sens, si bien que le sens n’est pas unique.

| Écrit | Résultat |
|---|---|
| `{#M1 #M2} @SET-SYMBOL $X` | Deux objets, non scindés. L’emplacement d’instance reçoit un ensemble : M1 et M2 reçoivent chacun le symbole |
| `#M1 #M2 @SET-SYMBOL $X` | Trois objets pour deux emplacements : l’instruction échoue |
| `{#M1 #M2} @DISTANCE(…)`, `%pair_set @DISTANCE(…)` | Le seul objet est un ensemble et est scindé : de M1 à M2 |
| `%a_set %b_set @DISTANCE(…)` | Deux objets, chacun étant un ensemble : de chaque membre de a_set à chaque membre de b_set (14.5) |
| `{#M1 #M2} @ALIGN("y")`, `#M1 #M2 @ALIGN("y")` | ALIGN a un seul emplacement, qui reçoit {M1 M2} |
| `%bias_set %lbl @ALIGN("x")`, avec `%bias_set = {#R1 #R2 #R3}` | Les objets libres sont réunis en `{%bias_set %lbl}` : deux éléments, R1–R3 comme premier, le texte comme second ; R1, R2 et R3 reçoivent chacun le x du texte (14.1) |
| `{%bias_set} %lbl @ALIGN("x")` | Les mêmes deux éléments : R1, R2 et R3 reçoivent chacun le x du texte |
| `{%bias_set} @ALIGN("x")`, `{{#M1 #M2 #M3}} @ALIGN("y")` | Le seul objet est un ensemble et est scindé d’un niveau en son unique élément (point 2) ; cet élément est un ensemble et entre dans l’emplacement tel quel : aligne R1, R2 et R3, ou M1, M2 et M3 |
| `%pair_set @PLACE(10, 20)` | L’emplacement reçoit pair_set lui-même et le place membre par membre : les deux membres en (10, 20). Un ensemble n’a pas de position propre (14.1) |
| `%a_set %b_set @PRIORITY("disabled")`, les deux ensembles avec GROUP | Les objets libres sont dépliés comme dans `{…}` : PRIORITY reçoit les membres et échoue sur eux (mauvaise sorte). Pour désactiver deux groupes, écrivez un PRIORITY pour chacun (11.5, 14.8) |
| `{#M1 #M2} @SET-SYMBOL $X => @ALIGN("y")` | Le résultat du chaînage est le seul objet et entre dans ALIGN : aligne M1 et M2 |
| `{#M1 #M2} @SET-SYMBOL $X => #M3 @DISTANCE(…)` | Deux objets, non scindés. L’emplacement `from` reçoit l’ensemble : de M1 à M3 et de M2 à M3 (14.5) |
| `# @SELECT-NET(…) => $PULL_DOWN @ATTACH &vss` | Trois objets pour trois emplacements (deux rôles et la définition). Le premier rôle est un rôle simple, donc la définition est appliquée une fois pour chaque net sélectionné, chaque fois avec vss dans le second rôle |
| `# @SELECT-INSTANCE(type="nmos") => @ORIENT("MY")` | Mis en miroir un par un ; sans NMOS, exécuté zéro fois, et rien ne se passe |
| `# @SELECT-INSTANCE(name="Q*") => @SET-SYMBOL $NPN` | Deux objets, non scindés ; sans BJT, exécuté zéro fois, et rien ne se passe |
| `#R1 {#R2 #R3} #R4 @AS $CHAIN_ROW` | Quatre objets pour quatre emplacements (trois rôles et la définition) ; le rôle d’ensemble `chain` reçoit {R2 R3} (9.4) |
| `&D<7:0> @WIRE {…}` | Deux objets, non scindés. L’emplacement de net prend la liste de nets entière : un bus (13.1) |
| `#XI0<3:0> @ALIGN("y")` | Le seul objet est la liste en laquelle se développe un indice, et elle est scindée : aligne les quatre instances (6.11) |

- Une contrainte comportant trop peu de membres est trivialement satisfaite et n’échoue pas, telle qu’un ALIGN à un seul membre (chapitre 14).
- Un ensemble n’a pas de position propre (14.1) ; son `::position` est l’ensemble des positions de ses membres (6.5). Dans une contrainte, une utilisation sans symbole représente ses membres (14.1). Pour relier un groupe d’objets à d’autres comme un seul endroit, faites-en les membres d’une utilisation dont le corps les attache au point propre de l’utilisation (9.4), et contraignez `%u::position` ; pour maintenir un ensemble d’un côté par rapport à d’autres objets, donnez-le à DISTANCE (14.5).
- Les rôles d’une définition sont des rôles simples ou des rôles d’ensemble (9.4). Un rôle écrit `"{name}"` dans la liste de rôles est un rôle d’ensemble et prend un ensemble entier ; tout autre rôle est un rôle simple, et recevant un ensemble, la définition est appliquée une fois pour chaque membre.

#### 5.2.2 Ajout et retrait de membres, et doublons

- `X @ADD-TO S` ajoute X à la fin de l’ensemble S (les membres de X lorsque X est un ensemble) ; `X @REMOVE-FROM S` retire toute occurrence de ces membres de S. La cible est écrite en dernier, afin qu’une sélection puisse y être transmise par chaînage : `# @SELECT-NET(name="vss") => @ADD-TO %gnd_set`.
- La cible doit être un ensemble ; elle n’est pas enveloppée dans un ensemble. Après `%tail = #M5`, `#M3 @ADD-TO %tail` échoue, parce que M5 n’est pas un ensemble.
- Les ajouts et retraits sont appliqués dans l’ordre de position (1.5 point 2). Un ensemble évalué dans un niveau ne contient que les membres de ce niveau (10.3).
- `%a_set %b_set @UNION` crée un nouvel ensemble sans doublons, dans l’ordre de première apparition ; le fait que deux membres soient identiques est décidé par poignée.
- Pour créer un nouvel ensemble en laissant les anciens tels quels : la concaténation est `{%a_set %b_set}` ; l’union `%a_set %b_set @UNION` ; la différence `%c_set = {%a_set}; %b_set @REMOVE-FROM %c_set`.
- Un ensemble dépend des ensembles écrits dans l’instruction qui le crée (à l’intérieur de ses accolades, dans UNION, ou comme portée d’une sélection) ainsi que des ensembles que ADD-TO et REMOVE-FROM lui ajoutent ou lui retirent. Les dépendances qui forment un cycle sont une erreur, avec ou sans retrait : en prenant les instructions dans l’ordre de position (8.5), celle dont la dépendance ferme le cycle échoue, et son diagnostic énumère tout le cycle ; les autres prennent effet normalement. Ainsi `%a_set @ADD-TO %b_set; %b_set @ADD-TO %a_set` échoue à la seconde instruction, et `%b_set = {%a_set}; %b_set @REMOVE-FROM %a_set`, dont le retrait n’aurait aucun résultat défini, échoue au retrait.

#### 5.2.3 Places de membres

```rnl
%row_set = {#M1 #M2 #M3 #M4};
%row_set{1} @PLACE(0, 0);                                         /* M1 */
%row_set{1} %row_set{end} @DISTANCE("left-to-right", exact=12);   /* from M1 to M4 */
%row_set{2:end} @ALIGN("y")                                       /* M2, M3 and M4 */
```

- Une référence à un ensemble suivie directement de `{…}` prend des membres de l’ensemble par leurs places, telles que `%a{1}`, `%a{end}`, `%a{end-1}` et `%a{2:end}`. Cela s’écrit après les noms en `%` et `$` liés à des ensembles, et après le paramètre générique d’un rôle d’ensemble dans le corps d’une définition générale, tel que `?ref{1}` (9.4).
- **Forme** : à l’intérieur des accolades, des éléments séparés par `,` ; chaque élément est `i`, `a:b` ou `a:b:s`, et les formes peuvent être mélangées, comme dans les indices (6.11). Une place est un entier positif, `end` ou `end-k`, où k est un entier non signé ; s est un entier non signé d’au moins 1. Les espaces blancs et les commentaires peuvent être écrits à l’intérieur des accolades.
- **Comptage** : les places se comptent à partir de 1 dans l’ordre des membres de l’ensemble, après tous ses ajouts et retraits (5.2), y compris ceux écrits après la référence, si bien que `%a{2}` après `%a = {#M1}; #M2 @ADD-TO %a` est M2 ; les membres sont dépliés d’un niveau comme le dit 5.2, et non comptés par éléments : après `%a = {%b_set %lbl}`, `%a{2}` est le deuxième membre de b_set, ou le texte lorsque b_set a un seul membre. `end` est le dernier membre. Les éléments sont développés dans l’ordre écrit et joints ; les répétitions sont retenues. `a:b` va de a à b, à rebours lorsque a est supérieur à b, et `a:b:s` par pas de s sans dépasser b.
- **Un seul ou un ensemble** : un élément unique qui est une place unique donne le membre lui-même. Tout autre cas donne un nouvel ensemble des membres à ces places, dans cet ordre : un nouvel objet ensemble, seul cas où une référence crée un objet (5.1). Un membre qui est lui-même un ensemble (un `{…}` écrit tel quel, 5.2) est pris tel quel.
- **Au-delà des membres** : une place que l’ensemble ne possède pas ne donne rien : une telle place unique donne un ensemble vide, et dans une liste elle est omise. Ce n’est pas une erreur, car le nombre de membres d’un ensemble peut ne pas être connu au moment où l’instruction est écrite (comme pour un ALIGN d’un seul membre, 14.2).
- **Uniquement des places** : les accolades juste après une référence ne contiennent que des places. `%a{#b}` est une erreur de syntaxe ; pour écrire une référence puis un ensemble, séparez-les par un espace blanc, comme dans `%a {#b}` (3.5).
- **Erreurs** :
  - Les places d’un objet qui n’est pas un ensemble, tel qu’une instance, échouent (mauvaise sorte).
  - Une utilisation n’est pas un ensemble : ses membres sont pris par rôle, tel que `%dp1::left` (6.5), et des places après une utilisation sont une erreur.
  - Les places du résultat d’une sélection sont une erreur, que la sélection soit écrite là ou liée au nom, tout comme les places de tout ensemble qui dépend d’une sélection, par ses éléments, UNION, ADD-TO ou REMOVE-FROM (10.4), tel que `%a{1}` après `%a = {%s}` avec `%s` une sélection : leurs membres ne sont connus que lorsque la phase qui les utilise les évalue (5.2, 8.2).

#### 5.2.4 Objets donnés pour une autre sorte

Lorsqu’un emplacement prend une sorte et reçoit un objet d’une autre sorte, l’objet n’est converti que selon le tableau ; toute autre discordance est une erreur de sorte à la lecture du texte (5.1, 7.1), quel que soit le sens envisageable. Un objet ne se convertit qu’en ce qu’il représente sans choix : vers son dessin, d’une borne à sa broche et de là à un point, et d’un objet dessiné à sa position ; vers le circuit, d’une broche à la borne qu’elle lie, et d’une broche ou d’une borne à son net. Pour tout autre cas, écrivez le sous-objet (`::position`, 6.5) ou une sélection (10.1).

| Donné \ l’emplacement prend | point (graphiques) | point ou broche (fils) | position (membres de contraintes) | broche | borne | net |
|---|---|---|---|---|---|---|
| point | = | = | = | non | non | non |
| broche | 1 | = | 1 | = | 8 | 9 |
| borne | 2 | 3 | 2 | 3 | = | 10 |
| instance, faisceau | non | non | 4 | non | non | non |
| utilisation | non | non | 5 | non | non | non |
| texte | non | non | 6 | non | non | non |
| graphique écrit avec un centre | non | non | 7 | non | non | non |
| autre graphique, fil | non | non | non | non | non | non |
| net | non | non | non | non | non | = |

« = » indique la même sorte ; « non » est une erreur de sorte ; un numéro renvoie à une conversion ci-dessous. Une conversion depuis ou vers une broche nécessite le symbole de l’objet ; elle s’effectue donc une fois les symboles attribués (8.2) ; les étapes précédentes voient l’objet tel qu’il est écrit. Là où un emplacement ou un rôle prend un objet unique, une telle conversion qui donne un ensemble, comme à partir d’une broche de bus, fait échouer l’instruction lors de cette vérification (7.1) : l’instruction a été créée avec un seul objet et n’est pas scindée après coup. Un fil recevant une broche aboutit sur cette broche, ce qui le connecte (10.2, 13.1) ; il ne se contente pas de prendre les coordonnées de la broche.

1. **Broche vers point ou position** : la position de la broche.
2. **Borne vers point ou position** : la position de sa broche, comme en 3.
3. **Borne vers broche** : la broche qui lie la borne dans le symbole de son objet (9.2.4). Lorsqu’il n’y en a aucune, l’instruction échoue (9.2.4) ; lorsqu’il y en a plusieurs, c’est celle écrite en dernier dans le symbole. Pour une borne d’un faisceau, la broche du symbole du faisceau (11.8).
4. **Instance ou faisceau vers position** : son origine, `::position` (6.5). Une instance ne se convertit pas en point d’un fil ou d’un graphique : pour terminer un fil ou dessiner un graphique à l’origine, écrivez `#M1::position`.
5. **Utilisation vers position** : son point propre (14.1). Une utilisation ne se convertit pas en point d’un fil ou d’un graphique : pour terminer un fil ou dessiner un graphique à son point, écrivez `%u::position` (6.5).
6. **Texte vers position** : la position du texte lui-même (13.3).
7. **Graphique écrit avec un centre vers position** : le centre (13.4). Un graphique écrit avec deux points, LINE, ARC et BEZIER ont plusieurs points et ne se convertissent pas.
8. **Broche vers borne** : la borne que la broche lie (9.2.4) ; pour une broche de bus, l’ensemble de ses bornes. Une broche qui ne lie aucune borne, telle qu’une broche d’une utilisation ou une broche uniquement dessinée, n’en a pas, et l’instruction échoue.
9. **Broche vers net** : son `::net` (6.5), le net de la borne qu’elle lie ou le net qui remplit le rôle qu’elle lie ; pour une broche de bus, l’ensemble de ces nets. Une broche qui ne lie rien n’en a pas, et l’instruction échoue.
10. **Borne vers net** : son `::net` (6.5) ; pour une borne d’un faisceau dont les membres sont sur des nets différents, l’ensemble de ces nets (11.8).

- PLACE ne prend que les objets énumérés en 11.1 : une borne ou une broche ne se déplace qu’avec son objet, et ne se place pas isolément (14.1).
- Un ensemble n’est pas converti : un emplacement qui prend un objet unique effectue son travail pour chaque membre (5.2.1), et dans les contraintes une place contenant un ensemble représente chacun de ses membres (14.1).
- Le nœud vide, `&` seul (6.3), n’est pas converti non plus : un rôle de sorte `"net"` et l’emplacement de net de WIRE le prennent tel quel, et tout autre endroit prenant un net le prend comme un ensemble vide de nets.
- Un net ne se convertit en rien : il n’a ni position ni broche, et ne représente aucune borne unique. Un objet ne se convertit pas non plus en plusieurs objets d’une autre sorte : une instance ne devient pas ses bornes. Pour cela, écrivez une sélection, telle que `#M1 @SELECT-TERMINAL` ou `&en @SELECT-PIN` (10.1).
- Ne sont pas des conversions et sont traités ailleurs : ce à quoi un TEXT est attaché (13.3), comment la portée d’une sélection mène à la sorte sélectionnée (10.1), et WIRE recevant un fil (13.1).

### 5.3 Instructions et signatures

- Une instruction est constituée d’au plus un mot-clé, de certains objets et d’une liste de paramètres.
- Chaque mot-clé possède une signature, qui déclare :
  - le nombre d’objets, ainsi que le nom de chaque emplacement et les sortes d’objets qu’il prend. Pour AS et ATTACH, le nombre est la longueur de la liste de rôles de la définition plus un pour la définition elle-même (écrite en dernier pour AS, en premier pour ATTACH), et les sortes sont celles déclarées pour les rôles (9.4) ;
  - les paramètres : leurs noms, types, s’ils sont requis, leurs valeurs autorisées, et l’ordre dans lequel ils peuvent être donnés par position ;
  - la sortie : un nouvel objet, la poignée de l’entrée, un sous-objet, une définition, ou aucune poignée ; WIRE donne un nouveau fil lorsqu’il reçoit un net, et le fil qu’il reçoit lorsqu’il reçoit un fil (13.1) ;
  - comment chaque entrée est consommée (5.4), et si elle est stricte ou tolérante aux échecs (7.3) ;
  - où le mot-clé peut être écrit (voir la liste de mots ci-dessous).
- Le nombre d’objets est fixe ; un nombre variable d’objets est donné sous forme d’ensemble. Il n’y a pas d’emplacements facultatifs, à ces exceptions près : un rôle d’ensemble d’AS et ATTACH qui n’est pas rempli est un ensemble vide (9.4) ; la définition de SELECT-USAGE peut être omise (10.1) ; le corps d’USE-PACKAGE peut être omis (16.2). Une définition occupe également un emplacement.
- Les objets sont mis en correspondance avec les emplacements par rôle ou par ordre, tous marqués ou aucun ; un rôle ne peut être marqué qu’une seule fois. L’objet transmis par un chaînage est le premier objet ; AS et ATTACH le font correspondre par rôle, et il ne prend pas l’emplacement de la définition (5.5). La manière dont les objets sont mis en correspondance avec les emplacements figure en 5.2.1.
- Les objets écrits dans une instruction peuvent être marqués avec des rôles, quel que soit le mot-clé. Le nom de rôle est le nom d’emplacement dans la signature : `#M5:3(to) #M1:3(from) @DISTANCE("left-to-right", min=1)` équivaut à `#M1:3 #M5:3 @DISTANCE("left-to-right", min=1)` écrit dans l’ordre. Un nom de rôle qui n’est pas un nom d’emplacement du mot-clé fait échouer l’instruction ; il n’est pas ignoré (Principes 2.3, 2.8). Les objets d’un mot-clé à un seul emplacement peuvent être marqués aussi, mais le marquage n’ajoute rien.
- Les paramètres d’AS et ATTACH sont ceux de la liste de paramètres de la définition (9.4), plus un paramètre standard, `priority` (14.8) ; aucun paramètre d’une définition ne peut s’appeler `priority`.
- **Liste de mots standard** : chaque version du langage publie une liste lisible par machine de ses mots standards, qui fait partie de la norme. Ce que prend chaque mot-clé, et quels paramètres il possède, ne sont donnés que dans cette liste ; le texte principal en donne le sens. L’Annexe A est générée à partir de la liste, et une contradiction entre les deux est un erratum. Les vérifications d’un interpréteur, la documentation, la complétion dans les éditeurs, le squelette des tests de conformité et les références d’écriture pour les LLM sont générés à partir de la liste. Pour la version 0.4, la liste est publiée sous le nom de `rnl-words-0.4.json`.
  - La liste contient tous les mots standards : les mots-clés avec leurs signatures, noms d’emplacements, noms et valeurs de paramètres, les champs de l’en-tête d’enregistrement, noms de sortes, noms de sous-objets, noms d’attributs que lit `${…}`, propriétés de style, et `true` et `false`. Les alias (16.5) sont déclarés pour des mots de la liste, et les vérifications de version et suggestions du nom le plus proche y recherchent les mots.
  - La liste est un fichier JSON, et le JSON Schema `rnl-words-0.4.schema.json` décrit la structure de la liste elle-même. Les outils et les mainteneurs de la norme la lisent et l’écrivent, et non ceux qui décrivent des circuits. Chaque mot possède `since`, la version qui l’a ajouté ; les mots réservés sont marqués (la version 0.4 ne réserve aucun mot-clé). Sous `removed`, la liste nomme les mots standards des versions antérieures que cette version ne possède plus, avec la section qui indique ce qui les remplace, afin qu’un diagnostic puisse le signaler. Un mot qui ne figure pas dans la version qu’un fichier déclare est traité comme un mot inconnu, et le diagnostic indique quelle version est requise (16.1).
  - La liste consigne, pour chaque mot-clé :
    - chaque emplacement : son nom, les sortes qu’il prend, s’il prend un objet unique ou un ensemble (5.2.1), et comment il est consommé (5.4), ce qui détermine s’il est strict ou tolérant (7.3) ; l’emplacement de la définition ; pour AS et ATTACH, les emplacements suivent la liste de rôles de la définition ;
    - chaque paramètre : son nom ; son type (nombre, chaîne, valeur de vérité, ou tableau avec le type de ses éléments ; là où les valeurs sont fixes, les valeurs) ; s’il est requis ; sa valeur par défaut ; l’ordre des paramètres qui peuvent être donnés par position (les autres ne sont donnés que par nom) ; et les conditions sur les combinaisons, telles qu’au moins un parmi `x` et `y` de PLACE, ou une seule parmi la forme par centre et la forme par coins de RECTANGLE ;
    - la sortie : un nouvel objet et sa sorte, l’entrée transmise plus loin, la cible transmise plus loin (ADD-TO), une définition, un sous-objet (TEXT avec `key=`), ou aucun ; pour WIRE, également la sorte d’objet qu’il réécrit lorsqu’il en reçoit un (13.1) ;
    - où il peut être écrit : dans un niveau ; uniquement dans un corps de symbole (PIN) ; uniquement dans l’enregistrement d’en-tête (tels que VERSION, ALIAS, DEVICE) ; uniquement dans l’enregistrement d’en-tête d’un fichier de paquet (PACKAGE) ;
    - comment ses faits sont surchargés (5.8) : par clé, avec la clé ; en tant que faits géométriques (8.5) ; par les files d’apparence (8.3) ; ou cumulés ;
    - sa forme standard et sa phrase standard (4.3, 4.5, Annexe B) ;
    - une description d’une ligne et la section du texte principal.
  - Ne figurent pas dans la liste : le sens (quels faits produit un mot-clé, dans quelle phase, comment ils entrent en conflit) est donné dans le texte principal. La signature d’une définition générale dans un paquet est constituée de sa liste de rôles et de sa liste de paramètres (9.4).

  <!-- JSON excerpt: added by the integrator -->

- **Noms d’emplacements des mots-clés intégrés** : chaque emplacement a un nom, qui est le nom de rôle pouvant être marqué. Les noms d’emplacements sont des mots standards et sont versionnés comme les mots-clés : en renommer un est une modification incompatible ; les emplacements d’un même mot-clé ont des noms différents. L’emplacement d’une définition a aussi un nom, utilisé dans la liste et dans les diagnostics, mais il ne peut pas être marqué (4.2 point 7). Il existe un jeu unique de noms de sortes, utilisé à la fois dans les signatures et dans les déclarations de rôles des définitions (9.4), `instance`, `terminal`, `pin`, `net`, `point`, `wire`, `usage`, `bundle`, `text`, `annotation` et `label` (Annexe A).

### 5.4 Modes de consommation de l’entrée

| Mode | Mots-clés | Faits de l’entrée | Échecs (par défaut) | Noms |
|---|---|---|---|---|
| absorber | la plupart des mots-clés | portés vers la sortie | stricts | les noms dans un ensemble imbriqué ne valent que dans cet ensemble |
| enclore | DEFINE, DEFINE-SYMBOL, DEFINE-STYLE | deviennent le contenu de la définition et ne sont pas transmis | tolérants | locaux ; renommés pour chaque application, afin que chaque utilisation ait les siens (9.4) |
| verser | l’enregistrement, `@RNL` | fusionnés dans le niveau dans l’ordre | tolérants | publiés dans le niveau et partagés par tous ses enregistrements ; peuvent être utilisés avant d’être liés |

- Ce que produit un membre est l’ensemble des faits créés par ses segments et par les ensembles qui y sont imbriqués, et non seulement la poignée à son extrémité. Ils sont versés avec le membre de premier niveau qui les contient.
- Chaque instruction d’un enregistrement est versée, qu’elle soit nommée ou non. Ce qu’elle fait ensuite dépend de l’objet : une description dans la scène est dessinée ou prend part à la résolution ; une définition devient disponible et ne dessine rien par elle-même ; les références, et les ensembles constitués uniquement de références, n’ont aucun effet. Les instructions dans un `{…}` ordinaire sont versées avec le membre qui le contient, si bien que la ligne de `{ @LINE {…}}` est dessinée (13.4).
- Les instructions dans le corps d’une définition ne sont pas versées ; elles deviennent le contenu de la définition. Une définition générale n’est développée que lorsqu’elle est appliquée (9.4).
- Un USE-PACKAGE comportant un corps prend aussi le corps par mode enclore : les instructions du corps constituent le contenu du paquet et ne sont pas versées dans le niveau. La manière dont les noms d’un paquet sont vus figure en 16.2.

### 5.5 Chaînages et imbrication

- `a => @K(…)` fait du résultat de `a` le premier objet de K ; AS et ATTACH le placent dans le premier rôle, et non dans l’emplacement de la définition. Un résultat qui est un ensemble reste un seul objet (5.2.1). Le segment après `=>` ne prend cet objet que par position et ne peut pas le marquer avec un rôle.
- Un chaînage ne s’exécute pas étape par étape. Il se développe en plusieurs descriptions reliées par des poignées : chaque segment décrit la poignée qui résulte du segment précédent, et un segment qui travaille membre par membre se développe en descriptions parallèles (1.5 point 1).
- **L’ordre des segments.** Chaque segment a sa propre place, et un segment ultérieur compte comme écrit après un segment antérieur partout où l’endroit où l’on écrit importe (1.5 point 2) : dans la surcharge par clé (8.4), dans les files d’apparence (8.3), dans l’ordre de résolution (8.5), dans l’ordre de tracé (11.6) et pour les réécritures d’un fil (13.1). Ainsi dans `#M1 @SET-SYMBOL $A => @SET-SYMBOL $B`, M1 est dessiné avec `$B`.
- **Un échec n’atteint que les descriptions qui en dépendent.** Une description échoue pour l’une de ces deux seules raisons : elle est incompréhensible ou se contredit elle-même ; ou une poignée qu’elle utilise aurait dû être créée par une description qui a échoué.
  - Dans `$GROUND @ATTACH &vss => @PLACE("20", 38) => @ORIENT("R90") => %g1`, seul PLACE échoue (une chaîne là où PLACE prend un nombre, 5.3) ; la masse est tout de même créée, et ORIENT prend tout de même effet.
  - Dans `#XI0<3:0> @BUNDLE(bad=1) => @PLACE(10, 20) => %s`, le faisceau n’est pas créé (BUNDLE n’a pas de paramètre `bad`), si bien que PLACE n’a aucun objet à décrire et échoue également.
- Ce qu’un nom lie : `%x =` en tête lie la dernière poignée de tout le chaînage ; `=> %x` lie le résultat du segment à sa gauche, qui est la dernière poignée lorsqu’il est écrit à la fin, et le résultat de ce segment lorsqu’il est écrit au milieu, comme `#M1 @AS $A => %a => @AS $B => %b`. PLACE, ORIENT, SET-SYMBOL, SET-STYLE et GROUP gardent la poignée de leur entrée, donc dans `$GROUND @ATTACH &vss => @PLACE(24, 38) => %g0`, le nom `%g0` désigne l’utilisation de cette application, c’est-à-dire cette masse. AS et ATTACH créent de nouvelles poignées : le deuxième segment de `#M1 @AS $A => @AS $B` applique `$B` à l’utilisation créée par le premier segment (imbrication), et non à M1. Pour appliquer deux définitions à M1, écrivez deux instructions. Lorsque le segment à gauche d’un nom échoue : s’il aurait gardé la poignée de son entrée, le nom désigne toujours cette poignée (`%g1` ci-dessus est la masse) ; s’il aurait créé un nouvel objet, le nom désigne l’objet en échec (`%s` ci-dessus).
- Un objet peut être écrit sous forme d’ensemble contenant des instructions (imbrication). Les noms liés dans un ensemble imbriqué ne valent que dans cet ensemble. Chaque instruction imbriquée est une description à part entière : dans `{%a = #M1 @PLACE(1, 2); #M9} @ALIGN("y")`, lorsque l’hôte n’a pas de M9, ALIGN échoue et les coordonnées de M1 prennent tout de même effet.
- La forme standard écrit les instructions à plat : nommer un objet, puis s’y référer dans d’autres instructions. Une instruction isolée est nommée avec `%x =` devant, un chaînage avec `=> %x` (4.3).

### 5.6 Définitions en tant qu’objets

Un mot-clé qui utilise une définition la prend comme l’un de ses objets, et le mot-clé fixe quel emplacement la reçoit : pour ATTACH c’est le premier objet écrit ; pour SET-SYMBOL, SET-STYLE, AS, SELECT-USAGE et SELECT-ANCESTOR-USAGE c’est le dernier objet, et SELECT-USAGE peut l’omettre (10.1).

```rnl
%ax = (_, _);
%w13 = &x @WIRE {#M1:1 #M3:1};
#M1 @SET-SYMBOL $NMOS;
%w13 @SET-STYLE $signal;
#M1(left) #M2(right) %ax(axis) @AS $MIRRORED_LAYOUT;
$GROUND @ATTACH &vss;
# @SELECT-USAGE $GROUND => @SET-SYMBOL $GROUND_SYMBOL
```

- Une définition n’est pas marquée avec un rôle et ne compte pas pour « tous marqués ou aucun ». C’est un objet et elle occupe un emplacement, si bien que `{#M1 #M2} @SET-SYMBOL $X` représente deux objets (5.2.1).
- Comme le mot-clé fixe l’emplacement de la définition, une instruction peut être analysée sans savoir si un nom en `$` désigne une définition ou un ensemble : dans `$GROUND_STUB @ATTACH $GROUND_FILL_SET`, le premier est une définition et le second un ensemble.
- Une définition ne peut pas être transmise par un chaînage : l’objet transmis remplit le premier emplacement, et AS et ATTACH le font correspondre par rôle, en dehors de l’emplacement de la définition (5.5). Ainsi `$GROUND @ATTACH &vss` ne peut pas s’écrire `$GROUND => @ATTACH &vss`.
- Une définition peut être une définition anonyme imbriquée : `#M1 @SET-SYMBOL { @DEFINE-SYMBOL {…}}`.
- Une définition générale peut être écrite plus loin ou dans un autre niveau ; le nombre d’objets, les noms de rôles et les paramètres d’AS et ATTACH sont donc vérifiés lorsque le niveau prend effet (7.1).

### 5.7 Poignées et noms

- L’hôte fournit les poignées des objets hôtes : noms d’instances et noms de nets. Une borne est une instance et un numéro de borne, si bien que `#M1:1` et `#M1:d` désignent la même borne.
- Pour ce que crée RNL, la poignée provient de l’instruction qui le crée. Une autre instruction ne peut s’y référer que par un nom : le nom est le seul moyen de le retrouver, sauf dans l’enregistrement du solveur, qui désigne un objet sans nom par son numéro (5.9). Un objet sans nom ne peut sinon être utilisé qu’à l’intérieur de sa propre instruction (dans un chaînage ou par imbrication).
- Un nom ne désigne jamais deux choses : il n’est lié qu’une seule fois dans un niveau (6.6).
- La poignée d’un sous-objet est son objet parent et son nom ; il n’a pas besoin de nom propre.
- La poignée d’un objet local créé par le développement d’une définition générale est son utilisation et son nom local, écrit `%c1::mid` depuis l’extérieur (9.4). Les objets locaux d’une utilisation sans nom ne sont indiqués dans les diagnostics que par leur position ; seul l’enregistrement du solveur peut s’y référer, par l’intermédiaire du numéro de l’utilisation, tel que `%#5::mid` (5.9).
- Un objet isolé d’un ensemble généré n’a pas de nom et ne peut pas être référencé isolément : les objets ne sont pas identifiés par leur sorte et les objets qu’ils relient (il n’y a pas de clés naturelles), et les ensembles n’offrent pas de référence par indice.

### 5.8 Clés des faits

- Les faits pourvus d’une clé sont surchargés par clé : pour une même clé, le fait ultérieur prend effet ; lorsque le fait ultérieur échoue, la valeur valide antérieure est retenue.
- Les faits géométriques font exception : le x et le y de PLACE (une clé pour chaque coordonnée) et ORIENT (une clé pour chaque objet, et une pour chaque broche, 11.2). Deux valeurs différentes écrites pour la même clé entrent en conflit et sont résolues selon 8.5 : à priorité égale le fait ultérieur est retenu, ce qui donne le même résultat que la surcharge par clé ; à priorités différentes, celui de priorité supérieure est retenu. PLACE ne modifie que les coordonnées qu’il donne ; une coordonnée jamais donnée reste libre.
- Créer un objet sous un nom déjà lié n’est pas une surcharge par clé : un nom n’est lié qu’une seule fois dans un niveau (6.6). Pour modifier un objet créé par RNL, écrivez une instruction qui le modifie, telle que `%g0 @PLACE(40, 38)`.
- Les faits sans clé s’additionnent : ceux qui ne se contredisent pas valent ensemble, et les conflits sont résolus selon 8.5.
- SET-SYMBOL et SET-STYLE ne sont pas surchargés par clé ; ils prennent effet par les files d’apparence (8.3).
- VERSION et PACKAGE ne s’écrivent qu’une seule fois, dans l’enregistrement d’en-tête (16.1, 16.2), et ne sont donc jamais surchargés : un deuxième est une erreur, il échoue, et le premier prend effet.
- CURRENT a une clé pour chaque borne (15.2), et LAYER une clé pour chaque objet (11.6).
- Les utilisations n’ont pas de clé : une définition appliquée plusieurs fois crée autant d’utilisations, qui ne se surchargent pas mutuellement. Deux applications identiques créent deux utilisations, si bien que `$GROUND @ATTACH &vss` écrit deux fois dessine deux masses. Les faits écrits sur une utilisation par des instructions qui la modifient, telles que PLACE, ont bien des clés et suivent les règles ci-dessus.
- La priorité d’un objet est un fait pourvu d’une clé ; l’ordre dans lequel les priorités se surchargent figure en 14.8.
- Un TEXT pourvu d’une clé (`key=`) représente un texte par couple objet attaché et clé, et un TEXT ultérieur le surcharge paramètre par paramètre (13.3). Un TEXT sans clé n’est pas surchargé par clé : chacun qui est écrit fait un texte de plus (13.3).
- Un fil n’a pas de clé : écrit deux fois, il forme deux fils (13.1). WIRE recevant un fil le réécrit ; la réécriture possède une clé, le fil, de sorte qu’une réécriture ultérieure remplace une réécriture antérieure (13.1).
- Les contraintes n’ont pas de clé. Pour modifier une contrainte, désactivez-la (`"disabled"`, 14.8) et écrivez-en une nouvelle, ou éditez le texte.

### 5.9 Numéros des objets sans nom

```spice fragment
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4")}
.model nch nmos level=1
.subckt amp in out vdd vss
M1 out in vss vss nch
R1 vdd out 10k
* @RNL {
*   #M1 #R1 @ALIGN("x");
*   &out @WIRE {#M1:1 (_, _) #R1:2};         /* %#1: the wire; %#2: the free point */
*   {#M1 #R1} @TEXT("${name}")               /* %#3: the label of M1; %#4: that of R1 */
* }
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig1:…", numbering="…") {
*   #M1 @PLACE(10, 0); #R1 @PLACE(10, 8);
*   %#2 @PLACE(12, 4);
*   %#1 @WIRE(mode="straight") {#M1:1 (12, 2) %#2 (12, 6) #R1:2};
*   %#3 @PLACE(13, 1); %#4 @PLACE(13, 9)
* }
.ends amp
```

- L’enregistrement du solveur désigne par numéro les objets qui ne peuvent pas être atteints par un nom, afin de pouvoir réécrire chaque décision (14.8). Un numéro s’écrit `%#` suivi d’un entier non signé, tel que `%#3`, et s’utilise comme un nom : `%#5::pin` est une broche du symbole de l’utilisation `%#5`, et `%#5::mid` l’un de ses objets locaux. Un numéro ne fait que désigner son objet ; il n’est pas lié comme un nom, donc `%#3 = …` et `=> %#3` sont des erreurs de syntaxe (4.1).
- **Quels objets.** Les points, fils, étiquettes, faisceaux, annotations et utilisations qui n’ont pas de nom propre et ne peuvent pas être atteints par `::` depuis un objet pourvu d’un nom ou d’un numéro (6.5, 9.4). Un nom lié uniquement dans un ensemble imbriqué (6.7) ne compte pas, puisque l’enregistrement du solveur ne peut pas l’utiliser. Pour un enregistrement du solveur, les objets sont, dans cet ordre :
  1. ceux créés par les membres des enregistrements rédigés à la main du niveau (`type="authored"`, 2.8), ainsi que dans les corps des utilisations que ces membres créent. Ils sont comptés sur le niveau interprété tel que le solveur le voit, sans les enregistrements portant `type="generated"` (2.8), et gardent leurs numéros lorsque le niveau est interprété avec ces enregistrements ;
  2. ceux créés par les membres de l’enregistrement lui-même, ainsi que dans les corps des utilisations qu’ils créent. Ils sont comptés sur l’enregistrement tel qu’il est écrit, comme si chaque numéro qu’il contient pouvait être résolu ; le fait que la vérification ci-dessous corresponde ou non ne les modifie pas.

  Les contraintes et les ensembles ne sont pas numérotés. Les objets que les règles génèrent un par un (10.5) ne sont pas numérotés non plus : le solveur les écrit sous forme d’instructions explicites, qui relèvent du point 2. Chaque enregistrement du solveur possède ses propres numéros : le point 1 est identique pour tous, et le point 2 ne compte que les membres propres de l’enregistrement.
- **Ordre.** Les enregistrements rédigés à la main sont pris dans l’ordre de 2.6 : le texte propre du niveau, puis les enregistrements écrits dans le niveau depuis l’extérieur avec `scope`. Au sein d’un membre, les objets sont numérotés dans l’ordre du jeton qui les crée : le `(` d’un point, le mot-clé des autres. Lorsqu’un jeton crée un objet pour chaque membre d’un ensemble (5.2.1), chaque objet possède un numéro propre, dans l’ordre des membres de l’ensemble. Les objets dans le corps d’une utilisation sont numérotés juste après l’utilisation, dans l’ordre du corps, et ainsi de suite de haut en bas à travers les utilisations imbriquées. Les numéros commencent à 1.
- **Où.** Les numéros ne peuvent être écrits que dans les enregistrements portant `type="generated"` qui comportent le champ `numbering` et pas de `scope` (2.6, 2.8). Partout ailleurs, un numéro est une erreur : un enregistrement rédigé à la main désigne les objets par nom. Au sein de son enregistrement, un numéro peut être utilisé avant le membre qui crée son objet, tout comme un nom (6.7).
- **Vérification.** `numbering` contient une vérification de la façon dont les objets numérotés sont créés. Pour chaque numéro, dans l’ordre, il y a une ligne :
  - les jetons du membre de l’enregistrement, ou du corps de définition, qui crée l’objet, avec tous ses segments (4.1), sans texte libre ni commentaires, séparés par une espace. Ici un jeton est une unité lexicale : un mot-clé ; une référence, avec ses bornes, sous-objets et indice, écrite sans espace blanc ; une chaîne ou un nombre tel qu’écrit ; un nom ; `_` ; ou l’un de `{ } ( ) [ ] , = ; =>`, ou une partie de places de membres (`:`, `end`, `-`, 5.2.3). Ainsi `&out @WIRE {#M1:1 (_, _) #R1:2}` donne `&out @WIRE { #M1:1 ( _ , _ ) #R1:2 }` ;
  - pour un objet dans le corps d’une utilisation, la ligne de l’utilisation vient en premier, formée de la même façon que l’utilisation ait un numéro ou non, suivie d’une tabulation et des jetons du membre du corps, tel que la définition l’écrit ;
  - lorsque le jeton crée un objet pour chaque membre d’un ensemble, suivent une tabulation et la poignée du membre, écrite comme une référence : une instance ou un net hôte par son nom ; une borne par son instance et son numéro de borne, tel que `#M1:1` ; une broche par son objet et le nom de la broche, tel que `#X1::vdd_t`, ou, sans nom, par sa borne ; la position d’un objet, ou un texte sur une clé, par l’objet suivi de `::position`, ou de `::` et de la clé ; un objet RNL par un nom en `%` que le niveau lui lie, celui lié en premier dans l’ordre de 2.6 lorsqu’il y en a plusieurs, quel que soit le nom avec lequel l’ensemble a été écrit ; un objet sans un tel nom par son numéro, ou, en tant qu’objet local d’une utilisation, par la référence de l’utilisation, formée selon ces mêmes règles, `::` et son nom local : le nom lié en premier dans le corps de la définition lorsqu’il y en a plusieurs, y compris au sein d’un même membre (dans `%p = (0, 0) => %q`, c’est `p`), quel que soit le nom avec lequel l’ensemble a été écrit, et via l’utilisation appliquée en premier lorsque plusieurs utilisations l’ont comme objet local. Un nom s’écrit tel quel lorsqu’il n’est composé que de lettres ASCII, chiffres et `_`, et sinon entre guillemets doubles, avec chaque `"` doublé (3.6). Par exemple, après `%p = (0, 0); %q = %p; %s = {%q}`, l’annotation que crée `%s @TEXT("x")` a la ligne `%s @TEXT ( "x" )`, une tabulation et `%p`, et la vérification `675df5171032726d`. Avec `$D = @DEFINE(roles=["x"]) { %p = (0, 0) => %q }; %u = #M1 @AS $D; %s = {%u::q}`, la ligne de `%s @TEXT("x")` se termine par une tabulation et `%u::p`, et la vérification est `a8c9571193175903`.

  La vérification est constituée des 16 premiers chiffres hexadécimaux, en minuscules, du SHA-256 de ces lignes en UTF-8, chacune terminée par un saut de ligne.
- Lorsque la vérification d’un enregistrement ne correspond pas au niveau, les instructions de l’enregistrement qui utilisent un numéro sont sans effet, et, lorsqu’il y en a, l’enregistrement donne une indication : la solution est périmée pour les objets sans nom. Un enregistrement qui n’utilise aucun numéro n’a pas besoin de `numbering`. Les autres instructions de l’enregistrement prennent effet normalement (chapitre 17). Un numéro au-delà du dernier objet numéroté est une erreur.
- Dans l’exemple, la vérification comporte une ligne pour chacun des six numéros : le membre WIRE deux fois (pour le fil, puis pour le point), le membre TEXT deux fois, suivi de `#M1` et de `#R1`, et le WIRE de réécriture deux fois de plus, puisque les deux points écrits comme coordonnées dans celui-ci sont des objets de l’enregistrement lui-même, portant les numéros 5 et 6. Une visionneuse qui ne résout pas dessine le point libre, le tracé du fil et les deux étiquettes tels que le solveur les a choisis.
- Écrire `{#R1 #M1}` à la place échange les étiquettes 3 et 4 : la vérification ne correspond plus, et les instructions comportant des numéros sont sans effet jusqu’à ce que le niveau soit résolu à nouveau, tandis que M1 et R1 gardent leurs positions. Écrire l’ALIGN différemment, ou déplacer M1, laisse les numéros tels quels.
- Note : la vérification ne couvre que la façon dont les objets numérotés sont créés. Déplacer un composant, ou modifier une instruction qui ne crée aucun objet sans nom, laisse les numéros tels quels ; insérer, retirer, modifier ou réordonner un membre qui crée de tels objets les rend périmés jusqu’à ce que le niveau soit résolu à nouveau. Un enregistrement du solveur ne contient normalement aucune instruction qui modifie des ensembles ou des définitions ; le point 1 est compté sans ces enregistrements de toute façon.

<!-- translated from en/ch06.md sha256:01f4618ca3e4 -->
## 6 Noms et portée

### 6.1 Hiérarchie des niveaux

- **Les niveaux suivent le texte.** Le RNL d’un niveau s’applique à ce niveau et aux niveaux qu’il contient dans son texte ; quel sous-circuit instancie quel autre n’entre pas en ligne de compte.
  - Un fichier contient les sous-circuits qu’il définit et les fichiers qu’il inclut.
  - Un `.include` écrit dans un sous-circuit, ainsi qu’une définition `.subckt` imbriquée dans un sous-circuit (une définition locale dans SPICE3 et ngspice), se trouvent au-dessous de ce sous-circuit.
- Depuis le sommet, la chaîne des niveaux jusqu’à un sous-circuit est : les paquets à la racine (les paquets d’USE-PACKAGE dans les enregistrements d’en-tête, le paquet standard en premier parmi eux, 16.3) → le niveau de fichier du fichier principal → … (les fichiers inclus, l’un dans l’autre) → le fichier qui définit le sous-circuit → le sous-circuit lui-même. Par exemple :

  ```text
  packages at the root (such as the standard package)
  └─ file level of tb.sp (the top level)
     ├─ load                    a subcircuit defined in tb.sp
     ├─ file level of ota.sp    a file included by tb.sp
     │  ├─ ota                  ota instantiates cell
     │  └─ cell
     └─ file level of bias.sp
        └─ bias                 also instantiates cell, but neither the RNL of bias.sp nor that of bias reaches cell
  ```

- **Le RNL d’un sous-circuit ne porte que sur le sous-circuit.** Lors de la résolution des conflits, ses instructions viennent après le contenu des niveaux de fichier, et ont donc priorité sur lui (8.5) ; mais elles ne sont pas transmises aux sous-circuits qu’il instancie, sauf à ceux qui sont définis à l’intérieur de lui. Lorsque amp instancie cell, les définitions et règles d’amp n’atteignent pas cell.
- **Un fichier inclus n’affecte pas le niveau qui l’inclut.** Les définitions, règles et USE-PACKAGE de son niveau de fichier ne s’appliquent qu’à ses sous-circuits et aux fichiers qu’il inclut à son tour. Dans la netlist, les composants qu’il écrit hors des sous-circuits appartiennent au niveau qui l’inclut, mais son RNL ne peut pas les modifier. Un fichier n’est donc jamais modifié par les fichiers qu’il inclut.
- **Le niveau de fichier d’un fichier inclus n’a pas de circuit propre.** Seul le niveau de fichier du fichier principal est un circuit. Les références hôtes (`#M1`, `&vss`) au niveau de fichier d’un fichier inclus échouent (6.3) ; seuls les définitions, règles, USE-PACKAGE et `scope` ont effet à cet endroit. Un fichier ouvert isolément est le fichier principal.
- **Un fichier inclus hérite du niveau qui l’inclut.** Le style maison du fichier principal (ses règles et les paquets de son USE-PACKAGE) descend dans les sous-circuits des fichiers de bibliothèque. Ainsi un même fichier de bibliothèque peut être dessiné différemment sous différents fichiers principaux, ou lorsqu’il est ouvert isolément, tandis que le résultat de son solveur n’est stocké qu’une seule fois (chapitre 17). Pour garder les symboles utilisés par une bibliothèque à l’abri des règles d’apparence des niveaux supérieurs, utilisez à nouveau le paquet standard avec USE-PACKAGE dans le fichier de bibliothèque ou le sous-circuit (16.2). Pour fixer uniquement la version du paquet standard tout en laissant le style maison du dessus le supplanter, écrivez l’USE-PACKAGE dans l’enregistrement d’en-tête du fichier de bibliothèque (16.3). Les règles du fichier principal ne s’appliquent qu’à son niveau supérieur, à moins que leur sélection n’écrive `recursive=true` (10.1) ; les règles d’un style maison devant atteindre les fichiers de bibliothèque l’écrivent.
- **Fichiers Verilog-A.** Un fichier Verilog-A que la netlist importe avec `ahdl_include` ou `.hdl` se place là où il est importé, comme le fait un fichier inclus, et chacun de ses modules est un niveau au-dessous de son niveau de fichier, lequel n’a ni circuit ni schéma et ne fait que donner au module son symbole de bloc (2.9).
- **Un seul schéma pour chaque sous-circuit.** Une visionneuse ne développe pas les sous-circuits : une instance d’un sous-circuit est toujours dessinée avec son symbole, et l’intérieur du sous-circuit est dessiné et résolu séparément. Chaque sous-circuit possède une seule chaîne de niveaux supérieurs, si bien que dans un document chaque sous-circuit a exactement un schéma.
- **Paquets.** Les définitions en `$` d’un paquet prennent effet dans le niveau où `@USE-PACKAGE` est écrit et dans les niveaux inférieurs ; ses règles comptent comme écrites dans ce niveau et atteignent les niveaux inférieurs avec `recursive=true` (16.2, 10.4). Les paquets d’USE-PACKAGE dans un enregistrement d’en-tête sont placés à la racine, le paquet standard parmi eux lorsqu’il est écrit ou ajouté par un réglage de l’outil (16.3). Une inclusion place un fichier au-dessous du niveau, et le fichier hérite du niveau ; un USE-PACKAGE place un paquet au-dessus du niveau, et le niveau hérite du paquet.
- **Les règles s’appliquent à leur propre niveau par défaut.** Une règle écrite dans un niveau n’est évaluée que dans ce niveau. Lorsqu’elle atteint les niveaux inférieurs, par `recursive=true` dans une sélection qu’elle écrit ou, si elle n’en comporte pas, par les ensembles qu’elle utilise (10.1), elle est également évaluée une fois dans chaque niveau inférieur, en n’examinant à chaque fois que ce seul niveau (10.4). C’est ce que demande Principes 2.5 aux règles : une règle peut limiter son effet au niveau courant ou laisser les niveaux inférieurs en hériter, et s’applique à son propre niveau par défaut, tandis que les définitions en `$` sont héritées par défaut (6.8).

### 6.2 Préfixes

| Préfixe | Désigne | Résolu dans |
|---|---|---|
| `#` | Une instance de l’hôte ; `#` seul est ce niveau | L’hôte de ce niveau |
| `&` | Un net de l’hôte ; `&` seul est le nœud vide, aucun net (6.3) | L’hôte de ce niveau |
| `%` | Un nom donné dans ce niveau : d’un objet de la scène, d’un objet existant tel que `%tail = #M5`, ou d’une définition utilisée uniquement dans ce niveau | Ce niveau |
| `$` | Une définition, ou un ensemble (6.6) | Le niveau où il est lié, et les niveaux inférieurs |
| `?` | Un rôle ou un paramètre d’une définition ; `?` seul est, dans un corps de symbole, l’objet dessiné avec le symbole (13.3), et dans le corps d’une définition générale, l’utilisation en cours de développement (9.4) | Le corps d’une définition générale ou d’un symbole |

`$X` et `%X` sont deux noms différents.

### 6.3 Références hôtes

- `#M1` et `&vss` ne sont résolus que dans l’hôte de ce niveau, jamais dans un autre niveau (Principes 2.5). Des instances portant le même nom dans différents sous-circuits sont sans lien.
- Une référence hôte introuvable échoue (Principes 2.4). Le niveau de fichier d’un fichier inclus n’a pas de circuit propre, donc toute référence hôte y échoue (6.1).
- Un nom peut être suivi directement d’un indice, qui représente un ou plusieurs noms hôtes, tels que `&D<3>` et `#XI0<3:0>` (6.11).
- L’adaptateur hôte décide quels nets possède un niveau, selon la sémantique de l’hôte :
  - Nets locaux : les nœuds qu’utilisent les lignes de la netlist du niveau, ports compris.
  - Nets globaux : `0` est la masse globale dans chaque dialecte SPICE. Les alias de masse qu’un simulateur connaît lui-même (tels que `gnd` dans ngspice, et `GND`, `GND!`, `!GND` et `GROUND` dans HSPICE) sont fusionnés dans `0` par l’adaptateur du dialecte ; après fusion, `&gnd` et `&0` sont la même poignée. Un nom déclaré avec `.GLOBAL vdd` (`global vdd` dans Spectre) est le même nœud dans chaque sous-circuit. Un nom se terminant par `!`, tel que `vdd!`, est une convention des schémas Virtuoso ; ce qui compte est la déclaration.
  - Un net global possède une poignée dans chaque niveau qui l’utilise : la copie dans ce niveau. Un niveau dont les lignes de netlist ne l’utilisent pas n’en a pas de copie. Ainsi `&0` désigne la copie dans le niveau où il est écrit : chaque schéma dessine la sienne, les faits appartiennent à ce niveau, et les descriptions d’un niveau inférieur ne peuvent pas modifier un niveau supérieur.
  - Pour désigner le net global dans chaque niveau, sélectionnez-le par nom dans un niveau supérieur avec la condition `global=true` (10.1).
- **Le nœud vide.** `&` écrit seul, sans nom, indique qu’il n’y a aucun net : ce qui le reçoit n’est connecté à aucun nœud électrique du circuit. Il s’écrit là où un net est attendu et où il n’en existe aucun, comme pour les broches de composants virtuels qui dessinent un schéma-bloc sans lignes de netlist : `$INTEGRATOR @ATTACH &(in) &y(out)`, `& @WIRE {%sub1::y %int1::a}`.
  - Ce n’est pas un objet : il n’a pas de poignée, ne peut pas être lié à un nom, ne peut pas être membre d’un ensemble et n’est jamais sélectionné. Deux endroits qui écrivent `&` n’ont rien en commun ; une visionneuse qui met en surbrillance une broche ou un fil sur le nœud vide ne met en surbrillance que celui-ci.
  - Deux endroits le prennent tel quel : un rôle dont la sorte est `"net"` (9.4) et l’emplacement de net de WIRE (13.1). Une broche liée à un rôle qu’il remplit n’est sur aucun net, et son `::net` est le nœud vide (12.1) ; `${net}` et `${p}` de celle-ci sont le texte vide (13.6).
  - Partout ailleurs où un net est attendu, il compte comme un ensemble vide de nets : ROUTE-NET ne relie rien, ASSERT-NET est trivialement vérifié, une sélection qui l’a pour portée ne sélectionne rien, et ADD-TO n’ajoute rien (5.2.1). Écrit tel quel à cet endroit, il produit un avertissement, comme le fait un ensemble vide écrit tel quel ; atteint par l’intermédiaire d’un rôle, comme dans le corps d’une définition, il n’en produit aucun. Un corps qui utilise le net d’un rôle n’a donc pas besoin d’un cas particulier pour lui.
  - Ce n’est pas un net de l’hôte, donc `&` suivi d’un nom que l’hôte ne possède pas échoue toujours comme auparavant.

### 6.4 Bornes

- `object:terminal` est une borne : `#M1:1`, `#M1:d`, `#XU1:inp`, ainsi qu’une borne d’un faisceau telle que `%x:in` (11.8). `#:inp` est le port inp de ce niveau. Seuls les instances hôtes, les faisceaux et les ports d’un niveau ont des bornes ; une utilisation n’en a aucune (12.1).
- Un nombre est le numéro de borne de l’hôte, compté à partir de 1. Une borne écrite uniquement avec des chiffres est un nombre, même lorsqu’elle est entre guillemets (4.1), partout où une borne est nommée : dans les références en `:`, dans `terminal=` de PIN, dans `terminals=` de SET-SYMBOL, et dans la condition `terminal` des sélections. Ainsi un nom de port composé uniquement de chiffres, comme dans `.subckt amp 3 1 2`, est atteint par sa place, et `#X1:1` est le port nommé 3 ; un nom dans DEVICE composé uniquement de chiffres est une erreur (6.12).
- Un nom est un nom de borne de l’hôte : un nom donné par `@DEVICE` dans l’enregistrement d’en-tête (6.12), ou un nom de port du sous-circuit. Un nom qui ne peut pas s’écrire sans guillemets (3.6) est mis entre guillemets, tel que `#XU1:"in+"`. Un nom de borne ou de port peut être suivi directement d’un indice, tel que `#X1:D<7:0>` et `#:D<7:0>` (6.11).
- Casse : les noms de ports des sous-circuits et les noms de bornes donnés par DEVICE suivent les règles de l’hôte : SPICE ignore la casse, donc `#M1:D` fonctionne aussi ; Spectre n’ignore pas la casse.
- Le `label=` d’une broche ne sert qu’à l’affichage et ne peut pas être utilisé pour désigner une borne : les bornes appartiennent à la netlist, et une référence à l’une d’elles ne dépend pas du symbole utilisé.
- **Bornes et broches** : une borne appartient à la netlist ; une broche appartient au symbole d’un objet dessiné (9.2.4). Une borne est dessinée aux broches qui la lient, et là où une broche ou un point est nécessaire, une borne représente sa broche (5.2.4). Une broche s’écrit par l’intermédiaire de sa borne, telle que `#M1:d`, ou, lorsqu’elle porte un nom dans son symbole, par ce nom en tant que sous-objet, tel que `#X1::vdd_t` (6.5) ; un nombre n’est jamais un nom de broche.
- Une borne introuvable échoue. Le fait qu’une borne possède une broche dans son symbole est vérifié après que les symboles ont été décidés (7.1).
- Mis à part les noms de ports des sous-circuits, les noms de bornes sont donnés par DEVICE ; 6.12 énumère les noms standards de la famille SPICE.
- Un sous-circuit que la netlist ne définit pas (tel que le composant de PDK dans `XM1 … sky130_fd_pr__nfet_01v8`) n’a de noms de bornes que lorsque l’utilisateur les donne avec DEVICE dans l’enregistrement d’en-tête (6.12), ou utilise un paquet contenant des instructions DEVICE (16.2) ; sinon seuls les numéros peuvent être utilisés.

### 6.5 Sous-objets

- `object::name` est un sous-objet. C’est toujours un objet, jamais une valeur. Sa poignée est l’objet parent et le nom, si bien que deux noms forment deux membres partout où les membres sont comptés avant l’attribution des symboles (5.2.4) : `{#R1::p #R1::q} @UNION` comporte deux membres, même lorsque les deux noms finissent par désigner une seule et même broche.
- Les sous-objets sont :
  - Une clé d’un texte : le sous-objet qu’un TEXT avec `key=` crée sur l’objet auquel il est attaché, tel que `#M1::wl`. Un texte avec une clé dans un symbole est un sous-objet de chaque objet dessiné avec le symbole (13.3). `::name` et `::value` ne sont pas des sous-objets standards, seulement des clés usuelles (13.3).
  - `::position` : l’origine de l’objet. `#M1::position @PLACE(…)` équivaut à `#M1 @PLACE(…)`. Pour une utilisation, c’est son point propre (14.1), l’origine de son symbole lorsqu’elle en a un (12.1). Pour un ensemble, c’est l’ensemble des `::position` de chaque membre, les ensembles imbriqués étant suivis vers l’intérieur : `{%dp1 %dp2}::position` vaut `{%dp1::position %dp2::position}`, les deux points propres, tandis que `{%dp1 %dp2}` dans une contrainte représente les six transistors (14.1). Pour un TEXT, c’est la position du texte lui-même (13.3) ; pour un RECTANGLE, CIRCLE ou ELLIPSE écrit sous forme par centre, le centre (13.4).
  - `::net` d’une borne : le net sur lequel elle se trouve, tel que `#M5:1::net`, qui est `&tail`. Pour les bornes d’un faisceau, voir 11.8 : lorsque les membres sont sur des nets différents, c’est l’ensemble de ces nets. `::net` d’une broche : le net de la borne qu’elle lie, ou de ce qui remplit le rôle qu’elle lie (12.1) ; pour une broche de bus, l’ensemble de ces nets ; une broche qui ne lie rien n’en a aucun.
  - Une broche nommée du symbole de l’objet : `#X1::vdd_t` est la broche nommée vdd_t dans le symbole de X1, `%g0::pin` la broche nommée pin dans le symbole de l’utilisation `%g0` (9.2.4).
  - Un nom de rôle d’une utilisation : ce qui remplit ce rôle, tel que `%dp1::left`, qui est M1, et `%g0::p`, qui est le net vss. Pour un rôle d’ensemble, c’est cet ensemble. C’est cet objet lui-même et non un second objet : `{%dp1::left #M1} @UNION` n’a qu’un membre, et un PLACE écrit pour `%dp1::left` porte la clé de M1.
  - Un objet local nommé dans le corps d’une utilisation, tel que `%c1::mid` (9.4).
- Les noms de sous-objets standards sont réservés : aucun nom de rôle, nom de paramètre ou nom local d’une définition générale, ni aucune clé d’un texte, ne peut être identique à l’un d’eux (comparés tels qu’écrits, 3.6). Un nom local ne peut pas être un nom de rôle, et la clé d’un texte ne peut être ni un nom de rôle ni un nom local de l’utilisation à laquelle il est attaché. Un nom local peut être un nom de paramètre : un paramètre est une valeur, et `::` ne prend pas de valeurs. Aucun paramètre d’une définition ne peut non plus s’appeler `priority` (14.8).
- **Le nom après `::` est recherché dans cet ordre** : les noms de rôles et noms locaux de l’utilisation elle-même ; leurs alias `definition=` ; les noms de sous-objets standards et leurs alias ; les noms des broches du symbole de l’objet ; les clés de textes (alias : 16.5). Un nom de broche identique à un nom de rôle ou un nom local de l’utilisation, ou à un nom de sous-objet standard, est une erreur lorsque le symbole est attribué à l’objet ; il en va de même pour une clé de texte identique à un nom de broche.
  - Les clés viennent en dernier, si bien qu’une clé ne peut pas s’orthographier comme un nom qui passe avant elle : un nom de sous-objet standard ou l’un de ses alias déclarés dans ce fichier (tel que `"位置"` d’`alias-zh`), un nom de rôle ou nom local de l’utilisation, ou un alias `definition=` de ces noms. Une telle clé est une erreur, comme les collisions ci-dessus (13.3).
- `::` ne prend que des parties de l’objet lui-même ; la recherche d’objets dans la scène par conditions se fait toujours avec les mots-clés de sélection. Ainsi « toutes les utilisations d’une définition » s’écrit comme une sélection (10.1), et non comme un sous-objet de la définition.
- Les bornes utilisent `:` et les sous-objets `::`, afin que les noms de sous-objets ajoutés dans des versions ultérieures ne puissent pas entrer en conflit avec les noms de ports d’une netlist.
- **Noms de sous-objets** : il existe deux noms de sous-objets standards, `position` et `net`, et tous deux sont réservés. Tous les autres noms de sous-objets sont dynamiques : clés de textes, noms de rôles d’utilisations, noms locaux dans les corps de définitions, et noms de broches. Les noms de sous-objets standards ajoutés dans des versions ultérieures s’appliquent selon la version (16.1) : dans un fichier d’une version antérieure, une clé portant le même nom reste valide.

### 6.6 Nommage

- `%name = statement` et `$name = statement` nomment le résultat d’une instruction ; dans un chaînage, `=> %name` et `=> $name` nomment le résultat du segment à leur gauche (5.5). Un nom n’est qu’une autre manière de désigner un objet et ne crée aucun objet.
- `$name` lie une définition ou un ensemble, y compris le résultat d’une sélection. Un ensemble en `$` évalué dans un niveau ne contient que les membres de ce niveau (10.3). Les autres objets de la scène appartiennent à un niveau et ne peuvent pas porter un nom en `$`.
- `%name` peut lier tout objet de ce niveau : un objet créé par l’instruction, un objet existant (`%tail = #M5`), un ensemble (`%pair_set = {#M1 #M2}`), ou une définition utilisée uniquement dans ce niveau.
- **Un nom n’est lié qu’une seule fois dans un niveau.** Le lier à nouveau au même objet est autorisé, comme `%x = #M1` écrit deux fois ; le lier à un autre objet est une erreur, et le nom garde sa première liaison valide. Ce que lie une liaison suit 5.5. Un segment qui garde la poignée de son entrée, tel que PLACE ou ORIENT, lie cette poignée, y compris lorsque le segment échoue, et une telle liaison est valide exactement lorsque cette poignée l’est : lorsque l’entrée elle-même est un objet en échec, comme après un BUNDLE qui échoue, la liaison n’est pas valide non plus (5.5). Un segment qui crée un nouvel objet le lie, et la liaison est valide lorsque ce segment n’échoue pas pendant la lecture et la création du niveau (étapes 1 à 3 de 8.2) ; lorsqu’il échoue plus tard, lors d’une vérification effectuée après l’attribution des symboles (8.2), il garde le nom, une liaison ultérieure ne le reprend pas, et le nom désigne dès lors l’objet en échec. Ce n’est que lorsqu’aucune de ses liaisons n’est valide que le nom est lié à un objet en échec (7.2). Une liaison qui n’est pas valide ne retient pas le nom, donc une liaison ultérieure à un autre objet ne constitue pas l’erreur de lier deux fois : après le faisceau en échec de 5.5, `%s = #XI1` lie `%s` à XI1. Ceci vaut également pour les instructions qui créent des objets, des définitions, des ensembles et les résultats de sélections.
- La surcharge s’applique aux faits, non aux noms : pour modifier un objet créé par RNL, écrivez une instruction qui le modifie, telle que `%g0 @PLACE(40, 38)` (5.8).

### 6.7 Portée des noms

- Les noms versés par `@RNL` sont publiés dans le niveau : tous les enregistrements du niveau les partagent, et un nom peut être utilisé avant l’instruction qui le lie (Principes 2.9).
- Un nom lié dans un ensemble imbriqué ne vaut que dans cet ensemble.
- Un nom lié dans un corps de définition est local au corps (6.9).

### 6.8 Héritage et masquage

- Les définitions en `$` sont héritées vers le bas de la hiérarchie des niveaux (6.1), jamais vers le haut : dans les sous-circuits du fichier et dans les fichiers inclus. Un niveau ne voit pas les définitions en `$` des fichiers qu’il inclut ; pour fournir des définitions aux niveaux supérieurs, créez un paquet et utilisez-le avec USE-PACKAGE (16.2). Le symbole de bloc d’un sous-circuit est la seule exception qui remonte (9.2.6).
- Un `$X` défini dans un niveau inférieur avec le même nom qu’une définition héritée masque la définition héritée dans ce niveau et les niveaux inférieurs, et produit une indication ; le niveau supérieur voit toujours le sien.
- Les définitions en `$` d’un paquet utilisé avec USE-PACKAGE dans un niveau comptent comme un niveau au-dessus de ce niveau (16.2). Lorsqu’elles masquent des définitions des niveaux supérieurs ou des paquets à la racine, une indication est émise, comme pour tout masquage. Une indication est également émise lorsqu’un paquet utilisé dans un enregistrement d’en-tête définit un nom du paquet standard, ou d’un paquet placé à la racine avant lui (16.3).
- Les noms en `%` ne valent que dans leur propre niveau et ne sont pas hérités : ni dans les sous-circuits du fichier, ni à travers une inclusion. Les définitions en `%` d’un paquet sont des auxiliaires privés du paquet.

### 6.9 Noms dans les corps de définition

- Les noms dans un corps de définition sont résolus là où la définition est écrite, et non à nouveau là où elle est appliquée ; la seule exception est un `#` isolé dans le corps d’une définition générale (ci-dessous).
- Masquer une définition ne modifie donc que ce que désigne le nom : les définitions déjà utilisées à l’intérieur d’autres définitions restent telles quelles. Par exemple, masquer `$DIFFERENTIAL_PAIR_N` dans un niveau inférieur ne modifie pas la paire différentielle utilisée dans le corps d’une définition écrite dans un niveau supérieur.
- Un `%name` dans un corps est recherché d’abord parmi les noms locaux du corps, puis parmi les définitions du niveau. Les objets de la scène du niveau (points, fils, etc.) ne peuvent pas être référencés depuis un corps ; une telle référence est une erreur. Un ensemble ou le résultat d’une sélection que le niveau nomme avec `%` est un tel objet aussi, quels que soient ses membres lorsqu’il est évalué : pour en partager un avec des corps, nommez-le avec `$`, auquel un corps peut se référer et qui est évalué dans le niveau de chaque utilisation (6.6, 10.3).
- **Notation dans les corps** : les rôles et paramètres s’écrivent `?name`. Les noms locaux sont liés avec `%name =` (ou `=> %name` dans un chaînage) et référencés par `%name`. De l’extérieur, les noms locaux sont atteints par l’utilisation (9.4).
- Un corps de définition ne voit pas le nom en cours de définition : dans `$NMOS = @DEFINE-SYMBOL {$NMOS; …}`, le `$NMOS` dans le corps est la définition de ce nom à l’extérieur (dans un niveau supérieur ou un paquet). C’est ainsi qu’un niveau inférieur masque et étend une définition supérieure ; sans une telle définition externe, c’est une erreur.
- Les références hôtes dans les corps de DEFINE-SYMBOL et DEFINE sont des erreurs : `#M1`, `&vss`, `#:inp` et `#M1:d` ne peuvent pas y être écrits, ni un `#` isolé dans un corps de symbole. Dans le corps d’une définition générale, un `#` isolé désigne le niveau dans lequel se trouve l’utilisation, lequel n’est connu que là où la définition est appliquée (9.4) ; il s’y écrit comme portée d’une sélection. Un symbole de bloc ne peut pas être attribué depuis un corps : `# @SET-SYMBOL` à cet endroit est une erreur, car le symbole de bloc d’un niveau entre dans la file des symboles avant les utilisations du niveau, et une utilisation peut être générée après lui (8.3, 9.2.6). Les noms dans un corps sont résolus là où la définition est écrite, mais une définition générale est appliquée dans d’autres niveaux et d’autres fichiers, où une référence hôte pointerait encore vers le niveau de la définition, de sorte que son sens ne serait pas unique. Une définition générale qui a besoin d’objets hôtes les reçoit par ses rôles.
- **Quand le corps d’une définition générale est vérifié** :
  - Une fois pour chaque définition : tout ce qui ne dépend pas des arguments est vérifié à ce moment-là, y compris les mots-clés, les signatures (nombre d’objets, noms et types de paramètres), les noms en `$` et `%` (résolus là où la définition est écrite), les références hôtes, les collisions de noms locaux (9.4), et le fait que les rôles déclarés avec une sorte soient utilisés correctement : lorsque le rôle t est déclaré `"instance"`, `?t::net` dans le corps est une erreur. Les signatures sont vérifiées lors de la lecture du texte (7.1) ; les noms ne peuvent être résolus qu’une fois tout le niveau lu, donc les vérifications nécessitant les noms sont faites lorsque le niveau prend effet. Les deux ne sont faites qu’une seule fois pour chaque définition, quel que soit le nombre de fois qu’elle est appliquée.
  - Pour chaque application : ce qui dépend des arguments, comme les objets que reçoivent les rôles sans sorte déclarée, ou si le symbole possède une broche, est vérifié à chaque application et signalé à l’application, en pointant également vers l’instruction dans la définition générale.
  - Étendue des erreurs : une instruction du corps reconnue erronée (y compris un nom local qui entre en collision) est signalée une seule fois, à la définition, et est omise de chaque développement ; le reste du corps fonctionne normalement (le corps est tolérant, 9.1). Une liste de rôles ou liste de paramètres erronée (noms répétés, collisions avec les noms de sous-objets standards, noms de sortes inconnus) fait échouer toute la définition générale, et avec elle toute application qui l’utilise.
  - Une définition générale qui n’est jamais appliquée est vérifiée elle aussi. Ces vérifications ne dépendent pas des applications, elles trouvent donc les erreurs dans les définitions inutilisées d’un paquet.

### 6.10 Paramètres génériques

- `?name` n’apparaît que dans le corps d’une définition générale, et le nom doit figurer dans la liste de rôles ou la liste de paramètres ; sinon c’est une erreur à la lecture du texte.
- Là où va un objet, c’est un rôle, tel que `?t` ou `?t::net` ; là où va un littéral, c’est un paramètre, tel que `min=?gap`, `[?w, 2]` et `(?x, 0)` (9.4). Un rôle là où va un littéral, ou un paramètre là où va un objet, est une erreur à la lecture du texte. Les noms de rôles et les noms de paramètres figurent dans une table unique et ne peuvent pas se répéter.
- Un rôle générique peut avoir un chemin, tel que `?t::net`, et le paramètre générique d’un rôle d’ensemble peut prendre des places de membres, telles que `?ref{1}` (5.2.3) ; un paramètre générique n’a ni l’un ni l’autre.

### 6.11 Indices

| Écrit | Représente |
|---|---|
| `&D<3>` | La même chose que `&"D<3>"` |
| `&D<7:0>` | Une liste : `D<7>`, `D<6>`, …, `D<0>` |
| `&D<1,2:3>` | `D<1>`, `D<2>`, `D<3>` |
| `&D<0:6:2, 9>` | `D<0>`, `D<2>`, `D<4>`, `D<6>`, `D<9>` |
| `&D[3:0]` | `D[3]`, `D[2]`, `D[1]`, `D[0]` |
| `#XI0<3:0>` | Quatre instances |
| `#X1:D<7:0>` | Huit bornes de X1 |
| `#:D<7:0>` | Huit ports de ce niveau |
| `&"D"<1:0>` | `D<1>`, `D<0>` |
| `&"D<7:0>"` | Le nœud dont le nom est `D<7:0>` : rien entre guillemets n’est analysé |

- Une netlist n’a pas de bus : lorsque des outils tels que Virtuoso écrivent une netlist, ils scindent un bus en bits individuels, tels que `D\<3\>` dans une netlist Spectre ; les ports de bus des sous-circuits et les instances itérées sont scindés aussi. Un indice est une manière abrégée d’écrire un ou plusieurs de ces noms. Il ne fait que construire des noms et ne repose pas sur le fait que l’hôte connaisse les bus, si bien que son sens est unique.
- **Forme** : un nom hôte, entre guillemets ou non, suivi directement de `<…>` ou `[…]` contenant des éléments séparés par `,`. Chaque élément est `i`, `a:b` ou `a:b:s`, et les formes peuvent être mélangées. Les nombres i, a, b et s sont des entiers décimaux non signés, et s vaut au moins 1. Il n’y a aucun espace blanc entre le nom et le crochet ; à l’intérieur des crochets, les espaces blancs et commentaires sont autorisés (3.5).
- **Développement** : les éléments sont développés dans l’ordre écrit et joints ; les répétitions sont retenues. `a:b` va de a à b, et `a:b:s` par pas de s sans dépasser b : `<7:0>` donne 7, 6, …, 0, et `<0:7:2>` donne 0, 2, 4, 6. Le pas vient en dernier, comme dans Virtuoso et Xschem, et non comme dans `a:s:b` de MATLAB. Un nom généré est constitué du nom avant le crochet, du crochet ouvrant, du nombre décimal sans zéros non significatifs et du crochet fermant, dans la sorte de crochet écrite : `&D<07>` est `D<7>`, et `&D[3]` est `D[3]`.
- **Un seul ou une liste** : un élément unique qui est un nombre unique donne une seule référence : `&D<3>` équivaut à `&"D<3>"`. Tout autre cas donne une liste, qui est une référence à un ensemble, comme `&D<3:3>` et `&D<3,5>`. Une liste dans un ensemble est dépliée d’un niveau (5.2) ; en tant qu’unique objet d’une instruction, elle est d’abord scindée d’un niveau (5.2.1), si bien que `#XI0<3:0> @ALIGN("y")` aligne quatre instances ; dans un emplacement qui prend un objet unique, elle opère membre par membre ; dans l’emplacement de net de WIRE, elle est prise entière et crée un bus (13.1).
- **Où** : noms hôtes après `#` et `&`, ainsi que noms de bornes et noms de ports dans les références en `#`. Les noms RNL (après `%`, `$` et `?`), noms de rôles et noms de sous-objets ne prennent pas d’indice ; un `<` ou `[` juste après eux est une erreur. Les membres d’un ensemble sont pris par leurs places avec des accolades à la place (5.2.3).
- **Aucun point juste après un nom** : un `<` ou `[` juste après un nom commence toujours un indice, et lorsque ce qui suit ne correspond pas à la forme, c’est une erreur de syntaxe ; ce n’est jamais lu comme un point. Pour écrire un point après un nom, séparez-les, comme `&D (1, 2)`.
- **Un seul développement** : un nom comporte au plus un indice, donc un nom tel que `A<1>[2]` s’écrit entre guillemets, `&"A<1>[2]"`. Au plus un indice dans une référence se développe en une liste : `#XI0<3:0>:a` et `#X1:D<7:0>` sont autorisés, `#XI0<1:0>:D<1:0>` est une erreur, et deux indices qui sont chacun un nombre unique sont autorisés, tels que `#XI0<3>:D<1>`. Lorsqu’une référence qui se développe en une liste est suivie de `::`, le sous-objet est pris pour chaque membre, tel que `#XI0<3:0>::name`.
- **Rien entre guillemets n’est analysé** : `&"D<7:0>"` désigne le nœud nommé `D<7:0>` ; `&"D"<7:0>` se développe. Lorsqu’un nom dans la netlist contient `<` ou `[` qui ne marquent pas les bits d’un bus, mettez-le entre guillemets pour vous y référer tel quel.
- **Les noms comparés sont ceux que lit l’adaptateur** (3.6) : `D\<7\>` dans une netlist Spectre, où la barre oblique inversée est l’échappement de Spectre, est lu par l’adaptateur comme `D<7>`, donc `&D<7>` s’y réfère.
- Chaque membre est résolu séparément. Un membre introuvable est un objet en échec, traité de manière stricte ou tolérante selon ce qu’indique l’emplacement qui le reçoit (7.3).
- Les chaînes dans les paramètres ne sont pas développées, telles que `terminal=` de PIN (9.2.4) et la condition `name=` d’une sélection.
- Ne sont pas acceptés : la répétition de Virtuoso, `<*2>A` et `A<0:2*2>`, ni sa juxtaposition `A,B` (des ensembles peuvent déjà être écrits) ; `D[0..7]` de KiCad et Xschem, dont les membres sont D0 … D7 sans crochets.
- Les instances itérées (telles que `I0<3:0>` de Virtuoso : plusieurs instances hôtes dessinées comme une seule) s’écrivent sous forme de faisceaux : `#XI0<3:0> @BUNDLE` (11.8).

### 6.12 Noms de bornes : DEVICE

```spice
* sky130 transistors, defined in the PDK and not in this netlist
* @RNL(type="header") {
*   @VERSION("0.4");
*   @DEVICE(name="X*", model="sky130_fd_pr__nfet_*", terminals=["d", "g", "s", "b"], type="nmos");
*   @DEVICE(name="X*", model="sky130_fd_pr__pfet_*", terminals=["d", "g", "s", "b"]);
*   @DEVICE(model="sky130_fd_pr__pfet_*", type="pmos")    /* adds the type to the names given above */
* }
XM1 out in 0 0 sky130_fd_pr__nfet_01v8 w=1 l=0.15
XM2 out in vdd vdd sky130_fd_pr__pfet_01v8 w=2 l=0.15
* @RNL { #XM1:d @CURRENT("in") /* XM1 is a sky130 NMOS; its terminal 1 is named d, and its type is nmos */}
.end
```

- **RNL nomme les bornes et donne le type.** `@DEVICE` décrit une classe de composants : `terminals` énumère les noms de leurs bornes dans l’ordre des nœuds sur la ligne de netlist, et `type` donne leur type (ci-dessous). Il ne fait que donner des noms aux numéros de bornes de l’hôte, et ne modifie ni l’ordre des bornes ni l’interface d’un sous-circuit ; c’est un alias d’affichage, ce qu’autorise Principes 2.2. Les noms dans une même liste `terminals` doivent différer selon les règles de casse de l’hôte et ne peuvent pas être composés uniquement de chiffres (6.4) ; par exemple, `"d"` et `"D"` sont égaux en SPICE. Dans l’enregistrement d’en-tête d’un fichier de paquet, qui peut être utilisé par des fichiers de l’un ou l’autre dialecte, ils doivent différer sans tenir compte de la casse. Une liste comportant deux noms égaux, ou un nom composé uniquement de chiffres, est une erreur, cette instruction DEVICE échoue, et les instances gardent les noms que leur donnent les instructions DEVICE antérieures et l’adaptateur.
- **Uniquement dans l’enregistrement d’en-tête** (2.8) : DEVICE écrit partout ailleurs est une erreur. L’enregistrement d’en-tête d’un fichier de paquet compte (16.2). Un paquet en ligne ne peut pas contenir DEVICE ; écrivez-le dans l’enregistrement d’en-tête du fichier.
- **Correspondance** : `name` est comparé au nom de l’instance, et `model` au nom de modèle ou nom de sous-circuit écrit sur la ligne d’instance (dans Spectre, le master de l’instance, tel que `resistor` pour une primitive) ; tous deux peuvent utiliser le joker `*` (n’importe quel nombre de caractères) et sont comparés selon les règles de casse de l’hôte. L’un ou l’autre peut être omis, mais pas les deux, et un DEVICE correspond à une instance lorsque ceux écrits correspondent. Un DEVICE donne `terminals`, `type` ou les deux ; un DEVICE qui n’a ni `name` ni `model`, ou ni `terminals` ni `type`, est une erreur et échoue. Les conditions des sélections qui comparent des noms prennent `*` selon les mêmes règles (10.1).
- **Type** : `type` est une chaîne, telle que `"nmos"` ; les mots du tableau ci-dessous sont les mots standards. C’est le type que lisent la condition `type` (10.1), `${type}` (13.6) et les assertions (15.3), pour une instance à laquelle son hôte n’en donne aucun, telle qu’une instance X, y compris les composants de PDK que la netlist ne définit pas. Une instance d’une carte de modèle garde le type de son modèle (ci-dessous) ; un DEVICE dont le type diffère produit un avertissement et ne le modifie pas, comme pour les noms de ports des sous-circuits.
- **Lorsque plusieurs correspondent, le dernier compte, élément par élément** : pour les noms de bornes et pour le type séparément, le dernier DEVICE correspondant qui écrit l’élément compte, si bien qu’un DEVICE peut donner les noms et un DEVICE ultérieur le type, et qu’un DEVICE ultérieur qui omet un élément ne le modifie pas. Les valeurs par défaut de l’adaptateur viennent d’abord, puis les instructions DEVICE des paquets utilisés par le niveau (dans l’ordre des positions des paquets, 16.2), puis celles dans les enregistrements d’en-tête des fichiers externes, puis celles des fichiers inclus ; au sein d’un même enregistrement d’en-tête, dans l’ordre écrit, et parmi les enregistrements d’en-tête d’un même fichier, celui sans `language` d’abord, puis ceux avec `language` dans l’ordre écrit, chacun n’agissant que sur les instances des sections de son langage (2.8). Ainsi ce qu’écrit un fichier supplante ses paquets, les paquets supplantent l’adaptateur, et les instructions spécifiques devraient être écrites après les instructions générales. Les instructions DEVICE de tous les paquets viennent avant celles des fichiers, quel que soit l’endroit où est écrit leur USE-PACKAGE : l’enregistrement d’en-tête est toujours au début du fichier, donc placer un paquet à son USE-PACKAGE le laisserait supplanter ce qu’écrit le fichier lui-même. Un DEVICE qui donne `model` n’est pas classé comme plus spécifique ; il n’y a pas de règle de spécificité séparée.
- **Portée** : DEVICE dans l’enregistrement d’en-tête d’un fichier s’applique à ce fichier et aux fichiers qu’il inclut ; un fichier inclus ne peut modifier que lui-même et les fichiers qu’il inclut à son tour (6.1). DEVICE dans l’enregistrement d’en-tête d’un fichier de paquet s’applique aux niveaux qui utilisent le paquet (16.2). Par exemple, un paquet pour sky130 peut contenir des instructions DEVICE, des symboles et les règles qui choisissent les symboles, et un utilisateur qui écrit `@USE-PACKAGE(name="sky130-rnl")` obtient les noms de bornes, les types et les symboles.
- **Valeurs par défaut de l’adaptateur** : les définitions d’éléments standards de chaque dialecte, qui comptent comme écrites au niveau le plus externe. Elles n’ont pas besoin d’être écrites dans le fichier, et un outil peut les écrire. Les outils destinés au même dialecte doivent donner les mêmes définitions. Ce sont les valeurs par défaut de la version du fichier dans lequel se trouve la ligne d’instance (16.1), tout comme l’est un DEVICE dans l’enregistrement d’en-tête de ce fichier. Pour la famille SPICE :

  ```rnl header
  @DEVICE(name="M*", terminals=["d", "g", "s", "b"]);
  @DEVICE(name="Q*", terminals=["c", "b", "e", "s"]);
  @DEVICE(name="J*", terminals=["d", "g", "s"]);
  @DEVICE(name="Z*", terminals=["d", "g", "s"]);
  @DEVICE(name="D*", terminals=["a", "k"]);
  @DEVICE(name="R*", terminals=["p", "n"]);             /* and one each like it for C, L, V, I, B, F, H, W */
  @DEVICE(name="E*", terminals=["p", "n", "cp", "cn"]);  /* and one each like it for G, S */
  @DEVICE(name="T*", terminals=["p1", "n1", "p2", "n2"]); /* and one like it for O */
  @DEVICE(name="U*", terminals=["a", "b", "ref"])
  ```

  Les noms d’instances dans Spectre ne commencent pas par une lettre d’élément, donc les valeurs par défaut sont mises en correspondance par `model`, le master de l’instance. Pour les primitives de Spectre :

  ```rnl header
  @DEVICE(name="*", model="resistor", terminals=["p", "n"]);      /* and one each like it for capacitor, inductor, vsource, isource, ccvs, cccs, bsource, port */
  @DEVICE(name="*", model="diode", terminals=["a", "k"]);
  @DEVICE(name="*", model="vcvs", terminals=["p", "n", "cp", "cn"]); /* and one each like it for vccs, relay */
  @DEVICE(name="*", model="tline", terminals=["p1", "n1", "p2", "n2"]);       /* as T in SPICE */
  @DEVICE(name="*", model="transformer", terminals=["p1", "n1", "p2", "n2"])   /* as T in SPICE */
  ```

  - `switch` possède une borne pour chacun de ses nœuds, nommée selon la numérotation de positions de Spectre : `t0`, la borne commune, puis `t1` à `tn`, les sorties.
  - Une instance d’une carte de modèle, dont le master est le nom d’une instruction `model`, prend les noms de la famille du master de ce modèle. Son type, pour la condition `type` (10.1), s’écrit avec les mots de SPICE :

  | Famille | Masters du modèle | Bornes | Type |
  |---|---|---|---|
  | MOS | `bsim1`, `bsim2`, `bsim3`, `bsim3v3`, `bsim4`, `bsim6`, `bsimbulk`, `bsimsoi`, `bsimcmg`, `bsimimg`, `b3soipd`, `mos0`, `mos1`, `mos2`, `mos3`, `mos9`, `mos11`, `mos705`, `mos902`, `mos903`, `psp102`, `psp103`, `psp104`, `pspnqs102`, `pspnqs103`, `ekv`, `hisim`, `hisim2`, `hisimhv`, `hvmos`, `bta_soi`, `ucsd_hvmos`, `hisim_hv`, `utsoi`, `letisoi`, `bsimcmg_va` | `d g s b` | `nmos`, ou `pmos` lorsque le `type` du modèle est `p` ou `pmos` |
  | Bipolaire | `bjt`, `bjt500`, `bjt503`, `bjt504`, `bjt504t`, `bjt3500`, `bjt301`, `vbic`, `hicum`, `hicum0`, `hicum2`, `mextram` | `c b e s` | `npn`, ou `pnp` lorsque le `type` du modèle est `pnp` |
  | Diode | `diode`, `juncap`, `juncap200`, `dio500`, `junb` | `a k` | `d` |
  | JFET | `jfet` | `d g s` | `njf`, ou `pjf` lorsque le `type` du modèle est `p` ou `pjf` |
  | MESFET | `gaas`, `angelov`, `tom2`, `tom3` | `d g s` | `nmf`, ou `pmf` lorsque le `type` du modèle est `p` ou `pmf` |
  | Résistance | `resistor`, `rdiff`, `r3` | `p n` | `r` |
  | Condensateur | `capacitor` | `p n` | `c` |
  | Inductance | `inductor` | `p n` | `l` |
  | Relais | `relay` | `p n cp cn` | `sw` |
- **Sous-circuits définis dans la netlist** (y compris ceux des fichiers de bibliothèque importés par include ou lib) : les noms de ports sont ceux de la définition, ce qui est un fait de l’hôte (Principes 2.2). DEVICE ne les modifie pas ; un DEVICE qui correspond à une telle instance avec des noms différents produit un avertissement. DEVICE ne nomme que les composants sans noms de bornes : les primitives, et les sous-circuits que la netlist ne définit pas (composants de PDK), pour lesquels l’utilisateur, ou un LLM, écrit le DEVICE. Un DEVICE sans `terminals` peut tout de même donner son type à un sous-circuit défini dans la netlist.
- **Nombre de noms** : lorsqu’une instance comporte plus de nœuds que `terminals` ne contient de noms, les bornes supplémentaires n’ont que des numéros ; lorsqu’elle en comporte moins, une référence à un nom supplémentaire échoue.
- **Casse** : les noms de bornes donnés par DEVICE sont comparés selon les règles de l’hôte (6.4).
- Les numéros fonctionnent toujours, tout comme les noms de ports des sous-circuits qui ne sont pas composés uniquement de chiffres, ainsi que `#:port`.

<!-- translated from en/ch07.md sha256:7d1065e7e655 -->
## 7 Échecs et diagnostics

### 7.1 Deux phases de vérification

- **Lecture** : la structure lexicale ; la grammaire ; les signatures des mots-clés intégrés, c’est-à-dire le nombre d’objets, leurs sortes et les paramètres (y compris la forme des tableaux) ; et le fait que chaque `?name` dans le corps d’une définition générale figure dans sa liste de rôles ou sa liste de paramètres et se trouve à une place qui lui convient (6.10). Les erreurs de signature dans le corps d’une définition générale qui ne dépendent pas des arguments sont également signalées à ce stade (6.9).
- **Prise d’effet** :
  - une fois tout le niveau lu, les noms et les références hôtes sont résolus, et les sortes des objets que désignent les noms sont vérifiées ;
  - les noms dans le corps d’une définition générale sont résolus une seule fois pour chaque définition, et non à nouveau pour chaque application (6.9) ;
  - le nombre d’objets, les noms de rôles, les sortes déclarées pour les rôles et les paramètres d’AS et ATTACH sont vérifiés (9.4) ;
  - chaque application vérifie les parties de son développement qui dépendent des arguments ;
  - une fois les symboles attribués, on vérifie si les bornes auxquelles se connectent les fils et contraintes possèdent une broche dans leur symbole, si les utilisations auxquelles s’applique ORIENT possèdent un symbole, si les noms de broches atteints par l’intermédiaire d’une instance ou d’une utilisation, tels que `#R1::p` ou `%g::pin`, sont des broches de son symbole, et si les conversions depuis et vers des broches peuvent être effectuées (5.2.4).

### 7.2 Objets en échec

- Une instruction qui échoue produit un objet en échec, qui porte un diagnostic : la cause, l’emplacement et la cause en amont.
- Un objet en échec garde sa place parmi les membres, et les autres membres gardent la leur. Ainsi, lorsque la netlist hôte n’a pas de M9, `{#M1 #M2 #M9} @ALIGN("y")` échoue dans son ensemble ; il ne devient pas un alignement de M1 et M2 seulement.
- Lorsqu’un nom est lié à un objet en échec (y compris une définition qui échoue lors de sa lecture), les instructions qui se réfèrent à ce nom reçoivent elles aussi l’objet en échec. Les instructions qui dépendent d’une description en échec échouent avec elle, quel que soit leur ordre (Principes 2.9), sauf qu’une instruction reconnue en échec lors d’une vérification effectuée après l’attribution des symboles ne revient pas sur les étapes auxquelles elle a déjà pris part (8.2) ; ce sont également des erreurs, et leurs diagnostics indiquent la cause en amont (7.6).
- Lier un nom une deuxième fois à un autre objet est une erreur, et le nom garde sa première liaison valide ; ce n’est que lorsqu’aucune de ses liaisons n’est valide que le nom est lié à un objet en échec (6.6, 5.5).

### 7.3 Entrées strictes et tolérantes

- La signature déclare pour chaque entrée si elle est stricte ou tolérante :
  - stricte : lorsque l’entrée contient un objet en échec, l’instruction entière échoue ;
  - tolérante : les membres en échec sont retirés et un diagnostic est consigné ; le reste prend effet normalement.
- Par défaut, les entrées absorbées sont strictes ; les entrées encloses (corps de définitions) et les entrées versées (`@RNL`) sont tolérantes (5.4).
- Les arguments d’AS et ATTACH sont stricts ; les instructions du développement sont tolérantes : chacune qui échoue est écartée isolément.
- Dans les chaînages et l’imbrication, un échec n’affecte que les descriptions qui en dépendent (5.5) : lorsqu’une entrée stricte contient un objet en échec, l’instruction échoue, mais les instructions qui y sont imbriquées et ne dépendent pas de l’échec prennent effet normalement ; lorsqu’une instruction opère une fois pour chaque membre, chaque membre constitue une description à part entière et échoue isolément.

### 7.4 Reprise sur erreur

- Une erreur de syntaxe fait échouer tout le membre dans lequel elle se produit, jusqu’au prochain `;` ou à la fin de l’ensemble englobant.
- Lorsqu’une chaîne ou un commentaire `/* */` n’est pas fermé, ou que les accolades ne s’apparient pas, la fin de l’enregistrement ne peut pas être trouvée (2.4) : l’enregistrement est incomplet, et l’enregistrement entier échoue. Une parenthèse ou un crochet à l’intérieur d’un membre qui n’est pas fermé, comme pour des paramètres, un tableau, un rôle, un point ou un indice, n’est une erreur de syntaxe que de ce membre : les autres membres de l’enregistrement sont lus normalement.
- Lorsqu’un enregistrement est interrompu avant d’être complet, l’enregistrement entier échoue, et le diagnostic indique que l’enregistrement est incomplet (2.4).
- La reprise après une erreur ne dépasse jamais son enregistrement : les autres enregistrements sont lus normalement (Principes 2.9).

### 7.5 Les conflits ne sont pas des échecs

- Les conflits sont traités une fois tout le niveau lu, par les méthodes définies : surcharge par clé (8.4), files d’apparence (8.3) et résolution des faits géométriques (8.5).
- Un conflit pour lequel aucune méthode n’a encore été définie fait échouer l’interprétation entière (1.5 point 3 ; 7.6). La présente version ne comporte aucun cas de ce type : un cycle d’ensembles échoue à l’instruction qui le ferme (5.2.2).

### 7.6 Diagnostics

- **Vérifier et interpréter ne font qu’un processus.** Un interpréteur lit l’entrée et fournit la scène résultante accompagnée d’un ensemble unique de diagnostics ; utilisé uniquement pour la vérification, il est aussi appelé lint. La présente norme ne définit que cet unique ensemble de diagnostics. Les vérifications locales qu’un éditeur effectue pour un retour immédiat, telles que la coloration syntaxique, ne sont pas des diagnostics en ce sens : la présente section ne s’y applique pas, et elles ne peuvent pas remplacer les diagnostics.
- **Éléments.** Un échec s’applique à un élément : un champ d’un en-tête d’enregistrement, une instruction, un enregistrement, un fichier ou l’interprétation entière. Lequel est précisé là où chaque échec est défini (voir l’étendue d’un échec ci-dessous).
- **Quatre sortes de signalement** :
  - **Erreur** : un élément échoue, que la cause réside dans l’élément lui-même, dans un autre élément, ou en amont dans quelque chose dont il dépend. Les erreurs comprennent les erreurs de base de Principes 2.4 (erreurs de syntaxe, mots inconnus, signature discordante, référence introuvable, mauvaise sorte, auto-contradiction, version non prise en charge), les autres échecs définis dans la présente norme (tels qu’ORIENT appliqué à une utilisation sans symbole, ou une borne sans broche dans son symbole ; 12.1, 9.2.4), et les échecs qui en découlent (7.2).
  - **Avertissement** : l’élément est valide et prend effet normalement, sauf lorsque la présente norme en dispose autrement (comme pour un fait écarté parce que sa vérification n’a pas abouti, 8.5), mais pourrait ne pas correspondre à ce que voulait l’auteur, tel qu’un ensemble vide écrit tel quel pour un emplacement qui prend des objets uniques, comme `{} @PLACE(1, 2)` (5.2.1) ; une définition sans nom écrite directement comme membre d’un enregistrement, non imbriquée dans une instruction (9.1) ; un rôle d’une règle de génération atteignant les niveaux inférieurs (10.1), rempli par un objet de son propre niveau que ces niveaux ne possèdent pas (10.5) ; un champ inconnu dans un en-tête d’enregistrement (2.8) ; une version de paquet remplacée par une autre (16.2) ; un fil connecté à une borne qui n’est pas sur le net que nomme le fil (13.1) ; un fait géométrique écarté parce que sa vérification n’a pas abouti dans la limite du solveur (8.5).
  - **Indication** : une surcharge, un masquage ou une résolution courant qui peut aussi être une erreur, tel qu’un niveau inférieur, ou un paquet utilisé dans celui-ci, masquant une définition d’un niveau supérieur ou des paquets à la racine (6.8) ; un paquet d’un USE-PACKAGE ultérieur masquant celui d’un USE-PACKAGE antérieur (16.2) ; un fait géométrique écarté lors de la résolution des conflits (8.5) ; la dernière version d’un paquet utilisée parce qu’aucune n’est écrite (16.2) ; un fichier principal sans version déclarée, lu avec la version par défaut de l’outil (16.1) ; des noms du paquet standard introuvables là où le paquet standard n’est pas utilisé (16.3) ; un enregistrement du solveur dont les numéros sont périmés (5.9).
  - **Violation** : une instruction qu’énonce le fichier au sujet du circuit n’est pas vérifiée : une assertion dont la vérification n’est pas vérifiée (15.3, 15.4), ou les instructions CURRENT d’un net qui envoient tout son courant dans un seul sens (15.2). Rien n’échoue, et la vérification comme son signalement ne changent rien au dessin : une assertion se dessine comme si elle n’était pas écrite, et les instructions CURRENT que lit la vérification restent en vigueur. Le signalement est destiné à l’auteur et aux outils qui vérifient un fichier. Une visionneuse peut l’afficher sous un nom qui lui est propre.
- **L’étendue d’un échec** est indiquée là où chaque échec est défini : un champ (un champ connu d’un en-tête d’enregistrement mal écrit est traité comme non écrit, 2.8) ; une instruction (la plupart des cas) ; un enregistrement (une chaîne ou un commentaire non fermé, des accolades qui ne s’apparient pas, un enregistrement interrompu, une cible de `scope` introuvable ; 7.4, 2.4, 2.6) ; un fichier (une version non prise en charge, 16.1) ; l’interprétation entière (un conflit pour lequel aucune méthode n’a encore été définie, 1.5 point 3 : l’interprétation s’arrête et rien n’est dessiné ; une version ultérieure pourrait rendre un tel cas tolérant ; la présente version n’en comporte aucun).
- **Exigences** :
  - Les erreurs doivent être signalées, et les violations doivent l’être aussi : le signalement est la raison d’être d’une assertion (15.3). Les avertissements et indications sont ceux qu’énumère la présente norme, et les implémentations devraient les signaler.
  - Une implémentation peut ajouter ses propres vérifications, mais uniquement comme avertissements ou indications ; elles ne doivent pas faire échouer un élément, sans quoi des implémentations différentes dessineraient des résultats différents.
  - Une implémentation doit permettre à l’utilisateur de voir les diagnostics ; leur présentation est laissée à l’implémentation (Principes 2.9).
  - Une implémentation peut être plus stricte, par exemple en s’arrêtant ou en rejetant toute l’entrée (Principes 2.9), mais ce ne doit pas être son comportement par défaut. Elle peut permettre à l’utilisateur d’élever ou d’abaisser la gravité des avertissements, indications et violations ; les erreurs ne peuvent pas être abaissées, puisqu’une erreur est un échec.
  - Les tests de conformité vérifient les erreurs (leur emplacement et l’étendue de l’échec), les violations (quelle vérification n’est pas vérifiée, et pour quels membres) et la scène résultante ; les avertissements peuvent être des attentes facultatives ; les indications ne sont pas testées.
  - Note : les avertissements devinent l’intention de l’auteur et évolueront avec l’expérience ; en faire des exigences figerait ces suppositions dans la norme. Principes 2.4 n’exige également que les erreurs de base.
- **Contenu** : un diagnostic indique
  - l’emplacement : le fichier, la ligne et la colonne ou la plage ; quel enregistrement, instruction, champ ou référence (Principes 2.8) ;
  - la cause : ce qui était attendu et ce qui a été reçu, tel que « in_p prend une instance, a reçu un net » (9.4) ; lorsqu’un nom est introuvable, le nom le plus proche ;
  - la cause en amont : chaque instruction qui échoue par contrecoup indique sa cause en amont, et une présentation peut regrouper ces instructions sous la cause racine, afin qu’une faute de frappe ne produise pas des dizaines d’erreurs à travers le dessin. Lorsqu’une même règle échoue sur de nombreux objets, les échecs peuvent être regroupés en un seul diagnostic énumérant les objets, par exemple lorsqu’un paquet écrit `W/L=${w}/${l}` pour chaque MOS et que certaines instances n’ont pas de `w` ;
  - l’origine : pour une instruction développée à partir d’une définition générale, à la fois l’application et l’instruction d’origine dans la définition ; pour une instruction produite par une règle, la règle et le niveau où elle a été évaluée ; pour une instruction dans un paquet, le nom, la version et le fichier du paquet (16.2).
- La formulation et la langue des diagnostics ne sont pas spécifiées. La présente version ne comporte pas de codes de diagnostic standards ; les implémentations peuvent numéroter leurs diagnostics, et les numéros ne définissent pas de classes de tolérance.
- **Paquets** : dans un document qui utilise un paquet, les erreurs dans le contenu du paquet sont signalées normalement et attribuées au paquet ; les avertissements et indications sur le contenu du paquet ne sont donnés que lorsque le paquet lui-même est vérifié. Les violations des assertions qu’écrit un paquet sont signalées dans le document qui l’utilise, puisqu’elles portent sur son circuit, et nomment l’assertion dans le paquet ainsi que l’utilisation ou la règle qui l’a appliquée. Note : les utilisateurs ne peuvent pas modifier un paquet, et des règles larges que son auteur a écrites à dessein les inonderaient sinon d’avertissements.
- Un fichier ne peut ni supprimer les diagnostics ni demander un mode plus strict.
- Les mots-clés, paramètres et valeurs inconnus sont des erreurs, comme les erreurs de syntaxe (Principes 2.3) : l’instruction échoue (16.4), sauf dans SOLVER-CONFIG, où une valeur non acceptée ne fait échouer que sa clé (14.10). Dans la présente version, le contenu reconnu mais non interprété, et ne donnant qu’un avertissement, se limite à un champ inconnu d’un en-tête d’enregistrement (2.8) et à une clé de SOLVER-CONFIG que le solveur ne connaît pas (14.10).

<!-- translated from en/ch08.md sha256:bf7d27b5693d -->
## 8 Prise d’effet

### 8.1 Un niveau prend effet après avoir été lu entièrement

- L’interpréteur analyse entièrement la netlist hôte et lit tous les enregistrements ; puis chaque niveau prend effet selon les phases de 8.2.
- Au sein d’un niveau, l’ordre des instructions n’importe que comme l’indique 1.5 point 2 ; il n’affecte pas la manière dont les noms et références sont résolus (Principes 2.9).
- Lequel de deux niveaux supplante l’autre est décidé par la hiérarchie des niveaux, et non par l’ordre du texte (Principes 2.9 ; 8.3).

### 8.2 Phases

1. **Lecture** : la structure lexicale, la grammaire et les signatures.
2. **Noms et définitions** : les noms en `$`, les noms en `%` et les références hôtes sont résolus ; les corps de DEFINE-SYMBOL et DEFINE-STYLE sont évalués.
3. **Création des objets** : les objets de la scène écrits explicitement (points, fils, étiquettes, faisceaux, graphiques, contraintes, utilisations) et les ensembles sont créés, et les corps des définitions générales sont développés ; les règles dans un corps rejoignent les règles du niveau de l’utilisation (9.4). Les sélections ne sont pas développées ici.
4. **File des symboles** : SET-SYMBOL prend effet et attribue des symboles aux objets de l’étape 3. C’est seulement maintenant que les sélections des règles de cette file sont évaluées, dans chaque niveau.
5. **Arbres** : les règles de ROUTE-NET, ROUTE-OBJECT, UNROUTE-NET et UNROUTE-OBJECT (10.4, 13.7) sont évaluées une fois dans chaque niveau, dans l’ordre : paquets, niveaux supérieurs, ce niveau. Leurs conditions lisent les arbres des instructions de l’étape 3, et elles ne se voient pas mutuellement ; les arbres qu’elles créent sont vus à partir de l’étape 6 (10.2).
6. **Génération un par un** : AS et ATTACH dont les objets sont le résultat d’une sélection, TEXT attaché au résultat d’une sélection (10.5, 13.3), RECTANGLE, CIRCLE et ELLIPSE écrits avec une taille dont le point est le résultat d’une sélection (9.2.5), les règles de LINE, ARC et BEZIER, ainsi que les règles de CURRENT (10.4) sont évalués une fois dans chaque niveau, dans le même ordre.
7. **Symboles des objets générés** : la même file des symboles est exécutée une fois de plus pour les nouveaux objets, avec les instructions SET-SYMBOL et les règles du contenu généré (10.5). Les symboles des objets de l’étape 3 sont fixés après l’étape 4, puisque les arbres et `wired` sont calculés à partir d’eux (10.2) : un SET-SYMBOL du contenu généré qui atteint un tel objet est sans effet sur lui, et un avertissement est émis. La génération voit les symboles tels qu’ils étaient avant la génération.
   Les assertions (15.3) sont vérifiées ici, une fois les symboles connus.
8. **Géométrie** : PLACE, ORIENT et les contraintes sont pris en compte un par un dans l’ordre de résolution, et celui qui entre en conflit avec ceux déjà pris en compte est écarté (8.5, 14.8) ; puis le placement est résolu. Cette étape a besoin des positions des broches et des cadres des symboles (9.2.3), elle intervient donc après l’attribution des symboles ; ORIENT appliqué à une utilisation sans symbole échoue ici, tandis que PLACE et les contraintes prennent le point propre de l’utilisation (14.1). Les objets sans aucune position ni contrainte sont placés par le solveur, qui réécrit leurs positions (14.8).
9. **File des styles** : SET-STYLE prend effet, avec les deux paliers de 8.3. C’est seulement maintenant que les propriétés de style que les graphiques d’un symbole laissent non écrites, ainsi que les couleurs de `fill=`, reçoivent des valeurs concrètes issues du style de l’objet propriétaire.
10. **Rendu**.

- Ces phases constituent l’ordre interne de 1.5 point 4 : une description lit l’état antérieur au début de sa phase ; ni ses propres effets ni ceux des autres descriptions de la même phase ne sont inclus.
- Un ensemble est évalué dans la phase qui l’utilise (5.2) : le même ensemble ne contient aucun objet généré lorsqu’il est utilisé à l’étape 6, et les contient lorsqu’il est utilisé à l’étape 9.
- Les conditions de sélection ne lisent que des informations fixées lors de phases antérieures :
  - les informations de l’hôte, les sortes d’objets, les utilisations, les sens du courant écrits comme instructions sur des bornes et les fils explicites sont fixés à l’étape 3 ; les sens que fixent les règles CURRENT (étape 6) et CURRENT sur des broches (une fois les symboles attribués) sont vus à partir de l’étape 7, par exemple par la file des styles et la vérification de 15.2 ;
  - une sélection dans la file des symboles ne peut pas sélectionner selon le symbole qu’utilise un objet ;
  - les règles de génération peuvent voir les symboles.
- Le fait qu’une borne possède une broche dans son symbole est vérifié après l’étape 7 (7.1). Une instruction qui échoue lors d’une vérification effectuée après l’attribution des symboles, telle qu’une application dont le rôle de sorte `"pin"` reçoit une borne qui n’a pas de broche, a pris part aux étapes précédentes telle qu’elle était écrite : ces étapes ne sont pas recalculées, et dès lors elle est sans effet. Les fils sont comptés de la même façon (10.2).
- Les méthodes de résolution et de rendu ne sont pas spécifiées par la présente norme ; seules leurs entrées le sont (Principes 1).

### 8.3 Files d’apparence

- Il existe deux files : la file des symboles reçoit SET-SYMBOL, et la file des styles reçoit SET-STYLE.
- Pour un niveau, chaque file se compose de ces groupes, dans cet ordre :
  1. les règles des paquets à la racine : le paquet standard d’abord, puis les autres paquets d’USE-PACKAGE dans les enregistrements d’en-tête (16.3) ; lorsqu’un paquet est utilisé à nouveau avec USE-PACKAGE dans ce niveau ou un niveau supérieur (dans un niveau de fichier ou un sous-circuit), il est placé à la position de cet USE-PACKAGE à la place (16.2) ;
  2. les symboles que les sous-circuits attribuent à leur propre `#`, c’est-à-dire leurs symboles de bloc (file des symboles uniquement ; 9.2.6) ;
  3. les règles des niveaux supérieurs, du plus externe vers l’intérieur ;
  4. toutes les instructions SET-SYMBOL ou SET-STYLE écrites dans ce niveau, tant les règles que les instructions sur des objets individuels ;
  5. SET-SYMBOL ou SET-STYLE écrits dans ce niveau depuis l’extérieur avec `scope`, ceux des niveaux externes plus tard (2.6).
- Au sein de chaque groupe, l’ordre est l’ordre du texte. Le contenu d’un autre paquet est placé à la position de son USE-PACKAGE (16.2) : un paquet utilisé dans un niveau supérieur va avec ce niveau, et un paquet utilisé dans ce niveau va à la position de l’USE-PACKAGE dans le texte de ce niveau. SET-SYMBOL et SET-STYLE produits par génération un par un appartiennent au groupe de la règle qui les génère, à la position de cette règle.
- **Deux paliers** :
  - Direct : SET-SYMBOL et SET-STYLE qui nomment cet objet directement, c’est-à-dire dont l’entrée est sa poignée, son nom ou un chemin `::` vers lui, ou un ensemble écrit dans `{…}` qui le cite comme membre (y compris lorsque cet ensemble est nommé). Les paramètres de style écrits lors de la définition de l’objet (9.2.5, 9.3) comptent aussi, comme si un SET-STYLE avait été écrit pour l’objet à l’instruction qui le crée.
  - Transmis ou sélectionné : ce qu’un membre reçoit par une sélection ou par génération un par un ; ce qu’un objet propriétaire transmet aux graphiques de son symbole (9.2.5) ; ce qu’un objet transmet aux textes qui lui sont attachés (11.3) ; ce qu’une utilisation transmet à son symbole et aux objets dessinables créés dans son corps, et ainsi de suite de haut en bas à travers les utilisations imbriquées, mais non aux membres qui remplissent ses rôles (12.1 point 4). Un fait transmis garde la position de son instruction d’origine.
  - Pour chaque propriété (dans la file des symboles, le symbole), les faits directs sont examinés en premier et le dernier par position est pris ; s’il n’y en a aucun, les faits transmis ou sélectionnés sont examinés et le dernier par position est pris ; s’il n’y a ni l’un ni l’autre, une instance prend le symbole de bloc par défaut (9.2.6), une utilisation n’est pas dessinée (12.1), et une propriété de style prend la valeur par défaut du moteur de rendu (9.3). Les positions suivent l’ordre de la file ci-dessus.
  - Ainsi un symbole ou un style qu’un éditeur modifie pour un objet n’est pas supplanté par des règles écrites plus tard ; les règles, paquets et thèmes continuent de régir les objets qui n’ont pas été modifiés individuellement.
- `type` et `author` d’un en-tête d’enregistrement n’affectent pas l’ordre (2.8).
- Un symbole est remplacé en bloc ; les styles fusionnent propriété par propriété. Appliquer un style revient à écrire une fois chaque propriété qu’il définit, si bien qu’un style qui ne définit que `linewidth` ne modifie que l’épaisseur de trait.
- Les règles s’appliquent au niveau où elles sont écrites et, lorsqu’elles les atteignent, par `recursive=true` dans une sélection qu’elles écrivent ou, si elles n’en comportent pas, par les ensembles qu’elles utilisent (10.1), aux niveaux inférieurs, jamais aux niveaux supérieurs ; les règles d’un paquet comptent comme écrites dans le niveau de son USE-PACKAGE, et celles d’un paquet à la racine dans le niveau supérieur de son fichier (10.4, 16.2). Elles sont évaluées une fois dans chaque niveau, en n’examinant que les objets de ce niveau.

### 8.4 Surcharge par clé

Les faits pourvus d’une clé (5.8) se surchargent mutuellement par clé : le dernier prend effet (Principes 2.9). La surcharge est une manière légitime de décrire, et non une erreur ; le fait d’émettre ou non une indication est laissé à l’implémentation. PLACE et ORIENT sont des faits géométriques et sont résolus selon 8.5 (5.8).

### 8.5 Conflits

Des faits géométriques pris ensemble peuvent n’avoir aucune solution même lorsque deux d’entre eux pris isolément ne se contredisent pas, tels que `{#M1 #M2} @ALIGN("y")`, `#M1 @PLACE(y=0)` et `#M2 @PLACE(y=10)`. Ce qui suit détermine quels faits sont écartés.

- **Les faits géométriques** sont les quatre contraintes atomiques (ALIGN, ORDER, DISTRIBUTE, DISTANCE), la relation d’orientation ORIENT-RELATION (14.9), TABLE (14.11), chaque coordonnée de PLACE, `dx` et `dy` de TEXT (13.3 ; sur le même axe, ceux-ci et PLACE se remplacent mutuellement, ce qui n’est pas résolu ici) et ORIENT. Les symboles (SET-SYMBOL) n’en font pas partie ; ils sont fixés avant la géométrie (8.2). Il en va de même des critères propres de l’implémentation (les composants ne se chevauchent pas, une grille, etc.) : ce sont les considérations propres du solveur et elles se classent après toutes les descriptions du langage. Si les descriptions forcent des composants à se chevaucher, ils se chevauchent.
- **Unités de résolution** :
  - Une instruction de contrainte atomique constitue une unité, écartée en bloc lors d’un conflit (ORIENT-RELATION également) ; elle n’est jamais retenue pour une partie seulement de ses membres. Cela concorde avec les échecs : les membres d’une contrainte atomique sont des entrées strictes (7.3). Lorsqu’un emplacement qui prend un objet unique reçoit un ensemble et que l’instruction opère une fois pour chaque membre, chacune de ces descriptions est une unité (5.2.1) ; il en va de même pour chaque contrainte que représente une liste de membres, une pour chaque combinaison (14.1) ou chaque paire de DISTANCE (14.5). Une TABLE est une unité (14.11).
  - Un PLACE constitue deux unités, une pour x et une pour y, qui peuvent être écartées séparément. `dx` et `dy` de TEXT constituent de même une unité chacun ; ils remplacent d’abord, ou sont remplacés par, un PLACE du même texte sur le même axe selon la position (13.3), et ceux qui restent prennent part à la résolution.
  - ORIENT constitue une unité : l’orientation de cet objet vaut une valeur, ou l’une des valeurs de sa liste (11.2) ; un ORIENT d’une broche, le fait que l’orientation de son objet soit l’une de celles qui tournent la broche vers la direction, ou vers l’une des directions, écrites. Sans ORIENT il n’y a pas de contrainte : le solveur choisit et réécrit l’orientation (14.8) ; un moteur de rendu qui n’a aucune solution pour elle dessine `"R0"`. Une orientation est une variable à huit valeurs à partir desquelles les positions des bornes sont calculées ; la vérification de conflit peut essayer les orientations directement et n’a pas besoin de les réécrire comme des positions relatives de bornes.
  - Une définition générale n’est pas une contrainte : chaque fait développé à partir d’une utilisation est une unité, ordonné et écarté isolément.
- **Faits qui se contredisent eux-mêmes** : chaque fait est d’abord vérifié isolément. Un fait qui n’a aucune solution à lui seul, tel qu’une DISTANCE d’un objet à lui-même avec `min=4` (9.5), se contredit lui-même : il échoue (7.6) et ne prend aucune part à la résolution. Seuls les faits qui ont une solution à eux seuls sont ordonnés et pris en compte.
- **Ordre de résolution** : les faits géométriques sont rangés dans un ordre unique. Ils sont ordonnés d’abord par `priority`, `"high"` avant `"normal"` avant `"low"` (14.8) ; à priorité égale, par position, le dernier écrit en premier. Positions :
  - par niveau et ordre du texte : les paquets à la racine (16.3), le contenu des niveaux supérieurs, les instructions de ce niveau, les instructions écrites dans ce niveau depuis l’extérieur avec `scope` (celles des niveaux externes plus tard, 2.6) ; le contenu d’un autre paquet est à la position de l’USE-PACKAGE le plus proche de ce niveau (16.2) ;
  - les faits développés à partir d’une définition générale prennent la position de l’instruction AS ou ATTACH, et les faits générés un par un prennent la position de la règle de génération (10.5) ;
  - lorsqu’une instruction produit plusieurs faits, ceux d’un développement suivent l’ordre du corps de la définition, ceux d’une instruction qui opère une fois pour chaque membre suivent l’ordre des membres, et pour PLACE x vient avant y ; dans chaque cas le dernier vient en premier. Les segments d’un chaînage sont des descriptions à part entière, et le segment ultérieur est écrit plus tard (5.5) : les faits de `#R1 @PLACE(0, 0) => @PLACE(5, 5)` viennent dans l’ordre y = 5, x = 5, y = 0, x = 0, et dans `#M1 @PLACE(0, 0) => @ORIENT("R90")` l’ORIENT vient avant le PLACE.
- **Résolution** : les faits sont pris en compte un par un dans l’ordre de résolution ; après chacun, on vérifie si les faits pris en compte jusque-là admettent une solution, et s’il n’y en a aucune, ce fait est écarté. Le résultat est unique : c’est le sous-ensemble cohérent qui est lexicographiquement le plus grand dans cet ordre. Les faits ont une solution lorsqu’il existe des coordonnées réelles, et pour chaque orientation libre l’une de ses huit valeurs, qui les satisfont tous dans la tolérance (11.7) ; les points entiers ne sont qu’un objectif du solveur.
- **Limite** : la vérification d’une solution peut être longue, car certains faits valent de l’une de plusieurs façons qui doivent être essayées tour à tour : une liste d’orientations (11.2, 14.9) et une orientation libre. Une implémentation devrait donc fixer une limite appropriée au temps ou au travail qu’elle consacre à la vérification, afin qu’un grand circuit ne prenne pas trop de temps. La limite est une option du solveur que l’utilisateur peut régler (14.8), donnée par l’outil et non par le fichier (14.10) ; la présente norme n’en fixe ni la mesure ni la valeur. Lorsque la vérification d’un fait n’aboutit pas dans la limite, le fait est écarté, avec un avertissement indiquant qu’il n’a pas été décidé (7.6) ; le résultat peut alors différer de celui défini ci-dessus, et d’une exécution à l’autre. Sans limite, et chaque fois qu’aucune vérification ne l’atteint, le résultat est celui défini ci-dessus.
- Les faits pris en compte sont tous durs pour le solveur ; les faits écartés n’ont aucun autre effet. Les faits portant la priorité `"disabled"` sont retirés avant le classement (14.8).
- Note : la présente section fixe quels faits sont retenus, et non comment ils sont trouvés. Une implémentation peut vérifier toute une série de faits à la fois et ne chercher le fait à écarter que lorsque la série n’a pas de solution, ou ne vérifier que les faits qui partagent des objets avec le nouveau, tant qu’elle retient les mêmes faits.
- Une implémentation qui résout doit résoudre les conflits comme le spécifie la présente section, et son résultat doit satisfaire à tous les faits pris en compte. Une implémentation qui ne résout pas ne dessine que la géométrie explicite et n’est pas tenue de vérifier les conflits entre instructions (Principes 2.4).
- Écarter un fait est une résolution légitime et donne une indication, qu’une implémentation peut élever au rang d’avertissement. Lorsqu’elle est émise, l’indication nomme chaque ensemble de faits pris en compte qui n’a aucune solution conjointement avec le fait écarté et dont aucun fait ne peut être omis, chaque fait étant désigné par l’instruction qui l’a écrit (7.6). Chaque instruction dont les faits figurent dans un tel ensemble reçoit également une indication nommant les faits écartés à cause d’elle, afin que l’auteur voie quelles instructions ont perdu leur effet par son fait. Lorsque la recherche de tous ces ensembles n’aboutit pas dans la limite de temps, l’indication précise que sa liste est incomplète. Écarter un fait `"low"` est attendu, et ses indications peuvent être omises (7.6) : une implémentation n’est pas tenue d’énumérer les ensembles de faits avec lesquels il entre en conflit, ni de donner une indication aux instructions qui y figurent.
- Un conflit n’est pas un échec (7.5). La vérification des conflits de sens de haut niveau entre instructions, ainsi que de l’accord avec la netlist, n’est pas exigée (Principes 2.4).

<!-- translated from en/ch09.md sha256:6683c3ad47d6 -->
## 9 Définitions

### 9.1 Règles communes

- Il existe trois sortes de définition : DEFINE-SYMBOL crée un symbole, DEFINE-STYLE crée un style, et DEFINE crée une définition générale. Les symboles et les styles sont les définitions spécifiques (1.4.4). Chacune prend un ensemble comme corps ; les instructions d’un corps sont tolérantes une par une (7.3).
- Une définition est un objet. Elle est nommée avec `$name =` ou `%name =` (6.6) et s’utilise comme objet d’un mot-clé, dans l’emplacement que le mot-clé lui réserve (5.6).
- Verser une définition ne fait que publier son nom. Une définition ne dessine rien par elle-même et n’a d’effet que lorsqu’elle est utilisée : un symbole lorsque SET-SYMBOL l’attribue, un style lorsque SET-STYLE l’attribue, une définition générale lorsqu’AS ou ATTACH l’applique.
- Une définition écrite directement comme membre d’un enregistrement, non imbriquée dans une instruction, et sans nom n’est jamais utilisée, et produit un avertissement. Une définition anonyme imbriquée dans une instruction est utilisée normalement.
- **Inclusion** : une référence à une définition de la même sorte dans le corps de DEFINE-SYMBOL ou DEFINE-STYLE inclut son contenu à cet endroit. Les membres avec une clé écrits plus tard surchargent ceux écrits plus tôt ; les membres sans clé s’accumulent. Il n’y a pas de paramètre `base=`. Une inclusion qui ramène, directement ou par d’autres inclusions ou par les symboles d’utilisations dessinées dans un corps de symbole (9.2.7), la définition dont le contenu est en cours de construction ferme un cycle : cette inclusion échoue, son diagnostic énumère la chaîne, et le reste du corps prend effet. Le contenu d’une définition est construit à partir de cette définition, si bien qu’avec `$A` incluant `$B` et `$B` incluant `$A`, `$A` est son propre contenu et celui de `$B` sans `$A`, et `$B` l’inverse.
- Les définitions générales ne s’incluent pas les unes les autres ; elles se combinent par AS et ATTACH dans leurs corps. La dérivation et l’extension fonctionnent de la même façon ; il n’existe pas de relation d’héritage (9.4).

### 9.2 Symboles : DEFINE-SYMBOL

#### 9.2.1 Évaluation

- Un symbole est une valeur : son corps est évalué là où il est défini, et les instructions qu’il contient produisent des broches et des graphiques dans les coordonnées locales du symbole. Ceux-ci deviennent le contenu de la définition et ne sont pas versés dans la scène.
- L’ordre des membres est l’ordre de tracé.
- Les coordonnées locales ont la même unité et les mêmes axes que la scène (x vers la droite, y vers le haut), et (0, 0) est l’origine du symbole (6.5) ; la présente version ne comporte pas de mise à l’échelle (11.7). Un point dans le corps doit donner des nombres sur les deux axes ; `_` ne peut pas être utilisé.

#### 9.2.2 Instructions dans le corps

- Le corps ne peut contenir que des graphiques, des broches, des points locaux nommés (tels que `%tip = (0, 0.7)`), des inclusions d’autres symboles, des utilisations appliquées uniquement pour le dessin (avec ATTACH, ou AS, 9.4) accompagnées des SET-SYMBOL et ORIENT écrits sur elles (9.2.7), ainsi que des TEXT attachés à l’objet propriétaire `?`, c’est-à-dire l’objet dessiné avec ce symbole, accompagnés du PLACE qui positionne un tel texte (13.3).
- Toute autre instruction échoue et est écartée (le corps est tolérant) ; les références hôtes échouent aussi.
- Les broches ne peuvent être écrites que dans un corps de DEFINE-SYMBOL. Les graphiques peuvent aussi être écrits dans la scène, en coordonnées de la scène ; ils y sont des annotations (13.4).

#### 9.2.3 Paramètres

- `box=[x1, y1, x2, y2]` : le cadre du symbole, en coordonnées locales et aligné sur les axes ; il est transformé avec l’orientation de l’instance. Le langage lui-même ne l’utilise pas ; les visionneuses l’utilisent habituellement comme cadre de sélection et les solveurs comme limite pour tenir les symboles séparés, et son usage dépend de l’implémentation. Lorsqu’il n’est pas écrit, c’est le cadre englobant tous les graphiques du symbole à l’exception du texte, puisque la taille du texte dépend de la police et du moteur de rendu.

#### 9.2.4 Broches

```rnl
$RES_BOX = @DEFINE-SYMBOL {
  %p = (0, 3) @PIN(terminal=1, label="P", direction="up");
  %n = (0, -3) @PIN(terminal=2, label="N", direction="down");
  @LINE {%p (0, 1.5)};
  @LINE {(0, -1.5) %n};
  (0, 0) @RECTANGLE(width=1.2, height=3)
}
```

- **Une broche est un objet du symbole** : chaque instruction PIN crée une broche, un point qui peut lier des bornes. Dans le corps, une broche nommée peut être utilisée comme un point, tout comme les pattes ci-dessus commencent en `%p` et se terminent en `%n`. Son nom local est aussi son nom comme sous-objet de chaque objet dessiné avec le symbole, tel que `#R1::p` (6.5) ; une broche sans nom local n’a pas de nom et est atteinte par la borne qu’elle lie (5.2.4).
- **Les noms ne sont liés qu’une seule fois** : un nom de broche n’est lié qu’une seule fois dans un symbole, les broches des symboles inclus comprises (9.2.7, 6.6) ; un nom lié à deux broches est une erreur. Les broches ne se remplacent pas mutuellement : chaque instruction PIN crée une broche qui lui est propre.
- Les broches devraient être placées sur des points entiers (11.7) : lorsque l’instance est placée sur un point entier, ses broches et les fils qui s’y connectent tombent aussi sur des points entiers. D’autres positions ne sont pas une erreur ; une implémentation peut émettre une indication. Les graphiques d’un symbole ne sont pas soumis à cette règle.
- `terminal=` : la borne que lie la broche, un nom ou un nombre ; ce sont les noms de bornes du symbole. SET-SYMBOL les met en correspondance avec les bornes de l’objet (11.3) : un nom avec la borne de ce nom, donné par DEVICE (6.12) ou par le nom de port du sous-circuit, un nombre avec la borne à cette place. Pour une utilisation, qui n’a pas de bornes, un nom est mis en correspondance avec le rôle de ce nom dans sa définition, et un nombre ne lie aucun rôle (11.3, 12.1). `terminal=` peut être omis : la broche ne lie rien et n’est que dessinée, comme les sorties d’un interrupteur dessiné comme une figure.
- **Plusieurs broches d’une même borne** : une borne peut être dessinée sur plusieurs broches, comme une alimentation des deux côtés d’un grand bloc. Chacune est une broche à part entière et est atteinte par son nom ; là où la borne est écrite pour une broche, elle représente la broche écrite en dernier (5.2.4). Chaque broche est câblée ou non pour son propre compte (10.2).
- **Broches de bus** : `terminal=` peut prendre un tableau, donnant une broche unique pour plusieurs bornes, comme un port de bus d’un sous-circuit : `(-4, 0) @PIN(terminal=["D<3>", "D<2>", "D<1>", "D<0>"], label="D<3:0>", direction="left")`. Le tableau comporte au moins un élément, et chaque élément est un nombre ou un nom. Les chaînes dans les paramètres ne développent pas les indices (6.11), donc les noms sont écrits un par un.
  - Toutes ces bornes sont dessinées à cette broche : un fil ou une contrainte connecté à l’une quelconque d’entre elles est connecté à cette broche, et la broche est câblée (10.2).
  - Plusieurs broches au même point ne sauraient en tenir lieu : des coordonnées égales ne connectent pas (13.1), un fil ne se connecte qu’à une seule des broches, les autres restent `wired=false`, et les règles de complétion du paquet standard leur ajouteraient des étiquettes et des masses.
- `label=` : le texte affiché à la broche, pour l’affichage uniquement. Ce n’est pas un nom : la broche est désignée par le nom auquel elle est liée dans le symbole, tel que `p` dans `%p = (0, 3) @PIN(…)`, écrit `#R1::p`, et la borne par son nom de borne, tel que `#R1:1` ; `label="P"` ne donne ni `#R1::P` ni `#R1:P` (6.4, 6.5). Le fait qu’il soit affiché et l’endroit où il l’est dépendent de la visionneuse. Il peut utiliser le balisage de texte, tel que `\overline{CS}` (13.6).
- **Une borne non dessinée n’a pas de broche.** Une borne sans broche n’a pas de sens pour le dessin : un fil ou une contrainte qui s’y connecte échoue (« le symbole n’a pas de broche pour cette borne ») ; et la complétion ne la sélectionne pas (10.1). Il n’existe aucune notation pour cacher une borne qui possède une broche ; pour omettre le substrat, utilisez un symbole à trois bornes sans broche de substrat.
- `direction=` : la direction dans laquelle un fil quitte la broche : `"right"`, `"up"`, `"left"` ou `"down"`. Elle est écrite dans les coordonnées locales du symbole et tourne avec l’orientation (11.2). C’est une indication pour les solveurs et les visionneuses, telle que la direction dans laquelle le routage libre quitte la broche, le sens dans lequel circule le courant à la borne (15.2), ou une référence lors du choix des orientations ; par elle-même ce n’est pas une contrainte et elle ne prend aucune part aux conflits, mais ORIENT et ORIENT-RELATION écrits sur la broche la contraignent (11.2, 14.9). Elle peut être omise ; la broche n’a alors aucune direction : une implémentation peut en déduire une des graphiques à titre d’indication, mais ORIENT et ORIENT-RELATION sur cette broche échouent.
- Il n’y a pas de longueur de patte : les pattes sont dessinées dans le symbole avec LINE.
- Il n’y a pas de type électrique : les broches et les bornes n’ont pas de paramètre tel qu’entrée ou sortie, et la netlist n’a pas une telle information non plus. Là où c’est nécessaire, les utilisateurs l’expriment par des utilisations de définitions sémantiques ; un exportateur vers des outils tels que Virtuoso fait correspondre les définitions générales à des types électriques selon sa propre configuration.
- Une broche n’est qu’un point : le fait qu’une patte soit dessinée dépend du symbole, et la manière dont le point d’une broche est marqué (un petit carré, un point ou rien du tout) dépend de la visionneuse. Une broche sans patte ne nécessite donc aucune notation particulière : écrivez PIN et aucun LINE. Par exemple, un dessin dont les fils se connectent à une broche de substrat que son éditeur masque est importé comme un symbole qui retient le PIN de substrat et ne dessine aucune patte de substrat. Les symboles ordinaires, y compris ceux du paquet standard, dessinent des pattes.

#### 9.2.5 Graphiques

Les positions sont des objets points ; les scalaires sont des paramètres. Dans le tableau, *point* désigne tout objet point.

| Forme | Signification |
|---|---|
| `@LINE {point point …}` | Deux points créent un segment de droite, davantage créent une polyligne ; lorsque le dernier point est à la même position que le premier, la figure est fermée et le joint est dessiné comme un coin |
| `@RECTANGLE {point point}` | Un rectangle ; les deux points sont des coins opposés, dans l’un ou l’autre ordre |
| `(0, 0) @RECTANGLE(width=4, height=2)` | Un rectangle ; le point est le centre, et `width` et `height` sont sa largeur et sa hauteur |
| `@CIRCLE {point point}` | Un cercle ; le premier point est le centre et le second est sur le cercle |
| `(0, 0) @CIRCLE(radius=1)` | Un cercle ; le point est le centre |
| `@ELLIPSE {point point}` | Une ellipse ; les deux points sont des coins opposés de son cadre |
| `(0, 0) @ELLIPSE(width=4, height=2)` | Une ellipse ; le point est le centre, et `width` et `height` sont ceux de son cadre |
| `@ARC {point point point}` | Un arc de cercle du premier point au troisième en passant par le deuxième |
| `@BEZIER {point point point point …}` | Une courbe de Bézier cubique de 3n+1 points : le point de départ, puis pour chaque segment deux points de contrôle et un point d’arrivée ; des segments voisins partagent leurs points d’extrémité |
| `(1, 0) @TEXT("…")` | Un texte attaché à un point : texte fixe du symbole, qui ne peut pas changer d’une instance à l’autre |
| `? @TEXT("${name}", key="name") => @PLACE(1, 0)` | Un texte attaché à l’objet propriétaire (l’objet dessiné avec ce symbole) qui lit ses attributs ; sa clé est `name`, donc pour M1 c’est `#M1::name`, et un texte portant la même clé dans la scène le surcharge paramètre par paramètre (13.3) |

- **Deux formes** : RECTANGLE, CIRCLE et ELLIPSE se distinguent selon qu’un paramètre de taille est écrit ou non. Avec une taille (`width` et `height` pour RECTANGLE et ELLIPSE, `radius` pour CIRCLE), l’emplacement d’objet prend un point, le centre ; recevant un ensemble, il dessine une figure pour chaque point (5.2.1). Recevant le résultat d’une sélection, tel que `# @SELECT-PIN => @RECTANGLE(width=0.4, height=0.4, fill="linecolor")`, il est généré un par un à l’étape 6 de 8.2, comme TEXT, et voit donc les symboles ; un graphique écrit avec deux points ne prend pas de sélection (10.4) : combien de points donne une sélection n’est pas connu lors de la lecture, et la forme prend exactement deux points. Sans taille, l’emplacement d’objet prend un ensemble de deux points, et un nombre différent de points fait échouer le graphique. Une taille écrite seulement en partie est un mauvais paramètre, et le graphique échoue. Dans la forme à deux points, les points sont des objets : ils peuvent être nommés ou être les positions de broches, et la figure les suit.
- Les rectangles et les ellipses sont alignés sur les axes ; un rectangle incliné se dessine avec un LINE fermé.
- **Remplissage** : chaque graphique sauf TEXT prend `fill=`. Un LINE, ARC ou BEZIER ouvert est rempli comme la région fermée par une droite reliant son dernier point au premier, et cette ligne de fermeture n’est pas tracée ; un ARC rempli est le segment délimité par la corde et l’arc. Une figure remplie mais non tracée (la grille d’un MOS, une pointe de flèche, un point de jonction) s’écrit `fill="linecolor", linestyle="none"` (9.3).
- **Pointes de flèche** : LINE, ARC et BEZIER prennent `arrow=` : `"none"` (par défaut), `"start"`, `"end"` ou `"both"`, tel que `@LINE(arrow="end") {%from %to}`. La pointe se trouve au point de départ ou d’arrivée et pointe dans la direction de la ligne à cette extrémité : pour LINE, le premier ou le dernier segment ; pour ARC, la tangente à l’arc au point d’extrémité ; pour BEZIER, la droite allant du point de contrôle voisin au point d’extrémité (lorsque les deux coïncident, le point suivant qui n’y coïncide pas). La longueur de la pointe de flèche est donnée par `arrowsize` (9.3), et sa forme dépend de l’implémentation. Les points d’extrémité sont des objets points ; lorsque des contraintes les déplacent, la pointe de flèche suit, sous n’importe quel angle, tandis qu’une utilisation de flèche dessinée en un point n’a que huit orientations. WIRE ne prend pas `arrow=`.
- **Paramètres de style** : les graphiques peuvent prendre des propriétés de style directement comme paramètres (9.3) : LINE, ARC et BEZIER prennent `linecolor`, `linestyle`, `linewidth`, `cap`, `join`, `fillcolor` et `arrowsize` ; RECTANGLE, CIRCLE et ELLIPSE prennent les six premiers ; TEXT prend `textcolor`, `size`, `font`, `bold`, `italic` et `textvisible`. Ceux écrits forment le style direct du graphique, qui se classe au-dessus du style transmis par l’objet propriétaire (8.3), de sorte qu’une couleur écrite dans un symbole ne change pas avec le thème ; ceux non écrits proviennent du style de l’objet propriétaire, de sorte que changer le thème ne nécessite aucune modification du symbole. Le remplissage s’écrit avec `fill=` : `"none"` (par défaut), `"linecolor"` (la couleur de `linecolor`) ou `"fillcolor"` (la couleur de `fillcolor`).
- **Non fournis** : les chaînes de chemin SVG, POLYGON (utilisez un LINE fermé), un graphique de flèche séparé (pour des pointes de flèche aux extrémités de ligne, voir ci-dessus ; ailleurs, utilisez un LINE fermé rempli ou une utilisation de flèche dessinée en un point, 9.2.7), les rectangles arrondis, les arcs elliptiques (approchez-les avec BEZIER), et les images.
- **Texte** : TEXT possède également `size=` (taille de police, 11.7), `halign=` (horizontal : `"start"` (par défaut), `"middle"`, `"end"`), `valign=` (vertical : `"baseline"` (par défaut), `"top"`, `"middle"`, `"bottom"`) et `rotate=` (0, 90, 180 ou 270 ; par défaut 0). Dans un corps de symbole, un TEXT attaché à `?` se réfère aux attributs de l’objet propriétaire, et un TEXT attaché à un point est du texte fixe ; dans la scène, un TEXT est une annotation ou une étiquette selon ce à quoi il est attaché (13.3, 13.4). Le balisage de texte est décrit en 13.6.
- **Ligne de base** : avec `valign="baseline"`, l’ancre (anchor) se trouve sur la ligne de base de la première ligne, et les lignes suivantes vont vers le bas, comme en SVG, PostScript et dans la plupart des éditeurs ; `"top"`, `"middle"` et `"bottom"` alignent la boîte de texte entière. La ligne de base est la valeur par défaut.
- **Direction du texte** : l’ancre est la propre position du texte (13.3). La boîte de texte est placée à côté de l’ancre selon `halign` et `valign`, et tourne autour de l’ancre selon `rotate`, un angle en degrés, dans le sens antihoraire, n’importe quel nombre tel que `30` ; un texte dans un symbole est ensuite transformé avec l’orientation de l’instance, tant les rotations que la symétrie miroir. Avec `upright=true`, valeur par défaut, les caractères dans la boîte se lisent à l’endroit : le texte horizontal de gauche à droite, le texte vertical de bas en haut, et les glyphes ne sont jamais mis en miroir ; un texte dont l’angle final, après l’orientation, est strictement supérieur à 90 degrés et d’au plus 270 est tourné de 180 degrés autour de son ancre. Ainsi `rotate=180` est affiché comme 0 et 270 comme 90, la boîte de texte se trouvant de l’autre côté de l’ancre ; lorsque l’instance est mise en miroir, l’ancre est mise en miroir normalement et la boîte de texte passe de l’autre côté de l’ancre dans le sens de la symétrie miroir, mais les caractères ne sont pas mis en miroir. Cela vaut pour chaque alignement : `start` et `end` échangent leurs places, et de même `top` et `bottom`. Un texte avec `valign="baseline"` est tourné de la même façon : il pend alors au-dessous de son ancre autant qu’il se tenait au-dessus. Un texte avec `fix_rotate=true` garde la direction de ses caractères, et seul le côté de sa boîte suit l’orientation (13.3). Avec `upright=false`, un texte n’est jamais retourné pour la lecture : ses caractères suivent exactement la rotation et la symétrie miroir, glyphes en miroir compris, ce qui convient aux flèches et autres signes écrits comme texte, tels que `→`. Les deux se combinent : `fix_rotate` détermine l’angle final (`rotate` dans la scène, ou `rotate` transformé avec l’objet, une symétrie miroir changeant son signe), et `upright` détermine si cet angle est ensuite tourné de 180 degrés pour la lecture ; un texte avec `fix_rotate=true` n’est jamais mis en miroir. Par exemple, le nom d’un NMOS a son ancre en (1.5, 1.5) et s’étend vers la droite avec `halign="start"` ; en `"MY"`, l’ancre passe en (−1.5, 1.5) et le texte s’étend vers la gauche, en s’éloignant du corps ; en `"R90"`, l’ancre passe en (−1.5, 1.5) et le texte s’affiche verticalement, lu de bas en haut. Les textes dans la scène ne tournent pas avec les instances.
- **Étiquettes en une colonne** : les textes qu’un symbole attache à `?` au même x, avec les mêmes `halign`, `valign`, `rotate`, `fix_rotate` et `upright` et avec un contenu non vide, forment une colonne, lue de haut en bas, comme le nom au-dessus de la valeur d’une résistance. Lorsque l’orientation de l’objet renverserait une colonne, en la mettant sens dessus dessous (`"MX"`, `"R180"`) ou en la couchant sur le côté de sorte qu’elle se lise de droite à gauche (`"R270"`, `"MXR90"`), ses textes échangent leurs places, le premier prenant la place du dernier et ainsi de suite, afin que la colonne se lise dans son ordre là où elle se trouve. Les textes ayant des alignements différents, tels qu’un nom au-dessus d’un cadre et une formule en son milieu, ne forment pas une colonne, et un texte dont la position est donnée par la scène (13.3) n’y participe pas.

#### 9.2.6 Symboles de bloc

Écrit à l’intérieur du sous-circuit `.subckt ota5t … .ends` :

```rnl subckt
%ota5t = @DEFINE-SYMBOL(box=[-4, -4, 4, 4]) {
  (-4, 1) @PIN(terminal="inp");
  (-4, -1) @PIN(terminal="inn");
  (4, 0) @PIN(terminal="out");
  @LINE {(-4, -4) (-4, 4) (4, 0) (-4, -4)};
  ? @TEXT("${name}", key="name") => @PLACE(-1, 0)
};
# @SET-SYMBOL %ota5t
```

- Le symbole qu’un sous-circuit attribue à son propre `#` est l’apparence par défaut des instances de ce sous-circuit, où qu’elles se trouvent : son symbole de bloc. Les broches correspondent aux ports du sous-circuit par nom.
- C’est l’exception à la règle de Principes 2.5 selon laquelle les définitions ne prennent effet que vers le bas : il s’applique aux niveaux qui instancient le sous-circuit.
- Les ports de bus d’un sous-circuit sont déjà scindés en ports simples dans la netlist (6.11) ; un symbole de bloc les dessine en un seul à l’aide d’une broche de bus (9.2.4).
- Dans la file des symboles, il vient après les règles des paquets à la racine et avant les règles des niveaux, y compris les paquets utilisés dans un niveau, de sorte qu’un niveau supérieur peut le remplacer par une règle ou par un SET-SYMBOL pour une instance particulière (8.3).
- Aucune instance n’instancie un niveau de fichier, donc `# @SET-SYMBOL` écrit dans un niveau de fichier est sans effet.
- Un module Verilog-A donne son symbole de bloc de la même façon, avec `# @SET-SYMBOL` écrit dans le module dans son fichier Verilog-A ; ses broches correspondent aux ports du module par nom, et un port vectoriel à une borne par bit (2.9).
- **Le symbole de bloc par défaut.** Une instance à laquelle aucune règle, instruction ni symbole de bloc ne donne de symbole est dessinée avec le symbole de bloc par défaut, construit à partir de ses bornes :
  - **Broches.** Il possède une broche par borne. Pour une instance d’un sous-circuit ou d’un module que le fichier lit (2.9), ce sont les ports dans l’ordre de la définition, chaque broche étant nommée par son port, telle que `terminal="inp"`. Pour toute autre instance, telle qu’une instance d’une cellule dont le fichier ne lit pas la définition ou d’un composant dont aucune règle n’indique la sorte, ce sont les nœuds de sa ligne, chaque broche étant nommée par son numéro, telle que `terminal=1`. La première moitié des broches se trouve sur le côté gauche et le reste sur le côté droit ; lorsque leur nombre est impair, le côté gauche en a une de plus. Chaque broche est étiquetée à l’intérieur du cadre par le nom de son port, ou à défaut par le nom de sa borne (6.12), ou à défaut par son numéro.
  - **Cadre.** Le cadre est un carré dont le côté vaut deux fois le nombre de broches du côté gauche, et au moins 2 ; son centre est l’origine du symbole. Les broches sont espacées de 2 et tournées vers l’extérieur du cadre : celles de gauche descendent à partir de 1 sous le coin supérieur, et celles de droite sont centrées sur leur côté. Ainsi une instance à une borne a un cadre de côté 2 avec sa broche au milieu du côté gauche, et une à trois bornes un cadre de côté 4 avec des broches en (−2, 1), (−2, −1) et (2, 0) ; chaque broche est sur des unités entières.
  - **Textes.** Au-dessus du cadre se trouve le nom de l’instance (`key="name"`), et au-dessous `${model}` (`key="model"`), le nom du sous-circuit ou du modèle (13.6).
  - **Place dans la file.** Il vient après toute la file des symboles (8.3), en dehors du groupe des symboles de bloc, de sorte que toute règle ou instruction qui donne un symbole à l’instance le remplace, de même que le symbole de bloc de son sous-circuit. Il compte comme un symbole pour `drawn` et ne correspond à aucun `symbol=` (10.1).
  - Ainsi chaque instance a un symbole, avec une broche pour chacune de ses bornes, sauf si un symbole qui lui est donné en omet une (9.2.4).
- Lorsqu’un sous-circuit comporte plusieurs instructions `# @SET-SYMBOL`, elles se trouvent dans le même groupe de la file des symboles, dans l’ordre du texte, et la dernière prend effet (8.3) ; une implémentation peut émettre un avertissement.

#### 9.2.7 Variantes et fragments

```rnl
$NMOS_BULK = @DEFINE-SYMBOL(box=[-3, -3, 2, 3]) {
  $NMOS;
  %b = (1, 0) @PIN(terminal=4, label="B", direction="right");
  @LINE {(0, 0) %b}
}
```

- Inclure un autre symbole dans un corps puis y ajouter crée une variante : les broches et les graphiques s’accumulent, et les noms de broches du symbole inclus comptent comme noms de la variante (9.2.4). Pour déplacer une broche, séparez la partie commune en fragments et écrivez les broches dans chaque symbole, comme le fait le paquet standard.
- Un fragment utilisé uniquement dans son propre niveau est nommé avec `%`, tel que `%MOS_CHANNEL = @DEFINE-SYMBOL {…}`, et un autre corps de symbole l’inclut avec `%MOS_CHANNEL;`.
- Les graphiques n’ont pas de clé : l’inclusion ne fait qu’ajouter, et un graphique inclus ne peut pas être retiré. Pour dessiner moins, définissez un autre symbole : séparez la partie commune en fragments et incluez dans chaque symbole les parties dont il a besoin, comme `$NMOS3 = @DEFINE-SYMBOL {%MOS_BODY}` et `$NMOS_4TERM = @DEFINE-SYMBOL {%MOS_BODY; %BULK}`.
- **Utilisations dans le corps** : une figure nécessaire à différentes positions ou sous différentes orientations, telle qu’une pointe de flèche, s’écrit comme une utilisation dessinée en un point :

  ```rnl
  $ARROWHEAD = @DEFINE(roles=[["p", "point"]]) {};
  $ARROWHEAD_SYM = @DEFINE-SYMBOL {
    @LINE(fill="linecolor", linestyle="none") {(0.4, 0) (-0.4, 0.3) (-0.4, -0.3) (0.4, 0)}
  };
  $ISRC_DOWN = @DEFINE-SYMBOL {
    (0, 3) @PIN(terminal=1, direction="up");
    (0, -3) @PIN(terminal=2, direction="down");
    @LINE {(0, 3) (0, 1.5)};
    @LINE {(0, -1.5) (0, -3)};
    (0, 0) @CIRCLE(radius=1.5);
    @LINE {(0, 0.9) (0, -0.5)};
    $ARROWHEAD @ATTACH (0, -0.6) => @SET-SYMBOL $ARROWHEAD_SYM => @ORIENT("R270")
  }
  ```

  - Le premier rôle est rempli par un point du corps, en coordonnées locales. L’utilisation est dessinée avec son symbole en ce point, son orientation tourne autour de ce point, et elle est ensuite transformée conjointement avec le symbole englobant ; les transformations des utilisations imbriquées se composent tour à tour vers l’extérieur, si bien que la pointe de flèche tourne avec le composant.
  - Uniquement pour le dessin : les règles extérieures ne peuvent pas voir à l’intérieur d’un corps de symbole, son symbole est donc attribué avec SET-SYMBOL dans le corps ; une utilisation qui en est dépourvue n’est pas dessinée, et un avertissement est émis. Les corps des définitions générales ne sont pas développés à l’intérieur d’un symbole, et les contraintes et fils n’y ont aucun sens ; ORIENT ici est une donnée interne du symbole, ne prend aucune part aux conflits et n’a pas de priorité.
  - Un SET-SYMBOL ici qui ramène, directement ou par des inclusions et d’autres utilisations de ce type, le symbole en cours de construction ferme un cycle, comme le fait une inclusion (9.1) : ce SET-SYMBOL échoue, l’utilisation n’est pas dessinée, et le reste du corps prend effet.
  - Dessiner une utilisation au point qui remplit son premier rôle ne vaut que dans les corps de symboles, que le solveur ne place pas. Dans la scène, la même notation crée une utilisation ayant son point propre (12.1 point 5), et des règles peuvent lui attribuer un symbole ; les règles d’un paquet de style maison ne peuvent pas pénétrer à l’intérieur des symboles.
- Noms de broches et formules : le fait que les noms de broches soient affichés dépend de la visionneuse (9.2.4) ; une formule est simplement un TEXT qui se réfère à des attributs (13.6), sans règles particulières. Le balisage de texte est décrit en 13.6.

#### 9.2.8 Mise en correspondance des symboles avec les objets

Les symboles ne sont pas divisés en symboles pour composants et symboles pour utilisations. Le fait qu’un symbole puisse être utilisé pour un objet dépend de la correspondance entre ses broches et les bornes de l’objet.

- **Instances sans symbole.** Aucune instance n’est laissée sans symbole : lorsqu’aucune instruction ni règle ne donne de symbole à une instance, elle a le symbole de bloc par défaut (9.2.6), avec une broche par borne, et elle est dessinée et résolue comme toute autre. Un paquet peut laisser un composant sans symbole lorsqu’il ne peut pas en déterminer la sorte, plutôt que de le dessiner comme un composant qu’il n’est peut-être pas, comme le fait le paquet standard pour un transistor dont la carte de modèle n’a pas été lue ; le composant est alors dessiné comme un cadre avec une broche par nœud.

### 9.3 Styles : DEFINE-STYLE

```rnl
$ink = @DEFINE-STYLE(linecolor="#202020", textcolor="#202020", fillcolor="#ffffff", size=1.2) {};
$accent = @DEFINE-STYLE(linecolor="#1f4fd8", linestyle="bold") {$ink};
$shade = @DEFINE-STYLE(fillcolor="#1f4fd833") {}
```

- Les paramètres entre parenthèses sont les propriétés propres du style, et ils prennent effet en dernier. L’ensemble ne peut se référer qu’à d’autres styles, dont les propriétés sont incluses à cet endroit, les plus tardifs supplantant les plus précoces. Un style sans style de base s’écrit avec `{}`.
- Un style ne prend effet que lorsque SET-STYLE l’attribue (11.3). Les mêmes propriétés peuvent également être écrites directement comme paramètres de graphiques, TEXT et WIRE (9.2.5, 13.1), avec des valeurs prises selon les deux paliers de 8.3.
- Un style ne règle que les traits et les couleurs : la position, l’orientation, la forme (les points d’un symbole ou d’un graphique) et le contenu du texte ne font pas partie d’un style.
- **Propriétés** :

| Groupe | Propriété | Valeurs |
|---|---|---|
| Trait | `linecolor` | Une couleur ; également la couleur de `fill="linecolor"` |
| | `linestyle` | `"solid"` (par défaut), `"dashed"`, `"dotted"`, `"dash-dot"`, `"double"` (deux traits continus parallèles), `"bold"`, `"none"` (pas de trait) |
| | `linewidth` | L’épaisseur de trait, en unités de coordonnées (11.7) ; `0` est un filet fin, d’épaisseur constante à tout zoom, et est la valeur par défaut |
| | `cap` | Extrémités de trait : `"butt"`, `"round"`, `"square"` |
| | `join` | Coins : `"miter"`, `"round"`, `"bevel"` |
| Remplissage | `fillcolor` | La couleur de `fill="fillcolor"` |
| Texte | `textcolor` | La couleur du texte ; lorsqu’elle n’est pas donnée, `linecolor` |
| | `size` | La taille de police, en unités de coordonnées (11.7) |
| | `font` | Un nom de police, seulement une indication ; une visionneuse sans cette police utilise la sienne |
| | `bold`, `italic` | Gras ou italique pour tout le texte, `true` ou `false` ; le balisage `\textbf` et `\textit` peut toujours les appliquer à des parties (13.6) |
| | `textvisible` | Visibilité du texte (textvisible) : si le texte est affiché : `true`, par défaut, ou `false`. Un texte avec `false` n’est pas dessiné, ne prend pas de place et ne peut pas être sélectionné : il est exclu des collisions, du cadre d’un symbole et des colonnes de 9.2.5. Son contenu, sa clé, sa position et ses références d’attributs restent et sont évalués normalement ; une visionneuse peut proposer d’afficher les textes masqués. Un paquet qui masque les étiquettes définit un style et le leur attribue : `$hidden = @DEFINE-STYLE(textvisible=false) {};` et `# @SELECT-LABEL(recursive=true) => @SET-STYLE $hidden` ; un texte unique peut aussi le prendre directement, comme `#M1 @TEXT("note", key="memo", textvisible=false)` |
| Autre | `arrowsize` | La longueur des pointes de flèche aux extrémités de ligne, en unités de coordonnées (9.2.5) |

- Un trait n’a que cinq propriétés : couleur, style, épaisseur, extrémité et jointure. `"bold"` dessine deux fois l’épaisseur de trait, et deux fois un filet fin lorsque l’épaisseur vaut 0 ; les longueurs des tirets et espaces des traits discontinus et l’espacement des traits doubles dépendent de l’implémentation, proportionnellement à l’épaisseur de trait. D’autres valeurs de `linestyle` pourront être ajoutées dans des versions ultérieures du langage (16.1).
- `"none"` ne dessine aucun trait, ni aucune pointe de flèche aux extrémités de ligne (elles appartiennent au trait, 9.2.5) ; `fill="linecolor"` prend tout de même la couleur de `linecolor`, de sorte qu’une figure remplie mais non tracée s’écrit `fill="linecolor", linestyle="none"`, et sa couleur continue de suivre un changement de thème. Les fils et les graphiques l’acceptent tous deux ; un fil avec cette valeur est un fil invisible, mais reste un fil (13.1).
- **Couleurs** : elles s’écrivent comme des couleurs hexadécimales CSS : `"#RRGGBB"` ou `"#RRGGBBAA"`, ou les formes courtes `"#RGB"` et `"#RGBA"`. L’alpha vient en dernier : `00` est totalement transparent et `FF` opaque ; sans lui, la couleur est opaque. Les traits et les remplissages ont chacun leur propre couleur, si bien que leur transparence se règle séparément, comme `$shade` ci-dessus ne rend que le remplissage partiellement transparent. La présente version n’accepte ni les noms de couleurs ni `rgb()`. Les couleurs sont des chaînes.
- La présente version ne fournit ni motifs de remplissage, ni dégradés, ni ombres ; ni la visibilité d’objets autres que les textes (les afficher et les masquer relève de la visionneuse, 16.2 ; `textvisible` ne porte que sur les textes, afin qu’une telle visibilité puisse être ajoutée plus tard sous des noms propres, et un texte est alors affiché lorsque les deux valent true) ; ni des objets tels que des thèmes et des profils (un thème est un paquet de styles et de règles, importé avec USE-PACKAGE, 16.2).
- Lorsqu’aucun style ni aucun paramètre ne donne de valeur pour une propriété, la valeur par défaut propre au moteur de rendu est utilisée ; ce n’est pas une erreur (Principes 2.7). Le paquet standard fournit une apparence portable par défaut (16.3), que supplantent les règles et instructions écrites par l’utilisateur.
- Les propriétés inconnues sont traitées comme des erreurs de syntaxe (Principes 2.3).

### 9.4 Définitions générales : DEFINE

```rnl id=gdef
$TAIL_MID = @DEFINE(roles=["a", "b", "t"]) {
  %mid = (_, _);
  {?a %mid ?b} @DISTRIBUTE("left-to-right");
  {%mid ?t} @ALIGN("x")
};
$CHAIN_ROW = @DEFINE(roles=["source", "{chain}", "load"]) {
  {?source ?chain ?load} @ORDER("left-to-right")
};
$GND_BELOW = @DEFINE(roles=[["t", "pin"]], parameters=[["gap", 2]]) {
  %g = $GROUND @ATTACH ?t::net;
  ?t::net @WIRE {?t %g::pin};
  {?t %g::pin} @ALIGN("x");
  %g::pin ?t @DISTANCE("bottom-to-top", exact=?gap)
};
%c1 = #M1:3(a) #M2:3(b) #M5:1(t) @AS $TAIL_MID;
#R1(source) {#R2 #R3}(chain) #R4(load) @AS $CHAIN_ROW;
# @SELECT-INSTANCE => @SELECT-PIN(net="vss", wired=false) => $GND_BELOW @ATTACH(gap=3)
```

- Une définition générale se compose d’une liste de rôles, d’une liste de paramètres et d’un corps. Dans le corps, les rôles et paramètres sont désignés par `?name` (6.10). Ce que fait une définition est décidé par son contenu ; voir 9.5.
- **Application** : AS et ATTACH remplissent les rôles d’une définition avec des objets et produisent une utilisation. Les deux mots-clés font la même chose et ne diffèrent que par l’endroit où s’écrit la définition :
  - AS place la définition en dernier et se lit « ces objets, en tant que tel-et-tel » ; il s’emploie pour le sens et le placement, tel que `#M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N` ;
  - ATTACH place la définition en premier, avant les objets écrits, et la forme standard met le mot-clé après la définition, comme `$GROUND @ATTACH &vss` ; il se lit « tel-et-tel, ajouté sur ces objets », et s’emploie pour les composants virtuels et la complétion (chapitre 12, 10.5).
  - Choisir l’autre mot-clé n’est pas une erreur, seulement maladroit à lire ; écrire la définition au mauvais endroit est une erreur (4.2 point 7). Les objets transmis par un chaînage sont mis en correspondance directement avec les rôles, et les deux mots-clés se ressemblent alors ; choisissez selon ce à quoi sert la définition.
  - Comment choisir : utilisez AS lorsque les objets forment ensemble la chose, comme une structure de circuit, un placement ou une manière locale de dessiner constituée de ces objets ; utilisez ATTACH lorsque la chose est ajoutée sur les objets, comme une masse, une étiquette de port, une marque de non-connexion, un point de jonction, une étiquette pour chaque instance ou la complétion. Les phrases standards sont `#M1(left), #M2(right) and #M5(tail) are treated @AS $DIFFERENTIAL_PAIR_N` et `$GROUND is @ATTACH to &vss` (4.5).
- **Liste de rôles** : le paramètre `roles`, un tableau dont les éléments sont des noms de rôles ou des paires `[name, kind]`. Les deux paramètres de DEFINE ne peuvent être donnés que par nom.
  - La liste peut être vide, `roles=[]` : la définition est appliquée seule, telle que `$PORT_LIST @ATTACH`, et l’utilisation n’a pas d’objet principal ; elle possède tout de même son point propre (14.1).
  - Un élément écrit `"{name}"` est un rôle d’ensemble, qui prend un ensemble entier ; les autres sont des rôles simples (5.2.1 point 4).
  - Un rôle d’ensemble recevant un objet unique le prend comme un ensemble ne contenant que cet objet ; un ensemble vide est valide également, et les contraintes dans le corps qui reçoivent trop peu de membres sont trivialement satisfaites (chapitre 14).
  - Un rôle simple recevant un ensemble applique la définition une fois pour chaque membre, et zéro fois pour un ensemble vide ; dans une même application, au plus un rôle simple peut recevoir un ensemble de plus d’un membre (sinon l’instruction échoue, 5.2.1).
  - Une règle qui applique une définition dont tous les rôles sont des rôles d’ensemble ne crée aucune utilisation dans un niveau où tous sont vides : la règle est évaluée dans chaque niveau qu’elle atteint (10.4), et une utilisation n’est créée que là où ses rôles d’ensemble contiennent quelque chose, comme un rôle simple recevant un ensemble vide applique la définition zéro fois. Une instruction qui n’est pas une règle crée son utilisation même si ses rôles d’ensemble sont vides. Ainsi la complétion de la série standard 0.4 ne crée la colonne d’étiquettes de port d’un niveau que dans les niveaux qui reçoivent des étiquettes de port (12.4).
  - Lorsque le `?name` d’un rôle d’ensemble est écrit à l’intérieur de `{…}`, c’est un élément de cet ensemble (5.2) : dans une contrainte, il représente chacun de ses membres à cet élément (14.1), si bien que l’ORDER dans l’exemple ci-dessus place chacun de R2 et R3 entre R1 et R4.
  - Un rôle d’ensemble recevant un ensemble qui compte des ensembles parmi ses membres le prend tel quel ; le corps décide comment l’utiliser.
  - Un rôle d’ensemble ne correspond pas à une borne (6.4).
  - Dans une même application, chaque rôle ne peut être marqué qu’une seule fois ; pour donner plusieurs objets à un rôle d’ensemble, écrivez-les comme un ensemble, tel que `{#R2 #R3}(chain)` ci-dessus.
  - **Sortes** : un élément écrit `[name, kind]` déclare quelle sorte d’objet prend le rôle ; un élément ne comportant qu’un nom prend n’importe quelle sorte. Les noms de sortes sont ceux de la liste de mots (5.3) : `"instance"`, `"terminal"`, `"pin"`, `"net"`, `"point"`, `"wire"`, `"usage"`, `"bundle"`, `"text"`, `"annotation"` et `"label"`. Dans une déclaration de rôle, `"instance"` prend aussi les faisceaux, et `"terminal"` les bornes des faisceaux (11.8) ; un tableau signifie l’une quelconque de plusieurs sortes, tel que `["p", ["net", "terminal"]]`. Un objet de l’une des sortes est pris tel quel ; un objet d’aucune d’entre elles qui se convertit vers certaines d’entre elles (5.2.4) est converti vers la première d’entre elles dans l’ordre du tableau, si bien que `#R1::p` donné pour `["net", "terminal"]` devient le net de la broche. Un rôle qui prend `"net"` prend aussi le nœud vide, `&` seul, ce qui indique que le rôle n’est sur aucun net (6.3) ; un rôle d’ensemble ne le prend pas, puisque le nœud vide n’est membre d’aucun ensemble. La sorte d’un rôle d’ensemble s’applique à chacun de ses membres, tel que `["{members}", "instance"]`. Il n’existe aucune sorte pour les utilisations d’une définition particulière.
  - Lorsqu’un rôle reçoit un objet de mauvaise sorte, toute l’application échoue, et le diagnostic indique quel rôle, ce qu’il prend et ce qu’il a reçu ; il en va de même lorsqu’un membre d’un rôle d’ensemble a la mauvaise sorte. Lorsqu’une sélection crée les applications une par une, seule l’application qui a reçu le mauvais objet échoue.
  - Note : dans une définition à corps vide, rien ne vérifie ce qui remplit les rôles, donc seule une sorte déclarée détecte un rôle mal rempli ; dans un corps non vide, chaque instruction du développement est également vérifiée isolément (9.5). Les sortes déclarées permettent aussi de vérifier les emplois des rôles dans le corps au niveau de la définition (6.9).
  - **Rôles facultatifs et nombres** : un rôle simple doit être rempli ; un rôle d’ensemble peut être laissé vide, ce qui en fait un ensemble vide. Lorsque des objets sont donnés dans l’ordre et qu’il y a moins d’objets que de rôles, les rôles restants doivent tous être des rôles d’ensemble ; lorsque des objets sont marqués avec des rôles, un rôle d’ensemble non écrit est un ensemble vide. Il n’y a ni nombre minimum ni nombre maximum : les contraintes avec trop peu de membres sont trivialement satisfaites (chapitre 14). Une variante d’une autre forme s’écrit comme une autre définition (voir dérivation et extension ci-dessous). Les règles de validité telles que le même objet remplissant deux rôles, ou les connexions attendues, sont traitées en 9.5.
- **Liste de paramètres** : le paramètre `parameters`, qui peut être omis. Chaque élément est un nom de paramètre ou `[name, default]`.
  - Les paramètres sont passés entre les parenthèses d’AS et ATTACH, tel que `$GND_BELOW @ATTACH(gap=3)`, par position dans l’ordre de la liste de paramètres ou par nom ; lorsque les deux sont utilisés, ceux par position viennent en premier (4.2 point 9). La forme standard les écrit par nom (4.3). Un nom absent de la liste de paramètres fait échouer l’application. Lorsqu’une sélection crée les applications une par une, chaque application utilise les mêmes paramètres.
  - Partout où le corps peut contenir un littéral, un paramètre générique peut être écrit à la place : la valeur d’un paramètre, un élément d’un tableau, une coordonnée d’un point. Ainsi, à l’intérieur d’un corps de définition, la règle selon laquelle les paramètres ne contiennent que des littéraux (4.1) devient « des littéraux ou des paramètres génériques » ; après développement, ce sont à nouveau des littéraux. Les paramètres d’AS et ATTACH eux-mêmes ne peuvent être que des littéraux et ne peuvent pas passer d’objets ; les objets entrent par les rôles.
  - Un paramètre qui n’est pas passé et n’a pas de valeur par défaut compte comme non écrit là où il est utilisé : en tant que paramètre nommé il n’est pas écrit, tel que `size=?size` dans `@TEXT(?label, size=?size)`, après quoi le texte prend la taille par défaut propre à TEXT ou la taille de son style (11.3) ; en tant que coordonnée d’un point, c’est `_` ; dans un paramètre donné par position ou dans un élément d’un tableau, il n’a pas de sens défini, et cette instruction échoue. Aucun marqueur pour les paramètres requis n’est donc nécessaire.
  - Les paramètres sont destinés aux grandeurs telles que les nombres et le texte ; une variante qui diffère par sa structure devrait être écrite comme une définition distincte ; le langage n’impose pas cela. Les paramètres ne sont que transmis dans le corps ; les utilisations ne peuvent pas être sélectionnées selon les valeurs de leurs paramètres.
- Les noms de rôles et les noms de paramètres figurent dans une table unique et doivent tous être différents ; aucun d’eux ne peut être un nom de sous-objet standard (6.5). Un paramètre ne peut pas s’appeler `priority`, qui est un paramètre standard d’AS et ATTACH (14.8).
- **L’utilisation elle-même** : dans le corps d’une définition générale, un `?` isolé désigne l’utilisation en cours de développement, par son point propre (14.1). `{?members::position ?} @ALIGN("y")` place le point de l’utilisation à la hauteur de ses membres, de sorte qu’une contrainte sur ce point, telle qu’un DISTRIBUTE autour d’un axe dans une autre structure, les déplace avec elle ; `::position` maintient en un seul point un membre qui est une utilisation sans symbole (14.1). Un `?` isolé est un objet, non un rôle. Il prend un chemin comme le fait une utilisation (6.5) : `?::position` est son point propre, auquel un fil ou un graphique dans le corps peut aboutir (5.2.4), et `?::` suivi du nom d’une broche de son symbole est cette broche, tel que `?::pin` pour une broche nommée pin.
- **Règles dans un corps** : un corps peut contenir des règles, une sélection et les instructions chaînées à sa suite (10.1). Elles agissent comme des règles du niveau dans lequel se trouve l’utilisation, écrites à la position de son instruction AS ou ATTACH (8.5, 10.4), et prennent effet aux étapes de 8.2 comme le font les règles écrites à cet endroit : SET-SYMBOL dans la file des symboles, les règles des mots-clés de routage à l’étape 5, la génération à l’étape 6, SET-STYLE dans la file des styles. Dans un corps, `#` est ce niveau (10.3). Dans le corps d’une utilisation qui est elle-même générée à l’étape 6, une règle de l’étape 5 ou 6 (10.4) échoue et les autres règles prennent effet aux étapes suivantes, comme l’indique 10.5.
- Les noms en `%` liés dans le corps sont des noms locaux : chaque application crée de nouveaux objets pour eux, qui sont référencés de l’extérieur par l’intermédiaire de l’utilisation (voir ci-dessous). Un objet partagé par plusieurs applications, tel qu’un axe de symétrie, est transmis par un rôle.
- Une application peut marquer les objets avec des rôles (`#M1(a) #M2(b) … @AS $T`, `$T @ATTACH #M1(a) #M2(b) …`) ou les donner dans l’ordre de la liste de rôles (`#M1 #M2 … @AS $T`). Lorsqu’un rôle simple est laissé vide, ou qu’il est donné plus d’objets qu’il n’y a de rôles, l’instruction entière échoue ; un rôle d’ensemble laissé vide est un ensemble vide.
- Une application produit une utilisation, qui consigne la définition et ce qui remplit chaque rôle, et qui peut être nommée (comme `%c1`). Supprimer l’instruction supprime également les descriptions développées à partir d’elle. Une utilisation n’a pas de clé : chaque application est un objet distinct (5.8).
- **Développement** :
  - les paramètres génériques sont remplacés par les arguments et les valeurs de paramètres ;
  - les noms locaux sont remplacés par de nouvelles poignées, « utilisation + nom local », écrites `%c1::mid` ; une utilisation sans nom est indiquée dans les diagnostics par la position de son instruction ;
  - les noms provenant de l’extérieur du corps ne sont pas résolus à nouveau là où la définition est appliquée (6.9) ;
  - le résultat du développement est versé à la position de l’instruction AS ou ATTACH, et il prend cette position dans la résolution des conflits (8.5).
- **Coordonnées dans un corps** : une coordonnée littérale dans le corps d’une définition générale, dans un point tel que `(1, 0)` ou dans PLACE, est un décalage par rapport au point propre de l’utilisation (14.1), selon les axes du niveau : `(1, 0)` est une unité à droite de ce point, et `(0, 0)` est le point lui-même, `?::position`. Un `_` reste libre, et un paramètre générique dans une coordonnée est un décalage une fois remplacé. Seul le point déplace le corps : ORIENT sur l’utilisation tourne son symbole, et non ce que son corps dessine ou place (11.2). Une définition peut ainsi dessiner une marque autour de son point propre, telle que deux LINE qui se croisent en `(0, 0)`, et lier son point à une broche (12.1 point 5). PLACE sur le `?` isolé dans son propre corps échoue, puisque son décalage serait par rapport à lui-même. Hors du corps, comme dans l’enregistrement du solveur, les positions sont des coordonnées du niveau (14.8).
- Les utilisations imbriquées sont atteintes étape par étape (`%c1::inner::mid`). Une utilisation peut aussi remplir un rôle d’une autre définition : avec une définition propre `$OTA = @DEFINE(roles=[["pair", "usage"], ["load", "usage"]]) {}`, `#M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N => %cm1 @AS $OTA` est un amplificateur dont le rôle pair est rempli par l’utilisation de la paire différentielle, et son rôle load par le miroir de courant `%cm1` (5.5).
- **Application cyclique** : une application au sein d’un développement qui applique une définition déjà en cours de développement dans une utilisation englobante, directement ou par d’autres définitions, ferme un cycle. Cette application échoue, et son diagnostic énumère toute la chaîne ; le reste prend effet normalement. Ci-dessous, appliquer `$A` à M1 développe `$B`, dont l’application de `$A` ferme le cycle `$A → $B → $A` et échoue ; l’ORIENT dans `$B` retourne tout de même M1 :

  ```rnl error
  $A = @DEFINE(roles=["x"]) {?x @AS $B};
  $B = @DEFINE(roles=["x"]) {
    ?x @AS $A;               /* closes the cycle $A → $B → $A: this application fails */
    ?x @ORIENT("MY")
  };
  #M1 @AS $A
  ```

- **Référence aux objets locaux depuis l’extérieur** :

  ```rnl use=gdef
  $DP_BLOCK = @DEFINE(roles=["in_p", "in_n"]) {
    %pair = {?in_p ?in_n} @GROUP
  };
  %dp1 = #M1(in_p) #M2(in_n) @AS $DP_BLOCK;
  %c1::mid @PLACE(x=20);              /* moves the midpoint of c1 */
  {%c1::mid #M9:1} @ALIGN("x");       /* aligns the drain of M9 with the midpoint of c1 */
  %dp1::pair @PRIORITY("disabled")    /* turns off only the group expanded for dp1 */
  ```

  - Tout objet local nommé d’un corps peut être référencé de l’extérieur par l’utilisation, écrit `usage::local_name` ; ceux sans nom ne peuvent pas être atteints, sauf par numéro depuis l’enregistrement du solveur (5.9). En nommer un ou non relève du choix de l’auteur de la définition, selon le principe « pour se référer à un objet, nommez-le » (5.7) ; il n’y a pas de liste d’exportation séparée.
  - Les chemins peuvent être enchaînés : `%st1::dp::mid` est le mid de l’utilisation locale dp dans le corps de st1. Passer par un rôle fonctionne de la même façon : `%st1::pair::mid` est le mid de l’utilisation qui remplit le rôle pair.
  - Ce sont des références ordinaires : des faits peuvent leur être ajoutés (PLACE, SET-STYLE, TEXT, PRIORITY, etc.), ils peuvent être utilisés dans d’autres instructions, et ils peuvent remplir des rôles d’autres définitions ; les conflits avec les faits du développement sont résolus selon 8.5. Un nom local ne peut pas être lié à nouveau depuis l’extérieur : `%c1::mid = …` est une erreur.
  - Noms : les noms atteints par `::` sur une utilisation doivent être uniques. Un nom local ne peut donc pas être identique à un nom de rôle ou à un nom de sous-objet standard, ce qui est vérifié à la définition, et la clé d’un texte sur l’utilisation ne peut être identique à aucun de ces noms (6.5, 13.3). Un nom local peut être identique à un nom de paramètre : les paramètres sont des valeurs, et `::` n’atteint pas les valeurs.
  - Priorité : une instruction PRIORITY écrite depuis l’extérieur directement sur un objet local supplante la priorité de l’utilisation (14.8).
  - Hors d’atteinte : les objets locaux des utilisations sans nom, sauf par numéro depuis l’enregistrement du solveur (5.9), et ceux des utilisations générées une par une par une sélection. Une sélection ayant une utilisation pour portée ne sélectionne toujours que les membres de ses rôles, non ses objets locaux (10.1) ; à travers une utilisation, les objets locaux ne sont atteints que par nom. Une sélection ayant le niveau pour portée les sélectionne comme objets de ce niveau, comme elle sélectionne tout autre objet, tel que `# @SELECT-USAGE $FILTER` ci-dessous ; ce qu’une règle ne peut pas faire est nommer un objet local d’une définition qu’elle n’a pas écrite.
  - Les noms locaux font donc partie de l’interface d’une définition : lorsqu’une définition modifie un nom local, les références depuis l’extérieur ne le trouvent plus et échouent (chapitre 7), le résultat du solveur devient périmé avec elles, et une indication suggère de résoudre à nouveau (chapitre 17). Le paquet standard précise quels noms locaux de ses définitions sont fiables.
- **Notation dans le corps** : les rôles et paramètres s’écrivent `?name`, les noms locaux `%name` (6.9, 6.10).
- **Dérivation et extension** : uniquement par composition ; il n’y a pas de relation d’héritage (9.1). Le corps d’une définition dérivée applique la définition de base et ajoute ses propres instructions :

  ```rnl
  $FILTER = @DEFINE(roles=["in", "out"]) {};
  $LOWPASS = @DEFINE(roles=["in", "out", "r", "c"]) {
    %f = ?in(in) ?out(out) @AS $FILTER;
    {?r ?c} @ORDER("left-to-right")
  }
  ```

  - Le contenu est ajouté par des instructions dans le corps, et les variantes numériques par des paramètres ; pour masquer et étendre une définition dans un niveau inférieur, le corps se réfère à la définition externe de même nom (6.9).
  - Sélectionner selon la définition de base ne nécessite aucune règle spéciale : l’utilisation FILTER développée depuis la définition dérivée se trouve dans la scène, si bien que `# @SELECT-USAGE $FILTER` la sélectionne, et les règles écrites pour `$FILTER` fonctionnent également sur les filtres passe-bas, en n’agissant que sur l’utilisation FILTER à l’intérieur et non sur les instructions qu’ajoute la définition dérivée.
  - Il n’existe pas de mécanisme séparé pour écarter une partie de la définition de base : le corps d’une définition dérivée peut écrire `%f::x @PRIORITY("disabled")` pour désactiver une partie de celle-ci, mais une priorité sur l’utilisation externe supplante cela (14.8) ; pour omettre une partie de façon fiable, scindez la définition de base en définitions plus petites, ou écrivez-en une autre.

### 9.5 Usages des définitions générales

Ce que fait une définition est décidé par son contenu, et non par un mot-clé :

```rnl
$PAIR = @DEFINE(roles=["in_p", "in_n"]) {};                /* meaning: roles only */
$SIDE_BY_SIDE = @DEFINE(roles=["a", "b"]) {               /* layout: constraints in the body */
  {?a ?b} @ALIGN("y");
  {?a ?b} @ORDER("left-to-right")
};
$PROBE = @DEFINE(roles=[["p", "net"]]) {};                 /* virtual part: a rule gives its usages a symbol (chapter 12) */
$PROBE_SYM = @DEFINE-SYMBOL {(0, 0) @PIN(terminal="p"); (0, 0) @CIRCLE(radius=0.5)};
# @SELECT-USAGE $PROBE => @SET-SYMBOL $PROBE_SYM
```

Dans le paquet standard, `$CURRENT_MIRROR_N`, `$MIRRORED_LAYOUT` et `$GROUND` appartiennent à ces trois sortes ; `$DIFFERENTIAL_PAIR_N` possède ses rôles et, dans son corps, un placement par défaut (ci-dessous).

- Ce sont tous la même sorte de chose : une utilisation (consignant quelle définition a été appliquée et ce qui remplit chaque rôle), plus les descriptions développées à partir du corps. Les deux parties ont des utilisateurs différents :
  - les contraintes développées à partir du corps sont traitées par le solveur, doivent être satisfaites, et sont résolues selon 8.5 en cas de conflit ;
  - l’utilisation est destinée à être lue par les humains et les LLM, et aux sélections et règles de RNL lui-même (10.1).

  Ainsi les utilisations de définitions de placement portent elles aussi du sens ; une définition à corps vide, telle que `$PAIR`, consigne du sens par ses seuls rôles, et son sens est énoncé par le paquet standard ou par son auteur. Une définition qui consigne du sens peut également porter un dessin dans son corps (ci-dessous).
- Une utilisation sémantique n’est pas un commentaire : ses références doivent être trouvées, ses rôles doivent correspondre, sa validité de base est vérifiée (Principes 2.4), et les sélections et règles peuvent l’utiliser ; seule la façon dont elle est dessinée n’est pas spécifiée.
- « Ce que c’est » et « comment c’est dessiné » peuvent s’écrire en une définition ou en deux. Une définition peut porter un dessin par défaut dans son corps, à priorité `"low"` : les faits propres de l’auteur passent avant lui, et la priorité de l’utilisation le désactive ou l’élève, comme `%dp1 @PRIORITY("disabled")`, ou `# @SELECT-USAGE $DIFFERENTIAL_PAIR_N => @PRIORITY("disabled")` pour chaque utilisation de la définition, tandis que l’utilisation continue de consigner ce que sont les objets (14.8). Le passage du sens au dessin peut aussi s’écrire comme une règle dans un paquet, et changer de paquet change le dessin : `# @SELECT-USAGE(recursive=true) $DIFFERENTIAL_PAIR_N => @AS $DP_LAYOUT` ; supprimer l’utilisation de placement n’affecte pas l’utilisation sémantique. Le paquet standard écrit ses dessins par défaut dans les corps de ses définitions, chaque contrainte de ceux-ci à priorité `"low"`, comme le transistor de pied d’une paire différentielle sous le milieu de la paire, ou les membres de `$BRANCH` en une colonne de haut en bas (16.3).
- **Le dessin est toujours écrit en RNL** : un sens n’est dessiné d’une certaine façon que par ce qu’écrit RNL, dans le corps de sa définition ou dans une définition de placement appliquée par une règle, comme ci-dessus, éventuellement dans un paquet ; ne vous fiez pas à des conventions par défaut d’un moteur de rendu pour un sens. Le fait qu’une définition porte un dessin dépend de son paquet ; la présente norme ne pose aucune autre règle à ce sujet. La façon dont sont placés les composants que RNL ne décrit pas dépend de l’implémentation (Principes 1), mais ce n’est pas une convention de dessin pour un sens et on ne peut pas s’y fier. Le fait qu’un moteur de rendu décide du style par défaut des objets sans descriptions (Principes 2.7), et qu’une instance sans symbole ait le symbole de bloc par défaut (9.2.6), est une tout autre question.
- **Le vocabulaire appartient au paquet standard** : les premières définitions sémantiques et les composants virtuels standards sont le contenu du paquet standard, qui ne fait pas partie de la norme du langage (16.3). La manière dont les définitions du paquet standard énoncent leur sens et leurs rôles relève également du paquet standard. Le sens de la définition d’un utilisateur est son corps ; son nom n’est destiné qu’à la lecture par des humains.
- **Validité de base** (Principes 2.4) : la syntaxe des définitions ne vérifie pas si deux rôles sont remplis par le même objet. La validité de base d’une application est : les références sont trouvées, les rôles reçoivent les bonnes sortes, les rôles simples sont remplis (9.4), et chaque instruction du développement est valide par elle-même.

  ```rnl error
  $DP_LAYOUT = @DEFINE(roles=[["in_p", "instance"], ["in_n", "instance"]]) {
    {?in_p ?in_n} @ALIGN("y");
    ?in_p ?in_n @DISTANCE("left-to-right", min=4)
  };
  #M1(in_p) #M1(in_n) @AS $DP_LAYOUT      /* the expanded DISTANCE asks M1 to be at least 4 from itself */
  ```

  - Dans cet exemple, la DISTANCE développée se contredit elle-même : elle échoue avec un diagnostic, et elle seule est écartée (5.5, 7.3) ; l’ALIGN vaut normalement. Une définition à corps vide, telle que `$PAIR` ci-dessus, ne peut pas détecter un tel remplissage : `#M1(in_p) #M1(in_n) @AS $PAIR` ne signale aucune erreur.
  - Un composant virtuel dont les deux extrémités sont sur le même net a un sens défini, donc « le même objet remplit deux rôles » ne peut pas être une erreur en général.
  - D’autres vérifications, telles qu’exiger que deux rôles soient différents, s’écrivent sous forme d’assertions (15.3).
- **Mots-clés et définitions générales** (1.5 point 7) : ce qu’une implémentation doit comprendre elle-même, et ce qui ne peut pas être écrit avec d’autres descriptions, est un mot-clé, comme les contraintes atomiques, GROUP, WIRE, TEXT et CURRENT ; ce qui peut être écrit dans le corps d’une définition générale est une définition générale, comme les paires différentielles, miroirs de courant, masses et alimentations, et flux de signal (14.6). DEFINE-SYMBOL, DEFINE-STYLE, SET-SYMBOL et SET-STYLE sont aussi des mots-clés : un corps de symbole est en coordonnées locales et est évalué là où il est défini ; les propriétés d’un style sont des littéraux ; SET-SYMBOL et SET-STYLE fixent une propriété d’objets existants et sont décidés en files une fois tous les objets existants, tandis qu’AS et ATTACH produisent un nouvel objet à chaque fois.
- Une définition est identifiée par la définition vers laquelle son nom se résout, et non par la chaîne du nom : lorsqu’un niveau inférieur définit son propre `$DIFFERENTIAL_PAIR_N`, il s’agit d’une autre définition, et les règles d’un paquet écrites pour le `$DIFFERENTIAL_PAIR_N` standard ne la reconnaissent pas.

<!-- translated from en/ch10.md sha256:bff47dee5256 -->
## 10 Sélections, règles et génération un par un

Ce chapitre couvre les mots-clés de sélection, les règles qui les utilisent et la génération un par un.

### 10.1 Mots-clés de sélection

Il existe un mot-clé de sélection pour chaque sorte d’objet sélectionné, chacun avec un jeu fixe de conditions. Le résultat d’une sélection est un ensemble (5.2).

Chaque mot-clé de sélection prend un objet de portée. `#` est le niveau dans lequel la sélection est évaluée (10.3). Recevant un objet ou un ensemble, la sélection cherche parmi ces objets : recevant un ensemble de nets, par exemple, SELECT-TERMINAL sélectionne les bornes sur ces nets (la façon dont la portée mène à la sorte sélectionnée est décrite après le tableau). La portée ne peut pas être omise, donc une sélection simple s’écrit aussi avec `#` : `# @SELECT-INSTANCE(name="R*") => @SET-SYMBOL $RESISTOR`.

| Mot-clé | Sélectionne | Conditions |
|---|---|---|
| SELECT-INSTANCE | Instances hôtes. Avec des bornes dans la portée, les instances auxquelles elles appartiennent ; avec des nets, les instances qui y ont une borne | `name` (le nom de l’instance, tel que `name="R*"`) ; `model` (le nom en position de modèle sur la ligne d’instance : un nom de `.model` ou un nom de sous-circuit) ; `type` (le type de l’instance, issu de son `.model` ou d’un DEVICE, tel que `"nmos"` ; prend `*`) ; `terminals` (le nombre de bornes de l’instance, tel que `terminals=3`) ; `symbol` et `drawn` (ci-dessous) |
| SELECT-TERMINAL | Bornes d’instances hôtes, qu’elles aient des broches ou non : une sélection du circuit | `name`, `model` et `type` de l’instance ; `terminal` (un nom ou numéro de borne) ; `net` (le nom du net sur lequel elle se trouve) ; `current` (le sens du courant, 15.2 : `"in"`, `"out"` ou `"unspecified"`) |
| SELECT-PIN | Broches d’objets dessinés : une sélection du dessin (9.2.4) | `name`, `model` et `type` de l’objet propriétaire de la broche ; `terminal` (un nom de borne ou de rôle que lie la broche) ; `net` (le nom de son `::net`) ; `wired` (10.2) |
| SELECT-NET | Nets. Avec des instances, bornes ou fils dans la portée, les nets sur lesquels ils se trouvent | `name` ; `port` (s’il s’agit d’un port de ce niveau) ; `wired` (10.2) ; `global` (s’il s’agit d’un net global, 6.3) |
| SELECT-WIRE | Fils. Avec des nets dans la portée, les fils dessinés sur eux | `net` (le nom du net du fil ; un bus correspond lorsque l’un de ses nets correspond) ; `bus` (`true` : uniquement les bus, 13.1 ; `false` : uniquement les fils qui ne sont pas des bus, dont l’emplacement de net reçoit un net plutôt qu’un ensemble) |
| SELECT-LABEL | Étiquettes, c’est-à-dire les TEXT attachés à des objets (13.3). Avec des objets dans la portée, les étiquettes qui leur sont attachées | `key` (la clé du texte ; prend `*` et est sensible à la casse ; une étiquette sans clé ne correspond à aucun `key=`) |
| SELECT-ANNOTATION | Annotations, c’est-à-dire les graphiques dans la scène et les TEXT attachés à des points (13.4) | Aucune condition autre que `role` et `recursive` |
| SELECT-USAGE | `scope @SELECT-USAGE $D` : les utilisations de la définition `$D` dans la portée, en regardant vers l’intérieur. Avec `#`, les utilisations du niveau ; avec des utilisations, ces utilisations elles-mêmes et les utilisations à l’intérieur d’elles. Sans `$D`, les utilisations de toute définition | La définition est le dernier objet (5.6) et peut être omise. `symbol` et `drawn` (ci-dessous) |
| SELECT-ANCESTOR-USAGE | `objects @SELECT-ANCESTOR-USAGE $D` : les utilisations de `$D` qui ont ces objets comme membres, en regardant vers l’extérieur ; les membres indirects comptent | La définition est le dernier objet (5.6) et est requise. Aucune condition autre que `recursive` |

En outre, chaque mot-clé de sélection prend la condition `recursive`, et les sept premiers prennent `role`, qui n’a de sens que lorsque la portée contient des utilisations (les deux ci-dessous).

- **Un mot-clé par sorte.** Chaque mot-clé de sélection porte le nom de la sorte qu’il sélectionne, tel que `&clk @SELECT-WIRE => @SET-STYLE $signal` pour les fils d’un net, ou `#M1 @SELECT-LABEL` pour les étiquettes de M1. Aucune sélection ne prend les objets de toutes les sortes : pour couvrir chaque sorte, écrivez une règle par sorte, comme le fait le paquet standard pour donner à chaque objet son style de base.
- **Comment la portée mène à la sorte sélectionnée.** Chaque objet de la portée effectue un pas vers la sorte sélectionnée ; un objet qui n’a pas un tel pas ne sélectionne rien, ce qui n’est pas une erreur :
  - SELECT-INSTANCE : une instance est elle-même ; une borne mène à son instance ; un net mène aux instances qui y ont une borne.
  - SELECT-TERMINAL : une instance mène à ses bornes ; un net mène aux bornes qui s’y trouvent ; une borne est elle-même ; une broche mène aux bornes qu’elle lie.
  - SELECT-PIN : un objet dessiné (une instance, un faisceau, une utilisation pourvue d’un symbole) mène aux broches de son propre symbole, et avec `role`, une utilisation mène à la place aux broches qui remplissent ce rôle, comme les autres sélections prennent les membres (ci-dessous) ; une utilisation sans symbole mène aux broches parmi les objets qui remplissent ses rôles ; une borne mène aux broches qui la lient ; un net mène aux broches dont le `::net` est ce net, y compris les broches liées à des rôles qu’il remplit ; une broche est elle-même. Avec `#`, toutes les broches du niveau.
  - SELECT-NET : un net est lui-même ; une instance mène aux nets de ses bornes ; une borne mène à son net ; un fil mène aux nets sur lesquels il est dessiné.
  - SELECT-WIRE : un net mène aux fils dessinés sur lui ; un fil est lui-même. SELECT-LABEL : un objet mène aux étiquettes qui lui sont attachées ; une étiquette est elle-même. SELECT-ANNOTATION ne prend que les annotations dans la portée.
  - Un pas à travers des bornes compte chaque borne, avec ou sans broche : les bornes appartiennent au circuit. Ce qui est dessiné est sélectionné par les broches. Ainsi `&vss @SELECT-INSTANCE` inclut un NMOS dont la seule borne sur vss est un substrat que son symbole ne dessine pas. Un port de ce niveau, tel que `#:inp`, mène à son net.
  - Avec des utilisations comme portée, seuls les membres de la sorte sélectionnée sont pris ; les membres ne sont pas réutilisés comme portée (voir ci-dessous).
  - Par exemple, les transistors commandés par l’horloge sont `&clk @SELECT-INSTANCE` ; uniquement ceux dont la grille est sur clk sont `&clk @SELECT-TERMINAL(terminal="g") => @SELECT-INSTANCE` ; les nets de M1 sont `#M1 @SELECT-NET`.
  - Note : une règle qui ajoute des masses, fils, étiquettes ou styles au dessin sélectionne des broches (SELECT-PIN), afin que ce qu’elle ajoute corresponde à ce qui est dessiné ; une règle qui vérifie le circuit sélectionne des bornes.
- **Par symbole : `symbol` et `drawn`.** SELECT-INSTANCE et SELECT-USAGE prennent `symbol="$X"`, une chaîne : l’objet est dessiné avec un symbole donné via ce nom en `$`, le nom écrit dans le SET-SYMBOL qui l’a attribué, comparé tel qu’écrit et avec le joker `*`, tel que `symbol="$IO_PORT*"` ; un symbole donné via un nom en `%` ou écrit anonymement ne correspond à aucun `symbol=`. Un alias d’un nom en `$` dans ce SET-SYMBOL est lu comme le nom d’origine (16.5), donc c’est l’original qui est comparé ; la chaîne de `symbol=` n’est pas traduite (16.5) et s’écrit avec le nom d’origine. À la différence de la définition de SELECT-USAGE, qui est reconnue par ce vers quoi son nom se résout (9.5), il s’agit ici d’une comparaison de noms ; les paramètres ne contiennent que des littéraux (3.7), donc le nom s’écrit sous forme de chaîne, comme d’autres conditions écrivent des noms. Ils prennent aussi `drawn` (`true` : l’objet est dessiné, avec un symbole donné par une instruction, une règle, un symbole de bloc ou le symbole de bloc par défaut (9.2.6) ; `false` : il ne l’est pas, comme une utilisation sans symbole). Toute instance est dessinée. Ces conditions lisent les symboles, elles fonctionnent donc dans les règles de génération et dans la file des styles, qui interviennent après l’attribution des symboles (8.2) ; écrites dans une sélection de SET-SYMBOL, elles sont une erreur, car la file des symboles ne peut pas sélectionner selon le symbole qu’elle est en train d’attribuer. SELECT-PIN lit les symboles aussi, avec la même restriction, tout comme toute sélection dont la portée est une broche ou passe par des broches, telle que `#R1::p @SELECT-TERMINAL` : dans une sélection de SET-SYMBOL, c’est une erreur. Il en va de même pour une portée écrite avec `::` et un nom qui n’est ni un nom de rôle, ni un nom local, ni un nom de sous-objet standard de l’objet, tel que `#X1::k` : le fait qu’il nomme une broche ou une clé d’un texte apporté par le symbole n’est connu qu’une fois les symboles attribués (6.5).
- **Ce niveau, ou aussi les niveaux inférieurs.** Chaque mot-clé de sélection possède la condition `recursive`, `false` par défaut. Une règle est évaluée dans le niveau où elle est écrite, et jusqu’où elle descend est décidé comme suit ; à chaque évaluation, chaque sélection et chaque ensemble qu’elle contient sont évalués dans ce niveau et ne contiennent que les membres de ce niveau (10.3).
  - **Une règle comportant une sélection qui y est écrite**, dans son chaînage ou dans un `{…}` qui y est écrit, atteint les niveaux inférieurs lorsqu’une de ces sélections écrit `recursive=true`, et seulement dans ce cas ; les ensembles qu’elle utilise ne changent rien à cela. Ainsi `# @SELECT-INSTANCE(recursive=true) => @SELECT-PIN(net="vss", wired=false) => $GROUND_STUB @ATTACH` ajoute des masses dans ce niveau et dans chaque niveau au-dessous, tandis que `# @SELECT-NET(name="vdd") => @REMOVE-FROM $PORT_FILL_SET` n’agit que dans son propre niveau.
  - **Une règle ne comportant aucune sélection écrite**, qui prend des ensembles dépendant de sélections (10.4), atteint chaque niveau au-dessous du sien qu’atteint l’un de ces ensembles. Un ensemble atteint son propre niveau ainsi que les niveaux qu’atteignent les instructions qui y ajoutent des résultats de sélections : l’instruction à laquelle il est lié, les ensembles parmi ses éléments, et les instructions ADD-TO qui lui ajoutent, où un ADD-TO sans sélection écrite compte comme atteignant aussi loin que l’ensemble qu’il ajoute, et non que l’ensemble auquel il ajoute. Dans un niveau qu’un ensemble n’atteint pas, il n’a aucun membre, y compris lorsqu’une règle qui y parvient l’utilise. Ainsi avec `$GROUND_NET_SET = # @SELECT-NET(name="0", recursive=true)`, l’ensemble contient le net 0 de chaque niveau, `%g_set = {$GROUND_NET_SET}` atteint aussi loin, et `$IO_PORT @ATTACH $PORT_FILL_SET` ou `$SUPPLY_NET_SET @REMOVE-FROM $PORT_FILL_SET` agit dans chaque niveau où parviennent ces ensembles.
  - Un objet hôte unique est simplement référencé, tel que `&vss` : les références hôtes ne se résolvent que dans leur propre niveau, et un ensemble de celles-ci, tel que `{&vss}`, n’atteint aucun niveau inférieur.
- **SELECT-USAGE et SELECT-ANCESTOR-USAGE.** Tous deux reconnaissent une définition par ce vers quoi son nom se résout (9.5), sélectionnent toujours des utilisations, et n’ont pas de `role=`.
  - SELECT-USAGE regarde vers l’intérieur, comme les autres mots-clés de sélection. Avec `#` comme portée, il sélectionne les utilisations du niveau en cours d’évaluation ; avec une utilisation ou un ensemble d’utilisations, ces utilisations elles-mêmes et les utilisations à l’intérieur d’elles, jusqu’aux plus internes. Les instances, nets et bornes dans la portée ne sont pas des utilisations et ne sélectionnent rien ; un avertissement renvoie à SELECT-ANCESTOR-USAGE pour les utilisations dont ils font partie.
  - SELECT-ANCESTOR-USAGE regarde vers l’extérieur : il sélectionne les utilisations qui ont un objet de la portée comme membre, directement ou indirectement (voir l’exemple ci-dessous). Les utilisations dans la portée ne sont pas incluses elles-mêmes. Avec `#` comme portée, c’est une erreur : un niveau n’est membre de rien.
- **Utilisations comme portée.** Avec des utilisations comme portée, les autres mots-clés de sélection sélectionnent les membres de leur sorte dans ces utilisations, et `role=` ne prend que les membres de ce rôle. SELECT-PIN ne diffère que pour une utilisation pourvue d’un symbole et sans `role`, qui donne les broches de son propre symbole (ci-dessus). Les membres qui sont des utilisations sont suivis vers l’intérieur, jusqu’aux plus internes :

  ```rnl
  $OTA = @DEFINE(roles=[["pair", "usage"], ["load", "usage"]]) {};
  %dp1 = #M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N;
  %cm1 = #M3(ref) #M4(out) @AS $CURRENT_MIRROR_P;
  %ota1 = %dp1(pair) %cm1(load) @AS $OTA;
  %all_set = %ota1 @SELECT-INSTANCE;                 /* {#M1 #M2 #M5 #M3 #M4} */
  %pair_set = %ota1 @SELECT-INSTANCE(role="pair");   /* {#M1 #M2 #M5} */
  %up_set = #M1 @SELECT-ANCESTOR-USAGE $OTA          /* {%ota1} */
  ```

  - La valeur de `role=` accepte également les alias `definition=` de la définition de l’utilisation (16.5). Pour les seuls membres directs, utilisez le rôle comme sous-objet, tel que `%dp1::tail`. Le parcours s’arrête sur une utilisation déjà visitée. Les membres sont les objets qui remplissent les rôles, et non les objets locaux du corps (9.4).
  - **Uniquement des membres de la sorte sélectionnée.** Les membres ne sont pas réutilisés comme portée pour aller plus loin, et les membres d’autres sortes ne comptent pas. SELECT-TERMINAL ne prend que les membres bornes : après `$GROUND @ATTACH &0; $CURRENT_ARROW @ATTACH #M5:1`, `# @SELECT-USAGE => @SELECT-TERMINAL` vaut uniquement {#M5:1}, non les bornes sur le net 0. Les membres de la paire différentielle %dp1 sont des instances, donc `%dp1 @SELECT-TERMINAL` est vide. Pour les bornes des instances membres, écrivez `%dp1 @SELECT-INSTANCE => @SELECT-TERMINAL(terminal=2)` (les grilles des trois transistors, transistor de pied inclus) ; pour les bornes sur les nets membres, `… => @SELECT-NET => @SELECT-TERMINAL`. Chaque segment fait une seule chose ; pour aller plus loin, ajoutez un segment. Les objets donnés directement dans la portée suivent les règles ci-dessus : un net donne les bornes sur lui, une instance ses bornes.
  - **Un rôle rempli par une conversion** (5.2.4), tel qu’un rôle de sorte `"pin"` recevant une borne, contient l’objet tel qu’écrit jusqu’à l’attribution des symboles, et l’objet converti à partir de là. Ainsi, dans les règles de génération et ensuite, `@SELECT-PIN(role="t")` sélectionne la broche, que le rôle ait reçu la broche par son nom ou une borne qui la représente.
- Pour trouver des objets dans la scène par condition, utilisez toujours un mot-clé de sélection ; `::` ne prend que des parties de l’objet lui-même (6.5).
- **Faisceaux.** Les membres d’un faisceau ne peuvent pas être sélectionnés ; le faisceau est sélectionné à la place (11.8). SELECT-INSTANCE sélectionne le faisceau lui-même selon les conditions de son premier membre, une fois par faisceau. SELECT-TERMINAL sélectionne les bornes du faisceau, et `net=` correspond lorsqu’il correspond au net de l’un quelconque des membres.
- **Les conditions** s’écrivent par nom, sous la forme `name=value` ; un objet n’est sélectionné que lorsque toutes les conditions écrites sont vérifiées. Pour « ou », écrivez plusieurs règles. Les conditions qui comparent des noms (`name`, `model`, `net`) prennent le joker `*`, qui correspond à n’importe quel nombre de caractères, et comparent selon les règles de casse de l’hôte, comme le fait DEVICE (6.12). Pour sélectionner par lettre d’élément, écrivez `name="R*"`. Les noms d’instances Spectre ne commencent pas par une lettre d’élément ; sélectionnez-les par `model`, tel que `model="resistor"` (6.12).
- **`model`** compare le nom écrit en position de modèle sur la ligne d’instance : pour M, Q, D et autres éléments, le nom `.model` ; pour les instances X, le nom de sous-circuit, y compris les composants de PDK que la netlist ne définit pas, tel que `# @SELECT-INSTANCE(model="sky130_fd_pr__nfet_01v8")` ; dans Spectre, le master. C’est la même chose que `model=` de DEVICE (6.12). `type` compare le type de l’instance : le type de son `.model`, ou, pour une instance à laquelle son hôte n’en donne aucun, comme une instance X, le type que donne un DEVICE (6.12) ; sans l’un ou l’autre, l’instance n’a aucun type et ne correspond à aucun `type=`. `type` prend le joker `*` comme le font les conditions qui comparent des noms : `type="*mos"` correspond aux types nmos et pmos, et `type="*"` sélectionne les instances qui ont un type, quel qu’il soit, ce qui permet à une règle ou à une assertion d’exclure les composants dont le type n’est pas connu.
- **`terminals`** compare le nombre de bornes de l’instance, un entier : les nœuds que l’hôte lit sur sa ligne, qui portent les numéros 1, 2, … (6.12), quel que soit le nombre de noms que DEVICE leur donne. Ainsi `terminals=3` sélectionne `Q1 c b e qn` et non `Q2 c b e s qn`. Pour une instance X, il compte les nœuds de sa ligne, c’est-à-dire les ports du sous-circuit ; dans Spectre, les nœuds entre parenthèses, donc `S1 (com t1 t2) switch` en a 3. Il se combine avec les autres conditions comme elles, tel que `# @SELECT-INSTANCE(type="nmos", terminals=4) => @SET-SYMBOL $NMOS_4TERM` ; un faisceau est sélectionné d’après son premier membre (11.8).
- La comparaison suit 3.6 : les noms de conditions et valeurs d’énumération standards sont sensibles à la casse ; les valeurs qui nomment des choses de l’hôte (noms, noms de modèles, noms de nets) suivent les règles de l’hôte.
- Un net peut être sélectionné par nom : `# @SELECT-NET(name="0")` le sélectionne, et `# @SELECT-TERMINAL(net="0")` les bornes qui s’y trouvent. Les objets peuvent aussi être sélectionnés par sens : `# @SELECT-USAGE $DIFFERENTIAL_PAIR_N => @SELECT-INSTANCE(role="left")` sélectionne le transistor gauche de chaque paire différentielle.
- Il n’existe pas de mot-clé pour une intersection, telle que les membres d’un ensemble appartenant à une utilisation ; écrivez-la avec REMOVE-FROM deux fois (5.2.2) :

  ```rnl
  %s_set = {#M1 #M3 #M5};
  %s_set @SELECT-ANCESTOR-USAGE $DIFFERENTIAL_PAIR_N => @SELECT-INSTANCE => %dp_m_set;
  %out_set = {%s_set};
  %dp_m_set @REMOVE-FROM %out_set;    /* the members of %s_set in no differential pair */
  %in_set = {%s_set};
  %out_set @REMOVE-FROM %in_set       /* the members of %s_set in a differential pair */
  ```

- **Nets de masse et d’alimentation** : ils sont donnés par des ensembles que fournit le paquet standard, `$GROUND_NET_SET` (net 0 par défaut) et `$SUPPLY_NET_SET` (vide par défaut). Il n’y a pas de classes de nets.
  - Un paquet ne peut pas contenir de références hôtes (16.2), donc le contenu par défaut d’un tel ensemble ne peut s’écrire que comme une sélection, telle que `# @SELECT-NET(name="0", recursive=true)`.
  - L’utilisateur ajoute et retire des nets avec ADD-TO et REMOVE-FROM :

    ```rnl
    &vss @ADD-TO $GROUND_NET_SET;                                       /* the vss of this level */
    # @SELECT-NET(name="vss", recursive=true) => @ADD-TO $GROUND_NET_SET;   /* the nets named vss of every level */
    # @SELECT-NET(name="vdd!", global=true, recursive=true) => @ADD-TO $SUPPLY_NET_SET  /* only the global vdd! */
    ```

    La deuxième instruction ajoute aussi les nets locaux nommés vss dans les sous-circuits. Pour exclure un net à un seul endroit, copiez l’ensemble et retirez le net de la copie : `%g_set = {$GROUND_NET_SET}; &vss2 @REMOVE-FROM %g_set`.
  - Une règle de complétion utilise un tel ensemble comme portée, comme `$GROUND_NET_SET @SELECT-PIN(wired=false, recursive=true) => $GROUND_STUB @ATTACH`, qui prend aussi les broches que les composants virtuels ont sur ces nets ; le paquet standard retire ensuite les broches des composants qu’il place lui-même, telles que celles des masses.
  - Écrire `$GROUND_NET_SET = …` dans son propre niveau masque l’ensemble du paquet, et les règles du paquet ne voient pas le nouveau (6.8).
  - Les masses et alimentations ne sont pas devinées d’après les noms de nets. Un importateur écrit le domaine d’alimentation d’un net dans un fichier d’éditeur (tel que `powerDomain`) sous forme d’ADD-TO.
  - Un solveur peut lire les ensembles standards comme des indications (telles que les alimentations en haut et les masses en bas), comme il peut lire les définitions du paquet standard ; ce n’est pas exigé.
  - Note : il n’y a pas de mot-clé pour les nets de masse et d’alimentation (1.5 point 7) : les emplois de ces ensembles, tels que sélectionner des bornes pour ajouter des masses ou changer des symboles, peuvent tous s’écrire sous forme de règles dans un paquet.
- **Le nœud vide** (6.3) n’est pas un net : SELECT-NET ne le donne jamais, une broche sur lui est sélectionnée par SELECT-PIN uniquement par son objet ou `#`, non par un net ou `net=`, et un fil sur lui par SELECT-WIRE sans `net=`.
- **Nets globaux.** La condition `global=true` de SELECT-NET ne sélectionne que les nets globaux (6.3). Des nets locaux portant le même nom dans différents niveaux sont des nœuds différents, et sélectionner par nom les sélectionne tous ; pour ne prendre que le net global, ajoutez cette condition.

### 10.2 La condition `wired`

- Une broche est `wired=false` lorsqu’elle ne fait pas partie des points d’un fil explicite (une extrémité ou un point intermédiaire ; un fil écrit vers une borne aboutit sur la broche de la borne, 5.2.4), et qu’elle ne figure dans aucun arbre de ROUTE-NET ou ROUTE-OBJECT (13.7), puisque le solveur dessine les fils d’un arbre. Une broche de bus est une seule broche, câblée lorsqu’un fil aboutit sur elle par l’une quelconque de ses bornes (9.2.4). Les fils de bus sont aussi des fils explicites (13.1). Un fil vers une borne d’un faisceau connecte cette borne de chaque membre (11.8).
- Un net est `wired=false` lorsqu’aucune borne d’une instance hôte sur lui n’est touchée par un fil explicite et qu’aucun arbre de ROUTE-NET ou ROUTE-OBJECT ne se trouve sur lui ; les broches des composants virtuels ne comptent pas.
- Quelles broches sont dans des arbres est fixé en deux temps de 8.2. Après l’étape 4, une fois les symboles attribués, à partir des instructions de l’étape 3 : les règles de ROUTE-NET, ROUTE-OBJECT, UNROUTE-NET et UNROUTE-OBJECT, qui sont évaluées à l’étape 5 (10.4), lisent ces arbres, comme `{&a &b} @SELECT-NET(wired=false) => @ROUTE-NET`, et ne se voient pas mutuellement, si bien qu’une règle ne lit jamais les arbres qu’elle crée elle-même. Après l’étape 5, avec les arbres que créent et modifient ces règles : les conditions de l’étape 6 et des étapes suivantes lisent ceux-ci. Ainsi une règle de génération, telle que la complétion du paquet standard (16.3), compte les broches que relie une règle ROUTE comme câblées, et les broches qu’une règle UNROUTE retire comme non câblées, sauf si un fil écrit les touche. Les instructions de routage dans du contenu généré à l’étape 6, tel que le corps d’une utilisation générée, créent et modifient aussi des arbres, mais aucune condition ne les voit.
- Un fil écrit sur le nœud vide (13.1) compte comme tout autre : les broches sur lesquelles il aboutit sont câblées.
- Seuls les fils écrits explicitement comptent, non ceux qui sont générés. La présence de composants virtuels ne fait aucune différence.
- Les fils comptent tels qu’ils sont écrits, à partir de l’étape 3 de 8.2, y compris un fil qui échoue plus tard parce qu’une borne n’a pas de broche (7.1, 9.2.4). Pour un fil réécrit, la dernière réécriture compte (13.1).

### 10.3 Évaluation des sélections

- Une sélection n’est pas développée là où elle est écrite, et chaque évaluation n’examine qu’un seul niveau. Une règle (10.4) est évaluée une fois dans le niveau où elle est écrite et une fois dans chaque niveau inférieur qu’elle atteint (10.1) ; dans chaque évaluation, `#` est ce niveau, et la sélection ne sélectionne que des objets de ce niveau. Ainsi `# @SELECT-INSTANCE(model="nch", recursive=true) => @ALIGN("y")` écrit au niveau supérieur aligne une fois dans chaque dessin, et n’aligne pas entre elles des instances de dessins différents.
- Une sélection est évaluée dans la phase qui l’utilise : règles d’apparence dans la file des symboles et la file des styles, règles des mots-clés de routage à l’étape 5, règles de génération dans la phase de génération (8.2). Il en va de même pour les ensembles (5.2).
- La sorte que sélectionne une sélection est connue lors de sa lecture. La donner à un emplacement qui ne prend pas cette sorte est une erreur immédiate, y compris à l’intérieur d’un ensemble, tel que `# @SELECT-TERMINAL(net="vss") => @SET-SYMBOL $NMOS`.
- SELECT-USAGE et SELECT-ANCESTOR-USAGE sélectionnent toujours des utilisations ; lorsqu’une sélection passe des utilisations à leurs membres, la sorte sélectionnée est donnée par son mot-clé. La sorte de chaque sélection est donc connue lors de sa lecture.
- Le résultat d’une sélection est un ensemble et peut être nommé avec `%` ou `$`. Un ensemble évalué dans un niveau ne prend que les membres de ce niveau : `&vss @ADD-TO $GROUND_NET_SET` écrit au niveau supérieur n’ajoute que le vss du niveau supérieur, qui ne figure pas dans l’ensemble lorsqu’il est évalué dans un sous-circuit (6.6).
- **L’ordre du résultat.** Les membres du résultat d’une sélection sont dans cet ordre, qui importe partout où l’ordre d’un ensemble compte, comme pour ORDER, DISTRIBUTE, TABLE et LINE, ainsi que pour les résultats d’une règle appliquée un par un (5.2.1) :
  - Avec `#` comme portée, les objets du niveau dans l’ordre de la scène :
    - instances dans l’ordre de leurs lignes dans la netlist, un faisceau à la ligne de son premier membre ; bornes par instance ou faisceau, puis par numéro ;
    - nets : les ports du niveau dans l’ordre de la ligne de sous-circuit, puis les autres nets dans l’ordre où ils apparaissent pour la première fois sur les lignes d’instance ;
    - broches selon l’objet auquel elles appartiennent, d’abord les instances et faisceaux dans l’ordre des lignes (un faisceau à la ligne de son premier membre), puis les utilisations dans leur ordre ; au sein d’un même objet, dans l’ordre des instructions PIN de son symbole ;
    - utilisations, fils, textes et graphiques : d’abord ceux créés à l’étape 3 de 8.2, dans l’ordre de leurs positions (8.5), ce que crée le corps d’une utilisation venant juste après cette utilisation, dans l’ordre du corps ; puis ceux créés aux étapes suivantes, étape par étape. Au sein d’une même étape, ils sont dans l’ordre des positions des instructions ou règles qui les créent, les résultats d’une règle dans l’ordre de son entrée (5.2.1), et les textes apportés par les symboles selon l’objet auquel ils appartiennent, les objets étant pris dans l’ordre donné pour les broches, puis dans l’ordre du symbole. Un texte de la scène qui surcharge un texte du symbole par clé (13.3) constitue un texte unique, à la position de la première instruction qui l’écrit.
  - Avec toute autre portée, chaque membre de la portée donne ses résultats tour à tour, chacun dans l’ordre ci-dessus, et un objet déjà donné n’est pas redonné. Cela vaut pour tout mot-clé de sélection, SELECT-ANCESTOR-USAGE compris.
  - Les ensembles gardent l’ordre dans lequel leurs membres sont écrits et ajoutés (5.2, 5.2.2) : ADD-TO ajoute en fin et garde les répétitions, et UNION les supprime.
- Lorsque la portée d’une sélection est écrite comme des objets hôtes de son niveau, telle que `&Q<0:3:2, 1> @SELECT-TERMINAL(name="R*", recursive=true) => @TEXT("${net}", key="netname")` au niveau supérieur, ces objets n’existent pas lorsque la règle est évaluée dans un niveau inférieur : la sélection y est vide et ne fait rien, sans avertissement, tout comme un ensemble ne prend que les membres du niveau. L’avertissement de 10.5 n’est émis que pour les autres rôles des règles de génération.

### 10.4 Règles

- Une instruction qui prend le résultat d’une sélection, directement ou via un ensemble qui dépend de l’une d’elles, est une règle. Un ensemble dépend d’une sélection lorsqu’il est lié à l’une d’elles, lorsqu’il en contient une ou un tel ensemble parmi ses éléments, lorsqu’il est créé par UNION de tels ensembles, ou lorsque ADD-TO ou REMOVE-FROM le modifie avec le résultat d’une sélection ou avec un tel ensemble (5.2.2) : ses membres ne sont alors connus que dans la phase qui l’utilise. Dans le corps d’une utilisation, un rôle d’ensemble dépend d’une sélection lorsque ce qui le remplit en dépend.
- Tout mot-clé sauf BUNDLE (11.8), WIRE (13.1), et RECTANGLE, CIRCLE et ELLIPSE écrits avec deux points (9.2.5), peut prendre le résultat d’une sélection, un par un ou en bloc comme l’indique 5.2.1 : `# @SELECT-INSTANCE(type="nmos") => @ORIENT("MY")`, `# @SELECT-INSTANCE(model="nch") => @ALIGN("y")`.
- SET-SYMBOL et SET-STYLE prenant le résultat d’une sélection comme objet sont des règles d’apparence et entrent dans les files d’apparence (8.3). AS et ATTACH sont des règles de génération (10.5), et il en va de même de TEXT attaché au résultat d’une sélection (13.3), de RECTANGLE, CIRCLE et ELLIPSE écrits avec une taille dont le point est ce résultat (9.2.5), et des règles de LINE, ARC et BEZIER, qui créent des objets dessinés ; leur emplacement prend un ensemble, donc une telle règle dessine une seule figure à travers tout le résultat, dans son ordre (5.2.1, 10.3). Les règles de ROUTE-NET, ROUTE-OBJECT, UNROUTE-NET et UNROUTE-OBJECT sont évaluées avant elles, à l’étape 5 : leurs conditions lisent les arbres des instructions de l’étape 3, et les règles de génération voient les arbres qu’elles créent (10.2). Les règles de CURRENT sont évaluées avec les règles de génération, à l’étape 6 ; ensemble elles forment les règles de l’étape 6 : les sens du courant sont fixés à partir des instructions de l’étape 3, et les conditions de l’étape 6 ne voient pas ce que modifient ces règles (15.2). BUNDLE ne prend pas le résultat d’une sélection : un faisceau doit exister avant que les symboles soient attribués (11.8). Les règles d’autres mots-clés, tels que les contraintes, PLACE et ORIENT, prennent effet à l’étape de leur mot-clé et voient les objets générés. Avec tout autre mot-clé, une règle équivaut à une instruction écrite pour chaque objet sélectionné, ou à une seule écrite pour eux tous, et prend effet dans la phase de son mot-clé.
- Une règle s’applique au niveau où elle est écrite, et aux niveaux inférieurs qu’elle atteint (10.1), par `recursive=true` dans une sélection qu’elle écrit ou, si elle n’en comporte pas, par les ensembles qu’elle utilise, évaluée une fois dans chaque niveau et n’examinant que ce niveau (10.3). Une règle dans un paquet compte comme écrite dans le niveau de l’USE-PACKAGE du paquet, et un paquet à la racine comme utilisé dans le niveau supérieur de son fichier (16.2), si bien qu’elle n’atteint les niveaux inférieurs que comme le dit 10.1, ce qui explique pourquoi le paquet standard écrit `recursive=true` dans ses sélections. Une règle dans le corps d’une définition générale compte comme écrite dans le niveau de l’utilisation (9.4).
- **Objets hôtes d’un niveau dans une règle qui atteint les niveaux inférieurs** : une règle qui atteint les niveaux inférieurs et dont les autres emplacements nomment des objets hôtes de son niveau, telle que `%clk_set %rest_set @DISTANCE("bottom-to-top", min=3)` avec `%clk_set = {#VP1 #VP2}` et `%rest_set` lié à une sélection avec `recursive=true`, ne trouve aucun de ces objets dans un niveau inférieur : l’instruction y est sans effet, et aucun avertissement n’est émis. Seules les règles de génération avertissent, comme l’indique 10.5.

### 10.5 Génération un par un

« Pour chaque borne sur vss sans fil, placer une masse et la relier par un fil » :

```rnl
$GROUND_STUB = @DEFINE(roles=[["t", "pin"]], parameters=[["gap", 2]]) {
  %g = $GROUND @ATTACH ?t::net;
  ?t::net @WIRE {?t %g::pin};
  {?t %g::pin} @ALIGN("x");
  %g::pin ?t @DISTANCE("bottom-to-top", exact=?gap)
};
# @SELECT-INSTANCE => @SELECT-PIN(net="vss", wired=false) => $GROUND_STUB @ATTACH => %stubs_set
```

Le paquet standard fournit un `$GROUND_STUB` semblable à celui-ci, à ceci près que son écart vaut 1 par défaut et qu’il ne donne que la distance minimale (`min=?gap`), afin qu’un solveur puisse dessiner le fil plus long, par exemple lorsqu’il aligne les masses de composants de longueurs différentes, qu’il oriente la masse vers le bas (`@ORIENT("R0")`), et que ses contraintes sont de priorité `"low"` ; la définition ici le masque (6.8).

- Lorsqu’un rôle simple d’AS ou ATTACH reçoit le résultat d’une sélection, la définition est appliquée une fois pour chaque objet sélectionné, et les autres rôles sont identiques dans chaque application (5.2.1, 9.4). Un rôle d’ensemble prend le résultat d’une sélection en bloc, et la définition est appliquée une fois ; quand tous ses rôles sont des rôles d’ensemble, seulement dans les niveaux où l’un d’eux contient quelque chose (9.4).
- Les composants virtuels sont créés un par un de la même façon : `# @SELECT-NET(port=true) => $IO_PORT @ATTACH` donne à chaque net de port une étiquette de port ; avec `$RLOAD`, une définition à deux rôles dessinée comme une résistance, `# @SELECT-NET(name="out*", recursive=true) => $RLOAD @ATTACH &vss` en attache une entre chaque net sélectionné et vss ; puisque `&vss` appartient à ce niveau, la règle ne prend effet qu’ici, avec l’avertissement décrit ci-dessous.
- La complétion du paquet standard ajoute des masses et des alimentations avec un fil (`$GROUND_STUB`, `$SUPPLY_STUB`) : une fois celles-ci réécrites, le fil fait que la broche n’est plus `wired=false`, et la complétion la saute. Une masse placée sur une broche par des contraintes seules, sans fil, laisse la broche `wired=false`, de sorte que la complétion en ajoute tout de même une là ; un fil de longueur 0 entre les deux broches (13.1) rend la broche câblée.
- L’appariement se fait au sein de chaque application : les noms locaux d’une définition sont nouveaux dans chaque application, donc le `%g` d’une application est la masse créée dans cette application.
- Le résultat est un ensemble généré. Il peut être nommé et donné en bloc à SET-SYMBOL ou SET-STYLE (tel que `%stubs_set @SET-STYLE $grey`), mais aucun de ses membres pris isolément ne peut être référencé (5.7).
- **En écrire un à la main.** Pour en décrire un isolément, créez-le à la main et câblez-le. Sa broche n’est alors plus `wired=false`, et la complétion la saute. Il n’y a pas de moyen séparé d’annuler une génération.
- **Instantané.** Toutes les règles de génération voient le même instantané, le résultat des étapes 3 à 5 de 8.2 (après surcharge par clé, avec les symboles de tous les objets et les arbres de l’étape 5), et ne voient pas les résultats les unes des autres. La même règle écrite deux fois génère donc deux exemplaires.
- Les faits générés prennent la position de leur règle de génération dans la résolution des conflits (8.5).
- Les objets générés sans position sont placés par le solveur ; pour indiquer comment ils sont placés, utilisez une définition avec contraintes.
- **Réécriture.** Un objet généré isolé ne peut pas être référencé (5.7). Lorsque le solveur les réécrit, il les écrit sous forme d’instructions explicites, telles que `$IO_PORT @ATTACH &out => @PLACE(40, 20) => %p_out` avec le fil qui y mène. L’objet explicite ne prend la place de l’objet généré que lorsque la règle exclut ce qui est déjà écrit, comme le font les complétions du paquet standard en ne sélectionnant que les broches sans fils (10.1) et comme le fait une complétion par net en écrivant la différence de deux ensembles (ci-dessous) ; une règle écrite autrement crée à nouveau l’objet à côté de l’objet explicite, sans position. Une règle dont le solveur place les objets devrait donc exclure ce qui est déjà écrit, de l’une de ces deux manières. Les objets dans le corps d’une utilisation explicite écrite de cette façon qui n’ont pas de nom local sont désignés par numéro (5.9). L’utilisation est écrite avec la priorité que la règle écrit sur son application, telle qu’écrite, ou avec `"disabled"` lorsqu’elle la tient de l’en-tête de l’enregistrement de la règle (14.8) ; sinon elle est écrite sans priorité, afin que son corps garde les priorités qu’il avait, et un en-tête d’enregistrement portant une autre priorité n’est pas écrit.
- La génération ne récurse pas : lorsque le contenu développé par une règle de génération contient des règles de l’étape 5 ou 6 (10.4), celles-ci échouent, puisque chacune de ces étapes s’exécute une seule fois. Ses autres règles prennent effet aux étapes suivantes normalement : SET-SYMBOL dans la file des symboles de l’étape 7 ; PLACE, ORIENT, LAYER, PRIORITY et les contraintes à l’étape 8, de sorte que le corps d’une utilisation générée puisse placer et orienter ce que reçoivent ses rôles ; SET-STYLE dans la file des styles ; les assertions lorsqu’elles sont vérifiées (8.2, 15.3).
- Lorsqu’un autre rôle d’une règle de génération qui atteint les niveaux inférieurs (10.1) est rempli par un objet du niveau (un objet de la scène ou un objet hôte), la règle ne prend effet que dans le niveau où elle est écrite, avec un avertissement : ces objets appartiennent à ce niveau, et les niveaux inférieurs ne les possèdent pas. Un rôle recevant une définition n’a pas une telle limite.
- Un SET-SYMBOL ou SET-STYLE créé par une génération appartient au groupe de la règle qui l’a créé, à la position de la règle (8.3).
- Un nom lié à une sélection ou à un ensemble généré ne peut pas être lié à nouveau (6.6).
- **Complétion par net.** Il n’existe pas de règle intégrée « n’en ajouter aucun là où il en existe un ». Une complétion écrit la différence de deux ensembles ; un paquet qui ajoute des étiquettes de port écrit :

  ```rnl
  $PORT_FILL_SET = # @SELECT-NET(port=true, recursive=true);                                /* the nets that get a port tag */
  # @SELECT-USAGE(recursive=true) $IO_PORT => @SELECT-NET => @REMOVE-FROM $PORT_FILL_SET;   /* less those that have one */
  $IO_PORT @ATTACH $PORT_FILL_SET
  ```

  - La complétion de la série standard 0.4 s’écrit de cette façon, avec l’ensemble dans `rnl-std-basic`, et elle place aussi les étiquettes de chaque niveau en une colonne (12.4). Elle laisse les nets d’alimentation et de masse dans l’ensemble ; un fichier qui ne veut aucune étiquette là où se trouvent ses raccords écrit `$SUPPLY_NET_SET @REMOVE-FROM $PORT_FILL_SET; $GROUND_NET_SET @REMOVE-FROM $PORT_FILL_SET`.
  - Une étiquette de port écrite à la main prend la place de celle qui est générée : après `$IO_PORT @ATTACH &inp => @PLACE(0, 20) => %p`, inp ne reçoit aucune étiquette de port générée. L’utilisateur peut aussi retirer des nets de `$PORT_FILL_SET`, comme `# @SELECT-NET(name="vdd") => @REMOVE-FROM $PORT_FILL_SET`, ou compter les utilisations de sa propre définition de port : `# @SELECT-USAGE $MY_PORT => @SELECT-NET => @REMOVE-FROM $PORT_FILL_SET`.
  - Les étiquettes de port générées ne comptent pas : les règles de génération utilisent `$PORT_FILL_SET` à l’étape 6, et les sélections dans la différence sont évaluées sur l’état avant l’étape 6 (5.2).
  - Note : « il en existe déjà un » n’est pas intégré, car c’est ambigu pour une définition à plusieurs rôles, les propres définitions de port de l’utilisateur ne compteraient pas, et une définition de placement répétée ne fait que répéter des contraintes, que traitent la résolution des conflits et les priorités (8.5, 14.8).
- Dans une complétion par borne, c’est toujours le fil d’un objet écrit à la main qui prend la place de l’objet généré : une broche avec un fil n’est plus `wired=false`. Il n’y a toujours aucun moyen d’annuler une génération : retirer un net de l’ensemble ne modifie que l’ensemble.

<!-- translated from en/ch11.md sha256:693deb0f50aa -->
## 11 Placement et apparence

### 11.1 Position : PLACE

```rnl
#M1 @PLACE(10, 20);
#M1 @PLACE(x=10)
```

- PLACE donne une position à un objet ; au moins l’un de `x` et `y` doit être écrit. PLACE ne fixe que les coordonnées écrites. Chaque coordonnée est libre au départ, et une coordonnée jamais écrite reste libre, pour être décidée par les contraintes et le solveur.
- PLACE ne fixe que les coordonnées, et non la rotation, la symétrie miroir, l’échelle ni les directions des bornes ; l’orientation est fixée avec ORIENT (11.2).
- Un PLACE constitue deux faits géométriques, un pour x et un pour y, et en cas de conflit chacun peut être écarté séparément (8.5). Le paramètre `priority=` donne sa priorité (14.8), tel que `#M1 @PLACE(12, 20, priority="high")` : x et y donnés par position viennent en premier, les paramètres par nom après eux (4.2 point 9).
- Objets : les instances, les utilisations (par leur point propre, 14.1), les faisceaux (11.8), les points, les TEXT (annotations et étiquettes ; PLACE fixe la position du texte lui-même, 13.3), ainsi que l’origine `::position` (y compris le centre d’un RECTANGLE, CIRCLE ou ELLIPSE écrit avec un centre). PLACE ne prend pas d’autres graphiques ; pour les déplacer, déplacez leurs points (13.4). Dans le corps d’une définition générale, ses coordonnées sont des décalages par rapport au point propre de l’utilisation (9.4). Dans un corps de symbole, PLACE ne positionne que les TEXT attachés à `?`, en coordonnées locales (9.2.2).
- La sortie conserve la poignée de l’entrée (5.1).
- Un ensemble donné comme objet est placé membre par membre (5.2.1) : `%pair_set @PLACE(10, 20)` place les deux membres en (10, 20). Un ensemble n’a pas de position propre (14.1).
- Pour les axes, l’unité, les décimales et la tolérance des coordonnées, voir 11.7.

### 11.2 Orientation : ORIENT

```rnl
%pair_set = {#M3 #M4};
#M2 @ORIENT("MY");          /* flipped left-right */
%pair_set @ORIENT("R90");   /* each member turns about its own origin and stays where it is */
#M5 @ORIENT(["R0", "MY"])   /* not turned, or flipped left-right: the solver chooses */
#M6::g @ORIENT("left")      /* M6 is oriented so that its gate faces left */
```

- Objets : les instances, les utilisations pourvues d’un symbole et les faisceaux ; ainsi que les broches (ci-dessous). ORIENT sur une utilisation sans symbole échoue (12.1) ; les points, étiquettes, graphiques, fils et nets ne sont pas acceptés.
- Un ensemble donné comme objet est orienté membre par membre (5.2.1) : chaque membre tourne autour de sa propre origine et reste là où il se trouve, et chaque membre constitue une description. L’ensemble n’est pas tourné d’un bloc. Tourner un ensemble entier d’un bloc n’est pas prévu : un ensemble n’a pas de position (14.1).
- Une utilisation pourvue d’un symbole tourne autour de son point propre ; elle possède sa propre orientation et ne tourne pas avec les objets qui remplissent ses rôles (12.1). Ce que le corps de sa définition dessine ou place ne tourne pas non plus avec elle (9.4).
- **Une liste de codes**, telle que `["R0", "MY"]`, forme un seul fait, qui est vérifié lorsque l’objet a l’une des orientations listées. Il est retenu ou écarté en bloc (8.5), et le solveur choisit parmi les codes. La liste comporte au moins un code ; une liste d’un seul code équivaut à ce code. Le solveur réécrit l’orientation qu’il a choisie sous forme d’un code unique dans son enregistrement (14.8) ; la liste dans l’enregistrement de l’auteur demeure, et la résolution suivante choisit à nouveau. Un moteur de rendu qui ne résout pas, et qui n’a aucun résultat pour l’objet, dessine le premier code de la liste.
- Sans ORIENT, il n’y a aucune contrainte et le solveur choisit ; un moteur de rendu qui ne résout pas dessine `"R0"` (8.5). Les relations entre les orientations de plusieurs objets (même orientation, images en miroir) s’écrivent avec ORIENT-RELATION (14.9).
- ORIENT est un fait géométrique unique et prend part à la résolution des conflits (8.5) ; le paramètre `priority=` donne sa priorité (14.8).
- **Codes d’orientation.** Chaque code désigne une orientation. Seules les huit graphies du tableau, ainsi que les quatre mots ci-dessous, sont acceptées, et elles sont sensibles à la casse (3.6). Les noms sont ceux de Virtuoso et, avec y vers le haut (11.7), leurs significations le sont également. Les graphies équivalentes (telles que `"MXR180"`, qui est MY) et les angles arbitraires ne sont pas acceptés, seuls ou dans une liste : ce sont de mauvaises valeurs de paramètre, et l’instruction ORIENT échoue. Les outils ne réécrivent que ces huit codes, un à la fois. Il n’existe pas de forme relative qui fasse tourner davantage un objet à partir de son orientation actuelle (chapitre 17).

  | Code | (x, y) devient | Signification |
  |---|---|---|
  | `"R0"` | (x, y) | Non tourné |
  | `"R90"` | (−y, x) | Tourné de 90° dans le sens antihoraire |
  | `"R180"` | (−x, −y) | Tourné de 180° |
  | `"R270"` | (y, −x) | Tourné de 270° dans le sens antihoraire, soit 90° dans le sens horaire |
  | `"MY"` | (−x, y) | En miroir par rapport à l’axe y : retourné gauche-droite |
  | `"MX"` | (x, −y) | En miroir par rapport à l’axe x : retourné haut-bas |
  | `"MYR90"` | (−y, −x) | MY, puis R90 |
  | `"MXR90"` | (y, x) | MX, puis R90 |

- **Mots pour les quatre rotations.** Une rotation peut aussi s’écrire comme une direction, qui désigne le même code : `"left"` est `"R0"`, `"down"` est `"R90"`, `"right"` est `"R180"` et `"up"` est `"R270"`, dans les listes également. ORIENT-RELATION ne prend que les codes (14.9). Pour un objet, le mot n’est qu’une autre graphie : `#M2 @ORIENT("left")` équivaut à `#M2 @ORIENT("R0")`. Pour une broche, il se lit littéralement : la direction vers laquelle pointe la broche, mesurée à partir de la gauche.
- **Broches.** ORIENT donné à une broche, ou à une borne qui la représente (5.2.4), contraint la direction de la broche dans le dessin : la direction écrite avec `direction=` dans le symbole (9.2.4), tournée par l’orientation de son objet, l’une des quatre. Le fait est vérifié lorsque l’objet a l’une des orientations qui tournent la broche dans cette direction, deux des huit, si bien qu’il ne fixe pas l’orientation de l’objet ; une liste autorise chacune des directions listées. Un code en miroir donné à une broche désigne la direction vers laquelle il tourne la gauche : `"MY"` est `"right"`, `"MX"` est `"left"`. Un tel ORIENT est un fait géométrique à part entière, ayant la broche pour clé, en plus de tout ORIENT de l’objet ; les deux sont résolus comme deux faits quelconques (8.5). Une broche sans `direction=` écrit n’a rien à contraindre : l’ORIENT de cette broche échoue, et les autres membres de la même instruction prennent tout de même effet (5.2.1). Un moteur de rendu qui ne résout pas, et qui n’a aucun résultat pour l’objet, le dessine comme si l’ORIENT de ses broches n’était pas écrit.
- **Ordre de composition.** MXR90 et MYR90 appliquent d’abord la symétrie miroir puis la rotation ; les noms se lisent dans l’ordre des opérations, comme dans Cadence et dans OpenDB d’OpenROAD.
- **Comment elle agit.** L’orientation agit sur les coordonnées locales du symbole, autour de son origine, puis l’origine est placée à la position de l’objet : un point du symbole est dessiné à la position de l’objet augmentée de l’orientation appliquée à ses coordonnées locales. Les graphiques et les positions de broches du symbole sont transformés rigidement avec lui, et les directions dans lesquelles les fils quittent les broches tournent aussi (9.2.4). Les textes sont affichés comme l’indique 9.2.5 ; les glyphes ne sont pas mis en miroir.
- **Uniquement là où les éléments sont dessinés.** Une orientation ne change que l’endroit où les broches sont dessinées, et non les bornes auxquelles elles appartiennent. Lorsqu’un symbole de bloc est retourné haut-bas, la broche de inp est dessinée en bas, et le fil vers inp la suit. Les deux broches ne sont pas interverties pour garder inp en haut : cela changerait quel fil se connecte à quelle borne, ce qui relève de la netlist (Principes 2.2). Pour dessiner inp ailleurs, utilisez un autre symbole (11.3).

### 11.3 Symboles et styles : SET-SYMBOL et SET-STYLE

```rnl
%w13 = &x @WIRE {#M1:1 #M3:1};
#M1 @SET-SYMBOL $NMOS;
# @SELECT-INSTANCE(model="nch") => @SET-SYMBOL $NMOS;
%w13 @SET-STYLE $signal;
# @SELECT-INSTANCE => @SET-STYLE $base
```

- SET-SYMBOL attribue un symbole à un objet. L’objet peut être une instance, une utilisation ou un faisceau, ou un ensemble de ceux-ci (y compris les résultats de sélections et de génération un par un) ; chaque membre reçoit le symbole une fois (5.2.1). Dans un sous-circuit, `# @SET-SYMBOL` attribue le symbole de bloc (9.2.6).
- Une utilisation qui reçoit un symbole est dessinée : c’est un composant virtuel (chapitre 12).
- **Mise en correspondance des bornes et des broches.** Les noms de bornes d’un symbole sont ceux que lient ses broches (9.2.4). Lorsque SET-SYMBOL attribue un symbole à une instance, chaque nom de borne du symbole est mis en correspondance avec la borne de l’instance portant ce nom, donnée par DEVICE ou par les ports du sous-circuit (6.12) ; un nombre est mis en correspondance avec la borne à cette place. Une borne de l’instance qu’aucune broche ne lie n’a pas de broche, comme un substrat dans un symbole à trois bornes. Un nom de borne du symbole que l’instance ne possède pas laisse sa broche liée à rien, avec un avertissement.
  - `terminals=` modifie la mise en correspondance pour ce SET-SYMBOL. Sa forme standard énumère, dans l’ordre des nœuds de la ligne d’instance (comme le paramètre `terminals` de DEVICE, 6.12), les noms de bornes du symbole : avec un symbole dont les broches lient `"a"`, `"b"` et `"c"`, `#X1 @SET-SYMBOL(terminals=["b", "a", "c"]) $GATE` lie le nœud 1 à la broche de b, le nœud 2 à celle de a et le nœud 3 à celle de c. Il peut aussi énumérer des paires, la borne de l’instance en premier et celle du symbole en second : `terminals=[["d", "b"], ["g", "a"], ["s", "c"]]` ; les bornes non énumérées dans les paires sont mises en correspondance par nom. Une liste qui n’est ni l’une ni l’autre, ou qui nomme deux fois une même borne, est un mauvais paramètre, et le SET-SYMBOL échoue.
  - Pour une utilisation, les noms de bornes du symbole sont mis en correspondance avec les noms de rôles de sa définition. Un nombre, ou un nom qu’aucun rôle ne porte, ne lie aucun rôle, et un avertissement est émis. `terminals=` énumère des noms de bornes du symbole dans l’ordre de la liste de rôles, ou des paires formées d’un nom de rôle et d’un nom de borne du symbole (12.1).
- SET-STYLE attribue un style à un objet. L’objet peut être une instance, une utilisation, un faisceau, un fil ou un graphique (textes compris), ou un ensemble de ceux-ci (y compris les résultats de sélections et de génération un par un). Il ne prend ni nets, ni niveaux, ni contraintes, ni définitions, ni points. Les bornes sont dessinées avec le symbole de leur objet et prennent le style de leur objet. Un style attribué à une utilisation est transmis à son symbole et aux objets dessinables créés dans son corps (8.3), si bien qu’un style attribué à une utilisation sémantique sans symbole n’est pas une erreur ; lorsque rien n’est dessiné dans le corps, il n’a aucun effet visible.
  - Note : sinon la règle `# @SELECT-USAGE => @SET-STYLE $base` du paquet standard produirait un avertissement pour chaque utilisation sémantique.
- La sortie conserve la poignée de l’entrée ; lorsque l’objet est un ensemble, la sortie est cet ensemble.
- Tous deux prennent effet via les files d’apparence, et non par surcharge par clé. Il y a deux paliers : un SET-SYMBOL ou SET-STYLE qui nomme directement l’objet se situe au-dessus de ceux qui atteignent l’objet via une sélection ou lui sont transmis (8.3).
- Là où des règles par défaut existent (le paquet standard, 16.3), il n’est pas nécessaire de les écrire.
- **Paramètres et styles.** Les paramètres de style écrits sur un objet, tels que `size=` de TEXT, forment le style direct de cet objet, au-dessus des styles issus de sélections ou transmis. Un SET-STYLE qui nomme directement l’objet et est écrit plus tard les surcharge selon la position (8.3) ; c’est ainsi qu’un éditeur modifie un objet isolé. Un paramètre d’une définition qui n’est pas passé compte comme non écrit, de sorte que le style prend effet (9.4).
- **Étiquettes et points de jonction.** Le style d’un objet est transmis aux textes qui lui sont attachés. Dans le palier des styles sélectionnés ou transmis, il est comparé selon la position avec les règles qui sélectionnent le texte (telles que SELECT-LABEL) ; les paramètres du texte lui-même et un SET-STYLE qui nomme directement le texte se trouvent dans le palier direct. Ainsi, lorsque `#M1 @SET-STYLE $signal` est écrit après les règles d’étiquettes, le nom de M1 prend aussi cette couleur, et déplacer le nom à la souris ne change pas sa couleur. Les textes attachés à `?` dans un symbole sont également des étiquettes, et SELECT-LABEL les sélectionne ; les textes fixes attachés à des points sont des graphiques du symbole et prennent le style de l’objet. Les textes attachés à des nets ou à des points ne reçoivent aucun style de ce à quoi ils sont attachés et n’utilisent que le leur. Un point de jonction est une utilisation de `$JUNCTION` (13.2) ; le point est son symbole, et sa couleur suit le style de l’utilisation.

### 11.4 Plusieurs dessins d’un même objet

Il n’existe aucun moyen de dessiner un même objet hôte plusieurs fois : une poignée hôte telle que `#M1` désigne toujours l’unique dessin de cet objet. Si une version ultérieure ajoutait plusieurs dessins d’un même objet, ce seraient des objets distincts dotés de leurs propres noms, et les poignées hôtes garderaient leur signification (Annexe C).

Note : plusieurs instances dessinées comme une seule forment un faisceau (11.8).

### 11.5 Groupes : GROUP

```rnl
%stage1_set = {#M1 #M2 #M5} @GROUP;
%stage2_set = {#M3 #M4} @GROUP(spacing_x=-1);
{%stage1_set %stage2_set} @GROUP;
%stage1_set %stage2_set @DISTANCE("left-to-right", min=1)
```

- GROUP ajoute un fait à un ensemble : le solveur traite ses membres comme un seul bloc. Il conserve la poignée de l’ensemble, si bien que `%stage1_set` demeure cet ensemble.
- **Un groupe n’est pas un objet.** Il n’a ni position ni cadre, et aucune contrainte ne peut être écrite sur un groupe : donné dans une contrainte, l’ensemble groupé est un ensemble, et un emplacement qui le reçoit représente chacun de ses membres (14.1). GROUP n’est pas non plus une cinquième contrainte atomique. Les structures du paquet standard qui sont des groupes comportent des groupes anonymes.
- **Membres.** Les membres d’un groupe sont les objets de son ensemble, les ensembles imbriqués étant suivis jusqu’aux objets les plus internes. Une utilisation sans symbole représente les objets qui remplissent ses rôles, et ainsi de suite jusqu’aux objets les plus bas ; une utilisation dessinée avec un symbole est elle-même un membre. ROUTE-OBJECT suit les rôles des utilisations de la même manière, et prend également les propres broches d’une utilisation dessinée avec un symbole (13.7).
- **Ce qu’il fait.** Un groupe guide le solveur de deux manières seulement : il dispose les membres d’un groupe ensemble, avant ou à l’écart du reste, et il maintient les autres objets hors de la région du groupe autant que les faits le permettent. Tous deux sont des critères du solveur, classés après toutes les descriptions du langage (8.5) : un groupe ne garantit aucune relation géométrique, et un objet que des faits forcent à entrer dans la région d’un groupe y est dessiné. Les exigences sur les positions, telles que ne pas s’entrelacer (le DISTANCE de l’exemple) ou être centré, s’écrivent comme des contraintes atomiques.
- **L’arborescence des groupes appartient au solveur.** Les groupes s’imbriquent par inclusion : les membres de `{%stage1_set %stage2_set}` sont les cinq instances, donc les deux étages se trouvent à l’intérieur. Deux groupes qui partagent des membres sans que l’un contienne l’autre ne peuvent pas tous deux former des blocs ; le solveur retient l’un d’eux selon sa propre règle, et une indication nomme l’autre. La manière dont les groupes influent sur la recherche et le classement au-delà de cela dépend de l’implémentation.
- **Espacement.** `spacing_x` et `spacing_y` sont des nombres entiers d’unités de grille (11.7) que le solveur ajoute à son espacement par défaut à l’intérieur du groupe, en x et en y ; chacun peut être omis, et une valeur négative resserre le groupe. Entre deux objets, le solveur utilise l’espacement du groupe le plus interne qui les contient tous deux ; entre des objets qui ne partagent aucun groupe, l’espacement du niveau, que `@SOLVER-CONFIG` fixe avec des clés de mêmes noms (14.10) ; entre un groupe et ce qui lui est extérieur, l’espacement du groupe englobant ou du niveau. Un solveur peut limiter la plage des valeurs et l’indique. L’espacement est un critère du solveur, non un fait : il cède devant tout fait retenu.
- GROUP est un mot-clé, non une définition du paquet standard : sa signification n’est connue que du solveur et ne peut pas être écrite dans le corps d’une définition (1.5 point 7).
- Une visionneuse peut dessiner un groupe, par exemple à l’aide d’un cadre ; cela dépend de la visionneuse, et la présente norme ne le spécifie pas.
- GROUP n’est pas une contrainte atomique : il ne prend aucune part à la résolution des conflits (8.5) et n’a pas de priorité, seulement activé ou désactivé. `%stage1_set @PRIORITY("disabled")` retire le groupe au solveur, ce qui annule le groupement. Lorsqu’une utilisation englobante est désactivée, les groupes dans son développement sont également désactivés, sauf si un PRIORITY écrit de l’extérieur via `::` directement sur un groupe le réactive (14.8). Un groupe dans le développement d’une définition ne peut être désactivé isolément que lorsqu’il s’agit d’un objet local nommé, référencé depuis l’extérieur (9.4).

### 11.6 Ordre de tracé et couches : LAYER

```rnl
@RECTANGLE(fill="fillcolor") {(0, 0) (40, 30)} => @LAYER("bottom") => %f1;    /* a frame, drawn below everything */
(2, 28) @TEXT("input stage") => @LAYER("top") => %n1
```

- Il existe trois couches : `"bottom"`, `"middle"` et `"top"`. La couche bottom est dessinée en premier, puis middle, puis top. Au sein d’une même couche, les objets sont dessinés par position : un objet ultérieur est dessiné par-dessus un objet antérieur et le recouvre.
- `@LAYER(…)` fixe la couche d’un objet ; sans lui, l’objet se trouve sur `"middle"`. Il existe une clé pour chaque objet, et le LAYER ultérieur l’emporte (5.8). LAYER ne modifie pas la position de l’objet au sein de la couche.
- **La position** est l’ordre d’écriture (1.5 point 2), le même ordre qu’en 8.3 et 8.5 : au sein d’un niveau, l’ordre du texte ; le contenu d’un paquet à la position de son USE-PACKAGE, avec les paquets à la racine en premier (16.3) ; les instructions écrites avec `scope=` après le texte propre du sous-circuit cible. Chaque sorte d’objet prend la position de :
  - fils, graphiques et textes (annotations et étiquettes) : l’instruction qui le crée ;
  - utilisations : le SET-SYMBOL qui prend effet, ou, pour un symbole attribué par une règle, la règle (8.3) ;
  - objets créés par des règles, par le développement de définitions et par génération un par un : l’instruction qui les crée, et au sein d’un même développement l’ordre dans le corps ;
  - instances : tout en bas de leur couche, dans l’ordre de la netlist entre elles ; une instance placée sur une autre couche se trouve en bas de cette couche. Un faisceau compte comme une instance, avec l’ordre de la netlist de son premier membre.
- Les instructions qui modifient un objet, telles que PLACE, ORIENT, SET-STYLE et LAYER, ne déplacent pas sa position.
- Objets : les instances, les utilisations pourvues d’un symbole, les faisceaux, les fils, les graphiques et les étiquettes ; un ensemble membre par membre (5.2.1). LAYER sur une utilisation sans symbole n’est pas une erreur mais reste sans effet (comme pour SET-STYLE, 11.3) ; sur des points, des nets, des contraintes et des définitions, il échoue.
- Les membres d’un symbole n’ont pas de couches : ils sont dessinés dans l’ordre des membres (9.2.1), en même temps que leur objet.
- Il n’existe pas de `z=` numérique. La surcharge (par la clé d’un fait) et le recouvrement (par la couche et la position) sont distincts.
- « Couche » désigne ici l’ordre de tracé ; cela n’a rien à voir avec les niveaux de l’hôte (2.6).

### 11.7 Coordonnées et unités

- **Axes.** x va vers la droite et y va vers le haut ; un plus grand y est plus haut. Ainsi `"bottom-to-top"` de ORDER et DISTANCE va vers le haut (14.3, 14.5), et R90 est dans le sens antihoraire (11.2). SVG, d’autres formats et certains éditeurs ont y dirigé vers le bas ; l’implémentation inverse y lors du tracé, de l’importation et de l’exportation.
- **Unité.** Une unité n’a pas de taille physique. La taille à laquelle une unité est dessinée ou imprimée dépend de la visionneuse.
- **Décimales.** Les coordonnées et les longueurs peuvent s’écrire avec des décimales (3.7) ; elles n’ont pas besoin d’être entières, et il n’y a aucune limite au nombre de chiffres. Les coordonnées sont dessinées telles qu’écrites et ne sont pas arrondies à des entiers.
- **Grille.** Les broches devraient se trouver sur des points entiers (9.2.4) ; les graphiques dans les symboles ne sont pas limités. Lorsqu’une instance est placée sur un point entier avec l’une des huit orientations, les positions entières des broches restent entières après la transformation, si bien que les fils aboutissent aussi sur des points entiers : les points entiers forment la grille habituelle. Le langage n’a pas de grille propre. Le fait que le solveur arrondisse les coordonnées libres à des entiers relève de sa propre règle, classée après toutes les descriptions du langage (8.5) ; lorsque des contraintes imposent une décimale, la décimale est utilisée. Les points de coude que calcule un mode de routage gardent leurs valeurs exactes et ne sont pas arrondis (13.1).
- **Tolérance.** Lors de la vérification qu’une contrainte est satisfaite et lors de la résolution des conflits (8.5), un écart inférieur ou égal à la tolérance compte comme satisfait. La tolérance est une option du solveur que l’utilisateur peut régler ; la présente norme ne fixe pas sa valeur, mais elle est supérieure à 0, afin qu’une valeur sans écriture décimale finie puisse être réécrite (ci-dessous). Une visionneuse qui ne résout pas utilise son propre réglage lorsqu’elle vérifie. Deux outils peuvent donc diverger sur un cas situé juste à la limite de la tolérance. Un solveur réécrit les coordonnées sous forme de décimales qui se relisent à la même valeur, sans troncature supplémentaire ; une valeur qui n’a pas d’écriture décimale finie, telle que 1/3, est écrite avec suffisamment de chiffres pour que les faits qu’elle satisfait restent vérifiés dans les limites de la tolérance lorsque l’enregistrement est relu (chapitre 17).
- **Coordonnées des symboles.** Les coordonnées locales dans un corps de DEFINE-SYMBOL ont l’unité et les axes de la scène ; leur origine est l’origine du symbole (9.2.1, 6.5). Il n’y a pas de mise à l’échelle : les symboles ne prennent aucun paramètre d’échelle, et les instances ne peuvent pas être mises à l’échelle. Un importateur convertit les symboles d’autres outils en unités RNL.
- **Autres longueurs.** Toutes les longueurs du langage sont en unités de coordonnées et suivent l’échelle du dessin : `min`, `max` et `exact` de DISTANCE, `radius=` de CIRCLE, `width=` et `height=` de RECTANGLE et ELLIPSE, la taille de texte `size` (comme `font-size` dans SVG), l’épaisseur de trait `linewidth`, la longueur de flèche `arrowsize` (9.3), ainsi que les paramètres de définitions utilisés comme longueurs (tels que `gap=3`). Une épaisseur de trait de 0 fait exception : un trait d’épaisseur minimale (hairline), tout aussi fin à chaque niveau de zoom, ce qui est aussi la valeur par défaut (9.3).
- **Origine et papier.** L’origine de la scène n’a aucune signification particulière. Chaque sous-circuit forme un dessin (6.1) doté de ses propres coordonnées ; la visionneuse cadre la vue selon l’étendue du contenu. RNL n’enregistre ni format de papier ni grille d’accrochage d’un éditeur.

### 11.8 Faisceaux : BUNDLE

Une instance itérée de Virtuoso, telle que `I0<3:0>`, est un seul symbole mais quatre lignes dans la netlist :

```spice
* A unit cell repeated four times
* @RNL(type="header") { @VERSION("0.4")}
.subckt unit in out vdd vss
R1 in out 1k
.ends unit
XI0<3> D<3> out vdd vss unit
XI0<2> D<2> out vdd vss unit
XI0<1> D<1> out vdd vss unit
XI0<0> D<0> out vdd vss unit
* @RNL {
*   #XI0<3:0> @BUNDLE => @PLACE(30, 10) => %x;   /* four instances drawn as one symbol */
*   &D<3:0> @WIRE {(20, 10) %x:in};              /* a bus to the in pin of the bundle */
*   &out @WIRE {%x:out (40, 10)};                /* all four members are on out: an ordinary pin */
*   %x @TEXT("I0<3:0>", key="name")              /* the name, written out */
* }
.end
```

- **Un nouvel objet.** BUNDLE prend un ensemble d’instances hôtes et crée un faisceau, qui est dessiné une seule fois à la place de ces instances. Les membres sont dans l’ordre écrit, et le premier membre décide des broches et du symbole.
- **Broches et symbole.** Les noms de bornes et le nombre de bornes sont ceux du premier membre. Les files d’apparence traitent le faisceau comme son premier membre : les sélections atteignent le faisceau selon les conditions de son premier membre (10.1), et le symbole de bloc du sous-circuit s’applique. SET-SYMBOL, les styles, PLACE, ORIENT, LAYER, les contraintes et les textes écrits directement sur le faisceau appartiennent au faisceau.
- **Bornes.** Une borne du faisceau, telle que `%x:in`, représente les bornes de même nom de tous les membres : c’est une broche de bus.
  - **Par nom** : la borne du faisceau représente les bornes portant ce nom chez les membres, où qu’elles se trouvent dans l’ordre de chaque membre ; un membre qui n’a aucune borne de ce nom n’en fait pas partie.
  - `::net` : lorsque tous les membres sont sur le même net, ce net, tel que `&out` pour `%x:out::net` ; sinon l’ensemble de leurs nets dans l’ordre des membres, tel que D<3>, D<2>, D<1>, D<0> pour `%x:in::net`. Ainsi `?t::net @WIRE {…}` dans une définition dessine un bus lorsqu’il reçoit un ensemble, et un fil ordinaire lorsqu’il reçoit un seul net (13.1).
  - Un fil vers une borne du faisceau connecte cette borne de chaque membre (10.2).
- **Attributs** (13.6) : lorsqu’une valeur est identique pour tous les membres, cette valeur ; sinon les valeurs jointes par `,`. `${w}` affiche 1u lorsque tous valent 1u ; `${name}` vaut `XI0<3>,XI0<2>,XI0<1>,XI0<0>`, et pour afficher `I0<3:0>`, écrivez un texte portant la même clé, comme dans l’exemple. `${net}` d’une borne fonctionne de la même façon, tel que `D<3>,D<2>,D<1>,D<0>`.
- **Sélections.** Les membres ne peuvent pas être sélectionnés ; le faisceau est sélectionné à la place (10.1). Ainsi une règle telle que « un nom de net pour chaque broche non câblée » donne au faisceau une seule étiquette, qui affiche tout le bus. Un rôle d’une définition qui prend des instances prend aussi des faisceaux, et un rôle qui prend des bornes prend aussi les bornes des faisceaux (9.4).
- **Les membres ne sont pas dessinés séparément.** Une description relative au dessin écrite directement sur un membre ou sur une borne d’un membre (position, orientation, symbole, style, couche, contrainte, fil ou texte) échoue : le membre n’est dessiné qu’à travers le faisceau. Les descriptions qui ne concernent pas le dessin fonctionnent comme d’habitude, telles que CURRENT et les rôles d’utilisations sémantiques.
- **Tous de même sorte.** Les membres devraient avoir le même nom de modèle ou de sous-circuit et les mêmes noms de bornes, en nombre, en noms et en ordre. Lorsqu’ils ne les ont pas, un avertissement est émis ; ils sont tout de même groupés en faisceau, avec les broches et le symbole du premier membre. Les membres qui ne diffèrent que par leurs paramètres ne donnent aucun avertissement : leurs attributs affichent les valeurs jointes (ci-dessus).
- **Un seul faisceau pour chaque instance.** Une instance écrite dans plusieurs BUNDLE appartient à celui écrit plus tard (1.5 point 2) ; le précédent garde ses autres membres, et une indication peut être émise. Un faisceau antérieur dont tous les membres ont été pris est vide et n’est pas dessiné, avec un avertissement.
- BUNDLE prend des objets écrits explicitement : s’il reçoit le résultat d’une sélection, ou un ensemble qui en dépend (10.4), il échoue, car un faisceau doit exister avant que les symboles soient attribués (8.2, 10.4). Pour la même raison, un BUNDLE dans du contenu que génère l’étape 6 échoue. BUNDLE avec un ensemble vide échoue (5.2.1). Un membre qui n’est pas une instance hôte échoue (mauvaise sorte). Un membre introuvable fait échouer l’instruction entière (stricte, 7.3).

<!-- translated from en/ch12.md sha256:5da84c59c21c -->
## 12 Composants virtuels

### 12.1 Concept

- Un composant virtuel est un composant qui n’existe qu’en RNL, tel qu’une masse, une alimentation, une étiquette de port ou une marque de non-connexion. Il n’existe pas électriquement : il est seulement dessiné, et il ne crée ni ne modifie aucun fait électrique.
- Un composant virtuel est une utilisation pourvue d’un symbole ; il n’y a ni mot-clé ni sorte d’objet distincte pour lui. `$GROUND @ATTACH &vss` crée une utilisation ; le paquet standard attribue aux utilisations de `$GROUND` un symbole de masse, si bien qu’elle est dessinée.
- Règles :
  1. **Broches et rôles.** Une utilisation n’a pas de bornes. Les broches de son symbole sont atteintes par nom, telles que `%g0::pin` (6.5), et `%g0::p` reste le membre qui remplit le rôle p, le net vss. Une broche dont `terminal=` nomme un rôle de la définition, mis en correspondance par nom ou par `terminals=` (11.3), est liée à ce rôle : elle est dessinée sur ce qui remplit le rôle, de sorte qu’un fil sur ce net peut s’y terminer et que ROUTE-NET la relie (13.7), et `::net` de la broche est ce net. Une broche qui lie un rôle est dessinée sur le net de ce qui le remplit : un net, ou le net d’une borne ou d’une broche (5.2.4). Ainsi `$GROUND @ATTACH #M5:3` place une masse sur le net de M5:3, à sa propre position, et non sur cette broche ; pour dessiner une masse à la broche, utilisez une définition dont le corps le fait, telle que `$GROUND_STUB` (10.5). Un rôle lié rempli par le nœud vide (6.3) ne place la broche sur aucun net, ce qui est valide : elle n’est alors reliée que par des fils écrits sur le nœud vide (13.1), et la complétion du paquet standard ne lui ajoute rien. Un rôle lié rempli par une autre sorte, telle qu’une instance ou un point, ne donne aucun net à la broche : une erreur de sorte, pour laquelle SET-SYMBOL échoue pour cette utilisation, qui n’est pas dessinée, et le diagnostic pointe également vers l’application. Un rôle déclaré avec la sorte `"net"` convertit une borne en son net lorsque la définition est appliquée (9.4), et une broche une fois les symboles attribués (5.2.4). Un rôle qu’aucune broche ne lie prend les sortes que déclare sa définition, telle que la borne de `$NO_CONNECT` (12.6). Ce n’est pas une borne électrique : SELECT-TERMINAL ne le sélectionne pas et ASSERT-TERMINAL ne le vérifie pas ; SELECT-PIN le fait (10.1). Une broche qui ne lie aucun rôle n’est que dessinée ; elle est câblée à la main par son nom. Un nom de broche ne peut pas être identique à un nom de rôle (6.5).
  2. Une utilisation n’est dessinée que lorsqu’un symbole lui a été attribué ; une utilisation sans symbole n’est pas dessinée. Le paquet standard n’attribue des symboles qu’aux définitions destinées à être dessinées, de sorte que les utilisations sémantiques telles que les paires différentielles ne sont pas dessinées.
  3. Une utilisation sans symbole ne dessine rien par elle-même : ORIENT sur elle échoue, tout comme un fil vers un nom de broche de celle-ci, car elle n’a aucune broche (9.2.4) ; cela est vérifié après l’étape 7 de 8.2. Elle possède tout de même son point propre, `%u::position` : PLACE prend ce point, tout comme les contraintes écrites sur `%u::position`, tandis qu’une contrainte sur l’utilisation elle-même prend ses membres (14.1) ; le point n’apparaît dans le dessin qu’à travers ce que le corps de sa définition y attache (14.1, 9.4). Un fil ou un graphique ne peut aboutir à ce point que par `::position`, tel que `&n {%u::position %pt} @WIRE` : c’est un point, non une broche, de sorte que le fil ne connecte rien (5.2.4, 6.5). Un style qui lui est attribué n’est pas une erreur : le style est transmis aux objets dessinables créés dans son corps (8.3), et n’a aucun effet visible lorsque rien n’est dessiné dans le corps (11.3).
  4. Une utilisation dessinée ne dessine que son propre symbole. Elle ne contient ni ne déplace ses membres, et n’est pas un conteneur visuel.
  5. **Où elle est dessinée.** L’origine de son symbole est le point propre de l’utilisation (14.1) : PLACE et les contraintes le déplacent, ORIENT fait tourner le symbole autour de lui (11.2), et il ne suit pas les objets qui remplissent les rôles, sauf par des faits, tels que des fils vers ses broches et les faits dans le corps de sa définition. Une définition qui dessine une marque sur une broche ou à un point lie le point de l’utilisation à celui-ci dans son corps, tel que `{? ?t} @ALIGN("x"); {? ?t} @ALIGN("y")` (9.4). Sans ORIENT, le solveur choisit son orientation et la réécrit (14.8), et un moteur de rendu qui n’a aucune solution pour elle dessine `"R0"` (8.5) ; elle ne suit pas la direction dans laquelle les fils quittent une broche. L’objet qui remplit le premier rôle est l’objet principal de l’utilisation ; une utilisation n’a pas de nom propre, et les textes se réfèrent à l’objet principal par le rôle, tel que `${p}` (13.6).
  6. Le corps d’une définition est toujours développé, si bien que la définition d’un composant virtuel peut comporter un corps.
- Selon les termes de Principes 2.2, un composant virtuel est une utilisation pourvue d’un symbole dont les rôles sont remplis par des objets hôtes ; un composant virtuel standard est une définition du paquet standard accompagnée d’une règle qui lui attribue un symbole par défaut ; un composant virtuel défini par l’utilisateur est une définition propre à l’utilisateur, accompagnée d’une règle de symbole ou d’un symbole attribué dans son corps.
- **Définir un composant virtuel.** La manière standard consiste en une définition générale suivie immédiatement de la règle qui attribue leur symbole à ses utilisations : `$GROUND = @DEFINE(roles=[["p", "net"]]) {}; # @SELECT-USAGE(recursive=true) $GROUND => @SET-SYMBOL $GROUND_SYMBOL`. La règle prend sa place dans la file des symboles selon sa position (8.3), si bien qu’une règle ultérieure, d’un paquet de style maison ou du fichier, dessine les mêmes utilisations autrement, telle que `# @SELECT-USAGE(recursive=true) $GROUND => @SET-SYMBOL $EARTH`, y compris celles créées à l’intérieur d’autres définitions ou par la complétion ; une utilisation isolée est dessinée autrement en la nommant, `%g1 @SET-SYMBOL $EARTH`. Le corps peut aussi attribuer le symbole, `? @SET-SYMBOL $X`, où `?` seul est l’utilisation (9.4) : cela nomme directement l’utilisation (8.3), de sorte qu’aucune règle ne le remplace, ce qui convient à un composant qui n’est pas destiné à être dessiné autrement. Changer le dessin ne change ni les bornes ni les fils qui s’y trouvent.
- Le fait qu’une utilisation soit un composant virtuel dépend uniquement du fait qu’elle possède un symbole.
- Note : une définition qui devrait être dessinée mais à laquelle aucun symbole n’est attribué n’est signalée que lorsqu’une de ses utilisations est orientée ou câblée, ou lorsqu’une contrainte sur elle n’a aucun effet visible (14.1).

### 12.2 Création de composants virtuels

```rnl id=g0
$GROUND @ATTACH &vss => @PLACE(24, 38) => %g0;
&vss @WIRE {#M5:3 %g0::pin}
```

- Dans `$D @ATTACH &net`, les objets qui suivent la définition sont mis en correspondance avec la liste des rôles de la définition. Cela s’écrit avec ATTACH et se lit comme « ajouter ceci sur le net » (9.4). Un composant virtuel à plusieurs rôles s’écrit comme une ligne d’instance de la netlist : pour une définition `$DIFF_PORT` ayant les rôles a et b, `$DIFF_PORT @ATTACH &inp(a) &inn(b)`, ou par ordre, `$DIFF_PORT @ATTACH &inp &inn`.
- Les rôles sont habituellement remplis par des nets : les broches d’un composant virtuel qui lient ses rôles sont dessinées sur ces nets (12.1 point 1), et sa relation à une broche d’un composant (où il est dessiné, à quoi il se connecte) est donnée par des fils et des contraintes (12.3). Une broche qui lie un rôle est dessinée sur le net de ce qui le remplit (12.1 point 1) : `$GROUND @ATTACH #M5:3` attache une masse au net de M5:3, comme le fait `$GROUND @ATTACH #M5:3::net`, à sa propre position. Les sortes d’objets qu’un rôle accepte peuvent être déclarées dans la définition (9.4), telles que `roles=[["p", "net"]]`.
- Cette association n’est que descriptive : elle ne crée aucune connexion électrique et n’est pas vérifiée (Principes 2.2).
- Un net peut porter un nombre quelconque de composants virtuels, tels que cinq masses et deux sondes. Les utilisations n’ont pas de clé : chaque application crée une utilisation de plus (5.8).
- Un même composant virtuel isolé peut recevoir un autre symbole, tel que `%g1 @SET-SYMBOL $EARTH` avec un symbole `$EARTH` propre.
- ATTACH n’a aucun paramètre propre autre que `priority` (14.8). La direction d’un port relève du contenu du paquet standard (12.4) ; le nom affiché est celui de l’objet principal, et pour afficher un autre texte, écrivez une étiquette (13.3). Les paramètres de la définition sont passés par ATTACH comme d’habitude (9.4).
- `%g0` n’est lié qu’une seule fois dans un niveau (6.6) ; pour modifier sa position, écrivez `%g0 @PLACE(…)`.

### 12.3 Positions relatives à une borne

Les paramètres ne peuvent pas contenir de références hôtes, donc une position relative à une borne s’écrit sous forme de contraintes :

```rnl use=g0
{%g0::pin #M5:3} @ALIGN("x");
%g0::pin #M5:3 @DISTANCE("bottom-to-top", exact=2)    /* the pin of the ground 2 below the source of M5 */
```

Avec `@ALIGN("y")` à la place de DISTANCE, la broche de la masse se superpose à la broche de M5. Un fil de longueur 0 entre les deux broches, `&vss @WIRE {#M5:3 %g0::pin}` (13.1), rend aussi la broche câblée, afin que la complétion n’ajoute pas une seconde masse à cet endroit (10.2, 10.5).

### 12.4 Ports

```rnl
$IO_PORT @ATTACH &inp => @PLACE(0, 20) => %p_inp
```

- Le nom d’un port d’un sous-circuit est le nom du net à l’intérieur, si bien qu’une étiquette de port est associée au net `&inp`.
- Un port lui-même (`#:inp`) n’est pas dessiné ; il n’est affiché que par un composant virtuel. La complétion de la série standard 0.4 donne à chaque net de port une étiquette de port, y compris aux nets d’alimentation et de masse, par l’ensemble `$PORT_FILL_SET`, en excluant les nets qui ont une étiquette de port écrite (10.5) ; les étiquettes d’un niveau se tiennent en une colonne, chacune à R0 avec sa broche à droite, et la colonne reste à au moins 6 unités à gauche de chaque instance du niveau, mesurée depuis son point propre, par des contraintes de priorité `"low"` ; le solveur décide du reste. Un fichier retire un net de l’ensemble pour le laisser sans étiquette, tel que `# @SELECT-NET(name="vdd") => @REMOVE-FROM $PORT_FILL_SET` ; écrit `$IO_PORT @ATTACH` pour une étiquette qu’il place lui-même ; ou écrit `$PORT_LIST @ATTACH` une fois dans un niveau pour la même colonne, écrite explicitement, et la complétion n’en ajoute alors aucune dans ce niveau. Le point propre d’un `$PORT_LIST` se trouve sur sa colonne (14.1), de sorte qu’une contrainte sur l’utilisation déplace la colonne.
- La direction d’un port n’est qu’une question d’affichage : la netlist n’a pas de directions, et l’utilisateur décide quel symbole est dessiné. Quelles définitions et quels symboles de ports existent relève du contenu du paquet standard.
- **Orientation des étiquettes de port.** Une étiquette de port possède une direction : la broche de `$IO_PORT_SYMBOL` dans le paquet standard, par exemple, se trouve à une extrémité, et l’étiquette s’étend de l’autre côté. Sans ORIENT, le solveur choisit l’orientation d’après les directions des fils, en visant le moins de coudes possible. C’est un objectif du solveur, et non une partie du langage (14.8), si bien que l’ORIENT d’une étiquette de port n’est généralement pas écrit à la main.

### 12.5 Composants virtuels standards

Les composants virtuels standards appartiennent au paquet standard, qui ne fait pas partie de la norme du langage (16.3). Le paquet standard fournit, par exemple, une masse `$GROUND`, une alimentation `$SUPPLY`, une étiquette de port `$IO_PORT` et une marque de non-connexion `$NO_CONNECT` (12.6), avec leurs symboles, les règles qui attribuent ces symboles et la complétion (10.5).

Pour un même net vss, trois façons d’écrire produisent trois dessins différents :

1. `&vss @ADD-TO $GROUND_NET_SET` : chaque borne sur vss sans fil reçoit une masse au-dessous d’elle avec un fil court (la complétion du paquet standard, 10.5). C’est ainsi que les manuels le dessinent habituellement : un symbole de masse sous chaque borne reliée à la masse.
2. `$GROUND @ATTACH &vss => %g` : une seule masse pour tout le net, placée par le solveur. Les bornes sont connectées à sa broche par des fils, tels que `&vss @WIRE {#M5:3 %g::pin}`, ou par ROUTE-NET pour tout le net (13.7).
3. `$GROUND_STUB @ATTACH #M5:3` : une masse avec un fil court sous la source de M5 uniquement, comme la complétion la dessine pour chaque broche (10.5).

- Une alimentation s’écrit de la même manière, avec l’ensemble `$SUPPLY_NET_SET` et la définition `$SUPPLY`. Le net 0 se trouve par défaut dans `$GROUND_NET_SET` (10.1).
- Pour la troisième façon, le fil dans le corps du raccord (stub) rend la broche câblée, si bien que la complétion n’ajoute pas une seconde masse à cet endroit (10.2, 10.5).
- Ports : la complétion donne à chaque net de port d’un sous-circuit une étiquette de port (12.4). Pour en placer une, la contraindre ou changer le nom qu’elle affiche, écrivez-la explicitement : `$IO_PORT @ATTACH &out => %pout`, avec des fils vers `%pout::pin`, et `%pout @TEXT("V_{OUT}", key="name")` pour le nom (13.3). Un net qui ne doit pas avoir d’étiquette de port est retiré de `$PORT_FILL_SET`.

### 12.6 Marques de non-connexion

```rnl
$NO_CONNECT @ATTACH #M1:4    /* terminal 4 of M1 is left unconnected on purpose */
```

- Une marque de non-connexion est la définition `$NO_CONNECT` du paquet standard, dont le rôle prend une borne ou une broche ; il n’y a aucun mot-clé pour elle. Une borne est prise telle quelle et non convertie (5.2.4), afin qu’une marque sur un membre d’une broche de bus (9.2.4), telle que `$NO_CONNECT @ATTACH #U1:d<3>`, indique quelle borne est laissée ouverte ; plusieurs marques sur une même broche de bus sont dessinées à cette broche, les unes sur les autres. Aucune broche du symbole de la marque ne lie le rôle (12.1 point 1) ; la définition attache le point de l’utilisation à la broche de ce qui remplit le rôle (12.1 point 5), et le paquet standard lui attribue un symbole en croix dont l’origine est au centre de la croix et sans broche.
- L’utilisation consigne que la borne est laissée déconnectée volontairement : `# @SELECT-USAGE $NO_CONNECT => @SELECT-TERMINAL` sélectionne ces bornes.
- Des vérifications (autres bornes hôtes sur le net, fil touchant la borne) ne sont pas exigées (Principes 2.4) ; une implémentation qui connaît `$NO_CONNECT` standard peut émettre un avertissement.
- Une marque de non-connexion ne compte pas comme un fil (10.2) : une broche qui n’a que la marque reste `wired=false`. Pour qu’une complétion par broche saute ces broches, soustrayez leurs broches, `# @SELECT-USAGE $NO_CONNECT => @SELECT-TERMINAL => @SELECT-PIN`, comme dans la différence de 10.5.
- Note : une marque de non-connexion n’est pas un mot-clé parce qu’elle n’a aucune autre utilité pour une implémentation que le dessin : le solveur ne la lit pas, et la manière dont elle est dessinée peut s’écrire dans une définition et un symbole (1.5 point 7).
- Les étiquettes de nets ne sont pas des composants virtuels ; voir 13.3.

<!-- translated from en/ch13.md sha256:492780434318 -->
## 13 Fils et texte

### 13.1 Fils : WIRE

```rnl id=w13
&x @WIRE(mode="straight") {#M1:1 (10, 37) #M3:1} => @SET-STYLE $signal => %w13;
&out @WIRE(mode="manhattan") {#M2:1 #M4:1};
&tail @WIRE {#M1:3 #M5:1}                    /* no mode: "auto", routed freely */
```

- WIRE prend un net (ou un ensemble de nets, ce qui crée un bus ; voir ci-dessous) et une liste ordonnée de points, au moins deux. Un point est un objet point ou une broche, y compris une broche d’un composant virtuel ; une borne représente sa broche (5.2.4). Les points peuvent se répéter ; lorsque le premier et le dernier point sont identiques, le fil forme une boucle.
- WIRE ne prend pas le résultat d’une sélection, ni un ensemble qui en dépend (10.4) : s’il en reçoit un, il échoue. Un fil est réécrit par le solveur comme ce même fil, avec ses coudes (14.8), et un fil créé à partir d’une sélection n’aurait rien sous la forme duquel être réécrit ; pour dessiner les broches d’une sélection sous forme de fils, utilisez ROUTE-OBJECT (13.7).
- **Mode de routage** : le fil passe par chaque point de la liste ; la façon dont il chemine entre deux points voisins est fixée par le paramètre `mode=`. Pour des points voisins A(x1, y1) et B(x2, y2), avec dx = x2 − x1 et dy = y2 − y1 :

  | `mode` | De A à B |
  |---|---|
  | `"auto"` (par défaut) | Décidé par le solveur, qui peut l’optimiser selon la situation et réécrit le tracé (voir ci-dessous, 14.8) ; avant que le fil ne soit résolu, il est dessiné selon le tracé par défaut ci-dessous, qui est aussi ce que le solveur lui donne lorsque rien ne lui barre le passage |
  | `"straight"` | Un seul segment droit |
  | `"horizontal-vertical"` | A → (x2, y1) → B |
  | `"vertical-horizontal"` | A → (x1, y2) → B |
  | `"manhattan"` | Si \|dx\| ≥ \|dy\| : la moitié de dx horizontalement, tout dy verticalement, puis l’autre moitié horizontalement : A → (x1 + dx/2, y1) → (x1 + dx/2, y2) → B ; sinon la même chose en échangeant x et y |
  | `"manhattan45"` | Si \|dx\| ≥ \|dy\| : (\|dx\| − \|dy\|)/2 horizontalement, puis une diagonale à 45° (\|dy\| selon chaque axe), puis le reste horizontalement ; sinon la même chose en échangeant x et y |

  - Tout mode sauf `"auto"` est une règle stricte, et la visionneuse le dessine tel que défini. Lorsque dx ou dy vaut 0, chaque mode donne un seul segment droit. Lorsque |dx| = |dy|, `"manhattan"` prend le cas |dx| ≥ |dy|, et `"manhattan45"` est un seul segment diagonal. Les coudes que calcule un mode gardent leurs valeurs exactes et ne sont pas arrondis ; ils peuvent tomber sur des demi-unités (11.7).
  - **Le tracé par défaut de `"auto"`** : chaque segment entre deux points voisins court horizontalement et verticalement. Une extrémité qui est une broche munie d’une direction (9.2.4), prise après l’orientation de son objet, est dirigée : le segment en part selon cette direction ou à angle droit avec elle, et y arrive dans le sens opposé à cette direction ou à angle droit avec elle, jamais autrement ; partir ou arriver à angle droit compte comme un coude. Une broche sans direction et un point sont non dirigés et ne contraignent rien. Le segment ne passe par aucune de ses extrémités et ne se chevauche pas lui-même. Parmi les tracés qui satisfont à cela, le tracé par défaut est celui qui a le moins de coudes ; puis le plus court, où un tronçon qui ne sort que pour revenir est long de 1 ; puis celui qui tourne sur le moins d’extrémités dirigées, en gardant la direction de la première extrémité de préférence à celle de la dernière ; puis celui dont le tronçon central se trouve à mi-chemin entre les extrémités, comme pour `"manhattan"` ; entre deux extrémités non dirigées, celui qui part horizontalement en premier, un L avec un coude ; et un détour part vers la gauche de la première extrémité dirigée (sa direction tournée de 90 degrés dans le sens antihoraire). Un fil comportant des points entre ses extrémités est choisi dans son ensemble : ses points intermédiaires sont non dirigés, un virage en un point intermédiaire compte comme un coude, et le fil ne peut pas faire demi-tour en un point intermédiaire, si bien qu’il le traverse tout droit lorsqu’il le peut. Ses coudes, eux aussi, gardent leurs valeurs exactes et peuvent tomber sur des demi-unités (11.7), comme lorsque son tronçon central se trouve à mi-chemin entre deux extrémités distantes d’une unité.
  - Les coudes ajoutés par un mode ne sont pas des objets : ils ne peuvent pas être référencés, et aucun point de jonction ne peut y être placé. Pour embrancher à un coude ou y dessiner un point de jonction, écrivez le coude comme un point. Un point libre `(_, _)` reçoit d’abord ses coordonnées du solveur puis est connecté selon le mode.
  - Une valeur de `mode` qui ne figure pas dans le tableau est une valeur de paramètre invalide : le fil échoue.
- **Paramètres de style** : WIRE prend également les propriétés de style de trait comme paramètres : `linecolor`, `linestyle`, `linewidth`, `cap`, `join` (9.3). Ils constituent le style direct du fil (8.3), tel que `@WIRE(linestyle="bold")` pour un rail d’alimentation ou `@WIRE(linestyle="dashed")` pour une connexion de substrat.
- **Réécrire un fil.** Recevant un fil à la place d’un net, WIRE ne crée aucun nouveau fil : il réécrit ce fil. L’instruction est lue comme le serait un nouveau WIRE, et sa liste de points ainsi que son mode remplacent ceux du fil ; le net du fil reste inchangé. Les paramètres de style constituent le style direct du fil (8.3).
  - La réécriture a une clé, le fil : une réécriture ultérieure remplace la liste et le mode d’une réécriture antérieure (8.4) ; entre deux segments d’un même chaînage, le segment ultérieur est le plus tardif (5.5). Les paramètres de style de chaque réécriture restent le style direct du fil et sont résolus selon 8.3, propriété par propriété. Une réécriture n’est pas un fait géométrique (8.5).
  - Une réécriture qui échoue, par exemple parce qu’elle énumère une borne sans broche (9.2.4), est une erreur et n’a aucun effet, ses paramètres de style compris ; le fil garde sa liste et son mode antérieurs : ceux d’une réécriture antérieure, ou ceux avec lesquels il a été créé (5.8). Chaque réécriture qui échoue est signalée, même lorsqu’une réécriture ultérieure la remplace.
  - Comme un nouveau fil, une réécriture compte à partir de l’étape 3 de 8.2 : `wired=` (10.2) lit les points de la dernière réécriture, même lorsque cette réécriture échoue plus tard, après l’étape 7 (7.1).
  - L’emplacement de net prend un net, un ensemble de nets ou un fil ; un ensemble qui contient un fil est d’une mauvaise sorte.
  - Le solveur réécrit de cette manière le tracé qu’il a choisi pour chaque fil `"auto"` (14.8). Il devrait garder, dans l’ordre, les points que le fil possède dans les enregistrements écrits par l’auteur, après leur dernière réécriture de ce fil, n’ajouter entre eux que des coudes écrits en coordonnées, et écrire `mode="straight"` : les connexions d’un fil sont écrites par son auteur, et le solveur ne décide que de sa géométrie.
  - **Scinder un fil.** Sur un net que relie un arbre de ROUTE-NET ou ROUTE-OBJECT (13.7), le solveur peut embrancher depuis le milieu d’un fil écrit par l’auteur. Il réécrit alors ce fil dans son enregistrement, par nom ou par numéro (5.9), en gardant les points de l’auteur dans leur ordre et en insérant parmi eux le point d’embranchement comme un point propre, et y connecte l’embranchement. Les autres fils ne sont modifiés que comme l’indique le point ci-dessus. Il s’agit d’une réécriture, non d’une suppression : l’instruction de l’auteur demeure.
  - Un éditeur dans lequel l’utilisateur remodèle un fil écrit la même chose dans son propre enregistrement ; le solveur garde ensuite le fil tel qu’il a été réécrit.
  - Pour fixer le tracé dans l’instruction elle-même, écrivez les coudes comme des points et utilisez un mode autre que `"auto"` ; un exportateur qui conserve la forme exacte écrit `mode="straight"` et énumère chaque coude.
- **Plusieurs bornes sur un même net** (Principes 2.6) sont connectées par plusieurs fils qui partagent des points. Des fils ne sont connectés que lorsqu’ils partagent le même objet point (un point nommé) ou la même borne ; des coordonnées égales ne les connectent pas (Principes 2.2). Il n’existe ni instruction écrivant un arbre, ni objet graphe.
- **Jonction en T** : le fil traversant énumère le point de jonction comme l’un de ses points intermédiaires et n’a pas besoin d’être scindé en deux ; l’embranchement commence à ce point (voir l’exemple en 13.2). Il n’existe pas de « point sur un segment de fil ».
- **Broches qui se touchent** : lorsque deux broches sont dessinées au même point sans fil entre elles, un fil est tout de même écrit pour connecter les deux bornes, par exemple pour un symbole de masse dessiné directement sur la broche d’un condensateur : `&0 @WIRE(mode="straight") {#C1:2 %g3::pin}`. Ses deux extrémités coïncident et sa longueur vaut 0, donc rien n’est dessiné, mais il énonce la connexion : les deux broches sont câblées (10.2), et le paquet standard n’ajoute pas d’autre masse. Des coordonnées égales seules ne connectent pas (Principes 2.2). Un exportateur qui convertit un schéma dans lequel des broches se connectent par position écrit de tels fils ; là où un fil traverse tout droit une broche ou un point de jonction, il énumère ce point comme point intermédiaire (jonction en T) ; là où un point de jonction n’a qu’un seul fil et se trouve directement sur une broche, il connecte directement le fil à cette borne.
- **Le net est seulement énoncé**, et il doit être écrit. Les nets sur lesquels se trouvent réellement les extrémités n’ont pas besoin de concorder avec lui, et cela n’est pas vérifié (Principes 2.2) ; le net n’est pas non plus déduit des extrémités, et les extrémités peuvent se trouver sur des nets différents.
  - Lorsqu’une borne ou une broche de la liste ne se trouve pas sur le net du fil (pour un fil de bus : pas dans son ensemble de nets), cela produit un avertissement : très probablement la borne a été mal saisie, comme `#R1:1` pour `#R1:2`. Le net d’une borne ou d’une broche est connu (une broche d’un composant virtuel se trouve sur le net qui remplit le rôle qu’elle lie, 12.1) ; un point, ainsi qu’une broche qui ne lie rien, n’ont aucun net et ne sont pas vérifiés. Le fil est valide et est dessiné (Principes 2.2, 7.6).
- **Un fil sur aucun net** : l’emplacement de net prend aussi le nœud vide, comme dans `& @WIRE {%sub1::y %int1::a}` (6.3). Le fil est dessiné et résolu comme tout autre, et les broches sur lesquelles il aboutit sont câblées (10.2), mais il n’appartient à aucun net : deux tels fils qui partagent un point sont dessinés joints et ne forment toujours pas un même net, aucun arbre de ROUTE-NET ou ROUTE-OBJECT ne les prend en compte, et `${net}` du fil est le texte vide. Une borne ou une broche dans sa liste qui se trouve sur un net produit l’avertissement ci-dessus, tout comme une broche sur le nœud vide dans la liste d’un fil d’un net : le dessin montrerait une connexion que le circuit n’a pas. Le nœud vide ne peut pas être un élément de l’ensemble d’un bus.
- **Bus** : l’emplacement de net prend un net, ou un ensemble de nets en bloc. Recevant un ensemble, le fil est un fil de bus, même lorsque l’ensemble n’a qu’un seul membre. Une netlist n’a pas de bus (6.11) ; un bus n’est qu’une manière de dessiner plusieurs nets en une seule ligne.

  ```spice
  * A 4-bit bus between two blocks, with two taps
  * @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4")}
  .subckt blk D<3> D<2> D<1> D<0>
  RL D<3> D<0> 1k
  * @RNL {
  *   $BLK_SYM = @DEFINE-SYMBOL {
  *     (-3, 0) @PIN(terminal=["D<3>", "D<2>", "D<1>", "D<0>"], label="D<3:0>", direction="left");
  *     @RECTANGLE {(-3, -2) (3, 2)}
  *   };
  *   # @SET-SYMBOL $BLK_SYM                                             /* one bus pin for four ports (9.2.4) */
  * }
  .ends blk
  XA D<3> D<2> D<1> D<0> blk
  XB D<3> D<2> D<1> D<0> blk
  RT3 D<3> 0 1k
  RT2 D<2> 0 1k
  * @RNL {
  *   %t3 = (20, 10); %t2 = (24, 10);
  *   %bus = &D<3:0> @WIRE(linestyle="bold") {#XA:D<3> %t3 %t2 #XB:D<3>};   /* a bus between the bus pins of XA and XB */
  *   %bus @TEXT("D<3:0>");                                                /* the name of the bus, written out */
  *   &D<3> @WIRE {%t3 #RT3:1};                                            /* a tap: a single wire that shares %t3 with the bus */
  *   &D<2> @WIRE(mode="straight") {#RT2:1 (23, 9) %t2}                    /* its last stretch enters the bus at 45° */
  * }
  .end
  ```

  - L’ordre de l’ensemble est l’ordre des bits, et les doublons sont retenus ; les ensembles écrits directement à l’intérieur de l’ensemble sont développés jusqu’à leurs membres les plus internes, dans l’ordre. Un membre qui n’est pas un net fait échouer le fil (mauvaise sorte). Un ensemble vide échoue aussi : un fil porte au moins un net (5.2.1).
  - L’emplacement de net est strict (7.3) : lorsqu’un bit est introuvable, le fil entier échoue, comme lorsqu’un net unique est introuvable.
  - Les points, `mode=`, les paramètres de style, les points de jonction et les jonctions en T fonctionnent comme pour les autres fils. Il n’y a aucun mot-clé BUS : une implémentation dessine un bus comme elle dessine un fil, et le dessiner en trait gras relève du style (1.5 point 7). Un importateur peut écrire directement `linestyle="bold"` ; le paquet standard rend les fils de bus gras par défaut grâce à la condition `bus=true` de SELECT-WIRE (10.1, 16.3).
  - **Dérivations (taps)** : elles n’ont besoin d’aucun mécanisme : un fil unique qui partage un point avec un bus est une dérivation, et le bus énumère le point de dérivation comme point intermédiaire (jonction en T). Un court tronçon à 45° entrant dans le bus n’est que la manière dont chemine le dernier tronçon du fil unique. Un symbole de dérivation est une définition appliquée au point de dérivation et appartient au paquet standard (16.3). Le nom du bit est une étiquette `${net}` sur le fil unique (13.3). Que le net du fil unique appartienne au bus n’est pas vérifié (Principes 2.2) ; une implémentation peut émettre une indication.
  - Une dérivation peut aussi partir d’une extrémité d’un bus : un bus qui se termine à un point peut se poursuivre sous forme de plusieurs bus ou fils uniques qui partent de ce point, tel un tronc `BOT<10:0>` qui s’arrête là où repartent `BOT<10:3>` et `BOT<2:0>`, avec un point de jonction à cet endroit (13.2). Le tronc énumère ce point comme extrémité, et chaque partie prend ses propres bits. Des fils qui partent d’un même point où passe ou se termine aussi un fil portant tous leurs nets, ici le tronc, sont des dérivations de ce fil et ne sont pas joints entre eux : une implémentation qui avertit lorsqu’un fil aboutit sur un fil d’un autre net, comme si les deux étaient joints, n’avertit pas à leur sujet.
  - L’attribut `net` d’un fil de bus est la liste des noms de ses nets dans l’ordre, joints par `,` et sans espaces, tels que `D<3>,D<2>,D<1>,D<0>` ; pour afficher `D<3:0>`, écrivez ce texte. Les implémentations n’abrègent pas la liste : l’abréviation dépend de la manière dont l’hôte écrit les noms et n’a pas de résultat unique.
  - Ainsi `{&a &b} @WIRE {…}` est un seul fil de bus, et non deux fils dessinés l’un sur l’autre.
  - Plusieurs instances hôtes dessinées comme une seule (une instance itérée) s’écrivent comme un faisceau (11.8).
- Une borne à laquelle se connecte un fil doit posséder une broche dans son symbole ; sinon le fil échoue (9.2.4).
- Des lignes qui se touchent, se croisent ou se chevauchent ne sont que de la géométrie et ne créent aucune connexion (Principes 2.2). Les fils qui se croisent sans partager de point ne reçoivent aucun point de jonction ; de petits arcs aux croisements (ponts de fil) et autres marques semblables relèvent du style et de l’implémentation.
- Chaque WIRE recevant un net est un nouvel objet sans clé : écrit deux fois, ce sont deux fils (5.8). Pour désigner un fil depuis une autre instruction, nommez-le ; l’enregistrement du solveur désigne un fil sans nom par son numéro (5.9). Une extrémité qui est une borne se déplace avec son composant ; un point écrit avec des coordonnées ne se déplace pas ; un point nommé peut être contraint.
- **Connexions non dessinées** : la manière dont est montrée une connexion qui n’a aucun fil dans le RNL (routée par le solveur et réécrite sous forme de WIRE, une ligne du chevelu, une étiquette) est décidée par le style maison du solveur et de la visionneuse et par les paquets de style maison utilisés ; le langage ne la spécifie pas. Un paquet de style maison peut créer des étiquettes ou des fils par des règles, et ceux-ci sont du RNL. Pour qu’un net soit dessiné en fils avec un routage confié au solveur, écrivez ROUTE-NET ou ROUTE-OBJECT (13.7).

### 13.2 Points de jonction

```rnl
%j1 = (10, 30);
&x @WIRE {#M1:1 %j1 #M3:1};          /* the through wire passes %j1 */
&x @WIRE {%j1 (30, 30) #M4:2};       /* the branch starts at %j1 */
$JUNCTION @ATTACH %j1                /* draws the dot */
```

- Un embranchement est constitué de plusieurs fils qui partagent un même point (13.1).
- Les points de jonction s’écrivent explicitement : un point de jonction n’est dessiné que là où une définition qui en dessine un, telle que `$JUNCTION` du paquet standard, est appliquée ; un moteur de rendu n’ajoute pas de points de jonction de lui-même.
- Un point de jonction est une utilisation, non un mot-clé. `$JUNCTION` est une définition du paquet standard à un seul rôle, qui prend un point ou une borne ; sa définition attache le point de l’utilisation à ce point ou à la broche de la borne (12.1 point 5), comme pour `$NO_CONNECT` (12.6) et la flèche de courant (15.2). Il peut être appliqué à une borne lorsque deux fils se rejoignent à sa broche. Le paquet standard attribue à ces utilisations un disque plein comme symbole : la taille du point de jonction provient du symbole, et sa couleur du `linecolor` du style de l’utilisation ; il n’y a aucun réglage séparé pour la taille des points de jonction.
  - Appliquer `$JUNCTION` deux fois au même point produit deux disques l’un sur l’autre, qui ressemblent à un seul.
  - Tous les points de jonction sont sélectionnés avec `# @SELECT-USAGE $JUNCTION`.
  - L’ordre de tracé suit la position de la règle qui attribue le symbole (11.6). Les règles du paquet standard viennent en premier, si bien que les points de jonction sont dessinés avant les fils ; un point de jonction a la couleur des fils et est plus large qu’eux, de sorte que cela ne fait aucune différence visible, et aucun LAYER n’est nécessaire.
- Les points de jonction ne sont pas déduits du nombre de fils qui se rejoignent en un point partagé : de vrais schémas comportent des points d’embranchement à seulement deux fils dessinés avec un point de jonction, et un point de jonction déduit nécessiterait encore un autre moyen pour le supprimer. Un exportateur écrit les points de jonction que montre l’original. Une implémentation peut émettre une indication lorsque trois fils ou plus se rejoignent en un point partagé sans point de jonction ; la présente norme ne l’exige pas.
- **Points de jonction ajoutés par le solveur** : la sortie du solveur est aussi du RNL et peut contenir des points de jonction. La définition qu’il utilise est donnée par la clé `junction` de SOLVER-CONFIG (14.10), un nom en `$` écrit sous forme de chaîne, tel que `junction="$JUNCTION"`, ou, dans le SOLVER-CONFIG d’un enregistrement de niveau, un nom en `%` de ce niveau, tel que `junction="%BIG_DOT"` pour un point de jonction plus grand défini pour ce seul sous-circuit ; le paquet standard fixe `$JUNCTION` pour les niveaux qui l’utilisent (16.3). Avec cette clé, le solveur applique cette définition à chaque point d’embranchement des arbres de ROUTE-NET et ROUTE-OBJECT (13.7), et là où des fils écrits par l’utilisateur se rejoignent à trois ou plus en un point partagé sans utilisation de cette définition, il peut également en ajouter une dans sa sortie. Sans la clé, il n’écrit aucun point de jonction, et `junction=""` les désactive là où un paquet l’a fixée. Le nom est recherché comme un nom écrit dans l’enregistrement du solveur de ce niveau (6.8) ; toute autre valeur qui ne nomme aucune définition à cet endroit, ou une définition dont le premier rôle ne prend pas de point, est une valeur non acceptée à ce niveau (14.10) : une erreur, et la clé compte comme non écrite à cet endroit, de sorte que la valeur d’avant s’applique ou, s’il n’y en a aucune, que le solveur n’écrit aucun point de jonction. Un point partagé est toujours un point nommé ou une borne (des coordonnées égales ne comptent pas), de sorte que le point de jonction peut être écrit. Les autres outils qui ajoutent des points de jonction, tels qu’un éditeur, prennent la définition à partir de la même clé.
- Un point de jonction n’est que du dessin : il ne crée aucune connexion et n’est pas vérifié (Principes 2.2). Un point muni de `$JUNCTION` et sans fil reçoit tout de même son disque.

### 13.3 Texte : TEXT

```rnl use=w13
(50, 18) @TEXT("5T OTA testbench");                         /* an annotation: attached to a point */
(1, 1) @TEXT("see here") => @PLACE(2, 2);                   /* attached to (1, 1); the text itself is at (2, 2) */
#M1 @TEXT("W/L=${w}/${l}", key="wl") => @PLACE(12, 18);     /* a label: attached to M1, with the key wl, so it is #M1::wl */
#R1 @TEXT("R_{D}", key="name");                             /* a key the symbol has: new content, the position still from the symbol */
#M1::name @PLACE(13, 18);                                   /* the name of M1, dragged in an editor */
%w13 @TEXT("${net}");                                       /* the net name on a wire; no key; placed by the implementation */
&out @TEXT("${name}");                                      /* shows the name of the net out; placed by the implementation */
# @SELECT-INSTANCE => @SELECT-PIN(wired=false) => @TEXT("${net}", key="netname")    /* net names on pins without wires */
```

- **Texte vide** : un TEXT dont le contenu est la chaîne vide ne dessine rien. Avec une clé, il remplace l’étiquette du symbole portant cette clé et la masque ainsi. Le paquet standard `rnl-std-no-labels` masque plutôt les étiquettes par défaut avec le style `textvisible=false` (9.3, 16.3), ce qui garde leur contenu, de sorte qu’un TEXT portant la même clé et `textvisible=true` en réaffiche une.
- **Attachement** : TEXT prend un point, ou un objet dessiné, et le texte lui est attaché. Ce à quoi il est attaché décide trois choses : de qui `${…}` lit les attributs (13.6) ; où va le texte lorsque sa position n’est pas donnée ; et, lorsqu’il a une clé, de quel objet le texte est un sous-objet.
  - Un point (`(x, y)`, un point nommé, un `::position`) : le texte est une annotation (13.4). Il ne peut se référer qu’aux attributs `x` et `y` du point.
  - Un objet : le texte est une étiquette. L’objet peut être une instance, une utilisation dessinée, un faisceau, une broche, une borne (à sa broche, 5.2.4), un fil ou un graphique. Il peut aussi être un net : un net lui-même n’est pas dessiné, et l’implémentation place un texte attaché à un net à côté de l’un de ses fils ou de l’une de ses bornes. C’est ainsi qu’un fichier indique quels nets affichent leurs noms.
  - Une utilisation sans symbole, ou une borne sans broche, n’a aucune signification pour le dessin : si le TEXT en reçoit une, il échoue (12.1, 9.2.4).
  - Recevant un ensemble, TEXT attache un texte à chaque membre (5.2.1), comme à la dernière ligne de l’exemple.
  - Dans un corps de symbole, un texte attaché à un point est un texte fixe du symbole ; un texte attaché à `?` seul est attaché à l’objet dessiné avec le symbole (voir ci-dessous).
- **Contenu** : `text` est requis, y compris lorsqu’une clé est donnée ; sans lui le TEXT échoue. Il peut être donné par position, en premier, comme dans `@TEXT("${name}", key="name")`. Il n’y a pas de contenu par défaut : pour afficher le nom, écrivez `"${name}"`. Le balisage et les références aux attributs sont décrits en 13.6.
- **Position** : un texte possède sa propre position, qui n’est pas le point ou l’objet auquel il est attaché.
  - La position d’un texte commence libre, comme `(_, _)`. Lorsqu’aucun PLACE, `dx`, `dy` ni contrainte ne la fixe, le solveur la décide et la réécrit (14.8) ; sans solution, la visionneuse décide : un texte attaché à un point va habituellement à ce point, et un texte attaché à un objet à côté de l’objet ; l’un comme l’autre peut être éloigné pour éviter les chevauchements.
  - PLACE fixe la position du texte lui-même. Ainsi `(1, 1) @TEXT(…) => @PLACE(2, 2)` n’est pas une contradiction : la première partie attache le texte à (1, 1), la seconde place le texte en (2, 2). Pour placer exactement une annotation quelque part, écrivez PLACE.
  - `dx`, `dy` : décalages, pris uniquement par TEXT. Lorsque ce à quoi le texte est attaché possède un point de coordonnées (un point, l’origine d’une instance, le point où une utilisation est dessinée, la broche d’une borne), le texte va à ce point augmenté de (dx, dy). Les fils et les nets n’ont pas de point de coordonnées : un décalage sur un texte attaché à l’un d’eux fait échouer le TEXT.
    - Le texte n’a toujours qu’une seule position, celle avec le décalage ; les contraintes et le solveur l’utilisent, et il n’y a aucune position « avant le décalage ». Lorsque l’objet attaché est déplacé ou placé par le solveur, le texte se déplace avec lui ; lorsque d’autres contraintes déplacent le texte, l’objet attaché se déplace avec lui. Un décalage équivaut à un DISTANCE exact pour x et un pour y, écrits sur le texte : un décalage positif dans la direction croissante (`"left-to-right"`, `"bottom-to-top"`), un négatif dans la direction décroissante, avec sa grandeur comme `exact` (14.5).
    - Les décalages suivent les axes de la scène et ne tournent pas avec l’orientation de l’objet attaché, car les textes dans la scène ne tournent pas avec les objets. Dans un corps de symbole ils peuvent être écrits aussi ; ils y sont en coordonnées locales et se transforment avec l’instance, comme une position locale écrite avec `=> @PLACE(…)`.
    - **Les décalages et PLACE se remplacent mutuellement** : sur chaque axe, la position d’un texte est soit absolue (le x ou le y de PLACE), soit un décalage (`dx` ou `dy`), jamais les deux. Sur le même axe du même texte, la forme écrite plus tard (selon la position de 8.5) prend effet, et la forme écrite plus tôt n’a aucun effet du tout : après un décalage, la position absolue sur cet axe est libre et découle du décalage ; après un PLACE, le décalage sur cet axe ne s’applique plus. C’est un remplacement, non un conflit : la priorité ne joue aucun rôle, et aucune indication n’est émise. Par exemple, après `dx` et `dy`, un `#M1::name @PLACE(x=13)` ultérieur fixe x à la valeur absolue 13, `dx` ne s’applique plus, et y suit toujours `dy` ; un éditeur écrit ceci lorsqu’un texte placé par décalage est déplacé à la souris. Une position donnée dans le symbole vient en premier (voir ci-dessous), et un PLACE ou un décalage dans la scène la remplace.
    - Celle des deux formes qui prend effet est un fait géométrique ordinaire : le x et le y d’un décalage forment une unité chacun, leur position est celle de l’instruction TEXT qui les écrit, et leur priorité est celle par défaut de l’en-tête d’enregistrement (14.8) ; ils sont résolus avec les autres faits selon 8.5. Plusieurs PLACE sur le même axe sont résolus entre eux selon 8.5. Pour un texte muni d’une clé, `dx` et `dy` sont surchargés séparément (8.4).
  - Un texte peut être membre de contraintes par sa propre position (14.1). Le `::position` d’un texte est sa propre position : `%n1::position @PLACE(…)` équivaut à `%n1 @PLACE(…)`.
  - Les textes dans la scène ne tournent pas avec l’orientation de ce à quoi ils sont attachés ; pour en tourner un, écrivez `rotate=`. Lorsque la scène fournit les deux axes de la position d’un texte, par PLACE ou par décalages, aucun axe n’utilise la position locale du symbole : le texte n’hérite pas de l’orientation de l’objet attaché, et `halign` et `valign` sont interprétés dans les directions de la scène.
  - Le solveur réécrit la position qu’il a choisie (14.8) : par la clé (ci-dessous) ou le nom du texte, ou sinon par son numéro (5.9). Un texte sans clé qu’une règle de génération crée directement (13.3, 10.5) ne peut pas être référencé : le solveur laisse sa position à la visionneuse. Un texte dans le corps d’une utilisation que crée une règle est désigné par numéro une fois que le solveur écrit l’utilisation comme instruction explicite (10.5, 5.9).
- **Clé `key=`** :
  - Sans clé, le texte est un texte sans clé de l’objet attaché ; écrit une seconde fois, c’est un autre texte, comme pour les fils (13.1). Il peut être nommé avec `%x =` et être référencé et modifié dans son niveau. Les textes créés par des règles sans clé n’ont aucune poignée, donc le solveur ne peut pas réécrire leurs positions.
  - Avec une clé, le texte est le sous-objet `object::key` de l’objet attaché, tel que `#M1::wl`. Le même objet avec la même clé est le même texte, et les instructions ultérieures le surchargent paramètre par paramètre : la taille, l’alignement, la rotation et ainsi de suite sont chacun surchargés séparément, et les paramètres non écrits restent tels quels (8.4). Le contenu est écrit dans chaque instruction (`text` est requis), et le contenu ultérieur remplace toujours l’antérieur ; pour ne changer que l’alignement ou la rotation, écrivez le contenu à nouveau, comme dans `#M1 @TEXT("${name}", key="name", halign="end")`. La position suit les règles de PLACE (8.5). D’autres enregistrements, les éditeurs et le solveur retrouvent le texte par cette poignée.
  - Un texte attaché à un point ne prend pas de `key=`, car un point n’a aucun sous-objet : avec une clé, le TEXT échoue.
  - Les noms après `::` sont recherchés dans un ordre fixe, et les clés viennent en dernier (6.5) : les noms de rôles et les noms locaux d’une utilisation, puis leurs alias `definition=`, puis les noms de sous-objets standards et leurs alias, puis les noms des broches du symbole de l’objet, puis les clés des textes. Une clé ne doit donc pas s’écrire comme un nom qui vient avant elle : un nom de sous-objet standard (`position`, `net`) ou un alias d’un tel nom déclaré pour le fichier (tel que `位置` d’`alias-zh`) ; un nom de rôle ou nom local de l’utilisation à laquelle le texte est attaché ; ou un alias `definition=` de l’un d’eux. Une telle clé est une erreur, comme les collisions de noms de 6.5. Pour un texte écrit dans un symbole, les alias sont ceux du fichier dans lequel le symbole est écrit. `name` et `value` ne sont pas des noms de sous-objets standards, mais seulement des clés courantes.
- **Textes dans les symboles** :
  - Dans un corps de symbole, `?` seul est l’objet dessiné avec le symbole : M1 lorsque M1 est dessiné, `%g0` lorsque l’utilisation `%g0` est dessinée. Il ne peut être que l’objet de TEXT ; partout ailleurs il échoue, comme `? @PLACE(…)` dans un symbole, qui n’a aucun sens.
  - `? @TEXT("${name}", key="name") => @PLACE(1.5, 1.5)` : le texte est attaché à l’objet dessiné avec le symbole, et `${…}` lit ses attributs. Ce PLACE est en coordonnées locales et se transforme avec l’instance (voir la direction des textes en 9.2.5). Sans PLACE, l’implémentation place le texte à côté de l’objet.
  - Un texte muni d’une clé est un sous-objet de l’objet dessiné avec le symbole, tel que `#M1::name`. Les textes dans le symbole viennent en premier, et un texte portant la même clé dans la scène le surcharge paramètre par paramètre : `#R1 @TEXT("R_{D}", key="name")` ne change que le contenu et garde la position issue du symbole ; `#M1::name @PLACE(13, 18)` ne change que la position et garde le contenu issu du symbole. Une position donnée dans la scène est en coordonnées de la scène et ne tourne plus avec l’instance.
  - Les clés qu’apporte un symbole n’existent qu’une fois les symboles attribués (8.2 étapes 4 et 7). Lorsque le symbole de M1 n’a aucun texte portant la clé `name` et que la scène n’en écrit aucun, `#M1::name` échoue. Pour être indépendant du symbole, écrivez un TEXT avec cette clé dans la scène.
  - Un texte attaché à un point dans le symbole est un texte fixe, tel que `(-2, 2) @TEXT("+")` dans un symbole d’amplificateur opérationnel, et ne peut pas être modifié pour une seule instance.
- **Règles** : une règle telle que `# @SELECT-INSTANCE => @SELECT-PIN(wired=false) => @TEXT(…)` sélectionne des broches, qui proviennent des symboles, donc elle est évaluée à l’étape 6 avec la génération un par un (8.2). Lorsqu’une implémentation ajoute d’elle-même un chevelu ou des étiquettes, elle saute les broches qui portent déjà une étiquette.
- Quels noms de nets sont affichés et où, et si les connexions sans fils sont affichées avec des étiquettes ou avec un chevelu, sont décidés par défaut par l’implémentation : ils font partie de son style maison (Principes 2.7). Afficher des étiquettes à la place de fils change le placement, si bien que le solveur et le moteur de rendu utilisent le même réglage. RNL n’écrit ceux-ci qu’en cas de besoin : pour énoncer une intention (sans positions), ou pour garder des positions déjà décidées.
- Il n’existe pas de positions le long d’un fil : les coordonnées d’un fil ne sont pas fixes elles-mêmes.
- **`fix_rotate`** : une valeur booléenne, `false` par défaut. Avec `true`, le `rotate` du texte est pris dans la scène : les caractères gardent cette direction, quelle que soit l’orientation de l’objet auquel le texte est attaché. Son ancre suit toujours l’objet, tout comme le côté de l’ancre sur lequel repose la boîte de texte. Un alignement indique où se situe la boîte par rapport à l’ancre : à droite avec `start`, à gauche avec `end`, au-dessus avec `bottom` et `baseline`, au-dessous avec `top`, et d’aucun côté avec `middle`. Cette direction est transformée avec l’orientation de l’objet, et le texte est aligné comme l’indique le résultat. Ainsi un nom écrit au-dessus d’un symbole, avec `halign="middle"` et `valign="bottom"`, se place à gauche de son ancre, aligné `end` et `middle`, lorsque le symbole est tourné de `"R90"`, et au-dessous, aligné `top`, pour `"MX"` ; il se lit horizontalement dans chaque orientation et reste à l’extérieur du symbole. Une boîte qui reste au-dessus de son ancre garde le `bottom` ou le `baseline` avec lequel elle a été écrite.
- **`upright`** : une valeur booléenne, `true` par défaut : le texte est tourné de 180 degrés lorsqu’il se lirait autrement à l’envers, et ses glyphes ne sont pas mis en miroir. Avec `false`, ses caractères suivent la rotation et la symétrie miroir telles quelles (9.2.5).
- Paramètres : `text` (requis ; peut être donné par position), `key`, `dx`, `dy`, `halign`, `valign`, `rotate`, `fix_rotate`, `upright`, et les propriétés de style de texte `textcolor`, `size`, `font`, `bold`, `italic`, `textvisible` (9.3) ; un texte attaché à un point n’a pas de `key`. Pour `halign`, `valign` et `rotate`, voir 9.2.5. Les paramètres de style constituent le style direct du texte (8.3) ; la façon dont un texte reçoit son style est en 11.3.

### 13.4 Annotations : graphiques dans la scène

```rnl
(50, 18) @TEXT("5T OTA testbench") => @SET-STYLE $note => %n1;
%f1 = @RECTANGLE {(0, 0) (40, 30)};
%n1 @PLACE(60, 18)                            /* moves the text; may also be written in another record */
```

- LINE, RECTANGLE, CIRCLE, ELLIPSE, ARC, BEZIER et TEXT peuvent être écrits directement dans la scène, et donc aussi dans le corps d’une définition générale. Dans un corps de symbole leurs coordonnées sont locales ; dans la scène ce sont des coordonnées de la scène. L’endroit où ils sont écrits en décide ; le mot-clé est identique.
- Les objets de note de Principes 2.2 (texte libre, graphiques, cadres et autres objets qui ne sont que dessinés, sans bornes et sans signification électrique) sont les annotations de la présente norme : les graphiques écrits dans la scène, les TEXT attachés à des points et les symboles dessinés par ANNOTATION (13.5).
- PIN ne peut être écrit que dans le corps de DEFINE-SYMBOL : une broche lie une borne de l’objet dessiné avec le symbole, et la scène n’a pas un tel objet.
- Un TEXT attaché à un point est une annotation (13.3) : il ne peut se référer qu’aux attributs `x` et `y` du point, et ne prend pas de `key=`, car un point n’a aucun sous-objet (avec une clé, le TEXT échoue). Pour afficher le nom ou la valeur d’un objet, attachez le TEXT à l’objet, ce qui en fait une étiquette.
- La position d’un graphique est donnée par ses points. Un point peut être n’importe quel objet point : un point nommé, un point libre avec contraintes, ou `#M1::position`, qui suit le composant. Dans le corps d’une définition générale, un point littéral est un décalage par rapport au point propre de l’utilisation (9.4). PLACE ne prend pas un graphique lui-même : pour déplacer, depuis un autre enregistrement, un RECTANGLE, CIRCLE ou ELLIPSE écrit avec un centre, déplacez son `::position` ; LINE, ARC, BEZIER et les graphiques écrits avec deux points ont plusieurs points, donc nommez les points et déplacez-les. TEXT fait exception : il a sa propre position, le point auquel il est attaché ne donne que sa place par défaut, et PLACE fixe directement la position du texte (13.3).
- Apparence : les graphiques peuvent prendre des paramètres de style directement (9.2.5) et prennent aussi SET-STYLE, avec les deux paliers de 8.3 ; une propriété que ni l’un ni l’autre ne donne prend la valeur par défaut du moteur de rendu.
- Un cadre est un RECTANGLE, ou un LINE dont le dernier point est le premier. Il n’existe aucun graphique qui entoure un groupe d’objets : dessinez un RECTANGLE et contraignez ses coins à se trouver à l’extérieur du groupe, tel que `%tl %stage1_set @DISTANCE("left-to-right", min=1)`, ce qui maintient le coin à au moins 1 à gauche de chaque membre (14.5) ; ce qui est entouré correspond aux points des membres plus la marge. Une visionneuse peut dessiner un GROUP sous forme de cadre ; cela dépend de la visionneuse (11.5). Il n’existe pas de `::box`.
- L’ordre de tracé suit les couches et les positions (11.6).
- **Classe** : le fait qu’un élément soit une annotation découle de sa sorte d’objet : les graphiques dans la scène sont des annotations, de sorte qu’un moteur de rendu peut les afficher ou les masquer en tant que classe. SELECT-ANNOTATION sélectionne tous les graphiques dans la scène et ce que dessine ANNOTATION (13.5), y compris ceux développés depuis les corps de définitions générales (tels que les cadres dessinés par un paquet de style maison) ; les graphiques dans les symboles, les étiquettes, les fils et les utilisations (points de jonction compris) ne sont pas des annotations. Des classes plus fines (explications, cartouches (title blocks), décorations dessinées par un paquet) sont des ensembles nommés, et un graphique peut appartenir à plusieurs ensembles. Afficher et masquer relève de la visionneuse ; le langage n’a pas de `visible`.
- Un texte sur un composant est un TEXT attaché au composant, ce qui est une étiquette (13.3). Un TEXT attaché à `#M1::position` est une annotation ; par défaut il suit aussi le composant, mais il ne peut se référer qu’aux coordonnées de ce point, et il n’a pas de clé.

### 13.5 Symboles dessinés comme annotations : ANNOTATION

```rnl
%a1 = (40, 20) @ANNOTATION(rotate=30) $ARROW_SYMBOL;     /* an arrow at any angle */
#M1 @ANNOTATION(rotate=-45, mirror=true) $ARROW_SYMBOL;   /* follows M1 */
%a1::position @PLACE(44, 20)                             /* moves the first one */
```

- ANNOTATION rassemble les graphiques de ce qui lui est donné et les dessine comme une seule annotation (13.4), tournée d’un angle quelconque. Deux emplacements : `at`, écrit en premier, un objet localisable : un point, ou un objet dont l’annotation suit la position ; et `source`, écrit en dernier, l’un de :
  - un symbole (DEFINE-SYMBOL) : les graphiques et les TEXT de son corps ;
  - une définition générale (DEFINE), avec ou sans rôles : les graphiques écrits directement dans son corps dont tous les points sont des points littéraux, qui sont des décalages par rapport au point d’une utilisation (9.4), et les TEXT attachés à de tels points ou à `?`. La définition n’est pas appliquée : aucune utilisation n’est créée, donc rien n’est dessiné deux fois ;
  - une utilisation : les graphiques de son symbole, lorsqu’elle en a un, une fois les symboles attribués (8.2), et les graphiques que son corps a dessinés avec des points littéraux uniquement, comme pour une définition, autour du point de l’utilisation. L’utilisation elle-même reste dessinée là où elle est.
- Tout le reste est laissé de côté : les PIN, `box=`, les contraintes, les utilisations créées dans un corps, et les graphiques d’un corps qui ont un point sur un autre objet, tel qu’un trait dessiné vers un rôle, qui n’aurait aucun sens une fois déplacé et tourné. Un graphique laissé de côté pour cette raison donne une indication ; le reste est laissé de côté sans indication, afin qu’un symbole muni de broches puisse être réutilisé pour un signe. Rien ne remplit un rôle ; l’annotation n’a pas de broches, ne peut pas être une extrémité d’un fil, et ne fait partie d’aucun arbre de 13.7.
- **Paramètres** : `rotate`, un angle en degrés, dans le sens antihoraire, un nombre quelconque, `0` par défaut ; `mirror`, une valeur booléenne, `false` par défaut, qui met le symbole en miroir par rapport à son axe vertical avant qu’il soit tourné ; et les propriétés de style, données à ses graphiques et textes comme les donne SET-STYLE (11.3). Il prend également, par nom, les paramètres d’un symbole ou d’une définition générale donné comme `source`, comme le fait AS (9.4), tel que `(0, 0) @ANNOTATION(rotate=30, w=5) $D` : un paramètre non donné prend sa valeur par défaut, et un paramètre sans valeur par défaut compte comme non écrit (9.4). Lorsqu’un paramètre de la source porte le nom d’un paramètre d’ANNOTATION lui-même, tel que `rotate` ou `mirror`, c’est le paramètre d’ANNOTATION qui est visé, et une indication est émise. Une utilisation donnée comme `source` garde les valeurs de ses propres paramètres, et un paramètre de sa définition écrit dans ANNOTATION est alors un paramètre invalide. Les textes du symbole tournent avec lui comme les textes d’un symbole tournent avec une instance, selon leurs propres `rotate`, `fix_rotate` et `upright` (9.2.5).
- Dans les textes du symbole, `?` est l’annotation, qui ne possède que les attributs `x` et `y`, comme un point (13.4) ; toute autre référence d’attribut à cet endroit est dessinée comme le texte vide.
- **Résultat** : une nouvelle annotation, dotée du sous-objet `::position`, son ancre. Recevant un point écrit, l’ancre commence là et peut être déplacée par son `::position`, comme un graphique écrit avec un centre (13.4) ; recevant un objet, elle suit la position de l’objet. SELECT-ANNOTATION la sélectionne, SET-STYLE lui donne un style, et `textvisible` masque ses textes.
- Ce qu’il est est connu là où il est écrit : tout autre objet qu’un symbole, une définition générale ou une utilisation dans l’emplacement `source` est une erreur ; une source sans aucun graphique à dessiner produit un avertissement et ne dessine rien.
- **Comparé au reste** : les graphiques écrits dans la scène (13.4) ont chacun leurs propres points ; une utilisation pourvue d’un symbole, telle qu’une figure du paquet standard, se dessine dans les huit orientations de 11.2, peut avoir des broches et peut être câblée. ANNOTATION dessine un symbole à n’importe quel angle et n’a aucune signification électrique.

### 13.6 Balisage de texte et références aux attributs

```rnl
#R1 @TEXT("R_{D}", key="name");                              /* subscript */
(50, 18) @TEXT("\textbf{5T OTA} testbench\\VDD = 1.8 V");    /* bold, two lines */
#M1 @TEXT("W/L=${w}/${l}", key="wl");                        /* host parameters */
$NMOS_M = @DEFINE-SYMBOL {
  $NMOS;
  ? @TEXT("${model}", key="model") => @PLACE(1.5, -1.5)      /* the model name, in a symbol */
}
```

- **Le balisage** est un sous-ensemble du mode texte de LaTeX, et non TeX complet :

  | Balisage | Signification |
  |---|---|
  | `_{…}`, `^{…}` | Indice, exposant |
  | `\textbf{…}`, `\textit{…}` | Gras, italique |
  | `\overline{…}` | Trait suscrit, tel que `\overline{RST}` actif à l’état bas |
  | `\subrest{…}` | Le premier caractère tel qu’écrit, le reste en indice : `\subrest{${name}}` affiche M1 comme M₁ et VDD comme V suivi de DD en indice |
  | `\subdigits{…}` | Les chiffres à la fin en indice : R12 comme R₁₂, Vout1 comme Vout₁ ; un contenu sans chiffres à la fin est affiché tel quel |
  | `\(…\)` | Mode mathématique (ci-dessous) |
  | `\frac{…}{…}` | Fraction : numérateur au-dessus, dénominateur au-dessous, une barre entre les deux, telle que `\frac{${w}}{${l}}` ; une visionneuse qui ne peut pas les empiler affiche « numérateur/dénominateur » |
  | `\alpha` … `\omega`, `\Gamma` … `\Omega` | Lettres grecques, avec les noms de LaTeX |
  | `\pm`, `\times`, `\cdot`, `\infty`, `\approx`, `\neq`, `\leq`, `\geq`, `\ll`, `\gg`, `\propto`, `\parallel`, `\angle`, `\circ`, `\to`, `\uparrow`, `\downarrow` | Symboles courants |
  | `\\` | Saut de ligne |
  | `\_`, `\{`, `\}`, `\$`, `\%`, `\&`, `\#`, `\textbackslash` | Les caractères `_`, `{`, `}`, `$`, `%`, `&`, `#`, `\` eux-mêmes |

- Lettres grecques : minuscules `\alpha` à `\omega` (LaTeX n’a pas `\omicron` ; écrivez o), ainsi que les variantes `\varepsilon`, `\vartheta`, `\varpi`, `\varrho`, `\varsigma`, `\varphi` ; majuscules uniquement les lettres qui diffèrent des lettres latines : `\Gamma`, `\Delta`, `\Theta`, `\Lambda`, `\Xi`, `\Pi`, `\Sigma`, `\Upsilon`, `\Phi`, `\Psi`, `\Omega`. Un symbole absent du tableau s’écrit directement comme caractère Unicode (les chaînes peuvent contenir des caractères non ASCII, 3.7).
- Le balisage peut être imbriqué, tel que `\textit{V}_{\textit{DD}}` et `I_{D_{1}}`. La manière dont les scripts de `V_{in}^{+}` sont composés (empilés ou l’un après l’autre) dépend de la visionneuse.
- **Les indices et exposants exigent des accolades** : un `_` ou `^` isolé est un caractère ordinaire. Cela diffère de LaTeX, où le m de `g_m` est mis en indice : les noms et les descriptions contiennent souvent `vin_1` ou `bias_ctrl`, que cette règle abîmerait. En contrepartie, `g_m` est affiché tel qu’écrit.
- Les autres caractères sont affichés tels qu’écrits, y compris un `{`, `}`, `$`, `~`, `%`, `&` ou `#` isolé, qui ont d’autres significations en LaTeX.
- **Barre oblique inversée** : `\\` est un saut de ligne, comme en LaTeX ; une barre oblique inversée elle-même s’écrit `\textbackslash`. Une barre oblique inversée qui n’est pas suivie d’une commande connue est affichée telle qu’écrite et produit un avertissement, comme `C:\temp` (`\temp` est une commande inconnue ; voir le point suivant) ; pour éviter l’avertissement, écrivez `C:\textbackslash{}temp`. `\textbackslash` n’est nécessaire que lorsque la barre oblique inversée est directement suivie de `\`, `{`, `}`, `_`, `^`, `$`, `%`, `&`, `#` ou du nom d’une commande connue. Les chaînes RNL n’ont pas de séquences d’échappement à la barre oblique inversée (3.7) : `"\textbf{V}"` s’écrit exactement ainsi, et non `\\textbf`.
- Une commande inconnue (`\` suivi de lettres qui ne figurent pas dans le tableau, telles que `\sqrt` ou `\alpah`) et des accolades non appariées sont affichées telles qu’écrites et produisent un avertissement ; le texte reste valide.
- **Après un nom de commande** : un nom de commande est la suite de lettres directement après `\`. Les espaces après le nom d’une commande connue sont écartées et non affichées, et un `{}` vide juste après ne sert qu’à séparer et n’est pas affiché non plus. Ainsi `10 \mu A` et `10 \mu{}A` affichent tous deux 10 μA ; pour garder l’espace, écrivez `\mu{} A` ; `\textbackslash{}temp` affiche \temp. Les espaces après les caractères échappés tels que `\_` et `\{` sont retenues.

  Note : la prise en charge d’une plus grande partie de TeX dans une version ultérieure ne fait qu’en autoriser davantage. Le mode mathématique s’écrit `\(…\)`, et non `$…$`, car `$` est utilisé par les références aux attributs.
- **Formats de noms** : `\subrest{…}` et `\subdigits{…}` ne changent que la façon dont leur contenu est affiché : les caractères affichés sont scindés en une partie normale et un indice. Le contenu peut être référencé, comme `\subrest{${name}}` ; il n’est toujours pas lu comme du balisage (ci-dessous). Ils peuvent être imbriqués dans un autre balisage, tel que `\textit{\subrest{${name}}}`.
- **Mode mathématique** : `\(…\)` contient une formule, dans un sous-ensemble du mode mathématique de LaTeX :
  - les lettres sont des variables, composées en italique ; les chiffres et les symboles du tableau sont droits, et les espaces ne sont pas affichées ;
  - `_` et `^` prennent un caractère ou un groupe entre accolades, comme en LaTeX, si bien que `g_m` est g avec un m en indice à l’intérieur de `\(…\)` ;
  - `\frac{…}{…}`, `\sqrt{…}`, les lettres grecques et les symboles courants du tableau, ainsi que les relations `=`, `<`, `>` ;
  - `\mathrm{…}` compose son contenu en romain droit, comme pour les unités et les noms, tel que `\(V_\mathrm{DD}\)` ;
  - une référence d’attribut à l’intérieur est affichée telle qu’écrite, droite.

  Une commande qui ne fait pas partie de ce sous-ensemble est affichée telle qu’écrite et produit un avertissement.
- **Fractions** : `\frac` prend deux groupes entre accolades directement après lui ; lorsqu’il en manque un, c’est un balisage inconnu, affiché tel qu’écrit avec un avertissement. Le numérateur et le dénominateur peuvent contenir du balisage et `${…}`.
- De combien les indices, exposants et fractions sont élevés ou abaissés et de combien ils sont réduits dépend de la visionneuse. La police, ainsi que le gras et l’italique pour tout le texte, proviennent du style ou des paramètres `font`, `bold`, `italic` (9.3).
- Le balisage est lu dans le `text` de TEXT et dans le `label` de PIN (tel que `\overline{CS}`). Les autres paramètres de type chaîne (`key`, valeurs d’énumération, conditions de sélection) ne lisent pas le balisage.
- Note : à l’importation et à l’exportation, `_{…}` et `^{…}` de KiCad sont identiques, et son `~{…}` est `\overline{…}` ; un trait suscrit qu’Altium écrit avec `\` après chaque caractère est réécrit sous la forme `\overline{…}`.
- **Références aux attributs** `${name}` affichent un attribut de l’objet auquel le texte est attaché. Pour écrire `${` lui-même, écrivez `\${` ; tout ce qui va de `\${` jusqu’à son `}` correspondant est affiché tel qu’écrit, et ce `}` ne compte pas comme une accolade non appariée. Les attributs sont :
  - Instances : `name` ; `value`, la valeur écrite sur la ligne hôte, telle qu’écrite, vide s’il n’y en a aucune (elle ne se rabat pas sur le nom du modèle) ; `model` et `type`, comme dans les conditions de sélection (10.1), si bien que `${model}` d’une instance X affiche le nom du sous-circuit ; et les paramètres hôtes, tels que `w`. Un paramètre hôte portant le nom d’un attribut standard s’écrit `${#…}` (voir ci-dessous).
  - Bornes, broches et fils : `net`, le net sur lequel ils se trouvent, affiché par son nom ; pour une broche, le net de `::net` (6.5), et une broche qui ne lie rien n’a pas de `net`. Sur le nœud vide (6.3), `net` est le texte vide, tout comme un rôle qui en est rempli, tel que `${p}` : rien n’est dessiné (13.3). Le `net` d’un fil de bus est formé des noms de ses nets dans l’ordre, joints par `,` et sans espaces (13.1). Pour une borne d’un faisceau, c’est le nom du net lorsque tous les membres sont sur le même net, et sinon les noms joints par `,` (11.8).
  - Nets : `name`.
  - Utilisations : les rôles et les paramètres de leur définition. `${p}` est l’objet qui remplit le rôle p, affiché selon les règles ci-dessous. Une utilisation n’a pas de nom propre ; les symboles des masses, alimentations et ports écrivent `${p}`, qui affiche le nom du net lorsque p est un net ; un rôle rempli par une borne ou une broche, tel que t de `$GROUND_STUB`, affiche aussi le nom de son net.
  - Faisceaux : un attribut qui est identique pour tous les membres a cette valeur ; sinon les valeurs sont jointes par `,` (11.8).
  - Points : `x`, `y`, les coordonnées finales, c’est-à-dire après résolution.
  - Graphiques : aucun attribut.
- **Chemins d’attributs** : dans `${a::b::c}`, le premier nom prend un attribut de l’objet attaché, et chaque `::name` suivant prend un attribut de la valeur qui le précède, tels que `${p::x}` (le x du rôle p lorsqu’il est un point) et `${pair::left::w}`.
  - La valeur d’un attribut peut être un objet (un rôle d’une utilisation ; le `net` d’une borne ou d’un fil) ou un texte (`name`, `value`, un paramètre hôte, etc.). Un texte n’a aucun attribut, donc aucun `::` ne peut le suivre.
  - Une valeur sur le chemin peut aussi être un ensemble, tel qu’un rôle d’ensemble. Le reste du chemin est alors pris sur chacun de ses membres, un ensemble parmi eux par ses propres membres, dans l’ordre de l’ensemble, et les valeurs sont affichées comme l’est un ensemble (ci-dessous) ; lorsqu’un membre manque de l’attribut, le texte échoue. Ainsi pour une utilisation dont le rôle d’ensemble `pairs` contient deux paires différentielles, `${pairs::left::w}` affiche `w` pour les deux transistors `left`, tel que `1u,2u`.
  - Dans `${…}`, `::` ne prend que des attributs, contrairement à `::` dans les instructions, qui prend des sous-objets (6.5) : `${m::name}` est le nom de l’instance m, tandis que `#M1::name` dans une instruction est le texte portant la clé `name` sur M1 (13.3).
  - `${m::#type}` ne cherche qu’un paramètre hôte, comme le fait `${#type}` (voir ci-dessous).
- **Affichage** : lorsque la dernière valeur du chemin est un texte, il est affiché tel qu’écrit. Lorsqu’elle est un objet : les instances et les nets affichent leurs noms ; les bornes, broches et fils affichent le nom de leur net, et un fil de bus les noms de ses nets joints par `,` ; les faisceaux comme décrit ci-dessus ; un ensemble (tel qu’un rôle d’ensemble) affiche ses membres, chacun de cette façon, joints par `,`. Les points, utilisations et graphiques ne peuvent pas être affichés : le texte échoue. Ainsi, lorsque le rôle p est un point, écrivez `${p::x}`, et non `${p}`.
- Les références donnent le texte tel qu’écrit dans la netlist, sans l’évaluer : `w=1u` affiche 1u. Ce que fournit chaque dialecte est défini par son adaptateur hôte ; dans Spectre, par exemple, la valeur d’une résistance est le paramètre `r`.
- Les attributs énumérés ci-dessus existent toujours ; un attribut qui n’a aucune valeur est vide. Une référence à un nom d’attribut qui n’existe pas fait échouer le texte.
- Le contenu référencé est affiché tel qu’écrit et n’est pas lu comme du balisage : les noms et valeurs de l’hôte ne sont pas écrits dans ce balisage.
- Les références ne descendent qu’à partir des attributs de l’objet attaché ; elles ne peuvent pas atteindre un objet arbitraire. Il n’existe aucune référence vers un autre objet telle que `${R3:VALUE}` de KiCad.
- **Alias** : les noms d’attributs dans `${…}` acceptent aussi les alias déclarés pour le fichier, recherchés d’abord comme attribut standard, puis comme alias, puis comme paramètre hôte ; `${#name}` ne cherche qu’un paramètre hôte (16.5). Les noms de rôles et de paramètres d’une utilisation sont des noms RNL : ils n’acceptent que les alias `definition=` de sa définition, recherchés à chaque étape dans la définition de l’utilisation de cette étape, tels que `${输入对::左管::w}` (16.5).
- **Paramètres hôtes portant des noms standards** : un attribut standard passe en premier, et un paramètre hôte portant le même nom s’écrit `${#type}` (`#` représente l’hôte en RNL). Tout paramètre hôte peut s’écrire de cette façon ; `${w}` est un raccourci pour `${#w}` là où aucun attribut standard ni aucun alias ne porte ce nom. Les noms d’attributs standards sont comparés exactement tels qu’écrits, les noms de paramètres hôtes selon les règles de l’hôte : dans un hôte SPICE, `${Type}` n’est pas un attribut standard, c’est donc le paramètre hôte type.
- **Mise en forme des noms** : le contenu référencé n’est mis en forme que dans son ensemble, avec du balisage autour, tel que `\textit{${name}}`, et par les deux commandes `\subrest{…}` et `\subdigits{…}`, qui affichent le reste d’un nom, ou ses chiffres à la fin, en indice. D’autres mises en forme de caractères isolés s’écrivent en texte littéral, telles que `#M1 @TEXT("M_{1}", key="name")`, qui ne suit pas un renommage ; toute convention supplémentaire d’une visionneuse relève de son style maison.

### 13.7 Dessiner les connexions sous forme de fils : ROUTE-NET, ROUTE-OBJECT, UNROUTE-NET, UNROUTE-OBJECT

```rnl
&x @ROUTE-NET;                                /* x is drawn as wires; the solver decides the branches and paths */
%jo = (_, _);
&out @WIRE {#M2:1 %jo};
&out @ROUTE-NET;                              /* this wire is drawn as written; the solver adds the rest */
{#M1 #M2 #M5} @ROUTE-OBJECT;                  /* on each net that two or more of their pins share, those pins are joined */
&vdd @UNROUTE-NET(priority="high");           /* the pins of vdd are not joined, whatever asks for it at a lower priority */
# @SELECT-NET(name="clk*", recursive=true) => @ROUTE-NET   /* as a house-style package writes it, with nets chosen by a rule */
```

- Ces mots-clés indiquent quelles connexions sont dessinées sous forme de fils, sans dire où ils s’embranchent ni où vont les points de jonction ; le solveur en décide.
- **ROUTE-NET** relie les broches d’un net en un arbre : les broches des composants sur le net, et les broches de composants virtuels dont `::net` est ce net, c’est-à-dire les broches liées aux rôles qu’il remplit (12.1). Un seul emplacement, qui prend des nets ; un ensemble est pris membre par membre (5.2.1), et n’est pas un bus.
- **ROUTE-OBJECT** relie les broches de certains objets : pour chaque net sur lequel se trouvent deux de leurs broches ou plus, ces broches sont reliées en un arbre. Un seul emplacement, qui prend un ensemble d’objets ; il transmet son entrée. Une instance ou un faisceau représente ses broches, une borne sa broche, et une utilisation les objets qui remplissent ses rôles, jusqu’aux objets les plus bas, comme pour les membres d’un groupe (11.5) ; une utilisation pourvue d’un symbole représente aussi ses propres broches, chacune sur le net du rôle qu’elle lie (12.1). Les objets sans broches, tels qu’un net ou un point qui remplit un rôle, n’en ajoutent aucune : un net entier se dessine avec ROUTE-NET. Ainsi une définition du paquet standard peut dessiner les connexions à l’intérieur de sa structure, telles que `{?left ?right ?tail} @ROUTE-OBJECT(priority="low")`, sans relier le reste des nets.
- **UNROUTE-NET** et **UNROUTE-OBJECT** retirent des broches : les broches d’un net, ou de certains objets (comptées comme pour ROUTE-OBJECT), ne sont pas reliées par les arbres de ROUTE-NET et ROUTE-OBJECT.
- **Chaque broche et chaque arbre séparément** : une broche couverte par un ROUTE-NET ou un ROUTE-OBJECT appartient à l’arbre de cette instruction sauf si un UNROUTE-NET ou un UNROUTE-OBJECT qui couvre la broche l’emporte sur cette instruction : il a une priorité supérieure (14.8), ou la même priorité et est écrit plus tard, dans l’ordre des positions de 8.5. La comparaison s’effectue avec chaque instruction ROUTE séparément, si bien qu’une broche peut être retirée de l’arbre d’un ROUTE-OBJECT de priorité `"low"` par un UNROUTE de priorité `"normal"` tout en restant dans l’arbre d’un ROUTE-NET de priorité `"high"`. Une instruction ayant `"disabled"` est écartée. Ces instructions ne sont pas des faits géométriques et ne prennent aucune part aux conflits (8.5) ; elles décident uniquement quelles broches relie le solveur.
- **Quand** : les instructions de ces mots-clés prennent effet à partir de l’étape 3 de 8.2. Leurs règles, telles que la dernière ligne de l’exemple, sont évaluées à l’étape 5, avant la génération : leurs conditions lisent les arbres des instructions, et la génération, telle que la complétion du paquet standard, voit les arbres que créent les règles (10.2, 10.4).
- **Les fils écrits ne sont pas affectés** : un WIRE explicite est dessiné tel qu’écrit, y compris sur un net ou à une broche retirée par UNROUTE. Les fils déjà écrits font partie des arbres qu’ils touchent, et le solveur ajoute le reste.
- **Arbres** : chaque ROUTE-NET crée un arbre sur son net, et chaque ROUTE-OBJECT un arbre sur chaque net qu’il relie. Les arbres sur un même net qui partagent une broche, ou que des fils écrits relient, forment un seul arbre ; il prend le style de l’instruction venant en premier dans l’ordre des positions, et une indication est émise lorsque leurs styles diffèrent. Un net peut avoir plusieurs arbres que rien ne relie, tels que deux ROUTE-OBJECT sur des composants éloignés.
- Seules des broches sont reliées : les bornes sans broche n’appartiennent à aucun arbre (9.2.4). Une broche sur le nœud vide ne se trouve sur aucun net (6.3), donc ROUTE-OBJECT ne la relie pas ; elle n’est connectée que par un fil écrit sur le nœud vide (13.1). Un arbre ayant moins de deux points à connecter ne fait rien. Qu’une broche soit câblée (10.2) se décide avant la génération, d’après les arbres sans les composants virtuels qu’ajoute la génération : un net qui n’a qu’une broche à lui, comme une entrée qui attaque une seule grille, forme un arbre d’un seul point ; cette broche n’est donc pas câblée, et la complétion lui donne ce qu’elle donne à une broche sans fil, même lorsque le net est routé vers le port de son niveau. Pour router un tel net vers son port, écrivez l’étiquette de port dans le fichier, comme `$IO_PORT @ATTACH &x` ou `$IO_PORT_STUB` du paquet standard (12.4, 16.3) : une étiquette de port qu’ajoute la complétion ne compte pas.
- **Paramètres** : ROUTE-NET et ROUTE-OBJECT prennent `mode` (une indication pour le solveur, avec les valeurs de WIRE, par défaut `"auto"`), les paramètres de style des fils (13.1) et `priority` ; UNROUTE-NET et UNROUTE-OBJECT prennent `priority`.
- Note : un WIRE avec une liste vide de points ne signifie pas ROUTE-NET. Une liste de points peut provenir d’un ensemble qui se trouve être vide, et une liste vide connecterait alors tout silencieusement ; ce sont donc des mots-clés à part entière ; la présente version ne comporte pas non plus d’emplacements optionnels (5.3).
- **Réécrit par le solveur** : les WIRE concrets (avec leurs coudes et `mode="straight"`), les points nommés où ils s’embranchent, et les points de jonction, lorsque `junction` est fixé (13.2), avec les paramètres de style de l’arbre, vont dans le propre enregistrement du solveur. Les instructions ROUTE-NET et ROUTE-OBJECT représentent l’intention de l’utilisateur et demeurent ; la résolution suivante les calcule à nouveau (chapitre 17). Pour embrancher depuis le milieu d’un fil écrit par l’auteur, le solveur scinde ce fil (13.1).
- **Les visionneuses qui ne résolvent pas** dessinent un chevelu ou rien, selon ce que décide la visionneuse (Principes 2.7).
- Un paquet de style maison choisit quels nets sont dessinés sous forme de fils à l’aide d’ensembles et de règles, comme dans l’exemple ; il n’y a aucun mot-clé pour la manière dont un net est affiché.

<!-- translated from en/ch14.md sha256:76f571ca1b76 -->
## 14 Intention de placement

### 14.1 Règles communes

- Une contrainte crée un objet contrainte. Une contrainte nommée peut être référencée, ce qui aide à la retrouver et à la modifier, tel que `%row1 = {#M1 #M2} @ALIGN("y")` ; une contrainte peut aussi être anonyme.
- **Vocabulaire** : les contraintes atomiques sur les positions sont uniquement ALIGN, ORDER, DISTRIBUTE et DISTANCE, outre PLACE, ORIENT, GROUP et TABLE (14.11), qui sont des ordres écrits sous forme de table. Les autres relations s’écrivent comme des définitions générales : symétrie (14.7), centrage (un DISTRIBUTE à trois membres), lignes et colonnes (ALIGN avec ORDER), flux de signal (14.6), paires en miroir, espacements en proportion (plusieurs DISTANCE exacts). Il n’existe ni LEFT ni ABOVE (ce sont un ORDER ou un DISTANCE avec une direction), ni RATIO, NEAR ou INSIDE. L’orientation relative (même orientation, images en miroir) est ORIENT-RELATION (14.9) ; le fait que les grilles d’une paire se fassent face ou regardent vers l’extérieur reste fixé par un ORIENT, ou par un ORDER sur les bornes (14.3). Le non-chevauchement des symboles, les constantes d’espacement et les grilles appartiennent au solveur (11.7) et ne font pas partie du vocabulaire des contraintes.
- **Aucune position relative à l’ensemble du dessin** : il n’existe aucun mot-clé pour « le plus à gauche », « le plus à droite » ou « le plus haut », ni aucune étendue du dessin entier ou des contours des symboles. Écrivez un ORDER ou un DISTANCE par rapport à un groupe d’objets, et le solveur en déduit le placement. Pour un port tout à droite : `{#M6 #M7 #CL} %pout @DISTANCE("left-to-right", min=2)`, ou `{#M7 %pout} @ORDER("left-to-right")` lorsque M7 est déjà le plus à droite.
- **Les membres sont les objets les plus bas** : les instances, les faisceaux, les bornes et les broches, les points, TEXT (par sa propre position, 13.3), le centre d’un RECTANGLE, CIRCLE ou ELLIPSE écrit avec un centre, et les utilisations dessinées avec un symbole, chacune par son point propre. Une utilisation sans symbole n’est pas elle-même un membre mais représente ses membres : les objets qui remplissent ses rôles, tout membre qui est une utilisation sans symbole étant suivi vers l’intérieur jusqu’aux objets les plus bas, comme pour GROUP (11.5), les nets parmi eux étant omis. Son point propre s’écrit `%u::position` (6.5). Les fils, nets, contraintes et définitions ne peuvent pas être membres : si elle en reçoit un, la contrainte échoue. Une position sur un fil doit d’abord être écrite comme un point (13.1). Un ensemble n’est pas un membre : il n’a aucune position propre (ci-dessous).
- **Quel point compte** : une instance compte par son origine, l’origine de son symbole, qui est aussi ce que place PLACE ; une borne ou une broche par la position de la broche, qui est la position du composant augmentée de la transformation rigide de la position de la broche dans le symbole et ne peut pas être déplacée seule ; un point par lui-même ; une utilisation avec symbole, ou `::position` de toute utilisation, par son point propre (ci-dessous) ; un TEXT par sa propre position ; un RECTANGLE, CIRCLE ou ELLIPSE écrit avec un centre par son centre. Le centre du cadre d’un symbole n’est pas utilisé : les cadres des symboles MOS à trois bornes et à quatre bornes diffèrent, et une ligne composée des deux ne serait pas alignée. Pour aligner par les broches, écrivez les bornes. La symétrie visuelle exige que l’origine se trouve sur l’axe de symétrie du symbole, ce qui relève de la bibliothèque de symboles.
- **Le point propre d’une utilisation** : chaque utilisation possède un point propre, y compris une utilisation sans symbole. Le point commence libre, comme `(_, _)`, et est lié par des faits : PLACE, contraintes, fils vers les broches de son symbole, et les faits que le corps de sa définition écrit sur lui ; le solveur place ce qui reste libre. Une utilisation pourvue d’un symbole est dessinée avec l’origine de son symbole à ce point (12.1), et une contrainte sur elle agit sur ce point. Une contrainte sur une utilisation sans symbole agit sur ses membres (ci-dessus) ; pour contraindre son point, écrivez `%u::position` : `{%dp::position #X1} @ALIGN("y")` place X1 à la hauteur du centre de la paire, là où `{%dp #X1} @ALIGN("y")` placerait X1 et les trois transistors de la paire à une même hauteur. Le corps de sa définition lie le point à ses membres là où il écrit des faits sur `?` seul (9.4), tel que `{?members::position ?} @ALIGN("y")`. Lorsque le point d’une utilisation n’est lié par aucun fait à quoi que ce soit de dessiné, une contrainte sur `%u::position` n’a aucun effet visible, et une indication le signale.
- **Les ensembles n’ont pas de position** : un ensemble n’est qu’une liste de membres, sans centre et sans étendue. Dans la liste des membres d’ALIGN, ORDER ou DISTRIBUTE, un élément qui est un ensemble, un `{…}` écrit explicitement ou une référence à un ensemble (5.2), représente chacun de ses membres les plus internes à cet élément : l’instruction équivaut à une contrainte pour chaque combinaison qui prend un membre dans chacun de ces éléments. `{#M1 {#M2 #M3} #M4} @ORDER("top-to-bottom")` vaut `{#M1 #M2 #M4}` et `{#M1 #M3 #M4}`, si bien que M2 et M3 sont chacun entre M1 et M4 et ne sont pas ordonnés entre eux. Un élément qui est une utilisation sans symbole représente ses membres (ci-dessus) de la même manière. Un ensemble dans un emplacement de DISTANCE représente chacun de ses membres, une contrainte pour chaque paire (14.5). Un élément qui est un ensemble vide est omis. Toute combinaison qui prend deux fois un même objet est omise : deux structures partageant un composant, ordonnées par `{%stage1 %stage2} @ORDER("left-to-right")`, ordonnent chaque autre paire de leurs membres sans ordonner ce composant par rapport à lui-même.
  - Les contraintes ainsi créées ont la priorité, le nom et la position de l’instruction écrite (8.5, 14.8) ; un nom lie leur ensemble. Une combinaison ayant trop peu de membres pour dire quoi que ce soit produit une indication.
  - Une implémentation peut limiter le nombre de contraintes qu’une même instruction représente ; au-delà de sa limite l’instruction échoue, avec une erreur indiquant qu’il s’agit d’une limite de l’implémentation (1.6).
  - Un ensemble qui est le seul objet est d’abord scindé d’un niveau (5.2.1) : `%row_set @ORDER("left-to-right")` et `{%row_set} @ORDER("left-to-right")` ordonnent tous deux les membres de l’ensemble.
- **Les directions sont écrites explicitement.** ALIGN prend une projection, `"x"` ou `"y"` (14.2) ; ORDER, DISTRIBUTE et DISTANCE prennent une direction, l’une des quatre (14.3). Aucune forme ne laisse la direction ouverte : une relation qui peut être dessinée dans un sens ou dans l’autre s’écrit comme un fait pour un sens, et une définition qui doit permettre les deux dessins se présente comme deux définitions, telles que `$BRANCH` et `$BRANCH_HORIZONTAL` du paquet standard. Les seuls faits qui offrent un choix sont les listes d’orientations d’ORIENT (11.2) et d’ORIENT-RELATION (14.9).
- Les contraintes ne modifient pas la netlist. Dans un conflit, une contrainte atomique est retenue en bloc ou écartée en bloc (8.5).
- Les quatre contraintes atomiques, ORIENT-RELATION et TABLE prennent le paramètre `priority=` (14.8).

### 14.2 ALIGN

`{…} @ALIGN("y")` : les membres sont égaux selon la projection `axis` : `"y"` leur donne le même y (une ligne), `"x"` le même x (une colonne). `axis` est requis, peut être donné par position, et est une seule projection : une liste de projections est une mauvaise valeur de paramètre, et la contrainte échoue. Avec moins de deux membres, la contrainte est satisfaite trivialement et n’échoue pas : le nombre de membres d’une sélection ou d’un ensemble n’est pas connu lorsque l’instruction est écrite.

### 14.3 ORDER

```rnl
%rc_set = {#R2 #C2};
{#M1 #M1:2 #M2:2 #M2} @ORDER("left-to-right");  /* M1 on the left, the two gates face each other */
{#R1 {%rc_set} #M1} @ORDER("left-to-right")     /* R2 and C2 each between R1 and M1 */
```

- `{…} @ORDER(direction)` : les membres sont strictement monotones selon la projection de la direction. Avec moins de deux membres, elle est satisfaite trivialement.
- **Direction** : le paramètre `direction`, requis, donné par position en premier. Ses valeurs sont :
  - `"left-to-right"`, `"right-to-left"` : x croissant, x décroissant ;
  - `"bottom-to-top"`, `"top-to-bottom"` : y croissant, y décroissant (y va vers le haut, 11.7).

  DISTRIBUTE et DISTANCE prennent les mêmes valeurs. Il n’existe aucune valeur qui ne fixe que l’axe : `"x"` et `"y"` sont de mauvaises valeurs de paramètre, et la contrainte échoue.
- Un élément qui est un ensemble représente chacun de ses membres (14.1) : le second exemple ordonne R1, R2, M1 et R1, C2, M1 ; R2 et C2 ne sont pas ordonnés entre eux.
- ORDER est un fait sur les points : il compare les points des membres, et non leurs cadres. Tenir les cadres des symboles séparés le long d’un ORDER est un critère du solveur, classé après chaque fait (8.5) ; lorsque des faits forcent les cadres à se chevaucher, ils se chevauchent, et une indication peut le signaler.
- ORDER est strict : des membres voisins diffèrent d’au moins une unité de coordonnée (11.7). C’est un raccourci pour un DISTANCE dans sa direction avec `min=1` entre chaque paire de membres voisins (14.5). Pour des coordonnées égales, utilisez ALIGN.

### 14.4 DISTRIBUTE

```rnl
{#R1 #R2 #R3} @DISTRIBUTE("left-to-right")     /* R1, R2 and R3 from left to right, in equal steps */
```

- `{…} @DISTRIBUTE(direction)` : les membres se suivent strictement dans la direction, comme pour ORDER (14.3), et les différences entre membres voisins selon sa projection sont égales. Les pas se mesurent entre les points des membres, non comme des intervalles entre cadres ; un élément qui est un ensemble représente chacun de ses membres (14.1).
- **Direction** : le paramètre `direction`, requis, donné par position en premier, avec les valeurs d’ORDER (14.3).
- Avec deux membres, DISTRIBUTE équivaut à ORDER ; avec moins de deux, il est satisfait trivialement.

### 14.5 DISTANCE

```rnl
#M1:2 #M1 @DISTANCE("left-to-right", min=1);      /* the gate is on the left */
#R1 #R2 @DISTANCE("left-to-right", exact=8)       /* R2 exactly 8 units to the right of R1 */
```

```rnl
%stage1_set = {#M1 #M2 #M5};
%stage2_set = {#M3 #M4};
%stage1_set %stage2_set @DISTANCE("left-to-right", min=1)     /* all of stage 2 is to the right of all of stage 1 */
```

- `A B @DISTANCE(direction, min=…, max=…)` : A remplit l’emplacement `from` et B l’emplacement `to` ; chacun est un objet ou un ensemble.
- **Direction** : le paramètre `direction`, requis, donné par position en premier, avec les valeurs d’ORDER (14.3). Seul cet axe est contraint ; ce n’est pas la distance euclidienne.
  - Une direction croissante (`"left-to-right"`, `"bottom-to-top"`) contraint B − A ;
  - une direction décroissante (`"right-to-left"`, `"top-to-bottom"`) contraint A − B.
- **Ensembles** : un ensemble dans un emplacement représente chacun de ses membres (14.1), tout comme une utilisation sans symbole, et le DISTANCE s’applique de chaque membre de A à chaque membre de B, une contrainte pour chaque paire ; toute paire d’un objet avec lui-même est omise. Ainsi `%stage1_set %stage2_set @DISTANCE("left-to-right", min=1)` place tout l’étage 2 à droite de tout l’étage 1, et c’est ainsi que s’écrivent deux ensembles qui ne doivent pas s’entrelacer. Lorsqu’un côté est un ensemble vide, il n’y a aucune paire et rien ne se passe.
- Au moins l’un de `min`, `max` et `exact` doit être écrit ; `min` ne doit pas être supérieur à `max`. Un DISTANCE énonce aussi l’ordre : B n’est jamais avant A dans la direction. Les bornes ne sont donc pas négatives, et un `min`, `max` ou `exact` négatif est une mauvaise valeur de paramètre, pour laquelle la contrainte échoue ; un `min` non écrit vaut 0, donc `max=10` est satisfait pour une distance de 0 à 10.
- `exact=` est une distance exacte, raccourci pour `min` et `max` égaux : B − A = exact pour une direction croissante, A − B = exact pour une direction décroissante. Il ne peut pas être écrit en même temps que `min` ou `max` : c’est un paramètre invalide, et l’instruction échoue. Les outils écrivent les distances exactes avec `exact` (4.3).

### 14.6 Flux de signal

Le flux de signal (Principes 2.4) s’écrit comme l’utilisation d’une définition générale ; il n’y a aucun mot-clé pour lui. Le fait que le paquet standard fournisse une telle définition, et sous quel nom, dépend du paquet standard (16.3) ; les exemples utilisent `$SIGNAL_FLOW` de la série standard 0.4, qui nomme un chemin, donné sous forme d’ensemble, et peut en nommer les sources en amont et les charges en aval. C’est une définition sémantique : elle ne place rien.

```rnl
%rc_set = {#R2 #C2} @GROUP;
{#R1 {%rc_set} #M1} @AS $SIGNAL_FLOW                                    /* the signal goes from R1 through the block of R2 and C2 to M1 */
{#:inp #:inn}(source) {#R1 {%rc_set} #M1}(path) #CL(load) @AS $SIGNAL_FLOW   /* from the two inputs of the level to the load CL */
```

- Le flux de signal est une notion souple : tout circuit n’a pas une direction stricte. Un solveur peut le lire comme une préférence ; pour mettre les étages dans l’ordre sur le dessin, écrivez ORDER ou TABLE (14.3, 14.11).
- Un étage du chemin qui est composé de plusieurs objets s’écrit comme un `{…}` propre, tel que `{%rc_set}`. Un GROUP (11.5) fait seulement en sorte que le solveur le traite comme un seul bloc, et peut être omis.
- Les sources et les charges forment chacune un groupe sans ordre, tel que les deux entrées d’un signal différentiel. Il peut s’agir de composants, de structures, de ports du niveau tels que `#:inp` (6.4), d’autres bornes ou de nets. Sans rôles indiqués, les objets constituent le chemin, comme à la première ligne.
- Le flux de puissance, depuis une alimentation à travers les étages qui la convertissent ou la commutent jusqu’aux parties qui l’utilisent, s’écrit de la même manière avec `$POWER_FLOW`.

### 14.7 Symétrie

La symétrie s’écrit comme l’utilisation d’une définition générale. L’axe de symétrie n’est qu’un point libre, et aucun mot-clé n’est nécessaire. La série standard 0.4 fournit `$MIRRORED_LAYOUT`, avec les rôles `left`, `right` et `axis` : gauche et droite à la même hauteur, l’axe entre eux au milieu, et droite orientée comme gauche retournée gauche-droite ; et `$MIRRORED_VERTICAL_LAYOUT`, avec `top`, `bottom` et `axis`, la même chose tournée verticalement. Le point propre de l’une ou l’autre utilisation se trouve au centre entre les deux côtés (14.1), de sorte qu’une contrainte sur l’utilisation déplace la paire. Comme dans toute définition du paquet standard, ces contraintes sont de priorité `"low"` (9.5). L’axe est un seul objet : lorsque plusieurs composants se trouvent sur l’axe, ils sont d’abord réunis en une utilisation, qui compte par son point propre (14.1), ou bien l’un d’eux est donné.

```rnl
%ax = (_, _);
%sym12 = #M1(left) #M2(right) %ax(axis) @AS $MIRRORED_LAYOUT
```

- `$MIRRORED_LAYOUT` ne fixe pas l’orientation réelle : le fait que les grilles soient tournées vers l’intérieur ou vers l’extérieur nécessite toujours un ORIENT d’un côté (14.9). Pour une paire symétrique ayant la même orientation des deux côtés, écrivez une autre définition générale, sans son ORIENT-RELATION.

### 14.8 Priorité : PRIORITY

```rnl id=dpl
$DP_LAYOUT = @DEFINE(roles=["in_p", "in_n"]) {
  {?in_p ?in_n} @ALIGN("y", priority="low")                    /* a default layout, the first to give way */
};
%dp2 = #M1(in_p) #M2(in_n) @AS(priority="high") $DP_LAYOUT;     /* this pair keeps the default layout strictly */
#M1 @PLACE(12, 20, priority="high")                             /* fixes the position of M1 */
```

```rnl use=dpl
# @SELECT-USAGE(recursive=true) $DP_LAYOUT => @PRIORITY("disabled")    /* a house-style package turns the default layout off, to apply its own */
```

- **Quatre valeurs** : `"disabled"`, `"low"`, `"normal"`, `"high"`. Un fait sans priorité est `"normal"`, qu’il soit écrit dans un enregistrement de l’utilisateur, dans un paquet ou dans le corps d’une définition.
- **Classées, non pondérées** : en cas de conflit, les priorités sont comparées en premier, et la plus élevée est retenue ; à priorités égales, celle qui est écrite plus tard est retenue (8.5). Les contraintes ne sont pas additionnées avec des poids.

  Note : avec des poids, dix contraintes de priorité basse pourraient l’emporter sur une contrainte de priorité haute, et les solveurs diffèrent dans la façon dont ils mesurent à quel point une contrainte est violée.
- **Tout fait retenu est strict** : `"low"` signifie « écarté en premier en cas de conflit », et non « aussi proche que possible ». Le solveur doit satisfaire tout fait qui est retenu, quelle que soit sa priorité ; les propres objectifs de l’implémentation pour un dessin agréable ne sont optimisés qu’à l’intérieur des faits retenus, après toutes les descriptions du langage.
- **`"disabled"`** : le fait est retiré avant le classement et n’est pas transmis au solveur. Ce n’est pas un échec : l’objet existe toujours, les descriptions qui en dépendent ne sont pas affectées, aucun avertissement n’est émis, et une priorité écrite plus tard peut le réactiver.
- **Comment elle s’écrit** :
  - PLACE, ORIENT, les quatre contraintes atomiques, ORIENT-RELATION, TABLE, ROUTE-NET, ROUTE-OBJECT, UNROUTE-NET, UNROUTE-OBJECT, AS et ATTACH prennent le paramètre `priority=`, et la priorité accompagne le fait. `priority` est le seul paramètre standard d’AS et d’ATTACH ; un paramètre d’une définition ne peut pas porter ce nom.
  - L’instruction PRIORITY modifie une priorité par la suite. Elle ne prend que des objets dotés de leur propre poignée : contraintes, utilisations, et ensembles pourvus de GROUP (activé et désactivé uniquement, 11.5). Un ensemble pourvu de GROUP est pris comme le groupe lui-même ; tout autre ensemble est pris membre par membre (5.2.1). Recevant une instance ou un autre objet, elle échoue (mauvaise sorte) : `#M1 @PLACE(…) => @PRIORITY(…)` est une erreur, car PLACE transmet la poignée de M1 ; écrivez la priorité comme paramètre de PLACE.
  - L’en-tête d’enregistrement `@RNL(priority=…)` donne une valeur par défaut aux instructions de l’enregistrement qui prennent ce paramètre et ne l’écrivent pas, ainsi qu’aux `dx` et `dy` de TEXT (13.3). À AS et ATTACH il ne donne que `"disabled"`, afin qu’un enregistrement désactivé désactive ses utilisations avec leurs corps, comme ses autres instructions. Sous un en-tête portant `"high"`, `"normal"` ou `"low"`, une utilisation n’a de priorité que lorsque sa propre instruction ou une instruction PRIORITY lui en donne une, et sinon garde les priorités de son corps, comme lorsqu’une règle la crée, afin qu’une utilisation que le solveur écrit explicitement (10.5) signifie ce que signifiait celle générée. Une priorité qu’écrit une utilisation surcharge tout de même son corps (ci-dessous).
- **Ordre de surcharge**, du plus bas au plus haut : la valeur par défaut de l’en-tête d’enregistrement ; le propre paramètre de l’instruction ; les instructions PRIORITY (entre plusieurs, la plus tardive l’emporte ; pour celles des paquets, selon la position de l’USE-PACKAGE) ; la priorité d’une utilisation englobante ; les instructions PRIORITY écrites depuis l’extérieur d’une utilisation, via `::`, directement sur l’un de ses objets locaux (entre plusieurs, la plus tardive l’emporte). Lorsqu’aucun de ceux-ci n’est donné, la priorité est `"normal"`.
- **La priorité d’une utilisation descend** : la priorité d’une utilisation (son propre paramètre, ou une instruction PRIORITY sur elle) s’applique à tous les faits qu’elle développe, y compris ceux des utilisations imbriquées, et surcharge leurs propres priorités ; en cas d’imbrication, l’utilisation la plus externe qui possède une priorité décide. Pour n’ajuster qu’une partie intérieure, écrivez directement sur cet objet local depuis l’extérieur, via `::` (9.4) : cela est plus spécifique que la priorité de l’utilisation et la surcharge : la plus spécifique l’emporte. Une telle instruction écrite dans le corps d’une définition englobante reste soumise à la priorité de l’utilisation englobante. Chaque instruction d’un enregistrement peut être modifiée directement, si bien que l’en-tête d’enregistrement ne donne qu’une valeur par défaut.
  - En entier : la priorité d’une utilisation gouverne toute instruction PRIORITY écrite dans le corps de sa définition, qui n’a alors aucun effet. Un PRIORITY écrit depuis l’extérieur via `::` directement sur une utilisation locale gouverne de la même manière : cette utilisation compte comme la plus externe pourvue d’une priorité, et l’instruction décide pour tout ce que développe cette utilisation. Parmi les instructions que rien ne gouverne, celle écrite sur l’objet le plus interne l’emporte ; sur le même objet, celle écrite à l’extérieur du corps qui crée l’objet l’emporte sur celle écrite dans ce corps ; puis la plus tardive l’emporte.
  - Par exemple, avec `%o1` désactivé, `%o1::in @PRIORITY("high")` au sommet et `%in::row @PRIORITY("low")` dans le corps de la définition de `%o1`, les faits de `row` sont `"high"` : l’instruction dans le corps est gouvernée par la priorité de `%o1`, que l’instruction au sommet surcharge pour `in`. Si `%o1` n’avait aucune priorité, ils seraient `"low"`, écrits sur l’objet le plus interne. Pour avoir `"low"` dans les deux cas, écrivez `%o1::in::row @PRIORITY("low")` depuis l’extérieur.
- **La sortie du solveur** : le solveur écrit ses résultats sous forme de PLACE, ORIENT et ainsi de suite, avec `"high"`, dans le dernier enregistrement du texte propre du niveau, tel que `@RNL(type="generated", priority="high") {…}`. Pour un sous-circuit, cela se trouve à l’intérieur de son `.subckt … .ends`, même lorsque celui-ci est dans un fichier inclus ; pour le niveau supérieur, c’est dans le niveau de fichier du fichier principal (chapitre 17). Lorsque le solveur résout, le niveau est interprété sans les enregistrements portant `type="generated"` (2.8), afin que les anciens résultats ne contraignent pas la résolution suivante ; après la résolution, il les supprime et écrit les nouveaux résultats à la fin.
  - Entre deux résolutions, une nouvelle instruction de priorité normale qui entre en conflit avec les résultats du solveur est écartée et ne prend effet qu’après la résolution suivante. Une visionneuse peut suggérer de résoudre à nouveau, d’après l’empreinte dans l’en-tête d’enregistrement ; de tels écarts n’ont pas à être signalés un par un.
  - Un glissement à la souris dans un éditeur est écrit comme un PLACE avec `"high"`, également dans le texte propre du niveau, après l’enregistrement du solveur. Cela fixe la position : toute résolution ultérieure la prend comme contrainte stricte. Pour la libérer, supprimez cette instruction.
- **Ce qu’écrit le solveur.** L’enregistrement du solveur doit contenir chaque position, orientation et tracé que le solveur a décidé et que le texte laisse libre : par nom ou par clé pour les objets qui en ont, par numéro pour les autres (5.9), avec WIRE pour le tracé d’un fil (13.1), et sous forme d’instructions explicites pour les objets que génèrent les règles (10.5). Une décision qui n’y est pas écrite n’est pas un résultat : l’objet reste libre, comme si le niveau n’avait pas été résolu. Ainsi une visionneuse qui ne résout pas dessine le niveau exactement comme le solveur l’a dessiné. Le solveur ne décide pas de ce qu’il ne peut pas référencer, comme la position d’un texte sans clé qu’une règle de génération crée directement ; la visionneuse place un tel texte (13.3). Le point propre d’une utilisation créée par une règle n’est pas écrit non plus lorsque cette utilisation ne peut pas être écrite explicitement, par exemple lorsque sa définition est privée à un paquet (16.2), n’a ni symbole ni texte, et qu’aucun autre objet ni fil n’est placé par son point : un tel point n’aide que le solveur, rien de dessiné ne s’y tient, et le laisser libre ne change aucun dessin ; cela ne compte pas comme une décision laissée non écrite.
- **Les résultats d’un solveur sont du RNL.** Lorsqu’un solveur transmet ses résultats à d’autres outils sous une autre forme, comme les messages entre un solveur et une visionneuse, cette forme ne doit contenir que ce qu’un enregistrement du solveur peut contenir, afin que tout résultat puisse être enregistré sous forme d’un tel enregistrement.
- **Les objectifs d’un solveur** ne choisissent que parmi les placements qui satisfont à tout fait retenu ; un fait retenu est strict, quelle que soit sa priorité. Les objectifs ne font pas partie du langage : les sortes d’objectifs et leur ordre diffèrent selon les solveurs, et la présente norme ne peut pas définir un vocabulaire commun pour eux. Une valeur par défaut est fixée : un fil `"auto"` sans rien sur son chemin et non fusionné avec d’autres fils est résolu selon son tracé par défaut (13.1), qui compte d’abord les coudes et ensuite la longueur. Les options du solveur font partie de l’entrée que couvre l’empreinte (chapitre 17) ; un fichier les garde avec SOLVER-CONFIG, afin qu’un résultat puisse être reproduit (14.10).

### 14.9 Relation d’orientation : ORIENT-RELATION

```rnl
#M1 #M2 @ORIENT-RELATION("MY");                  /* M2 is drawn as M1 flipped left-right */
#M5 {#M7 #M8} @ORIENT-RELATION("R0");            /* two facts: M7 and M8 each have the orientation of M5 */
#M3 #M4 @ORIENT-RELATION(["R0", "MY"])           /* M4 as M3, or as M3 flipped left-right */
#M1::g #M2::g @ORIENT-RELATION("R180")           /* the gates of M1 and M2 face opposite ways */
```

- `A B @ORIENT-RELATION(relation)` : deux emplacements, `from` et `to`, chacun prenant un objet. La relation est l’un des huit codes d’orientation d’ORIENT (11.2), ou une liste de ceux-ci, donnés par position. L’orientation de B est l’orientation de A suivie de la transformation de la relation. La transformation est prise dans les coordonnées du dessin, si bien que `"MY"` retourne toujours gauche-droite par rapport à une droite verticale, que A lui-même soit tourné ou non.
- **Une liste de relations**, telle que `["R0", "MY"]`, forme un seul fait, qui est vérifié lorsque l’une des relations listées est vérifiée entre A et B. Il est retenu ou écarté en bloc (8.5), et le solveur choisit quelle relation il dessine. La liste comporte au moins un code ; une liste d’un seul code équivaut à ce code.
- **Un objet vers un objet.** Un ensemble dans un emplacement est traité membre par membre (5.2.1), un fait pour chaque membre : `#M5 {#M7 #M8} @ORIENT-RELATION("R0")` donne à M7 et M8 chacun l’orientation de M5, et donc à tous trois la même. Un ensemble de plus d’un membre dans les deux emplacements est une erreur : apparier les membres dans l’ordre et relier chaque paire auraient tous deux un sens (5.2.1).
- Il n’existe aucune forme avec un seul ensemble dont tous les membres ont la même orientation : le mot-clé nomme une relation entre deux choses, et avec un seul ensemble il n’est pas clair à quoi s’applique la relation. Un objet unique ne correspond pas à la signature, ce qui est une erreur. Un ensemble unique est d’abord scindé d’un niveau, comme le dit 5.2.1, si bien que `{#M1 #M2} @ORIENT-RELATION("MY")` est `#M1 #M2 @ORIENT-RELATION("MY")` ; lorsque la scission ne donne pas deux objets, c’est une erreur.
- Les membres sont des objets qui ont une orientation, comme pour ORIENT (11.2) : instances, faisceaux et utilisations pourvues d’un symbole ; ou bien tous deux sont des broches (ci-dessous), une borne représentant sa broche (5.2.4). Un point, net, fil ou utilisation sans symbole est une erreur, tout comme une broche reliée à un objet : la contrainte échoue. Lorsqu’un emplacement reçoit un ensemble vide, il n’y a rien à relier, et rien ne se passe (5.2.1).
- **Broches.** Avec une broche dans chaque emplacement, la relation porte entre les directions des deux broches dans le dessin (11.2) : la direction de B est la direction de A tournée par la relation, telle que `"R180"` pour des broches qui pointent en sens opposés, et un code en miroir tourne une direction comme il tourne un point. La relation ne s’écrit qu’avec les codes : les mots de 11.2 nomment des directions, non des rotations, et écrits ici ce sont de mauvaises valeurs de paramètre, si bien que la contrainte échoue. Une broche sans `direction=` écrit fait échouer ce fait (9.2.4).
- C’est une contrainte : elle crée un objet contrainte, peut être nommée, prend `priority=` et peut être modifiée avec PRIORITY (14.8) ; dans les conflits, elle est retenue ou écartée comme une seule unité (8.5). Traitée membre par membre, elle crée une contrainte pour chaque membre, et un nom lie leur ensemble (5.2.1). Un membre pourvu d’un ORIENT d’un seul code a déjà son orientation fixée ; pour les autres membres, le solveur choisit parmi les orientations qui satisfont à la relation et à leurs listes ORIENT (11.2).
- Elle ne fixe pas l’orientation réelle : lorsque deux transistors MOS sont images en miroir, les grilles tournées vers l’intérieur et les grilles tournées vers l’extérieur satisfont toutes deux à `"MY"`. Pour trancher, écrivez aussi un ORIENT, ou un ORDER sur les bornes (14.3) ; pour restreindre sans trancher, écrivez ORIENT avec une liste (11.2).

### 14.10 Réglages du solveur : SOLVER-CONFIG

```rnl header
@SOLVER-CONFIG(spacing_x=2, spacing_y=1, layout="lines", lines_weights=[1, 4])    /* the default of the file */
```

```rnl
@SOLVER-CONFIG(spacing_x=1)                                                       /* this level: overrides the key spacing_x */
```

- SOLVER-CONFIG fournit des réglages au solveur, afin que le fichier puisse être résolu à nouveau avec les mêmes réglages. Il ne crée aucun fait : le solveur n’utilise les réglages que pour choisir parmi les placements qui satisfont à tout fait retenu (14.8).
- **Forme** : paramètres nommés en littéraux RNL : nombres, chaînes, `true` et `false`, et tableaux de ceux-ci (3.7). Il n’y a pas de réglages imbriqués. Hormis les clés que nomme la présente norme (ci-dessous), les clés ne sont pas des mots standards : chaque solveur définit les siennes. Le cœur lit les paramètres comme littéraux RNL et les transmet au solveur tels qu’écrits, dans l’ordre ci-dessous, sans en fusionner aucun ; parmi les valeurs, il ne vérifie que celles des clés que nomme la présente norme. Le solveur juge les autres clés et valeurs : une clé qu’il ne connaît pas produit un avertissement et est ignorée. Un solveur peut aussi n’accepter une clé que sous certains de ses propres réglages, par exemple un objectif que l’un de ses placements utilise et pas un autre ; sous un réglage qui n’utilise pas la clé, la clé est ignorée de la même façon, avec un avertissement.
- **Clés nommées par la présente norme** : `spacing_x` et `spacing_y`, l’espacement du niveau en unités de grille, ajouté à l’espacement par défaut du solveur, comme pour les groupes (11.5) ; et `junction`, la définition avec laquelle le solveur dessine les points de jonction aux points d’embranchement, un nom en `$` écrit sous forme de chaîne, tel que `"$JUNCTION"` ; dans le SOLVER-CONFIG d’un enregistrement de niveau aussi un nom en `%`, recherché dans ce niveau (6.6), afin qu’une variante définie pour un sous-circuit reste dans celui-ci ; ou `""` pour aucun point de jonction (13.2). Tout solveur prend `junction`. Ce sont les seules clés que nomme la présente version, non les seules clés qu’un solveur peut prendre ; quelles autres clés possède un solveur, telles que `layout` et `lines_weights` dans l’exemple, et ce qu’elles signifient, relève de ce solveur, qui les documente. Un solveur qui dispose d’un espacement du niveau le prend sous ces deux noms.
- **Où** : dans l’enregistrement d’en-tête sans `language`, pour les niveaux de ce fichier et des fichiers qu’il inclut, comme pour DEVICE (2.8, 6.12) ; dans l’enregistrement d’en-tête d’un fichier de paquet, pour les niveaux qui utilisent le paquet (16.2) ; et dans les enregistrements d’un niveau, pour ce niveau. Il n’a aucun objet. Écrit dans le contenu d’un paquet, dans un paquet en ligne, dans un enregistrement avec `language` ou dans un enregistrement du solveur, il est sans effet, et un avertissement est émis ; ainsi un paquet utilisé uniquement par des enregistrements d’en-tête avec `language` ne fournit aucun réglage.
- **Ordre** : les réglages se surchargent clé par clé, le plus tardif l’emportant sur le plus précoce : les valeurs par défaut du solveur, les enregistrements d’en-tête des paquets utilisés par le niveau, dans l’ordre de leurs positions (comme pour DEVICE, 6.12), les enregistrements d’en-tête des fichiers externes, puis ceux des fichiers inclus, les enregistrements du niveau, puis les réglages donnés à l’outil, par exemple sur sa ligne de commande ou dans une page d’une visionneuse. Un outil les prend dans le même texte que les parenthèses, afin qu’ils puissent être copiés dans les deux sens.
- **Une valeur non acceptée**, telle qu’une chaîne là où un nombre est attendu, est une erreur pour cette seule clé (16.4) : la clé compte comme non écrite à cet endroit, de sorte que la valeur d’avant reste en vigueur, et les autres clés de la même instruction s’appliquent comme d’habitude. Une valeur est jugée dans chaque niveau auquel elle s’applique : un `junction` dans un enregistrement d’en-tête qui ne nomme aucune définition dans un niveau n’est pas accepté dans ce seul niveau, et les autres niveaux l’utilisent toujours. Le cœur juge les valeurs des clés que nomme la présente norme, et écarte celles qu’il ne prend pas ; le solveur juge les valeurs de ses propres clés. Les réglages qu’un solveur reçoit sont donc une liste dans l’ordre ci-dessus, où une clé peut figurer plus d’une fois, et pour chaque clé le solveur prend la dernière valeur qu’il accepte.
- **Les budgets de temps ou d’effort** (8.5) ne sont donnés que par l’outil, par exemple sur sa ligne de commande ou dans une page d’un visualiseur, jamais par le fichier : un budget dépend de la machine qui résout. Les clés d’un solveur qui fixent un budget, comme `limit`, `seconds` et `work` du solveur intégré, sont ses clés réservées à l’outil : écrite dans SOLVER-CONFIG, dans n’importe quel enregistrement, une telle clé est ignorée, avec un avertissement, comme une clé inconnue, et le budget donné par l’outil, ou la valeur par défaut du solveur, reste en vigueur.
- Les réglages en vigueur pour un niveau, ceux des paquets compris, font partie de l’entrée que couvre l’empreinte du solveur (chapitre 17).

### 14.11 Tables : TABLE

```rnl
@TABLE layout: {
  {#M1 #M2 #M3}
  {#M4 {#M5 #M6} #M7}
}
```

- TABLE dispose des objets sous forme d’une table écrite avec des `{…}` imbriqués : l’exemple comporte deux lignes de haut en bas ; dans la première, M1, M2 et M3 de gauche à droite ; dans la seconde, M4, la cellule de M5 et M6, et M7 de gauche à droite ; dans cette cellule, M5 au-dessus de M6. Une table peut être écrite sur plusieurs lignes. La forme standard écrit le mot-clé en premier, lu comme « @TABLE layout: {…} » (4.3) ; le mot `layout:` y est du texte libre (3.3), et l’emplacement de TABLE qui prend la table est `rows` (Annexe A).
- **Lecture** : la liste externe va de haut en bas, un élément pour chaque ligne ; chaque liste imbriquée à l’intérieur va perpendiculairement à la liste qui la contient, de gauche à droite dans une liste qui va de haut en bas, et de haut en bas dans une liste qui va de gauche à droite. `transpose=true` fait aller la liste externe de gauche à droite. L’ensemble donné à TABLE est sa liste externe telle qu’écrite et n’est pas scindé d’abord (5.2.1 point 2), si bien que `@TABLE layout: {{#M1 #M2 #M3}}` est une seule ligne, et `@TABLE layout: %port_set` place chaque élément de l’ensemble sur sa propre ligne.
- **Éléments** : un élément qui est un objet est une cellule ; un élément qui est un ensemble, un `{…}` écrit dans la table ou une référence à un ensemble, est une liste un degré plus à l’intérieur. C’est la règle de TABLE : un ensemble qui s’y trouve n’est pas un élément qui représente chacun de ses membres (14.1). Une utilisation dessinée avec un symbole forme une cellule, par son point propre ; une utilisation sans symbole forme une cellule qui contient ses membres (14.1), chacun d’eux précédant l’élément suivant ; `%u::position` fait de son point propre une cellule.
- **Le fait** : un TABLE est un seul fait géométrique, retenu ou écarté en bloc dans les conflits (8.5). Il est satisfait lorsque, dans chaque liste de la table, chaque point d’un élément se trouve strictement avant chaque point de l’élément suivant, dans la direction de cette liste : à au moins une unité de coordonnée avant, comme pour ORDER (14.3). Un vérificateur peut la développer en de tels ordres ; le solveur reçoit la table comme un seul élément, avec la direction de sa liste externe et des listes imbriquées à l’intérieur.
- **Un groupe** : les objets de la table entière forment un seul groupe, comme avec `@GROUP` sur le même ensemble (11.5) ; ses lignes et ses colonnes ne forment pas de groupes propres.
- TABLE n’aligne pas : les lignes n’ont pas besoin d’avoir une même hauteur, ni les colonnes un même x. Pour les aligner, écrivez ALIGN. Une cellule ne s’étend pas sur plusieurs lignes ou colonnes.
- Une liste vide dans une table est omise, et ses voisins deviennent voisins (14.1). Une table d’une seule cellule est satisfaite trivialement et ne crée aucun groupe. Un objet figurant dans deux cellules d’une même table devrait venir avant lui-même, donc le TABLE échoue, tout comme lorsqu’un membre ne peut pas être localisé, tel qu’un net ou une borne sans broche (14.1).
- TABLE prend `priority=` (14.8) et crée un objet contrainte, qui peut être nommé.

<!-- translated from en/ch15.md sha256:7d292a08ebfb -->
## 15 Sémantique électrique

### 15.1 Application des définitions sémantiques

```rnl
%dp1 = #M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N;
%cm1 = {#M3 #M9}(ref) {#M4 #M6}(out) @AS $CURRENT_MIRROR_P;
%in = {#M8 #M5} @AS $MATCHED
```

- Ce que signifient les objets est marqué en appliquant une définition sémantique avec AS (9.5) : l’application crée une utilisation, et les objets remplissent les rôles de la liste de rôles de la définition. Marquer une signification et énoncer une relation de placement sont la même sorte d’instruction.
- Une utilisation peut être membre d’une autre utilisation (imbrication), comme dans les exemples de 9.4 et 10.1.
- Un rôle d’ensemble, écrit `"{name}"` dans la liste des rôles (tel que `"{members}"` de `$MATCHED`, et `"{ref}"` et `"{out}"` de `$CURRENT_MIRROR_P` dans l’exemple), prend un seul ensemble, pour un nombre de membres qui n’est pas fixe (9.4).
- Un rôle peut déclarer la sorte d’objet qu’il prend, telle que `roles=[["in_p", "instance"], ["in_n", "instance"]]` ; un objet d’une mauvaise sorte fait échouer toute l’application (9.4). Une définition au corps vide ne vérifie rien de ce qui remplit les rôles, si bien que sans de telles déclarations un mauvais objet ne peut pas y être détecté.
- Les noms de rôles d’une utilisation sont ses sous-objets, tels que `%dp1::left`.
- L’application d’une définition développe son corps, et le corps peut contenir des contraintes, telles qu’un dessin par défaut de priorité `"low"`, que la priorité de l’utilisation désactive (9.5, 14.8).
- Non vérifié : la conformité de la signification avec les connexions, les types ou les paramètres de la netlist n’est pas vérifiée (Principes 2.4). Par exemple, le langage ne vérifie pas pour `#M3(ref) #M4(out) @AS $MY_MIRROR` si la grille et le drain de M3 sont connectés ; une définition vérifie ce qu’elle affirme à l’aide des assertions de son corps (15.3, 15.4).
- Les utilisations d’une signification donnée sont sélectionnées avec SELECT-USAGE, et les utilisations qui contiennent un objet donné avec SELECT-ANCESTOR-USAGE ; les membres des utilisations sont atteints par une sélection dont la portée est ces utilisations (10.1).
- Le vocabulaire des définitions sémantiques appartient au paquet standard (9.5), qui fournit des définitions telles que `$DIFFERENTIAL_PAIR_N`.
- La validité de base d’une description (Principes 2.4) est la suivante : ses références peuvent être trouvées, ses rôles reçoivent des objets des bonnes sortes, ses rôles simples sont remplis, et chaque instruction qu’elle développe est valide elle-même (9.5). Le fait que deux rôles soient remplis par le même objet n’est pas vérifié.

### 15.2 Sens du courant : CURRENT

```rnl
#M5:1 @CURRENT("in");                 /* drain: the current flows in */
#M5:3 @CURRENT("out")                 /* source: the current flows out */
```

- CURRENT fixe le sens du courant statique à une borne : `"in"` (il entre), `"out"` (il sort) ou `"unspecified"` (utilisé pour surcharger une valeur donnée par une règle d’un paquet). Le paramètre est `direction`. Une règle de CURRENT est évaluée à l’étape 6 (10.4). Un CURRENT dont l’objet est une broche, écrite ainsi ou atteinte via un rôle, a besoin du symbole pour trouver sa borne (5.2.4) : ce n’est pas une règle, et il prend effet une fois que les symboles de son objet sont attribués, à l’étape 4, ou à l’étape 7 pour un objet généré à l’étape 6. Jusqu’à l’étape 6, la condition `current` des sélections ne lit que les instructions CURRENT écrites sur des bornes ; à partir de l’étape 7, comme dans la file des styles, elle lit tous les sens, tout comme la géométrie et la vérification ci-dessous.
- C’est un fait concernant la borne, et non un fait géométrique : il ne prend aucune part aux conflits (8.5), et un solveur ne l’utilise que comme indication, par exemple pour l’ordre dans lequel il essaie les orientations. Comme ORIENT, c’est une instruction modificatrice et elle transmet sa poignée d’entrée. Elle possède une clé, une par borne, et la plus tardive l’emporte. Une borne pour laquelle elle n’est jamais écrite est non spécifiée.
- Il n’est donné que sur des bornes. Le courant à l’intérieur d’un composant à plusieurs bornes ne peut pas être découpé clairement : pour un composant à trois bornes, « a coule vers b, b coule vers c » laisse incertain le fait que le courant en b entre ou sorte, tandis que le sens du courant statique à une borne a toujours une signification définie.
- Il ne prend que des bornes de l’hôte, y compris les ports de ce niveau : `#:inp @CURRENT("in")` signifie que le courant entre dans le sous-circuit à cet endroit. Une broche d’un composant se convertit en sa borne, telle que `#M1::d @CURRENT("in")` ; les composants virtuels n’ont pas de bornes, donc leurs broches ne se convertissent pas (5.2.4). Une borne sans broche est autorisée (contrairement aux fils).
- Non vérifié par rapport à la netlist (Principes 2.4). Les instructions CURRENT d’un même net forment ensemble une assertion (15.3) : lorsque chaque borne sur le net, les ports de ce niveau compris, a un CURRENT de `"in"` ou `"out"`, et que toutes déplacent le courant dans le même sens par rapport au net, cela produit une violation (7.6) qui nomme le net et ses bornes, puisque le courant n’a nulle part où aller. Une borne de composant ayant `"in"` retire du courant du net, et un port de ce niveau ayant `"in"` y apporte du courant. Une borne sur le net sans CURRENT, ou avec `"unspecified"`, laisse le net non vérifié. Un net global (6.3), tel que `0`, n’est pas vérifié non plus : c’est le même nœud dans chaque niveau, si bien que le courant peut sortir par les bornes d’autres niveaux. Un CURRENT dans un enregistrement, ou dans le corps d’une utilisation, dont la priorité est `"disabled"` ne compte pas. Il est vérifié avec les assertions, après l’étape 7 de 8.2, et comme elles ce n’est qu’un rapport, qui nomme le net, ses bornes et leurs instructions CURRENT ; les faits CURRENT restent en vigueur.
- Utilisation : le courant à la borne circule dans la direction dans laquelle un fil quitte la broche (9.2.4), tournée avec l’orientation du symbole, lorsqu’il sort, et en sens inverse lorsqu’il entre. Un solveur peut l’utiliser pour choisir parmi les placements, par exemple en préférant que le courant descende vers le bas (14.8) ; vers le haut ou vers le bas relève du style maison. Une borne sans CURRENT, avec `"unspecified"`, ou dont la broche n’écrit pas `direction=` (9.2.4) ne donne aucun sens ; une direction qu’une implémentation déduit des graphiques ne compte pas ici. Des règles peuvent sélectionner des bornes selon le sens de leur courant, avec la condition `current` de SELECT-TERMINAL (10.1).
- **Flèches** : CURRENT ne dessine rien. Le fait que des flèches soient dessinées à partir de CURRENT et la façon dont elles le sont constituent une préférence de la visionneuse (partie de son style maison), comme quels noms de nets sont affichés (13.3). Une visionneuse qui les dessine elle-même saute les bornes qui possèdent déjà une utilisation de la flèche de courant standard, tout comme elle reconnaît `$NO_CONNECT` standard (12.6). Les flèches à garder (provenant d’un schéma importé, ou décidées par l’auteur) s’écrivent comme des objets explicites :

  ```rnl
  $CURRENT_ARROW @ATTACH #M5:1 => @ORIENT("R270") => @TEXT("I_{SS}", dx=1);   /* on the drain pin, with I_SS beside it */
  $CURRENT_ARROW @ATTACH (12, 20) => @ORIENT("R270")                           /* in the middle of a wire, at absolute coordinates */
  ```

  - Une flèche dessinée sur une borne est une utilisation dont la définition attache son point à la broche de la borne (12.1 point 5). Elle ne tourne pas avec la broche, donc son orientation est écrite ; le texte à côté est attaché à l’utilisation (13.3).
  - Une flèche au milieu d’un fil n’est pas ancrée au fil : elle est dessinée en un point, avec des coordonnées absolues.
  - La manière dont une flèche pointant vers l’intérieur est distinguée d’une flèche pointant vers l’extérieur, ainsi que la manière dont sont dessinés la définition et le symbole de la flèche, appartiennent au paquet standard. Le symbole peut être simplement une ligne munie d’une pointe de flèche à son extrémité (9.2.5).
  - Les flèches dessinées sont tenues à l’écart de CURRENT : si elles ne formaient qu’une seule chose, exporter une flèche changerait ce que vise le placement.

### 15.3 Assertions : ASSERT-INSTANCE, ASSERT-TERMINAL, ASSERT-NET, ASSERT-USAGE, ASSERT-IN

```rnl
$MATCHED_PAIR = @DEFINE(roles=[["left", "instance"], ["right", "instance"]]) {
  {?left ?right} @ASSERT-INSTANCE(same=["model", "w", "l"], different=["name"]);
  {?left ?right} @SELECT-TERMINAL(terminal="s") => @ASSERT-TERMINAL(same=["net"], message="the sources of a pair share a net")
};
# @SELECT-INSTANCE(name="MD*") => @ASSERT-INSTANCE(type="nmos");
%dp1 @ASSERT-USAGE $DIFFERENTIAL_PAIR_N;                      /* %dp1 is a usage of this definition */
#M5:s @SELECT-NET => @ASSERT-IN $GROUND_NET_SET              /* the source of M5 is on a ground net */
```

- Une assertion vérifie que ce qu’énonce un fichier ou une définition concorde avec les faits du circuit, par exemple que les deux moitiés d’une paire ont le même modèle et la même taille, ou que des composants factices sont d’un même type. Elle ne fait que signaler : elle ne crée aucun fait, ne modifie rien dans le dessin et ne prend aucune part à la résolution (8.5).
- **Un mot-clé par sorte**, comme pour les sélections (10.1) : ASSERT-INSTANCE vérifie les instances (et les faisceaux, par leur premier membre, 11.8), ASSERT-TERMINAL les bornes, ASSERT-NET les nets, ASSERT-USAGE les utilisations. L’unique emplacement prend un ensemble d’objets de cette sorte ; un objet unique est un ensemble d’un seul élément. La sorte d’un objet est connue lors de la lecture du texte, et une mauvaise sorte est une erreur à ce moment (5.2.4), donc aucune assertion ne vérifie les sortes.
- **Conditions** : elles ont les noms et les significations des conditions du mot-clé de sélection de la même sorte (10.1), tels que `type="nmos"`, `terminals=4`, `net="vss"` ou `port=true` : chaque membre doit satisfaire à chaque condition écrite.
- **`same=[…]`** : pour chaque attribut de la liste, tous les membres ont la même valeur. **`different=[…]`** : pour chaque attribut de la liste, deux membres quelconques ont des valeurs différentes. Les noms d’attributs sont ceux de `${…}` (13.6) : pour les instances `name`, `value`, `model`, `type` et les noms de leurs paramètres ; pour les bornes `net` ; pour les nets `name` ; pour les utilisations `definition`. Pour les bornes, les nets et les utilisations, ce sont les seuls : tout autre nom est une mauvaise valeur de paramètre, et l’assertion échoue (7.1). Pour les instances, les noms de paramètres sont ouverts, donc tout nom est accepté, et un attribut qu’un membre n’a pas compte comme une valeur propre, « absent » : les membres qui en sont dépourvus sont égaux entre eux et diffèrent de toute valeur écrite. Ainsi `same=["nf"]` est vérifié lorsqu’aucun membre n’écrit `nf`, et produit une violation lorsque certains l’écrivent et d’autres non ; un procédé dont les composants n’ont pas ce paramètre n’a pas besoin d’assertion propre. Une instance sans type (10.1) est dépourvue de `type`. Les valeurs sont comparées telles que les écrit l’hôte, sans évaluation (il n’y a pas d’expressions, 3.7).
- **`has=[…]`**, pour ASSERT-INSTANCE : chaque membre possède chaque attribut de la liste, tel que `has=["w", "l"]` ; un membre qui en manque d’un produit une violation. `same` n’exige pas qu’un attribut soit écrit ; `has` l’exige.
- **ASSERT-USAGE** prend une définition comme dernier objet, comme le fait SELECT-USAGE (5.6, 10.1) ; elle peut être omise. Avec elle, chaque membre doit être une utilisation de cette définition, reconnue par ce vers quoi se résout son nom (9.5), et non par la graphie du nom. Ses conditions sont `symbol` et `drawn` (10.1). `same=["definition"]` est vérifié lorsque tous les membres sont des utilisations d’une même définition, et `different=["definition"]` lorsque aucun couple ne l’est.
- **ASSERT-IN** : `X @ASSERT-IN S` vérifie que chaque membre de X est membre de l’ensemble S, qui est écrit en dernier, comme pour ADD-TO (5.2.2) ; S doit être un ensemble et n’est pas enveloppé dans un ensemble. Le fait que deux membres soient identiques est décidé par poignée (5.2). Les membres peuvent être de n’importe quelle sorte, et S peut dépendre de sélections (10.4). Il n’a aucune condition, seulement `message`, et une violation nomme les membres de X que S ne contient pas. Avec une différence d’ensembles (5.2.2), il permet aussi d’énoncer qu’un ensemble est vide.
- `different=["name"]` vérifie que des instances ou des nets sont des objets différents, par exemple lorsque deux rôles ne doivent pas être remplis par le même objet ; les bornes n’ont pas de `name`, si bien que pour elles `name` est une mauvaise valeur de paramètre, dans `different` comme dans `same`.
- **Le rapport** : une vérification non satisfaite produit une violation (7.6) qui nomme les membres et l’attribut ; **`message`** y ajoute un texte. Rien n’échoue, et le dessin est identique à ce qu’il serait sans l’assertion : une assertion n’est qu’un rapport, pour l’auteur et pour des outils tels que `rnl check`.
- **Où et quand** : dans n’importe quel niveau, et dans le corps d’une définition générale, où elle est vérifiée pour chaque application (9.4). Les assertions sont vérifiées après l’étape 7 de 8.2, lorsque chaque condition peut être lue, y compris `drawn` et `symbol` (10.1). Un ensemble sans membres est satisfait trivialement, tout comme une assertion recevant le nœud vide (6.3).
- Une assertion n’a aucune priorité ; dans un enregistrement portant `priority="disabled"`, et dans le corps d’une utilisation dont la priorité est `"disabled"` (14.8), elle n’est pas vérifiée.

### 15.4 Assertions de topologie : ASSERT-TOPOLOGY

```rnl
{#R1 #R2 #R3} @ASSERT-TOPOLOGY("series");                  /* a chain: R1 to R2 to R3 */
{#M3 #M4} @ASSERT-TOPOLOGY("parallel", exclude=[["nmos", "g"], ["nmos", "b"]]);   /* drains on one net, sources on one net */
{#L1 #C1 #L2} @ASSERT-TOPOLOGY("star");                    /* a T network: three branches meet at one net */
{#C1 #L1 #C2} @ASSERT-TOPOLOGY("loop");                    /* a pi network: C1, L1 and C2 go round */
{%branch1 %branch2} @ASSERT-TOPOLOGY("parallel")           /* two branches, each a usage or a set taken as a black box, between the same nets */
```

- ASSERT-TOPOLOGY vérifie que certains membres sont connectés d’une manière donnée : en série, en parallèle, en étoile ou en une seule boucle. C’est une assertion comme celles de 15.3, et ce que 15.3 en dit vaut pour elle aussi : elle ne fait que signaler, une vérification non satisfaite produit une violation (7.6), à laquelle `message` ajoute un texte, et où, quand et sous quelle priorité elle est vérifiée est identique.
- **Les membres** : l’unique emplacement prend un ensemble de membres, dans l’ordre, comptés par éléments (5.2) : un objet est un membre, et un élément qui est un `{…}` ou une référence à un ensemble est un membre, une boîte noire. Un ensemble qui est le seul objet est d’abord scindé d’un niveau (5.2.1), si bien que `%chain @ASSERT-TOPOLOGY("series")` vérifie les éléments de la chaîne, et `{%branch1 %branch2}` compte deux membres. Un membre est donc une instance (ou un faisceau, par son premier membre, 11.8), une utilisation ou un ensemble. Recevant le résultat d’une sélection, qui comporte un élément par membre, les membres sont dans l’ordre du résultat (10.3). Avec moins de deux membres, la vérification est satisfaite trivialement.
- **Les nets d’un membre** :
  - une instance : les nets sur lesquels se trouvent celles de ses bornes que `exclude` ne retire pas ;
  - une utilisation ou un ensemble : une boîte noire, un seul élément formé des bornes qu’elle apporte. Une utilisation, pourvue ou non d’un symbole, représente ce qui remplit ses rôles, et un ensemble ses membres, suivis vers l’intérieur jusqu’aux plus internes ; une instance apporte toutes ses bornes, une borne elle-même uniquement, une broche les bornes qu’elle lie, et les nets et les points n’apportent rien ; un faisceau compte par son premier membre. Les nets de la boîte noire sont ceux sur lesquels se trouvent les bornes qu’elle apporte que `exclude` ne retire pas, et qui sont utilisés à l’extérieur d’elle aussi : par une borne qu’elle n’apporte pas, celles des autres membres d’un faisceau comprises, ou comme port du niveau. Un net utilisé uniquement par les bornes qu’elle apporte ne compte pas. Note : un net que deux membres partagent est utilisé à l’extérieur de chacun d’eux par l’autre, de sorte que le fait qu’un net soit un port ne change un verdict que pour des membres qui apportent la même borne ; sinon il ne change que les nets qui sont ceux de la boîte noire, tels qu’un outil peut les montrer.
- **`exclude=[[type, terminal], …]`** : bornes qui ne comptent pas, sous forme de paires composées d’un type, comparé comme la condition `type` (10.1), et d’un nom ou numéro de borne, telles que `exclude=[["nmos", "g"], ["nmos", "b"]]`. La borne d’une paire est comparée comme la condition `terminal` de SELECT-TERMINAL (10.1) : un nombre par sa place, un nom selon les règles de casse de l’hôte ; un nom que l’instance n’a pas ne correspond à rien. Une borne d’une instance sans type, ou d’un type que la liste ne nomme pas, compte. Sans `exclude`, chaque borne compte. Une paire qui n’est pas un tableau de deux chaînes, ou d’une chaîne et d’un nombre entier, est une mauvaise valeur de paramètre, et l’assertion échoue (7.1).
- **Les topologies**, le premier paramètre, donné par position, l’une de quatre :
  - `"series"` : chaque couple de deux membres voisins dans l’ordre partage exactement un net, et deux membres non voisins n’en partagent aucun. Ce qui se trouve d’autre sur un net partagé n’est pas vérifié : le milieu d’un diviseur peut mener ailleurs.
  - `"parallel"` : au moins deux nets, les extrémités, sont partagés par tous les membres. Les autres nets d’un membre, tels qu’une dérivation qu’une branche fait sortir, ne sont pas vérifiés, tout comme rien d’autre sur un net partagé n’est vérifié pour `"series"`.
  - `"star"` : un seul net est partagé par tous les membres, et aucun couple de membres ne partage d’autre net.
  - `"loop"` : au moins trois membres. En faisant le tour dans l’ordre, le dernier et le premier compris, chaque couple de deux membres voisins partage exactement un net, ces nets sont tous différents, et deux membres non voisins n’en partagent aucun. Avec deux membres, `"loop"` produit une violation ; écrivez `"parallel"`.

<!-- translated from en/ch16.md sha256:fa0479c7afe6 -->
## 16 Fichiers, versions, paquets et alias

Ce chapitre couvre la version du langage d’un fichier, les paquets et les fichiers de paquets, le paquet standard, le contenu inconnu et les alias.

### 16.1 Versions

```rnl
* @RNL(type="header") { @VERSION("0.4")}
```

- **Une seule version de langage par fichier.** Du RNL de versions différentes s’écrit dans des fichiers différents, réunis par inclusion (6.1).
- **Déclarée dans l’enregistrement d’en-tête.** VERSION ne peut être écrit que dans l’enregistrement d’en-tête (`type="header"`), qui est le premier enregistrement du fichier (2.8). La version ne peut être déclarée qu’une seule fois, de sorte qu’un fichier n’a jamais deux valeurs de version. Un VERSION écrit ailleurs est une erreur et est ignoré.
- **Ordre de lecture :**
  1. l’adaptateur hôte fournit les commentaires ;
  2. tous les enregistrements sont découpés selon leur forme externe, et leurs en-têtes d’enregistrement sont lus ; cette étape ne dépend pas de la version ;
  3. lorsque le premier enregistrement est l’enregistrement d’en-tête, son VERSION est lu ;
  4. les autres enregistrements sont analysés dans cette version.
- **Parties qui ne dépendent pas de la version.** Les porteurs de commentaires, l’entrée `@RNL`, la forme de l’en-tête d’enregistrement et la forme de VERSION dans l’enregistrement d’en-tête ne changent dans aucune version ultérieure ; sinon un analyseur ne pourrait pas trouver la version. La prise en charge d’un nouvel hôte ne fait qu’ajouter des porteurs et ne modifie pas les porteurs existants (2.2). Le reste de l’enregistrement d’en-tête est lu dans la version déclarée.
- **Absence d’enregistrement d’en-tête.** L’enregistrement d’en-tête peut être omis, tout comme VERSION à l’intérieur. Un fichier inclus prend alors la version du fichier qui l’inclut ; un fichier principal prend la version par défaut de l’outil, avec une indication. Les outils doivent écrire l’enregistrement d’en-tête et la version lorsqu’ils enregistrent un fichier, si bien que les fichiers écrits à la main peuvent les omettre et les fichiers écrits par des outils les contiennent. Un fichier de paquet doit comporter un enregistrement d’en-tête (16.2) ; sans VERSION il prend la version par défaut de l’outil, comme le fait un fichier principal. Un paquet en ligne a la version du fichier dans lequel il est écrit (16.2).
- **Version non prise en charge.** Lorsqu’un outil ne prend pas en charge la version d’un fichier, rien du RNL de ce fichier n’est interprété, une erreur est signalée, et le fichier est gardé tel quel lors de l’enregistrement ; les autres fichiers sont interprétés comme d’habitude.
- Les implémentations doivent déclarer les versions qu’elles prennent en charge (Principes 2.3). La version du langage, la version d’un paquet (`version=` d’USE-PACKAGE) et la version de l’hôte sont trois choses différentes. La façon dont les versions de paquets sont écrites et mises en correspondance est en 16.2. Un fichier de la présente version n’utilise que des paquets de la présente version : un fichier de paquet ou un paquet d’alias qui déclare une autre version du langage, ou qui n’en déclare aucune et est donc lu dans une version antérieure, est une erreur à l’USE-PACKAGE ou l’USE-ALIASES qui l’atteint, directement ou via d’autres paquets (16.2, 16.5).
- **Numéros de version.** Une version s’écrit `"major.minor"` : deux entiers décimaux non signés, tels que `"1.0"` ou `"1.2"`. Il n’y a ni numéros de correctif, ni dates, ni profils. Les versions antérieures à `"1.0"` sont des projets, écrits `"0.minor"` ; les versions 0.x ne font aucune promesse de compatibilité entre elles, et chacune compte comme une version majeure à part entière. Une version mal formée, telle que `"1"` ou `"v1.2"`, est traitée comme une version non prise en charge, et non comme une version manquante, afin que le fichier ne soit pas lu à tort dans une autre version.
- **Un fichier est lu dans la version qu’il déclare.** Les outils doivent savoir dans quelle version chaque mot standard a été ajouté : dans un fichier qui déclare une version antérieure, un mot ajouté ultérieurement n’est pas un mot standard. Ainsi un nom de sous-objet standard ajouté ultérieurement n’affecte pas une clé de même graphie dans un fichier plus ancien (6.5), un nouveau nom d’attribut n’affecte pas un paramètre hôte du même nom (13.6), et un nouveau paramètre standard d’AS et d’ATTACH n’affecte pas un paramètre de définition du même nom (9.4).
- **Au sein d’une même version majeure, une nouvelle version mineure ne fait qu’ajouter et ne modifie pas le sens.**
  - Autorisé : ajouter des mots-clés, des paramètres, des valeurs, des mots standards, des noms de sous-objets et des noms d’attributs ; donner un sens à une forme qui échouait auparavant. Les éléments de l’Annexe C, et tout ce que la présente norme appelle un assouplissement pur, sont de cette sorte, tout comme rendre tolérants les cas où l’interprétation entière échoue (1.5 point 3).
  - Interdit sans nouvelle version majeure : modifier le sens d’une forme valide ; supprimer quoi que ce soit ; modifier une valeur par défaut ; transformer une forme valide en erreur.
  - Un ancien fichier garde son sens dans un outil plus récent. Dans un outil plus ancien, les parties d’un fichier plus récent que possède l’ancienne version sont lues comme d’habitude, les nouvelles parties constituent du contenu inconnu (16.4), et un avertissement indique que la version du fichier est plus récente que celles prises en charge par l’outil. L’outil peut aussi choisir de ne pas dessiner le fichier, mais il ne doit pas faire semblant de le comprendre.
- **Une version majeure différente.** Un fichier d’une version majeure que l’outil ne prend pas en charge n’est pas interprété et est gardé tel quel (voir ci-dessus). Ses paquets ne sont pas utilisés non plus : rien du RNL du fichier n’est interprété, et une visionneuse dessine la netlist avec sa propre apparence par défaut et signale que la version n’est pas prise en charge. Un outil peut prendre en charge plusieurs versions majeures. Faire passer un fichier à une nouvelle version majeure est une migration explicite, dans laquelle l’outil réécrit le fichier et son enregistrement d’en-tête ; un outil ne doit pas lire silencieusement un ancien fichier selon les règles d’une nouvelle version majeure. Chaque nouvelle version majeure de la présente norme s’accompagne de notes de migration.
- **Versions prises en charge.** Un outil indique, pour chaque version majeure, jusqu’à quelle version mineure il prend en charge, par exemple `"1.0"`–`"1.3"`. Prendre en charge `"1.3"` signifie être capable de lire également de `"1.0"` à `"1.2"`.
- **Relever la version.** Lorsqu’un outil écrit dans un fichier une forme que seule une version plus récente possède, il relève la version dans l’enregistrement d’en-tête. Avant cela, il vérifie si les mots standards ajoutés entre-temps ont la graphie de noms figurant déjà dans le fichier (clés de textes, noms de rôles, paramètres de définitions, paramètres hôtes) et émet un avertissement pour chacun de ces noms, car relever la version peut changer son sens ; la version est relevée tout de même.
- **Versions 0.3 et 0.2.** La présente version remplace la version 0.3. Les fichiers de versions 0.3 et 0.2 sont des fichiers d’autres versions majeures (ci-dessus), que la présente norme ne décrit pas. Ils passent à la version 0.4 grâce à un outil de mise à niveau qui lit la version 0.3 : il relève la version, fait passer le fichier à la série standard 0.4 et écrit `@USE-PACKAGE(name="rnl-std-all", version="0.4")` dans son enregistrement d’en-tête à la place de toute version du paquet standard qui y était fixée, réécrit le fichier comme l’indique l’Annexe D, et signale ce qu’il ne peut pas réécrire, tel que l’utilisation d’une définition que la série 0.4 ne possède plus.

### 16.2 Paquets et USE-PACKAGE

```rnl fragment
@USE-PACKAGE(name="acme-style", version="1.2");                               /* looked up in the package directories of the tool */
@USE-PACKAGE(name="acme-style", version="1.2", path="lib/acme-style.rnl");    /* reads this package file */
@USE-PACKAGE(name="acme-style", version="1.2") {…}                            /* an inline package */
```

**Où un paquet prend effet.**

- Les définitions en `$` d’un paquet prennent effet dans le niveau où son USE-PACKAGE est écrit et dans les niveaux inférieurs. Ses règles comptent comme écrites dans ce niveau : elles s’y appliquent, et dans les niveaux inférieurs uniquement lorsqu’elles les atteignent, par `recursive=true` dans une sélection écrite à l’intérieur ou, si elles n’en ont aucune, par les ensembles qu’elles utilisent (10.1, 10.4). La présente version ne comporte aucun USE-PACKAGE qui ne s’applique qu’à son propre niveau. Écrit dans un sous-circuit, un paquet ne s’applique qu’à ce sous-circuit ; écrit à un niveau de fichier, aux sous-circuits du fichier et aux fichiers qu’il inclut (6.1) ; écrit dans l’enregistrement d’en-tête, aux mêmes niveaux qu’au niveau de fichier, mais placé à la racine (16.3).
- Les paquets à la racine, qu’ils soient écrits dans un enregistrement d’en-tête ou ajoutés par un réglage de l’outil (16.3), comptent comme utilisés au niveau supérieur du fichier : dans le fichier principal, son niveau supérieur. Ainsi les règles d’un paquet de style maison utilisé dans un enregistrement d’en-tête n’atteignent les sous-circuits qu’avec `recursive=true`, comme les écrit le paquet standard.
- **Place dans l’ordre.** Le contenu d’un paquet est placé à la position de son USE-PACKAGE, et à l’intérieur du paquet dans l’ordre de son texte, tout comme les faits développés depuis une définition prennent la position de l’instruction AS ou ATTACH. La résolution des conflits (8.5) et les files d’apparence (8.3) utilisent cette position. Un paquet utilisé avant les instructions d’un niveau agit comme un jeu de valeurs par défaut ; utilisé après elles, ses règles peuvent surcharger des instructions antérieures. Un USE-PACKAGE dans l’enregistrement d’en-tête est placé à la racine (16.3).

**Noms dans un paquet.** Seule la place suit l’USE-PACKAGE ; les noms d’un paquet restent les siens.

- Un nom dans un paquet est recherché d’abord parmi les définitions du paquet lui-même, puis dans les paquets qu’il référence (voir « Paquets qui référencent des paquets » ci-dessous), puis dans le paquet standard en vigueur au niveau de l’USE-PACKAGE. Il n’est recherché ni parmi les définitions de l’utilisateur de ce niveau ou des niveaux supérieurs, ni dans d’autres paquets ; comme dans un corps de définition (6.9), une définition de même nom dans le niveau de l’USE-PACKAGE ne le capture pas. Ainsi les noms d’un paquet en ligne se résolvent comme ceux d’un fichier de paquet de même contenu, et son texte peut être déplacé hors du fichier dans un tel fichier ; et un fichier de paquet peut être vérifié isolément, avec le paquet standard qu’ajoute un réglage de l’outil (16.3). Le paquet standard et les paquets qu’il référence, directement ou non, ne voient qu’eux-mêmes et les paquets qu’ils référencent, et non le paquet standard, de sorte qu’une recherche ne revient jamais sur eux.
- Les définitions en `$` d’un paquet sont publiques, et elles comptent comme appartenant au niveau situé juste au-dessus du niveau de l’USE-PACKAGE : une définition de même nom dans ce niveau masque celle du paquet et produit une indication (6.8) ; ce n’est pas une seconde liaison dans un même niveau. Lorsque les définitions en `$` d’un paquet masquent des définitions des niveaux supérieurs, ou des paquets à la racine, une indication est émise également (6.8).
- Les définitions en `%` d’un paquet sont privées au paquet.

**Ce que contient un paquet.**

- Un paquet contient des définitions, des ensembles en `$` et des règles. L’enregistrement d’en-tête d’un fichier de paquet contient VERSION et PACKAGE, et peut aussi contenir USE-PACKAGE, DEVICE (6.12) et SOLVER-CONFIG (14.10), qui s’appliquent aux niveaux utilisant le paquet, ainsi que ALIAS et USE-ALIASES, qui ne s’appliquent qu’à ce fichier (16.5). USE-PACKAGE référence d’autres paquets (ci-dessous), et peut aussi figurer dans un enregistrement d’en-tête pour un seul langage (2.8). Un paquet d’alias est un fichier de paquet ne comportant qu’un enregistrement d’en-tête, chargé avec USE-ALIASES (16.5).
- Un paquet n’a aucun circuit propre, comme le niveau de fichier d’un fichier inclus (6.1) :
  - les instructions qui nomment des objets hôtes particuliers (`#M1`, `&vss`, `#:inp`) échouent ; un `#` seul reste le niveau dans lequel une règle est évaluée, y compris dans le corps d’une définition générale (9.4) ;
  - une instruction qui n’est pas une règle et dessine un seul objet de la scène, telle que `(0, 0) @RECTANGLE …`, n’a aucun niveau dans lequel être dessinée et est une erreur ;
  - `scope=` et `type="generated"` dans un en-tête d’enregistrement sont des erreurs (2.8) ;
  - USE-PACKAGE dans un enregistrement d’un paquet autre que son enregistrement d’en-tête est une erreur : les références à d’autres paquets ne s’écrivent que dans l’enregistrement d’en-tête.
- Le corps d’un paquet en ligne suit les mêmes règles mais n’a pas d’enregistrement d’en-tête : le nom et la version du paquet sont les paramètres de son USE-PACKAGE, et DEVICE et SOLVER-CONFIG s’écrivent dans l’enregistrement d’en-tête du fichier lui-même. Un paquet en ligne ne peut pas référencer d’autres paquets ; pour figer une collection de paquets dans un fichier, son contenu développé est écrit dans le corps dans l’ordre, avec les changements qui en préservent le sens (16.3).

**Fichiers de paquets.** La façon dont un fichier de paquet est lu est donnée en 2.7, avec un exemple. Son enregistrement d’en-tête déclare par `@PACKAGE(name=…, version=…)` quel paquet et quelle version est le fichier ; un fichier sans PACKAGE n’est pas un paquet, et un USE-PACKAGE de celui-ci est une erreur. Un paquet est un fichier : un paquet réparti sur plusieurs fichiers nécessiterait un ordre entre eux, ce que la présente version ne prévoit pas. Un fichier de paquet déclare sa version de langage avec VERSION, qui doit être la version de langage des fichiers qui l’utilisent (16.1) ; sinon l’USE-PACKAGE est une erreur et les instructions qui utilisent les noms du paquet échouent comme noms introuvables.

**Recherche.**

- Un USE-PACKAGE muni d’un corps est un paquet en ligne et n’est pas recherché.
- Avec `path=`, ce fichier est lu. Le chemin est relatif au répertoire du fichier dans lequel l’USE-PACKAGE est écrit, avec `/` comme séparateur ; il nomme un fichier local, non une URL. Lorsque le fichier n’existe pas, ou que son PACKAGE nomme un autre paquet, ou une version que la règle ci-dessous n’accepte pas, le paquet est recherché par nom, avec un avertissement. Lorsque le fichier existe mais n’est pas un paquet, sans PACKAGE ou avec un PACKAGE dont `version=` est mal écrit, l’USE-PACKAGE échoue, comme pour tout fichier qui n’est pas un paquet (ci-dessus).
- Sinon le paquet est recherché par le nom et la version de son PACKAGE dans les répertoires de paquets configurés dans l’outil. Le paquet standard est fourni avec l’outil ou installé dans un répertoire de paquets. Les répertoires de paquets sont un réglage de l’outil et ne s’écrivent pas dans le fichier ; lorsqu’il existe plusieurs copies d’une même version, l’ordre des répertoires dans ce réglage décide.
- Les diagnostics indiquent quel fichier et quelle version ont été utilisés.
- Lorsque le paquet est introuvable, l’USE-PACKAGE échoue (une erreur), et les instructions qui utilisent des noms provenant du paquet échouent avec lui (7.3) ; le diagnostic donne le paquet manquant comme cause.
- `name=` est requis. Il est comparé comme une chaîne entière et est sensible à la casse ; les noms de paquets devraient utiliser des lettres minuscules, des chiffres et `-`.
- Avec `path=`, une netlist et ses fichiers de paquets peuvent être gardés ensemble et déplacés d’un bloc ; la présente version ne comporte aucun format de manifeste pour eux.

**Versions de paquets.**

- Une version de paquet s’écrit comme une version de langage, `"major.minor"` ; chaque 0.x compte comme une version majeure à part entière (16.1).
- Lorsque la version écrite existe, elle est utilisée. Sinon la plus petite des versions plus récentes au sein de la même version majeure est utilisée, avec un avertissement indiquant que la version du paquet diffère et que le dessin peut différer ; lorsque la version majeure ne possède que des versions plus anciennes, ou aucune, le paquet est introuvable. Une version plus ancienne n’est pas utilisée, car elle peut manquer de noms. Un fichier nommé par `path=` est vérifié selon la même règle.
- Sans `version=`, la version la plus récente trouvée est utilisée, avec une indication. Un `version=` qui n’est pas écrit comme l’indique 16.1, dans USE-PACKAGE, USE-ALIASES ou PACKAGE, est une erreur et n’est pas recherché : cet USE-PACKAGE ou USE-ALIASES échoue, et un fichier de paquet dont PACKAGE comporte cela n’est pas un paquet (ci-dessus). Le `version=` d’un paquet en ligne ne fait qu’identifier la version et n’est pas recherché ; mal écrit, c’est également une erreur, et ce paquet en ligne échoue.
- Un paquet trouvé par nom et version n’est lu qu’une seule fois dans un document : la même version constitue le même ensemble de définitions partout où il est utilisé. Un paquet trouve les noms des paquets qu’il référence, ainsi que du paquet standard en vigueur, là où il est utilisé : leur version et leur contenu en ligne sont décidés comme ci-dessous : par l’USE-PACKAGE le plus proche du niveau pour un paquet qu’un fichier utilise, et par la version la plus récente sur la chaîne des niveaux pour un paquet amené uniquement par d’autres paquets. Lorsque ceux-ci diffèrent entre deux endroits où le paquet est utilisé, il est lu une fois pour chacun, et les lectures forment des ensembles de définitions distincts, comme le sont deux versions (9.5). Chaque paquet en ligne est une copie propre : deux paquets en ligne sont distincts même lorsque leur texte est identique, de sorte qu’un paquet qui les trouve en deux endroits est lu une fois pour chacun. Des définitions de même nom dans des versions différentes sont deux définitions distinctes (9.5) ; voir l’exemple en 16.3.
- Les auteurs de paquets devraient suivre la règle du langage (16.1) : au sein d’une même version majeure, ne supprimez ni ne modifiez les noms, rôles, paramètres ou broches existants ; le dessin peut changer. C’est une convention que les outils ne peuvent pas vérifier.
- La présente version ne comporte aucun condensat de contenu pour les paquets (tel qu’un paramètre `digest=` d’USE-PACKAGE). Pour garder un dessin exactement identique, fixez la version, ou écrivez le paquet en ligne dans le fichier. La manière dont le `hash` d’un en-tête d’enregistrement est calculé dépend de l’outil (chapitre 17).

**Paquets qui référencent des paquets.** Un paquet peut référencer d’autres paquets, afin qu’un grand paquet puisse être scindé en parties et assemblé à nouveau en plusieurs tailles ; le paquet standard est construit ainsi (16.3). Un paquet dont le fichier n’a qu’un enregistrement d’en-tête et ne fait que référencer d’autres paquets s’appelle une *collection de paquets* ; la présente norme l’appelle habituellement simplement un paquet.

- **Forme.** Dans l’enregistrement d’en-tête d’un fichier de paquet, écrivez `@USE-PACKAGE(name=…, version=…)` pour chaque paquet référencé. Un `path=` est relatif au répertoire de ce fichier de paquet ; la recherche et les versions sont comme ci-dessus. Une référence n’a pas de corps ; une référence avec un corps est une erreur, car le contenu en ligne n’a pas de version et ne peut pas prendre part au choix de la version la plus récente (ci-dessous).
- **Ordre.** Lorsqu’un paquet est utilisé, les paquets que référence son enregistrement d’en-tête sont développés en premier, dans l’ordre des références, et placés avant lui ; un paquet référencé qui en référence d’autres est développé de la même façon en premier. Vient ensuite le contenu du paquet lui-même. Au sein d’un même développement, chaque paquet n’est placé qu’une seule fois, à sa première place. Ainsi le contenu d’une collection de paquets est le contenu des paquets qu’elle référence, les uns après les autres.
- **Noms.** Les noms en `$` d’un paquet référencé sont visibles dans le paquet qui le référence, et sont transmis à ses utilisateurs ; comme les noms en `$` du paquet lui-même, ils comptent comme appartenant au niveau juste au-dessus du niveau de l’USE-PACKAGE. Lorsque plusieurs paquets référencés définissent `$X`, la référence ultérieure masque l’antérieure, et une définition du paquet lui-même les masque toutes ; chaque masquage de ce type produit une indication, comme pour plusieurs paquets utilisés dans un même niveau. La présente version ne distingue pas les références destinées au propre usage du paquet de celles transmises à ses utilisateurs. Les noms en `%` restent privés à chaque paquet.
- **Un seul en vigueur.** Les paquets sont comptés par nom, comme dans « Quel USE-PACKAGE prend effet » ci-dessous, y compris ceux amenés par d’autres paquets, à l’endroit où ils sont développés.
  - Pour un paquet qu’un fichier utilise directement, l’USE-PACKAGE écrit dans un fichier qui est le plus proche du niveau prend effet, comme dans « Quel USE-PACKAGE prend effet » ci-dessous, avec sa version et son contenu en ligne, y compris là où d’autres paquets référencent le paquet. Lorsque sa version majeure diffère de celle que demande un paquet qui le référence, un avertissement est émis : le paquet qui le référence pourrait utiliser des noms qu’il ne possède pas.
  - Pour un paquet qui n’est amené qu’à travers d’autres paquets, la version la plus récente au sein d’une même version majeure parmi toutes les références sur une même chaîne de niveaux est utilisée, avec une indication lorsque les versions diffèrent ; au sein d’une même version majeure une version plus récente ne fait qu’ajouter, donc elle répond aussi aux références plus anciennes. Des références de versions majeures différentes sont une erreur : la référence développée plus tard échoue, et les instructions qui utilisent des noms qui en proviennent échouent avec elle (7.3).
- **En remplacer ou en écarter un** découle de « un seul en vigueur » ; il n’y a aucune règle supplémentaire. Un paquet en ligne vide de même nom l’écarte, tel que `@USE-PACKAGE(name="rnl-std-fill") {}`, ce qui désactive la complétion automatique ; une autre version ou un autre contenu en ligne sous le même nom le remplace. Lorsqu’un paquet écarté est utilisé par d’autres paquets, les instructions dans ceux-ci qui utilisent ses noms échouent.
- **Cycles.** Des références qui forment un cycle sont une erreur, et la référence qui ferme le cycle échoue. Quelle est cette référence dépend de l’endroit où commence le développement ; lorsque plusieurs paquets d’un cycle sont utilisés directement, le cycle est signalé une fois pour chacun d’eux.
- **Où les diagnostics sont signalés.** L’indication relative aux versions différentes et l’avertissement relatif à une version majeure différente sont signalés sur l’USE-PACKAGE écrit dans le fichier (les indications et avertissements à l’intérieur des paquets ne sont pas montrés à l’utilisateur, 7.6). L’erreur d’une référence qui échoue est signalée sur cette référence et indique quel USE-PACKAGE du fichier l’a amenée.
- Un paquet d’alias ne peut pas être référencé (16.5). USE-ALIASES dans l’enregistrement d’en-tête d’un fichier de paquet ne s’applique qu’à ce fichier, comme d’habitude.

**Un même nom en `$` dans plusieurs paquets.**

- Lorsque plusieurs paquets utilisés dans un même niveau définissent `$X`, celui utilisé plus tard masque le précédent, avec une indication.
- Un nom est recherché dans cet ordre : ce niveau → les paquets utilisés dans ce niveau (le plus tardif en premier) → le niveau supérieur → les paquets utilisés dans le niveau supérieur → … → les paquets à la racine (celui placé plus tard en premier).
- La présente version ne comporte ni noms qualifiés par paquet, ni autres noms pour les paquets ou étiquettes de niveaux (tels que `$acme.DIFFERENTIAL_PAIR` ou `as=`). Note : une version ultérieure qui les ajoute a besoin d’une autre forme qu’un `.` après le nom, qui est fréquent en fin de phrase (`align #M1 and #M2.`).

**Quel USE-PACKAGE prend effet.** Sur la chaîne de niveaux de la racine jusqu’à un niveau, un paquet n’est en vigueur qu’une seule fois. L’USE-PACKAGE écrit dans le niveau le plus proche de ce niveau prend effet, avec sa version et son contenu en ligne ; au sein d’un même niveau, le plus tardif compte comme plus proche, et un écrit dans l’enregistrement d’en-tête compte comme écrit tout au début du niveau de fichier. Un paquet amené par d’autres paquets compte comme écrit à l’USE-PACKAGE qui l’amène, avec sa version choisie comme dans « Paquets qui référencent des paquets » ci-dessus. Le paquet est placé à la position de cet USE-PACKAGE, ou à la racine lorsque cet USE-PACKAGE est dans l’enregistrement d’en-tête.

Note : sinon les règles génératrices d’un paquet, telles que la complétion de masse, s’exécuteraient une fois de plus dans un niveau inférieur et généreraient leurs objets deux fois.

**Utiliser un paquet à nouveau.** Un sous-circuit ou un fichier de bibliothèque peut utiliser le paquet standard à nouveau, à son niveau de fichier ou dans un sous-circuit. Les règles d’apparence du paquet standard sont alors placées après les règles qui descendent des niveaux supérieurs (telles que celles du fichier principal) et les surchargent à nouveau, et les noms en `$` du paquet, comptés comme appartenant au niveau juste au-dessus de cet USE-PACKAGE, masquent les définitions de même nom des niveaux supérieurs. Les symboles utilisés dans ce niveau ne sont alors pas affectés par les règles d’apparence des niveaux supérieurs (6.1).

- Cela n’affecte que les symboles et les noms. Les règles géométriques et génératrices qui descendent des niveaux supérieurs s’appliquent toujours ; lorsqu’une solution devient périmée à cause d’elles, une indication invitant à résoudre à nouveau est émise (chapitre 17).
- Pour fixer uniquement la version, tout en laissant le style des niveaux supérieurs surcharger le paquet, écrivez l’USE-PACKAGE dans l’enregistrement d’en-tête (16.3).

**Paquets en ligne.** USE-PACKAGE peut avoir un corps, qui est le contenu du paquet. Le fichier porte alors le paquet, et rien n’est recherché :

```rnl
@USE-PACKAGE(name="acme-style", version="1.2") {
  $NMOS_ACME = @DEFINE-SYMBOL {$NMOS; (-1, 0) @CIRCLE(radius=2.5)};
  # @SELECT-INSTANCE(type="nmos", recursive=true) => @SET-SYMBOL $NMOS_ACME
}
```

- Un paquet en ligne signifie la même chose qu’un USE-PACKAGE sans corps : le nom et la version du paquet, sa place à l’USE-PACKAGE, ses noms recherchés comme dans « Noms dans un paquet », ses noms en `$` comptés comme le niveau au-dessus, ses noms en `%` privés, et « un seul en vigueur » compté par nom de paquet valent tous comme ci-dessus. Le corps est consommé par inclusion fermée (5.4). Un paquet en ligne a la version de langage du fichier dans lequel il est écrit (16.1).
- Un paquet en ligne ne diffère d’un fichier de paquet qu’en ce que son contenu est écrit dans la netlist et qu’il n’a pas d’enregistrement d’en-tête : sa version de langage est celle du fichier, son `version=` ne fait qu’identifier la version, il ne peut déclarer ni DEVICE, ni SOLVER-CONFIG, ni ALIAS, et il ne peut pas référencer d’autres paquets. Chaque paquet en ligne est une copie propre. Ainsi déplacer un paquet en ligne hors du fichier garde la façon dont ses noms sont trouvés, mais pas toujours quelles définitions sont lesquelles : là où un fichier de paquet de même nom et version est déjà utilisé, les deux deviennent un même ensemble de définitions (ci-dessus), et des règles qui sélectionnent par définition, telles que SELECT-USAGE (9.5), peuvent alors atteindre davantage d’utilisations.
- Les paquets en ligne ne sont normalement utilisés que pour deux choses : écarter un paquet, avec un paquet en ligne vide de même nom (16.3), et figer tout un ensemble de paquets dans le fichier, avec un corps qui contient tout leur contenu développé et qui est complet par lui-même.
- Pour étendre le paquet standard, ou pour remplacer une de ses parties, écrivez un fichier de paquet dont l’enregistrement d’en-tête référence les paquets qu’il utilise (voir « Paquets qui référencent des paquets » ci-dessus), et pointez vers lui depuis la netlist avec `path=`. Un paquet en ligne muni d’un contenu qui remplace un sous-paquet du paquet standard compte comme faisant partie du paquet standard et ne voit que lui-même : il ne peut pas utiliser de noms provenant des autres sous-paquets du paquet standard.

**Autres règles.**

- Le traitement d’un paquet ne doit ni exécuter de code ni accéder au réseau implicitement (Principes 2.7).
- La façon dont une visionneuse affiche un dessin, et si elle peut modifier l’apparence selon les préférences de l’utilisateur (comme un arrière-plan sombre ou l’impression), sortent du cadre de la présente norme. Une propriété à laquelle aucun style ni paramètre ne donne de valeur prend la valeur par défaut du moteur de rendu (9.3, Principes 2.7).

### 16.3 Le paquet standard

- Le paquet standard ne fait pas partie de la présente norme. Il est écrit en RNL comme tout autre paquet et publié séparément ; ses définitions sémantiques, composants virtuels et autres contenus sont définis par le paquet lui-même.
- Les noms dans le paquet standard n’ont pas de préfixe obligatoire, tel que `$DIFFERENTIAL_PAIR_N`. Un utilisateur peut écrire une définition de même nom, qui remplace celle standard en la masquant (6.8).
- **Une collection de paquets.** Le paquet standard `rnl-std-all` n’a que des enregistrements d’en-tête : c’est l’unique collection de la série et il référence tous les autres paquets de la série à l’exception des deux paquets facultatifs nommés ci-dessous. Un fichier utilise la collection entière, ou écrit un par un les paquets qu’il souhaite.
  - `rnl-std-style` (les styles et les règles de style par défaut), `rnl-std-basic` et `rnl-std-semiconductors` (les symboles et les définitions, y compris les composants virtuels, tels que les masses, alimentations et étiquettes de port, et les ensembles de nets de masse, d’alimentation et de polarisation), ainsi que les paquets de règles des deux dialectes, qui attribuent leurs symboles aux instances : `rnl-std-spice` par lettre d’élément et type de modèle, `rnl-std-spectre` par master et type de modèle (voir les dialectes ci-dessous). Ceux-ci dessinent la netlist et n’ajoutent rien. L’enregistrement d’en-tête de `rnl-std-basic` contient `@SOLVER-CONFIG(junction="$JUNCTION")`, afin que le solveur dessine ses points de jonction avec la définition standard (13.2, 14.10).
  - `rnl-std-fill` contient les règles qui ajoutent des masses, alimentations, marques de polarisation et étiquettes de nets aux broches sans fils, chacune sur un raccord (stub) (10.5) ; il référence `rnl-std-basic`. Il sélectionne des broches, donc les broches que des composants virtuels ont sur un net sont complétées aussi, à l’exception de celles des composants qu’il place lui-même. Dans la série 0.4 il donne à chaque net de port de `$PORT_FILL_SET` une étiquette de port, en une colonne pour chaque niveau (12.4) ; aucun net n’est dessiné sous forme de fils à moins qu’un fichier ou un paquet ne le demande avec ROUTE-NET ou ROUTE-OBJECT, les nets de masse, d’alimentation et de polarisation sont écartés avec `@UNROUTE-NET(priority="high")`, et un fichier qui souhaite qu’ils soient dessinés sous forme de fils écrit `@ROUTE-NET(priority="high")` ou des WIRE (13.7). Les broches pourvues de fils, écrits à la main ou dessinés par ROUTE-NET ou ROUTE-OBJECT, écrits comme instructions ou comme règles, ne reçoivent aucun symbole de masse ou d’alimentation de la complétion ; un fichier qui en souhaite un l’attache (10.5). Les règles des mots-clés de routage sont évaluées avant la complétion, qui voit les arbres qu’elles créent (8.2, 10.2). Les ensembles `$GROUND_FILL_SET`, `$SUPPLY_FILL_SET`, `$BIAS_FILL_SET`, `$LABEL_FILL_SET` et `$PORT_FILL_SET` contiennent ce que la complétion doit encore ajouter : une fois qu’une résolution réécrit ses raccords et ses étiquettes (14.8), ils ne contiennent plus ces broches et ces nets, de sorte qu’un fichier ne construit aucune autre règle sur eux, telle qu’un TEXT sur leurs broches.
  - `rnl-std-semantics` contient les définitions des structures de circuits, telles que la paire différentielle, le miroir de courant et les flux de signal et de puissance, et des placements, tels que `$MIRRORED_LAYOUT` ; les contraintes dans leurs corps sont toutes de priorité `"low"` (9.5, 14.8). Une définition qui ne fait que placer, sans assertion ni routage, ne dit rien sur le circuit, et son nom se termine par `_LAYOUT`, tel que `$MIRRORED_LAYOUT`. `_N` et `_P` à la fin du nom d’une structure, comme dans `$DIFFERENTIAL_PAIR_N` et `$CURRENT_MIRROR_P`, donnent la polarité de ses transistors, que la structure affirme pour ceux dont le type est connu (15.3), ainsi que le dessin qui va avec, sources ou émetteurs vers le bas pour le type n et vers le haut pour le type p.
  - Les paquets de domaines particuliers sont `rnl-std-functional` (amplificateurs, comparateurs, convertisseurs et similaires), `rnl-std-rf`, `rnl-std-logic` et `rnl-std-signal-flow`.
  - Les noms dans les paquets ne se répètent pas, de sorte qu’un utilisateur n’a pas besoin de savoir de quel paquet provient un nom.
  - Le paquet facultatif `rnl-std-45deg` contient des symboles dessinés à 45 degrés, par exemple pour les ponts, les paires croisées et les enroulements en diagonale ; il ne figure pas dans la collection, et un fichier qui le souhaite écrit son USE-PACKAGE.
  - Le paquet facultatif `rnl-std-no-labels` désactive les étiquettes par défaut des symboles standards, pour les fichiers qui écrivent toutes leurs étiquettes eux-mêmes ; il ne figure pas dans la collection. Dans la série 0.4 il ne sélectionne que les objets dessinés (`drawn=true`, 10.1) et donne à leurs étiquettes de nom, de valeur et de modèle, et aux noms de nets que dessinent les symboles des utilisations, le style `textvisible=false` (9.3) ; un fichier réaffiche l’une d’elles en écrivant `textvisible=true` dessus (13.3). Le paquet facultatif `rnl-std-no-instance-labels` fait de même pour les seules instances dessinées, de sorte que les étiquettes des utilisations restent, comme les noms de nets des masses, des alimentations, des étiquettes de port et des étiquettes de net ; il ne figure pas non plus dans la collection. D’autres paquets peuvent être ajoutés ultérieurement. Toute la série standard est publiée ensemble sous un même numéro de version, et la collection fixe les versions de ses paquets.
  - Telle est la structure de la série standard 0.4 ; son `rnl-std-all` possède un enregistrement d’en-tête pour chaque langage (2.8).
- **Aucun chargement implicite.** Un fichier de version 0.4 n’utilise le paquet standard `rnl-std-all` que lorsqu’un USE-PACKAGE l’écrit, comme tout autre paquet (16.2). Un outil peut offrir un réglage qui l’ajoute lorsque l’enregistrement d’en-tête du fichier principal n’écrit `rnl-std-all` sous aucune forme, tel qu’une option `--include-std` sur sa ligne de commande ou un réglage d’une visionneuse, qui peut être activé par défaut. Lorsque ce réglage est activé, l’outil lit le fichier comme si son enregistrement d’en-tête sans `language` écrivait `@USE-PACKAGE(name="rnl-std-all", version="…")` en premier, avec la version la plus récente que l’outil possède de la série standard écrite dans la version de langage du fichier, et lorsqu’il écrit dans le fichier principal il écrit cet USE-PACKAGE dans son enregistrement d’en-tête, afin que le fichier soit dessiné de la même manière par tout outil. Une version fixée dans l’enregistrement d’en-tête d’un fichier est utilisée pour ce fichier et les fichiers qu’il inclut (ci-dessous).
  - Note : le paquet standard est un paquet comme un autre. Une netlist qui n’a pas été écrite pour RNL n’est dessinée avec les symboles standards que grâce à un tel réglage, et un fichier qui utilise les définitions du paquet standard, telles que `$GROUND`, écrit l’USE-PACKAGE, faute de quoi leurs noms sont introuvables. Chaque nom introuvable est une erreur comme d’habitude ; lorsqu’un paquet de la série standard qui n’est pas utilisé définit un tel nom, une indication le signale une fois pour le fichier, en nommant ce paquet et en renvoyant à USE-PACKAGE et au réglage de l’outil.
- **Le paquet de règles de l’autre dialecte.** Les noms d’instances signifient des choses différentes dans les deux dialectes : en SPICE, `I0` est une source de courant ; en Spectre, le nom d’une instance ne dit rien, et Virtuoso nomme `I0` l’instance d’un sous-circuit. La série standard 0.4 exprime cela avec `language` : sa collection `rnl-std-all` comporte un enregistrement d’en-tête sans `language` pour ce que partagent les deux langages, un avec `language="spice"` qui utilise `rnl-std-spice`, et un avec `language="spectre"` qui utilise `rnl-std-spectre` (2.8). Ainsi dans un fichier Spectre une section SPICE trouve ses symboles selon les règles de SPICE et les sections Spectre selon celles de Spectre.
  - Un outil qui fige la série 0.4 écrit la partie de chaque langage comme un paquet en ligne dans l’enregistrement d’en-tête de ce langage, et la partie partagée dans l’enregistrement d’en-tête sans `language` : un paquet en ligne contient des instructions, non des enregistrements, et ne peut pas porter `language` lui-même.
- **Paquets à la racine.** L’enregistrement d’en-tête d’un fichier principal ou d’un fichier inclus, parmi lesquels un fichier Verilog-A (2.9), peut contenir USE-PACKAGE :

  ```rnl
  * @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4")}
  ```

  - Sa portée est celle d’un USE-PACKAGE au niveau de fichier : le fichier qui l’écrit et les fichiers qu’il inclut. Un écrit dans un fichier inclus s’applique à ce fichier et aux fichiers qu’il inclut à son tour, non au fichier qui l’inclut (6.1).
  - Il est placé à la racine : avant les règles de tous les niveaux, et aussi avant les symboles de bloc (8.3, groupe 1). Ses noms en `$` sont à la racine aussi, si bien qu’une définition de même nom dans un niveau, par exemple dans le fichier principal, le masque toujours. Un USE-PACKAGE au niveau de fichier est placé à sa propre position à la place : il utilise le paquet à nouveau (16.2), si bien qu’il surcharge les règles qui descendent, et ses noms en `$` comptent comme appartenant au niveau supérieur. Écrit dans l’enregistrement d’en-tête, un paquet dit « ce fichier dépend de cette version, et le style des niveaux supérieurs peut encore le surcharger » ; écrit au niveau de fichier, il dit « ce fichier utilise le paquet pour fixer son propre dessin ».
  - Quel USE-PACKAGE prend effet est comme en 16.2 : celui écrit dans le niveau le plus proche de ce niveau, où un écrit dans l’enregistrement d’en-tête compte comme écrit tout au début du niveau de fichier.
  - L’ordre des paquets à la racine : le paquet standard `rnl-std-all` en premier, partout où un enregistrement d’en-tête l’écrit et aussi lorsqu’un réglage de l’outil l’ajoute, puis les autres dans l’ordre du premier fichier qui écrit chacun (du fichier le plus externe vers l’intérieur) et de son enregistrement d’en-tête. Lorsque l’enregistrement d’en-tête d’un fichier interne écrit le même paquet à nouveau, cela ne change que la version ou le contenu en ligne utilisé dans les niveaux de ce fichier ; la place demeure, donc fixer une version ne change pas l’ordre.
  - Un paquet dans un enregistrement d’en-tête qui définit un nom en `$` du paquet standard, ou d’un paquet placé à la racine avant lui, masque cette définition et produit une indication (6.8).
  - Fixer une version relève du choix de l’auteur. Un fichier principal devrait la fixer. Un fichier inclus qui ne la fixe pas suit le fichier qui l’inclut ; il ne devrait fixer une version que lorsqu’il dépend d’une version donnée. Les outils ne doivent pas ajouter de versions d’eux-mêmes aux fichiers inclus, à l’exception de la fixation qu’écrit la mise à niveau d’un fichier d’une version plus ancienne (16.1).
  - Une version ne devrait pas être fixée par un USE-PACKAGE au niveau de fichier du fichier principal : là, le paquet est placé après les symboles de bloc, et lorsque le fichier est inclus ailleurs, l’USE-PACKAGE utilise le paquet à nouveau (16.2) et modifie le sens du fichier.
- **Exemple.** L’enregistrement d’en-tête de `tb.sp` écrit `@USE-PACKAGE(name="rnl-std-all", version="1.2")`, et celui de `ota.sp`, que `tb.sp` inclut, écrit la version `"1.0"`.
  - Le niveau supérieur de `tb.sp` et les sous-circuits définis dans `tb.sp` utilisent `"1.2"` ; les sous-circuits dans `ota.sp` utilisent `"1.0"`. Les règles de style de `tb.sp` descendent tout de même dans `ota.sp` et surchargent les valeurs par défaut de `"1.0"`.
  - Les choix par défaut sont faits par des règles dans le paquet standard, telles que la règle qui attribue aux utilisations de `$GROUND` un symbole de masse. Chaque version sélectionne son propre `$GROUND` dans les niveaux qui l’utilisent, donc `ota.sp` est dessiné comme d’habitude.
  - Des définitions de même nom dans les deux versions sont deux définitions distinctes (16.2). Ainsi une règle de `tb.sp` qui sélectionne par définition, telle que `# @SELECT-USAGE(recursive=true) $GROUND => @SET-SYMBOL $MY_GND`, ne trouve que les utilisations du `$GROUND` de `"1.2"` ; `ota.sp` ne la suit que s’il ne fixe pas sa propre version. Les règles qui sélectionnent par les propriétés de l’hôte, telles que `# @SELECT-INSTANCE(type="nmos", recursive=true) => …`, ne sont pas affectées.
  - Pour que ses symboles soient affranchis des règles de `tb.sp`, `ota.sp` utilise le paquet à son niveau de fichier (16.2).
- **Moins du paquet standard.** Un réglage de l’outil n’ajoute que le nom de paquet `rnl-std-all`.
  - Pour omettre certains paquets, écartez-les avec des paquets en ligne vides de même nom : `@USE-PACKAGE(name="rnl-std-fill") {}` désactive la complétion automatique. Un paquet de même nom n’est en vigueur qu’une seule fois, jamais deux fois.
  - Pour n’en utiliser que quelques-uns, écrivez-les un par un, avec un `rnl-std-all` vide afin qu’un réglage de l’outil n’ajoute pas la collection en plus :

  ```rnl
  * @RNL(type="header") {
  *   @VERSION("0.4");
  *   @USE-PACKAGE(name="rnl-std-all") {};
  *   @USE-PACKAGE(name="rnl-std-style", version="0.4");
  *   @USE-PACKAGE(name="rnl-std-basic", version="0.4");
  *   @USE-PACKAGE(name="rnl-std-spice", version="0.4")
  * }
  ```

- **Étendre le paquet standard ou en remplacer une partie.** Écrivez un fichier de paquet dont l’enregistrement d’en-tête référence les paquets standards qu’il utilise, et ajoutez son propre contenu (16.2, paquets en ligne). Des définitions et des règles qui ne font qu’ajouter au paquet standard peuvent aussi être écrites directement dans la netlist ; les règles que possède le paquet standard ne peuvent être retirées ou remplacées que par un paquet de même nom.
- **Figer ou écarter tout le paquet standard.** Écrivez un paquet en ligne de même nom dans l’enregistrement d’en-tête. Cela découle de « un seul en vigueur », compté par nom de paquet ; il n’y a aucune règle supplémentaire :
  - `@USE-PACKAGE(name="rnl-std-all", version="1.2") {…}` fige le paquet standard dans le fichier, rendant le fichier autonome ; le corps est le contenu de ses paquets après développement, les uns après les autres dans l’ordre du développement (16.2). Réunis dans un même corps, deux paquets peuvent entrer en collision : un nom en `%` privé utilisé dans les deux, ou un nom en `$` que l’un masque à l’autre. Un outil qui fige renomme ce qui entre en collision, afin que chaque référence trouve toujours ce qu’elle trouvait dans son propre paquet, et écrit les instructions DEVICE et SOLVER-CONFIG des enregistrements d’en-tête des paquets figés dans les enregistrements d’en-tête du fichier, dans leur ordre et avant celles du fichier lui-même (2.8, 6.12, 14.10), puisqu’un paquet en ligne ne peut pas les contenir : chacune dans l’enregistrement d’en-tête du langage par lequel son paquet est utilisé, ou dans celui sans `language` pour un paquet utilisé sans langage ; le SOLVER-CONFIG d’un paquet utilisé uniquement par des enregistrements d’en-tête avec `language` n’est pas copié, car il est sans effet (14.10). Les copies viennent alors après les paquets qui venaient après celui figé. Lorsqu’un tel paquet écrit une clé de SOLVER-CONFIG qu’une copie écrit, ou un élément de DEVICE (noms de bornes ou type) pour des instances que concorde aussi un DEVICE copié, l’outil écrit cette instruction dans l’enregistrement d’en-tête qui contient la copie en collision, après elle et avant celles du fichier lui-même, lorsque le paquet est utilisé dans un enregistrement d’en-tête du même fichier, et signale la collision lorsqu’il est utilisé dans un fichier inclus ou dans un niveau. En dehors des collisions signalées, le paquet figé a le même sens ;
  - `@USE-PACKAGE(name="rnl-std-all") {}` n’utilise aucun paquet standard, même lorsqu’un réglage de l’outil en ajouterait un ; chaque instance a alors le symbole de bloc par défaut (9.2.6), et le reste de l’apparence est laissé aux valeurs par défaut du moteur de rendu (Principes 2.7).
- Les conventions telles que les bus en gras, les symboles de dérivation pour les bus et les marques pour la largeur d’un bus relèvent aussi du contenu du paquet standard. Sans paquet standard, chaque instance a le symbole de bloc par défaut (9.2.6), et le moteur de rendu décide du style de tracé (Principes 2.7).

### 16.4 Extensions et contenu inconnu

- **Contenu inconnu** (Principes 2.3). Les mots-clés, paramètres et valeurs inconnus sont des erreurs : l’instruction échoue, les instructions qui en dépendent échouent avec elle (5.5, 7.3), et le reste est interprété comme d’habitude (7.6). Les champs inconnus d’un en-tête d’enregistrement font exception et ne produisent qu’un avertissement (2.8). Dans SOLVER-CONFIG, une clé que le solveur ne connaît pas produit un avertissement, et une valeur non acceptée ne fait échouer que sa clé (14.10). Le dessin peut alors être incomplet, et les diagnostics doivent le signaler.
- **Gardé lors de l’enregistrement.** Principes 2.3 recommande de garder un tel contenu inchangé ; la présente norme l’exige :
  - les enregistrements qui ne sont pas ceux propres à l’outil ne sont pas modifiés (chapitre 17), et le texte hôte ainsi que les commentaires ordinaires sont laissés tels quels ;
  - lorsqu’un outil réécrit ses propres enregistrements, les instructions qui s’y trouvent qu’il ne connaît pas ou qui échouent sont gardées telles quelles, avec leur texte libre, dans le même ordre ;
  - un fichier dont la version n’est pas prise en charge est gardé entièrement tel quel (16.1) ;
  - les enregistrements qu’un outil réécrit n’ont pas besoin de rester identiques octet par octet.
- **Aucun mécanisme d’extension dans la présente version.** Les nouvelles sémantiques, composants virtuels et manières de dessiner s’écrivent comme des définitions, symboles et styles dans des paquets (1.5 point 7), et non comme de nouveaux mots-clés ; un nouveau mécanisme que les implémentations doivent comprendre attend une nouvelle version de la présente norme. Les données privées d’un outil, telles que les verrous ou l’état de vue d’un éditeur, ne s’écrivent pas dans les enregistrements RNL ; elles vont dans des fichiers de l’outil, ou dans des commentaires ordinaires en dehors des enregistrements.
  - Note : la présente version fait d’abord en sorte que tous les outils lisent le même RNL ; des dialectes d’éditeurs si tôt fragmenteraient le langage, et de nouvelles sémantiques peuvent déjà s’écrire dans des paquets.
  - Note : une version ultérieure pourra ajouter des extensions sous forme de mots-clés munis d’un préfixe fournisseur et d’un `.`, tels que `@ACME.ROUTE`, déclarés dans l’enregistrement d’en-tête : une extension requise qu’un outil ne connaît pas laisserait tout le fichier non interprété, et une extension facultative serait sautée avec une indication. Un tel mot-clé est aujourd’hui un mot-clé inconnu, une erreur, si bien que l’ajouter plus tard est un assouplissement pur (16.1) ; un préfixe fournisseur ne serait pas autorisé à porter le nom d’un mot-clé standard.
- Aucune forme n’est réservée pour les extensions : un `.` juste après un mot-clé standard est du texte libre, si bien que `@BUNDLE.LOCK` est BUNDLE suivi du texte libre `.LOCK`, et `{#M1 #M2} @GROUP.` en fin de phrase est valide (3.5).
- **Reconnu mais non interprété.** Dans la présente version, le contenu de cette sorte est un champ inconnu d’un en-tête d’enregistrement (2.8), et une clé de SOLVER-CONFIG que le solveur ne connaît pas (14.10). Les extensions facultatives en seraient une deuxième sorte, si une version ultérieure ajoute des extensions.

### 16.5 Alias

```rnl
* @RNL(type="header") {
*   @VERSION("0.4");
*   @USE-PACKAGE(name="rnl-std-all", version="0.4");
*   @USE-ALIASES(name="alias-zh");
*   @ALIAS("@at", "@PLACE")
* }
* @RNL {#M1 @位于(10, 20); {#M1 #M2} @对齐("y"); #M3 @at(x=4); #M1(左管) #M2(右管) #M5(尾管) @视作 $"差分对"}
```

- Les alias donnent aux mots standards d’autres graphies, pour la compatibilité avec un autre usage des majuscules et minuscules, ou pour écrire du RNL dans le style d’une autre langue naturelle, telle que le chinois.
- **D’où viennent les alias :**
  - des instructions ALIAS écrites directement dans l’enregistrement d’en-tête, pour quelques alias propres ;
  - des paquets d’alias, chargés avec `@USE-ALIASES(name=…, version=…, path=…)` dans l’enregistrement d’en-tête. Les alias d’une même langue sont réunis en un seul paquet, afin que les fichiers n’aient pas à les répéter et que la formulation reste identique partout. Le paquet d’alias chinois standard est `alias-zh` ; il est publié avec le paquet standard et accompagne les outils (16.3).
  - Les deux peuvent être utilisés ensemble, et plusieurs paquets d’alias peuvent être chargés ; ensemble ils sont vérifiés selon les règles de correspondance ci-dessous.
  - Les alias ne s’appliquent qu’au fichier qui les déclare et ne sont pas hérités par inclusion ; un fichier inclus ou un fichier de paquet qui utilise des alias les déclare dans son propre enregistrement d’en-tête. Le même mot standard peut avoir des alias différents dans des fichiers différents. Les clés des textes écrits dans un symbole sont vérifiées par rapport aux alias du fichier dans lequel le symbole est écrit (6.5, 13.3).
  - L’enregistrement d’en-tête lui-même est toujours lu dans l’orthographe standard et ne reconnaît pas les alias : tant que USE-ALIASES et ALIAS n’ont pas été lus, les alias ne sont pas connus, et les enregistrements d’en-tête sont le plus souvent écrits par des outils. Ainsi les alias des mots-clés qui ne peuvent être écrits que dans l’enregistrement d’en-tête (VERSION, ALIAS, USE-ALIASES, DEVICE, PACKAGE) ne servent à rien.
- **Paquets d’alias :**
  - Un paquet d’alias est un fichier de paquet (16.2) dont l’enregistrement d’en-tête ne contient que VERSION, PACKAGE et une liste d’instructions ALIAS, et qui ne comporte aucun autre enregistrement. Il est recherché, et ses versions sont choisies, comme en 16.2.
  - USE-ALIASES d’un fichier qui n’est pas un paquet d’alias, ou USE-PACKAGE d’un paquet d’alias, est une erreur. Lorsqu’un paquet d’alias est introuvable, son USE-ALIASES est une erreur, et les instructions qui utilisent ses alias échouent avec lui (7.3) ; le diagnostic donne le paquet d’alias manquant comme cause.
  - Note : les paquets d’alias ne sont pas chargés avec USE-PACKAGE parce qu’USE-PACKAGE peut être écrit dans n’importe quel niveau, porte sur des définitions et des règles, et est ordonné par position, tandis que les alias portent sur la façon dont le texte d’un fichier est lu et ne peuvent s’appliquer que par fichier. Tenus séparés, un paquet de style maison ne peut pas modifier la façon dont le texte est lu.
  - Le contenu d’un paquet d’alias suit ce à quoi il se réfère : lorsqu’un nom dans le paquet standard change, `alias-zh` change avec lui et sa version est relevée.
- **Forme.** `@ALIAS("alias", "original")` : l’alias en premier. Les deux paramètres sont `alias` et `original`, normalement donnés par position ; le troisième paramètre, `definition`, est donné par nom uniquement (ci-dessous).
- **Originaux :**
  - Un mot-clé ou mot standard défini par la présente norme, dans son orthographe standard. Un mot-clé s’écrit avec son `@`, tel que `"@PLACE"` ; les autres mots sans lui, tels que `"axis"` ou `"low"`. L’alias s’écrit de la même façon : l’alias d’un mot-clé porte `@`, les autres alias non. Le `@` distingue un mot-clé d’un mot standard de même graphie, et un alias apparaît dans le texte exactement tel qu’il est déclaré.
  - Un nom en `$`. L’alias et l’original portent tous deux le `$`, tel que `@ALIAS("$接地补位集", "$GROUND_FILL_SET")` ; dans le texte il s’écrit `$"接地补位集"` (un nom non ASCII après `$` exige des guillemets, 3.6) et se lit comme `$GROUND_FILL_SET`. Les noms en `%` n’ont pas d’alias : les auteurs peuvent donner directement leurs propres noms dans une autre langue, et les noms en `%` dans les paquets sont privés.
  - Un nom dans une définition, déclaré avec `definition=`, tel que `@ALIAS("左管", "left", definition="$DIFFERENTIAL_PAIR_N")` ou `@ALIAS("间距", "gap", definition="$GROUND_STUB")`. Un tel alias n’est recherché qu’à l’intérieur de cette définition : noms de rôles lorsqu’elle est appliquée (`#M1(左管) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N`), noms de paramètres qui lui sont passés par AS et ATTACH (`$GROUND_STUB @ATTACH(间距=3) …`), et noms locaux pris sur ses utilisations via `::` (`%c1::"中点"`). La valeur de `definition` peut elle-même être écrite comme un alias. Les alias de mots standards ne sont pas utilisés à ces endroits, à l’exception de `priority`, le paramètre d’AS et d’ATTACH eux-mêmes.
    - Les alias suivent le nom : ils suivent le nom en `$`, et s’appliquent aussi lorsqu’un niveau inférieur masque la définition par une autre de même nom, tout comme un alias d’un nom en `$` en remplace la graphie.
    - Lorsque l’alias est déclaré, l’original n’est pas vérifié par rapport à la définition ; lorsque la définition est appliquée, un nom introuvable est une erreur comme d’habitude (telle que « aucun rôle in_p »).
    - Les noms après `::` sont recherchés dans l’ordre de 6.5 : les noms de l’utilisation elle-même, leurs alias `definition=`, les noms de sous-objets standards et leurs alias, les noms des broches du symbole de l’objet, les clés des textes.
    - La valeur de la condition de sélection `role=`, ainsi qu’un chemin dans `${…}` qui descend le long des rôles, reconnaissent aussi ces alias, recherchés dans la définition de l’utilisation à chaque étape, tels que `%dp1 @SELECT-INSTANCE(role="左管")` ou `${左管::w}`, pour une utilisation `%dp1` de `$DIFFERENTIAL_PAIR_N`. Lorsque la portée contient des utilisations de plusieurs définitions, chacune est recherchée dans sa propre définition ; les originaux fonctionnent toujours.
- **Ce qui peut avoir des alias :** tous les mots de la liste de mots sauf ceux énumérés au point suivant : mots-clés, noms d’emplacements des mots-clés intégrés, noms et valeurs de paramètres, noms de sortes, noms de sous-objets, noms d’attributs, propriétés de style, `true` et `false` (5.3) ; ainsi que les noms en `$` et les noms dans les définitions ci-dessus.
- **Ce qui ne peut pas avoir d’alias :** la forme externe, c’est-à-dire `@RNL`, les noms de champs d’un en-tête d’enregistrement et les valeurs de `type`, et `@VERSION` ; ainsi que `@ALIAS` et `@USE-ALIASES` eux-mêmes.
- **Autres noms.** Un nom est déjà un autre nom d’un objet : après `$"差分对" = $DIFFERENTIAL_PAIR_N`, `$"差分对"` est la même définition. Un tel nom fait partie du texte, ne peut plus être retiré, et n’atteint ni les noms de rôles ni les noms de paramètres ; pour un jeu complet de noms dans une autre langue, utilisez des alias.
- **Correspondance :**
  - Un original peut avoir plusieurs alias ; au sein d’une même portée un alias ne désigne qu’un seul original. Les alias `definition=` de chaque définition forment une portée.
  - Un alias ne doit avoir la graphie d’aucun mot standard de la version que déclare le fichier, et ne doit pas être déclaré à nouveau pour un autre original. Un alias `definition=` ne doit avoir la graphie d’aucun nom de rôle, de paramètre ou nom local que possède la définition, ni d’un alias de `priority`, le paramètre d’AS et d’ATTACH eux-mêmes.
  - Un alias ne peut pas être utilisé comme original, et ne peut pas être identique à son original. Lorsque la version d’un fichier est relevée et qu’un mot standard nouvellement ajouté a la graphie d’un alias, un avertissement est émis et la version est relevée tout de même (16.1) ; dans la nouvelle version l’alias a alors la graphie d’un mot standard, ce que la règle ci-dessus rend une erreur.
  - Un paquet d’alias doit déclarer la version de langage du fichier qui l’utilise (16.1) ; sinon son USE-ALIASES est une erreur.
- **Les alias ne font qu’ajouter des graphies.** Les graphies standards fonctionnent toujours, et les deux peuvent être mélangées dans un même fichier.
- **Les alias ne remplacent que des jetons entiers :**
  - un alias remplace un jeton entier ; ce n’est pas une macro textuelle, si bien que les positions et les diagnostics restent exacts ;
  - les mots standards sans `@` reçoivent des alias selon leur graphie, si bien que les mots de même graphie (tels que le nom de paramètre `x` et la valeur `"x"`) partagent leurs alias ;
  - un alias d’un nom en `$` remplace la graphie : un auteur qui écrit `$"接地补位集" = {…}` définit `$GROUND_FILL_SET` dans ce niveau, ce qui masque celui du paquet standard avec une indication (6.8).
- **Règles lexicales :**
  - Un alias d’un mot-clé est constitué des lettres (de n’importe quelle écriture), chiffres et tirets bas qui suivent directement `@`, sans `-` ; ils forment un seul jeton, et le `@` doit être précédé d’une espace (2.3), y compris après une ponctuation chinoise : `对 &b，@选出元件` est une erreur, et doit s’écrire `对 &b， @选出元件`. Tout caractère qui n’est pas une lettre, un chiffre ou un tiret bas termine l’alias : une espace, ou un signe de ponctuation de n’importe quelle écriture, tel que `，` ou `。`. Ainsi `@对齐两个器件` est un seul mot-clé inconnu, une erreur, et son diagnostic doit demander s’il manque une espace après `@对齐`. Il n’existe aucune règle propre à telle ou telle langue : ce qui n’est pas une lettre, un chiffre ou un tiret bas de n’importe quelle écriture termine un alias, et les caractères non ASCII hors des parenthèses et crochets sont du texte libre (3.3). Les jetons ne sont pas découpés selon les alias déclarés, afin que la lecture des jetons n’ait besoin d’aucune déclaration.
  - Les mots non ASCII sans guillemets n’apparaissent qu’à deux endroits : après `@`, et à l’intérieur des parenthèses et crochets. Après `@`, le mot doit être un alias déclaré ; sinon c’est une erreur (« un nom non ASCII exige des guillemets »). À l’intérieur des parenthèses et crochets (noms de paramètres, noms d’emplacements des mots-clés intégrés, `true` et `false`, noms de rôles), un alias déclaré est un alias ; à la place d’un nom de rôle ou d’un nom de paramètre d’une définition, un mot qui n’est pas un alias est le nom lui-même (3.6) ; aux autres endroits, un mot qui n’est pas un alias est une erreur. Ailleurs, 3.6 s’applique : les noms hôtes, noms RNL, noms de bornes et noms après `::` n’ont que des caractères ASCII lorsqu’ils ne sont pas entre guillemets, et un alias utilisé après `::` est aussi mis entre guillemets, tel que `#M1::"位置"`.
  - Un alias sans guillemets dans des parenthèses ou crochets qui a la graphie d’un nom de paramètre ou d’un nom de rôle d’une définition utilisable au même endroit est une erreur, car son sens n’est pas unique. Un mot entre guillemets dans des parenthèses ou crochets est toujours un nom, jamais un alias. Les alias de noms en `$` et les alias après `::` s’écrivent de toute façon entre guillemets (3.6).
  - Les alias de valeurs d’énumération s’écrivent à l’intérieur des chaînes et ne fonctionnent que dans les paramètres dont les valeurs forment une énumération. Les noms d’attributs dans `${…}` reconnaissent aussi les alias (13.6). Les autres chaînes, telles que le `text` de TEXT, ne sont pas remplacées.
- **Diagnostics et traduction :**
  - Les diagnostics affichent les mots tels qu’écrits dans le fichier, accompagnés de leur graphie standard.
  - Un traducteur opère mécaniquement dans les deux sens. En traduisant vers des alias, il écrit USE-ALIASES dans l’enregistrement d’en-tête ; pour un fichier autonome il écrit plutôt les alias utilisés sous forme d’instructions ALIAS. Lorsqu’un original a plusieurs alias, le premier déclaré est retenu. La retraduction vers l’orthographe standard supprime USE-ALIASES et ALIAS et ne perd rien.
  - Le texte libre, les commentaires et les chaînes ne sont pas traduits, à l’exception des valeurs d’énumération et des noms d’attributs dans `${…}`.
  - Les LLM, les solveurs et les éditeurs écrivent l’orthographe standard. Lorsqu’ils modifient un fichier écrit avec des alias, les instructions qu’ils ajoutent sont également dans l’orthographe standard ; le mélange est valide.

<!-- translated from en/ch17.md sha256:801f28995842 -->
## 17 Modèle de modification

- RNL ne décrit que l’état actuel, et non l’édition qui y a mené :
  - il n’y a aucune opération d’annulation ni de suppression ; supprimer consiste à éditer le texte ;
  - il n’y a aucun déplacement relatif ; un éditeur écrit de nouvelles valeurs absolues.
- **Qui modifie quels enregistrements.** Le solveur remplace les enregistrements portant `type="generated"`. Les autres outils ne doivent modifier que les enregistrements dont `author` est eux-mêmes, par exemple un éditeur qui met en ordre les modifications qu’il a écrites, ou une nouvelle importation qui remplace ce que l’importation précédente a écrit. Les outils ne doivent pas toucher aux enregistrements d’auteur dépourvus d’`author` (2.8).
- Les descriptions ajoutées plus tard surchargent les précédentes, dans l’ordre (8.4) ; pour les faits géométriques, la priorité est comparée en premier (8.5). `type` et `author` n’affectent pas l’ordre (8.3).
- **La sortie va avec le sous-circuit.** La sortie du solveur et les modifications qu’un utilisateur effectue dans un éditeur sont écrites dans le texte propre du niveau : pour un sous-circuit, à l’intérieur de son `.subckt … .ends`, même lorsqu’il se trouve dans un fichier inclus ; pour le niveau supérieur, au niveau de fichier du fichier principal. Le solveur écrit sa sortie avec `priority="high"` à la fin du texte propre du niveau ; lorsqu’il résout, les enregistrements portant `type="generated"` sont exclus de l’interprétation (2.8), et par la suite il les remplace (14.8). Il doit réécrire chaque décision qu’il prend (14.8). `scope=` ne sert qu’aux instructions que l’auteur choisit d’écrire à l’extérieur (2.6). Le fait qu’un fichier en lecture seule soit enregistré dépend de l’outil.
- **Fichiers Verilog-A.** Un niveau de module n’a rien à résoudre et ne reçoit aucune sortie du solveur. Dans un fichier Verilog-A, un outil ne modifie que l’enregistrement d’en-tête et les enregistrements de symboles de bloc, à l’intérieur de leurs modules, écrits avec `//` (2.9).
- **Une solution périmée ne donne qu’une indication.** Lorsqu’un niveau supérieur ou un paquet change, la solution d’un sous-circuit dans une bibliothèque peut devenir périmée, par exemple lorsque le fichier principal change un symbole. L’ancienne solution prend toujours effet ; une visionneuse qui constate que le `hash` dans l’en-tête d’enregistrement ne correspond pas à l’entrée actuelle émet une indication invitant à résoudre à nouveau (2.8).
  - L’empreinte est calculée à partir de l’entrée effective du niveau après héritage : les règles qui descendent et prennent effet, les symboles choisis, les versions des paquets et les options du solveur, sans les enregistrements portant `type="generated"` et sans les champs de métadonnées. Elle est calculée par niveau, non par fichier, et non à partir du seul texte propre du sous-circuit, sans quoi des modifications dans le fichier principal passeraient inaperçues.
  - L’empreinte ne modifie pas l’interprétation. Son algorithme appartient aux outils ; la valeur devrait porter une étiquette pour l’algorithme, telle que `"sig1:…"`, et un outil qui ne connaît pas l’algorithme ne la compare pas.
  - Note : si une empreinte qui ne concorde pas signifiait que l’ancienne solution n’est pas utilisée, l’algorithme deviendrait une partie du langage, chaque outil devrait le calculer de la même façon, et une visionneuse qui ne résout pas ne pourrait pas dessiner le niveau.
  - Les numéros font exception : lorsque les instructions qui créent des objets sans nom changent, la vérification des numéros ne concorde plus, et les instructions qui utilisent des numéros restent sans effet jusqu’à ce que le niveau soit résolu à nouveau (5.9). La vérification ne couvre que le texte du niveau et des définitions qu’il utilise, si bien que chaque outil la calcule de la même façon, et le reste de l’ancienne solution est toujours dessiné.
- **Objets générés.** Lorsque le solveur a placé et câblé des objets que génèrent des règles, il les écrit sous forme d’instructions explicites dans son propre enregistrement, accompagnées des fils. L’objet explicite prend le pas, et les règles de complétion ne génèrent plus le même objet (10.5). Les objets dans leurs corps qui n’ont pas de nom local sont désignés par numéro (5.9).
- **Modifier ce que le solveur a créé.** Un nom que le solveur donne à un objet qu’il écrit explicitement n’est lié que dans son propre enregistrement, qui est écarté lorsque le niveau est résolu (2.8). Ainsi une instruction d’un autre enregistrement qui utilise ce nom ne trouve rien lors de la résolution, et le solveur ne peut pas la prendre en compte. Pour décrire un tel objet isolément, un éditeur l’écrit à la main dans son propre enregistrement, comme l’indique 10.5, et ajoute la modification, telle que `%g_r4 = $GROUND_STUB @ATTACH #R4:2; %g_r4::g @PLACE(30, -4, priority="high")` : le fil du raccord (stub) rend la broche câblée, la complétion la saute, et la résolution suivante n’écrit plus cet objet ; jusque-là, l’objet dans l’enregistrement du solveur est dessiné également. Lorsque l’objet modifié est l’utilisation générée elle-même, telle qu’une étiquette de port, une sélection depuis l’objet hôte l’atteint aussi bien lors de la résolution que lors du dessin, telle que `&out @SELECT-ANCESTOR-USAGE $IO_PORT => @PLACE(40, 20, priority="high")` ; les objets locaux d’une utilisation ne peuvent pas être sélectionnés via l’utilisation, mais uniquement par nom ou comme objets du niveau (9.4).
- Les champs d’un en-tête d’enregistrement s’appliquent à chaque instruction de l’enregistrement (2.8).
- **Exemple : une modification dans l’éditeur.** Voici un fragment complet, avec les préfixes de commentaire de l’hôte. Après l’enregistrement d’en-tête, le concepteur a écrit le premier enregistrement et le solveur le second ; l’utilisateur a déplacé VIN vers la gauche à la souris dans l’éditeur, et l’éditeur a ajouté le troisième :

  ```spice
  * Common-source stage
  * @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4")}
  .model nch nmos level=1
  M1 out in 0 0 nch W=4u L=1u
  RD vdd out 5k
  VDD vdd 0 1.8
  VIN in 0 DC 0.6 AC 1
  * @RNL {
  *   {#RD #M1} @ALIGN("x");
  *   #M1 #RD @DISTANCE("bottom-to-top", min=2);
  *   &vdd @ADD-TO $SUPPLY_NET_SET
  * }
  * @RNL(type="generated", priority="high", author="rnl-solver 0.3", hash="sig1:5be1c09a") {
  *   #M1 @PLACE(10, 0); #RD @PLACE(10, 8); #VIN @PLACE(0, -2); #VDD @PLACE(20, 4)
  * }
  * @RNL(author="rnl-editor 0.1") { #VIN @PLACE(x=-4, priority="high")}
  .end
  ```

  - Le glissement est écrit comme un nouvel enregistrement, ajouté après l’enregistrement du solveur, avec la priorité `"high"`. Tous deux sont de priorité haute, si bien que le plus tardif est retenu et que le x écrit par le solveur est écarté, avec une indication (8.5) ; chaque fois que le niveau est résolu à nouveau, le nouveau x est une position fixe (14.8).
  - L’écrire dans l’enregistrement du concepteur ne fonctionne pas. Les instructions qui s’y trouvent sont `"normal"` par défaut et ne peuvent pas surcharger le `"high"` du solveur ; écrites comme `"high"`, elles perdent tout de même, car l’enregistrement du solveur vient plus tard, et à l’intérieur d’une même priorité le fait plus tardif est retenu. Cette erreur d’usage n’est pas une erreur ; elle produit seulement une indication sur la résolution. La position prend effet la prochaine fois que le niveau est résolu, lorsque l’instruction du concepteur fait partie des contraintes ; jusque-là le dessin ne change pas.
  - Les paramètres donnés par position viennent avant ceux donnés par nom (4.2 point 9) : `@PLACE(-4, -2, priority="high")` équivaut à `@PLACE(x=-4, y=-2, priority="high")`, et `@PLACE(x=-4, -2)` est une erreur.
  - Pour modifier ce qu’a écrit le concepteur (changer un symbole, renommer, supprimer une contrainte), éditez directement l’enregistrement du concepteur. L’enregistrement du solveur est laissé tranquille, et est remplacé en bloc la prochaine fois que le niveau est résolu.
- Pour désactiver une instruction momentanément, encadrez-la dans `/* … */`.
- **Allers-retours.** RNL ne garantit pas qu’un dessin reste identique après n’importe quelle exportation vers RNL suivie d’une réimportation (Principes 1) ; un exportateur peut choisir d’écrire du RNL exact au niveau des coordonnées (Principes 3). La présente version ne définit ni ce qu’est le RNL exact, ni de critères pour les allers-retours.

<!-- translated from en/annex-a.md sha256:2adc09eec4f4 -->
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

<!-- translated from en/annex-c.md sha256:e15f7ba635ad -->
## Annexe C Réservé pour les versions ultérieures (informative)

Cette annexe énumère ce que cette version de la norme laisse de côté et indique pouvoir apparaître dans une version ultérieure, avec les sections qui le mentionnent. Cette version ne réserve aucun mot-clé ni aucune forme d’écriture pour ces éléments (5.3, 16.4), et une version ultérieure n’est pas tenue de les ajouter.

| Réservé pour une version ultérieure | Voir |
|---|---|
| Porteurs pour d’autres hôtes, y compris les commentaires de bloc (tels que `#com … #endcom` d’Eldo), et la gestion d’un terminateur de commentaire de l’hôte que cela entraîne | 2.2, 2.5 |
| Identifiants d’enregistrement ; les outils trouvent leurs enregistrements par `type`, `author` et les clés des instructions | 2.8 |
| Expressions | 3.7 |
| Phrases standards qui indiquent si une définition s’applique à un ensemble entier ou une fois pour chaque membre (`each`) | 4.5 |
| Emplacements facultatifs ; sans eux une sélection a toujours une portée, ne serait-ce que `#` | 5.3, 10.1, 13.7 |
| Plusieurs indices développés dans une seule référence, et d’autres formes d’indices, telles que `<*2>A`, `A<0:2*2>` et `D[0..7]` | 6.11 |
| Codes de diagnostic standards | 7.6 |
| Un moyen de supprimer les diagnostics dans un fichier, et un mode strict, par exemple comme champs de la tête d’enregistrement | 7.6 |
| Unités de plusieurs faits : une définition ou une utilisation dont les faits sont retenus ou écartés ensemble lors de la résolution de conflits. Points encore ouverts : la priorité d’une telle unité, les utilisations imbriquées, et les priorités écrites de l’extérieur via `::` | 8.5 |
| Mise à l’échelle de symboles et d’instances | 9.2.1, 11.7 |
| Flèches aux extrémités des fils | 9.2.5 |
| Davantage de styles de ligne | 9.3 |
| Noms de couleurs et `rgb()` | 9.3 |
| Remplissages par motif, dégradés et ombres | 9.3 |
| L’intersection d’ensembles dans une même instruction | 10.1 |
| Choix du symbole d’une primitive d’après ses paramètres. Pour quelques primitives de chaque dialecte, le simulateur décide du genre d’après la ligne : en SPICE, une source B écrite avec `V=` est une source de tension et une écrite avec `I=` est une source de courant. Les sous-circuits et les modèles ne varient pas ainsi. Une possibilité est une condition de SELECT-INSTANCE telle que `param=` (la ligne comporte ce paramètre) ; une autre est un type que l’adaptateur du dialecte attribue à ces primitives (6.12), sélectionné avec `type=`. Le nombre de bornes est sélectionné avec `terminals=` (10.1). D’ici là, le paquet standard dessine les deux genres selon ses règles actuelles | 10.1, 6.12 |
| Rotation d’un ensemble entier comme un seul bloc avec ORIENT | 11.2 |
| Plusieurs dessins d’un même objet hôte, en tant qu’objets distincts ayant leurs propres noms | 11.4 |
| Un élément graphique qui encadre un groupe d’objets | 13.4 |
| Annotations ancrées sur des fils : positions le long d’un fil (telles que `t` ou `offset`), et symboles ou flèches ancrés au milieu d’un fil | 13.3, 15.2 |
| Références dans `${…}` aux attributs d’autres objets, telles que `${R3:VALUE}` de KiCad | 13.6 |
| Les projections diagonales `"x+y"` et `"x-y"` pour les contraintes | 14.1 |
| Un USE-PACKAGE qui s’applique uniquement à son propre niveau | 16.2 |
| Un paquet réparti sur plusieurs fichiers | 16.2 |
| Un format de manifeste pour une netlist et ses fichiers de paquet | 16.2 |
| Un condensat de contenu pour les paquets, tel qu’un paramètre `digest=` de USE-PACKAGE | 16.2 |
| Références qu’un paquet n’utilise que pour lui-même, séparées de celles qu’il transmet à ses utilisateurs | 16.2 |
| Noms qualifiés par le paquet, autres noms pour les paquets et étiquettes de niveau, tels que `$acme.DIFFERENTIAL_PAIR` ou `as=`, sous une forme autre qu’un `.` après le nom | 16.2 |
| Un mécanisme d’extension : mots-clés avec un préfixe fournisseur, tels que `@ACME.ROUTE`, déclarés dans l’enregistrement d’en-tête comme extensions requises ou facultatives | 2.8, 16.4 |
| Le RNL exact, c’est-à-dire les conditions dans lesquelles un fichier peut être dessiné sans solveur, et les critères pour les allers-retours ; ceux-ci peuvent plutôt être convenus entre outils | 17 |

<!-- translated from en/annex-d.md sha256:72752f1bf7f8 -->
## Annexe D Modifications par rapport à la version 0.3 (informative)

La version 0.4 écrit chaque fait portant sur les positions avec sa direction, de sorte qu’un fait désigne une unique disposition et non un choix entre plusieurs. Les versions 0.x ne font aucune promesse de compatibilité entre elles (16.1) ; cette annexe énumère ce qui a changé et comment un fichier passe à la version 0.4. La présente norme décrit uniquement la version 0.4 ; un fichier de la version 0.3 passe à la 0.4 au moyen d’un outil de mise à niveau qui lit la version 0.3, lequel effectue les modifications indiquées à la fin de cette annexe (16.1).

| Modification | Voir |
|---|---|
| Les directions sont écrites explicitement. ORDER et DISTANCE n’acceptent que les quatre directions ; `"x"` et `"y"`, qui laissaient la direction ouverte, sont des valeurs de paramètre erronées. DISTRIBUTE prend une direction au lieu d’un axe, dans le paramètre `direction` (et non plus `axis`) : ses membres se suivent strictement dans cette direction, par pas égaux, et deux membres forment un ORDER. ALIGN prend une seule projection, pas une liste | 14.1–14.5, 8.5 |
| ORIENT et ORIENT-RELATION prennent une liste de codes, telle que `["R0", "MY"]` : un seul fait, qui vaut lorsque l’un d’eux est satisfait ; le solveur choisit et réécrit un unique code. L’orientation est la seule sorte de fait qui offre un tel choix | 11.2, 14.9, 8.5 |
| ORIENT-RELATION relie un objet à un objet. Un ensemble dans un emplacement est traité membre par membre, un fait pour chaque membre ; un ensemble de plus d’un membre dans les deux emplacements est une erreur. La version 0.3 reliait deux ensembles comme des touts | 14.9, 5.2.1 |
| Places des membres : une référence à un ensemble suivie directement de `{…}` prend des membres selon leur place, comptée sur ses membres dépliés d’un niveau, comme `%a{1}`, `%a{end}`, `%a{2:end}` ; une utilisation prend ses membres par rôle, et les places sur une sélection sont une erreur. En 0.3, `%a{#b}` désignait deux objets | 5.2.3, 3.2, 3.3, 3.5, 4.1, 6.10 |
| Les contraintes ne portent que sur les objets les plus bas. Un ensemble n’a pas de position : ni centre ni étendue, et son `::position` est l’ensemble des positions de ses membres. Un ensemble garde ses éléments tels qu’ils sont écrits ; dans la liste de membres d’ALIGN, d’ORDER et de DISTRIBUTE, un élément qui est un ensemble (un `{…}` ou une référence à un ensemble) vaut pour chacun de ses membres, une contrainte pour chaque combinaison, chacune constituant une unité de résolution ; DISTANCE relie chaque membre d’un emplacement à chaque membre de l’autre. En 0.3, une référence dans un ensemble était dépliée, `{%x}` comptait par son centre, et DISTANCE comparait les étendues | 14.1, 14.3–14.5, 5.2, 5.2.1, 6.5, 8.5, 11.1, 11.2 |
| Une utilisation n’est plus dessinée au point de l’objet qui remplit son premier rôle : son symbole est dessiné à son point propre, et une définition qui dessine une marque sur une broche attache son point à la broche dans son corps. Une broche qui lie un rôle est dessinée sur le net de ce qui le remplit, si bien que `$GROUND @ATTACH #M5:3` attache une masse au net de M5:3, à une position qui lui est propre ; un numéro dans le symbole d’une utilisation ne lie aucun rôle. En 0.3, cette utilisation était dessinée sur la broche et n’avait pas de position propre | 12.1, 12.2, 12.5, 12.6, 6.5, 8.2, 9.2.7, 11.2, 11.3, 13.2, 14.1, 15.2 |
| Une définition générale peut ne pas avoir de rôles (`roles=[]`), appliquée comme `$PORT_LIST @ATTACH`. Son corps peut contenir des règles : elles agissent comme des règles du niveau de l’utilisation, à la position de son instruction, et `#` dans le corps désigne ce niveau. Un `?` isolé dans une définition générale accepte un chemin, tel que `?::position` | 9.4, 8.2, 10.4, 4.1 |
| La présente norme décrit uniquement la version 0.4 : les fichiers des versions 0.3 et 0.2 relèvent d’autres versions majeures, migrés à l’aide d’un outil de mise à niveau qui lit la version 0.3, vers le langage 0.4 et la série standard 0.4. La norme 0.3 décrivait comment un outil lit les fichiers de la version 0.2 | 16.1 |
| Le paquet standard n’est plus utilisé implicitement : un fichier écrit `@USE-PACKAGE(name="rnl-std-all", …)`, ou un outil l’ajoute via un réglage, tel que `--include-std` ou un réglage d’une visionneuse, et l’écrit dans le fichier lorsqu’il écrit le fichier. En 0.3, il était utilisé à la racine de chaque fichier | 16.3, 16.1, 16.2, 6.1 |
| Les coordonnées littérales dans le corps d’une définition générale, de points et de PLACE, sont des décalages par rapport au point propre de l’utilisation, et l’ORIENT de l’utilisation ne les fait pas pivoter. En 0.3, c’étaient des coordonnées du niveau | 9.4, 11.1, 11.2, 13.4 |
| DISTANCE énonce également l’ordre : ses bornes ne sont pas négatives, et un `min` absent vaut 0. En 0.3, un DISTANCE avec une direction acceptait des bornes de l’un ou l’autre signe | 14.5 |
| Règles et phases : les règles de WIRE disparaissent, et BUNDLE, WIRE ainsi que RECTANGLE, CIRCLE et ELLIPSE écrits avec deux points n’acceptent ni le résultat d’une sélection, ni un ensemble qui en dépend ; les règles des quatre mots-clés de routage sont évaluées à une étape propre, l’étape 5, avant la génération, qui voit leurs arbres ; les règles de LINE, ARC et BEZIER ainsi que de CURRENT sont évaluées à l’étape 6 avec les règles de génération ; un CURRENT sur une broche prend effet une fois les symboles attribués. En 0.3, ces règles prenaient effet dans la phase de leur mot-clé | 10.4, 8.2, 11.8, 13.1, 9.2.5, 15.2 |
| Chaque utilisation a son point propre, même sans symbole, écrit `%u::position` : PLACE s’y applique, ainsi que les contraintes sur une utilisation dessinée avec un symbole. Dans la liste des membres d’une contrainte, une utilisation sans symbole représente ses membres, jusqu’aux objets les plus bas, comme dans GROUP, et toute combinaison qui prend deux fois un même objet est omise. Un `?` isolé dans le corps d’une définition générale représente l’utilisation en cours de développement. Une indication est émise lorsqu’un point contraint d’une utilisation n’est attaché à rien de dessiné. En 0.3, PLACE et les contraintes sur une utilisation sans symbole échouaient | 14.1, 9.4, 12.1, 6.2, 6.5, 3.3, 7.1, 8.2, 11.1 |
| Routage sélectif : ROUTE est remplacé par ROUTE-NET (un net entier, comme le faisait ROUTE), ROUTE-OBJECT (les broches de certains objets, net par net), UNROUTE-NET et UNROUTE-OBJECT (broches écartées). Chaque broche est décidée par l’instruction de plus haute priorité qui la couvre, puis la plus tardive. Les arbres sur un même net qui partagent une broche ou sont reliés par des fils écrits forment un même arbre. Une broche dans un arbre compte comme `wired` | 13.7, 10.1, 10.2, Annexe A |
| Le solveur peut scinder un fil écrit par l’auteur, pour effectuer un branchement depuis son milieu, sur un net qu’un arbre relie ; il réécrit le fil dans son enregistrement et garde les points de l’auteur dans l’ordre | 13.1, 13.7 |
| Les groupes ne sont pas des objets : aucune position, aucune contrainte sur un groupe. Les membres d’un groupe sont les objets dans son ensemble, jusqu’aux plus bas ; une utilisation dessinée avec un symbole est elle-même un membre ; ROUTE-OBJECT suit aussi ses rôles et prend également ses propres broches. L’arbre des groupes, ainsi que le fait de maintenir les éléments extérieurs hors de la région d’un groupe, appartiennent au solveur comme critères après chaque fait. GROUP prend `spacing_x` et `spacing_y`, l’espacement à l’intérieur du groupe en unités de grille | 11.5, 13.7 |
| SELECT-INSTANCE accepte la condition `terminals`, le nombre de bornes de l’instance, de sorte que, par exemple, les transistors MOS à trois et à quatre bornes reçoivent des symboles différents | 10.1, Annexe A |
| La primitive Spectre `tline` a les noms de bornes par défaut `p1`, `n1`, `p2` et `n2`, comme T en SPICE ; le tableau des familles de modèles ajoute les maîtres (masters) et la famille MESFET | 6.12 |
| Une instance à laquelle aucune instruction ni règle ne donne de symbole a le symbole de bloc par défaut : un cadre carré centré sur son origine, avec une broche par port, ou par nœud pour une instance dont la définition n’est pas lue ; aucune instance n’est laissée non dessinée, et une visionneuse ne dessine plus de figure de remplacement qui lui soit propre | 9.2.6, 9.2.8 |
| Le champ `language` de l’en-tête d’enregistrement fait qu’un enregistrement n’agit que sur les objets situés dans des sections de ce langage de netlist ; un fichier peut avoir un enregistrement d’en-tête pour chaque langage en plus de celui partagé. La série standard 0.4 l’utilise pour donner aux sections SPICE d’un fichier Spectre les règles de SPICE ; la série 0.4 n’écarte pas implicitement le paquet de l’autre dialecte, comme le faisait la série 0.3 | 2.8, 2.2, 6.12, 16.3, Annexe C |
| Le langage dans lequel commence un fichier est une entrée fournie à l’outil, par défaut d’après son suffixe, et l’utilisateur peut le modifier pour chaque fichier | 2.2 |
| Par défaut, les règles s’appliquent à leur propre niveau : `recursive` vaut `false` par défaut, et une règle comportant une sélection écrite en elle n’atteint les niveaux inférieurs qu’avec `recursive=true` sur l’une de ces sélections, tandis qu’une règle sans sélection va aussi loin que les ensembles qu’elle utilise ; chaque sélection et chaque ensemble de la règle sélectionnent alors dans chaque niveau. En 0.3, chaque sélection avait son propre `recursive`. Une règle dans un paquet compte comme écrite dans le niveau de son USE-PACKAGE, et un paquet à la racine dans le niveau supérieur de son fichier, de sorte que les règles du paquet standard écrivent `recursive=true` ; une règle de génération n’avertit sur les objets de son niveau que lorsqu’elle atteint les niveaux inférieurs. Une règle qui atteint les niveaux inférieurs et dont d’autres emplacements nomment des objets hôtes de son niveau est sans effet dans les niveaux inférieurs, sans avertissement | 10.1, 10.3, 10.4, 10.5, 6.1, 8.3, 16.2, 4.5, Annexe B, Principes 2.5 |
| SOLVER-CONFIG garde les réglages du solveur dans le fichier, ou dans l’enregistrement d’en-tête d’un paquet pour les niveaux qui l’utilisent : paramètres nommés dont les clés sont définies par le solveur, parmi lesquels la norme ne nomme que `spacing_x`, `spacing_y` et `junction`, la définition des points de jonction avec lesquels chaque solveur dessine ; les réglages ultérieurs remplacent les précédents clé par clé ; le cœur transmet les réglages au solveur tels qu’ils sont écrits, dans l’ordre ; une valeur non acceptée ne fait échouer que sa clé, et la valeur d’avant reste ; les budgets de temps ou d’effort ne sont donnés que par l’outil | 14.10, 14.8, Annexe C |
| CURRENT n’est pas un fait géométrique, seulement une indication pour le solveur ; ORDER compare des points, et maintenir les boîtes des symboles séparées est un critère du solveur ; ORDER est un raccourci pour un DISTANCE avec `min=1` entre voisins ; un moteur de rendu sans résultat dessine le premier code d’une liste ORIENT | 15.2, 14.3, 11.2 |
| Clarifié : un TEXT au contenu vide ne dessine rien ; le solveur remplace les enregistrements générés en entier, qu’il comprenne ou non leurs instructions ; les bornes d’un faisceau se correspondent par nom, et les membres qui diffèrent par leurs noms de bornes ou leur modèle produisent un avertissement | 13.3, 2.8, 11.8 |
| Les broches sont des objets, de sorte `pin`, distincts des bornes. Une broche est créée par PIN dans un symbole et nommée par son nom local, `#X1::vdd_t` ; `terminal=` peut être omis. Une borne peut avoir plusieurs broches ; là où une broche est requise, une borne vaut pour sa broche, la dernière écrite. Les broches ne se remplacent plus l’une l’autre par borne. Un nom de broche ne peut pas être identique à un nom de rôle, un nom local, un nom standard de sous-objet ou une clé de texte ; `::` recherche les noms de broches avant les clés de textes | 9.2.4, 9.2.7, 5.1, 6.4, 6.5, 13.3 |
| SET-SYMBOL associe les noms de bornes du symbole aux bornes de l’objet par nom, ou par place pour les numéros ; `terminals=` modifie l’association, énumérée dans l’ordre des nœuds ou sous forme de paires | 11.3 |
| Les utilisations n’ont pas de bornes : `%g:p` de la version 0.3 est désormais une broche nommée telle que `%g::pin`. Les broches d’une utilisation lient des rôles par nom et sont dessinées sur ce qui les remplit ; ce ne sont pas des bornes électriques | 12.1, 6.4 |
| Les objets donnés pour une autre sorte ne sont convertis que selon un tableau : vers le dessin, de borne à broche puis à point, et d’objet dessiné à position ; vers le circuit, de broche à borne, et de broche ou borne à net | 5.2.4 |
| SELECT-PIN sélectionne des broches ; `wired` appartient aux broches et aux nets ; SELECT-TERMINAL sélectionne toute borne, avec broche ou non, et les étapes parcourant des bornes les comptent toutes. SELECT-INSTANCE et SELECT-USAGE acceptent `symbol` et `drawn`. RECTANGLE, CIRCLE et ELLIPSE écrits avec une taille prennent un par un le résultat d’une sélection ; la condition `type` accepte le joker `*`, et `type="*"` sélectionne les instances qui ont un type | 10.1, 10.2, 8.2, 9.2.5, 13.3 |
| Assertions : ASSERT-INSTANCE, ASSERT-TERMINAL, ASSERT-NET et ASSERT-USAGE vérifient que les membres satisfont aux conditions de la sélection de leur sorte, et ASSERT-USAGE vérifie aussi qu’ils sont des utilisations d’une définition ; ASSERT-IN vérifie que les membres appartiennent à un ensemble. Elles vérifient que des attributs sont écrits (`has`) et sont identiques (`same`) ou différents (`different`) ; ASSERT-TOPOLOGY vérifie que les membres sont connectés en série, en parallèle, en étoile ou en boucle simple, une utilisation ou un ensemble comptant comme une boîte noire ; une vérification qui n’est pas satisfaite produit une violation, une quatrième sorte de signalement à côté de l’erreur, de l’avertissement et de l’indication, et ne fait échouer aucune instruction. Les instructions CURRENT d’un même net vérifient ensemble que le courant ne s’écoule pas entièrement dans un seul sens ; en 0.3, rien n’était vérifié | 15.2, 15.3, 15.4, 7.6, Annexe C |
| TABLE dispose les objets en tableau de `{…}` imbriqués, en lignes de haut en bas (ou avec `transpose=true` en colonnes de gauche à droite), chaque liste imbriquée à angle droit ; un seul fait, et ses objets forment un même groupe | 14.11 |
| Texte : mode mathématique `\(…\)` avec un sous-ensemble de LaTeX mathématique ; `\subrest{…}` et `\subdigits{…}` affichent le reste d’un nom, ou ses chiffres à la fin, en indice ; TEXT prend `fix_rotate`, qui garde le sens des caractères tandis que le côté de la boîte de texte continue de suivre l’orientation de l’objet | 13.6, 13.3 |
| Les alias s’arrêtent à tout caractère qui n’est ni une lettre, ni un chiffre, ni un tiret bas, dans quelque écriture que ce soit ; aucune règle ne nomme de ponctuation chinoise. Le paquet d’alias chinois est `alias-zh`. Les phrases standards ne sont données qu’en anglais | 16.5, 4.5 |
| La liste des mots ajoute `removed`, les mots que la 0.4 n’a plus ; le schéma ajoute `open_params` et `removed`, et `at_least_one` devient une liste de groupes, comme le sont `together` et `exclusive`. | 5.3 |
| Le nœud vide, `&` seul, indique qu’il n’y a pas de net : un rôle de sorte `net` et l’emplacement de net de WIRE l’acceptent, afin que des composants virtuels puissent dessiner un schéma-bloc sans lignes de netlist ; ce n’est pas un objet, et deux endroits qui l’écrivent n’ont rien en commun | 6.3, 9.4, 12.1, 13.1 |
| Dans un enregistrement de niveau, `junction` de SOLVER-CONFIG peut nommer une définition `%` de ce niveau, afin qu’une variante de point de jonction définie pour un sous-circuit reste propre à celui-ci | 13.2, 14.10 |
| `$NO_CONNECT` prend une borne telle quelle, afin qu’une marque sur un membre d’un bus indique quelle borne est laissée ouverte | 12.6 |
| TEXT : `rotate` accepte n’importe quel angle ; `upright` (`true` par défaut) oriente un texte pour la lecture, et avec `false` les caractères suivent la rotation et la symétrie miroir, pour des signes tels que des flèches ; la propriété de style de texte `textvisible` masque un texte, qui n’occupe alors aucune place tout en gardant son contenu | 9.2.5, 9.3, 13.3 |
| ANNOTATION rassemble les éléments graphiques d’un symbole, d’une définition générale ou d’une utilisation en une seule annotation, en un point ou sur un objet, tournée d’un angle quelconque ; les broches et le reste sont laissés de côté, tout comme les éléments graphiques d’un corps attachés à d’autres objets | 13.5 |
| Une implémentation peut fixer des limites aux ressources, telles que la profondeur d’imbrication ou le nombre d’objets ; une entrée dépassant une limite produit une erreur de l’implémentation, jamais un résultat tronqué en silence ; au moins 64 niveaux d’imbrication de symboles sont pris en charge | 1.6 |
| Une implémentation fixe une limite au temps ou au travail qu’elle consacre à la vérification des faits ; cette norme ne fixe pas comment la limite est mesurée | 8.5 |
| DEVICE prend `type`, qui donne à une instance à laquelle l’hôte n’attribue aucun type, telle qu’un composant de PDK, le type que lisent les sélections et les assertions ; `name` ou `model`, et `terminals` ou `type`, peuvent chacun être omis mais pas les deux, et séparément pour les noms et le type, c’est le dernier DEVICE correspondant qui les écrit qui compte. Les noms de bornes par défaut du `transformer` de Spectre sont `p1 n1 p2 n2` | 6.12 |
| Le résultat d’une sélection est dans un ordre défini : l’ordre de la scène, ou membre de portée par membre de portée sans doublons, les objets créés par des règles après ceux qui sont écrits | 10.3 |
| Les broches pointent dans une direction dans le dessin, que contraignent ORIENT et ORIENT-RELATION écrits sur des broches ; ORIENT accepte les mots `left`, `down`, `right` et `up` pour R0, R90, R180 et R270 | 9.2.4, 11.2, 14.9 |
| Un module Verilog-A inclus avec `ahdl_include` ou `.hdl` peut donner à ses instances un symbole de bloc, écrit dans des enregistrements `//` à l’intérieur du module ; le module est un niveau sans circuit qui ne contient que des symboles et des styles, et un port vectoriel est une borne pour chaque bit, dans l’ordre de sa plage déclarée | 2.9, 2.2, 6.1, 9.2.6, 17 |

**Passer un fichier de la version 0.3 à la version 0.4.** Cette norme ne décrit pas les fichiers de la version 0.3 ; un outil de mise à niveau qui lit la version 0.3 les migre (16.1). Il élève la version dans l’enregistrement d’en-tête, fait passer le fichier à la série standard 0.4 et applique les changements ci-dessous ; ce qu’il ne peut pas modifier, il le signale à l’auteur :

- `"x"` ou `"y"` comme direction d’ORDER ou de DISTANCE, et `axis=` pour DISTRIBUTE : choisir une direction.
- Les noms de bornes `p_plus`, `p_minus`, `s_plus` et `s_minus` d’un `transformer` Spectre : écrire `p1`, `n1`, `p2` et `n2`.
- Ensembles des deux côtés d’ORIENT-RELATION : écrire un fait pour chaque paire.
- Règles sans `recursive` : un outil écrit `recursive=true` dans leurs sélections, dans le fichier, ses propres fichiers de paquet et ses paquets en ligne, afin qu’elles continuent d’atteindre les niveaux inférieurs. Une règle qui écrit `recursive=false` sur certaines de ses sélections et pas sur d’autres est signalée : en 0.4, un seul `true` fait atteindre les niveaux inférieurs à toute la règle.
- ROUTE : un outil écrit ROUTE-NET à sa place.
- WIRE ou BUNDLE recevant le résultat d’une sélection : écrire ROUTE-OBJECT pour les fils, et les membres d’un faisceau un par un.
- `%g:p` et autres bornes d’utilisations : écrire la broche nommée du symbole, telle que `%g::pin`. Les symboles de la série 0.3 lient les rôles des utilisations par numéro, ce que la 0.4 ne fait pas (11.3).
- `$GROUND @ATTACH #M5:3` et autres composants virtuels dont le premier rôle est une borne ou une broche : ils se placent désormais sur ce net à une position qui leur est propre ; pour les garder sur la broche, écrire un raccord, `$GROUND_STUB @ATTACH #M5:3`, ou des contraintes qui placent la broche de la masse sur la broche de M5 (12.3).
- `wired=` sur SELECT-TERMINAL : écrire SELECT-PIN, par exemple `# @SELECT-INSTANCE => @SELECT-PIN(wired=false)`.
- `{%x}` compté par son centre, et `::position` d’un ensemble, qui était son centre : un ensemble n’a pas de position en 0.4, et son `::position` est l’ensemble des positions de ses membres ; la mise à niveau laisse une telle instruction à vérifier. Pour placer les objets d’un seul tenant, faites-en les membres d’une utilisation dont le corps les attache à son point propre, et contraignez `%u::position` (14.1).
- Une utilisation dans la liste des membres d’une contrainte, qui comptait par un seul point : la mise à niveau écrit `%u::position` pour une utilisation sans symbole, qui sinon représenterait ses membres (14.1).
- DISTANCE entre deux ensembles : elle vaut désormais pour chaque paire, de sorte qu’un `max` borne chaque paire et non l’écart entre les étendues.
- Coordonnées littérales dans le corps d’une définition générale : ce sont des décalages par rapport au point propre de l’utilisation ; une définition qui visait les coordonnées du niveau place ses utilisations en `(0, 0)`, comme `$TITLE @ATTACH => @PLACE(0, 0)`.
- Le paquet standard : écrire `@USE-PACKAGE(name="rnl-std-all", version="0.4")`, et réécrire les utilisations de la série 0.3 comme l’indique « La série standard 0.4 » ci-dessous : les nouveaux noms et rôles, une étiquette de port `$IO_PORT @ATTACH` pour chaque net de port que la complétion de la 0.3 marquait, et un signalement pour chaque utilisation d’une définition supprimée.

**La série standard 0.4.** Le paquet standard ne fait pas partie de cette norme (16.3) ; ce qui suit est ce qui importe pour un fichier passant de la série 0.3 à la série 0.4.

- **L’utiliser** : un fichier écrit `@USE-PACKAGE(name="rnl-std-all", version="0.4")` dans son enregistrement d’en-tête, ou est lu avec un réglage de l’outil qui l’ajoute (16.3).
- **Ports** : la complétion donne à chaque net de port une étiquette de port, y compris aux nets d’alimentation et de masse, en une colonne à au moins 6 unités à gauche des instances du niveau, chaque étiquette à R0 ; un fichier retire des nets de `$PORT_FILL_SET`, écrit `$IO_PORT @ATTACH` pour une étiquette qui lui est propre, ou `$PORT_LIST @ATTACH` pour la colonne écrite explicitement (12.4).
- **La complétion** : elle ajoute des étiquettes de net, chacune étant un court raccord, aux broches sans fils ; aucun net n’est dessiné par des fils à moins qu’un fichier ou un paquet ne le demande avec ROUTE-NET ou ROUTE-OBJECT ; les nets de masse et d’alimentation sont écartés avec `@UNROUTE-NET(priority="high")`, et un fichier qui souhaite les voir dessinés écrit `@ROUTE-NET(priority="high")` ou des WIRE. La complétion n’ajoute aucun symbole de masse ou d’alimentation aux broches qui ont des fils, que le fichier les câble à la main ou les dessine avec ROUTE-NET ou ROUTE-OBJECT, écrits comme instructions ou comme règles ; un fichier qui en veut un l’attache, par exemple `$GROUND @ATTACH &vss` pour un net dessiné en entier ou `$GROUND_STUB @ATTACH #R1:2` sur une broche. Ses règles écrivent `recursive=true` et sélectionnent des broches, y compris celles des composants virtuels. Les nets de `$BIAS_NET_SET` reçoivent des marques de polarisation comme les nets de masse et d’alimentation reçoivent les leurs.
- **Les paquets** : la collection unique est `rnl-std-all`, qui référence chaque paquet de la série et possède un enregistrement d’en-tête pour chaque langage (2.8, 16.3) ; les collections `rnl-std`, `rnl-std-core` et `rnl-std-full` de la série 0.3 ont disparu, et `rnl-std-structures` devient `rnl-std-semantics`. `rnl-std-no-labels` sélectionne avec `drawn=true` et masque les étiquettes avec le style `textvisible=false` au lieu de textes vides, si bien qu’un fichier en réaffiche une avec `textvisible=true`. Le paquet facultatif `rnl-std-45deg` est nouveau.
- **Renommés ou modifiés** : `$DIFFERENTIAL_PAIR` devient `$DIFFERENTIAL_PAIR_N` et `$DIFFERENTIAL_PAIR_P`, avec les rôles `left`, `right` et `tail` ; `$MIRRORED` est renommé `$MIRRORED_LAYOUT`, avec les rôles `left`, `right` et un `axis`, et la nouvelle `$MIRRORED_VERTICAL_LAYOUT` a `top`, `bottom` et `axis`, puisqu’une définition qui ne fait que placer se termine par `_LAYOUT` ; les structures `_N` et `_P` affirment la polarité de leurs transistors ; `$FLOW` devient `$SIGNAL_FLOW`, qui ne place rien, ne prend aucune direction et peut nommer les sources et les charges de son chemin, et `$POWER_FLOW` et `$BIAS_CIRCUIT` sont nouveaux ; `$CURRENT_MIRROR` devient `$CURRENT_MIRROR_N` et `$CURRENT_MIRROR_P` ; le rôle de `$NO_CONNECT` accepte une borne ou une broche. La liste complète des définitions que la série renomme, fusionne, scinde ou supprime figure dans la description du paquet standard ; la commande `rnl upgrade` dans la dernière version du cœur RNL 0.5.x (une version de l’outil, pas de ce langage) réécrit les fichiers d’après celle-ci. Chaque fait de position dans les structures énonce sa direction, et l’autre dessin constitue une variante propre, telle que `$BRANCH_HORIZONTAL`.
- **Supprimés** : `$CHAIN`, `$SUPPLY_HIGH` et `$SUPPLY_LOW`.
