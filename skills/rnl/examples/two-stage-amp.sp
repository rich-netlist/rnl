* Two-stage op-amp: a five-transistor OTA and a common-source stage
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4");}
* For a solver (references/good-drawings.md): The bias diode, the two branches of the first stage and the second stage in four columns; the pair and its load wired to each other, the gates of each mirror facing each other, the inputs wired alike, the signal nets routed, the bias port short and routed, the bias of the second stage left to its name.
.model nch nmos level=1
.model pch pmos level=1
.subckt two_stage_amp vinp vinn out ibias vdd vss
M6 ibias ibias vss vss nch W=4u L=1u
M5 tail ibias vss vss nch W=8u L=1u
M1 n1 vinp tail vss nch W=10u L=1u
M2 n2 vinn tail vss nch W=10u L=1u
M3 n1 n1 vdd vdd pch W=6u L=1u
M4 n2 n1 vdd vdd pch W=6u L=1u
M7 out n2 vdd vdd pch W=12u L=1u
M8 out ibias vss vss nch W=8u L=1u
* @RNL {
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   &vss @ADD-TO $GROUND_NET_SET;
*   {#M6:d #M5:d #M1:d #M2:d #M8:d #M3:s #M4:s #M7:s} @CURRENT("in");
*   {#M6:s #M5:s #M1:s #M2:s #M8:s #M3:d #M4:d #M7:d} @CURRENT("out");
*   %left = {#M3 #M1} @AS $BRANCH;
*   %right = {#M4 #M2} @AS $BRANCH;
*   %out2 = {#M7 #M8} @AS $BRANCH;
*   %dp = #M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N;
*   %load = #M3(ref) #M4(out) @AS $CURRENT_MIRROR_P;
*   #M3:g @ORIENT("right"); #M4:g @ORIENT("left");
*   %bias = #M6(ref) {#M5 #M8}(out) @AS $CURRENT_MIRROR_N;
*   #M6:g @ORIENT("right"); #M5:g @ORIENT("left");
*   %stage2 = #M7(input) #M8(load) @AS $COMMON_SOURCE_AMPLIFIER;
*   {%dp %load} @ROUTE-OBJECT;
*   {#M6 %left %right %out2} @ORDER("left-to-right");
*   {#M4 #M7} @ALIGN("y");
*   $IO_PORT_STUB @ATTACH {#M1:g #M2:g};
*   %po = $IO_PORT @ATTACH &out;
*   %po::pin @ORIENT("left");
*   {%out2 %po::pin} @ORDER("left-to-right");
*   %pb = $IO_PORT @ATTACH &ibias;
*   %pb::pin @ORIENT("right");
*   {%pb::pin #M6} @ORDER("left-to-right");
*   {%pb #M6:d #M6:g #M5:g} @ROUTE-OBJECT;
*   {&n2 &out} @ROUTE-NET;
* }
.ends two_stage_amp
.end
