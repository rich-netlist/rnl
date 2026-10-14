<!-- translated from en/annex-b.md sha256:2516f90a5f7e -->
## 附录 B 标准例句（说明性）

标准例句是在标准写法的记号之间加上自由文本、读起来像句子的语句。自由文本不改变含义（3.3）；措辞的规则见 4.5。表里句首的 `…` 代表管道里它前面的段，`{…}` 和句末的 `…` 代表省略的内容。

中文版每条列出英文例句和中文例句。中文例句用代称包 `rnl-zh` 的代称（4.5、16.5）。

### B.1 放置与朝向

| 关键字 | 情况 | 英文 | 中文 |
|---|---|---|---|
| PLACE | 两轴 | `#M1 is at @PLACE(10, 20)` | `#M1 @位于(10, 20)` |
| | 一轴 | `the x of #M1 is at @PLACE(x=10)` | `#M1 的 x 坐标 @位于(x=10)` |
| | 编辑器写的 | `the x of #X2 is at @PLACE(x=3, priority="high")` | `#X2 的 x 坐标 @位于(x=3, 优先级="高")` |
| | 管道 | `… => it is at @PLACE(40, 0)` | `… => 其 @位于(40, 0)` |
| ORIENT | 转 | `#M2 @ORIENT("R90")` | `#M2 @朝向("R90")` |
| | 镜像 | `#M2 @ORIENT("MX"), i.e., flipped upside down` | `#M2 @朝向("MX")，即上下镜像` |
| | | `#M2 @ORIENT("MY"), i.e., flipped left-right` | `#M2 @朝向("MY")，即左右镜像` |
| | | `#M3 @ORIENT("MXR90"), i.e., flipped upside down, then rotated 90°` | `#M3 @朝向("MXR90")，即上下镜像再转 90°` |
| | | `#M3 @ORIENT("MYR90"), i.e., flipped left-right, then rotated 90°` | `#M3 @朝向("MYR90")，即左右镜像再转 90°` |
| | 管道 | `… => they @ORIENT("R90")` | `… => 其 @朝向("R90")` |
| ORIENT-RELATION | 两个对象 | `from #M1 to #M2, they have @ORIENT-RELATION("MY"), i.e., flipped left-right` | `#M1 到 #M2 的 @朝向关系("MY")，即左右镜像` |
| | 集合 | `from {#M1 #M3} to {#M2 #M4}, they have @ORIENT-RELATION("MY"), i.e., flipped left-right` | `{#M1 #M3} 到 {#M2 #M4} 的 @朝向关系("MY")，即左右镜像` |
| | 朝向相同 | `from #M5 to {#M7 #M8}, they have @ORIENT-RELATION("R0"), i.e., the same orientation` | `#M5 到 {#M7 #M8} 的 @朝向关系("R0")，即朝向相同` |
| LAYER | | `#X is on @LAYER("bottom")` | `#X 处于 @图层("底")` |
| | 管道 | `… => they are on @LAYER("top")` | `… => 其处于 @图层("顶")` |
| GROUP | | `{#M1 #M2 #M5} as a @GROUP` | `{#M1 #M2 #M5} 视作一个 @分组` |
| | 命名 | `%s1_set = {#M1 #M2 #M5} as a @GROUP` | `%s1_set = {#M1 #M2 #M5} 视作一个 @分组` |

### B.2 布局约束

| 关键字 | 情况 | 英文 | 中文 |
|---|---|---|---|
| ALIGN | y 相同 | `{#M1 #M2} @ALIGN("y"), i.e., same y` | `{#M1 #M2} @对齐("y")，即 y 坐标相同` |
| | x 相同 | `{#M1 #C1} @ALIGN("x"), i.e., same x` | `{#M1 #C1} @对齐("x")，即 x 坐标相同` |
| | 低优先级 | `{#M7 #M5} @ALIGN("y", priority="low"), i.e., same y` | `{#M7 #M5} @对齐("y", 优先级="低")，即 y 坐标相同` |
| | 管道 | `… => they @ALIGN("y")` | `… => 其 @对齐("y")` |
| ORDER | 有方向 | `{#M1 #M2 #M3} are in @ORDER("left-to-right")` | `{#M1 #M2 #M3} 按 @顺序("从左到右") 排列` |
| | 不分方向 | `the x of {#M1 #M1:2 #M2:2 #M2} are in @ORDER("x"), either ascending or descending` | `{#M1 #M1:2 #M2:2 #M2} 的 x 坐标按 @顺序("x") 排列，正倒序皆可` |
| | 管道 | `… => they are in @ORDER("bottom-to-top")` | `… => 其按 @顺序("从下到上") 排列` |
| DISTRIBUTE | | `the x of {#R1 #R2 #R3} @DISTRIBUTE("x"), i.e., in equal steps` | `{#R1 #R2 #R3} 的 x 坐标 @等距("x")` |
| | 管道 | `… => the x of them @DISTRIBUTE("x"), i.e., in equal steps` | `… => 其 x 坐标 @等距("x")` |
| DISTANCE | 有方向 | `from #M1 to #M2, they have @DISTANCE("bottom-to-top", min=4)` | `#M1 到 #M2 有 @距离("从下到上", 最小值=4)` |
| | 集合 | `from {#M6 #M7 #CL} to %pout, they have @DISTANCE("left-to-right", min=2)` | `{#M6 #M7 #CL} 到 %pout 有 @距离("从左到右", 最小值=2)` |
| | 不分方向 | `between %a_set and %b_set, they have @DISTANCE("x", min=1)` | `%a_set 和 %b_set 之间有 @距离("x", 最小值=1)` |
| PRIORITY | | `%c1 has @PRIORITY("disabled")` | `%c1 的 @优先级("停用")` |
| | 管道 | `… => they have @PRIORITY("low")` | `… => 其 @优先级("低")` |

- `the x of`（或 `the y of`），中文"的 x 坐标"（或"的 y 坐标"），只写在方向或轴是 `"x"`（或 `"y"`）的 ORDER、DISTRIBUTE 前面，以及只给一轴的 PLACE 前面（B.1）；ALIGN 改用注解 `, i.e., same x`，中文"，即 x 坐标相同"。
- 这里的 x 坐标是各成员的位置：元件是 PLACE 定的点（符号的原点），不是外框的中心；集合按中心算（14.1）。

### B.3 外观

| 关键字 | 情况 | 英文 | 中文 |
|---|---|---|---|
| SET-SYMBOL | 一个对象 | `#M1 @SET-SYMBOL to $NMOS` | `#M1 @使用符号 $NMOS` |
| | 集合 | `{#M1 #M2} @SET-SYMBOL to $NMOS` | `{#M1 #M2} @使用符号 $NMOS` |
| | 子电路的 block symbol | `this subcircuit # @SET-SYMBOL to $NAND2_SYM` | `本子电路 # @使用符号 $NAND2_SYM` |
| | 管道 | `… => they @SET-SYMBOL to $PMOS` | `… => 其 @使用符号 $PMOS` |
| SET-STYLE | | `{#CC #RZ} @SET-STYLE to $orange` | `{#CC #RZ} @使用样式 $orange` |
| | 管道 | `… => they @SET-STYLE to $grey` | `… => 其 @使用样式 $"灰色"` |
| DEFINE-STYLE | | `$orange = @DEFINE-STYLE(linecolor="#e07000", textcolor="#e07000") {}` | `$orange = @定义样式(线色="#e07000", 字色="#e07000") {}` |
| | 并入别的样式 | `$wide = @DEFINE-STYLE(linewidth=2) based on {$orange}` | `$wide = @定义样式(线宽=2)，以 {$orange} 为基础` |

### B.4 符号与图元

| 关键字 | 情况 | 英文 | 中文 |
|---|---|---|---|
| DEFINE-SYMBOL | | `$TRI = @DEFINE-SYMBOL {…}` | `$TRI = @定义符号 {…}` |
| PIN | | `at (-4, 0) there is a @PIN(terminal="in", direction="left")` | `在 (-4, 0) 有一个 @引脚(端子="in", 方向="左")` |
| LINE | | `a @LINE running through {(0, 0) (4, 0)}` | `一条 @折线 经过 {(0, 0) (4, 0)}` |
| ARC | | `an @ARC running through {(0, 2) (2, 0) (0, -2)}` | `一条 @圆弧 经过 {(0, 2) (2, 0) (0, -2)}` |
| BEZIER | | `a @BEZIER with control points {(0, 0) (1, 1) (2, 1) (3, 0)}` | `一条 @贝塞尔 曲线，控制点为 {(0, 0) (1, 1) (2, 1) (3, 0)}` |
| CIRCLE | | `centered at (0, 0), a @CIRCLE(radius=1)` | `以 (0, 0) 为中心的 @圆(半径=1)` |
| RECTANGLE | 中心加尺寸 | `centered at (0, 0), a @RECTANGLE(width=4, height=2)` | `以 (0, 0) 为中心的 @矩形(宽度=4, 高度=2)` |
| | 两个对角 | `a @RECTANGLE with corners {(0, 0) (4, 2)}` | `一个 @矩形 以 {(0, 0) (4, 2)} 为对角` |
| ELLIPSE | 中心加尺寸 | `centered at (0, 0), an @ELLIPSE(width=4, height=2)` | `以 (0, 0) 为中心的 @椭圆(宽度=4, 高度=2)` |
| | 外框的对角 | `an @ELLIPSE with box corners {(0, 0) (4, 2)}` | `一个 @椭圆 以 {(0, 0) (4, 2)} 为外框对角` |
| TEXT | 符号里 | `on ? there is a @TEXT("${name}", key="name") => it is at @PLACE(-2, 2.5)` | `? 上有 @文本("${名字}", 引用名="name") => 其 @位于(-2, 2.5)` |
| ATTACH | 符号里 | `$ARROW is @ATTACH to (0, -1.5) => …` | `$"箭头" @附着 在 (0, -1.5) 上 => …` |

- 写在场景里的图元（标注，13.4），句子相同。

### B.5 导线与文字

| 关键字 | 情况 | 英文 | 中文 |
|---|---|---|---|
| WIRE | | `net &n1 has a @WIRE connecting {#R1:2 %j #R2:1}` | `网络 &n1 的 @导线 连接了 {#R1:2 %j #R2:1}` |
| | 总线 | `net &D<3:0> has a @WIRE connecting {%p1 %p2}, which is a bus` | `网络 &D<3:0> 的 @导线 连接了 {%p1 %p2}，此为总线` |
| | 走法 | `net &in has a @WIRE(mode="vertical-horizontal") connecting {#VIN:1 #R1:1}` | `网络 &in 的 @导线(模式="先竖后横") 连接了 {#VIN:1 #R1:1}` |
| ROUTE | | `net &n1 is fully @ROUTE` | `网络 &n1 全部 @布线` |
| | 集合 | `nets {&a &b} are fully @ROUTE` | `网络 {&a &b} 全部 @布线` |
| | 管道 | `… => it is fully @ROUTE` | `… => 其全部 @布线` |
| TEXT | 对象上 | `on #M1 there is a @TEXT("W/L = ${w}/${l}", key="wl", dy=-3)` | `#M1 上有 @文本("W/L = ${w}/${l}", 引用名="wl", dy=-3)` |
| | 点上 | `at (_, _) there is a @TEXT("RC low-pass") => named %title` | `在 (_, _) 有 @文本("RC 低通") => 记作 %title` |
| | 管道 | `… => on them there is a @TEXT("${w}", key="size")` | `… => 其上有 @文本("${w}", 引用名="size")` |

- 给点命名，如 `%j = (_, _)`，不加词。

### B.6 定义与套用

| 关键字 | 情况 | 英文 | 中文 |
|---|---|---|---|
| DEFINE | | `$PAIR = a definition, @DEFINE(roles=["a", "b"], parameters=[["gap", 2]]) {…}` | `$PAIR = 一个普通定义， @定义(角色表=["a", "b"], 参数表=[["gap", 2]]) {…}` |
| AS | 两个角色 | `#M1(in_p) and #M2(in_n) are treated @AS $DIFFPAIR` | `#M1(正输入) 和 #M2(负输入) @视作 $"差分对"` |
| | 收集合的角色 | `{#M5 #M7 #M8} are treated @AS $MATCHED` | `{#M5 #M7 #M8} @视作 $"匹配组"` |
| | 一个对象 | `#X1:3 is treated @AS $SUPPLY_HIGH` | `#X1:3 @视作 $"高电源端"` |
| | 三个角色，带参数 | `#M1(a), #M2(b) and %ax(axis) are treated @AS(priority="low") $MIRROR_PAIR` | `#M1(甲)、#M2(乙) 和 %ax(轴) @视作(优先级="低") $"镜像对"` |
| | 带普通定义的参数 | `{#R1 {%rc} #M1} are treated @AS(direction="top-to-bottom") $FLOW` | `{#R1 {%rc} #M1} @视作(方向="从上到下") $"信号流向"` |
| | 填用例 | `%dp(pair) and #M5(tail) are treated @AS $INPUT_STAGE` | `%dp(输入对) 和 #M5(尾管) @视作 $"输入级"` |
| | 命名 | `%dp = #M1(in_p) and #M2(in_n) treated @AS $DIFFPAIR` | `%dp = #M1(正输入) 和 #M2(负输入) @视作 $"差分对"` |
| | 管道里命名、又加对象 | `#M1(in_p) and #M2(in_n) are treated @AS $DIFFPAIR => named %dp => it and #M5 are treated @AS $INPUT_STAGE` | `#M1(正输入) 和 #M2(负输入) @视作 $"差分对" => 记作 %dp => 其与 #M5 @视作 $"输入级"` |
| ATTACH | 网络上 | `$GND is @ATTACH to &vss` | `$"地" @附着 在 &vss 上` |
| | 端子上 | `$NC is @ATTACH to #M1:4` | `$"无连接" @附着 在 #M1:4 上` |
| | 点上 | `$JUNCTION is @ATTACH to %j` | `$"交汇点" @附着 在 %j 上` |
| | 两个角色 | `$DIFF_PORT is @ATTACH to &inp(a) and &inn(b)` | `$DIFF_PORT @附着 在 &inp(a) 和 &inn(b) 上` |
| | 后面接别的、命名 | `$GND is @ATTACH to &vss => it is at @PLACE(24, 38) => named %g0` | `$"地" @附着 在 &vss 上 => 其 @位于(24, 38) => 记作 %g0` |
| | 添在每个对象上 | `$SIZE_LABEL is @ATTACH to {#M1 #M2 #M3}` | `$SIZE_LABEL @附着 在 {#M1 #M2 #M3} 上` |
| | 补位（集合） | `$GND_STUB is @ATTACH to $GND_FILL_SET` | `$"接地桩" @附着 在 $"接地补位集" 上` |
| | 补位（管道） | `among the terminals at each level in # and below, @SELECT-TERMINAL(net="vss", wired=false) => $GND_STUB is @ATTACH(gap=3) to them` | `从本层 # 及以下每一层的端子中 @选出端子(网络="vss", 已接线=否) => $"接地桩" @附着(间距=3) 在其上` |

- 对象合起来构成这个东西时用 AS，如电路结构、摆法，或 `$RC_STAGE` 这样的局部画法；这个东西添在对象上时用 ATTACH，如地、端口符号、无连接标记、汇合圆点、每个元件一个的标签、补位。两个关键字做的是同一件事，只是定义写的位置不同（9.4）。
- `gap` 是 `$GND_STUB` 的参数：从端子往下到地符号的距离，默认 2。它和角色名 `in_p`、`axis` 一样，是定义里起的名字，不是标准词。

### B.7 选择

选择一律写成在候选里选：英文 `among <candidates>, @SELECT-…(conditions)`，中文 `从<候选>中 @选出…(条件)`。候选的说法按范围的种类和要选的种类定：

| 范围 | 要选的 | 英文的候选 | 中文的候选 |
|---|---|---|---|
| `#` | 任何种类 | `the <kind> at each level in # and below` | `本层 # 及以下每一层的<种类>` |
| `#`，`recursive=false` | 任何种类 | `the <kind> in # only` | `本层 # 的<种类>`，句首加"只"：`只从本层 # 的<种类>中` |
| 同一种类的集合（筛） | 同种类 | `{…}` | `{…}` |
| 网络 | 元件 | `the instances connected to &en` | `接在 &en 上的元件` |
| 网络 | 端子 | `the terminals connected to &en` | `接在 &en 上的端子` |
| 网络 | 导线 | `the wires on &en` | `&en 上的导线` |
| 元件 | 端子 | `the terminals of #R1` | `#R1 的端子` |
| 元件 | 网络 | `the nets connected to #M1` | `与 #M1 相连的网络` |
| 端子 | 元件 | `the instances owning #M1:g` | `#M1:g 所属的元件` |
| 端子 | 网络 | `the nets connected to #M1:g` | `#M1:g 所在的网络` |
| 导线 | 网络 | `the nets of %w13` | `%w13 所在的网络` |
| 元件、端子、导线、网络、图元 | 标签 | `the labels attached to #M1` | `附着在 #M1 上的标签` |
| 用例（成员里没有用例） | 任何种类 | `the members of %dp` | `%dp 的成员` |
| 用例（成员里有用例，或不知道） | 任何种类 | `all members of %st1 at any depth` | `%st1 的各级成员` |
| 用例 | 用例（SELECT-USAGE） | `%st1 itself and the usages inside it` | `%st1 自身及其内部的用例` |
| 任何对象 | 所在的用例（SELECT-ANCESTOR-USAGE） | `the usages containing #M1` | `包含 #M1 的用例` |
| 几种对象混在一起的集合（种类算不出） | | write `for %mixed_set, @SELECT-…` instead | 改写成 `对于 %mixed_set， @选出…` |

- `<kind>` 按关键字写：instances、terminals、nets、wires、labels、annotations、usages；中文是元件、端子、网络、导线、标签、标注、用例。
- 管道中间的选择，候选里的范围英文写 `them`，中文写"其"，如 `=> among all members of them at any depth,`、`=> 从其各级成员中`。写句子时不知道前一段选出的用例的成员里有没有用例，所以写 `at any depth`、"各级"。
- 用例的成员写 `at any depth`、"各级"，不写 `at any level`、"各层"：层指电路的层，如 `at each level in # and below`、"本层 # 及以下每一层"。
- 包里的规则作用于用到这个包的每一层，所以在包里，`at each level in # and below` 改成 `at every level that uses this package, #`，中文"本层 # 及以下每一层"改成"用到本包的每一层 #"。

| 关键字 | 情况 | 英文 | 中文 |
|---|---|---|---|
| SELECT-INSTANCE | 本层及以下 | `among the instances at each level in # and below, @SELECT-INSTANCE(model="nch") => they @SET-SYMBOL to $NMOS` | `从本层 # 及以下每一层的元件中 @选出元件(模型="nch") => 其 @使用符号 $NMOS` |
| | 只在本层 | `among the instances in # only, @SELECT-INSTANCE(type="pmos", recursive=false) => …` | `只从本层 # 的元件中 @选出元件(类型="pmos", 含下层=否) => …` |
| | 筛 | `among {#M1 #M2 #R1}, @SELECT-INSTANCE(name="M*") => they @ORIENT("MY")` | `从 {#M1 #M2 #R1} 中 @选出元件(名字="M*") => 其 @朝向("MY")` |
| | 网络上的 | `among the instances connected to &en, @SELECT-INSTANCE => they @SET-STYLE to $grey` | `从接在 &en 上的元件中 @选出元件 => 其 @使用样式 $"灰色"` |
| | 端子所属的 | `among the instances owning #M1:g, @SELECT-INSTANCE => they are on @LAYER("top")` | `从 #M1:g 所属的元件中 @选出元件 => 其处于 @图层("顶")` |
| | 用例的成员 | `among all members of %st1 at any depth, @SELECT-INSTANCE(role="pair")` | `从 %st1 的各级成员中 @选出元件(角色="输入对")` |
| | 单层的用例 | `among the members of %dp, @SELECT-INSTANCE` | `从 %dp 的成员中 @选出元件` |
| | 管道中间 | `among the usages at each level in # and below, @SELECT-USAGE of $DIFFPAIR => among all members of them at any depth, @SELECT-INSTANCE => they @SET-STYLE to $signal` | `从本层 # 及以下每一层的用例中 @选出用例 $"差分对" => 从其各级成员中 @选出元件 => 其 @使用样式 $"信号"` |
| SELECT-TERMINAL | 本层及以下 | `among the terminals at each level in # and below, @SELECT-TERMINAL(net="vss", wired=false) => $GND_STUB is @ATTACH(gap=3) to them` | `从本层 # 及以下每一层的端子中 @选出端子(网络="vss", 已接线=否) => $"接地桩" @附着(间距=3) 在其上` |
| | 元件的 | `among the terminals of #R1, @SELECT-TERMINAL => on them there is a @TEXT("${net}")` | `从 #R1 的端子中 @选出端子 => 其上有 @文本("${网络}")` |
| | 网络上的 | `among the terminals connected to $GROUND_NET_SET, @SELECT-TERMINAL(wired=false) => $GND_STUB is @ATTACH to them` | `从接在 $"地网络集" 上的端子中 @选出端子(已接线=否) => $"接地桩" @附着 在其上` |
| | 筛 | `among {#M1:s #M2:s}, @SELECT-TERMINAL(wired=false) => $GND is @ATTACH to them` | `从 {#M1:s #M2:s} 中 @选出端子(已接线=否) => $"地" @附着 在其上` |
| SELECT-NET | 本层及以下 | `among the nets at each level in # and below, @SELECT-NET(port=true) => $IOPORT is @ATTACH to them` | `从本层 # 及以下每一层的网络中 @选出网络(端口=是) => $"端口" @附着 在其上` |
| | 元件接的 | `among the nets connected to #M1, @SELECT-NET => named %m1_net_set` | `从与 #M1 相连的网络中 @选出网络 => 记作 %m1_net_set` |
| | 端子所在的 | `among the nets connected to #M1:g, @SELECT-NET => they are fully @ROUTE` | `从 #M1:g 所在的网络中 @选出网络 => 其全部 @布线` |
| | 导线所在的 | `among the nets of %w13, @SELECT-NET => they are fully @ROUTE` | `从 %w13 所在的网络中 @选出网络 => 其全部 @布线` |
| | 筛 | `among {&a &b}, @SELECT-NET(wired=false) => they are fully @ROUTE` | `从 {&a &b} 中 @选出网络(已接线=否) => 其全部 @布线` |
| SELECT-WIRE | 本层及以下 | `among the wires at each level in # and below, @SELECT-WIRE(bus=true) => they @SET-STYLE to $bus` | `从本层 # 及以下每一层的导线中 @选出导线(总线=是) => 其 @使用样式 $"总线"` |
| | 网络上的 | `among the wires on &clk, @SELECT-WIRE => they @SET-STYLE to $red` | `从 &clk 上的导线中 @选出导线 => 其 @使用样式 $red` |
| SELECT-LABEL | 本层及以下 | `among the labels at each level in # and below, @SELECT-LABEL(key="wl") => they @SET-STYLE to $grey` | `从本层 # 及以下每一层的标签中 @选出标签(引用名="wl") => 其 @使用样式 $"灰色"` |
| | 附着在对象上的 | `among the labels attached to #M1, @SELECT-LABEL => they @SET-STYLE to $grey` | `从附着在 #M1 上的标签中 @选出标签 => 其 @使用样式 $"灰色"` |
| SELECT-ANNOTATION | 本层及以下 | `among the annotations at each level in # and below, @SELECT-ANNOTATION => they @SET-STYLE to $base` | `从本层 # 及以下每一层的标注中 @选出标注 => 其 @使用样式 $"基础"` |
| | 筛 | `among {%n1 %f1 #M1}, @SELECT-ANNOTATION` | `从 {%n1 %f1 #M1} 中 @选出标注` |
| SELECT-USAGE | 本层及以下 | `among the usages at each level in # and below, @SELECT-USAGE of $DIFFPAIR => they have @PRIORITY("disabled")` | `从本层 # 及以下每一层的用例中 @选出用例 $"差分对" => 其 @优先级("停用")` |
| | 不写定义 | `among the usages at each level in # and below, @SELECT-USAGE => they @SET-STYLE to $base` | `从本层 # 及以下每一层的用例中 @选出用例 => 其 @使用样式 $"基础"` |
| | 用例自身和里面 | `among %st1 itself and the usages inside it, @SELECT-USAGE of $DIFFPAIR` | `从 %st1 自身及其内部的用例中 @选出用例 $"差分对"` |
| SELECT-ANCESTOR-USAGE | | `among the usages containing #M1, @SELECT-ANCESTOR-USAGE of $DIFFPAIR => named %dp_set` | `从包含 #M1 的用例中 @选出所处用例 $"差分对" => 记作 %dp_set` |
| | 端子 | `among the usages containing #M1:3, @SELECT-ANCESTOR-USAGE of $GND => they @ORIENT("R180")` | `从包含 #M1:3 的用例中 @选出所处用例 $"地" => 其 @朝向("R180")` |
| any selection | | `for %mixed_set, @SELECT-TERMINAL => …` | `对于 %mixed_set， @选出端子 => …` |

### B.8 集合与其他关键字

| 关键字 | 情况 | 英文 | 中文 |
|---|---|---|---|
| ADD-TO | | `&vdd @ADD-TO $SUPPLY_NET_SET` | `&vdd @加入 $"电源网络集"` |
| | 管道 | `… => they @ADD-TO $SUPPLY_NET_SET` | `… => 其 @加入 $"电源网络集"` |
| REMOVE-FROM | | `{&vdd &vss} @REMOVE-FROM $PORT_FILL_SET` | `{&vdd &vss} @移出 $"端口补位集"` |
| UNION | | `%u_set = %a_set and %b_set @UNION` | `%u_set = %a_set 和 %b_set 的 @并集` |
| BUNDLE | | `#XI<3:0> is drawn as a @BUNDLE => named %xi` | `#XI<3:0> @叠画 => 记作 %xi` |
| CURRENT | 流入 | `#M5:1 has @CURRENT("in")` | `#M5:1 有 @电流("流入")` |
| | 流出 | `#:iout has @CURRENT("out")` | `#:iout 有 @电流("流出")` |
| | 不定 | `#M5:2 has @CURRENT("unspecified")` | `#M5:2 的 @电流("不定")` |

### B.9 头段记录与包

不加词。头段记录按标准拼写读，不用代称（16.5）。

| 关键字 | 情况 | 英文 | 中文 |
|---|---|---|---|
| VERSION | | `@VERSION("0.2")` | `@VERSION("0.2")` |
| ALIAS | | `@ALIAS("@位于", "@PLACE")` | `@ALIAS("@位于", "@PLACE")` |
| USE-ALIASES | | `@USE-ALIASES(name="rnl-zh", version="0.2")` | `@USE-ALIASES(name="rnl-zh", version="0.2")` |
| DEVICE | | `@DEVICE(…)` | `@DEVICE(…)` |
| PACKAGE | | `@PACKAGE(name="acme-style", version="1.2")` | `@PACKAGE(name="acme-style", version="1.2")` |
| USE-PACKAGE | 头段记录里；包文件的头段记录里引用别的包也这样写 | `@USE-PACKAGE(name="rnl-std", version="0.2")` | `@USE-PACKAGE(name="rnl-std", version="0.2")` |
| | 写在层里 | `@USE-PACKAGE(name="acme-style", version="1.2")` | `@使用包(名字="acme-style", 版本="1.2")` |
