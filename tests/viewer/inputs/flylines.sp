* Two placed resistors joined by a wire through a free point: the core draws a flyline between them (13.7)
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-all", version="0.4"); @VERSION("0.4") }
* @RNL { /* rnl upgrade wrote this record: it keeps the labels and ports the file had before 0.4 */ # @SELECT-PIN(recursive=true) => @REMOVE-FROM $LABEL_FILL_SET; $SUPPLY_NET_SET @REMOVE-FROM $PORT_FILL_SET; $GROUND_NET_SET @REMOVE-FROM $PORT_FILL_SET }
R1 a b 1k
R2 b c 1k
* @RNL { #R1 @PLACE(0, 0); #R2 @PLACE(9, 0); &b @WIRE {#R1:2 (_, _) #R2:1} }
.end
