* Unkeyed labels attached to different wires need independent display groups.
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-all", version="0.4"); @VERSION("0.4") }
* @RNL { /* rnl upgrade wrote this record: it keeps the labels and ports the file had before 0.4 */ # @SELECT-PIN(recursive=true) => @REMOVE-FROM $LABEL_FILL_SET; $SUPPLY_NET_SET @REMOVE-FROM $PORT_FILL_SET; $GROUND_NET_SET @REMOVE-FROM $PORT_FILL_SET }
R1 in mid 1k
R2 mid out 2k
* @RNL {
*   #R1 @PLACE(0, 0); #R2 @PLACE(8, 0);
*   %input = &in @WIRE {(-8, 2) #R1:1};
*   %output = &out @WIRE {#R2:2 (14, -2)};
*   %input @TEXT("IN") => @PLACE(-5, 3);
*   %output @TEXT("OUT") => @PLACE(11, -1)
* }
.end
