* Terminal names of T, O, U and Z in a file of 0.3
*! 0.3 版的文件里，T、O 的端子默认叫 p1 n1 p2 n2，U 叫 a b ref，Z 叫 d g s，同 SPICE 的其他默认端子名一样由适配器给（6.12）。
*! T1、O1 是两段传输线，U1 是分布 RC，Z1、Z2 是 MESFET；导线都按端子名接。Z2 的名字由头段记录的 DEVICE 给出：文件写的仍然盖过默认。
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
