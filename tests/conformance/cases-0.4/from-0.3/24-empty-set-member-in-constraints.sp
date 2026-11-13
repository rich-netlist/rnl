* Empty collection items in 0.4 constraints
*! Empty items are omitted; R1 and R2 remain for distribution, alignment and order. The x alignment needs a solver; repeated R1 in ORDER fails as self-contradicting.
* @RNL(type="header") {
*   @USE-PACKAGE(name="rnl-std-all", version="0.4"); @VERSION("0.4")
* }

R1 a b 1k
R2 a b 2k

* @RNL {
*   #R1 @PLACE(0, 0) => @ORIENT("R0");
*   #R2 @PLACE(10, 0) => @ORIENT("R0");
*   %empty = # @SELECT-INSTANCE(name="NO_SUCH_*");
*
*   %dist_two = {#R1 {} #R2} @DISTRIBUTE("left-to-right");
*   %align_ok = {#R1 {%empty} #R2} @ALIGN("y");
*   %order_ok = {#R1 {} #R2} @ORDER("left-to-right");
*   %align_drop = {#R1 {} #R2} @ALIGN("x");
*   %order_self = {#R1 {} #R1} @ORDER("left-to-right")
* }
.end


