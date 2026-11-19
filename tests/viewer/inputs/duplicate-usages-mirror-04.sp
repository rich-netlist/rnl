* Usages of the same pairs written in other shapes (0.4), as a charge pump's cell writes those of its mirror loads and its input pair:
* %cm1 has R1 and R2 in two roles of one each; %m12 has them in one role; %x12 has them as left and right about the point %ax.
* All three have the same leaves, R1 and R2, and so have %m34 and %x34, of R3 and R4. %x345 is about R5, an instance, which is a
* leaf of its own, and %cp has R1 and R2 in a role beside R5: neither has the same leaves as another.
* @RNL(type="header") { @VERSION("0.4")}
R1 a b 1k
R2 b c 1k
R3 d e 1k
R4 e f 1k
R5 f 0 1k
* @RNL {
*   #R1 @PLACE(0, 8); #R2 @PLACE(8, 8); #R3 @PLACE(0, 0); #R4 @PLACE(8, 0); #R5 @PLACE(4, -6);
*   %big = @DEFINE(roles=[["{tail}", "instance"], ["{loads}", "instance"]]) { };
*   %mirror = @DEFINE(roles=[["{ref}", "instance"], ["{out}", "instance"]]) { };
*   %match = @DEFINE(roles=[["{members}", ["instance", "usage"]]]) { };
*   %mirrored = @DEFINE(roles=[["left", ["instance", "usage"]], ["right", ["instance", "usage"]], ["axis", ["point", "instance", "usage"]]]) { };
*   %cp = {#R5}(tail) {#R1 #R2}(loads) @AS %big;
*   %cm1 = {#R1}(ref) {#R2}(out) @AS %mirror;
*   %m34 = {#R3 #R4} @AS %match;
*   %m12 = {#R1 #R2} @AS %match;
*   %ax = (4, _);
*   %x34 = #R3(left) #R4(right) %ax(axis) @AS %mirrored;
*   %x12 = #R1(left) #R2(right) %ax(axis) @AS %mirrored;
*   %x345 = #R3(left) #R4(right) #R5(axis) @AS %mirrored;
* }
.end
