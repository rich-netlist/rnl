* Junction duplicate probe: no bundled standard
* @RNL(type="header") { @VERSION("0.4"); @SOLVER-CONFIG(junction="$DOT") }
R1 n a 1k
R2 n b 1k
R3 n c 1k
* @RNL {
*   $S = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1) };
*   $DOT_SYMBOL = @DEFINE-SYMBOL(box=[0,0,0,0]) { (0,0) @CIRCLE(radius=0.4, fill="linecolor") };
*   $DOT = @DEFINE(roles=[["p","point"]]) { {? ?p} @ALIGN("x"); {? ?p} @ALIGN("y") };
*   # @SELECT-USAGE $DOT => @SET-SYMBOL $DOT_SYMBOL;
*   {#R1 #R2 #R3} @SET-SYMBOL $S;
*   #R1 @PLACE(-6,0); #R2 @PLACE(6,0); #R3 @PLACE(0,1);
*   %p = (0,0);
*   &n @WIRE(mode="manhattan") {#R1:1 %p};
*   &n @WIRE(mode="manhattan") {%p #R2:1};
*   &n @ROUTE-NET
* }
.end
