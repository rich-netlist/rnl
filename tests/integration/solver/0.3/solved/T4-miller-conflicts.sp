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
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig3:a200a55759ff50f2.22233424deb2d8ec") {
*   %ground_stub_M5_s = $GROUND_STUB @ATTACH(gap=2) #M5:s;
*   %ground_stub_M7_s = $GROUND_STUB @ATTACH(gap=2) #M7:s;
*   %ground_stub_VDD_n = $GROUND_STUB @ATTACH(gap=2) #VDD:n;
*   %ground_stub_VB_n = $GROUND_STUB @ATTACH(gap=2) #VB:n;
*   %ground_stub_VIP_n = $GROUND_STUB @ATTACH(gap=2) #VIP:n;
*   %ground_stub_VIN_n = $GROUND_STUB @ATTACH(gap=2) #VIN:n;
*   %ground_stub_CL_n = $GROUND_STUB @ATTACH(gap=2) #CL:n;
*   %supply_stub_M3_s = $SUPPLY_STUB @ATTACH(gap=2) #M3:s;
*   %supply_stub_M4_s = $SUPPLY_STUB @ATTACH(gap=2) #M4:s;
*   %supply_stub_M6_s = $SUPPLY_STUB @ATTACH(gap=2) #M6:s;
*   %supply_stub_VDD_p = $SUPPLY_STUB @ATTACH(gap=2) #VDD:p;
*   %rt1 = (6, -1);
*   %rt2 = (3, 3);
*   %rt3 = (22, -2);
*   %rt4 = (40, 10);
*   %rt5 = (43, 5);
*   %rt6 = (9, 12);
*   %rt7 = (36, 2);
*   %rt8 = (36, 5);
*   #M1 @PLACE(0, 0) => @ORIENT("R0");
*   #M2 @PLACE(24, 3) => @ORIENT("MY");
*   #M3 @PLACE(6, 3) => @ORIENT("R0");
*   #M4 @PLACE(18, 3) => @ORIENT("MY");
*   #M5 @PLACE(12, 3) => @ORIENT("R0");
*   #M6 @PLACE(43, 10) => @ORIENT("R0");
*   #M7 @PLACE(33, 3) => @ORIENT("R0");
*   #CC @PLACE(43, 3) => @ORIENT("R0");
*   #VDD @PLACE(-27, 3) => @ORIENT("R0");
*   #VB @PLACE(-19, 3) => @ORIENT("R0");
*   #VIP @PLACE(-8, 3) => @ORIENT("R0");
*   #VIN @PLACE(-15, 9) => @ORIENT("R0");
*   #CL @PLACE(39, 0) => @ORIENT("R0");
*   %ground_stub_M5_s::g @PLACE(12, -1) => @ORIENT("R0");
*   %ground_stub_M7_s::g @PLACE(33, -1) => @ORIENT("R0");
*   %ground_stub_VDD_n::g @PLACE(-27, -1) => @ORIENT("R0");
*   %ground_stub_VB_n::g @PLACE(-19, -1) => @ORIENT("R0");
*   %ground_stub_VIP_n::g @PLACE(-8, -1) => @ORIENT("R0");
*   %ground_stub_VIN_n::g @PLACE(-15, 5) => @ORIENT("R0");
*   %ground_stub_CL_n::g @PLACE(39, -4) => @ORIENT("R0");
*   %supply_stub_M3_s::s @PLACE(6, 7) => @ORIENT("R0");
*   %supply_stub_M4_s::s @PLACE(18, 7) => @ORIENT("R0");
*   %supply_stub_M6_s::s @PLACE(43, 14) => @ORIENT("R0");
*   %supply_stub_VDD_p::s @PLACE(-27, 7) => @ORIENT("R0");
*   %ax @PLACE(12, 0);
*   &x @WIRE(mode="straight") {#M1:d (0, 3) %rt2};
*   &x @WIRE(mode="straight") {#M3:d %rt1};
*   &x @WIRE(mode="straight") {%rt1 (3, -1) %rt2};
*   &x @WIRE(mode="straight") {#M4:g (21, -5) (6, -5) %rt1};
*   &x @WIRE(mode="straight") {%rt2 #M3:g};
*   &inp @WIRE(mode="straight") {#VIP:p (-12, 5) (-12, -5) (-3, -5) #M1:g};
*   &tail @WIRE(mode="straight") {#M5:d (15, 5) (15, -2) %rt3};
*   &tail @WIRE(mode="straight") {%rt3 (24, -2) #M2:s};
*   &tail @WIRE(mode="straight") {#M1:s (0, -6) (22, -6) %rt3};
*   &y @WIRE(mode="straight") {%rt4 (40, 5) %rt5};
*   &y @WIRE(mode="straight") {#M2:d (24, 10) %rt4};
*   &y @WIRE(mode="straight") {#M4:d (18, -9) (46, -9) (46, 5) %rt5};
*   &y @WIRE(mode="straight") {%rt4 #M6:g};
*   &y @WIRE(mode="straight") {%rt5 #CC:p};
*   &inn @WIRE(mode="straight") {#VIN:p (27, 11) #M2:g};
*   &vb @WIRE(mode="straight") {#M5:g %rt6};
*   &vb @WIRE(mode="straight") {%rt6 (30, 12) #M7:g};
*   &vb @WIRE(mode="straight") {#VB:p (-19, 13) (9, 13) %rt6};
*   &out @WIRE(mode="straight") {#CL:p %rt7};
*   &out @WIRE(mode="straight") {%rt7 (36, -8) (43, -8) #CC:n};
*   &out @WIRE(mode="straight") {#M7:d %rt8};
*   &out @WIRE(mode="straight") {%rt8 %rt7};
*   &out @WIRE(mode="straight") {#M6:d (36, 8) %rt8};
*   $JUNCTION @ATTACH %rt1;
*   $JUNCTION @ATTACH %rt2;
*   $JUNCTION @ATTACH %rt3;
*   $JUNCTION @ATTACH %rt4;
*   $JUNCTION @ATTACH %rt5;
*   $JUNCTION @ATTACH %rt6;
*   $JUNCTION @ATTACH %rt7;
*   $JUNCTION @ATTACH %rt8
* }
.end
