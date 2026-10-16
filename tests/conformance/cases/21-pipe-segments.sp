* Facts of the segments of one pipe: the later segment is written later
*! 一条语句的管道里有几段写同一个对象的位置或朝向时，后面的那段算后写，同优先级时它先纳入、保留下来（8.5），先写的那段被它取代。
*! R1 先写 PLACE(0, 0)，后写 PLACE(5, 5)：画在 (5, 5)。R2 先写 ORIENT("R0")，后写 ORIENT("R90")：转 R90。
*! R3 先写 x=20，再写 y=3，最后写 x=25：画在 (25, 3)。R4 前一段写 high、后一段写 normal：high 的那段保留，画在 (0, -10)。
R1 a1 b1 1k
R2 a2 b2 2k
R3 a3 b3 3k
R4 a4 b4 4k
* @RNL(type="header") { @VERSION("0.2")}
* @RNL {
*   #R1 @PLACE(0, 0) => @PLACE(5, 5);
*   #R2 @PLACE(10, 0) => @ORIENT("R0") => @ORIENT("R90");
*   #R3 @PLACE(x=20) => @PLACE(y=3) => @PLACE(x=25);
*   #R4 @PLACE(0, -10, priority="high") => @PLACE(5, -15)
* }
.end
