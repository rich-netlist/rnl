* 5T OTA testbench
.model nch nmos level=1 vto=0.5
.model pch pmos level=1 vto=-0.5
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4")}
.subckt ota5t inp inn out vdd vss vbias
M1 x    inp   tail vss nch W=2u L=0.5u
M2 out  inn   tail vss nch W=2u L=0.5u
M3 x    x     vdd  vdd pch W=4u L=0.5u
M4 out  x     vdd  vdd pch W=4u L=0.5u
M5 tail vbias vss  vss nch W=4u L=1u
* @RNL {
*   %dp1 = #M1(left), #M2(right) and #M5(tail) treated @AS $DIFFERENTIAL_PAIR_N;
*   %cm1 = #M3(ref) and #M4(out) treated @AS $CURRENT_MIRROR_P;
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   &vss @ADD-TO $GROUND_NET_SET;
*   #M5:d has @CURRENT("in");
*   $OTA_SYM = @DEFINE-SYMBOL {
*     (-5, 2) @PIN(terminal="inp", direction="left");
*     (-5, -2) @PIN(terminal="inn", direction="left");
*     (5, 0) @PIN(terminal="out", direction="right");
*     (-1, 3) @PIN(terminal="vdd", direction="up");
*     (1, -2) @PIN(terminal="vss", direction="down");
*     (-3, -4) @PIN(terminal="vbias", direction="down");
*     @LINE {(-5, 5) (5, 0) (-5, -5) (-5, 5)};
*     ? @TEXT("${name}", key="name") => @PLACE(-2, 0)
*   };
*   this subcircuit # @SET-SYMBOL to $OTA_SYM
* }
* @RNL(type="generated", priority="high", author="rnl-solver", hash="sig2:ffad8672ab912e17.ffc36700559c31e7") {
*   #M1 @PLACE(12, 8) => @ORIENT("R0");  #M2 @PLACE(28, 8) => @ORIENT("MY");
*   #M3 @PLACE(12, 16) => @ORIENT("MY");  #M4 @PLACE(28, 16) => @ORIENT("R0");
*   #M5 @PLACE(20, 0);  %ja = (16, 16);  %jb = (12, 12);
*   &x @WIRE {#M3:d %jb #M1:d};  &x @WIRE {#M3:g %ja #M4:g};
*   &x @WIRE(mode="vertical-horizontal") {%ja %jb};
*   %jo = (28, 12);  %jt = (20, 4);
*   &out @WIRE {#M4:d %jo};  &out @WIRE(mode="horizontal-vertical") {%jo #M2:d};
*   &tail @WIRE(mode="vertical-horizontal") {#M1:s %jt};
*   &tail @WIRE(mode="vertical-horizontal") {#M2:s %jt};
*   &tail @WIRE {%jt #M5:d};
*   $JUNCTION @ATTACH {%ja %jb %jo %jt};
*   %pi = $IO_PORT @ATTACH &inp;  %pi @PLACE(5, 8);
*   %pn = $IO_PORT @ATTACH &inn;  %pn @PLACE(35, 8) => @ORIENT("MY");
*   %po = $IO_PORT @ATTACH &out;  %po @PLACE(35, 12) => @ORIENT("MY");
*   %pb = $IO_PORT @ATTACH &vbias;  %pb @PLACE(13, 0);
*   &inp @WIRE {#M1:g %pi:p};  &inn @WIRE {#M2:g %pn:p};
*   &out @WIRE {%jo %po:p};  &vbias @WIRE {#M5:g %pb:p};
*   %vdd1 = $SUPPLY_STUB @ATTACH #M3:s;  %vdd1::s @PLACE(12, 20);
*   %vdd2 = $SUPPLY_STUB @ATTACH #M4:s;  %vdd2::s @PLACE(28, 20);
*   %gnd = $GROUND_STUB @ATTACH #M5:s;  %gnd::g @PLACE(20, -4)
* }
.ends ota5t
XU1 inp inn out vdd 0 vb ota5t
VDD vdd 0 1.8
VB  vb  0 0.6
VIP inp 0 DC 0.9 AC 0.5
VIN inn 0 DC 0.9 AC 0.5 180
CL  out 0 1p
* @RNL {
*   #XU1 is at @PLACE(20, 14);
*   nets {&inp &inn &out &vdd &vb} are @ROUTE-NET;
*   at (-0.5, 24) there is a @TEXT("5T OTA testbench") => it @SET-STYLE to $note
* }
* @RNL(type="generated", priority="high", author="rnl-solver", hash="sig2:a753fcb23538f920.ffc36700559c31e7") {
*   #VIP @PLACE(4, 13) => @ORIENT("R0");
*   #VIN @PLACE(7, 10) => @ORIENT("R0");
*   #VB @PLACE(17, 4) => @ORIENT("R0");
*   #VDD @PLACE(35, 17) => @ORIENT("R0");
*   #CL @PLACE(29, 12) => @ORIENT("R0");
*   %g1 = $GROUND_STUB @ATTACH #VIP:n;  %g1::g @PLACE(4, 9);
*   %g2 = $GROUND_STUB @ATTACH #VIN:n;  %g2::g @PLACE(7, 6);
*   %g3 = $GROUND_STUB @ATTACH #VB:n;  %g3::g @PLACE(17, 0);
*   %g4 = $GROUND_STUB @ATTACH #XU1:vss;  %g4::g @PLACE(21, 10);
*   %g5 = $GROUND_STUB @ATTACH #VDD:n;  %g5::g @PLACE(35, 13);
*   %g6 = $GROUND_STUB @ATTACH #CL:n;  %g6::g @PLACE(29, 8);
*   &inp @WIRE(mode="vertical-horizontal") {#VIP:p #XU1:inp};
*   &inn @WIRE {#VIN:p #XU1:inn};
*   &vb @WIRE {#XU1:vbias #VB:p};
*   &vdd @WIRE(mode="vertical-horizontal") {#XU1:vdd #VDD:p};
*   &out @WIRE {#XU1:out #CL:p};
*   #VIP::name @PLACE(0, 13.25);  #VIP::value @PLACE(-4.5, 12.05);
*   #VIN::name @PLACE(8.3, 9.5);  #VIN::value @PLACE(7.7, 7.8);
*   #VB::name @PLACE(13.2, 4.25);  #VB::value @PLACE(13, 3.05);
*   &inp @TEXT("inp") => @PLACE(9.5, 16.75);  &inn @TEXT("inn") => @PLACE(12, 12.75);
*   &vdd @TEXT("vdd") => @PLACE(27, 19.75);  &out @TEXT("out") => @PLACE(27, 14.75)
* }
.end
