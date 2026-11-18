* Viewer colours and hairlines
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4") }
* @RNL { /* rnl upgrade wrote this record: it keeps the labels and ports the file had before 0.4 */ # @SELECT-PIN(recursive=true) => @REMOVE-FROM $LABEL_FILL_SET; $SUPPLY_NET_SET @REMOVE-FROM $PORT_FILL_SET; $GROUND_NET_SET @REMOVE-FROM $PORT_FILL_SET }
R1 a b 1k
R2 b 0 2k
* @RNL {
*   #R1 @PLACE(4, 2); #R2 @PLACE(16, 2);
*   @LINE {(0, -4) (20, -4)};
*   @LINE(linewidth=0) {(0, -7) (20, -7)};
*   @LINE(linewidth=0.2) {(0, -10) (20, -10)};
*   @LINE(linewidth=0, linestyle="double") {(0, -13) (20, -13)};
*   @LINE(linecolor="#00007f") {(0, -16) (20, -16)};
*   @LINE(linecolor="#000080") {(0, -19) (20, -19)};
*   @LINE(linecolor="#f00") {(0, -22) (20, -22)};
*   @LINE(linecolor="#00007f80") {(0, -25) (20, -25)};
*   (0, -28) @TEXT("Default ink");
*   (0, -31) @RECTANGLE(width=5, height=2, fill="fillcolor", fillcolor="#101030")
* }
.end
