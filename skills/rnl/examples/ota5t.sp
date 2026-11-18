* Five-transistor OTA with its bias mirror
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4");}
* For a solver (references/good-drawings.md): The bias diode and the two branches in three columns, the pair and its load wired to each other, both inputs wired alike, the bias port short and routed.
.model nch nmos level=1
.model pch pmos level=1
.subckt ota_5t vinp vinn out ibias vdd vss
M6 ibias ibias vss vss nch W=4u L=1u
M5 tail ibias vss vss nch W=8u L=1u
M1 n1 vinp tail vss nch W=10u L=1u
M2 out vinn tail vss nch W=10u L=1u
M3 n1 n1 vdd vdd pch W=6u L=1u
M4 out n1 vdd vdd pch W=6u L=1u
* @RNL {
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   &vss @ADD-TO $GROUND_NET_SET;
*   {#M6:d #M5:d #M1:d #M2:d #M3:s #M4:s} @CURRENT("in");
*   {#M6:s #M5:s #M1:s #M2:s #M3:d #M4:d} @CURRENT("out");
*   %left = {#M3 #M1} @AS $BRANCH;
*   %right = {#M4 #M2} @AS $BRANCH;
*   %dp = #M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N;
*   %load = #M3(ref) #M4(out) @AS $CURRENT_MIRROR_P;
*   %tailmirror = #M6(ref) #M5(out) @AS $CURRENT_MIRROR_N;
*   {%dp %load} @ROUTE-OBJECT;
*   {#M6 %left %right} @ORDER("left-to-right");
*   $IO_PORT_STUB @ATTACH {#M1:g #M2:g};
*   %po = $IO_PORT @ATTACH &out;
*   %po::pin @ORIENT("left");
*   {%right %po::pin} @ORDER("left-to-right");
*   %pb = $IO_PORT @ATTACH &ibias;
*   %pb::pin @ORIENT("right");
*   {%pb::pin #M6} @ORDER("left-to-right");
*   {&ibias &out} @ROUTE-NET;
* }
.ends ota_5t
.end
