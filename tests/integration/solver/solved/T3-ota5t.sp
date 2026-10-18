* T3 Five-transistor OTA in a testbench
* @RNL(type="header") { @VERSION("0.3"); @USE-PACKAGE(name="rnl-std", version="0.2")}
.model nch nmos level=1
.model pch pmos level=1
.subckt ota5t inp inn out vdd vss vbias
M1 x inp tail vss nch W=2u L=0.5u
M2 out inn tail vss nch W=2u L=0.5u
M3 x x vdd vdd pch W=4u L=0.5u
M4 out x vdd vdd pch W=4u L=0.5u
M5 tail vbias vss vss nch W=4u L=1u
* @RNL {
*   # @SELECT-INSTANCE(model="nch") => @SET-SYMBOL $NMOS;
*   # @SELECT-INSTANCE(model="pch") => @SET-SYMBOL $PMOS;
*   $OTA_SYM = @DEFINE-SYMBOL(box=[-4, -3, 4, 3]) {
*     (-4, 1) @PIN(terminal="inp", direction="left");
*     (-4, -1) @PIN(terminal="inn", direction="left");
*     (4, 0) @PIN(terminal="out", direction="right");
*     (0, 3) @PIN(terminal="vdd", direction="up");
*     (0, -3) @PIN(terminal="vss", direction="down");
*     (-2, -3) @PIN(terminal="vbias", direction="down");
*     @LINE {(-3, 2.5) (3, 0) (-3, -2.5) (-3, 2.5)};
*     @LINE {(-4, 1) (-3, 1)}; @LINE {(-4, -1) (-3, -1)}; @LINE {(3, 0) (4, 0)};
*     @LINE {(0, 1.25) (0, 3)}; @LINE {(0, -1.25) (0, -3)}; @LINE {(-2, -2.08) (-2, -3)};
*     ? @TEXT("${name}", key="name") => @PLACE(0, 3.3)
*   };
*   # @SET-SYMBOL $OTA_SYM;
*   %dp = #M1(in_p) #M2(in_n) @AS $DIFFPAIR;
*   %cm = #M3(ref) #M4(out) @AS $CURRENT_MIRROR;
*   %ax = (_, _);
*   #M1(a) #M2(b) %ax(axis) @AS $MIRROR_PAIR;
*   #M3(a) #M4(b) %ax(axis) @AS $MIRROR_PAIR;
*   %tc = #M1(a) #M2(b) #M5(t) @AS $TAIL_CENTER;
*   #M5 {#M1 #M2} @DISTANCE("bottom-to-top", min=2);
*   {#M1 #M2} {#M3 #M4} @DISTANCE("bottom-to-top", min=2);
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   &vss @ADD-TO $GROUND_NET_SET;
*   $WIRE_NET_SET @ROUTE
* }
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig3:2884dc320b3f48e9.22233424deb2d8ec") {
*   %gnd_stub_M5_s = $GND_STUB @ATTACH(gap=2) #M5:s;
*   %supply_stub_M3_s = $SUPPLY_STUB @ATTACH(gap=2) #M3:s;
*   %supply_stub_M4_s = $SUPPLY_STUB @ATTACH(gap=2) #M4:s;
*   %ioport_inp = $IOPORT @ATTACH &inp;
*   %ioport_inn = $IOPORT @ATTACH &inn;
*   %ioport_out = $IOPORT @ATTACH &out;
*   %ioport_vdd = $IOPORT @ATTACH &vdd;
*   %ioport_vss = $IOPORT @ATTACH &vss;
*   %ioport_vbias = $IOPORT @ATTACH &vbias;
*   %rt1 = (5, 4);
*   %rt2 = (-4, 5);
*   %rt3 = (-7, 7);
*   %rt4 = (0, -2);
*   #M1 @PLACE(-5, 0) => @ORIENT("R0");
*   #M2 @PLACE(5, 0) => @ORIENT("MY");
*   #M3 @PLACE(-4, 7) => @ORIENT("R0");
*   #M4 @PLACE(4, 7) => @ORIENT("MY");
*   #M5 @PLACE(0, -7) => @ORIENT("R0");
*   %gnd_stub_M5_s::g @PLACE(0, -11) => @ORIENT("R0");
*   %supply_stub_M3_s::s @PLACE(-4, 11) => @ORIENT("R0");
*   %supply_stub_M4_s::s @PLACE(4, 11) => @ORIENT("R0");
*   %ioport_inp @PLACE(-10, 0) => @ORIENT("R0");
*   %ioport_inn @PLACE(13, 0) => @ORIENT("R0");
*   %ioport_out @PLACE(5, 4) => @ORIENT("R0");
*   %ioport_vdd @PLACE(-4, 14) => @ORIENT("R0");
*   %ioport_vss @PLACE(0, -15) => @ORIENT("R0");
*   %ioport_vbias @PLACE(-5, -7) => @ORIENT("R0");
*   %ax @PLACE(0, 0);
*   %tc::mid @PLACE(0, 0);
*   &inp @WIRE(mode="straight") {%ioport_inp:p #M1:g};
*   &inn @WIRE(mode="straight") {#M2:g (8, -3) (13, -3) %ioport_inn:p};
*   &out @WIRE(mode="straight") {#M4:d (8, 5) (8, 4) %rt1};
*   &out @WIRE(mode="straight") {#M2:d %rt1};
*   &out @WIRE(mode="straight") {%rt1 %ioport_out:p};
*   &vbias @WIRE(mode="straight") {%ioport_vbias:p #M5:g};
*   &x @WIRE(mode="straight") {#M1:d (-4, 2) %rt2};
*   &x @WIRE(mode="straight") {%rt3 (-7, 5) %rt2};
*   &x @WIRE(mode="straight") {#M4:g (7, 10) (-7, 10) %rt3};
*   &x @WIRE(mode="straight") {%rt2 #M3:d};
*   &x @WIRE(mode="straight") {%rt3 #M3:g};
*   &tail @WIRE(mode="straight") {#M1:s %rt4};
*   &tail @WIRE(mode="straight") {%rt4 #M5:d};
*   &tail @WIRE(mode="straight") {#M2:s %rt4};
*   $JUNCTION @ATTACH %rt1;
*   $JUNCTION @ATTACH %rt2;
*   $JUNCTION @ATTACH %rt3;
*   $JUNCTION @ATTACH %rt4
* }
.ends ota5t
XU1 inp inn out vdd 0 vb ota5t
VDD vdd 0 1.8
VB vb 0 0.6
VIP inp 0 DC 0.9 AC 0.5
VIN inn 0 DC 0.9 AC 0.5 180
CL out 0 1p
* @RNL {
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   {{#VIP #VIN} #XU1 #CL} @AS $FLOW;
*   $WIRE_NET_SET @ROUTE
* }
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig3:32748af38187648e.22233424deb2d8ec") {
*   %gnd_stub_XU1_vss = $GND_STUB @ATTACH(gap=2) #XU1:vss;
*   %gnd_stub_VDD_n = $GND_STUB @ATTACH(gap=2) #VDD:n;
*   %gnd_stub_VB_n = $GND_STUB @ATTACH(gap=2) #VB:n;
*   %gnd_stub_VIP_n = $GND_STUB @ATTACH(gap=2) #VIP:n;
*   %gnd_stub_VIN_n = $GND_STUB @ATTACH(gap=2) #VIN:n;
*   %gnd_stub_CL_n = $GND_STUB @ATTACH(gap=2) #CL:n;
*   %supply_stub_XU1_vdd = $SUPPLY_STUB @ATTACH(gap=2) #XU1:vdd;
*   %supply_stub_VDD_p = $SUPPLY_STUB @ATTACH(gap=2) #VDD:p;
*   #XU1 @PLACE(43, -3) => @ORIENT("R0");
*   #VDD @PLACE(0, -8) => @ORIENT("R0");
*   #VB @PLACE(8, -8) => @ORIENT("R0");
*   #VIP @PLACE(15, -8) => @ORIENT("R0");
*   #VIN @PLACE(27, -8) => @ORIENT("R0");
*   #CL @PLACE(50, -6) => @ORIENT("R0");
*   %gnd_stub_XU1_vss::g @PLACE(43, -8) => @ORIENT("R0");
*   %gnd_stub_VDD_n::g @PLACE(0, -13) => @ORIENT("R0");
*   %gnd_stub_VB_n::g @PLACE(8, -13) => @ORIENT("R0");
*   %gnd_stub_VIP_n::g @PLACE(15, -13) => @ORIENT("R0");
*   %gnd_stub_VIN_n::g @PLACE(27, -13) => @ORIENT("R0");
*   %gnd_stub_CL_n::g @PLACE(50, -10) => @ORIENT("R0");
*   %supply_stub_XU1_vdd::s @PLACE(43, 2) => @ORIENT("R0");
*   %supply_stub_VDD_p::s @PLACE(0, -3) => @ORIENT("R0");
*   &inp @WIRE(mode="straight") {#VIP:p (15, -2) #XU1:inp};
*   &inn @WIRE(mode="straight") {#VIN:p (27, -4) #XU1:inn};
*   &out @WIRE(mode="straight") {#XU1:out (50, -3) #CL:p};
*   &vb @WIRE(mode="straight") {#VB:p (12, -5) (12, -16) (41, -16) #XU1:vbias}
* }
.end
