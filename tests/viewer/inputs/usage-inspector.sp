* Usage inspection and independent drawing scopes
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-all", version="0.4"); @VERSION("0.4") }
* @RNL { /* rnl upgrade wrote this record: it keeps the labels and ports the file had before 0.4 */ # @SELECT-PIN(recursive=true) => @REMOVE-FROM $LABEL_FILL_SET; $SUPPLY_NET_SET @REMOVE-FROM $PORT_FILL_SET; $GROUND_NET_SET @REMOVE-FROM $PORT_FILL_SET }
.subckt other p n
Rother p n 1k
* @RNL {
*   $OTHER = @DEFINE(roles=["part"]) {};
*   %other = #Rother @AS $OTHER;
*   #Rother @PLACE(0, 0)
* }
.ends other
R1 a b 1k
R2 b c 2k
R3 c d 3k
R4 d e 4k
* @RNL {
*   $PAIR = @DEFINE(roles=["left", "right"]) {};
*   $SET = @DEFINE(roles=["{members}"]) {};
*   $NEST = @DEFINE(roles=[["inner", "usage"]]) {};
*   $NET = @DEFINE(roles=[["signal", "net"]]) {};
*   $PIN = @DEFINE(roles=[["pin", "terminal"]]) {};
*   $POINT = @DEFINE(roles=[["axis", "point"]]) {};
*   #R1 @PLACE(0, 0); #R2 @PLACE(9, 0); #R3 @PLACE(20, 0);
*   %pair1 = #R1(left) #R2(right) @AS $PAIR;
*   %pair2 = #R2(left) #R3(right) @AS $PAIR;
*   %unseen = #R4(left) #R4(right) @AS $PAIR;
*   %set1 = {#R1 #R3} @AS $SET;
*   %nested = %pair1 @AS $NEST;
*   %signal = &a @AS $NET;
*   %pin1 = #R1:1 @AS $PIN;
*   %axis = (4, 5) @AS $POINT;
*   %ground = $GROUND @ATTACH #R1:2::net => @PLACE(0, -2);
*   %wire = &a @WIRE {(-6, 2) #R1:1}
* }
.end
