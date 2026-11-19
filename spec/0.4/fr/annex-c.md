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
