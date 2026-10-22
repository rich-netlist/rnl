* T2 Common-source stage
* @RNL(type="header") { @VERSION("0.3")}
.model nch nmos level=1
M1 out in 0 0 nch W=4u L=1u
RD vdd out 5k
VDD vdd 0 1.8
VIN in 0 DC 0.6 AC 1
* @RNL {
*   # @SELECT-INSTANCE(model="nch") => @SET-SYMBOL $NMOS;
*   {#RD #M1} @ALIGN("x");
*   #M1 #RD @DISTANCE("bottom-to-top", min=2);
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   {#VIN #M1} @AS $FLOW;
*   $WIRE_NET_SET @ROUTE
* }
.end
