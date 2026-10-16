* A rule package of the other dialect that the file writes is kept
*! 文件自己写了另一种方言的规则包（rnl-std-spectre 的 USE-PACKAGE），写的优先，它照常生效、不被隐式去掉（16.3）。
*! X1 是名叫 resistor 的子电路的实例，按 Spectre 的规则以 master 画成电阻；R1 按 SPICE 的规则画。
* @RNL(type="header") {
*   @VERSION("0.3");
*   @USE-PACKAGE(name="rnl-std-spectre", version="0.3")
* }
.subckt resistor p n
.ends
R1 a 0 1k
X1 a 0 resistor
* @RNL { #R1 @PLACE(0, 0); #X1 @PLACE(10, 0) }
.end
