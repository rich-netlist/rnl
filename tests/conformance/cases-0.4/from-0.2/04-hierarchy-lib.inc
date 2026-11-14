* library for 04-hierarchy: a resistor divider without a block symbol
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4"); @USE-PACKAGE(name="rnl-std-basic", version="0.4") }
.subckt rdiv top mid bot
R1 top mid 10k
R2 mid bot 10k
* @RNL {
*   &mid @WIRE(mode="auto") {#R1:2 #R2:1}
* }
* 





*! the port tags come from the fill of the standard package ($PORT_FILL_SET)
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:e34ea498bf6444fa.9d9ced86f665c0da") {
*   %net_label_stub_R1_p = $NET_LABEL_STUB @ATTACH #R1::p;
*   %net_label_stub_R2_n = $NET_LABEL_STUB @ATTACH #R2::n;
*   %io_port_top = $IO_PORT @ATTACH &top;
*   %io_port_mid = $IO_PORT @ATTACH &mid;
*   %io_port_bot = $IO_PORT @ATTACH &bot;
*   #R1 @PLACE(-6, 0) => @ORIENT("R90");
*   #R2 @PLACE(0, 0) => @ORIENT("R90");
*   %net_label_stub_R1_p @PLACE(0, 0);
*   %net_label_stub_R1_p::l @PLACE(-8, 0) => @ORIENT("MY");
*   %net_label_stub_R2_n @PLACE(0, 0);
*   %net_label_stub_R2_n::l @PLACE(2, 0) => @ORIENT("R0");
*   %io_port_top @PLACE(-13, 4) => @ORIENT("R0");
*   %io_port_mid @PLACE(-13, 0) => @ORIENT("R0");
*   %io_port_bot @PLACE(-13, -4) => @ORIENT("R0")
* }
.ends
