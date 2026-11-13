* RC low-pass filter
V1 in 0 0
R1 in out 1k
C1 out 0 1n
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-all", version="0.4"); @USE-PACKAGE(name="rnl-std-basic", version="0.4"); @VERSION("0.4")}
* @RNL {
*   &in @WIRE(mode="auto") {#V1:1 (0, 6) #R1:1};
*   %j = (12, 6);
*   &out @WIRE(mode="auto") {#R1:2 %j #C1:1};
*   %o = (17, 6);
*   &out @WIRE(mode="auto") {%j %o};
*   $JUNCTION @ATTACH %j => @PLACE(12, 6) => @TEXT("(${p::x}, ${p::y})", dx=0.4, dy=-0.9, size=0.7);
*   %o @TEXT("V_{out}", dx=0.4, dy=0.8);
*   #R1 @TEXT("${value} / ${name}", key="value")
* }
* 
* @RNL(type="generated", priority="high") {
*   #V1 @PLACE(0, 0); #R1 @PLACE(6, 6) => @ORIENT("R90"); #C1 @PLACE(12, 3);
*   %gs_V1_2 = $GROUND @ATTACH #V1:2::net => @PLACE(0, -5); #V1:2::net @WIRE(mode="auto") {#V1:2 %gs_V1_2:p};
*   %gs_C1_2 = $GROUND @ATTACH #C1:2::net => @PLACE(12, -1); #C1:2::net @WIRE(mode="auto") {#C1:2 %gs_C1_2:p};
* }

.end
