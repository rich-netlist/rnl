* Label positions axis by axis: what the scene writes replaces what the symbol gives
*! A symbol places its own labels: $TAG's name at (1, 1), its value at (1, -1). A PLACE or an offset (dx, dy) written in the scene for the same reference name replaces it axis by axis (13.3):
*! each axis takes the one written later; an axis with neither keeps the position the symbol gives, turning with the element. An offset is along the scene's axes and does not turn with the element.
*! R1's name has a PLACE of x only; R2 is turned R90 and its name has dx only; R3's name has an offset and then a PLACE of x; R4's value has a PLACE and then dy; R5 has nothing written and follows the symbol.
*! R6's name has dx and dy, as an editor commonly exports labels; a reference name the symbol lacks (R6's note) is placed by its offset as before.
R1 a1 b1 1k
R2 a2 b2 2k
R3 a3 b3 3k
R4 a4 b4 4k
R5 a5 b5 5k
R6 a6 b6 6k
* @RNL(type="header") { @VERSION("0.2")}
* @RNL {
*   $TAG = @DEFINE-SYMBOL(box=[-2, -0.5, 2, 0.5]) {
*     (-2, 0) @PIN(terminal=1, direction="left"); (2, 0) @PIN(terminal=2, direction="right");
*     @LINE {(-2, 0) (2, 0)};
*     ? @TEXT("${name}", key="name") => @PLACE(1, 1);
*     ? @TEXT("${value}", key="value") => @PLACE(1, -1)
*   };
*   # @SELECT-INSTANCE => @SET-SYMBOL $TAG;
*   #R1 @PLACE(0, 0); #R2 @PLACE(10, 0) => @ORIENT("R90"); #R3 @PLACE(20, 0);
*   #R4 @PLACE(0, -10); #R5 @PLACE(10, -10); #R6 @PLACE(20, -10);
*   #R1::name @PLACE(x=-3);
*   #R2 @TEXT("${name}", key="name", dx=3);
*   #R3 @TEXT("${name}", key="name", dx=1, dy=2);
*   #R3::name @PLACE(x=25);
*   #R4::value @PLACE(1, -15);
*   #R4 @TEXT("${value}", key="value", dy=-3);
*   #R6 @TEXT("${name}", key="name", dx=-1, dy=1.5);
*   #R6 @TEXT("note", key="note", dx=-1, dy=-1.5)
* }
.end
