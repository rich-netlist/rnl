* T3 Five-transistor OTA in a testbench
* @RNL(type="header") { @VERSION("0.3")}
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
*   %dp = #M1(in_p) #M2(in_n) #M5(tail) @AS $DIFFERENTIAL_PAIR;
*   %cm = #M3(ref) #M4(out) @AS $CURRENT_MIRROR;
*   %ax = (_, _);
*   #M1(a) #M2(b) %ax(axis) @AS $MIRRORED;
*   #M3(a) #M4(b) %ax(axis) @AS $MIRRORED;
*   #M5 {#M1 #M2} @DISTANCE("bottom-to-top", min=2);
*   {#M1 #M2} {#M3 #M4} @DISTANCE("bottom-to-top", min=2);
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   &vss @ADD-TO $GROUND_NET_SET;
*   $WIRE_NET_SET @ROUTE
* }
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig3:192f45cbfa197dd7.22233424deb2d8ec") {
*   %ground_stub_M5_s = $GROUND_STUB @ATTACH(gap=2) #M5:s;
*   %supply_stub_M3_s = $SUPPLY_STUB @ATTACH(gap=2) #M3:s;
*   %supply_stub_M4_s = $SUPPLY_STUB @ATTACH(gap=2) #M4:s;
*   %io_port_inp = $IO_PORT @ATTACH &inp;
*   %io_port_inn = $IO_PORT @ATTACH &inn;
*   %io_port_out = $IO_PORT @ATTACH &out;
*   %io_port_vbias = $IO_PORT @ATTACH &vbias;
*   %rt1 = (-3, -3);
*   %rt2 = (-8, 0);
*   %rt3 = (-11, 2);
*   %rt4 = (-8, -9);
*   #M1 @PLACE(-8, -7) => @ORIENT("R0");
*   #M2 @PLACE(0, -7) => @ORIENT("MY");
*   #M3 @PLACE(-8, 2) => @ORIENT("R0");
*   #M4 @PLACE(0, 2) => @ORIENT("MY");
*   #M5 @PLACE(-4, -14) => @ORIENT("R0");
*   %ground_stub_M5_s::g @PLACE(-4, -18) => @ORIENT("R0");
*   %supply_stub_M3_s::s @PLACE(-8, 6) => @ORIENT("R0");
*   %supply_stub_M4_s::s @PLACE(0, 6) => @ORIENT("R0");
*   %io_port_inp @PLACE(-14, -7) => @ORIENT("R0");
*   %io_port_inn @PLACE(7, -7) => @ORIENT("R0");
*   %io_port_out @PLACE(0, -3) => @ORIENT("R0");
*   %io_port_vbias @PLACE(-10, -14) => @ORIENT("R0");
*   %ax @PLACE(-4, 0);
*   &inp @WIRE(mode="straight") {%io_port_inp:p (-14, -11) (-11, -11) #M1:g};
*   &inn @WIRE(mode="straight") {#M2:g (3, -11) (7, -11) %io_port_inn:p};
*   &out @WIRE(mode="straight") {#M2:d (-3, -5) %rt1};
*   &out @WIRE(mode="straight") {%rt1 %io_port_out:p};
*   &out @WIRE(mode="straight") {#M4:d (-3, 0) %rt1};
*   &vbias @WIRE(mode="straight") {%io_port_vbias:p (-10, -19) (-7, -19) #M5:g};
*   &x @WIRE(mode="straight") {%rt3 (-11, 0) %rt2};
*   &x @WIRE(mode="straight") {#M1:d %rt2};
*   &x @WIRE(mode="straight") {#M4:g %rt3};
*   &x @WIRE(mode="straight") {%rt2 #M3:d};
*   &x @WIRE(mode="straight") {%rt3 #M3:g};
*   &tail @WIRE(mode="straight") {%rt4 (-8, -12) #M5:d};
*   &tail @WIRE(mode="straight") {#M2:s %rt4};
*   &tail @WIRE(mode="straight") {%rt4 #M1:s};
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
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig3:31d1751a4a6e10a9.22233424deb2d8ec") {
*   %ground_stub_XU1_vss = $GROUND_STUB @ATTACH(gap=2) #XU1:vss;
*   %ground_stub_VDD_n = $GROUND_STUB @ATTACH(gap=2) #VDD:n;
*   %ground_stub_VB_n = $GROUND_STUB @ATTACH(gap=2) #VB:n;
*   %ground_stub_VIP_n = $GROUND_STUB @ATTACH(gap=2) #VIP:n;
*   %ground_stub_VIN_n = $GROUND_STUB @ATTACH(gap=2) #VIN:n;
*   %ground_stub_CL_n = $GROUND_STUB @ATTACH(gap=2) #CL:n;
*   %supply_stub_XU1_vdd = $SUPPLY_STUB @ATTACH(gap=2) #XU1:vdd;
*   %supply_stub_VDD_p = $SUPPLY_STUB @ATTACH(gap=2) #VDD:p;
*   #XU1 @PLACE(45, -3) => @ORIENT("R0");
*   #VDD @PLACE(0, -7) => @ORIENT("R0");
*   #VB @PLACE(8, -7) => @ORIENT("R0");
*   #VIP @PLACE(16, -7) => @ORIENT("R0");
*   #VIN @PLACE(29, -7) => @ORIENT("R0");
*   #CL @PLACE(53, -6) => @ORIENT("R0");
*   %ground_stub_XU1_vss::g @PLACE(45, -8) => @ORIENT("R0");
*   %ground_stub_VDD_n::g @PLACE(0, -11) => @ORIENT("R0");
*   %ground_stub_VB_n::g @PLACE(8, -11) => @ORIENT("R0");
*   %ground_stub_VIP_n::g @PLACE(16, -11) => @ORIENT("R0");
*   %ground_stub_VIN_n::g @PLACE(29, -11) => @ORIENT("R0");
*   %ground_stub_CL_n::g @PLACE(53, -10) => @ORIENT("R0");
*   %supply_stub_XU1_vdd::s @PLACE(45, 2) => @ORIENT("R0");
*   %supply_stub_VDD_p::s @PLACE(0, -3) => @ORIENT("R0");
*   &inp @WIRE(mode="straight") {#VIP:p (16, -2) #XU1:inp};
*   &inn @WIRE(mode="straight") {#VIN:p (40, -5) (40, -4) #XU1:inn};
*   &out @WIRE(mode="straight") {#XU1:out (50, -3) (50, -4) #CL:p};
*   &vb @WIRE(mode="straight") {#VB:p (12, -5) (12, -15) (43, -15) #XU1:vbias}
* }
.end
