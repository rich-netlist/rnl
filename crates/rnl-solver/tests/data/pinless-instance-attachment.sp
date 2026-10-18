* Fresh source-review fixture: a drawn non-marker usage attached to an instance origin.
* This file has not been interpreted or solved by the reviewer.
* @RNL(type="header") { @VERSION("0.3") }
.subckt block p
.ends block
XH aux block
XA n block
XB n block
* @RNL {
*   $PIN_SYMBOL = @DEFINE-SYMBOL(box=[-0.5,-0.5,0.5,0.5]) {
*     (0,0) @PIN(terminal="p", direction="right");
*     @RECTANGLE {(-0.5,-0.5) (0.5,0.5)}
*   };
*   $BADGE_SYMBOL = @DEFINE-SYMBOL(box=[-2,3,2,5]) {
*     @RECTANGLE {(-2,3) (2,5)}
*   };
*   $BADGE = @DEFINE(roles=["p"]) {};
*   #XH @SET-SYMBOL $PIN_SYMBOL => @PLACE(0,0) => @ORIENT("R0");
*   #XA @SET-SYMBOL $PIN_SYMBOL => @PLACE(-5,4) => @ORIENT("R0");
*   #XB @SET-SYMBOL $PIN_SYMBOL => @PLACE(5,4) => @ORIENT("R180");
*   %badge = $BADGE @ATTACH #XH => @SET-SYMBOL $BADGE_SYMBOL => @ORIENT("R0");
*   &n @ROUTE
* }
.end
