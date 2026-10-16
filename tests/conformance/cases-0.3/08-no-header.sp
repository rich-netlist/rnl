* A main file without a header record is read in 0.3
*! 主文件没有头段记录：按工具的默认版本 0.3 读（16.1），隐式的标准包也就是 0.3 系列（16.3）。
*! $RESISTOR 是 0.3 系列的名字，0.2 系列没有；T1 有 0.3 给的默认端子名 p1 n1 p2 n2（6.12），导线按名字接。
R1 in out 1k
C1 out 0 1p
T1 out 0 far 0 z0=50 td=1n
R2 far 0 50
* @RNL {
*   #R1 @SET-SYMBOL $RESISTOR => @PLACE(0, 0);
*   #C1 @PLACE(4, -4);
*   #T1 @PLACE(10, 0);
*   #R2 @PLACE(16, -4);
*   &far @WIRE(mode="straight") {#T1:p2 #R2:1}
* }
.end
