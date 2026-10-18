* Distinct physical pins coincide: the budget fallback must retain their identities.
* @RNL(type="header") { @VERSION("0.3") }
R1 a b 1k
R2 a b 1k
* @RNL {
*   $S = @DEFINE-SYMBOL(box=[-1,-1,1,1]) {
*     (-1,0) @PIN(terminal="p", direction="left");
*     (1,0) @PIN(terminal="n", direction="right");
*     @RECTANGLE {(-1,-1) (1,1)}
*   };
*   #R1 @SET-SYMBOL $S; #R2 @SET-SYMBOL $S;
*   #R1 @PLACE(0,0); #R1 @ORIENT("R0");
*   #R2 @PLACE(0,0); #R2 @ORIENT("R0");
*   #R1:p @CURRENT("in"); #R1:n @CURRENT("out");
*   #R2:p @CURRENT("in"); #R2:n @CURRENT("out");
*   &a @ROUTE
* }
.end
