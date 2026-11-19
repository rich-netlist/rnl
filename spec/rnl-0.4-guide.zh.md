# RNL 0.4 简明指南

RNL 写在 SPICE 或 Spectre 网表的注释中，用来描述原理图的画法以及电路各部分的含义，绝不会改动电路本身。本文对应语言版本 `"0.4"` 与标准包系列 `"0.4"`，文中形如（14.8）的编号指向语言标准的相应小节。各节代码均为彼此独立的写法示例，并非同一份完整程序；实际编写时，应在对象所属层级中引用已有对象，并将代码片段写入 RNL 记录（示例中的 `/* */` 注释仅作说明）。文末目录以签名形式汇集了标准包的全部名称，仅供查阅用法，不可直接当作 RNL 代码照搬。

## 文件结构

```spice
* Two resistors                                   /* line 1 is the title, never read */
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4");}
R1 a b 1k
R2 b 0 1k
* @RNL {
*   {#R1 #R2} @ALIGN("x"); /* every statement ends with ; */
*   &b @WIRE {#R1:n #R2:p};
* }
.end
```

- RNL 记录写在整行注释中，每行第 1 列为 `*` 后接一个空格；在 Spectre 模式（`.scs` 文件，或 `simulator lang=spectre` 之后）下也可使用 `//`。头段记录须置于最前；写在 `.subckt … .ends` 内部的记录归属该子电路。
- 若未通过 USE-PACKAGE 引入标准包，所有标准名称均不可用。已有文件中的头段记录应原样保留，其中 `@DEVICE` 语句用于声明 PDK 器件的端子名称与器件类型（6.12），切勿自行编写或改动。
- 每个 `@` 前面必须留一个 ASCII 空格；参数圆括号紧贴关键字，角色圆括号紧贴对象，中间均不留空格。位置参数须排在具名参数之前，如 `@PLACE(10, y=20)`。数值不带单位，语言也不支持表达式；RNL 自身的名称与关键字区分大小写，而网表对象的名称则遵循宿主语言的大小写规则。

## 引用

```rnl
#M1; /* an instance of this level; # alone is the level */
&vdd; &"vdd!"; /* nets; quote other characters */
#M1:d; #M1:1; /* a terminal, by name or number: the same one */
#M1::d; /* the pin of M1's symbol that draws terminal d */
#:inp; /* a port of this subcircuit */
%dp1; $GROUND; /* a name of this level; a definition, symbol, style or shared set */
%g::pin; %dp1::left; /* a part: the pin of a ground, the left role of a pair */
(10, 20); (0, _); /* points; _ leaves an axis free */
{#M1 #M2 #M3}; /* a set, in order */
&D<3:0>; /* D<3> D<2> D<1> D<0> */
%row_set{1}; %row_set{end}; %row_set{2:end}; /* members of a written set by place */
```

- 各类元件的标准端子名称如下：M 为 `d g s b`，Q 为 `c b e s`，J、Z 为 `d g s`，D 为 `a k`，R、C、L、V、I、B、F、H、W 为 `p n`，E、G、S 为 `p n cp cn`，T、O 为 `p1 n1 p2 n2`，U 为 `a b ref`，X 则使用子电路各端口的名称。凡是需要传入引脚的位置，均可直接写端子来代表其对应的引脚。
- 同一层级内每个名称只能绑定一次，且允许在定义之前先行引用；若要移动已命名的 `%p`，应写 `%p @PLACE(...)`，而不可重复绑定 `%p = ...`。`%` 名称仅在当前层级局部有效，`$` 定义则会向下层继承，并可被下层的同名定义遮蔽。
- 不带前缀的词一律视作自由文本（若恰好与已有对象重名会触发警告），因此 `#M1 #M2 M3 @ALIGN("y");` 实际只对齐 M1 和 M2。

## 语句与管道

```rnl
#M1 @PLACE(10, 20); /* objects, one keyword, its parameters */
$GROUND @ATTACH &vss => @PLACE(20, -4); /* => passes the result on: the ground, not the net */
%g = $GROUND @ATTACH &vss; /* %x = names the result */
$GROUND @ATTACH &vdd => %g2 => @PLACE(20, 4); /* => %x names the segment before it */
#M1 #M2 @ALIGN("y"); /* loose objects are one set */
{#D1 #D2} @ORIENT("MX"); /* a one-object slot given a set does each member */
{#M1 #M2} @SET-SYMBOL $NMOS; /* two slots: exactly two objects */
{#R1 {#R2 #C2} #M1} @ORDER("left-to-right"); /* a set as one item stands for each member: R2 and C2 each between */
```

管道各段向后传递的结果依关键字而定：PLACE、ORIENT、SET-SYMBOL、SET-STYLE、GROUP 与 CURRENT 原样传递其输入对象；AS 与 ATTACH 传递新建的用例（逐个套用时则为用例构成的集合）；ALIGN、ORDER、DISTRIBUTE、DISTANCE、TABLE 与 ORIENT-RELATION 传递新建的约束对象；TEXT 传递新建的文字对象；各类 SELECT-… 则传递筛选所得的集合。

## 定义、角色与用例

```rnl
$SIDE = @DEFINE(roles=["a", "b"], parameters=[["gap", 4]]) {
  {?a ?b} @ALIGN("y"); /* ?a: a role in the body */
  ?a ?b @DISTANCE("left-to-right", min=?gap); /* ?gap: a parameter */
};
#M3(a) #M4(b) @AS(gap=6) $SIDE; /* apply with roles marked, and a parameter */
#M3 #M4 @AS $SIDE; /* mark every role or none; without marks, use role order */
%dp1 = #M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N; /* every one-object role is filled: tail too */
{#M5 #M3} @AS $MATCHED; /* a set role takes the whole set: one usage */
{#M1 #M2} @AS $DIODE_CONNECTED; /* a one-object role given a set: one usage each */
$KIND = @DEFINE(roles=[["p", "net"], ["{at}", "point"]]) {}; /* common role kinds: instance terminal pin net point usage */
{%dp1::left #M9} @ALIGN("y"); /* place a structure through a member */
%g = $GROUND @ATTACH &vss; {%g::pin #M5::s} @ALIGN("x"); /* a virtual part is drawn at its own point; place it by its pin */
```

- 声明为 `{role}` 的集合角色整体接收一个集合（传入单个对象时视作单成员集合），套用时可以省略；但若采用不标角色名的按序传参写法，则只能省略排在末尾的集合角色。单对象角色不可省略。每次套用中同一角色最多标记一次，如需传入多个成员，应合并写成一个集合，例如 `{#R1 #R2}(members)`。
- 在定义体内部，单独书写的 `?` 代表当前用例自身的点，字面坐标均为相对于该点的偏移量；用 `%mid = (_, _);` 创建的对象属于该用例的局部对象，在外部可通过 `%u::mid` 访问。
- 负责放置的标准结构会绑定用例自身的点：成员排成一行或一列时该点落在其轴线上，左右或上下镜像的两侧则落在两者中心，因此对 `%u::position` 施加约束即可移动整个结构。文末目录的“绘制”一栏标明了绑定位置：`$BRANCH` 仅与成员共用 x 坐标，`$BRANCH_HORIZONTAL` 仅共用 y 坐标，`$PARALLEL_HORIZONTAL` 和 `$MIRRORED_LAYOUT` 则取在成员的中心。其余结构均让该点保持自由，对其 `::position` 施加约束不会改变任何绘制结果。
- 建模时应优先选用既能标明电路语义、又能一并安排布局的结构。例如 `$BRANCH` 表示纵向叠放的串联结构，成员按从上到下的顺序书写，常用于电源到地之间的直流支路；每条此类通路可写为一个 `$BRANCH`，支路上若有并联器件对，则应搭配带放置效果的并联变体（单写 `$PARALLEL` 只负责布线而不控制位置），如 `{#R1 #M1 #R2} @AS $BRANCH; {#R2 #C2} @AS $PARALLEL_HORIZONTAL;`。支路结构本身并不声明电流方向，电流仍需通过 `@CURRENT` 另行标注。像开关电容采样器那样经由开关与电容传输的信号通路，适合直接用原子事实描绘，例如对同一行上的各引脚分别施加 ALIGN 与 ORDER：`{#S1:s #S1:d #C1:p #C1:n} @ALIGN("y"); {#S1:s #S1:d #C1:p #C1:n} @ORDER("left-to-right");`。至于目录中标为“仅表语义”的结构，它们不产生任何绘制效果——在电路关系确实成立时可以写上以补充语义，但切勿用它们替代负责放置的结构。仅在确有把握的清晰信号路径上标注 `$SIGNAL_FLOW`；若信号在电路大部分区域存在多条支路、构成反馈或双向流动，则无论路径成员多少均不要标注。

## 筛选与规则

```rnl
# @SELECT-INSTANCE(model="nch", recursive=true) => @SET-STYLE $grey; /* a rule: here and in the levels below */
# @SELECT-INSTANCE(type="pmos") => @ORIENT("MY"); /* without recursive=true: this level only */
&en @SELECT-TERMINAL(terminal="g"); /* the gate terminals on net en */
# @SELECT-PIN(wired=false); /* pins without wires */
# @SELECT-NET(port=true); # @SELECT-NET(global=true); # @SELECT-NET(wired=false);
# @SELECT-USAGE $DIFFERENTIAL_PAIR_N => @SELECT-INSTANCE(role="left"); /* the left device of every pair */
&vss @ADD-TO $GROUND_NET_SET; /* edit a shared set of the package */
&agnd @REMOVE-FROM $GROUND_NET_SET;
```

- 条件 `name`、`model`、`net` 与 `key` 均支持通配符 `*`；`type` 匹配 `.model` 模型卡或 DEVICE 语句所声明的器件类型。
- 筛选结果以及基于筛选结果构造的集合，既不支持按位置下标取成员（如 `%sel{1}`），也不能传给 WIRE；如需连线，应直接为对象命名，或改用 ROUTE-OBJECT。

## 布局事实

```rnl
#M1 @PLACE(10, 20); #M1 @PLACE(x=10); /* origin at (x, y); x right, y up, grid units */
#M2 @ORIENT("MY"); #M5 @ORIENT(["R0", "MY"]); /* an object takes a code; a list lets the solver choose */
#M1 #M2 @ORIENT-RELATION("MY"); /* M2 is M1 flipped left-right */
{#M1 #M2} @ALIGN("y"); /* one row; "x" is one column */
{#M1 #M2 #M3} @ORDER("left-to-right"); /* one after the other, at least 1 apart */
{#R1 #R2 #R3} @DISTRIBUTE("top-to-bottom"); /* in order, equal steps */
#M3 #M1 @DISTANCE("top-to-bottom", min=4); /* from the first to the second: min, max or exact, none negative */
{#M1 #M2 #M5} @GROUP; /* groups the set for the solver; no own origin or rigid transform */
@TABLE layout: {{#M1 #M2 #M3} {#M4 {#M5 #M6} #M7}}; /* rows top down, cells left to right, a cell column inside; one group; no alignment */
@TABLE(transpose=true) layout: {{#M1 #M2} {#M3 #M4}}; /* columns left to right */
```

- 方向参数取值为 `"left-to-right"`、`"right-to-left"`、`"bottom-to-top"` 或 `"top-to-bottom"`。朝向码中，`R0`、`R90`、`R180`、`R270` 表示逆时针旋转；`MY` 为左右翻转，`MX` 为上下翻转；`MYR90`、`MXR90` 则为先镜像翻转再逆时针旋转。
- 参与布局约束的各成员分别按各自的基准点计算位置：元件取符号原点，端子与引脚取引脚位置，点和文字取自身位置，带符号的用例取自身的点。没有符号的用例同 GROUP 一样向下展开并代表其最底层的成员；它自身的点是 `%u::position`。集合本身没有原点，即使加上了 GROUP，对其使用 PLACE 仍会分别放置集合中的每个成员；引脚与约束对象则不能用 PLACE 直接定位。语言中也没有“最左侧”这类针对整张图的绝对方位，必须通过与其他部件的相对位置关系来表达。
- 凡未加固定的自由度均由求解器决定，而求解器不会自行揣测器件在电路中的作用，因此必须成立的关系都应明确写出。通常应优先描述电路语义与布局约束，将具体坐标交给求解器计算；若任务明确要求给定坐标，则应改为对全部对象写出 PLACE、ORIENT 与 WIRE。切勿将两种方式混用：要么以语义和约束驱动求解器（仅写极少数甚至不写 PLACE），要么直接写全所有位置而不再求解。
- 排序约束宜施加在较大的功能模块或支路上，而非逐个排列零散器件，例如 `{#V2 #C1 %bias %stage #C3 #R5} @ORDER("left-to-right");` 即可将各列整体从左到右依次排开。结构代表它的成员，因此 `%stage` 的每个器件都排在 `%bias` 的每个器件与 C3 之间；两个结构共用的器件不会与自身排序。若要改为约束结构的中心，则写出它自身的点：`{%dp::position #X1} @ALIGN("y");` 将 X1 放在差分对中心的高度上，而 `{%dp #X1} @ALIGN("y");` 会把三个晶体管都放在 X1 的高度上（结构将自身的点绑在何处见目录的“绘制”一栏）。
- 电源、信号源等外围部件通常立在电路一侧，与主体略微拉开距离，可用 DISTANCE 表达：`#VDD {#VIN #XU1 #CL} @DISTANCE("left-to-right", min=4);` 使电源保持在其余各对象左侧至少 4 个单位处。
- 为整张图建立清晰的分组树：每个功能模块作为一个分组，小分组嵌套在更大的分组内，每个节点下直接包含五六个对象左右。许多结构本身已是分组（见目录“分组”一栏）；而像电流镜这样可能部分跨在另一结构之内的结构则不是分组。完整结构或放大级连同其端口应自行成组框起：`%amp %p_inp %p_inn %p_out @GROUP;`。分组不可交叉重叠：若两个分组共用成员却互不嵌套，求解器只保留其中一个作为整体块。
- 仅在读者有明确惯例预期、且单靠其他约束无法确定朝向的对象上才显式固定朝向，常见于信号源、独立电源与负载，如 `{#V1 #V2 #R5} @ORIENT("R0");`；其余器件的朝向交给求解器即可，电路结构与电流方向通常已足以将其确定。

## 引脚与方向

```rnl
#M1::d @ORIENT("up"); /* a pin takes a direction: left down right up */
#M1::g @ORIENT("left"); /* drain up and gate left: M1 is R0, nothing else */
#M3::s @ORIENT("up"); /* a PMOS upright */
#M1::g #M2::g @ORIENT-RELATION("R180"); /* two pins face opposite ways */
{#M3::d #M1::d} @ALIGN("x"); /* stack: the drains in one column... */
&x @WIRE {#M3:d #M1:d}; /* ...and wired, or the fill labels them */
%t = $IO_PORT @ATTACH &inp; %t::pin @ORIENT("right"); {%t::pin #M1::g} @ALIGN("y"); &inp @WIRE {%t::pin #M1::g};
$IO_PORT_STUB @ATTACH #M1:g; /* the same tag, put on the pin by the package */
```

- 只固定一个引脚的方向时，八种朝向中仍会留下两种可能：例如仅固定栅极朝向时器件仍可正立或倒置，仅固定漏极朝向时栅极仍可朝左或朝右；若要唯一锁定朝向，应同时固定两个引脚。
- 标准符号在 R0 朝向下的引脚方向：R、C、L 与各类源均为 p 朝上、n 朝下；`$DIODE` 为 a 朝上、k 朝下；`$NMOS` 为 d 朝上、g 朝左、s 朝下，四端符号（4TERM）另有 b 朝右；`$PMOS` 为 s 朝上、g 朝左、d 朝下；`$NPN` 为 c 朝上、b 朝左、e 朝下；`$PNP` 为 e 朝上、b 朝左、c 朝下。

## 电流

```rnl
#M1:d @CURRENT("in"); #M1:s @CURRENT("out"); /* the static current at a terminal: in, out, unspecified */
{#M1:d #M2:d #M5:d} @CURRENT("in"); /* a set does each member */
#:out @CURRENT("in"); /* at a port: current enters the subcircuit */
#M1::d @CURRENT("in"); /* a pin stands for its terminal */
```

- CURRENT 本身不绘制任何图形，而是向求解器标明电路的工作流向，供其排布各条电流路径；主要电流通路均应标出。
- 仅在确实存在静态直流电流且流向明确的端子上标注：电容、MOS 栅极以及无直流分量的信号源不承载静态电流，应予省略；方向拿不准的端子也不要标注。独立电源同样不要标注 CURRENT——电源内部的电流由 n 流向 p（自下而上），若标出后遇到偏好电流自上而下的求解器，反而会把电源符号倒转过来。
- 若同一网络上的全部端子与端口都被标成单向抽走或单向注入电流，检查器会报告一条违例；0 等全局网络则不作此项检查。

## 优先级

```rnl
#M5 @PLACE(x=3, priority="low"); /* disabled, low, normal (default), high */
%dp1 = #M1(left) #M2(right) #M5(tail) @AS(priority="high") $DIFFERENTIAL_PAIR_N; /* covers every fact of the body */
%dp1 @PRIORITY("disabled"); /* turns the default drawing of the pair off */
```

- 发生几何事实冲突时，先丢弃优先级较低的事实，同优先级下则先丢弃书写靠前的事实；凡是最终保留下来的事实均作为求解器必须满足的硬约束。标准包自带的默认画法优先级均为 `"low"`。记录头的 `priority=` 为记录内语句提供默认值，但 AS 与 ATTACH 仅从中继承 `"disabled"`。
- 结构本身无需指定优先级：仅在检查或求解暴露出冲突、确需裁决时才写出优先级。

## 导线与布线

```rnl
%j = (_, _);
&b @WIRE {#R1:n %j #R2:p}; /* wires join only at shared point objects and terminals */
&b @WIRE(mode="manhattan") {%j #R3:p}; /* auto straight horizontal-vertical vertical-horizontal manhattan manhattan45 */
$JUNCTION @ATTACH %j; /* the dot is written */
&a @ROUTE-NET; /* the solver draws the whole net */
{#R3 #R4} @ROUTE-OBJECT; /* the connections among these objects' pins */
#R3:p @UNROUTE-OBJECT; &x @UNROUTE-NET; /* take pins out of trees; on one pin the higher priority, then the later, wins */
&vss @ROUTE-NET(priority="high"); $GROUND @ATTACH &vss; /* ground and supply nets are unrouted at high: wire them so, and attach a ground */
&D<1:0> @WIRE {%a %b}; /* several nets: a bus */
%all = # @SELECT-NET; %all @ROUTE-NET; /* every net of this level as wires; the fill keeps the ground, supply and bias nets unrouted at high, so they keep their stubs */
```

- 两个引脚即使坐标重合也不会自动建立连接；必须在二者之间显式写一条 WIRE（即便长度为零），才能将其标为已接线，避免自动补位再生出多余短桩。
- 标准结构会以低优先级自动绘制其内部的连接导线；而两个结构之间的跨接网络默认通过网络标签相连，除非显式写出 ROUTE 语句。
- 只把画成实线更易读的连接布成导线，切忌全图一律连线：求解器仅绘制 WIRE、ROUTE-NET 与 ROUTE-OBJECT 所指定的导线，其余未接线的引脚则由自动补位挂上接地、电源或网络标签短桩。主信号通路以及相邻结构内部和彼此之间的连接宜画成导线；跨度较大或走线拥挤的连接（如偏置线及跨全图共用的网络）则宜留给网络标签。
- 在小型电路中，除电源、地与偏置轨之外的网络画成导线往往都很清晰，可按上方示例最后一行的写法统一布线，或写 `# @SELECT-NET => @ROUTE-NET;`，两者画出的图面一致。无论哪种写法，自动补位都会以高优先级取消地、电源与偏置网络的布线，每条电源轨网络都会收到一条提示（RNL9403），指出其布线树中的引脚已被全部移除；这条提示是预期之中的。
- 连到所在层级端口的网络，只有写出端口标签才会连到端口：`$IO_PORT @ATTACH &x;`，或在它驱动的引脚上用 `$IO_PORT_STUB`。自动补位是在决定哪些引脚已接线之后才加端口标签的，所以只有一个自己的引脚的网络（如只驱动一个栅极的输入）构成只有一个点的树：这个引脚得到网络标签，没有导线连到端口。

## 文字

```rnl
#M1 @TEXT("${w}/${l}", key="wl", dy=-3); /* a label that moves with M1; #M1::wl */
#M1 @TEXT("M_{in}", key="name"); /* the same key replaces the name the symbol shows */
(10, 10) @TEXT("bias", size=0.8); /* on a point: an annotation */
#M1 @TEXT("\overline{EN} \mu A \frac{W}{L}", halign="middle", valign="top", rotate=90);
```

- `${…}` 属性引用支持读取：`name`、`value`、`model`、`type` 以及 `w` 等网表参数；端子或导线的 `net`；用例的角色名；以及形如 `${pair::left::w}` 的多级属性路径。
- `dx`、`dy` 表示文字相对于所附着对象的坐标偏移，而 `#M1::wl @PLACE(5, 0);` 则将标签直接定位于图面坐标 (5, 0)；在每个坐标轴上，PLACE 绝对位置与偏移量彼此取代，以书写靠后者为准。在符号体内部，这两种写法均采用符号自身的局部坐标系。

## 符号与样式

```rnl
$TRI = @DEFINE-SYMBOL {
  %in = (-4, 0) @PIN(terminal="in", direction="left"); /* a named pin: where, which terminal, which way */
  %out = (4, 0) @PIN(terminal="out", direction="right");
  @LINE {(-2, 2) (2, 0) (-2, -2) (-2, 2)};
  (0, 0) @RECTANGLE(width=4, height=2); (0, 0) @CIRCLE(radius=1);
  ? @TEXT("${name}", key="name") => @PLACE(-2, 2.5); /* ? is the object drawn */
};
#X1 @SET-SYMBOL $TRI; /* terminal names match by name, numbers by place */
#X2 @SET-SYMBOL(terminals=["inp", "inn", "out"]) $OPAMP; /* names differ: the symbol's names in node order */
# @SELECT-INSTANCE(model="nch", recursive=true) => @SET-SYMBOL $NMOS; /* by rule; a direct SET-SYMBOL wins */
%blk = @DEFINE-SYMBOL { %i = (-3, 0) @PIN(terminal="in", direction="left"); (0, 0) @RECTANGLE(width=6, height=3); };
# @SET-SYMBOL %blk; /* inside a .subckt: its block symbol, for its instances above */
$orange = @DEFINE-STYLE(linecolor="#e07000", textcolor="#e07000") {};
# @SELECT-INSTANCE => @SET-STYLE $grey; #M1 @SET-STYLE $orange; /* M1 is orange: direct beats rule, per property */
# @SELECT-USAGE(recursive=true) $GROUND => @SET-STYLE $grey; /* virtual parts are objects of their own */
```

- 可用的样式属性包括 `linecolor linestyle linewidth fillcolor textcolor size font bold italic textvisible`，颜色值采用 `"#RRGGBB"` 十六进制格式。`%` 名称也可用于绑定局部定义。各样式属性分别独立结算：直接指定对象的设置优先于筛选规则；同一层级内则以书写靠后者为准。

## 检查

标准包中的电路结构会自动核查其所声明的电气与参数条件，例如差分对两管是否具有相同的模型与尺寸、串联结构是否确实首尾串接、pmos 角色填入的是否确为 PMOS 管、端口标签是否挂在真实的端口网络上等。核查未通过时会产生一条**违例**：它仅是一条诊断报告，不会使任何语句失效，也不会改变图面绘制结果。遇到违例时，应修正结构用法或角色绑定，或改用约束条件更宽泛的结构定义。

## 标准包

```rnl
@USE-PACKAGE(name="rnl-std-all", version="0.4"); /* everything: styles, symbols, rules for SPICE and Spectre, fill, structures, the blocks */
@USE-PACKAGE(name="rnl-std-no-labels", version="0.4"); /* optional, besides rnl-std-all: hides the labels symbols draw (name, value, model, net names); a label you write shows only with textvisible=true */
@USE-PACKAGE(name="rnl-std-45deg", version="0.4"); /* optional: symbols at 45 degrees */
@USE-PACKAGE(name="rnl-std-fill") {}; /* an empty package drops that part */
&vdd @ADD-TO $SUPPLY_NET_SET; &vss @ADD-TO $GROUND_NET_SET; /* supply and ground stubs on their pins; 0 is a ground already */
$PORT_LIST @ATTACH; /* a port tag on every port net, in one column */
$FIGURE @ATTACH %p => @SET-SYMBOL $MY_FIG; /* no netlist instance: use a symbol with unbound pins and no ${name}/${value}; not $RESISTOR */
```

- 标准包默认分配的符号如下：R 用 `$RESISTOR`，C 用 `$CAPACITOR`，L 用 `$INDUCTOR`，V 用 `$VOLTAGE_SOURCE`，I 用 `$CURRENT_SOURCE`，D 用 `$DIODE`，E 用 `$VCVS_4TERM`，G 用 `$VCCS_4TERM`，F 用 `$CONTROLLED_CURRENT_SOURCE`，H 用 `$CONTROLLED_VOLTAGE_SOURCE`，B 用 `$BEHAVIORAL_SOURCE`，S 用 `$VOLTAGE_CONTROLLED_SWITCH`，W 用 `$SWITCH`，T、O 用 `$TRANSMISSION_LINE`，U 用 `$RC_LINE`；晶体管按器件类型分配：nmos 用 `$NMOS_4TERM`，pmos 用 `$PMOS_4TERM`，npn 用 `$NPN`，pnp 用 `$PNP`，njf 用 `$NJFET`，pjf 用 `$PJFET`；其中衬底接地网络的 NMOS 与衬底接电源网络的 PMOS 会自动改用三引脚符号 `$NMOS` 与 `$PMOS`，无需手动编写 SET-SYMBOL。未指明类型的晶体管（既无 `.model` 模型卡，也无对应的 DEVICE 声明）没有器件符号，画成方框，每个节点一个引脚（9.2.8）。
- 应如上方示例所示，将各电源网络加入 `$SUPPLY_NET_SET`，并将 0 以外的各地网络加入 `$GROUND_NET_SET`，这样其上未接线的引脚会自动补上相应短桩，且网络本身保持不布线。自动补位会跳过已连有导线或挂有不连接标记的引脚；对于其余未接线的引脚（器件的或虚拟元件的），落在 `$GROUND_NET_SET`（默认含网络 0）上的补 `$GROUND_STUB`，落在 `$SUPPLY_NET_SET`（默认为空）上的补 `$SUPPLY_STUB`，落在 `$BIAS_NET_SET`（默认为空）上的补 `$BIAS_STUB`，落在其他网络上的补 `$NET_LABEL_STUB`，而四端 MOS 管的衬底引脚无论接在哪个网络上均补 `$NET_LABEL_STUB`。本层每个端口网络（含电源和地端口）都会自动补一个 `$IO_PORT`，在本层全部元件左侧至少 6 个单位处排成一列，均取 R0 朝向；若某端口无需标签，可写 `&vdd @REMOVE-FROM $PORT_FILL_SET;`，若要自行控制某个端口标签的位置，则可显式写出 `$IO_PORT @ATTACH &out;`。端口标签自带 key 为 `netname` 的文字以显示所属网络名称，切勿再额外附着显示网名的 TEXT，如需调整网名位置可写 `%t::netname @PLACE(x, y);`；测试平台中的内部节点名称则应使用 `$NET_LABEL`。已挂有 `$IO_PORT_STUB` 或 `$NET_LABEL_STUB` 的引脚同样不会再重复补网络标签（尽管它们的短引线由符号本身绘出而非 WIRE）。`*_FILL_SET` 系列集合专供自动补位内部生成短桩和端口标签使用：求解器将补位结果写回记录后，这些集合便不再包含已处理的引脚与网络，因此切勿基于它们编写其他规则（如为集合内引脚添加 TEXT），而应自行筛选目标引脚。若需调整哪些引脚参与补桩，可向这些集合增删引脚；注意，用户自己的规则所挂上的短桩不会被自动补位识别，补位仍会照常再补一个。

## 求解器记录

```spice fragment
* @RNL(type="generated", priority="high", author="rnl-solver") { #R1 @PLACE(0, 4); #R2 @PLACE(0, -4);} /* never edit; replaced on each solve */
* @RNL(author="editor") { #R2 @PLACE(y=-6, priority="high");} /* to fix a position: high, in a later record */
```

形如 `%#3` 的匿名对象编号仅用于求解器生成的记录；手写 RNL 时应先为对象显式命名再行引用。

## 其他关键字

以下关键字多由包文件或工具写入，平时极少手写，能够读懂即可：

```rnl
@VERSION("0.4"); @PACKAGE(name="my-pkg", version="1.0"); /* header records */
@DEVICE(model="sky130_fd_pr__nfet_01v8*", terminals=["d", "g", "s", "b"], type="nmos"); /* names and types of devices */
@ALIAS("@at", "@PLACE"); @USE-ALIASES(name="alias-zh"); /* other spellings */
@SOLVER-CONFIG(spacing_x=2, junction="$JUNCTION"); /* settings of the solver */
#XI<3:0> @BUNDLE; /* an array of instances drawn as one */
#X @LAYER("bottom"); /* drawing layer */
%u_set = %a_set %b_set @UNION; /* a set without repeats */
(0, 0) @ELLIPSE(width=4, height=2); @ARC {(0, 2) (2, 0) (0, -2)}; @BEZIER {(0, 0) (1, 1) (2, 1) (3, 0)}; /* more graphics */
&n1 @SELECT-WIRE(bus=true); #M1 @SELECT-LABEL(key="wl"); # @SELECT-ANNOTATION; #M1 @SELECT-ANCESTOR-USAGE $CURRENT_MIRROR_N;
{#M1 #M2} @ASSERT-INSTANCE(same=["model", "w", "l"]); {#M1:s #M2:s} @ASSERT-TERMINAL(same=["net"]);
{&inp &inn} @ASSERT-NET(port=true); {#R1 #R2 #R3} @ASSERT-TOPOLOGY("series"); /* the checks inside structures */
```

## 常见错误

下表左栏为非法或易生歧义的错误写法；编写时请只采用右栏的正确写法，并替换为你自己文件中的对象名称。

|错误写法|正确写法|
|---|---|
|`#M1 @PLACE (10, 20);`|`#M1 @PLACE(10, 20);`|
|`#M1 (left) #M2 (right) @AS $CROSS_COUPLED_PAIR;`|`#M1(left) #M2(right) @AS $CROSS_COUPLED_PAIR;`|
|`#M1 #M2 @SET-SYMBOL $NMOS;`|`{#M1 #M2} @SET-SYMBOL $NMOS;`|
|`#M5 #M3 @AS $MATCHED;`|`{#M5 #M3} @AS $MATCHED;`|
|`{#R1 #R2} @ORDER("x");`|`{#R1 #R2} @ORDER("left-to-right");`|
|`#R1 @PLACE(x=2u);`|`#R1 @PLACE(x=2);`|
|沿用 0.3 版名称 `$DIFFERENTIAL_PAIR` 及角色 `in_p/in_n`|`#M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N;`|
|`#R1 @SET-SYMBOL $GROUND;`|放置接地符号应写 `$GROUND @ATTACH &n;`，电阻仍保留原有符号|
|`%t #M1 @ALIGN("y");`（试图对齐二者的引脚）|`{%t::pin #M1::g} @ALIGN("y");`|
|在非端口网络上写 `$IO_PORT @ATTACH &n;`|`$NET_LABEL @ATTACH &n;`|
|vss 已在地网络集合中时只写 `&vss @ROUTE-NET;`|`&vss @ROUTE-NET(priority="high");`|
|在手写记录中使用 `%#3 @PLACE(1, 2);`|先为该对象起名，再通过名称引用|
|`#M1.d; M3; &vdd!;`|`#M1:d; #M3; &"vdd!";`|
|`{@VERSION("0.4");}`|`{ @VERSION("0.4");}`|

将导线连到三端符号 `$NMOS` 的衬底端子会导致语句失效，此时应改用四端符号 `$NMOS_4TERM`。类型未知的晶体管需补充模型卡、DEVICE 声明或显式指定符号。写在顶层的规则必须加上 `recursive=true` 才能作用于下层子电路。

## 完整示例

```spice
* Five-transistor OTA
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4");}
.model nch nmos level=1
.model pch pmos level=1
.subckt ota5t inp inn out vdd vss vbias
M1 x    inp   tail vss nch W=2u L=0.5u
M2 out  inn   tail vss nch W=2u L=0.5u
M3 x    x     vdd  vdd pch W=4u L=0.5u
M4 out  x     vdd  vdd pch W=4u L=0.5u
M5 tail vbias vss  vss nch W=4u L=1u
* @RNL {
*   &vss @ADD-TO $GROUND_NET_SET; /* grounds and supplies instead of labels */
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   %dp1 = #M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N; /* mirrored about the tail, tail below */
*   %cm1 = #M3(ref) #M4(out) @AS $CURRENT_MIRROR_P;
*   {%dp1 %cm1} @ROUTE-OBJECT; /* the nets between them as wires */
*   {#M1::d #M2::d #M5::d} @ORIENT("up"); /* NMOS upright */
*   #M1::g @ORIENT("left"); /* gates outwards */
*   {#M3::s #M4::s} @ORIENT("up"); /* PMOS upright */
*   {#M3::d #M1::d} @ALIGN("x"); /* each load over its input device */
*   {#M4::d #M2::d} @ALIGN("x");
*   #M3 #M1 @DISTANCE("top-to-bottom", min=4);
*   {#M1:d #M2:d #M5:d #M3:s #M4:s} @CURRENT("in"); /* current down each branch */
*   {#M1:s #M2:s #M5:s #M3:d #M4:d} @CURRENT("out");
*   $PORT_LIST @ATTACH; /* the ports in one column */
*   {#M1 #M2} @TEXT("${w}/${l}", key="wl", dy=-3);
* }
.ends ota5t
.end
```

交稿前请逐项核查：头段记录置于最前且包含 VERSION 与所用包；所有引用的名称均存在于所属层级；仅使用 0.4 版标准名称（以文末目录为准）；每个 `@` 前留有空格、每条语句末尾写有 `;`；约束方向均已写明；集合角色传入集合；地与电源网络已加入各自集合；仅在确有必要处固定朝向、仅在流向明确处标注电流；已对齐的相接引脚均已连上导线；未手工改动生成的记录；且不存在非预期的违例。

<!-- lists:begin -->
## 名称目录

本目录仅供查阅签名，不可直接复制为 RNL 代码：引用时请保留名称前的 `$` 前缀；套用普通定义使用 AS 或 ATTACH，指定符号使用 SET-SYMBOL，指定样式使用 SET-STYLE。签名中的角色按声明次序排列，并注明可接收的对象种类（未注明种类的角色可接收任何种类的对象）；写作 `{role}` 的集合角色接收整个集合且可省略；定义参数列于分号 `;` 之后。例如 `$MATCHED({members}: instance/usage)` 在代码中应写作 `{#M1 #M2} @AS $MATCHED;`。

### 包

以下各包均在头段记录中通过 `@USE-PACKAGE(name="...", version="0.4");` 引入。

|包|内容|
|---|---|
|rnl-std-all|0.4 标准系列唯一的包集合，自身不含具体定义，而是通过各宿主语言对应的头段记录统一引入样式包、基础与半导体符号及定义包、自动补位包、电路语义包、四个领域包以及各宿主方言的规则包（2.8、16.3）。|
|rnl-std-style|标准样式定义（9.3）及默认样式分配规则。|
|rnl-std-basic|无源器件、独立源与受控源、开关、传输线、虚拟元件与标记的符号；虚拟元件、标记、独立图形与短桩的定义（各附绘制其用例的默认规则）；以及地、电源与偏置网络的共享集合。|
|rnl-std-semiconductors|二极管以及 MOS、双极型（BJT）、JFET 和单结（UJT、PUT）晶体管的符号。|
|rnl-std-spice|SPICE 方言规则包：根据元件首字母、模型类型以及衬底或负控制端所接网络为元件自动指定符号（16.3）。|
|rnl-std-spectre|Spectre 方言规则包：根据 master 名称、模型类型以及衬底或负控制端所接网络为元件自动指定符号（16.3）。|
|rnl-std-fill|自动补位所用的集合与规则：为未接线的引脚（包括虚拟元件的引脚）自动挂上接地、电源、偏置与网络标签短桩，并为 `$PORT_FILL_SET` 中的每个端口网络添加端口标签（10.5、12.4、16.3）。|
|rnl-std-semantics|各类电路结构及其默认布局、布线画法的语义定义。|
|rnl-std-functional|行为级功能模块符号：运算放大器、比较器、电压放大器、跨导放大器、ADC、DAC 与斩波调制器，并提供带整数格点电源引脚 vp（顶）和 vn（底）的 *_SUPPLY 变体。|
|rnl-std-rf|射频器件符号：中心抽头电感与 T 型线圈（T-coil）。|
|rnl-std-logic|数字逻辑门、缓冲器、反相器、延迟单元与 D 触发器符号。|
|rnl-std-signal-flow|理想信号流虚拟元件及其符号：加法器、减法器、乘法器、连续时间与离散时间积分器、量化器及单位延迟。|
|rnl-std-no-labels|隐藏标准符号自带的全部默认标签，供自行编写全部标签的文件使用（16.3）。|
|rnl-std-no-instance-labels|仅隐藏元件的默认标签，同时保留接地、电源、偏置标记、端口标签与网络标签所显示的网络名称（16.3）。|
|rnl-std-45deg|适用于电桥、交叉耦合对、对角变压器绕组与 45 度斜向走线的 45 度倾角符号。|

### 定义

对于 rnl-std-semantics 中的电路结构，“绘制”一栏标明了套用效果：“仅表语义”既不放置对象也不布线；“放置”对成员施加位置事实；“布线”自动连接成员共有的内部网络。结构内置的位置事实与布线优先级均为 low。“分组”一栏读自各定义的定义体：“是”后列它归入同一分组的各个角色，“经由” `$X` 后列它传给成组定义 `$X` 的各个角色，留空则表示不成组；像电流镜这样可能部分跨在另一结构之内的结构不成组（见“布局事实”一节的分组树）。没有符号的用例在 ALIGN、ORDER、DISTRIBUTE、DISTANCE 中代表它的成员；它自身的点 `%u::position`（14.1）只在注明“自身的点”的结构里才与其成员绑定：只有这类用例才能凭其 `::position` 作为单个项移动，或充当另一放置结构的成员；其余结构则约束其成员，或约束由它们组成的命名的集合。其他包定义的是实际绘出的部件与标记。

|签名|绘制|分组|含义|
|---|---|---|---|
|**rnl-std-basic**||||
|`$GROUND(p: net)`|—||挂在网络上的接地符号，绘制于用例自身的点：接在网络 0 上时仅画三条接地横杠，接在其他任何网络上时还会在下方显示网络名称。|
|`$SUPPLY(p: net)`|—||挂在网络上的电源符号，绘制于用例自身的点。|
|`$BIAS(p: net)`|—||挂在网络上的偏置电压标记，绘制于用例自身的点。|
|`$IO_PORT(p: net)`|—||挂在网络上的端口标签，绘制于用例自身的点；所接网络必须是当前层级的端口。|
|`$NET_LABEL(p: net/pin)`|—||网络标签：带有一小段引线的网络名称，绘制于用例自身的点；若传入总线引脚（如叠画的引脚，11.8），则直接保留该引脚并以单个标签代表其全部网络，一并显示各网络名称（13.6）。|
|`$NO_CONNECT(t: terminal/pin)`|—||挂在刻意悬空的端子或引脚上的不连接标记；传入端子时原样保留端子对象，从而在总线某一位悬空时准确记录具体哪一位未连接（12.6）。|
|`$JUNCTION(p: point/terminal)`|—||绘制在点对象或端子上的导线交汇圆点。|
|`$CURRENT_ARROW(p: terminal/point)`|—||绘制在端子或点对象上的电流方向箭头。|
|`$ARROW(at: point)`|—||绘制在符号内部或图面场景某一点上的箭头图形（9.2.7）。|
|`$SPDT_SWITCH(at: point)`|—||在指定点上作为纯图形绘制的单刀双掷开关，无对应网表语句。|
|`$FIGURE(at: point)`|—||在指定点上绘制的独立图形，无对应网表语句，需配合 SET-SYMBOL 指定其符号。|
|`$PORT_LIST()`|—||为当前层级中尚未显式放置 `$IO_PORT` 的每个端口网络各挂一个端口标签，自上而下排成一列并成组，位于本层全部元件左侧至少 6 个单位处，均取 R0 朝向（引脚朝右），优先级均为 low，画法与 rnl-std-fill 一致；使用后自动补位不再为本层另加端口标签（12.4）。|
|`$COMMENT({target}; text="")`|—||附着在任意对象上的非绘制注释；省略 target 时附着在当前层级本身。|
|`$GROUND_STUB(t: pin; gap=3)`|—||靠近引脚的正立接地短桩，自带连接导线：与引脚同高或位于其下方，竖直方向相距至多 gap 个单位（优先级 low）。|
|`$SUPPLY_STUB(t: pin; gap=3)`|—||靠近引脚的正立电源短桩，自带连接导线：与引脚同高或位于其上方，竖直方向相距至多 gap 个单位（优先级 low）。|
|`$BIAS_STUB(t: pin)`|—||直接贴靠在引脚上的偏置电压短桩，像端口短桩一样背离引脚向外伸出；其斜杠在引脚上方、下方与左侧呈 / 斜向，在右侧呈 \ 斜向。|
|`$IO_PORT_STUB(t: pin)`|—||直接贴靠在引脚上的端口标签短桩，以符号自带的短引线充当连接线；与 `$NET_LABEL_STUB` 一样，该引脚与标签均不再参与自动布线。|
|`$NET_LABEL_STUB(t: pin)`|—||直接贴靠在引脚上的网络标签短桩，以符号自带的 1 单位短引线充当连接线；挂在总线引脚上时整条总线共用一个标签；该引脚与标签都不布线。|
|**rnl-std-semantics**||||
|`$MATCHED({members}: instance/usage)`|仅表语义||在器件类型（或结构定义）及版图布局上相互匹配的器件或子结构组。|
|`$MATCHED_MOS({members}: instance)`|仅表语义||类型、模型与沟道长度均匹配的 MOSFET 组。|
|`$MATCHED_BJT({members}: instance)`|仅表语义||类型、模型与单位发射极几何尺寸均匹配的双极型晶体管组。|
|`$MATCHED_RESISTOR({members}: instance)`|仅表语义||类型与模型匹配的电阻组。|
|`$MATCHED_CAPACITOR({members}: instance)`|仅表语义||类型与模型匹配的电容组。|
|`$MIRRORED_LAYOUT(left: instance/usage, right: instance/usage, axis: point/instance/usage)`|放置；自身的点在中心||处于同一高度、关于给定轴线互为镜像的两个对称对象：仅为镜像布局，不声明任何电路语义。|
|`$MIRRORED_VERTICAL_LAYOUT(top: instance/usage, bottom: instance/usage, axis: point/instance/usage)`|放置；自身的点在中心||处于同一列、关于给定水平轴互为镜像的两个对称对象：仅为镜像布局。|
|`$BRANCH({members}: instance/usage)`|放置、布线；自身的点绑定 x||纵向叠放的串联支路结构：各成员排成一列自上而下依次串联，常用于电源到地之间的直流支路。|
|`$BRANCH_HORIZONTAL({members}: instance/usage)`|放置、布线；自身的点绑定 y||横向排成一行的串联支路结构：各成员处于同一高度，自左向右依次串联。|
|`$CASCADE({stages}: instance/usage)`|仅表语义||沿信号通路依次级联的多级电路。|
|`$SERIES({members}: instance/usage)`|布线||沿同一条电流通路首尾依次串联的成员组。|
|`$PARALLEL({members}: instance/usage)`|布线||跨接在相同两个网络之间并联的成员组。|
|`$PARALLEL_HORIZONTAL({members}: instance/usage)`|放置、布线；自身的点在中心||在同一高度左右并排并联的成员组，在纵向支路列中整体占一个位置。|
|`$PARALLEL_VERTICAL({members}: instance/usage)`|放置、布线；自身的点在中心||在同一 x 坐标处上下叠放并联的成员组，在横向通路行中整体占一个位置。|
|`$SIGNAL_FLOW({path}: instance/usage, {source}: instance/usage/terminal/net, {load}: instance/usage/terminal/net)`|仅表语义||单向传输的信号流：从信号源出发，按书写顺序依次流经通路各级，最终到达负载。|
|`$POWER_FLOW({path}: instance/usage, {source}: instance/usage/terminal/net, {load}: instance/usage/terminal/net)`|仅表语义||功率传输路径（功率流）：从电源或电池等功率源出发，按书写顺序经由各级功率转换或开关电路，输送至用电负载。|
|`$BIAS_CIRCUIT({members}: instance/usage)`|仅表语义||偏置电路：如电流基准等用于为其他电路模块产生偏置的器件或结构。|
|`$FEEDBACK_LOOP({forward}: instance/usage, {feedback}: instance/usage)`|仅表语义||反馈环路：由前向通路与反馈网络构成的闭环结构。|
|`$DIFFERENTIAL_PAIR_N(left: instance, right: instance, tail: instance/usage)`|放置、布线；自身的点在中心|是：left, right, tail|尾管位于下方的 n 型晶体管差分对。|
|`$DIFFERENTIAL_PAIR_P(left: instance, right: instance, tail: instance/usage)`|放置、布线；自身的点在中心|是：left, right, tail|尾管位于上方的 p 型晶体管差分对。|
|`$COMPLEMENTARY_DIFFERENTIAL_PAIR(pairup: usage, pairdown: usage)`|布线|是：pairup, pairdown|轨到轨互补输入级：共用同一对差分输入的上下两组互补差分对。|
|`$PSEUDO_DIFFERENTIAL_PAIR(left: instance, right: instance)`|放置；自身的点在中心||伪差分对：不带尾管、以镜像方式绘制的两个匹配输入晶体管。|
|`$DEGENERATED_DIFFERENTIAL_PAIR_TYPE_H(left: instance, right: instance, tailleft: instance/usage, tailright: instance/usage, bridge: instance/usage)`|放置、布线；自身的点在中心|是：left, right, tailleft, tailright, bridge|H 型退化差分对：在两条独立尾支路之间跨接桥接支路以引入负反馈，整体呈 H 形。|
|`$DEGENERATED_DIFFERENTIAL_PAIR_TYPE_T_N(left: instance, right: instance, degleft: instance/usage, degright: instance/usage, tail: instance/usage)`|放置、布线；自身的点在中心|是：left, right, degleft, degright, tail|n 型 T 型退化差分对：在左右两个输入管下方各串入退化支路后再汇合至公共尾支路，整体呈 T 形，尾支路在下。|
|`$DEGENERATED_DIFFERENTIAL_PAIR_TYPE_T_P(left: instance, right: instance, degleft: instance/usage, degright: instance/usage, tail: instance/usage)`|放置、布线；自身的点在中心|是：left, right, degleft, degright, tail|p 型倒 T 型退化差分对：在左右两个输入管上方各串入退化支路后再汇合至公共尾支路，整体呈倒 T 形，尾支路在上。|
|`$CROSS_COUPLED_PAIR(left: instance, right: instance)`|放置、布线；自身的点在中心|是：left, right|交叉耦合对：交叉互连构成正反馈、以镜像方式绘制的两个匹配晶体管。|
|`$CURRENT_MIRROR_N({ref}: instance, {out}: instance)`|放置；自身的点绑定 y||由 n 型晶体管构成的低侧电流镜，源极或发射极朝下。|
|`$CURRENT_MIRROR_P({ref}: instance, {out}: instance)`|放置；自身的点绑定 y||由 p 型晶体管构成的高侧电流镜，源极或发射极朝上。|
|`$CASCODE_CURRENT_MIRROR_N({ref}: instance, {out}: instance, {refcascode}: instance, {outcascode}: instance)`|放置；自身的点绑定 y||低侧共源共栅电流镜：底层为电流镜管行，上方叠放共源共栅管行。|
|`$CASCODE_CURRENT_MIRROR_P({ref}: instance, {out}: instance, {refcascode}: instance, {outcascode}: instance)`|放置；自身的点绑定 y||高侧共源共栅电流镜：顶层为电流镜管行，下方叠放共源共栅管行。|
|`$TRANS_LINEAR_LOOP({a}: instance/usage, {b}: instance/usage)`|放置、布线；自身的点绑定 x|是：a, b|由两组晶体管构成的跨导线性环（translinear loop），按纵向单列排布。|
|`$TRANS_LINEAR_LOOP_HORIZONTAL({a}: instance/usage, {b}: instance/usage)`|放置、布线；自身的点绑定 y|是：a, b|由两组晶体管构成的跨导线性环，按横向单行排布。|
|`$DIODE_CONNECTED(device: instance)`|布线||二极管接法晶体管：栅极连向漏极，或基极连向集电极。|
|`$COMMON_SOURCE(device: instance)`|仅表语义||单场效应管共源级。|
|`$COMMON_GATE(device: instance)`|仅表语义||单场效应管共栅级。|
|`$COMMON_DRAIN(device: instance)`|仅表语义||单场效应管源极跟随器（共漏级）。|
|`$COMMON_EMITTER(device: instance)`|仅表语义||单双极型晶体管共射放大级。|
|`$COMMON_BASE(device: instance)`|仅表语义||单双极型晶体管共基级。|
|`$COMMON_COLLECTOR(device: instance)`|仅表语义||单双极型晶体管发射极跟随器（共集级）。|
|`$FOLLOWER(device: instance)`|仅表语义||单晶体管电压跟随器级。|
|`$CASCODE(source: instance/usage, cascode: instance)`|布线||共源共栅结构（cascode）：输入源器件或子结构与共栅晶体管串联。|
|`$FOLDED_CASCODE(source: instance/usage, cascode: instance, bias: instance/usage)`|布线||折叠式共源共栅结构：输入器件经偏置支路折入异型共栅晶体管。|
|`$COMMON_SOURCE_AMPLIFIER(input: instance, load: instance/usage)`|布线|经由 `$TRANSCONDUCTOR_Z_AMPLIFIER`：input, load|共源放大器：由场效应输入管及其负载组成。|
|`$COMMON_EMITTER_AMPLIFIER(input: instance, load: instance/usage)`|布线|经由 `$TRANSCONDUCTOR_Z_AMPLIFIER`：input, load|共射放大器：由双极型输入管及其负载组成。|
|`$DEGENERATED_COMMON_SOURCE_AMPLIFIER(input: instance, load: instance/usage, degenerator: instance/usage)`|布线|经由 `$DEGENERATED_TRANSCONDUCTOR_Z_AMPLIFIER`：input, load, degenerator|源极退化共源放大器：由场效应输入管、负载及源极退化支路组成。|
|`$DEGENERATED_COMMON_EMITTER_AMPLIFIER(input: instance, load: instance/usage, degenerator: instance/usage)`|布线|经由 `$DEGENERATED_TRANSCONDUCTOR_Z_AMPLIFIER`：input, load, degenerator|发射极退化共射放大器：由双极型输入管、负载及发射极退化支路组成。|
|`$MILLER_AMPLIFIER(stage: instance/usage, compensation: instance/usage)`|布线|是：stage, compensation|米勒补偿放大器：由增益级及其米勒补偿网络组成。|
|`$PUSH_PULL_STAGE(pullup: instance/usage, pulldown: instance/usage)`|布线|是：pullup, pulldown|推挽输出级：仅含输出晶体管本身，即串联的上拉管与下拉管；它们的偏置及前级驱动级不属于本结构。|
|`$TRANSCONDUCTOR_Z_AMPLIFIER(transconductor: instance/usage, load: instance/usage)`|布线|是：transconductor, load|跨导-阻抗放大级：由跨导放大器及其负载组成。|
|`$DEGENERATED_TRANSCONDUCTOR_Z_AMPLIFIER(transconductor: instance/usage, load: instance/usage, degenerator: instance/usage)`|布线|是：transconductor, load, degenerator|带退化的跨导-阻抗放大级：由跨导放大器、负载及退化支路组成。|
|`$PASS_SWITCH(device: instance/usage)`|仅表语义||用于导通或关断两侧信号的传输开关。|
|`$RESET_SWITCH(device: instance)`|仅表语义||用于预充电、均衡或复位的开关晶体管。|
|`$GROUND_SWITCH(device: instance)`|仅表语义||接往地网络的低侧接地开关。|
|`$POWER_SWITCH(device: instance)`|仅表语义||接往电源网络的高侧电源开关。|
|`$TRANSMISSION_GATE(pmos: instance/usage, nmos: instance/usage)`|放置、布线；自身的点在中心|是：pmos, nmos|CMOS 传输门，PMOS 在上、NMOS 在下。|
|`$TRANSMISSION_GATE_HORIZONTAL(pmos: instance/usage, nmos: instance/usage)`|放置、布线；自身的点在中心|是：pmos, nmos|左右并排绘制的横向 CMOS 传输门，PMOS 在左、NMOS 在右。|
|`$MULTI_POLE_SWITCH({throws}: instance/usage)`|布线|是：throws|共用同一刀端（公共端）的多掷开关组。|
|`$BOOTSTRAPPED_SWITCH(device: instance/usage)`|仅表语义||栅压自举采样开关。|
|`$SC_SAMPLER({switch}: instance/usage, {capacitor}: instance/usage)`|布线||开关电容采样器：由采样开关与采样电容组成。|
|`$CHARGE_SHARING(a: instance/usage, switch: instance/usage, b: instance/usage)`|布线||电荷共享网络：通过开关相连以分配电荷的两个电容或电容阵列。|
|`$CHOPPER({straight}: instance/usage, {crossed}: instance/usage)`|布线||斩波器：由四个开关构成、用于翻转差分信号极性的电桥结构。|
|`$FLYING_CAPACITOR(capacitor: instance/usage)`|仅表语义||随不同时钟相位切换连接关系的飞跨电容。|
|`$CMOS_INVERTER(pmos: instance, nmos: instance)`|放置、布线；自身的点在中心|经由 `$CMOS_GATE`：pmos, nmos|CMOS 反相器，PMOS 在上、NMOS 在下。|
|`$CMOS_INVERTER_HORIZONTAL(pmos: instance, nmos: instance)`|放置、布线；自身的点在中心|经由 `$CMOS_GATE`：pmos, nmos|左右并排绘制的横向 CMOS 反相器，PMOS 在左、NMOS 在右。|
|`$CMOS_GATE({pullup}: instance/usage, {pulldown}: instance/usage)`|布线|是：pullup, pulldown|静态 CMOS 逻辑门：上拉网络与下拉网络在输出节点串联汇合。|
|`$LATCH({members}: instance/usage)`|布线|是：members|锁存器：双稳态核心及其外围辅助器件。|
|`$INVERTER_LATCH(a: instance/usage, b: instance/usage)`|布线|经由 `$LATCH`：a, b|由两个反相器交叉耦合构成的锁存器。|
|`$SHIFT_REGISTER({stages}: instance/usage)`|仅表语义||移位寄存器：由触发器或锁存器依次级联而成的链式结构，常以叠瓦交叠方式绘制，且时钟引脚通常不画连线。|
|`$DIVIDER(upper: instance/usage, lower: instance/usage)`|布线||由上下两条支路串联构成的分压器。|
|`$RESISTOR_LADDER({resistors}: instance)`|放置、布线；自身的点绑定 x|是：resistors|排成一列、自上而下依次串联的电阻梯。|
|`$RESISTOR_LADDER_HORIZONTAL({resistors}: instance)`|放置、布线；自身的点绑定 y|是：resistors|排成一行横向依次串联的电阻梯。|
|`$PI_NETWORK(shunt1: instance/usage, series: instance/usage, shunt2: instance/usage)`|布线||π 型网络：依次由并联支路、串联支路与另一条并联支路构成。|
|`$T_NETWORK(series1: instance/usage, shunt: instance/usage, series2: instance/usage)`|布线||T 型网络：依次由串联支路、并联支路与另一条串联支路构成。|
|`$LC_TANK_PARALLEL(inductor: instance/usage, capacitor: instance/usage)`|布线|是：inductor, capacitor|并联 LC 谐振腔。|
|`$RLC_TANK_PARALLEL(resistor: instance/usage, inductor: instance/usage, capacitor: instance/usage)`|布线|是：resistor, inductor, capacitor|并联 RLC 谐振腔。|
|`$LC_TANK_SERIES(inductor: instance/usage, capacitor: instance/usage)`|布线|是：inductor, capacitor|串联 LC 谐振腔。|
|`$RLC_TANK_SERIES(resistor: instance/usage, inductor: instance/usage, capacitor: instance/usage)`|布线|是：resistor, inductor, capacitor|串联 RLC 谐振腔。|
|`$DC_FEED(device: instance/usage)`|仅表语义||向节点馈入直流电源或偏置的射频扼流圈或偏置电阻（直流馈电）。|
|`$DECAP(device: instance)`|放置；自身的点绑定 x||接在电源或偏置节点上的去耦电容。|
|`$AC_COUPLE(device: instance/usage)`|仅表语义||通交流、隔直流的串联交流耦合电容。|
|`$OSCILLATOR({members}: instance/usage)`|布线|是：members|由有源器件与谐振腔或延迟单元构成的振荡器。|
|**rnl-std-signal-flow**||||
|`$ADDER(in1: net, in2: net, out: net)`|—||理想信号流加法器（out = in1 + in2），使用 `$ADDER_SYMBOL` 绘制。|
|`$SUBTRACTOR(in1: net, in2: net, out: net)`|—||理想信号流减法器（out = in1 - in2），使用 `$SUBTRACTOR_SYMBOL` 绘制。|
|`$MULTIPLIER(in1: net, in2: net, out: net)`|—||理想信号流乘法器（out = in1 * in2），使用 `$MULTIPLIER_SYMBOL` 绘制。|
|`$INTEGRATOR(in: net, out: net)`|—||理想连续时间积分器，使用 `$INTEGRATOR_SYMBOL` 绘制。|
|`$DISCRETE_TIME_INTEGRATOR(in: net, out: net)`|—||理想离散时间积分器，使用 `$DISCRETE_TIME_INTEGRATOR_SYMBOL` 绘制。|
|`$QUANTIZER(in: net, out: net)`|—||理想量化器，使用 `$QUANTIZER_SYMBOL` 绘制。|
|`$UNIT_DELAY(in: net, out: net)`|—||理想单位延迟单元，使用 `$UNIT_DELAY_SYMBOL` 绘制。|
|**rnl-std-45deg**||||
|`$ARROW_45(at: point)`|—||绘制在符号内某点或场景中 45 度斜向导线上的 45 度箭头。|
|`$CURRENT_ARROW_45(p: terminal/point)`|—||沿 45 度斜向支路绘制在端子或点上的 45 度电流箭头。|

### 符号

下表按引脚所绑定的端子或角色列出各符号引脚：形如 `pin`（`p`）表示名为 `pin`、绑定到角色 `p` 的引脚；`pin` (nothing) 则表示不绑定任何端子或角色。

|符号|引脚|含义|
|---|---|---|
|**rnl-std-basic**|||
|`$RESISTOR`|`p` `n`|电阻。|
|`$VARIABLE_RESISTOR`|`p` `n`|可变电阻：带斜穿箭头的电阻符号。|
|`$CAPACITOR`|`p` `n`|电容。|
|`$VARIABLE_CAPACITOR`|`p` `n`|可变电容：带斜穿箭头的电容符号。|
|`$INDUCTOR`|`p` `n`|电感。|
|`$VARIABLE_INDUCTOR`|`p` `n`|可变电感：带斜穿箭头的电感符号。|
|`$TRANSFORMER`|`p1` `n1` `p2` `n2`|变压器：左右并立的两个耦合绕组，同名端极性圆点位于顶部。|
|`$POTENTIOMETER`|`n1` `nw` `n2`|三端电位器，中间引脚为滑动抽头（nw）。|
|`$PHOTORESISTOR`|`p` `n`|光敏电阻。|
|`$POLARIZED_CAPACITOR`|`p` `n`|极性（电解）电容，正极板位于 p 端。|
|`$CRYSTAL`|`n1` `n2`|石英晶体谐振器（晶振）。|
|`$SPEAKER`|`p` `n`|扬声器。|
|`$VOLTAGE_SOURCE`|`p` `n`|独立电压源。|
|`$CURRENT_SOURCE`|`p` `n`|独立电流源，内部箭头由 p 指向 n。|
|`$BATTERY`|`p` `n`|电池。|
|`$CONTROLLED_VOLTAGE_SOURCE`|`p` `n`|仅带输出引脚的菱形受控电压源符号。|
|`$CONTROLLED_CURRENT_SOURCE`|`p` `n`|仅带输出引脚的菱形受控电流源符号。|
|`$VCVS_4TERM`|`p` `n` `cp` `cn`|四端压控电压源（E），控制引脚位于左侧。|
|`$VCCS_4TERM`|`p` `n` `cp` `cn`|四端压控电流源（G），控制引脚位于左侧。|
|`$BEHAVIORAL_SOURCE`|`p` `n`|行为级受控源（B）：标有字母 B 的圆圈。|
|`$RF_PORT`|`p` `n`|射频端口：正弦源串联内阻，对应 Spectre 的 port 内置器件。|
|`$SINE_VOLTAGE_SOURCE`|`p` `n`|正弦电压源，+ 号位于 p 端。|
|`$PULSE_VOLTAGE_SOURCE`|`p` `n`|脉冲电压源，+ 号位于 p 端。|
|`$MICROPHONE`|`p` `n`|麦克风，+ 号位于 p 端。|
|`$AMMETER`|`p` `n`|电流表，测量由 p 流向 n 的电流。|
|`$SWITCH`|`p` `n`|呈断开状态的开关符号，用于流控开关（W）或 Spectre 的 switch。|
|`$CLOSED_SWITCH`|`p` `n`|呈闭合状态的开关符号。|
|`$EXTERNALLY_CONTROLLED_SWITCH`|`p` `n` `cp`|侧面带单个控制引脚的外控开关。|
|`$VOLTAGE_CONTROLLED_SWITCH`|`p` `n` `cp` `cn`|四端压控开关（S），差分控制引脚位于下方。|
|`$SPDT_SWITCH_SYMBOL`|`com` (nothing) `a` (nothing) `b` (nothing)|`$SPDT_SWITCH` 所用的符号：单刀双掷开关，其引脚 com、a、b 不绑定任何端子或角色，直接按引脚名称连线（9.2.4）。|
|`$TRANSMISSION_LINE`|`p1` `n1` `p2` `n2`|传输线（T 或 O），端口 1 在左、端口 2 在右。|
|`$RC_LINE`|`a` `b` `ref`|均匀分布 RC 传输线（U）。|
|`$GROUND_SYMBOL`|`pin` (`p`)|网络 0 上 `$GROUND` 所用的符号：引脚在顶部的三横杠接地符号。|
|`$NAMED_GROUND_SYMBOL`|`pin` (`p`)|非 0 网络上 `$GROUND` 所用的符号：下方附有网络名称的三横杠接地符号。|
|`$SUPPLY_SYMBOL`|`pin` (`p`)|`$SUPPLY` 所用的符号：上方带水平横杠与网络名称、引脚在底部的电源符号。|
|`$BIAS_SYMBOL`|`pin` (`p`)|`$BIAS` 所用的符号：上方带斜杠与网络名称、引脚在底部的偏置符号。|
|`$IO_PORT_SYMBOL`|`pin` (`p`)|`$IO_PORT` 所用的符号：带小圆圈与网络名称、引脚朝右的端口标签符号。|
|`$NET_LABEL_SYMBOL`|`pin` (`p`)|`$NET_LABEL` 所用的符号：与 `$IO_PORT_SYMBOL` 类似，引脚在右且朝右，左侧经 1 单位长短引线显示网络名称（总线引脚上则显示各网络名称）。|
|`$ARROW_SYMBOL`|—|指向右侧的实心箭头符号，即 `$ARROW` 的符号；`$CURRENT_ARROW` 画一个一半大小的。|
|`$NO_CONNECT_SYMBOL`|—|`$NO_CONNECT` 所用的符号：叉号标记。|
|`$JUNCTION_SYMBOL`|—|`$JUNCTION` 所用的符号：实心圆点。|
|**rnl-std-semiconductors**|||
|`$DIODE`|`a` `k`|二极管。|
|`$ZENER_DIODE`|`a` `k`|齐纳（稳压）二极管。|
|`$LED`|`a` `k`|发光二极管（LED）。|
|`$SCHOTTKY_DIODE`|`a` `k`|肖特基势垒二极管。|
|`$SHOCKLEY_DIODE`|`a` `k`|肖克利四层二极管。|
|`$PUT`|`a` `g` `k`|可编程单结晶体管（PUT）。|
|`$NMOS`|`d` `g` `s`|三引脚 NMOS 晶体管，不画出衬底端。|
|`$NMOS_4TERM`|`d` `g` `s` `b`|四引脚 NMOS 晶体管，衬底引脚与漏极、源极同列对齐，沟道处带衬底箭头。|
|`$PMOS`|`d` `g` `s`|三引脚 PMOS 晶体管，不画出衬底端。|
|`$PMOS_4TERM`|`d` `g` `s` `b`|四引脚 PMOS 晶体管，衬底引脚与漏极、源极同列对齐。|
|`$NMOS_LOGIC`|`d` `g` `s`|不带源极箭头的三引脚数字逻辑 NMOS 晶体管。|
|`$PMOS_LOGIC`|`d` `g` `s`|不带源极箭头、栅极带反相小圆圈的三引脚数字逻辑 PMOS 晶体管。|
|`$DEPLETION_NMOS`|`d` `g` `s`|三引脚耗尽型 NMOS 晶体管，不画出衬底端。|
|`$DEPLETION_NMOS_4TERM`|`d` `g` `s` `b`|四引脚耗尽型 NMOS 晶体管，衬底引脚与漏极、源极同列对齐。|
|`$DEPLETION_PMOS`|`d` `g` `s`|三引脚耗尽型 PMOS 晶体管，不画出衬底端。|
|`$DEPLETION_PMOS_4TERM`|`d` `g` `s` `b`|四引脚耗尽型 PMOS 晶体管，衬底引脚与漏极、源极同列对齐。|
|`$NDMOS`|`d` `g` `s`|三引脚 N 型 DMOS 晶体管，不画出衬底端。|
|`$NDMOS_4TERM`|`d` `g` `s` `b`|四引脚 N 型 DMOS 晶体管，衬底引脚与漏极、源极同列对齐。|
|`$PDMOS`|`d` `g` `s`|三引脚 P 型 DMOS 晶体管，不画出衬底端。|
|`$PDMOS_4TERM`|`d` `g` `s` `b`|四引脚 P 型 DMOS 晶体管，衬底引脚与漏极、源极同列对齐。|
|`$NPN`|`c` `b` `e`|NPN 双极型晶体管，不画出衬底端。|
|`$PNP`|`c` `b` `e`|PNP 双极型晶体管，不画出衬底端。|
|`$NJFET`|`d` `g` `s`|N 沟道 JFET，栅极箭头指向沟道。|
|`$PJFET`|`d` `g` `s`|P 沟道 JFET，栅极箭头背离沟道。|
|`$UJT`|`e` `b1` `b2`|单结晶体管（UJT），b2 在上、b1 在下。|
|**rnl-std-functional**|||
|`$OPAMP`|`inp` `inn` `out`|三端运算放大器：inp、inn、out。|
|`$OPAMP_SUPPLY`|`inp` `inn` `out` `vp` `vn`|带电源引脚的运算放大器：inp、inn、out、vp、vn。|
|`$OPAMP_INVERTED_SUPPLY`|`inp` `inn` `out` `vp` `vn`|带电源引脚的输入对调运算放大器：输入端上下对调（inn 在上、inp 在下），而电源引脚保持原位（vp 在上、vn 在下，不同于直接将 `$OPAMP_SUPPLY` 作 MX 翻转）：inp、inn、out、vp、vn。|
|`$DIFFERENTIAL_OPAMP`|`inp` `inn` `outp` `outn`|全差分运算放大器：inp、inn、outp、outn。|
|`$DIFFERENTIAL_OPAMP_SUPPLY`|`inp` `inn` `outp` `outn` `vp` `vn`|带电源引脚的全差分运算放大器：inp、inn、outp、outn、vp、vn。|
|`$DIFFERENTIAL_OPAMP_INVERTED_SUPPLY`|`inp` `inn` `outp` `outn` `vp` `vn`|带电源引脚的输入输出对调全差分运算放大器：输入与输出端均上下对调（inn 与 outn 在上、inp 与 outp 在下），电源引脚保持原位（vp 在上、vn 在下，不同于直接将 `$DIFFERENTIAL_OPAMP_SUPPLY` 作 MX 翻转）：inp、inn、outp、outn、vp、vn。|
|`$DIFFERENTIAL_OPAMP_TWISTED`|`inp` `inn` `outp` `outn`|输出极性交叉对调的全差分运算放大器（outn 在上、outp 在下）：inp、inn、outp、outn。|
|`$DIFFERENTIAL_OPAMP_TWISTED_SUPPLY`|`inp` `inn` `outp` `outn` `vp` `vn`|带电源引脚且输出极性交叉对调的全差分运算放大器：inp、inn、outp、outn、vp、vn。|
|`$DIFFERENTIAL_OPAMP_TWISTED_INVERTED_SUPPLY`|`inp` `inn` `outp` `outn` `vp` `vn`|带电源引脚且仅输入对调的全差分运算放大器：输入端上下对调（inn 在上、inp 在下），输出端与 `$DIFFERENTIAL_OPAMP_SUPPLY` 相同（outp 在上、outn 在下），电源引脚保持原位（vp 在上、vn 在下，不同于直接将 `$DIFFERENTIAL_OPAMP_TWISTED_SUPPLY` 作 MX 翻转）：inp、inn、outp、outn、vp、vn。|
|`$COMPARATOR`|`inp` `inn` `out`|三端比较器：inp、inn、out。|
|`$COMPARATOR_SUPPLY`|`inp` `inn` `out` `vp` `vn`|带电源引脚的比较器：inp、inn、out、vp、vn。|
|`$COMPARATOR_INVERTED_SUPPLY`|`inp` `inn` `out` `vp` `vn`|带电源引脚的输入对调比较器：输入端上下对调（inn 在上、inp 在下），电源引脚保持原位（vp 在上、vn 在下，不同于直接将 `$COMPARATOR_SUPPLY` 作 MX 翻转），内部阶跃符号走向仍与 `$COMPARATOR_SUPPLY` 一致：inp、inn、out、vp、vn。|
|`$VOLTAGE_AMPLIFIER`|`in` `out`|电压放大器，中央带可选的 key 为 label 的文字：in、out。|
|`$VOLTAGE_AMPLIFIER_SUPPLY`|`in` `out` `vp` `vn`|带电源引脚的电压放大器：in、out、vp、vn。|
|`$TRANSCONDUCTOR`|`in` `out`|跨导放大器（gm 单元），中央带 key 为 label 的文字（默认为 g_m）：in、out。|
|`$TRANSCONDUCTOR_SUPPLY`|`in` `out` `vp` `vn`|带电源引脚的跨导放大器，中央带 key 为 label 的文字（默认为 g_m）：in、out、vp、vn。|
|`$DIFFERENTIAL_TRANSCONDUCTOR`|`inp` `inn` `out`|带 key 为 label 的文字（默认为 g_m）的差分输入跨导放大器：inp、inn、out。|
|`$DIFFERENTIAL_TRANSCONDUCTOR_SUPPLY`|`inp` `inn` `out` `vp` `vn`|带电源引脚和 key 为 label 的文字（默认为 g_m）的差分输入跨导放大器：inp、inn、out、vp、vn。|
|`$DIFFERENTIAL_TRANSCONDUCTOR_INVERTED_SUPPLY`|`inp` `inn` `out` `vp` `vn`|带电源引脚和 key 为 label 的文字（默认为 g_m）的输入对调差分跨导放大器：输入端上下对调（inn 在上、inp 在下），电源引脚保持原位（vp 在上、vn 在下，不同于直接将 `$DIFFERENTIAL_TRANSCONDUCTOR_SUPPLY` 作 MX 翻转）：inp、inn、out、vp、vn。|
|`$ADC`|`in` `out`|模数转换器（ADC）：in、out。|
|`$ADC_SUPPLY`|`in` `out` `vp` `vn`|带电源引脚的模数转换器：in、out、vp、vn。|
|`$DAC`|`in` `out`|数模转换器（DAC）：in、out。|
|`$DAC_SUPPLY`|`in` `out` `vp` `vn`|带电源引脚的数模转换器：in、out、vp、vn。|
|`$CHOPPER_SYMBOL`|`inp` `inn` `outp` `outn`|斩波调制器功能框符号（区别于四开关电桥结构 `$CHOPPER`）：inp、inn、outp、outn。|
|**rnl-std-rf**|||
|`$CENTER_TAP_INDUCTOR`|`n1` `n2` `tap`|中心抽头电感。|
|`$T_COIL`|`n1` `n2` `tap`|T 型线圈（T-coil）：带中心抽头与桥接电容的耦合双线圈。|
|**rnl-std-logic**|||
|`$AND_2INPUT`|`a` `b` `y`|二输入与门：a、b、y。|
|`$AND_3INPUT`|`a` `b` `c` `y`|三输入与门：a、b、c、y。|
|`$NAND_2INPUT`|`a` `b` `y`|二输入与非门：a、b、y。|
|`$NAND_3INPUT`|`a` `b` `c` `y`|三输入与非门：a、b、c、y。|
|`$OR_2INPUT`|`a` `b` `y`|二输入或门：a、b、y。|
|`$OR_3INPUT`|`a` `b` `c` `y`|三输入或门：a、b、c、y。|
|`$NOR_2INPUT`|`a` `b` `y`|二输入或非门：a、b、y。|
|`$NOR_3INPUT`|`a` `b` `c` `y`|三输入或非门：a、b、c、y。|
|`$XOR_2INPUT`|`a` `b` `y`|二输入异或门：a、b、y。|
|`$XOR_3INPUT`|`a` `b` `c` `y`|三输入异或门：a、b、c、y。|
|`$XNOR_2INPUT`|`a` `b` `y`|二输入同或门：a、b、y。|
|`$XNOR_3INPUT`|`a` `b` `c` `y`|三输入同或门：a、b、c、y。|
|`$AND_4INPUT`|`a` `b` `c` `d` `y`|引脚落在整数格点上的四输入与门：a、b、c、d、y。|
|`$NAND_4INPUT`|`a` `b` `c` `d` `y`|引脚落在整数格点上的四输入与非门：a、b、c、d、y。|
|`$OR_4INPUT`|`a` `b` `c` `d` `y`|引脚落在整数格点上的四输入或门：a、b、c、d、y。|
|`$NOR_4INPUT`|`a` `b` `c` `d` `y`|引脚落在整数格点上的四输入或非门：a、b、c、d、y。|
|`$XOR_4INPUT`|`a` `b` `c` `d` `y`|引脚落在整数格点上的四输入异或门：a、b、c、d、y。|
|`$XNOR_4INPUT`|`a` `b` `c` `d` `y`|引脚落在整数格点上的四输入同或门：a、b、c、d、y。|
|`$BUFFER`|`a` `y`|缓冲器：a、y。|
|`$INVERTER`|`a` `y`|反相器（非门）：a、y。|
|`$DELAY_CELL`|`a` `y`|延迟单元：a、y。|
|`$D_FLIP_FLOP`|`d` `ck` `q` `qb`|带互补输出 Q 与 QB 的 D 触发器：d、ck、q、qb。|
|`$D_FLIP_FLOP_Q_ONLY`|`d` `ck` `q`|仅带同相输出 Q 的 D 触发器：d、ck、q。|
|`$D_FLIP_FLOP_WITH_RESET`|`d` `ck` `rst` `q` `qb`|带复位端的 D 触发器：d、ck、rst、q、qb。|
|`$AND_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|带电源引脚的二输入与门：a、b、y、vp、vn。|
|`$AND_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|带电源引脚的三输入与门：a、b、c、y、vp、vn。|
|`$AND_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|带电源引脚的四输入与门：a、b、c、d、y、vp、vn。|
|`$NAND_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|带电源引脚的二输入与非门：a、b、y、vp、vn。|
|`$NAND_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|带电源引脚的三输入与非门：a、b、c、y、vp、vn。|
|`$NAND_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|带电源引脚的四输入与非门：a、b、c、d、y、vp、vn。|
|`$OR_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|带电源引脚的二输入或门：a、b、y、vp、vn。|
|`$OR_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|带电源引脚的三输入或门：a、b、c、y、vp、vn。|
|`$OR_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|带电源引脚的四输入或门：a、b、c、d、y、vp、vn。|
|`$NOR_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|带电源引脚的二输入或非门：a、b、y、vp、vn。|
|`$NOR_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|带电源引脚的三输入或非门：a、b、c、y、vp、vn。|
|`$NOR_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|带电源引脚的四输入或非门：a、b、c、d、y、vp、vn。|
|`$XOR_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|带电源引脚的二输入异或门：a、b、y、vp、vn。|
|`$XOR_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|带电源引脚的三输入异或门：a、b、c、y、vp、vn。|
|`$XOR_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|带电源引脚的四输入异或门：a、b、c、d、y、vp、vn。|
|`$XNOR_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|带电源引脚的二输入同或门：a、b、y、vp、vn。|
|`$XNOR_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|带电源引脚的三输入同或门：a、b、c、y、vp、vn。|
|`$XNOR_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|带电源引脚的四输入同或门：a、b、c、d、y、vp、vn。|
|`$BUFFER_SUPPLY`|`a` `y` `vp` `vn`|带电源引脚的缓冲器：a、y、vp、vn。|
|`$INVERTER_SUPPLY`|`a` `y` `vp` `vn`|带电源引脚的反相器：a、y、vp、vn。|
|`$DELAY_CELL_SUPPLY`|`a` `y` `vp` `vn`|带电源引脚的延迟单元：a、y、vp、vn。|
|`$D_FLIP_FLOP_SUPPLY`|`d` `ck` `q` `qb` `vp` `vn`|带电源引脚的 D 触发器：d、ck、q、qb、vp、vn。|
|`$D_FLIP_FLOP_Q_ONLY_SUPPLY`|`d` `ck` `q` `vp` `vn`|带电源引脚的单输出（仅 Q）D 触发器：d、ck、q、vp、vn。|
|`$D_FLIP_FLOP_WITH_RESET_SUPPLY`|`d` `ck` `rst` `q` `qb` `vp` `vn`|带复位端与电源引脚（vp 与 vn 在 x = 0 处上下对齐）的 D 触发器：d、ck、rst、q、qb、vp、vn。|
|**rnl-std-signal-flow**|||
|`$ADDER_SYMBOL`|`a` (`in1`) `b` (`in2`) `y` (`out`)|`$ADDER` 所用的符号：输入引线 a（in1）与 b（in2）末端带箭头的求和圆圈，输出为 y（out）。|
|`$SUBTRACTOR_SYMBOL`|`a` (`in1`) `b` (`in2`) `y` (`out`)|`$SUBTRACTOR` 所用的符号：圆心标有 +、负输入端 b（in2）外侧标有 - 的求和圆圈，输入为 a（in1）与 b（in2），输出为 y（out）。|
|`$MULTIPLIER_SYMBOL`|`a` (`in1`) `b` (`in2`) `y` (`out`)|`$MULTIPLIER` 所用的符号：输入引线 a（in1）与 b（in2）末端带箭头的乘法圆圈，输出为 y（out）。|
|`$INTEGRATOR_SYMBOL`|`a` (`in`) `y` (`out`)|`$INTEGRATOR` 所用的符号：标有 1/s 的方框，输入为 a（in），输出为 y（out）。|
|`$DISCRETE_TIME_INTEGRATOR_SYMBOL`|`a` (`in`) `y` (`out`)|`$DISCRETE_TIME_INTEGRATOR` 所用的符号：标有 z 域传递函数的方框，输入为 a（in），输出为 y（out）。|
|`$QUANTIZER_SYMBOL`|`a` (`in`) `y` (`out`)|`$QUANTIZER` 所用的符号：内绘阶梯折线的方框，输入为 a（in），输出为 y（out）。|
|`$UNIT_DELAY_SYMBOL`|`a` (`in`) `y` (`out`)|`$UNIT_DELAY` 所用的符号：标有 z⁻¹ 的方框，输入为 a（in），输出为 y（out）。|
|**rnl-std-45deg**|||
|`$RESISTOR_45`|`p` `n`|45 度电阻。|
|`$CAPACITOR_45`|`p` `n`|45 度电容。|
|`$INDUCTOR_45`|`p` `n`|45 度电感（亦可用作对角变压器或巴伦绕组）。|
|`$SWITCH_45`|`p` `n`|45 度断开开关。|
|`$VOLTAGE_SOURCE_45`|`p` `n`|45 度独立电压源，靠近 p 的 + 号与靠近 n 的 - 号保持正立。|
|`$CURRENT_SOURCE_45`|`p` `n`|45 度独立电流源，内部箭头由 p 指向 n。|
|`$DIODE_45`|`a` `k`|45 度二极管。|
|`$ZENER_DIODE_45`|`a` `k`|45 度齐纳（稳压）二极管。|
|`$SCHOTTKY_DIODE_45`|`a` `k`|45 度肖特基二极管。|
|`$LED_45`|`a` `k`|45 度发光二极管（LED）。|
|`$NMOS_45`|`d` `g` `s`|45 度三引脚 NMOS 晶体管。|
|`$NMOS_4TERM_45`|`d` `g` `s` `b`|45 度四引脚 NMOS 晶体管，衬底引脚位于原点 (0, 0)。|
|`$PMOS_45`|`d` `g` `s`|45 度三引脚 PMOS 晶体管。|
|`$PMOS_4TERM_45`|`d` `g` `s` `b`|45 度四引脚 PMOS 晶体管，衬底引脚位于原点 (0, 0)。|
|`$NPN_45`|`c` `b` `e`|45 度 NPN 双极型晶体管。|
|`$PNP_45`|`c` `b` `e`|45 度 PNP 双极型晶体管。|
|`$ARROW_45_SYMBOL`|—|指向 -45 度方向（右下方，即 45 度符号在 R90 朝向下 p->n 或 a->k 的方向）的实心箭头符号，配合 R0/R90/R180/R270 可转向全部四条对角线方向。|

### 集合与样式

|名称|所属包|含义|
|---|---|---|
|`$GROUND_NET_SET`|rnl-std-basic|地网络集合，默认含网络 0，可通过 ADD-TO 与 REMOVE-FROM 增删；rnl-std-fill 会为其中未接线的引脚补上接地短桩，并令这些网络保持不布线。接地符号本身的外观与该集合无关。|
|`$SUPPLY_NET_SET`|rnl-std-basic|电源网络集合，默认为空，可通过 ADD-TO 与 REMOVE-FROM 增删。|
|`$BIAS_NET_SET`|rnl-std-basic|偏置网络集合，默认为空，可通过 ADD-TO 与 REMOVE-FROM 增删。|
|`$PORT_FILL_SET`|rnl-std-basic|由 rnl-std-fill 自动补上端口标签的端口网络集合：初始包含每一层的全部端口网络（含电源和地端口），从中扣除已显式放置 `$IO_PORT` 的网络以及已套用 `$PORT_LIST` 的层级中的网络。|
|`$GROUND_FILL_SET`|rnl-std-fill|地网络上待由自动补位挂上接地短桩的未接线引脚集合。|
|`$SUPPLY_FILL_SET`|rnl-std-fill|电源网络上待由自动补位挂上电源短桩的未接线引脚集合。|
|`$BIAS_FILL_SET`|rnl-std-fill|偏置网络上待由自动补位挂上偏置短桩的未接线引脚集合。|
|`$LABEL_FILL_SET`|rnl-std-fill|其余网络上待由自动补位挂上网络标签短桩的未接线引脚集合；总线引脚只计一个。|
|`$base`|rnl-std-style|所有对象的基础样式：按 Analog Canvas 的默认视觉风格绘制线条、文字与填充。|
|`$bus`|rnl-std-style|总线样式：加粗线条。|
|`$signal`|rnl-std-style|信号导线样式：蓝色加粗线条。|
|`$note`|rnl-std-style|注释说明样式：灰色小字号文字。|
|`$grey`|rnl-std-style|线条与文字均设为灰色的样式。|

<!-- lists:end -->
