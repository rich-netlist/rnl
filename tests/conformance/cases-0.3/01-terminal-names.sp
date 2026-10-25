* Terminal names of T, O, U and Z in a file of 0.3
*! In a file of version 0.3, the terminals of T and O are named p1 n1 p2 n2 by default, those of U a b ref, and those of Z d g s, given by the adapter like SPICE's other default terminal names (6.12).
*! T1 and O1 are two transmission lines, U1 a distributed RC line, Z1 and Z2 MESFETs; every wire connects by terminal name. Z2's names come from the DEVICE of the header record: what the file writes still overrides the default.
* @RNL(type="header") {
*   @VERSION("0.3");
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
* @RNL {
*   #T1 @PLACE(0, 0); #O1 @PLACE(10, 0); #U1 @PLACE(20, 0); #Z1 @PLACE(30, 0); #Z2 @PLACE(40, 0);
*   &b @WIRE(mode="straight") {#T1:p2 #O1:p1};
*   &c @WIRE(mode="straight") {#O1:p2 #U1:a};
*   &d @WIRE {#U1:b #Z1:d #Z2:drain};
*   &0 @WIRE {#T1:n1 #T1:n2 #O1:n1 #O1:n2 #U1:ref #Z1:s #Z2:source}
* }
.end
