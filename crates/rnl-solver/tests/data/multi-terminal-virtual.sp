* Virtual role terminals retain their own physical pins.
* @RNL(type="header") { @VERSION("0.3") }
R1 na nb 1k
* @RNL {
*   $PAIR_SYMBOL = @DEFINE-SYMBOL(box=[-1,-1,1,1]) {
*     (-1,0) @PIN(terminal="a", direction="left");
*     (1,0) @PIN(terminal="b", direction="right");
*     @RECTANGLE {(-1,-1) (1,1)}
*   };
*   $PAIR = @DEFINE(roles=["a","b"]) {};
*   %pair = $PAIR @ATTACH &na &nb => @SET-SYMBOL $PAIR_SYMBOL
* }
.end
