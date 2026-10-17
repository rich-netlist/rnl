* Included .inc file starts in SPICE mode, switches to Spectre before first statement
simulator lang=spectre
// @RNL(type="header") {
//   @VERSION("0.3")
// }
subckt box_cell (p n)
ends box_cell

subckt sub_inc (a b)
  X3 (a b) resistor r=1k
  I0 (a b) box_cell
// @RNL {
//   #X3 @PLACE(0, 0);
//   #I0 @PLACE(10, 0)
// }
ends sub_inc
