// Library sections with independent dialects (16.3)
library mylib

section sec_sp
simulator lang=spice
* @RNL(type="header") {
*   @VERSION("0.3")
* }
.subckt sub_sec_sp a b
Rsec a b 1k
* @RNL {
*   #Rsec @PLACE(0, 0)
* }
.ends sub_sec_sp
simulator lang=spectre
endsection sec_sp

section sec_scs
// @RNL(type="header") {
//   @VERSION("0.3")
// }
subckt sub_sec_scs (a b)
  Xsec (a b) resistor r=1k
// @RNL {
//   #Xsec @PLACE(0, 0)
// }
ends sub_sec_scs
endsection sec_scs

endlibrary mylib
