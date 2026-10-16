* Numbers and a rewritten wire in a file of 0.3
*! 求解器的记录用编号指没有名字的对象（5.9）：作者记录里的导线是 %#1，导线上的自由点是 %#2，记录自己的对象接着编号，(4, 6) 是 %#3。
*! numbering 是这些对象的校验，对得上，编号才生效。WIRE 给的是一根导线（13.1）：这条语句改写 %#1 的点列和模式，不另建导线。
* @RNL(type="header") { @VERSION("0.3")}
R1 a b 1k
R2 b 0 1k
* @RNL {
*   #R1 @PLACE(0, 8);
*   #R2 @PLACE(0, 0);
*   &b @WIRE {#R1:2 (_, _) #R2:1}
* }
* @RNL(type="generated", priority="high", numbering="1457f767879ae90b") {
*   %#2 @PLACE(4, 4);
*   %#1 @WIRE(mode="straight") {#R1:2 (4, 6) %#2 #R2:1}
* }
.end
