<!-- translated from en/annex-b.md sha256:f5f5cbcacdf4 -->
## Anexo B Sentencias estándar (informativo)

Una oración estándar es una sentencia en la forma estándar con texto libre añadido entre sus tókenes, de modo que la sentencia se lea como una oración. El texto libre no cambia el significado (3.3); las reglas de redacción figuran en 4.5. En las tablas, `…` al comienzo de una oración representa los segmentos anteriores en una tubería, y `{…}` o un `…` final el contenido que se omite.

### B.1 Colocación y orientación

| Palabra clave | Caso | Oración |
|---|---|---|
| PLACE | dos ejes | `#M1 is at @PLACE(10, 20)` |
| | un eje | `the x of #M1 is at @PLACE(x=10)` |
| | escrito por un editor | `the x of #X2 is at @PLACE(x=3, priority="high")` |
| | en una tubería | `… => it is at @PLACE(40, 0)` |
| ORIENT | rotado | `#M2 @ORIENT("R90")` |
| | reflejado | `#M2 @ORIENT("MX"), i.e., flipped upside down` |
| | | `#M2 @ORIENT("MY"), i.e., flipped left-right` |
| | | `#M3 @ORIENT("MXR90"), i.e., flipped upside down, then rotated 90°` |
| | | `#M3 @ORIENT("MYR90"), i.e., flipped left-right, then rotated 90°` |
| | cualquiera de dos | `#M5 @ORIENT(["R0", "MY"]), i.e., not turned or flipped left-right` |
| | en una tubería | `… => they @ORIENT("R90")` |
| ORIENT-RELATION | dos objetos | `from #M1 to #M2, they have @ORIENT-RELATION("MY"), i.e., flipped left-right` |
| | cualquiera de dos | `from #M3 to #M4, they have @ORIENT-RELATION(["R0", "MY"]), i.e., the same or flipped left-right` |
| | la misma orientación | `from #M5 to {#M7 #M8}, they have @ORIENT-RELATION("R0"), i.e., the same orientation` |
| LAYER | | `#X is on @LAYER("bottom")` |
| | en una tubería | `… => they are on @LAYER("top")` |
| GROUP | | `{#M1 #M2 #M5} as a @GROUP` |
| | nombrado | `%s1_set = {#M1 #M2 #M5} as a @GROUP` |
| | espaciado | `{#M3 #M4} as a @GROUP(spacing_x=-1), i.e., one grid unit tighter in x` |

### B.2 Restricciones de disposición

| Palabra clave | Caso | Oración |
|---|---|---|
| ALIGN | misma y | `{#M1 #M2} @ALIGN("y"), i.e., same y` |
| | misma x | `{#M1 #C1} @ALIGN("x"), i.e., same x` |
| | prioridad baja | `{#M7 #M5} @ALIGN("y", priority="low"), i.e., same y` |
| | en una tubería | `… => they @ALIGN("y")` |
| ORDER | con una dirección | `{#M1 #M2 #M3} are in @ORDER("left-to-right")` |
| | en una tubería | `… => they are in @ORDER("bottom-to-top")` |
| DISTRIBUTE | | `{#R1 #R2 #R3} are in @DISTRIBUTE("left-to-right"), i.e., in equal steps` |
| | en una tubería | `… => they are in @DISTRIBUTE("bottom-to-top"), i.e., in equal steps` |
| DISTANCE | con una dirección | `from #M1 to #M2, they have @DISTANCE("bottom-to-top", min=4)` |
| | desde un conjunto | `from {#M6 #M7 #CL} to %pout, they have @DISTANCE("left-to-right", min=2)` |
| TABLE | | `@TABLE layout: {{#M1 #M2 #M3} {#M4 {#M5 #M6} #M7}}` |
| | transpuesta | `@TABLE(transpose=true) layout: {{#M1 #M2} {#M3 #M4}}` |
| PRIORITY | | `%c1 has @PRIORITY("disabled")` |
| | en una tubería | `… => they have @PRIORITY("low")` |
| SOLVER-CONFIG | | `@SOLVER-CONFIG(spacing_x=2, spacing_y=1)` |

- `the x of` (o `the y of`) se escribe solo antes de un PLACE que da un eje (B.1); ALIGN lleva en su lugar la glosa `, i.e., same x`.
- La x es la de la posición de cada miembro: para una instancia, el punto que fija PLACE (el origen de su símbolo), no el centro de su contorno; un conjunto representa a cada uno de sus miembros (14.1).

### B.3 Apariencia

| Palabra clave | Caso | Oración |
|---|---|---|
| SET-SYMBOL | un objeto | `#M1 @SET-SYMBOL to $NMOS` |
| | un conjunto | `{#M1 #M2} @SET-SYMBOL to $NMOS` |
| | el símbolo de bloque de un subcircuito | `this subcircuit # @SET-SYMBOL to $NAND2_SYM` |
| | en una tubería | `… => they @SET-SYMBOL to $PMOS` |
| SET-STYLE | | `{#CC #RZ} @SET-STYLE to $orange` |
| | en una tubería | `… => they @SET-STYLE to $grey` |
| DEFINE-STYLE | | `$orange = @DEFINE-STYLE(linecolor="#e07000", textcolor="#e07000") {}` |
| | fusionando otro estilo | `$wide = @DEFINE-STYLE(linewidth=2) based on {$orange}` |

### B.4 Símbolos y gráficos

| Palabra clave | Caso | Oración |
|---|---|---|
| DEFINE-SYMBOL | | `$TRI = @DEFINE-SYMBOL {…}` |
| PIN | | `at (-4, 0) there is a @PIN(terminal="in", direction="left")` |
| LINE | | `through {(0, 0) (4, 0)} runs a @LINE` |
| ARC | | `through {(0, 2) (2, 0) (0, -2)} runs an @ARC` |
| BEZIER | | `the control points {(0, 0) (1, 1) (2, 1) (3, 0)} make a @BEZIER` |
| CIRCLE | | `centered at (0, 0), a @CIRCLE(radius=1)` |
| RECTANGLE | centro y tamaño | `centered at (0, 0), a @RECTANGLE(width=4, height=2)` |
| | dos esquinas | `with corners {(0, 0) (4, 2)}, a @RECTANGLE` |
| ELLIPSE | centro y tamaño | `centered at (0, 0), an @ELLIPSE(width=4, height=2)` |
| | esquinas de su cuadro delimitador | `in the box with corners {(0, 0) (4, 2)}, an @ELLIPSE` |
| TEXT | en un símbolo | `on ? there is a @TEXT("${name}", key="name") => it is at @PLACE(-2, 2.5)` |
| ATTACH | en un símbolo | `$ARROW is @ATTACH to (0, -1.5) => …` |

- Los gráficos escritos en la escena, como anotaciones (13.4), se leen igual.

### B.5 Cables y texto

| Palabra clave | Caso | Oración |
|---|---|---|
| WIRE | | `net &n1, through {#R1:2 %j #R2:1}, has a @WIRE` |
| | un bus | `net &D<3:0>, through {%p1 %p2}, has a @WIRE, which is a bus` |
| | un modo de trazado | `net &in, through {#VIN:1 #R1:1}, has a @WIRE(mode="vertical-horizontal")` |
| ROUTE-NET | | `net &n1 is drawn with @ROUTE-NET` |
| | un conjunto | `nets {&a &b} are drawn with @ROUTE-NET` |
| | en una tubería | `… => it is drawn with @ROUTE-NET` |
| ROUTE-OBJECT | | `the pins of {#M1 #M2 #M5} are joined with @ROUTE-OBJECT` |
| UNROUTE-NET | | `net &vdd is left out with @UNROUTE-NET(priority="high")` |
| UNROUTE-OBJECT | | `the pins of #M3 are left out with @UNROUTE-OBJECT` |
| TEXT | sobre un objeto | `on #M1 there is a @TEXT("W/L = ${w}/${l}", key="wl", dy=-3)` |
| | en un punto | `at (_, _) there is a @TEXT("RC low-pass") => named %title` |
| | en una tubería | `… => on them there is a @TEXT("${w}", key="size")` |

- Nombrar un punto, como `%j = (_, _)`, no lleva palabras añadidas.

### B.6 Definiciones y aplicaciones

| Palabra clave | Caso | Oración |
|---|---|---|
| DEFINE | | `$PAIR = a definition, @DEFINE(roles=["a", "b"], parameters=[["gap", 2]]) {…}` |
| AS | tres funciones | `#M1(left), #M2(right) and #M5(tail) are treated @AS $DIFFERENTIAL_PAIR_N` |
| | una función que toma un conjunto | `{#M5 #M7 #M8} are treated @AS $MATCHED` |
| | un objeto | `#M3 is treated @AS $DIODE_CONNECTED` |
| | con una prioridad | `#M1(left), #M2(right) and %ax(axis) are treated @AS(priority="high") $MIRRORED_LAYOUT` |
| | un parámetro de la definición | `{#R1 #R2} are treated @AS(gap=6) $SPACED` |
| | un uso en una función | `%dp(pair) and %cm(load) are treated @AS $OTA` |
| | nombrado | `%dp = #M1(left), #M2(right) and #M5(tail) treated @AS $DIFFERENTIAL_PAIR_N` |
| | nombrado en una tubería, con otro objeto | `#M1(left), #M2(right) and #M5(tail) are treated @AS $DIFFERENTIAL_PAIR_N => named %dp => it and %cm are treated @AS $OTA` |
| ATTACH | en una red | `$GROUND is @ATTACH to &vss` |
| | en un terminal | `$NO_CONNECT is @ATTACH to #M1:4` |
| | en un punto | `$JUNCTION is @ATTACH to %j` |
| | dos funciones | `$DIFF_PORT is @ATTACH to &inp(a) and &inn(b)` |
| | seguido de otros segmentos, nombrado | `$GROUND is @ATTACH to &vss => it is at @PLACE(24, 38) => named %g0` |
| | en cada uno de varios objetos | `$SIZE_LABEL is @ATTACH to {#M1 #M2 #M3}` |
| | en un conjunto | `$GROUND_STUB is @ATTACH to $GROUND_FILL_SET` |
| | el relleno, en una tubería | `among the instances at each level in # and below, @SELECT-INSTANCE(recursive=true) => among the pins of them, @SELECT-PIN(net="vss", wired=false) => $GROUND_STUB is @ATTACH(gap=3) to them` |

- AS se usa cuando los objetos juntos forman la entidad, como una estructura de circuito, una colocación o un modo local de dibujo como `$RC_STAGE`; ATTACH cuando la entidad se añade sobre los objetos, como una tierra, una etiqueta de puerto, una marca de no conexión, un punto de unión, una etiqueta por cada instancia o el relleno. Las dos palabras clave hacen lo mismo; solo difiere el lugar de la definición (9.4).
- `gap` es un parámetro de `$GROUND_STUB`, la distancia mínima desde el pin hacia abajo hasta el símbolo de tierra (1 por defecto). Al igual que los nombres de función `left` y `axis`, es un nombre dado en una definición, no una palabra estándar.

### B.7 Selecciones

Una selección se escribe como una elección entre candidatos: `among <candidates>, @SELECT-…(conditions)`. La redacción de los candidatos depende de la clase del ámbito y de la clase seleccionada:

| Ámbito | Selección | Candidatos |
|---|---|---|
| `#` | cualquier clase | `the <kind> in # only` |
| `#`, en una sentencia que alcanza los niveles inferiores (10.1) | cualquier clase | `the <kind> at each level in # and below` |
| un conjunto de la clase seleccionada (un filtro) | esa clase | `{…}` |
| red | instancias | `the instances connected to &en` |
| red | terminales | `the terminals connected to &en` |
| red | cables | `the wires on &en` |
| instancia | terminales | `the terminals of #R1` |
| instancia, haz, uso con símbolo | pines | `the pins of #M1` |
| terminal | pines | `the pins of #X1:vdd` |
| red | pines | `the pins on &en` |
| el resultado de una tubería | pines | `the pins of them` |
| instancia | redes | `the nets connected to #M1` |
| terminal | instancias | `the instances owning #M1:g` |
| terminal | redes | `the nets connected to #M1:g` |
| cable | redes | `the nets of %w13` |
| instancia, terminal, cable, red, gráfico | etiquetas | `the labels attached to #M1` |
| uso (sin usos entre sus miembros) | cualquier clase | `the members of %dp` |
| uso (con usos entre sus miembros, o no se sabe) | cualquier clase | `all members of %st1 at any depth` |
| uso | usos (SELECT-USAGE) | `%st1 itself and the usages inside it` |
| cualquier objeto | los usos que lo contienen (SELECT-ANCESTOR-USAGE) | `the usages containing #M1` |
| un conjunto de varias clases (clase no sabida) | | escríbase `for %mixed_set, @SELECT-…` en su lugar |

- `<kind>` sigue la palabra clave: instances, terminals, pins, nets, wires, labels, annotations, usages.
- En mitad de una tubería, el ámbito en los candidatos se escribe `them`, como `=> among all members of them at any depth,`. Si los usos seleccionados antes tienen usos entre sus miembros no se sabe cuando se escribe la oración, por lo que se usa `at any depth`.
- Para los miembros de usos la redacción es `at any depth`, no `at any level`: un nivel es un nivel del circuito, como en `at each level in # and below`.
- Una regla en un paquete cuenta como escrita en el nivel de su USE-PACKAGE (10.4); con `recursive=true`, un paquete escribe `at every level that uses this package, #` en lugar de `at each level in # and below`.

| Palabra clave | Caso | Oración |
|---|---|---|
| SELECT-INSTANCE | este nivel y por debajo | `among the instances at each level in # and below, @SELECT-INSTANCE(model="nch", recursive=true) => they @SET-SYMBOL to $NMOS` |
| | solo este nivel | `among the instances in # only, @SELECT-INSTANCE(type="pmos") => …` |
| | un filtro | `among {#M1 #M2 #R1}, @SELECT-INSTANCE(name="M*") => they @ORIENT("MY")` |
| | en una red | `among the instances connected to &en, @SELECT-INSTANCE => they @SET-STYLE to $grey` |
| | propietaria de un terminal | `among the instances owning #M1:g, @SELECT-INSTANCE => they are on @LAYER("top")` |
| | miembros de un uso | `among all members of %st1 at any depth, @SELECT-INSTANCE(role="pair")` |
| | miembros de un uso sin usos anidados | `among the members of %dp, @SELECT-INSTANCE` |
| | en mitad de una tubería | `among the usages at each level in # and below, @SELECT-USAGE(recursive=true) of $DIFFERENTIAL_PAIR_N => among all members of them at any depth, @SELECT-INSTANCE => they @SET-STYLE to $signal` |
| SELECT-TERMINAL | este nivel y por debajo | `among the terminals at each level in # and below, @SELECT-TERMINAL(net="vss", recursive=true) => they have @ASSERT-TERMINAL(same=["net"])` |
| | de una instancia | `among the terminals of #R1, @SELECT-TERMINAL => on them there is a @TEXT("${net}")` |
| | en una red | `among the terminals connected to $GROUND_NET_SET, @SELECT-TERMINAL => among the pins of them, @SELECT-PIN(wired=false) => $GROUND_STUB is @ATTACH to them` |
| SELECT-PIN | en el nivel | `among the pins in # only, @SELECT-PIN(wired=false) => on them there is a @TEXT("${net}", key="netname")` |
| | de terminales | `among the pins of {#M1:s #M2:s}, @SELECT-PIN(wired=false) => $GROUND_STUB is @ATTACH to them` |
| | de un terminal | `among the pins of #X1:vdd, @SELECT-PIN => they @SET-STYLE to $red` |
| SELECT-NET | este nivel y por debajo | `among the nets at each level in # and below, @SELECT-NET(port=true, recursive=true) => $IO_PORT is @ATTACH to them` |
| | de una instancia | `among the nets connected to #M1, @SELECT-NET => named %m1_net_set` |
| | de un terminal | `among the nets connected to #M1:g, @SELECT-NET => they are drawn with @ROUTE-NET` |
| | de un cable | `among the nets of %w13, @SELECT-NET => they are drawn with @ROUTE-NET` |
| | un filtro | `among {&a &b}, @SELECT-NET(wired=false) => they are drawn with @ROUTE-NET` |
| SELECT-WIRE | este nivel y por debajo | `among the wires at each level in # and below, @SELECT-WIRE(bus=true, recursive=true) => they @SET-STYLE to $bus` |
| | en una red | `among the wires on &clk, @SELECT-WIRE => they @SET-STYLE to $red` |
| SELECT-LABEL | este nivel y por debajo | `among the labels at each level in # and below, @SELECT-LABEL(key="wl", recursive=true) => they @SET-STYLE to $grey` |
| | adjunta a un objeto | `among the labels attached to #M1, @SELECT-LABEL => they @SET-STYLE to $grey` |
| SELECT-ANNOTATION | este nivel y por debajo | `among the annotations at each level in # and below, @SELECT-ANNOTATION(recursive=true) => they @SET-STYLE to $base` |
| | un filtro | `among {%n1 %f1 #M1}, @SELECT-ANNOTATION` |
| SELECT-USAGE | este nivel y por debajo | `among the usages at each level in # and below, @SELECT-USAGE(recursive=true) of $DIFFERENTIAL_PAIR_N => they have @PRIORITY("disabled")` |
| | sin una definición | `among the usages at each level in # and below, @SELECT-USAGE(recursive=true) => they @SET-STYLE to $base` |
| | un uso y los usos dentro de él | `among %st1 itself and the usages inside it, @SELECT-USAGE of $DIFFERENTIAL_PAIR_N` |
| SELECT-ANCESTOR-USAGE | | `among the usages containing #M1, @SELECT-ANCESTOR-USAGE of $DIFFERENTIAL_PAIR_N => named %dp_set` |
| cualquier selección | clase del ámbito no sabida | `for %mixed_set, @SELECT-TERMINAL => …` |

### B.8 Conjuntos y otras palabras clave

| Palabra clave | Caso | Oración |
|---|---|---|
| ADD-TO | | `&vdd @ADD-TO $SUPPLY_NET_SET` |
| | en una tubería | `… => they @ADD-TO $SUPPLY_NET_SET` |
| REMOVE-FROM | | `&agnd @REMOVE-FROM $GROUND_NET_SET` |
| UNION | | `%u_set = %a_set and %b_set @UNION` |
| BUNDLE | | `#XI<3:0> is drawn as a @BUNDLE => named %xi` |
| CURRENT | entrada (in) | `#M5:1 has @CURRENT("in")` |
| | salida (out) | `#:iout has @CURRENT("out")` |
| | sin especificar (unspecified) | `#M5:2 has @CURRENT("unspecified")` |
| ASSERT-INSTANCE | | `{#M1 #M2} have @ASSERT-INSTANCE(same=["model", "w", "l"])` |
| ASSERT-TERMINAL | | `{#M1:s #M2:s} have @ASSERT-TERMINAL(same=["net"])` |
| ASSERT-NET | | `{&inp &inn} have @ASSERT-NET(port=true)` |
| WIRE sin red | | `between {%sub1::y %int1::a}, on no net &, there is a @WIRE` |
| ANNOTATION | | `at (40, 20) there is an @ANNOTATION(rotate=30) of $ARROW_SYMBOL` |
| ASSERT-USAGE | | `%dp1 is, by @ASSERT-USAGE, a usage of $DIFFERENTIAL_PAIR_N` |
| ASSERT-IN | | `{&vss} is, by @ASSERT-IN, in $GROUND_NET_SET` |
| ASSERT-TOPOLOGY | serie | `{#R1 #R2 #R3} are connected as @ASSERT-TOPOLOGY("series")` |
| | paralelo | `{#M3 #M4} are connected as @ASSERT-TOPOLOGY("parallel")` |
| | estrella | `{#L1 #C1 #L2} are connected as @ASSERT-TOPOLOGY("star")` |
| | bucle | `{#C1 #L1 #C2} are connected as @ASSERT-TOPOLOGY("loop")` |

### B.9 Registro de cabecera y paquetes

No se añaden palabras. El registro de cabecera se lee en la ortografía estándar y no utiliza alias (16.5).

| Palabra clave | Caso | Oración |
|---|---|---|
| VERSION | | `@VERSION("0.4")` |
| ALIAS | | `@ALIAS("@位于", "@PLACE")` |
| USE-ALIASES | | `@USE-ALIASES(name="alias-zh", version="0.4")` |
| DEVICE | | `@DEVICE(…)` |
| PACKAGE | | `@PACKAGE(name="acme-style", version="1.2")` |
| USE-PACKAGE | en un registro de cabecera; también una referencia en el registro de cabecera de un archivo de paquete | `@USE-PACKAGE(name="rnl-std-all", version="0.4")` |
| | en un nivel | `@USE-PACKAGE(name="acme-style", version="1.2")` |
