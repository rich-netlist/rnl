* T3 Five-transistor OTA in a testbench
* @RNL(type="header") { @VERSION("0.2")}
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
*   #M1(a) #M2(b) #M5(t) @AS $TAIL_CENTER;
*   #M5 {#M1 #M2} @DISTANCE("bottom-to-top", min=2);
*   {#M1 #M2} {#M3 #M4} @DISTANCE("bottom-to-top", min=2);
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   &vss @ADD-TO $GROUND_NET_SET;
*   $WIRE_NET_SET @ROUTE
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
.end
