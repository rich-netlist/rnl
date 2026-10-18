# RNL 0.3 简明指南

RNL（Rich Netlist）写在 SPICE 或 Spectre 网表的注释里。它说明电路怎样画成原理图（符号、位置、对齐、导线、文字、颜色），以及各部分是什么（差分对、电流镜）。电路只由网表定义；RNL 从不改动网表。本指南讲的是语言 `"0.3"` 和标准包 `rnl-std` `"0.3"`；（14.8）这样的节号指向标准。

## 术语

| 术语 | 含义 |
|---|---|
| 记录 | 注释行里的一条 `@RNL(…) {…}`，里面是用 `;` 分开的语句 |
| 头段记录 | 记录 `@RNL(type="header") {…}`，文件的第一条记录 |
| 层 | 文件的顶层，或一个子电路；每一层各画成一张原理图 |
| 集合 | 有序的一组对象，写成 `{…}`，或由选择得到 |
| 定义 | DEFINE（用 AS 或 ATTACH 套用）、DEFINE-SYMBOL 或 DEFINE-STYLE 产生的东西；用 `$` 起名 |
| 角色 | 定义的具名槽位：体里写 `?a`，套用时写 `#M1(a)` |
| 用例 | 一次 AS 或 ATTACH 产生的对象：用的哪个定义，哪些对象填哪些角色 |
| 虚拟元件 | 画出来、但不在网表里的用例：地、电源、端口标签 |
| 规则 | 带选择（SELECT-…）的语句；在它所在的层和下面每一层都起作用 |
| 事实 | 位置、朝向或约束 |
| 求解器 | 决定没定下的位置、把结果写进 `type="generated"` 记录的工具 |
| 补位 | 标准包自动添上的地、电源和端口标签 |

## 文件的样子

```spice
* Two resistors
* @RNL(type="header") { @VERSION("0.3")}
R1 a b 1k
R2 b 0 1k
* @RNL {
*   {#R1 #R2} @ALIGN("x");
*   &b @WIRE {#R1:2 #R2:1}
* }
.end
```

```spectre
// Two resistors in Spectre
// @RNL(type="header") { @VERSION("0.3")}
simulator lang=spectre
R1 (a b) resistor r=1k
R2 (b 0) resistor r=1k
// @RNL {
//   {#R1 #R2} @ALIGN("x");
//   &b @WIRE {#R1:2 #R2:1}
// }
```

- 记录写在整行注释里：第 1 列的 `*` 加一个空格（Spectre 里也可以是 `//`）。记录的每一行都得是这样的行；空一行就写一个单独的 `*`。行尾注释从来不读（2.2）。
- 第 1 行是标题，从来不读。头段记录排在最前，写在文件层，最早在第 2 行。里面写 `@VERSION("0.3")`，需要时再写 DEVICE、ALIAS、USE-ALIASES 和 USE-PACKAGE（2.8）。文件按它声明的版本读（16.1）。
- 写在 `.subckt … .ends` 之间的记录属于那个子电路；写在外面的属于顶层。引用只在自己的层里找。`.end` 后面的文字不读。
- 记录头字段都写名字：`type`（默认 `"authored"`，还有 `"generated"`、`"header"`），`scope`（从顶层写进的子电路），`priority`，`author`。

## 名字和引用

| 写法 | 含义 |
|---|---|
| `#M1`、`#` | 本层的元件；单独的 `#` 是本层 |
| `&vdd`、`&"vdd!"` | 网络；名字里有别的字符时加引号 |
| `#M1:d`、`#M1:1` | 端子，按名字或从 1 起的序号：是同一个端子 |
| `#:inp` | 本子电路的端口 |
| `%dp1` | 本层起的名字 |
| `$GROUND` | 定义、符号、样式或共享的集合 |
| `?a` | 定义体里的角色或参数 |
| `%dp1::in_p`、`#M1::position` | 对象的一部分：角色、局部对象、原点、按引用名取的文字 |
| `(x, y)`、`(0, _)` | 点；`_` 让这一轴空着 |
| `{a b c}` | 集合，有次序 |
| `&D<3:0>` | 下标：D<3>、D<2>、D<1>、D<0> |

- 默认的端子名（6.12）：M 是 `d g s b`；Q 是 `c b e s`；J、Z 是 `d g s`；D 是 `a k`；R、C、L、V、I、B、F、H、W 是 `p n`；E、G、S 是 `p n cp cn`；T、O 是 `p1 n1 p2 n2`；U 是 `a b ref`。X 元件的端子是它的子电路的端口。网表里没有定义的子电路（PDK 器件）没有端子名：在头段记录里用 DEVICE 起名，或者写序号。
- 网表里的名字照宿主的大小写规则；RNL 自己的名字和关键字分大小写。
- 数字不带单位（`2u` 是错误）；枚举值是字符串，如 `"left-to-right"`；真假是 `true`、`false`；数组写成 `[…]`。没有表达式。
- `%#3` 是没有名字的对象的编号；只出现在求解器的记录里（5.9）。

## 语句

- 一条语句是：对象，至多一个关键字和它的参数，以及它用的定义：`#M1 @SET-SYMBOL $NMOS`。ATTACH 先写定义：`$GROUND @ATTACH &vss`。
- 按位置的参数在前，写名字的在后：`@PLACE(10, 20, priority="high")`。参数的括号紧贴关键字：`@PLACE (10, 20)` 是 PLACE 加一个点。角色的括号紧贴对象：`#M1(in_p)`。
- `=>` 把一段的结果交给下一段：`$GROUND @ATTACH &vss => @PLACE(20, -4)`。
- `%x = statement` 给结果起名；管道里 `… => %x` 给它左边那一段起名。一个名字在一层里只绑定一次，可以写在它的语句前面使用。
- 收集合的槽位把零散的对象当一个集合：`#M1 #M2 @ALIGN("y")`。收单个对象的槽位收到集合时，对每个成员各做一次：`{#D1 #D2} @ORIENT("MX")`。两个槽位要正好两个对象：`{#M1 #M2} @SET-SYMBOL $NMOS`。
- `{…}` 里的集合展开一层；`{{%a_set} #M3}` 让 `%a_set` 作为一个成员，按中心算。
- 记号之间的自由文本不表示任何意思，所以语句可以读起来像句子：`#M1 is at @PLACE(10, 20)`。没带前缀的名字也是自由文本：`#M1 #M2 M3 @ALIGN("y")` 只对齐 M1 和 M2。说明写在 `/* */` 里。
- 事实可以叠加；语句的先后只在冲突或互相覆盖时才有关系。

## 定义、角色和集合

```spice
* An input pair with its tail and a mirror load
* @RNL(type="header") { @VERSION("0.3")}
.model nch nmos level=1
.model pch pmos level=1
M1 x inp tail 0 nch
M2 out inn tail 0 nch
M3 x x vdd vdd pch
M4 out x vdd vdd pch
M5 tail vb 0 0 nch
* @RNL {
*   %dp1 = #M1(in_p) #M2(in_n) #M5(tail) @AS $DIFFERENTIAL_PAIR;
*   %cm1 = #M3(ref) #M4(out) @AS $CURRENT_MIRROR;
*   {#M1 #M2 #M5} @AS $MATCHED;
*   $SIDE = @DEFINE(roles=["a", "b"], parameters=[["gap", 4]]) {
*     {?a ?b} @ALIGN("y");
*     ?a ?b @DISTANCE("left-to-right", min=?gap)
*   };
*   #M3(a) #M4(b) @AS(gap=6) $SIDE
* }
.end
```

- **角色**：每一项是一个名字，或 `[name, kind]`（`"instance"`、`"terminal"`、`"net"`、`"point"`、`"usage"`）。`"{name}"` 是集合角色：收一个集合，可以不填。**参数**是名字或 `[name, default]`，写在 AS 或 ATTACH 的括号里（9.4）。
- **套用**：对象按角色表的次序写，或全都标上角色。集合角色收一个集合：`{#M1 #M2 #M5} @AS $MATCHED`。单个角色收到集合时，对每个成员各套用一次。
- **体**：`?name` 指角色和参数；体里起了名的对象，如 `%mid = (_, _)`，是每个用例的局部对象，从外面写 `%u::mid`。体里不能写网表对象。
- **用例是一个对象**：不管它体里的事实怎样，它都记着这些对象是什么（9.5）。
- **选择**给出集合：`# @SELECT-INSTANCE(type="pmos")`、`&en @SELECT-TERMINAL(terminal="g")`、`# @SELECT-NET(port=true)`、`# @SELECT-USAGE $DIFFERENTIAL_PAIR`、`#M1 @SELECT-ANCESTOR-USAGE $CURRENT_MIRROR`。条件都写名字；`name`、`model`、`net`、`key` 可以带 `*`。`model` 是 `.model` 名，X 元件则是子电路名；`type` 是 `.model` 的类型；X 元件没有类型（10.1）。

## 布局的事实

| 关键字 | 写法 | 含义 |
|---|---|---|
| PLACE | `#M1 @PLACE(10, 20)`、`@PLACE(x=10)` | 原点放在 (x, y)；x 向右，y 向上，网格单位 |
| ORIENT | `#M2 @ORIENT("MY")` | `"R0"`、`"R90"`、`"R180"`、`"R270"`、`"MY"`、`"MX"`、`"MYR90"`、`"MXR90"` |
| ORIENT-RELATION | `#M1 #M2 @ORIENT-RELATION("MY")` | 第二个是第一个再做这个变换 |
| ALIGN | `{#M1 #M2} @ALIGN("y")` | 一行（`"y"`）或一列（`"x"`） |
| ORDER | `{#M1 #M2} @ORDER("left-to-right")` | 沿一个方向按次序排；`"x"`、`"y"` 只定轴 |
| DISTRIBUTE | `{#R1 #R2 #R3} @DISTRIBUTE("x")` | 等距 |
| DISTANCE | `#M3 #M1 @DISTANCE("top-to-bottom", min=4)` | 从第一个到第二个：`min`、`max` 或 `exact` |
| GROUP | `{#M1 #M2 #M5} @GROUP` | 一块；也是评判对齐、间距和对称的范围（11.5） |

- 成员可以是元件（按原点）、端子（按引脚）、点、文字和画出的用例；作为一个成员的集合按中心算。网络和导线不能做成员。
- 没有"最左边"：相对别的元件写，如 `{#M6 #M7} %pout @DISTANCE("left-to-right", min=2)`。
- RNL 没定下的，由求解器决定；RNL 只说必须成立的。

## 优先级

```spice
* Priorities
* @RNL(type="header") { @VERSION("0.3")}
.model nch nmos level=1
M1 x inp tail 0 nch
M2 out inn tail 0 nch
M5 tail vb 0 0 nch
* @RNL(priority="high") {
*   %dp1 = #M1(in_p) #M2(in_n) #M5(tail) @AS $DIFFERENTIAL_PAIR;
*   {#M1 #M2} @ALIGN("y");
*   #M5 @PLACE(x=3, priority="low")
* }
* @RNL { %dp1 @PRIORITY("disabled")}
.end
```

- PLACE、ORIENT、ORIENT-RELATION、四种约束、AS 和 ATTACH 收 `priority=`：`"disabled"`、`"low"`、`"normal"`（默认）、`"high"`。冲突时先丢低的，一样低时丢先写的；留下的事实不论优先级都是硬的（14.8）。
- 用例的优先级管它体里的全部事实：`@AS(priority="high")` 把它们都抬高；PRIORITY 事后改它，如 `%dp1 @PRIORITY("disabled")` 关掉这个差分对的默认画法。
- 记录头的 `priority` 给没写优先级的语句作默认值，只是 AS、ATTACH 从记录头只取 `"disabled"`：上例里差分对体里的 low 约束照旧是 low。
- 标准包把默认画法写成 `"low"`，所以你写的总会胜出。

## 导线和 ROUTE

```spice
* Wires
* @RNL(type="header") { @VERSION("0.3")}
R1 a b 1k
R2 b 0 1k
R3 b c 1k
* @RNL {
*   %j = (0, 0);
*   &b @WIRE {#R1:2 %j #R2:1};
*   &b @WIRE {%j #R3:1};
*   $JUNCTION @ATTACH %j;
*   &a @ROUTE
* }
.end
```

- WIRE 在网络上画一条导线，经过至少两个点或端子；`mode=` 是 `"auto"`（默认）、`"straight"`、`"horizontal-vertical"`、`"vertical-horizontal"`、`"manhattan"` 或 `"manhattan45"`（13.1）。
- 导线只在共用同一个点对象或端子时才连上；坐标相同不算。要画 T 形，直通的导线经过一个起了名的点，分支从这个点出发；连接点写成 `$JUNCTION @ATTACH %j`。
- ROUTE 把整个网络画成导线，一棵经过它上面每个引脚和虚拟元件的树（13.7）。
- 网络槽位里放几个网络就是总线：`&D<1:0> @WIRE {%a %b}`。
- 不写 WIRE 或 ROUTE，连接就不画成导线。

## 文字和标签

```spice
* Labels
* @RNL(type="header") { @VERSION("0.3")}
.model nch nmos level=1
M1 x inp 0 0 nch W=2u L=0.5u
* @RNL {
*   #M1 @TEXT("${w}/${l}", key="wl", dy=-3);
*   #M1 @TEXT("Min", key="name");
*   (10, 10) @TEXT("bias", size=0.8)
* }
.end
```

- 附着在对象上的 TEXT 是标签，跟着对象走；附着在点上的是标注。`key=` 给它起名，`#M1::wl`；引用名相同的文字替换先写的，如 `key="name"` 替换符号显示的名字（13.3）。
- `${…}` 显示属性：元件的 `name`、`value`、`model`、`type` 和网表参数（如 `${w}`）；端子或导线的 `net`；用例的角色；路径如 `${pair::in_p::w}`（13.6）。
- 标记：`_{…}`、`^{…}`、`\overline{…}`、`\frac{…}{…}`、`\mu`、`\Omega`。还有 `dx`、`dy`、`halign`、`valign`、`rotate` 和样式参数。

## 符号和样式

```spice
* A symbol of its own
* @RNL(type="header") { @VERSION("0.3")}
X1 in out buf
.subckt buf in out
R1 in out 1k
.ends
* @RNL {
*   $TRI = @DEFINE-SYMBOL {
*     (-4, 0) @PIN(terminal="in", direction="left");
*     (4, 0) @PIN(terminal="out", direction="right");
*     @LINE {(-2, 2) (2, 0) (-2, -2) (-2, 2)};
*     @LINE {(-4, 0) (-2, 0)};
*     @LINE {(2, 0) (4, 0)};
*     ? @TEXT("${name}", key="name") => @PLACE(-2, 2.5)
*   };
*   #X1 @SET-SYMBOL $TRI;
*   $orange = @DEFINE-STYLE(linecolor="#e07000", textcolor="#e07000") {};
*   #X1 @SET-STYLE $orange
* }
.end
```

- 符号体用以对象原点为原点的局部坐标。`@PIN(terminal=…, direction=…)` 放一个端子的引脚；没有引脚的端子不画，连到它的导线失效（9.2.4）。`? @TEXT(…)` 是用这个符号画出的对象的标签。
- 直接给对象的符号胜过规则给的；同一类里后写的胜，标准包的规则排在最前。规则一次给很多对象符号：`# @SELECT-INSTANCE(model="nch") => @SET-SYMBOL $NMOS`。
- 样式按属性合并：`linecolor`、`linestyle`、`linewidth`、`fillcolor`、`textcolor`、`size`、`font`、`bold`、`italic` 等（9.3）。颜色写成 `"#RRGGBB"`。虚拟元件是独立的对象；用规则给它们样式，如 `# @SELECT-USAGE $GROUND => @SET-STYLE $grey`。
- 子电路里写 `# @SET-SYMBOL %blk` 给它块符号，上层的元件就用它画（9.2.6）。

## 标准包

### 它带来什么

文件不用写就使用 `rnl-std` `"0.3"`：样式，`rnl-std-basic` 和 `rnl-std-semiconductors` 的符号，按方言选的 `rnl-std-spice` 或 `rnl-std-spectre` 的符号规则，补位（`rnl-std-fill`）和结构（`rnl-std-structures`）。`$OPAMP` 等功能块在 `rnl-std-functional` 里，要在头段记录里点名：`@USE-PACKAGE(name="rnl-std-functional", version="0.3")`；`rnl-std-full` 带上全部包。写一个空包就去掉一个：`@USE-PACKAGE(name="rnl-std-fill") {}`（16.3）。

### 默认的符号和引脚

R `$RESISTOR`，C `$CAPACITOR`，L `$INDUCTOR`，V `$VOLTAGE_SOURCE`，I `$CURRENT_SOURCE`，D `$DIODE`，E `$VCVS_4TERM`，G `$VCCS_4TERM`，F `$CCCS`，H `$CCVS`，B `$BEHAVIORAL_SOURCE`，S `$VOLTAGE_CONTROLLED_SWITCH`，W `$SWITCH`，J 和 Z `$NJFET`，T 和 O `$TRANSMISSION_LINE`，U `$RC_LINE`。晶体管按 `.model` 的类型：nmos `$NMOS_4TERM`，pmos `$PMOS_4TERM`，npn `$NPN`，pnp `$PNP`，njf `$NJFET`，pjf `$PJFET`；写上 `.model` 卡，或者自己给符号，极性才是确定的。

| 符号 | R0 时的引脚 |
|---|---|
| 两端元件和各种源 | 1 在 `(0, 2)`，2 在 `(0, −2)` |
| `$DIODE` | a `(−2, 0)`，k `(2, 0)` |
| `$NMOS`、`$NMOS_4TERM` | d `(0, 2)`，g `(−3, 0)`，s `(0, −2)`，b `(1, 0)` |
| `$PMOS`、`$PMOS_4TERM` | s `(0, 2)`，g `(−3, 0)`，d `(0, −2)`，b `(1, 0)` |
| `$NPN` / `$PNP` | b `(−3, 0)`；NPN 的 c 在上，PNP 的 e 在上 |
| `$VCVS_4TERM`、`$VCCS_4TERM` | 1 `(0, 2)`，2 `(0, −2)`，3 `(−4, 1)`，4 `(−4, −1)` |
| `$GROUND_SYMBOL`、`$SUPPLY_SYMBOL`、`$IO_PORT_SYMBOL` | 一个引脚，在原点 |

### 定义

| 名字 | 角色 | 用途 |
|---|---|---|
| `$GROUND`、`$SUPPLY`、`$IO_PORT` | p（网络或端子） | `$GROUND @ATTACH &vss`；p 是端子时画在引脚上 |
| `$NO_CONNECT`、`$JUNCTION`、`$CURRENT_ARROW` | t；p；p | 不连接的标记、连接点、电流箭头 |
| `$GROUND_STUB`、`$SUPPLY_STUB` | t；gap（2） | 端子下面的地、上面的电源，带它的导线，低优先级 |
| `$DIFFERENTIAL_PAIR` | in_p、in_n、{tail} | 体里把尾管放在中间下方，低优先级 |
| `$CURRENT_MIRROR` | {ref}、{out} | 不论几个输出管都是一个用例 |
| `$MATCHED`、`$BRANCH` | {members} | 匹配的器件；一条支路，排成一列，低优先级 |
| `$CHAIN`、`$CASCADE` | source、{chain}、load；{first}、{second}、{feedback}、{load} | 信号链，从左到右，低优先级；级联 |
| `$MIRRORED` | a、b、axis | a 和 b 一样高，关于轴点互为镜像，a 在左，低优先级 |
| `$FLOW` | {path}；direction | 路径按这个方向排，低优先级 |

### 补位

`$GROUND_NET_SET`（网络 0）的网络上每个没接导线的端子都得到一个 `$GROUND_STUB`，`$SUPPLY_NET_SET`（空的）上的得到 `$SUPPLY_STUB`，子电路的每个端口（地和电源网络除外）得到一个端口标签（10.5）。自己的网络要加进去：`&vss @ADD-TO $GROUND_NET_SET`、`&vdd @ADD-TO $SUPPLY_NET_SET`；不要标签的网络这样去掉：`{&vb} @REMOVE-FROM $PORT_FILL_SET`。需要时再用：`$LABEL_TERMINAL_SET @TEXT("${net}", key="netname")` 给没接导线的端子标网络名，`$WIRE_NET_SET @ROUTE` 把其余网络画成导线。

## 代称和 rnl-zh

```spice
* The same pair in Chinese
* @RNL(type="header") { @VERSION("0.3"); @USE-ALIASES(name="rnl-zh")}
.model nch nmos level=1
M1 x inp tail 0 nch
M2 out inn tail 0 nch
* @RNL {
*   #M1(正输入) #M2(负输入) @视作 $"差分对";
*   {#M1 #M2} @对齐("y")
* }
.end
```

- 头段记录里的 USE-ALIASES 加载一个代称包；`rnl-zh` 给关键字和标准包里的名字提供中文写法（16.5）。标准写法照样可以用。ALIAS 声明一个代称：`@ALIAS("@并排", "@ALIGN")`。中文的 `$` 名字要加引号，如 `$"差分对"`。

## 求解器的记录

```spice
* Solved, then dragged
* @RNL(type="header") { @VERSION("0.3")}
R1 a b 1k
R2 b 0 1k
* @RNL { {#R1 #R2} @ALIGN("x")}
* @RNL(type="generated", priority="high", author="rnl-solver") { #R1 @PLACE(0, 4); #R2 @PLACE(0, -4)}
* @RNL(author="editor") { #R2 @PLACE(y=-6, priority="high")}
.end
```

- 求解器把它决定的写进这一层末尾的一条 `type="generated"` 记录。这一层再求解时，这条记录不读，并被整个换掉：不要改它（2.8、14.8）。
- 要定住一个位置，在同一层后面的记录里写一条 `"high"` 的 PLACE，如上；两条 high 的事实里，先写的被丢掉。
- 没有名字的对象在那里写成编号，`%#3`，由字段 `numbering` 校验。你自己的记录里不要写编号；给对象起名。

## 常见错误

| 写的 | 报的 | 应写 |
|---|---|---|
| `#M1 @PLACE (10, 20)` | `PLACE takes x, y or both` | `#M1 @PLACE(10, 20)` |
| `#M1 (in_p) #M2 (in_n) @AS …` | `role parentheses touch the object` | `#M1(in_p) #M2(in_n) @AS …` |
| `#M1 #M2 @SET-SYMBOL $NMOS` | `@SET-SYMBOL has 2 slots, and 3 objects are written` | `{#M1 #M2} @SET-SYMBOL $NMOS` |
| `#M5 #M3 @AS $MATCHED` | `$MATCHED has 1 roles, 2 objects are written` | `{#M5 #M3} @AS $MATCHED` |
| `#M1 @PLACE(1, 2), #M2 @PLACE(3, 4)` | `at most one keyword in a segment` | 语句之间用 `;` |
| `#M1 @SET-SYMBOL($NMOS)` | `parameters hold literals only` | `#M1 @SET-SYMBOL $NMOS` |
| `#R1 @PLACE(x=2u)` | `a number takes no unit or suffix` | `@PLACE(x=2)` |
| `$GND @ATTACH &vss` | `$GND cannot be found: the standard series 0.3 calls it $GROUND` | `$GROUND @ATTACH &vss` |
| `#R1 @SET-SYMBOL $GROUND` | `must be a symbol, not $GROUND` | 符号是 `$GROUND_SYMBOL` |
| 没点名包就用 `$OPAMP` | `$OPAMP cannot be found: … rnl-std-functional` | 用 USE-PACKAGE 点名 `rnl-std-functional` |
| 自己的记录里写 `%#3 @PLACE(1, 2)` | `a number is written only in the record of the solver` | 给对象起名 |
| 往 `$NMOS` 的体端接导线 | `#M1:b has no pin in its symbol: the wire fails` | `$NMOS_4TERM` |
| `#M1.d`、`M3`、`&vdd!` | 不报错：它们是自由文本 | `#M1:d`、`#M3`、`&"vdd!"` |

```spice error
* A space before the bracket
* @RNL(type="header") { @VERSION("0.3")}
R1 a 0 1k
* @RNL { #R1 @PLACE (10, 20)}
.end
```

```spice error
* A name of the series 0.2
* @RNL(type="header") { @VERSION("0.3")}
R1 a vss 1k
* @RNL { $GND @ATTACH &vss}
.end
```

## 交还文件之前

- 头段记录是第一条记录，在第 2 行或更后，声明 `@VERSION("0.3")`。
- 每条记录的每一行都是整行注释，注释符在第 1 列。
- 每个 `#`、`&` 和端子名都在它那一层的网表里；端子名和元件对得上（`#M1:d`、`#Q1:c`）。
- 只用标准系列 `"0.3"` 的名字：`$GROUND`、`$DIFFERENTIAL_PAIR`、`$RESISTOR`、`$MIRRORED`。
- 括号紧贴关键字或对象；语句用 `;` 分开；数字不带单位。
- 每个集合角色都收到一个集合；两个槽位的关键字正好两个对象。
- 没有改动 `type="generated"` 的记录，别处没有写 `%#` 编号。
- 当作地和电源的网络加进了 `$GROUND_NET_SET` 和 `$SUPPLY_NET_SET`。
- 晶体管有 `.model` 卡，或者明写了符号。

## 完整的例子

```spice
* Five-transistor OTA
* @RNL(type="header") { @VERSION("0.3")}
.model nch nmos level=1
.model pch pmos level=1
.subckt ota5t inp inn out vdd vss vbias
M1 x    inp   tail vss nch W=2u L=0.5u
M2 out  inn   tail vss nch W=2u L=0.5u
M3 x    x     vdd  vdd pch W=4u L=0.5u
M4 out  x     vdd  vdd pch W=4u L=0.5u
M5 tail vbias vss  vss nch W=4u L=1u
* @RNL {
*   &vss @ADD-TO $GROUND_NET_SET;
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   %dp1 = #M1(in_p) #M2(in_n) #M5(tail) @AS $DIFFERENTIAL_PAIR;
*   %cm1 = #M3(ref) #M4(out) @AS $CURRENT_MIRROR;
*   %ax = (_, _);
*   #M1(a) #M2(b) %ax(axis) @AS $MIRRORED;
*   #M3(a) #M4(b) %ax(axis) @AS $MIRRORED;
*   #M3 #M1 @DISTANCE("top-to-bottom", min=4);
*   #M1 #M5 @DISTANCE("top-to-bottom", min=4);
*   {#M1 #M2 #M3 #M4 #M5} @GROUP;
*   {#M1 #M2} @TEXT("${w}/${l}", key="wl", dy=-3);
*   #M5:d @CURRENT("in");
*   #M5:s @CURRENT("out")
* }
.ends ota5t
.end
```

- 两个结构说明电路是什么；它们的体以低优先级把尾管放在中间下方，把每对镜像管关于轴放好。两条 DISTANCE 把负载放在差分对上面、尾管放在它下面。
- CURRENT 说明端子上静态电流的方向；它什么也不画（15.2）。
- 补位加上地、电源和端口标签；没定下的留给求解器决定。
