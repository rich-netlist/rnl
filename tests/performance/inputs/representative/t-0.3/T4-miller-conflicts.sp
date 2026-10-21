* T4 Two-stage Miller OTA, with facts that conflict
* @RNL(type="header") { @VERSION("0.3")}
.model nch nmos level=1
.model pch pmos level=1
M1 x inp tail 0 nch W=2u L=0.5u
M2 y inn tail 0 nch W=2u L=0.5u
M3 x x vdd vdd pch W=4u L=0.5u
M4 y x vdd vdd pch W=4u L=0.5u
M5 tail vb 0 0 nch W=4u L=1u
M6 out y vdd vdd pch W=16u L=0.5u
M7 out vb 0 0 nch W=8u L=1u
CC y out 1p
VDD vdd 0 1.8
VB vb 0 0.6
VIP inp 0 DC 0.9 AC 0.5
VIN inn 0 DC 0.9 AC 0.5 180
CL out 0 2p
* @RNL {
*   # @SELECT-INSTANCE(model="nch") => @SET-SYMBOL $NMOS;
*   # @SELECT-INSTANCE(model="pch") => @SET-SYMBOL $PMOS;
*   %ax = (_, _);
*   #M1(a) #M2(b) %ax(axis) @AS $MIRRORED;
*   #M3(a) #M4(b) %ax(axis) @AS $MIRRORED;
*   %dp = #M1(in_p) #M2(in_n) #M5(tail) @AS $DIFFERENTIAL_PAIR;
*   %stage1 = {#M1 #M2 #M3 #M4 #M5} @GROUP;
*   %stage2 = {#M6 #M7 #CC} @GROUP;
*   %stage1 %stage2 @DISTANCE("left-to-right", min=3);
*   {#M6 #M7} @ALIGN("x");
*   #M7 #M6 @DISTANCE("bottom-to-top", min=4);
*   {#M5 #M7} @ALIGN("y");
*   {#VIP #VIN} @ALIGN("y", priority="low");
*   {{#VIP #VIN} {%stage1} {%stage2} #CL} @AS $FLOW;
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   $WIRE_NET_SET @ROUTE
* }
* @RNL {
*   #M1 @PLACE(0, 0, priority="high");
*   #M2 @PLACE(y=3);
*   #M6 #M7 @DISTANCE("x", min=2);
*   #VIP #VIN @DISTANCE("bottom-to-top", exact=6)
* }
.end
