* Usages made inside others (0.4): %pair makes %match of the same members and %one of one of them; %quad makes a %pair of the same members; %m is a %match of its own
* @RNL(type="header") { @VERSION("0.4")}
R1 a b 1k
R2 b 0 1k
R3 c d 1k
R4 d 0 1k
* @RNL {
*   #R1 @PLACE(0, 0); #R2 @PLACE(0, -4); #R3 @PLACE(8, 0); #R4 @PLACE(8, -4);
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
*   %q = #R3(a) #R4(b) @AS %quad;
* }
.end
