* A file of 0.4 that includes a file without a header record
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-all", version="0.4"); @VERSION("0.4")}
*! The included file has no header record and inherits 0.4 from the file that includes it (16.1); its $RESISTOR and $GROUND resolve from the parent's standard package.
*! The parent declares 0.4, so the library is read in 0.4 without declaring its own version.
.include 09-include-without-header-lib.inc
X1 in 0 divider
* @RNL { #X1 @PLACE(0, 0) }
.end
