* The hints of a conflict of geometric facts in a file of 0.3
*! 几何事实冲突时，按 8.5 的顺序丢掉的那条给提示，列出和它冲突的每一组留下的事实；冲突里的每条语句也给提示，列出因它丢掉的事实（8.5）。
*! R1 的 x 先写 0 后写 10，后写的留下；R2 的朝向写了两次，后写的留下。%row 写在最前，在 8.5 的顺序里排在坐标之后：
*! R1、R2 写定的 y 不同，它和这两个 y 一起无解，被丢掉，提示列出这一组。
* @RNL(type="header") { @VERSION("0.3")}
R1 a b 1k
R2 b 0 1k
* @RNL {
*   %row = {#R1 #R2} @ALIGN("y");
*   #R1 @PLACE(0, 0);
*   #R1 @PLACE(x=10);
*   #R2 @PLACE(0, -8) => @ORIENT("R90");
*   #R2 @ORIENT("MY")
* }
.end
