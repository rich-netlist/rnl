* A configurable net-attached symbol, without an ORIENT fact for that symbol.
* @RNL(type="header") { @VERSION("0.3") }
R1 a b 1k
* @RNL {
*   $HOST_SYMBOL = @DEFINE-SYMBOL(box=[-2,-1,2,1]) {
*     (-2,0) @PIN(terminal=1, direction="left");
*     (2,0) @PIN(terminal=2, direction="right");
*     @RECTANGLE {(-1,-1) (1,1)}
*   };
*   $TAG_SYMBOL = @DEFINE-SYMBOL(box=[-1,-1,1,1]) {
*     (0,1) @PIN(terminal="p", direction="up");
*     @RECTANGLE {(-1,-1) (1,0)}
*   };
*   $TAG = @DEFINE(roles=["p"]) {};
*   #R1 @SET-SYMBOL $HOST_SYMBOL;
*   #R1 @ORIENT("R0");
*   %tag = $TAG @ATTACH &a => @SET-SYMBOL $TAG_SYMBOL;
*   &a @ROUTE
* }
.end
