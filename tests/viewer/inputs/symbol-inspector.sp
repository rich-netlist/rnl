* Symbol inspection and semantic RNL
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4") }
* @RNL { /* rnl upgrade wrote this record: it keeps the labels and ports the file had before 0.4 */ # @SELECT-PIN(recursive=true) => @REMOVE-FROM $LABEL_FILL_SET; $SUPPLY_NET_SET @REMOVE-FROM $PORT_FILL_SET; $GROUND_NET_SET @REMOVE-FROM $PORT_FILL_SET }
.subckt leaf p n
R1 p n 1k
* @RNL {
*   %own = @DEFINE-SYMBOL(box=[-2, -1, 2, 1]) {
*     (0, 1) @PIN(terminal=1, direction="up", label="p");
*     (0, -1) @PIN(terminal=2, direction="down", label="n");
*     @LINE {(-2, -1) (2, 1)};
*     ? @TEXT("${name}", key="name") => @PLACE(2.5, 0)
*   };
*   # @SET-SYMBOL %own;
*   #R1 is placed here @PLACE(0, 0);
*   (3, 0) @TEXT("this quoted string is not free text")
* }
.ends leaf
Xown a 0 leaf
Xparent a 0 leaf
Xpackage a 0 leaf
Xunplaced a 0 leaf
* @RNL {
*   %own = @DEFINE-SYMBOL(box=[-4, -2, 4, 2]) {
*     (-4, 0) @PIN(terminal=1, direction="left");
*     (4, 0) @PIN(terminal=2, direction="right");
*     (0, 0) @RECTANGLE(width=8, height=4);
*     ? @TEXT("${name}", key="name") => @PLACE(0, 3)
*   };
*   #Xown @PLACE(0, 0);
*   #Xparent @SET-SYMBOL %own => @PLACE(12, 0);
*   #Xpackage @SET-SYMBOL(terminals=[[1, "p"], [2, "n"]]) $RESISTOR => @PLACE(24, 0);
*   #Xunplaced @SET-SYMBOL %own;
*   (0, 8) descriptive 中文 text @TEXT("hello");
*   (0, 11) @TEXT("literal only", bold=true); /* comment only */
* }
* /* @RNL { (0, 0) disabled words @TEXT("disabled") } */
.end
* @RNL { (0, 0) ignored after end @TEXT("ignored") }
