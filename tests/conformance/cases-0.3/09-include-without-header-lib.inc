* library for 09-include-without-header: no header record, so it is read in the version of the file that includes it
.subckt divider top bot
R1 top mid 10k
R2 mid bot 10k
* @RNL {
*   #R1 @SET-SYMBOL $RES => @PLACE(0, 4);
*   #R2 @SET-SYMBOL $RES => @PLACE(0, -4);
*   &mid @WIRE {#R1:2 #R2:1};
*   $GND @ATTACH &bot
* }
.ends
