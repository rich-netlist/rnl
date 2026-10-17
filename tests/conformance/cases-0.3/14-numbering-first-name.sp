* Numbering check with multiple `%` names on one object (5.9, 2.6)
*! 5.9: When a token makes an object for each member of a set, the member handle in
*! the numbering check line uses the `%` name bound first in the order of 2.6,
*! regardless of which name the set was written with:
*!   - `%p = (0, 0); %q = %p; %s = {%q}; %s @TEXT("x")` -> check `675df5171032726d`
*!   - `%q = (0, 0); %p = %q; %s = {%q}; %s @TEXT("x")` -> check `951bfe5c096e483e`
* @RNL(type="header") {
*   @VERSION("0.3")
* }

.subckt first_p a
R1 a 0 1k
* @RNL {
*   #R1 @PLACE(0, 0);
*   %p = (0, 0); %q = %p; %s = {%q}; %s @TEXT("x")
* }
* @RNL(type="generated", priority="high", numbering="675df5171032726d") {
*   %#1 @PLACE(4, 2)
* }
.ends first_p

.subckt first_q a
R1 a 0 1k
* @RNL {
*   #R1 @PLACE(0, 0);
*   %q = (0, 0); %p = %q; %s = {%q}; %s @TEXT("x")
* }
* @RNL(type="generated", priority="high", numbering="951bfe5c096e483e") {
*   %#1 @PLACE(6, 2)
* }
.ends first_q

X1 n1 first_p
X2 n1 first_q
.end
