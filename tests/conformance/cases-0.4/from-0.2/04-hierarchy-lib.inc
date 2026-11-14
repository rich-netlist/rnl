* library for 04-hierarchy: a resistor divider without a block symbol
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4"); @USE-PACKAGE(name="rnl-std-basic", version="0.4") }
.subckt rdiv top mid bot
R1 top mid 10k
R2 mid bot 10k
* @RNL {
*   %jm = (0, -0.5);
*   &mid @WIRE(mode="auto") {#R1:2 %jm #R2:1};
*   &mid @WIRE(mode="auto") {%jm (2, -0.5)};
*   $JUNCTION @ATTACH %jm => @PLACE(0, -0.5);
*   &top @WIRE(mode="auto") {#R1:1 (0, 7)};
*   &bot @WIRE(mode="auto") {#R2:2 (0, -8)}
* }
* 





*! the port tags come from the fill of the standard package ($PORT_FILL_SET)
* @RNL(type="generated", priority="high") { #R1 @PLACE(0, 3); #R2 @PLACE(0, -4); &top @SELECT-ANCESTOR-USAGE $IO_PORT => @PLACE(0, 7) => @ORIENT("R270"); &mid @SELECT-ANCESTOR-USAGE $IO_PORT => @PLACE(2, -0.5) => @ORIENT("MY"); &bot @SELECT-ANCESTOR-USAGE $IO_PORT => @PLACE(0, -8) => @ORIENT("R90") }
.ends
