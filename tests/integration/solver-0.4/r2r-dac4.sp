* Four-bit R-2R ladder DAC, voltage mode, input code 1010
* The series resistors RS1..RS3 and the 2R legs to the four bit sources; the output is taken at the MSB end.
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-all", version="0.4"); @VERSION("0.4") }
VB0 b0 0 DC 0
VB1 b1 0 DC 1
VB2 b2 0 DC 0
VB3 b3 0 DC 1
RT n0 0 2k
R2B0 n0 b0 2k
R2B1 n1 b1 2k
R2B2 n2 b2 2k
R2B3 out b3 2k
RS1 n0 n1 1k
RS2 n1 n2 1k
RS3 n2 out 1k
* @RNL {
*   $NET_LABEL @ATTACH &out => %pout;
*   %routed_nets = # @SELECT-NET;
*   %routed_nets @ROUTE-NET;
*
*   /* what the circuit is: R-2R ladder, the series resistors RS1..RS3 and the 2R legs R2B0..R2B3 to the bit sources, terminated by RT */
*   %matched_series = {#RS1 #RS2 #RS3} @AS $MATCHED_RESISTOR;
*   %matched_legs = {#RT #R2B0 #R2B1 #R2B2 #R2B3} @AS $MATCHED_RESISTOR;
*
*   /* the series resistors are one row, lying, from the LSB on the left to the MSB on the right */
*   %series_row = {#RS1 #RS2 #RS3} @AS $BRANCH_HORIZONTAL;
*
*   /* each leg is a branch from the top down, the 2R resistor and its bit source */
*   %leg0 = {#R2B0 #VB0} @AS $BRANCH;
*   %leg1 = {#R2B1 #VB1} @AS $BRANCH;
*   %leg2 = {#R2B2 #VB2} @AS $BRANCH;
*   %leg3 = {#R2B3 #VB3} @AS $BRANCH;
*
*   /* from the termination at the LSB end, through the legs, to the output at the MSB end */
*   %ladder_order = {#RT %leg0 %leg1 %leg2 %leg3 %pout} @ORDER("left-to-right");
* }
.end
