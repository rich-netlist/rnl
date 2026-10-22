* CC0-1.0 — 08-unsatisfiable-downward-current.sp: Physically/topologically unsatisfiable simultaneous downward current
*! 目标测试项（条目 8）：构造从几何群变换（D4 八种朝向）与拓扑上**不可能**让所有已标 `@CURRENT` 的端口同时满足“电流朝下 `(0, -1)`”的电路，检验求解器在不可同时满足时的折中行为。
*! 必然冲突的三个典型结构：
*!   1. 正交三端器件全标（`#Q1` 使用标准 `$NPN`，同时对 `:c`、`:b`、`:e` 标注 `@CURRENT`）：
*!      - 在 `$NPN` 的 `R0` 符号中，`:c` 朝上 `(0,+1)`，`:e` 朝下 `(0,-1)`，而 `:b` 朝左 `(-1,0)`（与 c-e 轴正交 90°）。
*!      - 要使流入端电流朝下 `(0,-1)`，引脚外法向必须朝上 `(0,+1)`；要使流出端电流朝下 `(0,-1)`，引脚外法向必须朝下 `(0,-1)`。
*!      - 由于刚体变换（旋转/镜像）保持引脚间夹角不变，正交的 `:c`（`in`）与 `:b`（`in`）永远不可能同时朝上！
*!      - 最优折中朝向为 `R0` 或 `MY`：此时 `:c`（流入朝下）与 `:e`（流出朝下）2 个端口完全满足，`:b`（流入水平向右或向左）必然偏离 90°。若旋转为 `R270`/`MXR90` 迁就 `:b`，则 `:c` 与 `:e` 两个端口同时变为水平而失效（1 满足 vs 2 违反，代价更高）。
*!   2. 反向双输出分流器子电路（`#XSPLIT`，1 个顶部流入端 `in_top` + 左右两个反向流出端 `out_l`、`out_r`）：
*!      - `out_l` 引脚朝左 `(-1,0)`，`out_r` 引脚朝右 `(+1,0)`，两者夹角 180°。
*!      - 在任意朝向 θ 下，`vec(out_l) = -vec(out_r)`，因此 `out_l` 与 `out_r` 的流出电流方向永远相反，绝不可能同时朝下！
*!      - 最优折中朝向为 `R0` 或 `MY`：满足 `in_top` 电流朝下，而 `out_l`、`out_r` 各自水平偏离 90°（避免出现某个端口逆流朝上偏离 180° 的最大惩罚）。
*!   3. 不平衡惠斯通电桥横向桥臂（`#R1`–`#R4` 构成左右对称支路，`#RBRIDGE` 跨接在等高对称中点 `va` 与 `vb` 之间）：
*!      - 左右桥臂 `#R1/#R2` 与 `#R3/#R4` 关于垂直轴镜像对称（`$MIRRORED`），使中点 `va`（左）与 `vb`（右）位于同一水平高度。
*!      - 当不平衡电桥中直流电流从 `va` 流向 `vb`（`#RBRIDGE:p @CURRENT("in")`, `#RBRIDGE:n @CURRENT("out")`）时，`current_direction` 偏好 `#RBRIDGE` 采用 `R0`/`MY`（竖直向下），但两侧对称端点 `va`（左）与 `vb`（右）是水平排列的，几何走线方向（水平从左到右，`R270`/`MYR90`）与 `current_direction` 竖直向下偏好产生拓扑与布局张力。
* @RNL(type="header") { @VERSION("0.3") }

.model npn_std npn level=1

* -----------------------------------------------------------------------------
* Subcircuit: T-junction current splitter with opposing left/right outputs
* -----------------------------------------------------------------------------
.subckt current_splitter in_top out_l out_r
RSL in_top out_l 1k
RSR in_top out_r 1k
* @RNL {
*   # @SELECT-INSTANCE(type="resistor") => @SET-SYMBOL $RESISTOR;
*   $SPLITTER_SYM = @DEFINE-SYMBOL(box=[-3, -2, 3, 2]) {
*     (0, 2) @PIN(terminal="in_top", direction="up");
*     (-3, 0) @PIN(terminal="out_l", direction="left");
*     (3, 0) @PIN(terminal="out_r", direction="right");
*     @LINE {(-2, -1.5) (2, -1.5) (2, 1.5) (-2, 1.5) (-2, -1.5)};
*     @LINE {(0, 1.5) (0, 2)};
*     @LINE {(-3, 0) (-2, 0)};
*     @LINE {(2, 0) (3, 0)};
*     ? @TEXT("${name}", key="name") => @PLACE(0, 2.4)
*   };
*   # @SET-SYMBOL $SPLITTER_SYM;
*
*   {#RSL:p #RSR:p #:in_top} @CURRENT("in");
*   {#RSL:n #RSR:n #:out_l #:out_r} @CURRENT("out");
*   $WIRE_NET_SET @ROUTE
* }
.ends current_splitter

* -----------------------------------------------------------------------------
* Top-level circuit combining the 3 conflict scenarios
* -----------------------------------------------------------------------------
VCC vcc 0 DC 5.0

* Scenario 1: BJT Q1 with ALL THREE terminals (c, b, e) annotated with @CURRENT
RC1 vcc vc1 2k
RB1 vcc vb1 50k
Q1 vc1 vb1 ve1 npn_std
RE1 ve1 0 1k

* Scenario 2: T-junction current splitter XSPLIT (in_top="in", out_l="out", out_r="out")
XSPLIT vcc va vb current_splitter

* Scenario 3: Unbalanced symmetric bridge fed by XSPLIT outputs va and vb
R1 vcc va 2k
R2 va 0 4k
R3 vcc vb 4k
R4 vb 0 2k
RBRIDGE va vb 1k

* @RNL {
*   # @SELECT-INSTANCE(type="npn") => @SET-SYMBOL $NPN;
*   # @SELECT-INSTANCE(type="resistor") => @SET-SYMBOL $RESISTOR;
*
*   {#RC1 #Q1 #RE1} @AS $BRANCH;
*   {#R1 #R2} @AS $BRANCH;
*   {#R3 #R4} @AS $BRANCH;
*
*   %ax_br = (_, _);
*   #R1(a) #R3(b) %ax_br(axis) @AS $MIRRORED;
*   #R2(a) #R4(b) %ax_br(axis) @AS $MIRRORED;
*
*   {#RC1:p #RB1:p #XSPLIT:in_top #R1:p #R3:p} @AS $SUPPLY_HIGH;
*   {#RE1:n #R2:n #R4:n} @AS $SUPPLY_LOW;
*
*   /* Scenario 1: Q1 has c="in", b="in", e="out" — c and b are orthogonal (90 deg), impossible to both point down */
*   {#RC1:p #RB1:p #Q1:c #Q1:b #RE1:p} @CURRENT("in");
*   {#RC1:n #RB1:n #Q1:e #RE1:n} @CURRENT("out");
*
*   /* Scenario 2: XSPLIT has in_top="in" and opposing out_l="out", out_r="out" (180 deg apart) */
*   #XSPLIT:in_top @CURRENT("in");
*   {#XSPLIT:out_l #XSPLIT:out_r} @CURRENT("out");
*
*   /* Scenario 3: Unbalanced bridge arms + cross-bridge resistor RBRIDGE */
*   {#R1:p #R2:p #R3:p #R4:p #RBRIDGE:p} @CURRENT("in");
*   {#R1:n #R2:n #R3:n #R4:n #RBRIDGE:n} @CURRENT("out");
*
*   &vcc @ADD-TO $SUPPLY_NET_SET;
*   &0 @ADD-TO $GROUND_NET_SET;
*   $WIRE_NET_SET @ROUTE
* }

.op
.end
