.lib active
.subckt active_cell p n
R1 p n 1k
.ends active_cell
.endl active
.lib inactive
* .subckt commented_out p n
.subckt unused_cell p n
R2 p n 2k
.ends unused_cell
.endl inactive
