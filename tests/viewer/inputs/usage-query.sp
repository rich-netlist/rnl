* The usages an object is a member of: written, nested two deep, of a set role, disabled, failed, of a
* terminal, of a net, made by a rule, and one whose PLACE loses a conflict (the core's tests of its reverse index)
.model nch nmos level=1
R1 a b 1k
R2 b c 1k
R3 c 0 1k
M1 d g 0 0 nch
* @RNL(type="header") { @VERSION("0.3")}
* @RNL {
*   $PAIR = @DEFINE(roles=["left", "right"]) {};
*   $TWO = @DEFINE(roles=[["first", "usage"], "second"]) {};
*   $SET = @DEFINE(roles=["{members}"]) {};
*   $PIN = @DEFINE(roles=[["pin", "terminal"]]) {};
*   $NET = @DEFINE(roles=[["signal", "net"]]) {};
*   $LONE = @DEFINE(roles=["part"]) {};
*   %pair1 = #R1(left) #R2(right) @AS $PAIR;
*   %outer = %pair1(first) #R3(second) @AS $TWO;
*   %top = %outer(first) #R1(second) @AS $TWO;
*   %all = {#R1 #R3} @AS $SET;
*   %off = #R2(left) #R3(right) @AS(priority="disabled") $PAIR;
*   %bad = #R1(left) #R2(right) #R3(extra) @AS $PAIR;
*   %drain = #M1:d @AS $PIN;
*   %sig = &b @AS $NET;
*   # @SELECT-INSTANCE(name="R3") => @AS $LONE;
*   %gnd = $GROUND @ATTACH #R3:2;
*   %port = $IO_PORT @ATTACH &c => @PLACE(0, 0);
*   %port @PLACE(5, 5, priority="high");
*   #R1 @PLACE(-6, 0); #R2 @PLACE(6, 0); #R3 @PLACE(12, 0); #M1 @PLACE(20, 0)
* }
.end
