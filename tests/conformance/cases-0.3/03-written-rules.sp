* A rule package of the other dialect that the file writes is kept
*! The file itself writes the rule package of the other dialect (a USE-PACKAGE of rnl-std-spectre); what is written comes first, so it takes effect as usual and is not implicitly left out (16.3).
*! X1 is an instance of a subcircuit named resistor, drawn as a resistor by its master under the Spectre rules; R1 is drawn by the SPICE rules.
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
