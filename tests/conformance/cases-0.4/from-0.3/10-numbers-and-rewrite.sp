* Numbers and a rewritten wire in a file of 0.4
*! The solver's record refers to objects without a name by number (5.9): the wire in the author's record is %#1, the free point on it %#2, and the record's own objects follow; the ground-stub body wire is %#3 and (4, 6) is %#4.
*! numbering is the check of these objects; the numbers take effect only when it matches. WIRE given a wire rewrites it (13.1): this statement rewrites the points of %#1 in Manhattan mode and makes no new wire.
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-all", version="0.4"); @VERSION("0.4")}
* @RNL { # @SELECT-PIN(recursive=true) => @REMOVE-FROM $LABEL_FILL_SET; $UNFILLED_PORT_NETS = # @SELECT-NET(port=true, recursive=true); $SUPPLY_NET_SET @REMOVE-FROM $UNFILLED_PORT_NETS; $GROUND_NET_SET @REMOVE-FROM $UNFILLED_PORT_NETS; # @SELECT-USAGE(recursive=true) $IO_PORT => @SELECT-NET(recursive=true) => @REMOVE-FROM $UNFILLED_PORT_NETS; $IO_PORT @ATTACH $UNFILLED_PORT_NETS }
R1 a b 1k
R2 b 0 1k
* @RNL {
*   #R1 @PLACE(0, 8);
*   #R2 @PLACE(0, 0);
*   &b @WIRE(mode="auto") {#R1:2 (_, _) #R2:1}
* }
* 



* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:b8ce203d3de47e22.ffc36700559c31e7", numbering="18b547eba9e450d7") {
*   %ground_stub_R2_n = $GROUND_STUB @ATTACH(gap=1) #R2::n;
*   #R1 @PLACE(0, 8) => @ORIENT("R0");
*   #R2 @PLACE(0, 0) => @ORIENT("R0");
*   %ground_stub_R2_n @PLACE(0, 0);
*   %ground_stub_R2_n::g @PLACE(0, -3) => @ORIENT("R0");
*   %#2 @PLACE(4, 4);
*   %#1 @WIRE(mode="manhattan") {#R1:2 (0, 5) (4, 6) %#2 (0, 3) #R2:1}
* }
.end



