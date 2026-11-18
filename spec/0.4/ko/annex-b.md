<!-- translated from en/annex-b.md sha256:f5f5cbcacdf4 -->
## 부록 B 표준 문장(참고용)

표준 문장은 표준 형식의 구문에 토큰 사이 자유 텍스트를 넣어 문장처럼 읽히게 한 것이다. 자유 텍스트는 의미를 바꾸지 않으며(3.3), 문구 규칙은 4.5에 있다. 표에서 문장 시작 부분의 `…`는 파이프에서 그 앞에 오는 세그먼트들을 뜻하고, `{…}`나 끝의 `…`는 생략된 내용을 뜻한다.

### B.1 배치와 방향

| 키워드 | 경우 | 표준 문장 |
|---|---|---|
| PLACE | 두 축 | `#M1 is at @PLACE(10, 20)` |
| | 한 축 | `the x of #M1 is at @PLACE(x=10)` |
| | 편집기가 기록함 | `the x of #X2 is at @PLACE(x=3, priority="high")` |
| | 파이프 안에서 | `… => it is at @PLACE(40, 0)` |
| ORIENT | 회전 | `#M2 @ORIENT("R90")` |
| | 거울 반전 | `#M2 @ORIENT("MX"), i.e., flipped upside down` |
| | | `#M2 @ORIENT("MY"), i.e., flipped left-right` |
| | | `#M3 @ORIENT("MXR90"), i.e., flipped upside down, then rotated 90°` |
| | | `#M3 @ORIENT("MYR90"), i.e., flipped left-right, then rotated 90°` |
| | 둘 중 하나 | `#M5 @ORIENT(["R0", "MY"]), i.e., not turned or flipped left-right` |
| | 파이프 안에서 | `… => they @ORIENT("R90")` |
| ORIENT-RELATION | 두 객체 | `from #M1 to #M2, they have @ORIENT-RELATION("MY"), i.e., flipped left-right` |
| | 둘 중 하나 | `from #M3 to #M4, they have @ORIENT-RELATION(["R0", "MY"]), i.e., the same or flipped left-right` |
| | 같은 방향 | `from #M5 to {#M7 #M8}, they have @ORIENT-RELATION("R0"), i.e., the same orientation` |
| LAYER | | `#X is on @LAYER("bottom")` |
| | 파이프 안에서 | `… => they are on @LAYER("top")` |
| GROUP | | `{#M1 #M2 #M5} as a @GROUP` |
| | 이름 붙음 | `%s1_set = {#M1 #M2 #M5} as a @GROUP` |
| | 간격 | `{#M3 #M4} as a @GROUP(spacing_x=-1), i.e., one grid unit tighter in x` |

### B.2 레이아웃 제약

| 키워드 | 경우 | 표준 문장 |
|---|---|---|
| ALIGN | 같은 y | `{#M1 #M2} @ALIGN("y"), i.e., same y` |
| | 같은 x | `{#M1 #C1} @ALIGN("x"), i.e., same x` |
| | 낮은 우선순위 | `{#M7 #M5} @ALIGN("y", priority="low"), i.e., same y` |
| | 파이프 안에서 | `… => they @ALIGN("y")` |
| ORDER | 방향 지정 | `{#M1 #M2 #M3} are in @ORDER("left-to-right")` |
| | 파이프 안에서 | `… => they are in @ORDER("bottom-to-top")` |
| DISTRIBUTE | | `{#R1 #R2 #R3} are in @DISTRIBUTE("left-to-right"), i.e., in equal steps` |
| | 파이프 안에서 | `… => they are in @DISTRIBUTE("bottom-to-top"), i.e., in equal steps` |
| DISTANCE | 방향 지정 | `from #M1 to #M2, they have @DISTANCE("bottom-to-top", min=4)` |
| | 집합에서 | `from {#M6 #M7 #CL} to %pout, they have @DISTANCE("left-to-right", min=2)` |
| TABLE | | `@TABLE layout: {{#M1 #M2 #M3} {#M4 {#M5 #M6} #M7}}` |
| | 전치 | `@TABLE(transpose=true) layout: {{#M1 #M2} {#M3 #M4}}` |
| PRIORITY | | `%c1 has @PRIORITY("disabled")` |
| | 파이프 안에서 | `… => they have @PRIORITY("low")` |
| SOLVER-CONFIG | | `@SOLVER-CONFIG(spacing_x=2, spacing_y=1)` |

- `the x of`(또는 `the y of`)는 한 축만 주는 PLACE 앞에만 쓰며(B.1), ALIGN은 대신 `, i.e., same x` 보충 설명을 쓴다.
- x는 각 멤버 위치의 x로서, 인스턴스의 경우에는 외곽선의 중심이 아니라 PLACE가 놓는 점(심볼의 원점)이며, 집합은 각 멤버를 대신한다(14.1).

### B.3 외관

| 키워드 | 경우 | 표준 문장 |
|---|---|---|
| SET-SYMBOL | 객체 하나 | `#M1 @SET-SYMBOL to $NMOS` |
| | 집합 | `{#M1 #M2} @SET-SYMBOL to $NMOS` |
| | 서브서킷의 블록 심볼 | `this subcircuit # @SET-SYMBOL to $NAND2_SYM` |
| | 파이프 안에서 | `… => they @SET-SYMBOL to $PMOS` |
| SET-STYLE | | `{#CC #RZ} @SET-STYLE to $orange` |
| | 파이프 안에서 | `… => they @SET-STYLE to $grey` |
| DEFINE-STYLE | | `$orange = @DEFINE-STYLE(linecolor="#e07000", textcolor="#e07000") {}` |
| | 다른 스타일 병합 | `$wide = @DEFINE-STYLE(linewidth=2) based on {$orange}` |

### B.4 심볼과 도형

| 키워드 | 경우 | 표준 문장 |
|---|---|---|
| DEFINE-SYMBOL | | `$TRI = @DEFINE-SYMBOL {…}` |
| PIN | | `at (-4, 0) there is a @PIN(terminal="in", direction="left")` |
| LINE | | `through {(0, 0) (4, 0)} runs a @LINE` |
| ARC | | `through {(0, 2) (2, 0) (0, -2)} runs an @ARC` |
| BEZIER | | `the control points {(0, 0) (1, 1) (2, 1) (3, 0)} make a @BEZIER` |
| CIRCLE | | `centered at (0, 0), a @CIRCLE(radius=1)` |
| RECTANGLE | 중심과 크기 | `centered at (0, 0), a @RECTANGLE(width=4, height=2)` |
| | 두 꼭짓점 | `with corners {(0, 0) (4, 2)}, a @RECTANGLE` |
| ELLIPSE | 중심과 크기 | `centered at (0, 0), an @ELLIPSE(width=4, height=2)` |
| | 외접 상자의 꼭짓점 | `in the box with corners {(0, 0) (4, 2)}, an @ELLIPSE` |
| TEXT | 심볼 안에서 | `on ? there is a @TEXT("${name}", key="name") => it is at @PLACE(-2, 2.5)` |
| ATTACH | 심볼 안에서 | `$ARROW is @ATTACH to (0, -1.5) => …` |

- 장면에 어노테이션으로 쓴 도형도 똑같이 읽는다(13.4).

### B.5 배선과 텍스트

| 키워드 | 경우 | 표준 문장 |
|---|---|---|
| WIRE | | `net &n1, through {#R1:2 %j #R2:1}, has a @WIRE` |
| | 버스 | `net &D<3:0>, through {%p1 %p2}, has a @WIRE, which is a bus` |
| | 라우팅 모드 | `net &in, through {#VIN:1 #R1:1}, has a @WIRE(mode="vertical-horizontal")` |
| ROUTE-NET | | `net &n1 is drawn with @ROUTE-NET` |
| | 집합 | `nets {&a &b} are drawn with @ROUTE-NET` |
| | 파이프 안에서 | `… => it is drawn with @ROUTE-NET` |
| ROUTE-OBJECT | | `the pins of {#M1 #M2 #M5} are joined with @ROUTE-OBJECT` |
| UNROUTE-NET | | `net &vdd is left out with @UNROUTE-NET(priority="high")` |
| UNROUTE-OBJECT | | `the pins of #M3 are left out with @UNROUTE-OBJECT` |
| TEXT | 객체 위 | `on #M1 there is a @TEXT("W/L = ${w}/${l}", key="wl", dy=-3)` |
| | 점 위 | `at (_, _) there is a @TEXT("RC low-pass") => named %title` |
| | 파이프 안에서 | `… => on them there is a @TEXT("${w}", key="size")` |

- `%j = (_, _)`처럼 점에 이름을 붙일 때는 추가 단어를 쓰지 않는다.

### B.6 정의와 적용

| 키워드 | 경우 | 표준 문장 |
|---|---|---|
| DEFINE | | `$PAIR = a definition, @DEFINE(roles=["a", "b"], parameters=[["gap", 2]]) {…}` |
| AS | 세 역할 | `#M1(left), #M2(right) and #M5(tail) are treated @AS $DIFFERENTIAL_PAIR_N` |
| | 집합을 받는 역할 | `{#M5 #M7 #M8} are treated @AS $MATCHED` |
| | 객체 하나 | `#M3 is treated @AS $DIODE_CONNECTED` |
| | 우선순위 지정 | `#M1(left), #M2(right) and %ax(axis) are treated @AS(priority="high") $MIRRORED_LAYOUT` |
| | 정의의 매개변수 | `{#R1 #R2} are treated @AS(gap=6) $SPACED` |
| | 역할 안의 용례 | `%dp(pair) and %cm(load) are treated @AS $OTA` |
| | 이름 붙음 | `%dp = #M1(left), #M2(right) and #M5(tail) treated @AS $DIFFERENTIAL_PAIR_N` |
| | 파이프에서 이름 붙고 다른 객체와 함께 | `#M1(left), #M2(right) and #M5(tail) are treated @AS $DIFFERENTIAL_PAIR_N => named %dp => it and %cm are treated @AS $OTA` |
| ATTACH | 넷 위 | `$GROUND is @ATTACH to &vss` |
| | 단자 위 | `$NO_CONNECT is @ATTACH to #M1:4` |
| | 점 위 | `$JUNCTION is @ATTACH to %j` |
| | 두 역할 | `$DIFF_PORT is @ATTACH to &inp(a) and &inn(b)` |
| | 다른 세그먼트가 뒤따르고 이름 붙음 | `$GROUND is @ATTACH to &vss => it is at @PLACE(24, 38) => named %g0` |
| | 여러 객체 각각 위에 | `$SIZE_LABEL is @ATTACH to {#M1 #M2 #M3}` |
| | 집합 위에 | `$GROUND_STUB is @ATTACH to $GROUND_FILL_SET` |
| | 파이프의 자동 보충 | `among the instances at each level in # and below, @SELECT-INSTANCE(recursive=true) => among the pins of them, @SELECT-PIN(net="vss", wired=false) => $GROUND_STUB is @ATTACH(gap=3) to them` |

- AS는 객체들이 모여 회로 구조, 배치, 또는 `$RC_STAGE` 같은 국소 그리기 방식을 이룰 때 쓰고, ATTACH는 접지, 포트 태그, 미연결 표시, 접합점, 각 인스턴스용 라벨 또는 자동 보충처럼 객체 위에 덧붙일 때 쓴다. 두 키워드는 같은 일을 하며 정의의 위치만 다르다(9.4).
- `gap`은 핀에서 접지 심볼까지 내려가는 최소 거리(기본값 1)를 정하는 `$GROUND_STUB`의 매개변수이다. 역할 이름 `left`와 `axis`처럼 표준 단어가 아니라 정의 안에서 붙인 이름이다.

### B.7 선택식

선택식은 후보들 중 고르는 형식으로 `among <candidates>, @SELECT-…(conditions)`처럼 쓴다. 후보의 문구는 범위의 종류와 선택되는 종류에 따른다.

| 범위 | 선택 대상 | 후보 |
|---|---|---|
| `#` | 모든 종류 | `the <kind> in # only` |
| 아래 레벨에 도달하는 구문 안의 `#`(10.1) | 모든 종류 | `the <kind> at each level in # and below` |
| 선택되는 종류의 집합(필터) | 그 종류 | `{…}` |
| 넷 | 인스턴스 | `the instances connected to &en` |
| 넷 | 단자 | `the terminals connected to &en` |
| 넷 | 배선 | `the wires on &en` |
| 인스턴스 | 단자 | `the terminals of #R1` |
| 인스턴스, 번들, 심볼 있는 용례 | 핀 | `the pins of #M1` |
| 단자 | 핀 | `the pins of #X1:vdd` |
| 넷 | 핀 | `the pins on &en` |
| 파이프의 결과 | 핀 | `the pins of them` |
| 인스턴스 | 넷 | `the nets connected to #M1` |
| 단자 | 인스턴스 | `the instances owning #M1:g` |
| 단자 | 넷 | `the nets connected to #M1:g` |
| 배선 | 넷 | `the nets of %w13` |
| 인스턴스, 단자, 배선, 넷, 도형 | 라벨 | `the labels attached to #M1` |
| 용례(멤버 중에 용례가 없음) | 모든 종류 | `the members of %dp` |
| 용례(멤버 중에 용례가 있거나 미상임) | 모든 종류 | `all members of %st1 at any depth` |
| 용례 | 용례(SELECT-USAGE) | `%st1 itself and the usages inside it` |
| 아무 객체 | 이를 담고 있는 용례(SELECT-ANCESTOR-USAGE) | `the usages containing #M1` |
| 여러 종류의 집합(종류 미상) | | 대신 `for %mixed_set, @SELECT-…`로 쓴다 |

- `<kind>`는 키워드를 따른다: instances, terminals, pins, nets, wires, labels, annotations, usages.
- 파이프의 중간에서는 후보 안의 범위를 `=> among all members of them at any depth,`처럼 `them`으로 쓴다. 앞에서 선택된 용례들의 멤버 중에 용례가 있는지는 문장을 쓸 때 알 수 없으므로 `at any depth`를 쓴다.
- 용례의 멤버에 대해서는 `at any level`이 아니라 `at any depth`라고 쓴다. level은 `at each level in # and below`에서처럼 회로의 레벨이기 때문이다.
- 패키지의 규칙은 그 USE-PACKAGE의 레벨에 쓰인 것으로 친다(10.4). `recursive=true`가 있으면 패키지는 `at each level in # and below` 대신 `at every level that uses this package, #`라고 쓴다.

| 키워드 | 경우 | 표준 문장 |
|---|---|---|
| SELECT-INSTANCE | 이 레벨과 그 아래 | `among the instances at each level in # and below, @SELECT-INSTANCE(model="nch", recursive=true) => they @SET-SYMBOL to $NMOS` |
| | 이 레벨만 | `among the instances in # only, @SELECT-INSTANCE(type="pmos") => …` |
| | 필터 | `among {#M1 #M2 #R1}, @SELECT-INSTANCE(name="M*") => they @ORIENT("MY")` |
| | 넷 위 | `among the instances connected to &en, @SELECT-INSTANCE => they @SET-STYLE to $grey` |
| | 단자를 소유함 | `among the instances owning #M1:g, @SELECT-INSTANCE => they are on @LAYER("top")` |
| | 용례의 멤버 | `among all members of %st1 at any depth, @SELECT-INSTANCE(role="pair")` |
| | 중첩 용례 없는 용례의 멤버 | `among the members of %dp, @SELECT-INSTANCE` |
| | 파이프의 중간 | `among the usages at each level in # and below, @SELECT-USAGE(recursive=true) of $DIFFERENTIAL_PAIR_N => among all members of them at any depth, @SELECT-INSTANCE => they @SET-STYLE to $signal` |
| SELECT-TERMINAL | 이 레벨과 그 아래 | `among the terminals at each level in # and below, @SELECT-TERMINAL(net="vss", recursive=true) => they have @ASSERT-TERMINAL(same=["net"])` |
| | 인스턴스의 단자 | `among the terminals of #R1, @SELECT-TERMINAL => on them there is a @TEXT("${net}")` |
| | 넷 위 | `among the terminals connected to $GROUND_NET_SET, @SELECT-TERMINAL => among the pins of them, @SELECT-PIN(wired=false) => $GROUND_STUB is @ATTACH to them` |
| SELECT-PIN | 이 레벨 안 | `among the pins in # only, @SELECT-PIN(wired=false) => on them there is a @TEXT("${net}", key="netname")` |
| | 단자들의 핀 | `among the pins of {#M1:s #M2:s}, @SELECT-PIN(wired=false) => $GROUND_STUB is @ATTACH to them` |
| | 단자의 핀 | `among the pins of #X1:vdd, @SELECT-PIN => they @SET-STYLE to $red` |
| SELECT-NET | 이 레벨과 그 아래 | `among the nets at each level in # and below, @SELECT-NET(port=true, recursive=true) => $IO_PORT is @ATTACH to them` |
| | 인스턴스의 넷 | `among the nets connected to #M1, @SELECT-NET => named %m1_net_set` |
| | 단자의 넷 | `among the nets connected to #M1:g, @SELECT-NET => they are drawn with @ROUTE-NET` |
| | 배선의 넷 | `among the nets of %w13, @SELECT-NET => they are drawn with @ROUTE-NET` |
| | 필터 | `among {&a &b}, @SELECT-NET(wired=false) => they are drawn with @ROUTE-NET` |
| SELECT-WIRE | 이 레벨과 그 아래 | `among the wires at each level in # and below, @SELECT-WIRE(bus=true, recursive=true) => they @SET-STYLE to $bus` |
| | 넷 위 | `among the wires on &clk, @SELECT-WIRE => they @SET-STYLE to $red` |
| SELECT-LABEL | 이 레벨과 그 아래 | `among the labels at each level in # and below, @SELECT-LABEL(key="wl", recursive=true) => they @SET-STYLE to $grey` |
| | 객체에 부착됨 | `among the labels attached to #M1, @SELECT-LABEL => they @SET-STYLE to $grey` |
| SELECT-ANNOTATION | 이 레벨과 그 아래 | `among the annotations at each level in # and below, @SELECT-ANNOTATION(recursive=true) => they @SET-STYLE to $base` |
| | 필터 | `among {%n1 %f1 #M1}, @SELECT-ANNOTATION` |
| SELECT-USAGE | 이 레벨과 그 아래 | `among the usages at each level in # and below, @SELECT-USAGE(recursive=true) of $DIFFERENTIAL_PAIR_N => they have @PRIORITY("disabled")` |
| | 정의 없음 | `among the usages at each level in # and below, @SELECT-USAGE(recursive=true) => they @SET-STYLE to $base` |
| | 용례와 그 안의 용례 | `among %st1 itself and the usages inside it, @SELECT-USAGE of $DIFFERENTIAL_PAIR_N` |
| SELECT-ANCESTOR-USAGE | | `among the usages containing #M1, @SELECT-ANCESTOR-USAGE of $DIFFERENTIAL_PAIR_N => named %dp_set` |
| 모든 선택식 | 범위의 종류 미상 | `for %mixed_set, @SELECT-TERMINAL => …` |

### B.8 집합과 기타 키워드

| 키워드 | 경우 | 표준 문장 |
|---|---|---|
| ADD-TO | | `&vdd @ADD-TO $SUPPLY_NET_SET` |
| | 파이프 안에서 | `… => they @ADD-TO $SUPPLY_NET_SET` |
| REMOVE-FROM | | `&agnd @REMOVE-FROM $GROUND_NET_SET` |
| UNION | | `%u_set = %a_set and %b_set @UNION` |
| BUNDLE | | `#XI<3:0> is drawn as a @BUNDLE => named %xi` |
| CURRENT | 들어감 | `#M5:1 has @CURRENT("in")` |
| | 나감 | `#:iout has @CURRENT("out")` |
| | 미지정 | `#M5:2 has @CURRENT("unspecified")` |
| ASSERT-INSTANCE | | `{#M1 #M2} have @ASSERT-INSTANCE(same=["model", "w", "l"])` |
| ASSERT-TERMINAL | | `{#M1:s #M2:s} have @ASSERT-TERMINAL(same=["net"])` |
| ASSERT-NET | | `{&inp &inn} have @ASSERT-NET(port=true)` |
| 넷 없는 WIRE | | `between {%sub1::y %int1::a}, on no net &, there is a @WIRE` |
| ANNOTATION | | `at (40, 20) there is an @ANNOTATION(rotate=30) of $ARROW_SYMBOL` |
| ASSERT-USAGE | | `%dp1 is, by @ASSERT-USAGE, a usage of $DIFFERENTIAL_PAIR_N` |
| ASSERT-IN | | `{&vss} is, by @ASSERT-IN, in $GROUND_NET_SET` |
| ASSERT-TOPOLOGY | 직렬 | `{#R1 #R2 #R3} are connected as @ASSERT-TOPOLOGY("series")` |
| | 병렬 | `{#M3 #M4} are connected as @ASSERT-TOPOLOGY("parallel")` |
| | 성형 | `{#L1 #C1 #L2} are connected as @ASSERT-TOPOLOGY("star")` |
| | 단일 고리 | `{#C1 #L1 #C2} are connected as @ASSERT-TOPOLOGY("loop")` |

### B.9 헤더 레코드와 패키지

어떤 단어도 추가하지 않는다. 헤더 레코드는 표준 철자로 읽히며 별칭을 쓰지 않는다(16.5).

| 키워드 | 경우 | 표준 문장 |
|---|---|---|
| VERSION | | `@VERSION("0.4")` |
| ALIAS | | `@ALIAS("@位于", "@PLACE")` |
| USE-ALIASES | | `@USE-ALIASES(name="alias-zh", version="0.4")` |
| DEVICE | | `@DEVICE(…)` |
| PACKAGE | | `@PACKAGE(name="acme-style", version="1.2")` |
| USE-PACKAGE | 헤더 레코드 안; 패키지 파일의 헤더 레코드 내 참조 포함 | `@USE-PACKAGE(name="rnl-std-all", version="0.4")` |
| | 한 레벨 안 | `@USE-PACKAGE(name="acme-style", version="1.2")` |
