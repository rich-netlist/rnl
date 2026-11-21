* library for 04-hierarchy: a resistor divider without a block symbol
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4"); @USE-PACKAGE(name="rnl-std-basic", version="0.4") }
.subckt rdiv top mid bot
R1 top mid 10k
R2 mid bot 10k
* @RNL {
*   &mid @WIRE(mode="auto") {#R1:2 #R2:1};
*   %div = {#R1 #R2} @AS $BRANCH;
*   # @SELECT-NET(port=true) => @REMOVE-FROM $PORT_FILL_SET;
*   %ptop = $IO_PORT @ATTACH &top; %pmid = $IO_PORT @ATTACH &mid; %pbot = $IO_PORT @ATTACH &bot;
*   # @SELECT-NET => @ROUTE-NET(priority="high")
* }
* 





*! the ports are written with $IO_PORT; the fill of port tags ($PORT_FILL_SET) is turned off
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:fa28bc282c38a31b.534ef91d5eaac648", numbering="5fdd2f94ed590283") {
*   %rt1 = (3, 8);
*   #R1 @PLACE(3, 11) => @ORIENT("R0");
*   #R2 @PLACE(3, 5) => @ORIENT("R0");
*   %div @PLACE(3, 1);
*   %#2 @PLACE(3, 1);
*   %ptop @PLACE(3, 15) => @ORIENT("R270");
*   %pmid @PLACE(0, 8) => @ORIENT("R0");
*   %pbot @PLACE(3, 1) => @ORIENT("R90");
*   &mid @WIRE(mode="straight") {%pmid::pin %rt1};
*   &mid @WIRE(mode="straight") {%rt1 #R2::p};
*   &top @WIRE(mode="straight") {#R1::p %ptop::pin};
*   &bot @WIRE(mode="straight") {%pbot::pin #R2::n};
*   %#1 @WIRE(mode="straight") {#R1:n %rt1 #R2:p};
*   $JUNCTION @ATTACH %rt1 => @PLACE(3, 8)
* }
.ends
