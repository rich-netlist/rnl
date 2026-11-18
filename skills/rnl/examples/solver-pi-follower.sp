* Passive pi filter followed by a source follower
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4");}
* For a solver (references/good-drawings.md): A signal path from left to right, the pi filter and the follower each a group, every net routed; no static current in the filter.
.model nch nmos level=1
.subckt rc_pi_follower in out vdd vss
R1 in nmid 2k
C1 in vss 2p
C2 nmid vss 2p
M1 vdd nmid out vss nch W=12u L=1u
I1 out vss DC 50u
* @RNL {
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   &vss @ADD-TO $GROUND_NET_SET;
*   %routed_nets = # @SELECT-NET;
*   %routed_nets @ROUTE-NET;
*   {#M1:d #I1:p} @CURRENT("in");
*   {#M1:s #I1:n} @CURRENT("out");
*   #C1(shunt1) #R1(series) #C2(shunt2) @AS $PI_NETWORK => @GROUP;
*   #M1 @AS $COMMON_DRAIN;
*   %buf = {#M1 #I1} @AS $BRANCH => @GROUP;
*   %p_in = $IO_PORT @ATTACH &in;
*   %p_out = $IO_PORT @ATTACH &out;
*   {%p_in #C1 #R1 #C2 %buf %p_out} @ORDER("left-to-right");
* }
.ends rc_pi_follower
.end
