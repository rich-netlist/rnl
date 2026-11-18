<!-- translated from en/annex-b.md sha256:f5f5cbcacdf4 -->
## 付録 B 標準文（参考情報）

標準文とは、標準形式の文のトークン間に自由テキストを加えて英語の文のように読めるようにしたものである。自由テキストは意味を変えず（3.3）、語句の規則は4.5にある。表中で文頭の `…` はパイプ内の先行セグメントを、`{…}` や末尾の `…` は省略された内容を表す。

### B.1 配置と向き

| キーワード | 場合 | 例文 |
|---|---|---|
| PLACE | 2軸 | `#M1 is at @PLACE(10, 20)` |
| | 1軸 | `the x of #M1 is at @PLACE(x=10)` |
| | エディタによる記述 | `the x of #X2 is at @PLACE(x=3, priority="high")` |
| | パイプ内 | `… => it is at @PLACE(40, 0)` |
| ORIENT | 回転 | `#M2 @ORIENT("R90")` |
| | 鏡像反転 | `#M2 @ORIENT("MX"), i.e., flipped upside down` |
| | | `#M2 @ORIENT("MY"), i.e., flipped left-right` |
| | | `#M3 @ORIENT("MXR90"), i.e., flipped upside down, then rotated 90°` |
| | | `#M3 @ORIENT("MYR90"), i.e., flipped left-right, then rotated 90°` |
| | 2つのいずれか | `#M5 @ORIENT(["R0", "MY"]), i.e., not turned or flipped left-right` |
| | パイプ内 | `… => they @ORIENT("R90")` |
| ORIENT-RELATION | 2つのオブジェクト | `from #M1 to #M2, they have @ORIENT-RELATION("MY"), i.e., flipped left-right` |
| | 2つのいずれか | `from #M3 to #M4, they have @ORIENT-RELATION(["R0", "MY"]), i.e., the same or flipped left-right` |
| | 同じ向き | `from #M5 to {#M7 #M8}, they have @ORIENT-RELATION("R0"), i.e., the same orientation` |
| LAYER | | `#X is on @LAYER("bottom")` |
| | パイプ内 | `… => they are on @LAYER("top")` |
| GROUP | | `{#M1 #M2 #M5} as a @GROUP` |
| | 名前付き | `%s1_set = {#M1 #M2 #M5} as a @GROUP` |
| | 間隔 | `{#M3 #M4} as a @GROUP(spacing_x=-1), i.e., one grid unit tighter in x` |

### B.2 レイアウト制約

| キーワード | 場合 | 例文 |
|---|---|---|
| ALIGN | 同じy | `{#M1 #M2} @ALIGN("y"), i.e., same y` |
| | 同じx | `{#M1 #C1} @ALIGN("x"), i.e., same x` |
| | 低優先度 | `{#M7 #M5} @ALIGN("y", priority="low"), i.e., same y` |
| | パイプ内 | `… => they @ALIGN("y")` |
| ORDER | 方向付き | `{#M1 #M2 #M3} are in @ORDER("left-to-right")` |
| | パイプ内 | `… => they are in @ORDER("bottom-to-top")` |
| DISTRIBUTE | | `{#R1 #R2 #R3} are in @DISTRIBUTE("left-to-right"), i.e., in equal steps` |
| | パイプ内 | `… => they are in @DISTRIBUTE("bottom-to-top"), i.e., in equal steps` |
| DISTANCE | 方向付き | `from #M1 to #M2, they have @DISTANCE("bottom-to-top", min=4)` |
| | セットから | `from {#M6 #M7 #CL} to %pout, they have @DISTANCE("left-to-right", min=2)` |
| TABLE | | `@TABLE layout: {{#M1 #M2 #M3} {#M4 {#M5 #M6} #M7}}` |
| | 転置 | `@TABLE(transpose=true) layout: {{#M1 #M2} {#M3 #M4}}` |
| PRIORITY | | `%c1 has @PRIORITY("disabled")` |
| | パイプ内 | `… => they have @PRIORITY("low")` |
| SOLVER-CONFIG | | `@SOLVER-CONFIG(spacing_x=2, spacing_y=1)` |

- `the x of`（または `the y of`）は1軸を与えるPLACEの前でのみ書かれる（B.1）。ALIGNはその代わりに補注 `, i.e., same x` をとる。
- xとは各メンバーの位置のxである：インスタンスではPLACEが設定する点（そのシンボルの原点）であり、外形の中心ではない。セットはその各メンバーを表す（14.1）。

### B.3 外観

| キーワード | 場合 | 例文 |
|---|---|---|
| SET-SYMBOL | 1つのオブジェクト | `#M1 @SET-SYMBOL to $NMOS` |
| | セット | `{#M1 #M2} @SET-SYMBOL to $NMOS` |
| | サブサーキットのブロックシンボル | `this subcircuit # @SET-SYMBOL to $NAND2_SYM` |
| | パイプ内 | `… => they @SET-SYMBOL to $PMOS` |
| SET-STYLE | | `{#CC #RZ} @SET-STYLE to $orange` |
| | パイプ内 | `… => they @SET-STYLE to $grey` |
| DEFINE-STYLE | | `$orange = @DEFINE-STYLE(linecolor="#e07000", textcolor="#e07000") {}` |
| | 別スタイルの統合 | `$wide = @DEFINE-STYLE(linewidth=2) based on {$orange}` |

### B.4 シンボルと図形

| キーワード | 場合 | 例文 |
|---|---|---|
| DEFINE-SYMBOL | | `$TRI = @DEFINE-SYMBOL {…}` |
| PIN | | `at (-4, 0) there is a @PIN(terminal="in", direction="left")` |
| LINE | | `through {(0, 0) (4, 0)} runs a @LINE` |
| ARC | | `through {(0, 2) (2, 0) (0, -2)} runs an @ARC` |
| BEZIER | | `the control points {(0, 0) (1, 1) (2, 1) (3, 0)} make a @BEZIER` |
| CIRCLE | | `centered at (0, 0), a @CIRCLE(radius=1)` |
| RECTANGLE | 中心とサイズ | `centered at (0, 0), a @RECTANGLE(width=4, height=2)` |
| | 2つの角 | `with corners {(0, 0) (4, 2)}, a @RECTANGLE` |
| ELLIPSE | 中心とサイズ | `centered at (0, 0), an @ELLIPSE(width=4, height=2)` |
| | 外接枠の角 | `in the box with corners {(0, 0) (4, 2)}, an @ELLIPSE` |
| TEXT | シンボル内 | `on ? there is a @TEXT("${name}", key="name") => it is at @PLACE(-2, 2.5)` |
| ATTACH | シンボル内 | `$ARROW is @ATTACH to (0, -1.5) => …` |

- 注釈としてシーン内に書かれた図形（13.4）も同様に読まれる。

### B.5 配線とテキスト

| キーワード | 場合 | 例文 |
|---|---|---|
| WIRE | | `net &n1, through {#R1:2 %j #R2:1}, has a @WIRE` |
| | バス | `net &D<3:0>, through {%p1 %p2}, has a @WIRE, which is a bus` |
| | 配線モード | `net &in, through {#VIN:1 #R1:1}, has a @WIRE(mode="vertical-horizontal")` |
| ROUTE-NET | | `net &n1 is drawn with @ROUTE-NET` |
| | セット | `nets {&a &b} are drawn with @ROUTE-NET` |
| | パイプ内 | `… => it is drawn with @ROUTE-NET` |
| ROUTE-OBJECT | | `the pins of {#M1 #M2 #M5} are joined with @ROUTE-OBJECT` |
| UNROUTE-NET | | `net &vdd is left out with @UNROUTE-NET(priority="high")` |
| UNROUTE-OBJECT | | `the pins of #M3 are left out with @UNROUTE-OBJECT` |
| TEXT | オブジェクト上 | `on #M1 there is a @TEXT("W/L = ${w}/${l}", key="wl", dy=-3)` |
| | 点上 | `at (_, _) there is a @TEXT("RC low-pass") => named %title` |
| | パイプ内 | `… => on them there is a @TEXT("${w}", key="size")` |

- `%j = (_, _)` のように点に名前を付ける場合、語は追加されない。

### B.6 定義と適用

| キーワード | 場合 | 例文 |
|---|---|---|
| DEFINE | | `$PAIR = a definition, @DEFINE(roles=["a", "b"], parameters=[["gap", 2]]) {…}` |
| AS | 3つのロール | `#M1(left), #M2(right) and #M5(tail) are treated @AS $DIFFERENTIAL_PAIR_N` |
| | セットをとるロール | `{#M5 #M7 #M8} are treated @AS $MATCHED` |
| | 1つのオブジェクト | `#M3 is treated @AS $DIODE_CONNECTED` |
| | 優先度付き | `#M1(left), #M2(right) and %ax(axis) are treated @AS(priority="high") $MIRRORED_LAYOUT` |
| | 定義のパラメータ | `{#R1 #R2} are treated @AS(gap=6) $SPACED` |
| | ロール内の適用オブジェクト | `%dp(pair) and %cm(load) are treated @AS $OTA` |
| | 名前付き | `%dp = #M1(left), #M2(right) and #M5(tail) treated @AS $DIFFERENTIAL_PAIR_N` |
| | パイプ内で名前付き、他オブジェクトとともに | `#M1(left), #M2(right) and #M5(tail) are treated @AS $DIFFERENTIAL_PAIR_N => named %dp => it and %cm are treated @AS $OTA` |
| ATTACH | ネット上 | `$GROUND is @ATTACH to &vss` |
| | 端子上 | `$NO_CONNECT is @ATTACH to #M1:4` |
| | 点上 | `$JUNCTION is @ATTACH to %j` |
| | 2つのロール | `$DIFF_PORT is @ATTACH to &inp(a) and &inn(b)` |
| | 他のセグメントが後続、名前付き | `$GROUND is @ATTACH to &vss => it is at @PLACE(24, 38) => named %g0` |
| | 複数のオブジェクトの各々に | `$SIZE_LABEL is @ATTACH to {#M1 #M2 #M3}` |
| | セット上 | `$GROUND_STUB is @ATTACH to $GROUND_FILL_SET` |
| | パイプ内の自動補完 | `among the instances at each level in # and below, @SELECT-INSTANCE(recursive=true) => among the pins of them, @SELECT-PIN(net="vss", wired=false) => $GROUND_STUB is @ATTACH(gap=3) to them` |

- ASは、回路構造、配置、または `$RC_STAGE` のような局所的描画法など、オブジェクトがまとまってそのものを作る場合に用いられ、ATTACHはグラウンド、ポートタグ、未接続マーク、接続点の黒丸、各インスタンスのラベル、または自動補完のように、そのものがオブジェクト上に追加される場合に用いられる。2つのキーワードは同じことを行い、定義の記述位置のみが異なる（9.4）。
- `gap` は `$GROUND_STUB` のパラメータであり、ピンからグラウンドシンボルまでの最小下方向距離（デフォルトは1）である。ロール名 `left` や `axis` と同様に定義内で与えられた名前であり、標準語ではない。

### B.7 選択

選択は候補からの選択として `among <candidates>, @SELECT-…(conditions)` のように書かれる。候補の言い回しはスコープの種別と選択対象の種別による：

| スコープ | 選択対象 | 候補 |
|---|---|---|
| `#` | 任意の種別 | `the <kind> in # only` |
| `#`、下位レベルに到達する文において（10.1） | 任意の種別 | `the <kind> at each level in # and below` |
| 選択対象の種別のセット（絞り込み） | その種別 | `{…}` |
| ネット | インスタンス | `the instances connected to &en` |
| ネット | 端子 | `the terminals connected to &en` |
| ネット | 配線 | `the wires on &en` |
| インスタンス | 端子 | `the terminals of #R1` |
| インスタンス、バンドル、シンボルを持つ適用オブジェクト | ピン | `the pins of #M1` |
| 端子 | ピン | `the pins of #X1:vdd` |
| ネット | ピン | `the pins on &en` |
| パイプの結果 | ピン | `the pins of them` |
| インスタンス | ネット | `the nets connected to #M1` |
| 端子 | インスタンス | `the instances owning #M1:g` |
| 端子 | ネット | `the nets connected to #M1:g` |
| 配線 | ネット | `the nets of %w13` |
| インスタンス、端子、配線、ネット、図形 | ラベル | `the labels attached to #M1` |
| 適用オブジェクト（メンバーに適用オブジェクトなし） | 任意の種別 | `the members of %dp` |
| 適用オブジェクト（メンバーに適用オブジェクトあり、または不明） | 任意の種別 | `all members of %st1 at any depth` |
| 適用オブジェクト | 適用オブジェクト（SELECT-USAGE） | `%st1 itself and the usages inside it` |
| 任意のオブジェクト | それを含む適用オブジェクト（SELECT-ANCESTOR-USAGE） | `the usages containing #M1` |
| 複数の種別のセット（種別不明） | | 代わりに `for %mixed_set, @SELECT-…` と記述 |

- `<kind>` はキーワードに従う：instances、terminals、pins、nets、wires、labels、annotations、usages。
- パイプの途中では候補内のスコープは `=> among all members of them at any depth,` のように `them` と書かれる。前に選択された適用オブジェクトがメンバーに適用オブジェクトを持つかは文を書く時点では不明なため、`at any depth` が使われる。
- 適用オブジェクトのメンバーには `at any level` ではなく `at any depth` という言い回しを用いる：levelとは `at each level in # and below` におけるように回路のレベルを指すためである。
- パッケージ内のルールはそのUSE-PACKAGEのレベルに書かれたものとして数えられる（10.4）。`recursive=true` を持つ場合、パッケージは `at each level in # and below` の代わりに `at every level that uses this package, #` と書く。

| キーワード | 場合 | 例文 |
|---|---|---|
| SELECT-INSTANCE | このレベルと下位 | `among the instances at each level in # and below, @SELECT-INSTANCE(model="nch", recursive=true) => they @SET-SYMBOL to $NMOS` |
| | このレベルのみ | `among the instances in # only, @SELECT-INSTANCE(type="pmos") => …` |
| | 絞り込み | `among {#M1 #M2 #R1}, @SELECT-INSTANCE(name="M*") => they @ORIENT("MY")` |
| | ネット上 | `among the instances connected to &en, @SELECT-INSTANCE => they @SET-STYLE to $grey` |
| | 端子を所有 | `among the instances owning #M1:g, @SELECT-INSTANCE => they are on @LAYER("top")` |
| | 適用オブジェクトのメンバー | `among all members of %st1 at any depth, @SELECT-INSTANCE(role="pair")` |
| | 入れ子の適用オブジェクトを持たない適用オブジェクトのメンバー | `among the members of %dp, @SELECT-INSTANCE` |
| | パイプの途中 | `among the usages at each level in # and below, @SELECT-USAGE(recursive=true) of $DIFFERENTIAL_PAIR_N => among all members of them at any depth, @SELECT-INSTANCE => they @SET-STYLE to $signal` |
| SELECT-TERMINAL | このレベルと下位 | `among the terminals at each level in # and below, @SELECT-TERMINAL(net="vss", recursive=true) => they have @ASSERT-TERMINAL(same=["net"])` |
| | インスタンスのもの | `among the terminals of #R1, @SELECT-TERMINAL => on them there is a @TEXT("${net}")` |
| | ネット上 | `among the terminals connected to $GROUND_NET_SET, @SELECT-TERMINAL => among the pins of them, @SELECT-PIN(wired=false) => $GROUND_STUB is @ATTACH to them` |
| SELECT-PIN | レベル内 | `among the pins in # only, @SELECT-PIN(wired=false) => on them there is a @TEXT("${net}", key="netname")` |
| | 端子群のもの | `among the pins of {#M1:s #M2:s}, @SELECT-PIN(wired=false) => $GROUND_STUB is @ATTACH to them` |
| | 端子のもの | `among the pins of #X1:vdd, @SELECT-PIN => they @SET-STYLE to $red` |
| SELECT-NET | このレベルと下位 | `among the nets at each level in # and below, @SELECT-NET(port=true, recursive=true) => $IO_PORT is @ATTACH to them` |
| | インスタンスのもの | `among the nets connected to #M1, @SELECT-NET => named %m1_net_set` |
| | 端子のもの | `among the nets connected to #M1:g, @SELECT-NET => they are drawn with @ROUTE-NET` |
| | 配線のもの | `among the nets of %w13, @SELECT-NET => they are drawn with @ROUTE-NET` |
| | 絞り込み | `among {&a &b}, @SELECT-NET(wired=false) => they are drawn with @ROUTE-NET` |
| SELECT-WIRE | このレベルと下位 | `among the wires at each level in # and below, @SELECT-WIRE(bus=true, recursive=true) => they @SET-STYLE to $bus` |
| | ネット上 | `among the wires on &clk, @SELECT-WIRE => they @SET-STYLE to $red` |
| SELECT-LABEL | このレベルと下位 | `among the labels at each level in # and below, @SELECT-LABEL(key="wl", recursive=true) => they @SET-STYLE to $grey` |
| | オブジェクトに取り付けられたもの | `among the labels attached to #M1, @SELECT-LABEL => they @SET-STYLE to $grey` |
| SELECT-ANNOTATION | このレベルと下位 | `among the annotations at each level in # and below, @SELECT-ANNOTATION(recursive=true) => they @SET-STYLE to $base` |
| | 絞り込み | `among {%n1 %f1 #M1}, @SELECT-ANNOTATION` |
| SELECT-USAGE | このレベルと下位 | `among the usages at each level in # and below, @SELECT-USAGE(recursive=true) of $DIFFERENTIAL_PAIR_N => they have @PRIORITY("disabled")` |
| | 定義なし | `among the usages at each level in # and below, @SELECT-USAGE(recursive=true) => they @SET-STYLE to $base` |
| | ある適用オブジェクトとその内部の適用オブジェクト | `among %st1 itself and the usages inside it, @SELECT-USAGE of $DIFFERENTIAL_PAIR_N` |
| SELECT-ANCESTOR-USAGE | | `among the usages containing #M1, @SELECT-ANCESTOR-USAGE of $DIFFERENTIAL_PAIR_N => named %dp_set` |
| 任意の選択 | スコープの種別不明 | `for %mixed_set, @SELECT-TERMINAL => …` |

### B.8 セットとその他のキーワード

| キーワード | 場合 | 例文 |
|---|---|---|
| ADD-TO | | `&vdd @ADD-TO $SUPPLY_NET_SET` |
| | パイプ内 | `… => they @ADD-TO $SUPPLY_NET_SET` |
| REMOVE-FROM | | `&agnd @REMOVE-FROM $GROUND_NET_SET` |
| UNION | | `%u_set = %a_set and %b_set @UNION` |
| BUNDLE | | `#XI<3:0> is drawn as a @BUNDLE => named %xi` |
| CURRENT | in | `#M5:1 has @CURRENT("in")` |
| | out | `#:iout has @CURRENT("out")` |
| | unspecified | `#M5:2 has @CURRENT("unspecified")` |
| ASSERT-INSTANCE | | `{#M1 #M2} have @ASSERT-INSTANCE(same=["model", "w", "l"])` |
| ASSERT-TERMINAL | | `{#M1:s #M2:s} have @ASSERT-TERMINAL(same=["net"])` |
| ASSERT-NET | | `{&inp &inn} have @ASSERT-NET(port=true)` |
| ネットなしのWIRE | | `between {%sub1::y %int1::a}, on no net &, there is a @WIRE` |
| ANNOTATION | | `at (40, 20) there is an @ANNOTATION(rotate=30) of $ARROW_SYMBOL` |
| ASSERT-USAGE | | `%dp1 is, by @ASSERT-USAGE, a usage of $DIFFERENTIAL_PAIR_N` |
| ASSERT-IN | | `{&vss} is, by @ASSERT-IN, in $GROUND_NET_SET` |
| ASSERT-TOPOLOGY | 直列 | `{#R1 #R2 #R3} are connected as @ASSERT-TOPOLOGY("series")` |
| | 並列 | `{#M3 #M4} are connected as @ASSERT-TOPOLOGY("parallel")` |
| | スター | `{#L1 #C1 #L2} are connected as @ASSERT-TOPOLOGY("star")` |
| | ループ | `{#C1 #L1 #C2} are connected as @ASSERT-TOPOLOGY("loop")` |

### B.9 ヘッダーレコードとパッケージ

語は追加されない。ヘッダーレコードは標準の綴りで読まれ、別名を用いない（16.5）。

| キーワード | 場合 | 例文 |
|---|---|---|
| VERSION | | `@VERSION("0.4")` |
| ALIAS | | `@ALIAS("@位于", "@PLACE")` |
| USE-ALIASES | | `@USE-ALIASES(name="alias-zh", version="0.4")` |
| DEVICE | | `@DEVICE(…)` |
| PACKAGE | | `@PACKAGE(name="acme-style", version="1.2")` |
| USE-PACKAGE | ヘッダーレコード内；パッケージファイルのヘッダーレコード内の参照も同様 | `@USE-PACKAGE(name="rnl-std-all", version="0.4")` |
| | レベル内 | `@USE-PACKAGE(name="acme-style", version="1.2")` |
