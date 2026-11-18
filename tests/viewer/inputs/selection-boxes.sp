* Explicit and inferred symbol boxes
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-all", version="0.4"); @VERSION("0.4") }
* @RNL { /* rnl upgrade wrote this record: it keeps the labels and ports the file had before 0.4 */ # @SELECT-PIN(recursive=true) => @REMOVE-FROM $LABEL_FILL_SET; $SUPPLY_NET_SET @REMOVE-FROM $PORT_FILL_SET; $GROUND_NET_SET @REMOVE-FROM $PORT_FILL_SET }
.subckt cell a b
R1 a b 1k
.ends
Xexplicit a b cell
Xinferred b 0 cell
* @RNL {
*   $explicit = @DEFINE-SYMBOL(box=[-3, -2, 3, 2]) {
*     %rnl_upgrade_local_1 = (-3, 0) @PIN(terminal="a", direction="left"); %rnl_upgrade_local_2 = (3, 0) @PIN(terminal="b", direction="right");
*     @RECTANGLE {(-3, -2) (3, 2)};
*     (0, -1.5) @CIRCLE(radius=0.2, fill="fillcolor", fillcolor="#000000", linestyle="none");
*     (0, 0) @TEXT("Explicit box", halign="middle", valign="middle")
*   };
*   $inferred = @DEFINE-SYMBOL {
*     %rnl_upgrade_local_1 = (-3, 0) @PIN(terminal="a", direction="left"); %rnl_upgrade_local_2 = (3, 0) @PIN(terminal="b", direction="right");
*     @LINE {(-3, 0) (-2, 0) (-2, 1) (2, 1) (2, 0) (3, 0)};
*     @LINE {(-2, 0) (-2, -1) (2, -1) (2, 0)};
*     (0, 0) @TEXT("Inferred box", halign="middle", valign="middle")
*   };
*   #Xexplicit @SET-SYMBOL $explicit; #Xexplicit @PLACE(0, 0); #Xexplicit @ORIENT("R90");
*   #Xinferred @SET-SYMBOL $inferred; #Xinferred @PLACE(12, 0)
* }
.end
