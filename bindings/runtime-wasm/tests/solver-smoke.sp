* Native and WebAssembly solver acceptance fixture
* @RNL(type="header") { @VERSION("0.3") }
R1 a 0 1k
R2 a 0 2k
* @RNL {
*   #R1 @PLACE(-6, 0);
*   #R1 @ORIENT("R0");
*   #R2 @PLACE(6, 0);
*   #R2 @ORIENT("R0");
*   {&a &0} @ROUTE
* }
.end
