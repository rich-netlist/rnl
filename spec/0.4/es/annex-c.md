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
