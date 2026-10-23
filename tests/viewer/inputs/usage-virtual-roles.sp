* Multiple roles on virtual symbols
* @RNL(type="header") { @VERSION("0.3") }
R1 a b 1k
R2 b c 2k
* @RNL {
*   $PAIR = @DEFINE(roles=["left", "right"]) {};
*   $CONNECTIONS = @DEFINE(roles=[["input", "net"], ["output", "net"]]) {};
*   $ROUTE = @DEFINE(roles=[["route", "wire"]]) {};
*   %box = @DEFINE-SYMBOL(box=[-2, -1, 2, 1]) { (0, 0) @RECTANGLE(width=4, height=2) };
*   #R1 @PLACE(0, 0); #R2 @PLACE(9, 0);
*   %pair = #R1(left) #R2(right) @AS $PAIR => @SET-SYMBOL %box;
*   %connections = &a(input) &c(output) @AS $CONNECTIONS => @SET-SYMBOL %box => @PLACE(15, 7);
*   %wire = &b @WIRE {(0, -6) (10, -6)};
*   %route = %wire @AS $ROUTE => @SET-SYMBOL %box => @PLACE(20, -6)
* }
.end
