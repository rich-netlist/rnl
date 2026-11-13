* Numbering order (5.9): authored records, usage bodies, scope= records, and solver record's own objects
*! 5.9: Objects without a `%` name of the level are numbered from 1 in order:
*!   1. authored records of the level (its own text, then records with `scope=`),
*!      with the unnamed objects of each usage body right after the usage, and
*!      one number per member when a statement creates an object for each member
*!      of a set;
*!   2. the solver record's own unnamed objects, where a number may be referenced
*!      before the statement that makes its object.
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-all", version="0.4");
*   @VERSION("0.4")
* }
* @RNL { # @SELECT-PIN(recursive=true) => @REMOVE-FROM $LABEL_FILL_SET; $UNFILLED_PORT_NETS = # @SELECT-NET(port=true, recursive=true); $SUPPLY_NET_SET @REMOVE-FROM $UNFILLED_PORT_NETS; $GROUND_NET_SET @REMOVE-FROM $UNFILLED_PORT_NETS; # @SELECT-USAGE(recursive=true) $IO_PORT => @SELECT-NET(recursive=true) => @REMOVE-FROM $UNFILLED_PORT_NETS; $IO_PORT @ATTACH $UNFILLED_PORT_NETS }

* @RNL {
* $TAP = @DEFINE(roles=["dev"]) {
*     (0, 0)
*   }
* }

.subckt cell a b
R1 a b 1k
R2 a b 2k
* @RNL {
*   #R1 @PLACE(0, 0);
*   #R2 @PLACE(10, 0);
*   #R1(dev) @AS $TAP;
*   {#R1 #R2} @TEXT("${name}")
* }
* 








* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:ac559c55d4bc1c84.ffc36700559c31e7", numbering="82a812682a7f21e2") {
*   %io_port_a = $IO_PORT @ATTACH &a;
*   %io_port_b = $IO_PORT @ATTACH &b;
*   #R1 @PLACE(0, 0) => @ORIENT("R0");
*   #R2 @PLACE(10, 0) => @ORIENT("R0");
*   %#1 @PLACE(1, 2);
*   %io_port_a @PLACE(-5, 0) => @ORIENT("R0");
*   %io_port_b @PLACE(-2, 0) => @ORIENT("R0");
*   %#2 @PLACE(1, 2);
*   %#5 @PLACE(5, 4);
*   %#3 @PLACE(0, 3);
*   %#4 @PLACE(10, 3);
*   %#6 @LAYER("top");
*   %#7 @PLACE(5, 1);
*   &a @WIRE(mode="auto") {#R1:1 (_, _) #R2:1}
* }
.ends cell

* @RNL(scope="cell") {
*   (_, _)
* }

X1 n1 n2 cell
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:a6df848bca94022b.ffc36700559c31e7") {
*   #X1 @PLACE(0, 0) => @ORIENT("R0")
* }
.end




