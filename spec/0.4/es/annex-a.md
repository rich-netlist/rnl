<!-- translated from en/annex-a.md sha256:2adc09eec4f4 -->
## Anexo A Firmas de las palabras clave (normativo)

Este anexo se genera a partir de la lista de palabras `rnl-words-0.4.json` (5.3); donde ambos difieran, se trata de una errata. Enumera qué admite y qué devuelve cada palabra clave; su significado se encuentra en la sección indicada en la última columna.

- Las **ranuras** se enumeran en el orden estándar, con sus nombres (que pueden escribirse como roles, 5.3) y lo que admiten: *one*: un único objeto; dado un conjunto, la palabra clave se aplica a cada miembro (5.2.1), salvo que PRIORITY admite un conjunto marcado con GROUP como el propio grupo (14.8); *set*: un conjunto; los objetos sueltos escritos para la ranura forman un conjunto (5.2.1); *any*: un objeto o un conjunto, usado según indica la palabra clave (para DISTANCE, un conjunto representa a cada uno de sus miembros, 14.5); *shape*: un punto, o un conjunto de puntos (dos formas de un gráfico, 9.2.5); *body*: sentencias entre llaves, encerradas (5.4); *definition*: una definición: un nombre `$`, un nombre `%` vinculado a una definición, o una definición anónima (5.6, 6.6). La entrada absorbida es estricta; la entrada encerrada es tolerante (5.4, 7.3).
- Las **clases** de las ranuras son nombres de clase (`instance`, `terminal`, `pin`, `net`, `point`, `wire`, `usage`, `bundle`, `text`, `annotation`, `label`) o: *any*, cualquier objeto; *level*, `#` aislado: el propio nivel; *owner*, `?` aislado: en un cuerpo de símbolo, el objeto dibujado con el símbolo; en el cuerpo de una definición general, el uso que se está expandiendo (9.4); *position*, el subobjeto `::position` de un objeto (su origen); *locatable*, un objeto con posición: instancia, agrupación, pin, terminal con pin, punto, texto, el centro de un gráfico escrito con un centro, un uso por su propio punto; un conjunto dado para él representa a cada uno de sus miembros (14.1); *orientable*, un objeto con orientación: instancia, agrupación, uso con símbolo; *constraint*, un objeto creado por una palabra clave de restricción; *set*, un conjunto, escrito como un nombre; en ASSERT-TOPOLOGY también un elemento que es un `{…}` escrito explícitamente (15.4); *empty node*, `&` aislado: ninguna red; admitido por los roles de clase `net` y por la ranura de red de WIRE, y en otros lugares como un conjunto vacío de redes (6.3).
- Los **parámetros** marcados como *posición 1*, *posición 2* pueden darse por posición, en ese orden; todos los demás únicamente por nombre (4.2). `priority` admite siempre `"disabled"`, `"low"`, `"normal"` o `"high"`; sin él se aplica la prioridad del encabezado del registro, salvo que AS y ATTACH toman únicamente `"disabled"` de él (14.8).
- **Resultado**: el objeto que devuelve la sentencia (5.5) y cómo sustituyen sus hechos (5.8). Sin sustitución, los hechos se acumulan.
- Salvo indicación en contrario, una palabra clave puede escribirse en cualquier registro de un nivel; *cuerpos de símbolo* significa que también puede escribirse en un cuerpo de DEFINE-SYMBOL.

### A.1 El registro

`@RNL(…) {…}` consigna sus sentencias en el nivel (5.4). Campos del encabezado del registro (2.8), todos dados por nombre:

| Campo | Valores | Por defecto | Afecta a la interpretación |
|---|---|---|---|
| `type` | `"authored"`, `"generated"`, `"header"` | `"authored"` | sí |
| `scope` | cadena | — | sí |
| `priority` | `"disabled"`, `"low"`, `"normal"`, `"high"` | `"normal"` | sí |
| `author` | cadena | — | no |
| `hash` | cadena | — | no |
| `numbering` | cadena | — | sí |
| `modified` | cadena | — | no |
| `language` | `"spice"`, `"spectre"` | — | sí |

### A.2 Registros de encabezado y paquetes

| Palabra clave | Ranuras | Parámetros | Resultado | Sección |
|---|---|---|---|---|
| `@VERSION` (solo registros de encabezado) | — | `version` (posición 1; obligatorio) | — | 16.1 |
| `@PACKAGE` (solo registro de encabezado de un archivo de paquete) | — | `name` (obligatorio), `version` (obligatorio) | — | 2.7 |
| `@USE-PACKAGE` (también en registros de encabezado) | `body`: body, opcional | `name` (obligatorio), `version`, `path` | — | 16.2 |
| `@ALIAS` (solo registros de encabezado) | — | `alias` (posición 1; obligatorio), `original` (posición 2; obligatorio), `definition` | — | 16.5 |
| `@USE-ALIASES` (solo registros de encabezado) | — | `name` (obligatorio), `version`, `path` | — | 16.5 |
| `@DEVICE` (solo registros de encabezado) | — | `name`, `model`, `terminals` (matriz), `type` (cadena); al menos uno de `name`, `model`; al menos uno de `terminals`, `type` | — | 6.12 |

### A.3 Definiciones

| Palabra clave | Ranuras | Parámetros | Resultado | Sección |
|---|---|---|---|---|
| `@DEFINE-SYMBOL` (también en cuerpos de símbolo) | `body`: body | `box` (matriz) | una definición (símbolo) | 9.2 |
| `@DEFINE-STYLE` | `extends`: body (un estilo) | propiedades de estilo (9.3) | una definición (estilo) | 9.3 |
| `@DEFINE` | `body`: body | `roles` (obligatorio; matriz), `parameters` (matriz) | una definición (general) | 9.4 |

### A.4 Contenido de símbolos y gráficos

| Palabra clave | Ranuras | Parámetros | Resultado | Sección |
|---|---|---|---|---|
| `@PIN` (solo cuerpos de símbolo) | `position`: one (point) | `terminal` (terminal, o matriz no vacía de terminales; puede omitirse), `label`, `direction` (`"right"`, `"up"`, `"left"`, `"down"`) | un pin nuevo | 9.2.4 |
| `@LINE` (también en cuerpos de símbolo) | `points`: set (point, pin, terminal) | `arrow` (`"none"`, `"start"`, `"end"`, `"both"`; por defecto `"none"`), `fill` (`"none"`, `"linecolor"`, `"fillcolor"`; por defecto `"none"`), propiedades de estilo de línea | un gráfico del símbolo; en la escena, una anotación nueva (13.4) | 9.2.5 |
| `@RECTANGLE` (también en cuerpos de símbolo) | `points`: shape (point, pin, terminal) | `width` (número), `height` (número), `fill` (`"none"`, `"linecolor"`, `"fillcolor"`; por defecto `"none"`), propiedades de estilo de área; `width` y `height` juntos o ninguno | un gráfico del símbolo; en la escena, una anotación nueva (13.4) | 9.2.5 |
| `@CIRCLE` (también en cuerpos de símbolo) | `points`: shape (point, pin, terminal) | `radius` (número), `fill` (`"none"`, `"linecolor"`, `"fillcolor"`; por defecto `"none"`), propiedades de estilo de área | un gráfico del símbolo; en la escena, una anotación nueva (13.4) | 9.2.5 |
| `@ELLIPSE` (también en cuerpos de símbolo) | `points`: shape (point, pin, terminal) | `width` (número), `height` (número), `fill` (`"none"`, `"linecolor"`, `"fillcolor"`; por defecto `"none"`), propiedades de estilo de área; `width` y `height` juntos o ninguno | un gráfico del símbolo; en la escena, una anotación nueva (13.4) | 9.2.5 |
| `@ARC` (también en cuerpos de símbolo) | `points`: set (point, pin, terminal) | `arrow` (`"none"`, `"start"`, `"end"`, `"both"`; por defecto `"none"`), `fill` (`"none"`, `"linecolor"`, `"fillcolor"`; por defecto `"none"`), propiedades de estilo de línea | un gráfico del símbolo; en la escena, una anotación nueva (13.4) | 9.2.5 |
| `@BEZIER` (también en cuerpos de símbolo) | `points`: set (point, pin, terminal) | `arrow` (`"none"`, `"start"`, `"end"`, `"both"`; por defecto `"none"`), `fill` (`"none"`, `"linecolor"`, `"fillcolor"`; por defecto `"none"`), propiedades de estilo de línea | un gráfico del símbolo; en la escena, una anotación nueva (13.4) | 9.2.5 |
| `@ANNOTATION` | `at`: one (locatable); `source`: one (un símbolo, una definición general o un uso), escrito al final | `rotate` (número, grados en sentido antihorario; por defecto `0`), `mirror` (booleano; por defecto `false`), propiedades de estilo, y los parámetros de un símbolo o definición general dados como `source`, por nombre (9.4) | una anotación nueva | 13.5 |
| `@TEXT` (también en cuerpos de símbolo) | `attach`: one (point, instance, usage, bundle, pin, terminal, wire, annotation, net, `?`) | `text` (posición 1; obligatorio), `key`, `dx` (número), `dy` (número), `halign` (`"start"`, `"middle"`, `"end"`; por defecto `"start"`), `valign` (`"baseline"`, `"top"`, `"middle"`, `"bottom"`; por defecto `"baseline"`), `rotate` (número, grados en sentido antihorario; por defecto `0`), `fix_rotate` (booleano; por defecto `false`), `upright` (booleano; por defecto `true`), propiedades de estilo de texto | un texto nuevo; con `key`, un subobjeto del objeto adjuntado; sustituye por clave (clave: objeto adjuntado y clave, cuando se da clave) | 13.3 |

### A.5 Colocación y apariencia

| Palabra clave | Ranuras | Parámetros | Resultado | Sección |
|---|---|---|---|---|
| `@PLACE` (también en cuerpos de símbolo) | `target`: one (instance, usage, bundle, point, text, `::position`) | `x` (posición 1; número), `y` (posición 2; número), `priority`; al menos uno de `x`, `y` | el objeto de entrada; hecho geométrico (clave: objeto y eje) | 11.1 |
| `@ORIENT` (también en cuerpos de símbolo) | `target`: one (instance, usage, bundle, pin) | `code` (posición 1; obligatorio; `"R0"`, `"R90"`, `"R180"`, `"R270"`, `"MX"`, `"MY"`, `"MXR90"`, `"MYR90"`, o las palabras `"left"`, `"down"`, `"right"`, `"up"` para los cuatro primeros; o una matriz no vacía de ellos), `priority` | el objeto de entrada; hecho geométrico (clave: objeto, o pin) | 11.2 |
| `@LAYER` | `target`: one (instance, usage, bundle, wire, annotation, text) | `layer` (posición 1; obligatorio; `"bottom"`, `"middle"`, `"top"`) | el objeto de entrada; sustituye por clave (clave: objeto) | 11.6 |
| `@SET-SYMBOL` (también en cuerpos de símbolo) | `target`: one (instance, usage, bundle, `#`); `symbol`: definition (un símbolo) | `terminals` (matriz de nombres de terminal, o de pares) | el objeto de entrada; cola de apariencia | 11.3 |
| `@SET-STYLE` | `target`: one (instance, usage, bundle, wire, annotation, text); `style`: definition (un estilo) | — | el objeto de entrada; cola de apariencia | 11.3 |

### A.6 Selecciones

| Palabra clave | Ranuras | Parámetros | Resultado | Sección |
|---|---|---|---|---|
| `@SELECT-INSTANCE` | `scope`: any (`#`, cualquier objeto) | `name`, `model`, `type`, `terminals` (número entero), `symbol` (cadena: un nombre `$`, con `*`), `drawn` (booleano), `role`, `recursive` (booleano; por defecto `false`) | un conjunto nuevo de instancias | 10.1 |
| `@SELECT-TERMINAL` | `scope`: any (`#`, cualquier objeto) | `name`, `model`, `type`, `terminal` (terminal), `net`, `current` (`"in"`, `"out"`, `"unspecified"`), `role`, `recursive` (booleano; por defecto `false`) | un conjunto nuevo de terminales | 10.1 |
| `@SELECT-PIN` | `scope`: any (`#`, cualquier objeto) | `name`, `model`, `type`, `terminal` (terminal), `net`, `wired` (booleano), `role`, `recursive` (booleano; por defecto `false`) | un conjunto nuevo de pines | 10.1 |
| `@SELECT-NET` | `scope`: any (`#`, cualquier objeto) | `name`, `port` (booleano), `wired` (booleano), `global` (booleano), `role`, `recursive` (booleano; por defecto `false`) | un conjunto nuevo de redes | 10.1 |
| `@SELECT-WIRE` | `scope`: any (`#`, cualquier objeto) | `net`, `bus` (booleano), `role`, `recursive` (booleano; por defecto `false`) | un conjunto nuevo de cables | 10.1 |
| `@SELECT-LABEL` | `scope`: any (`#`, cualquier objeto) | `key`, `role`, `recursive` (booleano; por defecto `false`) | un conjunto nuevo de etiquetas | 10.1 |
| `@SELECT-ANNOTATION` | `scope`: any (`#`, cualquier objeto) | `role`, `recursive` (booleano; por defecto `false`) | un conjunto nuevo de anotaciones | 10.1 |
| `@SELECT-USAGE` | `scope`: any (`#`, cualquier objeto); `definition`: definition (una definición general), opcional | `symbol` (cadena: un nombre `$`, con `*`), `drawn` (booleano), `recursive` (booleano; por defecto `false`) | un conjunto nuevo de usos | 10.1 |
| `@SELECT-ANCESTOR-USAGE` | `scope`: any (cualquier objeto); `definition`: definition (una definición general) | `recursive` (booleano; por defecto `false`) | un conjunto nuevo de usos | 10.1 |

### A.7 Aplicación de definiciones

| Palabra clave | Ranuras | Parámetros | Resultado | Sección |
|---|---|---|---|---|
| `@AS` (también en cuerpos de símbolo) | los roles de la definición, luego la definición (`definition`, escrita al final) | los parámetros de la definición (por posición en el orden de su lista de parámetros, o por nombre; 9.4), `priority` | un uso nuevo | 9.4 |
| `@ATTACH` (también en cuerpos de símbolo) | la definición, luego sus roles (`definition`, escrita al principio) | los parámetros de la definición (por posición en el orden de su lista de parámetros, o por nombre; 9.4), `priority` | un uso nuevo | 9.4 |

### A.8 Cables

| Palabra clave | Ranuras | Parámetros | Resultado | Sección |
|---|---|---|---|---|
| `@WIRE` | `net`: any (net, o el nodo vacío); `points`: set (point, pin; un terminal representa a su pin, 5.2.4) | `mode` (`"auto"`, `"straight"`, `"horizontal-vertical"`, `"vertical-horizontal"`, `"manhattan"`, `"manhattan45"`; por defecto `"auto"`), propiedades de estilo de cable | un cable nuevo; dado un cable, lo reescribe, sustituyendo por clave (clave: ese cable) | 13.1 |
| `@ROUTE-NET` | `net`: one (net) | `mode` (`"auto"`, `"straight"`, `"horizontal-vertical"`, `"vertical-horizontal"`, `"manhattan"`, `"manhattan45"`; por defecto `"auto"`), propiedades de estilo de cable, `priority` | un trazado nuevo | 13.7 |
| `@ROUTE-OBJECT` | `objects`: set (cualquier objeto) | `mode` (como ROUTE-NET), propiedades de estilo de cable, `priority` | el objeto de entrada | 13.7 |
| `@UNROUTE-NET` | `net`: one (net) | `priority` | el objeto de entrada | 13.7 |
| `@UNROUTE-OBJECT` | `objects`: set (cualquier objeto) | `priority` | el objeto de entrada | 13.7 |

### A.9 Conjuntos, grupos y agrupaciones

| Palabra clave | Ranuras | Parámetros | Resultado | Sección |
|---|---|---|---|---|
| `@BUNDLE` | `members`: set (instance) | — | una agrupación nueva | 11.8 |
| `@GROUP` | `members`: set (cualquier objeto) | `spacing_x` (número entero), `spacing_y` (número entero) | el objeto de entrada | 11.5 |
| `@ADD-TO` | `items`: any (cualquier objeto); `target`: any (set) | — | el conjunto destino | 5.2.2 |
| `@REMOVE-FROM` | `items`: any (cualquier objeto); `target`: any (set) | — | el conjunto destino | 5.2.2 |
| `@UNION` | `sets`: set (cualquier objeto) | — | un conjunto nuevo | 5.2.2 |

### A.10 Restricciones y prioridad

| Palabra clave | Ranuras | Parámetros | Resultado | Sección |
|---|---|---|---|---|
| `@ALIGN` | `members`: set (locatable) | `axis` (posición 1; obligatorio; `"x"`, `"y"`), `priority` | una restricción nueva; hecho geométrico | 14.2 |
| `@ORDER` | `members`: set (locatable) | `direction` (posición 1; obligatorio; `"left-to-right"`, `"right-to-left"`, `"bottom-to-top"`, `"top-to-bottom"`), `priority` | una restricción nueva; hecho geométrico | 14.3 |
| `@DISTRIBUTE` | `members`: set (locatable) | `direction` (posición 1; obligatorio; `"left-to-right"`, `"right-to-left"`, `"bottom-to-top"`, `"top-to-bottom"`), `priority` | una restricción nueva; hecho geométrico | 14.4 |
| `@DISTANCE` | `from`: any (locatable); `to`: any (locatable) | `direction` (posición 1; obligatorio; `"left-to-right"`, `"right-to-left"`, `"bottom-to-top"`, `"top-to-bottom"`), `min` (número), `max` (número), `exact` (número), `priority`; al menos uno de `min`, `max`, `exact`; `exact` no con `min`; `exact` no con `max` | una restricción nueva; hecho geométrico | 14.5 |
| `@ORIENT-RELATION` | `from`: one (orientable, o pin); `to`: one (orientable, o pin, como `from`) | `relation` (posición 1; obligatorio; los ocho códigos de ORIENT, no sus palabras, o una matriz no vacía de ellos), `priority` | una restricción nueva; hecho geométrico | 14.9 |
| `@TABLE` | `rows`: set (locatable; los conjuntos anidados son las listas de la tabla) | `transpose` (booleano; por defecto `false`), `priority` | una restricción nueva; hecho geométrico | 14.11 |
| `@SOLVER-CONFIG` (registro de encabezado sin `language`, registro de encabezado de un archivo de paquete, y nivel) | — | parámetros por nombre del solucionador: números, cadenas, valores de verdad y matrices de estos; `spacing_x`, `spacing_y` (número entero) y `junction` (cadena: un nombre `$`; en un registro de nivel también un nombre `%` del nivel; o vacía) son nombrados por el estándar; las demás claves son las del solucionador | ningún objeto; ajustes del solucionador | 14.10 |
| `@PRIORITY` | `target`: one (constraint, usage, set) | `priority` (posición 1; obligatorio) | el objeto de entrada; sustituye por clave (clave: objeto) | 14.8 |

### A.11 Semántica eléctrica

| Palabra clave | Ranuras | Parámetros | Resultado | Sección |
|---|---|---|---|---|
| `@CURRENT` | `terminal`: one (terminal) | `direction` (posición 1; obligatorio; `"in"`, `"out"`, `"unspecified"`) | el objeto de entrada; sustituye por clave (clave: terminal) | 15.2 |
| `@ASSERT-INSTANCE` | `members`: set (instance) | las condiciones de SELECT-INSTANCE salvo `role` y `recursive`, `has` (matriz de nombres de atributo), `same` (matriz de nombres de atributo), `different` (matriz de nombres de atributo), `message` | ningún objeto; una comprobación | 15.3 |
| `@ASSERT-TERMINAL` | `members`: set (terminal) | las condiciones de SELECT-TERMINAL salvo `role` y `recursive`, `same`, `different`, `message` | ningún objeto; una comprobación | 15.3 |
| `@ASSERT-NET` | `members`: set (net) | las condiciones de SELECT-NET salvo `role` y `recursive`, `same`, `different`, `message` | ningún objeto; una comprobación | 15.3 |
| `@ASSERT-USAGE` | `members`: set (usage); `definition`: definition (una definición general), opcional | `symbol`, `drawn` (booleano), `same`, `different` (matrices; únicamente `"definition"`), `message` | ningún objeto; una comprobación | 15.3 |
| `@ASSERT-IN` | `members`: set (cualquier objeto); `set`: any (set) | `message` | ningún objeto; una comprobación | 15.3 |
| `@ASSERT-TOPOLOGY` | `members`: set (instance, bundle, usage, set) | `topology` (posición 1; obligatorio; `"series"`, `"parallel"`, `"star"`, `"loop"`), `exclude` (matriz de pares: type, terminal), `message` | ningún objeto; una comprobación | 15.4 |

### A.12 Otras palabras estándar

- Clases: `instance`, `terminal`, `pin`, `net`, `point`, `wire`, `usage`, `bundle`, `text`, `annotation`, `label` (5.3, 9.4).
- Subobjetos tras `::`: `position`, `net` (6.5).
- Atributos leídos por `${…}`: `name`, `value`, `model`, `type`, `net`, `x`, `y` (13.6).
- Valores de verdad: `true`, `false` (3.7).

Propiedades de estilo (9.3):

| Propiedad | Tipo y valores | Por defecto |
|---|---|---|
| `linecolor` | color | — |
| `linestyle` | `"solid"`, `"dashed"`, `"dotted"`, `"dash-dot"`, `"double"`, `"bold"`, `"none"` | `"solid"` |
| `linewidth` | número | `0` |
| `cap` | `"butt"`, `"round"`, `"square"` | — |
| `join` | `"miter"`, `"round"`, `"bevel"` | — |
| `fillcolor` | color | — |
| `textcolor` | color | — |
| `size` | número | — |
| `font` | cadena | — |
| `bold` | booleano | — |
| `italic` | booleano | — |
| `arrowsize` | número | — |
| `textvisible` | booleano | `true` |

Propiedades de estilo que admiten las palabras clave como parámetros (9.2.5): *línea*: `linecolor`, `linestyle`, `linewidth`, `cap`, `join`, `fillcolor`, `arrowsize`; *área*: `linecolor`, `linestyle`, `linewidth`, `cap`, `join`, `fillcolor`; *cable*: `linecolor`, `linestyle`, `linewidth`, `cap`, `join`; *texto*: `textcolor`, `size`, `font`, `bold`, `italic`, `textvisible`.
