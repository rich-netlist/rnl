* A file written for a language version this tool does not support
*! 16.1: the major version declared is not supported, so none of the file's RNL is interpreted and it is kept as it is; the host netlist is read as usual, and the viewer draws it by its own defaults.
R1 a b 1k
C1 b 0 1p
* @RNL(type="header") { @VERSION("9.0")}
* @RNL { #R1 @PLACE(0, 0) => @FUTURE_KEYWORD(x=1)}
.end
