* Passive pi filter followed by a source follower
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4");}
* For a solver (references/good-drawings.md): A signal path from left to right through series parts, turned by their pins; no static current in the filter.
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
*   {#M1:d #I1:p} @CURRENT("in");
*   {#M1:s #I1:n} @CURRENT("out");
*   #C1(shunt1) #R1(series) #C2(shunt2) @AS $PI_NETWORK;
*   #M1 @AS $COMMON_DRAIN;
*   %buf = {#M1 #I1} @AS $BRANCH;
*   {#R1::p #R1::n} @ALIGN("y");
*   %pi = $IO_PORT @ATTACH &in;
*   %pi::pin @ORIENT("right");
*   %po = $IO_PORT @ATTACH &out;
*   %po::pin @ORIENT("left");
*   {%pi::pin #C1 #R1 #C2 %buf %po::pin} @ORDER("left-to-right");
*   {%pi::pin #R1::p} @ALIGN("y");
*   {#R1::n #M1::g} @ALIGN("y");
*   {&in &nmid &out} @ROUTE-NET;
* }
.ends rc_pi_follower
.end
