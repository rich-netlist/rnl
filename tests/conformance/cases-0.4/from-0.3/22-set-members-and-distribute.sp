* Collection item expansion and DISTRIBUTE boundaries in 0.4
*! Collection items expand to combinations, without a collection centre. Nested and repeated items exercise expansion; repeated R1 produces a self-contradiction. Zero and one surviving items produce too-few-places hints.
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-all", version="0.4");
*   @VERSION("0.4")
* }
* @RNL { # @SELECT-PIN(recursive=true) => @REMOVE-FROM $LABEL_FILL_SET; $SUPPLY_NET_SET @REMOVE-FROM $PORT_FILL_SET; $GROUND_NET_SET @REMOVE-FROM $PORT_FILL_SET }

R1 a b 1k
R2 a b 2k
R3 a b 3k
R4 a b 4k
R5 a b 5k

* @RNL {
*   #R1 @PLACE(0, -2) => @ORIENT("R0");
*   #R2 @PLACE(4, 2) => @ORIENT("R0");
*   #R3 @PLACE(6, 0) => @ORIENT("R0");
*   #R4 @PLACE(2, 10) => @ORIENT("R0");
*   #R5 @PLACE(10, 0) => @ORIENT("R0");
*
*   %pair = {#R1 #R2};
*   %nested = {{%pair} #R3};
*
* %align_pair = {{%pair} #R4} @ALIGN("x");
* %order_pair = {#R1 {%pair} #R3} @ORDER("left-to-right");
* %dist_pair = {{%pair} #R3 #R5} @DISTRIBUTE("left-to-right");
* %align_nested = {{%nested} (3, -5)} @ALIGN("x");
*
* %dist_two = {#R1 #R2} @DISTRIBUTE("left-to-right");
* %dist_one = {#R1} @DISTRIBUTE("left-to-right");
* %dist_zero = {} @DISTRIBUTE("left-to-right")
* }
.end



