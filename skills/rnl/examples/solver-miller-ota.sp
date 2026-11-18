* Two-stage Miller-compensated op-amp
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4");}
* For a solver (references/good-drawings.md): A sideways compensation network between two stages; the signal nets routed, the bias net left to its name.
.model nch nmos level=1
.model pch pmos level=1
.subckt miller_ota vinp vinn out ibias vdd vss
Ib vdd ibias DC 20u
M8 ibias ibias vss vss nch W=4u L=1u
M5 tail ibias vss vss nch W=8u L=1u
M1 d1 vinp tail vss nch W=10u L=1u
M2 stg1 vinn tail vss nch W=10u L=1u
M3 d1 d1 vdd vdd pch W=6u L=1u
M4 stg1 d1 vdd vdd pch W=6u L=1u
M6 out stg1 vdd vdd pch W=12u L=1u
M7 out ibias vss vss nch W=8u L=1u
CC stg1 mz 2p
RC mz out 1k
* @RNL {
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   &vss @ADD-TO $GROUND_NET_SET;
*   {#Ib:p #M8:d #M5:d #M1:d #M2:d #M7:d #M3:s #M4:s #M6:s} @CURRENT("in");
*   {#Ib:n #M8:s #M5:s #M1:s #M2:s #M7:s #M3:d #M4:d #M6:d} @CURRENT("out");
*   %bias = {#Ib #M8} @AS $BRANCH;
*   %left = {#M3 #M1} @AS $BRANCH;
*   %right = {#M4 #M2} @AS $BRANCH;
*   %out2 = {#M6 #M7} @AS $BRANCH;
*   %comp = {#CC #RC} @AS $BRANCH_HORIZONTAL;
*   %dp = #M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N;
*   %load = #M3(ref) #M4(out) @AS $CURRENT_MIRROR_P;
*   #M8(ref) {#M5 #M7}(out) @AS $CURRENT_MIRROR_N;
*   %stage2 = #M6(input) #M7(load) @AS $COMMON_SOURCE_AMPLIFIER;
*   %stage2(stage) %comp(compensation) @AS $MILLER_AMPLIFIER;
*   {%dp %load} @ROUTE-OBJECT;
*   {%bias %left %right #CC #RC %out2} @ORDER("left-to-right");
*   {#M4 #M6} @ALIGN("y");
*   $IO_PORT_STUB @ATTACH {#M1:g #M2:g};
*   %po = $IO_PORT @ATTACH &out;
*   %po::pin @ORIENT("left");
*   {%out2 %po::pin} @ORDER("left-to-right");
*   {&stg1 &out} @ROUTE-NET;
* }
.ends miller_ota
.end
