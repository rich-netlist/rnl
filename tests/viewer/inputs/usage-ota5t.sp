* 5T OTA: M1 in two usages of the standard series 0.4, for the Usage panel
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-all", version="0.4"); @VERSION("0.4") }
.model nch nmos level=1 vto=0.5
.model pch pmos level=1 vto=-0.5
.subckt ota5t inp inn out vdd vss vbias
M1 x    inp   tail vss nch W=2u L=0.5u
M2 out  inn   tail vss nch W=2u L=0.5u
M3 x    x     vdd  vdd pch W=4u L=0.5u
M4 out  x     vdd  vdd pch W=4u L=0.5u
M5 tail vbias vss  vss nch W=4u L=1u
* @RNL {
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   &vss @ADD-TO $GROUND_NET_SET;
*   %dp1 = {#M1 #M2} @AS $MATCHED;
*   %ax = (_, _);
*   %sym12 = #M1(left) #M2(right) %ax(axis) @AS $MIRRORED_LAYOUT;
*   %sym34 = #M3(left) #M4(right) %ax(axis) @AS $MIRRORED_LAYOUT;
*   {#M5 #M1 #M3} @ORDER("bottom-to-top")
* }
.ends ota5t
.end
