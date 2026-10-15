* Label positions axis by axis: what the scene writes replaces what the symbol gives
*! 符号给自己的标签定了位置：$TAG 的 name 在 (1, 1)，value 在 (1, -1)。场景里给同一个引用名写 PLACE 或偏移（dx、dy），按轴取代它（13.3）：
*! 每一轴取后写的那种；两种都没写的轴，仍用符号给的位置，随元件转。偏移按场景的轴，不随元件转。
*! R1 的 name 只写 x 的 PLACE；R2 转了 R90，name 只写 dx；R3 的 name 先写偏移、后写 x 的 PLACE；R4 的 value 先写 PLACE、后写 dy；R5 什么都没写，照符号。
*! R6 的 name 写 dx、dy，是编辑器导出标签的常见写法；符号没有的引用名（R6 的 note）照旧按偏移放。
R1 a1 b1 1k
R2 a2 b2 2k
R3 a3 b3 3k
R4 a4 b4 4k
R5 a5 b5 5k
R6 a6 b6 6k
* @RNL(type="header") { @VERSION("0.2")}
* @RNL {
*   $TAG = @DEFINE-SYMBOL(box=[-2, -0.5, 2, 0.5]) {
*     (-2, 0) @PIN(terminal=1, direction="left"); (2, 0) @PIN(terminal=2, direction="right");
*     @LINE {(-2, 0) (2, 0)};
*     ? @TEXT("${name}", key="name") => @PLACE(1, 1);
*     ? @TEXT("${value}", key="value") => @PLACE(1, -1)
*   };
*   # @SELECT-INSTANCE => @SET-SYMBOL $TAG;
*   #R1 @PLACE(0, 0); #R2 @PLACE(10, 0) => @ORIENT("R90"); #R3 @PLACE(20, 0);
*   #R4 @PLACE(0, -10); #R5 @PLACE(10, -10); #R6 @PLACE(20, -10);
*   #R1::name @PLACE(x=-3);
*   #R2 @TEXT("${name}", key="name", dx=3);
*   #R3 @TEXT("${name}", key="name", dx=1, dy=2);
*   #R3::name @PLACE(x=25);
*   #R4::value @PLACE(1, -15);
*   #R4 @TEXT("${value}", key="value", dy=-3);
*   #R6 @TEXT("${name}", key="name", dx=-1, dy=1.5);
*   #R6 @TEXT("note", key="note", dx=-1, dy=-1.5)
* }
.end
