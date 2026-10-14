* Cycles of sets: the statement that closes a cycle fails, and the rest is drawn
*! 集合依赖成环就是错误（5.2.2）：按位置的次序，闭合环的那条语句失效，诊断列出整个环，其余语句照常生效，整个解释不会失败。
*! %row 加进 %col 之后又把 %col 加进 %row：后一条失效，%row 仍只有 R1、R2，对齐照常；%b = {%a} 之后 %b @REMOVE-FROM %a：删除那条失效，C1 照常定位。
*! 两个包集合都含选择：顶层的规则把 $LEFT 加进 $RIGHT，子电路 cell 又把 $RIGHT 加进 $LEFT。环在 cell 里闭合，cell 的那条失效，顶层不受影响，也不再无限递归。
R1 a b 1k
R2 b c 1k
R3 c 0 1k
C1 a 0 1p
X1 a c cell
.subckt cell p q
R1 p q 1k
* @RNL { $RIGHT @ADD-TO $LEFT; #R1 @PLACE(0, 0) }
.ends
* @RNL(type="header") { @VERSION("0.2")}
* @RNL {
*   @USE-PACKAGE(name="cycles") { $LEFT = {# @SELECT-INSTANCE(name="R*")}; $RIGHT = {# @SELECT-INSTANCE(name="C*")} };
*   $LEFT @ADD-TO $RIGHT;
*   $RIGHT @SET-STYLE $grey;
*
*   %row = {#R1 #R2};
*   %col = {#R3};
*   %row @ADD-TO %col;
*   %col @ADD-TO %row;
*   %row @ALIGN("y");
*
*   %a = {#C1};
*   %b = {%a};
*   %b @REMOVE-FROM %a;
*   %a @PLACE(0, -6);
*
*   #R1 @PLACE(0, 0); #R2 @PLACE(6, 0); #R3 @PLACE(12, -3); #X1 @PLACE(20, 0)
* }
.end
