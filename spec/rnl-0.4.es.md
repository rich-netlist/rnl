<!-- translated from en/front.md sha256:0cc8c8aaa953 -->

# Estándar del lenguaje RNL, versión 0.4

RNL (Rich Netlist) se escribe en los comentarios de un netlist de circuito. Describe cómo se dibuja el circuito como un esquema y qué significan sus partes, mientras que el netlist por sí solo define el circuito. Este documento es el estándar de la versión 0.4 del lenguaje.

- El texto en inglés es normativo. Sus traducciones, nombradas `rnl-0.4.<lang>.md` según una etiqueta de idioma como `zh` o `ja`, son informativas; cuando una traducción difiera, prevalece el texto en inglés.
- La lista de palabras `rnl-words-0.4.json` y su esquema `rnl-words-0.4.schema.json` forman parte de este estándar (5.3, Anexo A).
- Los principios de RNL se encuentran en `rich-netlist.md` (en inglés, texto autorizado; traducciones: `rich-netlist.<lang>.md`); este estándar se refiere a ellos como "Principios".
- Las versiones 0.x no prometen compatibilidad entre sí (16.1).
- Este texto revisa la versión 0.3 conforme al borrador de la versión 0.4 del 2026-11-19; el Anexo D enumera los cambios.

## Contenido

- [1 Descripción general](#1-descripción-general)
  - [1.1 Alcance](#11-alcance)
  - [1.2 Estructura de este estándar](#12-estructura-de-este-estándar)
  - [1.3 Vistazo rápido](#13-vistazo-rápido)
  - [1.4 Términos y convenciones](#14-términos-y-convenciones)
  - [1.5 Principios fundamentales](#15-principios-fundamentales)
  - [1.6 Conformidad](#16-conformidad)
- [2 Incrustación en el anfitrión](#2-incrustación-en-el-anfitrión)
  - [2.1 Análisis en dos etapas](#21-análisis-en-dos-etapas)
  - [2.2 Portadores](#22-portadores)
  - [2.3 La entrada](#23-la-entrada)
  - [2.4 Alcance de un registro](#24-alcance-de-un-registro)
  - [2.5 Registros desactivados y texto del anfitrión](#25-registros-desactivados-y-texto-del-anfitrión)
  - [2.6 Niveles](#26-niveles)
  - [2.7 Archivos de paquete](#27-archivos-de-paquete)
  - [2.8 Encabezado del registro](#28-encabezado-del-registro)
  - [2.9 Archivos Verilog-A](#29-archivos-verilog-a)
- [3 Estructura léxica](#3-estructura-léxica)
  - [3.1 Caracteres](#31-caracteres)
  - [3.2 Tokens](#32-tokens)
  - [3.3 Texto libre](#33-texto-libre)
  - [3.4 Comentarios](#34-comentarios)
  - [3.5 Espacios en blanco y adyacencia](#35-espacios-en-blanco-y-adyacencia)
  - [3.6 Nombres](#36-nombres)
  - [3.7 Literales](#37-literales)
  - [3.8 Puntos](#38-puntos)
- [4 Sintaxis](#4-sintaxis)
  - [4.1 Gramática](#41-gramática)
  - [4.2 Restricciones fuera de la gramática](#42-restricciones-fuera-de-la-gramática)
  - [4.3 Orden de palabras](#43-orden-de-palabras)
  - [4.4 Ejemplos de errores](#44-ejemplos-de-errores)
  - [4.5 Sentencias estándar](#45-sentencias-estándar)
- [5 Objetos y sentencias](#5-objetos-y-sentencias)
  - [5.1 Objetos](#51-objetos)
  - [5.2 Conjuntos](#52-conjuntos)
  - [5.3 Sentencias y firmas](#53-sentencias-y-firmas)
  - [5.4 Modos de consumir la entrada](#54-modos-de-consumir-la-entrada)
  - [5.5 Tuberías y anidamiento](#55-tuberías-y-anidamiento)
  - [5.6 Definiciones como objetos](#56-definiciones-como-objetos)
  - [5.7 Manejadores y nombres](#57-manejadores-y-nombres)
  - [5.8 Claves de hechos](#58-claves-de-hechos)
  - [5.9 Números de objetos sin nombre](#59-números-de-objetos-sin-nombre)
- [6 Nombres y ámbito](#6-nombres-y-ámbito)
  - [6.1 Jerarquía de niveles](#61-jerarquía-de-niveles)
  - [6.2 Prefijos](#62-prefijos)
  - [6.3 Referencias al anfitrión](#63-referencias-al-anfitrión)
  - [6.4 Terminales](#64-terminales)
  - [6.5 Subobjetos](#65-subobjetos)
  - [6.6 Nombramiento](#66-nombramiento)
  - [6.7 Ámbito de los nombres](#67-ámbito-de-los-nombres)
  - [6.8 Herencia y ocultación](#68-herencia-y-ocultación)
  - [6.9 Nombres en cuerpos de definición](#69-nombres-en-cuerpos-de-definición)
  - [6.10 Marcadores](#610-marcadores)
  - [6.11 Subíndices](#611-subíndices)
  - [6.12 Nombres de terminal: DEVICE](#612-nombres-de-terminal-device)
- [7 Fallos y diagnósticos](#7-fallos-y-diagnósticos)
  - [7.1 Dos fases de comprobación](#71-dos-fases-de-comprobación)
  - [7.2 Objetos fallidos](#72-objetos-fallidos)
  - [7.3 Entradas estrictas y tolerantes](#73-entradas-estrictas-y-tolerantes)
  - [7.4 Recuperación](#74-recuperación)
  - [7.5 Los conflictos no son fallos](#75-los-conflictos-no-son-fallos)
  - [7.6 Diagnósticos](#76-diagnósticos)
- [8 Entrada en vigor](#8-entrada-en-vigor)
  - [8.1 Un nivel surte efecto después de leerse completamente](#81-un-nivel-surte-efecto-después-de-leerse-completamente)
  - [8.2 Fases](#82-fases)
  - [8.3 Colas de apariencia](#83-colas-de-apariencia)
  - [8.4 Sustitución por clave](#84-sustitución-por-clave)
  - [8.5 Conflictos](#85-conflictos)
- [9 Definiciones](#9-definiciones)
  - [9.1 Reglas comunes](#91-reglas-comunes)
  - [9.2 Símbolos: DEFINE-SYMBOL](#92-símbolos-define-symbol)
  - [9.3 Estilos: DEFINE-STYLE](#93-estilos-define-style)
  - [9.4 Definiciones generales: DEFINE](#94-definiciones-generales-define)
  - [9.5 Usos de las definiciones generales](#95-usos-de-las-definiciones-generales)
- [10 Selecciones, reglas y generación uno por uno](#10-selecciones-reglas-y-generación-uno-por-uno)
  - [10.1 Palabras clave de selección](#101-palabras-clave-de-selección)
  - [10.2 La condición `wired`](#102-la-condición-wired)
  - [10.3 Evaluación de selecciones](#103-evaluación-de-selecciones)
  - [10.4 Reglas](#104-reglas)
  - [10.5 Generación uno por uno](#105-generación-uno-por-uno)
- [11 Colocación y apariencia](#11-colocación-y-apariencia)
  - [11.1 Posición: PLACE](#111-posición-place)
  - [11.2 Orientación: ORIENT](#112-orientación-orient)
  - [11.3 Símbolos y estilos: SET-SYMBOL y SET-STYLE](#113-símbolos-y-estilos-set-symbol-y-set-style)
  - [11.4 Varios dibujos de un mismo objeto](#114-varios-dibujos-de-un-mismo-objeto)
  - [11.5 Grupos: GROUP](#115-grupos-group)
  - [11.6 Orden de dibujo y capas: LAYER](#116-orden-de-dibujo-y-capas-layer)
  - [11.7 Coordenadas y unidades](#117-coordenadas-y-unidades)
  - [11.8 Agrupaciones: BUNDLE](#118-agrupaciones-bundle)
- [12 Componentes virtuales](#12-componentes-virtuales)
  - [12.1 Concepto](#121-concepto)
  - [12.2 Creación de componentes virtuales](#122-creación-de-componentes-virtuales)
  - [12.3 Posiciones relativas a un terminal](#123-posiciones-relativas-a-un-terminal)
  - [12.4 Puertos](#124-puertos)
  - [12.5 Componentes virtuales estándar](#125-componentes-virtuales-estándar)
  - [12.6 Marcas de no conexión](#126-marcas-de-no-conexión)
- [13 Cables y texto](#13-cables-y-texto)
  - [13.1 Cables: WIRE](#131-cables-wire)
  - [13.2 Puntos de unión](#132-puntos-de-unión)
  - [13.3 Texto: TEXT](#133-texto-text)
  - [13.4 Anotaciones: gráficos en la escena](#134-anotaciones-gráficos-en-la-escena)
  - [13.5 Símbolos dibujados como anotaciones: ANNOTATION](#135-símbolos-dibujados-como-anotaciones-annotation)
  - [13.6 Marcado de texto y referencias a atributos](#136-marcado-de-texto-y-referencias-a-atributos)
  - [13.7 Dibujo de conexiones como cables: ROUTE-NET, ROUTE-OBJECT, UNROUTE-NET, UNROUTE-OBJECT](#137-dibujo-de-conexiones-como-cables-route-net-route-object-unroute-net-unroute-object)
- [14 Intención de disposición](#14-intención-de-disposición)
  - [14.1 Reglas comunes](#141-reglas-comunes)
  - [14.2 ALIGN](#142-align)
  - [14.3 ORDER](#143-order)
  - [14.4 DISTRIBUTE](#144-distribute)
  - [14.5 DISTANCE](#145-distance)
  - [14.6 Flujo de señal](#146-flujo-de-señal)
  - [14.7 Simetría](#147-simetría)
  - [14.8 Prioridad: PRIORITY](#148-prioridad-priority)
  - [14.9 Relación de orientación: ORIENT-RELATION](#149-relación-de-orientación-orient-relation)
  - [14.10 Ajustes del solucionador: SOLVER-CONFIG](#1410-ajustes-del-solucionador-solver-config)
  - [14.11 Tablas: TABLE](#1411-tablas-table)
- [15 Semántica eléctrica](#15-semántica-eléctrica)
  - [15.1 Aplicación de definiciones semánticas](#151-aplicación-de-definiciones-semánticas)
  - [15.2 Dirección de corriente: CURRENT](#152-dirección-de-corriente-current)
  - [15.3 Aserciones: ASSERT-INSTANCE, ASSERT-TERMINAL, ASSERT-NET, ASSERT-USAGE, ASSERT-IN](#153-aserciones-assert-instance-assert-terminal-assert-net-assert-usage-assert-in)
  - [15.4 Aserciones de topología: ASSERT-TOPOLOGY](#154-aserciones-de-topología-assert-topology)
- [16 Archivos, versiones, paquetes y alias](#16-archivos-versiones-paquetes-y-alias)
  - [16.1 Versiones](#161-versiones)
  - [16.2 Paquetes y USE-PACKAGE](#162-paquetes-y-use-package)
  - [16.3 El paquete estándar](#163-el-paquete-estándar)
  - [16.4 Extensiones y contenido desconocido](#164-extensiones-y-contenido-desconocido)
  - [16.5 Alias](#165-alias)
- [17 Modelo de modificación](#17-modelo-de-modificación)
- [Anexo A Firmas de las palabras clave (normativo)](#anexo-a-firmas-de-las-palabras-clave-normativo)
  - [A.1 El registro](#a1-el-registro)
  - [A.2 Registros de encabezado y paquetes](#a2-registros-de-encabezado-y-paquetes)
  - [A.3 Definiciones](#a3-definiciones)
  - [A.4 Contenido de símbolos y gráficos](#a4-contenido-de-símbolos-y-gráficos)
  - [A.5 Colocación y apariencia](#a5-colocación-y-apariencia)
  - [A.6 Selecciones](#a6-selecciones)
  - [A.7 Aplicación de definiciones](#a7-aplicación-de-definiciones)
  - [A.8 Cables](#a8-cables)
  - [A.9 Conjuntos, grupos y agrupaciones](#a9-conjuntos-grupos-y-agrupaciones)
  - [A.10 Restricciones y prioridad](#a10-restricciones-y-prioridad)
  - [A.11 Semántica eléctrica](#a11-semántica-eléctrica)
  - [A.12 Otras palabras estándar](#a12-otras-palabras-estándar)
- [Anexo B Sentencias estándar (informativo)](#anexo-b-sentencias-estándar-informativo)
  - [B.1 Colocación y orientación](#b1-colocación-y-orientación)
  - [B.2 Restricciones de disposición](#b2-restricciones-de-disposición)
  - [B.3 Apariencia](#b3-apariencia)
  - [B.4 Símbolos y gráficos](#b4-símbolos-y-gráficos)
  - [B.5 Cables y texto](#b5-cables-y-texto)
  - [B.6 Definiciones y aplicaciones](#b6-definiciones-y-aplicaciones)
  - [B.7 Selecciones](#b7-selecciones)
  - [B.8 Conjuntos y otras palabras clave](#b8-conjuntos-y-otras-palabras-clave)
  - [B.9 Registro de cabecera y paquetes](#b9-registro-de-cabecera-y-paquetes)
- [Anexo C Reservado para versiones posteriores (informativo)](#anexo-c-reservado-para-versiones-posteriores-informativo)
- [Anexo D Cambios respecto a la versión 0.3 (informativo)](#anexo-d-cambios-respecto-a-la-versión-03-informativo)

<!-- translated from en/ch01.md sha256:e38c6540c72f -->

## 1 Descripción general

### 1.1 Alcance

RNL (Rich Netlist) se escribe en los comentarios de un netlist anfitrión. Describe cómo se entiende y se presenta como un esquema el circuito que define el netlist: qué símbolos se utilizan, dónde se colocan los componentes, cómo se orientan, se alinean, se cablean, se etiquetan y se estilizan, y qué significan los grupos de componentes (Principios 1).

Los dispositivos, tipos, parámetros, conexiones e interfaces de subcircuitos los define únicamente el netlist anfitrión. RNL hace referencia a ellos y no puede modificarlos. RNL no comprueba el significado eléctrico que sugiere un dibujo (Principios 2.2).

Este estándar define:

- cómo se toma RNL de los comentarios de un netlist anfitrión y de los archivos Verilog-A que este incorpora (capítulo 2);
- la estructura léxica y la sintaxis del texto RNL (capítulos 3 y 4);
- qué objetos describe el texto, cómo se resuelven los nombres, cómo se tratan los fallos y cómo surten efecto en conjunto las descripciones (capítulos 5–8);
- el vocabulario de cada tipo de descripción (capítulos 9–17);
- la lista estándar de palabras, publicada junto con este texto como `rnl-words-0.4.json` (5.3, Anexo A).

Este estándar no define un algoritmo de disposición ni una representación única (Principios 1), ni define cómo se analiza un netlist anfitrión (Principios 2.1). El paquete estándar `rnl-std-all` y los paquetes de alias como `alias-zh` están escritos en RNL; este estándar define cómo funcionan los paquetes (16.2, 16.3, 16.5), y su contenido se publica por separado.

Este texto define la versión 0.4 del lenguaje. Las versiones 0.x no prometen compatibilidad entre sí (16.1).

### 1.2 Estructura de este estándar

El estándar consta de un modelo abstracto y una sintaxis textual. El modelo abstracto comprende objetos, hechos, modos de consumir la entrada y fases de entrada en vigor (capítulos 5–8); la sintaxis textual es su forma escrita (capítulos 2–4). Las herramientas intercambian el texto.

El Anexo A enumera las firmas de las palabras clave; se genera a partir de la lista de palabras y es normativo. El Anexo B ofrece las sentencias estándar, el Anexo C enumera lo reservado para versiones posteriores y el Anexo D recoge los cambios respecto a la versión 0.3; los tres son informativos. Las notas y los ejemplos son informativos. Todo lo demás es normativo.

### 1.3 Vistazo rápido

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

- `* ` inicia un comentario de SPICE. `@RNL {…}` es un registro; dentro de las llaves hay miembros separados por `;` (capítulo 2). El registro de encabezado al comienzo del archivo declara la versión del lenguaje (16.1).
- `#M1`, `&vss` y `#M5:3` hacen referencia a una instancia, una red y un terminal del anfitrión. `%dp1` y `%g0` son nombres RNL de este nivel. `$DIFFERENTIAL_PAIR_N` y `$GROUND` son definiciones generales (9.4) del paquete estándar, que utiliza el registro de encabezado (16.3). `%g0` es el uso creado al aplicar `$GROUND`; el paquete estándar le asigna un símbolo de tierra, por lo que se dibuja como una tierra (capítulo 12).
- `@AS`, `@ALIGN`, `@PLACE`, `@ORIENT`, `@ATTACH` y `@WIRE` son palabras clave. Los parámetros se escriben entre paréntesis unidos a la palabra clave y solo admiten literales. AS rellena los roles de la definición escrita después con los objetos escritos antes; ATTACH añade la definición escrita antes a los objetos escritos después (9.4, capítulo 12).
- `(left)` marca un rol. `%dp1 =` da nombre al resultado de una sentencia. `=>` pasa el resultado de un segmento al siguiente; seguido únicamente de un nombre, da nombre al resultado del segmento anterior (5.5).
- `and`, `treated`, `, i.e., same y`, `is at`, `it`, `is … to`, `net`, `, through`, `, has a` son texto libre y no alteran el significado (3.3). Las sentencias estándar añaden esas palabras entre los tokens para que las sentencias se lean como frases (4.5, Anexo B). Sin ellas, el registro se lee así:

```rnl
%dp1 = #M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N;
{#M1 #M2} @ALIGN("y");
#M5 @PLACE(20, 6) => @ORIENT("R0");
%g0 = $GROUND @ATTACH &vss;
&vss {#M5:3 %g0::pin} @WIRE
```

### 1.4 Términos y convenciones

#### 1.4.1 Palabras de requisito

- **debe**, **no debe**: un requisito para los documentos o las implementaciones.
- **debería**, **no debería**: una recomendación; puede haber razones para proceder de otro modo, y conviene comprenderlas primero.
- **puede**: algo permitido.

El significado del lenguaje en sí se expresa en presente: "la sentencia falla", "la posterior prevalece".

#### 1.4.2 Fallos y diagnósticos

- Un elemento (un campo, sentencia, registro o archivo; 7.6) que **falla** no tiene efecto; una sentencia que falla tiene como valor un objeto fallido (7.2). Todo fallo es un **error**, sea cual sea su causa (7.6).
- Una **advertencia** marca un elemento que es válido, pero puede no ser lo que el autor pretendía; surte efecto como de costumbre, salvo donde este estándar indique lo contrario, como en el caso de un hecho que el solucionador no pudo decidir dentro de su límite de tiempo (8.5). Una **violación** informa de que una afirmación que hace el archivo sobre el circuito, como una aserción, no se cumple; nada falla (15.3, 15.4). Una **indicación** marca una sustitución, ocultación o resolución habitual que también puede ser una equivocación. Los casos se enumeran en 7.6; donde este estándar indique que algo produce una advertencia o una indicación, las implementaciones deberían informarlo.
- Cuando **la interpretación falla**, no se dibuja nada y se informa de un error (1.5 punto 3, 7.6).

#### 1.4.3 Notación

- En el texto, las palabras clave se escriben en mayúsculas sin `@`: PLACE, SET-SYMBOL. En la sintaxis y los ejemplos se escriben con `@`.
- Las palabras estándar, los nombres y los literales se escriben en fuente de código: `priority`, `"high"`, `instance`, `$GROUND`.
- Los números de sección entre paréntesis remiten a este estándar: (9.4). "Principios 2.4" remite a la sección 2.4 de los principios de RNL, `rich-netlist.md`.
- Los ejemplos están escritos en la forma estándar (4.3) y sin texto libre, salvo cuando el texto libre es el tema. Un ejemplo marcado como error muestra una equivocación.
- Los ejemplos utilizan el paquete estándar: uno que no muestra ningún registro de encabezado se lee como si su archivo principal hubiera escrito `@USE-PACKAGE(name="rnl-std-all", version="0.4")` en su registro de encabezado (16.3).

#### 1.4.4 Términos

| Término | Inglés | Significado |
|---|---|---|
| alias | alias | Otra grafía de una palabra estándar, o de un nombre `$` y de los nombres de rol, parámetro y locales de una definición, declarada con ALIAS o cargada con USE-ALIASES (16.5) |
| paquete de alias | alias package | Un archivo de paquete que solo declara alias; se carga con USE-ALIASES (2.7, 16.5) |
| anotación | annotation | Un gráfico o un texto escrito en la escena en lugar de adjuntado a un objeto (13.4) |
| aplicar, aplicación | apply, application | Lo que hacen AS y ATTACH: rellenar los roles de una definición con objetos. Cada aplicación crea un uso (9.4) |
| adjuntar | attach | Un TEXT se adjunta al punto u objeto que recibe: puede leer los atributos del objeto y se coloca en el objeto o junto a él salvo que se indique una posición (13.3). No debe confundirse con la palabra clave ATTACH, que aplica una definición general (9.4) |
| cuerpo | body | Las sentencias entre las llaves de una definición o de un paquete en línea (9.1, 16.2) |
| agrupación | bundle | Un objeto creado con BUNDLE: varias instancias del anfitrión dibujadas como una sola, en su lugar. Sus terminales representan los terminales del mismo nombre de todos sus miembros (11.8) |
| bus | bus | Un cable cuya ranura de red recibe un conjunto de redes, incluso un conjunto de una sola; el netlist no tiene buses, un bus es solo una forma de dibujar (13.1) |
| portador | carrier | Un comentario de línea completa del anfitrión que contiene RNL: el carácter de comentario (`*`, o `//` en el modo de lenguaje Spectre) en la columna 1, seguido de nada, solo de espacios en blanco o de un espacio y el contenido (2.2) |
| consignar | commit | Lo que hace un registro con sus sentencias: cada sentencia de un registro pasa a formar parte de su nivel, tenga nombre o no (5.4) |
| restricción | constraint | Una condición sobre posiciones u orientaciones para el solucionador: ALIGN, ORDER, DISTRIBUTE, DISTANCE, ORIENT-RELATION, TABLE (capítulo 14) |
| definición | definition | Lo que crean DEFINE, DEFINE-SYMBOL o DEFINE-STYLE, normalmente nombrado con `$` (capítulo 9). Lo que crea DEFINE es una **definición general**; los símbolos (DEFINE-SYMBOL) y estilos (DEFINE-STYLE) son **definiciones específicas**. Este texto dice "definición" para referirse a una definición general siempre que los roles, parámetros, aplicaciones o usos aclaren la clase, y "definición general" cuando debe distinguirse de los símbolos y estilos |
| hecho | fact | Una afirmación que porta una sentencia, como "M1 está en (10, 20)" (5.1) |
| objeto fallido | failed object | El valor de una sentencia que falló (7.2) |
| relleno | fill | Las tierras, alimentaciones y etiquetas de red que las reglas del paquete estándar añaden para los terminales y redes que no se dibujan de otro modo (10.5, 16.3) |
| texto libre | free text | Texto entre los tokens de una sentencia; no altera el significado (3.3) |
| conjunto generado | generated set | El conjunto que crea una regla de generación; no se puede hacer referencia a sus miembros individuales (10.5) |
| grupo | group | Un conjunto marcado con GROUP: no es un objeto; el solucionador dispone sus miembros juntos y mantiene a otros fuera de su región, como criterios posteriores a todo hecho (11.5) |
| manejador | handle | Aquello sobre lo que trata un hecho: una instancia, red o terminal del anfitrión, o un punto, cable, conjunto u otro objeto creado por RNL. Los hechos sobre un mismo manejador surten efecto juntos (5.1, 5.7) |
| registro de encabezado | header record | Un registro con `type="header"` al comienzo de un archivo: uno sin `language`, y como máximo uno por cada lenguaje (2.8) |
| ocultación | hiding | Una definición de un nivel inferior, o de un paquete posterior, utilizada en lugar de otra con el mismo nombre procedente de arriba (6.8, 16.2) |
| anfitrión, netlist anfitrión | host, host netlist | El netlist que contiene RNL. El adaptador del anfitrión lo lee y proporciona sus comentarios y objetos a RNL (2.1) |
| estilo de la casa | house style | La forma general de dibujar establecida por un paquete o una implementación: qué símbolos, estilos y definiciones de disposición se usan, si las conexiones sin cables se muestran con etiquetas o líneas de conexión, etc. Un paquete que fija uno es un paquete de estilo de la casa. No es un estilo en el sentido de DEFINE-STYLE |
| elemento | item | Aquello a lo que se aplica un fallo: un campo de un encabezado del registro, una sentencia, un registro, un archivo o toda la interpretación (7.6) |
| instancia | instance | Un elemento del netlist anfitrión, incluidas las instancias de subcircuito (líneas X). Una instancia iterada como `XI<3:0>` representa varias instancias (6.11) |
| clave | key | El nombre de un texto en el objeto al que se adjunta: un TEXT con `key="wl"` en M1 es `#M1::wl` (13.3) |
| palabra clave | keyword | `@` seguido de una palabra en mayúsculas, como `@PLACE`, o un alias declarado (16.5); como máximo una por segmento (4.1, 5.3) |
| etiqueta | label | Un TEXT adjuntado a un objeto; puede hacer referencia a los atributos del objeto, como su nombre o su valor (13.3) |
| nivel | level | Un ámbito del texto: el nivel de archivo del archivo principal (el nivel superior del circuito), el nivel de archivo de un archivo incluido o una definición de subcircuito; cada paquete es también un nivel. Los niveles forman un árbol por contención textual (2.6, 6.1) |
| objeto local | local object | Un objeto nombrado en el cuerpo de una definición general; cada uso tiene el suyo propio, al que se hace referencia desde fuera como `%c1::mid` (9.4) |
| objeto principal | main object | El objeto que rellena el primer rol de un uso; los textos hacen referencia a él a través del rol (13.6). Un uso se dibuja en su propio punto, no en su objeto principal (12.1) |
| miembro, segmento, sentencia | member, segment, statement | Un miembro es una parte de un conjunto separada por `;`. Un segmento es una parte de un miembro separada por `=>` (un nombre solo después de `=>` no es un segmento, 4.2). Una sentencia es un segmento con una palabra clave, o un objeto en un segmento sin ella (4.2, 5.3) |
| nombre | name | `%x =`, `$X =`, `=> %x` o `=> $X` da nombre a un objeto; un nombre no crea ningún objeto y se vincula una sola vez en un nivel (6.6) |
| punto propio | own point | El punto de un uso: el origen de su símbolo, o un punto libre que el cuerpo de su definición liga a sus miembros; escrito `%u::position`. Una restricción lo toma para un uso con símbolo, y PLACE para todo uso (14.1, 6.5) |
| paquete | package | Un conjunto de definiciones y reglas situado por encima de un nivel mediante USE-PACKAGE; escrito como un archivo de paquete (2.7) o en línea en el cuerpo de USE-PACKAGE (16.2). Un paquete que solo referencia otros paquetes es una **colección de paquetes (package collection)**, como `rnl-std-all` (16.3) |
| paquetes en la raíz | packages at the root | Paquetes situados antes de todos los niveles: los paquetes de USE-PACKAGE en registros de encabezado, figurando el paquete estándar primero entre ellos cuando se escribe o se añade mediante un ajuste de la herramienta (16.3) |
| pin | pin | Un objeto de un símbolo: un punto donde se conectan cables, que vincula terminales del objeto dibujado con el símbolo, o roles de un uso, o nada (9.2.4) |
| prioridad | priority | El grado de un hecho geométrico: `"disabled"`, `"low"`, `"normal"` o `"high"`. En un conflicto, se compara primero la prioridad y después el lugar donde está escrito el hecho (8.5, 14.8) |
| registro | record | Un `@RNL(…) {…}` en los comentarios. Los campos entre sus paréntesis forman el encabezado del registro y se aplican a cada sentencia del registro (2.3, 2.8) |
| objeto RNL | RNL object | Lo que crea una sentencia con una palabra clave o un `{…}`: un manejador y hechos (5.1) |
| rol | role | La ranura que rellena un objeto en una sentencia, marcada entre paréntesis justo después del objeto, como en `#M1(in_p)`. Los roles de una definición se nombran en su lista de roles y se escriben `?name` en su cuerpo; los roles de una palabra clave integrada son sus nombres de ranura (5.3, 9.4) |
| regla | rule | Una sentencia que toma el resultado de una selección. Una regla escrita en un nivel se evalúa en ese nivel y, con `recursive=true`, una vez más en cada nivel inferior, cada vez únicamente con los objetos de ese nivel (10.4) |
| escena | scene | Los objetos de un nivel, procedentes del anfitrión y de RNL, con los hechos que han surtido efecto (capítulo 8) |
| selección | selection | El resultado de una palabra clave SELECT-…: los objetos de una clase en su ámbito que cumplen sus condiciones, evaluados cuando se utilizan (capítulo 10) |
| conjunto | set | Una colección ordenada de objetos, escrita `{…}` u obtenida mediante una selección (5.2) |
| ranura | slot | Uno de los lugares donde una palabra clave toma objetos, como `from` y `to` en DISTANCE. Cada ranura tiene un nombre, que puede escribirse como rol (5.2.1, 5.3) |
| solucionador | solver | La herramienta que calcula posiciones y orientaciones a partir del RNL y las escribe en un registro generado (14.8, capítulo 17) |
| forma estándar | standard form | La única manera de escribir cada palabra clave que producen las herramientas: dónde se sitúa la palabra clave, qué parámetros se dan por posición y si se marcan los roles (4.3, 5.3) |
| paquete estándar | standard package | La colección de paquetes `rnl-std-all` con definiciones, símbolos, estilos y reglas, que un archivo utiliza escribiéndola en su registro de encabezado o que una herramienta añade mediante un ajuste (16.3) |
| sentencia estándar | standard sentence | Una sentencia en la forma estándar con texto libre añadido entre los tokens para que se lea como una frase (4.5, Anexo B) |
| palabra estándar | standard word | Una palabra definida por la lista de palabras: palabras clave, nombres de ranura, nombres y valores de parámetro, campos del encabezado del registro, clases, nombres de subobjeto, nombres de atributo, propiedades de estilo, `true` y `false` (5.3) |
| estilo | style | Únicamente las propiedades definidas con DEFINE-STYLE: colores, estilo de línea, grosor de línea, tamaño de texto, etc. (9.3). Las preferencias de disposición no son estilos |
| subíndice | subscript | `<…>` o `[…]` justo después de un nombre del anfitrión o un nombre de terminal, como `&D<3>` o `&D<7:0>`, que representa uno o varios nombres (6.11); no los subíndices del marcado de texto (13.6) |
| subobjeto | sub-object | Una parte de un objeto a la que se accede con `::`, como `#M1::position` o `%dp::left` (6.5) |
| símbolo | symbol | Lo que crea DEFINE-SYMBOL: gráficos, pines y textos, dibujados para una instancia, agrupación o uso (9.2) |
| surtir efecto | take effect | Después de haber leído completamente un nivel, sus hechos pasan a formar la escena, fase por fase (capítulo 8) |
| token | token | Una palabra clave, referencia, punto, paréntesis, llave, `;`, `=>` o `=`; todo lo demás es texto libre o comentarios (3.2) |
| terminal | terminal | Una conexión de una instancia, tal como figura en su línea del netlist: `#M1:d`, o `#M1:1` por número. `#:inp` es el puerto `inp` del nivel (6.4) |
| uso | usage | El objeto que crea una aplicación. Registra la definición y qué objetos rellenan sus roles; cuando tiene símbolo, se dibuja (9.4, 9.5). En una restricción, un uso sin símbolo representa a sus miembros (14.1) |
| componente virtual | virtual part | Un uso que tiene un símbolo: se dibuja, pero no forma parte del netlist, como una tierra, una alimentación, una etiqueta de puerto o un punto de unión (capítulo 12) |
| lista de palabras | word list | La lista legible por máquina de todas las palabras estándar y firmas de palabras clave de una versión del lenguaje, `rnl-words-0.4.json` para esta versión (5.3, Anexo A) |

### 1.5 Principios fundamentales

Las reglas de este estándar siguen estos principios. Los puntos 1–6 refinan Principios 2.9; el punto 7 explica cómo se divide el vocabulario.

1. **Todo es una descripción.** El texto RNL es un conjunto de descripciones. El intérprete las recopila todas antes de ofrecer un resultado. Nada se ejecuta, y un nombre puede usarse antes de la sentencia que lo vincula.
2. **El lugar de escritura es solo dato de ordenación.** Dónde está escrita una descripción (su nivel y su lugar en el texto) importa únicamente en tres casos: cuando dos descripciones se contradicen entre sí, se conserva la posterior (los hechos geométricos comparan primero la prioridad; 8.3, 8.4, 8.5); el orden de dibujo (11.6); y el orden de los miembros de un conjunto (5.2). Por lo demás, el resultado no depende del orden de escritura. Cada segmento de una tubería tiene su propio lugar: un segmento posterior se considera escrito después de uno anterior (5.5).
3. **Contradicciones.**
   - Una descripción que no puede entenderse (un error de sintaxis, un nombre no encontrado, una clase incorrecta, etc.), o que se contradice a sí misma, falla (Principios 2.4, 2.9; capítulo 7).
   - Las descripciones que pueden entenderse pero se contradicen entre sí se resuelven donde este estándar establece una vía definida, como la sustitución por clave (8.4), las colas de apariencia (8.3) y la resolución de hechos geométricos (8.5); puede emitirse una indicación.
   - Donde aún no hay una vía definida, toda la interpretación falla (7.6). Esta versión no tiene ningún caso así (7.5). Una versión posterior puede definir una vía y hacerlos tolerantes; los archivos que funcionan ahora no se ven afectados.
4. **Cada descripción ve el estado previo a su entrada en vigor.** Una descripción lee el estado tal como estaba cuando comenzó su fase; su propio efecto y los efectos de otras descripciones de la misma fase no están incluidos (8.2).
5. **Objetos y nombres.** Toda sentencia con una palabra clave y todo `{…}` crea un objeto nuevo, compuesto por un manejador y hechos. Una referencia no crea ningún objeto, salvo que tomar miembros por sus lugares crea un conjunto nuevo (5.2.3). Un nombre solo hace referencia a un objeto y se vincula una sola vez en un nivel. La sustitución se aplica a los hechos, no a los nombres (5.1, 6.6).
6. **Sin límites por legibilidad.** Una forma difícil de leer pero con un significado definido está permitida; queda en manos de los autores moderarse. Solo las formas sin significado definido son errores.
7. **Palabras clave y definiciones.** Lo que una implementación debe comprender por sí misma, y lo que no puede escribirse con otras descripciones, es una palabra clave. Lo que puede escribirse con descripciones existentes en el cuerpo de una definición general es una definición, y su nombre es solo una pista (9.5).

### 1.6 Conformidad

**Documentos.** Un netlist anfitrión es conforme con RNL 0.4 cuando sus registros, leídos según definen los capítulos 2–4 e interpretados según definen los capítulos 5–17, no producen errores (7.6), incluidos los errores en el contenido de los paquetes que utiliza. Las advertencias, indicaciones y violaciones no afectan a la conformidad: una violación indica que el circuito no respalda lo que afirma el archivo, no que el archivo esté mal escrito. Un archivo de paquete es conforme del mismo modo cuando se comprueba por separado (16.2). Un archivo cuyo registro de encabezado declara otra versión del lenguaje no se rige por esta versión (16.1).

**Implementaciones** son los intérpretes (un intérprete utilizado solo para comprobación también se denomina lint), visores, solucionadores, editores y otras herramientas que leen o escriben RNL. Una implementación conforme:

1. toma RNL de los comentarios del anfitrión según define el capítulo 2, y lo lee según definen los capítulos 3 y 4;
2. da a cada descripción el efecto que definen los capítulos 5–17, con las firmas de la lista de palabras (5.3);
3. informa de todo error, con su ubicación y causa (7.6); los errores no pueden rebajarse ni desactivarse;
4. debería informar de las advertencias e indicaciones que enumera este estándar, y debe informar de las violaciones (7.6);
5. puede añadir comprobaciones propias solo como advertencias o indicaciones: no debe hacer que falle un elemento que este estándar deja surtir efecto, ni dejar que surta efecto un elemento que este estándar hace fallar;
6. puede ofrecer un modo más estricto que se detenga, o que rechace toda la entrada, pero no por defecto;
7. permite al usuario ver los diagnósticos; cómo se presentan queda a criterio de la implementación (7.6);
8. declara las versiones del lenguaje que admite y trata un archivo de una versión que no admite según define 16.1;
9. trata las palabras clave, parámetros y valores desconocidos como errores (16.4);
10. sitúa el paquete estándar en la raíz cuando un archivo lo escribe, según define 16.3;
11. cuando escribe RNL, escribe las sentencias nuevas en la forma estándar y no reordena las sentencias que escribieron los usuarios (4.3); escribe el registro de encabezado y la versión al guardar un archivo (16.1); conserva tal cual las sentencias desconocidas y fallidas, el texto del anfitrión, los comentarios ordinarios y los archivos de versiones no admitidas (16.4); y modifica únicamente los registros que el capítulo 17 le permite modificar.
12. cuando resuelve, resuelve los conflictos según define 8.5, escribe de vuelta cada decisión que toma y pasa sus resultados a otras herramientas solo en una forma que pueda contener un registro del solucionador (14.8);
13. puede establecer límites sobre los recursos que utiliza, como la profundidad de anidamiento de símbolos (9.2.7), de expansiones de usos (9.4), de archivos incluidos y de subcircuitos, o el número de objetos. Una entrada que supere tal límite produce un error que indica que se alcanzó un límite de la implementación, con su ubicación; la implementación no debe recortar la entrada en silencio, ni dibujar o resolver un resultado distinto del que define este estándar. Admite al menos 64 niveles de anidamiento de símbolos. El límite para comprobar hechos durante una resolución no es un límite de este tipo: es una opción del solucionador, y un hecho que deje sin decidir se descarta con una advertencia (8.5).

Este estándar no define cómo elige un solucionador entre posiciones que cumplen las restricciones, ni cómo dibuja un visor lo que RNL deja abierto; los lugares donde queda a su criterio se indican en el texto (por ejemplo 13.1).

**Las pruebas de conformidad** comprueban los errores, con sus ubicaciones y el alcance de cada fallo, y la escena resultante. Las advertencias pueden comprobarse como expectativas opcionales; las indicaciones no se comprueban (7.6).

<!-- translated from en/ch02.md sha256:e9821c4aa800 -->

## 2 Incrustación en el anfitrión

### 2.1 Análisis en dos etapas

- El adaptador del anfitrión:
  - reconoce los comentarios ordinarios válidos del anfitrión y decide qué líneas son portadores (2.2);
  - determina a qué nivel pertenece cada comentario y de qué archivo y de qué inclusión proceden cada registro y cada subcircuito (6.1);
  - proporciona las posiciones en el código fuente y el orden del texto;
  - suministra los objetos del anfitrión: instancias, redes, terminales, puertos de subcircuito y tipos de modelo.
- El núcleo de RNL busca registros únicamente en estos comentarios. No debe buscar `@RNL` por todo el netlist sin atender al contexto: los mismos caracteres en sentencias del anfitrión o en cadenas del anfitrión no son RNL.
- El intérprete analiza el netlist anfitrión completo antes de procesar el RNL (Principios 2.9).

### 2.2 Portadores

Los registros se escriben en comentarios de línea completa del anfitrión; tales líneas de comentario son portadores. El adaptador del anfitrión decide línea por línea qué líneas son portadores, elimina el carácter de comentario y el único espacio posterior, y pasa el resto al núcleo de RNL (2.1).

- **Solo comentarios de línea completa.** Los comentarios al final de línea (`$`, `;`, `//`, `!`, etc.) ni se leen ni se escriben, y no se utilizan comentarios de bloque.
  - `@RNL` en un comentario al final de línea es solo texto del anfitrión y produce una advertencia.
  - Nota: que el final de una línea sea un comentario depende del dialecto, del modo y de las opciones: en HSPICE `$` puede seguir directamente a un número (`1k$x`), en el modo `ps` de ngspice `$` es un carácter ordinario y Eldo puede cambiarlo con `.COMCHAR`; en CDL, `$SUB=` y similares son datos. Escribir al final de una línea también alteraría la línea de una sentencia del anfitrión (Principios 2.1). SPICE y Spectre carecen de comentarios de bloque (Spectre solo dentro de `reliability{}`).
- **Caracteres de comentario de los anfitriones.**

  | Anfitrión | Carácter de comentario |
  |---|---|
  | La familia SPICE: SPICE3, ngspice, HSPICE, Xyce, LTspice, PSpice, WRspice, Eldo y netlists CDL | `*` |
  | Secciones en el modo de lenguaje Spectre: archivos `.scs`, o texto posterior a `simulator lang=spectre` | `//` o `*` |
  | Secciones en el modo de lenguaje SPICE de Spectre, incluidos archivos incluidos sin sufijo `.scs` que no comienzan con `simulator lang=spectre` | `*` |

  - `*` es un comentario en ambos modos de lenguaje de Spectre. `//` está documentado únicamente para el modo de lenguaje Spectre, y no en Eldo ni en CDL.
  - Las líneas de un mismo registro pueden mezclar `//` y `*`. Una herramienta que escribe un registro debe seguir el estilo de los registros ya presentes en el archivo; cuando no haya ninguno, escribe `*` en el modo SPICE y `//` en el modo Spectre.
  - Los portadores pertenecen a la forma externa, que no cambia (16.1). El soporte para un anfitrión nuevo solo añade portadores; los existentes no se modifican.
- **Líneas portadoras.** Cada línea de un registro debe cumplir ambas condiciones:
  - El carácter de comentario está en la columna 1, sin espacios en blanco antes.
  - Después del carácter de comentario no hay nada, o solo espacios en blanco, lo que forma una línea de comentario vacía; o un espacio ASCII seguido del contenido. Por ello, `*@RNL`, `*#M1` y `//@RNL` no son líneas portadoras.

    Nota: HSPICE y PSpice exigen `*` en la columna 1, y las herramientas tratan de forma distinta las líneas sangradas. El texto pegado al carácter de comentario significa algo para otras herramientas: ngspice y WRspice ejecutan las líneas `*#` como órdenes, y WRspice también las líneas `*@`; `*.` inicia una directiva CDL (como `*.PININFO`), `*|` inicia datos DSPF y `//pragma` es el envoltorio de cifrado de Spectre.
  - Una línea de comentario que no cumpla estas condiciones (como `*#M1 …`, `** …` o un `* …` sangrado) corta un registro que aún no está completo: todo el registro falla (2.4). Fuera de un registro, `@RNL` en una línea así no es una entrada y produce una advertencia.
  - Una línea que contenga RNL no debería terminar en `\`. Tal línea produce una advertencia, y las herramientas no deben escribir ninguna. El RNL se lee como de costumbre y nada falla; lo que corre peligro es el netlist anfitrión. Nota: WRspice une la línea siguiente a un comentario así, y lo mismo pueden hacer Spectre ejecutado con `-E` (a través del preprocesador de C) y Eldo con `SLASHCONT`; una sentencia del anfitrión en la línea siguiente quedaría absorbida dentro del comentario.
- **Lugares donde no se lee RNL.** `@RNL` en estos lugares produce una advertencia, y las herramientas no deben escribir allí:
  - La línea 1 de cada archivo anfitrión y otras líneas que el anfitrión toma como títulos (como la línea posterior a `.newjob` en WRspice). La línea 1 del archivo principal es su título, pero no es un título cuando el archivo se incluye; nunca se lee, de modo que un archivo se lea igual como archivo principal y como archivo incluido. Así, el registro de encabezado puede escribirse como pronto en la línea 2 (2.8).
  - Texto posterior al `.end` del archivo principal. Nota: después de `.end`, ngspice sigue leyendo, Xyce se detiene y HSPICE y PSpice pueden tomar el texto como el circuito siguiente.
  - Regiones que el anfitrión no trata como comentarios de netlist: scripts de control (ngspice ejecuta líneas `*#` dentro de `.control … .endc`), bloques y archivos de otros lenguajes (`.verilog` en WRspice, `reliability{}` en Spectre; los archivos Verilog-A incorporados con `ahdl_include` o `.hdl` se leen únicamente como indica 2.9), texto cifrado, partes eliminadas por preprocesamiento y secciones `.lib` no seleccionadas. Una región que el adaptador no pueda clasificar con certeza se trata como no leída, con una advertencia.
- **Lugares que se leen con una advertencia:**
  - Líneas de comentario entre una sentencia y sus líneas de continuación (líneas `+`). Su significado para RNL es unívoco, por lo que se leen; pero los anfitriones las tratan de forma distinta (PSpice informa de un error, SPICE 3f4 une la línea `+` siguiente al comentario). Producen una advertencia, y las herramientas no deben escribir allí.
  - Regiones cifradas escritas en texto plano (`.prot … .unprot`, `protect … unprotect`). Se leen, con una advertencia: una vez cifrada la región, los registros desaparecen.
- **El lenguaje en que comienza un archivo** es una entrada para la herramienta. Por defecto lo decide la extensión: `.scs` el modo Spectre y cualquier otro archivo el modo SPICE, tal como hace el propio Spectre; el usuario puede cambiarlo para cada archivo, incluidos los archivos incluidos. Tras el comienzo, cada línea sigue las reglas anteriores (`simulator lang=`). Qué líneas están en modo SPICE y cuáles en modo Spectre decide sobre qué actúan los registros con `language` (2.8), por lo que una herramienta lee cada archivo en el lenguaje que eligió su usuario; cómo permite la herramienta elegir al usuario no forma parte de este estándar.
- **Recomendaciones para la escritura:**
  - RNL no limita la longitud de las líneas (2.4). Los generadores deberían dividir las líneas en espacios en blanco, con como máximo 132 caracteres por línea: el límite de versiones antiguas de PSpice, el más estricto entre los anfitriones comunes (HSPICE admite 1024, Eldo 2000). Un adaptador que conozca el límite de su anfitrión sigue al anfitrión. Cuando un único token no quepa en una línea, la herramienta debe informar de un error y no debe escribirlo (las cadenas no pueden continuar en la línea siguiente, 3.7).
  - Para la codificación, véase 3.1.

### 2.3 La entrada

- Un registro comienza con `@RNL`. La entrada se reconoce solo en mayúsculas.
- Todos los anfitriones usan la misma entrada, `@RNL`; es la subetiqueta de Principios 2.1.
- `@RNL` puede escribirse en mitad de un comentario; el texto anterior en la misma línea pertenece al comentario del anfitrión.
- **El espacio antes de `@`.** El carácter inmediatamente anterior al `@` de la entrada `@RNL` y de una palabra clave debe ser un espacio ASCII (U+0020); un TAB, un salto de línea o el comienzo de una línea no cuentan. En una línea portadora, el espacio que sigue al carácter de comentario cuenta como el espacio anterior a `@`, como en `* @RNL`. Un `@` dentro de una cadena, un nombre entrecomillado o un comentario `/* */` no está restringido.
  - Por tanto, una palabra clave después de `{`, `=>`, `=` o `;` también necesita un espacio delante: `#M1 @SET-SYMBOL { @DEFINE-SYMBOL {…}}`, `%x = @DEFINE(…)`, `#M2 @PLACE(30, 20) => @ORIENT("MY")`.
  - Nota: `@` es habitual en comentarios del anfitrión, como en direcciones de correo electrónico y `@param`; con esta regla, tal texto no se toma por RNL. WRspice también ejecuta líneas que comienzan con `*@` (2.2).
  - Fuera de un registro, `x@RNL` no es una entrada, sino solo un comentario del anfitrión. Dentro de un registro, un `@` sin un espacio delante no es ni una palabra clave ni un carácter de texto libre: es un error de sintaxis, y el miembro en que se encuentra falla (7.4).

### 2.4 Alcance de un registro

- Un registro se escribe `@RNL(…) {…}`. El encabezado del registro entre paréntesis puede omitirse; las llaves no, y una sola sentencia también las necesita. El paréntesis del encabezado del registro toca a `@RNL`, igual que los paréntesis de parámetros de una palabra clave tocan a la palabra clave (3.5): `@RNL (type="header") {…}` es un error, y el registro falla en bloque.
- Un registro va desde `@RNL` hasta la `}` que cierra su primera `{`. Las cadenas y los comentarios `/* */` se omiten al buscar la `}` coincidente. Los paréntesis del encabezado del registro también se omiten; tras la primera `{`, cuenta toda llave, también una dentro de los paréntesis de parámetros, de modo que un paréntesis dejado abierto hace fallar solo a su miembro (7.4). Una cadena no puede cruzar de línea (3.7): cuando no se cierra en su línea, termina ahí y el registro queda incompleto (7.4).
- **Registros en varias líneas.** Un registro puede escribirse en varias líneas de comentario consecutivas (portadores, 2.2). Se elimina el prefijo de comentario de cada línea y se unen los contenidos; un salto de línea es espacio en blanco (3.5).
  - Una línea de comentario vacía, solo con `*` (o `*` y espacios en blanco), es espacio en blanco, y el registro continúa.
  - Cuando se encuentra una línea que no es portadora antes de la `}` de cierre —una sentencia del anfitrión, una línea como `.subckt`, `.ends` o `.include`, una línea totalmente en blanco o una línea de comentario que no está escrita como portador (2.2)— o termina el archivo, el registro queda incompleto y falla en bloque. Así, un registro nunca cruza el límite de un subcircuito o de una inclusión. Para dejar una línea vacía dentro de un registro, escríbase una línea solo con `*`.
  - Cuando aparece una entrada `@RNL` fuera de cadenas y comentarios antes de que termine el registro, el registro anterior queda incompleto y falla en bloque, y la lectura vuelve a comenzar en la nueva entrada. Una `}` omitida pierde así un solo registro.
  - Un registro incompleto es un error, y el diagnóstico indica que el registro está incompleto y dónde se interrumpió. Una línea en blanco de más hace fallar todo un registro; esto ocurre con facilidad y no debe pasar en silencio (7.6).
  - Toda línea de comentario dentro de un registro es RNL. Una línea de observaciones ordinarias intercalada, como `* the next line is for M2`, se lee como texto libre, y es un error si contiene caracteres que no pueden ser texto libre. Las observaciones dentro de un registro se escriben entre `/* */` (3.4).
  - RNL no limita la longitud de las líneas; los límites del anfitrión incumben al adaptador. Una sentencia larga puede dividirse en varias líneas por espacios en blanco; los tokens y las cadenas no pueden partirse (3.5). Las recomendaciones para generadores figuran en 2.2.
- Un token en la misma línea después de terminado un registro es un error: esos tokens fallan, y el registro en sí surte efecto como de costumbre. Un nuevo `@RNL` es la excepción, por lo que una línea puede contener varios registros.
- Los portadores de comentario, la entrada `@RNL` y el encabezado del registro no dependen de la versión del lenguaje, y versiones posteriores no los cambian (16.1). Las llaves del cuerpo se emparejan como se indica arriba en esta versión. Los campos del encabezado del registro están en 2.8.

### 2.5 Registros desactivados y texto del anfitrión

- Una línea de comentario fuera de registros que no contenga `@RNL` es un comentario del anfitrión y se ignora en bloque.
- **Registros desactivados.** En una línea de comentario, un `/*` que solo tenga espacios en blanco o comentarios antes, o que venga justo después de un registro, inicia un comentario que llega hasta `*/`. Un `@RNL` dentro de él no es un registro.
- Un `/*` en medio del texto de una línea de comentario, como en `models/*.lib`, es solo texto del anfitrión.
- **Fin de un comentario del anfitrión.** Los portadores son comentarios de línea completa (2.2): un comentario del anfitrión termina al final de su línea, y nada dentro de la línea puede terminarlo antes, de modo que el fin de un comentario del anfitrión no entra en conflicto con el texto RNL.

### 2.6 Niveles

- Los niveles se dividen por el texto; su jerarquía está en 6.1:
  - El nivel de archivo del archivo principal: lo escrito en el archivo principal fuera de todo `.subckt … .ends`. Es el nivel superior del circuito.
  - El nivel de archivo de cada archivo incluido. Las secciones de `.lib` se tratan del mismo modo.
  - Cada `.subckt … .ends`.
  - El nivel de archivo de cada archivo Verilog-A que incorpora el netlist, y cada módulo dentro de él (2.9).
- Un registro pertenece al nivel en el que está escrito (Principios 2.5).
- **Orden entre archivos.** Un archivo incluido es un nivel propio, por debajo del nivel que lo incluye (6.1). Por tanto, dónde esté escrita la línea de inclusión no afecta al orden, a diferencia de USE-PACKAGE (16.2), y los archivos incluidos en paralelo no se afectan entre sí. Dentro de un nivel, el orden es el orden del texto.
- **scope.** Un registro puede nombrar un nivel destino con `@RNL(scope="name")`. El nombre es el nombre de un subcircuito y designa la definición de ese subcircuito. En el archivo siguiente, el último registro está escrito en el archivo principal y se aplica al subcircuito `ota`:

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

  - Cada subcircuito tiene un único esquema (6.1), por lo que scope nombra la definición, no una instancia.
  - scope solo desciende: el destino debe ser un subcircuito que este nivel contenga en su texto, es decir, uno definido en este archivo o en un archivo que este nivel incluya directa o indirectamente mediante nuevas inclusiones. scope no se escribe dentro de un subcircuito: el RNL de un subcircuito cubre solo a sí mismo. Un `scope` escrito en un registro dentro de un subcircuito es un error y se trata como no escrito (2.8). Cuando el destino no se encuentra o no está por debajo de este nivel, es un error, y todas las sentencias del registro, que carecen de nivel en el que resolverse, fallan (capítulo 7).
  - Todas las referencias del registro se resuelven en el nivel destino. Sus sentencias se consignan en el nivel destino, y los nombres que vinculan se publican allí y se comparten con los registros propios del nivel destino (6.7); vincular un nombre que el nivel destino ya vincula a otro objeto es un error, y vincularlo de nuevo al mismo objeto está permitido (6.6). scope afecta únicamente a su propio registro y no cambia el netlist anfitrión.
  - Orden: las sentencias que scope introduce en un subcircuito van después de su propio texto; cuando varios lugares aplican scope al mismo subcircuito, los más externos van más tarde (8.3, 8.5). Lo escrito explícitamente fuera sustituye a lo escrito dentro, del mismo modo que la prioridad de un uso externo prevalece sobre su cuerpo (14.8).
  - La salida del solucionador y los cambios que realizan los usuarios en un editor no deben utilizar scope; se escriben en el propio texto del subcircuito (capítulo 17). scope es únicamente para sentencias que los autores eligen escribir fuera.

### 2.7 Archivos de paquete

Un paquete puede escribirse como un archivo de paquete propio, que USE-PACKAGE localiza (16.2). Un archivo de paquete es un archivo RNL puro; su extensión sugerida es `.rnl`, y se escribe en UTF-8. No es un netlist y no se carga con `.include`.

- Un archivo de paquete se **lee** como los portadores de un netlist (2.2, 2.4), salvo que toda línea es un portador, y tanto el carácter de comentario como el espacio posterior están vacíos:
  - un `@` al comienzo de una línea cuenta como si tuviera un espacio delante (2.3), por lo que `@RNL` y las palabras clave pueden comenzar en la columna 1;
  - una línea vacía es una línea de comentario vacía, y un registro continúa a través de ella;
  - el texto fuera de los registros son observaciones y se ignora, como los comentarios del anfitrión (2.5); `/* */` funciona como de costumbre;
  - la línea 1 se lee: un archivo de paquete no es un netlist y no tiene línea de título (2.2).
- Contiene registros `@RNL(…) {…}`, analizados en el orden de 16.1. El primer registro es el registro de encabezado sin `language`, que contiene VERSION y PACKAGE; un archivo de paquete también puede tener un registro de encabezado para cada lenguaje (2.8). Para lo que puede contener un paquete, incluidas las referencias a otros paquetes, véase 16.2; los paquetes de alias también son archivos de paquete (16.5).

A continuación figura el texto completo de un archivo de paquete, sin prefijos de comentario. La tercera línea está fuera de los registros y es una observación. El paquete usa `$NMOS` sin hacer referencia al paquete estándar, por lo que lo encuentra en el paquete estándar vigente allí donde se utiliza o, cuando se comprueba por separado, en el que añade un ajuste de la herramienta (16.2, 16.3):

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

### 2.8 Encabezado del registro

Los campos entre los paréntesis de `@RNL(…)` se aplican a cada sentencia del registro, y todos ellos se indican por nombre. Los nombres de campo y los valores de `type` se escriben en la grafía estándar; distinguen entre mayúsculas y minúsculas y no pueden tener alias (3.6, 16.5).

```rnl
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4")}    /* the first record of the file: read as version 0.4 (16.1) */
* @RNL { %dp1 = #M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N}    /* no type: "authored" */
* @RNL(type="generated", priority="high", author="rnl-solver 0.3", hash="sig1:9f3a62c1") {
*   #M1 @PLACE(12, 20); #M2 @PLACE(28, 20)
* }
* @RNL(author="rnl-editor 0.1") { #M2 @PLACE(x=30, priority="high")}
```

| Campo | Significado | Afecta a la interpretación |
|---|---|---|
| `type` | La clase de registro: `"authored"` (por defecto), `"generated"`, `"header"` | sí |
| `scope` | El subcircuito destino (2.6) | sí |
| `priority` | La prioridad por defecto de las sentencias del registro; para AS y ATTACH solo `"disabled"` (14.8) | sí |
| `author` | El autor, la herramienta y su versión, la versión de un paquete, etc.; cualquier cadena | no |
| `hash` | Un hash de la entrada del solucionador (capítulo 17) | no |
| `numbering` | La comprobación de los números que utiliza el registro (5.9); solo con `type="generated"` | sí |
| `modified` | La hora del último cambio, una cadena ISO 8601 | no |
| `language` | El lenguaje de netlist cuyas secciones afecta el registro: `"spice"` o `"spectre"` (abajo) | sí |

- **type**: los valores son fijos, y cada uno tiene un comportamiento definido.
  - `"authored"`, el valor por defecto: escrito a mano, por un editor o mediante una importación.
  - `"generated"`: el resultado del solucionador. Cuando el solucionador resuelve, el nivel se interpreta sin los registros con `type="generated"`: no añaden objetos, nombres ni hechos, y no restringen el resultado. Cómo los use por lo demás el solucionador queda a su criterio. Después los reemplaza en bloque, sea cual sea su autor y comprenda o no sus sentencias: la regla de 16.4 según la cual una herramienta conserva las sentencias que no entiende al reescribir sus propios registros no se aplica a este reemplazo. Lo que debe conservarse se escribe en un registro de otro tipo. Los cables que un editor vuelve a dibujar mientras se arrastra un dispositivo son también resultados del solucionador (14.8, capítulo 17). `type` no implica prioridad alguna: el solucionador escribe `priority="high"` explícitamente.
  - `"header"`: los ajustes del archivo (16.1). El registro de encabezado puede omitirse. Cuando se escribe, está al nivel de archivo, no dentro de `.subckt` o de un módulo de Verilog-A (2.9), y no tiene `scope`; los registros de encabezado son los primeros registros del archivo, como máximo uno sin `language` y uno por cada lenguaje (véase `language` abajo). Contiene VERSION (16.1), los alias de ALIAS y los paquetes de alias de USE-ALIASES (16.5), los nombres de terminales y tipos de DEVICE (6.12), los paquetes en la raíz de USE-PACKAGE (16.3) y los ajustes de SOLVER-CONFIG (14.10); un archivo de paquete también escribe allí PACKAGE (2.7, 16.2). El propio registro de encabezado se lee en la grafía estándar, sin alias (16.5). Un registro de encabezado escrito después de otros registros, o un segundo sin `language` o para el mismo lenguaje, es un error y se ignora en bloque; los registros ya leídos no se revisan de nuevo. Una sección de `.lib` que se lee cuenta como un archivo (2.6): su registro de encabezado es el primer registro de la sección. Una herramienta escribe el registro de encabezado con un marcador de comentario del modo de lenguaje donde lo escribe (2.2). Cuando la primera sentencia de un archivo aparece después de una línea `simulator lang=`, el registro de encabezado puede ir justo después de esa línea, en el modo del resto del archivo, o antes de ella, como en la línea 2, en el modo en que comienza el archivo; en cualquiera de los dos sitios es el primer registro del archivo.
- **language** hace que un registro actúe únicamente sobre los objetos en secciones de ese lenguaje.
  - **Secciones**: el adaptador divide cada archivo en secciones por modo de lenguaje, mediante `simulator lang=` (2.2). Los valores son `"spice"` para la familia SPICE, incluidos HSPICE y los demás dialectos de SPICE, y `"spectre"`; versiones posteriores pueden añadir un lenguaje incompatible con estos. Los valores distinguen entre mayúsculas y minúsculas.
  - **Alcance nominal**: los niveles que alcanza un registro son los mismos que sin `language`: un registro escrito en un nivel alcanza ese nivel y los niveles inferiores, y un registro en un paquete alcanza los niveles que utilizan el paquete (16.2). De lo que alcanza, un registro con `language` actúa solo sobre los objetos en secciones de su lenguaje; un registro sin `language` actúa sobre todas las secciones, de modo que en un archivo Spectre actúa también sobre una sección SPICE.
  - **En qué sección está un objeto**: una instancia y sus terminales, la sección de la línea de instancia; un puerto de un subcircuito (`#:port`), la sección de la línea que inicia el subcircuito; una red, toda sección con una línea que la escriba, de modo que una red escrita en ambos tipos de sección es alcanzada por registros de ambos lenguajes; un objeto que crea RNL (un uso, punto, gráfico o texto), la sección del objeto del anfitrión al que está adjuntado o que lo rellena, como una tierra adjuntada a `&vss`; el nivel mismo y lo que no se adjunta a ningún objeto del anfitrión, todo lenguaje que tenga alguna línea en el nivel.
  - **Sentencias entre secciones**: una sentencia cuyos objetos quedan parcialmente fuera de las secciones del lenguaje del registro, como una restricción, un cable o un GROUP, no tiene efecto en bloque. Una referencia a un objeto fuera de esas secciones no tiene efecto y no es un error, y lo que depende de ella tampoco tiene efecto; un nombre no encontrado en absoluto es un error como de costumbre, cuando el nivel tiene secciones de ese lenguaje. Un registro cuyo lenguaje no tiene ninguna sección en lo que alcanza no actúa sobre nada y no se comprueba.
  - **Las definiciones no se dividen por sección**: los nombres `$`, símbolos y estilos son visibles en el alcance nominal como de costumbre (6.8); solo las sentencias que actúan sobre objetos se filtran por sección.
  - **Registros de encabezado**: un archivo puede tener un registro de encabezado sin `language` y, para cada lenguaje, un registro de encabezado con él. Se escriben primero, uno tras otro, primero el que no tiene `language`. VERSION, PACKAGE, ALIAS y USE-ALIASES se escriben únicamente en el registro de encabezado sin `language`; un registro de encabezado con `language` contiene solo USE-PACKAGE y DEVICE, que entonces actúan únicamente sobre las secciones de su lenguaje (6.12, 16.2). Todos los registros de encabezado surten efecto juntos. El mismo paquete escrito en dos registros de encabezado con versiones distintas o contenido en línea distinto es un error, pues un nivel puede tener secciones de ambos lenguajes y utiliza un paquete de un nombre una sola vez (16.2): el USE-PACKAGE del registro de encabezado con `language` falla, y el del registro de encabezado sin `language` surte efecto para cada sección; entre dos registros de encabezado con `language`, falla el posterior.
  - Un registro con `type="generated"` no puede tener `language`: el solucionador resuelve todo un nivel y reemplaza sus registros en bloque. Escrito allí, `language` es un error y se trata como no escrito.
  - Un valor que esta versión no enumera hace que el registro no actúe sobre nada, con una advertencia que nombra el valor más cercano; esto es una excepción a la regla siguiente según la cual un campo conocido mal escrito se trata como no escrito, lo que haría que el registro actuara sobre toda sección. Las herramientas que escriben `language` escriben `@VERSION("0.4")` en el registro de encabezado sin `language` (16.1). Los registros que las herramientas escriben nuevos no tienen `language`; los registros que lo tienen se conservan tal cual (16.4).
- **Metadatos**: `author`, `hash` y `modified` no afectan a la interpretación; el dibujo es el mismo si se eliminan. Las herramientas los usan para decidir cómo tratar un registro (capítulo 17). Ni `type` ni `author` afectan al orden de resolución (8.3).
- **Errores**:
  - Un campo desconocido solo produce una advertencia y se ignora; no afecta al registro ni a los demás campos. Esto deja margen para extensiones posteriores. Cuando se da la advertencia, debe nombrar el nombre de campo más cercano. Nota: en consecuencia, un campo mal escrito que afectaría a la interpretación (como `priorty="low"`) solo recibe una advertencia, y el registro surte efecto con prioridad `"normal"`; los campos de esa clase añadidos más adelante confían en la versión (16.1) para indicar a las herramientas antiguas que están desactualizadas.
  - Un campo conocido mal escrito (un valor erróneo, `numbering` en un registro cuyo `type` no es `"generated"` o que tiene `scope`, o el campo escrito dos veces) es un error, y el campo se trata como no escrito: `priority` revierte a `"normal"`, `type` a `"authored"`, y los metadatos se descartan. La excepción es un `scope` cuyo destino no se encuentra: las sentencias del registro carecen de nivel en el que resolverse y todas fallan (2.6). Un campo dado por posición es un error y se ignora.
- Los registros no tienen identificadores. Las herramientas encuentran sus propios registros por `type`, `author` y las claves de las sentencias (5.8); se hace referencia a un objeto desde otro registro por su nombre o, desde el registro del solucionador, por su número (5.9). Otras observaciones se escriben en comentarios.
- Un resultado del solucionador no es una propuesta que deba adoptarse: surte efecto directamente, y un resultado obsoleto solo produce una indicación (capítulo 17).

### 2.9 Archivos Verilog-A

Un subcircuito da a sus instancias su símbolo de bloque (block symbol) (9.2.6). Un módulo de Verilog-A que el netlist incorpora puede hacer lo mismo, con registros escritos en el propio archivo Verilog-A:

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

Un netlist Spectre que escriba `ahdl_include "vco.va"` y `I1 (ctl out vdd 0) vco f0=2e9` dibuja I1 con este símbolo, sin ningún RNL propio.

- **Qué archivos.** Los archivos fuente Verilog-A que incorpora el netlist: con `ahdl_include` en Spectre, en cualquiera de los dos modos de lenguaje, y con `.hdl` en HSPICE. Tal archivo se lee donde se incorpora, igual que un archivo incluido (6.1). Con `ahdl_include … -master name` de Spectre, el módulo se conoce con ese nombre en el netlist, y su símbolo de bloque va a las instancias de ese nombre. Un modelo que un anfitrión incorpora ya compilado, como un archivo `.osdi` de ngspice, no tiene texto que leer; sus instancias tienen el símbolo de bloque por defecto, con un pin por cada nodo de sus líneas (9.2.6), y un nivel superior puede darles un símbolo con una regla.
- **Portadores.** Comentarios de línea completa `//`, con las reglas de 2.2: `//` en la columna 1, seguido de nada, solo de espacios en blanco o de un espacio y el contenido. Los comentarios al final de línea y los comentarios `/* */` no se leen, y `@RNL` en ellos produce una advertencia. La línea 1 se lee: un archivo Verilog-A no tiene línea de título. El texto que el preprocesador de Verilog-A pueda excluir, entre `` `ifdef `` o `` `ifndef `` y su `` `endif ``, no se lee, y `@RNL` allí produce una advertencia, como en las regiones de 2.2 que el adaptador no puede clasificar.
- **Niveles.** El texto fuera de todo `module … endmodule` (y `macromodule … endmodule`) es el nivel de archivo del archivo Verilog-A. Cada módulo es un nivel propio, un nivel de módulo (module level), por debajo de ese nivel de archivo, el cual está por debajo del nivel cuyo texto incorpora el archivo (6.1).
  - El nivel de archivo contiene únicamente los registros de encabezado (2.8), escritos antes del primer módulo. Cualquier otro registro allí es un error y falla.
  - Un nivel de módulo no tiene circuito ni esquema: nada en él se dibuja ni se resuelve, y no tiene registro del solucionador.
- **Qué contiene un nivel de módulo.** DEFINE-SYMBOL y DEFINE-STYLE, y una sentencia `# @SET-SYMBOL` con un símbolo, donde `#` es el propio módulo. Este SET-SYMBOL da el símbolo de bloque del módulo, con todo lo que 9.2.6 dice del símbolo de bloque de un subcircuito: se aplica dondequiera que se instancie el módulo, y su lugar en la cola de símbolos es el mismo (8.3). Cualquier otra cosa escrita en un nivel de módulo es un error y falla: referencias al anfitrión, restricciones, PLACE, ORIENT, WIRE, un TEXT fuera del cuerpo de un símbolo, reglas distintas de `# @SET-SYMBOL`, AS y ATTACH, aserciones, `scope`, DEVICE y SOLVER-CONFIG. USE-PACKAGE se escribe únicamente en el registro de encabezado del archivo.
- **Terminales.** Los terminales de una instancia de un módulo son los nodos que el anfitrión lee en su línea, en el orden de los puertos en la lista de puertos del módulo, y reciben el nombre de los puertos. Verilog-A compara los nombres distinguiendo mayúsculas y minúsculas, por lo que los nombres de los puertos también lo hacen.
  - Un puerto vectorial (vector port) es un terminal por cada uno de sus bits, desde el límite izquierdo de su intervalo declarado hasta su límite derecho, ya que un vector se conecta por posición: el puerto `inout [3:0] d` toma los cuatro nodos siguientes de la línea, como `d[3]`, `d[2]`, `d[1]` y `d[0]`, y `inout [0:2] e` toma tres, como `e[0]`, `e[1]` y `e[2]`. Estos son sus nombres de terminal, escritos con un subíndice en las referencias (6.11), como `#I1:d[3:0]`, y en `terminal=` de PIN como cadenas, como un pin de bus con `terminal=["d[3]", "d[2]", "d[1]", "d[0]"]` (9.2.4).
  - Un módulo no da ningún tipo a sus instancias (6.12); DEVICE en el netlist puede darles uno, como para otras instancias.
- **Versiones y paquetes.** El registro de encabezado de un archivo Verilog-A es el de un archivo incluido: sin él, o sin VERSION, el archivo toma la versión del archivo que lo incorpora (16.1), y su USE-PACKAGE se aplica únicamente al archivo y a sus módulos (16.2).
- **Escritura.** Una herramienta escribe `//` en un archivo Verilog-A, y únicamente el registro de encabezado y los registros de símbolos de bloque; no escribe allí ninguna salida del solucionador (capítulo 17).

<!-- translated from en/ch03.md sha256:9148069f1090 -->

## 3 Estructura léxica

### 3.1 Caracteres

- Los tokens están formados por caracteres ASCII.
- Los caracteres no ASCII aparecen únicamente en cadenas (incluidos los nombres entrecomillados), texto libre, comentarios y alias, y en nombres de rol y de parámetro sin comillas. Fuera del texto libre y los comentarios, una palabra no ASCII sin comillas aparece únicamente después de `@` y dentro de paréntesis o corchetes: después de `@` debe ser un alias declarado; dentro de paréntesis o corchetes es un alias, o bien un nombre de rol o un nombre de parámetro en sí mismo (3.6, 16.5).
- **Codificación.** El adaptador del anfitrión decodifica el texto según las reglas del anfitrión; RNL trabaja sobre caracteres. Una herramienta que escribe RNL no debe cambiar la codificación ni los finales de línea del archivo anfitrión y no debe añadir una marca de orden de bytes; no debe escribir un carácter que la codificación del anfitrión no pueda representar y en su lugar emite una advertencia. Los archivos nuevos deberían usar UTF-8.
- **Caracteres no ASCII en los comentarios del anfitrión.** Los adaptadores deben declarar los anfitriones para los que se han verificado caracteres no ASCII en los comentarios; para otros anfitriones, un carácter no ASCII produce una indicación.

  Nota: el soporte difiere entre anfitriones. ngspice (a partir de la versión 42), WRspice (salvo en el título), Xyce y las versiones recientes de LTspice aceptan caracteres no ASCII en los comentarios; la documentación de HSPICE, PSpice y Eldo no lo indica; el manual de Spectre describe los netlists como archivos ASCII; QSPICE utiliza Windows-1252. Otra razón para no añadir una marca de orden de bytes: versiones antiguas de ngspice la unen al primer token de un archivo incluido, y WRspice deja de leer cuando la línea 1 comienza con una.

### 3.2 Tokens

| Token | Forma |
|---|---|
| Palabra clave | `@` seguido de un nombre en mayúsculas, como `@PLACE`; o un alias declarado (16.5) |
| Referencia | Un prefijo seguido de un nombre, posiblemente con terminales y subobjetos, como `#M1`, `#M1:d`, `%dp1::left` (capítulo 6); o `#`, `?` o `&` solos (6.2). Un nombre del anfitrión o un nombre de terminal puede ir seguido directamente de un subíndice, como `&D<3>`, `&D<7:0>` (6.11). Una referencia a un conjunto puede ir seguida directamente de lugares de miembros, como `%a{1}`, `?ref{2:end}` (5.2.3). En un registro del solucionador, también `%#` seguido de un entero sin signo, un número, como `%#3` (5.9) |
| Punto | `(x, y)`, sin tocar una palabra clave ni un objeto: después de espacio en blanco, un comentario, `{`, `;`, `=>` o `=` (3.8) |
| Paréntesis | Los `(` `)` de parámetros y roles, tocando la palabra clave o el objeto (3.5) |
| Llaves | Las `{` `}` de conjuntos |
| Separadores | `;` (separa miembros), `=>` (tuberías y nombramiento), `=` (nombramiento al comienzo de un miembro) |

Dentro de paréntesis y corchetes (los `()` de parámetros, roles y puntos, y los `[]` de matrices) hay también cadenas, números, `,`, `=`, `_` y nombres (nombres de parámetro, nombres de rol, `true`, `false`), y, en el cuerpo de una definición general, marcadores de parámetro `?name` (6.10). Dentro de `<>` y `[]` de subíndices solo hay enteros sin signo, `,` y `:` (6.11); dentro de las llaves de lugares de miembros, solo enteros sin signo, `end`, `-`, `,` y `:` (5.2.3).

### 3.3 Texto libre

- Fuera de paréntesis y corchetes, cualquier carácter puede ser texto libre salvo los enumerados abajo. Esto incluye todos los caracteres no ASCII (como la puntuación china `：`, `、`, `；`, `（`, `）`) y `-`, `:`, `'`, `!`, `+`, `/`, `*` y `|`. Las excepciones son:
  - `@`, `#`, `&`, `%`, `$`, `?`, `"`, `;`, `=`, `{`, `}`, `(`, `)`: fuera de paréntesis y corchetes inician tokens o pertenecen a ellos (3.2). `&` solo, sin nombre después, es el nodo vacío (empty node) (6.3). `?` inicia un marcador (6.10); un `?` aislado hace referencia, en un cuerpo de símbolo, al objeto dibujado con el símbolo (13.3), y en el cuerpo de una definición general, al uso que se está expandiendo (9.4);
  - `[`, `]`, `<`, `>`: los corchetes y delimitadores de matrices y subíndices (3.7, 6.11);
  - `/*`: inicia un comentario (3.4).
- Las reglas de posición mantienen el significado unívoco:
  - Un `:` o `::` inmediatamente después de una referencia y seguido de un nombre o una comilla es un terminal o un subobjeto, como de costumbre (6.4, 6.5). Cuando no sigue ningún nombre, el `:` es texto libre, como en `Input pair: %dp1 = …` y `#M1: the left one`.
  - Un `<` o `[` inmediatamente después de un nombre es un subíndice (6.11).
  - Una `{` inmediatamente después de una referencia, sin espacios en blanco, inicia lugares de miembros (5.2.3); después de espacio en blanco inicia un conjunto.
  - En el nombre de una palabra clave, un `-` pertenece al nombre solo cuando le sigue una letra (palabras clave: 3.6). `@GROUP- the pair` es GROUP seguido de texto libre.
- El texto libre puede escribirse entre cualesquiera dos tokens fuera de paréntesis y corchetes, y al comienzo de un miembro.
- **El texto libre forma parte de la sentencia.** Una referencia escrita en él no es una explicación, sino un objeto de la sentencia: `Unlike #M2, #M1 @PLACE(x=0)` sitúa también a M2 en x = 0. El texto destinado únicamente a explicación se escribe en un comentario (3.4), o como un miembro propio, separado del miembro siguiente por `;`: un miembro que solo contiene texto libre es un miembro vacío y se ignora (4.1).
- El estándar no comprueba el texto libre, y el texto libre no altera el significado (Principios 2.8): `do not move #M2 @PLACE(x=30)` sigue moviendo a M2. Una implementación puede emitir una advertencia propia para palabras como negaciones (7.6).
- Del mismo modo, en un `#M1.d` erróneo, `.d` es texto libre, y la referencia es el propio M1, no su drenador (se emite una advertencia, véase abajo).
- No hay texto libre dentro de paréntesis ni corchetes.
- **Advertencias.** Los siguientes errores podrían pasar en silencio; producen una advertencia (7.6):
  - Una palabra en texto libre que sea el nombre de una instancia del anfitrión de este nivel, un nombre RNL visible (sin su prefijo) o el nombre de una palabra clave, como `M3` en `#M1 #M2 M3 @ALIGN("y")` o `WIRE` en `WIRE {…}`: probablemente falta un `#`, `%`, `$` o `@`. Los nombres de red suelen ser palabras corrientes (como in y out) y no se comprueban.
  - Una referencia seguida directamente de `.` y un nombre, como `#M1.d`: los terminales se escriben con `:`.
  - Una referencia seguida directamente de `-`, `!`, `'`, `+`, `/`, `*`, `\`, `^`, `~` o `|`, como `&vdd!` o `#X1-a`: probablemente el nombre contiene ese carácter y necesita comillas, como en `&"vdd!"` (3.6); sin ellas, la referencia designa silenciosamente a `vdd` o `X1`.
  - `|` en texto libre: una tubería se escribe `=>` (4.2 punto 3).
  - `:` o `::` inmediatamente después de una referencia y seguido de una palabra no ASCII, como `%c1::中点`: un nombre con caracteres no ASCII después de `::` necesita comillas, como en `%c1::"中点"` (3.6); sin ellas, `::` y la palabra son texto libre, y la referencia designa a `%c1`.
- La mayoría de las demás equivocaciones de este tipo ya son errores: no hay texto libre dentro de paréntesis o corchetes, por lo que `@ORIENT(MY)` es un error; `#M1 @PLACE 1 2` no coincide con la firma; `#M1 (in_p)` es un error, pues un paréntesis tras un espacio en blanco inicia un punto y `(in_p)` no es un punto (3.5); y el `>` de `R1 -> R2` no puede ser texto libre.
- Las guías de escritura para autores se generan a partir de la lista de palabras (5.3). Los registros escritos por solucionadores y editores no deben contener texto libre.

### 3.4 Comentarios

`/* … */` puede escribirse dondequiera que se permita espacio en blanco, incluso dentro de paréntesis y corchetes. Los comentarios no se anidan y dentro de ellos puede escribirse cualquier carácter. Una explicación que contenga referencias, o caracteres como `@`, `#` y `;`, se escribe en un comentario (3.3).

### 3.5 Espacios en blanco y adyacencia

- El espacio en blanco es el espacio, el TAB y el salto de línea. Los tokens no pueden partirse entre líneas y las cadenas no pueden abarcar varias líneas.
- No se permiten espacios en blanco ni comentarios: entre un prefijo y su nombre, ni entre `%#` y su número; a ninguno de los lados de `:` y `::`; entre un nombre y su subíndice (6.11); entre una referencia y la `{` de sus lugares de miembros (5.2.3); entre una palabra clave y sus paréntesis de parámetros, ni entre `@RNL` y el paréntesis de su encabezado del registro (2.4); entre un objeto y sus paréntesis de rol.
- Un `(` tras espacio en blanco o un comentario inicia un punto (3.8). Así, `#M1 @PLACE (10, 20)` es PLACE seguido de un punto, y PLACE no tiene parámetros: es un error. `#M1 (in_p)` es M1 seguido de un punto que no está escrito correctamente: también es un error. El diagnóstico sugiere eliminar el espacio en blanco.
- Un `<` o `[` inmediatamente después de un nombre, entrecomillado o no, inicia siempre un subíndice (6.11). Los puntos se escriben `(x, y)`, lo que no entra en conflicto con los subíndices; para escribir un punto después de un nombre, sepárense con espacio en blanco, como en `&D (1, 2)`. Dentro de un subíndice pueden escribirse espacios en blanco y comentarios, como en `&D<1, 2:3>` (6.11).
- Un `.` inmediatamente después de una palabra clave es texto libre, como el punto final en `{#M1 #M2} @GROUP.`; `@BUNDLE.LOCK` es BUNDLE seguido del texto libre `.LOCK` (16.4).

### 3.6 Nombres

- **Los nombres sin comillas** son una o más letras ASCII, dígitos y guiones bajos, tomados por coincidencia más larga.
- **Los nombres del anfitrión** (después de `#` y `&`) y los nombres de terminal siguen las reglas del dialecto anfitrión (Principios 2.1): qué nombres son válidos, si importan las mayúsculas y minúsculas y si un nombre puede empezar por un dígito, como en `&0`. SPICE no distingue entre mayúsculas y minúsculas; Spectre sí. Un nombre con otros caracteres se escribe entre comillas, como `#"M1<0>"`, `&"vdd!"`, `#XU1:"in+"`; el texto entre comillas se escribe como en las cadenas (3.7). Un nombre puede ir seguido directamente de un subíndice: `#M1<0>` es `#"M1<0>"`, y `&D<7:0>` es una lista de redes (6.11). Un `<` o `[` dentro de comillas forma parte del nombre y no se analiza.
- **Nombres RNL** (los nombres después de `%`, `$` y `?`, los nombres de rol y los nombres de parámetro de definiciones generales):
  - Distinguen entre mayúsculas y minúsculas: `$DIFFERENTIAL_PAIR` y `$differential_pair` son dos nombres distintos.
  - Después de `%`, `$` y `?` un nombre puede empezar por un dígito, como en `%1st`. En los lugares sin prefijo (`(role)` y `parameter=`), un nombre sin comillas consta de letras ASCII, dígitos, guiones bajos y cualesquiera caracteres no ASCII, y no empieza por un dígito ASCII, para que no parezca un número: `#M1(正输入)`, `@AS(间距=3) $X`. Ambos lugares están dentro de paréntesis, donde no hay texto libre (3.3), de modo que no se incorpora ninguna otra palabra al nombre.
  - Pueden ir entrecomillados, con el mismo significado: `$"a"` es `$a`; las comillas solo escapan. Los nombres con otros caracteres y los nombres no ASCII se escriben entre comillas, como `%"差分对"`, `%"input pair"`, `?"正输入"`, `%dp::"正输入"`. En los lugares de rol y nombre de parámetro, los nombres no ASCII pueden ir sin comillas (el punto anterior), pero un espacio u otra puntuación ASCII sigue necesitando comillas, como en `#M1("input pair")`. En los lugares de rol y nombre de parámetro, un nombre entrecomillado es siempre el nombre mismo, nunca un alias. Esto no se aplica a los nombres `$` ni a los nombres después de `::`, cuyos alias no ASCII se escriben entre comillas de todos modos (16.5).
  - Se comparan carácter por carácter tal como están escritos, sin normalización Unicode. Un nombre vacío es un error.
  - No admiten subíndice (6.11): un `<` o `[` inmediatamente después de ellos es un error, como en `%p<1, 2>`. Un punto se escribe `(1, 2)`, separado del nombre por espacio en blanco.
- **Los alias** (16.5) se declaran en el registro de encabezado o proceden de paquetes de alias. Los alias de palabras clave y palabras estándar pueden contener letras no ASCII; se escriben sin comillas únicamente después de `@` y dentro de paréntesis o corchetes. Un alias de un nombre `$` se escribe como un nombre `$`, entre comillas cuando es no ASCII, como en `$"接地补位集"`. Los nombres sin comillas después de `#`, `&`, `%`, `$`, `?` y `::` son solo ASCII, como arriba, por lo que `#M1和#M2` es `#M1`, el texto libre `和` y `#M2`.
- **Las palabras clave** van solo en mayúsculas, con palabras unidas por `-`, como `@AS` y `@SET-SYMBOL`; las mayúsculas también facilitan encontrarlas en el texto libre. `@at` es una palabra clave desconocida, salvo que el registro de encabezado la declare como alias (16.5).
- **Las palabras estándar** distinguen entre mayúsculas y minúsculas y se escriben tal como las escribe el estándar: nombres de parámetro, nombres de campo del encabezado del registro, nombres de subobjeto, nombres de condiciones de selección y valores de enumeración (como `"x"`, `"low"`, `"MY"`). Los valores que nombran cosas del anfitrión (nombres en condiciones de selección, nombres de modelo, nombres de red) siguen las reglas del anfitrión. Otras grafías pueden aceptarse mediante alias (16.5).
- **`true` y `false`** están reservados y van solo en minúsculas. No pueden ser nombres de rol ni nombres de parámetro.

### 3.7 Literales

- **Los números** son decimales. Pueden llevar signo y ceros iniciales, omitir el cero antes del punto decimal o los dígitos posteriores, y llevar exponente: `0`, `+1`, `01`, `.5`, `1.`, `-2.5E-4`. `01` es 1, no octal. No hay unidades ni sufijos de ingeniería (`2u` y `10k` son errores), ni números hexadecimales, ni `NaN`, ni `Infinity`. El número de dígitos no está limitado. Cuando una implementación no pueda albergar un valor, la sentencia falla en esa implementación, con un error que indica que es un límite de la implementación; el valor no se altera en silencio. Las coordenadas se comparan con una tolerancia (11.7).
- **Las cadenas** se escriben entre comillas dobles y no pueden abarcar varias líneas. Los caracteres entre comillas se toman tal como están; únicamente `"` se escribe `""`, como en `"say ""hi"""`. No hay escapes con barra invertida: `\` es un carácter ordinario, y no existen `\n` ni `\uXXXX`; los caracteres no ASCII se escriben directamente. No puede escribirse un salto de línea en una cadena; un salto de línea en un texto se escribe con el marcado `\\` (13.6). TEXT tiene órdenes propias con barra invertida (13.6); son la forma en que TEXT interpreta su texto, no escapes de la cadena. Las cadenas en el encabezado del registro se escriben del mismo modo; esto pertenece a la forma externa y no cambia (16.1).
- **Las matrices** se escriben `[value, value, …]`. Sus elementos son literales, y las matrices pueden anidarse.
- **Los valores de verdad** son `true` y `false`.
- No hay otros literales. Las enumeraciones se escriben como cadenas, como `axis="x"`; no hay `null`, ni mapas, ni valores construidos.
- No hay expresiones: los operadores son errores.

### 3.8 Puntos

- Un punto se escribe `(x, y)`, donde x e y son números o `_` (en el cuerpo de una definición general, también un marcador de parámetro, 6.10). `_` deja ese eje libre, para que lo decidan las restricciones o el solucionador.
- Escribir un punto crea un objeto punto (5.1).
- No hay aritmética sobre puntos; las posiciones relativas se escriben como restricciones (capítulo 14). En el cuerpo de una definición general, las coordenadas literales son desplazamientos respecto al punto propio del uso (9.4).
- El `(` de un punto lleva espacio en blanco o un comentario delante, o viene justo después de `{`, `;`, `=>` o `=`. Los paréntesis que tocan una palabra clave o un objeto contienen sus parámetros o su rol (3.5). Un `<` inmediatamente después de un nombre es un subíndice en todo caso (6.11).

<!-- translated from en/ch04.md sha256:86b9e3072d96 -->

## 4 Sintaxis

### 4.1 Gramática

La gramática de un registro se expresa en EBNF. Cada producción se escribe `name = expression ;`; `|` separa alternativas, `[ … ]` es opcional, `{ … }` se repite cero o más veces, `( … )` agrupa, el texto entre comillas dobles se escribe literalmente y `(* … *)` es un comentario.

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

- `name`, `bracket-name`, `keyword-name`, `string`, `number` e `integer` se escriben como define el capítulo 3. `name` es un nombre sin comillas (3.6). `bracket-name` es un nombre sin comillas en los paréntesis de un rol o antes del `=` de un parámetro: letras ASCII, dígitos, `_` y cualesquiera caracteres no ASCII, sin empezar por un dígito ASCII (3.6). `keyword-name` se compone de letras, dígitos y `_`, con palabras unidas por `-`; un `-` pertenece a él solo cuando le sigue una letra (3.3). Una palabra clave estándar va en mayúsculas (3.6); un alias se compone de letras de cualquier escritura, dígitos y `_`, sin `-` (16.5). `string` y `number` están en 3.7; `integer` es un entero decimal sin signo.
- `rnl-name`, `host-name`, `terminal-name`, `role-label` y `parameter-name` también pueden ir entrecomillados, con el mismo significado, salvo que en los lugares de rol y nombre de parámetro un nombre entrecomillado nunca es un alias (3.6). Las palabras clave, nombres de parámetro, nombres de ranura entre paréntesis de rol, nombres de subobjeto, `true` y `false` pueden escribirse como alias declarados; los alias no ASCII sin comillas se utilizan únicamente después de `@` y dentro de paréntesis o corchetes, y un alias después de `::` va entrecomillado (16.5).
- Un subíndice sigue directamente a un nombre del anfitrión o a un nombre de terminal, sin espacio en blanco entre ellos; dentro de un subíndice pueden escribirse espacios en blanco y comentarios (3.5, 6.11). Qué nombres admiten subíndice: 4.2 punto 10.
- Los paréntesis de parámetros tocan a la palabra clave, y los paréntesis de rol tocan al objeto; un `(` después de espacio en blanco o un comentario inicia un punto (3.5).
- `=>` es una tubería o un nombramiento, según lo que le siga (4.2 punto 3).
- Un marcador con nombre se escribe únicamente en el cuerpo de una definición general (6.10). En un lugar de objeto es un rol y puede tener un camino; en un lugar que toma un literal (un valor de parámetro, un elemento de matriz, una coordenada de punto) es un parámetro de la definición y no tiene camino (6.10, 9.4). El `?` aislado se escribe en un cuerpo de símbolo (13.3) y en el cuerpo de una definición general (9.4); únicamente en una definición general puede tener un camino, como `?::position`.
- Pueden escribirse texto libre y comentarios entre cualesquiera dos tokens fuera de paréntesis y corchetes, y al comienzo de un miembro (3.3, 3.4); también pueden escribirse espacios en blanco y comentarios dentro de paréntesis y corchetes. La gramática no los muestra.
- Un miembro vacío (como en `;;`) se ignora; el `;` después del último miembro puede omitirse.

### 4.2 Restricciones fuera de la gramática

1. Un segmento no es vacío.
2. Un segmento tiene como máximo una palabra clave. `@RNL` aparece únicamente al comienzo de un registro.
3. Un `=>` seguido de un nombre `%` o `$`, tras el cual viene `=>`, `;`, `}` o el final del miembro, es un nombramiento. Cualquier otro `=>` inicia el segmento siguiente; ese segmento tiene exactamente una palabra clave, y un nombre escrito allí es un objeto del segmento. El primer segmento de una tubería puede no tener palabra clave; entonces tiene exactamente un objeto. Así, para nombrar un resultado y continuar la tubería, escríbase `=> %g0 => @PLACE(1, 2)`. En `=> %g0 @PLACE(1, 2)`, `%g0` es un objeto de PLACE; por lo general no está vinculado, y la sentencia falla porque no se encuentra `%g0`. El diagnóstico puede preguntar si falta un `=>`.
4. En un segmento sin palabra clave, cada argumento es una sentencia propia: `#M1 #M2` en `{#M1 #M2}` son dos sentencias. Tal segmento, cuando se nombra (`%p =` o `=> %p`), tiene un único objeto: escríbase `%p = {#M1 #M2}`, no `%p = #M1 #M2`.
5. Los roles se marcan únicamente en los argumentos de un segmento que tiene palabra clave. Los segmentos posteriores a una tubería no marcan roles.
6. En un segmento, o bien todos los argumentos se marcan con roles, o bien ninguno. El mismo rol no se marca dos veces.
7. Para las palabras clave que toman una definición, la palabra clave fija la ranura de la definición: para ATTACH es el primer objeto escrito; para AS, SET-SYMBOL, SET-STYLE, SELECT-USAGE y SELECT-ANCESTOR-USAGE es el último objeto. SELECT-USAGE puede omitir su definición: su último objeto rellena la ranura de la definición solo cuando es una definición (10.1). La definición no se marca con un rol y no cuenta para la regla 6 (5.6).
8. Hay dos formas de nombrar, y pueden usarse juntas. `name =` solo puede ser el primer token de un miembro (puede haber texto libre antes), como máximo una vez en un miembro. `=> name` sigue a un segmento, al final del miembro o antes del `=>` del segmento siguiente, y un miembro puede tener varios. La regla 3 lo distingue de una tubería.
9. Los parámetros dados por posición van primero, luego los dados por nombre: después de un `name=value`, todos los parámetros siguientes se dan por nombre. Solo los parámetros para los que la firma indica un orden de posición pueden darse por posición, en ese orden. Ningún parámetro se da dos veces.
10. Un subíndice sigue únicamente a un nombre del anfitrión después de `#` o `&`, o a un nombre de terminal o nombre de puerto en una referencia `#`. Los nombres RNL, los nombres de rol y los nombres de subobjeto no admiten subíndice (6.11).
11. Una referencia tiene como máximo un subíndice que se expanda a una lista de nombres (6.11).

### 4.3 Orden de palabras

- La palabra clave, con sus parámetros, puede situarse en cualquier lugar entre los objetos; el orden de los objetos importa. `#M1 #M2 @DISTANCE(…)`, `#M1 @DISTANCE(…) #M2` y `@DISTANCE(…) #M1 #M2` son equivalentes.
- Cada palabra clave tiene una forma estándar, y las herramientas deben escribir únicamente esa forma. La forma estándar está registrada en la lista de palabras (5.3): dónde se sitúa la palabra clave, qué parámetros se dan por posición y cuáles por nombre, y si se marcan los roles.
- Por defecto, la forma estándar es: primero los objetos, luego la palabra clave con sus parámetros, y después la definición tras la palabra clave. ATTACH escribe primero su definición, luego la palabra clave y después los objetos: `$GROUND @ATTACH &vss`. TABLE escribe primero su palabra clave, antes de la tabla: `@TABLE layout: {…}` (14.11). Los parámetros para los que la firma indica un orden de posición se dan por posición, primero; los demás se dan por nombre, en el orden de la lista de palabras: `@PLACE(10, 20)`, `@ORIENT("R90")`, `@DISTANCE("bottom-to-top", min=4)`, `@TEXT("W/L", key="wl")`. PLACE da x e y por posición solo cuando se escriben ambos; un único eje se escribe por nombre, como en `@PLACE(x=10)`. Los parámetros que pueden suprimirse sin cambiar el significado no se escriben, como `mode="auto"` de WIRE. Las palabras clave integradas se escriben sin roles; AS y ATTACH marcan todos los roles cuando la definición general tiene dos o más roles.
- Disposición tipográfica: un espacio después de cada coma dentro de paréntesis y corchetes; ningún espacio alrededor del `=` de un parámetro; un espacio a cada lado del `=` de un nombramiento y de `=>`; la palabra clave toca su paréntesis; un espacio entre `{` y una palabra clave, como en `{ @…` (2.3); los números se escriben como el decimal más corto que al leerse de nuevo da el mismo valor.
- **Forma estándar de nombramiento.** Un miembro sin tubería se nombra al comienzo, `%x = statement`; un miembro con tubería se nombra al final, `statement => … => %x`, con el nombre después del segmento al que da nombre. Así, un nombre al comienzo nombra siempre a todo un miembro, y los lectores no tienen que adivinar qué segmento de una tubería nombra. La sintaxis acepta ambas formas.
- "Únicamente esa forma" se aplica a las sentencias que una herramienta escribe de nuevo. Cuando una herramienta reescribe registros existentes, no debe reordenar las sentencias que escribieron los usuarios (16.4); las lleva a la forma estándar solo cuando el usuario solicita formatear.
- Las sentencias estándar están en 4.5.

### 4.4 Ejemplos de errores

Cada uno de los siguientes es un error; el comentario indica por qué.

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

### 4.5 Sentencias estándar

Una sentencia estándar es una sentencia en la forma estándar con texto libre añadido entre sus tokens, de modo que se lea como una frase. Las sentencias estándar se usan en las guías de escritura y por las herramientas de formateo que añaden o eliminan las palabras en las sentencias. El texto libre no altera el significado (3.3): las sentencias son solo sugerencias, y los usuarios y modelos de lenguaje pueden escribirlas, no escribir ninguna o escribir otras palabras.

- **Hechos.** Una sentencia dice lo que se cumple en el dibujo, no una operación que ejecutar. Los objetos van primero como sujeto, la palabra clave hace de verbo o de sustantivo y los parámetros son sus valores. Solo se añaden palabras de enlace, como `is`, `are`, `has`, `is at` y `there is`. Algunas sentencias pueden leerse como operaciones, como `#M1 @SET-SYMBOL to $NMOS`.
- **Las palabras se deducen de la sentencia.** Las palabras que genera una herramienta dependen únicamente de la palabra clave, los parámetros, el número y las clases de los objetos y el nivel en que está escrita la sentencia, no del significado del circuito. Por ejemplo, `is` o `are` sigue al número de objetos; `the x of` se escribe cuando solo se da un eje; la redacción de los candidatos de una selección sigue al ámbito y a la clase seleccionada (Anexo B). Así es como se generan las sentencias; no restringe a los usuarios.
- **Sin palabras que parezcan significar algo que la sentencia no dice.** Palabras como `must`, `required` y `only` no se escriben, y una palabra de dirección se escribe solo cuando el parámetro tiene esa dirección. Los lectores confían en las palabras de una frase, y las guías de escritura deberían advertírselo.
- **Glosas.** Es posible que los lectores no conozcan los códigos de orientación y los ejes. Después de la palabra clave, `, i.e., …` puede decir qué significa el propio código, como en `#M2 @ORIENT("MX"), i.e., flipped upside down`.
- **ORIENT-RELATION y DISTANCE**, que tienen dos ranuras, se leen `from A to B`.
- **Las selecciones** siempre se leen como una elección entre candidatos: `among <candidates>, @SELECT-…`. Cuando el ámbito es `#`, los candidatos son `the <kind> in # only`, y cuando la sentencia alcanza los niveles inferiores, `the <kind> at each level in # and below`, lo que indica que la regla se evalúa en cada nivel (10.1, 10.4). Cuando no puede determinarse la clase, la frase se lee `for X,`.
- **AS y ATTACH.** AS se usa cuando los objetos juntos forman la entidad: `#M1(left), #M2(right) and #M5(tail) are treated @AS $DIFFERENTIAL_PAIR_N`. ATTACH se usa cuando la entidad se añade a los objetos: `$GROUND is @ATTACH to &vss` (9.4).
- **Tuberías.** El segmento siguiente hace referencia al resultado del anterior. El inglés usa `it` (después de AS, ATTACH o un único objeto) o `they` (después de una selección). Un nombre en una tubería va precedido de `named`, como en `… => named %pout`; sigue siendo un nombramiento (4.2 punto 3).
- **Nombramiento.** El nombre de una sola sentencia se escribe al comienzo (4.3). Cuando se nombra una sentencia AS, el inglés omite `are`: `%dp = #M1(left), #M2(right) and #M5(tail) treated @AS $DIFFERENTIAL_PAIR_N`.
- **Nombres de conjuntos.** Un nombre que hace referencia a un conjunto debería terminar en `_SET` para los nombres `$` y `_set` para los nombres `%`, como `$GROUND_FILL_SET` y `%bias_set`; el paquete estándar sigue esta convención.
- **Sin `each`.** Palabras como `each` no se escriben. Que un conjunto se aplique en bloque o miembro a miembro depende de la lista de roles de la definición, y las sentencias estándar no indican cuál de los dos casos es.
- **Otros idiomas.** Este estándar da las sentencias en inglés. Las sentencias en otro idioma utilizan los alias de un paquete de alias de ese idioma, como `alias-zh` para el chino, y siguen su documentación (16.5); después de un signo de puntuación de ese idioma, sigue haciendo falta un espacio antes de `@` (2.3).

La sentencia estándar de cada palabra clave se enumera en el Anexo B.

<!-- translated from en/ch05.md sha256:53b51cde1141 -->

## 5 Objetos y sentencias

### 5.1 Objetos

- Toda sentencia con una palabra clave crea un objeto RNL nuevo, compuesto por un manejador y hechos. Un `{…}` también crea un objeto nuevo, un conjunto (5.2).
- Una referencia (`#M1`, `&vss`, `%x`, `$X`, un terminal o un subobjeto) no crea ningún objeto; toma un manejador existente. Los lugares de miembros que dan varios miembros crean un conjunto nuevo (5.2.3). Un `#M1` aislado es también una sentencia: da M1 sin hechos. Un `#` aislado da este nivel sin hechos.
- Un manejador es la entidad descrita: una instancia, red o terminal del anfitrión, o el nivel; o un punto, cable, conjunto, uso, restricción, etc., creados por RNL. Un manejador nunca es otro objeto RNL.
- El objeto de una sentencia o bien tiene un manejador nuevo (la sentencia crea, como WIRE, los gráficos, `{…}`, las selecciones, AS, ATTACH y las restricciones), o bien conserva el manejador de su entrada y le añade hechos nuevos (la sentencia modifica, como PLACE, ORIENT, SET-SYMBOL, SET-STYLE, GROUP y CURRENT; ADD-TO y REMOVE-FROM conservan el manejador del conjunto destino). La firma indica cuál de los dos casos es (5.3); WIRE crea cuando recibe una red y modifica cuando recibe un cable (13.1).
- Dos sentencias con el mismo contenido son dos objetos, como dos `#M1 @PLACE(1, 2)`. Sus hechos no se contradicen entre sí y surten efecto juntos.
- Un hecho se consigna una sola vez, en la sentencia que lo crea (5.4). Cuando el nivel surte efecto, todos los hechos sobre un mismo manejador se combinan; las contradicciones se tratan según indica 1.5 punto 3.
- Las propiedades son hechos: posición, orientación, símbolo y estilo son hechos sobre el objeto, y no existen campos de propiedad que sobrescribir. Cuando el nivel surte efecto, los hechos sobre un mismo manejador se combinan en los valores finales: la x y la y de PLACE como hechos geométricos (5.8, 8.5), símbolos y estilos mediante las colas de apariencia (8.3).
- La clase de un manejador se deduce de su procedencia: los objetos del anfitrión la obtienen del anfitrión, los objetos RNL de la sentencia que los crea. La firma indica qué clases admite cada ranura (Anexo A); una descripción que recibe un objeto de clase incorrecta falla. Lo que puede comprobarse al leer el texto se comprueba entonces; lo que solo se conoce después (si un uso tiene símbolo, los roles de una definición) se comprueba cuando el nivel surte efecto (7.1).

Clases de objetos:

| Clase | Creado por | Dibujado |
|---|---|---|
| nivel | el anfitrión; `#` | no |
| instancia | el anfitrión; `#M1` | con su símbolo |
| terminal | el anfitrión; `#M1:d`, `#:inp` (6.4). Los usos no tienen terminales | en los pines que lo vinculan (9.2.4) |
| pin | un PIN del símbolo de un objeto dibujado (9.2.4); `#X1::vdd_t` cuando el pin tiene nombre | como un punto del símbolo |
| red | el anfitrión; `&vss` | no; los cables y etiquetas son objetos propios |
| punto | `(x, y)` | no; un punto de unión es `$JUNCTION` aplicado a un punto (13.2) |
| cable | WIRE dada una red; un cable cuya ranura de red recibe un conjunto es un bus. Dado un cable, WIRE lo reescribe (13.1) | sí |
| etiqueta | un TEXT adjuntado a un objeto; con clave es el subobjeto `object::key` de ese objeto (13.3) | sí |
| agrupación | BUNDLE (11.8) | una vez, en lugar de sus miembros |
| anotación | un gráfico en la escena (LINE, RECTANGLE, CIRCLE, ELLIPSE, ARC, BEZIER), un TEXT adjuntado a un punto (13.4), o un símbolo dibujado por ANNOTATION (13.5) | sí |
| uso | AS, ATTACH (9.4) | solo cuando tiene símbolo; entonces es un componente virtual (capítulo 12) |
| restricción | ALIGN, ORDER, DISTRIBUTE, DISTANCE, ORIENT-RELATION, TABLE (capítulo 14) | no |
| conjunto | `{…}`, selecciones, UNION, el resultado de generar uno por uno (5.2, capítulo 10) | no por sí mismo; sus miembros se dibujan por separado |
| definición | DEFINE-SYMBOL, DEFINE-STYLE, DEFINE (capítulo 9) | no; actúa únicamente donde se utiliza |

De estas clases, instancia, terminal, pin, red, punto, cable, etiqueta, agrupación, anotación (annotation) y uso tienen nombres de clase (5.3), y también lo tiene `text`, que abarca todo TEXT, ya sea una etiqueta o una anotación; el Anexo A indica cómo nombran las firmas a las demás.

También hay subobjetos (6.5), cuyo manejador es un objeto padre y un nombre, y objetos fallidos (capítulo 7).

### 5.2 Conjuntos

- `{…}` crea un objeto conjunto. Sus miembros están ordenados. Un conjunto puede nombrarse (`%pair_set = {#M1 #M2}`), y otras sentencias pueden añadir miembros a un conjunto nombrado o quitárselos (5.2.2).
- Un conjunto conserva los miembros repetidos y el orden en que se escriben, como `{#M1 #M2 #M3 #M1}`. Si dos miembros son el mismo miembro se decide por manejador: dos `{…}` escritos por separado son dos manejadores y no son el mismo miembro, aun cuando su contenido sea igual.
- **Una referencia se despliega un nivel**: una referencia en un conjunto que hace referencia a un conjunto coloca los miembros de ese conjunto en su lugar, de modo que `{%a_set %b_set}` son los miembros de a_set seguidos de los de b_set. Lo mismo rige para una sentencia cuyo resultado es un conjunto, como una selección, y para la lista de referencias a la que se expande un subíndice, como `&D<7:0>` (6.11). Para eliminar duplicados, escríbase `%a_set %b_set @UNION` (5.2.2).
- **Un `{…}` escrito explícitamente se conserva**: un `{…}` escrito directamente en un conjunto se conserva como un miembro, de modo que `{{%a_set} {%b_set}}` es un conjunto de dos conjuntos.
- **Los objetos sueltos también se despliegan**: cuando una palabra clave con una sola ranura recibe varios objetos escritos sin llaves, se reúnen exactamente como si se hubieran escrito entre `{…}` (5.2.1 punto 3), de modo que un conjunto entre ellos se despliega. Tras `%bias_set = {#R1 #R2 #R3}` y un texto `%lbl`:
  - `%bias_set %lbl @ALIGN("x")` es `{%bias_set %lbl} @ALIGN("x")`;
  - un conjunto que es el único objeto se usa como tal: `%bias_set @ALIGN("x")` alinea R1, R2 y R3.
- **Elementos**: un conjunto también conserva sus elementos tal como se escriben, en orden: un objeto escrito en él, un `{…}` escrito explícitamente o una referencia a un conjunto. Donde este estándar cuenta elementos (las listas de miembros de ALIGN, ORDER y DISTRIBUTE, 14.1, los miembros de ASSERT-TOPOLOGY, 15.4, y la división de 5.2.1 punto 2), un objeto es un elemento, y un `{…}` o una referencia a un conjunto como elemento representa a cada uno de sus miembros más internos en ese elemento; en ASSERT-TOPOLOGY representa una caja negra formada por ellos. ADD-TO añade un elemento al final, y REMOVE-FROM elimina los objetos de cada elemento (5.2.2); un conjunto creado por una selección, por UNION o al generar uno por uno tiene un elemento por cada miembro, y lo mismo la lista a la que se expande un subíndice (6.11) y el conjunto que toman los lugares de miembros (5.2.3). En cualquier otro lugar, un conjunto designa a sus miembros, desplegados un nivel como arriba. Así, `{%bias_set %lbl} @ALIGN("x")` tiene dos elementos, R1–R3 como el primero y el texto como el segundo, y da a cada uno de R1, R2 y R3 la x del texto (14.1).
- El contenido de un conjunto se calcula en la fase que lo utiliza, como ocurre con una selección: su contenido inicial, y las selecciones de las sentencias que añaden o quitan miembros, se evalúan sobre el estado previo al comienzo de esa fase, y las adiciones y eliminaciones se aplican en el orden de escritura (su posición, 8.5). Así, un uso escrito antes también ve todas las adiciones y eliminaciones (1.5 punto 1), y un mismo conjunto puede diferir entre fases: las reglas que generan objetos lo ven sin los objetos generados, las reglas de estilo lo ven con ellos (8.2).

  Nota: El contenido no se calcula una sola vez en un único paso, porque entonces nombrar una selección cambiaría su resultado: `%g_set = # @SELECT-USAGE $GROUND; %g_set @SET-STYLE $grey` omitiría las tierras generadas, que la misma selección escrita sin nombre sí encuentra; un nombre es solo otra forma de hacer referencia a un objeto (1.5 punto 5).

#### 5.2.1 Objetos y ranuras

Los objetos escritos en una sentencia se emparejan con las ranuras de la firma como sigue.

1. **Marcados con roles**: un objeto marcado con un rol entra entero en ese rol y no se divide. En una misma sentencia o se marcan todos los objetos o ninguno.
2. **Dividir primero un nivel**: cuando no se marcan roles y toda la sentencia tiene un único objeto, que es un conjunto, ese conjunto se divide primero un nivel, en sus elementos (5.2). TABLE es la excepción: su ranura toma el conjunto tal como está escrito, como la lista externa de la tabla (14.11). Una definición y el resultado pasado por una tubería cuentan como objetos. Así, `{#M1 #M2} @DISTANCE(…)` es `#M1 #M2 @DISTANCE(…)`, mientras que `{#M1 #M2} @SET-SYMBOL $X` son dos objetos (el conjunto y la definición) y no se divide.
3. **Emparejamiento según la firma**:
   - Una palabra clave con varias ranuras (una definición también ocupa una ranura): el número de objetos debe ser igual al número de ranuras, y se emparejan uno a uno en orden; de lo contrario la sentencia falla. Para AS y ATTACH, los roles de conjunto al final de la lista de roles pueden quedar sin objetos (9.4).
   - Una palabra clave con una sola ranura (como PLACE, ORIENT, ALIGN, ORDER, DISTRIBUTE, GROUP, las selecciones distintas de SELECT-USAGE y SELECT-ANCESTOR-USAGE, que también toman una definición (5.6), y las palabras clave de definición, cuya ranura es el cuerpo): un único objeto entra en la ranura tal cual; varios objetos se reúnen en un conjunto, en el cual los conjuntos referenciados se despliegan un nivel y cada `{…}` escrito explícitamente se conserva como un miembro. Cuando los objetos son los elementos del único conjunto, dividido por el punto 2, la ranura recibe ese propio conjunto.
   - Cuando el único objeto es un conjunto vacío:
     - Una ranura que toma un único objeto realiza su trabajo cero veces: nada ocurre y nada falla. Un nombre se vincula al conjunto vacío como de costumbre, y las sentencias que usan el nombre no hacen nada.
     - Un conjunto vacío escrito explícitamente para una ranura que toma un único objeto produce una advertencia, como `{} @PLACE(1, 2)`. Un conjunto vacío procedente de una selección o de una regla no produce ninguna: una regla con `recursive=true` se evalúa también en cada nivel inferior, y un nivel sin objetos de esa clase es normal. Cuando una selección escrita por el usuario no selecciona nada en ninguno de los niveles donde se evalúa, una implementación puede emitir una advertencia; probablemente la condición sea errónea.
     - Una ranura que toma un conjunto acepta un conjunto vacío, como el cuerpo vacío de `@DEFINE(roles=["a"]) {}`, o una restricción sin miembros, que se cumple trivialmente. Las excepciones son BUNDLE y la ranura de red de WIRE: crean un objeto nuevo a partir del conjunto, el cual no puede crearse de la nada, por lo que la sentencia falla (11.8, 13.1).
     - Una sentencia sin ningún objeto, como un `@PLACE(1, 2)` aislado, no coincide con la firma y falla.
4. **Un conjunto en una ranura**:
   - Una ranura que toma un único objeto realiza su trabajo una vez por cada miembro. Cada miembro es una descripción propia, y el resultado es el conjunto de los resultados. Un miembro que es a su vez un conjunto se divide de nuevo y se ejecuta miembro por miembro. La única excepción es PRIORITY: dado un conjunto marcado con GROUP, actúa sobre el propio grupo (11.5, 14.8).
   - Una ranura que toma un conjunto lo toma entero, y toma un único objeto como un conjunto de ese único objeto: `#M3 @ROUTE-OBJECT` es `{#M3} @ROUTE-OBJECT`.
   - Cualquiera de las dos ranuras de DISTANCE toma un objeto o un conjunto; un conjunto representa a cada uno de sus miembros, y DISTANCE relaciona cada miembro de una ranura con cada miembro de la otra, una restricción por cada par (14.5). Esta es la única palabra clave cuyas dos ranuras pueden recibir conjuntos a la vez.
   - En una misma sentencia, como máximo una ranura que tome un único objeto puede recibir un conjunto de más de un miembro. Cuando más de una ranura de ese tipo lo hace, la sentencia falla: emparejar los miembros en orden y tomar todas las combinaciones tendrían ambos sentido, por lo que el significado no sería único.

| Escrito | Resultado |
|---|---|
| `{#M1 #M2} @SET-SYMBOL $X` | Dos objetos, sin dividir. La ranura de instancia recibe un conjunto: M1 y M2 reciben cada uno el símbolo |
| `#M1 #M2 @SET-SYMBOL $X` | Tres objetos para dos ranuras: la sentencia falla |
| `{#M1 #M2} @DISTANCE(…)`, `%pair_set @DISTANCE(…)` | El único objeto es un conjunto y se divide: de M1 a M2 |
| `%a_set %b_set @DISTANCE(…)` | Dos objetos, cada uno un conjunto: de cada miembro de a_set a cada miembro de b_set (14.5) |
| `{#M1 #M2} @ALIGN("y")`, `#M1 #M2 @ALIGN("y")` | ALIGN tiene una ranura, que recibe {M1 M2} |
| `%bias_set %lbl @ALIGN("x")`, con `%bias_set = {#R1 #R2 #R3}` | Los objetos sueltos se reúnen como `{%bias_set %lbl}`: dos elementos, R1–R3 como el primero y el texto como el segundo; R1, R2 y R3 reciben cada uno la x del texto (14.1) |
| `{%bias_set} %lbl @ALIGN("x")` | Los mismos dos elementos: R1, R2 y R3 reciben cada uno la x del texto |
| `{%bias_set} @ALIGN("x")`, `{{#M1 #M2 #M3}} @ALIGN("y")` | El único objeto es un conjunto y se divide un nivel en su único elemento (punto 2); ese elemento es un conjunto y entra en la ranura tal cual: alinea R1, R2 y R3, o M1, M2 y M3 |
| `%pair_set @PLACE(10, 20)` | La ranura recibe pair_set mismo y lo coloca miembro por miembro: ambos miembros en (10, 20). Un conjunto no tiene posición propia (14.1) |
| `%a_set %b_set @PRIORITY("disabled")`, ambos conjuntos con GROUP | Los objetos sueltos se despliegan como en `{…}`: PRIORITY recibe los miembros y falla sobre ellos (clase incorrecta). Para desactivar dos grupos, escríbase un PRIORITY para cada uno (11.5, 14.8) |
| `{#M1 #M2} @SET-SYMBOL $X => @ALIGN("y")` | El resultado de la tubería es el único objeto y entra en ALIGN: alinea M1 y M2 |
| `{#M1 #M2} @SET-SYMBOL $X => #M3 @DISTANCE(…)` | Dos objetos, sin dividir. La ranura `from` recibe el conjunto: de M1 a M3 y de M2 a M3 (14.5) |
| `# @SELECT-NET(…) => $PULL_DOWN @ATTACH &vss` | Tres objetos para tres ranuras (dos roles y la definición). El primer rol es un rol único, por lo que la definición se aplica una vez por cada red seleccionada, cada vez con vss en el segundo rol |
| `# @SELECT-INSTANCE(type="nmos") => @ORIENT("MY")` | Reflejados uno a uno; sin ningún NMOS, se ejecuta cero veces y nada ocurre |
| `# @SELECT-INSTANCE(name="Q*") => @SET-SYMBOL $NPN` | Dos objetos, sin dividir; sin ningún BJT, se ejecuta cero veces y nada ocurre |
| `#R1 {#R2 #R3} #R4 @AS $CHAIN_ROW` | Cuatro objetos para cuatro ranuras (tres roles y la definición); el rol de conjunto `chain` recibe {R2 R3} (9.4) |
| `&D<7:0> @WIRE {…}` | Dos objetos, sin dividir. La ranura de red toma entera la lista de redes: un único bus (13.1) |
| `#XI0<3:0> @ALIGN("y")` | El único objeto es la lista a la que se expande un subíndice, y se divide: alinea las cuatro instancias (6.11) |

- Una restricción con demasiado pocos miembros se cumple trivialmente y no falla, como un ALIGN con un solo miembro (capítulo 14).
- Un conjunto no tiene posición propia (14.1); su `::position` es el conjunto de las posiciones de sus miembros (6.5). Un uso sin símbolo en una restricción representa a sus miembros (14.1). Para relacionar un grupo de objetos con otros como un solo lugar, háganse miembros de un uso cuyo cuerpo los vincule al punto propio del uso (9.4), y restrínjase `%u::position`; para mantener un conjunto a un lado de otros objetos, pásese a DISTANCE (14.5).
- Los roles de una definición son roles únicos o roles de conjunto (9.4). Un rol escrito `"{name}"` en la lista de roles es un rol de conjunto y toma un conjunto entero; cualquier otro rol es un rol único y, dado un conjunto, la definición se aplica una vez por cada miembro.

#### 5.2.2 Añadir y eliminar miembros, y duplicados

- `X @ADD-TO S` añade X al final del conjunto S (los miembros de X cuando X es un conjunto); `X @REMOVE-FROM S` elimina de S toda ocurrencia de esos miembros. El destino se escribe al final, para poder pasar una selección mediante una tubería: `# @SELECT-NET(name="vss") => @ADD-TO %gnd_set`.
- El destino debe ser un conjunto; no se envuelve en uno. Tras `%tail = #M5`, `#M3 @ADD-TO %tail` falla, porque M5 no es un conjunto.
- Las adiciones y eliminaciones se aplican en orden de posición (1.5 punto 2). Un conjunto evaluado en un nivel contiene únicamente los miembros de ese nivel (10.3).
- `%a_set %b_set @UNION` crea un conjunto nuevo sin duplicados, en el orden de primera aparición; si dos miembros son iguales se decide por manejador.
- Para crear un conjunto nuevo dejando intactos los antiguos: la concatenación es `{%a_set %b_set}`; la unión `%a_set %b_set @UNION`; la diferencia `%c_set = {%a_set}; %b_set @REMOVE-FROM %c_set`.
- Un conjunto depende de los conjuntos escritos en la sentencia que lo crea (dentro de sus llaves, en UNION, o como ámbito de una selección) y de los conjuntos que ADD-TO y REMOVE-FROM le añaden o le quitan. Las dependencias que forman un ciclo son un error, con o sin eliminación: tomando las sentencias en orden de posición (8.5), falla la que con su dependencia cierra el ciclo, y su diagnóstico enumera el ciclo entero; las demás surten efecto como de costumbre. Así, `%a_set @ADD-TO %b_set; %b_set @ADD-TO %a_set` falla en la segunda sentencia, y `%b_set = {%a_set}; %b_set @REMOVE-FROM %a_set`, cuya eliminación no tendría resultado definido, falla en la eliminación.

#### 5.2.3 Lugares de miembros

```rnl
%row_set = {#M1 #M2 #M3 #M4};
%row_set{1} @PLACE(0, 0);                                         /* M1 */
%row_set{1} %row_set{end} @DISTANCE("left-to-right", exact=12);   /* from M1 to M4 */
%row_set{2:end} @ALIGN("y")                                       /* M2, M3 and M4 */
```

- Una referencia a un conjunto seguida directamente de `{…}` toma miembros del conjunto por sus lugares, como `%a{1}`, `%a{end}`, `%a{end-1}` y `%a{2:end}`. Se escribe después de nombres `%` y `$` vinculados a conjuntos, y después del marcador de un rol de conjunto en el cuerpo de una definición general, como `?ref{1}` (9.4).
- **Forma**: dentro de las llaves, elementos separados por `,`; cada elemento es `i`, `a:b` o `a:b:s`, y las formas pueden mezclarse, como en los subíndices (6.11). Un lugar es un entero positivo, `end` o `end-k`, donde k es un entero sin signo; s es un entero sin signo de al menos 1. Dentro de las llaves pueden escribirse espacios en blanco y comentarios.
- **Conteo**: los lugares se cuentan desde 1 en el orden de los miembros del conjunto, después de todas sus adiciones y eliminaciones (5.2), incluidas las escritas después de la referencia, de modo que `%a{2}` después de `%a = {#M1}; #M2 @ADD-TO %a` es M2; los miembros se despliegan un nivel según indica 5.2, no se cuentan por los elementos: después de `%a = {%b_set %lbl}`, `%a{2}` es el segundo miembro de b_set, o el texto cuando b_set tiene un miembro. `end` es el último miembro. Los elementos se expanden en el orden escrito y se unen; las repeticiones se conservan. `a:b` va de a hasta b, hacia atrás cuando a es mayor que b, y `a:b:s` en pasos de s sin sobrepasar b.
- **Uno o un conjunto**: un único elemento que sea un solo lugar da el miembro mismo. Cualquier otra cosa da un conjunto nuevo con los miembros en esos lugares, en ese orden: un objeto conjunto nuevo, el único caso en que una referencia crea un objeto (5.1). Un miembro que es a su vez un conjunto (un `{…}` escrito explícitamente, 5.2) se toma tal cual.
- **Más allá de los miembros**: un lugar que el conjunto no tiene no da nada: un único lugar así da un conjunto vacío, y en una lista se omite. Esto no es un error, pues puede desconocerse cuántos miembros tiene un conjunto al escribir la sentencia (como en un ALIGN de un miembro, 14.2).
- **Solo lugares**: las llaves inmediatamente después de una referencia contienen solo lugares. `%a{#b}` es un error de sintaxis; para escribir una referencia y después un conjunto, sepárense con espacio en blanco, como en `%a {#b}` (3.5).
- **Errores**:
  - Los lugares de un objeto que no es un conjunto, como una instancia, fallan (clase incorrecta).
  - Un uso no es un conjunto: sus miembros se toman por rol, como `%dp1::left` (6.5), y los lugares después de un uso son un error.
  - Los lugares del resultado de una selección son un error, tanto si la selección se escribe allí como si se vincula al nombre, y lo mismo los lugares de cualquier conjunto que dependa de una selección, a través de sus elementos, UNION, ADD-TO o REMOVE-FROM (10.4), como `%a{1}` después de `%a = {%s}` siendo `%s` una selección: sus miembros se conocen únicamente cuando la fase que los usa los evalúa (5.2, 8.2).

#### 5.2.4 Objetos dados en lugar de otra clase

Cuando una ranura toma una clase y recibe un objeto de otra clase, el objeto se convierte únicamente según indica la tabla; cualquier otra discordancia es un error de clase al leer el texto (5.1, 7.1), sea cual sea el significado pretendido. Un objeto se convierte únicamente a aquello que representa sin elección: hacia su dibujo, de un terminal a su pin y de ahí a un punto, y de un objeto dibujado a su posición; hacia el circuito, de un pin al terminal que vincula, y de un pin o un terminal a su red. Para cualquier otra cosa, escríbase el subobjeto (`::position`, 6.5) o una selección (10.1).

| Dado \ la ranura toma | punto (gráficos) | punto o pin (cables) | posición (miembros de restricciones) | pin | terminal | red |
|---|---|---|---|---|---|---|
| punto | = | = | = | no | no | no |
| pin | 1 | = | 1 | = | 8 | 9 |
| terminal | 2 | 3 | 2 | 3 | = | 10 |
| instancia, agrupación | no | no | 4 | no | no | no |
| uso | no | no | 5 | no | no | no |
| texto | no | no | 6 | no | no | no |
| gráfico escrito con un centro | no | no | 7 | no | no | no |
| otro gráfico, cable | no | no | no | no | no | no |
| red | no | no | no | no | no | = |

"=" es la misma clase; "no" es un error de clase; un número es una conversión abajo. Una conversión desde o hacia un pin necesita el símbolo del objeto, por lo que se realiza una vez asignados los símbolos (8.2); los pasos anteriores ven el objeto tal como está escrito. Donde una ranura o rol toma un único objeto, una conversión así que dé un conjunto, como desde un pin de bus, hace fallar la sentencia en esa comprobación (7.1): la sentencia se creó con un único objeto y no se divide después. Un cable que recibe un pin termina en ese pin, lo que lo conecta (10.2, 13.1); no toma únicamente las coordenadas del pin.

1. **Pin a punto o posición**: la posición del pin.
2. **Terminal a punto o posición**: la posición de su pin, como en 3.
3. **Terminal a pin**: el pin que vincula al terminal en el símbolo de su objeto (9.2.4). Cuando no hay ninguno, la sentencia falla (9.2.4); cuando hay varios, es el escrito en último lugar en el símbolo. Para un terminal de una agrupación, el pin del símbolo de la agrupación (11.8).
4. **Instancia o agrupación a posición**: su origen, `::position` (6.5). Una instancia no se convierte a un punto de un cable o un gráfico: para terminar un cable o dibujar un gráfico en el origen, escríbase `#M1::position`.
5. **Uso a posición**: su propio punto (14.1). Un uso no se convierte a un punto de un cable o un gráfico: para terminar un cable o dibujar un gráfico en su punto, escríbase `%u::position` (6.5).
6. **Texto a posición**: la posición del propio texto (13.3).
7. **Gráfico escrito con un centro a posición**: el centro (13.4). Un gráfico escrito con dos puntos, LINE, ARC y BEZIER tienen varios puntos y no se convierten.
8. **Pin a terminal**: el terminal que vincula el pin (9.2.4); para un pin de bus, el conjunto de sus terminales. Un pin que no vincula ningún terminal, como un pin de un uso o un pin que solo se dibuja, no tiene ninguno, y la sentencia falla.
9. **Pin a red**: su `::net` (6.5), la red del terminal que vincula o la red que rellena el rol que vincula; para un pin de bus, el conjunto de esas redes. Un pin que no vincula nada no tiene ninguna, y la sentencia falla.
10. **Terminal a red**: su `::net` (6.5); para un terminal de una agrupación cuyos miembros están en redes distintas, el conjunto de esas redes (11.8).

- PLACE toma únicamente los objetos que enumera 11.1: un terminal o un pin se mueve solo con su objeto y no se coloca por separado (14.1).
- Un conjunto no se convierte: una ranura que toma un único objeto realiza su trabajo para cada miembro (5.2.1), y en las restricciones un lugar que contiene un conjunto representa a cada uno de sus miembros (14.1).
- El nodo vacío, `&` solo (6.3), tampoco se convierte: un rol de clase `"net"` y la ranura de red de WIRE lo toman tal cual, y cualquier otro lugar que toma una red lo toma como un conjunto vacío de redes.
- Una red no se convierte a nada: no tiene posición ni pin, y no representa ningún terminal individual. Un objeto tampoco se convierte a varios objetos de otra clase: una instancia no se convierte en sus terminales. Para esto, escríbase una selección, como `#M1 @SELECT-TERMINAL` o `&en @SELECT-PIN` (10.1).
- No son conversiones, y se explican en otro lugar: a qué se adjunta un TEXT (13.3), cómo el ámbito de una selección conduce a la clase seleccionada (10.1) y WIRE dado un cable (13.1).

### 5.3 Sentencias y firmas

- Una sentencia se compone de como máximo una palabra clave, algunos objetos y una lista de parámetros.
- Toda palabra clave tiene una firma, que declara:
  - el número de objetos, y el nombre de cada ranura y las clases de objetos que toma. Para AS y ATTACH el número es la longitud de la lista de roles de la definición más uno para la definición misma (escrita al final para AS, al principio para ATTACH), y las clases son las declaradas para los roles (9.4);
  - los parámetros: sus nombres, tipos, si son obligatorios, sus valores permitidos y el orden en que pueden darse por posición;
  - la salida: un objeto nuevo, el manejador de la entrada, un subobjeto, una definición o ningún manejador; WIRE da un cable nuevo cuando recibe una red, y el cable recibido cuando recibe un cable (13.1);
  - cómo se consume cada entrada (5.4), y si es estricta o tolerante ante fallos (7.3);
  - dónde puede escribirse la palabra clave (véase la lista de palabras abajo).
- El número de objetos es fijo; un número variable de objetos se da como un conjunto. No hay ranuras opcionales, con estas excepciones: un rol de conjunto de AS y ATTACH que no se rellena es un conjunto vacío (9.4); la definición de SELECT-USAGE puede omitirse (10.1); el cuerpo de USE-PACKAGE puede omitirse (16.2). Una definición ocupa también una ranura.
- Los objetos se emparejan con las ranuras por rol o por orden, todos marcados o ninguno; un rol puede marcarse una sola vez. El objeto pasado por una tubería es el primer objeto; AS y ATTACH lo emparejan por rol, y no ocupa la ranura de la definición (5.5). Cómo se emparejan los objetos con las ranuras se indica en 5.2.1.
- Los objetos escritos en una sentencia pueden marcarse con roles, cualquiera que sea la palabra clave. El nombre de rol es el nombre de ranura en la firma: `#M5:3(to) #M1:3(from) @DISTANCE("left-to-right", min=1)` es `#M1:3 #M5:3 @DISTANCE("left-to-right", min=1)` escrito en orden. Un nombre de rol que no sea un nombre de ranura de la palabra clave hace fallar la sentencia; no se ignora (Principios 2.3, 2.8). Los objetos de una palabra clave con una sola ranura también pueden marcarse, pero la marca no añade nada.
- Los parámetros de AS y ATTACH son los de la lista de parámetros de la definición (9.4), más un parámetro estándar, `priority` (14.8); ningún parámetro de una definición puede llamarse `priority`.
- **Lista estándar de palabras**: cada versión del lenguaje publica una lista legible por máquina de sus palabras estándar, que forma parte del estándar. Qué toma cada palabra clave, y qué parámetros tiene, se indica únicamente en la lista; el texto principal da su significado. El Anexo A se genera a partir de la lista, y una contradicción entre ambos es una errata. Las comprobaciones de un intérprete, la documentación, el completado en editores, el esqueleto de las pruebas de conformidad y las referencias de escritura para LLM se generan a partir de la lista. Para la versión 0.4, la lista se publica como `rnl-words-0.4.json`.
  - La lista contiene todas las palabras estándar: las palabras clave con sus firmas, nombres de ranura, nombres y valores de parámetro, los campos del encabezado del registro, nombres de clase, nombres de subobjeto, los nombres de atributo que lee `${…}`, propiedades de estilo, y `true` y `false`. Los alias (16.5) se declaran para palabras de la lista, y las comprobaciones de versión y sugerencias del nombre más cercano buscan las palabras en ella.
  - La lista es un archivo JSON, y el esquema JSON `rnl-words-0.4.schema.json` describe la estructura de la propia lista. Las herramientas y los responsables del estándar la leen y escriben, no quienes describen circuitos. Cada palabra tiene `since`, la versión que la añadió; las palabras reservadas están marcadas (la versión 0.4 no reserva ninguna palabra clave). Bajo `removed`, la lista nombra las palabras estándar de versiones anteriores que esta versión ya no tiene, con la sección que indica qué las reemplaza, para que un diagnóstico pueda indicarlo. Una palabra que no está en la versión que declara un archivo se trata como palabra desconocida, y el diagnóstico indica qué versión es necesaria (16.1).
  - La lista registra, para cada palabra clave:
    - cada ranura: su nombre, las clases que toma, si toma un único objeto o un conjunto (5.2.1), y cómo se consume (5.4), lo que decide si es estricta o tolerante (7.3); la ranura de la definición; para AS y ATTACH, las ranuras siguen la lista de roles de la definición;
    - cada parámetro: su nombre; su tipo (número, cadena, valor de verdad o matriz con el tipo de sus elementos; donde los valores son fijos, los valores); si es obligatorio; su valor por defecto; el orden de los parámetros que pueden darse por posición (los demás se dan solo por nombre); y condiciones sobre combinaciones, como al menos uno entre `x` e `y` de PLACE, o solo una entre la forma de centro y la forma de esquinas de RECTANGLE;
    - la salida: un objeto nuevo y su clase, la entrada pasada adelante, el destino pasado adelante (ADD-TO), una definición, un subobjeto (TEXT con `key=`), o ninguna; para WIRE, también la clase de objeto que reescribe cuando recibe uno (13.1);
    - dónde puede escribirse: en un nivel; únicamente en un cuerpo de símbolo (PIN); únicamente en el registro de encabezado (como VERSION, ALIAS, DEVICE); únicamente en el registro de encabezado de un archivo de paquete (PACKAGE);
    - cómo se sustituyen sus hechos (5.8): por clave, con la clave; como hechos geométricos (8.5); mediante las colas de apariencia (8.3); o sumándose;
    - su forma estándar y sentencia estándar (4.3, 4.5, Anexo B);
    - una descripción de una línea y la sección del texto principal.
  - No figura en la lista: el significado (qué hechos crea una palabra clave, en qué fase, cómo entran en conflicto) se da en el texto principal. La firma de una definición general en un paquete es su lista de roles y su lista de parámetros (9.4).

  <!-- JSON excerpt: added by the integrator -->

- **Nombres de ranura de palabras clave integradas**: cada ranura tiene un nombre, que es el nombre de rol que puede marcarse. Los nombres de ranura son palabras estándar y llevan versión como las palabras clave: renombrar uno es un cambio incompatible; las ranuras de una misma palabra clave tienen nombres distintos. La ranura de una definición también tiene un nombre, utilizado en la lista y en diagnósticos, pero no puede marcarse (4.2 punto 7). Existe un único conjunto de nombres de clase, usado tanto en firmas como en las declaraciones de rol de las definiciones (9.4), `instance`, `terminal`, `pin`, `net`, `point`, `wire`, `usage`, `bundle`, `text`, `annotation` y `label` (Anexo A).

### 5.4 Modos de consumir la entrada

| Modo | Palabras clave | Hechos de la entrada | Fallos (por defecto) | Nombres |
|---|---|---|---|---|
| absorber | la mayoría de palabras clave | llevados a la salida | estricto | los nombres en un conjunto anidado valen solo en ese conjunto |
| encerrar | DEFINE, DEFINE-SYMBOL, DEFINE-STYLE | pasan a ser el contenido de la definición y no se pasan adelante | tolerante | locales; renombrados para cada aplicación, de modo que cada uso tenga los suyos (9.4) |
| consignar | el registro, `@RNL` | fusionados en el nivel en orden | tolerante | publicados en el nivel y compartidos por todos sus registros; pueden usarse antes de vincularse |

- Lo que produce un miembro son todos los hechos creados por sus segmentos y por los conjuntos anidados en él, no solo el manejador del final. Se consignan con el miembro de nivel superior que los contiene.
- Toda sentencia de un registro se consigna, tenga nombre o no. Lo que haga entonces depende del objeto: una descripción en la escena se dibuja o participa en la resolución; una definición pasa a estar disponible y no dibuja nada por sí sola; las referencias, y los conjuntos compuestos solo de referencias, no tienen efecto. Las sentencias en un `{…}` simple se consignan con el miembro que lo contiene, de modo que la línea de `{ @LINE {…}}` se dibuja (13.4).
- Las sentencias en el cuerpo de una definición no se consignan; pasan a ser el contenido de la definición. Una definición general se expande únicamente cuando se aplica (9.4).
- Un USE-PACKAGE con cuerpo toma el cuerpo también encerrándolo: las sentencias del cuerpo son el contenido del paquete y no se consignan en el nivel. Cómo se ven los nombres de un paquete se indica en 16.2.

### 5.5 Tuberías y anidamiento

- `a => @K(…)` convierte el resultado de `a` en el primer objeto de K; AS y ATTACH lo ponen en el primer rol, no en la ranura de la definición. Un resultado que sea un conjunto sigue siendo un único objeto (5.2.1). El segmento posterior a `=>` toma este objeto únicamente por posición y no puede marcarlo con un rol.
- Una tubería no se ejecuta paso a paso. Se expande en varias descripciones enlazadas por manejadores: cada segmento describe el manejador que resulta del segmento anterior, y un segmento que trabaja miembro por miembro se expande en descripciones paralelas (1.5 punto 1).
- **El orden de los segmentos.** Cada segmento tiene un lugar propio, y un segmento posterior cuenta como escrito después de uno anterior allí donde el lugar de escritura importa (1.5 punto 2): en la sustitución por clave (8.4), en las colas de apariencia (8.3), en el orden de resolución (8.5), en el orden de dibujo (11.6) y para las reescrituras de un cable (13.1). Así, en `#M1 @SET-SYMBOL $A => @SET-SYMBOL $B`, M1 se dibuja con `$B`.
- **Un fallo alcanza solo a las descripciones que dependen de él.** Una descripción falla únicamente por una de dos razones: no puede entenderse o se contradice a sí misma; o bien un manejador que utiliza debería haber sido creado por una descripción que falló.
  - En `$GROUND @ATTACH &vss => @PLACE("20", 38) => @ORIENT("R90") => %g1` falla solo PLACE (una cadena donde PLACE toma un número, 5.3); la tierra se crea de todos modos, y ORIENT sigue surtiendo efecto.
  - En `#XI0<3:0> @BUNDLE(bad=1) => @PLACE(10, 20) => %s` la agrupación no se crea (BUNDLE no tiene parámetro `bad`), por lo que PLACE carece de objeto que describir y también falla.
- Qué vincula un nombre: `%x =` delante vincula el último manejador de toda la tubería; `=> %x` vincula el resultado del segmento a su izquierda, que es el último manejador cuando se escribe al final, y el resultado de ese segmento cuando se escribe en medio, como `#M1 @AS $A => %a => @AS $B => %b`. PLACE, ORIENT, SET-SYMBOL, SET-STYLE y GROUP conservan el manejador de su entrada, por lo que en `$GROUND @ATTACH &vss => @PLACE(24, 38) => %g0` el nombre `%g0` es el uso de esta aplicación, es decir, esta tierra. AS y ATTACH crean manejadores nuevos: el segundo segmento de `#M1 @AS $A => @AS $B` aplica `$B` al uso creado por el primer segmento (anidamiento), no a M1. Para aplicar dos definiciones a M1, escríbanse dos sentencias. Cuando falla el segmento a la izquierda de un nombre: si habría conservado el manejador de su entrada, el nombre sigue haciendo referencia a ese manejador (`%g1` arriba es la tierra); si habría creado un objeto nuevo, el nombre hace referencia al objeto fallido (`%s` arriba).
- Un objeto puede escribirse como un conjunto que contiene sentencias (anidamiento). Los nombres vinculados en un conjunto anidado valen solo en ese conjunto. Cada sentencia anidada es una descripción propia: en `{%a = #M1 @PLACE(1, 2); #M9} @ALIGN("y")`, cuando el anfitrión no tiene M9, ALIGN falla y las coordenadas de M1 siguen surtiendo efecto.
- La forma estándar escribe las sentencias de forma plana: nombrar un objeto, luego hacer referencia a él en otras sentencias. Una sola sentencia se nombra con `%x =` delante, una tubería con `=> %x` (4.3).

### 5.6 Definiciones como objetos

Una palabra clave que utiliza una definición la toma como uno de sus objetos, y la palabra clave fija qué ranura la contiene: para ATTACH es el primer objeto escrito; para SET-SYMBOL, SET-STYLE, AS, SELECT-USAGE y SELECT-ANCESTOR-USAGE es el último objeto, y SELECT-USAGE puede omitirla (10.1).

```rnl
%ax = (_, _);
%w13 = &x @WIRE {#M1:1 #M3:1};
#M1 @SET-SYMBOL $NMOS;
%w13 @SET-STYLE $signal;
#M1(left) #M2(right) %ax(axis) @AS $MIRRORED_LAYOUT;
$GROUND @ATTACH &vss;
# @SELECT-USAGE $GROUND => @SET-SYMBOL $GROUND_SYMBOL
```

- Una definición no se marca con un rol y no cuenta para "todos marcados o ninguno". Es un objeto y ocupa una ranura, por lo que `{#M1 #M2} @SET-SYMBOL $X` son dos objetos (5.2.1).
- Como la palabra clave fija la ranura de la definición, una sentencia puede analizarse sin saber si un nombre `$` hace referencia a una definición o a un conjunto: en `$GROUND_STUB @ATTACH $GROUND_FILL_SET` el primero es una definición y el segundo un conjunto.
- Una definición no puede pasarse mediante una tubería: el objeto pasado rellena la primera ranura, y AS y ATTACH lo emparejan por rol, fuera de la ranura de la definición (5.5). Por tanto `$GROUND @ATTACH &vss` no puede escribirse `$GROUND => @ATTACH &vss`.
- Una definición puede ser una definición anónima anidada: `#M1 @SET-SYMBOL { @DEFINE-SYMBOL {…}}`.
- Una definición general puede estar escrita más tarde o en otro nivel, por lo que el número de objetos, los nombres de rol y los parámetros de AS y ATTACH se comprueban cuando el nivel surte efecto (7.1).

### 5.7 Manejadores y nombres

- El anfitrión proporciona los manejadores de los objetos del anfitrión: nombres de instancia y nombres de red. Un terminal es una instancia y un número de terminal, por lo que `#M1:1` y `#M1:d` son el mismo terminal.
- Para lo que crea RNL, el manejador procede de la sentencia que lo crea. Otra sentencia puede hacer referencia a él únicamente mediante un nombre: el nombre es la única forma de encontrarlo, salvo en el registro del solucionador, que hace referencia por su número a un objeto que no tiene nombre (5.9). Un objeto sin nombre solo puede utilizarse de otro modo dentro de su propia sentencia (en una tubería o por anidamiento).
- Un nombre nunca hace referencia a dos cosas: se vincula una sola vez en un nivel (6.6).
- El manejador de un subobjeto es su objeto padre y su nombre; no necesita nombre propio.
- El manejador de un objeto local creado al expandir una definición general es su uso y su nombre local, escrito `%c1::mid` desde fuera (9.4). Los objetos locales de un uso sin nombre se muestran en los diagnósticos únicamente por su posición; solo el registro del solucionador puede hacer referencia a ellos, a través del número del uso, como `%#5::mid` (5.9).
- Un objeto individual de un conjunto generado no tiene nombre y no puede referenciarse por separado: los objetos no se identifican por su clase y los objetos que relacionan (no hay claves naturales), y los conjuntos no ofrecen referencia por índice.

### 5.8 Claves de hechos

- Los hechos con clave se sustituyen por clave: para una misma clave, el hecho posterior surte efecto; cuando el posterior falla, se conserva el valor válido anterior.
- Los hechos geométricos son la excepción: la x y la y de PLACE (una clave para cada coordenada) y ORIENT (una clave para cada objeto, y una para cada pin, 11.2). Dos valores distintos escritos para la misma clave entran en conflicto y se resuelven según 8.5: con la misma prioridad se conserva el posterior, que es el mismo resultado que la sustitución por clave; con prioridades distintas se conserva el de mayor prioridad. PLACE cambia únicamente las coordenadas que indica; una coordenada nunca dada queda libre.
- Crear un objeto bajo un nombre ya vinculado no es sustitución por clave: un nombre se vincula una sola vez en un nivel (6.6). Para modificar un objeto creado por RNL, escríbase una sentencia que lo modifique, como `%g0 @PLACE(40, 38)`.
- Los hechos sin clave se suman: los que no entran en conflicto se cumplen juntos, y los conflictos se resuelven según 8.5.
- SET-SYMBOL y SET-STYLE no se sustituyen por clave; surten efecto mediante las colas de apariencia (8.3).
- VERSION y PACKAGE se escriben una sola vez, en el registro de encabezado (16.1, 16.2), por lo que nunca se sustituyen: un segundo es un error, falla y surte efecto el primero.
- CURRENT tiene una clave para cada terminal (15.2), y LAYER una para cada objeto (11.6).
- Los usos no tienen clave: una definición aplicada varias veces crea tantos usos, que no se sustituyen entre sí. Dos aplicaciones idénticas crean dos usos, de modo que `$GROUND @ATTACH &vss` escrito dos veces dibuja dos tierras. Los hechos escritos sobre un uso por sentencias que lo modifican, como PLACE, sí tienen clave y siguen las reglas anteriores.
- La prioridad de un objeto es un hecho con clave; el orden en que las prioridades se sustituyen entre sí se indica en 14.8.
- Un TEXT con clave (`key=`) es un texto por cada objeto adjuntado y clave, y uno posterior lo sustituye parámetro por parámetro (13.3). Un TEXT sin clave no se sustituye por clave: cada uno escrito es un texto más (13.3).
- Un cable no tiene clave: escrito dos veces, son dos cables (13.1). WIRE dado un cable lo reescribe; la reescritura tiene una clave, el cable, por lo que una reescritura posterior reemplaza a una anterior (13.1).
- Las restricciones no tienen clave. Para cambiar una restricción, desactívese (`"disabled"`, 14.8) y escríbase una nueva, o edítese el texto.

### 5.9 Números de objetos sin nombre

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

- El registro del solucionador hace referencia por número a los objetos a los que no puede llegarse por un nombre, para poder escribir de vuelta cada decisión (14.8). Un número se escribe `%#` seguido de un entero sin signo, como `%#3`, y se usa como un nombre: `%#5::pin` es un pin del símbolo del uso `%#5`, y `%#5::mid` uno de sus objetos locales. Un número solo hace referencia a su objeto; no se vincula como un nombre, por lo que `%#3 = …` y `=> %#3` son errores de sintaxis (4.1).
- **Qué objetos.** Los puntos, cables, etiquetas, agrupaciones, anotaciones y usos que no tienen nombre propio y no pueden alcanzarse mediante `::` desde un objeto que tenga un nombre o un número (6.5, 9.4). Un nombre vinculado solo en un conjunto anidado (6.7) no cuenta, pues el registro del solucionador no puede usarlo. Para un registro del solucionador, los objetos son, en este orden:
  1. los creados por los miembros de los registros de autor del nivel (`type="authored"`, 2.8), y en los cuerpos de los usos que crean esos miembros. Se cuentan en el nivel interpretado tal como lo ve el solucionador, sin los registros con `type="generated"` (2.8), y conservan sus números cuando el nivel se interpreta con esos registros;
  2. los creados por los miembros del propio registro, y en los cuerpos de los usos que estos crean. Se cuentan en el registro tal como está escrito, como si todo número en él pudiera resolverse; que la comprobación siguiente coincida o no, no los altera.

  Las restricciones y los conjuntos no se numeran. Los objetos que las reglas generan uno por uno (10.5) tampoco se numeran: el solucionador los escribe como sentencias explícitas, que entran en el punto 2. Cada registro del solucionador tiene sus propios números: el punto 1 es el mismo para todos ellos, y el punto 2 cuenta únicamente los miembros propios del registro.
- **Orden.** Los registros de autor se toman en el orden de 2.6: el texto propio del nivel, luego los registros escritos en el nivel desde fuera con `scope`. Dentro de un miembro, los objetos se numeran en el orden del token que los crea: el `(` de un punto, la palabra clave de los demás. Cuando un token crea un objeto por cada miembro de un conjunto (5.2.1), cada objeto tiene su propio número, en el orden de los miembros del conjunto. Los objetos en el cuerpo de un uso se numeran justo después del uso, en el orden del cuerpo, y así sucesivamente a través de usos anidados. Los números comienzan en 1.
- **Dónde.** Los números pueden escribirse únicamente en registros con `type="generated"` que tengan el campo `numbering` y no tengan `scope` (2.6, 2.8). En cualquier otro lugar un número es un error: un registro de autor hace referencia a los objetos por nombre. Dentro de su registro, un número puede usarse antes del miembro que crea su objeto, igual que un nombre (6.7).
- **Comprobación.** `numbering` contiene una comprobación de cómo se crean los objetos numerados. Por cada número, en orden, hay una línea:
  - los tokens del miembro del registro, o del cuerpo de la definición, que crea el objeto, con todos sus segmentos (4.1), sin texto libre ni comentarios, separados por un espacio. Aquí un token es una unidad léxica: una palabra clave; una referencia, con sus terminales, subobjetos y subíndice, escrita sin espacios en blanco; una cadena o un número tal como están escritos; un nombre; `_`; o uno de `{ } ( ) [ ] , = ; =>`, o una parte de lugares de miembros (`:`, `end`, `-`, 5.2.3). Así, `&out @WIRE {#M1:1 (_, _) #R1:2}` da `&out @WIRE { #M1:1 ( _ , _ ) #R1:2 }`;
  - para un objeto en el cuerpo de un uso, la línea del uso va primero, formada del mismo modo tenga o no número el uso, luego un tabulador y los tokens del miembro del cuerpo, tal como lo escribe la definición;
  - cuando el token crea un objeto por cada miembro de un conjunto, siguen un tabulador y el manejador del miembro, escrito como una referencia: una instancia o red del anfitrión por su nombre; un terminal por su instancia y número de terminal, como `#M1:1`; un pin por su objeto y el nombre del pin, como `#X1::vdd_t`, o, sin nombre, por su terminal; la posición de un objeto, o un texto con clave, por el objeto seguido de `::position`, o por `::` y la clave; un objeto RNL por un nombre `%` que el nivel vincule a él, el vinculado primero en el orden de 2.6 cuando haya varios, cualquiera que sea el nombre con el que se escribió el conjunto; un objeto sin tal nombre por su número, o, como objeto local de un uso, por la referencia del uso, formada por estas mismas reglas, `::` y su nombre local: el nombre vinculado primero en el cuerpo de la definición cuando haya varios, también dentro de un miembro (en `%p = (0, 0) => %q` es `p`), cualquiera que sea el nombre con el que se escribió el conjunto, y a través del uso aplicado primero cuando varios usos lo tengan como objeto local. Un nombre se escribe tal cual cuando consta únicamente de letras ASCII, dígitos y `_`, y en caso contrario entre comillas dobles, con cada `"` duplicada (3.6). Por ejemplo, tras `%p = (0, 0); %q = %p; %s = {%q}`, la anotación que crea `%s @TEXT("x")` tiene la línea `%s @TEXT ( "x" )`, un tabulador y `%p`, y la comprobación `675df5171032726d`. Con `$D = @DEFINE(roles=["x"]) { %p = (0, 0) => %q }; %u = #M1 @AS $D; %s = {%u::q}`, la línea de `%s @TEXT("x")` termina con un tabulador y `%u::p`, y la comprobación es `a8c9571193175903`.

  La comprobación son los primeros 16 dígitos hexadecimales, en minúsculas, del SHA-256 de esas líneas en UTF-8, cada una terminada en un salto de línea.
- Cuando la comprobación de un registro no coincide con el nivel, las sentencias del registro que usan un número no surten efecto y, cuando hay alguna, el registro produce una indicación: la solución está desactualizada para los objetos sin nombre. Un registro que no usa ningún número no necesita tener `numbering`. Las demás sentencias del registro surten efecto como de costumbre (capítulo 17). Un número mayor que el último objeto numerado es un error.
- En el ejemplo, la comprobación tiene una línea por cada uno de seis números: el miembro WIRE dos veces (para el cable, luego para el punto), el miembro TEXT dos veces, seguido de `#M1` y de `#R1`, y el WIRE que reescribe dos veces más, puesto que los dos puntos escritos como coordenadas en él son objetos del propio registro, con los números 5 y 6. Un visor que no resuelve dibuja el punto libre, el trazado del cable y las dos etiquetas tal como los eligió el solucionador.
- Escribir en su lugar `{#R1 #M1}` intercambia las etiquetas 3 y 4: la comprobación deja de coincidir, y las sentencias con números no surten efecto hasta que el nivel se resuelva de nuevo, mientras que M1 y R1 conservan sus posiciones. Escribir el ALIGN de otro modo, o mover M1, deja los números como están.
- Nota: la comprobación cubre únicamente cómo se crean los objetos numerados. Mover un dispositivo, o cambiar una sentencia que no crea ningún objeto sin nombre, deja los números tal como están; insertar, eliminar, cambiar o reordenar un miembro que crea tales objetos los deja desactualizados hasta que el nivel se resuelva de nuevo. Un registro del solucionador normalmente no contiene sentencias que cambien conjuntos o definiciones; el punto 1 se cuenta sin tales registros en todo caso.

<!-- translated from en/ch06.md sha256:01f4618ca3e4 -->

## 6 Nombres y ámbito

### 6.1 Jerarquía de niveles

- **Los niveles siguen el texto.** El RNL de un nivel se aplica a ese nivel y a los niveles que contiene en su texto; qué subcircuito instancia a cuál no interviene.
  - Un archivo contiene los subcircuitos que define y los archivos que incluye.
  - Un `.include` escrito en un subcircuito, y una definición `.subckt` anidada en un subcircuito (una definición local en SPICE3 y ngspice), están por debajo de ese subcircuito.
- Desde arriba, la cadena de niveles hasta un subcircuito es: los paquetes en la raíz (los paquetes de USE-PACKAGE en registros de encabezado, figurando el paquete estándar primero entre ellos, 16.3) → el nivel de archivo del archivo principal (el nivel superior) → … (los archivos incluidos, unos dentro de otros) → el archivo que define el subcircuito → el propio subcircuito. Por ejemplo:

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

- **El RNL de un subcircuito cubre solo al subcircuito.** Al resolver conflictos, sus sentencias van después del contenido de los niveles de archivo, y por tanto prevalecen sobre él (8.5); pero no se transmiten a los subcircuitos que instancia, salvo a los definidos dentro de él. Cuando amp instancia a cell, las definiciones y reglas de amp no alcanzan a cell.
- **Un archivo incluido no afecta al nivel que lo incluye.** Las definiciones, reglas y USE-PACKAGE de su nivel de archivo se aplican únicamente a sus subcircuitos y a los archivos que incluye a su vez. En el netlist, los dispositivos que escribe fuera de subcircuitos pertenecen al nivel que lo incluye, pero su RNL no puede modificarlos. Así, un archivo nunca se ve alterado por los archivos que incluye.
- **El nivel de archivo de un archivo incluido no tiene circuito propio.** Solo el nivel de archivo del archivo principal es un circuito. Las referencias al anfitrión (`#M1`, `&vss`) en el nivel de archivo de un archivo incluido fallan (6.3); únicamente las definiciones, reglas, USE-PACKAGE y `scope` surten efecto allí. Un archivo abierto por separado es el archivo principal.
- **Un archivo incluido hereda del nivel que lo incluye.** El estilo de la casa del archivo principal (sus reglas y los paquetes de su USE-PACKAGE) desciende hacia los subcircuitos de los archivos de biblioteca. Así, un mismo archivo de biblioteca puede dibujarse de forma distinta bajo distintos archivos principales, o al abrirse por separado, mientras que su resultado del solucionador se almacena una sola vez (capítulo 17). Para mantener los símbolos que utiliza una biblioteca libres de las reglas de apariencia de niveles superiores, úsese de nuevo el paquete estándar con USE-PACKAGE en el archivo de biblioteca o en el subcircuito (16.2). Para fijar únicamente la versión del paquete estándar y dejar aun así que el estilo de la casa superior lo sustituya, escríbase el USE-PACKAGE en el registro de encabezado del archivo de biblioteca (16.3). Las reglas del archivo principal se aplican solo a su nivel superior salvo que su selección escriba `recursive=true` (10.1); las reglas de un estilo de la casa que deban alcanzar a los archivos de biblioteca lo escriben.
- **Archivos Verilog-A.** Un archivo Verilog-A que el netlist incorpora con `ahdl_include` o `.hdl` se sitúa donde se incorpora, igual que un archivo incluido, y cada uno de sus módulos es un nivel por debajo de su nivel de archivo, el cual no tiene circuito ni esquema y únicamente da al módulo su símbolo de bloque (2.9).
- **Un esquema por cada subcircuito.** Un visor no expande subcircuitos: una instancia de un subcircuito se dibuja siempre con su símbolo, y el interior del subcircuito se dibuja y se resuelve por separado. Cada subcircuito tiene una única cadena de niveles superiores, por lo que en un documento cada subcircuito tiene exactamente un esquema.
- **Paquetes.** Las definiciones `$` de un paquete surten efecto en el nivel donde está escrito `@USE-PACKAGE` y en los niveles inferiores; sus reglas cuentan como escritas en ese nivel y alcanzan a los niveles inferiores con `recursive=true` (16.2, 10.4). Los paquetes de USE-PACKAGE en un registro de encabezado se sitúan en la raíz, figurando entre ellos el paquete estándar cuando se escribe o se añade mediante un ajuste de la herramienta (16.3). Una inclusión coloca un archivo por debajo del nivel, y el archivo hereda del nivel; un USE-PACKAGE coloca un paquete por encima del nivel, y el nivel hereda del paquete.
- **Las reglas se aplican por defecto a su propio nivel.** Una regla escrita en un nivel se evalúa únicamente en ese nivel. Cuando alcanza los niveles inferiores, mediante `recursive=true` en una selección escrita en ella o, si no tiene ninguna, mediante los conjuntos que usa (10.1), se evalúa también una vez en cada nivel inferior, atendiendo cada vez únicamente a ese nivel (10.4). Esto es lo que Principios 2.5 exige a las reglas: una regla puede limitar su efecto al nivel actual o dejar que los niveles inferiores lo hereden, y se aplica por defecto a su propio nivel, mientras que las definiciones `$` se heredan por defecto (6.8).

### 6.2 Prefijos

| Prefijo | Hace referencia a | Se resuelve en |
|---|---|---|
| `#` | Una instancia del anfitrión; `#` solo es este nivel | El anfitrión de este nivel |
| `&` | Una red del anfitrión; `&` solo es el nodo vacío, ninguna red en absoluto (6.3) | El anfitrión de este nivel |
| `%` | Un nombre dado en este nivel: de un objeto de la escena, de un objeto existente como `%tail = #M5`, o de una definición usada solo en este nivel | Este nivel |
| `$` | Una definición o un conjunto (6.6) | El nivel donde se vincula, y los niveles inferiores |
| `?` | Un rol o un parámetro de una definición; `?` solo es, en un cuerpo de símbolo, el objeto dibujado con el símbolo (13.3), y en el cuerpo de una definición general, el uso que se está expandiendo (9.4) | El cuerpo de una definición general o de un símbolo |

`$X` y `%X` son dos nombres distintos.

### 6.3 Referencias al anfitrión

- `#M1` y `&vss` se resuelven únicamente en el anfitrión de este nivel, nunca en otro nivel (Principios 2.5). Las instancias con el mismo nombre en subcircuitos distintos no guardan relación.
- Una referencia al anfitrión que no se encuentra falla (Principios 2.4). El nivel de archivo de un archivo incluido no tiene circuito propio, por lo que toda referencia al anfitrión falla allí (6.1).
- Un nombre puede ir seguido directamente de un subíndice, que representa uno o varios nombres del anfitrión, como `&D<3>` y `#XI0<3:0>` (6.11).
- El adaptador del anfitrión decide qué redes tiene un nivel, siguiendo la semántica del anfitrión:
  - Redes locales: los nodos que utilizan las líneas del netlist del nivel, incluidos los puertos.
  - Redes globales: `0` es la tierra global en todo dialecto de SPICE. Los alias de tierra que conoce el propio simulador (como `gnd` en ngspice, y `GND`, `GND!`, `!GND` y `GROUND` en HSPICE) los fusiona en `0` el adaptador del dialecto; tras la fusión, `&gnd` y `&0` son el mismo manejador. Un nombre declarado con `.GLOBAL vdd` (`global vdd` en Spectre) es el mismo nodo en todo subcircuito. Un nombre terminado en `!`, como `vdd!`, es una convención de los esquemas de Virtuoso; lo que cuenta es la declaración.
  - Una red global tiene un manejador en cada nivel que la utiliza: la copia en ese nivel. Un nivel cuyas líneas del netlist no la utilizan no tiene copia. Así, `&0` hace referencia a la copia en el nivel donde está escrito: cada esquema dibuja la suya, los hechos pertenecen a ese nivel y las descripciones de un nivel inferior no pueden alterar un nivel superior.
  - Para referirse a la red global en todo nivel, selecciónese por nombre en un nivel superior con la condición `global=true` (10.1).
- **El nodo vacío.** `&` escrito solo, sin nombre, indica que no hay ninguna red: lo que se le pasa está conectado a ningún nodo eléctrico del circuito. Se escribe donde se admite una red y no existe ninguna, como en los pines de componentes virtuales que dibujan un diagrama de bloques sin líneas de netlist: `$INTEGRATOR @ATTACH &(in) &y(out)`, `& @WIRE {%sub1::y %int1::a}`.
  - No es un objeto: no tiene manejador, no puede vincularse a un nombre, no puede ser miembro de un conjunto y nunca se selecciona. Dos lugares que escriben `&` no tienen nada en común; un visor que resalta un pin o un cable en el nodo vacío resalta únicamente ese.
  - Dos lugares lo toman tal cual: un rol cuya clase es `"net"` (9.4) y la ranura de red de WIRE (13.1). Un pin vinculado a un rol que rellena no está en ninguna red, y su `::net` es el nodo vacío (12.1); `${net}` y `${p}` de él son el texto vacío (13.6).
  - En cualquier otro lugar que tome una red, cuenta como un conjunto vacío de redes: ROUTE-NET no une nada, ASSERT-NET se cumple trivialmente, una selección con él como ámbito no selecciona nada y ADD-TO no añade nada (5.2.1). Escrito explícitamente allí produce una advertencia, como hace un conjunto vacío escrito explícitamente; alcanzado a través de un rol, como en el cuerpo de una definición, no produce ninguna. Así, un cuerpo que utiliza la red de un rol no necesita un caso aparte para él.
  - No es una red del anfitrión, por lo que `&` seguido de un nombre que el anfitrión no tiene sigue fallando como antes.

### 6.4 Terminales

- `object:terminal` es un terminal: `#M1:1`, `#M1:d`, `#XU1:inp`, y un terminal de una agrupación como `%x:in` (11.8). `#:inp` es el puerto inp de este nivel. Solo las instancias del anfitrión, las agrupaciones y los puertos de un nivel tienen terminales; un uso no tiene ninguno (12.1).
- Un número es el número de terminal del anfitrión, contado desde 1. Un terminal escrito solo con dígitos es un número, también cuando va entrecomillado (4.1), dondequiera que se nombre un terminal: en referencias `:`, en `terminal=` de PIN, en `terminals=` de SET-SYMBOL y en la condición `terminal` de selecciones. Así, un nombre de puerto compuesto solo por dígitos, como en `.subckt amp 3 1 2`, se alcanza por su lugar, y `#X1:1` es el puerto llamado 3; un nombre de DEVICE compuesto solo por dígitos es un error (6.12).
- Un nombre es un nombre de terminal del anfitrión: un nombre dado por `@DEVICE` en el registro de encabezado (6.12), o un nombre de puerto del subcircuito. Un nombre que no pueda escribirse sin comillas (3.6) va entrecomillado, como `#XU1:"in+"`. Un nombre de terminal o de puerto puede ir seguido directamente de un subíndice, como `#X1:D<7:0>` y `#:D<7:0>` (6.11).
- Mayúsculas y minúsculas: los nombres de puerto de los subcircuitos y los nombres de terminal dados por DEVICE siguen las reglas del anfitrión: SPICE ignora mayúsculas y minúsculas, de modo que `#M1:D` también funciona; Spectre no.
- El `label=` de un pin es solo para mostrar en pantalla y no puede usarse para hacer referencia a un terminal: los terminales pertenecen al netlist, y una referencia a uno no depende del símbolo en uso.
- **Terminales y pines**: un terminal pertenece al netlist; un pin pertenece al símbolo de un objeto dibujado (9.2.4). Un terminal se dibuja en los pines que lo vinculan, y donde se necesita un pin o un punto, un terminal representa a su pin (5.2.4). Un pin se escribe a través de su terminal, como `#M1:d`, o, cuando tiene nombre en su símbolo, por ese nombre como subobjeto, como `#X1::vdd_t` (6.5); un número nunca es un nombre de pin.
- Un terminal que no se encuentra falla. Si un terminal tiene un pin en su símbolo se comprueba después de decididos los símbolos (7.1).
- Aparte de los nombres de puerto de los subcircuitos, los nombres de terminal los da DEVICE; 6.12 enumera los nombres estándar de la familia SPICE.
- Un subcircuito que el netlist no define (como el dispositivo de PDK en `XM1 … sky130_fd_pr__nfet_01v8`) tiene nombres de terminal únicamente cuando el usuario los da con DEVICE en el registro de encabezado (6.12), o utiliza un paquete que contiene sentencias DEVICE (16.2); de lo contrario solo pueden usarse números.

### 6.5 Subobjetos

- `object::name` es un subobjeto. Es siempre un objeto, nunca un valor. Su manejador es el objeto padre y el nombre, por lo que dos nombres son dos miembros dondequiera que se cuenten los miembros antes de asignarse los símbolos (5.2.4): `{#R1::p #R1::q} @UNION` tiene dos miembros, incluso cuando ambos nombres acaben designando un mismo pin.
- Los subobjetos son:
  - Una clave de un texto: el subobjeto que un TEXT con `key=` crea en el objeto al que está adjuntado, como `#M1::wl`. Un texto con clave en un símbolo es un subobjeto de cada objeto dibujado con el símbolo (13.3). `::name` y `::value` no son subobjetos estándar, sino solo claves habituales (13.3).
  - `::position`: el origen del objeto. `#M1::position @PLACE(…)` es `#M1 @PLACE(…)`. Para un uso es su propio punto (14.1), el origen de su símbolo cuando tiene uno (12.1). En un conjunto es el conjunto de los `::position` de cada miembro, siguiendo los conjuntos anidados hacia dentro: `{%dp1 %dp2}::position` es `{%dp1::position %dp2::position}`, los dos puntos propios, mientras que `{%dp1 %dp2}` en una restricción representa a los seis transistores (14.1). Para un TEXT es la posición del propio texto (13.3); para un RECTANGLE, CIRCLE o ELLIPSE escrito en la forma de centro, el centro (13.4).
  - `::net` de un terminal: la red en la que está, como `#M5:1::net`, que es `&tail`. Para los terminales de una agrupación véase 11.8: cuando los miembros están en redes distintas, es el conjunto de esas redes. `::net` de un pin: la red del terminal que vincula, o de lo que rellena el rol que vincula (12.1); para un pin de bus, el conjunto de esas redes; un pin que no vincula nada no tiene ninguna.
  - Un pin nombrado del símbolo del objeto: `#X1::vdd_t` es el pin llamado vdd_t en el símbolo de X1, `%g0::pin` el pin llamado pin en el símbolo del uso `%g0` (9.2.4).
  - Un rol de un uso: lo que rellena ese rol, como `%dp1::left`, que es M1, y `%g0::p`, que es la red vss. Para un rol de conjunto es ese conjunto. Es ese objeto mismo y no un segundo objeto: `{%dp1::left #M1} @UNION` tiene un miembro, y un PLACE escrito para `%dp1::left` tiene la clave de M1.
  - Un objeto local nombrado en el cuerpo de un uso, como `%c1::mid` (9.4).
- Los nombres estándar de subobjeto están reservados: ningún nombre de rol, nombre de parámetro ni nombre local de una definición general, ni ninguna clave de un texto, puede coincidir con uno de ellos (comparado tal como está escrito, 3.6). Un nombre local no puede ser un nombre de rol, y la clave de un texto no puede ser un nombre de rol ni un nombre local del uso al que está adjuntado. Un nombre local sí puede ser un nombre de parámetro: un parámetro es un valor, y `::` no toma valores. Ningún parámetro de una definición puede llamarse `priority` tampoco (14.8).
- **El nombre después de `::` se busca en este orden**: los nombres de rol y nombres locales del propio uso; sus alias de `definition=`; los nombres estándar de subobjeto y sus alias; los nombres de los pines del símbolo del objeto; las claves de los textos (alias: 16.5). Un nombre de pin que coincida con un nombre de rol o nombre local del uso, o con un nombre estándar de subobjeto, es un error cuando se asigna el símbolo al objeto; y lo mismo una clave de un texto que coincida con un nombre de pin.
  - Las claves van al final, por lo que una clave no puede escribirse igual que un nombre que vaya antes: un nombre estándar de subobjeto o uno de sus alias declarados en este archivo (como `"位置"` de `alias-zh`), un nombre de rol o nombre local del uso, o un alias de `definition=` de esos nombres. Tal clave es un error, como las colisiones anteriores (13.3).
- `::` toma únicamente partes del propio objeto; buscar objetos en la escena por condiciones se hace siempre con palabras clave de selección. Por tanto, "todos los usos de una definición" se escribe como una selección (10.1), no como un subobjeto de la definición.
- Los terminales usan `:` y los subobjetos `::`, de modo que los nombres de subobjeto añadidos en versiones posteriores no puedan chocar con los nombres de puerto de un netlist.
- **Nombres de subobjeto**: hay dos nombres estándar de subobjeto, `position` y `net`, y ambos están reservados. Todos los demás nombres de subobjeto son dinámicos: claves de textos, nombres de rol de usos, nombres locales en cuerpos de definiciones y nombres de pines. Los nombres estándar de subobjeto añadidos en versiones posteriores se aplican por versión (16.1): en un archivo de una versión anterior, una clave con el mismo nombre sigue siendo válida.

### 6.6 Nombramiento

- `%name = statement` y `$name = statement` dan nombre al resultado de una sentencia; en una tubería, `=> %name` y `=> $name` dan nombre al resultado del segmento a su izquierda (5.5). Un nombre es solo otra forma de hacer referencia a un objeto y no crea ningún objeto.
- `$name` vincula una definición o un conjunto, incluido el resultado de una selección. Un conjunto `$` evaluado en un nivel contiene únicamente los miembros de ese nivel (10.3). Otros objetos de la escena pertenecen a un nivel y no pueden tener nombre `$`.
- `%name` puede vincular cualquier objeto de este nivel: un objeto creado por la sentencia, un objeto existente (`%tail = #M5`), un conjunto (`%pair_set = {#M1 #M2}`), o una definición usada solo en este nivel.
- **Un nombre se vincula una sola vez en un nivel.** Vincularlo de nuevo al mismo objeto está permitido, como `%x = #M1` escrito dos veces; vincularlo a otro objeto es un error, y el nombre conserva su primera vinculación válida. Lo que vincula una vinculación sigue a 5.5. Un segmento que conserva el manejador de su entrada, como PLACE u ORIENT, vincula ese manejador, incluso cuando el segmento falla, y tal vinculación es válida exactamente cuando lo es ese manejador: cuando la entrada misma es un objeto fallido, como tras un BUNDLE que falla, la vinculación tampoco es válida (5.5). Un segmento que crea un objeto nuevo lo vincula, y la vinculación es válida cuando ese segmento no falla al leerse y crearse el nivel (pasos 1 a 3 de 8.2); cuando falla después, en una comprobación hecha tras asignarse los símbolos (8.2), conserva el nombre, una vinculación posterior no lo toma, y el nombre hace referencia al objeto fallido desde ese momento. Solo cuando ninguna de sus vinculaciones es válida queda el nombre vinculado a un objeto fallido (7.2). Una vinculación no válida no retiene el nombre, por lo que una vinculación posterior a otro objeto no es el error de vincular dos veces: tras el bundle fallido de 5.5, `%s = #XI1` vincula `%s` a XI1. Esto vale por igual para sentencias que crean objetos, definiciones, conjuntos y resultados de selecciones.
- La sustitución se aplica a los hechos, no a los nombres: para modificar un objeto creado por RNL, escríbase una sentencia que lo modifique, como `%g0 @PLACE(40, 38)` (5.8).

### 6.7 Ámbito de los nombres

- Los nombres consignados por `@RNL` se publican en el nivel: todos los registros del nivel los comparten, y un nombre puede usarse antes de la sentencia que lo vincula (Principios 2.9).
- Un nombre vinculado en un conjunto anidado vale solo en ese conjunto.
- Un nombre vinculado en el cuerpo de una definición es local al cuerpo (6.9).

### 6.8 Herencia y ocultación

- Las definiciones `$` se heredan hacia abajo en la jerarquía de niveles (6.1), nunca hacia arriba: hacia los subcircuitos del archivo y hacia los archivos incluidos. Un nivel no ve las definiciones `$` de los archivos que incluye; para proporcionar definiciones a niveles superiores, créese un paquete y úsese con USE-PACKAGE (16.2). El símbolo de bloque de un subcircuito es la única excepción que sube (9.2.6).
- Una `$X` definida en un nivel inferior con el mismo nombre que una heredada oculta la definición heredada en ese nivel y los niveles inferiores, y produce una indicación; el nivel superior sigue viendo la suya.
- Las definiciones `$` de un paquete usado con USE-PACKAGE en un nivel cuentan como un nivel por encima de ese nivel (16.2). Cuando ocultan definiciones de los niveles superiores o de los paquetes en la raíz, se produce una indicación, como para cualquier ocultación. También se produce una indicación cuando un paquete usado en un registro de encabezado define un nombre del paquete estándar, o de un paquete situado en la raíz antes que él (16.3).
- Los nombres `%` valen solo en su propio nivel y no se heredan: ni hacia los subcircuitos del archivo, ni a través de inclusiones. Las definiciones `%` de un paquete son auxiliares privados del paquete.

### 6.9 Nombres en cuerpos de definición

- Los nombres en el cuerpo de una definición se resuelven donde está escrita la definición, no de nuevo donde se aplica; la única excepción es un `#` aislado en el cuerpo de una definición general (abajo).
- Por tanto, ocultar una definición cambia únicamente a qué hace referencia el nombre: las definiciones ya usadas dentro de otras definiciones permanecen tal como están. Por ejemplo, ocultar `$DIFFERENTIAL_PAIR_N` en un nivel inferior no cambia el par diferencial usado en el cuerpo de una definición escrita en un nivel superior.
- Un `%name` en un cuerpo se busca primero entre los nombres locales del cuerpo y luego entre las definiciones del nivel. No se puede hacer referencia desde un cuerpo a los objetos de la escena del nivel (puntos, cables, etc.); tal referencia es un error. Un conjunto o el resultado de una selección que el nivel nombra con `%` también es un objeto así, cualesquiera que sean sus miembros al evaluarse: para compartir uno con cuerpos, nómbrese con `$`, al que un cuerpo sí puede hacer referencia y que se evalúa en el nivel de cada uso (6.6, 10.3).
- **Notación en los cuerpos**: los roles y parámetros se escriben `?name`. Los nombres locales se vinculan con `%name =` (o `=> %name` en una tubería) y se referencian como `%name`. Desde fuera, a los nombres locales se llega a través del uso (9.4).
- El cuerpo de una definición no ve el nombre que se está definiendo: en `$NMOS = @DEFINE-SYMBOL {$NMOS; …}`, el `$NMOS` del cuerpo es la definición de ese nombre fuera (en un nivel superior o un paquete). Así es como un nivel inferior oculta y extiende una definición superior; sin tal definición externa es un error.
- Las referencias al anfitrión en los cuerpos de DEFINE-SYMBOL y DEFINE son errores: `#M1`, `&vss`, `#:inp` y `#M1:d` no pueden escribirse allí, ni tampoco un `#` aislado en un cuerpo de símbolo. En el cuerpo de una definición general, un `#` aislado designa el nivel en el que está el uso, que se conoce únicamente donde se aplica la definición (9.4); se escribe allí como ámbito de una selección. Tampoco puede asignarse un símbolo de bloque desde un cuerpo: `# @SET-SYMBOL` allí es un error, pues el símbolo de bloque de un nivel entra en la cola de símbolos antes que los usos del nivel, y un uso puede generarse después de él (8.3, 9.2.6). Los nombres de un cuerpo se resuelven donde está escrita la definición, pero una definición general se aplica en otros niveles y otros archivos, donde una referencia al anfitrión seguiría apuntando al nivel de la definición, por lo que su significado no sería unívoco. Una definición general que necesite objetos del anfitrión los recibe a través de sus roles.
- **Cuándo se comprueba el cuerpo de una definición general**:
  - Una vez por cada definición: todo lo que no depende de los argumentos se comprueba entonces, incluidas las palabras clave, las firmas (número de objetos, nombres de parámetro y tipos), los nombres `$` y `%` (resueltos donde está escrita la definición), las referencias al anfitrión, las colisiones de nombres locales (9.4) y si los roles declarados con una clase se usan correctamente: cuando el rol t se declara `"instance"`, `?t::net` en el cuerpo es un error. Las firmas se comprueban al leer el texto (7.1); los nombres pueden resolverse solo después de leído todo el nivel, por lo que las comprobaciones que necesitan nombres se realizan cuando el nivel surte efecto. Ambas se realizan una sola vez por definición, por muchas veces que se aplique.
  - Para cada aplicación: lo que depende de los argumentos, como los objetos que reciben los roles sin clase declarada, o si el símbolo tiene un pin, se comprueba en cada aplicación y se informa en la aplicación, señalando también a la sentencia en la definición general.
  - Alcance de los errores: una sentencia del cuerpo hallada errónea (incluido un nombre local que colisiona) se informa una sola vez, en la definición, y se excluye de toda expansión; el resto del cuerpo funciona como de costumbre (el cuerpo es tolerante, 9.1). Una lista de roles o lista de parámetros errónea (nombres repetidos, colisiones con nombres estándar de subobjeto, nombres de clase desconocidos) hace fallar toda la definición general, y con ella toda aplicación que la use.
  - Una definición general que nunca se aplica se comprueba también. Estas comprobaciones no dependen de aplicaciones, por lo que detectan errores en las definiciones sin usar de un paquete.

### 6.10 Marcadores

- `?name` aparece únicamente en el cuerpo de una definición general, y el nombre debe estar en la lista de roles o en la lista de parámetros; de lo contrario es un error al leer el texto.
- Donde va un objeto, es un rol, como `?t` o `?t::net`; donde va un literal, es un parámetro, como `min=?gap`, `[?w, 2]` y `(?x, 0)` (9.4). Un rol donde va un literal, o un parámetro donde va un objeto, es un error al leer el texto. Los nombres de rol y nombres de parámetro están en una misma tabla y no pueden repetirse.
- Un marcador de rol puede llevar un camino, como `?t::net`, y el marcador de un rol de conjunto puede tomar lugares de miembros, como `?ref{1}` (5.2.3); un marcador de parámetro no admite ninguno de los dos.

### 6.11 Subíndices

| Escrito | Representa |
|---|---|
| `&D<3>` | Lo mismo que `&"D<3>"` |
| `&D<7:0>` | Una lista: `D<7>`, `D<6>`, …, `D<0>` |
| `&D<1,2:3>` | `D<1>`, `D<2>`, `D<3>` |
| `&D<0:6:2, 9>` | `D<0>`, `D<2>`, `D<4>`, `D<6>`, `D<9>` |
| `&D[3:0]` | `D[3]`, `D[2]`, `D[1]`, `D[0]` |
| `#XI0<3:0>` | Cuatro instancias |
| `#X1:D<7:0>` | Ocho terminales de X1 |
| `#:D<7:0>` | Ocho puertos de este nivel |
| `&"D"<1:0>` | `D<1>`, `D<0>` |
| `&"D<7:0>"` | El nodo cuyo nombre es `D<7:0>`: nada dentro de comillas se analiza |

- Un netlist no tiene buses: cuando herramientas como Virtuoso escriben un netlist, dividen un bus en bits individuales, como `D\<3\>` en un netlist Spectre; los puertos de bus de subcircuitos y las instancias iteradas también se dividen. Un subíndice es una forma abreviada de escribir uno o varios nombres así. Solo construye nombres y no depende de que el anfitrión conozca buses, por lo que su significado es unívoco.
- **Forma**: un nombre del anfitrión, entrecomillado o no, seguido directamente de `<…>` o `[…]` que contiene elementos separados por `,`. Cada elemento es `i`, `a:b` o `a:b:s`, y las formas pueden mezclarse. Los números i, a, b y s son enteros decimales sin signo, y s es al menos 1. No hay espacio en blanco entre el nombre y el delimitador; dentro de los delimitadores se permiten espacios en blanco y comentarios (3.5).
- **Expansión**: los elementos se expanden en el orden escrito y se unen; las repeticiones se conservan. `a:b` va de a hasta b, y `a:b:s` en pasos de s sin sobrepasar b: `<7:0>` es 7, 6, …, 0, y `<0:7:2>` es 0, 2, 4, 6. El paso va al final, como en Virtuoso y Xschem, no como `a:s:b` de MATLAB. Un nombre generado es el nombre anterior al delimitador, el delimitador de apertura, el número decimal sin ceros iniciales y el delimitador de cierre, en el tipo de delimitador escrito: `&D<07>` es `D<7>`, y `&D[3]` es `D[3]`.
- **Uno o una lista**: un único elemento que sea un solo número da una referencia: `&D<3>` es `&"D<3>"`. Cualquier otra cosa da una lista, que es una referencia a un conjunto, como `&D<3:3>` y `&D<3,5>`. Una lista en un conjunto se despliega un nivel (5.2); como único objeto de una sentencia se divide primero un nivel (5.2.1), de modo que `#XI0<3:0> @ALIGN("y")` alinea cuatro instancias; en una ranura que toma un único objeto actúa miembro por miembro; en la ranura de red de WIRE se toma entera y crea un bus (13.1).
- **Dónde**: nombres del anfitrión después de `#` y `&`, y nombres de terminal y nombres de puerto en referencias `#`. Los nombres RNL (después de `%`, `$` y `?`), los nombres de rol y los nombres de subobjeto no admiten subíndice; un `<` o `[` justo después de ellos es un error. Los miembros de un conjunto se toman por sus lugares con llaves en su lugar (5.2.3).
- **Ningún punto justo después de un nombre**: un `<` o `[` justo después de un nombre inicia siempre un subíndice, y cuando lo que sigue no se ajusta a la forma, es un error de sintaxis; nunca se lee como un punto. Para escribir un punto después de un nombre, sepárense, como `&D (1, 2)`.
- **Una sola expansión**: un nombre tiene como máximo un subíndice, de modo que un nombre como `A<1>[2]` se escribe entre comillas, `&"A<1>[2]"`. Como máximo un subíndice en una referencia se expande a una lista: `#XI0<3:0>:a` y `#X1:D<7:0>` están permitidos, `#XI0<1:0>:D<1:0>` es un error, y dos subíndices que sean cada uno un solo número están permitidos, como `#XI0<3>:D<1>`. Cuando una referencia que se expande a una lista va seguida de `::`, el subobjeto se toma para cada miembro, como `#XI0<3:0>::name`.
- **Nada dentro de comillas se analiza**: `&"D<7:0>"` hace referencia al nodo llamado `D<7:0>`; `&"D"<7:0>` se expande. Cuando un nombre en el netlist contiene `<` o `[` que no marcan los bits de un bus, entrecomíllese para hacer referencia a él tal cual.
- **Los nombres comparados son los que lee el adaptador** (3.6): `D\<7\>` en un netlist Spectre, donde la barra invertida es el escape de Spectre, lo lee el adaptador como `D<7>`, por lo que `&D<7>` hace referencia a él.
- Cada miembro se resuelve por separado. Un miembro que no se encuentra es un objeto fallido, tratado de forma estricta o tolerante según indique la ranura que lo recibe (7.3).
- Las cadenas en parámetros no se expanden, como `terminal=` de PIN (9.2.4) y la condición `name=` de una selección.
- No se aceptan: la repetición de Virtuoso, `<*2>A` y `A<0:2*2>`, ni su yuxtaposición `A,B` (ya pueden escribirse conjuntos); `D[0..7]` de KiCad y Xschem, cuyos miembros son D0 … D7 sin corchetes.
- Las instancias iteradas (como `I0<3:0>` de Virtuoso: varias instancias del anfitrión dibujadas como una) se escriben como agrupaciones: `#XI0<3:0> @BUNDLE` (11.8).

### 6.12 Nombres de terminal: DEVICE

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

- **RNL nombra los terminales y da el tipo.** `@DEVICE` describe una clase de dispositivos: `terminals` enumera los nombres de sus terminales en el orden de los nodos en la línea del netlist, y `type` da su tipo (abajo). Solo da nombres a los números de terminal del anfitrión, y no cambia ni el orden de los terminales ni la interfaz de un subcircuito; es un alias de visualización, lo cual permite Principios 2.2. Los nombres de una misma lista `terminals` deben diferir según las reglas de mayúsculas y minúsculas del anfitrión y no pueden componerse solo de dígitos (6.4); por ejemplo, `"d"` y `"D"` se comparan iguales en SPICE. En el registro de encabezado de un archivo de paquete, que puede ser usado por archivos de cualquiera de los dos dialectos, deben diferir sin atender a mayúsculas y minúsculas. Una lista con dos nombres iguales, o con un nombre compuesto solo de dígitos, es un error, esa sentencia DEVICE falla, y las instancias conservan los nombres que les dan las sentencias DEVICE anteriores y el adaptador.
- **Únicamente en el registro de encabezado** (2.8): DEVICE escrito en cualquier otro lugar es un error. El registro de encabezado de un archivo de paquete cuenta (16.2). Un paquete en línea no puede contener DEVICE; escríbase en el registro de encabezado del archivo.
- **Emparejamiento**: `name` se compara con el nombre de la instancia, y `model` con el nombre de modelo o nombre de subcircuito escrito en la línea de instancia (en Spectre, el maestro de la instancia, como `resistor` para una primitiva); ambos pueden usar el comodín `*` (cualquier número de caracteres) y se comparan según las reglas de mayúsculas y minúsculas del anfitrión. Puede omitirse cualquiera de los dos, pero no ambos, y un DEVICE coincide con una instancia cuando coinciden los escritos. Un DEVICE da `terminals`, `type` o ambos; uno que no tenga ni `name` ni `model`, o ni `terminals` ni `type`, es un error y falla. Las condiciones de selecciones que comparan nombres admiten `*` por las mismas reglas (10.1).
- **Tipo**: `type` es una cadena, como `"nmos"`; las palabras de la tabla siguiente son las estándar. Es el tipo que leen la condición `type` (10.1), `${type}` (13.6) y las aserciones (15.3), para una instancia a la que su anfitrión no le da ninguno, como una instancia X, incluidos los dispositivos de PDK que el netlist no define. Una instancia de una tarjeta de modelo conserva el tipo de su modelo (abajo): un DEVICE cuyo tipo difiera produce una advertencia y no lo modifica, como ocurre con los nombres de puerto de los subcircuitos.
- **Cuando coinciden varios, cuenta el posterior, elemento por elemento**: para los nombres de terminal y para el tipo por separado, cuenta el último DEVICE coincidente que escriba el elemento, de modo que un DEVICE puede dar los nombres y uno posterior el tipo, y un DEVICE posterior que omita un elemento no lo modifica. Los valores por defecto del adaptador van primero, luego las sentencias DEVICE de los paquetes usados por el nivel (en el orden de las posiciones de los paquetes, 16.2), luego las de los registros de encabezado de los archivos externos, luego las de los archivos incluidos; dentro de un mismo registro de encabezado, en el orden escrito, y entre los registros de encabezado de un mismo archivo, primero el que no tiene `language` y luego los que tienen `language` en el orden escrito, actuando cada uno solo sobre las instancias en secciones de su lenguaje (2.8). Así, lo que escribe un archivo prevalece sobre sus paquetes, los paquetes prevalecen sobre el adaptador, y las sentencias específicas deberían escribirse después de las generales. Las sentencias DEVICE de todos los paquetes van antes que las de los archivos, dondequiera que esté escrito su USE-PACKAGE: el registro de encabezado está siempre al comienzo del archivo, de modo que situar un paquete en su USE-PACKAGE le permitiría prevalecer sobre lo que escribe el propio archivo. Un DEVICE que da `model` no se clasifica como más específico; no hay ninguna regla aparte de especificidad.
- **Ámbito**: DEVICE en el registro de encabezado de un archivo se aplica a ese archivo y a los archivos que incluye; un archivo incluido puede modificar solo a sí mismo y a los archivos que incluye a su vez (6.1). DEVICE en el registro de encabezado de un archivo de paquete se aplica a los niveles que usan el paquete (16.2). Por ejemplo, un paquete para sky130 puede contener sentencias DEVICE, símbolos y las reglas que eligen los símbolos, y un usuario que escriba `@USE-PACKAGE(name="sky130-rnl")` obtiene nombres de terminal, tipos y símbolos.
- **Valores por defecto del adaptador**: las definiciones estándar de elementos de cada dialecto, que cuentan como escritas en el nivel más externo. No hace falta escribirlas en el archivo, y una herramienta puede escribirlas explícitamente. Las herramientas para un mismo dialecto deben dar las mismas definiciones. Son los valores por defecto de la versión del archivo en que figura la línea de instancia (16.1), tal como es un DEVICE en el registro de encabezado de ese archivo. Para la familia SPICE:

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

  Los nombres de instancia en Spectre no empiezan por una letra de elemento, por lo que los valores por defecto se emparejan por `model`, el maestro de la instancia. Para las primitivas de Spectre:

  ```rnl header
  @DEVICE(name="*", model="resistor", terminals=["p", "n"]);      /* and one each like it for capacitor, inductor, vsource, isource, ccvs, cccs, bsource, port */
  @DEVICE(name="*", model="diode", terminals=["a", "k"]);
  @DEVICE(name="*", model="vcvs", terminals=["p", "n", "cp", "cn"]); /* and one each like it for vccs, relay */
  @DEVICE(name="*", model="tline", terminals=["p1", "n1", "p2", "n2"]);       /* as T in SPICE */
  @DEVICE(name="*", model="transformer", terminals=["p1", "n1", "p2", "n2"])   /* as T in SPICE */
  ```

  - `switch` tiene un terminal por cada uno de sus nodos, nombrados como Spectre numera sus posiciones: `t0`, el terminal común, y luego `t1` a `tn`, las salidas conmutadas.
  - Una instancia de una tarjeta de modelo, cuyo maestro es el nombre de una sentencia `model`, toma los nombres de la familia del maestro de ese modelo. Su tipo, para la condición `type` (10.1), se escribe con las palabras de SPICE:

  | Familia | Maestros del modelo | Terminales | Tipo |
  |---|---|---|---|
  | MOS | `bsim1`, `bsim2`, `bsim3`, `bsim3v3`, `bsim4`, `bsim6`, `bsimbulk`, `bsimsoi`, `bsimcmg`, `bsimimg`, `b3soipd`, `mos0`, `mos1`, `mos2`, `mos3`, `mos9`, `mos11`, `mos705`, `mos902`, `mos903`, `psp102`, `psp103`, `psp104`, `pspnqs102`, `pspnqs103`, `ekv`, `hisim`, `hisim2`, `hisimhv`, `hvmos`, `bta_soi`, `ucsd_hvmos`, `hisim_hv`, `utsoi`, `letisoi`, `bsimcmg_va` | `d g s b` | `nmos`, o `pmos` cuando el `type` del modelo es `p` o `pmos` |
  | Bipolar | `bjt`, `bjt500`, `bjt503`, `bjt504`, `bjt504t`, `bjt3500`, `bjt301`, `vbic`, `hicum`, `hicum0`, `hicum2`, `mextram` | `c b e s` | `npn`, o `pnp` cuando el `type` del modelo es `pnp` |
  | Diodo | `diode`, `juncap`, `juncap200`, `dio500`, `junb` | `a k` | `d` |
  | JFET | `jfet` | `d g s` | `njf`, o `pjf` cuando el `type` del modelo es `p` o `pjf` |
  | MESFET | `gaas`, `angelov`, `tom2`, `tom3` | `d g s` | `nmf`, o `pmf` cuando el `type` del modelo es `p` o `pmf` |
  | Resistencia | `resistor`, `rdiff`, `r3` | `p n` | `r` |
  | Condensador | `capacitor` | `p n` | `c` |
  | Inductor | `inductor` | `p n` | `l` |
  | Relé | `relay` | `p n cp cn` | `sw` |
- **Subcircuitos definidos en el netlist** (incluidos los de archivos de biblioteca incorporados por include o lib): los nombres de puerto son los de la definición, lo que es un hecho del anfitrión (Principios 2.2). DEVICE no los modifica; un DEVICE que coincida con una instancia así con nombres distintos produce una advertencia. DEVICE nombra únicamente dispositivos sin nombres de terminal: primitivas, y subcircuitos que el netlist no define (dispositivos de PDK), para los que el usuario, o un LLM, escribe el DEVICE. Un DEVICE sin `terminals` puede dar aun así su tipo a un subcircuito definido en el netlist.
- **Número de nombres**: cuando una instancia tiene más nodos que nombres tiene `terminals`, los terminales adicionales tienen únicamente números; cuando tiene menos, una referencia a un nombre adicional falla.
- **Mayúsculas y minúsculas**: los nombres de terminal dados por DEVICE se comparan según las reglas del anfitrión (6.4).
- Los números funcionan siempre, y también los nombres de puerto de subcircuitos que no están compuestos solo de dígitos, así como `#:port`.

<!-- translated from en/ch07.md sha256:7d1065e7e655 -->

## 7 Fallos y diagnósticos

### 7.1 Dos fases de comprobación

- **Lectura**: la estructura léxica; la gramática; las firmas de las palabras clave integradas, es decir, el número de objetos, sus clases y los parámetros (incluida la forma de las matrices); y si cada `?name` en el cuerpo de una definición general está en su lista de roles o lista de parámetros y figura en un lugar adecuado para él (6.10). Los errores de firma en el cuerpo de una definición general que no dependen de los argumentos se informan también en este momento (6.9).
- **Entrada en vigor**:
  - una vez leído todo el nivel, se resuelven los nombres y las referencias al anfitrión, y se comprueban las clases de los objetos a los que hacen referencia los nombres;
  - los nombres en el cuerpo de una definición general se resuelven una sola vez por definición, no de nuevo para cada aplicación (6.9);
  - se comprueban el número de objetos, los nombres de rol, las clases declaradas para los roles y los parámetros de AS y ATTACH (9.4);
  - cada aplicación comprueba las partes de su expansión que dependen de los argumentos;
  - una vez asignados los símbolos, se comprueba si los terminales a los que se conectan cables y restricciones tienen un pin en su símbolo, si los usos a los que se aplica ORIENT tienen un símbolo, si los nombres de pin alcanzados a través de una instancia o un uso, como `#R1::p` o `%g::pin`, son pines de su símbolo, y si pueden realizarse las conversiones desde y hacia pines (5.2.4).

### 7.2 Objetos fallidos

- Una sentencia que falla produce un objeto fallido, el cual porta un diagnóstico: la causa, la ubicación y la causa de origen.
- Un objeto fallido conserva su lugar entre los miembros, y los demás miembros conservan el suyo. Por ello, cuando el netlist anfitrión no tiene M9, `{#M1 #M2 #M9} @ALIGN("y")` falla en bloque; no se convierte en una alineación únicamente de M1 y M2.
- Cuando un nombre se vincula a un objeto fallido (incluida una definición que falla al leerse), las sentencias que hacen referencia al nombre reciben también el objeto fallido. Las sentencias que dependen de una descripción fallida fallan con ella, sea cual sea su orden (Principios 2.9), salvo que una sentencia cuyo fallo se detecta en una comprobación posterior a la asignación de símbolos no revierte los pasos en los que ya participó (8.2); estos son también errores, y sus diagnósticos indican la causa de origen (7.6).
- Vincular un nombre una segunda vez a otro objeto es un error, y el nombre conserva su primera vinculación válida; solo cuando ninguna de sus vinculaciones es válida queda el nombre vinculado a un objeto fallido (6.6, 5.5).

### 7.3 Entradas estrictas y tolerantes

- La firma declara para cada entrada si es estricta o tolerante:
  - estricta: cuando la entrada contiene un objeto fallido, falla toda la sentencia;
  - tolerante: se eliminan los miembros fallidos y se registra un diagnóstico; el resto surte efecto como de costumbre.
- Por defecto, las entradas absorbidas son estrictas; las entradas encerradas (cuerpos de definición) y las entradas consignadas (`@RNL`) son tolerantes (5.4).
- Los argumentos de AS y ATTACH son estrictos; las sentencias de la expansión son tolerantes: cada una que falla se descarta por separado.
- En tuberías y anidamiento, un fallo afecta únicamente a las descripciones que dependen de él (5.5): cuando una entrada estricta contiene un objeto fallido, la sentencia falla, pero las sentencias anidadas en ella que no dependen del fallo surten efecto como de costumbre; cuando una sentencia trabaja una vez por cada miembro, cada miembro es una descripción por sí solo y falla por separado.

### 7.4 Recuperación

- Un error de sintaxis hace fallar todo el miembro en el que ocurre, hasta el siguiente `;` o el final del conjunto que lo encierra.
- Cuando una cadena o un comentario `/* */` no se cierra, o las llaves no coinciden, no puede encontrarse el final del registro (2.4): el registro queda incompleto y falla todo el registro. Un paréntesis o corchete dentro de un miembro que no se cierra, como el de parámetros, de una matriz, de un rol, de un punto o de un subíndice, es un error de sintaxis solo de ese miembro: los demás miembros del registro se leen como de costumbre.
- Cuando un registro se interrumpe antes de estar completo, falla todo el registro, y el diagnóstico indica que el registro está incompleto (2.4).
- La recuperación de un error nunca rebasa su registro: los demás registros se leen como de costumbre (Principios 2.9).

### 7.5 Los conflictos no son fallos

- Los conflictos se tratan una vez leído todo el nivel, mediante los métodos definidos: la sustitución por clave (8.4), las colas de apariencia (8.3) y la resolución de hechos geométricos (8.5).
- Un conflicto para el que aún no se haya definido ningún método hace fallar toda la interpretación (1.5 punto 3; 7.6). Esta versión no tiene ningún caso así: un ciclo de conjuntos falla en la sentencia que lo cierra (5.2.2).

### 7.6 Diagnósticos

- **Comprobar e interpretar son un solo proceso.** Un intérprete lee la entrada y devuelve la escena resultante junto con un único conjunto de diagnósticos; cuando se usa solo para comprobación, también se denomina lint. Este estándar define únicamente ese conjunto de diagnósticos. Las comprobaciones locales que realiza un editor para ofrecer respuesta inmediata, como el resaltado de sintaxis, no son diagnósticos en este sentido: esta sección no se les aplica y no pueden reemplazar a los diagnósticos.
- **Elementos.** Un fallo se aplica a un elemento: un campo de un encabezado del registro, una sentencia, un registro, un archivo o toda la interpretación. Cuál de ellos es se indica donde se define cada fallo (véase el alcance de un fallo abajo).
- **Cuatro clases de informe**:
  - **Error**: un elemento falla, ya resida la causa en el propio elemento, en otro elemento o aguas arriba en algo de lo que depende. Los errores incluyen los errores básicos de Principios 2.4 (errores de sintaxis, palabras desconocidas, una firma que no coincide, una referencia no encontrada, una clase incorrecta, autocontradicción, una versión no admitida), los demás fallos definidos en este estándar (como ORIENT aplicado a un uso sin símbolo, o un terminal sin pin en su símbolo; 12.1, 9.2.4) y los fallos derivados de ellos (7.2).
  - **Advertencia**: el elemento es válido y surte efecto como de costumbre, salvo donde este estándar indique lo contrario (como en el caso de un hecho descartado porque su comprobación no terminó dentro del límite del solucionador, 8.5), pero puede no ser lo que el autor pretendía, como un conjunto vacío escrito explícitamente para una ranura que toma objetos individuales, como en `{} @PLACE(1, 2)` (5.2.1); una definición sin nombre escrita directamente como miembro de un registro, sin anidar en una sentencia (9.1); un rol de una regla de generación que alcanza los niveles inferiores (10.1), rellenado con un objeto de su propio nivel que esos niveles no tienen (10.5); un campo desconocido en un encabezado del registro (2.8); una versión de paquete reemplazada por otra (16.2); un cable conectado a un terminal que no está en la red que nombra el cable (13.1); un hecho geométrico descartado porque su comprobación no terminó dentro del límite del solucionador (8.5).
  - **Indicación**: una sustitución, ocultación o resolución habitual que también puede ser una equivocación, como un nivel inferior, o un paquete usado en él, que oculta una definición de un nivel superior o de los paquetes en la raíz (6.8); un paquete de un USE-PACKAGE posterior que oculta otro de un USE-PACKAGE anterior (16.2); un hecho geométrico descartado en la resolución de conflictos (8.5); la última versión de un paquete usada porque no se escribió ninguna (16.2); un archivo principal sin versión declarada, leído con la versión por defecto de la herramienta (16.1); nombres del paquete estándar no encontrados donde el paquete estándar no está en uso (16.3); un registro del solucionador cuyos números están desactualizados (5.9).
  - **Violación**: una afirmación que hace el archivo sobre el circuito no se cumple: una aserción cuya comprobación no se cumple (15.3, 15.4), o las sentencias CURRENT de una red que envían toda su corriente en un mismo sentido (15.2). Nada falla, y la comprobación y su informe no modifican nada en el dibujo: una aserción se dibuja como si no estuviera escrita, y las sentencias CURRENT que lee la comprobación siguen surtiendo efecto. El informe es para el autor y para las herramientas que comprueban un archivo. Un visor puede mostrarlo con un nombre propio.
- **El alcance de un fallo** se indica donde se define cada fallo: un campo (un campo conocido de un encabezado del registro mal escrito se trata como no escrito, 2.8); una sentencia (la mayoría de los casos); un registro (una cadena o comentario sin cerrar, llaves que no coinciden, un registro interrumpido, un destino de `scope` no encontrado; 7.4, 2.4, 2.6); un archivo (una versión no admitida, 16.1); toda la interpretación (un conflicto para el que aún no se ha definido ningún método, 1.5 punto 3: la interpretación se detiene y no se dibuja nada; una versión posterior podría hacer tolerante tal caso; esta versión no tiene ninguno).
- **Requisitos**:
  - Los errores deben informarse, y también deben informarse las violaciones: informar es la razón por la que se escribe una aserción (15.3). Las advertencias y las indicaciones son las enumeradas en este estándar, y las implementaciones deberían informarlas.
  - Una implementación puede añadir comprobaciones propias, pero solo como advertencias o indicaciones; no deben hacer fallar un elemento, pues de lo contrario implementaciones distintas dibujarían resultados distintos.
  - Una implementación debe permitir al usuario ver los diagnósticos; cómo se presentan queda a criterio de la implementación (Principios 2.9).
  - Una implementación puede ser más estricta, como detenerse o rechazar toda la entrada (Principios 2.9), pero este no debe ser su comportamiento por defecto. Puede permitir al usuario elevar o rebajar la gravedad de las advertencias, indicaciones y violaciones; los errores no pueden rebajarse, puesto que un error es un fallo.
  - Las pruebas de conformidad comprueban los errores (su ubicación y el alcance del fallo), las violaciones (qué comprobación no se cumple y para qué miembros) y la escena resultante; las advertencias pueden ser expectativas opcionales; las indicaciones no se prueban.
  - Nota: las advertencias conjeturan qué pretendía el autor y cambiarán con la experiencia; convertirlas en requisitos congelaría esas conjeturas en el estándar. Principios 2.4 también exige únicamente los errores básicos.
- **Contenido**: un diagnóstico proporciona
  - la ubicación: el archivo, la línea y columna o intervalo; qué registro, sentencia, campo o referencia (Principios 2.8);
  - la causa: qué se esperaba y qué se recibió, como "in_p toma una instancia, se recibió una red" (9.4); cuando no se encuentra un nombre, el nombre más cercano;
  - la causa de origen: cada sentencia que falla como consecuencia indica su causa de origen, y una presentación puede agrupar tales sentencias bajo la causa raíz, para que una errata no produzca docenas de errores por todo el dibujo. Cuando una misma regla falla en muchos objetos, los fallos pueden combinarse en un único diagnóstico que enumere los objetos, como cuando un paquete escribe `W/L=${w}/${l}` para todo MOS y algunas instancias carecen de `w`;
  - el origen: para una sentencia expandida a partir de una definición general, tanto la aplicación como la sentencia original en la definición; para una producida por una regla, la regla y el nivel donde se evaluó; para una en un paquete, el nombre, la versión y el archivo del paquete (16.2).
- La redacción y el idioma de los diagnósticos no se especifican. Esta versión no tiene códigos de diagnóstico estándar; las implementaciones pueden numerar sus diagnósticos, y los números no definen clases de tolerancia.
- **Paquetes**: en un documento que utiliza un paquete, los errores en el contenido del paquete se informan como de costumbre y se atribuyen al paquete; las advertencias e indicaciones sobre el contenido del paquete se emiten únicamente cuando se comprueba el paquete por separado. Las violaciones de las aserciones que escribe un paquete se informan en el documento que lo utiliza, pues atañen a su circuito, y nombran la aserción en el paquete y el uso o regla que la aplicó. Nota: los usuarios no pueden modificar un paquete, y reglas amplias que su autor escribió a propósito los inundarían de advertencias de otro modo.
- Un archivo no puede suprimir diagnósticos ni pedir un modo más estricto.
- Las palabras clave, parámetros y valores desconocidos son errores, igual que los errores de sintaxis (Principios 2.3): la sentencia falla (16.4), salvo en SOLVER-CONFIG, donde un valor que no se admite hace fallar solo su clave (14.10). En esta versión, el único contenido que se reconoce pero no se interpreta, y produce solo una advertencia, es un campo desconocido de un encabezado del registro (2.8) y una clave de SOLVER-CONFIG que el solucionador desconoce (14.10).

<!-- translated from en/ch08.md sha256:bf7d27b5693d -->

## 8 Entrada en vigor

### 8.1 Un nivel surte efecto después de leerse completamente

- El intérprete analiza el netlist anfitrión completo y lee todos los registros; después cada nivel surte efecto en las fases de 8.2.
- Dentro de un nivel, el orden de las sentencias importa únicamente según indica 1.5 punto 2; no afecta a cómo se resuelven los nombres y las referencias (Principios 2.9).
- Cuál de dos niveles prevalece sobre el otro se decide por la jerarquía de niveles, no por el orden del texto (Principios 2.9; 8.3).

### 8.2 Fases

1. **Lectura**: la estructura léxica, la gramática y las firmas.
2. **Nombres y definiciones**: se resuelven los nombres `$`, los nombres `%` y las referencias al anfitrión; se evalúan los cuerpos de DEFINE-SYMBOL y DEFINE-STYLE.
3. **Creación de objetos**: se crean los objetos de la escena escritos explícitamente (puntos, cables, etiquetas, agrupaciones, gráficos, restricciones, usos) y los conjuntos, y se expanden los cuerpos de las definiciones generales; las reglas de un cuerpo se incorporan a las reglas del nivel del uso (9.4). Las selecciones no se expanden aquí.
4. **Cola de símbolos**: SET-SYMBOL surte efecto y asigna símbolos a los objetos del paso 3. Solo ahora se evalúan en cada nivel las selecciones de las reglas en esta cola.
5. **Árboles**: las reglas de ROUTE-NET, ROUTE-OBJECT, UNROUTE-NET y UNROUTE-OBJECT (10.4, 13.7) se evalúan una vez en cada nivel, en el orden: paquetes, niveles superiores, este nivel. Sus condiciones leen los árboles de las sentencias del paso 3 y no se ven entre sí; los árboles que crean se ven a partir del paso 6 (10.2).
6. **Generación uno por uno**: AS y ATTACH cuyos objetos son el resultado de una selección, TEXT adjuntado al resultado de una selección (10.5, 13.3), RECTANGLE, CIRCLE y ELLIPSE escritos con un tamaño cuyo punto es el resultado de una selección (9.2.5), las reglas de LINE, ARC y BEZIER, y las reglas de CURRENT (10.4) se evalúan una vez en cada nivel, en el mismo orden.
7. **Símbolos para objetos generados**: la misma cola de símbolos se ejecuta una vez más para los objetos nuevos, con las sentencias y reglas SET-SYMBOL del contenido generado (10.5). Los símbolos de los objetos del paso 3 quedan fijados tras el paso 4, puesto que los árboles y `wired` se calculan a partir de ellos (10.2): un SET-SYMBOL del contenido generado que alcance a uno de esos objetos no tiene efecto sobre él, y se emite una advertencia. La generación ve los símbolos tal como estaban antes de la generación.
   Las aserciones (15.3) se comprueban aquí, una vez conocidos los símbolos.
8. **Geometría**: PLACE, ORIENT y las restricciones se incorporan uno a uno en el orden de resolución, y el que entra en conflicto con los ya incorporados se descarta (8.5, 14.8); después se resuelve la disposición. Este paso necesita las posiciones de los pines y los marcos de los símbolos (9.2.3), por lo que va después de la asignación de símbolos; ORIENT aplicado a un uso sin símbolo falla aquí, mientras que PLACE y las restricciones toman el punto propio del uso (14.1). Los objetos sin ninguna posición ni restricción los coloca el solucionador, que escribe de vuelta sus posiciones (14.8).
9. **Cola de estilos**: SET-STYLE surte efecto, con los dos rangos de 8.3. Solo ahora las propiedades de estilo que los gráficos de un símbolo dejan sin escribir, y los colores de `fill=`, toman valores concretos del estilo del objeto propietario.
10. **Representación**.

- Estas fases son el orden interno de 1.5 punto 4: una descripción lee el estado previo al comienzo de su fase; no se incluyen ni sus propios efectos ni los de las demás descripciones de la misma fase.
- Un conjunto se evalúa en la fase que lo utiliza (5.2): el mismo conjunto no contiene objetos generados cuando se utiliza en el paso 6, y sí los contiene cuando se utiliza en el paso 9.
- Las condiciones de selección leen únicamente información fijada en fases anteriores:
  - la información del anfitrión, las clases de los objetos, los usos, las direcciones de corriente escritas como sentencias sobre terminales y los cables explícitos quedan fijados en el paso 3; las direcciones que fijan las reglas CURRENT (paso 6) y CURRENT sobre pines (una vez asignados los símbolos) se ven a partir del paso 7, por ejemplo en la cola de estilos y en la comprobación de 15.2;
  - una selección en la cola de símbolos no puede seleccionar por el símbolo que utiliza un objeto;
  - las reglas de generación sí pueden ver los símbolos.
- Si un terminal tiene un pin en su símbolo se comprueba tras el paso 7 (7.1). Una sentencia que falla en una comprobación hecha después de asignados los símbolos, como una aplicación cuyo rol de clase `"pin"` recibe un terminal sin pin, participó en los pasos anteriores tal como fue escrita: esos pasos no se vuelven a calcular y desde ese momento no tiene efecto. Los cables se cuentan del mismo modo (10.2).
- Este estándar no especifica los métodos de resolución ni de representación; únicamente sus entradas (Principios 1).

### 8.3 Colas de apariencia

- Hay dos colas: la cola de símbolos toma SET-SYMBOL, y la cola de estilos toma SET-STYLE.
- Para un nivel, cada cola consta de estos grupos, en este orden:
  1. las reglas de los paquetes en la raíz: primero el paquete estándar, luego los demás paquetes de USE-PACKAGE en registros de encabezado (16.3); cuando un paquete vuelve a usarse con USE-PACKAGE en este nivel o en un nivel superior (en un nivel de archivo o un subcircuito), se sitúa en la posición de ese USE-PACKAGE en su lugar (16.2);
  2. los símbolos que los subcircuitos asignan a su propio `#`, es decir, sus símbolos de bloque (solo cola de símbolos; 9.2.6);
  3. las reglas de los niveles superiores, de fuera hacia dentro;
  4. todas las sentencias SET-SYMBOL o SET-STYLE escritas en este nivel, tanto reglas como sentencias sobre objetos individuales;
  5. SET-SYMBOL o SET-STYLE escritos en este nivel desde fuera con `scope`, los de niveles más externos más tarde (2.6).
- Dentro de cada grupo, el orden es el orden del texto. El contenido de otro paquete se sitúa en la posición de su USE-PACKAGE (16.2): un paquete usado en un nivel superior va con ese nivel, y uno usado en este nivel va en la posición del USE-PACKAGE en el texto de este nivel. Los SET-SYMBOL y SET-STYLE producidos por la generación uno por uno pertenecen al grupo de la regla que los genera, en la posición de esa regla.
- **Dos rangos**:
  - Directo: SET-SYMBOL y SET-STYLE que nombran directamente a este objeto, es decir, cuya entrada es su manejador, su nombre o un camino `::` hacia él, o un conjunto escrito entre `{…}` que lo enumera como miembro (también cuando ese conjunto tiene nombre). Los parámetros de estilo escritos al definir el objeto (9.2.5, 9.3) también cuentan, como si se hubiera escrito un SET-STYLE para el objeto en la sentencia que lo crea.
  - Transmitido o seleccionado: lo que un miembro recibe a través de una selección o mediante generación uno por uno; lo que un objeto propietario transmite a los gráficos de su símbolo (9.2.5); lo que un objeto transmite a los textos adjuntados a él (11.3); lo que un uso transmite a su símbolo y a los objetos dibujables creados en su cuerpo, y así sucesivamente a través de usos anidados, pero no a los miembros que rellenan sus roles (12.1 punto 4). Un hecho transmitido conserva la posición de su sentencia original.
  - Para cada propiedad (en la cola de símbolos, el símbolo), se consideran primero los hechos directos y se toma el último por posición; si no hay ninguno, se consideran los hechos transmitidos o seleccionados y se toma el último por posición; si no hay ninguno de ninguno de los dos, una instancia toma el símbolo de bloque por defecto (9.2.6), un uso no se dibuja (12.1) y una propiedad de estilo toma el valor por defecto del renderizador (9.3). Las posiciones siguen el orden de la cola anterior.
  - Así, un símbolo o estilo que un editor modifica para un objeto no queda sustituido por reglas escritas después; las reglas, paquetes y temas siguen rigiendo los objetos que no se modificaron individualmente.
- El `type` y el `author` de un encabezado del registro no afectan al orden (2.8).
- Un símbolo se reemplaza en bloque; los estilos se fusionan propiedad por propiedad. Aplicar un estilo equivale a escribir una vez cada propiedad que define, por lo que un estilo que define únicamente `linewidth` modifica únicamente el grosor de línea.
- Las reglas se aplican al nivel donde están escritas y, cuando los alcanzan, mediante `recursive=true` en una selección escrita en ellas o, si no tienen ninguna, mediante los conjuntos que usan (10.1), a los niveles inferiores, nunca a los niveles superiores; las reglas de un paquete cuentan como escritas en el nivel de su USE-PACKAGE, y las de un paquete en la raíz en el nivel superior de su archivo (10.4, 16.2). Se evalúan una vez en cada nivel, atendiendo únicamente a los objetos de ese nivel.

### 8.4 Sustitución por clave

Los hechos con clave (5.8) se sustituyen entre sí por clave: surte efecto el posterior (Principios 2.9). Sustituir es una forma legítima de describir, no un error; emitir o no una indicación queda a criterio de la implementación. PLACE y ORIENT son hechos geométricos y se resuelven según 8.5 (5.8).

### 8.5 Conflictos

Los hechos geométricos en conjunto pueden no tener solución aun cuando no haya dos que se contradigan entre sí, como `{#M1 #M2} @ALIGN("y")`, `#M1 @PLACE(y=0)` y `#M2 @PLACE(y=10)`. Lo siguiente decide qué hechos se descartan.

- **Los hechos geométricos** son las cuatro restricciones atómicas (ALIGN, ORDER, DISTRIBUTE, DISTANCE), la relación de orientación ORIENT-RELATION (14.9), TABLE (14.11), cada coordenada de PLACE, `dx` y `dy` de TEXT (13.3; en el mismo eje estos y PLACE se reemplazan entre sí, lo cual no se resuelve aquí) y ORIENT. Los símbolos (SET-SYMBOL) no están entre ellos; se fijan antes de la geometría (8.2). Tampoco están los criterios propios de la implementación (que los dispositivos no se solapen, una cuadrícula, etc.): son consideraciones propias del solucionador y se sitúan después de todas las descripciones del lenguaje. Si las descripciones obligan a los dispositivos a solaparse, se solapan.
- **Unidades de resolución**:
  - Una sentencia de restricción atómica es una unidad, descartada en bloque en un conflicto (ORIENT-RELATION también); nunca se conserva solo para algunos de sus miembros. Esto concuerda con los fallos: los miembros de una restricción atómica son entradas estrictas (7.3). Cuando una ranura que toma un único objeto recibe un conjunto y la sentencia trabaja una vez por cada miembro, cada una de esas descripciones es una unidad (5.2.1); y lo mismo cada restricción que representa una lista de miembros, una por cada combinación (14.1) o cada par de DISTANCE (14.5). Un TABLE es una unidad (14.11).
  - Un PLACE son dos unidades, una para x y otra para y, que pueden descartarse por separado. `dx` y `dy` de TEXT son igualmente una unidad cada uno; primero reemplazan a un PLACE del mismo texto en el mismo eje, o son reemplazados por él, según posición (13.3), y los restantes participan en la resolución.
  - ORIENT es una unidad: la orientación de este objeto es igual a un valor, o a uno de los valores de su lista (11.2); un ORIENT de un pin, que la orientación de su objeto es una de las que giran el pin hacia la dirección, o una de las direcciones, escritas. Sin ORIENT no hay restricción: el solucionador elige y escribe la orientación de vuelta (14.8); un renderizador que no tenga solución para él dibuja `"R0"`. Una orientación es una variable con ocho valores a partir de la cual se calculan las posiciones de los terminales; la comprobación de conflictos puede probar las orientaciones directamente y no necesita reescribirlas como posiciones relativas de terminales.
  - Una definición general no es una restricción: cada hecho expandido de un uso es una unidad, ordenada y descartada por separado.
- **Hechos que se contradicen a sí mismos**: cada hecho se comprueba primero por separado. Un hecho que no tiene solución por sí solo, como un DISTANCE de un objeto a sí mismo con `min=4` (9.5), se contradice a sí mismo: falla (7.6) y no participa en la resolución. Solo los hechos que tienen solución por sí solos se ordenan e incorporan.
- **Orden de resolución**: los hechos geométricos se colocan en un único orden. Se ordenan primero por `priority`, `"high"` por encima de `"normal"` por encima de `"low"` (14.8); a igual prioridad, por posición, primero el posterior. Posiciones:
  - por nivel y orden del texto: los paquetes en la raíz (16.3), el contenido de los niveles superiores, las sentencias de este nivel, las sentencias escritas en este nivel desde fuera con `scope` (las de niveles más externos más tarde, 2.6); el contenido de otro paquete está en la posición del USE-PACKAGE más cercano a este nivel (16.2);
  - los hechos expandidos de una definición general toman la posición de la sentencia AS o ATTACH, y los hechos generados uno por uno toman la posición de la regla de generación (10.5);
  - cuando una sentencia produce varios hechos, los de una expansión siguen el orden del cuerpo de la definición, los de una sentencia que trabaja una vez por cada miembro siguen el orden de los miembros, y para PLACE x va antes que y; en cada caso va primero el posterior. Los segmentos de una tubería son descripciones propias, y el segmento posterior está escrito después (5.5): los hechos de `#R1 @PLACE(0, 0) => @PLACE(5, 5)` van en el orden y = 5, x = 5, y = 0, x = 0, y en `#M1 @PLACE(0, 0) => @ORIENT("R90")` el ORIENT va antes que el PLACE.
- **Resolución**: los hechos se incorporan uno a uno en el orden de resolución; después de cada uno, se comprueba si los hechos incorporados hasta ese momento tienen solución y, si no la hay, este hecho se descarta. El resultado es único: es el subconjunto consistente lexicográficamente máximo en este orden. Los hechos tienen solución cuando existen coordenadas reales, y para cada orientación libre uno de sus ocho valores, que los satisfacen todos dentro de la tolerancia (11.7); los puntos enteros son solo un objetivo del solucionador.
- **Límite**: una comprobación de solución puede tardar mucho, porque algunos hechos se cumplen de una entre varias maneras que deben probarse sucesivamente: una lista de orientaciones (11.2, 14.9) y una orientación libre. Por tanto, una implementación debería fijar un límite adecuado sobre el tiempo o el trabajo que dedica a comprobar, para que un circuito grande no tarde demasiado. El límite es una opción del solucionador que el usuario puede configurar (14.8), dada por la herramienta y no por el archivo (14.10); este estándar no fija cómo se mide ni su valor. Cuando la comprobación de un hecho no termina dentro del límite, el hecho se descarta, con una advertencia que indica que no fue decidido (7.6); el resultado puede entonces diferir del definido arriba, y de una ejecución a otra. Sin límite, y siempre que ninguna comprobación lo alcance, el resultado es el definido arriba.
- Los hechos incorporados son todos obligatorios para el solucionador; los descartados no tienen más efecto. Los hechos con prioridad `"disabled"` se eliminan antes de ordenar (14.8).
- Nota: esta sección fija qué hechos se conservan, no cómo se encuentran. Una implementación puede comprobar toda una serie de hechos a la vez y buscar el hecho que descartar solo cuando la serie no tenga solución, o comprobar únicamente los hechos que comparten objetos con el nuevo, siempre que conserve los mismos hechos.
- Una implementación que resuelve debe resolver los conflictos según especifica esta sección, y su resultado debe satisfacer todos los hechos incorporados. Una implementación que no resuelve dibuja únicamente la geometría explícita y no está obligada a comprobar conflictos entre sentencias (Principios 2.4).
- Descartar es una resolución legítima y produce una indicación, que una implementación puede elevar a advertencia. Cuando se emite, la indicación nombra todo conjunto de hechos incorporados que no tenga solución junto con el hecho descartado y del que no pueda omitirse ningún hecho, cada hecho por la sentencia que lo escribió (7.6). Cada sentencia cuyos hechos estén en uno de esos conjuntos también recibe una indicación que nombra los hechos descartados por su causa, para que el autor vea qué sentencias perdieron su efecto a través de ella. Cuando encontrar todos esos conjuntos no termina dentro del límite de tiempo, la indicación señala que su lista no está completa. Descartar un hecho `"low"` es previsible, y sus indicaciones pueden omitirse (7.6): una implementación no necesita enumerar los conjuntos de hechos con los que entra en conflicto ni dar una indicación a las sentencias en ellos.
- Un conflicto no es un fallo (7.5). No es obligatorio comprobar conflictos de significado de alto nivel entre sentencias ni la concordancia con el netlist (Principios 2.4).

<!-- translated from en/ch09.md sha256:6683c3ad47d6 -->

## 9 Definiciones

### 9.1 Reglas comunes

- Hay tres clases de definición: DEFINE-SYMBOL crea un símbolo, DEFINE-STYLE crea un estilo y DEFINE crea una definición general. Los símbolos y los estilos son las definiciones específicas (1.4.4). Cada uno toma un conjunto como cuerpo; las sentencias de un cuerpo son tolerantes una por una (7.3).
- Una definición es un objeto. Se nombra con `$name =` o `%name =` (6.6) y se utiliza como objeto de una palabra clave, en la ranura que la palabra clave fija para ella (5.6).
- Consignar una definición únicamente publica su nombre. Una definición no dibuja nada por sí sola y surte efecto únicamente cuando se utiliza: un símbolo cuando SET-SYMBOL lo asigna, un estilo cuando SET-STYLE lo asigna, una definición general cuando AS o ATTACH la aplica.
- Una definición escrita directamente como miembro de un registro, sin anidar en una sentencia y sin nombre, nunca se utiliza y produce una advertencia. Una definición anónima anidada en una sentencia se utiliza como de costumbre.
- **Inclusión**: una referencia a una definición de la misma clase en el cuerpo de DEFINE-SYMBOL o DEFINE-STYLE incluye su contenido en ese lugar. Los miembros con clave escritos después sustituyen a los escritos antes; los miembros sin clave se acumulan. No existe parámetro `base=`. Una inclusión que traiga de vuelta, directamente o mediante otras inclusiones o mediante los símbolos de usos dibujados en un cuerpo de símbolo (9.2.7), la definición cuyo contenido se está construyendo cierra un ciclo: esa inclusión falla, su diagnóstico enumera la cadena y el resto del cuerpo surte efecto. El contenido de una definición se construye partiendo de esa definición, de modo que si `$A` incluye `$B` y `$B` incluye `$A`, `$A` es su propio contenido y el de `$B` sin `$A`, y `$B` a la inversa.
- Las definiciones generales no se incluyen unas a otras; se combinan mediante AS y ATTACH en sus cuerpos. La derivación y la extensión funcionan del mismo modo; no hay relación de herencia (9.4).

### 9.2 Símbolos: DEFINE-SYMBOL

#### 9.2.1 Evaluación

- Un símbolo es un valor: su cuerpo se evalúa donde se define, y las sentencias en él producen pines y gráficos en las coordenadas locales del símbolo. Estos pasan a ser el contenido de la definición y no se consignan en la escena.
- El orden de los miembros es el orden de dibujo.
- Las coordenadas locales tienen la misma unidad y los mismos ejes que la escena (x hacia la derecha, y hacia arriba), y (0, 0) es el origen del símbolo (6.5); esta versión no tiene escalado (11.7). Un punto en el cuerpo debe dar números en ambos ejes; no puede usarse `_`.

#### 9.2.2 Sentencias en el cuerpo

- El cuerpo puede contener únicamente gráficos, pines, puntos locales nombrados (como `%tip = (0, 0.7)`), inclusiones de otros símbolos, usos aplicados solo para dibujar (con ATTACH, o AS, 9.4) junto con el SET-SYMBOL y ORIENT escritos sobre ellos (9.2.7), y TEXT adjuntados al objeto propietario `?`, es decir, el objeto dibujado con este símbolo, junto con el PLACE que posiciona tal texto (13.3).
- Cualquier otra sentencia falla y se descarta (el cuerpo es tolerante); las referencias al anfitrión también fallan.
- Los pines pueden escribirse únicamente en un cuerpo de DEFINE-SYMBOL. Los gráficos también pueden escribirse en la escena, en coordenadas de escena; allí son anotaciones (13.4).

#### 9.2.3 Parámetros

- `box=[x1, y1, x2, y2]`: el marco del símbolo, en coordenadas locales y alineado con los ejes; se transforma con la orientación de la instancia. El lenguaje en sí no lo utiliza; los visores suelen usarlo como caja de selección y los solucionadores como límite para mantener separados los símbolos, y cómo se utilice queda a criterio de la implementación. Cuando no se escribe, es el cuadro delimitador de todos los gráficos del símbolo salvo el texto, pues el tamaño del texto depende de la fuente y del renderizador.

#### 9.2.4 Pines

```rnl
$RES_BOX = @DEFINE-SYMBOL {
  %p = (0, 3) @PIN(terminal=1, label="P", direction="up");
  %n = (0, -3) @PIN(terminal=2, label="N", direction="down");
  @LINE {%p (0, 1.5)};
  @LINE {(0, -1.5) %n};
  (0, 0) @RECTANGLE(width=1.2, height=3)
}
```

- **Un pin es un objeto del símbolo**: cada sentencia PIN crea un pin, un punto que puede vincular terminales. En el cuerpo, un pin nombrado puede usarse como punto, como las patillas anteriores empiezan en `%p` y terminan en `%n`. Su nombre local es también su nombre como subobjeto de cada objeto dibujado con el símbolo, como `#R1::p` (6.5); un pin sin nombre local no tiene nombre y se alcanza a través del terminal que vincula (5.2.4).
- **Los nombres se vinculan una sola vez**: un nombre de pin se vincula una sola vez en un símbolo, contados los pines de los símbolos incluidos (9.2.7, 6.6); un nombre vinculado a dos pines es un error. Los pines no se reemplazan entre sí: toda sentencia PIN crea un pin propio.
- Los pines deberían colocarse en puntos enteros (11.7): cuando la instancia se coloca en un punto entero, sus pines y los cables conectados a ellos caen también en puntos enteros. Otras posiciones no son un error; una implementación puede emitir una indicación. Los gráficos de un símbolo no están sujetos a esto.
- `terminal=`: el terminal que vincula el pin, un nombre o un número; estos son los nombres de terminal del símbolo. SET-SYMBOL los empareja con los terminales del objeto (11.3): un nombre con el terminal de ese nombre, dado por DEVICE (6.12) o el nombre de puerto del subcircuito, un número con el terminal en ese lugar. Para un uso, que no tiene terminales, un nombre se empareja con el rol de ese nombre en su definición, y un número no vincula ningún rol (11.3, 12.1). `terminal=` puede omitirse: el pin no vincula nada y solo se dibuja, como los contactos de un interruptor dibujado como figura.
- **Varios pines de un mismo terminal**: un terminal puede dibujarse en varios pines, como una alimentación a ambos lados de un bloque grande. Cada uno es un pin propio y se alcanza por su nombre; donde se escribe el terminal en lugar de un pin, representa al pin escrito en último lugar (5.2.4). Cada pin está cableado o no por separado (10.2).
- **Pines de bus**: `terminal=` puede tomar una matriz, dando un pin para varios terminales, como un puerto de bus de un subcircuito: `(-4, 0) @PIN(terminal=["D<3>", "D<2>", "D<1>", "D<0>"], label="D<3:0>", direction="left")`. La matriz tiene al menos un elemento, y cada elemento es un número o un nombre. Las cadenas en parámetros no expanden subíndices (6.11), por lo que los nombres se escriben uno por uno.
  - Todos estos terminales se dibujan en este pin: un cable o restricción conectado a cualquiera de ellos está conectado a este pin, y el pin está cableado (10.2).
  - Varios pines en el mismo punto no son un sustituto: coordenadas iguales no conectan (13.1), un cable conecta solo con uno de los pines, los demás permanecen `wired=false`, y las reglas de relleno del paquete estándar les añadirían etiquetas y tierras.
- `label=`: el texto mostrado en el pin, solo para visualización. No es un nombre: al pin se hace referencia por el nombre al que está vinculado en el símbolo, como `p` en `%p = (0, 3) @PIN(…)`, escrito `#R1::p`, y al terminal por su nombre de terminal, como `#R1:1`; `label="P"` no da ni `#R1::P` ni `#R1:P` (6.4, 6.5). Si se muestra y dónde queda a criterio del visor. Puede usar marcado de texto, como `\overline{CS}` (13.6).
- **Un terminal que no se dibuja no tiene pin.** Un terminal sin pin carece de significado para el dibujo: un cable o restricción conectado a él falla ("el símbolo no tiene pin para este terminal"); y el relleno no lo selecciona (10.1). No hay notación para ocultar un terminal que tiene pin; para omitir el sustrato, úsese un símbolo de tres terminales sin pin de sustrato.
- `direction=`: la dirección en que sale un cable del pin: `"right"`, `"up"`, `"left"` o `"down"`. Se escribe en las coordenadas locales del símbolo y gira con la orientación (11.2). Es una pista para solucionadores y visores, como la dirección en que el trazado libre sale del pin, el sentido en que circula la corriente por el terminal (15.2), o una referencia al elegir orientaciones; por sí sola no es una restricción y no participa en conflictos, pero ORIENT y ORIENT-RELATION escritos sobre el pin la restringen (11.2, 14.9). Puede omitirse; entonces el pin no tiene dirección: una implementación puede inferir una a partir de los gráficos como indicación, pero ORIENT y ORIENT-RELATION sobre ese pin fallan.
- No hay longitud de patilla: las patillas se dibujan en el símbolo con LINE.
- No hay tipo eléctrico: los pines y terminales no tienen parámetros como entrada o salida, y el netlist tampoco contiene tal información. Donde sea necesaria, los usuarios la expresan mediante usos de definiciones semánticas; un exportador a herramientas como Virtuoso asigna definiciones generales a tipos eléctricos según su propia configuración.
- Un pin es solo un punto: si se dibuja una patilla depende del símbolo, y cómo se marca el punto de un pin (un cuadradito, un punto o nada) queda a criterio del visor. Por tanto, un pin sin patilla no necesita notación especial: escríbase PIN y ningún LINE. Por ejemplo, un dibujo cuyos cables conectan con un pin de sustrato que su editor oculta se importa como un símbolo que conserva el PIN de sustrato y no dibuja patilla de sustrato. Los símbolos ordinarios, incluidos los del paquete estándar, dibujan patillas.

#### 9.2.5 Gráficos

Las posiciones son objetos punto; los escalares son parámetros. En la tabla, *punto* representa cualquier objeto punto.

| Forma | Significado |
|---|---|
| `@LINE {point point …}` | Dos puntos forman un segmento de línea, más forman una polilínea; cuando el último punto está en la misma posición que el primero, la figura es cerrada y la unión se dibuja como una esquina |
| `@RECTANGLE {point point}` | Un rectángulo; los dos puntos son esquinas opuestas, en cualquier orden |
| `(0, 0) @RECTANGLE(width=4, height=2)` | Un rectángulo; el punto es el centro, y `width` y `height` son su anchura y altura |
| `@CIRCLE {point point}` | Un círculo; el primer punto es el centro y el segundo está sobre la circunferencia |
| `(0, 0) @CIRCLE(radius=1)` | Un círculo; el punto es el centro |
| `@ELLIPSE {point point}` | Una elipse; los dos puntos son esquinas opuestas de su cuadro delimitador |
| `(0, 0) @ELLIPSE(width=4, height=2)` | Una elipse; el punto es el centro, y `width` y `height` son los de su cuadro delimitador |
| `@ARC {point point point}` | Un arco circular desde el primer punto pasando por el segundo hasta el tercero |
| `@BEZIER {point point point point …}` | Una curva de Bézier cúbica de 3n+1 puntos: el punto inicial, luego para cada segmento dos puntos de control y un punto final; los segmentos vecinos comparten sus puntos finales |
| `(1, 0) @TEXT("…")` | Un texto adjuntado a un punto: texto fijo del símbolo, que no puede cambiar de una instancia a otra |
| `? @TEXT("${name}", key="name") => @PLACE(1, 0)` | Un texto adjuntado al objeto propietario (el objeto dibujado con este símbolo) que lee sus atributos; su clave es `name`, por lo que para M1 es `#M1::name`, y un texto con la misma clave en la escena lo sustituye parámetro por parámetro (13.3) |

- **Dos formas**: RECTANGLE, CIRCLE y ELLIPSE se distinguen por si se escribe un parámetro de tamaño. Con un tamaño (`width` y `height` para RECTANGLE y ELLIPSE, `radius` para CIRCLE), la ranura de objeto toma un punto, el centro; dado un conjunto, dibuja una figura por cada punto (5.2.1). Dado el resultado de una selección, como `# @SELECT-PIN => @RECTANGLE(width=0.4, height=0.4, fill="linecolor")`, se genera uno por uno en el paso 6 de 8.2, como TEXT, y por tanto ve los símbolos; un gráfico escrito con dos puntos no admite una selección (10.4): cuántos puntos da una selección no se sabe al leerlo, y la forma toma exactamente dos. Sin tamaño, la ranura de objeto toma un conjunto de dos puntos, y un número distinto de puntos hace fallar el gráfico. Un tamaño escrito solo en parte es un parámetro erróneo, y el gráfico falla. En la forma de dos puntos, los puntos son objetos: pueden nombrarse o ser las posiciones de pines, y la figura los sigue.
- Los rectángulos y las elipses están alineados con los ejes; un rectángulo inclinado se dibuja con un LINE cerrado.
- **Relleno**: todo gráfico salvo TEXT admite `fill=`. Un LINE, ARC o BEZIER abierto se rellena como la región cerrada por una línea recta desde su último punto hasta el primero, y esta línea de cierre no se traza; un ARC relleno es el segmento delimitado por la cuerda y el arco. Una figura rellena pero sin trazo (la puerta de un MOS, una punta de flecha, un punto de unión) se escribe `fill="linecolor", linestyle="none"` (9.3).
- **Puntas de flecha**: LINE, ARC y BEZIER admiten `arrow=`: `"none"` (por defecto), `"start"`, `"end"` o `"both"`, como `@LINE(arrow="end") {%from %to}`. La punta está en el punto inicial o final y apunta en la dirección de la línea en ese extremo: para LINE, el primer o el último segmento; para ARC, la tangente del arco en el punto final; para BEZIER, la recta desde el punto de control vecino al punto final (cuando ambos coinciden, el siguiente punto que no coincida). La longitud de la punta de flecha viene dada por `arrowsize` (9.3), y su forma queda a criterio de la implementación. Los puntos extremos son objetos punto; cuando las restricciones los mueven, la punta de flecha los sigue, en cualquier ángulo, mientras que un uso de flecha dibujado en un punto tiene solo ocho orientaciones. WIRE no admite `arrow=`.
- **Parámetros de estilo**: los gráficos pueden tomar propiedades de estilo directamente como parámetros (9.3): LINE, ARC y BEZIER admiten `linecolor`, `linestyle`, `linewidth`, `cap`, `join`, `fillcolor` y `arrowsize`; RECTANGLE, CIRCLE y ELLIPSE admiten las seis primeras; TEXT admite `textcolor`, `size`, `font`, `bold`, `italic` y `textvisible`. Las escritas son el estilo directo del gráfico, que tiene rango superior al estilo transmitido por el objeto propietario (8.3), por lo que un color escrito en un símbolo no cambia con el tema; las no escritas proceden del estilo del objeto propietario, por lo que cambiar el tema no requiere modificar el símbolo. El relleno se escribe con `fill=`: `"none"` (por defecto), `"linecolor"` (el color de `linecolor`) o `"fillcolor"` (el color de `fillcolor`).
- **No proporcionados**: cadenas de path SVG, POLYGON (úsese un LINE cerrado), un gráfico de flecha separado (para puntas de flecha en extremos de línea, véase arriba; en otro caso, úsese un LINE cerrado relleno o un uso de flecha dibujado en un punto, 9.2.7), rectángulos redondeados, arcos elípticos (aproxímense con BEZIER) e imágenes.
- **Texto**: TEXT también tiene `size=` (tamaño de fuente, 11.7), `halign=` (horizontal: `"start"` (por defecto), `"middle"`, `"end"`), `valign=` (vertical: `"baseline"` (por defecto), `"top"`, `"middle"`, `"bottom"`) y `rotate=` (0, 90, 180 o 270; por defecto 0). En un cuerpo de símbolo, un TEXT adjuntado a `?` hace referencia a los atributos del objeto propietario, y uno adjuntado a un punto es texto fijo; en la escena, un TEXT es una anotación o una etiqueta según aquello a lo que esté adjuntado (13.3, 13.4). El marcado de texto se describe en 13.6.
- **Línea base**: con `valign="baseline"`, el anclaje (anchor) está en la línea base de la primera línea, y las líneas siguientes descienden, como en SVG, PostScript y la mayoría de editores; `"top"`, `"middle"` y `"bottom"` alinean todo el cuadro de texto. La línea base es el valor por defecto.
- **Dirección del texto**: el anclaje es la propia posición del texto (13.3). El cuadro de texto se sitúa junto al anclaje según `halign` y `valign`, y gira alrededor del anclaje un ángulo `rotate`, en grados, en sentido antihorario, cualquier número como `30`; un texto en un símbolo se transforma luego con la orientación de la instancia, tanto rotaciones como reflexión. Con `upright=true`, el valor por defecto, los caracteres del cuadro se leen derechos: el texto horizontal de izquierda a derecha, el texto vertical de abajo arriba, y los glifos nunca se reflejan; un texto cuyo ángulo final, tras la orientación, sea mayor que 90 grados y como máximo 270 se gira 180 grados alrededor de su anclaje. Así, `rotate=180` se muestra como 0 y 270 como 90, con el cuadro de texto al otro lado del anclaje; cuando la instancia se refleja, el anclaje se refleja como de costumbre y el cuadro de texto pasa al otro lado del anclaje en la dirección de la reflexión, pero los caracteres no se reflejan. Esto rige para toda alineación: `start` y `end` intercambian lugares, y lo mismo `top` y `bottom`. Un texto con `valign="baseline"` se gira del mismo modo: entonces cuelga por debajo de su anclaje tanto como se alzaba por encima. Un texto con `fix_rotate=true` conserva la dirección de sus caracteres, y únicamente el lado de su cuadro sigue la orientación (13.3). Con `upright=false` un texto nunca se gira para su lectura: sus caracteres siguen exactamente la rotación y la reflexión, incluidos glifos reflejados, lo que conviene a flechas y otros signos escritos como texto, como `→`. Ambos se combinan: `fix_rotate` decide el ángulo final (`rotate` en la escena, o `rotate` transformado con el objeto, cambiando una reflexión su signo), y `upright` decide si ese ángulo se gira luego 180 grados para su lectura; un texto con `fix_rotate=true` nunca se refleja. Por ejemplo, el nombre de un NMOS tiene su anclaje en (1.5, 1.5) y se extiende hacia la derecha con `halign="start"`; en `"MY"` el anclaje pasa a (−1.5, 1.5) y el texto se extiende hacia la izquierda, alejándose del cuerpo; en `"R90"` el anclaje pasa a (−1.5, 1.5) y el texto discurre verticalmente, leyéndose de abajo arriba. Los textos en la escena no giran con las instancias.
- **Etiquetas en una columna**: los textos que un símbolo adjunta a `?` con la misma x, los mismos `halign`, `valign`, `rotate`, `fix_rotate` y `upright`, y con contenido no vacío, forman una columna, leída de arriba abajo, como el nombre sobre el valor de una resistencia. Cuando la orientación del objeto invertiría una columna, al ponerla boca abajo (`"MX"`, `"R180"`) o acostarla de lado de modo que se leyera de derecha a izquierda (`"R270"`, `"MXR90"`), sus textos intercambian lugares, ocupando el primero el lugar del último y así sucesivamente, para que la columna se lea en su orden donde está. Los textos con distintas alineaciones, como un nombre encima de una caja y una fórmula en su centro, no forman una columna, y un texto cuya posición da la escena (13.3) no participa.

#### 9.2.6 Símbolos de bloque

Escrito dentro del subcircuito `.subckt ota5t … .ends`:

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

- El símbolo que un subcircuito asigna a su propio `#` es la apariencia por defecto de las instancias de ese subcircuito, dondequiera que estén: su símbolo de bloque. Los pines corresponden a los puertos del subcircuito por nombre.
- Es la excepción a la regla de Principios 2.5 de que las definiciones surten efecto solo hacia abajo: se aplica a los niveles que instancian el subcircuito.
- Los puertos de bus de un subcircuito ya están divididos en puertos individuales en el netlist (6.11); un símbolo de bloque los dibuja como uno solo con un pin de bus (9.2.4).
- En la cola de símbolos va después de las reglas de los paquetes en la raíz y antes de las reglas de los niveles, incluidos los paquetes usados en un nivel, por lo que un nivel superior puede reemplazarlo con una regla o con un SET-SYMBOL para una instancia individual (8.3).
- Ninguna instancia instancia un nivel de archivo, por lo que `# @SET-SYMBOL` escrito en un nivel de archivo no tiene efecto.
- Un módulo de Verilog-A proporciona su símbolo de bloque del mismo modo, con `# @SET-SYMBOL` escrito en el módulo en su archivo Verilog-A; sus pines corresponden a los puertos del módulo por nombre, y un puerto vectorial a un terminal por cada bit (2.9).
- **El símbolo de bloque por defecto.** Una instancia a la que ninguna regla, sentencia ni símbolo de bloque asigna un símbolo se dibuja con el símbolo de bloque por defecto, formado a partir de sus terminales:
  - **Pines.** Tiene un pin por cada terminal. Para una instancia de un subcircuito o de un módulo que el archivo lee (2.9), son los puertos en el orden de la definición, cada pin con el nombre de su puerto, como `terminal="inp"`. Para cualquier otra instancia, como la de una celda cuya definición el archivo no lee o un dispositivo cuya clase ninguna regla indica, son los nodos de su línea, cada pin con su número como nombre, como `terminal=1`. La primera mitad de los pines está en el lado izquierdo y el resto en el derecho; cuando su número es impar, el lado izquierdo tiene uno más. Cada pin se etiqueta dentro del marco con el nombre de su puerto, o si no con el nombre de su terminal (6.12), o si no con su número.
  - **Marco.** El marco es un cuadrado cuyo lado es el doble del número de pines del lado izquierdo, y al menos 2; su centro es el origen del símbolo. Los pines están separados 2 y miran hacia fuera del marco: los de la izquierda bajan desde 1 por debajo de la esquina superior, y los de la derecha están centrados en su lado. Así, una instancia con un terminal tiene un marco de lado 2 con su pin en el medio del lado izquierdo, y una con tres un marco de lado 4 con pines en (−2, 1), (−2, −1) y (2, 0); todo pin está en unidades enteras.
  - **Textos.** Encima del marco está el nombre de la instancia (`key="name"`), y debajo `${model}` (`key="model"`), el nombre del subcircuito o del modelo (13.6).
  - **Lugar en la cola.** Viene después de toda la cola de símbolos (8.3), fuera del grupo de los símbolos de bloque, de modo que cualquier regla o sentencia que asigne un símbolo a la instancia lo sustituye, y también el símbolo de bloque de su subcircuito. Cuenta como un símbolo para `drawn` y no coincide con ningún `symbol=` (10.1).
  - Así, toda instancia tiene un símbolo, con un pin por cada uno de sus terminales salvo que un símbolo que se le asigne omita alguno (9.2.4).
- Cuando un subcircuito tiene varias sentencias `# @SET-SYMBOL`, están en el mismo grupo de la cola de símbolos, en orden del texto, y surte efecto la posterior (8.3); una implementación puede emitir una advertencia.

#### 9.2.7 Variantes y fragmentos

```rnl
$NMOS_BULK = @DEFINE-SYMBOL(box=[-3, -3, 2, 3]) {
  $NMOS;
  %b = (1, 0) @PIN(terminal=4, label="B", direction="right");
  @LINE {(0, 0) %b}
}
```

- Incluir otro símbolo en un cuerpo y luego añadirle elementos crea una variante: los pines y los gráficos se acumulan, y los nombres de pin del símbolo incluido cuentan como nombres de la variante (9.2.4). Para desplazar un pin, divídase la parte común en fragmentos y escríbanse los pines en cada símbolo, como hace el paquete estándar.
- Un fragmento usado solo en su propio nivel se nombra con `%`, como `%MOS_CHANNEL = @DEFINE-SYMBOL {…}`, y otro cuerpo de símbolo lo incluye con `%MOS_CHANNEL;`.
- Los gráficos no tienen clave: la inclusión solo añade, y un gráfico incluido no puede eliminarse. Para dibujar menos, defínase otro símbolo: divídase la parte común en fragmentos e inclúyanse en cada símbolo las partes que necesita, como `$NMOS3 = @DEFINE-SYMBOL {%MOS_BODY}` y `$NMOS_4TERM = @DEFINE-SYMBOL {%MOS_BODY; %BULK}`.
- **Usos en el cuerpo**: una figura que se necesita en distintas posiciones o en distintas orientaciones, como una punta de flecha, se escribe como un uso dibujado en un punto:

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

  - El primer rol se rellena con un punto del cuerpo, en coordenadas locales. El uso se dibuja con su símbolo en ese punto, su orientación gira alrededor de ese punto, y luego se transforma junto con el símbolo que lo encierra; las transformaciones de los usos anidados se componen sucesivamente hacia fuera, de modo que la punta de flecha gira con el dispositivo.
  - Solo para dibujar: las reglas externas no pueden ver dentro de un cuerpo de símbolo, por lo que su símbolo se asigna con SET-SYMBOL en el cuerpo; un uso sin él no se dibuja, y se emite una advertencia. Los cuerpos de las definiciones generales no se expanden dentro de un símbolo, y las restricciones y cables no tienen significado allí; ORIENT aquí es un dato interno del símbolo, no participa en conflictos y no tiene prioridad.
  - Un SET-SYMBOL aquí que traiga de vuelta, directamente o mediante inclusiones y otros usos así, el símbolo que se está construyendo cierra un ciclo, como hace una inclusión (9.1): ese SET-SYMBOL falla, el uso no se dibuja y el resto del cuerpo surte efecto.
  - Dibujar un uso en el punto que rellena su primer rol rige únicamente en cuerpos de símbolo, que el solucionador no dispone. En la escena la misma notación crea un uso con un punto propio (12.1 punto 5), y las reglas pueden asignarle un símbolo; las reglas de un paquete de estilo de la casa no pueden alcanzar dentro de los símbolos.
- Nombres de pin y fórmulas: si se muestran los nombres de pin queda a criterio del visor (9.2.4); una fórmula es simplemente un TEXT que hace referencia a atributos (13.6), sin reglas especiales. El marcado de texto se describe en 13.6.

#### 9.2.8 Emparejamiento de símbolos con objetos

Los símbolos no se dividen en símbolos para dispositivos y símbolos para usos. Que un símbolo pueda utilizarse para un objeto depende de si sus pines coinciden con los terminales del objeto.

- **Instancias sin símbolo.** Ninguna instancia se queda sin él: cuando ninguna sentencia ni regla asigna un símbolo a una instancia, tiene el símbolo de bloque por defecto (9.2.6), con un pin por cada terminal, y se dibuja y se resuelve como cualquier otra. Un paquete puede dejar un dispositivo sin símbolo cuando no pueda determinar su clase, en lugar de dibujarlo como un dispositivo que quizá no sea, como hace el paquete estándar para un transistor cuya tarjeta de modelo no se leyó; el dispositivo se dibuja entonces como un recuadro con un pin por cada nodo.

### 9.3 Estilos: DEFINE-STYLE

```rnl
$ink = @DEFINE-STYLE(linecolor="#202020", textcolor="#202020", fillcolor="#ffffff", size=1.2) {};
$accent = @DEFINE-STYLE(linecolor="#1f4fd8", linestyle="bold") {$ink};
$shade = @DEFINE-STYLE(fillcolor="#1f4fd833") {}
```

- Los parámetros entre paréntesis son las propiedades propias del estilo, y surten efecto en último lugar. El conjunto puede hacer referencia únicamente a otros estilos, cuyas propiedades se incluyen en ese lugar, sustituyendo los posteriores a los anteriores. Un estilo sin estilo base se escribe con `{}`.
- Un estilo surte efecto únicamente cuando SET-STYLE lo asigna (11.3). Las mismas propiedades pueden escribirse también directamente como parámetros de gráficos, TEXT y WIRE (9.2.5, 13.1), con valores tomados según los dos rangos de 8.3.
- Un estilo solo traza y colorea: la posición, la orientación, la forma (los puntos de un símbolo o gráfico) y el contenido del texto no forman parte de un estilo.
- **Propiedades**:

| Grupo | Propiedad | Valores |
|---|---|---|
| Línea | `linecolor` | Un color; también el color de `fill="linecolor"` |
| | `linestyle` | `"solid"` (por defecto), `"dashed"`, `"dotted"`, `"dash-dot"`, `"double"` (dos líneas continuas paralelas), `"bold"`, `"none"` (sin línea) |
| | `linewidth` | El grosor de línea, en unidades de coordenadas (11.7); `0` es una línea fina, igualmente delgada a cualquier escala, y es el valor por defecto |
| | `cap` | Extremos de línea: `"butt"`, `"round"`, `"square"` |
| | `join` | Esquinas: `"miter"`, `"round"`, `"bevel"` |
| Relleno | `fillcolor` | El color de `fill="fillcolor"` |
| Texto | `textcolor` | El color del texto; cuando no se da, `linecolor` |
| | `size` | El tamaño de fuente, en unidades de coordenadas (11.7) |
| | `font` | Un nombre de fuente, solo una indicación; un visor sin esta fuente usa la suya |
| | `bold`, `italic` | Negrita o cursiva para todo el texto, `true` o `false`; el marcado `\textbf` y `\textit` puede seguir aplicándolas a partes (13.6) |
| | `textvisible` | Si se muestra el texto: `true`, el valor por defecto, o `false`. Un texto con `false` no se dibuja, no ocupa espacio y no puede seleccionarse: se excluye de las colisiones, del marco de un símbolo y de las columnas de 9.2.5. Su contenido, clave, posición y referencias a atributos permanecen y se evalúan como de costumbre; un visor puede ofrecer mostrar los textos ocultos. Un paquete que oculta etiquetas define un estilo y se lo asigna: `$hidden = @DEFINE-STYLE(textvisible=false) {};` y `# @SELECT-LABEL(recursive=true) => @SET-STYLE $hidden`; un texto individual también puede tomarla directamente, como `#M1 @TEXT("note", key="memo", textvisible=false)` (visibilidad del texto (textvisible)) |
| Otros | `arrowsize` | La longitud de las puntas de flecha en extremos de línea, en unidades de coordenadas (9.2.5) |

- Una línea tiene únicamente cinco propiedades: color, estilo, grosor, remate y unión. `"bold"` dibuja el doble del grosor de línea, y el doble de una línea fina cuando el grosor es 0; las longitudes de trazo y huecos de las líneas discontinuas y el espaciado de las líneas dobles quedan a criterio de la implementación, en proporción al grosor de línea. En versiones posteriores del lenguaje pueden añadirse más valores de `linestyle` (16.1).
- `"none"` no dibuja línea, ni tampoco puntas de flecha en los extremos de línea (pertenecen a la línea, 9.2.5); `fill="linecolor"` sigue tomando el color de `linecolor`, de modo que una figura rellena pero sin trazo se escribe `fill="linecolor", linestyle="none"`, y su color sigue obedeciendo a un cambio de tema. Tanto los cables como los gráficos la admiten; un cable con ella es un cable invisible, pero sigue siendo un cable (13.1).
- **Los colores** se escriben como colores hexadecimales CSS: `"#RRGGBB"` o `"#RRGGBBAA"`, o las formas cortas `"#RGB"` y `"#RGBA"`. El alfa va al final: `00` es totalmente transparente y `FF` opaco; sin él el color es opaco. Las líneas y los rellenos tienen cada uno su propio color, por lo que su transparencia se fija por separado, como `$shade` arriba hace solo parcialmente transparente el relleno. Esta versión no acepta nombres de color ni `rgb()`. Los colores son cadenas.
- Esta versión no proporciona rellenos con patrón, degradados ni sombras; tampoco la visibilidad de objetos distintos de los textos (mostrarlos y ocultarlos es asunto del visor, 16.2; `textvisible` trata únicamente de textos, para que una visibilidad así pueda añadirse más adelante con nombres propios, y entonces un texto se muestra cuando ambos son verdaderos); ni objetos como temas y perfiles (un tema es un paquete de estilos y reglas, incorporado con USE-PACKAGE, 16.2).
- Cuando ningún estilo y ningún parámetro da un valor para una propiedad, se utiliza el valor por defecto del propio renderizador; esto no es un error (Principios 2.7). El paquete estándar ofrece una apariencia por defecto portable (16.3), que las reglas y sentencias escritas por el usuario sustituyen.
- Las propiedades desconocidas se tratan como errores de sintaxis (Principios 2.3).

### 9.4 Definiciones generales: DEFINE

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

- Una definición general consta de una lista de roles, una lista de parámetros y un cuerpo. En el cuerpo, a los roles y parámetros se hace referencia con `?name` (6.10). Qué hace una definición lo decide su contenido; véase 9.5.
- **Aplicar**: AS y ATTACH rellenan los roles de una definición con objetos y producen un uso. Ambas palabras clave hacen lo mismo y difieren únicamente en dónde se escribe la definición:
  - AS lleva la definición al final y se lee "estos objetos, como tal cosa"; se usa para significado y disposición, como `#M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N`;
  - ATTACH lleva la definición primero, antes de los objetos escritos explícitamente, y la forma estándar sitúa la palabra clave después de la definición, como `$GROUND @ATTACH &vss`; se lee "tal cosa, añadida a estos objetos", y se usa para componentes virtuales y el relleno (capítulo 12, 10.5).
  - Elegir la otra palabra clave no es un error, solo resulta incómodo de leer; escribir la definición en el lugar equivocado sí es un error (4.2 punto 7). Los objetos pasados por una tubería se emparejan con los roles directamente, y ambas palabras clave tienen entonces el mismo aspecto; elíjase según para qué sea la definición.
  - Cómo elegir: úsese AS cuando los objetos juntos forman la entidad, como una estructura de circuito, una disposición o una forma local de dibujar formada por esos objetos; úsese ATTACH cuando la entidad se añade sobre los objetos, como una tierra, una etiqueta de puerto, una marca de no conexión, un punto de unión, una etiqueta para cada instancia o el relleno. Las sentencias estándar son `#M1(left), #M2(right) and #M5(tail) are treated @AS $DIFFERENTIAL_PAIR_N` y `$GROUND is @ATTACH to &vss` (4.5).
- **Lista de roles**: el parámetro `roles`, una matriz cuyos elementos son nombres de rol o pares `[name, kind]`. Ambos parámetros de DEFINE solo pueden darse por nombre.
  - La lista puede estar vacía, `roles=[]`: la definición se aplica sola, como `$PORT_LIST @ATTACH`, y el uso no tiene objeto principal; sigue teniendo su propio punto (14.1).
  - Un elemento escrito `"{name}"` es un rol de conjunto, que toma un conjunto entero; los demás son roles únicos (5.2.1 punto 4).
  - Un rol de conjunto que recibe un único objeto lo toma como un conjunto con ese único objeto; un conjunto vacío también es válido, y las restricciones del cuerpo que reciban demasiado pocos miembros se cumplen trivialmente (capítulo 14).
  - Un rol único que recibe un conjunto aplica la definición una vez por cada miembro, y cero veces para un conjunto vacío; en una misma aplicación, como máximo un rol único puede recibir un conjunto con más de un miembro (de lo contrario la sentencia falla, 5.2.1).
  - Una regla que aplica una definición cuyos roles son todos roles de conjunto no crea ningún uso en un nivel donde todos están vacíos: la regla se evalúa en cada nivel al que llega (10.4), y un uso se crea solo donde sus roles de conjunto contienen algo, igual que un rol único que recibe un conjunto vacío aplica la definición cero veces. Una sentencia que no es una regla crea su uso aunque sus roles de conjunto estén vacíos. Así, el relleno de la serie estándar 0.4 crea la columna de etiquetas de puerto de un nivel solo en los niveles que reciben etiquetas de puerto (12.4).
  - Cuando el `?name` de un rol de conjunto se escribe dentro de `{…}`, es un elemento de ese conjunto (5.2): en una restricción representa a cada uno de sus miembros en ese elemento (14.1), por lo que el ORDER en el ejemplo anterior sitúa a R2 y R3 cada uno entre R1 y R4.
  - Un rol de conjunto que recibe un conjunto que tiene conjuntos entre sus miembros lo toma tal cual; el cuerpo decide cómo utilizarlo.
  - Un rol de conjunto no corresponde a un terminal (6.4).
  - En una misma aplicación, cada rol puede marcarse una sola vez; para dar varios objetos a un rol de conjunto, escríbanse como un conjunto, como `{#R2 #R3}(chain)` arriba.
  - **Clases**: un elemento escrito `[name, kind]` declara qué clase de objeto admite el rol; un elemento con solo un nombre admite cualquier clase. Los nombres de clase son los de la lista de palabras (5.3): `"instance"`, `"terminal"`, `"pin"`, `"net"`, `"point"`, `"wire"`, `"usage"`, `"bundle"`, `"text"`, `"annotation"` y `"label"`. En una declaración de rol, `"instance"` admite también agrupaciones, y `"terminal"` los terminales de agrupaciones (11.8); una matriz significa cualquiera de varias clases, como `["p", ["net", "terminal"]]`. Un objeto de una de las clases se toma tal cual; un objeto de ninguna de ellas que se convierta a algunas de ellas (5.2.4) se convierte a la primera de ellas en el orden de la matriz, de modo que `#R1::p` dado para `["net", "terminal"]` pasa a ser la red del pin. Un rol que admite `"net"` admite también el nodo vacío, `&` solo, lo que indica que el rol no está en ninguna red (6.3); un rol de conjunto no, puesto que el nodo vacío no es miembro de ningún conjunto. La clase de un rol de conjunto se aplica a cada uno de sus miembros, como `["{members}", "instance"]`. No existe ninguna clase para los usos de una definición concreta.
  - Cuando un rol recibe un objeto de la clase incorrecta, falla toda la aplicación, y el diagnóstico indica qué rol, qué admite y qué recibió; lo mismo rige cuando un miembro de un rol de conjunto tiene la clase incorrecta. Cuando una selección crea las aplicaciones una por una, falla únicamente la aplicación que recibió el objeto incorrecto.
  - Nota: en un cuerpo vacío de definición, nada comprueba qué rellena los roles, por lo que únicamente una clase declarada detecta un rol rellenado erróneamente; en un cuerpo no vacío, cada sentencia de la expansión se comprueba también por sí sola (9.5). Las clases declaradas permiten además comprobar en la definición los usos de los roles en el cuerpo (6.9).
  - **Roles opcionales y cantidades**: un rol único debe rellenarse; un rol de conjunto puede quedar sin rellenar, lo que lo convierte en un conjunto vacío. Cuando los objetos se dan en orden y hay menos objetos que roles, todos los roles restantes deben ser roles de conjunto; cuando los objetos se marcan con roles, un rol de conjunto que no se escribe es un conjunto vacío. No hay cantidad mínima ni máxima: las restricciones con demasiado pocos miembros se cumplen trivialmente (capítulo 14). Una variante de distinta forma se escribe como otra definición (véase derivación y extensión abajo). Reglas de validez como que un mismo objeto rellene dos roles, o las conexiones esperadas, se tratan en 9.5.
- **Lista de parámetros**: el parámetro `parameters`, que puede omitirse. Cada elemento es un nombre de parámetro o `[name, default]`.
  - Los parámetros se pasan entre los paréntesis de AS y ATTACH, como `$GND_BELOW @ATTACH(gap=3)`, por posición en el orden de la lista de parámetros o por nombre; cuando se usan ambos, los de posición van primero (4.2 punto 9). La forma estándar los escribe por nombre (4.3). Un nombre que no esté en la lista de parámetros hace fallar la aplicación. Cuando una selección crea las aplicaciones una por una, cada aplicación usa los mismos parámetros.
  - Dondequiera que el cuerpo pueda contener un literal, puede escribirse un marcador de parámetro en su lugar: el valor de un parámetro, un elemento de una matriz, una coordenada de un punto. Así, dentro de un cuerpo de definición, la regla de que los parámetros solo admiten literales (4.1) pasa a ser "literales o marcadores de parámetro"; tras la expansión vuelven a ser literales. Los parámetros de AS y ATTACH mismos solo pueden ser literales y no pueden pasar objetos; los objetos entran a través de roles.
  - Un parámetro que no se pasa y no tiene valor por defecto cuenta como no escrito donde se usa: como parámetro con nombre no está escrito, como `size=?size` en `@TEXT(?label, size=?size)`, tras lo cual el texto tiene el tamaño por defecto propio de TEXT o el tamaño de su estilo (11.3); como coordenada de un punto es `_`; en un parámetro dado por posición o en un elemento de una matriz no tiene significado definido, y esa sentencia falla. Por ello no hace falta ningún marcador para parámetros obligatorios.
  - Los parámetros están pensados para magnitudes como números y texto; una variante que difiera en estructura debería escribirse como una definición aparte; el lenguaje no impone esto. Los parámetros únicamente se pasan hacia el interior del cuerpo; no pueden seleccionarse usos por los valores de sus parámetros.
- Los nombres de rol y nombres de parámetro están en una misma tabla y deben ser todos distintos; ninguno de ellos puede ser un nombre estándar de subobjeto (6.5). Un parámetro no puede llamarse `priority`, que es un parámetro estándar de AS y ATTACH (14.8).
- **El uso mismo**: en el cuerpo de una definición general, un `?` aislado es el uso que se está expandiendo, por su propio punto (14.1). `{?members::position ?} @ALIGN("y")` sitúa el punto del uso a la altura de sus miembros, para que una restricción sobre ese punto, como un DISTRIBUTE respecto a un eje en otra estructura, los mueva con él; `::position` mantiene como un solo punto a un miembro que sea un uso sin símbolo (14.1). Un `?` aislado es un objeto, no un rol. Admite un camino igual que un uso (6.5): `?::position` es su propio punto, en el que un cable o gráfico del cuerpo puede terminar (5.2.4), y `?::` seguido del nombre de un pin de su símbolo es ese pin, como `?::pin` para un pin llamado pin.
- **Reglas en un cuerpo**: un cuerpo puede contener reglas, una selección y las sentencias canalizadas desde ella (10.1). Actúan como reglas del nivel en que está el uso, escritas en la posición de su sentencia AS o ATTACH (8.5, 10.4), y surten efecto en los pasos de 8.2 como lo hacen las reglas escritas allí: SET-SYMBOL en la cola de símbolos, las reglas de las palabras clave de trazado en el paso 5, la generación en el paso 6, SET-STYLE en la cola de estilos. En un cuerpo, `#` es ese nivel (10.3). En el cuerpo de un uso que se genera a su vez en el paso 6, una regla del paso 5 o 6 (10.4) falla y las demás reglas surten efecto en los pasos posteriores, según indica 10.5.
- Los nombres `%` vinculados en el cuerpo son nombres locales: cada aplicación crea objetos nuevos para ellos, a los que se hace referencia desde fuera a través del uso (abajo). Un objeto compartido por varias aplicaciones, como un eje de simetría, se pasa a través de un rol.
- Una aplicación puede marcar los objetos con roles (`#M1(a) #M2(b) … @AS $T`, `$T @ATTACH #M1(a) #M2(b) …`) o darlos en el orden de la lista de roles (`#M1 #M2 … @AS $T`). Cuando un rol único queda sin rellenar, o se dan más objetos que roles hay, falla toda la sentencia; un rol de conjunto dejado sin rellenar es un conjunto vacío.
- Una aplicación produce un uso, que registra la definición y qué rellena cada rol, y que puede nombrarse (como `%c1`). Eliminar la sentencia elimina también las descripciones expandidas a partir de ella. Un uso no tiene clave: cada aplicación es un objeto aparte (5.8).
- **Expansión**:
  - los marcadores se reemplazan por los argumentos y valores de parámetro;
  - los nombres locales se reemplazan por manejadores nuevos, "uso + nombre local", escritos `%c1::mid`; un uso sin nombre se muestra en los diagnósticos por la posición de su sentencia;
  - los nombres de fuera del cuerpo no se resuelven de nuevo donde se aplica la definición (6.9);
  - el resultado de la expansión se consigna en la posición de la sentencia AS o ATTACH, y toma esa posición en la resolución de conflictos (8.5).
- **Coordenadas en un cuerpo**: una coordenada literal en el cuerpo de una definición general, en un punto como `(1, 0)` o en PLACE, es un desplazamiento respecto al punto propio del uso (14.1), en los ejes del nivel: `(1, 0)` está una unidad a la derecha de ese punto, y `(0, 0)` es el punto mismo, `?::position`. Un `_` queda libre, y un marcador de parámetro en una coordenada es un desplazamiento una vez reemplazado. Únicamente el punto mueve el cuerpo: ORIENT sobre el uso gira su símbolo, no lo que dibuja o coloca su cuerpo (11.2). Así, una definición puede dibujar una marca alrededor de su propio punto, como dos LINE que se cruzan en `(0, 0)`, y vincular su punto a un pin (12.1 punto 5). PLACE sobre el `?` aislado en su propio cuerpo falla, pues su desplazamiento sería respecto a sí mismo. Fuera del cuerpo, como en el registro del solucionador, las posiciones son coordenadas del nivel (14.8).
- A los usos anidados se accede paso a paso (`%c1::inner::mid`). Un uso también puede rellenar un rol de otra definición: con una definición propia `$OTA = @DEFINE(roles=[["pair", "usage"], ["load", "usage"]]) {}`, `#M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N => %cm1 @AS $OTA` es un amplificador cuyo rol pair está rellenado con el uso del par diferencial, y su rol load con el espejo de corriente `%cm1` (5.5).
- **Aplicación cíclica**: una aplicación dentro de una expansión que aplica una definición que ya se está expandiendo en un uso que la encierra, directamente o a través de otras definiciones, cierra un ciclo. Esa aplicación falla, y su diagnóstico enumera toda la cadena; el resto surte efecto como de costumbre. Abajo, aplicar `$A` a M1 expande `$B`, cuya aplicación de `$A` cierra el ciclo `$A → $B → $A` y falla; el ORIENT en `$B` sigue reflejando a M1:

  ```rnl error
  $A = @DEFINE(roles=["x"]) {?x @AS $B};
  $B = @DEFINE(roles=["x"]) {
    ?x @AS $A;               /* closes the cycle $A → $B → $A: this application fails */
    ?x @ORIENT("MY")
  };
  #M1 @AS $A
  ```

- **Referencia a objetos locales desde fuera**:

  ```rnl use=gdef
  $DP_BLOCK = @DEFINE(roles=["in_p", "in_n"]) {
    %pair = {?in_p ?in_n} @GROUP
  };
  %dp1 = #M1(in_p) #M2(in_n) @AS $DP_BLOCK;
  %c1::mid @PLACE(x=20);              /* moves the midpoint of c1 */
  {%c1::mid #M9:1} @ALIGN("x");       /* aligns the drain of M9 with the midpoint of c1 */
  %dp1::pair @PRIORITY("disabled")    /* turns off only the group expanded for dp1 */
  ```

  - A todo objeto local nombrado de un cuerpo puede hacerse referencia desde fuera a través del uso, escrito `usage::local_name`; los no nombrados no pueden alcanzarse, salvo por número desde el registro del solucionador (5.9). Nombrar uno o no es elección del autor de la definición, conforme a "para hacer referencia a un objeto, nómbrelo" (5.7); no hay ninguna lista de exportación separada.
  - Los caminos pueden encadenarse: `%st1::dp::mid` es el mid del uso local dp en el cuerpo de st1. Pasar por un rol funciona igual: `%st1::pair::mid` es el mid del uso que rellena el rol pair.
  - Estas son referencias ordinarias: pueden añadírseles hechos (PLACE, SET-STYLE, TEXT, PRIORITY, etc.), pueden usarse en otras sentencias y pueden rellenar roles de otras definiciones; los conflictos con los hechos de la expansión se resuelven según 8.5. Un nombre local no puede vincularse de nuevo desde fuera: `%c1::mid = …` es un error.
  - Nombres: los nombres alcanzados mediante `::` en un uso deben ser únicos. Por tanto, un nombre local no puede coincidir con un nombre de rol ni con un nombre estándar de subobjeto, lo que se comprueba en la definición, y la clave de un texto sobre el uso no puede coincidir con ninguno de esos nombres (6.5, 13.3). Un nombre local sí puede coincidir con un nombre de parámetro: los parámetros son valores, y `::` no alcanza valores.
  - Prioridad: una sentencia PRIORITY escrita desde fuera directamente sobre un objeto local prevalece sobre la prioridad del uso (14.8).
  - Fuera de alcance: los objetos locales de usos sin nombre, salvo por número desde el registro del solucionador (5.9), y los de usos generados uno por uno por una selección. Una selección cuyo ámbito sea un uso sigue seleccionando únicamente los miembros de sus roles, no sus objetos locales (10.1): a través de un uso, a los objetos locales se accede solo por nombre. Una selección cuyo ámbito sea el nivel los selecciona como objetos de ese nivel, igual que selecciona cualquier otro objeto, como `# @SELECT-USAGE $FILTER` abajo; lo que una regla no puede hacer es nombrar un objeto local de una definición que ella no escribió.
  - Los nombres locales forman parte, por tanto, de la interfaz de una definición: cuando una definición cambia un nombre local, las referencias desde fuera dejan de encontrarlo y fallan (capítulo 7), el resultado del solucionador queda obsoleto con ellas y una indicación sugiere volver a resolver (capítulo 17). El paquete estándar indica en qué nombres locales de sus definiciones puede confiarse.
- **Notación en el cuerpo**: los roles y parámetros se escriben `?name`, los nombres locales `%name` (6.9, 6.10).
- **Derivación y extensión**: únicamente por composición; no existe relación de herencia (9.1). El cuerpo de una definición derivada aplica la definición base y añade sus propias sentencias:

  ```rnl
  $FILTER = @DEFINE(roles=["in", "out"]) {};
  $LOWPASS = @DEFINE(roles=["in", "out", "r", "c"]) {
    %f = ?in(in) ?out(out) @AS $FILTER;
    {?r ?c} @ORDER("left-to-right")
  }
  ```

  - Se añade contenido con sentencias en el cuerpo, y variantes numéricas con parámetros; para ocultar y extender una definición en un nivel inferior, el cuerpo hace referencia a la definición externa del mismo nombre (6.9).
  - Seleccionar por la definición base no necesita ninguna regla especial: el uso FILTER expandido a partir de la definición derivada está en la escena, por lo que `# @SELECT-USAGE $FILTER` lo selecciona, y las reglas escritas para `$FILTER` funcionan también sobre los filtros paso bajo, actuando únicamente sobre el uso FILTER dentro de ellos y no sobre las sentencias que añade la definición derivada.
  - No existe ningún mecanismo aparte para descartar parte de la definición base: el cuerpo de una definición derivada puede escribir `%f::x @PRIORITY("disabled")` para desactivar parte de ella, pero una prioridad sobre el uso externo prevalece sobre eso (14.8); para excluir una parte de manera fiable, divídase la definición base en definiciones más pequeñas o escríbase otra.

### 9.5 Usos de las definiciones generales

Qué hace una definición lo decide su contenido, no una palabra clave:

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

En el paquete estándar, `$CURRENT_MIRROR_N`, `$MIRRORED_LAYOUT` y `$GROUND` son de estas tres clases; `$DIFFERENTIAL_PAIR_N` tiene sus roles y, en su cuerpo, una disposición por defecto (abajo).

- Todos son la misma clase de entidad: un uso (que registra qué definición se aplicó y qué rellena cada rol), más las descripciones expandidas a partir del cuerpo. Las dos partes tienen distintos usuarios:
  - las restricciones expandidas a partir del cuerpo las maneja el solucionador, deben satisfacerse y se resuelven según 8.5 en caso de conflicto;
  - el uso es para que lo lean personas y LLM, y para las selecciones y reglas del propio RNL (10.1).

  Así, los usos de definiciones de disposición también portan significado; una definición con cuerpo vacío, como `$PAIR`, registra significado únicamente a través de sus roles, y su significado lo establece el paquete estándar o su autor. Una definición que registra significado puede llevar también un dibujo en su cuerpo (abajo).
- Un uso semántico no es un comentario: sus referencias deben encontrarse, sus roles deben coincidir, se comprueba su validez básica (Principios 2.4) y las selecciones y reglas pueden utilizarlo; únicamente cómo se dibuja no está especificado.
- "Qué es" y "cómo se dibuja" pueden escribirse en una sola definición o en dos. Una definición puede llevar un dibujo por defecto en su cuerpo, con prioridad `"low"`: los hechos propios del autor van antes que él, y la prioridad del uso lo desactiva o lo eleva, como `%dp1 @PRIORITY("disabled")`, o `# @SELECT-USAGE $DIFFERENTIAL_PAIR_N => @PRIORITY("disabled")` para cada uso de la definición, mientras que el uso sigue registrando qué son los objetos (14.8). El paso de significado a dibujo puede escribirse también como una regla en un paquete, y cambiar el paquete cambia el dibujo: `# @SELECT-USAGE(recursive=true) $DIFFERENTIAL_PAIR_N => @AS $DP_LAYOUT`; eliminar el uso de disposición no afecta al uso semántico. El paquete estándar escribe sus dibujos por defecto en los cuerpos de sus definiciones, todas sus restricciones con prioridad `"low"`, como el transistor de cola de un par diferencial debajo del punto medio del par, y los miembros de `$BRANCH` en una columna de arriba abajo (16.3).
- **El dibujo se escribe siempre en RNL**: un significado se dibuja de cierta manera únicamente por lo que escribe RNL, en el cuerpo de su definición o en una definición de disposición aplicada por una regla, como arriba, posiblemente en un paquete; no se confíe en convenciones por defecto de un renderizador para un significado. Que una definición lleve un dibujo depende de su paquete; este estándar no establece ninguna regla adicional al respecto. Cómo se disponen las partes que RNL no describe queda a criterio de la implementación (Principios 1), pero eso no es una convención de dibujo para un significado y no se puede confiar en ello. Que un renderizador decida el estilo por defecto de objetos sin descripciones (Principios 2.7), y que una instancia sin símbolo tenga el símbolo de bloque por defecto (9.2.6), son un asunto distinto.
- **El vocabulario pertenece al paquete estándar**: las primeras definiciones semánticas y los componentes virtuales estándar son contenido del paquete estándar, que no forma parte del estándar del lenguaje (16.3). Cómo establecen su significado y sus roles las definiciones del paquete estándar corresponde también al paquete estándar. El significado de una definición de usuario es su cuerpo; su nombre es solo para que lo lean las personas.
- **Validez básica** (Principios 2.4): la sintaxis de las definiciones no comprueba si dos roles se rellenan con el mismo objeto. La validez básica de una aplicación es: se encuentran las referencias, los roles reciben las clases correctas, los roles únicos están rellenos (9.4) y cada sentencia de la expansión es válida por sí sola.

  ```rnl error
  $DP_LAYOUT = @DEFINE(roles=[["in_p", "instance"], ["in_n", "instance"]]) {
    {?in_p ?in_n} @ALIGN("y");
    ?in_p ?in_n @DISTANCE("left-to-right", min=4)
  };
  #M1(in_p) #M1(in_n) @AS $DP_LAYOUT      /* the expanded DISTANCE asks M1 to be at least 4 from itself */
  ```

  - En este ejemplo, el DISTANCE expandido se contradice a sí mismo: falla con un diagnóstico, y solo él se descarta (5.5, 7.3); el ALIGN se cumple como de costumbre. Una definición con cuerpo vacío, como `$PAIR` arriba, no puede detectar tal rellenado: `#M1(in_p) #M1(in_n) @AS $PAIR` no informa de ningún error.
  - Un componente virtual con ambos extremos en la misma red tiene un significado definido, por lo que "el mismo objeto rellena dos roles" no puede ser un error en general.
  - Comprobaciones adicionales, como exigir que dos roles sean distintos, se escriben como aserciones (15.3).
- **Palabras clave y definiciones generales** (1.5 punto 7): lo que una implementación debe comprender por sí misma, y lo que no puede escribirse con otras descripciones, es una palabra clave, como las restricciones atómicas, GROUP, WIRE, TEXT y CURRENT; lo que puede escribirse en el cuerpo de una definición general es una definición general, como pares diferenciales, espejos de corriente, tierras y alimentaciones, y flujo de señal (14.6). DEFINE-SYMBOL, DEFINE-STYLE, SET-SYMBOL y SET-STYLE también son palabras clave: un cuerpo de símbolo está en coordenadas locales y se evalúa donde se define; las propiedades de un estilo son literales; SET-SYMBOL y SET-STYLE fijan una propiedad de objetos existentes y se deciden en colas una vez que existen todos los objetos, mientras que AS y ATTACH producen un objeto nuevo cada vez.
- Una definición se identifica por la definición a la que se resuelve su nombre, no por la cadena del nombre: cuando un nivel inferior define su propio `$DIFFERENTIAL_PAIR_N`, esa es otra definición, y las reglas de un paquete escritas para el `$DIFFERENTIAL_PAIR_N` estándar no la reconocen.

<!-- translated from en/ch10.md sha256:bff47dee5256 -->

## 10 Selecciones, reglas y generación uno por uno

Este capítulo trata las palabras clave de selección, las reglas que las utilizan y la generación uno por uno.

### 10.1 Palabras clave de selección

Hay una palabra clave de selección para cada clase de objeto seleccionado, cada una con un conjunto fijo de condiciones. El resultado de una selección es un conjunto (5.2).

Cada palabra clave de selección toma un objeto ámbito. `#` es el nivel en el que se evalúa la selección (10.3). Dado un objeto o un conjunto, la selección busca entre esos objetos: dado un conjunto de redes, por ejemplo, SELECT-TERMINAL selecciona los terminales en esas redes (cómo conduce el ámbito a la clase seleccionada se describe después de la tabla). El ámbito no puede omitirse, por lo que una selección simple también se escribe con `#`: `# @SELECT-INSTANCE(name="R*") => @SET-SYMBOL $RESISTOR`.

| Palabra clave | Selecciona | Condiciones |
|---|---|---|
| SELECT-INSTANCE | Instancias del anfitrión. Con terminales en el ámbito, las instancias a las que pertenecen; con redes, las instancias que tienen algún terminal en ellas | `name` (el nombre de la instancia, como `name="R*"`); `model` (el nombre en la posición de modelo de la línea de instancia: un nombre de `.model` o un nombre de subcircuito); `type` (el tipo de la instancia, procedente de su `.model` o de un DEVICE, como `"nmos"`; admite `*`); `terminals` (el número de terminales de la instancia, como `terminals=3`); `symbol` y `drawn` (abajo) |
| SELECT-TERMINAL | Terminales de instancias del anfitrión, tengan pines o no: una selección del circuito | `name`, `model` y `type` de la instancia; `terminal` (un nombre o número de terminal); `net` (el nombre de la red en que está); `current` (la dirección de corriente, 15.2: `"in"`, `"out"` o `"unspecified"`) |
| SELECT-PIN | Pines de objetos dibujados: una selección del dibujo (9.2.4) | `name`, `model` y `type` del objeto al que pertenece el pin; `terminal` (un nombre de terminal o de rol que vincula el pin); `net` (el nombre de su `::net`); `wired` (10.2) |
| SELECT-NET | Redes. Con instancias, terminales o cables en el ámbito, las redes en que están | `name`; `port` (si es un puerto de este nivel); `wired` (10.2); `global` (si es una red global, 6.3) |
| SELECT-WIRE | Cables. Con redes en el ámbito, los cables dibujados en ellas | `net` (el nombre de la red del cable; un bus coincide cuando coincide una de sus redes); `bus` (`true`: solo buses, 13.1; `false`: solo cables que no son buses, cuya ranura de red recibe una red en lugar de un conjunto) |
| SELECT-LABEL | Etiquetas, es decir, TEXT adjuntados a objetos (13.3). Con objetos en el ámbito, las etiquetas adjuntadas a ellos | `key` (la clave del texto; admite `*` y distingue entre mayúsculas y minúsculas; una etiqueta sin clave no coincide con ningún `key=`) |
| SELECT-ANNOTATION | Anotaciones, es decir, gráficos en la escena y TEXT adjuntados a puntos (13.4) | Ninguna condición aparte de `role` y `recursive` |
| SELECT-USAGE | `scope @SELECT-USAGE $D`: los usos de la definición `$D` en el ámbito, mirando hacia dentro. Con `#`, los usos del nivel; con usos, esos propios usos y los usos dentro de ellos. Sin `$D`, los usos de cualquier definición | La definición es el último objeto (5.6) y puede omitirse. `symbol` y `drawn` (abajo) |
| SELECT-ANCESTOR-USAGE | `objects @SELECT-ANCESTOR-USAGE $D`: los usos de `$D` que tienen estos objetos como miembros, mirando hacia fuera; los miembros indirectos cuentan | La definición es el último objeto (5.6) y es obligatoria. Ninguna condición aparte de `recursive` |

Además de estas, toda palabra clave de selección admite la condición `recursive`, y las siete primeras admiten `role`, que tiene significado únicamente cuando el ámbito contiene usos (ambas abajo).

- **Una palabra clave por cada clase.** Cada palabra clave de selección recibe el nombre de la clase que selecciona, como `&clk @SELECT-WIRE => @SET-STYLE $signal` para los cables de una red, o `#M1 @SELECT-LABEL` para las etiquetas de M1. Ninguna selección toma objetos de todas las clases: para abarcar todas las clases, escríbase una regla por cada clase, como hace el paquete estándar para dar a cada objeto su estilo base.
- **Cómo conduce el ámbito a la clase seleccionada.** Cada objeto del ámbito da un paso hacia la clase que se está seleccionando; un objeto que no tiene tal paso no selecciona nada, lo cual no es un error:
  - SELECT-INSTANCE: una instancia es ella misma; un terminal conduce a su instancia; una red conduce a las instancias que tienen un terminal en ella.
  - SELECT-TERMINAL: una instancia conduce a sus terminales; una red conduce a los terminales en ella; un terminal es él mismo; un pin conduce a los terminales que vincula.
  - SELECT-PIN: un objeto dibujado (una instancia, una agrupación, un uso con símbolo) conduce a los pines de su propio símbolo, y con `role`, un uso conduce en su lugar a los pines que rellenan ese rol, como las demás selecciones toman miembros (abajo); un uso sin símbolo conduce a los pines entre los objetos que rellenan sus roles; un terminal conduce a los pines que lo vinculan; una red conduce a los pines cuyo `::net` es esa red, incluidos los pines vinculados a roles rellenados por ella; un pin es él mismo. Con `#`, todos los pines del nivel.
  - SELECT-NET: una red es ella misma; una instancia conduce a las redes de sus terminales; un terminal conduce a su red; un cable conduce a las redes sobre las que se dibuja.
  - SELECT-WIRE: una red conduce a los cables dibujados en ella; un cable es él mismo. SELECT-LABEL: un objeto conduce a las etiquetas adjuntadas a él; una etiqueta es ella misma. SELECT-ANNOTATION toma únicamente las anotaciones en el ámbito.
  - Un paso a través de terminales cuenta todo terminal, tenga pin o no: los terminales pertenecen al circuito. Lo que se dibuja se selecciona a través de los pines. Así, `&vss @SELECT-INSTANCE` incluye un NMOS cuyo único terminal en vss es un sustrato que su símbolo no dibuja. Un puerto de este nivel, como `#:inp`, conduce a su red.
  - Con usos como ámbito, se toman únicamente los miembros de la clase seleccionada; los miembros no vuelven a usarse como ámbito (véase abajo).
  - Por ejemplo, los transistores controlados por el reloj son `&clk @SELECT-INSTANCE`; solo aquellos cuya puerta está en clk son `&clk @SELECT-TERMINAL(terminal="g") => @SELECT-INSTANCE`; las redes de M1 son `#M1 @SELECT-NET`.
  - Nota: una regla que añade tierras, cables, etiquetas o estilos al dibujo selecciona pines (SELECT-PIN), de modo que lo que añade se ajuste a lo que se dibuja; una regla que comprueba el circuito selecciona terminales.
- **Por símbolo: `symbol` y `drawn`.** SELECT-INSTANCE y SELECT-USAGE admiten `symbol="$X"`, una cadena: el objeto se dibuja con un símbolo asignado a través de ese nombre `$`, el nombre escrito en el SET-SYMBOL que lo asignó, comparado tal como está escrito y con el comodín `*`, como `symbol="$IO_PORT*"`; un símbolo asignado a través de un nombre `%` o escrito de forma anónima no coincide con ningún `symbol=`. Un alias de un nombre `$` en ese SET-SYMBOL se lee como el nombre original (16.5), por lo que se compara el original; la cadena de `symbol=` no se traduce (16.5) y se escribe con el nombre original. A diferencia de la definición de SELECT-USAGE, que se reconoce por aquello a lo que se resuelve su nombre (9.5), esto es una comparación de nombres; los parámetros solo admiten literales (3.7), por lo que el nombre se escribe como cadena, igual que otras condiciones escriben nombres. También admiten `drawn` (`true`: el objeto se dibuja, con un símbolo asignado por una sentencia, una regla, un símbolo de bloque o el símbolo de bloque por defecto (9.2.6); `false`: no se dibuja, como un uso sin símbolo). Toda instancia se dibuja. Estas condiciones leen los símbolos, por lo que funcionan en reglas de generación y en la cola de estilos, que vienen después de asignados los símbolos (8.2); escritas en una selección de SET-SYMBOL son un error, pues la cola de símbolos no puede seleccionar por el símbolo que está asignando. SELECT-PIN también lee símbolos, con la misma restricción, y lo mismo cualquier selección cuyo ámbito sea un pin o pase por pines, como `#R1::p @SELECT-TERMINAL`: en una selección de SET-SYMBOL es un error. Lo mismo ocurre con un ámbito escrito con `::` y un nombre que no sea un nombre de rol, un nombre local ni un nombre estándar de subobjeto del objeto, como `#X1::k`: si nombra un pin o una clave de un texto que trae el símbolo se sabe solo una vez asignados los símbolos (6.5).
- **Este nivel, o también los niveles inferiores.** Toda palabra clave de selección tiene la condición `recursive`, `false` por defecto. Una regla se evalúa en el nivel donde está escrita, y hasta dónde alcanza por debajo se decide como sigue; en cada evaluación, cada selección y cada conjunto en ella se evalúa en ese nivel y contiene únicamente los miembros de ese nivel (10.3).
  - **Una regla con una selección escrita en ella**, en su tubería o en un `{…}` escrito en ella, alcanza los niveles inferiores cuando alguna de esas selecciones escribe `recursive=true`, y solo entonces; los conjuntos que usa no cambian esto. Así, `# @SELECT-INSTANCE(recursive=true) => @SELECT-PIN(net="vss", wired=false) => $GROUND_STUB @ATTACH` añade tierras en este nivel y en todo nivel por debajo de él, mientras que `# @SELECT-NET(name="vdd") => @REMOVE-FROM $PORT_FILL_SET` actúa únicamente en su propio nivel.
  - **Una regla sin ninguna selección escrita en ella**, que toma conjuntos que dependen de selecciones (10.4), alcanza todo nivel inferior al suyo al que alcance uno de esos conjuntos. Un conjunto alcanza su propio nivel y los niveles a los que alcanzan las sentencias que añaden resultados de selecciones a él: la sentencia a la que está vinculado, los conjuntos entre sus elementos y las sentencias ADD-TO que le añaden, donde un ADD-TO sin selección escrita en él cuenta como si alcanzara hasta donde alcanza el conjunto que añade, no el conjunto al que añade. En un nivel al que un conjunto no alcanza, no tiene miembros, incluso cuando lo use una regla que alcance hasta allí. Así, con `$GROUND_NET_SET = # @SELECT-NET(name="0", recursive=true)`, el conjunto tiene la red 0 de cada nivel, `%g_set = {$GROUND_NET_SET}` alcanza igual de lejos, y `$IO_PORT @ATTACH $PORT_FILL_SET` o `$SUPPLY_NET_SET @REMOVE-FROM $PORT_FILL_SET` actúa en todo nivel donde alcancen esos conjuntos.
  - A un único objeto del anfitrión simplemente se le hace referencia, como `&vss`: las referencias al anfitrión se resuelven únicamente en su propio nivel, y un conjunto de ellas, como `{&vss}`, no alcanza ningún nivel inferior.
- **SELECT-USAGE y SELECT-ANCESTOR-USAGE.** Ambas reconocen una definición por aquello a lo que se resuelve su nombre (9.5), seleccionan siempre usos y no tienen `role=`.
  - SELECT-USAGE mira hacia dentro, como las demás palabras clave de selección. Con `#` como ámbito, selecciona los usos del nivel que se está evaluando; con un uso o un conjunto de usos, esos propios usos y los usos dentro de ellos, hasta los más internos. Las instancias, redes y terminales en el ámbito no son usos y no seleccionan nada; una advertencia remite a SELECT-ANCESTOR-USAGE para los usos en los que están.
  - SELECT-ANCESTOR-USAGE mira hacia fuera: selecciona los usos que tienen un objeto del ámbito como miembro, directa o indirectamente (véase el ejemplo abajo). Los usos en el ámbito no se incluyen ellos mismos. Con `#` como ámbito es un error: un nivel no es miembro de nada.
- **Usos como ámbito.** Con usos como ámbito, las demás palabras clave de selección seleccionan los miembros de su clase en esos usos, y `role=` toma únicamente los miembros de ese rol. SELECT-PIN difiere solo para un uso con símbolo y sin `role`, que da los pines de su propio símbolo (arriba). Los miembros que son usos se siguen hacia dentro, hasta los más internos:

  ```rnl
  $OTA = @DEFINE(roles=[["pair", "usage"], ["load", "usage"]]) {};
  %dp1 = #M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N;
  %cm1 = #M3(ref) #M4(out) @AS $CURRENT_MIRROR_P;
  %ota1 = %dp1(pair) %cm1(load) @AS $OTA;
  %all_set = %ota1 @SELECT-INSTANCE;                 /* {#M1 #M2 #M5 #M3 #M4} */
  %pair_set = %ota1 @SELECT-INSTANCE(role="pair");   /* {#M1 #M2 #M5} */
  %up_set = #M1 @SELECT-ANCESTOR-USAGE $OTA          /* {%ota1} */
  ```

  - El valor de `role=` admite también los alias de `definition=` de la definición del uso (16.5). Para los miembros directos únicamente, úsese el rol como subobjeto, como `%dp1::tail`. El recorrido se detiene en un uso ya visitado. Los miembros son los objetos que rellenan roles, no los objetos locales del cuerpo (9.4).
  - **Únicamente miembros de la clase seleccionada.** Los miembros no se usan como ámbito para ir más allá, y los miembros de otras clases no cuentan. SELECT-TERMINAL toma únicamente miembros terminales: tras `$GROUND @ATTACH &0; $CURRENT_ARROW @ATTACH #M5:1`, `# @SELECT-USAGE => @SELECT-TERMINAL` es únicamente {#M5:1}, no los terminales en la red 0. Los miembros del par diferencial %dp1 son instancias, por lo que `%dp1 @SELECT-TERMINAL` es vacío. Para los terminales de las instancias miembro, escríbase `%dp1 @SELECT-INSTANCE => @SELECT-TERMINAL(terminal=2)` (las puertas de los tres transistores, incluido el transistor de cola); para los terminales en las redes miembro, `… => @SELECT-NET => @SELECT-TERMINAL`. Cada segmento hace una sola cosa; para ir más allá, añádase un segmento. Los objetos dados directamente en el ámbito siguen las reglas anteriores: una red da los terminales en ella, una instancia sus terminales.
  - **Un rol rellenado mediante una conversión** (5.2.4), como un rol de clase `"pin"` que recibe un terminal, conserva el objeto tal como fue escrito hasta que se asignan los símbolos, y el objeto convertido desde ese momento. Por ello, en las reglas de generación y después, `@SELECT-PIN(role="t")` selecciona el pin, tanto si el rol recibió el pin por su nombre como si recibió un terminal que lo representa.
- Para buscar objetos en la escena por condición, úsese siempre una palabra clave de selección; `::` toma únicamente partes del propio objeto (6.5).
- **Agrupaciones.** Los miembros de una agrupación no pueden seleccionarse; en su lugar se selecciona la agrupación (11.8). SELECT-INSTANCE selecciona la propia agrupación según las condiciones de su primer miembro, una vez por cada agrupación. SELECT-TERMINAL selecciona los terminales de la agrupación, y `net=` coincide cuando coincide con la red de cualquiera de los miembros.
- **Las condiciones** se escriben por nombre, como `name=value`; un objeto se selecciona únicamente cuando se cumplen todas las condiciones escritas. Para "o", escríbanse varias reglas. Las condiciones que comparan nombres (`name`, `model`, `net`) admiten el comodín `*`, que coincide con cualquier número de caracteres, y comparan según las reglas de mayúsculas y minúsculas del anfitrión, igual que DEVICE (6.12). Para seleccionar por letra de elemento, escríbase `name="R*"`. Los nombres de instancia de Spectre no empiezan por una letra de elemento; selecciónense por `model`, como `model="resistor"` (6.12).
- **`model`** compara el nombre escrito en la posición de modelo de la línea de instancia: para M, Q, D y otros elementos, el nombre de `.model`; para instancias X, el nombre del subcircuito, incluidos dispositivos de PDK que el netlist no define, como `# @SELECT-INSTANCE(model="sky130_fd_pr__nfet_01v8")`; en Spectre, el maestro. Es igual que `model=` de DEVICE (6.12). `type` compara el tipo de la instancia: el tipo de su `.model`, o, para una instancia a la que su anfitrión no le da ninguno, como una instancia X, el tipo que le da un DEVICE (6.12); sin ninguno de los dos, la instancia no tiene tipo y no coincide con ningún `type=`. `type` admite el comodín `*` como las condiciones que comparan nombres: `type="*mos"` coincide con los tipos nmos y pmos, y `type="*"` selecciona las instancias que tienen algún tipo, cualquiera que sea, que es como una regla o una aserción excluye los dispositivos cuyo tipo se desconoce.
- **`terminals`** compara el número de terminales de la instancia, un número entero: los nodos que el anfitrión lee en su línea, que tienen los números 1, 2, … (6.12), por muchos nombres que les dé DEVICE. Así, `terminals=3` selecciona `Q1 c b e qn` y no `Q2 c b e s qn`. Para una instancia X cuenta los nodos de su línea, es decir, los puertos del subcircuito; en Spectre, los nodos entre paréntesis, por lo que `S1 (com t1 t2) switch` tiene 3. Se combina con las demás condiciones como ellas, por ejemplo `# @SELECT-INSTANCE(type="nmos", terminals=4) => @SET-SYMBOL $NMOS_4TERM`; una agrupación se selecciona por su primer miembro (11.8).
- La comparación sigue a 3.6: los nombres de condición y los valores de enumeración estándar distinguen entre mayúsculas y minúsculas; los valores que nombran cosas del anfitrión (nombres, nombres de modelo, nombres de red) siguen las reglas del anfitrión.
- Una red puede seleccionarse por nombre: `# @SELECT-NET(name="0")` la selecciona, y `# @SELECT-TERMINAL(net="0")` los terminales en ella. Los objetos también pueden seleccionarse por significado: `# @SELECT-USAGE $DIFFERENTIAL_PAIR_N => @SELECT-INSTANCE(role="left")` selecciona el transistor izquierdo de cada par diferencial.
- No hay ninguna palabra clave para una intersección, como los miembros de un conjunto que pertenecen a un uso; escríbase con REMOVE-FROM dos veces (5.2.2):

  ```rnl
  %s_set = {#M1 #M3 #M5};
  %s_set @SELECT-ANCESTOR-USAGE $DIFFERENTIAL_PAIR_N => @SELECT-INSTANCE => %dp_m_set;
  %out_set = {%s_set};
  %dp_m_set @REMOVE-FROM %out_set;    /* the members of %s_set in no differential pair */
  %in_set = {%s_set};
  %out_set @REMOVE-FROM %in_set       /* the members of %s_set in a differential pair */
  ```

- **Redes de tierra y alimentación** vienen dadas por conjuntos que proporciona el paquete estándar, `$GROUND_NET_SET` (la red 0 por defecto) y `$SUPPLY_NET_SET` (vacío por defecto). No existen clases de red.
  - Un paquete no puede contener referencias al anfitrión (16.2), por lo que el contenido por defecto de tal conjunto solo puede escribirse como una selección, como `# @SELECT-NET(name="0", recursive=true)`.
  - El usuario añade y elimina redes con ADD-TO y REMOVE-FROM:

    ```rnl
    &vss @ADD-TO $GROUND_NET_SET;                                       /* the vss of this level */
    # @SELECT-NET(name="vss", recursive=true) => @ADD-TO $GROUND_NET_SET;   /* the nets named vss of every level */
    # @SELECT-NET(name="vdd!", global=true, recursive=true) => @ADD-TO $SUPPLY_NET_SET  /* only the global vdd! */
    ```

    La segunda sentencia añade también redes locales llamadas vss en subcircuitos. Para excluir una red en un solo lugar, cópiese el conjunto y elimínese la red de la copia: `%g_set = {$GROUND_NET_SET}; &vss2 @REMOVE-FROM %g_set`.
  - Una regla de relleno usa tal conjunto como su ámbito, como `$GROUND_NET_SET @SELECT-PIN(wired=false, recursive=true) => $GROUND_STUB @ATTACH`, que también toma los pines que los componentes virtuales tienen en esas redes; el paquete estándar excluye luego los pines de los componentes que él mismo coloca, como los de las tierras.
  - Escribir `$GROUND_NET_SET = …` en el propio nivel oculta el conjunto del paquete, y las reglas del paquete no ven el nuevo (6.8).
  - Las tierras y alimentaciones no se adivinan por los nombres de red. Un importador escribe el dominio de alimentación de una red en un archivo de editor (como `powerDomain`) como ADD-TO.
  - Un solucionador puede leer los conjuntos estándar como pistas (como alimentaciones arriba y tierras abajo), del mismo modo que puede leer las definiciones del paquete estándar; esto no es obligatorio.
  - Nota: no hay ninguna palabra clave para redes de tierra y alimentación (1.5 punto 7): los usos de estos conjuntos, como seleccionar terminales para añadir tierras o cambiar símbolos, pueden escribirse todos como reglas en un paquete.
- **El nodo vacío** (6.3) no es una red: SELECT-NET nunca lo devuelve, un pin en él es seleccionado por SELECT-PIN únicamente a través de su objeto o `#`, no a través de una red ni de `net=`, y un cable en él por SELECT-WIRE sin `net=`.
- **Redes globales.** La condición `global=true` de SELECT-NET selecciona únicamente redes globales (6.3). Las redes locales con el mismo nombre en distintos niveles son nodos distintos, y seleccionar por nombre las selecciona todas; para tomar solo la global, añádase esta condición.

### 10.2 La condición `wired`

- Un pin tiene `wired=false` cuando no es ninguno de los puntos de ningún cable explícito (un extremo o un punto intermedio; un cable escrito hacia un terminal termina en el pin del terminal, 5.2.4), y no está en ningún árbol de ROUTE-NET o ROUTE-OBJECT (13.7), pues el solucionador dibuja los cables de un árbol. Un pin de bus es un único pin, cableado cuando un cable termina en él a través de cualquiera de sus terminales (9.2.4). Los cables de bus son también cables explícitos (13.1). Un cable hacia un terminal de una agrupación conecta ese terminal de cada miembro (11.8).
- Una red tiene `wired=false` cuando ningún terminal de una instancia del anfitrión en ella es tocado por un cable explícito y ningún árbol de ROUTE-NET o ROUTE-OBJECT está en ella; los pines de componentes virtuales no cuentan.
- Qué pines están en árboles se fija en dos etapas de 8.2. Tras el paso 4, una vez asignados los símbolos, a partir de las sentencias del paso 3: las reglas de ROUTE-NET, ROUTE-OBJECT, UNROUTE-NET y UNROUTE-OBJECT, que se evalúan en el paso 5 (10.4), leen estos árboles, como `{&a &b} @SELECT-NET(wired=false) => @ROUTE-NET`, y no se ven unas a otras, de modo que una regla nunca lee los árboles que ella misma crea. Tras el paso 5, con los árboles que esas reglas crean y modifican: las condiciones del paso 6 y de los pasos posteriores leen estos. Por ello, una regla de generación, como el relleno del paquete estándar (16.3), cuenta los pines que una regla ROUTE une como cableados, y los pines que una regla UNROUTE excluye como no cableados, salvo que un cable escrito los toque. Las sentencias de trazado en contenido generado en el paso 6, como el cuerpo de un uso generado, también crean y modifican árboles, pero ninguna condición las ve.
- Un cable escrito en el nodo vacío (13.1) cuenta como cualquier otro: los pines en los que termina quedan cableados.
- Solo cuentan los cables escritos explícitamente, no los generados. Que haya componentes virtuales no marca diferencia alguna.
- Los cables cuentan tal como están escritos, desde el paso 3 de 8.2, incluido un cable que falle después porque un terminal carece de pin (7.1, 9.2.4). Para un cable que se reescribe, cuenta la última reescritura (13.1).

### 10.3 Evaluación de selecciones

- Una selección no se expande donde está escrita, y cada evaluación atiende únicamente a un nivel. Una regla (10.4) se evalúa una vez en el nivel donde está escrita y una vez en cada nivel inferior al que alcanza (10.1); en cada evaluación, `#` es ese nivel, y la selección selecciona únicamente objetos de ese nivel. Así, `# @SELECT-INSTANCE(model="nch", recursive=true) => @ALIGN("y")` escrito en el nivel superior alinea una vez en cada dibujo, y no alinea instancias de dibujos distintos entre sí.
- Una selección se evalúa en la fase que la utiliza: las reglas de apariencia en la cola de símbolos y la cola de estilos, las reglas de las palabras clave de trazado en el paso 5, las reglas de generación en la fase de generación (8.2). Lo mismo rige para los conjuntos (5.2).
- La clase que selecciona una selección se conoce al leerla. Pasarla a una ranura que no admite esa clase es un error de inmediato, incluso dentro de un conjunto, como `# @SELECT-TERMINAL(net="vss") => @SET-SYMBOL $NMOS`.
- SELECT-USAGE y SELECT-ANCESTOR-USAGE seleccionan siempre usos; cuando una selección pasa de usos a sus miembros, la clase seleccionada viene dada por su palabra clave. Por tanto, la clase de toda selección se conoce al leerla.
- El resultado de una selección es un conjunto y puede nombrarse con `%` o `$`. Un conjunto evaluado en un nivel toma únicamente los miembros de ese nivel: `&vss @ADD-TO $GROUND_NET_SET` escrito en el nivel superior añade solo la vss del nivel superior, que no está en el conjunto cuando se evalúa en un subcircuito (6.6).
- **El orden del resultado.** Los miembros del resultado de una selección están en este orden, que importa dondequiera que importe el orden de un conjunto, como en ORDER, DISTRIBUTE, TABLE y LINE, y para los resultados de una regla aplicada uno por uno (5.2.1):
  - Con `#` como ámbito, los objetos del nivel en el orden de la escena:
    - instancias en el orden de sus líneas en el netlist, una agrupación en la línea de su primer miembro; terminales por instancia o agrupación, luego por número;
    - redes: los puertos del nivel en el orden de la línea de subcircuito, luego las demás redes en el orden en que aparecen por primera vez en las líneas de instancia;
    - pines por el objeto al que pertenecen, primero instancias y agrupaciones en el orden de las líneas (una agrupación en la línea de su primer miembro), luego usos en su orden; dentro de un mismo objeto, en el orden de las sentencias PIN de su símbolo;
    - usos, cables, textos y gráficos: primero los creados en el paso 3 de 8.2, en el orden de sus posiciones (8.5), viniendo lo que crea el cuerpo de un uso justo después de ese uso, en el orden del cuerpo; luego los creados en pasos posteriores, paso por paso. Dentro de un mismo paso, están en el orden de las posiciones de las sentencias o reglas que los crean, los resultados de una regla en el orden de su entrada (5.2.1), y los textos que traen los símbolos por el objeto al que pertenecen, tomados los objetos en el orden dado para los pines, luego en el orden del símbolo. Un texto de la escena que sustituye a un texto del símbolo por clave (13.3) es un solo texto, en la posición de la primera sentencia que lo escribe.
  - Con cualquier otro ámbito, cada miembro del ámbito da sus resultados por turno, cada uno en el orden anterior, y un objeto ya dado no se da de nuevo. Esto rige para toda palabra clave de selección, incluida SELECT-ANCESTOR-USAGE.
  - Los conjuntos conservan el orden en que se escriben y añaden sus miembros (5.2, 5.2.2): ADD-TO añade al final y conserva repeticiones, y UNION las elimina.
- Cuando el ámbito de una selección se escribe como objetos del anfitrión de su nivel, como `&Q<0:3:2, 1> @SELECT-TERMINAL(name="R*", recursive=true) => @TEXT("${net}", key="netname")` en el nivel superior, esos objetos no existen cuando la regla se evalúa en un nivel inferior: la selección es vacía allí y no hace nada, sin advertencia, igual que un conjunto toma únicamente los miembros del nivel. La advertencia de 10.5 se emite únicamente para los demás roles de reglas de generación.

### 10.4 Reglas

- Una sentencia que toma el resultado de una selección, directamente o a través de un conjunto que depende de una, es una regla. Un conjunto depende de una selección cuando está vinculado a una, cuando tiene una o tal conjunto entre sus elementos, cuando se crea mediante UNION de tales conjuntos, o cuando ADD-TO o REMOVE-FROM lo modifica con el resultado de una selección o con tal conjunto (5.2.2): sus miembros se conocen entonces únicamente en la fase que lo utiliza. En el cuerpo de un uso, un rol de conjunto depende de una selección cuando lo hace lo que lo rellena.
- Toda palabra clave salvo BUNDLE (11.8), WIRE (13.1), y RECTANGLE, CIRCLE y ELLIPSE escritos con dos puntos (9.2.5), puede tomar el resultado de una selección, uno por uno o en bloque según indica 5.2.1: `# @SELECT-INSTANCE(type="nmos") => @ORIENT("MY")`, `# @SELECT-INSTANCE(model="nch") => @ALIGN("y")`.
- SET-SYMBOL y SET-STYLE con el resultado de una selección como su objeto son reglas de apariencia y entran en las colas de apariencia (8.3). AS y ATTACH son reglas de generación (10.5), y también lo son TEXT adjuntado al resultado de una selección (13.3), RECTANGLE, CIRCLE y ELLIPSE escritos con un tamaño cuyo punto es ese resultado (9.2.5), y las reglas de LINE, ARC y BEZIER, que crean objetos dibujados; su ranura toma un conjunto, por lo que tal regla dibuja una sola figura a través de todo el resultado, en su orden (5.2.1, 10.3). Las reglas de ROUTE-NET, ROUTE-OBJECT, UNROUTE-NET y UNROUTE-OBJECT se evalúan antes que ellas, en el paso 5: sus condiciones leen los árboles de las sentencias del paso 3, y las reglas de generación ven los árboles que crean (10.2). Las reglas de CURRENT se evalúan con las reglas de generación, en el paso 6; juntas son las reglas del paso 6: las direcciones de corriente quedan fijadas a partir de las sentencias del paso 3, y las condiciones del paso 6 no ven lo que modifican estas reglas (15.2). BUNDLE no admite el resultado de una selección: una agrupación debe existir antes de asignarse los símbolos (11.8). Las reglas de otras palabras clave, como las restricciones, PLACE y ORIENT, surten efecto en el paso de su palabra clave y ven los objetos generados. Con cualquier otra palabra clave, una regla equivale a una sentencia escrita para cada objeto seleccionado, o una escrita para todos ellos, y surte efecto en la fase de su palabra clave.
- Una regla se aplica al nivel donde está escrita, y a los niveles inferiores que alcanza (10.1), mediante `recursive=true` en una selección escrita en ella o, si no tiene ninguna, mediante los conjuntos que usa, evaluada una vez en cada nivel y atendiendo únicamente a ese nivel (10.3). Una regla en un paquete cuenta como escrita en el nivel del USE-PACKAGE del paquete, y un paquete en la raíz como usado en el nivel superior de su archivo (16.2), de modo que alcanza los niveles inferiores únicamente según indica 10.1, razón por la cual el paquete estándar escribe `recursive=true` en sus selecciones. Una regla en el cuerpo de una definición general cuenta como escrita en el nivel del uso (9.4).
- **Objetos del anfitrión de un nivel en una regla que alcanza los niveles inferiores**: una regla que alcanza los niveles inferiores y cuyas otras ranuras nombran objetos del anfitrión de su nivel, como `%clk_set %rest_set @DISTANCE("bottom-to-top", min=3)` con `%clk_set = {#VP1 #VP2}` y `%rest_set` vinculado a una selección con `recursive=true`, no encuentra tales objetos en un nivel inferior: allí la sentencia no tiene efecto, y no se emite advertencia. Únicamente las reglas de generación advierten, según indica 10.5.

### 10.5 Generación uno por uno

"Para cada terminal en vss sin cable, colóquese una tierra y conéctese con un cable":

```rnl
$GROUND_STUB = @DEFINE(roles=[["t", "pin"]], parameters=[["gap", 2]]) {
  %g = $GROUND @ATTACH ?t::net;
  ?t::net @WIRE {?t %g::pin};
  {?t %g::pin} @ALIGN("x");
  %g::pin ?t @DISTANCE("bottom-to-top", exact=?gap)
};
# @SELECT-INSTANCE => @SELECT-PIN(net="vss", wired=false) => $GROUND_STUB @ATTACH => %stubs_set
```

El paquete estándar proporciona un `$GROUND_STUB` como este, salvo que su hueco es 1 por defecto y da únicamente la distancia mínima (`min=?gap`), para que un solucionador pueda dibujar el cable más largo, como cuando alinea las tierras de componentes de distintas longitudes, que apunta la tierra hacia abajo (`@ORIENT("R0")`), y que sus restricciones son de prioridad `"low"`; la definición aquí lo oculta (6.8).

- Cuando un rol único de AS o ATTACH recibe el resultado de una selección, la definición se aplica una vez por cada objeto seleccionado, y los demás roles son los mismos en cada aplicación (5.2.1, 9.4). Un rol de conjunto toma el resultado de una selección en bloque, y la definición se aplica una sola vez; cuando todos sus roles son roles de conjunto, solo en los niveles donde alguno contiene algo (9.4).
- Los componentes virtuales se crean uno por uno del mismo modo: `# @SELECT-NET(port=true) => $IO_PORT @ATTACH` da a cada red de puerto una etiqueta de puerto; con `$RLOAD`, una definición de dos roles dibujada como una resistencia, `# @SELECT-NET(name="out*", recursive=true) => $RLOAD @ATTACH &vss` adjunta una entre cada red seleccionada y vss; como `&vss` pertenece a este nivel, la regla surte efecto únicamente aquí, con la advertencia descrita abajo.
- El relleno del paquete estándar añade tierras y alimentaciones con un cable (`$GROUND_STUB`, `$SUPPLY_STUB`): una vez escritas de vuelta, el cable hace que el pin deje de tener `wired=false`, y el relleno lo omite. Una tierra puesta en un pin solo mediante restricciones, sin cable, deja el pin con `wired=false`, por lo que el relleno sigue añadiendo una allí; un cable de longitud 0 entre ambos pines (13.1) convierte al pin en cableado.
- El emparejamiento se da dentro de cada aplicación: los nombres locales de una definición son nuevos en cada aplicación, de modo que el `%g` de una aplicación es la tierra creada en esa aplicación.
- El resultado es un conjunto generado. Puede nombrarse y pasarse en bloque a SET-SYMBOL o SET-STYLE (como `%stubs_set @SET-STYLE $grey`), pero no puede hacerse referencia a ningún miembro individual suyo (5.7).
- **Escribir uno a mano.** Para describir uno de ellos por separado, créese a mano y cabléese. Su pin deja entonces de tener `wired=false`, y el relleno lo omite. No existe ninguna forma aparte de cancelar una generación.
- **Instantánea.** Todas las reglas de generación ven la misma instantánea, el resultado de los pasos 3 a 5 de 8.2 (tras la sustitución por clave, con los símbolos de todos los objetos y los árboles del paso 5), y no ven los resultados unas de otras. Por tanto, la misma regla escrita dos veces genera dos copias.
- Los hechos generados toman la posición de su regla de generación en la resolución de conflictos (8.5).
- Los objetos generados sin posición los coloca el solucionador; para indicar cómo se colocan, úsese una definición con restricciones.
- **Escritura de vuelta.** No puede hacerse referencia a un único objeto generado (5.7). Cuando el solucionador los escribe de vuelta, los escribe como sentencias explícitas, como `$IO_PORT @ATTACH &out => @PLACE(40, 20) => %p_out` con el cable hasta él. El objeto explícito ocupa el lugar del generado únicamente cuando la regla excluye lo que ya está escrito, como hacen los rellenos del paquete estándar al seleccionar solo pines sin cables (10.1) y como hace un relleno por red al escribir la diferencia de dos conjuntos (abajo); una regla escrita de otro modo crea el objeto de nuevo junto al explícito, sin posición. Por tanto, una regla cuyos objetos coloca el solucionador debería excluir lo que ya está escrito, de una de estas dos maneras. Los objetos en el cuerpo de un uso explícito escrito así que carezcan de nombre local se referencian por número (5.9). El uso se escribe con la prioridad que escribe la regla en su aplicación, tal como está escrita, o con `"disabled"` cuando la recibe del encabezado del registro de la regla (14.8); de otro modo se escribe sin prioridad, para que su cuerpo conserve las prioridades que tenía, y no se escribe un encabezado de registro con otra prioridad.
- La generación no es recursiva: cuando el contenido expandido por una regla de generación contiene reglas del paso 5 o 6 (10.4), estas fallan, puesto que cada uno de esos pasos se ejecuta una vez. Sus demás reglas surten efecto en los pasos posteriores como de costumbre: SET-SYMBOL en la cola de símbolos del paso 7; PLACE, ORIENT, LAYER, PRIORITY y las restricciones en el paso 8, de modo que el cuerpo de un uso generado pueda colocar y orientar lo que reciben sus roles; SET-STYLE en la cola de estilos; las aserciones cuando se comprueban (8.2, 15.3).
- Cuando otro rol de una regla de generación que alcanza los niveles inferiores (10.1) se rellena con un objeto del nivel (un objeto de la escena o un objeto del anfitrión), la regla surte efecto únicamente en el nivel donde está escrita, con una advertencia: esos objetos pertenecen a ese nivel, y los niveles inferiores no los tienen. Un rol que recibe una definición no tiene tal límite.
- Un SET-SYMBOL o SET-STYLE creado por una generación pertenece al grupo de la regla que lo creó, en la posición de la regla (8.3).
- Un nombre vinculado a una selección o a un conjunto generado no puede vincularse de nuevo (6.6).
- **Relleno por red.** No existe ninguna regla integrada "no añadir ninguno donde ya exista uno". Un relleno escribe la diferencia de dos conjuntos; un paquete que añade etiquetas de puerto escribe:

  ```rnl
  $PORT_FILL_SET = # @SELECT-NET(port=true, recursive=true);                                /* the nets that get a port tag */
  # @SELECT-USAGE(recursive=true) $IO_PORT => @SELECT-NET => @REMOVE-FROM $PORT_FILL_SET;   /* less those that have one */
  $IO_PORT @ATTACH $PORT_FILL_SET
  ```

  - El relleno de la serie estándar 0.4 se escribe de esta manera, con el conjunto en `rnl-std-basic`, y además coloca las etiquetas de cada nivel en una columna (12.4). Deja las redes de alimentación y de tierra en el conjunto; un archivo que no quiere etiqueta donde están sus tramos cortos escribe `$SUPPLY_NET_SET @REMOVE-FROM $PORT_FILL_SET; $GROUND_NET_SET @REMOVE-FROM $PORT_FILL_SET`.
  - Una etiqueta de puerto escrita a mano ocupa el lugar de la generada: tras `$IO_PORT @ATTACH &inp => @PLACE(0, 20) => %p`, inp no recibe ninguna etiqueta de puerto generada. El usuario también puede quitar redes de `$PORT_FILL_SET`, como `# @SELECT-NET(name="vdd") => @REMOVE-FROM $PORT_FILL_SET`, o contar los usos de su propia definición de puerto: `# @SELECT-USAGE $MY_PORT => @SELECT-NET => @REMOVE-FROM $PORT_FILL_SET`.
  - Las etiquetas de puerto generadas no cuentan: las reglas de generación usan `$PORT_FILL_SET` en el paso 6, y las selecciones de la diferencia se evalúan sobre el estado previo al paso 6 (5.2).
  - Nota: "ya existe uno" no está integrado porque no queda claro para una definición con varios roles, las definiciones de puerto propias del usuario no contarían, y una definición de disposición repetida solo repite restricciones, que la resolución de conflictos y las prioridades manejan (8.5, 14.8).
- En un relleno por terminal, sigue siendo el cable de un objeto escrito a mano el que ocupa el lugar del generado: un pin con un cable deja de tener `wired=false`. Sigue sin haber manera de cancelar una generación: quitar una red del conjunto solo cambia el conjunto.

<!-- translated from en/ch11.md sha256:693deb0f50aa -->

## 11 Colocación y apariencia

### 11.1 Posición: PLACE

```rnl
#M1 @PLACE(10, 20);
#M1 @PLACE(x=10)
```

- PLACE asigna una posición a un objeto; debe escribirse al menos uno de `x` e `y`. PLACE fija únicamente las coordenadas escritas. Cada coordenada es libre al principio, y una coordenada que nunca se escribe permanece libre, para que la decidan las restricciones y el solucionador.
- PLACE fija únicamente coordenadas, no rotación, reflexión, escala ni las direcciones de los terminales; la orientación se fija con ORIENT (11.2).
- Un PLACE son dos hechos geométricos, uno para x y otro para y, y en un conflicto cada uno puede descartarse por separado (8.5). El parámetro `priority=` indica su prioridad (14.8), como `#M1 @PLACE(12, 20, priority="high")`: x e y por posición van primero, y los parámetros por nombre después de ellos (4.2 punto 9).
- Objetos: instancias, usos (por su propio punto, 14.1), agrupaciones (11.8), puntos, TEXT (anotaciones y etiquetas; PLACE fija la posición del propio texto, 13.3) y el origen `::position` (incluido el centro de un RECTANGLE, CIRCLE o ELLIPSE escrito con un centro). PLACE no admite otros gráficos; para moverlos, muévanse sus puntos (13.4). En el cuerpo de una definición general, sus coordenadas son desplazamientos respecto al propio punto del uso (9.4). En un cuerpo de símbolo, PLACE únicamente posiciona los TEXT adjuntados a `?`, en coordenadas locales (9.2.2).
- La salida conserva el manejador de la entrada (5.1).
- Un conjunto como objeto se coloca miembro por miembro (5.2.1): `%pair_set @PLACE(10, 20)` sitúa ambos miembros en (10, 20). Un conjunto no tiene posición propia (14.1).
- Para los ejes, la unidad, los decimales y la tolerancia de las coordenadas, véase 11.7.

### 11.2 Orientación: ORIENT

```rnl
%pair_set = {#M3 #M4};
#M2 @ORIENT("MY");          /* flipped left-right */
%pair_set @ORIENT("R90");   /* each member turns about its own origin and stays where it is */
#M5 @ORIENT(["R0", "MY"])   /* not turned, or flipped left-right: the solver chooses */
#M6::g @ORIENT("left")      /* M6 is oriented so that its gate faces left */
```

- Objetos: instancias, usos con símbolo y agrupaciones; y pines (abajo). ORIENT sobre un uso sin símbolo falla (12.1); no se admiten puntos, etiquetas, gráficos, cables ni redes.
- Un conjunto como objeto se orienta miembro por miembro (5.2.1): cada miembro gira alrededor de su propio origen y permanece donde está, y cada miembro es una descripción. El conjunto no se gira en bloque. No se proporciona el giro de todo un conjunto a la vez: un conjunto no tiene posición (14.1).
- Un uso con símbolo gira alrededor de su propio punto; tiene una orientación propia y no gira con los objetos que rellenan sus roles (12.1). Lo que dibuja o coloca el cuerpo de su definición tampoco gira con él (9.4).
- **Una lista de códigos**, como `["R0", "MY"]`, forma un solo hecho, que se cumple cuando el objeto tiene una de las orientaciones enumeradas. Se conserva o se descarta en bloque (8.5), y el solucionador elige entre los códigos. La lista tiene al menos un código; una lista de un solo código equivale a ese código. El solucionador escribe de vuelta la orientación que eligió como un código único en su registro (14.8); la lista en el registro del autor permanece, y la siguiente resolución vuelve a elegir. Un renderizador que no resuelve, y no tiene resultado para el objeto, dibuja el primer código de la lista.
- Sin ORIENT no hay restricción, y el solucionador elige; un renderizador que no resuelve dibuja `"R0"` (8.5). Las relaciones entre las orientaciones de varios objetos (la misma orientación, imágenes reflejadas) se escriben con ORIENT-RELATION (14.9).
- ORIENT es un hecho geométrico y participa en la resolución de conflictos (8.5); el parámetro `priority=` indica su prioridad (14.8).
- **Códigos de orientación.** Cada código representa una orientación. Únicamente se admiten las ocho grafías de la tabla y las cuatro palabras siguientes, y distinguen entre mayúsculas y minúsculas (3.6). Los nombres son los de Virtuoso y, con y hacia arriba (11.7), también lo son sus significados. Las grafías equivalentes (como `"MXR180"`, que es MY) y los ángulos arbitrarios no se admiten, ni solos ni en una lista: son valores de parámetro erróneos y el ORIENT falla. Las herramientas escriben de vuelta únicamente estos ocho, uno cada vez. No existe ninguna forma relativa que gire un objeto más allá de su orientación actual (capítulo 17).

  | Código | (x, y) pasa a | Significado |
  |---|---|---|
  | `"R0"` | (x, y) | Sin giro |
  | `"R90"` | (−y, x) | Girado 90° en sentido antihorario |
  | `"R180"` | (−x, −y) | Girado 180° |
  | `"R270"` | (y, −x) | Girado 270° en sentido antihorario, es decir, 90° en sentido horario |
  | `"MY"` | (−x, y) | Reflejado respecto al eje y: invertido izquierda-derecha |
  | `"MX"` | (x, −y) | Reflejado respecto al eje x: invertido arriba-abajo |
  | `"MYR90"` | (−y, −x) | MY, luego R90 |
  | `"MXR90"` | (y, x) | MX, luego R90 |

- **Palabras para los cuatro giros.** Un giro también puede escribirse como una dirección, que significa el mismo código: `"left"` es `"R0"`, `"down"` es `"R90"`, `"right"` es `"R180"` y `"up"` es `"R270"`, también en listas. ORIENT-RELATION admite únicamente los códigos (14.9). Para un objeto, la palabra es solo otra grafía: `#M2 @ORIENT("left")` es `#M2 @ORIENT("R0")`. Para un pin se lee tal como dice: la dirección hacia la que mira el pin, medida desde la izquierda.
- **Pines.** ORIENT aplicado a un pin, o a un terminal que representa a su pin (5.2.4), restringe la dirección del pin en el dibujo: la dirección escrita con `direction=` en el símbolo (9.2.4), girada por la orientación de su objeto, una de las cuatro. Se cumple cuando el objeto tiene una de las orientaciones que giran el pin en ese sentido, dos de las ocho, por lo que no fija la orientación del objeto; una lista admite cada dirección enumerada. Un código reflejado aplicado a un pin representa la dirección en la que convierte a la izquierda: `"MY"` es `"right"`, `"MX"` es `"left"`. Tal ORIENT es un hecho geométrico propio, con el pin como clave, además de cualquier ORIENT del objeto; ambos se resuelven como dos hechos cualesquiera (8.5). Un pin sin `direction=` escrito no tiene nada que restringir: el ORIENT de ese pin falla, y los demás miembros de la misma sentencia siguen surtiendo efecto (5.2.1). Un renderizador que no resuelve, y no tiene resultado para el objeto, lo dibuja como si no se hubiera escrito el ORIENT de sus pines.
- **Orden de composición.** MXR90 y MYR90 primero reflejan y después giran; los nombres se leen en el orden de las operaciones, como en Cadence y en OpenDB de OpenROAD.
- **Cómo actúa.** La orientación actúa sobre las coordenadas locales del símbolo, alrededor de su origen, y después el origen se sitúa en la posición del objeto: un punto del símbolo se dibuja en la posición del objeto más la orientación aplicada a sus coordenadas locales. Los gráficos y las posiciones de los pines del símbolo se transforman rígidamente con él, y las direcciones en que salen los cables de los pines también giran (9.2.4). Los textos se muestran como indica 9.2.5; los glifos no se reflejan.
- **Solo dónde se dibujan las cosas.** Una orientación cambia únicamente dónde se dibujan los pines, no a qué terminales pertenecen. Cuando un símbolo de bloque se invierte de arriba abajo, el pin de inp se dibuja abajo y el cable hacia inp lo sigue. Los dos pines no se intercambian para mantener inp arriba: eso cambiaría qué cable se conecta a qué terminal, algo propio del netlist (Principios 2.2). Para dibujar inp en otro lugar, úsese otro símbolo (11.3).

### 11.3 Símbolos y estilos: SET-SYMBOL y SET-STYLE

```rnl
%w13 = &x @WIRE {#M1:1 #M3:1};
#M1 @SET-SYMBOL $NMOS;
# @SELECT-INSTANCE(model="nch") => @SET-SYMBOL $NMOS;
%w13 @SET-STYLE $signal;
# @SELECT-INSTANCE => @SET-STYLE $base
```

- SET-SYMBOL asigna un símbolo a un objeto. El objeto puede ser una instancia, un uso o una agrupación, o un conjunto de ellos (incluidos los resultados de selecciones y de generación uno por uno); a cada miembro se le asigna el símbolo una vez (5.2.1). En un subcircuito, `# @SET-SYMBOL` asigna el símbolo de bloque (9.2.6).
- Un uso al que se asigna un símbolo se dibuja: es un componente virtual (capítulo 12).
- **Emparejamiento de terminales con pines.** Los nombres de terminal de un símbolo son los que vinculan sus pines (9.2.4). Cuando SET-SYMBOL asigna un símbolo a una instancia, cada nombre de terminal del símbolo se empareja con el terminal de la instancia que tiene ese nombre, dado por DEVICE o por los puertos del subcircuito (6.12); un número se empareja con el terminal en ese lugar. Un terminal de la instancia que ningún pin vincula no tiene pin, como un sustrato en un símbolo de tres terminales. Un nombre de terminal del símbolo que la instancia no tiene deja su pin sin vincular a nada, con una advertencia.
  - `terminals=` cambia el emparejamiento para este SET-SYMBOL. Su forma estándar enumera, en el orden de los nodos de la línea de instancia (como `terminals` de DEVICE, 6.12), los nombres de terminal del símbolo: con un símbolo cuyos pines vinculan `"a"`, `"b"` y `"c"`, `#X1 @SET-SYMBOL(terminals=["b", "a", "c"]) $GATE` vincula el nodo 1 al pin de b, el nodo 2 al de a y el nodo 3 al de c. También puede enumerar pares, el terminal de la instancia primero y el del símbolo después: `terminals=[["d", "b"], ["g", "a"], ["s", "c"]]`; los terminales no enumerados en pares se emparejan por nombre. Una lista que no sea ni lo uno ni lo otro, o que nombre dos veces un terminal, es un parámetro erróneo y el SET-SYMBOL falla.
  - Para un uso, los nombres de terminal del símbolo se emparejan con los nombres de rol de su definición. Un número, o un nombre que ningún rol tiene, no vincula ningún rol y se emite una advertencia. `terminals=` enumera los nombres de terminal del símbolo en el orden de la lista de roles, o pares de un nombre de rol y un nombre de terminal del símbolo (12.1).
- SET-STYLE asigna un estilo a un objeto. El objeto puede ser una instancia, un uso, una agrupación, un cable o un gráfico (textos incluidos), o un conjunto de ellos (incluidos los resultados de selecciones y de generación uno por uno). No admite redes, niveles, restricciones, definiciones ni puntos. Los terminales se dibujan con el símbolo de su objeto y toman el estilo de su objeto. Un estilo asignado a un uso pasa a su símbolo y a los objetos dibujables creados en su cuerpo (8.3), de modo que un estilo asignado a un uso semántico sin símbolo no es un error; cuando no se dibuja nada en el cuerpo, no tiene efecto visible.
  - Nota: de otro modo, la regla `# @SELECT-USAGE => @SET-STYLE $base` del paquete estándar produciría una advertencia para todo uso semántico.
- La salida conserva el manejador de la entrada; cuando el objeto es un conjunto, la salida es ese conjunto.
- Ambos surten efecto mediante las colas de apariencia, no mediante sustitución por clave. Hay dos rangos: un SET-SYMBOL o SET-STYLE que nombra el objeto directamente está por encima de los que llegan al objeto a través de una selección o se le transmiten (8.3).
- Donde existen reglas por defecto (el paquete estándar, 16.3), no es necesario escribirlas.
- **Parámetros y estilos.** Los parámetros de estilo escritos en un objeto, como `size=` de TEXT, son el estilo directo de ese objeto, por encima de los estilos procedentes de selecciones o transmitidos. Un SET-STYLE que nombra el objeto directamente y se escribe después los sustituye por posición (8.3); así es como un editor cambia un solo objeto. Un parámetro de una definición que no se pasa cuenta como no escrito, de modo que el estilo surte efecto (9.4).
- **Etiquetas y puntos de unión.** El estilo de un objeto se transmite a los textos adjuntados a él. En el rango de estilos seleccionados o transmitidos, se compara por posición con las reglas que seleccionan el texto (como SELECT-LABEL); los parámetros del propio texto y un SET-STYLE que nombra directamente el texto están en el rango directo. Así, cuando `#M1 @SET-STYLE $signal` se escribe después de las reglas de etiquetas, el nombre de M1 toma ese color también, y arrastrar el nombre no cambia su color. Los textos adjuntados a `?` en un símbolo son también etiquetas, y SELECT-LABEL los selecciona; los textos fijos adjuntados a puntos son gráficos del símbolo y toman el estilo del objeto. Los textos adjuntados a redes o puntos no reciben ningún estilo de aquello a lo que están adjuntados y usan únicamente el suyo propio. Un punto de unión es un uso de `$JUNCTION` (13.2); el punto es su símbolo y su color sigue el estilo del uso.

### 11.4 Varios dibujos de un mismo objeto

No hay forma de dibujar varias veces un mismo objeto del anfitrión: un manejador del anfitrión como `#M1` significa siempre el único dibujo de ese objeto. Si una versión posterior añadiera varios dibujos de un mismo objeto, serían objetos separados con nombres propios, y los manejadores del anfitrión conservarían su significado (Anexo C).

Nota: varias instancias dibujadas como una sola forman una agrupación (11.8).

### 11.5 Grupos: GROUP

```rnl
%stage1_set = {#M1 #M2 #M5} @GROUP;
%stage2_set = {#M3 #M4} @GROUP(spacing_x=-1);
{%stage1_set %stage2_set} @GROUP;
%stage1_set %stage2_set @DISTANCE("left-to-right", min=1)
```

- GROUP añade un hecho a un conjunto: el solucionador maneja sus miembros como un bloque. Conserva el manejador del conjunto, de modo que `%stage1_set` sigue siendo ese conjunto.
- **Un grupo no es un objeto.** No tiene posición ni marco, y no puede escribirse una restricción sobre un grupo: pasado a una restricción, el conjunto agrupado es un conjunto, y una ranura que lo recibe representa a cada uno de sus miembros (14.1). GROUP tampoco es una quinta restricción atómica. Las estructuras del paquete estándar que son grupos tienen grupos anónimos.
- **Miembros.** Los miembros de un grupo son los objetos de su conjunto, siguiendo los conjuntos anidados hasta los objetos más internos. Un uso sin símbolo representa a los objetos que rellenan sus roles, y así sucesivamente hasta los objetos más bajos; un uso dibujado con un símbolo es él mismo un miembro. ROUTE-OBJECT sigue los roles de los usos del mismo modo y toma también los propios pines de un uso dibujado con un símbolo (13.7).
- **Qué hace.** Un grupo guía al solucionador únicamente de dos maneras: dispone juntos los miembros de un grupo, antes o aparte del resto, y mantiene a otros objetos fuera de la región del grupo en la medida en que los hechos lo permitan. Ambos son criterios del solucionador, clasificados después de todas las descripciones del lenguaje (8.5): un grupo no garantiza ninguna relación geométrica, y un objeto al que los hechos fuerzan a entrar en la región de un grupo se dibuja allí. Los requisitos sobre posiciones, como no intercalarse (el DISTANCE del ejemplo) o estar centrado, se escriben como restricciones atómicas.
- **El árbol de grupos pertenece al solucionador.** Los grupos se anidan por contención: los miembros de `{%stage1_set %stage2_set}` son las cinco instancias, por lo que ambas etapas están dentro de él. Dos grupos que comparten miembros sin que ninguno contenga al otro no pueden ser ambos bloques; el solucionador conserva uno de ellos según su propia regla, y una indicación nombra al otro. Cómo afectan los grupos a la búsqueda y a la puntuación más allá de esto queda a criterio de la implementación.
- **Espaciado.** `spacing_x` y `spacing_y` son números enteros de unidades de cuadrícula (11.7) que el solucionador suma a su espaciado por defecto dentro del grupo, en x y en y; cada uno puede omitirse, y un valor negativo dibuja el grupo más compacto. Entre dos objetos, el solucionador usa el espaciado del grupo más interno que contiene a ambos; entre objetos que no comparten ningún grupo, el espaciado del nivel, que `@SOLVER-CONFIG` fija con claves de los mismos nombres (14.10); entre un grupo y lo que está fuera de él, el espaciado del grupo que lo encierra o del nivel. Un solucionador puede limitar el intervalo de valores y así lo indica. El espaciado es un criterio del solucionador, no un hecho: cede ante todo hecho conservado.
- GROUP es una palabra clave, no una definición del paquete estándar: su significado lo conoce únicamente el solucionador y no puede escribirse en el cuerpo de una definición (1.5 punto 7).
- Un visor puede dibujar un grupo, por ejemplo con un recuadro; esto queda a criterio del visor, y este estándar no lo especifica.
- GROUP no es una restricción atómica: no participa en la resolución de conflictos (8.5) y no tiene prioridad, únicamente activado y desactivado. `%stage1_set @PRIORITY("disabled")` retiene el grupo del solucionador, lo que cancela la agrupación. Cuando un uso externo está desactivado, los grupos de su expansión también están desactivados, salvo que un PRIORITY escrito desde fuera mediante `::` directamente sobre un grupo lo active de nuevo (14.8). Un grupo en la expansión de una definición puede desactivarse por sí solo únicamente cuando es un objeto local nombrado, al que se hace referencia desde fuera (9.4).

### 11.6 Orden de dibujo y capas: LAYER

```rnl
@RECTANGLE(fill="fillcolor") {(0, 0) (40, 30)} => @LAYER("bottom") => %f1;    /* a frame, drawn below everything */
(2, 28) @TEXT("input stage") => @LAYER("top") => %n1
```

- Hay tres capas: `"bottom"`, `"middle"` y `"top"`. La capa inferior se dibuja primero, después la intermedia y después la superior. Dentro de una capa, los objetos se dibujan por posición: un objeto posterior se dibuja sobre uno anterior y lo cubre.
- `@LAYER(…)` fija la capa de un objeto; sin él, el objeto está en `"middle"`. Hay una clave por cada objeto, y el LAYER posterior sustituye al anterior (5.8). LAYER no cambia la posición del objeto dentro de la capa.
- **Posición** es el orden de escritura (1.5 punto 2), el mismo orden que en 8.3 y 8.5: dentro de un nivel, el orden del texto; el contenido de un paquete en la posición de su USE-PACKAGE, con los paquetes en la raíz primero (16.3); las sentencias escritas dentro con `scope=` después del propio texto del subcircuito destino. Cada clase de objeto toma la posición de:
  - cables, gráficos y textos (anotaciones y etiquetas): la sentencia que lo crea;
  - usos: el SET-SYMBOL que surte efecto o, para un símbolo dado por una regla, la regla (8.3);
  - objetos creados por reglas, por la expansión de definiciones y por generación uno por uno: la sentencia que los crea, y dentro de una expansión el orden en el cuerpo;
  - instancias: el fondo mismo de su capa, en orden del netlist entre ellas; una instancia puesta en otra capa está al fondo de esa capa. Una agrupación cuenta como una instancia, con el orden del netlist de su primer miembro.
- Las sentencias que cambian un objeto, como PLACE, ORIENT, SET-STYLE y LAYER, no mueven su posición.
- Objetos: instancias, usos con símbolo, agrupaciones, cables, gráficos y etiquetas; un conjunto miembro por miembro (5.2.1). LAYER sobre un uso sin símbolo no es un error pero no tiene efecto (como con SET-STYLE, 11.3); sobre puntos, redes, restricciones y definiciones, falla.
- Los miembros de un símbolo no tienen capas: se dibujan en orden de miembros (9.2.1), junto con su objeto.
- No hay `z=` numérico. La sustitución (por la clave de un hecho) y la superposición (por capa y posición) están separadas.
- "Capa" aquí significa el orden de dibujo; no tiene nada que ver con los niveles del anfitrión (2.6).

### 11.7 Coordenadas y unidades

- **Ejes.** x va hacia la derecha e y hacia arriba; una y mayor está más arriba. Así, `"bottom-to-top"` de ORDER y DISTANCE va hacia arriba (14.3, 14.5), y R90 es antihorario (11.2). SVG, otros formatos y algunos editores tienen y hacia abajo; la implementación invierte y al dibujar, importar y exportar.
- **Unidad.** Una unidad no tiene tamaño físico. Qué tan grande se dibuja o imprime una unidad queda a criterio del visor.
- **Decimales.** Las coordenadas y longitudes pueden escribirse con decimales (3.7); no tienen por qué ser enteros, y no hay límite en el número de cifras. Las coordenadas se dibujan tal como están escritas y no se ajustan a enteros.
- **Cuadrícula.** Los pines deberían estar en puntos enteros (9.2.4); los gráficos en símbolos no están limitados. Cuando una instancia se coloca en un punto entero con una de las ocho orientaciones, las posiciones enteras de los pines siguen siendo enteras tras la transformación, por lo que los cables terminan también en puntos enteros: los puntos enteros son la cuadrícula habitual. El lenguaje no tiene cuadrícula propia. Si el solucionador redondea las coordenadas libres a enteros es su propia regla, clasificada después de todas las descripciones del lenguaje (8.5); cuando las restricciones fuerzan un decimal, se usa el decimal. Los puntos de codo que calcula un modo de trazado se conservan tal como se calculan, sin redondearse (13.1).
- **Tolerancia.** Al comprobar si una restricción se cumple y al decidir conflictos (8.5), una diferencia dentro de la tolerancia cuenta como satisfecha. La tolerancia es una opción del solucionador que el usuario puede fijar; este estándar no fija su valor, pero es mayor que 0, para que un valor sin forma decimal finita pueda escribirse de vuelta (abajo). Un visor que no resuelve usa su propio ajuste cuando comprueba. Así, dos herramientas pueden discrepar en un caso justo al borde de la tolerancia. Un solucionador escribe de vuelta las coordenadas como decimales que al leerse de nuevo dan el mismo valor, sin mayor truncamiento; un valor que no tiene forma decimal finita, como 1/3, se escribe con suficientes cifras para que los hechos que satisface sigan cumpliéndose dentro de la tolerancia cuando se vuelve a leer el registro (capítulo 17).
- **Coordenadas de símbolo.** Las coordenadas locales en un cuerpo de DEFINE-SYMBOL tienen la unidad y los ejes de la escena; su origen es el origen del símbolo (9.2.1, 6.5). No hay escalado: los símbolos no admiten parámetro de escala, y las instancias no pueden escalarse. Un importador convierte los símbolos de otras herramientas a unidades de RNL.
- **Otras longitudes.** Todas las longitudes del lenguaje están en unidades de coordenadas y escalan con el dibujo: `min`, `max` y `exact` de DISTANCE, `radius=` de CIRCLE, `width=` y `height=` de RECTANGLE y ELLIPSE, el tamaño de texto `size` (como `font-size` en SVG), el grosor de línea `linewidth`, la longitud de flecha `arrowsize` (9.3) y los parámetros de definiciones usados como longitudes (como `gap=3`). Un grosor de línea de 0 es la excepción: un trazo mínimo (hairline), igual de fino con cualquier zoom, que es también el valor por defecto (9.3).
- **Origen y papel.** El origen de la escena no tiene ningún significado especial. Cada subcircuito es un dibujo (6.1) con sus propias coordenadas; el visor encuadra la vista según la extensión del contenido. RNL no registra ni tamaño de papel ni la cuadrícula de ajuste de un editor.

### 11.8 Agrupaciones: BUNDLE

Una instancia iterada de Virtuoso, como `I0<3:0>`, es un símbolo pero cuatro líneas en el netlist:

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

- **Un objeto nuevo.** BUNDLE toma un conjunto de instancias del anfitrión y crea una agrupación, que se dibuja una sola vez en lugar de esas instancias. Los miembros están en el orden escrito, y el primer miembro decide los pines y el símbolo.
- **Pines y símbolo.** Los nombres de terminal y el número de terminales son los del primer miembro. Las colas de apariencia tratan la agrupación como su primer miembro: las selecciones alcanzan a la agrupación mediante las condiciones de su primer miembro (10.1), y se aplica el símbolo de bloque del subcircuito. SET-SYMBOL, estilos, PLACE, ORIENT, LAYER, restricciones y textos escritos directamente sobre la agrupación pertenecen a la agrupación.
- **Terminales.** Un terminal de la agrupación, como `%x:in`, representa los terminales del mismo nombre de todos los miembros: es un pin de bus.
  - **Por nombre**: el terminal de la agrupación representa los terminales de ese nombre de los miembros, estén donde estén en el orden de cada miembro; un miembro que no tiene terminal de ese nombre no forma parte de él.
  - `::net`: cuando todos los miembros están en la misma red, esa red, como `&out` para `%x:out::net`; de lo contrario, el conjunto de sus redes en orden de miembros, como D<3>, D<2>, D<1>, D<0> para `%x:in::net`. Así, `?t::net @WIRE {…}` en una definición dibuja un bus cuando recibe un conjunto, y un cable ordinario cuando recibe una sola red (13.1).
  - Un cable hacia un terminal de la agrupación conecta ese terminal de cada miembro (10.2).
- **Atributos** (13.6): cuando un valor es igual para todos los miembros, ese valor; de lo contrario, los valores unidos con `,`. `${w}` muestra 1u cuando todos son 1u; `${name}` es `XI0<3>,XI0<2>,XI0<1>,XI0<0>`, y para mostrar `I0<3:0>`, escríbase un texto con la misma clave, como en el ejemplo. `${net}` de un terminal funciona del mismo modo, como `D<3>,D<2>,D<1>,D<0>`.
- **Selecciones.** Los miembros no pueden seleccionarse; en su lugar se selecciona la agrupación (10.1). Así, una regla como "un nombre de red para cada pin sin cable" da a una agrupación una sola etiqueta, que muestra todo el bus. Un rol de una definición que admite instancias también admite agrupaciones, y uno que admite terminales también admite los terminales de agrupaciones (9.4).
- **Los miembros no se dibujan por separado.** Una descripción sobre dibujo escrita directamente sobre un miembro o sobre un terminal de un miembro (una posición, orientación, símbolo, estilo, capa, restricción, cable o texto) falla: el miembro se dibuja únicamente a través de la agrupación. Las descripciones que no conciernen al dibujo funcionan como de costumbre, como CURRENT y los roles de usos semánticos.
- **Todos de una misma clase.** Los miembros deberían tener el mismo nombre de modelo o subcircuito y los mismos nombres de terminal, en número, nombres y orden. Cuando no es así, se emite una advertencia; siguen agrupándose, con los pines y el símbolo del primer miembro. Los miembros que difieren únicamente en sus parámetros no producen advertencia: sus atributos muestran los valores unidos (abajo).
- **Una agrupación por cada instancia.** Una instancia escrita en varios BUNDLE pertenece al escrito más tarde (1.5 punto 2); el anterior conserva sus demás miembros, y puede emitirse una indicación. Una agrupación anterior cuyos miembros hayan sido tomados todos queda vacía y no se dibuja, con una advertencia.
- BUNDLE toma objetos escritos explícitamente: si recibe el resultado de una selección, o un conjunto que depende de una (10.4), falla, puesto que una agrupación debe existir antes de asignarse los símbolos (8.2, 10.4). Por la misma razón, un BUNDLE en contenido que genera el paso 6 falla. BUNDLE con un conjunto vacío falla (5.2.1). Un miembro que no es una instancia del anfitrión falla (clase incorrecta). Un miembro que no puede encontrarse hace fallar toda la sentencia (estricto, 7.3).

<!-- translated from en/ch12.md sha256:5da84c59c21c -->

## 12 Componentes virtuales

### 12.1 Concepto

- Un componente virtual es un componente que existe únicamente en RNL, como una tierra, una alimentación, una etiqueta de puerto o una marca de no conexión. No existe eléctricamente: únicamente se dibuja, y ni crea ni altera hechos eléctricos.
- Un componente virtual es un uso que tiene un símbolo; no hay ninguna palabra clave ni clase de objeto aparte para él. `$GROUND @ATTACH &vss` crea un uso; el paquete estándar asigna a los usos de `$GROUND` un símbolo de tierra, por lo que se dibuja.
- Reglas:
  1. **Pines y roles.** Un uso no tiene terminales. Los pines de su símbolo se alcanzan por nombre, como `%g0::pin` (6.5), y `%g0::p` sigue siendo el miembro que rellena el rol p, la red vss. Un pin cuyo `terminal=` nombra un rol de la definición, emparejado por nombre o mediante `terminals=` (11.3), está vinculado a ese rol: se dibuja en lo que rellena el rol, de modo que un cable en esa red puede terminar allí y ROUTE-NET lo une (13.7), y `::net` del pin es esa red. Un pin que vincula un rol se dibuja en la red de lo que lo rellena: una red, o la red de un terminal o de un pin (5.2.4). Así, `$GROUND @ATTACH #M5:3` sitúa una tierra en la red de M5:3, en una posición propia, no en ese pin; para dibujar una tierra en el pin, úsese una definición cuyo cuerpo lo haga, como `$GROUND_STUB` (10.5). Un rol vinculado rellenado con el nodo vacío (6.3) sitúa el pin en ninguna red, lo cual es válido: entonces lo unen únicamente los cables escritos sobre el nodo vacío (13.1), y el relleno del paquete estándar no le añade nada. Un rol vinculado rellenado con otra clase, como una instancia o un punto, deja el pin sin red: un error de clase, por el cual SET-SYMBOL falla para ese uso, que no se dibuja, y el diagnóstico señala también la aplicación. Un rol declarado con la clase `"net"` convierte un terminal en su red cuando se aplica la definición (9.4), y un pin una vez asignados los símbolos (5.2.4). Un rol que ningún pin vincula admite las clases que declara su definición, como el terminal de `$NO_CONNECT` (12.6). No es un terminal eléctrico: SELECT-TERMINAL no lo selecciona y ASSERT-TERMINAL no lo comprueba; SELECT-PIN sí (10.1). Un pin que no vincula ningún rol solo se dibuja; se cablea a mano mediante su nombre. Un nombre de pin no puede coincidir con un nombre de rol (6.5).
  2. Un uso se dibuja únicamente cuando se le ha asignado un símbolo; un uso sin símbolo no se dibuja. El paquete estándar asigna símbolos únicamente a las definiciones destinadas a dibujarse, de modo que los usos semánticos como los pares diferenciales no se dibujan.
  3. Un uso sin símbolo no dibuja nada por sí mismo: ORIENT sobre él falla, y también un cable hacia un nombre de pin suyo, puesto que no tiene pines (9.2.4); esto se comprueba tras el paso 7 de 8.2. Sigue teniendo su propio punto, `%u::position`: PLACE toma ese punto, al igual que las restricciones escritas sobre `%u::position`, mientras que una restricción sobre el uso mismo toma sus miembros (14.1); el punto se muestra en el dibujo únicamente a través de lo que el cuerpo de su definición liga a él (14.1, 9.4). Un cable o un gráfico puede terminar en ese punto únicamente a través de `::position`, como `&n {%u::position %pt} @WIRE`: es un punto, no un pin, por lo que el cable no conecta nada (5.2.4, 6.5). Un estilo asignado a él no es un error: el estilo se transmite a los objetos dibujables creados en su cuerpo (8.3), y no tiene efecto visible cuando no se dibuja nada en el cuerpo (11.3).
  4. Un uso dibujado dibuja únicamente su propio símbolo. Ni contiene ni mueve a sus miembros, y no es un contenedor visual.
  5. **Dónde se dibuja.** El origen de su símbolo es el propio punto del uso (14.1): PLACE y las restricciones lo mueven, ORIENT gira el símbolo alrededor de él (11.2), y no sigue a los objetos que rellenan los roles, salvo a través de hechos, como cables hacia sus pines y los hechos del cuerpo de su definición. Una definición que dibuja una marca sobre un pin o en un punto liga el punto del uso a él en su cuerpo, como `{? ?t} @ALIGN("x"); {? ?t} @ALIGN("y")` (9.4). Sin ORIENT, el solucionador elige su orientación y la escribe de vuelta (14.8), y un renderizador que no tiene solución para él dibuja `"R0"` (8.5); no sigue la dirección en que salen los cables de un pin. El objeto que rellena el primer rol es el objeto principal del uso; un uso no tiene nombre propio, y los textos hacen referencia al objeto principal a través del rol, como `${p}` (13.6).
  6. El cuerpo de una definición se expande siempre, por lo que la definición de un componente virtual puede tener un cuerpo.
- En términos de Principios 2.2, un componente virtual es un uso con un símbolo cuyos roles se rellenan con objetos del anfitrión; un componente virtual estándar es una definición del paquete estándar que viene con una regla que le asigna un símbolo por defecto; un componente virtual definido por el usuario es una definición propia del usuario, junto con una regla de símbolo o un símbolo asignado en su cuerpo.
- **Cómo definir un componente virtual.** La forma estándar es una definición general con, escrita justo después, la regla que asigna a sus usos su símbolo: `$GROUND = @DEFINE(roles=[["p", "net"]]) {}; # @SELECT-USAGE(recursive=true) $GROUND => @SET-SYMBOL $GROUND_SYMBOL`. La regla ocupa su lugar en la cola de símbolos por posición (8.3), de modo que una regla posterior, de un paquete de estilo de la casa o del archivo, dibuja los mismos usos de otra manera, como `# @SELECT-USAGE(recursive=true) $GROUND => @SET-SYMBOL $EARTH`, incluidos los creados dentro de otras definiciones o por el relleno; un uso individual se dibuja de otra manera nombrándolo, `%g1 @SET-SYMBOL $EARTH`. El cuerpo también puede asignar el símbolo, `? @SET-SYMBOL $X`, donde el `?` aislado es el uso (9.4): eso nombra directamente el uso (8.3), de modo que ninguna regla lo reemplaza, lo que conviene a un componente no destinado a dibujarse de otra manera. Cambiar el dibujo no cambia los terminales ni los cables sobre ellos.
- Que un uso sea un componente virtual depende únicamente de si tiene símbolo.
- Nota: una definición que debería dibujarse pero a la que no se asigna ningún símbolo se señala únicamente cuando uno de sus usos se orienta o se cablea, o cuando una restricción sobre él no tiene efecto visible (14.1).

### 12.2 Creación de componentes virtuales

```rnl id=g0
$GROUND @ATTACH &vss => @PLACE(24, 38) => %g0;
&vss @WIRE {#M5:3 %g0::pin}
```

- En `$D @ATTACH &net`, los objetos posteriores a la definición se emparejan con la lista de roles de la definición. Se escribe con ATTACH y se lee como "añadir esto sobre la red" (9.4). Un componente virtual con varios roles se escribe como una línea de instancia del netlist: para una definición `$DIFF_PORT` con los roles a y b, `$DIFF_PORT @ATTACH &inp(a) &inn(b)`, o por orden, `$DIFF_PORT @ATTACH &inp &inn`.
- Los roles suelen rellenarse con redes: los pines de un componente virtual que vinculan sus roles se dibujan en esas redes (12.1 punto 1), y su relación con un pin de un dispositivo (dónde se dibuja, a qué se conecta) viene dada por cables y restricciones (12.3). Un pin que vincula un rol se dibuja en la red de lo que lo rellena (12.1 punto 1): `$GROUND @ATTACH #M5:3` adjunta una tierra a la red de M5:3, igual que `$GROUND @ATTACH #M5:3::net`, en una posición propia. Las clases de objeto que admite un rol pueden declararse en la definición (9.4), como `roles=[["p", "net"]]`.
- Esta asociación es únicamente descriptiva: no establece ninguna conexión eléctrica y no se comprueba (Principios 2.2).
- Una red puede tener cualquier número de componentes virtuales, como cinco tierras y dos sondas. Los usos no tienen clave: cada aplicación crea un uso más (5.8).
- A un componente virtual individual se le puede asignar otro símbolo, como `%g1 @SET-SYMBOL $EARTH` con un símbolo `$EARTH` propio.
- ATTACH no tiene parámetros propios aparte de `priority` (14.8). La dirección de un puerto pertenece al contenido del paquete estándar (12.4); el nombre mostrado es el del objeto principal, y para mostrar otro texto escríbase una etiqueta (13.3). Los parámetros de la definición se pasan mediante ATTACH como de costumbre (9.4).
- `%g0` se vincula una sola vez en un nivel (6.6); para cambiar su posición, escríbase `%g0 @PLACE(…)`.

### 12.3 Posiciones relativas a un terminal

Los parámetros no admiten referencias al anfitrión, por lo que una posición relativa a un terminal se escribe como restricciones:

```rnl use=g0
{%g0::pin #M5:3} @ALIGN("x");
%g0::pin #M5:3 @DISTANCE("bottom-to-top", exact=2)    /* the pin of the ground 2 below the source of M5 */
```

Con `@ALIGN("y")` en lugar del DISTANCE, el pin de la tierra queda sobre el pin de M5. Un cable de longitud 0 entre ambos pines, `&vss @WIRE {#M5:3 %g0::pin}` (13.1), convierte también el pin en cableado, de modo que el relleno no añade una segunda tierra allí (10.2, 10.5).

### 12.4 Puertos

```rnl
$IO_PORT @ATTACH &inp => @PLACE(0, 20) => %p_inp
```

- El nombre de un puerto de un subcircuito es el nombre de la red en su interior, por lo que una etiqueta de puerto se asocia a la red `&inp`.
- Un puerto mismo (`#:inp`) no se dibuja; se muestra únicamente mediante un componente virtual. El relleno de la serie estándar 0.4 da a cada red de puerto una etiqueta de puerto, también a las redes de alimentación y de tierra, mediante el conjunto `$PORT_FILL_SET`, omitiendo las redes que tienen una etiqueta de puerto escrita (10.5); las etiquetas de un nivel están en una columna, cada una en R0 con su pin a la derecha, y la columna se mantiene al menos 6 unidades a la izquierda de cada instancia del nivel, medida desde su propio punto, mediante restricciones de prioridad `"low"`; el solucionador decide el resto. Un archivo saca una red del conjunto para dejarla sin etiqueta, como `# @SELECT-NET(name="vdd") => @REMOVE-FROM $PORT_FILL_SET`; escribe `$IO_PORT @ATTACH` para una etiqueta que coloca él mismo; o escribe `$PORT_LIST @ATTACH` una vez en un nivel para la misma columna, escrita explícitamente, y el relleno entonces no añade ninguna en ese nivel. El punto propio de un `$PORT_LIST` está en su columna (14.1), de modo que una restricción sobre el uso mueve la columna.
- La dirección de un puerto es únicamente una cuestión de visualización: el netlist no tiene direcciones, y el usuario decide qué símbolo se dibuja. Qué definiciones y símbolos de puerto existen pertenece al contenido del paquete estándar.
- **Orientación de las etiquetas de puerto.** Una etiqueta de puerto tiene una dirección: el pin de `$IO_PORT_SYMBOL` en el paquete estándar, por ejemplo, está en un extremo, y la etiqueta se extiende hacia el otro lado. Sin ORIENT, el solucionador elige la orientación a partir de las direcciones de los cables, procurando obtener pocos codos. Esto es un objetivo del solucionador, no parte del lenguaje (14.8), por lo que el ORIENT de una etiqueta de puerto no suele escribirse a mano.

### 12.5 Componentes virtuales estándar

Los componentes virtuales estándar pertenecen al paquete estándar, que no forma parte del estándar del lenguaje (16.3). El paquete estándar proporciona, por ejemplo, una tierra `$GROUND`, una alimentación `$SUPPLY`, una etiqueta de puerto `$IO_PORT` y una marca de no conexión `$NO_CONNECT` (12.6), con sus símbolos, las reglas que asignan esos símbolos y el relleno (10.5).

Para una red vss, tres formas de escribir dan tres dibujos distintos:

1. `&vss @ADD-TO $GROUND_NET_SET`: cada terminal en vss sin cable recibe una tierra debajo con un cable corto (el relleno del paquete estándar, 10.5). Así es como suelen dibujarlo los libros de texto: un símbolo de tierra debajo de cada terminal conectado a tierra.
2. `$GROUND @ATTACH &vss => %g`: una sola tierra para toda la red, colocada por el solucionador. Los terminales se conectan a su pin mediante cables, como `&vss @WIRE {#M5:3 %g::pin}`, o mediante ROUTE-NET para toda la red (13.7).
3. `$GROUND_STUB @ATTACH #M5:3`: una tierra con un cable corto debajo de la fuente de M5 únicamente, tal como la dibuja el relleno para cada pin (10.5).

- Una alimentación se escribe del mismo modo, con el conjunto `$SUPPLY_NET_SET` y la definición `$SUPPLY`. La red 0 está en `$GROUND_NET_SET` por defecto (10.1).
- Para la tercera forma, el cable en el cuerpo del tramo corto (stub) convierte al pin en cableado, de modo que el relleno no añade una segunda tierra allí (10.2, 10.5).
- Puertos: el relleno da a cada red de puerto de un subcircuito una etiqueta de puerto (12.4). Para colocar una, restringirla o cambiar el nombre que muestra, escríbase explícitamente: `$IO_PORT @ATTACH &out => %pout`, con cables hacia `%pout::pin`, y `%pout @TEXT("V_{OUT}", key="name")` para el nombre (13.3). Una red que no debe tener etiqueta de puerto se saca de `$PORT_FILL_SET`.

### 12.6 Marcas de no conexión

```rnl
$NO_CONNECT @ATTACH #M1:4    /* terminal 4 of M1 is left unconnected on purpose */
```

- Una marca de no conexión es la definición `$NO_CONNECT` del paquete estándar, cuyo rol admite un terminal o un pin; no hay ninguna palabra clave para ella. Un terminal se toma tal como es, sin convertirse (5.2.4), de modo que una marca sobre un miembro de un pin de bus (9.2.4), como `$NO_CONNECT @ATTACH #U1:d<3>`, indica qué terminal se deja abierto; varias marcas sobre un mismo pin de bus se dibujan en ese pin, una encima de otra. Ningún pin del símbolo de la marca vincula el rol (12.1 punto 1); la definición liga el punto del uso al pin de lo que rellena el rol (12.1 punto 5), y el paquete estándar le asigna un símbolo de cruz con su origen en el centro de la cruz y sin pin.
- El uso registra que el terminal se deja sin conectar a propósito: `# @SELECT-USAGE $NO_CONNECT => @SELECT-TERMINAL` selecciona estos terminales.
- No se exigen comprobaciones (otros terminales del anfitrión en la red, un cable que toque el terminal) (Principios 2.4); una implementación que conozca el `$NO_CONNECT` estándar puede emitir una advertencia.
- Una marca de no conexión no cuenta como cable (10.2): un pin que tiene únicamente la marca sigue teniendo `wired=false`. Para que un relleno por pin omita estos pines, réstense sus pines, `# @SELECT-USAGE $NO_CONNECT => @SELECT-TERMINAL => @SELECT-PIN`, como en la diferencia de 10.5.
- Nota: una marca de no conexión no es una palabra clave porque no tiene utilidad para una implementación más allá del dibujo: el solucionador no la lee, y cómo se dibuja puede escribirse en una definición y un símbolo (1.5 punto 7).
- Las etiquetas de red no son componentes virtuales; véase 13.3.

<!-- translated from en/ch13.md sha256:492780434318 -->

## 13 Cables y texto

### 13.1 Cables: WIRE

```rnl id=w13
&x @WIRE(mode="straight") {#M1:1 (10, 37) #M3:1} => @SET-STYLE $signal => %w13;
&out @WIRE(mode="manhattan") {#M2:1 #M4:1};
&tail @WIRE {#M1:3 #M5:1}                    /* no mode: "auto", routed freely */
```

- WIRE toma una red (o un conjunto de redes, lo que crea un bus; véase abajo) y una lista ordenada de puntos, al menos dos. Un punto es un objeto punto o un pin, incluido un pin de un componente virtual; un terminal representa a su pin (5.2.4). Los puntos pueden repetirse; cuando el primer y el último punto son el mismo, el cable forma un bucle.
- WIRE no admite el resultado de una selección, ni un conjunto que dependa de una (10.4): si recibe uno, falla. El solucionador escribe de vuelta un cable como ese cable, con sus codos (14.8), y un cable creado a partir de una selección no tendría nada como qué escribirse de vuelta; para dibujar como cables los pines de una selección, úsese ROUTE-OBJECT (13.7).
- **Modo de trazado**: el cable pasa por cada punto de la lista; cómo discurre entre dos puntos vecinos lo fija el parámetro `mode=`. Para puntos vecinos A(x1, y1) y B(x2, y2), con dx = x2 − x1 y dy = y2 − y1:

  | `mode` | De A a B |
  |---|---|
  | `"auto"` (por defecto) | Decidido por el solucionador, que puede optimizarlo para la situación y escribe de vuelta el trazado (véase abajo, 14.8); antes de resolver el cable, se dibuja según el trazado por defecto siguiente, que es también el que le asigna el solucionador cuando nada se interpone |
  | `"straight"` | Un segmento recto |
  | `"horizontal-vertical"` | A → (x2, y1) → B |
  | `"vertical-horizontal"` | A → (x1, y2) → B |
  | `"manhattan"` | Si \|dx\| ≥ \|dy\|: la mitad de dx en horizontal, todo dy hacia arriba o hacia abajo, luego la otra mitad en horizontal: A → (x1 + dx/2, y1) → (x1 + dx/2, y2) → B; de lo contrario, lo mismo intercambiando x e y |
  | `"manhattan45"` | Si \|dx\| ≥ \|dy\|: (\|dx\| − \|dy\|)/2 en horizontal, luego una diagonal de 45° (\|dy\| en cada eje), luego el resto en horizontal; de lo contrario, lo mismo intercambiando x e y |

  - Todo modo excepto `"auto"` es una regla estricta, y el visor lo dibuja tal como se define. Cuando dx o dy es 0, todo modo da un segmento recto. Cuando |dx| = |dy|, `"manhattan"` toma el caso |dx| ≥ |dy|, y `"manhattan45"` es un segmento diagonal. Los codos que calcula un modo conservan sus valores exactos y no se redondean; pueden caer en medias unidades (11.7).
  - **El trazado por defecto de `"auto"`**: cada segmento entre dos puntos vecinos discurre horizontal y verticalmente. Un extremo que es un pin con dirección (9.2.4), tomada tras la orientación de su objeto, es dirigido: el segmento sale de él en esa dirección o en ángulo recto con ella, y llega a él en sentido contrario a esa dirección o en ángulo recto con ella, nunca de otra manera; salir o llegar en ángulo recto cuenta como un codo. Un pin sin dirección y un punto son no dirigidos y no restringen nada. El segmento no pasa por ninguno de sus extremos y no se solapa consigo mismo. Entre los trazados que cumplen esto, el trazado por defecto es el que tiene menos codos; luego el más corto, donde un tramo que sale solo para volver mide 1; luego el que gira en menos de sus extremos dirigidos, conservando la dirección del primer extremo antes que la del último; luego aquel cuyo tramo intermedio queda a mitad de camino entre los extremos, como en `"manhattan"`; entre dos extremos no dirigidos, el que va primero en horizontal, una L con un codo; y un rodeo va hacia la izquierda del primer extremo dirigido (su dirección girada 90 grados en sentido antihorario). Un cable con puntos entre sus extremos se elige en bloque: sus puntos intermedios son no dirigidos, un giro en un punto intermedio cuenta como un codo y el cable no puede dar media vuelta en un punto intermedio, de modo que sigue recto a través de uno donde puede. Sus codos también conservan sus valores exactos y pueden caer en medias unidades (11.7), como cuando su tramo intermedio queda a mitad de camino entre dos extremos separados una unidad.
  - Los codos añadidos por un modo no son objetos: no se puede hacer referencia a ellos y no puede ponerse ningún punto de unión en ellos. Para ramificarse en un codo o dibujar un punto allí, escríbase el codo como un punto. Un punto libre `(_, _)` obtiene primero sus coordenadas del solucionador y después se conecta según el modo.
  - Un valor de `mode` que no está en la tabla es un valor de parámetro inválido: el cable falla.
- **Parámetros de estilo**: WIRE también admite las propiedades de estilo de línea como parámetros: `linecolor`, `linestyle`, `linewidth`, `cap`, `join` (9.3). Son el estilo directo del cable (8.3), como `@WIRE(linestyle="bold")` para un raíl de alimentación o `@WIRE(linestyle="dashed")` para una conexión de sustrato.
- **Reescritura de un cable.** Dado un cable en lugar de una red, WIRE no crea ningún cable nuevo: reescribe ese cable. La sentencia se lee como se leería un nuevo WIRE, y su lista de puntos y su modo reemplazan a los del cable; la red del cable se mantiene. Los parámetros de estilo son estilo directo del cable (8.3).
  - La reescritura tiene una clave, el cable: una posterior reemplaza la lista y el modo de una anterior (8.4); de dos segmentos de una misma tubería, el segmento posterior es el posterior (5.5). Los parámetros de estilo de cada reescritura siguen siendo estilo directo del cable y se resuelven según 8.3, propiedad por propiedad. Una reescritura no es un hecho geométrico (8.5).
  - Una reescritura que falla, como una que enumera un terminal sin pin (9.2.4), es un error y no tiene efecto, incluidos sus parámetros de estilo; el cable conserva su lista y su modo anteriores: los de una reescritura anterior o aquellos con los que se creó (5.8). Se informa de cada reescritura que falla, también cuando una posterior la reemplaza.
  - Al igual que un cable nuevo, una reescritura cuenta desde el paso 3 de 8.2: `wired=` (10.2) lee los puntos de la última reescritura, también cuando esa reescritura falla después, tras el paso 7 (7.1).
  - La ranura de red admite una red, un conjunto de redes o un cable; un conjunto que contiene un cable es de clase incorrecta.
  - El solucionador escribe de vuelta el trazado que eligió para cada cable `"auto"` de esta manera (14.8). Debería conservar, en orden, los puntos que el cable tiene en los registros redactados por el autor, tras su última reescritura, añadir únicamente los codos escritos como coordenadas entre ellos y escribir `mode="straight"`: las conexiones de un cable las escribe su autor, y el solucionador decide únicamente su geometría.
  - **División de un cable.** En una red que une un árbol de ROUTE-NET o ROUTE-OBJECT (13.7), el solucionador puede ramificarse desde el punto medio de un cable que escribió el autor. Entonces reescribe ese cable en su registro, por nombre o por número (5.9), conservando los puntos del autor en su orden e insertando entre ellos el punto de ramificación como un punto propio, y conecta allí la ramificación. Otros cables los modifica únicamente como indica el punto anterior. Esto es una reescritura, no una eliminación: la sentencia del autor permanece.
  - Un editor en el que el usuario cambia la forma de un cable escribe lo mismo en su propio registro; el solucionador conserva entonces el cable tal como se reescribió.
  - Para fijar el trazado en la propia sentencia, escríbanse los codos como puntos y úsese un modo distinto de `"auto"`; un exportador que conserva la forma exacta escribe `mode="straight"` y enumera cada codo.
- **Varios terminales en una misma red** (Principios 2.6) se conectan mediante varios cables que comparten puntos. Los cables están conectados únicamente cuando comparten el mismo objeto punto (un punto con nombre) o el mismo terminal; coordenadas iguales no los conectan (Principios 2.2). No existe sentencia que escriba un árbol, ni objeto grafo.
- **Unión en T**: el cable pasante enumera el punto de unión como uno de sus puntos intermedios y no necesita partirse en dos; la ramificación empieza en ese punto (véase el ejemplo en 13.2). No existe un "punto sobre un segmento de un cable".
- **Pines que se tocan**: cuando dos pines se dibujan en el mismo punto sin cable entre ellos, se escribe igualmente un cable para conectar ambos terminales, como para un símbolo de tierra dibujado justo sobre el pin de un condensador: `&0 @WIRE(mode="straight") {#C1:2 %g3::pin}`. Sus dos extremos coinciden y su longitud es 0, por lo que no se dibuja nada, pero declara la conexión: ambos pines quedan cableados (10.2), y el paquete estándar no añade otra tierra. Coordenadas iguales por sí solas no conectan (Principios 2.2). Un exportador que convierte un esquema en el que los pines se conectan por posición escribe tales cables; donde un cable pasa recto por un pin o un punto de unión, enumera ese punto como punto intermedio (unión en T); donde un punto de unión tiene un solo cable y recae justo sobre un pin, conecta el cable directamente a ese terminal.
- **La red únicamente se declara**, y debe escribirse. Las redes en las que están realmente los extremos no tienen por qué coincidir con ella, y esto no se comprueba (Principios 2.2); la red tampoco se deduce de los extremos, y los extremos pueden estar en redes distintas.
  - Cuando un terminal o pin de la lista no está en la red del cable (para un cable de bus: no está en su conjunto de redes), esto produce una advertencia: lo más probable es que el terminal esté mal escrito, como `#R1:1` por `#R1:2`. La red de un terminal o pin se conoce (un pin de un componente virtual está en la red que rellena el rol que vincula, 12.1); un punto, y un pin que no vincula nada, no tienen red y no se comprueban. El cable es válido y se dibuja (Principios 2.2, 7.6).
- **Un cable en ninguna red**: la ranura de red admite también el nodo vacío, como en `& @WIRE {%sub1::y %int1::a}` (6.3). El cable se dibuja y se resuelve como cualquier otro, y los pines en los que termina quedan cableados (10.2), pero no pertenece a ninguna red: dos cables así que comparten un punto se dibujan unidos y siguen sin formar una red, ningún árbol de ROUTE-NET o ROUTE-OBJECT los incorpora, y `${net}` del cable es el texto vacío. Un terminal o pin de su lista que está en una red produce la advertencia anterior, y lo mismo un pin en el nodo vacío dentro de la lista de un cable de una red: el dibujo mostraría una conexión que el circuito no tiene. El nodo vacío no puede ser un elemento del conjunto de un bus.
- **Buses**: la ranura de red admite una red, o un conjunto de redes en bloque. Dado un conjunto, el cable es un cable de bus, también cuando el conjunto tiene un solo miembro. Un netlist no tiene buses (6.11); un bus es solo una manera de dibujar varias redes como una sola línea.

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

  - El orden del conjunto es el orden de los bits, y se conservan los duplicados; los conjuntos escritos directamente dentro del conjunto se recorren hasta sus miembros más internos, en orden. Un miembro que no es una red hace fallar el cable (clase incorrecta). Un conjunto vacío falla también: un cable transporta al menos una red (5.2.1).
  - La ranura de red es estricta (7.3): cuando un bit no puede encontrarse, falla todo el cable, igual que cuando no puede encontrarse una sola red.
  - Los puntos, `mode=`, los parámetros de estilo, los puntos de unión y las uniones en T son como en los demás cables. No existe palabra clave BUS: una implementación dibuja un bus como dibuja un cable, y dibujarlo en negrita es una cuestión de estilo (1.5 punto 7). Un importador puede escribir `linestyle="bold"` directamente; el paquete estándar hace gruesos los cables de bus por defecto mediante la condición `bus=true` de SELECT-WIRE (10.1, 16.3).
  - **Tomas (taps)** no necesitan ningún mecanismo: un cable simple que comparte un punto con un bus es una toma, y el bus enumera el punto de toma como un punto intermedio (unión en T). Un tramo corto a 45° hacia el bus es únicamente la forma en que discurre el último tramo del cable simple. Un símbolo de toma es una definición aplicada en el punto de toma y pertenece al paquete estándar (16.3). El nombre del bit es una etiqueta `${net}` sobre el cable simple (13.3). Si la red del cable simple está en el bus no se comprueba (Principios 2.2); una implementación puede emitir una indicación.
  - Una toma también puede comenzar en un extremo de un bus: un bus que termina en un punto puede continuar como varios buses o cables simples que parten de ese punto, como un troncal `BOT<10:0>` que termina donde salen `BOT<10:3>` y `BOT<2:0>`, con un punto de unión allí (13.2). El troncal enumera ese punto como extremo, y cada parte toma sus propios bits. Los cables que parten de un punto por donde también pasa o termina un cable que transporta todas sus redes, aquí el troncal, son tomas de ese cable y no están unidos entre sí: una implementación que advierte cuando un cable termina en un cable de otra red, como si ambos estuvieran unidos, no advierte sobre ellos.
  - El atributo `net` de un cable de bus son los nombres de sus redes en orden, unidos con `,` y sin espacios, como `D<3>,D<2>,D<1>,D<0>`; para mostrar `D<3:0>`, escríbase ese texto. Las implementaciones no acortan la lista: acortarla depende de cómo escriba los nombres el anfitrión y no tiene un resultado único.
  - Así, `{&a &b} @WIRE {…}` es un solo cable de bus, no dos cables dibujados uno encima de otro.
  - Varias instancias del anfitrión dibujadas como una sola (una instancia iterada) se escriben como una agrupación (11.8).
- Un terminal al que conecta un cable debe tener un pin en su símbolo; de lo contrario, el cable falla (9.2.4).
- Las líneas que se tocan, se cruzan o se solapan son únicamente geometría y no establecen ninguna conexión (Principios 2.2). Los cables que se cruzan sin compartir un punto no llevan punto de unión; los pequeños arcos en los cruces y similares son cuestión de estilo y de la implementación.
- Cada WIRE al que se pasa una red es un objeto nuevo sin clave: escrito dos veces, son dos cables (5.8). Para hacer referencia a un cable desde otra sentencia, nómbreselo; el registro del solucionador hace referencia a un cable sin nombre por su número (5.9). Un extremo que es un terminal se mueve con su dispositivo; un punto escrito con coordenadas no se mueve; un punto con nombre puede restringirse.
- **Conexiones no dibujadas**: cómo se muestra una conexión que no tiene cable en RNL (trazada por el solucionador y escrita de vuelta como WIRE, una línea de conexión o una etiqueta) lo deciden el estilo de la casa del solucionador y del visor y los paquetes de estilo de la casa en uso; el lenguaje no lo especifica. Un paquete de estilo de la casa puede crear etiquetas o cables mediante reglas, y estos son RNL. Para que una red se dibuje como cables dejando el trazado al solucionador, escríbase ROUTE-NET o ROUTE-OBJECT (13.7).

### 13.2 Puntos de unión

```rnl
%j1 = (10, 30);
&x @WIRE {#M1:1 %j1 #M3:1};          /* the through wire passes %j1 */
&x @WIRE {%j1 (30, 30) #M4:2};       /* the branch starts at %j1 */
$JUNCTION @ATTACH %j1                /* draws the dot */
```

- Una ramificación son varios cables que comparten un punto (13.1).
- Los puntos de unión se escriben explícitamente: se dibuja un punto de unión únicamente donde se aplica una definición que dibuja uno, como `$JUNCTION` del paquete estándar; un renderizador no añade puntos de unión por sí solo.
- Un punto de unión es un uso, no una palabra clave. `$JUNCTION` es una definición del paquete estándar con un rol, que toma un punto o un terminal; su definición liga el punto del uso a ese punto o al pin del terminal (12.1 punto 5), igual que en `$NO_CONNECT` (12.6) y la flecha de corriente (15.2). Puede aplicarse a un terminal cuando dos cables coinciden en su pin. El paquete estándar asigna a estos usos un punto relleno como símbolo: el tamaño del punto procede del símbolo, y su color de `linecolor` del estilo del uso; no hay ningún ajuste aparte para el tamaño de los puntos de unión.
  - Aplicar `$JUNCTION` dos veces al mismo punto produce dos puntos uno encima del otro, que se ven como uno solo.
  - Todos los puntos de unión se seleccionan con `# @SELECT-USAGE $JUNCTION`.
  - El orden de dibujo sigue la posición de la regla que asigna el símbolo (11.6). Las reglas del paquete estándar van primero, por lo que los puntos de unión se dibujan antes que los cables; un punto de unión tiene el color de los cables y es más ancho que ellos, por lo que esto no produce ninguna diferencia visible y no hace falta LAYER.
- Los puntos de unión no se deducen del número de cables que coinciden en un punto compartido: los esquemas reales tienen puntos de ramificación con solo dos cables dibujados con punto de unión, y un punto deducido necesitaría otra forma más de eliminarlo. Un exportador escribe los puntos de unión que muestra el original. Una implementación puede emitir una indicación donde tres o más cables coinciden en un punto compartido sin punto de unión; este estándar no lo exige.
- **Puntos de unión añadidos por el solucionador**: la salida del solucionador es también RNL y puede contener puntos de unión. La definición que usa viene dada por la clave `junction` de SOLVER-CONFIG (14.10), un nombre `$` escrito como cadena, como `junction="$JUNCTION"`, o, en el SOLVER-CONFIG de un registro de nivel, un nombre `%` de ese nivel, como `junction="%BIG_DOT"` para un punto más grande definido únicamente para ese subcircuito; el paquete estándar fija `$JUNCTION` para los niveles que lo usan (16.3). Con la clave, el solucionador aplica esa definición a cada punto de ramificación de los árboles de ROUTE-NET y ROUTE-OBJECT (13.7), y donde los cables escritos por el usuario coinciden tres o más en un punto compartido sin un uso de esa definición, puede añadir uno también en su salida. Sin la clave, no escribe puntos de unión, y `junction=""` los desactiva donde un paquete los haya fijado. El nombre se busca como un nombre escrito en el registro del solucionador en ese nivel (6.8); cualquier otro valor que no nombre ninguna definición allí, o una cuyo primer rol no admita un punto, es un valor que no se admite en ese nivel (14.10): un error, y la clave cuenta como no escrita allí, de modo que se aplica el valor anterior o, si no lo hay, el solucionador no escribe puntos de unión. Un punto compartido es siempre un punto con nombre o un terminal (coordenadas iguales no cuentan), por lo que el punto de unión puede escribirse. Otras herramientas que añaden puntos de unión, como un editor, toman la definición de la misma clave.
- Un punto de unión es únicamente dibujo: no establece ninguna conexión y no se comprueba (Principios 2.2). Un punto con `$JUNCTION` y sin cable recibe igualmente su punto de unión.

### 13.3 Texto: TEXT

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

- **Texto vacío**: un TEXT cuyo contenido es la cadena vacía no dibuja nada. Con una clave, reemplaza la etiqueta del símbolo con esa clave y así la oculta. El paquete estándar `rnl-std-no-labels` oculta las etiquetas por defecto con el estilo `textvisible=false` en su lugar (9.3, 16.3), lo que conserva su contenido, de modo que un TEXT con la misma clave y `textvisible=true` vuelve a mostrarla.
- **Adjuntar**: TEXT toma un punto, o un objeto que se dibuja, y el texto se adjunta a él. Aquello a lo que se adjunta decide tres cosas: qué atributos lee `${…}` (13.6); dónde va el texto cuando no se indica su posición; y, cuando tiene una clave, de qué objeto es subobjeto el texto.
  - Un punto (`(x, y)`, un punto con nombre, un `::position`): el texto es una anotación (13.4). Puede hacer referencia únicamente a los atributos `x` e `y` del punto.
  - Un objeto: el texto es una etiqueta. El objeto puede ser una instancia, un uso dibujado, una agrupación, un pin, un terminal (en su pin, 5.2.4), un cable o un gráfico. También puede ser una red: una red en sí no se dibuja, y la implementación coloca un texto adjuntado a una red junto a uno de sus cables o terminales. Así es como un archivo indica qué redes muestran sus nombres.
  - Un uso sin símbolo, o un terminal sin pin, carece de significado para el dibujo: si recibe uno, el TEXT falla (12.1, 9.2.4).
  - Dado un conjunto, TEXT adjunta un texto a cada miembro (5.2.1), como en la última línea del ejemplo.
  - En un cuerpo de símbolo, un texto adjuntado a un punto es texto fijo del símbolo; un texto adjuntado a un `?` aislado se adjunta al objeto dibujado con el símbolo (véase abajo).
- **Contenido**: `text` es obligatorio, también cuando se da una clave; sin él, el TEXT falla. Puede darse por posición, primero, como en `@TEXT("${name}", key="name")`. No hay contenido por defecto: para mostrar el nombre, escríbase `"${name}"`. El marcado y las referencias a atributos están en 13.6.
- **Posición**: un texto tiene una posición propia, que no es el punto ni el objeto al que está adjuntado.
  - La posición de un texto comienza libre, como `(_, _)`. Cuando ningún PLACE, `dx`, `dy` ni restricción la fija, el solucionador la decide y escribe de vuelta la posición (14.8); sin solución, el visor decide: un texto adjuntado a un punto suele ir en ese punto, y un texto adjuntado a un objeto, junto al objeto; cualquiera de los dos puede desplazarse para evitar solapamientos.
  - PLACE fija la posición del propio texto. Así, `(1, 1) @TEXT(…) => @PLACE(2, 2)` no es una contradicción: la primera parte adjunta el texto a (1, 1), la segunda sitúa el texto en (2, 2). Para poner una anotación exactamente en algún lugar, escríbase PLACE.
  - `dx`, `dy`: desplazamientos, admitidos únicamente por TEXT. Cuando aquello a lo que se adjunta el texto tiene un punto de coordenadas (un punto, el origen de una instancia, el punto donde se dibuja un uso, el pin de un terminal), el texto va en ese punto más (dx, dy). Los cables y las redes no tienen punto de coordenadas: un desplazamiento en un texto adjuntado a uno hace fallar el TEXT.
    - El texto sigue teniendo una sola posición, la que tiene el desplazamiento; las restricciones y el solucionador la utilizan, y no existe posición "antes del desplazamiento". Cuando el objeto adjuntado se mueve o lo coloca el solucionador, el texto se mueve con él; cuando otras restricciones mueven el texto, el objeto adjuntado se mueve con él. Un desplazamiento equivale a un DISTANCE exacto para x y otro para y, escritos sobre el texto: un desplazamiento positivo en la dirección creciente (`"left-to-right"`, `"bottom-to-top"`), uno negativo en la dirección decreciente, con su magnitud como `exact` (14.5).
    - Los desplazamientos siguen los ejes de la escena y no giran con la orientación del objeto adjuntado, puesto que los textos en la escena no giran con los objetos. En un cuerpo de símbolo también pueden escribirse; allí están en coordenadas locales y se transforman con la instancia, igual que una posición local escrita con `=> @PLACE(…)`.
    - **Los desplazamientos y PLACE se reemplazan entre sí**: en cada eje, la posición de un texto es absoluta (la x o y de PLACE) o un desplazamiento (`dx` o `dy`), nunca ambos. En el mismo eje del mismo texto, la forma escrita más tarde (según la posición de 8.5) surte efecto, y la escrita antes no tiene efecto alguno: tras un desplazamiento, la posición absoluta en ese eje queda libre y sigue al desplazamiento; tras un PLACE, el desplazamiento en ese eje deja de aplicarse. Esto es un reemplazo, no un conflicto: la prioridad no interviene y no se emite indicación. Por ejemplo, tras `dx` y `dy`, un `#M1::name @PLACE(x=13)` posterior hace que x sea el valor absoluto 13, `dx` deja de aplicarse e y sigue siguiendo a `dy`; un editor escribe esto cuando se arrastra un texto colocado por desplazamiento. Una posición dada en el símbolo va primero (véase abajo), y un PLACE o un desplazamiento en la escena la reemplaza.
    - Sea cual sea la forma que surta efecto, es un hecho geométrico ordinario: la x y la y de un desplazamiento son una unidad cada una, su posición es la de la sentencia TEXT que las escribe y su prioridad es el valor por defecto del encabezado del registro (14.8); se resuelven con los demás hechos según 8.5. Varios PLACE en el mismo eje se resuelven entre sí según 8.5. Para un texto con clave, `dx` y `dy` se sustituyen por separado (8.4).
  - Un texto puede ser miembro de restricciones por su propia posición (14.1). El `::position` de un texto es su propia posición: `%n1::position @PLACE(…)` es lo mismo que `%n1 @PLACE(…)`.
  - Los textos en la escena no giran con la orientación de aquello a lo que están adjuntados; para girar uno, escríbase `rotate=`. Cuando la escena proporciona ambos ejes de la posición de un texto, mediante PLACE o desplazamientos, ninguno de los ejes usa la posición local del símbolo: el texto no hereda la orientación del objeto adjuntado, y `halign` y `valign` se interpretan en las direcciones de la escena.
  - El solucionador escribe de vuelta la posición que eligió (14.8): por la clave (abajo) o el nombre del texto, o de lo contrario por su número (5.9). A un texto sin clave que crea directamente una regla de generación (13.3, 10.5) no se le puede hacer referencia: el solucionador deja su posición al visor. A un texto en el cuerpo de un uso que crea una regla se le hace referencia por número una vez que el solucionador escribe el uso como sentencia explícita (10.5, 5.9).
- **Clave `key=`**:
  - Sin clave, el texto es un texto sin clave del objeto adjuntado; escrito de nuevo, es otro texto, igual que ocurre con los cables (13.1). Puede nombrarse con `%x =` y referenciarse y cambiarse en su nivel. Los textos sin clave creados por reglas no tienen manejador, por lo que el solucionador no puede escribir de vuelta sus posiciones.
  - Con clave, el texto es el subobjeto `object::key` del objeto adjuntado, como `#M1::wl`. El mismo objeto con la misma clave es el mismo texto, y las sentencias posteriores lo sustituyen parámetro por parámetro: el tamaño, la alineación, la rotación, etc. se sustituyen cada uno por separado, y los parámetros no escritos permanecen como estaban (8.4). El contenido se escribe en toda sentencia (`text` es obligatorio), y el contenido posterior reemplaza siempre al anterior; para cambiar únicamente la alineación o la rotación, escríbase de nuevo el contenido, como en `#M1 @TEXT("${name}", key="name", halign="end")`. La posición sigue las reglas de PLACE (8.5). Otros registros, los editores y el solucionador encuentran el texto mediante este manejador.
  - Un texto adjuntado a un punto no admite `key=`, puesto que un punto no tiene subobjetos: con una clave, el TEXT falla.
  - Los nombres tras `::` se buscan en un orden fijo, y las claves van al final (6.5): los nombres de rol y nombres locales de un uso, luego sus alias de `definition=`, luego los nombres estándar de subobjeto y sus alias, luego los nombres de los pines del símbolo del objeto, luego las claves de los textos. Así pues, una clave no debe escribirse como ningún nombre que la preceda: un nombre estándar de subobjeto (`position`, `net`) o un alias de uno declarado para el archivo (como `位置` de `alias-zh`); un nombre de rol o nombre local del uso al que se adjunta el texto; o un alias de `definition=` de uno de estos. Tal clave es un error, como las colisiones de nombres de 6.5. Para un texto escrito en un símbolo, los alias son los del archivo en el que está escrito el símbolo. `name` y `value` no son nombres estándar de subobjeto, sino claves habituales.
- **Textos en símbolos**:
  - En un cuerpo de símbolo, un `?` aislado es el objeto dibujado con el símbolo: M1 cuando se dibuja M1, `%g0` cuando se dibuja el uso `%g0`. Únicamente puede ser el objeto de TEXT; en cualquier otro lugar falla, como `? @PLACE(…)` en un símbolo, que carece de significado.
  - `? @TEXT("${name}", key="name") => @PLACE(1.5, 1.5)`: el texto se adjunta al objeto dibujado con el símbolo, y `${…}` lee sus atributos. Este PLACE está en coordenadas locales y se transforma con la instancia (véase la dirección de los textos en 9.2.5). Sin PLACE, la implementación sitúa el texto junto al objeto.
  - Un texto con clave es un subobjeto del objeto dibujado con el símbolo, como `#M1::name`. Los textos del símbolo van primero, y un texto con la misma clave en la escena lo sustituye parámetro por parámetro: `#R1 @TEXT("R_{D}", key="name")` cambia únicamente el contenido y conserva la posición del símbolo; `#M1::name @PLACE(13, 18)` cambia únicamente la posición y conserva el contenido del símbolo. Una posición dada en la escena está en coordenadas de escena y ya no gira con la instancia.
  - Las claves que aporta un símbolo existen únicamente una vez asignados los símbolos (8.2 pasos 4 y 7). Cuando el símbolo de M1 no tiene ningún texto con clave `name` y la escena no escribe ninguno, `#M1::name` falla. Para no depender del símbolo, escríbase un TEXT con esa clave en la escena.
  - Un texto adjuntado a un punto en el símbolo es texto fijo, como `(-2, 2) @TEXT("+")` en un símbolo de amplificador operacional, y no puede cambiarse para una sola instancia.
- **Reglas**: una regla como `# @SELECT-INSTANCE => @SELECT-PIN(wired=false) => @TEXT(…)` selecciona pines, que proceden de los símbolos, por lo que se evalúa en el paso 6 con la generación uno por uno (8.2). Cuando una implementación añade líneas de conexión o etiquetas por sí misma, omite los pines que ya tienen etiqueta.
- Qué nombres de red se muestran y dónde, y si las conexiones sin cables se muestran con etiquetas o con líneas de conexión, lo decide la implementación por defecto: forma parte de su estilo de la casa (Principios 2.7). Mostrar etiquetas en lugar de cables cambia la disposición, por lo que el solucionador y el renderizador usan el mismo ajuste. RNL escribe esto únicamente cuando hace falta: para declarar una intención (sin posiciones) o para conservar posiciones ya decididas.
- No hay posiciones a lo largo de un cable: las coordenadas de un cable no están fijadas por sí mismas.
- **`fix_rotate`**: un valor de verdad, `false` por defecto. Con `true`, `rotate` del texto se toma en la escena: los caracteres conservan esa dirección, sea cual sea la orientación del objeto al que está adjuntado el texto. Su anclaje sigue al objeto, y también lo hace el lado del anclaje en el que queda el cuadro de texto. Una alineación indica dónde queda el cuadro respecto al anclaje: a la derecha con `start`, a la izquierda con `end`, arriba con `bottom` y `baseline`, abajo con `top`, y a ningún lado con `middle`. Esta dirección se transforma con la orientación del objeto, y el texto se alinea según el resultado. Así, un nombre escrito encima de un símbolo, con `halign="middle"` y `valign="bottom"`, queda a la izquierda de su anclaje, alineado `end` y `middle`, cuando el símbolo se gira `"R90"`, y debajo de él, alineado `top`, en `"MX"`; se lee horizontalmente en toda orientación y permanece fuera del símbolo. Un cuadro que permanece encima de su anclaje conserva el `bottom` o `baseline` con el que se escribió.
- **`upright`**: un valor de verdad, `true` por defecto: el texto se gira 180 grados cuando de otro modo se leería boca abajo, y sus glifos no se reflejan. Con `false`, sus caracteres siguen tal cual la rotación y la reflexión (9.2.5).
- Parámetros: `text` (obligatorio; puede darse por posición), `key`, `dx`, `dy`, `halign`, `valign`, `rotate`, `fix_rotate`, `upright` y las propiedades de estilo de texto `textcolor`, `size`, `font`, `bold`, `italic`, `textvisible` (9.3); un texto adjuntado a un punto no tiene `key`. Para `halign`, `valign` y `rotate`, véase 9.2.5. Los parámetros de estilo son el estilo directo del texto (8.3); cómo obtiene su estilo un texto está en 11.3.

### 13.4 Anotaciones: gráficos en la escena

```rnl
(50, 18) @TEXT("5T OTA testbench") => @SET-STYLE $note => %n1;
%f1 = @RECTANGLE {(0, 0) (40, 30)};
%n1 @PLACE(60, 18)                            /* moves the text; may also be written in another record */
```

- LINE, RECTANGLE, CIRCLE, ELLIPSE, ARC, BEZIER y TEXT pueden escribirse directamente en la escena, y por tanto también en el cuerpo de una definición general. En un cuerpo de símbolo sus coordenadas son locales; en la escena son coordenadas de escena. Dónde se escriben decide esto; la palabra clave es la misma.
- Los objetos de nota de Principios 2.2 (texto de forma libre, gráficos, recuadros y otros objetos que únicamente se dibujan, sin terminales ni significado eléctrico) son las anotaciones de este estándar: gráficos escritos en la escena, TEXT adjuntados a puntos y los símbolos dibujados por ANNOTATION (13.5).
- PIN puede escribirse únicamente en el cuerpo de DEFINE-SYMBOL: un pin vincula un terminal del objeto dibujado con el símbolo, y la escena no tiene tal objeto.
- Un TEXT adjuntado a un punto es una anotación (13.3): puede hacer referencia únicamente a los atributos `x` e `y` del punto, y no admite `key=`, puesto que un punto no tiene subobjetos (con una clave, el TEXT falla). Para mostrar el nombre o el valor de un objeto, adjúntese el TEXT al objeto, lo que lo convierte en una etiqueta.
- La posición de un gráfico viene dada por sus puntos. Un punto puede ser cualquier objeto punto: un punto con nombre, un punto libre con restricciones o `#M1::position`, que sigue al dispositivo. En el cuerpo de una definición general, un punto literal es un desplazamiento respecto al propio punto del uso (9.4). PLACE no admite un gráfico en sí: para mover, desde otro registro, un RECTANGLE, CIRCLE o ELLIPSE escrito con un centro, muévase su `::position`; LINE, ARC, BEZIER y los gráficos escritos con dos puntos tienen varios puntos, por lo que hay que nombrar los puntos y moverlos. TEXT es la excepción: tiene una posición propia, el punto al que se adjunta le da únicamente su lugar por defecto, y PLACE fija directamente la posición del texto (13.3).
- Apariencia: los gráficos pueden tomar parámetros de estilo directamente (9.2.5) y también admiten SET-STYLE, con los dos rangos de 8.3; una propiedad que ninguno de los dos proporciona toma el valor por defecto del renderizador.
- Un recuadro es un RECTANGLE, o un LINE cuyo último punto es el primero. No existe ningún gráfico que encierre un grupo de objetos: dibújese un RECTANGLE y restrínjanse sus esquinas para que queden fuera del grupo, como `%tl %stage1_set @DISTANCE("left-to-right", min=1)`, que mantiene la esquina al menos 1 unidad a la izquierda de cada miembro (14.5); lo encerrado son los puntos de los miembros más el margen. Un visor puede dibujar un GROUP como un recuadro; esto queda a criterio del visor (11.5). No existe `::box`.
- El orden de dibujo sigue las capas y posiciones (11.6).
- **Clase**: que algo sea una anotación se sigue de su clase de objeto: los gráficos en la escena son anotaciones, por lo que un renderizador puede mostrarlos u ocultarlos como una clase. SELECT-ANNOTATION selecciona todos los gráficos en la escena y lo que dibuja ANNOTATION (13.5), incluidos los expandidos desde los cuerpos de definiciones generales (como los recuadros dibujados por un paquete de estilo de la casa); los gráficos en símbolos, etiquetas, cables y usos (incluidos los puntos de unión) no son anotaciones. Clases más finas (explicaciones, cajetines (title block), adornos dibujados por un paquete) son conjuntos nombrados, y un gráfico puede estar en varios conjuntos. Mostrar y ocultar queda a criterio del visor; el lenguaje no tiene `visible`.
- Un texto sobre un dispositivo es un TEXT adjuntado al dispositivo, lo cual es una etiqueta (13.3). Un TEXT adjuntado a `#M1::position` es una anotación; por defecto sigue también al dispositivo, pero puede hacer referencia únicamente a las coordenadas de ese punto y no tiene clave.

### 13.5 Símbolos dibujados como anotaciones: ANNOTATION

```rnl
%a1 = (40, 20) @ANNOTATION(rotate=30) $ARROW_SYMBOL;     /* an arrow at any angle */
#M1 @ANNOTATION(rotate=-45, mirror=true) $ARROW_SYMBOL;   /* follows M1 */
%a1::position @PLACE(44, 20)                             /* moves the first one */
```

- ANNOTATION reúne los gráficos de lo que recibe y los dibuja como una sola anotación (13.4), girada cualquier ángulo. Dos ranuras: `at`, escrita primero, un objeto ubicable: un punto, o un objeto cuya posición sigue la anotación; y `source`, escrita al final, uno de:
  - un símbolo (DEFINE-SYMBOL): los gráficos y los TEXT de su cuerpo;
  - una definición general (DEFINE), con roles o sin ellos: los gráficos escritos directamente en su cuerpo cuyos puntos son todos puntos literales, que son desplazamientos respecto al punto de un uso (9.4), y los TEXT adjuntados a tales puntos o a `?`. La definición no se aplica: no se crea ningún uso, de modo que nada se dibuja dos veces;
  - un uso: los gráficos de su símbolo, cuando tiene uno, una vez asignados los símbolos (8.2), y los gráficos que su cuerpo dibujó con puntos literales únicamente, como para una definición, alrededor del punto del uso. El uso en sí sigue dibujado donde está.
- Todo lo demás se omite: los PIN, `box=`, las restricciones, los usos creados en un cuerpo y los gráficos de un cuerpo que tienen un punto en otro objeto, como una línea dibujada hacia un rol, que no significaría nada una vez movida y girada. Un gráfico omitido por esa razón produce una indicación; el resto se omite sin indicación, de modo que un símbolo con pines pueda reutilizarse para un signo. Nada rellena un rol; la anotación no tiene pines, no puede ser extremo de un cable y no está en ningún árbol de 13.7.
- **Parámetros**: `rotate`, un ángulo en grados, en sentido antihorario, cualquier número, `0` por defecto; `mirror`, un valor de verdad, `false` por defecto, que refleja el símbolo respecto a su eje vertical antes de girarlo; y las propiedades de estilo, pasadas a sus gráficos y textos tal como las pasa SET-STYLE (11.3). También admite, por nombre, los parámetros de un símbolo o una definición general pasados como `source`, como hace AS (9.4), tal como `(0, 0) @ANNOTATION(rotate=30, w=5) $D`: un parámetro no pasado toma su valor por defecto, y uno sin valor por defecto cuenta como no escrito (9.4). Donde un parámetro del origen tiene el nombre de un parámetro de la propia ANNOTATION, como `rotate` o `mirror`, se entiende el parámetro de ANNOTATION y se emite una indicación. Un uso pasado como `source` conserva los valores de sus propios parámetros, y un parámetro de su definición escrito en ANNOTATION es entonces un parámetro inválido. Los textos del símbolo se giran con él como los textos de un símbolo se giran con una instancia, según sus propios `rotate`, `fix_rotate` y `upright` (9.2.5).
- En los textos del símbolo, `?` es la anotación, que tiene únicamente los atributos `x` e `y`, como un punto (13.4); otra referencia a atributo allí se dibuja como el texto vacío.
- **Resultado**: una nueva anotación, con el subobjeto `::position`, su anclaje. Dado un punto escrito, el anclaje empieza allí y puede moverse por su `::position`, como un gráfico escrito con un centro (13.4); dado un objeto, sigue la posición del objeto. SELECT-ANNOTATION la selecciona, SET-STYLE la estiliza y `textvisible` oculta sus textos.
- Qué es se conoce donde está escrito: cualquier cosa que no sea un símbolo, una definición general o un uso en la ranura `source` es un error; un origen sin ningún gráfico que dibujar produce una advertencia y no dibuja nada.
- **Comparado con el resto**: los gráficos escritos en la escena (13.4) tienen cada uno sus propios puntos; un uso con símbolo, como una figura del paquete estándar, se dibuja en las ocho orientaciones de 11.2, puede tener pines y puede cablearse. ANNOTATION dibuja un símbolo en cualquier ángulo y no tiene significado eléctrico.

### 13.6 Marcado de texto y referencias a atributos

```rnl
#R1 @TEXT("R_{D}", key="name");                              /* subscript */
(50, 18) @TEXT("\textbf{5T OTA} testbench\\VDD = 1.8 V");    /* bold, two lines */
#M1 @TEXT("W/L=${w}/${l}", key="wl");                        /* host parameters */
$NMOS_M = @DEFINE-SYMBOL {
  $NMOS;
  ? @TEXT("${model}", key="model") => @PLACE(1.5, -1.5)      /* the model name, in a symbol */
}
```

- **El marcado** es un subconjunto del modo texto de LaTeX, no TeX completo:

  | Marcado | Significado |
  |---|---|
  | `_{…}`, `^{…}` | Subíndice, superíndice |
  | `\textbf{…}`, `\textit{…}` | Negrita, cursiva |
  | `\overline{…}` | Línea superior, como el activo en bajo `\overline{RST}` |
  | `\subrest{…}` | El primer carácter tal como está escrito, el resto como subíndice: `\subrest{${name}}` muestra M1 como M₁ y VDD como V con DD debajo |
  | `\subdigits{…}` | Los dígitos del final como subíndice: R12 como R₁₂, Vout1 como Vout₁; un contenido sin dígitos al final se muestra tal cual |
  | `\(…\)` | Modo matemático (abajo) |
  | `\frac{…}{…}` | Fracción: numerador arriba, denominador abajo, una barra en medio, como `\frac{${w}}{${l}}`; un visor que no pueda apilarlos muestra "numerador/denominador" |
  | `\alpha` … `\omega`, `\Gamma` … `\Omega` | Letras griegas, con los nombres de LaTeX |
  | `\pm`, `\times`, `\cdot`, `\infty`, `\approx`, `\neq`, `\leq`, `\geq`, `\ll`, `\gg`, `\propto`, `\parallel`, `\angle`, `\circ`, `\to`, `\uparrow`, `\downarrow` | Símbolos comunes |
  | `\\` | Salto de línea |
  | `\_`, `\{`, `\}`, `\$`, `\%`, `\&`, `\#`, `\textbackslash` | Los propios caracteres `_`, `{`, `}`, `$`, `%`, `&`, `#`, `\` |

- Letras griegas: minúsculas `\alpha` a `\omega` (LaTeX no tiene `\omicron`; escríbase o), y las variantes `\varepsilon`, `\vartheta`, `\varpi`, `\varrho`, `\varsigma`, `\varphi`; mayúsculas únicamente las letras que difieren de las latinas: `\Gamma`, `\Delta`, `\Theta`, `\Lambda`, `\Xi`, `\Pi`, `\Sigma`, `\Upsilon`, `\Phi`, `\Psi`, `\Omega`. Un símbolo que no está en la tabla se escribe directamente como carácter Unicode (las cadenas admiten caracteres no ASCII, 3.7).
- El marcado puede anidarse, como `\textit{V}_{\textit{DD}}` e `I_{D_{1}}`. Cómo se componen los índices de `V_{in}^{+}` (apilados o uno tras otro) queda a criterio del visor.
- **Los subíndices y superíndices necesitan llaves**: un `_` o `^` aislado es un carácter ordinario. Esto difiere de LaTeX, donde la m de `g_m` es un subíndice: los nombres y las descripciones suelen contener `vin_1` o `bias_ctrl`, que esa regla estropearía. A cambio, `g_m` se muestra tal como está escrito.
- Los demás caracteres se muestran tal como están escritos, incluidos `{`, `}`, `$`, `~`, `%`, `&` o `#` aislados, que tienen otros significados en LaTeX.
- **Barra invertida**: `\\` es un salto de línea, como en LaTeX; una barra invertida en sí se escribe `\textbackslash`. Una barra invertida que no va seguida de una orden conocida se muestra tal como está escrita y produce una advertencia, como `C:\temp` (`\temp` es una orden desconocida; véase el punto siguiente); para evitar la advertencia, escríbase `C:\textbackslash{}temp`. `\textbackslash` es necesario únicamente donde la barra invertida va seguida directamente de `\`, `{`, `}`, `_`, `^`, `$`, `%`, `&`, `#` o el nombre de una orden conocida. Las cadenas de RNL no tienen escapes con barra invertida (3.7): `"\textbf{V}"` se escribe exactamente así, no como `\\textbf`.
- Una orden desconocida (`\` seguida de letras que no están en la tabla, como `\sqrt` o `\alpah`) y las llaves desequilibradas se muestran tal como están escritas y producen una advertencia; el texto sigue siendo válido.
- **Después de un nombre de orden**: un nombre de orden es la secuencia de letras directamente tras `\`. Los espacios tras el nombre de una orden conocida se descartan y no se muestran, y un `{}` vacío justo después de ella únicamente separa y tampoco se muestra. Así, `10 \mu A` y `10 \mu{}A` muestran ambos 10 μA; para conservar el espacio, escríbase `\mu{} A`; `\textbackslash{}temp` muestra \temp. Los espacios tras los caracteres escapados como `\_` y `\{` se conservan.

  Nota: la compatibilidad con más elementos de TeX en una versión posterior únicamente permite añadir más. El modo matemático se escribe `\(…\)`, no `$…$`, porque `$` lo usan las referencias a atributos.
- **Formatos de nombres**: `\subrest{…}` y `\subdigits{…}` cambian únicamente cómo se muestra su contenido: los caracteres mostrados se dividen en una parte normal y un subíndice. El contenido puede estar referenciado, como `\subrest{${name}}`; sigue sin leerse como marcado (abajo). Pueden anidarse en otro marcado, como `\textit{\subrest{${name}}}`.
- **Modo matemático**: `\(…\)` contiene una fórmula, en un subconjunto del modo matemático de LaTeX:
  - las letras son variables, compuestas en cursiva; los dígitos y los símbolos de la tabla van en redonda, y los espacios no se muestran;
  - `_` y `^` toman un carácter o un grupo entre llaves, como en LaTeX, de modo que `g_m` es g con un subíndice m dentro de `\(…\)`;
  - `\frac{…}{…}`, `\sqrt{…}`, las letras griegas y los símbolos comunes de la tabla, y las relaciones `=`, `<`, `>`;
  - `\mathrm{…}` compone su contenido en redonda, como para unidades y nombres, tal como `\(V_\mathrm{DD}\)`;
  - una referencia a atributo en su interior se muestra tal como está escrita, en redonda.

  Una orden que no está en este subconjunto se muestra tal como está escrita y produce una advertencia.
- **Fracciones**: `\frac` toma dos grupos entre llaves directamente después de ella; cuando falta uno, es marcado desconocido, se muestra tal como está escrito con una advertencia. El numerador y el denominador pueden contener marcado y `${…}`.
- Cuánto se elevan o bajan los índices y las fracciones y cuánto se reducen queda a criterio del visor. La fuente, y la negrita y cursiva de todo el texto, proceden del estilo o de los parámetros `font`, `bold`, `italic` (9.3).
- El marcado se lee en `text` de TEXT y en `label` de PIN (como `\overline{CS}`). Otros parámetros de cadena (`key`, valores de enumeración, condiciones de selección) no leen marcado.
- Nota: al importar y exportar, `_{…}` y `^{…}` de KiCad son iguales, y su `~{…}` es `\overline{…}`; una línea superior que Altium escribe con `\` tras cada carácter se reescribe como `\overline{…}`.
- **Referencias a atributos** `${name}` muestran un atributo del objeto al que está adjuntado el texto. Para escribir `${` en sí, escríbase `\${`; todo desde `\${` hasta su `}` correspondiente se muestra tal como está escrito, y ese `}` no cuenta como llave desequilibrada. Los atributos son:
  - Instancias: `name`; `value`, el valor escrito en la línea del anfitrión, tal como está escrito, vacío si no hay ninguno (no recurre al nombre del modelo); `model` y `type`, como en las condiciones de selección (10.1), de modo que `${model}` de una instancia X muestra el nombre del subcircuito; y los parámetros del anfitrión, como `w`. Un parámetro del anfitrión con el nombre de un atributo estándar se escribe `${#…}` (véase abajo).
  - Terminales, pines y cables: `net`, la red en que están, mostrada por su nombre; para un pin, la red de `::net` (6.5), y un pin que no vincula nada no tiene `net`. En el nodo vacío (6.3), `net` es el texto vacío, y también lo es un rol rellenado con él, como `${p}`: no se dibuja nada (13.3). El `net` de un cable de bus son los nombres de sus redes en orden, unidos con `,` y sin espacios (13.1). Para un terminal de una agrupación, es el nombre de la red cuando todos los miembros están en la misma red, y de lo contrario los nombres unidos con `,` (11.8).
  - Redes: `name`.
  - Usos: los roles y los parámetros de su definición. `${p}` es el objeto que rellena el rol p, mostrado según las reglas siguientes. Un uso no tiene nombre propio: los símbolos de tierras, alimentaciones y puertos escriben `${p}`, que muestra el nombre de la red cuando p es una red; un rol rellenado con un terminal o un pin, como t de `$GROUND_STUB`, muestra el nombre de su red también.
  - Agrupaciones: un atributo que es igual para todos los miembros tiene ese valor; de lo contrario, los valores se unen con `,` (11.8).
  - Puntos: `x`, `y`, las coordenadas finales, es decir, después de resolver.
  - Gráficos: ningún atributo.
- **Caminos de atributos**: en `${a::b::c}`, el primer nombre toma un atributo del objeto adjuntado, y cada `::name` siguiente toma un atributo del valor anterior, como `${p::x}` (la x del rol p cuando es un punto) y `${pair::left::w}`.
  - El valor de un atributo puede ser un objeto (un rol de un uso; `net` de un terminal o cable) o texto (`name`, `value`, un parámetro del anfitrión, etc.). El texto no tiene atributos, por lo que ningún `::` puede seguirlo.
  - Un valor en el camino también puede ser un conjunto, como un rol de conjunto. El resto del camino se toma entonces de cada uno de sus miembros, un conjunto entre ellos por sus propios miembros, en el orden del conjunto, y los valores se muestran como se muestra un conjunto (abajo); cuando un miembro carece del atributo, el texto falla. Así, para un uso cuyo rol de conjunto `pairs` contiene dos pares diferenciales, `${pairs::left::w}` muestra `w` para ambos transistores `left`, como `1u,2u`.
  - En `${…}`, `::` toma únicamente atributos, a diferencia de `::` en sentencias, que toma subobjetos (6.5): `${m::name}` es el nombre de la instancia m, mientras que `#M1::name` en una sentencia es el texto con la clave `name` sobre M1 (13.3).
  - `${m::#type}` busca únicamente un parámetro del anfitrión, igual que `${#type}` (véase abajo).
- **Visualización**: cuando el último valor del camino es texto, se muestra tal como está escrito. Cuando es un objeto: las instancias y redes muestran sus nombres; los terminales, pines y cables muestran el nombre de su red, y un cable de bus los nombres de sus redes unidos con `,`; las agrupaciones tal como se describió arriba; un conjunto (como un rol de conjunto) muestra sus miembros, cada uno de esta manera, unidos con `,`. Los puntos, usos y gráficos no pueden mostrarse: el texto falla. Así que cuando el rol p es un punto, escríbase `${p::x}`, no `${p}`.
- Las referencias dan el texto tal como está escrito en el netlist, sin evaluarlo: `w=1u` muestra 1u. Lo que proporciona cada dialecto lo define su adaptador del anfitrión; en Spectre, por ejemplo, el valor de una resistencia es el parámetro `r`.
- Los atributos enumerados arriba existen siempre; uno que no tiene valor es vacío. Una referencia a un nombre de atributo que no existe hace fallar el texto.
- El contenido referenciado se muestra tal como está escrito y no se lee como marcado: los nombres y valores del anfitrión no están escritos en este marcado.
- Las referencias descienden únicamente desde los atributos del objeto adjuntado; no pueden alcanzar un objeto arbitrario. No existe referencia a otro objeto como `${R3:VALUE}` de KiCad.
- **Alias**: los nombres de atributo en `${…}` también admiten los alias declarados para el archivo, buscados primero como atributo estándar, luego como alias, luego como parámetro del anfitrión; `${#name}` busca únicamente un parámetro del anfitrión (16.5). Los nombres de rol y de parámetro de un uso son nombres RNL: admiten únicamente los alias de `definition=` de su definición, buscados en cada paso en la definición del uso de ese paso, como `${输入对::左管::w}` (16.5).
- **Parámetros del anfitrión con nombres estándar**: un atributo estándar va primero, y un parámetro del anfitrión con el mismo nombre se escribe `${#type}` (`#` representa al anfitrión en RNL). Cualquier parámetro del anfitrión puede escribirse así; `${w}` es abreviatura de `${#w}` donde ningún atributo estándar ni alias tiene ese nombre. Los nombres de atributos estándar se comparan exactamente tal como están escritos, los nombres de parámetros del anfitrión según las reglas del anfitrión: en un anfitrión SPICE, `${Type}` no es un atributo estándar, por lo que es el parámetro type del anfitrión.
- **Formato de nombres**: el contenido referenciado se formatea únicamente en bloque, con marcado alrededor de él, como `\textit{${name}}`, y mediante las dos órdenes `\subrest{…}` y `\subdigits{…}`, que muestran el resto de un nombre, o sus dígitos al final, como subíndice. Otros formatos de caracteres individuales se escriben como texto, como `#M1 @TEXT("M_{1}", key="name")`, que no sigue un renombrado; otra convención adicional de un visor es su estilo de la casa.

### 13.7 Dibujo de conexiones como cables: ROUTE-NET, ROUTE-OBJECT, UNROUTE-NET, UNROUTE-OBJECT

```rnl
&x @ROUTE-NET;                                /* x is drawn as wires; the solver decides the branches and paths */
%jo = (_, _);
&out @WIRE {#M2:1 %jo};
&out @ROUTE-NET;                              /* this wire is drawn as written; the solver adds the rest */
{#M1 #M2 #M5} @ROUTE-OBJECT;                  /* on each net that two or more of their pins share, those pins are joined */
&vdd @UNROUTE-NET(priority="high");           /* the pins of vdd are not joined, whatever asks for it at a lower priority */
# @SELECT-NET(name="clk*", recursive=true) => @ROUTE-NET   /* as a house-style package writes it, with nets chosen by a rule */
```

- Estas palabras clave indican qué conexiones se dibujan como cables, sin indicar dónde se ramifican ni dónde van los puntos de unión; eso lo decide el solucionador.
- **ROUTE-NET** une los pines de una red en un árbol: los pines de los dispositivos en la red, y los pines de los componentes virtuales cuyo `::net` es la red, es decir, los pines vinculados a roles rellenados con ella (12.1). Una ranura, que toma redes; un conjunto se toma miembro por miembro (5.2.1), y no es un bus.
- **ROUTE-OBJECT** une los pines de algunos objetos: para cada red en la que recaen dos o más de sus pines, esos pines se unen en un árbol. Una ranura, que toma un conjunto de objetos; pasa su entrada adelante. Una instancia o una agrupación representa sus pines, un terminal representa su pin, y un uso representa los objetos que rellenan sus roles, hasta los objetos más bajos, igual que para los miembros de un grupo (11.5); un uso con símbolo representa además sus propios pines, cada uno en la red del rol que vincula (12.1). Los objetos sin pines, como una red o un punto que rellena un rol, no añaden ninguno: una red entera se dibuja con ROUTE-NET. Así, una definición del paquete estándar puede dibujar las conexiones dentro de su estructura, como `{?left ?right ?tail} @ROUTE-OBJECT(priority="low")`, sin unir el resto de las redes.
- **UNROUTE-NET** y **UNROUTE-OBJECT** excluyen pines: los pines de una red, o de algunos objetos (contados como para ROUTE-OBJECT), no son unidos por los árboles de ROUTE-NET y ROUTE-OBJECT.
- **Cada pin y cada árbol por separado**: un pin cubierto por un ROUTE-NET o ROUTE-OBJECT está en el árbol de esa sentencia salvo que un UNROUTE-NET o UNROUTE-OBJECT que cubra el pin gane sobre esa sentencia: tiene mayor prioridad (14.8), o igual prioridad y está escrito después, en el orden de posiciones de 8.5. La comparación se hace con cada sentencia ROUTE por separado, de modo que un pin puede excluirse del árbol de un ROUTE-OBJECT de prioridad `"low"` mediante un UNROUTE de prioridad `"normal"` y permanecer en el árbol de un ROUTE-NET de prioridad `"high"`. Una sentencia con `"disabled"` se omite. Estas sentencias no son hechos geométricos y no participan en conflictos (8.5); deciden únicamente qué pines une el solucionador.
- **Cuándo**: las sentencias de estas palabras clave surten efecto desde el paso 3 de 8.2. Sus reglas, como la última línea del ejemplo, se evalúan en el paso 5, antes de la generación: sus condiciones leen los árboles de las sentencias, y la generación, como el relleno del paquete estándar, ve los árboles que crean las reglas (10.2, 10.4).
- **Los cables escritos no se ven afectados**: un WIRE explícito se dibuja tal como está escrito, también en una red o en un pin excluido por UNROUTE. Los cables ya escritos forman parte de los árboles que tocan, y el solucionador añade el resto.
- **Árboles**: cada ROUTE-NET forma un árbol en su red, y cada ROUTE-OBJECT un árbol en cada red que une. Los árboles en una misma red que comparten un pin, o a los que unen cables escritos, forman un solo árbol; este toma el estilo de la sentencia primera en el orden de posiciones, y se emite una indicación cuando sus estilos difieren. Una red puede tener varios árboles que nada une, como dos ROUTE-OBJECT sobre dispositivos alejados.
- Únicamente se unen pines: los terminales sin pin no están en ningún árbol (9.2.4). Un pin en el nodo vacío no está en ninguna red (6.3), por lo que ROUTE-OBJECT no lo une; se conecta únicamente mediante un cable escrito en el nodo vacío (13.1). Un árbol con menos de dos puntos que conectar no hace nada. El que un pin esté cableado (10.2) se decide antes de la generación, a partir de los árboles sin los componentes virtuales que añade la generación: una red que tiene un único pin propio, como una entrada que excita una sola puerta, forma un árbol de un punto, por lo que ese pin no está cableado y el relleno le da lo que da a un pin sin cable, aun cuando la red se trace hasta el puerto de su nivel. Para trazar una red así hasta su puerto, escríbase la etiqueta de puerto en el archivo, como `$IO_PORT @ATTACH &x` o `$IO_PORT_STUB` del paquete estándar (12.4, 16.3): una etiqueta de puerto que añade el relleno no cuenta.
- **Parámetros**: ROUTE-NET y ROUTE-OBJECT admiten `mode` (una pista para el solucionador, con los valores de WIRE, por defecto `"auto"`), los parámetros de estilo de los cables (13.1) y `priority`; UNROUTE-NET y UNROUTE-OBJECT admiten `priority`.
- Nota: un WIRE con una lista vacía de puntos no significa ROUTE-NET. Una lista de puntos puede proceder de un conjunto que resulte estar vacío, y una lista vacía conectaría entonces todo en silencio; por ello estas son palabras clave propias; esta versión tampoco tiene ranuras opcionales (5.3).
- **Escritura de vuelta por el solucionador**: los WIRE concretos (con sus codos y `mode="straight"`), los puntos con nombre donde se ramifican, y los puntos de unión, cuando `junction` está fijado (13.2), con los parámetros de estilo del árbol, van al propio registro del solucionador. Las sentencias ROUTE-NET y ROUTE-OBJECT son la intención del usuario y permanecen; la siguiente resolución vuelve a calcularlas (capítulo 17). Para ramificarse desde el punto medio de un cable escrito por el autor, el solucionador divide ese cable (13.1).
- **Los visores que no resuelven** dibujan líneas de conexión o nada, según decida el visor (Principios 2.7).
- Un paquete de estilo de la casa elige qué redes se dibujan como cables mediante conjuntos y reglas, como en el ejemplo; no hay ninguna palabra clave para cómo se muestra una red.

<!-- translated from en/ch14.md sha256:76f571ca1b76 -->

## 14 Intención de disposición

### 14.1 Reglas comunes

- Una restricción crea un objeto restricción. A una restricción con nombre se le puede hacer referencia, lo que ayuda a encontrarla y editarla, como `%row1 = {#M1 #M2} @ALIGN("y")`; una restricción también puede ser anónima.
- **Vocabulario**: las restricciones atómicas sobre posiciones son únicamente ALIGN, ORDER, DISTRIBUTE y DISTANCE, además de PLACE, ORIENT, GROUP y TABLE (14.11), que son órdenes escritas en forma de tabla. Otras relaciones se escriben como definiciones generales: simetría (14.7), centrado (un DISTRIBUTE de tres miembros), filas y columnas (ALIGN con ORDER), flujo de señal (14.6), pares reflejados, espaciados en proporción (varios DISTANCE exactos). No existen LEFT ni ABOVE (son un ORDER o un DISTANCE con una dirección), ni tampoco RATIO, NEAR ni INSIDE. La orientación relativa (la misma orientación, imágenes reflejadas) es ORIENT-RELATION (14.9); si las puertas de un par se miran entre sí o miran hacia fuera se sigue fijando con un ORIENT, o con un ORDER sobre terminales (14.3). Que los símbolos no se solapen, las constantes de espaciado y las cuadrículas pertenecen al solucionador (11.7) y no forman parte del vocabulario de restricciones.
- **No hay posiciones relativas a todo el dibujo**: no existe ninguna palabra clave para "más a la izquierda", "más a la derecha" o "más arriba", ni extensión de todo el dibujo o de los contornos de los símbolos. Escríbase un ORDER o un DISTANCE respecto a un grupo de objetos, y el solucionador encuentra la disposición a partir de él. Para un puerto en el extremo derecho: `{#M6 #M7 #CL} %pout @DISTANCE("left-to-right", min=2)`, o `{#M7 %pout} @ORDER("left-to-right")` cuando M7 ya es el más a la derecha.
- **Los miembros son los objetos más bajos**: instancias, agrupaciones, terminales y pines, puntos, TEXT (por su propia posición, 13.3), el centro de un RECTANGLE, CIRCLE o ELLIPSE escrito con un centro, y usos dibujados con símbolo, cada uno por su propio punto. Un uso sin símbolo no es un miembro en sí mismo, sino que representa a sus miembros: los objetos que rellenan sus roles, siguiendo hacia dentro todo miembro que sea un uso sin símbolo hasta los objetos más bajos, como en GROUP (11.5), omitiendo las redes que haya entre ellos. Su propio punto se escribe `%u::position` (6.5). Los cables, las redes, las restricciones y las definiciones no pueden ser miembros: si se pasa uno, la restricción falla. Una posición sobre un cable debe escribirse primero como un punto (13.1). Un conjunto no es un miembro: no tiene posición propia (abajo).
- **Qué punto cuenta**: una instancia cuenta por su origen, el origen de su símbolo, que es también lo que coloca PLACE; un terminal o un pin por la posición del pin, que es la posición del dispositivo más la transformación rígida de la posición del pin en el símbolo y no puede moverse por separado; un punto por sí mismo; un uso con símbolo, o `::position` de cualquier uso, por su propio punto (abajo); un TEXT por su propia posición; un RECTANGLE, CIRCLE o ELLIPSE escrito con un centro por su centro. El centro del marco de un símbolo no se usa: los marcos de los símbolos MOS de tres y de cuatro terminales difieren, y una fila de ellos no quedaría alineada. Para alinear por pines, escríbanse terminales. La simetría visual exige que el origen quede en el eje de simetría del símbolo, lo cual depende de la biblioteca de símbolos.
- **El propio punto de un uso**: todo uso tiene un punto propio, también un uso sin símbolo. El punto comienza libre, como `(_, _)`, y queda ligado por hechos: PLACE, restricciones, cables hacia los pines de su símbolo y los hechos que escribe sobre él el cuerpo de su definición; el solucionador coloca lo que queda libre. Un uso con símbolo se dibuja con el origen de su símbolo en este punto (12.1), y una restricción sobre él actúa sobre este punto. Una restricción sobre un uso sin símbolo actúa sobre sus miembros (arriba); para restringir su punto, escríbase `%u::position`: `{%dp::position #X1} @ALIGN("y")` sitúa X1 a la altura del centro del par, mientras que `{%dp #X1} @ALIGN("y")` situaría X1 y los tres transistores del par a una misma altura. El cuerpo de su definición liga el punto a sus miembros donde escribe hechos sobre el `?` aislado (9.4), como `{?members::position ?} @ALIGN("y")`. Cuando el punto de un uso no está ligado por ningún hecho a nada que se dibuje, una restricción sobre `%u::position` no tiene efecto visible, y una indicación lo señala.
- **Los conjuntos no tienen posición**: un conjunto es únicamente una lista de miembros, sin centro y sin extensión. En la lista de miembros de ALIGN, ORDER o DISTRIBUTE, un elemento que es un conjunto, un `{…}` escrito explícitamente o una referencia a un conjunto (5.2), representa a cada uno de sus miembros más internos en ese elemento: la sentencia es una restricción por cada combinación que toma un miembro de cada uno de tales elementos. `{#M1 {#M2 #M3} #M4} @ORDER("top-to-bottom")` es `{#M1 #M2 #M4}` y `{#M1 #M3 #M4}`, de modo que M2 y M3 quedan cada uno entre M1 y M4 y no están ordenados entre sí. Un elemento que sea un uso sin símbolo representa a sus miembros (arriba) del mismo modo. Un conjunto en una ranura de DISTANCE representa a cada uno de sus miembros, una restricción por cada par (14.5). Un elemento que es un conjunto vacío se omite. Una combinación que tome un mismo objeto dos veces se omite: dos estructuras que comparten un dispositivo, ordenadas mediante `{%stage1 %stage2} @ORDER("left-to-right")`, ordenan todos los demás pares de sus miembros y no ordenan ese dispositivo respecto de sí mismo.
  - Las restricciones así creadas tienen la prioridad, el nombre y la posición de la sentencia escrita (8.5, 14.8); un nombre vincula el conjunto de ellas. Una combinación con demasiado pocos miembros para afirmar nada produce una indicación.
  - Una implementación puede limitar cuántas restricciones representa una sentencia; más allá de su límite la sentencia falla, con un error que indica que es un límite de la implementación (1.6).
  - Un conjunto que es el único objeto se divide un nivel primero (5.2.1): `%row_set @ORDER("left-to-right")` y `{%row_set} @ORDER("left-to-right")` ordenan ambos los miembros del conjunto.
- **Las direcciones se escriben explícitamente.** ALIGN toma una proyección, `"x"` o `"y"` (14.2); ORDER, DISTRIBUTE y DISTANCE toman una dirección, una de cuatro (14.3). Ninguna forma deja la dirección abierta: una relación que puede dibujarse de cualquiera de dos maneras se escribe como un hecho para una manera, y una definición que debe permitir ambos dibujos viene como dos definiciones, como `$BRANCH` y `$BRANCH_HORIZONTAL` del paquete estándar. Los únicos hechos que ofrecen una elección son las listas de orientaciones de ORIENT (11.2) y ORIENT-RELATION (14.9).
- Las restricciones no cambian el netlist. En un conflicto, una restricción atómica se conserva en bloque o se descarta en bloque (8.5).
- Las cuatro restricciones atómicas, ORIENT-RELATION y TABLE admiten el parámetro `priority=` (14.8).

### 14.2 ALIGN

`{…} @ALIGN("y")`: los miembros son iguales en la proyección `axis`: `"y"` les da la misma y (una fila), `"x"` la misma x (una columna). `axis` es obligatorio, puede darse por posición y es una sola proyección: una lista de proyecciones es un valor de parámetro erróneo y la restricción falla. Con menos de dos miembros, la restricción se cumple trivialmente y no falla: cuántos miembros tiene una selección o un conjunto no se sabe al escribir la sentencia.

### 14.3 ORDER

```rnl
%rc_set = {#R2 #C2};
{#M1 #M1:2 #M2:2 #M2} @ORDER("left-to-right");  /* M1 on the left, the two gates face each other */
{#R1 {%rc_set} #M1} @ORDER("left-to-right")     /* R2 and C2 each between R1 and M1 */
```

- `{…} @ORDER(direction)`: los miembros son estrictamente monótonos en la proyección de la dirección. Con menos de dos miembros, se cumple trivialmente.
- **Dirección**: el parámetro `direction`, obligatorio, dado por posición en primer lugar. Sus valores son:
  - `"left-to-right"`, `"right-to-left"`: x creciente, x decreciente;
  - `"bottom-to-top"`, `"top-to-bottom"`: y creciente, y decreciente (y va hacia arriba, 11.7).

  DISTRIBUTE y DISTANCE admiten los mismos valores. No existe ningún valor que fije únicamente el eje: `"x"` e `"y"` son valores de parámetro erróneos y la restricción falla.
- Un elemento que es un conjunto representa a cada uno de sus miembros (14.1): el segundo ejemplo ordena R1, R2, M1 y R1, C2, M1; R2 y C2 no están ordenados entre sí.
- ORDER es un hecho sobre puntos: compara los puntos de los miembros, no sus marcos. Mantener separados los marcos de los símbolos a lo largo de un ORDER es un criterio del solucionador, clasificado después de todo hecho (8.5); cuando los hechos obligan a los marcos a solaparse, se solapan, y una indicación puede señalarlo.
- ORDER es estricto: los miembros vecinos difieren en al menos una unidad de coordenadas (11.7). Es abreviatura de un DISTANCE en su dirección con `min=1` entre cada par de miembros vecinos (14.5). Para coordenadas iguales, úsese ALIGN.

### 14.4 DISTRIBUTE

```rnl
{#R1 #R2 #R3} @DISTRIBUTE("left-to-right")     /* R1, R2 and R3 from left to right, in equal steps */
```

- `{…} @DISTRIBUTE(direction)`: los miembros se suceden estrictamente en la dirección, como en ORDER (14.3), y las diferencias entre miembros vecinos en su proyección son iguales. Los pasos son entre los puntos de los miembros, no huecos entre marcos; un elemento que es un conjunto representa a cada uno de sus miembros (14.1).
- **Dirección**: el parámetro `direction`, obligatorio, dado por posición en primer lugar, con los valores de ORDER (14.3).
- Con dos miembros, DISTRIBUTE es lo mismo que ORDER; con menos de dos, se cumple trivialmente.

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

- `A B @DISTANCE(direction, min=…, max=…)`: A rellena la ranura `from` y B la ranura `to`; cada uno es un objeto o un conjunto.
- **Dirección**: el parámetro `direction`, obligatorio, dado por posición en primer lugar, con los valores de ORDER (14.3). Únicamente se restringe ese eje; no es la distancia euclídea.
  - Una dirección creciente (`"left-to-right"`, `"bottom-to-top"`) restringe B − A;
  - una dirección decreciente (`"right-to-left"`, `"top-to-bottom"`) restringe A − B.
- **Conjuntos**: un conjunto en una ranura representa a cada uno de sus miembros (14.1), al igual que un uso sin símbolo, y el DISTANCE se cumple desde cada miembro de A hasta cada miembro de B, una restricción por cada par; todo par de un objeto consigo mismo se omite. Así, `%stage1_set %stage2_set @DISTANCE("left-to-right", min=1)` sitúa toda la etapa 2 a la derecha de toda la etapa 1, y dos conjuntos que no deben intercalarse se escriben así. Cuando un lado es un conjunto vacío, no hay ningún par y no sucede nada.
- Debe escribirse al menos uno de `min`, `max` y `exact`; `min` no debe ser mayor que `max`. Un DISTANCE declara también el orden: B nunca está antes que A en la dirección. Por tanto, las cotas no son negativas, y un `min`, `max` o `exact` negativo es un valor de parámetro erróneo, por el que la restricción falla; un `min` no escrito vale 0, de modo que `max=10` se cumple para una distancia de 0 a 10.
- `exact=` es una distancia exacta, abreviatura de `min` y `max` iguales: B − A = exact para una dirección creciente, A − B = exact para una decreciente. No puede escribirse junto con `min` o `max`: eso es un parámetro inválido y la sentencia falla. Las herramientas escriben las distancias exactas con `exact` (4.3).

### 14.6 Flujo de señal

El flujo de señal (Principios 2.4) se escribe como el uso de una definición general; no hay ninguna palabra clave para él. Si el paquete estándar proporciona tal definición, y bajo qué nombre, depende del paquete estándar (16.3); los ejemplos utilizan `$SIGNAL_FLOW` de la serie estándar 0.4, que nombra un camino, dado como un conjunto, y puede nombrar las fuentes antes de él y las cargas después de él. Es una definición semántica: no coloca nada.

```rnl
%rc_set = {#R2 #C2} @GROUP;
{#R1 {%rc_set} #M1} @AS $SIGNAL_FLOW                                    /* the signal goes from R1 through the block of R2 and C2 to M1 */
{#:inp #:inn}(source) {#R1 {%rc_set} #M1}(path) #CL(load) @AS $SIGNAL_FLOW   /* from the two inputs of the level to the load CL */
```

- El flujo de señal es una noción blanda: no todo circuito tiene una dirección estricta. Un solucionador puede leerlo como una preferencia; para poner las etapas en orden en el dibujo, escríbase ORDER o TABLE (14.3, 14.11).
- Una etapa del camino formada por varios objetos se escribe como un `{…}` propio, como `{%rc_set}`. Un GROUP (11.5) únicamente hace que el solucionador lo trate como un bloque, y puede omitirse.
- Las fuentes y las cargas forman cada una un grupo sin orden, como las dos entradas de una señal diferencial. Pueden ser dispositivos, estructuras, puertos del nivel como `#:inp` (6.4), otros terminales o redes. Sin roles marcados, los objetos son el camino, como en la primera línea.
- El flujo de potencia, desde una alimentación a través de las etapas que la convierten o conmutan hasta las partes que la utilizan, se escribe del mismo modo con `$POWER_FLOW`.

### 14.7 Simetría

La simetría se escribe como el uso de una definición general. El eje de simetría es únicamente un punto libre, y no hace falta ninguna palabra clave. La serie estándar 0.4 proporciona `$MIRRORED_LAYOUT`, con los roles `left`, `right` y `axis`: izquierda y derecha a la misma altura, el eje en medio entre ambos, y el derecho orientado como el izquierdo reflejado izquierda-derecha; y `$MIRRORED_VERTICAL_LAYOUT`, con `top`, `bottom` y `axis`, lo mismo girado en vertical. El punto propio de cualquiera de ambos usos está en el centro entre los dos lados (14.1), de modo que una restricción sobre el uso mueve el par. Como en toda definición del paquete estándar, estas restricciones tienen prioridad `"low"` (9.5). El eje es un solo objeto: cuando varios dispositivos están sobre el eje, primero se convierten en un uso, que cuenta por su propio punto (14.1), o se da uno de ellos.

```rnl
%ax = (_, _);
%sym12 = #M1(left) #M2(right) %ax(axis) @AS $MIRRORED_LAYOUT
```

- `$MIRRORED_LAYOUT` no fija la orientación real: si las puertas miran hacia dentro o hacia fuera sigue requiriendo un ORIENT en un lado (14.9). Para un par simétrico con la misma orientación en ambos lados, escríbase otra definición general, sin su ORIENT-RELATION.

### 14.8 Prioridad: PRIORITY

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

- **Cuatro valores**: `"disabled"`, `"low"`, `"normal"`, `"high"`. Un hecho sin prioridad es `"normal"`, ya esté escrito en un registro del usuario, en un paquete o en el cuerpo de una definición.
- **Por rangos, no ponderado**: en un conflicto, se comparan primero las prioridades, y se conserva la más alta; a igual prioridad, se conserva la escrita después (8.5). Las restricciones no se suman con pesos.

  Nota: con pesos, diez restricciones de prioridad baja podrían superar a una de prioridad alta, y los solucionadores difieren en cómo miden cuánto se incumple una restricción.
- **Todo hecho que se conserva es estricto**: `"low"` significa "descartado primero en un conflicto", no "lo más cerca posible". El solucionador debe satisfacer todo hecho conservado, sea cual sea su prioridad; los objetivos propios de la implementación para lograr un dibujo bien presentado se optimizan únicamente dentro de los hechos conservados, después de todas las descripciones del lenguaje.
- **`"disabled"`**: el hecho se elimina antes del ordenamiento y no se pasa al solucionador. Esto no es un fallo: el objeto sigue existiendo, las descripciones que dependen de él no se ven afectadas, no se emite advertencia y una prioridad escrita después puede activarlo de nuevo.
- **Cómo se escribe**:
  - PLACE, ORIENT, las cuatro restricciones atómicas, ORIENT-RELATION, TABLE, ROUTE-NET, ROUTE-OBJECT, UNROUTE-NET, UNROUTE-OBJECT, AS y ATTACH admiten el parámetro `priority=`, y la prioridad acompaña al hecho. `priority` es el único parámetro estándar de AS y ATTACH; un parámetro de una definición no puede tener este nombre.
  - La sentencia PRIORITY cambia una prioridad después. Admite únicamente objetos con manejador propio: restricciones, usos y conjuntos con GROUP (únicamente activar y desactivar, 11.5). Un conjunto con GROUP se toma como el propio grupo; cualquier otro conjunto se toma miembro por miembro (5.2.1). Si recibe una instancia u otro objeto, falla (clase incorrecta): `#M1 @PLACE(…) => @PRIORITY(…)` es un error, porque PLACE pasa adelante el manejador de M1; escríbase la prioridad como parámetro de PLACE.
  - El encabezado del registro `@RNL(priority=…)` da un valor por defecto a las sentencias del registro que admiten este parámetro y no lo escriben, y a `dx` y `dy` de TEXT (13.3). A AS y ATTACH les da únicamente `"disabled"`, de modo que un registro desactivado desactiva sus usos con sus cuerpos, como sus demás sentencias. Bajo un encabezado con `"high"`, `"normal"` o `"low"`, un uso tiene prioridad únicamente cuando su propia sentencia o una sentencia PRIORITY le da una, y en caso contrario conserva las prioridades de su cuerpo, igual que cuando lo crea una regla, de modo que un uso que escribe el solucionador (10.5) significa lo mismo que el generado. Una prioridad que escribe un uso sigue sustituyendo a la de su cuerpo (abajo).
- **Orden de sustitución**, de menor a mayor: el valor por defecto del encabezado del registro; el parámetro propio de la sentencia; las sentencias PRIORITY (entre varias, gana la posterior; para las de paquetes, según la posición del USE-PACKAGE); la prioridad de un uso que lo encierra; las sentencias PRIORITY escritas desde fuera de un uso, mediante `::`, directamente sobre uno de sus objetos locales (entre varias, gana la posterior). Cuando no se da ninguno de estos, la prioridad es `"normal"`.
- **La prioridad de un uso desciende**: la prioridad de un uso (su propio parámetro o una sentencia PRIORITY sobre él) se aplica a todos los hechos a los que se expande, incluidos los de usos anidados, y sustituye a sus propias prioridades; cuando hay anidamiento, decide el uso más externo que tenga prioridad. Para ajustar únicamente una parte interior, escríbase directamente sobre ese objeto local desde fuera, mediante `::` (9.4): esto es más específico que la prioridad del uso y la sustituye: gana la más específica. Tal sentencia escrita en el cuerpo de una definición que lo encierra sigue sujeta a la prioridad del uso que lo encierra. Toda sentencia de un registro puede cambiarse directamente, de modo que el encabezado del registro da únicamente un valor por defecto.
  - En detalle: la prioridad de un uso gobierna toda sentencia PRIORITY escrita en el cuerpo de su definición, que entonces no tiene efecto. Una PRIORITY escrita desde fuera mediante `::` directamente sobre un uso local gobierna del mismo modo: ese uso cuenta como el más externo con prioridad, y la sentencia decide para todo aquello a lo que el uso se expande. Entre las sentencias a las que nada gobierna, gana la escrita sobre el objeto más interno; sobre el mismo objeto, una escrita desde fuera del cuerpo que crea el objeto gana sobre una escrita en ese cuerpo; luego gana la posterior.
  - Por ejemplo, con `%o1` desactivado, `%o1::in @PRIORITY("high")` arriba y `%in::row @PRIORITY("low")` en el cuerpo de la definición de `%o1`, los hechos de `row` son `"high"`: la sentencia en el cuerpo queda gobernada por la prioridad de `%o1`, a la que sustituye la sentencia de arriba para `in`. Si `%o1` no tuviera prioridad, serían `"low"`, escritos sobre el objeto más interno. Para tener `"low"` en ambos casos, escríbase `%o1::in::row @PRIORITY("low")` desde fuera.
- **La salida del solucionador**: el solucionador escribe sus resultados como PLACE, ORIENT y demás, con `"high"`, en el último registro del propio texto del nivel, como `@RNL(type="generated", priority="high") {…}`. Para un subcircuito, esto está dentro de su `.subckt … .ends`, incluso cuando está en un archivo incluido; para el nivel superior, está en el nivel de archivo del archivo principal (capítulo 17). Cuando el solucionador resuelve, el nivel se interpreta sin los registros con `type="generated"` (2.8), de modo que los resultados antiguos no restringen la siguiente resolución; tras resolver, los elimina y escribe los nuevos resultados al final.
  - Entre resoluciones, una nueva sentencia de prioridad normal que entra en conflicto con los resultados del solucionador se descarta y surte efecto únicamente tras la siguiente resolución. Un visor puede sugerir resolver de nuevo, mediante el hash del encabezado del registro; tales descartes no necesitan señalarse uno por uno.
  - Un arrastre en un editor se escribe como un PLACE con `"high"`, también en el propio texto del nivel, después del registro del solucionador. Esto fija la posición: toda resolución posterior lo toma como una restricción estricta. Para liberarla, elimínese esa sentencia.
- **Qué escribe el solucionador.** El registro del solucionador debe contener toda posición, orientación y trazado que decidió el solucionador y que el texto deja libre: por nombre o clave para los objetos que tienen uno, por número para los demás (5.9), con WIRE para el trazado de un cable (13.1) y como sentencias explícitas para los objetos que generan las reglas (10.5). Una decisión que no está escrita allí no es un resultado: el objeto permanece libre, como si el nivel no se hubiera resuelto. Así, un visor que no resuelve dibuja el nivel exactamente como lo dibujó el solucionador. El solucionador no decide aquello a lo que no puede hacer referencia, como la posición de un texto sin clave que crea directamente una regla de generación; el visor coloca tal texto (13.3). Tampoco se escribe el punto propio de un uso creado por una regla cuando ese uso no puede escribirse explícitamente, como cuando su definición es privada de un paquete (16.2), no tiene símbolo ni texto, y ningún otro objeto o cable se coloca mediante su punto: tal punto solo ayuda al solucionador, nada dibujado descansa sobre él, y dejarlo libre no cambia ningún dibujo; no cuenta como una decisión dejada sin escribir.
- **Los resultados de un solucionador son RNL.** Cuando un solucionador pasa sus resultados a otras herramientas en otra forma, como los mensajes entre un solucionador y un visor, esa forma debe contener únicamente lo que puede contener un registro del solucionador, de modo que todo resultado pueda guardarse como un registro tal.
- **Los objetivos de un solucionador** eligen únicamente entre las disposiciones que satisfacen todo hecho conservado; un hecho conservado es estricto, sea cual sea su prioridad. Los objetivos no forman parte del lenguaje: las clases de objetivos y su orden difieren entre solucionadores, y este estándar no puede definir un vocabulario común para ellos. Se fija un valor por defecto: un cable `"auto"` sin nada que se interponga y no fusionado con otros cables se resuelve a su trazado por defecto (13.1), que cuenta primero los codos y segundo la longitud. Las opciones del solucionador forman parte de la entrada que cubre el hash (capítulo 17); un archivo las conserva con SOLVER-CONFIG, de modo que un resultado pueda reproducirse (14.10).

### 14.9 Relación de orientación: ORIENT-RELATION

```rnl
#M1 #M2 @ORIENT-RELATION("MY");                  /* M2 is drawn as M1 flipped left-right */
#M5 {#M7 #M8} @ORIENT-RELATION("R0");            /* two facts: M7 and M8 each have the orientation of M5 */
#M3 #M4 @ORIENT-RELATION(["R0", "MY"])           /* M4 as M3, or as M3 flipped left-right */
#M1::g #M2::g @ORIENT-RELATION("R180")           /* the gates of M1 and M2 face opposite ways */
```

- `A B @ORIENT-RELATION(relation)`: dos ranuras, `from` y `to`, cada una un objeto. La relación es uno de los ocho códigos de orientación de ORIENT (11.2), o una lista de ellos, dada por posición. La orientación de B es la orientación de A seguida de la transformación de la relación. La transformación se toma en las coordenadas del dibujo, de modo que `"MY"` invierte siempre izquierda-derecha respecto a una línea vertical, esté girado o no el propio A.
- **Una lista de relaciones**, como `["R0", "MY"]`, forma un hecho, que se cumple cuando entre A y B se cumple una de las relaciones enumeradas. Se conserva o se descarta en bloque (8.5), y el solucionador elige qué relación dibuja. La lista tiene al menos un código; una lista de un código equivale a ese código.
- **Un objeto a un objeto.** Un conjunto en una ranura se procesa miembro por miembro (5.2.1), un hecho para cada miembro: `#M5 {#M7 #M8} @ORIENT-RELATION("R0")` da a M7 y M8 cada uno la orientación de M5, y por tanto a los tres la misma. Un conjunto de más de un miembro en ambas ranuras es un error: emparejar los miembros en orden y relacionar cada par tendrían ambos sentido (5.2.1).
- No existe forma con un solo conjunto cuyos miembros tengan todos la misma orientación: la palabra clave nombra una relación entre dos cosas, y con un conjunto no queda claro a qué se aplica la relación. Un solo objeto no coincide con la firma, lo que es un error. Un único conjunto se divide primero un nivel, como indica 5.2.1, de modo que `{#M1 #M2} @ORIENT-RELATION("MY")` es `#M1 #M2 @ORIENT-RELATION("MY")`; cuando la división no da dos objetos, es un error.
- Los miembros son objetos que tienen orientación, como para ORIENT (11.2): instancias, agrupaciones y usos con símbolo; o ambos son pines (abajo), representando un terminal a su pin (5.2.4). Un punto, red, cable o uso sin símbolo es un error, y también un pin relacionado con un objeto: la restricción falla. Cuando una ranura recibe un conjunto vacío, no hay nada que relacionar y no sucede nada (5.2.1).
- **Pines.** Con un pin en cada ranura, la relación se cumple entre las direcciones de ambos pines en el dibujo (11.2): la dirección de B es la dirección de A girada por la relación, como `"R180"` para pines que miran en sentidos opuestos, y un código reflejado gira una dirección igual que gira un punto. La relación se escribe únicamente con los códigos: las palabras de 11.2 nombran direcciones, no giros, y escritas aquí son valores de parámetro erróneos, por lo que la restricción falla. Un pin sin `direction=` escrito hace fallar ese hecho (9.2.4).
- Es una restricción: crea un objeto restricción, puede nombrarse, admite `priority=` y puede cambiarse con PRIORITY (14.8); en conflictos se conserva o se descarta como una unidad (8.5). Procesada miembro por miembro, crea una restricción por cada miembro, y un nombre vincula el conjunto de ellas (5.2.1). Un miembro con un ORIENT de un código ya tiene fijada su orientación; para los demás miembros, el solucionador elige entre las orientaciones que cumplen la relación y sus listas de ORIENT (11.2).
- No fija la orientación real: cuando dos transistores MOS son imágenes reflejadas, las puertas mirando hacia dentro y las puertas mirando hacia fuera cumplen ambas `"MY"`. Para decidir, escríbase además un ORIENT, o un ORDER sobre terminales (14.3); para acotarla sin decidir, escríbase ORIENT con una lista (11.2).

### 14.10 Ajustes del solucionador: SOLVER-CONFIG

```rnl header
@SOLVER-CONFIG(spacing_x=2, spacing_y=1, layout="lines", lines_weights=[1, 4])    /* the default of the file */
```

```rnl
@SOLVER-CONFIG(spacing_x=1)                                                       /* this level: overrides the key spacing_x */
```

- SOLVER-CONFIG proporciona ajustes al solucionador, para que el archivo pueda resolverse de nuevo con los mismos ajustes. No crea hechos: el solucionador utiliza los ajustes únicamente para elegir entre las disposiciones que satisfacen todo hecho conservado (14.8).
- **Forma**: parámetros por nombre en literales RNL: números, cadenas, `true` y `false`, y matrices de estos (3.7). No hay ajustes anidados. Aparte de las claves que nombra este estándar (abajo), las claves no son palabras estándar: cada solucionador define las suyas. El núcleo lee los parámetros como literales RNL y los pasa al solucionador tal como están escritos, en el orden de abajo, sin fusionar ninguno; de los valores, comprueba únicamente los de las claves que nombra este estándar. El solucionador juzga las demás claves y valores: una clave que no conoce produce una advertencia y se ignora. Un solucionador también puede aceptar una clave solo bajo algunos de sus propios ajustes, por ejemplo un objetivo que usa una de sus disposiciones y otra no; bajo un ajuste que no usa la clave, la clave se ignora del mismo modo, con una advertencia.
- **Claves nombradas por este estándar**: `spacing_x` y `spacing_y`, el espaciado del nivel en unidades de cuadrícula, sumado al espaciado por defecto del solucionador, como para grupos (11.5); y `junction`, la definición con la que el solucionador dibuja los puntos en los puntos de ramificación, un nombre `$` escrito como cadena, como `"$JUNCTION"`; en el SOLVER-CONFIG de un registro de nivel también un nombre `%` buscado en ese nivel (6.6), de modo que una variante definida para un subcircuito permanezca en él; o `""` para ningún punto de unión (13.2). Todo solucionador admite `junction`. Son las únicas claves que nombra esta versión, no las únicas claves que puede admitir un solucionador: qué otras claves tiene un solucionador, como `layout` y `lines_weights` en el ejemplo, y qué significan, depende de ese solucionador, que las documenta. Un solucionador que tiene un espaciado del nivel lo toma por estos dos nombres.
- **Dónde**: en el registro de encabezado sin `language`, para los niveles de ese archivo y de los archivos que incluye, como para DEVICE (2.8, 6.12); en el registro de encabezado de un archivo de paquete, para los niveles que usan el paquete (16.2); y en los registros de un nivel, para ese nivel. No tiene objetos. Escrito en el contenido de un paquete, en un paquete en línea, en un registro con `language` o en un registro del solucionador, no tiene efecto y se emite una advertencia; así, un paquete usado únicamente mediante registros de encabezado con `language` no aporta ajustes.
- **Orden**: los ajustes se sustituyen clave por clave, el posterior sobre el anterior: los valores por defecto del solucionador, los registros de encabezado de los paquetes usados por el nivel, en el orden de sus posiciones (como para DEVICE, 6.12), los registros de encabezado de los archivos externos, luego los de los archivos incluidos, los registros del nivel, luego los ajustes dados a la herramienta, como en su línea de comandos o en una página de un visor. Una herramienta toma estos en el mismo texto que los paréntesis, de modo que puedan copiarse en cualquier sentido.
- **Un valor que no se admite**, como una cadena donde se admite un número, es un error solo para esa clave (16.4): la clave cuenta como no escrita en ese lugar, de modo que sigue en vigor el valor anterior, y las demás claves de la misma sentencia se aplican como de costumbre. Un valor se juzga en cada nivel al que se aplica: un `junction` en un registro de encabezado que no nombra ninguna definición en un nivel queda sin admitir solo en ese nivel, y los demás niveles lo siguen usando. El núcleo juzga los valores de las claves que nombra este estándar y omite los que no admite; el solucionador juzga los valores de sus propias claves. Así, los ajustes que recibe un solucionador son una lista en el orden de arriba, en la que una clave puede aparecer más de una vez, y para cada clave el solucionador toma el último valor que acepta.
- **Los presupuestos de tiempo o de esfuerzo** (8.5) los da solo la herramienta, como en su línea de comandos o en una página de un visor, nunca el archivo: un presupuesto depende de la máquina que resuelve. Las claves de un solucionador que fijan un presupuesto, como `limit`, `seconds` y `work` del solucionador integrado, son sus claves exclusivas de la herramienta: escrita en SOLVER-CONFIG, en cualquier registro, una clave así se ignora, con una advertencia, como una clave desconocida, y sigue en vigor el presupuesto dado por la herramienta, o el valor por defecto del solucionador.
- Los ajustes en vigor para un nivel, incluidos los de paquetes, forman parte de la entrada que cubre el hash del solucionador (capítulo 17).

### 14.11 Tablas: TABLE

```rnl
@TABLE layout: {
  {#M1 #M2 #M3}
  {#M4 {#M5 #M6} #M7}
}
```

- TABLE dispone objetos como una tabla escrita con `{…}` anidados: el ejemplo tiene dos filas de arriba abajo; en la primera, M1, M2 y M3 de izquierda a derecha; en la segunda, M4, la celda de M5 y M6, y M7 de izquierda a derecha; en esa celda, M5 encima de M6. Una tabla puede escribirse en varias líneas. La forma estándar escribe primero la palabra clave, leída como "@TABLE layout: {…}" (4.3); la palabra `layout:` allí es texto libre (3.3), y la ranura de TABLE que toma la tabla es `rows` (Anexo A).
- **Lectura**: la lista externa discurre de arriba abajo, un elemento por cada fila; cada lista anidada en ella discurre en ángulo recto respecto a la lista que la contiene, de izquierda a derecha en una lista que discurre de arriba abajo, y de arriba abajo en una que discurre de izquierda a derecha. `transpose=true` hace que la lista externa discurra de izquierda a derecha. El conjunto pasado a TABLE es su lista externa tal como está escrita y no se divide primero (5.2.1 punto 2), de modo que `@TABLE layout: {{#M1 #M2 #M3}}` es una fila, y `@TABLE layout: %port_set` sitúa cada elemento del conjunto en una fila propia.
- **Elementos**: un elemento que es un objeto es una celda; un elemento que es un conjunto, un `{…}` escrito en la tabla o una referencia a un conjunto, es una lista un nivel más adentro. Esta es la regla de TABLE: un conjunto en él no es un elemento que representa a cada uno de sus miembros (14.1). Un uso dibujado con símbolo es una celda, por su propio punto; un uso sin símbolo es una celda que contiene sus miembros (14.1), todos ellos antes del elemento siguiente; `%u::position` convierte su propio punto en una celda.
- **El hecho**: un TABLE es un hecho geométrico, conservado o descartado en bloque en conflictos (8.5). Se cumple cuando, en toda lista de la tabla, cada punto de un elemento queda estrictamente antes de cada punto del elemento siguiente, en la dirección de esa lista: al menos una unidad de coordenadas antes, como para ORDER (14.3). Un comprobador puede expandirlo en tales órdenes; al solucionador se le pasa la tabla como un elemento, con la dirección de su lista externa y de las listas anidadas en ella.
- **Un grupo**: los objetos de toda la tabla forman un grupo, como con `@GROUP` sobre el mismo conjunto (11.5); sus filas y columnas no son grupos propios.
- TABLE no alinea: las filas no tienen por qué tener una misma altura, ni las columnas una misma x. Para alinearlas, escríbase ALIGN. Una celda no abarca varias filas o columnas.
- Una lista vacía en una tabla se omite, y sus vecinas pasan a ser vecinas (14.1). Una tabla de una celda se cumple trivialmente y no crea ningún grupo. Un objeto en dos celdas de una misma tabla tendría que ir antes que sí mismo, por lo que el TABLE falla, igual que cuando un miembro no puede ubicarse, como una red o un terminal sin pin (14.1).
- TABLE admite `priority=` (14.8) y crea un objeto restricción, que puede nombrarse.

<!-- translated from en/ch15.md sha256:7d292a08ebfb -->

## 15 Semántica eléctrica

### 15.1 Aplicación de definiciones semánticas

```rnl
%dp1 = #M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N;
%cm1 = {#M3 #M9}(ref) {#M4 #M6}(out) @AS $CURRENT_MIRROR_P;
%in = {#M8 #M5} @AS $MATCHED
```

- Lo que significan los objetos se marca aplicando una definición semántica con AS (9.5): la aplicación crea un uso, y los objetos rellenan los roles de la lista de roles de la definición. Marcar un significado y declarar una relación de disposición son la misma clase de sentencia.
- Un uso puede ser miembro de otro uso (anidamiento), como en los ejemplos de 9.4 y 10.1.
- Un rol de conjunto, escrito `"{name}"` en la lista de roles (como `"{members}"` de `$MATCHED`, y `"{ref}"` y `"{out}"` de `$CURRENT_MIRROR_P` en el ejemplo), toma un conjunto, para un número de miembros que no es fijo (9.4).
- Un rol puede declarar la clase de objeto que admite, como `roles=[["in_p", "instance"], ["in_n", "instance"]]`; un objeto de clase incorrecta hace fallar toda la aplicación (9.4). Una definición con un cuerpo vacío no comprueba nada de lo que rellena los roles, de modo que sin tales declaraciones no puede detectarse allí un objeto erróneo.
- Los nombres de rol de un uso son sus subobjetos, como `%dp1::left`.
- Aplicar una definición expande su cuerpo, y el cuerpo puede contener restricciones, como un dibujo por defecto con prioridad `"low"`, que la prioridad del uso desactiva (9.5, 14.8).
- No se comprueba: si el significado concuerda con las conexiones, los tipos o los parámetros del netlist no se comprueba (Principios 2.4). Por ejemplo, el lenguaje no comprueba para `#M3(ref) #M4(out) @AS $MY_MIRROR` si la puerta y el drenador de M3 están conectados; una definición comprueba lo que afirma mediante las aserciones de su cuerpo (15.3, 15.4).
- Los usos de un significado dado se seleccionan con SELECT-USAGE, y los usos que contienen un objeto dado con SELECT-ANCESTOR-USAGE; los miembros de usos se alcanzan mediante una selección cuyo ámbito son los usos (10.1).
- El vocabulario de definiciones semánticas pertenece al paquete estándar (9.5), que proporciona definiciones como `$DIFFERENTIAL_PAIR_N`.
- La validez básica de una descripción (Principios 2.4) es: sus referencias pueden encontrarse, sus roles reciben objetos de las clases correctas, sus roles simples están rellenos y cada sentencia a la que se expande es válida por sí misma (9.5). Si dos roles se rellenan con el mismo objeto no se comprueba.

### 15.2 Dirección de corriente: CURRENT

```rnl
#M5:1 @CURRENT("in");                 /* drain: the current flows in */
#M5:3 @CURRENT("out")                 /* source: the current flows out */
```

- CURRENT fija la dirección de la corriente estática en un terminal: `"in"` (entra), `"out"` (sale) o `"unspecified"` (usado para sustituir un valor dado por una regla de un paquete). El parámetro es `direction`. Una regla de CURRENT se evalúa en el paso 6 (10.4). Un CURRENT cuyo objeto es un pin, escrito así o alcanzado a través de un rol, necesita el símbolo para hallar su terminal (5.2.4): no es una regla, y surte efecto una vez asignados los símbolos de su objeto, en el paso 4, o en el paso 7 para un objeto generado en el paso 6. Hasta el paso 6, la condición `current` de las selecciones lee únicamente las sentencias CURRENT escritas sobre terminales; desde el paso 7 en adelante, como en la cola de estilos, lee todas las direcciones, y lo mismo hacen la geometría y la comprobación siguiente.
- Es un hecho sobre el terminal, no un hecho geométrico: no participa en conflictos (8.5), y un solucionador lo utiliza únicamente como pista, por ejemplo para el orden en que prueba orientaciones. Como ORIENT, es una sentencia modificadora y pasa adelante su manejador de entrada. Tiene una clave, una por cada terminal, y gana la posterior. Un terminal para el que nunca se escribe queda sin especificar.
- Se da únicamente en terminales. La corriente en el interior de un dispositivo con varios terminales no puede dividirse con claridad: para un dispositivo de tres terminales, "a fluye hacia b, b fluye hacia c" deja sin aclarar si la corriente en b entra o sale, mientras que la dirección de la corriente estática en un terminal tiene siempre un significado definido.
- Admite únicamente terminales del anfitrión, incluidos los puertos de este nivel: `#:inp @CURRENT("in")` significa que la corriente entra en el subcircuito allí. Un pin de un dispositivo se convierte en su terminal, como `#M1::d @CURRENT("in")`; los componentes virtuales no tienen terminales, por lo que sus pines no se convierten (5.2.4). Un terminal sin pin está permitido (a diferencia de los cables).
- No se comprueba frente al netlist (Principios 2.4). Las sentencias CURRENT de una misma red forman juntas una aserción (15.3): cuando cada terminal en la red, incluidos los puertos de este nivel, tiene un CURRENT de `"in"` o `"out"`, y todos mueven la corriente en el mismo sentido respecto a la red, se produce una violación (7.6) que nombra la red y sus terminales, puesto que la corriente no tiene adónde ir. Un terminal de dispositivo con `"in"` saca corriente de la red, y un puerto de este nivel con `"in"` introduce corriente en ella. Un terminal en la red sin CURRENT, o con `"unspecified"`, deja la red sin comprobar. Una red global (6.3), como `0`, tampoco se comprueba: es el mismo nodo en todos los niveles, de modo que la corriente puede salir por los terminales de otros niveles. Un CURRENT en un registro, o en el cuerpo de un uso, cuya prioridad es `"disabled"` no cuenta. Se comprueba con las aserciones, tras el paso 7 de 8.2, y como ellas es únicamente un informe, que nombra la red, sus terminales y sus sentencias CURRENT; los hechos CURRENT permanecen en vigor.
- Uso: la corriente en el terminal discurre en la dirección en que un cable sale del pin (9.2.4), girada con la orientación del símbolo, cuando sale, y en sentido contrario cuando entra. Un solucionador puede usar esto para elegir entre las disposiciones, como preferir que la corriente discurra hacia abajo (14.8); hacia arriba o hacia abajo es cuestión de estilo de la casa. Un terminal sin CURRENT, con `"unspecified"` o cuyo pin no escribe `direction=` (9.2.4) no da ninguna dirección; una dirección que una implementación infiera de los gráficos no cuenta aquí. Las reglas pueden seleccionar terminales por la dirección de su corriente, con la condición `current` de SELECT-TERMINAL (10.1).
- **Flechas**: CURRENT no dibuja nada. Si se dibujan flechas a partir de CURRENT y cómo es una preferencia del visor (parte de su estilo de la casa), igual que qué nombres de red se muestran (13.3). Un visor que las dibuja por sí mismo omite los terminales que ya tienen un uso de la flecha de corriente estándar, igual que reconoce el `$NO_CONNECT` estándar (12.6). Las flechas que deben conservarse (de un esquema importado o decididas por el autor) se escriben como objetos explícitos:

  ```rnl
  $CURRENT_ARROW @ATTACH #M5:1 => @ORIENT("R270") => @TEXT("I_{SS}", dx=1);   /* on the drain pin, with I_SS beside it */
  $CURRENT_ARROW @ATTACH (12, 20) => @ORIENT("R270")                           /* in the middle of a wire, at absolute coordinates */
  ```

  - Una flecha dibujada en un terminal es un uso cuya definición liga su punto al pin del terminal (12.1 punto 5). No gira con el pin, por lo que se escribe su orientación; el texto junto a ella se adjunta al uso (13.3).
  - Una flecha en medio de un cable no está anclada al cable: se dibuja en un punto, con coordenadas absolutas.
  - Cómo se distingue una flecha que apunta hacia dentro de una que apunta hacia fuera, y cómo se dibujan la definición y el símbolo de la flecha, pertenecen al paquete estándar. El símbolo puede ser simplemente una línea con una punta de flecha en su extremo (9.2.5).
  - Las flechas dibujadas se mantienen separadas de CURRENT: si fueran la misma cosa, exportar una flecha cambiaría aquello a lo que aspira la disposición.

### 15.3 Aserciones: ASSERT-INSTANCE, ASSERT-TERMINAL, ASSERT-NET, ASSERT-USAGE, ASSERT-IN

```rnl
$MATCHED_PAIR = @DEFINE(roles=[["left", "instance"], ["right", "instance"]]) {
  {?left ?right} @ASSERT-INSTANCE(same=["model", "w", "l"], different=["name"]);
  {?left ?right} @SELECT-TERMINAL(terminal="s") => @ASSERT-TERMINAL(same=["net"], message="the sources of a pair share a net")
};
# @SELECT-INSTANCE(name="MD*") => @ASSERT-INSTANCE(type="nmos");
%dp1 @ASSERT-USAGE $DIFFERENTIAL_PAIR_N;                      /* %dp1 is a usage of this definition */
#M5:s @SELECT-NET => @ASSERT-IN $GROUND_NET_SET              /* the source of M5 is on a ground net */
```

- Una aserción comprueba que lo que afirma un archivo o una definición concuerda con los hechos del circuito, como que las dos mitades de un par tienen el mismo modelo y tamaño, o que los elementos ficticios (dummies) son de un mismo tipo. Únicamente informa: no crea ningún hecho, no cambia nada en el dibujo y no participa en la resolución (8.5).
- **Una palabra clave por cada clase**, como para las selecciones (10.1): ASSERT-INSTANCE comprueba instancias (y agrupaciones, por su primer miembro, 11.8), ASSERT-TERMINAL terminales, ASSERT-NET redes, ASSERT-USAGE usos. La única ranura toma un conjunto de objetos de esa clase; un objeto único es un conjunto de uno. La clase de un objeto se conoce al leer el texto, y una clase incorrecta es un error allí (5.2.4), de modo que ninguna aserción comprueba clases.
- **Las condiciones** tienen los nombres y significados de las condiciones de la palabra clave de selección de la misma clase (10.1), como `type="nmos"`, `terminals=4`, `net="vss"` o `port=true`: cada miembro debe cumplir cada condición escrita.
- **`same=[…]`**: para cada atributo de la lista, todos los miembros tienen el mismo valor. **`different=[…]`**: para cada atributo de la lista, dos miembros cualesquiera tienen valores distintos. Los nombres de atributo son los de `${…}` (13.6): para instancias `name`, `value`, `model`, `type` y los nombres de sus parámetros; para terminales `net`; para redes `name`; para usos `definition`. Para terminales, redes y usos estos son todos: cualquier otro nombre es un valor de parámetro erróneo y la aserción falla (7.1). Para instancias los nombres de parámetros son abiertos, por lo que se admite cualquier nombre, y un atributo que un miembro no tiene cuenta como un valor propio, "ausente": los miembros que carecen de él son iguales entre sí y difieren de todo valor escrito. Así, `same=["nf"]` se cumple cuando ningún miembro escribe `nf`, y produce una violación cuando unos lo escriben y otros no; un proceso cuyos dispositivos no tienen ese parámetro no necesita una aserción propia. Una instancia sin tipo (10.1) carece de `type`. Los valores se comparan tal como los escribe el anfitrión, sin evaluarlos (no hay expresiones, 3.7).
- **`has=[…]`**, para ASSERT-INSTANCE: cada miembro tiene cada atributo de la lista, como `has=["w", "l"]`; un miembro que carece de alguno produce una violación. `same` no exige que un atributo esté escrito; `has` sí.
- **ASSERT-USAGE** toma una definición como último objeto, igual que SELECT-USAGE (5.6, 10.1); puede omitirse. Con ella, cada miembro debe ser un uso de esa definición, reconocida por aquello a lo que se resuelve su nombre (9.5), no por la grafía del nombre. Sus condiciones son `symbol` y `drawn` (10.1). `same=["definition"]` se cumple cuando todos los miembros son usos de una misma definición, y `different=["definition"]` cuando no hay dos que lo sean.
- **ASSERT-IN**: `X @ASSERT-IN S` comprueba que cada miembro de X es miembro del conjunto S, que se escribe al final, como en ADD-TO (5.2.2); S debe ser un conjunto y no se envuelve en uno. Si dos miembros son el mismo se decide por manejador (5.2). Los miembros pueden ser de cualquier clase, y S puede depender de selecciones (10.4). No tiene condiciones, únicamente `message`, y una violación nombra los miembros de X que S no contiene. Con una diferencia de conjuntos (5.2.2), indica también que un conjunto está vacío.
- `different=["name"]` comprueba que las instancias o redes son objetos distintos, como cuando dos roles no deben rellenarse con el mismo objeto; los terminales no tienen `name`, por lo que para ellos `name` es un valor de parámetro erróneo, tanto en `different` como en `same`.
- **El informe**: una comprobación que no se cumple produce una violación (7.6) que nombra los miembros y el atributo; **`message`** añade un texto a ella. Nada falla, y el dibujo es el mismo que sin la aserción: una aserción es únicamente un informe, para el autor y para herramientas como `rnl check`.
- **Dónde y cuándo**: en cualquier nivel, y en el cuerpo de una definición general, donde se comprueba para cada aplicación (9.4). Las aserciones se comprueban tras el paso 7 de 8.2, cuando toda condición puede leerse, también `drawn` y `symbol` (10.1). Un conjunto sin miembros se cumple trivialmente, y también una aserción que recibe el nodo vacío (6.3).
- Una aserción no tiene prioridad; en un registro con `priority="disabled"`, y en el cuerpo de un uso cuya prioridad es `"disabled"` (14.8), no se comprueba.

### 15.4 Aserciones de topología: ASSERT-TOPOLOGY

```rnl
{#R1 #R2 #R3} @ASSERT-TOPOLOGY("series");                  /* a chain: R1 to R2 to R3 */
{#M3 #M4} @ASSERT-TOPOLOGY("parallel", exclude=[["nmos", "g"], ["nmos", "b"]]);   /* drains on one net, sources on one net */
{#L1 #C1 #L2} @ASSERT-TOPOLOGY("star");                    /* a T network: three branches meet at one net */
{#C1 #L1 #C2} @ASSERT-TOPOLOGY("loop");                    /* a pi network: C1, L1 and C2 go round */
{%branch1 %branch2} @ASSERT-TOPOLOGY("parallel")           /* two branches, each a usage or a set taken as a black box, between the same nets */
```

- ASSERT-TOPOLOGY comprueba que unos miembros están conectados de una manera determinada: en serie, en paralelo, en estrella o como un único bucle. Es una aserción como las de 15.3, y lo que 15.3 dice de ellas rige también para esta: únicamente informa, una comprobación que no se cumple produce una violación (7.6), a la que `message` añade un texto, y dónde, cuándo y bajo qué prioridad se comprueba es lo mismo.
- **Los miembros**: la única ranura toma un conjunto de miembros, en orden, contados por elementos (5.2): un objeto es un miembro, y un elemento que es un `{…}` o una referencia a un conjunto es un miembro, una caja negra. Un conjunto que es el único objeto se divide un nivel primero (5.2.1), de modo que `%chain @ASSERT-TOPOLOGY("series")` comprueba los elementos de la cadena, y `{%branch1 %branch2}` tiene dos miembros. Un miembro es así una instancia (o una agrupación, por su primer miembro, 11.8), un uso o un conjunto. Dado el resultado de una selección, que tiene un elemento por cada miembro, los miembros están en el orden del resultado (10.3). Con menos de dos miembros la comprobación se cumple trivialmente.
- **Las redes de un miembro**:
  - una instancia: las redes en las que están aquellos de sus terminales que `exclude` no excluye;
  - un uso o un conjunto: una caja negra, un único elemento formado por los terminales que aporta. Un uso, tenga símbolo o no, representa lo que rellena sus roles, y un conjunto sus miembros, seguidos hacia dentro hasta los más internos; una instancia aporta todos sus terminales, un terminal solo a sí mismo, un pin los terminales que vincula, y las redes y puntos no aportan nada; una agrupación cuenta por su primer miembro. Las redes de la caja negra son aquellas en las que están los terminales que aporta que `exclude` no excluye y que también se usan fuera de ella: por un terminal que no aporta, incluidos los de los demás miembros de una agrupación, o como puerto del nivel. Una red usada únicamente por los terminales que aporta no cuenta. Nota: una red que comparten dos miembros se usa fuera de cada uno de ellos por el otro, de modo que el que una red sea un puerto cambia un veredicto solo para miembros que aportan el mismo terminal; en los demás casos solo cambia cuáles son las redes de la caja negra, tal como una herramienta puede mostrarlas.
- **`exclude=[[type, terminal], …]`**: terminales que no cuentan, como pares de un tipo, comparado como la condición `type` (10.1), y un nombre o número de terminal, como `exclude=[["nmos", "g"], ["nmos", "b"]]`. El terminal de un par se compara como la condición `terminal` de SELECT-TERMINAL (10.1): un número por lugar, un nombre según las reglas de mayúsculas y minúsculas del anfitrión; un nombre que la instancia no tiene no coincide con nada. Un terminal de una instancia sin tipo, o de un tipo que la lista no nombra, cuenta. Sin `exclude`, cuenta todo terminal. Un par que no sea una matriz de dos cadenas, o una cadena y un número entero, es un valor de parámetro erróneo y la aserción falla (7.1).
- **Las topologías**, el primer parámetro, dado por posición, una de cuatro:
  - `"series"`: cada dos miembros contiguos en el orden comparten exactamente una red, y dos miembros no contiguos no comparten ninguna. Qué más haya en una red compartida no se comprueba: el centro de un divisor puede conducir a otro sitio.
  - `"parallel"`: al menos dos redes, los extremos, son compartidas por todos los miembros. Otras redes de un miembro, como una toma que saca una rama, no se comprueban, igual que no se comprueba nada más en una red compartida para `"series"`.
  - `"star"`: una red es compartida por todos los miembros, y ningún par de miembros comparte ninguna otra red.
  - `"loop"`: al menos tres miembros. Dando la vuelta en orden, incluidos el último y el primero, cada dos contiguos comparten exactamente una red, todas estas redes difieren entre sí, y dos miembros no contiguos no comparten ninguna. Con dos miembros, `"loop"` produce una violación; escríbase `"parallel"`.

<!-- translated from en/ch16.md sha256:fa0479c7afe6 -->

## 16 Archivos, versiones, paquetes y alias

Este capítulo trata la versión del lenguaje de un archivo, los paquetes y archivos de paquete, el paquete estándar, el contenido desconocido y los alias.

### 16.1 Versiones

```rnl
* @RNL(type="header") { @VERSION("0.4")}
```

- **Una sola versión del lenguaje por archivo.** El RNL de versiones distintas se escribe en archivos distintos, unidos mediante inclusión (6.1).
- **Declarada en el registro de encabezado.** VERSION puede escribirse únicamente en el registro de encabezado (`type="header"`), que es el primer registro del archivo (2.8). La versión puede declararse una sola vez, por lo que un archivo nunca tiene dos valores de versión. Un VERSION escrito en cualquier otro lugar es un error y se ignora.
- **Orden de lectura:**
  1. el adaptador del anfitrión proporciona los comentarios;
  2. todos los registros se recortan según su forma externa y se leen sus encabezados del registro; este paso no depende de la versión;
  3. cuando el primer registro es el registro de encabezado, se lee su VERSION;
  4. los demás registros se analizan en esa versión.
- **Partes que no dependen de la versión.** Los portadores de comentarios, la entrada `@RNL`, la forma del encabezado del registro y la forma de VERSION en el registro de encabezado no cambian en ninguna versión posterior; de otro modo un analizador no podría encontrar la versión. La compatibilidad con un nuevo anfitrión únicamente añade portadores y no cambia los existentes (2.2). El resto del registro de encabezado se lee en la versión declarada.
- **Sin registro de encabezado.** El registro de encabezado puede omitirse, y también VERSION en él. Un archivo incluido toma entonces la versión del archivo que lo incluye; un archivo principal toma la versión por defecto de la herramienta, con una indicación. Las herramientas deben escribir el registro de encabezado y la versión cuando guardan un archivo, de modo que los archivos escritos a mano pueden omitirlos y los archivos escritos por herramientas los llevan. Un archivo de paquete debe tener un registro de encabezado (16.2); sin VERSION toma la versión por defecto de la herramienta, igual que un archivo principal. Un paquete en línea tiene la versión del archivo en el que está escrito (16.2).
- **Versión no admitida.** Cuando una herramienta no admite la versión de un archivo, no se interpreta nada del RNL de ese archivo, se informa de un error y el archivo se conserva tal cual al guardarlo; los demás archivos se interpretan como de costumbre.
- Las implementaciones deben declarar las versiones que admiten (Principios 2.3). La versión del lenguaje, la versión de un paquete (`version=` de USE-PACKAGE) y la versión del anfitrión son tres cosas distintas. Cómo se escriben y emparejan las versiones de paquete está en 16.2. Un archivo de esta versión utiliza únicamente paquetes de esta versión: un archivo de paquete o un paquete de alias que declara otra versión del lenguaje, o que no declara ninguna y por ello se lee en una anterior, es un error en el USE-PACKAGE o USE-ALIASES que lo alcanza, directamente o mediante otros paquetes (16.2, 16.5).
- **Números de versión.** Una versión se escribe `"major.minor"`: dos enteros decimales sin signo, como `"1.0"` o `"1.2"`. No hay números de parche, fechas ni perfiles. Las versiones anteriores a `"1.0"` son borradores, escritos `"0.minor"`; las versiones 0.x no prometen compatibilidad entre sí, y cada una cuenta como una versión principal propia. Una versión mal formada, como `"1"` o `"v1.2"`, se trata como una versión no admitida, no como una versión ausente, para que el archivo no se lea erróneamente en otra versión.
- **Un archivo se lee en la versión que declara.** Las herramientas deben saber en qué versión se añadió cada palabra estándar: en un archivo que declara una versión anterior, una palabra añadida después no es una palabra estándar. Así, un nombre estándar de subobjeto añadido después no afecta a una clave de igual grafía en un archivo anterior (6.5), un nuevo nombre de atributo no afecta a un parámetro del anfitrión del mismo nombre (13.6) y un nuevo parámetro estándar de AS y ATTACH no afecta a un parámetro de definición del mismo nombre (9.4).
- **Dentro de una misma versión principal, una nueva versión menor únicamente añade y no cambia el significado.**
  - Permitido: añadir palabras clave, parámetros, valores, palabras estándar, nombres de subobjeto y nombres de atributo; dar significado a una forma que antes fallaba. Los elementos del Anexo C, y todo lo que este estándar denomina una relajación pura, son de esta clase, y también lo es volver tolerantes los casos en los que falla toda la interpretación (1.5 punto 3).
  - No permitido sin una nueva versión principal: cambiar el significado de una forma válida; eliminar algo; cambiar un valor por defecto; convertir una forma válida en un error.
  - Un archivo antiguo conserva su significado en una herramienta más nueva. En una herramienta más antigua, las partes de un archivo más nuevo que la versión anterior tiene se leen como de costumbre, las partes nuevas son contenido desconocido (16.4) y una advertencia indica que la versión del archivo es más nueva de lo que admite la herramienta. La herramienta también puede optar por no dibujar el archivo, pero no debe fingir que lo entiende.
- **Una versión principal distinta.** Un archivo de una versión principal que la herramienta no admite no se interpreta y se conserva tal cual (véase arriba). Sus paquetes tampoco se usan: no se interpreta nada del RNL del archivo, y un visor dibuja el netlist con su propia apariencia por defecto e informa de que la versión no está admitida. Una herramienta puede admitir varias versiones principales. Pasar un archivo a una nueva versión principal es una migración explícita, en la que la herramienta reescribe el archivo y su registro de encabezado; una herramienta no debe leer en silencio un archivo antiguo según las reglas de una nueva versión principal. Cada nueva versión principal de este estándar viene acompañada de notas de migración.
- **Versiones admitidas.** Una herramienta declara, para cada versión principal, hasta qué versión menor admite, como `"1.0"`–`"1.3"`. Admitir `"1.3"` significa poder leer también de `"1.0"` a `"1.2"`.
- **Subida de versión.** Cuando una herramienta escribe en un archivo una forma que solo tiene una versión más nueva, sube la versión en el registro de encabezado. Antes de eso, comprueba si las palabras estándar añadidas entretanto tienen la grafía de nombres ya presentes en el archivo (claves de textos, nombres de rol, parámetros de definición, parámetros del anfitrión) y emite una advertencia para cada nombre así, pues subir la versión puede cambiar su significado; la versión se sube de todos modos.
- **Versiones 0.3 y 0.2.** Esta versión reemplaza a la versión 0.3. Los archivos de las versiones 0.3 y 0.2 son archivos de otras versiones principales (arriba), que este estándar no describe. Se pasan a 0.4 con una herramienta de actualización que lee la versión 0.3: sube la versión, pasa el archivo a la serie estándar 0.4 y escribe `@USE-PACKAGE(name="rnl-std-all", version="0.4")` en su registro de encabezado en lugar de cualquier versión del paquete estándar fijada allí, reescribe el archivo según indica el Anexo D e informa de lo que no puede reescribir, como un uso de una definición que la serie 0.4 ya no tiene.

### 16.2 Paquetes y USE-PACKAGE

```rnl fragment
@USE-PACKAGE(name="acme-style", version="1.2");                               /* looked up in the package directories of the tool */
@USE-PACKAGE(name="acme-style", version="1.2", path="lib/acme-style.rnl");    /* reads this package file */
@USE-PACKAGE(name="acme-style", version="1.2") {…}                            /* an inline package */
```

**Dónde surte efecto un paquete.**

- Las definiciones `$` de un paquete surten efecto en el nivel donde está escrito su USE-PACKAGE y en los niveles inferiores. Sus reglas cuentan como escritas en ese nivel: se aplican allí, y en los niveles inferiores únicamente cuando los alcanzan, mediante `recursive=true` en una selección escrita en ellas o, si no tienen ninguna, mediante los conjuntos que usan (10.1, 10.4). Esta versión no tiene ningún USE-PACKAGE que se aplique únicamente a su propio nivel. Escrito en un subcircuito, un paquete se aplica únicamente a ese subcircuito; escrito en un nivel de archivo, a los subcircuitos del archivo y a los archivos que incluye (6.1); escrito en el registro de encabezado, a los mismos niveles que en el nivel de archivo, pero situado en la raíz (16.3).
- Los paquetes en la raíz, ya estén escritos en un registro de encabezado o añadidos por un ajuste de la herramienta (16.3), cuentan como usados en el nivel superior del archivo: en el archivo principal, su nivel superior. Así, las reglas de un paquete de estilo de la casa usado en un registro de encabezado alcanzan los subcircuitos únicamente con `recursive=true`, tal como las escribe el paquete estándar.
- **Lugar en el orden.** El contenido de un paquete se sitúa en la posición de su USE-PACKAGE, y dentro del paquete en el orden de su texto, igual que los hechos expandidos de una definición toman la posición de la sentencia AS o ATTACH. La resolución de conflictos (8.5) y las colas de apariencia (8.3) usan esta posición. Un paquete usado antes de las sentencias de un nivel actúa como un conjunto de valores por defecto; usado después de ellas, sus reglas pueden sustituir a sentencias anteriores. Un USE-PACKAGE en el registro de encabezado se sitúa en la raíz (16.3).

**Nombres en un paquete.** Únicamente el lugar sigue al USE-PACKAGE; los nombres de un paquete siguen siendo propios.

- Un nombre en un paquete se busca primero entre las definiciones del propio paquete, luego en los paquetes que referencia (véase "Paquetes que referencian paquetes" abajo), luego en el paquete estándar en vigor en el nivel del USE-PACKAGE. No se busca entre las definiciones del usuario de ese nivel ni de los niveles superiores, ni en otros paquetes; como en el cuerpo de una definición (6.9), una definición del mismo nombre en el nivel del USE-PACKAGE no lo captura. Así, los nombres de un paquete en línea se encuentran como los de un archivo de paquete con el mismo contenido, y su texto puede trasladarse a tal archivo; y un archivo de paquete puede comprobarse por separado, con el paquete estándar que añade un ajuste de la herramienta (16.3). El paquete estándar y los paquetes que referencia, directamente o no, ven únicamente a sí mismos y a los paquetes que referencian, no al paquete estándar, de modo que una búsqueda nunca vuelve a ellos.
- Las definiciones `$` de un paquete son públicas y cuentan como pertenecientes al nivel inmediatamente superior al nivel del USE-PACKAGE: una definición del mismo nombre en ese nivel oculta la del paquete y produce una indicación (6.8); no es una segunda vinculación en un mismo nivel. Cuando las definiciones `$` de un paquete ocultan definiciones de los niveles superiores, o de los paquetes en la raíz, también se emite una indicación (6.8).
- Las definiciones `%` de un paquete son privadas al paquete.

**Qué contiene un paquete.**

- Un paquete contiene definiciones, conjuntos `$` y reglas. El registro de encabezado de un archivo de paquete contiene VERSION y PACKAGE, y puede contener también USE-PACKAGE, DEVICE (6.12) y SOLVER-CONFIG (14.10), que se aplican a los niveles que usan el paquete, y ALIAS y USE-ALIASES, que se aplican únicamente a ese archivo (16.5). USE-PACKAGE referencia otros paquetes (abajo), y también puede figurar en un registro de encabezado para un lenguaje (2.8). Un paquete de alias es un archivo de paquete con únicamente un registro de encabezado, cargado con USE-ALIASES (16.5).
- Un paquete no tiene circuito propio, como el nivel de archivo de un archivo incluido (6.1):
  - las sentencias que nombran objetos particulares del anfitrión (`#M1`, `&vss`, `#:inp`) fallan; un `#` aislado sigue siendo el nivel en el que se evalúa una regla, también en el cuerpo de una definición general (9.4);
  - una sentencia que no es una regla y dibuja un solo objeto de la escena, como `(0, 0) @RECTANGLE …`, no tiene nivel en el que dibujarse y es un error;
  - `scope=` y `type="generated"` en un encabezado del registro son errores (2.8);
  - USE-PACKAGE en un registro de un paquete distinto de su registro de encabezado es un error: las referencias a otros paquetes se escriben únicamente en el registro de encabezado.
- El cuerpo de un paquete en línea sigue las mismas reglas pero no tiene registro de encabezado: el nombre y la versión del paquete son los parámetros de su USE-PACKAGE, y DEVICE y SOLVER-CONFIG se escriben en el registro de encabezado del propio archivo. Un paquete en línea no puede referenciar otros paquetes; para congelar una colección de paquetes en un archivo, su contenido expandido se escribe en el cuerpo en orden, con los cambios que conservan su significado (16.3).

**Archivos de paquete.** Cómo se lee un archivo de paquete se indica en 2.7, con un ejemplo. Su registro de encabezado declara con `@PACKAGE(name=…, version=…)` qué paquete y qué versión es el archivo; un archivo sin PACKAGE no es un paquete, y un USE-PACKAGE de él es un error. Un paquete es un archivo: un paquete dividido en varios archivos necesitaría un orden entre ellos, que esta versión no proporciona. Un archivo de paquete declara su versión del lenguaje con VERSION, que debe ser la versión del lenguaje de los archivos que lo usan (16.1); de lo contrario el USE-PACKAGE es un error y las sentencias que usan los nombres del paquete fallan como nombres no encontrados.

**Búsqueda.**

- Un USE-PACKAGE con cuerpo es un paquete en línea y no se busca.
- Con `path=`, se lee ese archivo. El camino es relativo al directorio del archivo en el que está escrito el USE-PACKAGE, con `/` como separador; nombra un archivo local, no una URL. Cuando el archivo no existe, o su PACKAGE nombra otro paquete, o una versión que la regla siguiente no admite, el paquete se busca por nombre, con una advertencia. Cuando el archivo existe pero no es un paquete, sin PACKAGE o con un PACKAGE cuyo `version=` está mal escrito, el USE-PACKAGE falla, como para cualquier archivo que no es un paquete (arriba).
- De otro modo, el paquete se busca por el nombre y la versión de su PACKAGE en los directorios de paquetes fijados en la herramienta. El paquete estándar viene con la herramienta o se instala en un directorio de paquetes. Los directorios de paquetes son un ajuste de la herramienta y no se escriben en el archivo; cuando hay varias copias de la misma versión, decide el orden de los directorios en ese ajuste.
- Los diagnósticos indican qué archivo y qué versión se utilizaron.
- Cuando no se encuentra el paquete, el USE-PACKAGE falla (un error), y las sentencias que usan nombres del paquete fallan con él (7.3); el diagnóstico indica el paquete ausente como causa.
- `name=` es obligatorio. Se compara como cadena completa y distingue entre mayúsculas y minúsculas; los nombres de paquete deberían usar letras minúsculas, dígitos y `-`.
- Con `path=`, un netlist y sus archivos de paquete pueden mantenerse juntos y moverse en bloque; esta versión no tiene formato de manifiesto para ellos.

**Versiones de paquete.**

- Una versión de paquete se escribe como una versión del lenguaje, `"major.minor"`; cada 0.x cuenta como una versión principal propia (16.1).
- Cuando la versión escrita existe, se usa. De lo contrario se usa la menor de las versiones más nuevas dentro de la misma versión principal, con una advertencia de que la versión del paquete difiere y el dibujo puede diferir; cuando la versión principal tiene únicamente versiones más antiguas, o ninguna, no se encuentra el paquete. No se usa una versión más antigua, porque pueden faltarle nombres. Un archivo nombrado con `path=` se comprueba mediante la misma regla.
- Sin `version=`, se usa la versión más nueva encontrada, con una indicación. Un `version=` que no está escrito como indica 16.1, en USE-PACKAGE, USE-ALIASES o PACKAGE, es un error y no se busca: ese USE-PACKAGE o USE-ALIASES falla, y un archivo de paquete cuyo PACKAGE lo lleva no es un paquete (arriba). El `version=` de un paquete en línea únicamente identifica la versión y no se busca; escrito erróneamente, es también un error y ese paquete en línea falla.
- Un paquete encontrado por nombre y versión se lee una sola vez en un documento: la misma versión es el mismo conjunto de definiciones dondequiera que se use. Un paquete encuentra los nombres de los paquetes que referencia, y del paquete estándar en vigor, allí donde se usa: su versión y contenido en línea se deciden como sigue abajo: por el USE-PACKAGE más cercano al nivel para un paquete que usa un archivo, y por la versión más nueva en la cadena de niveles para uno incorporado únicamente a través de otros paquetes. Cuando estos difieren entre dos lugares donde se usa el paquete, se lee una vez para cada uno, y las lecturas son conjuntos distintos de definiciones, como lo son dos versiones (9.5). Cada paquete en línea es una copia propia: dos paquetes en línea son distintos incluso cuando su texto es el mismo, por lo que un paquete que los encuentra en dos lugares se lee una vez para cada uno. Las definiciones del mismo nombre en versiones distintas son dos definiciones (9.5); véase el ejemplo en 16.3.
- Los autores de paquetes deberían seguir la regla del lenguaje (16.1): dentro de una misma versión principal, no eliminar ni cambiar nombres, roles, parámetros o pines existentes; el dibujo puede cambiar. Esta es una convención que las herramientas no pueden comprobar.
- Esta versión no tiene resumen de contenido para paquetes (como un parámetro `digest=` de USE-PACKAGE). Para mantener un dibujo exactamente igual, fíjese la versión o escríbase el paquete en línea en el archivo. Cómo se calcula el `hash` de un encabezado del registro depende de la herramienta (capítulo 17).

**Paquetes que referencian paquetes.** Un paquete puede referenciar otros paquetes, de modo que un paquete grande pueda dividirse en partes y ensamblarse de nuevo en varios tamaños; el paquete estándar se construye así (16.3). Un paquete cuyo archivo tiene únicamente un registro de encabezado y únicamente referencia otros paquetes se denomina *colección de paquetes*; este estándar suele llamarlo simplemente paquete.

- **Forma.** En el registro de encabezado de un archivo de paquete, escríbase `@USE-PACKAGE(name=…, version=…)` para cada paquete referenciado. Un `path=` es relativo al directorio de ese archivo de paquete; la búsqueda y las versiones son como arriba. Una referencia no tiene cuerpo; una con cuerpo es un error, porque el contenido en línea no tiene versión y no puede participar en la elección de la versión más nueva (abajo).
- **Orden.** Cuando se usa un paquete, los paquetes que referencia su registro de encabezado se expanden primero, en el orden de las referencias, y se sitúan antes que él; un paquete referenciado que referencia otros se expande del mismo modo primero. Luego viene el contenido del propio paquete. Dentro de una expansión cada paquete se sitúa una vez, en su primer lugar. Así, el contenido de una colección de paquetes es el contenido de los paquetes que referencia, uno tras otro.
- **Nombres.** Los nombres `$` de un paquete referenciado son visibles en el paquete que lo referencia, y se transmiten a sus usuarios; como los nombres `$` del propio paquete, cuentan como pertenecientes al nivel inmediatamente superior al nivel del USE-PACKAGE. Cuando varios paquetes referenciados definen `$X`, la referencia posterior oculta a la anterior, y una definición del propio paquete las oculta a todas; cada ocultación tal produce una indicación, como para varios paquetes usados en un nivel. Esta versión no distingue las referencias para uso propio del paquete de las referencias transmitidas a sus usuarios. Los nombres `%` permanecen privados en cada paquete.
- **Uno solo en vigor.** Los paquetes se cuentan por nombre, como en "Qué USE-PACKAGE surte efecto" abajo, incluidos los incorporados a través de otros paquetes, en el lugar donde se expanden.
  - Para un paquete que un archivo usa directamente, surte efecto el USE-PACKAGE escrito en un archivo más cercano al nivel, como en "Qué USE-PACKAGE surte efecto" abajo, con su versión y contenido en línea, también donde otros paquetes referencian el paquete. Cuando su versión principal difiere de la que pide un paquete que lo referencia, se emite una advertencia: el paquete que lo referencia puede usar nombres que no tiene.
  - Para un paquete que se incorpora únicamente a través de otros paquetes, se usa la versión más nueva dentro de una misma versión principal entre todas las referencias en una cadena de niveles, con una indicación cuando las versiones difieren; dentro de una versión principal una versión más nueva únicamente añade, por lo que sirve también a las referencias más antiguas. Las referencias de versiones principales distintas son un error: la referencia expandida después falla, y las sentencias que usan nombres de ella fallan con ella (7.3).
- **Reemplazar o descartar uno de ellos** se sigue de "uno solo en vigor"; no hay ninguna regla adicional. Un paquete en línea vacío del mismo nombre lo descarta, como `@USE-PACKAGE(name="rnl-std-fill") {}`, que desactiva el relleno automático; otra versión u otro contenido en línea bajo el mismo nombre lo reemplaza. Cuando un paquete descartado es usado por otros paquetes, las sentencias en ellos que usan sus nombres fallan.
- **Ciclos.** Las referencias que forman un ciclo son un error, y la referencia que cierra el ciclo falla. Qué referencia sea esa depende del lugar donde comienza la expansión; cuando varios paquetes de un ciclo se usan directamente, el ciclo se señala una vez para cada uno de ellos.
- **Dónde se señalan los diagnósticos.** La indicación sobre versiones distintas y la advertencia sobre una versión principal distinta se señalan en el USE-PACKAGE escrito en el archivo (las indicaciones y advertencias dentro de paquetes no se muestran al usuario, 7.6). El error de una referencia que falla se señala en esa referencia e indica qué USE-PACKAGE del archivo la incorporó.
- Un paquete de alias no puede referenciarse (16.5). USE-ALIASES en el registro de encabezado de un archivo de paquete se aplica únicamente a ese archivo, como de costumbre.

**El mismo nombre `$` en varios paquetes.**

- Cuando varios paquetes usados en un nivel definen `$X`, el usado después oculta al anterior, con una indicación.
- Un nombre se busca en este orden: este nivel → los paquetes usados en este nivel (primero el posterior) → el nivel superior → los paquetes usados en el nivel superior → … → los paquetes en la raíz (primero el situado después).
- Esta versión no tiene nombres cualificados por paquete, otros nombres para paquetes ni etiquetas de nivel (como `$acme.DIFFERENTIAL_PAIR` o `as=`). Nota: una versión posterior que los añada necesita una forma distinta de un `.` tras el nombre, que es habitual al final de una frase (`align #M1 and #M2.`).

**Qué USE-PACKAGE surte efecto.** En la cadena de niveles desde la raíz hasta un nivel, un paquete está en vigor una sola vez. Surte efecto el USE-PACKAGE escrito en el nivel más cercano a este nivel, con su versión y contenido en línea; dentro de un mismo nivel cuenta como más cercano el posterior, y uno en el registro de encabezado cuenta como escrito al principio mismo del nivel de archivo. Un paquete incorporado a través de otros paquetes cuenta como escrito en el USE-PACKAGE que lo incorpora, con su versión elegida como en "Paquetes que referencian paquetes" arriba. El paquete se sitúa en la posición de ese USE-PACKAGE, o en la raíz cuando ese USE-PACKAGE está en el registro de encabezado.

Nota: de otro modo, las reglas generadoras de un paquete, como el relleno de tierra, volverían a ejecutarse en un nivel inferior y generarían sus objetos dos veces.

**Volver a usar un paquete.** Un subcircuito o un archivo de biblioteca puede usar de nuevo el paquete estándar, en su nivel de archivo o en un subcircuito. Las reglas de apariencia del paquete estándar se sitúan entonces después de las reglas que descienden de los niveles superiores (como las del archivo principal) y vuelven a sustituirlas, y los nombres `$` del paquete, contados como pertenecientes al nivel inmediatamente superior a ese USE-PACKAGE, ocultan las definiciones del mismo nombre procedentes de los niveles superiores. Los símbolos usados en este nivel dejan entonces de verse afectados por las reglas de apariencia de los niveles superiores (6.1).

- Esto afecta únicamente a símbolos y nombres. Las reglas geométricas y generadoras que descienden de los niveles superiores siguen aplicándose; cuando una solución queda desactualizada por su causa, se emite una indicación para resolver de nuevo (capítulo 17).
- Para fijar únicamente la versión, y dejar aun así que el estilo de los niveles superiores sustituya al paquete, escríbase el USE-PACKAGE en el registro de encabezado (16.3).

**Paquetes en línea.** USE-PACKAGE puede tener un cuerpo, que es el contenido del paquete. El archivo transporta entonces el paquete y no se busca nada:

```rnl
@USE-PACKAGE(name="acme-style", version="1.2") {
  $NMOS_ACME = @DEFINE-SYMBOL {$NMOS; (-1, 0) @CIRCLE(radius=2.5)};
  # @SELECT-INSTANCE(type="nmos", recursive=true) => @SET-SYMBOL $NMOS_ACME
}
```

- Un paquete en línea significa lo mismo que un USE-PACKAGE sin cuerpo: el nombre y la versión del paquete, su lugar en el USE-PACKAGE, la búsqueda de sus nombres como en "Nombres en un paquete", sus nombres `$` contados como el nivel superior, sus nombres `%` privados y "uno solo en vigor" contado por nombre de paquete se cumplen todos como arriba. El cuerpo se consume por encerramiento (5.4). Un paquete en línea tiene la versión del lenguaje del archivo en el que está escrito (16.1).
- Un paquete en línea difiere de un archivo de paquete únicamente en que su contenido está escrito en el netlist y en que no tiene registro de encabezado: su versión del lenguaje es la del archivo, su `version=` únicamente identifica la versión, no puede declarar DEVICE, SOLVER-CONFIG ni ALIAS, y no puede referenciar otros paquetes. Cada paquete en línea es una copia propia. Por tanto, sacar fuera un paquete en línea conserva cómo se buscan sus nombres, pero no siempre qué definiciones son cuáles: donde ya se usa un archivo de paquete del mismo nombre y versión, ambos pasan a ser un mismo conjunto de definiciones (arriba), y las reglas que seleccionan por definición, como SELECT-USAGE (9.5), pueden alcanzar entonces más usos.
- Los paquetes en línea se usan normalmente para dos cosas únicamente: para descartar un paquete, con un paquete en línea vacío del mismo nombre (16.3), y para congelar todo un conjunto de paquetes en el archivo, con un cuerpo que contiene su contenido expandido completo y es completo en sí mismo.
- Para extender el paquete estándar, o para reemplazar una parte de él, escríbase un archivo de paquete cuyo registro de encabezado referencie los paquetes estándar que usa (véase "Paquetes que referencian paquetes" arriba), y apúntese a él desde el netlist con `path=`. Un paquete en línea con contenido que reemplaza un subpaquete del paquete estándar cuenta como parte del paquete estándar y solo se ve a sí mismo: no puede usar nombres de los demás subpaquetes del paquete estándar.

**Otras reglas.**

- Procesar un paquete no debe ejecutar código ni acceder a la red implícitamente (Principios 2.7).
- Cómo muestra un visor un dibujo, y si puede cambiar la apariencia según las preferencias del usuario (como un fondo oscuro o la impresión), queda fuera de este estándar. Una propiedad a la que ningún estilo ni parámetro da valor toma el valor por defecto del renderizador (9.3, Principios 2.7).

### 16.3 El paquete estándar

- El paquete estándar no forma parte de este estándar. Está escrito en RNL como cualquier otro paquete y se publica por separado; sus definiciones semánticas, componentes virtuales y demás contenido los define el propio paquete.
- Los nombres del paquete estándar no tienen ningún prefijo obligatorio, como `$DIFFERENTIAL_PAIR_N`. Un usuario puede escribir una definición del mismo nombre, que reemplaza a la estándar ocultándola (6.8).
- **Una colección de paquetes.** El paquete estándar `rnl-std-all` tiene únicamente registros de encabezado: es la única colección de la serie y referencia todos los demás paquetes de ella salvo los dos opcionales nombrados abajo. Un archivo usa toda la colección, o escribe los paquetes que desea uno por uno.
  - `rnl-std-style` (estilos y las reglas de estilo por defecto), `rnl-std-basic` y `rnl-std-semiconductors` (los símbolos y las definiciones, incluidos los componentes virtuales, como tierras, alimentaciones y etiquetas de puerto, y los conjuntos de redes de tierra, alimentación y polarización), y los paquetes de reglas de ambos dialectos, que asignan símbolos a las instancias: `rnl-std-spice` por letra de elemento y tipo de modelo, `rnl-std-spectre` por maestro y tipo de modelo (véanse los dialectos abajo). Estos dibujan el netlist y no añaden nada. El registro de encabezado de `rnl-std-basic` contiene `@SOLVER-CONFIG(junction="$JUNCTION")`, para que el solucionador dibuje sus puntos de unión con la definición estándar (13.2, 14.10).
  - `rnl-std-fill` contiene las reglas que añaden tierras, alimentaciones, marcas de polarización y etiquetas de red a los pines sin cables, cada uno sobre un tramo corto (10.5); referencia a `rnl-std-basic`. Selecciona pines, por lo que los pines que tienen los componentes virtuales en una red también se rellenan, todos salvo los de los componentes que él mismo coloca. En la serie 0.4 da a cada red de puerto de `$PORT_FILL_SET` una etiqueta de puerto, en una columna por cada nivel (12.4); ninguna red se dibuja como cables salvo que un archivo o un paquete lo indique con ROUTE-NET o ROUTE-OBJECT, las redes de tierra, alimentación y polarización se excluyen con `@UNROUTE-NET(priority="high")`, y un archivo que desea que se dibujen como cables escribe `@ROUTE-NET(priority="high")` o WIRE (13.7). Los pines con cables, escritos a mano o dibujados por ROUTE-NET o ROUTE-OBJECT, escritos como sentencias o como reglas, no reciben símbolo de tierra ni de alimentación del relleno; un archivo que desea uno lo adjunta (10.5). Las reglas de las palabras clave de trazado se evalúan antes del relleno, que ve los árboles que crean (8.2, 10.2). Los conjuntos `$GROUND_FILL_SET`, `$SUPPLY_FILL_SET`, `$BIAS_FILL_SET`, `$LABEL_FILL_SET` y `$PORT_FILL_SET` contienen lo que el relleno aún ha de añadir: una vez que una resolución escribe de vuelta sus tramos cortos y etiquetas (14.8), ya no contienen esos pines y redes, por lo que un archivo no construye ninguna otra regla sobre ellos, como un TEXT sobre sus pines.
  - `rnl-std-semantics` contiene las definiciones de estructuras de circuito, como el par diferencial, el espejo de corriente y los flujos de señal y de potencia, y de disposiciones, como `$MIRRORED_LAYOUT`; las restricciones en sus cuerpos tienen todas prioridad `"low"` (9.5, 14.8). Una definición que únicamente coloca, sin aserción ni trazado, no dice nada sobre el circuito, y su nombre termina en `_LAYOUT`, como `$MIRRORED_LAYOUT`. `_N` y `_P` al final del nombre de una estructura, como en `$DIFFERENTIAL_PAIR_N` y `$CURRENT_MIRROR_P`, dan la polaridad de sus transistores, que la estructura afirma para aquellos cuyo tipo es conocido (15.3), y el dibujo que la acompaña, fuentes o emisores hacia abajo para el tipo n y hacia arriba para el tipo p.
  - Los paquetes de ámbitos específicos son `rnl-std-functional` (amplificadores, comparadores, convertidores y similares), `rnl-std-rf`, `rnl-std-logic` y `rnl-std-signal-flow`.
  - Los nombres de los paquetes no se repiten, de modo que un usuario no necesita saber de qué paquete proviene un nombre.
  - El paquete opcional `rnl-std-45deg` contiene símbolos dibujados a 45 grados, como para puentes, pares acoplados en cruz y devanados en diagonal; no está en la colección, y un archivo que lo desea escribe su USE-PACKAGE.
  - El paquete opcional `rnl-std-no-labels` desactiva las etiquetas por defecto de los símbolos estándar, para archivos que escriben todas sus etiquetas por sí mismos; no está en la colección. En la serie 0.4 selecciona únicamente objetos dibujados (`drawn=true`, 10.1) y da a sus etiquetas de nombre, valor y modelo, y a los nombres de red que dibujan los símbolos de los usos, el estilo `textvisible=false` (9.3); un archivo vuelve a mostrar una de ellas escribiendo `textvisible=true` sobre ella (13.3). El paquete opcional `rnl-std-no-instance-labels` hace lo mismo solo para las instancias dibujadas, de modo que las etiquetas de los usos permanecen, como los nombres de red de las tierras, las alimentaciones, las etiquetas de puerto y las etiquetas de red; tampoco está en la colección. Pueden añadirse más paquetes después. Toda la serie estándar se publica junta bajo un único número de versión, y la colección fija las versiones de sus paquetes.
  - Esta es la estructura de la serie estándar 0.4; su `rnl-std-all` tiene un registro de encabezado para cada lenguaje (2.8).
- **Sin carga implícita.** Un archivo de la versión 0.4 usa el paquete estándar `rnl-std-all` únicamente cuando un USE-PACKAGE lo escribe, como cualquier otro paquete (16.2). Una herramienta puede ofrecer un ajuste que lo añada cuando el registro de encabezado del archivo principal no escribe `rnl-std-all` de ninguna forma, como una opción `--include-std` en su línea de comandos o un ajuste de un visor, que puede estar activado por defecto. Con el ajuste activado, la herramienta lee el archivo como si su registro de encabezado sin `language` escribiera `@USE-PACKAGE(name="rnl-std-all", version="…")` en primer lugar, con la versión más nueva que la herramienta tenga de la serie estándar escrita en la versión del lenguaje del archivo, y cuando escribe en el archivo principal escribe ese USE-PACKAGE en su registro de encabezado, para que cualquier herramienta dibuje igual el archivo. Una versión fijada en el registro de encabezado de un archivo se usa para ese archivo y los archivos que incluye (abajo).
  - Nota: el paquete estándar es un paquete como cualquier otro. Un netlist que no fue escrito para RNL se dibuja con símbolos estándar únicamente mediante un ajuste así, y un archivo que usa las definiciones del paquete estándar, como `$GROUND`, escribe el USE-PACKAGE, o sus nombres no se encuentran. Cada nombre no encontrado es un error como de costumbre; cuando un paquete de la serie estándar que no está en uso define tal nombre, una indicación lo señala una vez para el archivo, nombrando ese paquete y remitiendo a USE-PACKAGE y al ajuste de la herramienta.
- **El paquete de reglas del otro dialecto.** Los nombres de instancias significan cosas distintas en ambos dialectos: en SPICE, `I0` es una fuente de corriente; en Spectre, el nombre de una instancia no dice nada, y Virtuoso nombra `I0` a la instancia de un subcircuito. La serie estándar 0.4 expresa esto con `language`: su colección `rnl-std-all` tiene un registro de encabezado sin `language` para lo que comparten ambos lenguajes, uno con `language="spice"` que usa `rnl-std-spice`, y uno con `language="spectre"` que usa `rnl-std-spectre` (2.8). Así, en un archivo Spectre una sección SPICE encuentra sus símbolos según las reglas de SPICE y las secciones Spectre según las de Spectre.
  - Una herramienta que congela la serie 0.4 escribe la parte de cada lenguaje como un paquete en línea en el registro de encabezado de ese lenguaje, y la parte compartida en el registro de encabezado sin `language`: un paquete en línea contiene sentencias, no registros, y no puede llevar `language` por sí mismo.
- **Paquetes en la raíz.** El registro de encabezado de un archivo principal o de un archivo incluido, entre ellos un archivo Verilog-A (2.9), puede contener USE-PACKAGE:

  ```rnl
  * @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4")}
  ```

  - Su alcance es el de un USE-PACKAGE en el nivel de archivo: el archivo que lo escribe y los archivos que incluye. Uno escrito en un archivo incluido se aplica a ese archivo y a los archivos que incluye a su vez, no al archivo que lo incluye (6.1).
  - Se sitúa en la raíz: antes de las reglas de todos los niveles, y también antes de los símbolos de bloque (8.3, grupo 1). Sus nombres `$` están también en la raíz, de modo que una definición del mismo nombre en un nivel, como en el archivo principal, sigue ocultándola. Un USE-PACKAGE en el nivel de archivo se sitúa en su propia posición en su lugar: vuelve a usar el paquete (16.2), por lo que sustituye a las reglas que descienden, y sus nombres `$` cuentan como pertenecientes al nivel superior. Escrito en el registro de encabezado, un paquete dice "este archivo depende de esta versión, y el estilo de los niveles superiores aún puede sustituirlo"; escrito en el nivel de archivo, dice "este archivo usa el paquete para fijar su propio dibujo".
  - Qué USE-PACKAGE surte efecto es como en 16.2: el escrito en el nivel más cercano a este nivel, donde uno en el registro de encabezado cuenta como escrito al principio mismo del nivel de archivo.
  - El orden de los paquetes en la raíz: el paquete estándar `rnl-std-all` primero, dondequiera que lo escriba un registro de encabezado y también cuando un ajuste de la herramienta lo añade, luego los demás en el orden del primer archivo que escribe cada uno (desde el archivo más externo hacia dentro) y de su registro de encabezado. Cuando el registro de encabezado de un archivo interno vuelve a escribir el mismo paquete, esto cambia únicamente la versión o el contenido en línea usado en los niveles de ese archivo; el lugar se mantiene, de modo que fijar una versión no cambia el orden.
  - Un paquete en un registro de encabezado que define un nombre `$` del paquete estándar, o de un paquete situado en la raíz antes que él, oculta esa definición y produce una indicación (6.8).
  - Fijar una versión es elección del autor. Un archivo principal debería fijarla. Un archivo incluido que no la fija sigue al archivo que lo incluye; debería fijar una versión únicamente cuando depende de una. Las herramientas no deben añadir versiones a archivos incluidos por sí mismas, salvo la fijación que escribe subir un archivo de una versión anterior (16.1).
  - No debería fijarse una versión con un USE-PACKAGE en el nivel de archivo del archivo principal: allí el paquete se sitúa después de los símbolos de bloque, y cuando el archivo se incluye en otro sitio, el USE-PACKAGE vuelve a usar el paquete (16.2) y cambia el significado del archivo.
- **Ejemplo.** El registro de encabezado de `tb.sp` escribe `@USE-PACKAGE(name="rnl-std-all", version="1.2")`, y el de `ota.sp`, que `tb.sp` incluye, escribe la versión `"1.0"`.
  - El nivel superior de `tb.sp` y los subcircuitos definidos en `tb.sp` usan `"1.2"`; los subcircuitos en `ota.sp` usan `"1.0"`. Las reglas de estilo de `tb.sp` siguen descendiendo a `ota.sp` y sustituyen los valores por defecto de `"1.0"`.
  - Las elecciones por defecto las hacen reglas en el paquete estándar, como la regla que asigna a los usos de `$GROUND` un símbolo de tierra. Cada versión selecciona su propio `$GROUND` en los niveles que la usan, de modo que `ota.sp` se dibuja como de costumbre.
  - Las definiciones del mismo nombre en las dos versiones son dos definiciones (16.2). Así, una regla de `tb.sp` que selecciona por definición, como `# @SELECT-USAGE(recursive=true) $GROUND => @SET-SYMBOL $MY_GND`, encuentra únicamente los usos del `$GROUND` de `"1.2"`; `ota.sp` la sigue únicamente si no fija una versión propia. Las reglas que seleccionan por propiedades del anfitrión, como `# @SELECT-INSTANCE(type="nmos", recursive=true) => …`, no se ven afectadas.
  - Para que sus símbolos queden libres de las reglas de `tb.sp`, `ota.sp` usa el paquete en su nivel de archivo (16.2).
- **Menos del paquete estándar.** Un ajuste de la herramienta añade únicamente el nombre de paquete `rnl-std-all`.
  - Para dejar fuera algunos paquetes, descártense con paquetes en línea vacíos del mismo nombre: `@USE-PACKAGE(name="rnl-std-fill") {}` desactiva el relleno automático. Un paquete del mismo nombre está en vigor una sola vez, nunca dos.
  - Para usar solo unos pocos, escríbanse uno por uno, con un `rnl-std-all` vacío para que un ajuste de la herramienta no añada también la colección:

  ```rnl
  * @RNL(type="header") {
  *   @VERSION("0.4");
  *   @USE-PACKAGE(name="rnl-std-all") {};
  *   @USE-PACKAGE(name="rnl-std-style", version="0.4");
  *   @USE-PACKAGE(name="rnl-std-basic", version="0.4");
  *   @USE-PACKAGE(name="rnl-std-spice", version="0.4")
  * }
  ```

- **Extender el paquete estándar o reemplazar una parte de él.** Escríbase un archivo de paquete cuyo registro de encabezado referencie los paquetes estándar que usa, y añádase su propio contenido (16.2, paquetes en línea). Las definiciones y reglas que únicamente añaden al paquete estándar pueden escribirse también directamente en el netlist; las reglas que tiene el paquete estándar pueden eliminarse o reemplazarse únicamente mediante un paquete del mismo nombre.
- **Congelar o descartar todo el paquete estándar.** Escríbase un paquete en línea del mismo nombre en el registro de encabezado. Esto se sigue de "uno solo en vigor", contado por nombre de paquete; no hay ninguna regla adicional:
  - `@USE-PACKAGE(name="rnl-std-all", version="1.2") {…}` congela el paquete estándar en el archivo, haciendo el archivo autocontenido; el cuerpo es el contenido de sus paquetes tras la expansión, uno tras otro en el orden de la expansión (16.2). Reunidos en un solo cuerpo, dos paquetes pueden colisionar: un nombre `%` privado usado en ambos, o un nombre `$` que uno oculta a otro. Una herramienta que congela renombra lo que colisiona, de modo que cada referencia siga encontrando lo que encontraba en su propio paquete, y escribe las sentencias DEVICE y SOLVER-CONFIG de los registros de encabezado de los paquetes congelados en los registros de encabezado del archivo, en su orden y antes de las del propio archivo (2.8, 6.12, 14.10), puesto que un paquete en línea no puede contenerlas: cada una en el registro de encabezado para el lenguaje a través del cual se usa su paquete, o en el que no tiene `language` para un paquete usado sin él; el SOLVER-CONFIG de un paquete usado únicamente a través de registros de encabezado con `language` no se copia, puesto que no tiene efecto (14.10). Las copias quedan entonces después de los paquetes que iban después del congelado. Donde un paquete tal escribe una clave de SOLVER-CONFIG que escribe una copia, o un elemento de DEVICE (nombres de terminal o tipo) para instancias con las que también coincide un DEVICE copiado, la herramienta escribe esa sentencia en el registro de encabezado que contiene la copia en colisión, después de ella y antes de las del propio archivo, cuando el paquete se usa en un registro de encabezado del mismo archivo, e informa de la colisión cuando se usa en un archivo incluido o en un nivel. Aparte de las colisiones señaladas, el paquete congelado significa lo mismo;
  - `@USE-PACKAGE(name="rnl-std-all") {}` no usa ningún paquete estándar, también cuando un ajuste de la herramienta añadiría uno; toda instancia tiene entonces el símbolo de bloque por defecto (9.2.6), y el resto de la apariencia queda a los valores por defecto del renderizador (Principios 2.7).
- Convenciones como los buses en negrita, los símbolos de toma para buses y las marcas para la anchura de un bus son también contenido del paquete estándar. Sin un paquete estándar, toda instancia tiene el símbolo de bloque por defecto (9.2.6), y el renderizador decide el estilo de dibujo (Principios 2.7).

### 16.4 Extensiones y contenido desconocido

- **Contenido desconocido** (Principios 2.3). Las palabras clave, parámetros y valores desconocidos son errores: la sentencia falla, las sentencias que dependen de ella fallan con ella (5.5, 7.3) y el resto se interpreta como de costumbre (7.6). Los campos desconocidos de un encabezado del registro son la excepción y producen únicamente una advertencia (2.8). En SOLVER-CONFIG, una clave que el solucionador no conoce produce una advertencia, y un valor que no se admite hace fallar solo su clave (14.10). El dibujo puede entonces quedar incompleto, y los diagnósticos deben indicarlo.
- **Conservado al guardar.** Principios 2.3 recomienda conservar tal contenido sin cambios; este estándar lo exige:
  - los registros que no son propios de la herramienta no se cambian (capítulo 17), y el texto del anfitrión y los comentarios ordinarios se dejan tal como están;
  - cuando una herramienta reescribe sus propios registros, las sentencias en ellos que no conoce o que fallan se conservan tal como están, con su texto libre, en el mismo orden;
  - un archivo cuya versión no está admitida se conserva enteramente tal como está (16.1);
  - los registros que reescribe una herramienta no necesitan permanecer idénticos byte por byte.
- **Sin mecanismo de extensión en esta versión.** Nuevas semánticas, componentes virtuales y maneras de dibujar se escriben como definiciones, símbolos y estilos en paquetes (1.5 punto 7), no como nuevas palabras clave; un mecanismo nuevo que las implementaciones deban entender espera a una nueva versión de este estándar. Los datos privados de una herramienta, como los bloqueos o el estado de vista de un editor, no se escriben en registros RNL; van en archivos de la herramienta, o en comentarios ordinarios fuera de registros.
  - Nota: esta versión hace primero que todas las herramientas lean el mismo RNL; dialectos de proveedores tan pronto dividirían el lenguaje, y las nuevas semánticas ya pueden escribirse en paquetes.
  - Nota: una versión posterior puede añadir extensiones como palabras clave con un prefijo de proveedor y un `.`, como `@ACME.ROUTE`, declaradas en el registro de encabezado: una extensión obligatoria que una herramienta no conoce dejaría todo el archivo sin interpretar, y una opcional se omitiría con una indicación. Tal palabra clave es ahora una palabra clave desconocida, un error, de modo que añadir esto más tarde es una relajación pura (16.1); a un prefijo de proveedor no se le permitiría tener el nombre de una palabra clave estándar.
- No se reserva ninguna forma para extensiones: un `.` justo después de una palabra clave estándar es texto libre, de modo que `@BUNDLE.LOCK` es BUNDLE seguido del texto libre `.LOCK`, y `{#M1 #M2} @GROUP.` al final de una frase es válido (3.5).
- **Reconocido pero no interpretado.** En esta versión el contenido de esta clase es un campo desconocido de un encabezado del registro (2.8) y una clave de SOLVER-CONFIG que el solucionador no conoce (14.10). Las extensiones opcionales serían una segunda clase, si una versión posterior añade extensiones.

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

- Los alias dan a las palabras estándar otras grafías, por compatibilidad con otro uso de mayúsculas y minúsculas, o para escribir RNL al estilo de otro idioma natural, como el chino.
- **De dónde proceden los alias:**
  - sentencias ALIAS escritas directamente en el registro de encabezado, para unos pocos alias propios;
  - paquetes de alias, cargados con `@USE-ALIASES(name=…, version=…, path=…)` en el registro de encabezado. Los alias de un idioma se reúnen en un solo paquete, para que los archivos no tengan que repetirlos y la redacción sea la misma en todas partes. El paquete de alias estándar en chino es `alias-zh`; se publica con el paquete estándar y viene con las herramientas (16.3).
  - Ambos pueden usarse juntos, y pueden cargarse varios paquetes de alias; juntos se comprueban según las reglas de correspondencia siguientes.
  - Los alias se aplican únicamente al archivo que los declara y no se heredan mediante inclusión; un archivo incluido o un archivo de paquete que usa alias los declara en su propio registro de encabezado. La misma palabra estándar puede tener alias distintos en archivos distintos. Las claves de los textos escritos en un símbolo se comprueban frente a los alias del archivo en el que está escrito el símbolo (6.5, 13.3).
  - El propio registro de encabezado se lee siempre en la grafía estándar y no reconoce alias: hasta que se hayan leído USE-ALIASES y ALIAS, los alias no se conocen, y los registros de encabezado los escriben mayoritariamente herramientas. Por tanto, los alias de las palabras clave que pueden escribirse únicamente en el registro de encabezado (VERSION, ALIAS, USE-ALIASES, DEVICE, PACKAGE) no tienen utilidad.
- **Paquetes de alias:**
  - Un paquete de alias es un archivo de paquete (16.2) cuyo registro de encabezado contiene únicamente VERSION, PACKAGE y una lista de sentencias ALIAS, y que no tiene otros registros. Se busca, y sus versiones se eligen, como en 16.2.
  - USE-ALIASES de un archivo que no es un paquete de alias, o USE-PACKAGE de un paquete de alias, es un error. Cuando no se encuentra un paquete de alias, su USE-ALIASES es un error, y las sentencias que usan sus alias fallan con él (7.3); el diagnóstico indica el paquete de alias ausente como causa.
  - Nota: los paquetes de alias no se cargan con USE-PACKAGE porque USE-PACKAGE puede escribirse en cualquier nivel, trata de definiciones y reglas y se ordena por posición, mientras que los alias tratan de cómo se lee el texto de un archivo y pueden aplicarse únicamente por archivo. Al mantenerlos separados, un paquete de estilo de la casa no puede cambiar cómo se lee el texto.
  - El contenido de un paquete de alias sigue a aquello a lo que hace referencia: cuando un nombre del paquete estándar cambia, `alias-zh` cambia con él y su versión sube.
- **Forma.** `@ALIAS("alias", "original")`: el alias primero. Los dos parámetros son `alias` y `original`, normalmente dados por posición; el tercer parámetro, `definition`, se da únicamente por nombre (abajo).
- **Originales:**
  - Una palabra clave o palabra estándar definida por este estándar, en su grafía estándar. Una palabra clave se escribe con su `@`, como `"@PLACE"`; las demás palabras sin él, como `"axis"` o `"low"`. El alias se escribe del mismo modo: el alias de una palabra clave lleva `@`, los demás alias no. El `@` distingue una palabra clave de una palabra estándar de igual grafía, y un alias aparece en el texto exactamente tal como se declaró.
  - Un nombre `$`. Tanto el alias como el original llevan `$`, como `@ALIAS("$接地补位集", "$GROUND_FILL_SET")`; en el texto se escribe `$"接地补位集"` (un nombre no ASCII tras `$` necesita comillas, 3.6) y se lee como `$GROUND_FILL_SET`. Los nombres `%` no tienen alias: quienes escriben pueden dar sus propios nombres directamente en otro idioma, y los nombres `%` en paquetes son privados.
  - Un nombre en una definición, declarado con `definition=`, como `@ALIAS("左管", "left", definition="$DIFFERENTIAL_PAIR_N")` o `@ALIAS("间距", "gap", definition="$GROUND_STUB")`. Tal alias se busca únicamente dentro de esa definición: nombres de rol cuando se aplica (`#M1(左管) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N`), nombres de parámetros pasados a ella por AS y ATTACH (`$GROUND_STUB @ATTACH(间距=3) …`), y nombres locales tomados de sus usos mediante `::` (`%c1::"中点"`). El valor de `definition` puede escribirse él mismo como un alias. Los alias de palabras estándar no se usan en estos lugares, salvo para `priority`, el parámetro de los propios AS y ATTACH.
    - Los alias van por el nombre: siguen al nombre `$`, y también se aplican donde un nivel inferior oculta la definición con una del mismo nombre, igual que un alias de un nombre `$` reemplaza la grafía.
    - Cuando se declara el alias, el original no se comprueba frente a la definición; cuando se aplica la definición, un nombre que no se encuentra es un error como de costumbre (como "no hay rol in_p").
    - Los nombres tras `::` se buscan en el orden de 6.5: los nombres del propio uso, sus alias de `definition=`, los nombres estándar de subobjeto y sus alias, los nombres de los pines del símbolo del objeto, las claves de los textos.
    - El valor de la condición de selección `role=`, y un camino en `${…}` que desciende por roles, también reconocen estos alias, buscados en la definición del uso en cada paso, como `%dp1 @SELECT-INSTANCE(role="左管")` o `${左管::w}`, para un uso `%dp1` de `$DIFFERENTIAL_PAIR_N`. Cuando el ámbito contiene usos de varias definiciones, cada uno se busca en su propia definición; los originales siguen funcionando.
- **Qué puede tener alias:** todas las palabras de la lista de palabras salvo las enumeradas en el punto siguiente: palabras clave, nombres de ranura de palabras clave integradas, nombres y valores de parámetro, clases, nombres de subobjeto, nombres de atributo, propiedades de estilo, `true` y `false` (5.3); y los nombres `$` y nombres en definiciones anteriores.
- **Qué no puede tener alias:** la forma externa, es decir, `@RNL`, los nombres de campo de un encabezado del registro y los valores de `type`, y `@VERSION`; y los propios `@ALIAS` y `@USE-ALIASES`.
- **Otros nombres.** Un nombre es ya otro nombre de un objeto: tras `$"差分对" = $DIFFERENTIAL_PAIR_N`, `$"差分对"` es la misma definición. Tal nombre forma parte del texto, no puede volver a sacarse y no alcanza nombres de rol o de parámetro; para un conjunto completo de nombres en otro idioma, úsense alias.
- **Correspondencia:**
  - Un original puede tener varios alias; dentro de un mismo ámbito un alias representa un único original. Los alias de `definition=` de cada definición forman un ámbito.
  - Un alias no debe tener la grafía de ninguna palabra estándar de la versión que declara el archivo, y no debe declararse de nuevo para otro original. Un alias de `definition=` no debe tener la grafía de un rol, parámetro o nombre local que tenga la definición, ni de un alias de `priority`, el parámetro de los propios AS y ATTACH.
  - Un alias no puede usarse como original, y no puede ser igual que su original. Cuando se sube la versión de un archivo y una palabra estándar recién añadida tiene la grafía de un alias, se emite una advertencia y la versión se sube de todos modos (16.1); en la nueva versión el alias tiene entonces la grafía de una palabra estándar, lo que la regla anterior convierte en error.
  - Un paquete de alias debe declarar la versión del lenguaje del archivo que lo usa (16.1); de lo contrario su USE-ALIASES es un error.
- **Los alias únicamente añaden grafías.** Las grafías estándar funcionan siempre, y ambas pueden mezclarse en un mismo archivo.
- **Los alias reemplazan únicamente tokens completos:**
  - un alias reemplaza un token completo; no es una macro de texto, por lo que las posiciones y los diagnósticos se mantienen correctos;
  - las palabras estándar sin `@` reciben alias según su grafía, de modo que palabras de igual grafía (como el nombre de parámetro `x` y el valor `"x"`) comparten sus alias;
  - un alias de un nombre `$` reemplaza la grafía: un autor que escribe `$"接地补位集" = {…}` define `$GROUND_FILL_SET` en ese nivel, lo que oculta el del paquete estándar con una indicación (6.8).
- **Reglas léxicas:**
  - Un alias de una palabra clave son las letras (de cualquier escritura), dígitos y guiones bajos justo después de `@`, sin `-`; forman un token, y el `@` debe ir precedido de un espacio (2.3), también después de puntuación china: `对 &b，@选出元件` es un error, y debe escribirse `对 &b， @选出元件`. Cualquier carácter que no sea letra, dígito o guion bajo termina el alias: un espacio, o un signo de puntuación de cualquier escritura, como `，` o `。`. Así, `@对齐两个器件` es una palabra clave desconocida, un error, y su diagnóstico debe preguntar si falta un espacio después de `@对齐`. No hay reglas para un idioma u otro: lo que no es letra, dígito o guion bajo de ninguna escritura termina un alias, y los caracteres no ASCII fuera de paréntesis son texto libre (3.3). Los tokens no se dividen según los alias declarados, de modo que leer los tokens no requiere declaraciones.
  - Las palabras no ASCII sin comillas aparecen únicamente en dos lugares: después de `@` y dentro de paréntesis. Después de `@`, la palabra debe ser un alias declarado; de lo contrario es un error ("un nombre no ASCII necesita comillas"). Dentro de paréntesis (nombres de parámetro, nombres de ranura de palabras clave integradas, `true` y `false`, nombres de rol), un alias declarado es un alias; en el lugar de un nombre de rol o de un nombre de parámetro de una definición, una palabra que no es un alias es el nombre mismo (3.6); en otros lugares, una palabra que no es un alias es un error. En los demás sitios se aplica 3.6: los nombres del anfitrión, nombres RNL, nombres de terminal y nombres tras `::` tienen únicamente caracteres ASCII cuando no llevan comillas, y un alias usado tras `::` va también entre comillas, como `#M1::"位置"`.
  - Un alias sin comillas entre paréntesis que tiene la grafía de un nombre de parámetro o nombre de rol de una definición utilizable en el mismo lugar es un error, porque su significado no es único. Una palabra entre comillas dentro de paréntesis es siempre un nombre, nunca un alias. Los alias de nombres `$` y los alias tras `::` se escriben entre comillas de todos modos (3.6).
  - Los alias de valores de enumeración se escriben dentro de las cadenas y funcionan únicamente en parámetros cuyos valores son una enumeración. Los nombres de atributo en `${…}` reconocen también alias (13.6). Otras cadenas, como `text` de TEXT, no se reemplazan.
- **Diagnósticos y traducción:**
  - Los diagnósticos muestran las palabras tal como están escritas en el archivo, junto con su grafía estándar.
  - Un traductor funciona mecánicamente en ambos sentidos. Al traducir a alias, escribe USE-ALIASES en el registro de encabezado; para un archivo autocontenido escribe los alias usados como sentencias ALIAS en su lugar. Cuando un original tiene varios alias, se toma el declarado primero. Traducir de vuelta a la grafía estándar elimina USE-ALIASES y ALIAS y no pierde nada.
  - El texto libre, los comentarios y las cadenas no se traducen, salvo los valores de enumeración y los nombres de atributo en `${…}`.
  - Los LLM, los solucionadores y los editores escriben la grafía estándar. Cuando modifican un archivo escrito con alias, las sentencias que añaden están también en la grafía estándar; mezclarlas es válido.

<!-- translated from en/ch17.md sha256:801f28995842 -->

## 17 Modelo de modificación

- RNL describe únicamente el estado actual, no la edición que condujo a él:
  - no hay operaciones de deshacer ni de eliminar; eliminar es editar el texto;
  - no hay desplazamientos relativos; un editor escribe nuevos valores absolutos.
- **Quién modifica qué registros.** El solucionador reemplaza los registros con `type="generated"`. Las demás herramientas deben modificar únicamente los registros cuyo `author` sea ellas mismas, como un editor que ordena los cambios que escribió, o una nueva importación que reemplaza lo que escribió la importación anterior. Las herramientas no deben tocar los registros redactados por el autor sin un `author` (2.8).
- Las descripciones añadidas después sustituyen a las anteriores, en orden (8.4); para los hechos geométricos, se compara primero la prioridad (8.5). `type` y `author` no afectan al orden (8.3).
- **La salida va con el subcircuito.** La salida del solucionador y los cambios que realiza un usuario en un editor se escriben en el propio texto del nivel: para un subcircuito, dentro de su `.subckt … .ends`, incluso cuando está en un archivo incluido; para el nivel superior, en el nivel de archivo del archivo principal. El solucionador escribe su salida con `priority="high"` al final del propio texto del nivel; cuando resuelve, los registros con `type="generated"` se omiten de la interpretación (2.8), y después los reemplaza (14.8). Debe escribir de vuelta cada decisión que toma (14.8). `scope=` es únicamente para las sentencias que el autor decide escribir fuera (2.6). Si se guarda un archivo de solo lectura queda a criterio de la herramienta.
- **Archivos Verilog-A.** Un nivel de módulo no tiene nada que resolver y no recibe salida del solucionador. En un archivo Verilog-A una herramienta modifica únicamente el registro de encabezado y los registros de símbolos de bloque, dentro de sus módulos, escritos con `//` (2.9).
- **Una solución desactualizada únicamente produce una indicación.** Cuando cambia un nivel superior o un paquete, la solución de un subcircuito en una biblioteca puede quedar desactualizada, por ejemplo cuando el archivo principal cambia un símbolo. La solución antigua sigue surtiendo efecto; un visor que detecta que el `hash` del encabezado del registro no coincide con la entrada actual emite una indicación para volver a resolver (2.8).
  - El hash se calcula a partir de la entrada real del nivel tras la herencia: las reglas que descienden y surten efecto, los símbolos elegidos, las versiones de los paquetes y las opciones del solucionador, sin los registros con `type="generated"` y sin los campos de metadatos. Se calcula por nivel, no por archivo, y no solo a partir del propio texto del subcircuito, pues de otro modo los cambios en el archivo principal pasarían inadvertidos.
  - El hash no cambia la interpretación. Su algoritmo pertenece a las herramientas; el valor debería llevar una etiqueta para el algoritmo, como `"sig1:…"`, y una herramienta que no conoce el algoritmo no lo compara.
  - Nota: si un hash que no coincide implicara no usar la solución antigua, el algoritmo pasaría a formar parte del lenguaje, toda herramienta tendría que calcularlo del mismo modo y un visor que no resuelve no podría dibujar el nivel.
  - Los números son la excepción: cuando cambian las sentencias que crean objetos sin nombre, la comprobación de los números deja de coincidir, y las sentencias que usan números no surten efecto hasta que el nivel vuelve a resolverse (5.9). La comprobación abarca únicamente el texto del nivel y de las definiciones que usa, de modo que toda herramienta la calcula igual, y el resto de la solución antigua sigue dibujándose.
- **Objetos generados.** Cuando el solucionador ha colocado y cableado objetos que generan las reglas, los escribe como sentencias explícitas en su propio registro, junto con los cables. El objeto explícito prevalece, y las reglas de relleno dejan de generar el mismo objeto (10.5). A los objetos en sus cuerpos que no tienen nombre local se les hace referencia por número (5.9).
- **Cambio de lo que creó el solucionador.** Un nombre que asigna el solucionador a un objeto que escribe se vincula únicamente en su propio registro, que se omite cuando se resuelve el nivel (2.8). Por tanto, una sentencia de otro registro que usa este nombre no encuentra nada al resolver, y el solucionador no puede tenerlo en cuenta. Para describir tal objeto por separado, un editor lo escribe a mano en su propio registro, como indica 10.5, y añade el cambio, como `%g_r4 = $GROUND_STUB @ATTACH #R4:2; %g_r4::g @PLACE(30, -4, priority="high")`: el cable del tramo corto convierte al pin en cableado, el relleno lo omite y la siguiente resolución deja de escribir ese objeto; hasta entonces, el objeto en el registro del solucionador se dibuja también. Cuando el objeto cambiado es el propio uso generado, como una etiqueta de puerto, una selección desde el objeto del anfitrión lo alcanza tanto al resolver como al dibujar, como `&out @SELECT-ANCESTOR-USAGE $IO_PORT => @PLACE(40, 20, priority="high")`; los objetos locales de un uso no pueden seleccionarse a través del uso, únicamente por nombre o como objetos del nivel (9.4).
- Los campos de un encabezado del registro se aplican a todas las sentencias del registro (2.8).
- **Ejemplo: un cambio en el editor.** A continuación se muestra un fragmento completo, con los prefijos de comentario del anfitrión. Tras el registro de encabezado, el diseñador escribió el primer registro y el solucionador el segundo; el usuario arrastró VIN hacia la izquierda en el editor, y el editor añadió el tercero:

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

  - El arrastre se escribe como un nuevo registro, añadido después del registro del solucionador, con prioridad `"high"`. Ambos tienen prioridad high, por lo que se conserva el posterior y la x que escribió el solucionador se descarta, con una indicación (8.5); siempre que el nivel vuelve a resolverse, la nueva x es una posición fija (14.8).
  - Escribirlo en el registro del diseñador no funciona. Las sentencias allí son `"normal"` por defecto y no pueden sustituir al `"high"` del solucionador; escritas como `"high"`, siguen perdiendo, porque el registro del solucionador viene después, y dentro de una misma prioridad se conserva el hecho posterior. Esta equivocación no es un error; únicamente produce una indicación sobre la resolución. La posición surte efecto la próxima vez que se resuelve el nivel, cuando la sentencia del diseñador es una de las restricciones; hasta entonces el dibujo no cambia.
  - Los parámetros dados por posición van antes de los dados por nombre (4.2 punto 9): `@PLACE(-4, -2, priority="high")` equivale a `@PLACE(x=-4, y=-2, priority="high")`, y `@PLACE(x=-4, -2)` es un error.
  - Para cambiar lo que escribió el diseñador (cambiar un símbolo, renombrar, eliminar una restricción), edítese directamente el registro del diseñador. El registro del solucionador no se toca, y se reemplaza en bloque la próxima vez que se resuelve el nivel.
- Para desactivar temporalmente una sentencia, enciérrese entre `/* … */`.
- **Ida y vuelta.** RNL no garantiza que un dibujo permanezca igual tras cualquier exportación a RNL y reimportación (Principios 1); un exportador puede optar por escribir RNL exacto a nivel de coordenadas (Principios 3). Esta versión no define qué es el RNL exacto ni criterios para recorridos de ida y vuelta.

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

<!-- translated from en/annex-c.md sha256:e15f7ba635ad -->
## Anexo C Reservado para versiones posteriores (informativo)

Este anexo enumera lo que esta versión del estándar deja fuera y señala que puede llegar en una versión posterior, con las secciones que lo mencionan. Esta versión no reserva palabras clave ni formas de escritura para estos elementos (5.3, 16.4), y una versión posterior no tiene por qué añadirlos.

| Dejado para una versión posterior | Véase |
|---|---|
| Portadores para más anfitriones, incluidos los comentarios de bloque (como `#com … #endcom` de Eldo), y el tratamiento de un terminador de comentario del anfitrión que esto conlleva | 2.2, 2.5 |
| Identificadores de registro; las herramientas encuentran sus registros por `type`, `author` y las claves de las sentencias | 2.8 |
| Expresiones | 3.7 |
| Oraciones estándar que indiquen si una definición se aplica a todo un conjunto o una vez por cada miembro (`each`) | 4.5 |
| Ranuras opcionales; sin ellas, una selección tiene siempre un ámbito, aunque solo sea `#` | 5.3, 10.1, 13.7 |
| Más de un subíndice expandido en una misma referencia, y otras formas de subíndices, como `<*2>A`, `A<0:2*2>` y `D[0..7]` | 6.11 |
| Códigos de diagnóstico estándar | 7.6 |
| Una forma de suprimir diagnósticos en un archivo, y un modo estricto, por ejemplo como campos de la cabecera del registro | 7.6 |
| Unidades de varios hechos: una definición o un uso cuyos hechos se conservan o descartan conjuntamente en la resolución de conflictos. Sigue abierto: la prioridad de dicha unidad, los usos anidados y las prioridades escritas desde el exterior mediante `::` | 8.5 |
| Escalado de símbolos y de instancias | 9.2.1, 11.7 |
| Flechas en los extremos de los cables | 9.2.5 |
| Más estilos de línea | 9.3 |
| Nombres de colores y `rgb()` | 9.3 |
| Rellenos de patrón, degradados y sombras | 9.3 |
| La intersección de conjuntos en una sola sentencia | 10.1 |
| Elegir el símbolo de una primitiva por sus parámetros. Para unas pocas primitivas de cada dialecto, el simulador decide la clase por la línea: en SPICE, una fuente B escrita con `V=` es una fuente de tensión y una con `I=` es una fuente de corriente. Los subcircuitos y los modelos no varían de este modo. Una vía es una condición de SELECT-INSTANCE como `param=` (la línea tiene este parámetro); otra es un tipo que el adaptador del dialecto asigna a estas primitivas (6.12), seleccionado con `type=`. El número de terminales se selecciona con `terminals=` (10.1). Hasta entonces, el paquete estándar dibuja ambas clases según sus reglas actuales | 10.1, 6.12 |
| Rotar un conjunto entero como un solo bloque con ORIENT | 11.2 |
| Varios dibujos de un mismo objeto del anfitrión, como objetos independientes con nombres propios | 11.4 |
| Un gráfico que enmarque un grupo de objetos | 13.4 |
| Anotaciones ancladas en cables: posiciones a lo largo de un cable (como `t` u `offset`), y símbolos o flechas anclados en mitad de un cable | 13.3, 15.2 |
| Referencias en `${…}` a los atributos de otros objetos, como `${R3:VALUE}` de KiCad | 13.6 |
| Las proyecciones diagonales `"x+y"` y `"x-y"` para las restricciones | 14.1 |
| Un USE-PACKAGE que se aplique solo a su propio nivel | 16.2 |
| Un paquete dividido en varios archivos | 16.2 |
| Un formato de manifiesto para un netlist y sus archivos de paquete | 16.2 |
| Un resumen de contenido para paquetes, como un parámetro `digest=` de USE-PACKAGE | 16.2 |
| Referencias que un paquete utiliza solo para sí mismo, aparte de las que transmite a sus usuarios | 16.2 |
| Nombres cualificados por paquete, otros nombres para paquetes y etiquetas de nivel, como `$acme.DIFFERENTIAL_PAIR` o `as=`, en una forma distinta de un `.` después del nombre | 16.2 |
| Un mecanismo de extensión: palabras clave con prefijo de proveedor, como `@ACME.ROUTE`, declaradas en el registro de cabecera como extensiones obligatorias u opcionales | 2.8, 16.4 |
| RNL exacto, es decir, las condiciones bajo las cuales un archivo puede dibujarse sin un solucionador, y los criterios para los recorridos de ida y vuelta; estos pueden acordarse en su lugar entre herramientas | 17 |

<!-- translated from en/annex-d.md sha256:72752f1bf7f8 -->
## Anexo D Cambios respecto a la versión 0.3 (informativo)

La versión 0.4 escribe cada hecho sobre posiciones con su dirección, de modo que un hecho significa una única disposición y no una elección entre varias. Las versiones 0.x no prometen compatibilidad entre sí (16.1); este anexo enumera qué cambió y cómo pasa un archivo a la versión 0.4. Este estándar describe solo la versión 0.4; un archivo de 0.3 pasa a 0.4 con una herramienta de actualización que lee la versión 0.3, la cual realiza los cambios indicados al final de este anexo (16.1).

| Cambio | Véase |
|---|---|
| Las direcciones se escriben explícitamente. ORDER y DISTANCE admiten solo las cuatro direcciones; `"x"` e `"y"`, que dejaban abierta la dirección, son valores de parámetro erróneos. DISTRIBUTE toma una dirección en lugar de un eje, en el parámetro `direction` (ya no `axis`): sus miembros se suceden estrictamente en esa dirección, en pasos iguales, y dos miembros forman un ORDER. ALIGN toma una sola proyección, no una lista | 14.1–14.5, 8.5 |
| ORIENT y ORIENT-RELATION toman una lista de códigos, como `["R0", "MY"]`: un único hecho, que se cumple cuando se cumple alguno de ellos; el solucionador elige y escribe de vuelta un único código. La orientación es la única clase de hecho que ofrece tal elección | 11.2, 14.9, 8.5 |
| ORIENT-RELATION relaciona un objeto con un objeto. Un conjunto en una ranura se procesa miembro por miembro, un hecho por cada miembro; un conjunto de más de un miembro en ambas ranuras es un error. La versión 0.3 relacionaba dos conjuntos como un todo | 14.9, 5.2.1 |
| Lugares de miembros: una referencia a un conjunto seguida directamente de `{…}` toma miembros por su lugar, contado sobre sus miembros desplegados un nivel, como `%a{1}`, `%a{end}`, `%a{2:end}`; un uso toma sus miembros por rol, y los lugares de una selección son un error. En 0.3, `%a{#b}` eran dos objetos | 5.2.3, 3.2, 3.3, 3.5, 4.1, 6.10 |
| Las restricciones toman solo los objetos más bajos. Un conjunto no tiene posición: ni centro ni extensión, y su `::position` es el conjunto de las posiciones de sus miembros. Un conjunto conserva sus elementos tal como se escriben; en la lista de miembros de ALIGN, ORDER y DISTRIBUTE, un elemento que sea un conjunto (un `{…}` o una referencia a un conjunto) representa a cada uno de sus miembros, una restricción por cada combinación, cada una como una unidad de resolución; DISTANCE relaciona cada miembro de una ranura con cada miembro de la otra. En 0.3 una referencia dentro de un conjunto se desplegaba, `{%x}` contaba por su centro y DISTANCE comparaba extensiones | 14.1, 14.3–14.5, 5.2, 5.2.1, 6.5, 8.5, 11.1, 11.2 |
| Un uso ya no se dibuja en el punto del objeto que rellena su primer rol: su símbolo se dibuja en su propio punto, y una definición que dibuja una marca sobre un pin liga su punto al pin en su cuerpo. Un pin que vincula un rol se dibuja en la red de lo que lo rellena, de modo que `$GROUND @ATTACH #M5:3` adjunta una tierra a la red de M5:3, en una posición propia; un número en el símbolo de un uso no vincula ningún rol. En 0.3 ese uso se dibujaba sobre el pin y no tenía posición propia | 12.1, 12.2, 12.5, 12.6, 6.5, 8.2, 9.2.7, 11.2, 11.3, 13.2, 14.1, 15.2 |
| Una definición general puede no tener roles (`roles=[]`), aplicada como `$PORT_LIST @ATTACH`. Su cuerpo puede contener reglas: actúan como reglas del nivel del uso, en la posición de su sentencia, y `#` en el cuerpo es ese nivel. Un `?` aislado en una definición general admite un camino, como `?::position` | 9.4, 8.2, 10.4, 4.1 |
| Este estándar describe solo la versión 0.4: los archivos de 0.3 y 0.2 son archivos de otras versiones principales, que pasan mediante una herramienta de actualización que lee la versión 0.3 al lenguaje 0.4 y a la serie estándar 0.4. El estándar 0.3 describía cómo leía una herramienta los archivos de 0.2 | 16.1 |
| El paquete estándar ya no se usa de forma implícita: un archivo escribe `@USE-PACKAGE(name="rnl-std-all", …)`, o una herramienta lo añade mediante un ajuste, como `--include-std` o un ajuste de un visor, y lo escribe en el archivo cuando escribe el archivo. En 0.3 se usaba en la raíz de todo archivo | 16.3, 16.1, 16.2, 6.1 |
| Las coordenadas literales en el cuerpo de una definición general, de puntos y de PLACE, son desplazamientos desde el propio punto del uso, y el ORIENT del uso no las gira. En 0.3 eran coordenadas del nivel | 9.4, 11.1, 11.2, 13.4 |
| DISTANCE establece también el orden: sus cotas no son negativas, y un `min` ausente vale 0. En 0.3 un DISTANCE con dirección admitía cotas de cualquier signo | 14.5 |
| Reglas y fases: las reglas de WIRE desaparecen, y BUNDLE, WIRE y RECTANGLE, CIRCLE y ELLIPSE escritos con dos puntos no admiten el resultado de una selección ni un conjunto que dependa de una; las reglas de las cuatro palabras clave de trazado se evalúan en un paso propio, el paso 5, antes de la generación, que ve sus árboles; las reglas de LINE, ARC y BEZIER y de CURRENT se evalúan en el paso 6 con las reglas de generación; un CURRENT sobre un pin surte efecto una vez dados los símbolos. En 0.3 estas reglas surtían efecto en la fase de su palabra clave | 10.4, 8.2, 11.8, 13.1, 9.2.5, 15.2 |
| Todo uso tiene su propio punto, también uno sin símbolo, escrito `%u::position`: PLACE lo toma, al igual que las restricciones sobre un uso dibujado con símbolo. En la lista de miembros de una restricción, un uso sin símbolo representa a sus miembros, hasta los objetos más bajos, como en GROUP, y toda combinación que tome un mismo objeto dos veces se omite. Un `?` aislado en el cuerpo de una definición general es el uso que se está expandiendo. Se da una indicación cuando un punto restringido de un uso no está ligado a nada dibujado. En 0.3, PLACE y las restricciones sobre un uso sin símbolo fallaban | 14.1, 9.4, 12.1, 6.2, 6.5, 3.3, 7.1, 8.2, 11.1 |
| Trazado selectivo: ROUTE se reemplaza por ROUTE-NET (una red entera, como hacía ROUTE), ROUTE-OBJECT (los pines de algunos objetos, red por red), UNROUTE-NET y UNROUTE-OBJECT (pines dejados fuera). Cada pin se decide por la sentencia de mayor prioridad que lo cubre y, en igualdad, por la última. Los árboles en una misma red que comparten un pin o están unidos por cables escritos forman un solo árbol. Un pin en un árbol cuenta como `wired` | 13.7, 10.1, 10.2, Anexo A |
| El solucionador puede dividir un cable escrito por el autor para ramificar desde su parte media, en una red que un árbol une; reescribe el cable en su registro y conserva los puntos del autor en orden | 13.1, 13.7 |
| Los grupos no son objetos: no hay posición ni restricciones sobre un grupo. Los miembros de un grupo son los objetos de su conjunto, hasta los más bajos; un uso dibujado con un símbolo es un miembro él mismo; ROUTE-OBJECT sigue también sus roles y toma además sus propios pines. El árbol de grupos, y mantener fuera de la región de un grupo a los ajenos, pertenecen al solucionador como criterios después de todo hecho. GROUP toma `spacing_x` y `spacing_y`, el espaciado interior del grupo en unidades de cuadrícula | 11.5, 13.7 |
| SELECT-INSTANCE toma la condición `terminals`, el número de terminales de la instancia, de modo que, por ejemplo, los transistores MOS con tres y con cuatro terminales obtienen símbolos distintos | 10.1, Anexo A |
| La primitiva `tline` de Spectre tiene los nombres de terminal predeterminados `p1`, `n1`, `p2` y `n2`, como tiene T en SPICE; la tabla de familias de modelos añade maestros y la familia MESFET | 6.12 |
| Una instancia a la que ninguna sentencia ni regla da un símbolo tiene el símbolo de bloque por defecto: un recuadro cuadrado centrado en su origen, con un pin por cada puerto, o por cada nodo de una instancia cuya definición no se lee; ninguna instancia queda sin dibujar, y un visor ya no dibuja una figura sustituta propia | 9.2.6, 9.2.8 |
| El campo `language` del encabezado del registro hace que un registro actúe solo sobre los objetos en secciones de ese lenguaje de netlist; un archivo puede tener un registro de encabezado para cada lenguaje además del compartido. La serie estándar 0.4 lo usa para dar a las secciones SPICE de un archivo Spectre las reglas de SPICE; la serie 0.4 no descarta implícitamente el paquete del otro dialecto, como hacía la serie 0.3 | 2.8, 2.2, 6.12, 16.3, Anexo C |
| El lenguaje en que comienza un archivo es una entrada para la herramienta, por defecto a partir de su sufijo, y el usuario puede cambiarlo para cada archivo | 2.2 |
| Las reglas se aplican por defecto a su propio nivel: `recursive` es `false` por defecto, y una regla con una selección escrita en ella alcanza los niveles inferiores solo con `recursive=true` en una de esas selecciones, y una regla sin ninguna selección alcanza hasta donde lleguen los conjuntos que utiliza; cada selección y conjunto de la regla selecciona entonces en cada nivel. En 0.3 cada selección tenía su propio `recursive`. Una regla en un paquete cuenta como escrita en el nivel de su USE-PACKAGE, y un paquete en la raíz en el nivel superior de su archivo, de modo que las reglas del paquete estándar escriben `recursive=true`; una regla de generación advierte sobre objetos de su nivel solo cuando alcanza los niveles inferiores. Una regla que alcanza los niveles inferiores y cuyas otras ranuras nombran objetos del anfitrión de su nivel no surte efecto en los niveles inferiores, sin advertencia | 10.1, 10.3, 10.4, 10.5, 6.1, 8.3, 16.2, 4.5, Anexo B, Principios 2.5 |
| SOLVER-CONFIG guarda los ajustes del solucionador en el archivo, o en el registro de encabezado de un paquete para los niveles que lo usan: parámetros nombrados cuyas claves define el solucionador, de los cuales el estándar solo nombra `spacing_x`, `spacing_y` y `junction`, la definición de los puntos de unión con que dibuja todo solucionador; los ajustes posteriores sustituyen a los anteriores clave por clave; el núcleo pasa los ajustes al solucionador tal como están escritos, en orden; un valor que no se admite hace fallar solo su clave, y se conserva el valor anterior; los presupuestos de tiempo o de esfuerzo los da solo la herramienta | 14.10, 14.8, Anexo C |
| CURRENT no es un hecho geométrico, sino solo una indicación para el solucionador; ORDER compara puntos, y mantener apartados los marcos de los símbolos es un criterio del solucionador; ORDER es abreviatura de un DISTANCE con `min=1` entre vecinos; un renderizador sin resultado dibuja el primer código de una lista ORIENT | 15.2, 14.3, 11.2 |
| Aclarado: un TEXT con contenido vacío no dibuja nada; el solucionador reemplaza por entero los registros generados, entienda o no sus sentencias; los terminales de una agrupación corresponden por nombre, y los miembros que difieren en sus nombres de terminal o en su modelo producen una advertencia | 13.3, 2.8, 11.8 |
| Los pines son objetos, de clase `pin`, separados de los terminales. Un pin lo crea PIN en un símbolo y se nombra por su nombre local, `#X1::vdd_t`; puede omitirse `terminal=`. Un terminal puede tener varios pines; donde se necesita un pin, un terminal representa a su pin, el último escrito. Los pines ya no se reemplazan entre sí por terminal. Un nombre de pin no puede coincidir con un nombre de rol, un nombre local, un nombre de subobjeto estándar ni una clave de un texto; `::` busca los nombres de pin antes que las claves de los textos | 9.2.4, 9.2.7, 5.1, 6.4, 6.5, 13.3 |
| SET-SYMBOL empareja los nombres de terminal del símbolo con los terminales del objeto por nombre, o por lugar en el caso de los números; `terminals=` cambia el emparejamiento, listado en el orden de los nodos o como pares | 11.3 |
| Los usos no tienen terminales: `%g:p` de 0.3 es ahora un pin nombrado como `%g::pin`. Los pines de un uso vinculan roles por nombre y se dibujan sobre lo que los rellena; no son terminales eléctricos | 12.1, 6.4 |
| Los objetos dados para otra clase se convierten solo mediante una tabla: hacia el dibujo, terminal a pin a punto y objeto dibujado a posición; hacia el circuito, pin a terminal y pin o terminal a red | 5.2.4 |
| SELECT-PIN selecciona pines; `wired` pertenece a pines y redes; SELECT-TERMINAL selecciona todo terminal, tenga o no pin, y los recorridos por terminales los cuentan todos. SELECT-INSTANCE y SELECT-USAGE admiten `symbol` y `drawn`. RECTANGLE, CIRCLE y ELLIPSE escritos con un tamaño toman el resultado de una selección uno a uno; la condición `type` admite el comodín `*`, y `type="*"` selecciona las instancias que tienen un tipo | 10.1, 10.2, 8.2, 9.2.5, 13.3 |
| Aserciones: ASSERT-INSTANCE, ASSERT-TERMINAL, ASSERT-NET y ASSERT-USAGE comprueban que los miembros cumplan las condiciones de la selección de su clase, y ASSERT-USAGE también que sean usos de una definición; ASSERT-IN comprueba que los miembros estén en un conjunto. Comprueban que los atributos estén escritos (`has`) y que sean iguales (`same`) o distintos (`different`); ASSERT-TOPOLOGY comprueba que los miembros estén conectados en serie, en paralelo, en estrella o como un único bucle, contando un uso o un conjunto como una caja negra; una comprobación que no se cumple da una violación, una cuarta clase de informe además de error, advertencia e indicación, y no hace fallar ninguna sentencia. Las sentencias CURRENT de una misma red comprueban en conjunto que la corriente no fluya toda en un solo sentido; en 0.3 no se comprobaba nada | 15.2, 15.3, 15.4, 7.6, Anexo C |
| TABLE dispone objetos como una tabla de `{…}` anidados, filas de arriba abajo (o con `transpose=true`, columnas de izquierda a derecha), con cada lista anidada en ángulo recto; un único hecho, y sus objetos un grupo | 14.11 |
| Texto: modo matemático `\(…\)` con un subconjunto de matemáticas LaTeX; `\subrest{…}` y `\subdigits{…}` muestran el resto de un nombre, o sus dígitos finales, como subíndice; TEXT admite `fix_rotate`, que mantiene la dirección de los caracteres mientras el lado del cuadro de texto sigue la orientación del objeto | 13.6, 13.3 |
| Los alias terminan en cualquier carácter que no sea letra, dígito o guion bajo, en cualquier escritura; ninguna regla nombra puntuación china. El paquete de alias en chino es `alias-zh`. Las sentencias estándar se dan solo en inglés | 16.5, 4.5 |
| La lista de palabras añade `removed`, las palabras que 0.4 ya no tiene; el esquema JSON añade `open_params` y `removed`, y `at_least_one` pasa a ser una lista de grupos, como lo son `together` y `exclusive`. | 5.3 |
| El nodo vacío, `&` solo, indica que no hay red: un rol de clase `net` y la ranura de red de WIRE lo admiten, para que los componentes virtuales puedan dibujar un diagrama de bloques que no tenga líneas de netlist; no es un objeto, y dos lugares que lo escriben no tienen nada en común | 6.3, 9.4, 12.1, 13.1 |
| En un registro de nivel, `junction` de SOLVER-CONFIG puede nombrar una definición `%` de ese nivel, para que una variante de punto de unión definida para un subcircuito permanezca en él | 13.2, 14.10 |
| `$NO_CONNECT` admite un terminal como tal, para que una marca en un miembro de un bus indique qué terminal queda abierto | 12.6 |
| TEXT: `rotate` admite cualquier ángulo; `upright` (por defecto `true`) gira un texto para su lectura, y con `false` los caracteres siguen la rotación y la reflexión, para signos como flechas; la propiedad de estilo de texto `textvisible` oculta un texto, que entonces no ocupa espacio mientras su contenido se conserva | 9.2.5, 9.3, 13.3 |
| ANNOTATION reúne los gráficos de un símbolo, una definición general o un uso en una única anotación, en un punto o sobre un objeto, girada cualquier ángulo; los pines y lo demás quedan fuera, así como los gráficos de un cuerpo ligados a otros objetos | 13.5 |
| Una implementación puede fijar límites sobre los recursos, como la profundidad de anidamiento o el número de objetos; una entrada más allá de un límite produce un error de la implementación, nunca un resultado truncado en silencio; se admiten al menos 64 niveles de anidamiento de símbolos | 1.6 |
| Una implementación fija un límite al tiempo o al trabajo que dedica a comprobar hechos; este estándar no fija cómo se mide el límite | 8.5 |
| DEVICE toma `type`, lo que da a una instancia a la que el anfitrión no da ningún tipo, como un dispositivo de PDK, el tipo que leen las selecciones y las aserciones; `name` o `model`, y `terminals` o `type`, pueden omitirse individualmente pero no ambos, y para los nombres y el tipo por separado cuenta el último DEVICE coincidente que los escriba. Los nombres de terminal predeterminados del `transformer` de Spectre son `p1 n1 p2 n2` | 6.12 |
| El resultado de una selección sigue un orden definido: el orden de la escena, o miembro del ámbito por miembro del ámbito sin repeticiones, con los objetos creados por reglas después de los escritos | 10.3 |
| Los pines apuntan en una dirección en el dibujo, la cual restringen ORIENT y ORIENT-RELATION escritos sobre pines; ORIENT admite las palabras `left`, `down`, `right` y `up` para R0, R90, R180 y R270 | 9.2.4, 11.2, 14.9 |
| Un módulo de Verilog-A incorporado con `ahdl_include` o `.hdl` puede dar a sus instancias un símbolo de bloque, escrito en registros `//` dentro del módulo; el módulo es un nivel sin circuito que solo contiene símbolos y estilos, y un puerto vectorial es un terminal por cada bit, en el orden de su intervalo declarado | 2.9, 2.2, 6.1, 9.2.6, 17 |

**Paso de un archivo de 0.3 a 0.4.** Este estándar no describe archivos de la versión 0.3; una herramienta de actualización que lee la versión 0.3 los traslada (16.1). Eleva la versión en el registro de encabezado, pasa el archivo a la serie estándar 0.4 y realiza los cambios siguientes; lo que no puede cambiar, lo informa para el autor:

- `"x"` o `"y"` como dirección de ORDER o DISTANCE, y `axis=` para DISTRIBUTE: elegir una dirección.
- Los nombres de terminal `p_plus`, `p_minus`, `s_plus` y `s_minus` de un `transformer` de Spectre: escribir `p1`, `n1`, `p2` y `n2`.
- Conjuntos en ambos lados de ORIENT-RELATION: escribir un hecho por cada par.
- Reglas sin `recursive`: una herramienta escribe `recursive=true` en sus selecciones, en el archivo, sus propios archivos de paquete y sus paquetes en línea, para que sigan alcanzando los niveles inferiores. Una regla que escribe `recursive=false` en algunas de sus selecciones y no en otras se informa: en 0.4, un solo `true` hace que toda la regla alcance los niveles inferiores.
- ROUTE: una herramienta escribe ROUTE-NET en su lugar.
- WIRE o BUNDLE al que se pasa el resultado de una selección: escribir ROUTE-OBJECT para los cables, y los miembros de una agrupación uno por uno.
- `%g:p` y otros terminales de usos: escribir el pin nombrado del símbolo, como `%g::pin`. Los símbolos de la serie 0.3 vinculan los roles de los usos por número, lo cual 0.4 no hace (11.3).
- `$GROUND @ATTACH #M5:3` y otros componentes virtuales cuyo primer rol es un terminal o un pin: ahora se sitúan en esa red en una posición propia; para mantenerlos sobre el pin, escribir un tramo corto, `$GROUND_STUB @ATTACH #M5:3`, o restricciones que pongan el pin de la tierra sobre el pin de M5 (12.3).
- `wired=` en SELECT-TERMINAL: escribir SELECT-PIN, como `# @SELECT-INSTANCE => @SELECT-PIN(wired=false)`.
- `{%x}` contado por su centro, y `::position` de un conjunto, que era su centro: un conjunto no tiene posición en 0.4, y su `::position` es el conjunto de las posiciones de sus miembros; la actualización deja tal sentencia para revisión. Para colocar los objetos como uno solo, háganse miembros de un uso cuyo cuerpo los ligue a su propio punto, y restrínjase `%u::position` (14.1).
- Un uso en la lista de miembros de una restricción, que contaba por un punto: la actualización escribe `%u::position` para un uso sin símbolo, que de otro modo representaría a sus miembros (14.1).
- DISTANCE entre dos conjuntos: ahora se cumple para cada par, de modo que un `max` acota cada par y no la separación entre extensiones.
- Coordenadas literales en el cuerpo de una definición general: son desplazamientos desde el propio punto del uso; una definición que aludía a las coordenadas del nivel coloca sus usos en `(0, 0)`, como `$TITLE @ATTACH => @PLACE(0, 0)`.
- El paquete estándar: escribir `@USE-PACKAGE(name="rnl-std-all", version="0.4")`, y reescribir los usos de la serie 0.3 como indica "La serie estándar 0.4" más abajo: los nuevos nombres y roles, una etiqueta de puerto `$IO_PORT @ATTACH` por cada red de puerto que etiquetaba el relleno de 0.3, y un informe por cada uso de una definición que fue eliminada.

**La serie estándar 0.4.** El paquete estándar no forma parte de este estándar (16.3); lo que sigue es lo que importa a un archivo que pasa de la serie 0.3 a la serie 0.4.

- **Uso**: un archivo escribe `@USE-PACKAGE(name="rnl-std-all", version="0.4")` en su registro de encabezado, o se lee con un ajuste de la herramienta que lo añade (16.3).
- **Puertos**: el relleno da a cada red de puerto una etiqueta de puerto, también a las redes de alimentación y de tierra, en una columna al menos 6 unidades a la izquierda de las instancias del nivel, cada etiqueta en R0; un archivo saca redes de `$PORT_FILL_SET`, escribe `$IO_PORT @ATTACH` para una etiqueta propia, o `$PORT_LIST @ATTACH` para la columna escrita explícitamente (12.4).
- **El relleno**: añade etiquetas de red, cada una un tramo corto, a los pines sin cables; ninguna red se dibuja con cables salvo que un archivo o un paquete lo indique con ROUTE-NET o ROUTE-OBJECT; las redes de tierra y de alimentación se dejan fuera con `@UNROUTE-NET(priority="high")`, y un archivo que quiera dibujarlas escribe `@ROUTE-NET(priority="high")` o sentencias WIRE. El relleno no añade ningún símbolo de tierra ni de alimentación a pines que tengan cables, tanto si el archivo los cablea a mano como si los dibuja con ROUTE-NET o ROUTE-OBJECT, escritos como sentencias o como reglas; un archivo que quiera uno lo adjunta, como `$GROUND @ATTACH &vss` para una red dibujada entera o `$GROUND_STUB @ATTACH #R1:2` sobre un pin. Sus reglas escriben `recursive=true` y seleccionan pines, incluidos los de componentes virtuales. Las redes de `$BIAS_NET_SET` reciben marcas de polarización igual que las redes de tierra y de alimentación reciben las suyas.
- **Los paquetes**: la única colección es `rnl-std-all`, que referencia todos los paquetes de la serie y tiene un registro de encabezado para cada lenguaje (2.8, 16.3); las colecciones `rnl-std`, `rnl-std-core` y `rnl-std-full` de la serie 0.3 desaparecen, y `rnl-std-structures` pasa a ser `rnl-std-semantics`. `rnl-std-no-labels` selecciona con `drawn=true` y oculta etiquetas con el estilo `textvisible=false` en lugar de textos vacíos, de modo que un archivo vuelve a mostrar una con `textvisible=true`. El paquete opcional `rnl-std-45deg` es nuevo.
- **Renombrados o cambiados**: `$DIFFERENTIAL_PAIR` pasa a ser `$DIFFERENTIAL_PAIR_N` y `$DIFFERENTIAL_PAIR_P`, con los roles `left`, `right` y `tail`; `$MIRRORED` pasa a llamarse `$MIRRORED_LAYOUT`, con los roles `left`, `right` y un `axis`, y la nueva `$MIRRORED_VERTICAL_LAYOUT` tiene `top`, `bottom` y `axis`, ya que una definición que únicamente coloca termina en `_LAYOUT`; las estructuras `_N` y `_P` afirman la polaridad de sus transistores; `$FLOW` pasa a ser `$SIGNAL_FLOW`, que no coloca nada, no toma dirección y puede nombrar las fuentes y las cargas de su camino, y `$POWER_FLOW` y `$BIAS_CIRCUIT` son nuevos; `$CURRENT_MIRROR` pasa a ser `$CURRENT_MIRROR_N` y `$CURRENT_MIRROR_P`; el rol de `$NO_CONNECT` admite un terminal o un pin. La lista completa de definiciones que la serie renombra, fusiona, divide o elimina figura en la descripción del paquete estándar; la orden `rnl upgrade` de la última versión publicada del núcleo de RNL 0.5.x (una versión de la herramienta, no de este lenguaje) reescribe archivos conforme a ella. Todo hecho de posición en las estructuras declara su dirección, y el otro dibujo es una variante propia, como `$BRANCH_HORIZONTAL`.
- **Eliminados**: `$CHAIN`, `$SUPPLY_HIGH` y `$SUPPLY_LOW`.
