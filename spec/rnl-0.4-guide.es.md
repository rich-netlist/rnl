# Guía breve de RNL 0.4

RNL se escribe en los comentarios de un netlist de SPICE o Spectre e indica cómo se dibuja el circuito y qué significan sus partes; nunca modifica el circuito. Lenguaje `"0.4"`, serie de paquetes estándar `"0.4"`; los números como (14.8) remiten a secciones del estándar. Los ejemplos son casos independientes, no un único programa: utilice los objetos existentes en su propio nivel y escriba los fragmentos dentro de registros RNL. Los comentarios `/* */` los explican. El catálogo del final recoge cada nombre en forma de firma, no como código RNL para copiar.

## Un archivo

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

- Los registros son comentarios de línea completa, con `*` en la columna 1 seguido de un espacio; también `//` en modo Spectre (un archivo `.scs`, o tras `simulator lang=spectre`). El registro de encabezado va en primer lugar. Un registro dentro de `.subckt … .ends` pertenece a ese subcircuito.
- Sin ese USE-PACKAGE no se conoce ningún nombre estándar. Conserve intacto todo registro de encabezado que reciba; sus líneas `@DEVICE` asignan a los dispositivos de PDK sus nombres de terminal y tipos (6.12), así que no debe escribirlas ni modificarlas.
- Escriba un espacio ASCII delante de cada `@`; los paréntesis de parámetros van pegados a la palabra clave y los de rol, al objeto. Los parámetros posicionales preceden a los nombrados: `@PLACE(10, y=20)`. Los números no llevan unidades y no existen expresiones. Los nombres y las palabras clave de RNL distinguen entre mayúsculas y minúsculas; los nombres del netlist siguen las reglas del anfitrión.

## Referencias

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

- Nombres de terminal: M `d g s b`; Q `c b e s`; J y Z `d g s`; D `a k`; R, C, L, V, I, B, F, H y W `p n`; E, G y S `p n cp cn`; T y O `p1 n1 p2 n2`; U `a b ref`; y en X, los puertos del subcircuito. Donde hace falta un pin, un terminal representa a su pin.
- Cada nombre se vincula una sola vez por nivel y puede utilizarse antes de su definición. Para mover `%p`, escriba `%p @PLACE(...)`, nunca otro `%p = ...`. Los nombres `%` son locales; las definiciones `$` se heredan en los niveles inferiores, donde otra definición del mismo nombre puede ocultarlas.
- Toda palabra sin prefijo es texto libre (produce una advertencia si nombra algún objeto): `#M1 #M2 M3 @ALIGN("y");` alinea únicamente M1 y M2.

## Sentencias y tuberías

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

Qué pasa adelante cada segmento: PLACE, ORIENT, SET-SYMBOL, SET-STYLE, GROUP y CURRENT pasan su entrada; AS y ATTACH, el nuevo uso (o un conjunto de ellos, si se aplican uno por uno); ALIGN, ORDER, DISTRIBUTE, DISTANCE, TABLE y ORIENT-RELATION, la nueva restricción; TEXT, el nuevo texto; y SELECT-…, el conjunto seleccionado.

## Definiciones, roles y usos

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

- Un rol `{role}` recibe el conjunto entero (o un solo objeto como conjunto de un elemento) y puede omitirse; si los argumentos no llevan marca de rol, solo pueden omitirse los roles de conjunto finales. Todo rol de un único objeto es obligatorio. Marque cada rol como máximo una vez; para pasar varios miembros, agrúpelos en un conjunto: `{#R1 #R2}(members)`.
- En un cuerpo, `?` aislado representa el punto propio del uso, las coordenadas son desplazamientos respecto a él y `%mid = (_, _);` crea un objeto local, accesible desde fuera como `%u::mid`.
- Las estructuras estándar que colocan vinculan el punto propio de su uso: una fila o una columna de miembros lo tiene sobre su línea y dos lados reflejados en el centro entre ambos, de modo que una restricción sobre `%u::position` mueve la estructura. La columna *Dibuja* del catálogo indica dónde: `$BRANCH` comparte con él únicamente la coordenada x de los miembros, `$BRANCH_HORIZONTAL` únicamente su y, y `$PARALLEL_HORIZONTAL` y `$MIRRORED_LAYOUT` lo mantienen en su centro. Cualquier otra estructura lo deja libre, y una restricción sobre su `::position` no cambia nada dibujado.
- Prefiera estructuras que indiquen qué son las partes y, a la vez, las dibujen. Un `$BRANCH` es una estructura en serie apilada en vertical, con sus miembros de arriba abajo; su uso habitual es una rama de CC, como un camino entre una alimentación y tierra. Escriba cada camino de este tipo como un `$BRANCH`, y un par en paralelo en él mediante una variante que coloque: `{#R1 #M1 #R2} @AS $BRANCH; {#R2 #C2} @AS $PARALLEL_HORIZONTAL;`. `$PARALLEL` por sí solo únicamente traza. Una rama no dice nada acerca de la corriente, que se declara con `@CURRENT`. Un camino de señal a través de interruptores y condensadores, como en un muestreador de capacidades conmutadas, se dibuja mediante hechos atómicos, por ejemplo aplicando ALIGN y ORDER a los pines de cada fila: `{#S1:s #S1:d #C1:p #C1:n} @ALIGN("y"); {#S1:s #S1:d #C1:p #C1:n} @ORDER("left-to-right");`. Una estructura de *solo significado* no dibuja nada: utilícela donde su significado se cumpla, pero nunca en lugar de una estructura que coloca. Marque un `$SIGNAL_FLOW` únicamente sobre un camino de señal limpio del que esté seguro; omítalo allí donde la señal tome varios caminos, se realimente o circule en ambos sentidos por buena parte del circuito, por pocos o muchos miembros que tenga el camino.

## Selecciones y reglas

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

- `name`, `model`, `net` y `key` admiten el comodín `*`. `type` corresponde al tipo de la tarjeta `.model` o al de una sentencia DEVICE.
- Ni el resultado de una selección ni un conjunto construido a partir de él admiten lugares de miembros (`%sel{1}`) ni pueden pasarse a WIRE; nombre el objeto o utilice ROUTE-OBJECT.

## Hechos de disposición

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

- Direcciones: `"left-to-right"`, `"right-to-left"`, `"bottom-to-top"`, `"top-to-bottom"`. Códigos de orientación: `R0`, `R90`, `R180` y `R270` giran en sentido antihorario; `MY` refleja de izquierda a derecha y `MX` de arriba abajo; `MYR90` y `MXR90` primero reflejan y luego giran.
- Los miembros participan según su punto de referencia: las instancias por su origen, los terminales y pines por la posición del pin, los puntos y textos por sí mismos, y los usos con símbolo por su punto propio. Un uso sin símbolo representa a sus miembros hasta los más bajos, como en GROUP; su punto propio es `%u::position`. Los conjuntos carecen de origen: PLACE coloca cada miembro por separado, incluso tras GROUP. PLACE no coloca pines ni restricciones. No existe ningún «más a la izquierda»: expréselo respecto a otras partes.
- El solucionador decide todo lo que queda libre y no presupone qué función cumple cada parte: escriba explícitamente lo que debe cumplirse. Dé prioridad al significado y a las restricciones y deje colocar al solucionador; cuando la tarea pida coordenadas, escriba en su lugar PLACE, ORIENT y WIRE para todo el circuito. No mezcle ambos enfoques: o bien significado y restricciones para el solucionador con pocos PLACE o ninguno, o bien todas las posiciones escritas, sin resolución.
- Ordene los bloques grandes, no los dispositivos sueltos: `{#V2 #C1 %bias %stage #C3 #R5} @ORDER("left-to-right");` dispone columnas enteras una junto a otra. Una estructura representa a sus miembros, de modo que cada dispositivo de `%stage` queda entre cada dispositivo de `%bias` y C3; un dispositivo compartido por dos estructuras no se ordena respecto de sí mismo. Para restringir en cambio el centro de una estructura, escriba su punto propio: `{%dp::position #X1} @ALIGN("y");` sitúa X1 a la altura del centro del par, mientras que `{%dp #X1} @ALIGN("y");` situaría los tres transistores a la altura de X1 (la columna *Dibuja* del catálogo indica dónde vincula su punto cada estructura).
- Las partes que rodean el circuito, como las alimentaciones y las fuentes de señal, suelen situarse a un lado de él, algo apartadas del resto; un DISTANCE lo expresa: `#VDD {#VIN #XU1 #CL} @DISTANCE("left-to-right", min=4);` mantiene la alimentación al menos 4 unidades a la izquierda de cada uno de los demás elementos.
- Dé al dibujo un árbol de grupos claro: cada bloque funcional como un grupo, grupos anidados dentro de otros mayores y unos cinco o seis elementos directamente en cada nodo. Muchas estructuras ya son grupos (columna *Grupo*); una que pueda quedar parcialmente dentro de otra, como un espejo de corriente, no lo es. Agrupe usted mismo estructuras completas o una etapa junto con sus puertos: `%amp %p_inp %p_inn %p_out @GROUP;`. Los grupos no se cruzan: de dos que compartan miembros sin anidarse, el solucionador conserva solo uno como bloque.
- Fije las orientaciones únicamente allí donde el lector espera una que nada más determina, por lo general la fuente de señal, las fuentes de alimentación y la carga: `{#V1 #V2 #R5} @ORIENT("R0");`. Deje el resto en manos del solucionador; las estructuras y la corriente bastan para fijar la mayoría.

## Pines y direcciones

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

- Fijar un solo pin deja dos orientaciones posibles de ocho: solo con la puerta, derecho o invertido; solo con el drenador, con la puerta a la izquierda o a la derecha. Fije dos pines.
- Pines estándar en R0: en R, C, L y fuentes, p arriba y n abajo; `$DIODE`, a arriba y k abajo; `$NMOS`, d arriba, g izquierda, s abajo y b derecha (4TERM); `$PMOS`, s arriba, g izquierda y d abajo; `$NPN`, c arriba, b izquierda y e abajo; `$PNP`, e arriba, b izquierda y c abajo.

## Corriente

```rnl
#M1:d @CURRENT("in"); #M1:s @CURRENT("out"); /* the static current at a terminal: in, out, unspecified */
{#M1:d #M2:d #M5:d} @CURRENT("in"); /* a set does each member */
#:out @CURRENT("in"); /* at a port: current enters the subcircuit */
#M1::d @CURRENT("in"); /* a pin stands for its terminal */
```

- No dibuja nada; indica qué hace el circuito, y el solucionador dispone los caminos de corriente a partir de ello. Escríbalo en los caminos principales.
- Escríbalo únicamente donde circule una corriente estática y su sentido sea seguro: los condensadores, las puertas y las fuentes sin valor de CC no conducen corriente estática, así que omítalos, y omita también cualquier terminal cuando dude. Deje igualmente sin marcar las fuentes de alimentación: en su interior la corriente sube de n a p, y un solucionador que prefiera la corriente descendente les daría la vuelta.
- Una red cuyos terminales y puertos lleven todos la corriente en un mismo sentido produce una violación; las redes globales como 0 no se comprueban.

## Prioridad

```rnl
#M5 @PLACE(x=3, priority="low"); /* disabled, low, normal (default), high */
%dp1 = #M1(left) #M2(right) #M5(tail) @AS(priority="high") $DIFFERENTIAL_PAIR_N; /* covers every fact of the body */
%dp1 @PRIORITY("disabled"); /* turns the default drawing of the pair off */
```

- En caso de conflicto se descarta primero el hecho de menor prioridad y, en igualdad de prioridad, el escrito antes; todo hecho conservado es estricto. El paquete estándar dibuja con prioridad `"low"`. El campo `priority=` en el encabezado del registro actúa como valor por defecto, pero AS y ATTACH únicamente toman de él `"disabled"`.
- Una estructura no necesita prioridad: escríbala únicamente para resolver un conflicto que haya mostrado una comprobación o una resolución.

## Cables y trazado

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

- Coordenadas iguales no conectan pines. Escriba un WIRE entre ellos, aunque tenga longitud cero, para marcarlos como cableados y evitar relleno adicional.
- Las estructuras dibujan sus cables internos con prioridad baja. Lo que une dos estructuras entre sí es una etiqueta de red, salvo que escriba un ROUTE.
- Cablee únicamente lo que se lea mejor como cable, no todo el circuito: el solucionador dibuja solo lo que piden WIRE, ROUTE-NET y ROUTE-OBJECT, mientras que el relleno añade a cualquier otro pin sin cablear un tramo corto de tierra, alimentación o etiqueta de red. Trace el camino principal de señal y las conexiones dentro de las estructuras y entre estructuras vecinas; deje a las etiquetas las conexiones largas o congestionadas, como las líneas de polarización y las redes compartidas por todo el dibujo.
- En un circuito pequeño, todas las redes salvo los raíles suelen leerse bien como cables: trácelas como en la última línea anterior, o con `# @SELECT-NET => @ROUTE-NET;`, que dibuja lo mismo. En ambos casos el relleno excluye del trazado las redes de tierra, alimentación y polarización con prioridad alta, y cada red de raíl recibe una indicación (RNL9403) de que su árbol perdió todos sus pines; esa indicación es la esperada.
- Una red trazada hacia un puerto de su nivel solo llega al puerto si se escribe la etiqueta de puerto: `$IO_PORT @ATTACH &x;`, o `$IO_PORT_STUB` en el pin que excita. El relleno añade sus etiquetas de puerto después de haber decidido qué pines están cableados, así que una red con un único pin propio, como una entrada que excita una sola puerta, forma un árbol de un punto: ese pin recibe una etiqueta de red y ningún cable llega al puerto.

## Textos

```rnl
#M1 @TEXT("${w}/${l}", key="wl", dy=-3); /* a label that moves with M1; #M1::wl */
#M1 @TEXT("M_{in}", key="name"); /* the same key replaces the name the symbol shows */
(10, 10) @TEXT("bias", size=0.8); /* on a point: an annotation */
#M1 @TEXT("\overline{EN} \mu A \frac{W}{L}", halign="middle", valign="top", rotate=90);
```

- `${…}`: `name`, `value`, `model`, `type`, parámetros del netlist como `w`; `net`; roles; y caminos como `${pair::left::w}`.
- `dx` y `dy` son desplazamientos respecto al elemento sobre el que está el texto. `#M1::wl @PLACE(5, 0);` sitúa la etiqueta en las coordenadas (5, 0) del dibujo; en cada eje, un PLACE y un desplazamiento se sustituyen mutuamente y prevalece el escrito después. En el cuerpo de un símbolo, ambos se expresan en las coordenadas propias del símbolo.

## Símbolos y estilos

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

- Propiedades de estilo: `linecolor linestyle linewidth fillcolor textcolor size font bold italic textvisible`; los colores se escriben como `"#RRGGBB"`. Un nombre `%` también puede nombrar una definición local. Para cada propiedad, los ajustes directos prevalecen sobre las reglas; dentro de un mismo rango, prevalece el ajuste posterior.

## Comprobaciones

Las estructuras estándar comprueban lo que afirman: dos mitades de un par con el mismo modelo y tamaño, una serie conectada realmente en serie, un rol pmos que contiene un PMOS o una etiqueta de puerto en un puerto. Una comprobación que no se cumple produce una **violación**: es un mero informe; nada falla y el dibujo no cambia. Corrija la estructura o los roles, o elija una estructura que afirme menos.

## El paquete estándar

```rnl
@USE-PACKAGE(name="rnl-std-all", version="0.4"); /* everything: styles, symbols, rules for SPICE and Spectre, fill, structures, the blocks */
@USE-PACKAGE(name="rnl-std-no-labels", version="0.4"); /* optional, besides rnl-std-all: hides the labels symbols draw (name, value, model, net names); a label you write shows only with textvisible=true */
@USE-PACKAGE(name="rnl-std-45deg", version="0.4"); /* optional: symbols at 45 degrees */
@USE-PACKAGE(name="rnl-std-fill") {}; /* an empty package drops that part */
&vdd @ADD-TO $SUPPLY_NET_SET; &vss @ADD-TO $GROUND_NET_SET; /* supply and ground stubs on their pins; 0 is a ground already */
$PORT_LIST @ATTACH; /* a port tag on every port net, in one column */
$FIGURE @ATTACH %p => @SET-SYMBOL $MY_FIG; /* no netlist instance: use a symbol with unbound pins and no ${name}/${value}; not $RESISTOR */
```

- Símbolos por defecto: R `$RESISTOR`, C `$CAPACITOR`, L `$INDUCTOR`, V `$VOLTAGE_SOURCE`, I `$CURRENT_SOURCE`, D `$DIODE`, E `$VCVS_4TERM`, G `$VCCS_4TERM`, F `$CONTROLLED_CURRENT_SOURCE`, H `$CONTROLLED_VOLTAGE_SOURCE`, B `$BEHAVIORAL_SOURCE`, S `$VOLTAGE_CONTROLLED_SWITCH`, W `$SWITCH`, T y O `$TRANSMISSION_LINE`, U `$RC_LINE`; transistores según su tipo: nmos `$NMOS_4TERM`, pmos `$PMOS_4TERM`, npn `$NPN`, pnp `$PNP`, njf `$NJFET`, pjf `$PJFET`. Un NMOS cuyo sustrato esté en una red de tierra y un PMOS cuyo sustrato esté en una red de alimentación reciben automáticamente los símbolos de tres pines `$NMOS` y `$PMOS`, sin necesidad de que escriba ningún SET-SYMBOL. Sin un tipo (sin `.model` ni DEVICE), el transistor no recibe símbolo de dispositivo y se dibuja como una caja con un pin por cada nodo (9.2.8).
- Añada cada red de alimentación a `$SUPPLY_NET_SET` y cada tierra distinta de 0 a `$GROUND_NET_SET`, como en el ejemplo anterior: así sus pines sin cablear reciben tramos cortos y permanecen sin trazar. El relleno omite los pines que ya tienen cables o marcas de no conexión; cualquier otro pin sin cablear, de un dispositivo o de un componente virtual, recibe un `$GROUND_STUB` en las redes de `$GROUND_NET_SET` (red 0 por defecto), un `$SUPPLY_STUB` en las de `$SUPPLY_NET_SET` (vacío por defecto), un `$BIAS_STUB` en las de `$BIAS_NET_SET` (vacío por defecto) y un `$NET_LABEL_STUB` en el resto; el pin de sustrato de un MOS de cuatro terminales recibe siempre un `$NET_LABEL_STUB` en cualquier red. Cada red de puerto recibe un `$IO_PORT`, incluidas las redes de alimentación y de tierra, dispuestos en una columna al menos 6 unidades a la izquierda de las instancias del nivel, cada uno en R0; excluya una red con `&vdd @REMOVE-FROM $PORT_FILL_SET;`, o escriba `$IO_PORT @ATTACH &out;` si prefiere colocar una etiqueta por su cuenta. Cada etiqueta muestra por sí misma el nombre de su red mediante su texto `netname`: no le añada ningún TEXT con el nombre y, si desea desplazarlo, utilice `%t::netname @PLACE(x, y);`. Para un nombre en un banco de pruebas, utilice `$NET_LABEL`. Un pin que ya tenga un `$IO_PORT_STUB` o un `$NET_LABEL_STUB` tampoco recibe etiqueta de red, aunque sus patillas las dibuja el símbolo, no un WIRE. Los conjuntos `*_FILL_SET` están reservados para los tramos cortos y las etiquetas que genera el propio relleno: al resolver, estos se escriben de vuelta y los conjuntos dejan de contener dichos pines y redes, así que no base ninguna otra regla en ellos (como un TEXT sobre sus pines); seleccione usted mismo los pines. Para controlar qué pines reciben un tramo corto, añada pines a estos conjuntos o quítelos de ellos; el relleno no detecta un tramo corto adjuntado por una regla suya y añadirá además el suyo.

## Registros del solucionador

```spice fragment
* @RNL(type="generated", priority="high", author="rnl-solver") { #R1 @PLACE(0, 4); #R2 @PLACE(0, -4);} /* never edit; replaced on each solve */
* @RNL(author="editor") { #R2 @PLACE(y=-6, priority="high");} /* to fix a position: high, in a later record */
```

Los números como `%#3` solo aparecen en esos registros; cuando escriba RNL, nombre los objetos en su lugar.

## Otras palabras clave

Las escriben los paquetes y las herramientas; conviene saber leerlas, aunque rara vez las escribirá:

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

## Errores frecuentes

La columna izquierda no es válida o induce a error; copie únicamente la corrección de la derecha, adaptada con los nombres de su archivo.

|Error|Corrección|
|---|---|
|`#M1 @PLACE (10, 20);`|`#M1 @PLACE(10, 20);`|
|`#M1 (left) #M2 (right) @AS $CROSS_COUPLED_PAIR;`|`#M1(left) #M2(right) @AS $CROSS_COUPLED_PAIR;`|
|`#M1 #M2 @SET-SYMBOL $NMOS;`|`{#M1 #M2} @SET-SYMBOL $NMOS;`|
|`#M5 #M3 @AS $MATCHED;`|`{#M5 #M3} @AS $MATCHED;`|
|`{#R1 #R2} @ORDER("x");`|`{#R1 #R2} @ORDER("left-to-right");`|
|`#R1 @PLACE(x=2u);`|`#R1 @PLACE(x=2);`|
|en 0.3, `$DIFFERENTIAL_PAIR` con roles `in_p/in_n`|`#M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N;`|
|`#R1 @SET-SYMBOL $GROUND;`|`$GROUND @ATTACH &n;` para una tierra; conserve el símbolo de la resistencia|
|`%t #M1 @ALIGN("y");` para alinear sus pines|`{%t::pin #M1::g} @ALIGN("y");`|
|`$IO_PORT @ATTACH &n;` en una red que no es puerto|`$NET_LABEL @ATTACH &n;`|
|`&vss @ROUTE-NET;` cuando vss está en el conjunto de tierras|`&vss @ROUTE-NET(priority="high");`|
|`%#3 @PLACE(1, 2);` en texto escrito por el autor|Nombre ese objeto y utilice su nombre|
|`#M1.d; M3; &vdd!;`|`#M1:d; #M3; &"vdd!";`|
|`{@VERSION("0.4");}`|`{ @VERSION("0.4");}`|

Conectar un cable al sustrato de `$NMOS` falla: utilice `$NMOS_4TERM`. Los tipos de transistor desconocidos necesitan una tarjeta de modelo, una sentencia DEVICE o un símbolo explícito. Las reglas de nivel superior necesitan `recursive=true` para alcanzar los subcircuitos.

## Un ejemplo completo

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

Antes de entregar un archivo, compruebe lo siguiente: encabezado al principio, con VERSION y el paquete; todos los nombres existen en su nivel; solo nombres de 0.4 (los del catálogo); un espacio delante de cada `@` y un `;` tras cada sentencia; direcciones indicadas en las restricciones; los roles de conjunto reciben conjuntos; tierras y alimentaciones incluidas en sus conjuntos; orientaciones fijadas solo donde hace falta y corriente solo donde su sentido es seguro; pines alineados conectados con cables; ninguna edición en los registros generados; y ninguna violación no intencionada.

<!-- lists:begin -->
## Catálogo

Esta sección es una referencia, no código RNL para copiar: conserve siempre el `$` de cada nombre. Aplique las definiciones con AS o ATTACH, asigne símbolos con SET-SYMBOL y estilos con SET-STYLE. Los roles figuran en orden de declaración, cada uno con las clases que admite (un rol sin clases admite objetos de cualquier clase); `{role}` recibe un conjunto entero y puede omitirse; los parámetros van tras `;`. Así, `$MATCHED({members}: instance/usage)` se utiliza como `{#M1 #M2} @AS $MATCHED;`.

### Paquetes

Cada paquete se incluye en el encabezado mediante `@USE-PACKAGE(name="...", version="0.4");`.

|Paquete|Contenido|
|---|---|
|rnl-std-all|La única colección de paquetes de la serie 0.4, sin contenido propio: incorpora estilos, símbolos y definiciones básicos y de semiconductores, relleno automático, semántica de circuitos, los cuatro paquetes de dominio y el paquete de reglas de cada dialecto anfitrión mediante un registro de encabezado específico para ese lenguaje (2.8, 16.3).|
|rnl-std-style|Los estilos (9.3) y las reglas de estilo por defecto.|
|rnl-std-basic|Los símbolos de componentes pasivos, fuentes, interruptores, líneas de transmisión, componentes virtuales y marcas; las definiciones de componentes virtuales, marcas, figuras y tramos cortos, cada una acompañada de la regla que dibuja sus usos; y los conjuntos de redes de tierra, alimentación y polarización.|
|rnl-std-semiconductors|Los símbolos de diodos y de transistores MOS, bipolares, JFET y uniunión (UJT, PUT).|
|rnl-std-spice|El paquete de reglas de SPICE: asigna símbolos a las instancias según la letra de elemento, el tipo de modelo y la red de sustrato o de control negativo (16.3).|
|rnl-std-spectre|El paquete de reglas de Spectre: asigna símbolos a las instancias según el maestro, el tipo de modelo y la red de sustrato o de control negativo (16.3).|
|rnl-std-fill|Los conjuntos y las reglas del relleno automático: adjunta tramos cortos de tierra, alimentación, polarización y etiqueta de red a los pines sin cablear, incluidos los de componentes virtuales, y una etiqueta de puerto a cada red de puerto en `$PORT_FILL_SET` (10.5, 12.4, 16.3).|
|rnl-std-semantics|Las definiciones semánticas de estructuras de circuito y de sus dibujos por defecto.|
|rnl-std-functional|Modelos de comportamiento: amplificadores operacionales, comparadores, amplificadores, transconductores, ADC, DAC y modulador chopper, con variantes *_SUPPLY dotadas de pines vp (arriba) y vn (abajo) en cuadrícula entera.|
|rnl-std-rf|Componentes de RF: el inductor con toma central y la bobina en T.|
|rnl-std-logic|Puertas, búfer, inversor, celda de retardo y biestables D.|
|rnl-std-signal-flow|Componentes virtuales ideales de flujo de señal y sus símbolos: sumador, restador, multiplicador, integradores de tiempo continuo y de tiempo discreto, cuantizador y retardo unitario.|
|rnl-std-no-labels|Oculta las etiquetas por defecto de los símbolos estándar, pensado para archivos que escriben sus propias etiquetas (16.3).|
|rnl-std-no-instance-labels|Oculta únicamente las etiquetas por defecto de las instancias; permanecen los nombres de red que muestran las tierras, las alimentaciones, las marcas de polarización, las etiquetas de puerto y las etiquetas de red (16.3).|
|rnl-std-45deg|Símbolos a 45 grados para puentes, pares acoplados en cruz, devanados en diagonal y cables a 45 grados.|

### Definiciones

En la columna *Dibuja*, para las estructuras de rnl-std-semantics: *solo significado* no coloca ni traza nada; *coloca* añade hechos de posición sobre sus miembros; y *traza* une las redes que comparten sus miembros. Sus hechos de posición y trazados tienen prioridad low. En *Grupo*, leído del cuerpo de cada definición: *sí* con los roles cuyos miembros reúne en un grupo, *mediante* `$X` con los roles que pasa a una definición `$X` que forma el grupo, y vacío cuando no forma ninguno; una estructura que pueda quedar parcialmente dentro de otra, como un espejo de corriente, no forma ninguno (véase el árbol de grupos bajo *Hechos de disposición*). Un uso sin símbolo representa a sus miembros en ALIGN, ORDER, DISTRIBUTE y DISTANCE; su punto propio, `%u::position` (14.1), solo sigue a la estructura allí donde *punto propio* lo vincula a sus miembros: únicamente un uso así se mueve como un solo elemento por su `::position`, o como miembro de otra estructura que coloca; para los demás, restrinja sus miembros o un conjunto con nombre formado por ellos. Los demás paquetes definen los componentes y marcas que se dibujan.

|Firma|Dibuja|Grupo|Significado|
|---|---|---|---|
|**rnl-std-basic**||||
|`$GROUND(p: net)`|—||Una tierra en una red, dibujada en un punto propio: en la red 0 son solo las tres barras, y en cualquier otra red muestra el nombre de su red.|
|`$SUPPLY(p: net)`|—||Una alimentación en una red, dibujada en un punto propio.|
|`$BIAS(p: net)`|—||Una marca de tensión de polarización en una red, dibujada en un punto propio.|
|`$IO_PORT(p: net)`|—||Una etiqueta de puerto en una red, dibujada en un punto propio; su red debe ser un puerto de su nivel.|
|`$NET_LABEL(p: net/pin)`|—||Una etiqueta de red: el nombre de una red sobre una patilla corta, dibujada en un punto propio; si recibe un pin de bus, como el de una agrupación (11.8), conserva el pin y forma una sola etiqueta para todas sus redes, mostrando sus nombres (13.6).|
|`$NO_CONNECT(t: terminal/pin)`|—||Una marca de no conexión en un terminal o pin dejado sin conectar a propósito; el terminal se conserva tal cual para que una marca sobre un bit de un bus registre qué bit queda abierto (12.6).|
|`$JUNCTION(p: point/terminal)`|—||Un punto de unión en un punto o en un terminal.|
|`$CURRENT_ARROW(p: terminal/point)`|—||Una flecha de corriente en un terminal o en un punto.|
|`$ARROW(at: point)`|—||Una punta de flecha dibujada en un punto dentro de un símbolo o en la escena (9.2.7).|
|`$SPDT_SWITCH(at: point)`|—||Un interruptor unipolar de dos vías dibujado como figura en un punto, sin línea de netlist.|
|`$FIGURE(at: point)`|—||Una figura dibujada en un punto, sin línea de netlist, a la que se asigna un símbolo propio mediante SET-SYMBOL.|
|`$PORT_LIST()`|—||Una etiqueta de puerto en cada red de puerto de un nivel que no tenga ya un `$IO_PORT`, en una columna, ordenada de arriba abajo y agrupada, al menos 6 unidades a la izquierda de las instancias del nivel, cada una en R0 con su pin a la derecha, todas con prioridad low, tal como las dibuja rnl-std-fill; el relleno no añade entonces ninguna en ese nivel (12.4).|
|`$COMMENT({target}; text="")`|—||Un comentario que no se dibuja, adjunto a cualquier objeto o al propio nivel cuando se omite target.|
|`$GROUND_STUB(t: pin; gap=3)`|—||Una tierra cerca de un pin, en posición normal, con su propio cable: a la altura del pin o debajo de él, a lo sumo a gap unidades en vertical (prioridad low).|
|`$SUPPLY_STUB(t: pin; gap=3)`|—||Una alimentación cerca de un pin, en posición normal, con su propio cable: a la altura del pin o encima de él, a lo sumo a gap unidades en vertical (prioridad low).|
|`$BIAS_STUB(t: pin)`|—||Una marca de tensión de polarización directamente en un pin, orientada hacia fuera del pin como un tramo corto de puerto; su barra se inclina como / encima, debajo y a la izquierda del pin, y como \ a su derecha.|
|`$IO_PORT_STUB(t: pin)`|—||Una etiqueta de puerto directamente en un pin, cuya patilla sirve de cable corto del tramo corto; tanto el pin como la etiqueta permanecen sin trazar, como ocurre con `$NET_LABEL_STUB`.|
|`$NET_LABEL_STUB(t: pin)`|—||Una etiqueta de red directamente en un pin, cuya patilla de una unidad sirve de cable corto del tramo corto; en un pin de bus, una sola etiqueta para todo el bus; el pin y la etiqueta quedan sin trazar.|
|**rnl-std-semantics**||||
|`$MATCHED({members}: instance/usage)`|solo significado||Dispositivos o subestructuras emparejados en su tipo o definición y en su disposición.|
|`$MATCHED_MOS({members}: instance)`|solo significado||MOSFET emparejados en tipo, modelo y longitud de canal.|
|`$MATCHED_BJT({members}: instance)`|solo significado||Transistores bipolares emparejados en tipo, modelo y geometría de emisor unitario.|
|`$MATCHED_RESISTOR({members}: instance)`|solo significado||Resistencias emparejadas en tipo y modelo.|
|`$MATCHED_CAPACITOR({members}: instance)`|solo significado||Condensadores emparejados en tipo y modelo.|
|`$MIRRORED_LAYOUT(left: instance/usage, right: instance/usage, axis: point/instance/usage)`|coloca; punto propio en el centro||Dos objetos simétricos a la misma altura, imágenes reflejadas uno del otro respecto a un eje: únicamente una disposición, sin implicaciones sobre el circuito.|
|`$MIRRORED_VERTICAL_LAYOUT(top: instance/usage, bottom: instance/usage, axis: point/instance/usage)`|coloca; punto propio en el centro||Dos objetos simétricos en una misma columna, imágenes reflejadas uno del otro respecto a un eje horizontal: únicamente una disposición.|
|`$BRANCH({members}: instance/usage)`|coloca, traza; punto propio en x||Una estructura en serie apilada en vertical: miembros en serie en una columna, de arriba abajo; normalmente una rama de CC, como un camino entre una alimentación y tierra.|
|`$BRANCH_HORIZONTAL({members}: instance/usage)`|coloca, traza; punto propio en y||Una estructura en serie dibujada en fila: miembros en serie a la misma altura, de izquierda a derecha.|
|`$CASCADE({stages}: instance/usage)`|solo significado||Una cascada de etapas a lo largo de un camino de señal.|
|`$SERIES({members}: instance/usage)`|traza||Miembros conectados extremo con extremo, en serie, a lo largo de un mismo camino de corriente.|
|`$PARALLEL({members}: instance/usage)`|traza||Miembros conectados en paralelo entre las mismas dos redes.|
|`$PARALLEL_HORIZONTAL({members}: instance/usage)`|coloca, traza; punto propio en el centro||Miembros en paralelo colocados lado a lado a la misma altura, ocupando un único lugar en una columna.|
|`$PARALLEL_VERTICAL({members}: instance/usage)`|coloca, traza; punto propio en el centro||Miembros en paralelo colocados uno encima de otro con la misma x, ocupando un único lugar en una fila.|
|`$SIGNAL_FLOW({path}: instance/usage, {source}: instance/usage/terminal/net, {load}: instance/usage/terminal/net)`|solo significado||Un camino de señal que fluye en un sentido: desde sus fuentes, pasando por las etapas de su camino en el orden escrito, hasta sus cargas.|
|`$POWER_FLOW({path}: instance/usage, {source}: instance/usage/terminal/net, {load}: instance/usage/terminal/net)`|solo significado||Un camino de potencia: desde sus fuentes, como una alimentación o una batería, pasando por las etapas de su camino que la convierten o conmutan en el orden escrito, hasta las cargas que la utilizan.|
|`$BIAS_CIRCUIT({members}: instance/usage)`|solo significado||Un circuito de polarización: los dispositivos o estructuras que generan la polarización de otras partes, como una referencia de corriente.|
|`$FEEDBACK_LOOP({forward}: instance/usage, {feedback}: instance/usage)`|solo significado||Un bucle de realimentación: un camino directo y una red de realimentación.|
|`$DIFFERENTIAL_PAIR_N(left: instance, right: instance, tail: instance/usage)`|coloca, traza; punto propio en el centro|sí: left, right, tail|Un par diferencial de transistores de tipo n con su cola debajo.|
|`$DIFFERENTIAL_PAIR_P(left: instance, right: instance, tail: instance/usage)`|coloca, traza; punto propio en el centro|sí: left, right, tail|Un par diferencial de transistores de tipo p con su cola encima.|
|`$COMPLEMENTARY_DIFFERENTIAL_PAIR(pairup: usage, pairdown: usage)`|traza|sí: pairup, pairdown|Una etapa de entrada raíl a raíl: un par diferencial superior y otro inferior sobre las mismas entradas.|
|`$PSEUDO_DIFFERENTIAL_PAIR(left: instance, right: instance)`|coloca; punto propio en el centro||Un par seudodiferencial: dos transistores de entrada emparejados sin cola, dibujados como par reflejado.|
|`$DEGENERATED_DIFFERENTIAL_PAIR_TYPE_H(left: instance, right: instance, tailleft: instance/usage, tailright: instance/usage, bridge: instance/usage)`|coloca, traza; punto propio en el centro|sí: left, right, tailleft, tailright, bridge|Un par diferencial degenerado por un puente entre sus dos colas, formando una H.|
|`$DEGENERATED_DIFFERENTIAL_PAIR_TYPE_T_N(left: instance, right: instance, degleft: instance/usage, degright: instance/usage, tail: instance/usage)`|coloca, traza; punto propio en el centro|sí: left, right, degleft, degright, tail|Un par diferencial de transistores de tipo n degenerado por un elemento bajo cada entrada, formando una T, con su cola debajo.|
|`$DEGENERATED_DIFFERENTIAL_PAIR_TYPE_T_P(left: instance, right: instance, degleft: instance/usage, degright: instance/usage, tail: instance/usage)`|coloca, traza; punto propio en el centro|sí: left, right, degleft, degright, tail|Un par diferencial de transistores de tipo p degenerado por un elemento sobre cada entrada, formando una T invertida, con su cola encima.|
|`$CROSS_COUPLED_PAIR(left: instance, right: instance)`|coloca, traza; punto propio en el centro|sí: left, right|Un par acoplado en cruz: dos transistores emparejados acoplados en cruz con realimentación positiva, dibujados como par reflejado.|
|`$CURRENT_MIRROR_N({ref}: instance, {out}: instance)`|coloca; punto propio en y||Un espejo de corriente de lado bajo de transistores de tipo n, con las fuentes o emisores apuntando hacia abajo.|
|`$CURRENT_MIRROR_P({ref}: instance, {out}: instance)`|coloca; punto propio en y||Un espejo de corriente de lado alto de transistores de tipo p, con las fuentes o emisores apuntando hacia arriba.|
|`$CASCODE_CURRENT_MIRROR_N({ref}: instance, {out}: instance, {refcascode}: instance, {outcascode}: instance)`|coloca; punto propio en y||Un espejo de corriente cascodo de lado bajo: una fila de espejo abajo con una fila cascodo encima.|
|`$CASCODE_CURRENT_MIRROR_P({ref}: instance, {out}: instance, {refcascode}: instance, {outcascode}: instance)`|coloca; punto propio en y||Un espejo de corriente cascodo de lado alto: una fila de espejo arriba con una fila cascodo debajo.|
|`$TRANS_LINEAR_LOOP({a}: instance/usage, {b}: instance/usage)`|coloca, traza; punto propio en x|sí: a, b|Un bucle translineal de dos grupos de transistores, dibujado en una columna.|
|`$TRANS_LINEAR_LOOP_HORIZONTAL({a}: instance/usage, {b}: instance/usage)`|coloca, traza; punto propio en y|sí: a, b|Un bucle translineal de dos grupos de transistores, dibujado en una fila.|
|`$DIODE_CONNECTED(device: instance)`|traza||Un transistor conectado como diodo con su puerta unida hacia su drenador, o su base hacia su colector.|
|`$COMMON_SOURCE(device: instance)`|solo significado||Una etapa en fuente común de un FET.|
|`$COMMON_GATE(device: instance)`|solo significado||Una etapa en puerta común de un FET.|
|`$COMMON_DRAIN(device: instance)`|solo significado||Un seguidor de fuente (una etapa en drenador común) de un FET.|
|`$COMMON_EMITTER(device: instance)`|solo significado||Una etapa en emisor común de un transistor bipolar.|
|`$COMMON_BASE(device: instance)`|solo significado||Una etapa en base común de un transistor bipolar.|
|`$COMMON_COLLECTOR(device: instance)`|solo significado||Un seguidor de emisor (una etapa en colector común) de un transistor bipolar.|
|`$FOLLOWER(device: instance)`|solo significado||Una etapa seguidora de tensión de un transistor.|
|`$CASCODE(source: instance/usage, cascode: instance)`|traza||Un cascodo: un dispositivo fuente o subestructura en serie con un transistor en puerta común.|
|`$FOLDED_CASCODE(source: instance/usage, cascode: instance, bias: instance/usage)`|traza||Un cascodo plegado: un dispositivo de entrada plegado hacia un transistor en puerta común con su polarización.|
|`$COMMON_SOURCE_AMPLIFIER(input: instance, load: instance/usage)`|traza|mediante `$TRANSCONDUCTOR_Z_AMPLIFIER`: input, load|Un amplificador en fuente común: un transistor de entrada FET y su carga.|
|`$COMMON_EMITTER_AMPLIFIER(input: instance, load: instance/usage)`|traza|mediante `$TRANSCONDUCTOR_Z_AMPLIFIER`: input, load|Un amplificador en emisor común: un transistor de entrada bipolar y su carga.|
|`$DEGENERATED_COMMON_SOURCE_AMPLIFIER(input: instance, load: instance/usage, degenerator: instance/usage)`|traza|mediante `$DEGENERATED_TRANSCONDUCTOR_Z_AMPLIFIER`: input, load, degenerator|Un amplificador en fuente común degenerado: un transistor de entrada FET, su carga y su degeneración.|
|`$DEGENERATED_COMMON_EMITTER_AMPLIFIER(input: instance, load: instance/usage, degenerator: instance/usage)`|traza|mediante `$DEGENERATED_TRANSCONDUCTOR_Z_AMPLIFIER`: input, load, degenerator|Un amplificador en emisor común degenerado: un transistor de entrada bipolar, su carga y su degeneración.|
|`$MILLER_AMPLIFIER(stage: instance/usage, compensation: instance/usage)`|traza|sí: stage, compensation|Un amplificador compensado por Miller: una etapa de ganancia y su red de compensación.|
|`$PUSH_PULL_STAGE(pullup: instance/usage, pulldown: instance/usage)`|traza|sí: pullup, pulldown|Una etapa de salida push-pull: únicamente los transistores de salida, un pull-up en serie con un pull-down; su polarización y la etapa que los excita no forman parte de ella.|
|`$TRANSCONDUCTOR_Z_AMPLIFIER(transconductor: instance/usage, load: instance/usage)`|traza|sí: transconductor, load|Una etapa amplificadora: un transconductor y su carga.|
|`$DEGENERATED_TRANSCONDUCTOR_Z_AMPLIFIER(transconductor: instance/usage, load: instance/usage, degenerator: instance/usage)`|traza|sí: transconductor, load, degenerator|Una etapa amplificadora degenerada: un transconductor, su carga y su degeneración.|
|`$PASS_SWITCH(device: instance/usage)`|solo significado||Un interruptor de paso que deja pasar o bloquea una señal entre sus dos lados.|
|`$RESET_SWITCH(device: instance)`|solo significado||Un transistor de precarga, ecualización o reinicio.|
|`$GROUND_SWITCH(device: instance)`|solo significado||Un interruptor de lado bajo conectado a una red de tierra.|
|`$POWER_SWITCH(device: instance)`|solo significado||Un interruptor de lado alto conectado a una red de alimentación.|
|`$TRANSMISSION_GATE(pmos: instance/usage, nmos: instance/usage)`|coloca, traza; punto propio en el centro|sí: pmos, nmos|Una puerta de transmisión CMOS, con el PMOS encima del NMOS.|
|`$TRANSMISSION_GATE_HORIZONTAL(pmos: instance/usage, nmos: instance/usage)`|coloca, traza; punto propio en el centro|sí: pmos, nmos|Una puerta de transmisión CMOS dibujada lado a lado, con el PMOS a la izquierda.|
|`$MULTI_POLE_SWITCH({throws}: instance/usage)`|traza|sí: throws|Un interruptor de varias vías que comparten un mismo polo.|
|`$BOOTSTRAPPED_SWITCH(device: instance/usage)`|solo significado||Un interruptor de muestreo con bootstrap.|
|`$SC_SAMPLER({switch}: instance/usage, {capacitor}: instance/usage)`|traza||Un muestreador de capacidades conmutadas: interruptores de muestreo con condensadores de muestreo.|
|`$CHARGE_SHARING(a: instance/usage, switch: instance/usage, b: instance/usage)`|traza||Una red de reparto de carga: dos condensadores o bancos de condensadores que comparten carga a través de un interruptor.|
|`$CHOPPER({straight}: instance/usage, {crossed}: instance/usage)`|traza||Un chopper: un puente de cuatro interruptores que invierte la polaridad de una señal.|
|`$FLYING_CAPACITOR(capacitor: instance/usage)`|solo significado||Un condensador volante conmutado entre fases de reloj.|
|`$CMOS_INVERTER(pmos: instance, nmos: instance)`|coloca, traza; punto propio en el centro|mediante `$CMOS_GATE`: pmos, nmos|Un inversor CMOS, con el PMOS encima del NMOS.|
|`$CMOS_INVERTER_HORIZONTAL(pmos: instance, nmos: instance)`|coloca, traza; punto propio en el centro|mediante `$CMOS_GATE`: pmos, nmos|Un inversor CMOS dibujado lado a lado, con el PMOS a la izquierda.|
|`$CMOS_GATE({pullup}: instance/usage, {pulldown}: instance/usage)`|traza|sí: pullup, pulldown|Una puerta lógica CMOS estática: una red pull-up y una red pull-down que concurren en serie en la salida.|
|`$LATCH({members}: instance/usage)`|traza|sí: members|Un latch: un núcleo biestable con los dispositivos a su alrededor.|
|`$INVERTER_LATCH(a: instance/usage, b: instance/usage)`|traza|mediante `$LATCH`: a, b|Un latch de dos inversores acoplados en cruz.|
|`$SHIFT_REGISTER({stages}: instance/usage)`|solo significado||Un registro de desplazamiento: una cadena de biestables o latches, a menudo dibujada superpuesta y con los relojes rara vez cableados.|
|`$DIVIDER(upper: instance/usage, lower: instance/usage)`|traza||Un divisor de dos ramas en serie, superior e inferior.|
|`$RESISTOR_LADDER({resistors}: instance)`|coloca, traza; punto propio en x|sí: resistors|Una escalera de resistencias en serie en una columna, de arriba abajo.|
|`$RESISTOR_LADDER_HORIZONTAL({resistors}: instance)`|coloca, traza; punto propio en y|sí: resistors|Una escalera de resistencias en serie dibujada en fila.|
|`$PI_NETWORK(shunt1: instance/usage, series: instance/usage, shunt2: instance/usage)`|traza||Una red en pi: una rama en paralelo, una rama en serie y otra rama en paralelo.|
|`$T_NETWORK(series1: instance/usage, shunt: instance/usage, series2: instance/usage)`|traza||Una red en T: una rama en serie, una rama en paralelo y otra rama en serie.|
|`$LC_TANK_PARALLEL(inductor: instance/usage, capacitor: instance/usage)`|traza|sí: inductor, capacitor|Un tanque resonante LC en paralelo.|
|`$RLC_TANK_PARALLEL(resistor: instance/usage, inductor: instance/usage, capacitor: instance/usage)`|traza|sí: resistor, inductor, capacitor|Un tanque resonante RLC en paralelo.|
|`$LC_TANK_SERIES(inductor: instance/usage, capacitor: instance/usage)`|traza|sí: inductor, capacitor|Un tanque resonante LC en serie.|
|`$RLC_TANK_SERIES(resistor: instance/usage, inductor: instance/usage, capacitor: instance/usage)`|traza|sí: resistor, inductor, capacitor|Un tanque resonante RLC en serie.|
|`$DC_FEED(device: instance/usage)`|solo significado||Una bobina de choque de RF o resistencia de polarización que suministra una alimentación o polarización de CC a un nodo.|
|`$DECAP(device: instance)`|coloca; punto propio en x||Un condensador de desacoplo en un nodo de alimentación o polarización.|
|`$AC_COUPLE(device: instance/usage)`|solo significado||Un condensador de acoplamiento en serie que deja pasar la señal de CA y bloquea la CC.|
|`$OSCILLATOR({members}: instance/usage)`|traza|sí: members|Un oscilador formado por sus elementos activos y sus elementos resonantes o de retardo.|
|**rnl-std-signal-flow**||||
|`$ADDER(in1: net, in2: net, out: net)`|—||Un sumador ideal de flujo de señal (out = in1 + in2), dibujado con `$ADDER_SYMBOL`.|
|`$SUBTRACTOR(in1: net, in2: net, out: net)`|—||Un restador ideal de flujo de señal (out = in1 - in2), dibujado con `$SUBTRACTOR_SYMBOL`.|
|`$MULTIPLIER(in1: net, in2: net, out: net)`|—||Un multiplicador ideal de flujo de señal (out = in1 * in2), dibujado con `$MULTIPLIER_SYMBOL`.|
|`$INTEGRATOR(in: net, out: net)`|—||Un integrador ideal de tiempo continuo, dibujado con `$INTEGRATOR_SYMBOL`.|
|`$DISCRETE_TIME_INTEGRATOR(in: net, out: net)`|—||Un integrador ideal de tiempo discreto, dibujado con `$DISCRETE_TIME_INTEGRATOR_SYMBOL`.|
|`$QUANTIZER(in: net, out: net)`|—||Un cuantizador ideal, dibujado con `$QUANTIZER_SYMBOL`.|
|`$UNIT_DELAY(in: net, out: net)`|—||Un retardo unitario ideal, dibujado con `$UNIT_DELAY_SYMBOL`.|
|**rnl-std-45deg**||||
|`$ARROW_45(at: point)`|—||Una punta de flecha a 45 grados dibujada en un punto dentro de un símbolo o sobre un cable a 45 grados en la escena.|
|`$CURRENT_ARROW_45(p: terminal/point)`|—||Una flecha de corriente a 45 grados en un terminal o en un punto a lo largo de una rama a 45 grados.|

### Símbolos

Los pines se enumeran según el terminal o rol que vinculan; `pin` (`p`) designa el pin llamado `pin`, vinculado al rol `p`; `pin` (nothing) no vincula nada.

|Símbolo|Pines|Significado|
|---|---|---|
|**rnl-std-basic**|||
|`$RESISTOR`|`p` `n`|Una resistencia.|
|`$VARIABLE_RESISTOR`|`p` `n`|Una resistencia variable: una resistencia atravesada por una flecha.|
|`$CAPACITOR`|`p` `n`|Un condensador.|
|`$VARIABLE_CAPACITOR`|`p` `n`|Un condensador variable: un condensador atravesado por una flecha.|
|`$INDUCTOR`|`p` `n`|Un inductor.|
|`$VARIABLE_INDUCTOR`|`p` `n`|Un inductor variable: un inductor atravesado por una flecha.|
|`$TRANSFORMER`|`p1` `n1` `p2` `n2`|Un transformador: dos devanados acoplados dispuestos uno junto al otro, con sus puntos de polaridad arriba.|
|`$POTENTIOMETER`|`n1` `nw` `n2`|Un potenciómetro de tres terminales, con el cursor en el centro.|
|`$PHOTORESISTOR`|`p` `n`|Una fotorresistencia (resistencia dependiente de la luz).|
|`$POLARIZED_CAPACITOR`|`p` `n`|Un condensador polarizado (electrolítico), con su placa positiva en p.|
|`$CRYSTAL`|`n1` `n2`|Un resonador de cristal de cuarzo.|
|`$SPEAKER`|`p` `n`|Un altavoz.|
|`$VOLTAGE_SOURCE`|`p` `n`|Una fuente de tensión independiente.|
|`$CURRENT_SOURCE`|`p` `n`|Una fuente de corriente independiente, con su flecha de p a n.|
|`$BATTERY`|`p` `n`|Una batería.|
|`$CONTROLLED_VOLTAGE_SOURCE`|`p` `n`|Una fuente de tensión controlada dibujada como un rombo únicamente con sus pines de salida.|
|`$CONTROLLED_CURRENT_SOURCE`|`p` `n`|Una fuente de corriente controlada dibujada como un rombo únicamente con sus pines de salida.|
|`$VCVS_4TERM`|`p` `n` `cp` `cn`|Una fuente de tensión controlada por tensión (E) con sus pines de control a la izquierda.|
|`$VCCS_4TERM`|`p` `n` `cp` `cn`|Una fuente de corriente controlada por tensión (G) con sus pines de control a la izquierda.|
|`$BEHAVIORAL_SOURCE`|`p` `n`|Una fuente de comportamiento (B): un círculo marcado con B.|
|`$RF_PORT`|`p` `n`|Un puerto de RF: una fuente senoidal en serie con su resistencia interna, para el puerto de Spectre.|
|`$SINE_VOLTAGE_SOURCE`|`p` `n`|Una fuente de tensión senoidal, con + en p.|
|`$PULSE_VOLTAGE_SOURCE`|`p` `n`|Una fuente de tensión de pulsos, con + en p.|
|`$MICROPHONE`|`p` `n`|Un micrófono, con + en p.|
|`$AMMETER`|`p` `n`|Un amperímetro, con la corriente medida de p a n.|
|`$SWITCH`|`p` `n`|Un interruptor dibujado abierto, para un interruptor controlado por corriente (W) o el interruptor de Spectre.|
|`$CLOSED_SWITCH`|`p` `n`|Un interruptor dibujado cerrado.|
|`$EXTERNALLY_CONTROLLED_SWITCH`|`p` `n` `cp`|Un interruptor con un único pin de control en un costado.|
|`$VOLTAGE_CONTROLLED_SWITCH`|`p` `n` `cp` `cn`|Un interruptor controlado por tensión (S), con sus pines de control diferenciales debajo.|
|`$SPDT_SWITCH_SYMBOL`|`com` (nothing) `a` (nothing) `b` (nothing)|El símbolo de `$SPDT_SWITCH`: un interruptor unipolar de dos vías cuyos pines com, a y b no vinculan nada y se cablean por nombre (9.2.4).|
|`$TRANSMISSION_LINE`|`p1` `n1` `p2` `n2`|Una línea de transmisión (T u O), con el puerto 1 a la izquierda y el puerto 2 a la derecha.|
|`$RC_LINE`|`a` `b` `ref`|Una línea RC distribuida uniforme (U).|
|`$GROUND_SYMBOL`|`pin` (`p`)|El símbolo de `$GROUND` en la red 0: una tierra de tres barras con su pin arriba.|
|`$NAMED_GROUND_SYMBOL`|`pin` (`p`)|El símbolo de `$GROUND` en cualquier red distinta de 0: una tierra de tres barras con el nombre de su red debajo.|
|`$SUPPLY_SYMBOL`|`pin` (`p`)|El símbolo de `$SUPPLY`: una barra horizontal con el nombre de su red y su pin abajo.|
|`$BIAS_SYMBOL`|`pin` (`p`)|El símbolo de `$BIAS`: una barra inclinada con el nombre de su red y su pin abajo.|
|`$IO_PORT_SYMBOL`|`pin` (`p`)|El símbolo de `$IO_PORT`: un pequeño círculo con el nombre de su red y su pin a la derecha.|
|`$NET_LABEL_SYMBOL`|`pin` (`p`)|El símbolo de `$NET_LABEL`: el nombre de su red, o los nombres de las redes de un pin de bus, a la izquierda de una patilla de una unidad, con su pin orientado a la derecha, como `$IO_PORT_SYMBOL`.|
|`$ARROW_SYMBOL`|—|Una punta de flecha rellena apuntando a la derecha, el símbolo de `$ARROW`; `$CURRENT_ARROW` dibuja una de la mitad de su tamaño.|
|`$NO_CONNECT_SYMBOL`|—|El símbolo de `$NO_CONNECT`: un aspa.|
|`$JUNCTION_SYMBOL`|—|El símbolo de `$JUNCTION`: un punto relleno.|
|**rnl-std-semiconductors**|||
|`$DIODE`|`a` `k`|Un diodo.|
|`$ZENER_DIODE`|`a` `k`|Un diodo Zener.|
|`$LED`|`a` `k`|Un diodo emisor de luz (LED).|
|`$SCHOTTKY_DIODE`|`a` `k`|Un diodo de barrera Schottky.|
|`$SHOCKLEY_DIODE`|`a` `k`|Un diodo Shockley de cuatro capas.|
|`$PUT`|`a` `g` `k`|Un transistor uniunión programable (PUT).|
|`$NMOS`|`d` `g` `s`|Un transistor NMOS de tres pines, con el sustrato sin dibujar.|
|`$NMOS_4TERM`|`d` `g` `s` `b`|Un transistor NMOS con su pin de sustrato alineado con el drenador y la fuente, y su flecha de sustrato en el canal.|
|`$PMOS`|`d` `g` `s`|Un transistor PMOS de tres pines, con el sustrato sin dibujar.|
|`$PMOS_4TERM`|`d` `g` `s` `b`|Un transistor PMOS con su pin de sustrato alineado con el drenador y la fuente.|
|`$NMOS_LOGIC`|`d` `g` `s`|Un transistor NMOS lógico de tres pines sin flecha de fuente.|
|`$PMOS_LOGIC`|`d` `g` `s`|Un transistor PMOS lógico de tres pines sin flecha de fuente, con una burbuja de inversión en la puerta.|
|`$DEPLETION_NMOS`|`d` `g` `s`|Un transistor NMOS de modo de empobrecimiento de tres pines, con el sustrato sin dibujar.|
|`$DEPLETION_NMOS_4TERM`|`d` `g` `s` `b`|Un transistor NMOS de modo de empobrecimiento con su pin de sustrato alineado con el drenador y la fuente.|
|`$DEPLETION_PMOS`|`d` `g` `s`|Un transistor PMOS de modo de empobrecimiento de tres pines, con el sustrato sin dibujar.|
|`$DEPLETION_PMOS_4TERM`|`d` `g` `s` `b`|Un transistor PMOS de modo de empobrecimiento con su pin de sustrato alineado con el drenador y la fuente.|
|`$NDMOS`|`d` `g` `s`|Un transistor DMOS de tipo N de tres pines, con el sustrato sin dibujar.|
|`$NDMOS_4TERM`|`d` `g` `s` `b`|Un transistor DMOS de tipo N con su pin de sustrato alineado con el drenador y la fuente.|
|`$PDMOS`|`d` `g` `s`|Un transistor DMOS de tipo P de tres pines, con el sustrato sin dibujar.|
|`$PDMOS_4TERM`|`d` `g` `s` `b`|Un transistor DMOS de tipo P con su pin de sustrato alineado con el drenador y la fuente.|
|`$NPN`|`c` `b` `e`|Un transistor bipolar NPN, con el sustrato sin dibujar.|
|`$PNP`|`c` `b` `e`|Un transistor bipolar PNP, con el sustrato sin dibujar.|
|`$NJFET`|`d` `g` `s`|Un JFET de canal N, con la flecha de puerta apuntando hacia el canal.|
|`$PJFET`|`d` `g` `s`|Un JFET de canal P, con la flecha de puerta apuntando hacia fuera del canal.|
|`$UJT`|`e` `b1` `b2`|Un transistor uniunión (UJT), con b2 arriba y b1 abajo.|
|**rnl-std-functional**|||
|`$OPAMP`|`inp` `inn` `out`|Un amplificador operacional de tres terminales: inp, inn, out.|
|`$OPAMP_SUPPLY`|`inp` `inn` `out` `vp` `vn`|Un amplificador operacional con pines de alimentación: inp, inn, out, vp, vn.|
|`$OPAMP_INVERTED_SUPPLY`|`inp` `inn` `out` `vp` `vn`|Un amplificador operacional con pines de alimentación, con sus entradas intercambiadas (inn arriba, inp abajo) y sus alimentaciones conservadas (vp arriba, vn abajo), a diferencia de lo que ocurriría al invertir `$OPAMP_SUPPLY` con MX: inp, inn, out, vp, vn.|
|`$DIFFERENTIAL_OPAMP`|`inp` `inn` `outp` `outn`|Un amplificador operacional totalmente diferencial: inp, inn, outp, outn.|
|`$DIFFERENTIAL_OPAMP_SUPPLY`|`inp` `inn` `outp` `outn` `vp` `vn`|Un amplificador operacional totalmente diferencial con pines de alimentación: inp, inn, outp, outn, vp, vn.|
|`$DIFFERENTIAL_OPAMP_INVERTED_SUPPLY`|`inp` `inn` `outp` `outn` `vp` `vn`|Un amplificador operacional totalmente diferencial con pines de alimentación, con sus entradas y salidas intercambiadas (inn y outn arriba, inp y outp abajo) y sus alimentaciones conservadas (vp arriba, vn abajo), a diferencia de lo que ocurriría al invertir `$DIFFERENTIAL_OPAMP_SUPPLY` con MX: inp, inn, outp, outn, vp, vn.|
|`$DIFFERENTIAL_OPAMP_TWISTED`|`inp` `inn` `outp` `outn`|Un amplificador operacional totalmente diferencial con la polaridad de salida invertida (outn arriba, outp abajo): inp, inn, outp, outn.|
|`$DIFFERENTIAL_OPAMP_TWISTED_SUPPLY`|`inp` `inn` `outp` `outn` `vp` `vn`|Un amplificador operacional totalmente diferencial con salidas invertidas y pines de alimentación: inp, inn, outp, outn, vp, vn.|
|`$DIFFERENTIAL_OPAMP_TWISTED_INVERTED_SUPPLY`|`inp` `inn` `outp` `outn` `vp` `vn`|Un amplificador operacional totalmente diferencial con pines de alimentación, con sus entradas intercambiadas (inn arriba, inp abajo), sus salidas como en `$DIFFERENTIAL_OPAMP_SUPPLY` (outp arriba, outn abajo) y sus alimentaciones conservadas (vp arriba, vn abajo), a diferencia de lo que ocurriría al invertir `$DIFFERENTIAL_OPAMP_TWISTED_SUPPLY` con MX: inp, inn, outp, outn, vp, vn.|
|`$COMPARATOR`|`inp` `inn` `out`|Un comparador de tres terminales: inp, inn, out.|
|`$COMPARATOR_SUPPLY`|`inp` `inn` `out` `vp` `vn`|Un comparador con pines de alimentación: inp, inn, out, vp, vn.|
|`$COMPARATOR_INVERTED_SUPPLY`|`inp` `inn` `out` `vp` `vn`|Un comparador con pines de alimentación, con sus entradas intercambiadas (inn arriba, inp abajo) y sus alimentaciones conservadas (vp arriba, vn abajo), a diferencia de lo que ocurriría al invertir `$COMPARATOR_SUPPLY` con MX; su escalón se lee igual que en `$COMPARATOR_SUPPLY`: inp, inn, out, vp, vn.|
|`$VOLTAGE_AMPLIFIER`|`in` `out`|Un amplificador de tensión, con un texto centrado opcional de clave label: in, out.|
|`$VOLTAGE_AMPLIFIER_SUPPLY`|`in` `out` `vp` `vn`|Un amplificador de tensión con pines de alimentación: in, out, vp, vn.|
|`$TRANSCONDUCTOR`|`in` `out`|Un transconductor (una celda gm), con un texto centrado de clave label, g_m por defecto: in, out.|
|`$TRANSCONDUCTOR_SUPPLY`|`in` `out` `vp` `vn`|Un transconductor con pines de alimentación y un texto centrado de clave label, g_m por defecto: in, out, vp, vn.|
|`$DIFFERENTIAL_TRANSCONDUCTOR`|`inp` `inn` `out`|Un transconductor con entrada diferencial y un texto de clave label (g_m por defecto): inp, inn, out.|
|`$DIFFERENTIAL_TRANSCONDUCTOR_SUPPLY`|`inp` `inn` `out` `vp` `vn`|Un transconductor de entrada diferencial con pines de alimentación y un texto de clave label (g_m por defecto): inp, inn, out, vp, vn.|
|`$DIFFERENTIAL_TRANSCONDUCTOR_INVERTED_SUPPLY`|`inp` `inn` `out` `vp` `vn`|Un transconductor de entrada diferencial con pines de alimentación y un texto de clave label (g_m por defecto), con sus entradas intercambiadas (inn arriba, inp abajo) y sus alimentaciones conservadas (vp arriba, vn abajo), a diferencia de lo que ocurriría al invertir `$DIFFERENTIAL_TRANSCONDUCTOR_SUPPLY` con MX: inp, inn, out, vp, vn.|
|`$ADC`|`in` `out`|Un convertidor analógico-digital: in, out.|
|`$ADC_SUPPLY`|`in` `out` `vp` `vn`|Un convertidor analógico-digital con pines de alimentación: in, out, vp, vn.|
|`$DAC`|`in` `out`|Un convertidor digital-analógico: in, out.|
|`$DAC_SUPPLY`|`in` `out` `vp` `vn`|Un convertidor digital-analógico con pines de alimentación: in, out, vp, vn.|
|`$CHOPPER_SYMBOL`|`inp` `inn` `outp` `outn`|Un bloque modulador chopper (distinto de la estructura de puente de interruptores `$CHOPPER`): inp, inn, outp, outn.|
|**rnl-std-rf**|||
|`$CENTER_TAP_INDUCTOR`|`n1` `n2` `tap`|Un inductor con toma central.|
|`$T_COIL`|`n1` `n2` `tap`|Una bobina en T: dos bobinas acopladas con una toma central y un condensador en puente.|
|**rnl-std-logic**|||
|`$AND_2INPUT`|`a` `b` `y`|Una puerta AND de dos entradas: a, b, y.|
|`$AND_3INPUT`|`a` `b` `c` `y`|Una puerta AND de tres entradas: a, b, c, y.|
|`$NAND_2INPUT`|`a` `b` `y`|Una puerta NAND de dos entradas: a, b, y.|
|`$NAND_3INPUT`|`a` `b` `c` `y`|Una puerta NAND de tres entradas: a, b, c, y.|
|`$OR_2INPUT`|`a` `b` `y`|Una puerta OR de dos entradas: a, b, y.|
|`$OR_3INPUT`|`a` `b` `c` `y`|Una puerta OR de tres entradas: a, b, c, y.|
|`$NOR_2INPUT`|`a` `b` `y`|Una puerta NOR de dos entradas: a, b, y.|
|`$NOR_3INPUT`|`a` `b` `c` `y`|Una puerta NOR de tres entradas: a, b, c, y.|
|`$XOR_2INPUT`|`a` `b` `y`|Una puerta XOR de dos entradas: a, b, y.|
|`$XOR_3INPUT`|`a` `b` `c` `y`|Una puerta XOR de tres entradas: a, b, c, y.|
|`$XNOR_2INPUT`|`a` `b` `y`|Una puerta XNOR de dos entradas: a, b, y.|
|`$XNOR_3INPUT`|`a` `b` `c` `y`|Una puerta XNOR de tres entradas: a, b, c, y.|
|`$AND_4INPUT`|`a` `b` `c` `d` `y`|Una puerta AND de cuatro entradas sobre puntos enteros de cuadrícula: a, b, c, d, y.|
|`$NAND_4INPUT`|`a` `b` `c` `d` `y`|Una puerta NAND de cuatro entradas sobre puntos enteros de cuadrícula: a, b, c, d, y.|
|`$OR_4INPUT`|`a` `b` `c` `d` `y`|Una puerta OR de cuatro entradas sobre puntos enteros de cuadrícula: a, b, c, d, y.|
|`$NOR_4INPUT`|`a` `b` `c` `d` `y`|Una puerta NOR de cuatro entradas sobre puntos enteros de cuadrícula: a, b, c, d, y.|
|`$XOR_4INPUT`|`a` `b` `c` `d` `y`|Una puerta XOR de cuatro entradas sobre puntos enteros de cuadrícula: a, b, c, d, y.|
|`$XNOR_4INPUT`|`a` `b` `c` `d` `y`|Una puerta XNOR de cuatro entradas sobre puntos enteros de cuadrícula: a, b, c, d, y.|
|`$BUFFER`|`a` `y`|Un búfer: a, y.|
|`$INVERTER`|`a` `y`|Un inversor (una puerta NOT): a, y.|
|`$DELAY_CELL`|`a` `y`|Una celda de retardo: a, y.|
|`$D_FLIP_FLOP`|`d` `ck` `q` `qb`|Un biestable D con la salida Q y su complemento: d, ck, q, qb.|
|`$D_FLIP_FLOP_Q_ONLY`|`d` `ck` `q`|Un biestable D solo con la salida Q: d, ck, q.|
|`$D_FLIP_FLOP_WITH_RESET`|`d` `ck` `rst` `q` `qb`|Un biestable D con reinicio: d, ck, rst, q, qb.|
|`$AND_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|Una puerta AND de 2 entradas con pines de alimentación: a, b, y, vp, vn.|
|`$AND_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|Una puerta AND de 3 entradas con pines de alimentación: a, b, c, y, vp, vn.|
|`$AND_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|Una puerta AND de 4 entradas con pines de alimentación: a, b, c, d, y, vp, vn.|
|`$NAND_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|Una puerta NAND de 2 entradas con pines de alimentación: a, b, y, vp, vn.|
|`$NAND_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|Una puerta NAND de 3 entradas con pines de alimentación: a, b, c, y, vp, vn.|
|`$NAND_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|Una puerta NAND de 4 entradas con pines de alimentación: a, b, c, d, y, vp, vn.|
|`$OR_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|Una puerta OR de 2 entradas con pines de alimentación: a, b, y, vp, vn.|
|`$OR_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|Una puerta OR de 3 entradas con pines de alimentación: a, b, c, y, vp, vn.|
|`$OR_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|Una puerta OR de 4 entradas con pines de alimentación: a, b, c, d, y, vp, vn.|
|`$NOR_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|Una puerta NOR de 2 entradas con pines de alimentación: a, b, y, vp, vn.|
|`$NOR_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|Una puerta NOR de 3 entradas con pines de alimentación: a, b, c, y, vp, vn.|
|`$NOR_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|Una puerta NOR de 4 entradas con pines de alimentación: a, b, c, d, y, vp, vn.|
|`$XOR_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|Una puerta XOR de 2 entradas con pines de alimentación: a, b, y, vp, vn.|
|`$XOR_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|Una puerta XOR de 3 entradas con pines de alimentación: a, b, c, y, vp, vn.|
|`$XOR_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|Una puerta XOR de 4 entradas con pines de alimentación: a, b, c, d, y, vp, vn.|
|`$XNOR_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|Una puerta XNOR de 2 entradas con pines de alimentación: a, b, y, vp, vn.|
|`$XNOR_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|Una puerta XNOR de 3 entradas con pines de alimentación: a, b, c, y, vp, vn.|
|`$XNOR_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|Una puerta XNOR de 4 entradas con pines de alimentación: a, b, c, d, y, vp, vn.|
|`$BUFFER_SUPPLY`|`a` `y` `vp` `vn`|Un búfer con pines de alimentación: a, y, vp, vn.|
|`$INVERTER_SUPPLY`|`a` `y` `vp` `vn`|Un inversor con pines de alimentación: a, y, vp, vn.|
|`$DELAY_CELL_SUPPLY`|`a` `y` `vp` `vn`|Una celda de retardo con pines de alimentación: a, y, vp, vn.|
|`$D_FLIP_FLOP_SUPPLY`|`d` `ck` `q` `qb` `vp` `vn`|Un biestable D con pines de alimentación: d, ck, q, qb, vp, vn.|
|`$D_FLIP_FLOP_Q_ONLY_SUPPLY`|`d` `ck` `q` `vp` `vn`|Un biestable D solo con Q y pines de alimentación: d, ck, q, vp, vn.|
|`$D_FLIP_FLOP_WITH_RESET_SUPPLY`|`d` `ck` `rst` `q` `qb` `vp` `vn`|Un biestable D con reinicio y pines de alimentación (vp y vn alineados en x = 0): d, ck, rst, q, qb, vp, vn.|
|**rnl-std-signal-flow**|||
|`$ADDER_SYMBOL`|`a` (`in1`) `b` (`in2`) `y` (`out`)|El símbolo de `$ADDER`: un círculo sumador con puntas de flecha en sus patillas de entrada a (in1) y b (in2), y salida y (out).|
|`$SUBTRACTOR_SYMBOL`|`a` (`in1`) `b` (`in2`) `y` (`out`)|El símbolo de `$SUBTRACTOR`: un círculo sumador con + en el centro y - en el exterior junto a su entrada negativa b (in2), entradas a (in1) y b (in2), y salida y (out).|
|`$MULTIPLIER_SYMBOL`|`a` (`in1`) `b` (`in2`) `y` (`out`)|El símbolo de `$MULTIPLIER`: un círculo multiplicador con puntas de flecha en sus patillas de entrada a (in1) y b (in2), y salida y (out).|
|`$INTEGRATOR_SYMBOL`|`a` (`in`) `y` (`out`)|El símbolo de `$INTEGRATOR`: un recuadro marcado con 1/s, entrada a (in) y salida y (out).|
|`$DISCRETE_TIME_INTEGRATOR_SYMBOL`|`a` (`in`) `y` (`out`)|El símbolo de `$DISCRETE_TIME_INTEGRATOR`: un recuadro marcado con su función de transferencia en el dominio z, entrada a (in) y salida y (out).|
|`$QUANTIZER_SYMBOL`|`a` (`in`) `y` (`out`)|El símbolo de `$QUANTIZER`: un recuadro con trazo en escalera, entrada a (in) y salida y (out).|
|`$UNIT_DELAY_SYMBOL`|`a` (`in`) `y` (`out`)|El símbolo de `$UNIT_DELAY`: un recuadro marcado con z elevado a menos uno, entrada a (in) y salida y (out).|
|**rnl-std-45deg**|||
|`$RESISTOR_45`|`p` `n`|Una resistencia a 45 grados.|
|`$CAPACITOR_45`|`p` `n`|Un condensador a 45 grados.|
|`$INDUCTOR_45`|`p` `n`|Un inductor a 45 grados (sirve también para devanados en diagonal de transformador o balún).|
|`$SWITCH_45`|`p` `n`|Un interruptor abierto a 45 grados.|
|`$VOLTAGE_SOURCE_45`|`p` `n`|Una fuente de tensión independiente a 45 grados, con + junto a p y - junto a n mantenidos derechos.|
|`$CURRENT_SOURCE_45`|`p` `n`|Una fuente de corriente independiente a 45 grados, con flecha de p a n.|
|`$DIODE_45`|`a` `k`|Un diodo a 45 grados.|
|`$ZENER_DIODE_45`|`a` `k`|Un diodo Zener a 45 grados.|
|`$SCHOTTKY_DIODE_45`|`a` `k`|Un diodo Schottky a 45 grados.|
|`$LED_45`|`a` `k`|Un diodo emisor de luz (LED) a 45 grados.|
|`$NMOS_45`|`d` `g` `s`|Un transistor NMOS de tres pines a 45 grados.|
|`$NMOS_4TERM_45`|`d` `g` `s` `b`|Un transistor NMOS de cuatro pines a 45 grados, con el sustrato en (0, 0).|
|`$PMOS_45`|`d` `g` `s`|Un transistor PMOS de tres pines a 45 grados.|
|`$PMOS_4TERM_45`|`d` `g` `s` `b`|Un transistor PMOS de cuatro pines a 45 grados, con el sustrato en (0, 0).|
|`$NPN_45`|`c` `b` `e`|Un transistor bipolar NPN a 45 grados.|
|`$PNP_45`|`c` `b` `e`|Un transistor bipolar PNP a 45 grados.|
|`$ARROW_45_SYMBOL`|—|Una punta de flecha rellena que apunta a -45 grados (abajo a la derecha, a lo largo de p->n / a->k de los símbolos a 45 grados bajo R90), girando hacia las cuatro diagonales bajo R0/R90/R180/R270.|

### Conjuntos y estilos

|Nombre|Paquete|Significado|
|---|---|---|
|`$GROUND_NET_SET`|rnl-std-basic|Las redes de tierra (0 por defecto), que un archivo edita con ADD-TO y REMOVE-FROM; rnl-std-fill asigna un tramo corto de tierra a sus pines sin cablear y las deja sin trazar. Cómo se dibuja una tierra no depende de este conjunto.|
|`$SUPPLY_NET_SET`|rnl-std-basic|Las redes de alimentación (ninguna por defecto), que un archivo edita con ADD-TO y REMOVE-FROM.|
|`$BIAS_NET_SET`|rnl-std-basic|Las redes de polarización (ninguna por defecto), que un archivo edita con ADD-TO y REMOVE-FROM.|
|`$PORT_FILL_SET`|rnl-std-basic|Las redes de puerto a las que rnl-std-fill asigna una etiqueta de puerto: toda red de puerto de todo nivel, incluidas las redes de alimentación y de tierra, menos las que ya tienen un `$IO_PORT` escrito y las de un nivel con un `$PORT_LIST`.|
|`$GROUND_FILL_SET`|rnl-std-fill|Los pines sin cablear en redes de tierra a los que el relleno asigna un tramo corto de tierra.|
|`$SUPPLY_FILL_SET`|rnl-std-fill|Los pines sin cablear en redes de alimentación a los que el relleno asigna un tramo corto de alimentación.|
|`$BIAS_FILL_SET`|rnl-std-fill|Los pines sin cablear en redes de polarización a los que el relleno asigna un tramo corto de polarización.|
|`$LABEL_FILL_SET`|rnl-std-fill|Los pines restantes sin cablear en redes a los que el relleno asigna un tramo corto de etiqueta de red; un pin de bus recibe uno solo.|
|`$base`|rnl-std-style|El estilo base de todo objeto: líneas, textos y rellenos tal como los dibuja Analog Canvas.|
|`$bus`|rnl-std-style|El estilo de un bus: sus líneas en negrita.|
|`$signal`|rnl-std-style|Un estilo para cables de señal: en azul y en negrita.|
|`$note`|rnl-std-style|Un estilo para notas: en gris y con texto más pequeño.|
|`$grey`|rnl-std-style|Un estilo en gris para líneas y textos.|

<!-- lists:end -->
