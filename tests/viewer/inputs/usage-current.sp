* Shared inspection overlay geometry, including declaration ports
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-all", version="0.4"); @VERSION("0.4") }
* @RNL { /* rnl upgrade wrote this record: it keeps the labels and ports the file had before 0.4 */ # @SELECT-PIN(recursive=true) => @REMOVE-FROM $LABEL_FILL_SET; $SUPPLY_NET_SET @REMOVE-FROM $PORT_FILL_SET; $GROUND_NET_SET @REMOVE-FROM $PORT_FILL_SET }
.subckt cell input output
Rinside input output 1k
* @RNL {
*   #Rinside @PLACE(0, 0);
*   #:input @CURRENT("in"); #:output @CURRENT("out");
*   %input = $IO_PORT @ATTACH &input => @PLACE(-7, 4);
*   %output = $IO_PORT @ATTACH &output => @PLACE(7, -4) => @ORIENT("R180");
*   #Rinside:1 @CURRENT("in"); #Rinside:2 @CURRENT("out")
* }
* @RNL {
*   $BOX_cell = @DEFINE-SYMBOL {
*     @RECTANGLE(linestyle="dashed") {(-2, -1) (2, 1)};
*     @LINE {(-3, 0) (-2, 0)};
*     @LINE {(3, 0) (2, 0)};
*     (-3, 0) @PIN(terminal=1, direction="left", label="input");
*     (3, 0) @PIN(terminal=2, direction="right", label="output");
*     ? @TEXT("${name}", key="name", halign="middle", valign="bottom") => @PLACE(0, 1.3)
*   };
*   # @SET-SYMBOL $BOX_cell
* }
.ends cell
R1 a b 1k
R2 b c 2k
R3 c d 3k
R4 d e 4k
X1 a e cell
* @RNL {
*   $PAIR = @DEFINE(roles=["left", "right"]) {};
*   $SET = @DEFINE(roles=["{members}"]) {};
*   $NEST = @DEFINE(roles=[["inner", "usage"]]) {};
*   #R1 @PLACE(0, 0); #R2 @PLACE(9, 0) => @ORIENT("R90");
*   #R3 @PLACE(20, 0) => @ORIENT("MX"); #R4 @PLACE(30, 0) => @ORIENT("R270");
*   #X1 @PLACE(40, 0);
*   %pair = #R1(left) #R2(right) @AS $PAIR;
*   %nested = %pair @AS $NEST;
*   %spread = {#R1 #R3} @AS $SET;
*   %ground = $GROUND @ATTACH #R1:2::net => @PLACE(0, -2);
*   #R1:1 @CURRENT("in"); #R1:2 @CURRENT("out");
*   #R2:1 @CURRENT("out"); #R3:1 @CURRENT("in"); #R4:1 @CURRENT("out");
*   #R2:2 @CURRENT("in") => @CURRENT("unspecified");
*   #X1:1 @CURRENT("in"); #X1:2 @CURRENT("out")
* }
.end
