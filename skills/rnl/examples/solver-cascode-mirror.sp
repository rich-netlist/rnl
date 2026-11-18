* NMOS cascode current mirror with a reference current and a resistive load
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4");}
* For a solver (references/good-drawings.md): Two columns of three under a cascode mirror; only the tops of the columns are aligned by hand.
.model nch nmos level=1
.subckt cascode_cm vdd vss out
Iref vdd nref DC 50u
M2 nref nref m1d vss nch W=4u L=1u
M1 m1d m1d vss vss nch W=4u L=1u
M4 out nref m3d vss nch W=8u L=1u
M3 m3d m1d vss vss nch W=8u L=1u
RL vdd out 5k
* @RNL {
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   &vss @ADD-TO $GROUND_NET_SET;
*   {#Iref:p #RL:p #M1:d #M2:d #M3:d #M4:d} @CURRENT("in");
*   {#Iref:n #RL:n #M1:s #M2:s #M3:s #M4:s} @CURRENT("out");
*   %refcol = {#Iref #M2 #M1} @AS $BRANCH;
*   %outcol = {#RL #M4 #M3} @AS $BRANCH;
*   {#M1 #M2} @AS $DIODE_CONNECTED;
*   #M1(ref) #M3(out) #M2(refcascode) #M4(outcascode) @AS $CASCODE_CURRENT_MIRROR_N;
*   {%refcol %outcol} @ORDER("left-to-right");
*   {#Iref #RL} @ALIGN("y");
*   %po = $IO_PORT @ATTACH &out;
*   %po::pin @ORIENT("left");
*   {%outcol %po::pin} @ORDER("left-to-right");
*   &out @ROUTE-NET;
* }
.ends cascode_cm
.end
