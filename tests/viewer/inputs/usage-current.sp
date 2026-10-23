* Shared inspection overlay geometry, including declaration ports
* @RNL(type="header") { @VERSION("0.3") }
.subckt cell input output
Rinside input output 1k
* @RNL {
*   #Rinside @PLACE(0, 0);
*   #:input @CURRENT("in"); #:output @CURRENT("out");
*   %input = $IO_PORT @ATTACH &input => @PLACE(-7, 4);
*   %output = $IO_PORT @ATTACH &output => @PLACE(7, -4) => @ORIENT("R180");
*   #Rinside:1 @CURRENT("in"); #Rinside:2 @CURRENT("out")
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
*   %ground = $GROUND @ATTACH #R1:2;
*   #R1:1 @CURRENT("in"); #R1:2 @CURRENT("out");
*   #R2:1 @CURRENT("out"); #R3:1 @CURRENT("in"); #R4:1 @CURRENT("out");
*   #R2:2 @CURRENT("in") => @CURRENT("unspecified");
*   #X1:1 @CURRENT("in"); #X1:2 @CURRENT("out")
* }
.end
