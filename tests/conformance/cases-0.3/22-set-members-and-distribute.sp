* Sets as constraint members by centre, and DISTRIBUTE with fewer than three members (14.1, 14.4)
*! 14.1, 6.5: A member of ORDER, ALIGN or DISTRIBUTE that is a set counts as one
*!   member by its centre (`::position`), the middle of the extent of its innermost
*!   members. Here `%pair = {#R1 #R2}` has x in [0, 4] and y in [-2, 2], so its
*!   centre is (2, 0), and nested `{%nested}` with `#R3` at (6, 0) has x in [0, 6]
*!   so its centre is (3, 0).
*! 14.4: With fewer than three members (0, 1, or 2 members), DISTRIBUTE holds trivially.
* @RNL(type="header") {
*   @VERSION("0.3")
* }

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
*   %align_centre = {{%pair} #R4} @ALIGN("x");
*   %order_centre = {#R1 {%pair} #R3} @ORDER("left-to-right");
*   %dist_centre = {{%pair} #R3 #R5} @DISTRIBUTE("x");
*   %align_nested = {{%nested} (3, -5)} @ALIGN("x");
*
*   %dist_two = {#R1 #R2} @DISTRIBUTE("x");
*   %dist_one = {#R1} @DISTRIBUTE("x");
*   %dist_zero = {} @DISTRIBUTE("x")
* }
.end
