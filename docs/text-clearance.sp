* Synthetic text clearance
* @RNL(type="header") { @VERSION("0.4") }
R1 a b 1k
R2 a c 2k
R3 a d 3k
R4 b d 4k
* @RNL {
* $S = @DEFINE-SYMBOL { (-2,0) @PIN(terminal=1,direction="left"); (2,0) @PIN(terminal=2,direction="right"); (-2,0) (-1,0) @LINE; (1,0) (2,0) @LINE; (-1,-1) (1,1) @RECTANGLE; ? @TEXT("${name} = ${value}", key="name") => @PLACE(1, 1.1) };
* {#R1 #R2 #R3 #R4} @SET-SYMBOL $S => @ORIENT("R0");
* {#R1 #R2} @GROUP;
* {#R3 #R4} @GROUP;
* {&a &b &d} @ROUTE-NET;
* }
.end
