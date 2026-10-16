* Every element the SPICE adapter reads natively, each with its default symbol of the series 0.3
*! 0.3 版的 SPICE 文件隐式用标准包 0.3（16.3），按 SPICE 的规则包 rnl-std-spice 配符号：先按元件字母，后按模型类型，P 型最后；不用 rnl-std-spectre。
*! 除了子电路调用，SPICE 适配器原生读的每种元件各一行（取自 rnl-dev 的 std-0.3/sheet/native.sp），每个都有它的默认符号，没有一个用替身方框画。R2 是电阻模型卡的实例。
* @RNL(type="header") { @VERSION("0.3")}
R1 a1 b1 1k
C1 a2 b2 1p
L1 a3 b3 1n
V1 a4 0 dc 1
I1 a5 b5 dc 1m
E1 a6 b6 c6 d6 2
G1 a7 b7 c7 d7 1m
F1 a8 b8 V1 2
H1 a9 b9 V1 1k
B1 a10 b10 V=2*V(a1)
S1 a11 b11 c11 d11 smod
W1 a12 b12 V1 wmod
D1 a13 b13 dmod
Q1 a14 b14 c14 npnmod
Q2 a15 b15 c15 pnpmod
M1 a16 b16 c16 d16 nmosmod
M2 a17 b17 c17 d17 pmosmod
J1 a18 b18 c18 njfmod
J2 a19 b19 c19 pjfmod
Z1 a20 b20 c20 nmfmod
Z2 a21 b21 c21 pmfmod
T1 a22 b22 c22 d22 z0=50 td=1n
O1 a23 b23 c23 d23 ltramod
U1 a24 b24 c24 urcmod l=1u n=10
R2 a25 b25 rmod
.model smod sw
.model wmod csw
.model dmod d
.model npnmod npn
.model pnpmod pnp
.model nmosmod nmos
.model pmosmod pmos
.model njfmod njf
.model pjfmod pjf
.model nmfmod nmf
.model pmfmod pmf
.model ltramod ltra
.model urcmod urc
.model rmod r
* @RNL {
*   #R1 @PLACE(0, 0); #C1 @PLACE(10, 0); #L1 @PLACE(20, 0); #V1 @PLACE(30, 0); #I1 @PLACE(40, 0);
*   #E1 @PLACE(0, -10); #G1 @PLACE(10, -10); #F1 @PLACE(20, -10); #H1 @PLACE(30, -10); #B1 @PLACE(40, -10);
*   #S1 @PLACE(0, -20); #W1 @PLACE(10, -20); #D1 @PLACE(20, -20); #Q1 @PLACE(30, -20); #Q2 @PLACE(40, -20);
*   #M1 @PLACE(0, -30); #M2 @PLACE(10, -30); #J1 @PLACE(20, -30); #J2 @PLACE(30, -30); #Z1 @PLACE(40, -30);
*   #Z2 @PLACE(0, -40); #T1 @PLACE(10, -40); #O1 @PLACE(20, -40); #U1 @PLACE(30, -40); #R2 @PLACE(40, -40)
* }
.end
