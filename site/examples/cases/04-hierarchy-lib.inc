* library for 04-hierarchy: a resistor divider without a block symbol
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4"); @USE-PACKAGE(name="rnl-std-basic", version="0.4") }
.subckt rdiv top mid bot
R1 top mid 10k
R2 mid bot 10k
* @RNL {
*   &mid @WIRE(mode="auto") {#R1:2 #R2:1};
*   # @SELECT-NET => @ROUTE-NET(priority="high")
* }
* 





* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:ca0b1e338503f9a1.ad05c39eeaa64c43", numbering="591b5c4536d312bd") {
*   %net_label_stub_R1_p = $NET_LABEL_STUB @ATTACH #R1::p;
*   %net_label_stub_R2_n = $NET_LABEL_STUB @ATTACH #R2::n;
*   %io_port_top = $IO_PORT @ATTACH &top;
*   %io_port_mid = $IO_PORT @ATTACH &mid;
*   %io_port_bot = $IO_PORT @ATTACH &bot;
*   %rt1 = (6, 11);
*   %rt2 = (6, 1);
*   %rt3 = (6, 6);
*   #R1 @PLACE(6, 9) => @ORIENT("R0");
*   #R2 @PLACE(6, 3) => @ORIENT("R0");
*   %net_label_stub_R1_p @PLACE(0, -1.73);
*   %net_label_stub_R1_p::l @PLACE(6, 11) => @ORIENT("R270");
*   %net_label_stub_R2_n @PLACE(0, -1.73);
*   %net_label_stub_R2_n::l @PLACE(6, 1) => @ORIENT("R90");
*   %io_port_top @PLACE(0, 11) => @ORIENT("R0");
*   %io_port_mid @PLACE(0, 6) => @ORIENT("R0");
*   %io_port_bot @PLACE(0, 1) => @ORIENT("R0");
*   &top @WIRE(mode="straight") {#R1::p %rt1};
*   &top @WIRE(mode="straight") {%net_label_stub_R1_p::l::pin %rt1};
*   &top @WIRE(mode="straight") {%io_port_top::pin %rt1};
*   &mid @WIRE(mode="straight") {%io_port_mid::pin %rt3};
*   &mid @WIRE(mode="straight") {%rt3 #R2::p};
*   &bot @WIRE(mode="straight") {#R2::n %rt2};
*   &bot @WIRE(mode="straight") {%net_label_stub_R2_n::l::pin %rt2};
*   &bot @WIRE(mode="straight") {%io_port_bot::pin %rt2};
*   %#1 @WIRE(mode="straight") {#R1:n %rt3 #R2:p};
*   $JUNCTION @ATTACH %rt1 => @PLACE(6, 11);
*   $JUNCTION @ATTACH %rt2 => @PLACE(6, 1);
*   $JUNCTION @ATTACH %rt3 => @PLACE(6, 6)
* }
.ends
