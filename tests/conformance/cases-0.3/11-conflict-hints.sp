* The hints of a conflict of geometric facts in a file of 0.3
*! When geometric facts conflict, the one dropped by the order of 8.5 gets a hint listing each group of kept facts it conflicts with; each statement in the conflict gets a hint too, listing the facts dropped because of it (8.5).
*! R1's x is written 0 and then 10, and the later is kept; R2's orientation is written twice, and the later is kept. %row is written first, and comes after the coordinates in the order of 8.5:
*! R1 and R2 are given different y, so with those two y it has no solution and is dropped, with a hint listing that group.
* @RNL(type="header") { @VERSION("0.3")}
R1 a b 1k
R2 b 0 1k
* @RNL {
*   %row = {#R1 #R2} @ALIGN("y");
*   #R1 @PLACE(0, 0);
*   #R1 @PLACE(x=10);
*   #R2 @PLACE(0, -8) => @ORIENT("R90");
*   #R2 @ORIENT("MY")
* }
.end
