* Ties: names equally close to an unknown one, and several overridden symbol statements
*! Every reading gives the same result: when a name not found has several equally close candidates, the hint takes the first in the order of names ($ABCV);
*! all six SET-SYMBOL $A are overridden by the later {...} @SET-SYMBOL $B, and recorded as without effect in the order written (7.5, 8.3).
R1 a1 b1 1k
R2 a2 b2 1k
R3 a3 b3 1k
R4 a4 b4 1k
R5 a5 b5 1k
R6 a6 b6 1k
R7 a7 b7 1k
* @RNL(type="header") { @VERSION("0.2")}
* @RNL {
*   $A = @DEFINE-SYMBOL {
*     (-3, 0) @PIN(terminal=1, label="a", direction="left"); (3, 0) @PIN(terminal=2, label="b", direction="right");
*     @RECTANGLE {(-1.5, -0.6) (1.5, 0.6)}
*   };
*   $B = @DEFINE-SYMBOL {
*     (-2, 0) @PIN(terminal=1, label="a", direction="left"); (2, 0) @PIN(terminal=2, label="b", direction="right");
*     @LINE {(-2, 0) (2, 0)}
*   };
*   $ABCX = @DEFINE-SYMBOL { (-2, 0) @PIN(terminal=1); (2, 0) @PIN(terminal=2); @LINE {(-2, 0) (2, 0)}};
*   $ABCY = @DEFINE-SYMBOL { (-2, 0) @PIN(terminal=1); (2, 0) @PIN(terminal=2); @LINE {(-2, 0) (2, 0)}};
*   $ABCW = @DEFINE-SYMBOL { (-2, 0) @PIN(terminal=1); (2, 0) @PIN(terminal=2); @LINE {(-2, 0) (2, 0)}};
*   $ABCV = @DEFINE-SYMBOL { (-2, 0) @PIN(terminal=1); (2, 0) @PIN(terminal=2); @LINE {(-2, 0) (2, 0)}};
*   #R1 @SET-SYMBOL $A;
*   #R2 @SET-SYMBOL $A;
*   #R3 @SET-SYMBOL $A;
*   #R4 @SET-SYMBOL $A;
*   #R5 @SET-SYMBOL $A;
*   #R6 @SET-SYMBOL $A;
*   {#R1 #R2 #R3 #R4 #R5 #R6} @SET-SYMBOL $B;
*   #R7 @SET-SYMBOL $ABCZ;
*   #R1 @PLACE(0, 0); #R2 @PLACE(8, 0); #R3 @PLACE(16, 0);
*   #R4 @PLACE(0, 8); #R5 @PLACE(8, 8); #R6 @PLACE(16, 8); #R7 @PLACE(24, 0)
* }
.end
