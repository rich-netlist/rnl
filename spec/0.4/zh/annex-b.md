<!-- translated from en/annex-b.md sha256:f5f5cbcacdf4 -->
## 附录 B 标准例句（说明性）

标准例句是在标准写法的记号之间加上自由文本、读起来像句子的语句。自由文本不改变含义（3.3）；措辞的规则见 4.5。表里句首的 `…` 代表管道里它前面的段，`{…}` 和句末的 `…` 代表省略的内容。

### B.1 放置与朝向

| 关键字 | 情况 | 例句 |
|---|---|---|
| PLACE | 两轴 | `#M1 is at @PLACE(10, 20)` |
| | 一轴 | `the x of #M1 is at @PLACE(x=10)` |
| | 编辑器写的 | `the x of #X2 is at @PLACE(x=3, priority="high")` |
| | 管道 | `… => it is at @PLACE(40, 0)` |
| ORIENT | 转 | `#M2 @ORIENT("R90")` |
| | 镜像 | `#M2 @ORIENT("MX"), i.e., flipped upside down` |
| | | `#M2 @ORIENT("MY"), i.e., flipped left-right` |
| | | `#M3 @ORIENT("MXR90"), i.e., flipped upside down, then rotated 90°` |
| | | `#M3 @ORIENT("MYR90"), i.e., flipped left-right, then rotated 90°` |
| | 二选一 | `#M5 @ORIENT(["R0", "MY"]), i.e., not turned or flipped left-right` |
| | 管道 | `… => they @ORIENT("R90")` |
| ORIENT-RELATION | 两个对象 | `from #M1 to #M2, they have @ORIENT-RELATION("MY"), i.e., flipped left-right` |
| | 二选一 | `from #M3 to #M4, they have @ORIENT-RELATION(["R0", "MY"]), i.e., the same or flipped left-right` |
| | 朝向相同 | `from #M5 to {#M7 #M8}, they have @ORIENT-RELATION("R0"), i.e., the same orientation` |
| LAYER | | `#X is on @LAYER("bottom")` |
| | 管道 | `… => they are on @LAYER("top")` |
| GROUP | | `{#M1 #M2 #M5} as a @GROUP` |
| | 命名 | `%s1_set = {#M1 #M2 #M5} as a @GROUP` |
| | 间距 | `{#M3 #M4} as a @GROUP(spacing_x=-1), i.e., one grid unit tighter in x` |

### B.2 布局约束

| 关键字 | 情况 | 例句 |
|---|---|---|
| ALIGN | y 相同 | `{#M1 #M2} @ALIGN("y"), i.e., same y` |
| | x 相同 | `{#M1 #C1} @ALIGN("x"), i.e., same x` |
| | 低优先级 | `{#M7 #M5} @ALIGN("y", priority="low"), i.e., same y` |
| | 管道 | `… => they @ALIGN("y")` |
| ORDER | 有方向 | `{#M1 #M2 #M3} are in @ORDER("left-to-right")` |
| | 管道 | `… => they are in @ORDER("bottom-to-top")` |
| DISTRIBUTE | | `{#R1 #R2 #R3} are in @DISTRIBUTE("left-to-right"), i.e., in equal steps` |
| | 管道 | `… => they are in @DISTRIBUTE("bottom-to-top"), i.e., in equal steps` |
| DISTANCE | 有方向 | `from #M1 to #M2, they have @DISTANCE("bottom-to-top", min=4)` |
| | 从集合起 | `from {#M6 #M7 #CL} to %pout, they have @DISTANCE("left-to-right", min=2)` |
| TABLE | | `@TABLE layout: {{#M1 #M2 #M3} {#M4 {#M5 #M6} #M7}}` |
| | 横排 | `@TABLE(transpose=true) layout: {{#M1 #M2} {#M3 #M4}}` |
| PRIORITY | | `%c1 has @PRIORITY("disabled")` |
| | 管道 | `… => they have @PRIORITY("low")` |
| SOLVER-CONFIG | | `@SOLVER-CONFIG(spacing_x=2, spacing_y=1)` |

- `the x of`（或 `the y of`）只写在只给一轴的 PLACE 前面（B.1）；ALIGN 改用注解 `, i.e., same x`。
- 这里的 x 是各成员的位置：元件是 PLACE 定的点（它的符号的原点），不是外框的中心；集合分别代表它的每一个成员（14.1）。

### B.3 外观

| 关键字 | 情况 | 例句 |
|---|---|---|
| SET-SYMBOL | 一个对象 | `#M1 @SET-SYMBOL to $NMOS` |
| | 集合 | `{#M1 #M2} @SET-SYMBOL to $NMOS` |
| | 子电路的 block symbol | `this subcircuit # @SET-SYMBOL to $NAND2_SYM` |
| | 管道 | `… => they @SET-SYMBOL to $PMOS` |
| SET-STYLE | | `{#CC #RZ} @SET-STYLE to $orange` |
| | 管道 | `… => they @SET-STYLE to $grey` |
| DEFINE-STYLE | | `$orange = @DEFINE-STYLE(linecolor="#e07000", textcolor="#e07000") {}` |
| | 并入别的样式 | `$wide = @DEFINE-STYLE(linewidth=2) based on {$orange}` |

### B.4 符号与图元

| 关键字 | 情况 | 例句 |
|---|---|---|
| DEFINE-SYMBOL | | `$TRI = @DEFINE-SYMBOL {…}` |
| PIN | | `at (-4, 0) there is a @PIN(terminal="in", direction="left")` |
| LINE | | `through {(0, 0) (4, 0)} runs a @LINE` |
| ARC | | `through {(0, 2) (2, 0) (0, -2)} runs an @ARC` |
| BEZIER | | `the control points {(0, 0) (1, 1) (2, 1) (3, 0)} make a @BEZIER` |
| CIRCLE | | `centered at (0, 0), a @CIRCLE(radius=1)` |
| RECTANGLE | 中心加尺寸 | `centered at (0, 0), a @RECTANGLE(width=4, height=2)` |
| | 两个对角 | `with corners {(0, 0) (4, 2)}, a @RECTANGLE` |
| ELLIPSE | 中心加尺寸 | `centered at (0, 0), an @ELLIPSE(width=4, height=2)` |
| | 外框的对角 | `in the box with corners {(0, 0) (4, 2)}, an @ELLIPSE` |
| TEXT | 符号里 | `on ? there is a @TEXT("${name}", key="name") => it is at @PLACE(-2, 2.5)` |
| ATTACH | 符号里 | `$ARROW is @ATTACH to (0, -1.5) => …` |

- 写在场景里的图元（标注，13.4），句子相同。

### B.5 导线与文字

| 关键字 | 情况 | 例句 |
|---|---|---|
| WIRE | | `net &n1, through {#R1:2 %j #R2:1}, has a @WIRE` |
| | 总线 | `net &D<3:0>, through {%p1 %p2}, has a @WIRE, which is a bus` |
| | 走法 | `net &in, through {#VIN:1 #R1:1}, has a @WIRE(mode="vertical-horizontal")` |
| ROUTE-NET | | `net &n1 is drawn with @ROUTE-NET` |
| | 集合 | `nets {&a &b} are drawn with @ROUTE-NET` |
| | 管道 | `… => it is drawn with @ROUTE-NET` |
| ROUTE-OBJECT | | `the pins of {#M1 #M2 #M5} are joined with @ROUTE-OBJECT` |
| UNROUTE-NET | | `net &vdd is left out with @UNROUTE-NET(priority="high")` |
| UNROUTE-OBJECT | | `the pins of #M3 are left out with @UNROUTE-OBJECT` |
| TEXT | 对象上 | `on #M1 there is a @TEXT("W/L = ${w}/${l}", key="wl", dy=-3)` |
| | 点上 | `at (_, _) there is a @TEXT("RC low-pass") => named %title` |
| | 管道 | `… => on them there is a @TEXT("${w}", key="size")` |

- 给点命名，如 `%j = (_, _)`，不加词。

### B.6 定义与套用

| 关键字 | 情况 | 例句 |
|---|---|---|
| DEFINE | | `$PAIR = a definition, @DEFINE(roles=["a", "b"], parameters=[["gap", 2]]) {…}` |
| AS | 三个角色 | `#M1(left), #M2(right) and #M5(tail) are treated @AS $DIFFERENTIAL_PAIR_N` |
| | 收集合的角色 | `{#M5 #M7 #M8} are treated @AS $MATCHED` |
| | 一个对象 | `#M3 is treated @AS $DIODE_CONNECTED` |
| | 带优先级 | `#M1(left), #M2(right) and %ax(axis) are treated @AS(priority="high") $MIRRORED_LAYOUT` |
| | 带普通定义的参数 | `{#R1 #R2} are treated @AS(gap=6) $SPACED` |
| | 角色里填用例 | `%dp(pair) and %cm(load) are treated @AS $OTA` |
| | 命名 | `%dp = #M1(left), #M2(right) and #M5(tail) treated @AS $DIFFERENTIAL_PAIR_N` |
| | 管道里命名、又加对象 | `#M1(left), #M2(right) and #M5(tail) are treated @AS $DIFFERENTIAL_PAIR_N => named %dp => it and %cm are treated @AS $OTA` |
| ATTACH | 网络上 | `$GROUND is @ATTACH to &vss` |
| | 端子上 | `$NO_CONNECT is @ATTACH to #M1:4` |
| | 点上 | `$JUNCTION is @ATTACH to %j` |
| | 两个角色 | `$DIFF_PORT is @ATTACH to &inp(a) and &inn(b)` |
| | 后面接别的、命名 | `$GROUND is @ATTACH to &vss => it is at @PLACE(24, 38) => named %g0` |
| | 添在每个对象上 | `$SIZE_LABEL is @ATTACH to {#M1 #M2 #M3}` |
| | 集合上 | `$GROUND_STUB is @ATTACH to $GROUND_FILL_SET` |
| | 管道里的补位 | `among the instances at each level in # and below, @SELECT-INSTANCE(recursive=true) => among the pins of them, @SELECT-PIN(net="vss", wired=false) => $GROUND_STUB is @ATTACH(gap=3) to them` |

- 对象合起来构成这个东西时用 AS，如电路结构、摆法，或 `$RC_STAGE` 这样的局部画法；这个东西添在对象上时用 ATTACH，如地、端口标签、无连接标记、交汇圆点、每个元件一个的标签、补位。两个关键字做的是同一件事，只是定义写的位置不同（9.4）。
- `gap` 是 `$GROUND_STUB` 的参数：从引脚往下到地符号的最小距离，默认 1。它和角色名 `left`、`axis` 一样，是定义里起的名字，不是标准词。

### B.7 筛选

筛选一律写成在候选里选：`among <candidates>, @SELECT-…(conditions)`。候选的措辞按范围的种类和要选的种类定：

| 范围 | 要选的 | 候选 |
|---|---|---|
| `#` | 任何种类 | `the <kind> in # only` |
| `#`，会触及下层的语句（10.1） | 任何种类 | `the <kind> at each level in # and below` |
| 同一种类的集合（筛） | 同种类 | `{…}` |
| 网络 | 元件 | `the instances connected to &en` |
| 网络 | 端子 | `the terminals connected to &en` |
| 网络 | 导线 | `the wires on &en` |
| 元件 | 端子 | `the terminals of #R1` |
| 元件、叠画、有符号的用例 | 引脚 | `the pins of #M1` |
| 端子 | 引脚 | `the pins of #X1:vdd` |
| 网络 | 引脚 | `the pins on &en` |
| 管道的结果 | 引脚 | `the pins of them` |
| 元件 | 网络 | `the nets connected to #M1` |
| 端子 | 元件 | `the instances owning #M1:g` |
| 端子 | 网络 | `the nets connected to #M1:g` |
| 导线 | 网络 | `the nets of %w13` |
| 元件、端子、导线、网络、图元 | 标签 | `the labels attached to #M1` |
| 用例（成员里没有用例） | 任何种类 | `the members of %dp` |
| 用例（成员里有用例，或不知道） | 任何种类 | `all members of %st1 at any depth` |
| 用例 | 用例（SELECT-USAGE） | `%st1 itself and the usages inside it` |
| 任何对象 | 包含它的用例（SELECT-ANCESTOR-USAGE） | `the usages containing #M1` |
| 几种对象混在一起的集合（种类算不出） | | 改写成 `for %mixed_set, @SELECT-…` |

- `<kind>` 按关键字写：instances、terminals、pins、nets、wires、labels、annotations、usages。
- 管道中间的筛选，候选里的范围写成 `them`，如 `=> among all members of them at any depth,`。写句子时不知道前一段筛选出的用例的成员里有没有用例，所以写 `at any depth`。
- 用例的成员写 `at any depth`，不写 `at any level`：level 指电路的层，如 `at each level in # and below`。
- 包里的规则视作写在它的 USE-PACKAGE 那一层（10.4）；带 `recursive=true` 时，包里写 `at every level that uses this package, #`，代替 `at each level in # and below`。

| 关键字 | 情况 | 例句 |
|---|---|---|
| SELECT-INSTANCE | 本层及以下 | `among the instances at each level in # and below, @SELECT-INSTANCE(model="nch", recursive=true) => they @SET-SYMBOL to $NMOS` |
| | 只在本层 | `among the instances in # only, @SELECT-INSTANCE(type="pmos") => …` |
| | 筛 | `among {#M1 #M2 #R1}, @SELECT-INSTANCE(name="M*") => they @ORIENT("MY")` |
| | 网络上的 | `among the instances connected to &en, @SELECT-INSTANCE => they @SET-STYLE to $grey` |
| | 端子所属的 | `among the instances owning #M1:g, @SELECT-INSTANCE => they are on @LAYER("top")` |
| | 用例的成员 | `among all members of %st1 at any depth, @SELECT-INSTANCE(role="pair")` |
| | 不含嵌套用例的用例的成员 | `among the members of %dp, @SELECT-INSTANCE` |
| | 管道中间 | `among the usages at each level in # and below, @SELECT-USAGE(recursive=true) of $DIFFERENTIAL_PAIR_N => among all members of them at any depth, @SELECT-INSTANCE => they @SET-STYLE to $signal` |
| SELECT-TERMINAL | 本层及以下 | `among the terminals at each level in # and below, @SELECT-TERMINAL(net="vss", recursive=true) => they have @ASSERT-TERMINAL(same=["net"])` |
| | 元件的 | `among the terminals of #R1, @SELECT-TERMINAL => on them there is a @TEXT("${net}")` |
| | 网络上的 | `among the terminals connected to $GROUND_NET_SET, @SELECT-TERMINAL => among the pins of them, @SELECT-PIN(wired=false) => $GROUND_STUB is @ATTACH to them` |
| SELECT-PIN | 本层里的 | `among the pins in # only, @SELECT-PIN(wired=false) => on them there is a @TEXT("${net}", key="netname")` |
| | 多个端子的 | `among the pins of {#M1:s #M2:s}, @SELECT-PIN(wired=false) => $GROUND_STUB is @ATTACH to them` |
| | 单个端子的 | `among the pins of #X1:vdd, @SELECT-PIN => they @SET-STYLE to $red` |
| SELECT-NET | 本层及以下 | `among the nets at each level in # and below, @SELECT-NET(port=true, recursive=true) => $IO_PORT is @ATTACH to them` |
| | 元件接的 | `among the nets connected to #M1, @SELECT-NET => named %m1_net_set` |
| | 端子所在的 | `among the nets connected to #M1:g, @SELECT-NET => they are drawn with @ROUTE-NET` |
| | 导线所在的 | `among the nets of %w13, @SELECT-NET => they are drawn with @ROUTE-NET` |
| | 筛 | `among {&a &b}, @SELECT-NET(wired=false) => they are drawn with @ROUTE-NET` |
| SELECT-WIRE | 本层及以下 | `among the wires at each level in # and below, @SELECT-WIRE(bus=true, recursive=true) => they @SET-STYLE to $bus` |
| | 网络上的 | `among the wires on &clk, @SELECT-WIRE => they @SET-STYLE to $red` |
| SELECT-LABEL | 本层及以下 | `among the labels at each level in # and below, @SELECT-LABEL(key="wl", recursive=true) => they @SET-STYLE to $grey` |
| | 附着在对象上的 | `among the labels attached to #M1, @SELECT-LABEL => they @SET-STYLE to $grey` |
| SELECT-ANNOTATION | 本层及以下 | `among the annotations at each level in # and below, @SELECT-ANNOTATION(recursive=true) => they @SET-STYLE to $base` |
| | 筛 | `among {%n1 %f1 #M1}, @SELECT-ANNOTATION` |
| SELECT-USAGE | 本层及以下 | `among the usages at each level in # and below, @SELECT-USAGE(recursive=true) of $DIFFERENTIAL_PAIR_N => they have @PRIORITY("disabled")` |
| | 不写定义 | `among the usages at each level in # and below, @SELECT-USAGE(recursive=true) => they @SET-STYLE to $base` |
| | 用例自身和里面 | `among %st1 itself and the usages inside it, @SELECT-USAGE of $DIFFERENTIAL_PAIR_N` |
| SELECT-ANCESTOR-USAGE | | `among the usages containing #M1, @SELECT-ANCESTOR-USAGE of $DIFFERENTIAL_PAIR_N => named %dp_set` |
| 任何筛选 | 范围种类未知 | `for %mixed_set, @SELECT-TERMINAL => …` |

### B.8 集合与其他关键字

| 关键字 | 情况 | 例句 |
|---|---|---|
| ADD-TO | | `&vdd @ADD-TO $SUPPLY_NET_SET` |
| | 管道 | `… => they @ADD-TO $SUPPLY_NET_SET` |
| REMOVE-FROM | | `&agnd @REMOVE-FROM $GROUND_NET_SET` |
| UNION | | `%u_set = %a_set and %b_set @UNION` |
| BUNDLE | | `#XI<3:0> is drawn as a @BUNDLE => named %xi` |
| CURRENT | 流入 | `#M5:1 has @CURRENT("in")` |
| | 流出 | `#:iout has @CURRENT("out")` |
| | 不定 | `#M5:2 has @CURRENT("unspecified")` |
| ASSERT-INSTANCE | | `{#M1 #M2} have @ASSERT-INSTANCE(same=["model", "w", "l"])` |
| ASSERT-TERMINAL | | `{#M1:s #M2:s} have @ASSERT-TERMINAL(same=["net"])` |
| ASSERT-NET | | `{&inp &inn} have @ASSERT-NET(port=true)` |
| 没有网络的 WIRE | | `between {%sub1::y %int1::a}, on no net &, there is a @WIRE` |
| ANNOTATION | | `at (40, 20) there is an @ANNOTATION(rotate=30) of $ARROW_SYMBOL` |
| ASSERT-USAGE | | `%dp1 is, by @ASSERT-USAGE, a usage of $DIFFERENTIAL_PAIR_N` |
| ASSERT-IN | | `{&vss} is, by @ASSERT-IN, in $GROUND_NET_SET` |
| ASSERT-TOPOLOGY | 串联 | `{#R1 #R2 #R3} are connected as @ASSERT-TOPOLOGY("series")` |
| | 并联 | `{#M3 #M4} are connected as @ASSERT-TOPOLOGY("parallel")` |
| | 星形 | `{#L1 #C1 #L2} are connected as @ASSERT-TOPOLOGY("star")` |
| | 回路 | `{#C1 #L1 #C2} are connected as @ASSERT-TOPOLOGY("loop")` |

### B.9 头段记录与包

不加词。头段记录按标准拼写读，不用代称（16.5）。

| 关键字 | 情况 | 例句 |
|---|---|---|
| VERSION | | `@VERSION("0.4")` |
| ALIAS | | `@ALIAS("@位于", "@PLACE")` |
| USE-ALIASES | | `@USE-ALIASES(name="alias-zh", version="0.4")` |
| DEVICE | | `@DEVICE(…)` |
| PACKAGE | | `@PACKAGE(name="acme-style", version="1.2")` |
| USE-PACKAGE | 头段记录里；包文件的头段记录里引用别的包也这样写 | `@USE-PACKAGE(name="rnl-std-all", version="0.4")` |
| | 写在层里 | `@USE-PACKAGE(name="acme-style", version="1.2")` |
