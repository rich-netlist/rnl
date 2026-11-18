* Independent virtual symbols sharing a net
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-all", version="0.4"); @VERSION("0.4") }
* @RNL { /* rnl upgrade wrote this record: it keeps the labels and ports the file had before 0.4 */ # @SELECT-PIN(recursive=true) => @REMOVE-FROM $LABEL_FILL_SET; $SUPPLY_NET_SET @REMOVE-FROM $PORT_FILL_SET; $GROUND_NET_SET @REMOVE-FROM $PORT_FILL_SET }
.subckt cell a b
Rinside a b 1k
* @RNL {
*   $BOX_cell = @DEFINE-SYMBOL {
*     @RECTANGLE(linestyle="dashed") {(-2, -1) (2, 1)};
*     @LINE {(-3, 0) (-2, 0)};
*     @LINE {(3, 0) (2, 0)};
*     (-3, 0) @PIN(terminal=1, direction="left", label="a");
*     (3, 0) @PIN(terminal=2, direction="right", label="b");
*     ? @TEXT("${name}", key="name", halign="middle", valign="bottom") => @PLACE(0, 1.3)
*   };
*   # @SET-SYMBOL $BOX_cell
* }
.ends
R1 rail out 1k
R2 inp out 2k
X1 out rail cell
* @RNL {
*   #R1 @PLACE(0, 0); #R2 @PLACE(14, 0); #X1 @PLACE(22, 0);
*   %ground = $GROUND @ATTACH &rail; %ground @PLACE(-8, -4);
*   %ground2 = $GROUND @ATTACH &rail; %ground2 @PLACE(-2, -4);
*   %supply = $SUPPLY @ATTACH &rail; %supply @PLACE(8, 6);
*   %port = $IO_PORT @ATTACH &inp; %port @PLACE(10, 6);
*   %junction = $JUNCTION @ATTACH (4, -5) => @PLACE(4, -5);
*   &rail @WIRE {%ground::pin (-8, 3) #R1:1 (8, 3) %supply::pin};
*   &rail @WIRE {%ground2::pin (-2, 3)};
*   &inp @WIRE {%port::pin #R2:1}
* }
.end
