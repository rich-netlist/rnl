* T4 Two-stage Miller OTA, with facts that conflict
* @RNL(type="header") { @VERSION("0.2")}
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
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig1:2e2eb6c29c369899.1f3b310f8bee492a") {
*   %gnd_stub_M5_s = $GND_STUB @ATTACH(gap=2, priority="normal") #M5:s;
*   %gnd_stub_M7_s = $GND_STUB @ATTACH(gap=2, priority="normal") #M7:s;
*   %gnd_stub_VDD_n = $GND_STUB @ATTACH(gap=2, priority="normal") #VDD:n;
*   %gnd_stub_VB_n = $GND_STUB @ATTACH(gap=2, priority="normal") #VB:n;
*   %gnd_stub_VIP_n = $GND_STUB @ATTACH(gap=2, priority="normal") #VIP:n;
*   %gnd_stub_VIN_n = $GND_STUB @ATTACH(gap=2, priority="normal") #VIN:n;
*   %gnd_stub_CL_n = $GND_STUB @ATTACH(gap=2, priority="normal") #CL:n;
*   %supply_stub_M3_s = $SUPPLY_STUB @ATTACH(gap=2, priority="normal") #M3:s;
*   %supply_stub_M4_s = $SUPPLY_STUB @ATTACH(gap=2, priority="normal") #M4:s;
*   %supply_stub_M6_s = $SUPPLY_STUB @ATTACH(gap=2, priority="normal") #M6:s;
*   %supply_stub_VDD_p = $SUPPLY_STUB @ATTACH(gap=2, priority="normal") #VDD:p;
*   %rt1 = (4, 15);
*   %rt2 = (0, 12);
*   %rt3 = (4, -3);
*   %rt4 = (8, 9);
*   %rt5 = (8, 5);
*   %rt6 = (8, -17);
*   %rt7 = (19, 6);
*   %rt8 = (19, 5);
*   #M1 @PLACE(0, 0) => @ORIENT("MY");
*   #M2 @PLACE(8, 3) => @ORIENT("R0");
*   #M3 @PLACE(0, 15) => @ORIENT("MY");
*   #M4 @PLACE(8, 15) => @ORIENT("R0");
*   #M5 @PLACE(4, -17) => @ORIENT("MY");
*   #M6 @PLACE(18, 13) => @ORIENT("MY");
*   #M7 @PLACE(16, -17) => @ORIENT("R0");
*   #CC @PLACE(16, 5) => @ORIENT("MYR90");
*   #VDD @PLACE(-8, 15) => @ORIENT("R0");
*   #VB @PLACE(-38, -6) => @ORIENT("R0");
*   #VIP @PLACE(-22, -6) => @ORIENT("MY");
*   #VIN @PLACE(-17, 0) => @ORIENT("R0");
*   #CL @PLACE(21, 3) => @ORIENT("R0");
*   %gnd_stub_M5_s::g @PLACE(4, -21) => @ORIENT("R0");
*   %gnd_stub_M7_s::g @PLACE(16, -21) => @ORIENT("R0");
*   %gnd_stub_VDD_n::g @PLACE(-8, 10) => @ORIENT("R0");
*   %gnd_stub_VB_n::g @PLACE(-38, -11) => @ORIENT("R0");
*   %gnd_stub_VIP_n::g @PLACE(-22, -11) => @ORIENT("R0");
*   %gnd_stub_VIN_n::g @PLACE(-17, -5) => @ORIENT("R0");
*   %gnd_stub_CL_n::g @PLACE(21, -1) => @ORIENT("R0");
*   %supply_stub_M3_s::s @PLACE(0, 19) => @ORIENT("R0");
*   %supply_stub_M4_s::s @PLACE(8, 19) => @ORIENT("R0");
*   %supply_stub_M6_s::s @PLACE(18, 17) => @ORIENT("R0");
*   %supply_stub_VDD_p::s @PLACE(-8, 20) => @ORIENT("R0");
*   %ax @PLACE(4, 0);
*   %tc::mid @PLACE(4, 0);
*   &x @WIRE(mode="straight") {#M3:g %rt1};
*   &x @WIRE(mode="straight") {%rt1 #M4:g};
*   &x @WIRE(mode="straight") {#M3:d %rt2};
*   &x @WIRE(mode="straight") {%rt2 (4, 12) %rt1};
*   &x @WIRE(mode="straight") {#M1:d %rt2};
*   &inp @WIRE(mode="straight") {#VIP:p (-22, 5) (3, 5) #M1:g};
*   &tail @WIRE(mode="straight") {#M1:s (0, -3) %rt3};
*   &tail @WIRE(mode="straight") {%rt3 (8, -3) #M2:s};
*   &tail @WIRE(mode="straight") {#M5:d %rt3};
*   &y @WIRE(mode="straight") {%rt5 #CC:p};
*   &y @WIRE(mode="straight") {#M4:d %rt4};
*   &y @WIRE(mode="straight") {%rt4 %rt5};
*   &y @WIRE(mode="straight") {#M6:g (22, 13) (22, 9) %rt4};
*   &y @WIRE(mode="straight") {%rt5 #M2:d};
*   &inn @WIRE(mode="straight") {#VIN:p (-17, 4) (4, 4) (4, 3) #M2:g};
*   &vb @WIRE(mode="straight") {#M5:g %rt6};
*   &vb @WIRE(mode="straight") {%rt6 #M7:g};
*   &vb @WIRE(mode="straight") {#VB:p (-38, -2) (-25, -2) (-25, -14) (8, -14) %rt6};
*   &out @WIRE(mode="straight") {#CC:n %rt8};
*   &out @WIRE(mode="straight") {%rt7 (21, 6) #CL:p};
*   &out @WIRE(mode="straight") {#M6:d (18, 7) (19, 7) %rt7};
*   &out @WIRE(mode="straight") {%rt8 %rt7};
*   &out @WIRE(mode="straight") {#M7:d (16, 3) (19, 3) %rt8};
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
