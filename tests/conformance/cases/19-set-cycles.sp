* Cycles of sets: the statement that closes a cycle fails, and the rest is drawn
* @RNL(type="header") { @VERSION("0.2")}
*! A cycle of set dependencies is an error (5.2.2): in the order of position, the statement that closes the cycle fails, the diagnostic lists the whole cycle, the other statements take effect as usual, and the interpretation as a whole does not fail.
*! %row is added to %col and then %col to %row: the latter fails, %row still has only R1 and R2, and the alignment holds; %b = {%a} and then %b @REMOVE-FROM %a: the removal fails, and C1 is placed as usual.
*! Two package sets both with selections: the top level's rule adds $LEFT to $RIGHT, and the subcircuit cell adds $RIGHT to $LEFT. The cycle closes in cell, cell's statement fails, the top level is not affected, and there is no endless recursion any more.
R1 a b 1k
R2 b c 1k
R3 c 0 1k
C1 a 0 1p
X1 a c cell
.subckt cell p q
R1 p q 1k
* @RNL { $RIGHT @ADD-TO $LEFT; #R1 @PLACE(0, 0) }
.ends
* @RNL {
*   @USE-PACKAGE(name="cycles") { $LEFT = {# @SELECT-INSTANCE(name="R*")}; $RIGHT = {# @SELECT-INSTANCE(name="C*")} };
*   $LEFT @ADD-TO $RIGHT;
*   $RIGHT @SET-STYLE $grey;
*
*   %row = {#R1 #R2};
*   %col = {#R3};
*   %row @ADD-TO %col;
*   %col @ADD-TO %row;
*   %row @ALIGN("y");
*
*   %a = {#C1};
*   %b = {%a};
*   %b @REMOVE-FROM %a;
*   %a @PLACE(0, -6);
*
*   #R1 @PLACE(0, 0); #R2 @PLACE(6, 0); #R3 @PLACE(12, -3); #X1 @PLACE(20, 0)
* }
.end
