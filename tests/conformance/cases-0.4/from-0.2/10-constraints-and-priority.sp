* Constraints and priority without a solver
*! The four atomic constraints checked against fixed coordinates: satisfied, dropped by the order of precedence, the order of precedence before coordinates (the solver must move them), a member that is a free point (not checked), disabled;
*! the order in which priorities override: the record header's default < a statement's parameter < a PRIORITY change < an outer usage's priority < a PRIORITY written on a local object through ::;
*! PLACE dropped coordinate by coordinate, an ORIENT conflict, a PLACE with priority="disabled"; a high PLACE written by the editor after the solver overrides the solver's result; DISTANCE's exact is an exact distance.
R1 a b 1k
R2 b c 1k
R3 c d 1k
R4 d 0 1k
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-all", version="0.4"); @USE-PACKAGE(name="rnl-std-basic", version="0.4"); @VERSION("0.4")}
* @RNL {
*   %c1 = {#R1 #R2 #R3} @ALIGN("y");
* %c2 = {#R1 #R2 #R3 #R4} @DISTRIBUTE("left-to-right");
*   %c3 = {#R3 #R4} @ALIGN("y");
*   %c4 = {#R4 #R1} @ORDER("left-to-right");
*   %c4 @PRIORITY("disabled");
* %c5 = #R1 #R4 @DISTANCE("top-to-bottom", min=1, max=3);
*   %f = (_, _);
*   %c6 = {#R1 %f} @ALIGN("x");
*   %c8 = #R1 #R2 @DISTANCE("left-to-right", exact=8);
*
*   $PAIR_Y = @DEFINE(roles=["a", "b"]) { {?a ?b} @ALIGN("y", priority="low")};
*   %p1 = #R3 #R4 @AS $PAIR_Y;
*   $ROW = @DEFINE(roles=["{m}"]) {
*     %row = ?m @ALIGN("y", priority="low");
*     %ord = ?m @ORDER("left-to-right")
*   };
*   %r = {#R1 #R2 #R3} @AS(priority="low") $ROW;
*   %r::ord @PRIORITY("high");
*
*   #R1 @ORIENT("R0");
*   #R2 @PLACE(7, 0);
*   #R3 @PLACE(x=0, y=0, priority="disabled");
*   &b @WIRE(mode="auto") {#R1:2 #R2:1}; &c @WIRE(mode="auto") {#R2:2 #R3:1}; &d @WIRE(mode="auto") {#R3:2 #R4:1}
* }
* @RNL(priority="low") {
*   %c7 = {#R2 #R4} @ALIGN("x")
* }
* 

* @RNL(type="generated", priority="high") { #R1 @PLACE(0, 0) => @ORIENT("R90"); #R2 @PLACE(8, 0) => @ORIENT("R90"); #R3 @PLACE(16, 0) => @ORIENT("R90"); #R4 @PLACE(24, 2); %ground = $GROUND @ATTACH #R4:2::net => @PLACE(24, -7); #R4:2::net @WIRE(mode="auto") {#R4:2 %ground:p}; %net_label_stub_R1_p = $NET_LABEL_STUB @ATTACH #R1::p; %net_label_stub_R1_p @PLACE(0,0); %net_label_stub_R1_p::l @PLACE(-2,0) => @ORIENT("R180") }
* @RNL(author="rnl-editor 0.1") {
*   #R4 @PLACE(y=-2, priority="high");
*   %p2 = #R3 #R4 @AS(priority="high") $PAIR_Y;
*   # @SELECT-USAGE(recursive=true) $ROW => @PRIORITY("disabled");
*   %t = (0, 6) @TEXT("各约束的状态和生效的优先级见下方列表")
* }
.end
