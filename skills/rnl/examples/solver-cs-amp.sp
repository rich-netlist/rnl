* Common-source NMOS stage with a resistive load and a load capacitor
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4");}
* For a solver (references/good-drawings.md): One column for the DC path, the load capacitor and the output port to its right, the output net routed.
.model nch nmos level=1
.subckt cs_amp in out vdd vss
M1 out in vss vss nch W=8u L=1u
RD vdd out 10k
CL out vss 1p
* @RNL {
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   &vss @ADD-TO $GROUND_NET_SET;
*   {#RD:p #M1:d} @CURRENT("in");
*   {#RD:n #M1:s} @CURRENT("out");
*   %stage = {#RD #M1} @AS $BRANCH;
*   #M1(input) #RD(load) @AS $COMMON_SOURCE_AMPLIFIER;
*   $IO_PORT_STUB @ATTACH #M1:g;
*   %po = $IO_PORT @ATTACH &out;
*   %po::pin @ORIENT("left");
*   {%stage #CL %po::pin} @ORDER("left-to-right");
*   &out @ROUTE-NET;
* }
.ends cs_amp
.end
