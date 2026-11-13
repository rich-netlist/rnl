* Terminal names of T, O, U and Z in a file of 0.4
*! In a file of version 0.4, the terminals of T and O are named p1 n1 p2 n2 by default, those of U a b ref, and those of Z d g s, given by the adapter like SPICE's other default terminal names (6.12).
*! T1 and O1 are two transmission lines, U1 a distributed RC line, Z1 and Z2 MESFETs; every wire connects by terminal name. Z2's names come from the DEVICE of the header record: what the file writes still overrides the default.
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-spice", version="0.4");
*   @VERSION("0.4");
*   @DEVICE(name="Z2", terminals=["drain", "gate", "source"])
* }
T1 a 0 b 0 z0=50 td=1n
O1 b 0 c 0 lossy
U1 c d 0 urc
Z1 d g1 0 mesf
Z2 d g2 0 mesf
.model lossy ltra
.model urc urc
.model mesf nmf
*! DEVICE terminal names map explicitly to the standard symbol pin bindings (9.2.4).
* @RNL {
*   #Z2 @SET-SYMBOL(terminals=[["drain", "d"], ["gate", "g"], ["source", "s"]]) $NJFET;
*   #T1 @PLACE(0, 0); #O1 @PLACE(10, 0); #U1 @PLACE(20, 0); #Z1 @PLACE(30, 0); #Z2 @PLACE(40, 0);
*   &b @WIRE(mode="auto") {#T1:p2 #O1:p1};
*   &c @WIRE(mode="auto") {#O1:p2 #U1:a};
*   %d1 = (30, 3);
*   $JUNCTION @ATTACH %d1 => @PLACE(30, 3);
*   &d @WIRE(mode="auto") {#U1:b (26, 2) (26, 3) %d1 #Z2:drain};
*   &d @WIRE(mode="auto") {%d1 #Z1:d};
*   %g0=(-2,-3); %g1=(2,-3); %g2=(8,-3); %g3=(12,-3); %g4=(20,-3); %g5=(30,-3); %g6=(40,-3);
*   $JUNCTION @ATTACH %g1 => @PLACE(2,-3); $JUNCTION @ATTACH %g2 => @PLACE(8,-3);
*   $JUNCTION @ATTACH %g3 => @PLACE(12,-3); $JUNCTION @ATTACH %g4 => @PLACE(20,-3); $JUNCTION @ATTACH %g5 => @PLACE(30,-3);
*   &0 @WIRE(mode="auto") {%g0 %g1 %g2 %g3 %g4 %g5 %g6};
*   &0 @WIRE(mode="auto") {#T1:n1 %g0}; &0 @WIRE(mode="auto") {#T1:n2 %g1};
*   &0 @WIRE(mode="auto") {#O1:n1 %g2}; &0 @WIRE(mode="auto") {#O1:n2 %g3};
*   &0 @WIRE(mode="auto") {#U1:ref %g4}; &0 @WIRE(mode="auto") {#Z1:s %g5}; &0 @WIRE(mode="auto") {#Z2:source %g6}
* }
.end
