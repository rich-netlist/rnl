* Usages of the same devices (0.4): %pair makes %match of its two devices and %one of the first; %quad makes a %pair of its two;
* %dp is a %pair and %m, written after it, a %match of the same devices; %m2, a %match, is written before %dp2, a %pair of the same devices; %q is a %quad;
* %pa and %pb have R7 and a pin of it each: the same leaf, R7; %pp and %pr have R8 and the point %px, %pq R8 and %py: the same leaf, R8
* @RNL(type="header") { @VERSION("0.4")}
R1 a b 1k
R2 b 0 1k
R3 c d 1k
R4 d 0 1k
R5 e f 1k
R6 f 0 1k
R7 g 0 1k
R8 h 0 1k
* @RNL {
*   #R1 @PLACE(0, 0); #R2 @PLACE(0, -4); #R3 @PLACE(8, 0); #R4 @PLACE(8, -4); #R5 @PLACE(16, 0); #R6 @PLACE(16, -4); #R7 @PLACE(24, 0);
*   %match = @DEFINE(roles=[["a", "instance"], ["b", "instance"]]) { };
*   %one = @DEFINE(roles=[["a", "instance"]]) { };
*   %pair = @DEFINE(roles=[["a", "instance"], ["b", "instance"]]) {
*     ?a(a) ?b(b) @AS %match;
*     ?a(a) @AS %one;
*   };
*   %quad = @DEFINE(roles=[["a", "instance"], ["b", "instance"]]) {
*     ?a(a) ?b(b) @AS %pair;
*   };
*   %dp = #R1(a) #R2(b) @AS %pair;
*   %m = #R1(a) #R2(b) @AS %match;
*   %m2 = #R5(a) #R6(b) @AS %match;
*   %dp2 = #R5(a) #R6(b) @AS %pair;
*   %q = #R3(a) #R4(b) @AS %quad;
*   %onpin = @DEFINE(roles=[["dev", "instance"], ["p", "pin"]]) { };
*   %pa = #R7(dev) #R7:1(p) @AS %onpin;
*   %pb = #R7(dev) #R7:2(p) @AS %onpin;
*   #R8 @PLACE(32, 0); %px = (36, 0); %py = (32, 4);
*   %axis = @DEFINE(roles=[["dev", "instance"], ["at", "point"]]) { };
*   %pp = #R8(dev) %px(at) @AS %axis;
*   %pq = #R8(dev) %py(at) @AS %axis;
*   %pr = #R8(dev) %px(at) @AS %axis;
* }
.end
