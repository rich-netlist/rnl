* T5 Inverting amplifier with an RLC output filter
* @RNL(type="header") { @VERSION("0.3"); @USE-PACKAGE(name="rnl-std-functional", version="0.3")}
.subckt opamp inp inn out
E1 out 0 inp inn 1e5
* @RNL { # @SET-SYMBOL $OPAMP }
.ends opamp
XA 0 n1 vo opamp
RI vin n1 10k
RF n1 vo 100k
L1 vo n2 10u
C1 n2 0 1n
RL n2 0 1k
VIN vin 0 AC 1
* @RNL {
*   {#VIN #RI #XA #L1 {#C1 #RL}} @AS $FLOW;
*   $IO_PORT @ATTACH &n2 => %pout;
*   $WIRE_NET_SET @ROUTE
* }
.end
