<!-- translated from en/annex-b.md sha256:f5f5cbcacdf4 -->
## 附錄 B 標準例句（參考性）

標準例句是在標準寫法的敘述中於記號之間加入自由文字，使敘述讀起來像英文句子。自由文字不改變含義（3.3）；用詞規則見 4.5。在下列各表中，位於句首的 `…` 代表管線中排在它前面的各段，而 `{…}` 或位於句尾的 `…` 則代表省略的內容。

### B.1 位置與朝向

| 關鍵字 | 情況 | 例句 |
|---|---|---|
| PLACE | 雙軸 | `#M1 is at @PLACE(10, 20)` |
| | 單軸 | `the x of #M1 is at @PLACE(x=10)` |
| | 編輯器寫入 | `the x of #X2 is at @PLACE(x=3, priority="high")` |
| | 管線中 | `… => it is at @PLACE(40, 0)` |
| ORIENT | 旋轉 | `#M2 @ORIENT("R90")` |
| | 鏡像 | `#M2 @ORIENT("MX"), i.e., flipped upside down` |
| | | `#M2 @ORIENT("MY"), i.e., flipped left-right` |
| | | `#M3 @ORIENT("MXR90"), i.e., flipped upside down, then rotated 90°` |
| | | `#M3 @ORIENT("MYR90"), i.e., flipped left-right, then rotated 90°` |
| | 二選一 | `#M5 @ORIENT(["R0", "MY"]), i.e., not turned or flipped left-right` |
| | 管線中 | `… => they @ORIENT("R90")` |
| ORIENT-RELATION | 兩個物件 | `from #M1 to #M2, they have @ORIENT-RELATION("MY"), i.e., flipped left-right` |
| | 二選一 | `from #M3 to #M4, they have @ORIENT-RELATION(["R0", "MY"]), i.e., the same or flipped left-right` |
| | 相同朝向 | `from #M5 to {#M7 #M8}, they have @ORIENT-RELATION("R0"), i.e., the same orientation` |
| LAYER | | `#X is on @LAYER("bottom")` |
| | 管線中 | `… => they are on @LAYER("top")` |
| GROUP | | `{#M1 #M2 #M5} as a @GROUP` |
| | 具名 | `%s1_set = {#M1 #M2 #M5} as a @GROUP` |
| | 間距 | `{#M3 #M4} as a @GROUP(spacing_x=-1), i.e., one grid unit tighter in x` |

### B.2 佈局約束

| 關鍵字 | 情況 | 例句 |
|---|---|---|
| ALIGN | 相同 y | `{#M1 #M2} @ALIGN("y"), i.e., same y` |
| | 相同 x | `{#M1 #C1} @ALIGN("x"), i.e., same x` |
| | 低優先順序 | `{#M7 #M5} @ALIGN("y", priority="low"), i.e., same y` |
| | 管線中 | `… => they @ALIGN("y")` |
| ORDER | 帶方向 | `{#M1 #M2 #M3} are in @ORDER("left-to-right")` |
| | 管線中 | `… => they are in @ORDER("bottom-to-top")` |
| DISTRIBUTE | | `{#R1 #R2 #R3} are in @DISTRIBUTE("left-to-right"), i.e., in equal steps` |
| | 管線中 | `… => they are in @DISTRIBUTE("bottom-to-top"), i.e., in equal steps` |
| DISTANCE | 帶方向 | `from #M1 to #M2, they have @DISTANCE("bottom-to-top", min=4)` |
| | 起於集合 | `from {#M6 #M7 #CL} to %pout, they have @DISTANCE("left-to-right", min=2)` |
| TABLE | | `@TABLE layout: {{#M1 #M2 #M3} {#M4 {#M5 #M6} #M7}}` |
| | 轉置 | `@TABLE(transpose=true) layout: {{#M1 #M2} {#M3 #M4}}` |
| PRIORITY | | `%c1 has @PRIORITY("disabled")` |
| | 管線中 | `… => they have @PRIORITY("low")` |
| SOLVER-CONFIG | | `@SOLVER-CONFIG(spacing_x=2, spacing_y=1)` |

- `the x of`（或 `the y of`）僅寫在指定單一座標軸的 PLACE 之前（B.1）；ALIGN 則加上註解 `, i.e., same x`。
- 所指的 x 為每個成員的位置之 x：對實例而言是 PLACE 所擺放的點（其符號原點），而非其輪廓中心；集合代表其每個成員（14.1）。

### B.3 外觀

| 關鍵字 | 情況 | 例句 |
|---|---|---|
| SET-SYMBOL | 單一物件 | `#M1 @SET-SYMBOL to $NMOS` |
| | 集合 | `{#M1 #M2} @SET-SYMBOL to $NMOS` |
| | 子電路的方塊符號 | `this subcircuit # @SET-SYMBOL to $NAND2_SYM` |
| | 管線中 | `… => they @SET-SYMBOL to $PMOS` |
| SET-STYLE | | `{#CC #RZ} @SET-STYLE to $orange` |
| | 管線中 | `… => they @SET-STYLE to $grey` |
| DEFINE-STYLE | | `$orange = @DEFINE-STYLE(linecolor="#e07000", textcolor="#e07000") {}` |
| | 合併另一樣式 | `$wide = @DEFINE-STYLE(linewidth=2) based on {$orange}` |

### B.4 符號與圖形

| 關鍵字 | 情況 | 例句 |
|---|---|---|
| DEFINE-SYMBOL | | `$TRI = @DEFINE-SYMBOL {…}` |
| PIN | | `at (-4, 0) there is a @PIN(terminal="in", direction="left")` |
| LINE | | `through {(0, 0) (4, 0)} runs a @LINE` |
| ARC | | `through {(0, 2) (2, 0) (0, -2)} runs an @ARC` |
| BEZIER | | `the control points {(0, 0) (1, 1) (2, 1) (3, 0)} make a @BEZIER` |
| CIRCLE | | `centered at (0, 0), a @CIRCLE(radius=1)` |
| RECTANGLE | 中心與尺寸 | `centered at (0, 0), a @RECTANGLE(width=4, height=2)` |
| | 兩角點 | `with corners {(0, 0) (4, 2)}, a @RECTANGLE` |
| ELLIPSE | 中心與尺寸 | `centered at (0, 0), an @ELLIPSE(width=4, height=2)` |
| | 外接框兩角點 | `in the box with corners {(0, 0) (4, 2)}, an @ELLIPSE` |
| TEXT | 在符號中 | `on ? there is a @TEXT("${name}", key="name") => it is at @PLACE(-2, 2.5)` |
| ATTACH | 在符號中 | `$ARROW is @ATTACH to (0, -1.5) => …` |

- 寫在場景中作為標註的圖形（13.4），讀法相同。

### B.5 連線與文字

| 關鍵字 | 情況 | 例句 |
|---|---|---|
| WIRE | | `net &n1, through {#R1:2 %j #R2:1}, has a @WIRE` |
| | 匯流排 | `net &D<3:0>, through {%p1 %p2}, has a @WIRE, which is a bus` |
| | 走線模式 | `net &in, through {#VIN:1 #R1:1}, has a @WIRE(mode="vertical-horizontal")` |
| ROUTE-NET | | `net &n1 is drawn with @ROUTE-NET` |
| | 集合 | `nets {&a &b} are drawn with @ROUTE-NET` |
| | 管線中 | `… => it is drawn with @ROUTE-NET` |
| ROUTE-OBJECT | | `the pins of {#M1 #M2 #M5} are joined with @ROUTE-OBJECT` |
| UNROUTE-NET | | `net &vdd is left out with @UNROUTE-NET(priority="high")` |
| UNROUTE-OBJECT | | `the pins of #M3 are left out with @UNROUTE-OBJECT` |
| TEXT | 附著於物件 | `on #M1 there is a @TEXT("W/L = ${w}/${l}", key="wl", dy=-3)` |
| | 位於某點 | `at (_, _) there is a @TEXT("RC low-pass") => named %title` |
| | 管線中 | `… => on them there is a @TEXT("${w}", key="size")` |

- 為點命名（如 `%j = (_, _)`）不加任何多餘詞語。

### B.6 定義與套用

| 關鍵字 | 情況 | 例句 |
|---|---|---|
| DEFINE | | `$PAIR = a definition, @DEFINE(roles=["a", "b"], parameters=[["gap", 2]]) {…}` |
| AS | 三個角色 | `#M1(left), #M2(right) and #M5(tail) are treated @AS $DIFFERENTIAL_PAIR_N` |
| | 接收集合的角色 | `{#M5 #M7 #M8} are treated @AS $MATCHED` |
| | 單一物件 | `#M3 is treated @AS $DIODE_CONNECTED` |
| | 帶優先順序 | `#M1(left), #M2(right) and %ax(axis) are treated @AS(priority="high") $MIRRORED_LAYOUT` |
| | 定義的參數 | `{#R1 #R2} are treated @AS(gap=6) $SPACED` |
| | 角色中傳入用例 | `%dp(pair) and %cm(load) are treated @AS $OTA` |
| | 具名 | `%dp = #M1(left), #M2(right) and #M5(tail) treated @AS $DIFFERENTIAL_PAIR_N` |
| | 在管線中命名並搭配另一物件 | `#M1(left), #M2(right) and #M5(tail) are treated @AS $DIFFERENTIAL_PAIR_N => named %dp => it and %cm are treated @AS $OTA` |
| ATTACH | 在網路上 | `$GROUND is @ATTACH to &vss` |
| | 在端子上 | `$NO_CONNECT is @ATTACH to #M1:4` |
| | 在點上 | `$JUNCTION is @ATTACH to %j` |
| | 兩個角色 | `$DIFF_PORT is @ATTACH to &inp(a) and &inn(b)` |
| | 後接其他段並命名 | `$GROUND is @ATTACH to &vss => it is at @PLACE(24, 38) => named %g0` |
| | 逐一作用於多個物件 | `$SIZE_LABEL is @ATTACH to {#M1 #M2 #M3}` |
| | 在集合上 | `$GROUND_STUB is @ATTACH to $GROUND_FILL_SET` |
| | 管線中的自動補齊 | `among the instances at each level in # and below, @SELECT-INSTANCE(recursive=true) => among the pins of them, @SELECT-PIN(net="vss", wired=false) => $GROUND_STUB is @ATTACH(gap=3) to them` |

- 當各物件共同構成該事物時（如電路結構、佈局方式或 `$RC_STAGE` 等區域繪圖方式）使用 AS；當該事物加在物件上時（如接地、連接埠標記、未連接標記、交接圓點、每個實例的標籤或自動補齊）使用 ATTACH。這兩個關鍵字做的事完全相同，僅定義所在的槽位不同（9.4）。
- `gap` 是 `$GROUND_STUB` 的參數，表示從接腳往下到接地符號的最短距離（預設為 1）。如同角色名稱 `left` 與 `axis`，它是定義中給出的名稱，而非標準詞彙。

### B.7 篩選

篩選寫成從候選項目中挑選出來的形式：`among <candidates>, @SELECT-…(conditions)`。候選項目的措辭取決於作用域的種類與所選取的種類：

| 作用域 | 選取對象 | 候選項目 |
|---|---|---|
| `#` | 任意種類 | `the <kind> in # only` |
| `#`，在觸及下方層級的敘述中（10.1） | 任意種類 | `the <kind> at each level in # and below` |
| 所選種類的集合（過濾條件） | 該種類 | `{…}` |
| 網路 | 實例 | `the instances connected to &en` |
| 網路 | 端子 | `the terminals connected to &en` |
| 網路 | 連線 | `the wires on &en` |
| 實例 | 端子 | `the terminals of #R1` |
| 實例、疊畫、帶有符號的用例 | 接腳 | `the pins of #M1` |
| 端子 | 接腳 | `the pins of #X1:vdd` |
| 網路 | 接腳 | `the pins on &en` |
| 管線的結果 | 接腳 | `the pins of them` |
| 實例 | 網路 | `the nets connected to #M1` |
| 端子 | 實例 | `the instances owning #M1:g` |
| 端子 | 網路 | `the nets connected to #M1:g` |
| 連線 | 網路 | `the nets of %w13` |
| 實例、端子、連線、網路、圖形 | 標籤 | `the labels attached to #M1` |
| 用例（成員中不含用例） | 任意種類 | `the members of %dp` |
| 用例（成員中含用例，或未知） | 任意種類 | `all members of %st1 at any depth` |
| 用例 | 用例（SELECT-USAGE） | `%st1 itself and the usages inside it` |
| 任意物件 | 包含它的用例（SELECT-ANCESTOR-USAGE） | `the usages containing #M1` |
| 多種混合種類的集合（種類未知） | | 改寫作 `for %mixed_set, @SELECT-…` |

- `<kind>` 跟隨關鍵字：instances、terminals、pins、nets、wires、labels、annotations、usages。
- 在管線中間，候選項目中的作用域寫作 `them`，如 `=> among all members of them at any depth,`。撰寫句子時並不知道先前選出的用例之成員中是否還包含用例，因此使用 `at any depth`。
- 對於用例的成員使用 `at any depth`，而非 `at any level`：level 是電路的層級，如 `at each level in # and below`。
- 套件中的規則視同寫在其 USE-PACKAGE 的層級（10.4）；當帶有 `recursive=true` 時，套件改寫 `at every level that uses this package, #` 以取代 `at each level in # and below`。

| 關鍵字 | 情況 | 例句 |
|---|---|---|
| SELECT-INSTANCE | 本層級及以下 | `among the instances at each level in # and below, @SELECT-INSTANCE(model="nch", recursive=true) => they @SET-SYMBOL to $NMOS` |
| | 僅本層級 | `among the instances in # only, @SELECT-INSTANCE(type="pmos") => …` |
| | 過濾 | `among {#M1 #M2 #R1}, @SELECT-INSTANCE(name="M*") => they @ORIENT("MY")` |
| | 網路上 | `among the instances connected to &en, @SELECT-INSTANCE => they @SET-STYLE to $grey` |
| | 擁有某端子 | `among the instances owning #M1:g, @SELECT-INSTANCE => they are on @LAYER("top")` |
| | 用例成員 | `among all members of %st1 at any depth, @SELECT-INSTANCE(role="pair")` |
| | 不含巢狀用例之用例成員 | `among the members of %dp, @SELECT-INSTANCE` |
| | 管線中 | `among the usages at each level in # and below, @SELECT-USAGE(recursive=true) of $DIFFERENTIAL_PAIR_N => among all members of them at any depth, @SELECT-INSTANCE => they @SET-STYLE to $signal` |
| SELECT-TERMINAL | 本層級及以下 | `among the terminals at each level in # and below, @SELECT-TERMINAL(net="vss", recursive=true) => they have @ASSERT-TERMINAL(same=["net"])` |
| | 實例的端子 | `among the terminals of #R1, @SELECT-TERMINAL => on them there is a @TEXT("${net}")` |
| | 網路上的端子 | `among the terminals connected to $GROUND_NET_SET, @SELECT-TERMINAL => among the pins of them, @SELECT-PIN(wired=false) => $GROUND_STUB is @ATTACH to them` |
| SELECT-PIN | 層級中 | `among the pins in # only, @SELECT-PIN(wired=false) => on them there is a @TEXT("${net}", key="netname")` |
| | 多個端子的接腳 | `among the pins of {#M1:s #M2:s}, @SELECT-PIN(wired=false) => $GROUND_STUB is @ATTACH to them` |
| | 單一端子的接腳 | `among the pins of #X1:vdd, @SELECT-PIN => they @SET-STYLE to $red` |
| SELECT-NET | 本層級及以下 | `among the nets at each level in # and below, @SELECT-NET(port=true, recursive=true) => $IO_PORT is @ATTACH to them` |
| | 實例所連網路 | `among the nets connected to #M1, @SELECT-NET => named %m1_net_set` |
| | 端子所連網路 | `among the nets connected to #M1:g, @SELECT-NET => they are drawn with @ROUTE-NET` |
| | 連線的網路 | `among the nets of %w13, @SELECT-NET => they are drawn with @ROUTE-NET` |
| | 過濾 | `among {&a &b}, @SELECT-NET(wired=false) => they are drawn with @ROUTE-NET` |
| SELECT-WIRE | 本層級及以下 | `among the wires at each level in # and below, @SELECT-WIRE(bus=true, recursive=true) => they @SET-STYLE to $bus` |
| | 網路上的連線 | `among the wires on &clk, @SELECT-WIRE => they @SET-STYLE to $red` |
| SELECT-LABEL | 本層級及以下 | `among the labels at each level in # and below, @SELECT-LABEL(key="wl", recursive=true) => they @SET-STYLE to $grey` |
| | 附著於物件 | `among the labels attached to #M1, @SELECT-LABEL => they @SET-STYLE to $grey` |
| SELECT-ANNOTATION | 本層級及以下 | `among the annotations at each level in # and below, @SELECT-ANNOTATION(recursive=true) => they @SET-STYLE to $base` |
| | 過濾 | `among {%n1 %f1 #M1}, @SELECT-ANNOTATION` |
| SELECT-USAGE | 本層級及以下 | `among the usages at each level in # and below, @SELECT-USAGE(recursive=true) of $DIFFERENTIAL_PAIR_N => they have @PRIORITY("disabled")` |
| | 不指定定義 | `among the usages at each level in # and below, @SELECT-USAGE(recursive=true) => they @SET-STYLE to $base` |
| | 用例及其內部用例 | `among %st1 itself and the usages inside it, @SELECT-USAGE of $DIFFERENTIAL_PAIR_N` |
| SELECT-ANCESTOR-USAGE | | `among the usages containing #M1, @SELECT-ANCESTOR-USAGE of $DIFFERENTIAL_PAIR_N => named %dp_set` |
| 任一篩選 | 作用域種類未知 | `for %mixed_set, @SELECT-TERMINAL => …` |

### B.8 集合與其他關鍵字

| 關鍵字 | 情況 | 例句 |
|---|---|---|
| ADD-TO | | `&vdd @ADD-TO $SUPPLY_NET_SET` |
| | 管線中 | `… => they @ADD-TO $SUPPLY_NET_SET` |
| REMOVE-FROM | | `&agnd @REMOVE-FROM $GROUND_NET_SET` |
| UNION | | `%u_set = %a_set and %b_set @UNION` |
| BUNDLE | | `#XI<3:0> is drawn as a @BUNDLE => named %xi` |
| CURRENT | 流入 | `#M5:1 has @CURRENT("in")` |
| | 流出 | `#:iout has @CURRENT("out")` |
| | 未指定 | `#M5:2 has @CURRENT("unspecified")` |
| ASSERT-INSTANCE | | `{#M1 #M2} have @ASSERT-INSTANCE(same=["model", "w", "l"])` |
| ASSERT-TERMINAL | | `{#M1:s #M2:s} have @ASSERT-TERMINAL(same=["net"])` |
| ASSERT-NET | | `{&inp &inn} have @ASSERT-NET(port=true)` |
| 無網路的 WIRE | | `between {%sub1::y %int1::a}, on no net &, there is a @WIRE` |
| ANNOTATION | | `at (40, 20) there is an @ANNOTATION(rotate=30) of $ARROW_SYMBOL` |
| ASSERT-USAGE | | `%dp1 is, by @ASSERT-USAGE, a usage of $DIFFERENTIAL_PAIR_N` |
| ASSERT-IN | | `{&vss} is, by @ASSERT-IN, in $GROUND_NET_SET` |
| ASSERT-TOPOLOGY | 串聯 | `{#R1 #R2 #R3} are connected as @ASSERT-TOPOLOGY("series")` |
| | 並聯 | `{#M3 #M4} are connected as @ASSERT-TOPOLOGY("parallel")` |
| | 星形 | `{#L1 #C1 #L2} are connected as @ASSERT-TOPOLOGY("star")` |
| | 迴路 | `{#C1 #L1 #C2} are connected as @ASSERT-TOPOLOGY("loop")` |

### B.9 檔頭記錄與套件

未新增額外詞語。檔頭記錄以標準拼寫讀取，不使用別名（16.5）。

| 關鍵字 | 情況 | 例句 |
|---|---|---|
| VERSION | | `@VERSION("0.4")` |
| ALIAS | | `@ALIAS("@位于", "@PLACE")` |
| USE-ALIASES | | `@USE-ALIASES(name="alias-zh", version="0.4")` |
| DEVICE | | `@DEVICE(…)` |
| PACKAGE | | `@PACKAGE(name="acme-style", version="1.2")` |
| USE-PACKAGE | 在檔頭記錄中；亦為套件檔案檔頭記錄中的引用 | `@USE-PACKAGE(name="rnl-std-all", version="0.4")` |
| | 在層級中 | `@USE-PACKAGE(name="acme-style", version="1.2")` |
