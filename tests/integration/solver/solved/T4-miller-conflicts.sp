* T4 Two-stage Miller OTA, with facts that conflict
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-all", version="0.4"); @VERSION("0.4")}
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
*   # @SELECT-INSTANCE(model="nch", recursive=true) => @SET-SYMBOL $NMOS;
*   # @SELECT-INSTANCE(model="pch", recursive=true) => @SET-SYMBOL $PMOS;
*   #M1(left) #M2(right) #M5(axis) @AS $MIRRORED_LAYOUT;
*   #M3(left) #M4(right) #M5(axis) @AS $MIRRORED_LAYOUT;
*   {#M1 #M2} @ALIGN("y");
*   %stage1 = {#M1 #M2 #M3 #M4 #M5} @GROUP;
*   %stage2 = {#M6 #M7 #CC} @GROUP;
*   %stage1 %stage2 @DISTANCE("left-to-right", min=3);
*   {#M6 #M7} @ALIGN("x");
*   #M7 #M6 @DISTANCE("bottom-to-top", min=4);
*   {#M5 #M7} @ALIGN("y");
*   {#VIP #VIN} @ALIGN("y", priority="low");
*   {{#VIP #VIN} {%stage1} {%stage2} #CL} @AS $SIGNAL_FLOW; {{#VIP #VIN} {#M1 #M2 #M3 #M4 #M5} {#M6 #M7 #CC} #CL} @ORDER("left-to-right", priority="low");
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   # @SELECT-NET => @ROUTE-NET
* }
* @RNL {
*   #M1 @PLACE(0, 0, priority="high");
*   #M2 @PLACE(y=3);
*   #M6 #M7 @DISTANCE("left-to-right", min=2);
*   #VIP #VIN @DISTANCE("bottom-to-top", exact=6)
* }
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:61d3194e31fedd4c.4411a01fa037c77d", numbering="bd83df89422463be") {
*   %ground_stub_M5_s = $GROUND_STUB @ATTACH(gap=3) #M5::s;
*   %ground_stub_M7_s = $GROUND_STUB @ATTACH(gap=3) #M7::s;
*   %ground_stub_VDD_n = $GROUND_STUB @ATTACH(gap=3) #VDD::n;
*   %ground_stub_VB_n = $GROUND_STUB @ATTACH(gap=3) #VB::n;
*   %ground_stub_VIP_n = $GROUND_STUB @ATTACH(gap=3) #VIP::n;
*   %ground_stub_VIN_n = $GROUND_STUB @ATTACH(gap=3) #VIN::n;
*   %ground_stub_CL_n = $GROUND_STUB @ATTACH(gap=3) #CL::n;
*   %supply_stub_M3_s = $SUPPLY_STUB @ATTACH(gap=3) #M3::s;
*   %supply_stub_M4_s = $SUPPLY_STUB @ATTACH(gap=3) #M4::s;
*   %supply_stub_M6_s = $SUPPLY_STUB @ATTACH(gap=3) #M6::s;
*   %supply_stub_VDD_p = $SUPPLY_STUB @ATTACH(gap=3) #VDD::p;
*   %rt1 = (-4, 0);
*   %rt2 = (-4, 2);
*   %rt3 = (7, 0);
*   %rt4 = (28, -1);
*   %rt5 = (32, 0);
*   %rt6 = (10, -4);
*   %rt7 = (42, 3);
*   %rt8 = (40, 3);
*   #M1 @PLACE(0, 0) => @ORIENT("R90");
*   #M2 @PLACE(20, 3) => @ORIENT("MXR90");
*   #M3 @PLACE(-8, 2) => @ORIENT("MY");
*   #M4 @PLACE(28, 2) => @ORIENT("R0");
*   #M5 @PLACE(10, 0) => @ORIENT("R90");
*   #M6 @PLACE(40, 7) => @ORIENT("R0");
*   #M7 @PLACE(42, 0) => @ORIENT("R0");
*   #CC @PLACE(35, 0) => @ORIENT("R90");
*   #VDD @PLACE(-37, -4) => @ORIENT("R0");
*   #VB @PLACE(-15, -4) => @ORIENT("R270");
*   #VIP @PLACE(-9, -10) => @ORIENT("R0");
*   #VIN @PLACE(-25, -4) => @ORIENT("R270");
*   #CL @PLACE(49, 3) => @ORIENT("R90");
*   %#1 @PLACE(10, 0);
*   %#2 @PLACE(10, 2);
*   %#3 @PLACE(0, 0);
*   %ground_stub_M5_s @PLACE(0, 0);
*   %ground_stub_M5_s::g @PLACE(14, -2) => @ORIENT("R0");
*   %ground_stub_M7_s @PLACE(0, 0);
*   %ground_stub_M7_s::g @PLACE(42, -4) => @ORIENT("R0");
*   %ground_stub_VDD_n @PLACE(0, 0);
*   %ground_stub_VDD_n::g @PLACE(-37, -8) => @ORIENT("R0");
*   %ground_stub_VB_n @PLACE(0, 0);
*   %ground_stub_VB_n::g @PLACE(-19, -6) => @ORIENT("R0");
*   %ground_stub_VIP_n @PLACE(0, 0);
*   %ground_stub_VIP_n::g @PLACE(-9, -14) => @ORIENT("R0");
*   %ground_stub_VIN_n @PLACE(0, 0);
*   %ground_stub_VIN_n::g @PLACE(-29, -6) => @ORIENT("R0");
*   %ground_stub_CL_n @PLACE(0, 0);
*   %ground_stub_CL_n::g @PLACE(53, 1) => @ORIENT("R0");
*   %supply_stub_M3_s @PLACE(0, 0);
*   %supply_stub_M3_s::s @PLACE(-8, 6) => @ORIENT("R0");
*   %supply_stub_M4_s @PLACE(0, 0);
*   %supply_stub_M4_s::s @PLACE(28, 6) => @ORIENT("R0");
*   %supply_stub_M6_s @PLACE(0, 0);
*   %supply_stub_M6_s::s @PLACE(40, 11) => @ORIENT("R0");
*   %supply_stub_VDD_p @PLACE(0, 0);
*   %supply_stub_VDD_p::s @PLACE(-37, 0) => @ORIENT("R0");
*   &x @WIRE(mode="straight") {#M3::d (-8, -1) (-4, -1) %rt1};
*   &x @WIRE(mode="straight") {%rt1 %rt2};
*   &x @WIRE(mode="straight") {#M1::d %rt1};
*   &x @WIRE(mode="straight") {%rt2 #M3::g};
*   &x @WIRE(mode="straight") {#M4::g (24, 2) (24, 6) (-4, 6) %rt2};
*   &inp @WIRE(mode="straight") {#VIP::p (-9, -5.5) (0, -5.5) #M1::g};
*   &tail @WIRE(mode="straight") {#M1::s %rt3};
*   &tail @WIRE(mode="straight") {%rt3 #M5::d};
*   &tail @WIRE(mode="straight") {#M2::s (7, 3) %rt3};
*   &y @WIRE(mode="straight") {#M4::d %rt4};
*   &y @WIRE(mode="straight") {%rt4 (32, -1) %rt5};
*   &y @WIRE(mode="straight") {#M2::d (23, 3) (23, -1) %rt4};
*   &y @WIRE(mode="straight") {%rt5 #CC::p};
*   &y @WIRE(mode="straight") {#M6::g (32, 7) %rt5};
*   &inn @WIRE(mode="straight") {#VIN::p (-20, -4) (-20, 0) (-11, 0) (-11, -6) (20, -6) #M2::g};
*   &vb @WIRE(mode="straight") {#VB::p %rt6};
*   &vb @WIRE(mode="straight") {%rt6 #M5::g};
*   &vb @WIRE(mode="straight") {#M7::g (39, -7) (10, -7) %rt6};
*   &out @WIRE(mode="straight") {#M6::d %rt8};
*   &out @WIRE(mode="straight") {%rt7 #M7::d};
*   &out @WIRE(mode="straight") {#CL::p %rt7};
*   &out @WIRE(mode="straight") {%rt8 %rt7};
*   &out @WIRE(mode="straight") {#CC::n (38, 0) (38, 3) %rt8};
*   %#4 @WIRE(mode="straight") {#M5::s (14, 0) %ground_stub_M5_s::g::pin};
*   %#7 @WIRE(mode="straight") {#VB::n (-19, -4) %ground_stub_VB_n::g::pin};
*   %#9 @WIRE(mode="straight") {#VIN::n (-29, -4) %ground_stub_VIN_n::g::pin};
*   %#10 @WIRE(mode="straight") {#CL::n (53, 3) %ground_stub_CL_n::g::pin};
*   $JUNCTION @ATTACH %rt1 => @PLACE(-4, 0);
*   $JUNCTION @ATTACH %rt2 => @PLACE(-4, 2);
*   $JUNCTION @ATTACH %rt3 => @PLACE(7, 0);
*   $JUNCTION @ATTACH %rt4 => @PLACE(28, -1);
*   $JUNCTION @ATTACH %rt5 => @PLACE(32, 0);
*   $JUNCTION @ATTACH %rt6 => @PLACE(10, -4);
*   $JUNCTION @ATTACH %rt7 => @PLACE(42, 3);
*   $JUNCTION @ATTACH %rt8 => @PLACE(40, 3)
* }
.end
