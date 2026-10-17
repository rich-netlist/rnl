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
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:ab3e7987ac2f925c.1f3b310f8bee492a") {
*   %gnd_stub_M5_s = $GND_STUB @ATTACH(gap=2, priority="normal") #M5:s;
*   %supply_stub_M3_s = $SUPPLY_STUB @ATTACH(gap=2, priority="normal") #M3:s;
*   %supply_stub_M4_s = $SUPPLY_STUB @ATTACH(gap=2, priority="normal") #M4:s;
*   %ioport_inp = $IOPORT @ATTACH(priority="normal") &inp;
*   %ioport_inn = $IOPORT @ATTACH(priority="normal") &inn;
*   %ioport_out = $IOPORT @ATTACH(priority="normal") &out;
*   %ioport_vdd = $IOPORT @ATTACH(priority="normal") &vdd;
*   %ioport_vss = $IOPORT @ATTACH(priority="normal") &vss;
*   %ioport_vbias = $IOPORT @ATTACH(priority="normal") &vbias;
*   %rt1 = (0, -2);
*   %rt2 = (-4, 2);
*   %rt3 = (-8, -1);
*   %rt4 = (-4, -9);
*   #M1 @PLACE(-8, -5) => @ORIENT("R0");
*   #M2 @PLACE(0, -5) => @ORIENT("MY");
*   #M3 @PLACE(-8, 2) => @ORIENT("MY");
*   #M4 @PLACE(0, 2) => @ORIENT("R0");
*   #M5 @PLACE(-4, -12) => @ORIENT("R0");
*   %gnd_stub_M5_s::g @PLACE(-4, -16) => @ORIENT("R0");
*   %supply_stub_M3_s::s @PLACE(-8, 6) => @ORIENT("R0");
*   %supply_stub_M4_s::s @PLACE(0, 6) => @ORIENT("R0");
*   %ioport_inp @PLACE(-13, -5) => @ORIENT("R0");
*   %ioport_inn @PLACE(5, -5) => @ORIENT("MY");
*   %ioport_out @PLACE(5, -2) => @ORIENT("MY");
*   %ioport_vdd @PLACE(-13, 4) => @ORIENT("R0");
*   %ioport_vss @PLACE(-9, -14) => @ORIENT("R0");
*   %ioport_vbias @PLACE(-9, -12) => @ORIENT("R0");
*   %ax @PLACE(-4, 0);
*   %tc::mid @PLACE(-4, 0);
*   &inp @WIRE(mode="straight") {%ioport_inp:p #M1:g};
*   &inn @WIRE(mode="straight") {#M2:g %ioport_inn:p};
*   &out @WIRE(mode="straight") {#M2:d %rt1};
*   &out @WIRE(mode="straight") {%rt1 #M4:d};
*   &out @WIRE(mode="straight") {%ioport_out:p %rt1};
*   &vbias @WIRE(mode="straight") {%ioport_vbias:p #M5:g};
*   &x @WIRE(mode="straight") {#M3:g %rt2};
*   &x @WIRE(mode="straight") {%rt2 #M4:g};
*   &x @WIRE(mode="straight") {#M3:d %rt3};
*   &x @WIRE(mode="straight") {%rt3 (-4, -1) %rt2};
*   &x @WIRE(mode="straight") {#M1:d %rt3};
*   &tail @WIRE(mode="straight") {#M1:s (-8, -9) %rt4};
*   &tail @WIRE(mode="straight") {%rt4 #M5:d};
*   &tail @WIRE(mode="straight") {#M2:s (0, -9) %rt4};
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
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:cff169f7ae0ac311.1f3b310f8bee492a") {
*   %gnd_stub_XU1_vss = $GND_STUB @ATTACH(gap=2, priority="normal") #XU1:vss;
*   %gnd_stub_VDD_n = $GND_STUB @ATTACH(gap=2, priority="normal") #VDD:n;
*   %gnd_stub_VB_n = $GND_STUB @ATTACH(gap=2, priority="normal") #VB:n;
*   %gnd_stub_VIP_n = $GND_STUB @ATTACH(gap=2, priority="normal") #VIP:n;
*   %gnd_stub_VIN_n = $GND_STUB @ATTACH(gap=2, priority="normal") #VIN:n;
*   %gnd_stub_CL_n = $GND_STUB @ATTACH(gap=2, priority="normal") #CL:n;
*   %supply_stub_XU1_vdd = $SUPPLY_STUB @ATTACH(gap=2, priority="normal") #XU1:vdd;
*   %supply_stub_VDD_p = $SUPPLY_STUB @ATTACH(gap=2, priority="normal") #VDD:p;
*   #XU1 @PLACE(10, 1) => @ORIENT("R0");
*   #VDD @PLACE(0, 1) => @ORIENT("R0");
*   #VB @PLACE(19, -11) => @ORIENT("R0");
*   #VIP @PLACE(-7, -11) => @ORIENT("R0");
*   #VIN @PLACE(5, -11) => @ORIENT("R0");
*   #CL @PLACE(26, -11) => @ORIENT("R0");
*   %gnd_stub_XU1_vss::g @PLACE(10, -4) => @ORIENT("R0");
*   %gnd_stub_VDD_n::g @PLACE(0, -4) => @ORIENT("R0");
*   %gnd_stub_VB_n::g @PLACE(19, -16) => @ORIENT("R0");
*   %gnd_stub_VIP_n::g @PLACE(-7, -16) => @ORIENT("R0");
*   %gnd_stub_VIN_n::g @PLACE(5, -16) => @ORIENT("R0");
*   %gnd_stub_CL_n::g @PLACE(26, -15) => @ORIENT("R0");
*   %supply_stub_XU1_vdd::s @PLACE(10, 6) => @ORIENT("R0");
*   %supply_stub_VDD_p::s @PLACE(0, 6) => @ORIENT("R0");
*   &inp @WIRE(mode="straight") {#VIP:p (-7, -7) (4, -7) (4, 2) #XU1:inp};
*   &inn @WIRE(mode="straight") {#VIN:p (5, 0) #XU1:inn};
*   &out @WIRE(mode="straight") {#XU1:out (26, 1) #CL:p};
*   &vb @WIRE(mode="straight") {#XU1:vbias (8, -7) (19, -7) #VB:p}
* }
.end
