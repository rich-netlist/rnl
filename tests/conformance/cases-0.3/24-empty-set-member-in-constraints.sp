* Empty sets written as members of ALIGN, ORDER and DISTRIBUTE (14.1, 14.2-14.4, 8.5)
*! 14.1: "A member of ORDER, ALIGN or DISTRIBUTE that is a set counts as one member,
*!   by its centre, which is its `::position`... An empty set is not a member."
*! 14.2-14.4, 8.5: Because `{}` and `{%empty}` are not members:
*!   - `%dist_two` has 2 members (`#R1`, `#R2`), fewer than 3, so it holds trivially ("ok");
*!   - `%align_ok` has 2 members (`#R1`, `#R2`) at y=0, so it holds ("ok");
*!   - `%order_ok` has 2 members (`#R1` at x=0, `#R2` at x=10), so it holds ("ok");
*!   - `%align_drop` has 2 members (`#R1` at x=0, `#R2` at x=10): it comes before their
*!     coordinates in the order of 8.5, so it needs a solver ("needs-solver"), with a hint;
*!   - `%order_self` has 2 consecutive identical members (`#R1`, `#R1`), so it
*!     contradicts itself and fails ("failed") with an error (8.5).
* @RNL(type="header") {
*   @VERSION("0.3")
* }

R1 a b 1k
R2 a b 2k

* @RNL {
*   #R1 @PLACE(0, 0) => @ORIENT("R0");
*   #R2 @PLACE(10, 0) => @ORIENT("R0");
*   %empty = # @SELECT-INSTANCE(name="NO_SUCH_*");
*
*   %dist_two = {#R1 {} #R2} @DISTRIBUTE("x");
*   %align_ok = {#R1 {%empty} #R2} @ALIGN("y");
*   %order_ok = {#R1 {} #R2} @ORDER("left-to-right");
*   %align_drop = {#R1 {} #R2} @ALIGN("x");
*   %order_self = {#R1 {} #R1} @ORDER("x")
* }
.end
