* Numbering check with multiple `%` names on one object (5.9, 2.6)
*! 5.9: When a token makes an object for each member of a set, the member handle in
*! the numbering check line uses the `%` name bound first in the order of 2.6,
*! regardless of which name the set was written with:
*!   - `%p = (0, 0); %q = %p; %s = {%q}; %s @TEXT("x")` -> check `6186b06d412492b7`
*!   - `%q = (0, 0); %p = %q; %s = {%q}; %s @TEXT("x")` -> check `e2e60027389c0dcb`
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-all", version="0.4");
*   @VERSION("0.4")
* }
* @RNL { # @SELECT-PIN(recursive=true) => @REMOVE-FROM $LABEL_FILL_SET; $UNFILLED_PORT_NETS = # @SELECT-NET(port=true, recursive=true); $SUPPLY_NET_SET @REMOVE-FROM $UNFILLED_PORT_NETS; $GROUND_NET_SET @REMOVE-FROM $UNFILLED_PORT_NETS; # @SELECT-USAGE(recursive=true) $IO_PORT => @SELECT-NET(recursive=true) => @REMOVE-FROM $UNFILLED_PORT_NETS; $IO_PORT @ATTACH $UNFILLED_PORT_NETS }

.subckt first_p a
R1 a 0 1k
* @RNL {
*   #R1 @PLACE(0, 0);
*   %p = (0, 0); %q = %p; %s = {%q}; %s @TEXT("x")
* }
* 


* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:aa18cd608eb7d5e5.ffc36700559c31e7", numbering="6186b06d412492b7") {
*   %ground_stub_R1_n = $GROUND_STUB @ATTACH(gap=1) #R1::n;
*   %io_port_a = $IO_PORT @ATTACH &a;
*   #R1 @PLACE(0, 0) => @ORIENT("R0");
*   %ground_stub_R1_n @PLACE(0, 0);
*   %ground_stub_R1_n::g @PLACE(0, -3) => @ORIENT("R0");
*   %io_port_a @PLACE(-2, 0) => @ORIENT("R0");
*   %p @PLACE(0, 0);
*   %#1 @PLACE(4, 2)
* }
.ends first_p

.subckt first_q a
R1 a 0 1k
* @RNL {
*   #R1 @PLACE(0, 0);
*   %q = (0, 0); %p = %q; %s = {%q}; %s @TEXT("x")
* }
* 


* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:33378b8b7f3332fd.ffc36700559c31e7", numbering="e2e60027389c0dcb") {
*   %ground_stub_R1_n = $GROUND_STUB @ATTACH(gap=1) #R1::n;
*   %io_port_a = $IO_PORT @ATTACH &a;
*   #R1 @PLACE(0, 0) => @ORIENT("R0");
*   %ground_stub_R1_n @PLACE(0, 0);
*   %ground_stub_R1_n::g @PLACE(0, -3) => @ORIENT("R0");
*   %io_port_a @PLACE(-2, 0) => @ORIENT("R0");
*   %q @PLACE(0, 0);
*   %#1 @PLACE(6, 2)
* }
.ends first_q

X1 n1 first_p
X2 n1 first_q
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:bcf33acafef9d06f.ffc36700559c31e7") {
*   #X1 @PLACE(-7, 0) => @ORIENT("R0");
*   #X2 @PLACE(0, 0) => @ORIENT("R0")
* }
.end


