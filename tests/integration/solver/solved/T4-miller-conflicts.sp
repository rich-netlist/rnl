* T4 Two-stage Miller OTA, with facts that conflict
* @RNL(type="header") { @VERSION("0.3"); @USE-PACKAGE(name="rnl-std", version="0.2")}
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
*   #M1(a) #M2(b) %ax(axis) @AS $MIRROR_PAIR;
*   #M3(a) #M4(b) %ax(axis) @AS $MIRROR_PAIR;
*   %tc = #M1(a) #M2(b) #M5(t) @AS $TAIL_CENTER;
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
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig3:ca92044498bb24f0.22233424deb2d8ec") {
*   %gnd_stub_M5_s = $GND_STUB @ATTACH(gap=2) #M5:s;
*   %gnd_stub_M7_s = $GND_STUB @ATTACH(gap=2) #M7:s;
*   %gnd_stub_VDD_n = $GND_STUB @ATTACH(gap=2) #VDD:n;
*   %gnd_stub_VB_n = $GND_STUB @ATTACH(gap=2) #VB:n;
*   %gnd_stub_VIP_n = $GND_STUB @ATTACH(gap=2) #VIP:n;
*   %gnd_stub_VIN_n = $GND_STUB @ATTACH(gap=2) #VIN:n;
*   %gnd_stub_CL_n = $GND_STUB @ATTACH(gap=2) #CL:n;
*   %supply_stub_M3_s = $SUPPLY_STUB @ATTACH(gap=2) #M3:s;
*   %supply_stub_M4_s = $SUPPLY_STUB @ATTACH(gap=2) #M4:s;
*   %supply_stub_M6_s = $SUPPLY_STUB @ATTACH(gap=2) #M6:s;
*   %supply_stub_VDD_p = $SUPPLY_STUB @ATTACH(gap=2) #VDD:p;
*   %rt1 = (-17, 1);
*   %rt2 = (-6, 3);
*   %rt3 = (-13, 5);
*   %rt4 = (6, 6);
*   %rt5 = (-2, 6);
*   %rt6 = (-16, -5);
*   %rt7 = (7, 5);
*   %rt8 = (9, 7);
*   #M1 @PLACE(0, 0) => @ORIENT("R0");
*   #M2 @PLACE(-26, 3) => @ORIENT("MY");
*   #M3 @PLACE(-17, 3) => @ORIENT("R0");
*   #M4 @PLACE(-9, 3) => @ORIENT("MY");
*   #M5 @PLACE(-13, 3) => @ORIENT("R0");
*   #M6 @PLACE(9, 13) => @ORIENT("R0");
*   #M7 @PLACE(3, 3) => @ORIENT("R0");
*   #CC @PLACE(9, 3) => @ORIENT("R0");
*   #VDD @PLACE(-54, 3) => @ORIENT("R0");
*   #VB @PLACE(-46, 3) => @ORIENT("R0");
*   #VIP @PLACE(-39, 3) => @ORIENT("R0");
*   #VIN @PLACE(-22, 9) => @ORIENT("R0");
*   #CL @PLACE(7, 0) => @ORIENT("R0");
*   %gnd_stub_M5_s::g @PLACE(-13, -1) => @ORIENT("R0");
*   %gnd_stub_M7_s::g @PLACE(3, -1) => @ORIENT("R0");
*   %gnd_stub_VDD_n::g @PLACE(-54, -2) => @ORIENT("R0");
*   %gnd_stub_VB_n::g @PLACE(-46, -2) => @ORIENT("R0");
*   %gnd_stub_VIP_n::g @PLACE(-39, -2) => @ORIENT("R0");
*   %gnd_stub_VIN_n::g @PLACE(-22, 4) => @ORIENT("R0");
*   %gnd_stub_CL_n::g @PLACE(7, -4) => @ORIENT("R0");
*   %supply_stub_M3_s::s @PLACE(-17, 7) => @ORIENT("R0");
*   %supply_stub_M4_s::s @PLACE(-9, 7) => @ORIENT("R0");
*   %supply_stub_M6_s::s @PLACE(9, 17) => @ORIENT("R0");
*   %supply_stub_VDD_p::s @PLACE(-54, 8) => @ORIENT("R0");
*   %ax @PLACE(-13, 0);
*   %tc::mid @PLACE(-13, 0);
*   &x @WIRE(mode="straight") {#M3:g (-20, 1) %rt1};
*   &x @WIRE(mode="straight") {%rt2 (-6, -4) (-17, -4) %rt1};
*   &x @WIRE(mode="straight") {#M1:d (-5, 2) (-5, 3) %rt2};
*   &x @WIRE(mode="straight") {%rt1 #M3:d};
*   &x @WIRE(mode="straight") {%rt2 #M4:g};
*   &inp @WIRE(mode="straight") {#VIP:p (-39, 14) (-3, 14) #M1:g};
*   &tail @WIRE(mode="straight") {#M2:s (-28, 1) (-28, 13) (-13, 13) %rt3};
*   &tail @WIRE(mode="straight") {#M1:s (-11, -2) (-11, 5) %rt3};
*   &tail @WIRE(mode="straight") {%rt3 #M5:d};
*   &y @WIRE(mode="straight") {#M6:g %rt4};
*   &y @WIRE(mode="straight") {%rt4 (9, 6) #CC:p};
*   &y @WIRE(mode="straight") {#M4:d (-10, 1) (-10, 6) %rt5};
*   &y @WIRE(mode="straight") {%rt5 %rt4};
*   &y @WIRE(mode="straight") {#M2:d (-26, 15) (-2, 15) %rt5};
*   &inn @WIRE(mode="straight") {#M2:g (-23, -1) (-27, -1) (-27, 12) #VIN:p};
*   &vb @WIRE(mode="straight") {#M5:g %rt6};
*   &vb @WIRE(mode="straight") {%rt6 (-4, -5) (-4, 3) #M7:g};
*   &vb @WIRE(mode="straight") {#VB:p (-42, 6) (-42, -5) %rt6};
*   &out @WIRE(mode="straight") {#CL:p %rt7};
*   &out @WIRE(mode="straight") {%rt7 (7, 7) %rt8};
*   &out @WIRE(mode="straight") {#M7:d %rt7};
*   &out @WIRE(mode="straight") {%rt8 (11, 7) (11, 1) #CC:n};
*   &out @WIRE(mode="straight") {#M6:d %rt8};
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
