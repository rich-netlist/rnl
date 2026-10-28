* A file of 0.2 that includes a file without a header record
* @RNL(type="header") { @VERSION("0.2")}
*! The included file has no header record and is read as 0.2, like the file that includes it (16.1), with the 0.2 series (16.3): the library's $RES and $GND are names of the 0.2 series and are found as usual.
*! The default version 0.3 applies only to the main file and to package files without a version; the main file declares 0.2, so the whole document is 0.2.
.include 09-include-without-header-lib.inc
X1 in 0 divider
* @RNL { #X1 @PLACE(0, 0) }
.end
