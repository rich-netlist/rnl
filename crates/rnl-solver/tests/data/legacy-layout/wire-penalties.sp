* Controlled v2 Step1 score fixture; positions are explicit, no layout search choice is asserted.
* @RNL(type="header") { @VERSION("0.2") }
R1 a b 1k
R2 c d 1k
* @RNL {
*   $CONTROLLED_BODY = @DEFINE-SYMBOL(box=[-1,-1,1,1]) {
*     (0,0) @PIN(terminal="p", direction="right")
*   };
*   #R1 @SET-SYMBOL $CONTROLLED_BODY;
*   #R2 @SET-SYMBOL $CONTROLLED_BODY;
*   #R1 @PLACE(0,0) => @ORIENT("R0");
*   #R2 @PLACE(10,0) => @ORIENT("R0");
*   &a @WIRE {#R1:p (-2,2)};
*   &c @WIRE {(-2,0) (2,0)}
* }
.end
