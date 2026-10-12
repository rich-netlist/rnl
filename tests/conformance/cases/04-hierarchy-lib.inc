* library for 04-hierarchy: a resistor divider without a block symbol
.subckt rdiv top mid bot
R1 top mid 10k
R2 mid bot 10k
* @RNL {
*   %jm = (0, -0.5);
*   &mid @WIRE {#R1:2 %jm #R2:1};
*   &mid @WIRE {%jm (2, -0.5)};
*   $JUNCTION @ATTACH %jm;
*   &top @WIRE {#R1:1 (0, 7)};
*   &bot @WIRE {#R2:2 (0, -8)}
* }
* @RNL(type="generated", priority="high") {
*   #R1 @PLACE(0, 3); #R2 @PLACE(0, -4);
*   &top @SELECT-ANCESTOR-USAGE $IOPORT => @PLACE(0, 7) => @ORIENT("R270");
*   &mid @SELECT-ANCESTOR-USAGE $IOPORT => @PLACE(2, -0.5) => @ORIENT("MY");
*   &bot @SELECT-ANCESTOR-USAGE $IOPORT => @PLACE(0, -8) => @ORIENT("R90")
* }
.ends
