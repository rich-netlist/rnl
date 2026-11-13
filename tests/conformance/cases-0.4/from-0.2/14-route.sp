* Common-emitter amplifier, wiring left to the solver
*! ROUTE (13.7, PR5): it only says "draw these nets as wires", without branch or junction points, so a weak model can write it too; the solver writes back the actual wires and dots, and the prototype does not route but draws dashed flylines.
*! One rule for the standard package's $WIRE_NET_SET (every net but ground and supplies); out has a segment written by hand first, and ROUTE adds only the rest; the ground and supply stubs are written back by the "solver".
.model npn1 npn
Q1 c b e npn1
R1 vcc b 47k
R2 b 0 10k
RC vcc c 4.7k
RE e 0 1k
C1 in b 1u
C2 c out 1u
RL out 0 10k
Vcc vcc 0 12
Vin in 0 AC 1
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-all", version="0.4"); @USE-PACKAGE(name="rnl-std-basic", version="0.4"); @VERSION("0.4")}
* @RNL {
*   &vcc @ADD-TO $SUPPLY_NET_SET;
*   %route_nets = # @SELECT-NET; $GROUND_NET_SET @REMOVE-FROM %route_nets; $SUPPLY_NET_SET @REMOVE-FROM %route_nets; %route_nets @ROUTE-NET; %expected_routes = {&b &c &e &in &out}; %expected_routes @ASSERT-IN %route_nets; %route_nets @ASSERT-IN %expected_routes;
*   &out @WIRE(mode="auto") {#C2:2 (16, 3)};
*   (-5, 0) @TEXT("v_{in}", dx=-0.5, dy=0.8, halign="end");
*   (-2, 14) @TEXT("弱模型的写法：器件位置加 ROUTE，导线、分叉和圆点都交给求解器")
* }
* 

* @RNL(type="generated", priority="high") {
*   #Q1 @PLACE(8, 0);
*   #RC @PLACE(8, 6);
*   #RE @PLACE(8, -6);
*   #R1 @PLACE(2, 6);
*   #R2 @PLACE(2, -5);
*   #C1 @PLACE(-3, 0) => @ORIENT("R90");
*   #C2 @PLACE(12, 3) => @ORIENT("R90");
*   #RL @PLACE(18, 0);
*   #Vin @PLACE(-9, -3);
*   #Vcc @PLACE(24, 6);
*   %gs_R2_2 = $GROUND @ATTACH #R2:2::net => @PLACE(2, -10); #R2:2::net @WIRE(mode="auto") {#R2:2 %gs_R2_2:p};
*   %gs_RE_2 = $GROUND @ATTACH #RE:2::net => @PLACE(8, -11); #RE:2::net @WIRE(mode="auto") {#RE:2 %gs_RE_2:p};
*   %gs_RL_2 = $GROUND @ATTACH #RL:2::net => @PLACE(18, -5); #RL:2::net @WIRE(mode="auto") {#RL:2 %gs_RL_2:p};
*   %gs_Vcc_2 = $GROUND @ATTACH #Vcc:2::net => @PLACE(24, 1); #Vcc:2::net @WIRE(mode="auto") {#Vcc:2 %gs_Vcc_2:p};
*   %gs_Vin_2 = $GROUND @ATTACH #Vin:2::net => @PLACE(-9, -8); #Vin:2::net @WIRE(mode="auto") {#Vin:2 %gs_Vin_2:p};
*   %ss_R1_1 = $SUPPLY @ATTACH #R1:1::net => @PLACE(2, 11); #R1:1::net @WIRE(mode="auto") {#R1:1 %ss_R1_1:p};
*   %ss_RC_1 = $SUPPLY @ATTACH #RC:1::net => @PLACE(8, 11); #RC:1::net @WIRE(mode="auto") {#RC:1 %ss_RC_1:p};
*   %ss_Vcc_1 = $SUPPLY @ATTACH #Vcc:1::net => @PLACE(24, 11); #Vcc:1::net @WIRE(mode="auto") {#Vcc:1 %ss_Vcc_1:p};
* }
.end
