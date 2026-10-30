* the parameters of subcircuits: defaults, the values an instance writes, and many
.subckt amp in out wn=1u l=0.18u rl=10k
R1 in out rl
.ends
.subckt many in out p1=1 p2=2 p3=3 p4=4 p5=5 p6=6
R1 in out p1
.ends
.subckt gain in out k=
R1 in out k
.ends
X1 a b amp wn=2u rl=20k
X2 b c amp
X3 c d many p1=7
X4 d e amp wn={2*wn}
X5 e f gain
R9 d 0 1k
.end
