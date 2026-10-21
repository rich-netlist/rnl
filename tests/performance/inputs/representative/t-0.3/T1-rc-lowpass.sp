* T1 RC low-pass filter
* @RNL(type="header") { @VERSION("0.3")}
VIN in 0 DC 0 AC 1
R1 in out 1k
C1 out 0 1n
* @RNL {
*   {#VIN #R1 #C1} @AS $FLOW;
*   $WIRE_NET_SET @ROUTE
* }
.end
