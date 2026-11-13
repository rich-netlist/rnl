* Groups of a 0.4 file in the tree of groups: one that holds the others, two with the same members, one that crosses another, one turned off
* @RNL(type="header") { @VERSION("0.4")}
R1 a 0 1k
R2 a 0 1k
R3 a 0 1k
R4 a 0 1k
R5 a 0 1k
* @RNL {
*   %big = {#R1 #R2 #R3 #R4 #R5} @GROUP;
*   %outer = {#R1 #R2 #R3} @GROUP;
*   %inner = {#R1 #R2} @GROUP;
*   %again = {#R1 #R2} @GROUP;
*   %cross = {#R3 #R4} @GROUP;
*   %far = {#R5} @GROUP;
*   %off = {#R4 #R5} @GROUP;
*   %off @PRIORITY("disabled")
* }
.end
