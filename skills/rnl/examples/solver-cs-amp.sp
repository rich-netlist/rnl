* Common-source NMOS stage with a resistive load and a load capacitor
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4");}
* For a solver (references/good-drawings.md): One column for the DC path; the input port, the stage, the load capacitor and the output port from left to right, the capacitor level with the transistor; every net routed.
.model nch nmos level=1
.subckt cs_amp in out vdd vss
M1 out in vss vss nch W=8u L=1u
RD vdd out 10k
CL out vss 1p
* @RNL {
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   &vss @ADD-TO $GROUND_NET_SET;
*   %routed_nets = # @SELECT-NET;
*   %routed_nets @ROUTE-NET;
*   {#RD:p #M1:d} @CURRENT("in");
*   {#RD:n #M1:s} @CURRENT("out");
*   %branch = {#RD #M1} @AS $BRANCH;
*   %stage = #M1(input) #RD(load) @AS $COMMON_SOURCE_AMPLIFIER;
*   %p_in = $IO_PORT_STUB @ATTACH #M1:g;
*   %p_out = $IO_PORT @ATTACH &out;
*   {%p_in %stage #CL %p_out} @ORDER("left-to-right");
*   {#M1 #CL} @ALIGN("y");
* }
.ends cs_amp
.end
