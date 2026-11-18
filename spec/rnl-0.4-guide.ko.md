# RNL 0.4 요약 가이드

RNL은 SPICE 또는 Spectre 넷리스트의 주석 안에 작성하며, 회로를 어떻게 그리고 각 부품이 무엇을 뜻하는지 나타낼 뿐 회로 자체는 결코 바꾸지 않는다. 언어 버전은 `"0.4"`, 표준 패키지 시리즈는 `"0.4"` 기준이며, (14.8) 같은 번호는 표준 문서의 절을 가리킨다. 예제는 하나의 이어진 프로그램이 아니라 각기 독립된 활용 예이므로, 각 레벨에 이미 존재하는 객체를 사용하고 코드 조각은 RNL 레코드 안에 넣어 작성한다. `/* */`는 설명용 주석이다. 끝부분의 카탈로그는 모든 이름을 시그니처 형태로 나열한 목록이며, 그대로 복사해 쓰는 RNL 코드가 아니다.

## 파일

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

- 레코드는 1열의 `*` 뒤에 공백 하나가 오는 한 줄 전체 주석이며, Spectre 모드(`.scs` 파일 또는 `simulator lang=spectre` 이후)에서는 `//`도 쓸 수 있다. 헤더 레코드가 가장 먼저 오며, `.subckt … .ends` 내부의 레코드는 해당 서브서킷에 속한다.
- USE-PACKAGE가 없으면 표준 이름은 전혀 인식되지 않는다. 전달받은 헤더 레코드는 그대로 유지한다. 그 안의 `@DEVICE` 줄은 PDK 소자에 단자 이름과 타입을 부여하므로(6.12) 직접 작성하거나 변경하지 않는다.
- 모든 `@` 앞에는 ASCII 공백을 하나 둔다. 매개변수 괄호는 키워드에 붙여 쓰고, 역할 괄호는 객체에 붙여 쓴다. 위치로 전달하는 매개변수가 이름으로 전달하는 매개변수보다 앞에 온다: `@PLACE(10, y=20)`. 숫자에는 단위가 없고 표현식도 없다. RNL 이름과 키워드는 대소문자를 구분하며, 넷리스트 이름은 호스트 규칙을 따른다.

## 참조

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

- 단자 이름: M `d g s b`; Q `c b e s`; J Z `d g s`; D `a k`; R C L V I B F H W `p n`; E G S `p n cp cn`; T O `p1 n1 p2 n2`; U `a b ref`; X는 해당 서브서킷의 포트이다. 핀이 필요한 자리에서는 단자가 그 핀을 대신한다.
- 이름은 레벨마다 한 번만 바인딩되며 정의보다 앞에서 사용할 수도 있다. `%p`를 옮기려면 `%p = ...`를 다시 쓰지 말고 `%p @PLACE(...)`를 작성한다. `%` 이름은 해당 레벨에만 속하고, `$` 정의는 하위 레벨로 상속되며 하위 레벨의 다른 정의가 이를 가릴 수 있다.
- 접두사 없는 단어는 자유 텍스트이며(무언가의 이름과 같으면 경고가 난다) `#M1 #M2 M3 @ALIGN("y");`는 M1과 M2만 정렬한다.

## 구문과 파이프

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

각 세그먼트가 다음으로 넘기는 결과: PLACE, ORIENT, SET-SYMBOL, SET-STYLE, GROUP, CURRENT는 입력 객체를 그대로 넘긴다. AS, ATTACH는 새로 만든 용례(하나씩 적용할 때는 그 용례들의 집합)를 넘긴다. ALIGN, ORDER, DISTRIBUTE, DISTANCE, TABLE, ORIENT-RELATION은 새로운 제약을, TEXT는 새로운 텍스트를, SELECT-…는 선택된 집합을 넘긴다.

## 정의, 역할, 용례

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

- `{role}`은 집합 전체(객체 하나는 원소 하나짜리 집합)를 받으며 생략할 수 있다. 역할을 표시하지 않고 순서대로 적을 때는 뒤쪽의 집합 역할만 생략할 수 있으며, 단일 객체 역할은 필수이다. 각 역할은 최대 한 번만 표시하고, 여러 멤버는 하나의 집합으로 묶어 `{#R1 #R2}(members)`처럼 적는다.
- 정의 본문 안에서 단독 `?`는 용례 자신의 점이고 좌표는 그 점에서의 오프셋이며, `%mid = (_, _);`는 지역 객체를 만들어 바깥에서 `%u::mid`로 참조한다.
- 배치하는 표준 구조는 용례 자신의 점을 묶는다. 멤버들의 행이나 열은 그 직선 위에, 대칭인 양쪽은 그 한가운데에 자신의 점을 두므로 `%u::position`에 제약을 걸면 구조 전체가 움직인다. 카탈로그의 *그리기* 열에서 어느 위치에 묶는지 확인할 수 있다: `$BRANCH`는 멤버들의 x만 자신의 점과 공유하고, `$BRANCH_HORIZONTAL`은 y만 공유하며, `$PARALLEL_HORIZONTAL`과 `$MIRRORED_LAYOUT`은 자신의 점을 그 중심에 둔다. 그 밖의 구조는 이를 자유 상태로 두며, 그 `::position`에 제약을 걸어도 그려지는 것은 아무것도 달라지지 않는다.
- 부품의 역할을 나타내면서 그리기도 하는 구조를 우선 사용한다. `$BRANCH`는 멤버를 위에서 아래로 세로로 쌓는 직렬 구조로, 전원에서 접지로 이어지는 경로 같은 직류 경로(DC 가지)에 주로 쓰인다. 이런 경로는 각각 하나의 `$BRANCH`로 작성하고, 그 위의 병렬 쌍은 배치까지 수행하는 변형으로 작성한다: `{#R1 #M1 #R2} @AS $BRANCH; {#R2 #C2} @AS $PARALLEL_HORIZONTAL;`. `$PARALLEL`만으로는 라우팅만 수행한다. 직류 경로(DC 가지) 자체는 전류에 관해 아무것도 말하지 않으므로 전류는 `@CURRENT`로 작성한다. 스위치드 커패시터 샘플러처럼 스위치와 커패시터를 거치는 신호 경로는 각 행의 핀에 ALIGN과 ORDER를 거는 등 원자 사실로 그린다: `{#S1:s #S1:d #C1:p #C1:n} @ALIGN("y"); {#S1:s #S1:d #C1:p #C1:n} @ORDER("left-to-right");`. *의미만* 구조는 아무것도 그리지 않으므로 그 의미가 성립하는 곳에 적되, 배치하는 구조 대신 쓰지는 않는다.`$SIGNAL_FLOW`는 확실히 파악된 깔끔한 신호 경로에만 표시하고, 경로의 멤버 수와 관계없이 신호가 여러 갈래로 나뉘거나 피드백되거나 회로의 상당 부분에서 양방향으로 흐르는 곳에는 쓰지 않는다.

## 선택식과 규칙

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

- `name`, `model`, `net`, `key`에는 `*`를 쓸 수 있다. `type`은 `.model` 카드나 DEVICE가 부여하는 타입이다.
- 선택식 결과와 그로부터 만든 집합은 멤버 위치 지정(`%sel{1}`)을 받지 않으며 WIRE에도 넘길 수 없다. 객체에 직접 이름을 붙이거나 ROUTE-OBJECT를 사용한다.

## 레이아웃 사실

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

- 방향: `"left-to-right"`, `"right-to-left"`, `"bottom-to-top"`, `"top-to-bottom"`. 방향 코드: `R0`, `R90`, `R180`, `R270`은 반시계 방향 회전, `MY`는 좌우 반전, `MX`는 상하 반전이며, `MYR90`과 `MXR90`은 먼저 반전한 뒤 회전한다.
- 멤버의 기준점은 인스턴스는 원점, 단자와 핀은 핀 위치, 점·텍스트와 심볼이 있는 용례는 각자 자신의 점이다. 심볼 없는 용례는 GROUP에서처럼 가장 하위까지 내려가며 그 멤버들을 대신하고, 그 자신의 점은 `%u::position`이다. 집합에는 원점이 없으므로 GROUP 뒤에도 PLACE는 각 멤버를 배치한다. PLACE는 핀이나 제약을 배치하지 않는다. “맨 왼쪽” 같은 표현은 없으므로 다른 부품과의 관계로 작성한다.
- 솔버는 자유 상태로 남은 부분만 결정하며 부품이 무엇인지 스스로 추측하지 않으므로, 성립해야 할 조건은 직접 작성한다. 가급적 의미와 제약을 써서 솔버가 배치하게 하고, 작업이 좌표를 요구할 때는 대신 모든 대상에 PLACE, ORIENT, WIRE를 작성한다. 두 방식을 섞지 않는다: 솔버가 풀도록 의미와 제약을 적고 PLACE는 거의 또는 전혀 쓰지 않거나, 풀이 없이 모든 위치를 직접 적어야 한다.
- 개별 소자가 아니라 큰 블록 단위로 순서를 정한다: `{#V2 #C1 %bias %stage #C3 #R5} @ORDER("left-to-right");`는 열 전체를 나란히 배치한다. 구조는 그 멤버들을 대신하므로 `%stage`의 모든 소자는 `%bias`의 모든 소자와 C3 사이에 놓이며, 두 구조가 공유하는 소자는 자기 자신과 순서 지어지지 않는다. 대신 구조의 중심을 제약하려면 자신의 점을 쓴다: `{%dp::position #X1} @ALIGN("y");`는 X1을 차동쌍 중심 높이에 놓는 반면, `{%dp #X1} @ALIGN("y");`는 세 트랜지스터를 X1의 높이에 놓는다(구조가 점을 어디에 묶는지는 카탈로그의 *그리기* 열 참조).
- 전원이나 신호원처럼 회로 주변에 놓이는 부품은 대개 회로의 한쪽에 조금 떨어져 서며, 이는 DISTANCE로 나타낸다: `#VDD {#VIN #XU1 #CL} @DISTANCE("left-to-right", min=4);`는 전원을 나머지 각 대상보다 최소 4단위 왼쪽에 유지한다.
- 도면에 명확한 그룹 트리를 부여한다. 각 기능 블록을 하나의 그룹으로 삼고 작은 그룹을 더 큰 그룹 안에 중첩하며, 각 노드에 직접 속하는 항목은 대여섯 개 정도로 둔다. 많은 구조는 이미 그룹이지만(*그룹* 열 참조), 전류 거울처럼 다른 구조 안에 일부만 걸칠 수 있는 구조는 그룹이 아니다. 구조 전체나 증폭단과 그 포트들은 직접 그룹으로 묶는다: `%amp %p_inp %p_inn %p_out @GROUP;`. 그룹들은 서로 교차하지 않으므로, 중첩되지 않은 채 멤버를 공유하는 두 그룹 중에서는 솔버가 하나만 블록으로 유지한다.
- 다른 조건으로 정해지지 않으면서 독자가 특정 방향을 기대하는 곳에만 방향을 고정한다. 보통 신호원, 전원 소스, 부하가 여기에 해당한다: `{#V1 #V2 #R5} @ORIENT("R0");`. 나머지는 솔버에 맡기면 구조와 전류가 대부분을 결정한다.

## 핀과 방향

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

- 핀 하나만 고정하면 여덟 가지 방향 중 두 가지가 남는다. 게이트만 고정하면 똑바로 서거나 거꾸로 뒤집힌 상태가 되고, 드레인만 고정하면 게이트가 왼쪽이나 오른쪽을 향한다. 핀 두 개를 고정한다.
- R0에서의 표준 핀 방향: R, C, L과 전원은 p 위, n 아래; `$DIODE`는 a 위, k 아래; `$NMOS`는 d 위, g 왼쪽, s 아래, b 오른쪽(4TERM); `$PMOS`는 s 위, g 왼쪽, d 아래; `$NPN`은 c 위, b 왼쪽, e 아래; `$PNP`는 e 위, b 왼쪽, c 아래이다.

## 전류

```rnl
#M1:d @CURRENT("in"); #M1:s @CURRENT("out"); /* the static current at a terminal: in, out, unspecified */
{#M1:d #M2:d #M5:d} @CURRENT("in"); /* a set does each member */
#:out @CURRENT("in"); /* at a port: current enters the subcircuit */
#M1::d @CURRENT("in"); /* a pin stands for its terminal */
```

- 아무것도 그리지 않고 회로의 동작만 기술하며, 솔버가 이에 따라 전류 경로를 배치한다. 주 전류 경로에 작성한다.
- 정적 전류가 흐르고 방향이 확실한 곳에만 작성한다. 커패시터, 게이트, DC 값이 없는 전원은 전류가 흐르지 않으므로 빼고, 확실하지 않은 단자도 뺀다. 전원 소스 역시 표시하지 않는다. 내부에서 전류가 n에서 p로 위로 흐르기 때문에, 아래로 흐르는 전류를 선호하는 솔버가 전원 소스를 거꾸로 뒤집을 수 있다.
- 한 넷의 모든 단자와 포트가 전류를 한쪽으로만 보내는 방향을 가지면 위반이 보고된다. 0 같은 전역 넷은 검사하지 않는다.

## 우선순위

```rnl
#M5 @PLACE(x=3, priority="low"); /* disabled, low, normal (default), high */
%dp1 = #M1(left) #M2(right) #M5(tail) @AS(priority="high") $DIFFERENTIAL_PAIR_N; /* covers every fact of the body */
%dp1 @PRIORITY("disabled"); /* turns the default drawing of the pair off */
```

- 충돌이 나면 우선순위가 낮은 쪽이 먼저 밀리고, 같으면 먼저 쓴 쪽이 밀린다. 유지된 사실은 필수(hard) 제약이다. 표준 패키지는 `"low"` 우선순위로 그린다. 레코드 헤더의 `priority=`는 기본값으로 쓰이지만, AS와 ATTACH는 거기서 `"disabled"`만 받는다.
- 구조에는 우선순위가 필요 없다: 검사나 풀이에서 드러난 충돌을 해결할 때만 작성한다.

## 배선과 라우팅

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

- 좌표가 같다고 해서 핀이 연결되지는 않는다. 길이가 0이라도 사이에 WIRE를 작성해 배선됨을 표시해야 불필요한 자동 보충을 피할 수 있다.
- 구조는 내부 배선을 low 우선순위로 그린다. ROUTE를 쓰지 않으면 두 구조 사이는 넷 라벨로 이어진다.
- 모든 것이 아니라 배선으로 그렸을 때 읽기 좋은 곳만 배선한다. 솔버는 WIRE, ROUTE-NET, ROUTE-OBJECT로 요청한 것만 그리며, 나머지 배선되지 않은 핀에는 자동 보충이 접지·전원·넷 라벨 스텁을 붙인다. 주 신호 경로와 인접한 구조 내부 및 구조 간 연결은 라우팅하고, 바이어스선이나 도면 전반에서 공유되는 넷처럼 길거나 복잡한 연결은 라벨로 남긴다.
- 작은 회로에서는 전원·접지·바이어스 레일을 뺀 모든 넷을 배선으로 그리는 편이 대개 깔끔하므로 바로 위 마지막 줄처럼, 또는 `# @SELECT-NET => @ROUTE-NET;`로 라우팅한다. 둘의 결과는 같다. 어느 쪽이든 자동 보충이 접지·전원·바이어스 넷을 high 우선순위로 라우팅 해제하며, 각 레일 넷마다 트리가 모든 핀을 잃었다는 힌트(RNL9403)가 나온다. 이 힌트는 예상된 것이다.
- 자기 레벨의 포트로 연결되는 넷은 포트 태그를 써야만 포트에 닿는다: `$IO_PORT @ATTACH &x;`, 또는 그 넷이 구동하는 핀에 `$IO_PORT_STUB`. 자동 보충은 어느 핀이 배선되었는지 정한 뒤에 포트 태그를 더하므로, 자기 핀이 하나뿐인 넷(예컨대 게이트 하나를 구동하는 입력)은 점 하나짜리 트리를 이룬다. 그 핀에는 넷 라벨이 붙고, 포트에 닿는 배선은 없다.

## 텍스트

```rnl
#M1 @TEXT("${w}/${l}", key="wl", dy=-3); /* a label that moves with M1; #M1::wl */
#M1 @TEXT("M_{in}", key="name"); /* the same key replaces the name the symbol shows */
(10, 10) @TEXT("bias", size=0.8); /* on a point: an annotation */
#M1 @TEXT("\overline{EN} \mu A \frac{W}{L}", halign="middle", valign="top", rotate=90);
```

- `${…}`: `name`, `value`, `model`, `type`, `w` 같은 넷리스트 매개변수, `net`, 역할, 그리고 `${pair::left::w}` 같은 경로를 참조할 수 있다.
- `dx`, `dy`는 텍스트가 부착된 대상 기준의 오프셋이다. `#M1::wl @PLACE(5, 0);`은 라벨을 도면 좌표 (5, 0)에 두며, 각 축에서 PLACE와 오프셋은 서로를 대체해 나중에 쓴 쪽이 이긴다. 심볼 본문 안에서는 둘 다 심볼 자체의 좌표계를 따른다.

## 심볼과 스타일

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

- 스타일 속성: `linecolor linestyle linewidth fillcolor textcolor size font bold italic textvisible`; 색상은 `"#RRGGBB"` 형식이다. `%` 이름으로 지역 정의를 만들 수도 있다. 각 속성별로 직접 지정이 규칙보다 우선하며, 같은 단계 안에서는 나중에 쓴 설정이 이긴다.

## 검사

표준 구조는 자신이 선언한 조건을 검사한다. 쌍을 이루는 두 소자가 같은 모델과 크기인지, 직렬 연결이 실제로 직렬인지, pmos 역할에 PMOS가 들어 있는지, 포트 태그가 포트에 붙었는지 등이 이에 해당한다. 검사를 통과하지 못하면 **위반**으로 보고만 될 뿐, 실행이 실패하거나 도면이 달라지지는 않는다. 구조나 역할을 바로잡거나, 요구 조건이 덜 엄격한 구조를 선택한다.

## 표준 패키지

```rnl
@USE-PACKAGE(name="rnl-std-all", version="0.4"); /* everything: styles, symbols, rules for SPICE and Spectre, fill, structures, the blocks */
@USE-PACKAGE(name="rnl-std-no-labels", version="0.4"); /* optional, besides rnl-std-all: hides the labels symbols draw (name, value, model, net names); a label you write shows only with textvisible=true */
@USE-PACKAGE(name="rnl-std-45deg", version="0.4"); /* optional: symbols at 45 degrees */
@USE-PACKAGE(name="rnl-std-fill") {}; /* an empty package drops that part */
&vdd @ADD-TO $SUPPLY_NET_SET; &vss @ADD-TO $GROUND_NET_SET; /* supply and ground stubs on their pins; 0 is a ground already */
$PORT_LIST @ATTACH; /* a port tag on every port net, in one column */
$FIGURE @ATTACH %p => @SET-SYMBOL $MY_FIG; /* no netlist instance: use a symbol with unbound pins and no ${name}/${value}; not $RESISTOR */
```

- 기본 심볼: R `$RESISTOR`, C `$CAPACITOR`, L `$INDUCTOR`, V `$VOLTAGE_SOURCE`, I `$CURRENT_SOURCE`, D `$DIODE`, E `$VCVS_4TERM`, G `$VCCS_4TERM`, F `$CONTROLLED_CURRENT_SOURCE`, H `$CONTROLLED_VOLTAGE_SOURCE`, B `$BEHAVIORAL_SOURCE`, S `$VOLTAGE_CONTROLLED_SWITCH`, W `$SWITCH`, T O `$TRANSMISSION_LINE`, U `$RC_LINE`; 타입별 트랜지스터: nmos `$NMOS_4TERM`, pmos `$PMOS_4TERM`, npn `$NPN`, pnp `$PNP`, njf `$NJFET`, pjf `$PJFET`. 바디(bulk)가 접지 넷에 연결된 NMOS와 바디가 전원 넷에 연결된 PMOS는 별도로 SET-SYMBOL을 쓰지 않아도 자동으로 3핀 심볼인 `$NMOS`와 `$PMOS`를 받는다. `.model`이나 DEVICE를 통해 타입이 주어지지 않은 트랜지스터는 소자 심볼을 받지 못하고, 노드마다 핀이 하나씩 있는 상자로 그려진다(9.2.8).
- 위와 같이 각 전원 넷은 `$SUPPLY_NET_SET`에, 0이 아닌 접지 넷은 `$GROUND_NET_SET`에 넣는다. 그러면 배선되지 않은 핀에 스텁이 붙고 해당 넷은 라우팅되지 않은 채 남는다. 자동 보충은 배선이나 미연결 표시가 있는 핀은 건너뛰며, 소자의 것이든 가상 부품의 것이든 나머지 배선되지 않은 핀에는 `$GROUND_NET_SET`(기본값 넷 0)의 넷이면 `$GROUND_STUB`, `$SUPPLY_NET_SET`(기본값 비어 있음)의 넷이면 `$SUPPLY_STUB`, `$BIAS_NET_SET`(기본값 비어 있음)의 넷이면 `$BIAS_STUB`, 그 밖의 넷이면 `$NET_LABEL_STUB`을 붙인다. 4단자 MOS의 바디 핀에는 어느 넷에 연결되어 있든 `$NET_LABEL_STUB`이 붙는다. 전원 넷과 접지 넷을 포함한 모든 포트 넷에는 해당 레벨 인스턴스들보다 최소 6단위 왼쪽의 한 열에 `$IO_PORT`가 각각 R0으로 배치된다. 특정 넷을 빼려면 `&vdd @REMOVE-FROM $PORT_FILL_SET;`을 쓰고, 포트 태그를 직접 배치하려면 `$IO_PORT @ATTACH &out;`을 쓴다. 포트 태그는 자체 `netname` 텍스트로 넷 이름을 표시하므로 그 이름을 위한 TEXT를 따로 쓰지 말고, 위치를 옮길 때는 `%t::netname @PLACE(x, y);`를 쓴다. 테스트벤치의 넷 이름에는 `$NET_LABEL`을 쓴다. 이미 `$IO_PORT_STUB`이나 `$NET_LABEL_STUB`이 붙은 핀은 그 리드선을 WIRE가 아니라 심볼이 그리더라도 넷 라벨을 추가로 받지 않는다. `*_FILL_SET` 집합은 자동 보충 자체가 스텁과 태그를 추가하기 위한 용도이므로, 풀이 결과가 이를 다시 기록하고 나면 그 핀과 넷이 집합에서 빠진다. 따라서 그 핀에 TEXT를 다는 등 이 집합을 기반으로 다른 규칙을 작성하지 말고 핀을 직접 선택해야 한다. 어떤 핀에 스텁이 붙을지 바꾸려면 해당 집합에 핀을 추가하거나 뺀다. 사용자가 작성한 규칙이 붙인 스텁은 자동 보충이 인식하지 못하므로 자동 보충도 자신의 스텁을 함께 추가한다.

## 솔버의 레코드

```spice fragment
* @RNL(type="generated", priority="high", author="rnl-solver") { #R1 @PLACE(0, 4); #R2 @PLACE(0, -4);} /* never edit; replaced on each solve */
* @RNL(author="editor") { #R2 @PLACE(y=-6, priority="high");} /* to fix a position: high, in a later record */
```

`%#3` 같은 번호는 여기에서만 나타나며, 직접 작성할 때는 대신 객체에 이름을 붙여 사용한다.

## 기타 키워드

패키지와 도구가 작성하는 키워드이며, 주로 읽는 용도이고 직접 쓰는 경우는 드물다:

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

## 흔한 실수

왼쪽 열은 잘못되었거나 오해를 부르는 표기이다. 오른쪽의 수정된 형태만 자신의 파일 이름에 맞추어 복사한다.

|실수|수정|
|---|---|
|`#M1 @PLACE (10, 20);`|`#M1 @PLACE(10, 20);`|
|`#M1 (left) #M2 (right) @AS $CROSS_COUPLED_PAIR;`|`#M1(left) #M2(right) @AS $CROSS_COUPLED_PAIR;`|
|`#M1 #M2 @SET-SYMBOL $NMOS;`|`{#M1 #M2} @SET-SYMBOL $NMOS;`|
|`#M5 #M3 @AS $MATCHED;`|`{#M5 #M3} @AS $MATCHED;`|
|`{#R1 #R2} @ORDER("x");`|`{#R1 #R2} @ORDER("left-to-right");`|
|`#R1 @PLACE(x=2u);`|`#R1 @PLACE(x=2);`|
|0.3 이름 `$DIFFERENTIAL_PAIR`, 역할 `in_p/in_n`|`#M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N;`|
|`#R1 @SET-SYMBOL $GROUND;`|접지에는 `$GROUND @ATTACH &n;`을 쓰고 저항 심볼은 그대로 둔다|
|핀을 정렬하려고 쓴 `%t #M1 @ALIGN("y");`|`{%t::pin #M1::g} @ALIGN("y");`|
|포트가 아닌 넷에 쓴 `$IO_PORT @ATTACH &n;`|`$NET_LABEL @ATTACH &n;`|
|vss가 접지 집합에 있을 때 쓴 `&vss @ROUTE-NET;`|`&vss @ROUTE-NET(priority="high");`|
|직접 작성한 텍스트의 `%#3 @PLACE(1, 2);`|해당 객체에 이름을 붙이고 그 이름을 사용한다|
|`#M1.d; M3; &vdd!;`|`#M1:d; #M3; &"vdd!";`|
|`{@VERSION("0.4");}`|`{ @VERSION("0.4");}`|

`$NMOS`의 바디(bulk)에 배선하면 실패하므로 `$NMOS_4TERM`을 쓴다. 알 수 없는 트랜지스터 타입에는 모델 카드, DEVICE, 또는 명시적 심볼이 필요하다. 최상위 레벨의 규칙이 서브서킷에 닿으려면 `recursive=true`를 써야 한다.

## 전체 예제

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

파일을 넘기기 전 점검 사항: 헤더가 맨 앞에 있고 VERSION과 패키지가 들어 있는지, 이름이 해당 레벨에 존재하는지, 0.4 이름(카탈로그)만 썼는지, 모든 `@` 앞에 공백이 있고 모든 구문 뒤에 `;`가 있는지, 방향을 적었는지, 집합 역할에 집합을 넘겼는지, 접지와 전원이 각 집합에 들어 있는지, 방향은 필요한 곳에만 쓰고 전류는 확실한 곳에만 적었는지, 정렬한 핀들을 배선했는지, 생성 레코드를 건드리지 않았는지, 의도하지 않은 위반이 없는지 확인한다.

<!-- lists:begin -->
## 카탈로그

그대로 복사할 RNL 코드가 아니라 참조용 목록이므로 이름의 `$`는 유지한다. 정의는 AS나 ATTACH로 적용하고, 심볼은 SET-SYMBOL로, 스타일은 SET-STYLE로 지정한다. 역할은 선언 순서대로 나열되며 각각 받는 객체 종류가 표시된다(종류가 없는 역할은 어떤 종류의 객체든 받는다). `{role}`은 집합 전체를 받으며 생략할 수 있고, `;` 뒤에 매개변수가 이어진다. 따라서 `$MATCHED({members}: instance/usage)`는 `{#M1 #M2} @AS $MATCHED;`처럼 쓴다.

### 패키지

각 패키지는 헤더에서 `@USE-PACKAGE(name="...", version="0.4");`로 지정한다.

|패키지|내용|
|---|---|
|rnl-std-all|시리즈 0.4의 유일한 패키지 컬렉션으로, 자체 내용은 없이 스타일, 기본 및 반도체 심볼과 정의, 자동 보충, 회로 의미, 네 개의 도메인 패키지, 그리고 각 언어용 헤더 레코드를 통해 호스트 방언별 규칙 패키지를 불러온다(2.8, 16.3).|
|rnl-std-style|스타일(9.3)과 기본 스타일 규칙.|
|rnl-std-basic|수동 소자, 전원, 스위치, 전송선, 가상 부품과 마크의 심볼; 가상 부품, 마크, 도형 및 스텁의 정의와 각각의 용례를 그리는 규칙; 접지·전원·바이어스 넷 집합.|
|rnl-std-semiconductors|다이오드 및 MOS·바이폴라·JFET·단일접합(UJT, PUT) 트랜지스터의 심볼.|
|rnl-std-spice|SPICE 규칙 패키지: 소자 문자, 모델 타입, 그리고 바디 또는 제어 음극 넷에 따라 인스턴스에 심볼을 배정한다(16.3).|
|rnl-std-spectre|Spectre 규칙 패키지: 마스터, 모델 타입, 그리고 바디 또는 제어 음극 넷에 따라 인스턴스에 심볼을 배정한다(16.3).|
|rnl-std-fill|자동 보충용 집합과 규칙: 가상 부품의 핀을 포함해 배선되지 않은 핀에 접지·전원·바이어스·넷 라벨 스텁을 붙이고, `$PORT_FILL_SET`의 모든 포트 넷에 포트 태그를 붙인다(10.5, 12.4, 16.3).|
|rnl-std-semantics|회로 구조와 그 기본 그리기에 대한 의미 정의.|
|rnl-std-functional|동작 모델: 연산 증폭기, 비교기, 증폭기, 트랜스컨덕터, ADC, DAC, 초퍼 변조기와 정수 격자 vp(위)·vn(아래) 핀을 갖춘 *_SUPPLY 변형.|
|rnl-std-rf|RF 부품: 센터탭 인덕터와 T-코일.|
|rnl-std-logic|논리 게이트, 버퍼, 인버터, 지연 셀, D 플립플롭.|
|rnl-std-signal-flow|이상적 신호 흐름 가상 부품과 심볼: 가산기, 감산기, 곱셈기, 연속시간·이산시간 적분기, 양자화기, 단위 지연기.|
|rnl-std-no-labels|라벨을 직접 쓰는 파일을 위해 표준 심볼의 기본 라벨을 숨긴다(16.3).|
|rnl-std-no-instance-labels|인스턴스의 기본 라벨만 숨긴다. 접지, 전원, 바이어스 표시, 포트 태그, 넷 라벨이 그리는 넷 이름은 그대로 남는다(16.3).|
|rnl-std-45deg|브리지, 교차 결합 쌍, 대각선 권선, 45도 배선을 위한 45도 심볼.|

### 정의

그리기 열은 rnl-std-semantics의 구조에 대한 설명이다: *의미만*은 아무것도 배치하거나 라우팅하지 않고, *배치*는 멤버들에 위치 사실을 두며, *라우팅*은 멤버들이 공유하는 넷을 잇는다. 이들의 위치 사실과 라우팅은 모두 low 우선순위를 가진다. 그룹 열은 각 정의의 본문에서 읽어 온 것이다: 멤버들을 한 그룹으로 묶는 역할들과 함께 적는 *예*, 그룹을 만드는 정의 `$X`로 넘기는 역할들과 함께 적는 `$X` *경유*, 그룹을 만들지 않으면 빈칸이다. 전류 거울처럼 다른 구조 안에 일부만 걸칠 수 있는 구조는 그룹을 만들지 않는다(*레이아웃 사실* 아래의 그룹 트리 참조). 심볼이 없는 용례는 ALIGN, ORDER, DISTRIBUTE, DISTANCE에서 자기 멤버들을 대신한다. 그 자신의 점 `%u::position`(14.1)이 멤버들과 묶이는 것은 *자신의 점*이 표시된 구조뿐이다. 그런 용례만 `::position`으로 한 항목처럼 움직이거나 배치하는 다른 구조의 멤버로 움직이며, 나머지는 멤버들이나 그 이름 붙은 집합을 제약한다. 그 외 패키지는 그려지는 부품과 마크를 정의한다.

|시그니처|그리기|그룹|의미|
|---|---|---|---|
|**rnl-std-basic**||||
|`$GROUND(p: net)`|—||자신의 점에 그려지는 넷 위의 접지: 넷 0에서는 3선 접지 기호만, 다른 넷에서는 그 넷의 이름까지 표시한다.|
|`$SUPPLY(p: net)`|—||자신의 점에 그려지는 넷 위의 전원.|
|`$BIAS(p: net)`|—||자신의 점에 그려지는 넷 위의 바이어스 전압 마크.|
|`$IO_PORT(p: net)`|—||자신의 점에 그려지는 넷 위의 포트 태그; 해당 넷은 그 레벨의 포트여야 한다.|
|`$NET_LABEL(p: net/pin)`|—||넷 라벨: 자신의 점에 그려지는 짧은 리드선 위의 넷 이름; 번들의 핀 같은 버스 핀이 주어지면(11.8) 그 핀을 유지한 채 모든 넷을 아우르는 하나의 라벨이 되어 각 넷의 이름을 보여 준다(13.6).|
|`$NO_CONNECT(t: terminal/pin)`|—||의도적으로 연결하지 않고 남겨 둔 단자나 핀의 미연결 표시; 단자는 그대로 유지되므로 버스 비트에 붙은 표시가 어느 비트가 열려 있는지 기록한다(12.6).|
|`$JUNCTION(p: point/terminal)`|—||점이나 단자 위의 접합점(dot).|
|`$CURRENT_ARROW(p: terminal/point)`|—||단자나 점 위의 전류 화살표.|
|`$ARROW(at: point)`|—||심볼 안이나 장면 속의 점에 그려지는 화살표 머리(9.2.7).|
|`$SPDT_SWITCH(at: point)`|—||넷리스트 줄 없이 점 위에 도형으로 그려지는 단극 쌍투 스위치.|
|`$FIGURE(at: point)`|—||넷리스트 줄 없이 점 위에 그려지는 도형으로, SET-SYMBOL로 자체 심볼을 지정한다.|
|`$PORT_LIST()`|—||한 레벨에서 아직 `$IO_PORT`가 없는 모든 포트 넷에 붙는 포트 태그로, rnl-std-fill이 그리듯 그 레벨 인스턴스들보다 적어도 6단위 왼쪽의 한 열에 위에서 아래로 차례대로 그룹으로 묶어, 각각 R0으로 핀이 오른쪽을 향하게, 모두 우선순위 low로 배치된다. 이를 쓰면 자동 보충은 그 레벨에 아무것도 추가하지 않는다(12.4).|
|`$COMMENT({target}; text="")`|—||임의의 객체에 부착되거나 target을 생략하면 레벨 자체에 부착되는, 그려지지 않는 주석.|
|`$GROUND_STUB(t: pin; gap=3)`|—||자체 배선을 가진 핀 근처의 똑바로 선 접지: 핀과 같은 높이이거나 그 아래에 놓이며, 세로 방향 거리는 최대 gap 단위이다(우선순위 low).|
|`$SUPPLY_STUB(t: pin; gap=3)`|—||자체 배선을 가진 핀 근처의 똑바로 선 전원: 핀과 같은 높이이거나 그 위에 놓이며, 세로 방향 거리는 최대 gap 단위이다(우선순위 low).|
|`$BIAS_STUB(t: pin)`|—||핀에 직접 붙는 바이어스 전압 마크로, 포트 스텁처럼 핀 바깥쪽을 향하며, 그 막대는 핀의 위·아래·왼쪽에서는 /로, 오른쪽에서는 \로 기운다.|
|`$IO_PORT_STUB(t: pin)`|—||핀에 직접 붙는 포트 태그로, 리드선이 스텁의 짧은 배선 역할을 하며, 핀과 태그는 `$NET_LABEL_STUB`과 마찬가지로 라우팅되지 않은 채 남는다.|
|`$NET_LABEL_STUB(t: pin)`|—||핀에 직접 붙는 넷 라벨로, 1단위 길이의 리드선이 스텁의 짧은 배선 역할을 하며, 버스 핀에서는 버스를 대표하는 하나의 라벨이 붙고, 핀과 라벨은 라우팅하지 않은 채로 둔다.|
|**rnl-std-semantics**||||
|`$MATCHED({members}: instance/usage)`|의미만||타입이나 정의 및 레이아웃이 정합된 소자 또는 하위 구조.|
|`$MATCHED_MOS({members}: instance)`|의미만||타입, 모델, 채널 길이가 정합된 MOSFET.|
|`$MATCHED_BJT({members}: instance)`|의미만||타입, 모델, 단위 이미터 기하 형상이 정합된 바이폴라 트랜지스터.|
|`$MATCHED_RESISTOR({members}: instance)`|의미만||타입과 모델이 정합된 저항.|
|`$MATCHED_CAPACITOR({members}: instance)`|의미만||타입과 모델이 정합된 커패시터.|
|`$MIRRORED_LAYOUT(left: instance/usage, right: instance/usage, axis: point/instance/usage)`|배치; 자신의 점(중심)||같은 높이에 놓여 축을 기준으로 서로 거울상 대칭을 이루는 두 객체: 레이아웃만 정할 뿐 회로에 관해서는 아무것도 말하지 않는다.|
|`$MIRRORED_VERTICAL_LAYOUT(top: instance/usage, bottom: instance/usage, axis: point/instance/usage)`|배치; 자신의 점(중심)||한 열에 놓여 수평축을 기준으로 서로 거울상 대칭을 이루는 두 객체: 레이아웃만 정한다.|
|`$BRANCH({members}: instance/usage)`|배치, 라우팅; 자신의 점(x)||세로로 쌓인 직렬 구조: 위에서 아래로 한 열에 직렬로 놓인 멤버들; 대개 전원에서 접지로 이어지는 경로 같은 직류 경로(DC 가지)이다.|
|`$BRANCH_HORIZONTAL({members}: instance/usage)`|배치, 라우팅; 자신의 점(y)||행으로 그려진 직렬 구조: 왼쪽에서 오른쪽으로 같은 높이에 직렬로 놓인 멤버들.|
|`$CASCADE({stages}: instance/usage)`|의미만||신호 경로를 따라 종속 연결(캐스케이드)된 단들.|
|`$SERIES({members}: instance/usage)`|라우팅||한 전류 경로를 따라 끝과 끝이 직렬로 연결된 멤버들.|
|`$PARALLEL({members}: instance/usage)`|라우팅||같은 두 넷 사이에 병렬로 연결된 멤버들.|
|`$PARALLEL_HORIZONTAL({members}: instance/usage)`|배치, 라우팅; 자신의 점(중심)||같은 높이에 나란히 놓인 병렬 멤버들로, 열 안에서 한 자리를 차지한다.|
|`$PARALLEL_VERTICAL({members}: instance/usage)`|배치, 라우팅; 자신의 점(중심)||같은 x 좌표에 위아래로 놓인 병렬 멤버들로, 행 안에서 한 자리를 차지한다.|
|`$SIGNAL_FLOW({path}: instance/usage, {source}: instance/usage/terminal/net, {load}: instance/usage/terminal/net)`|의미만||한 방향으로 흐르는 신호 경로: 신호원에서 출발해 경로의 각 단을 적힌 순서대로 거쳐 부하로 이어진다.|
|`$POWER_FLOW({path}: instance/usage, {source}: instance/usage/terminal/net, {load}: instance/usage/terminal/net)`|의미만||전력 경로: 전원이나 배터리 같은 공급원에서 출발해 이를 변환하거나 스위칭하는 경로의 각 단을 적힌 순서대로 거쳐 전력을 사용하는 부하로 이어진다.|
|`$BIAS_CIRCUIT({members}: instance/usage)`|의미만||바이어스 회로: 전류 기준원처럼 다른 부분의 바이어스를 만드는 소자나 구조.|
|`$FEEDBACK_LOOP({forward}: instance/usage, {feedback}: instance/usage)`|의미만||피드백 루프: 순방향 경로와 피드백 회로망.|
|`$DIFFERENTIAL_PAIR_N(left: instance, right: instance, tail: instance/usage)`|배치, 라우팅; 자신의 점(중심)|예: left, right, tail|꼬리 트랜지스터가 아래에 놓이는 n형 트랜지스터 차동쌍.|
|`$DIFFERENTIAL_PAIR_P(left: instance, right: instance, tail: instance/usage)`|배치, 라우팅; 자신의 점(중심)|예: left, right, tail|꼬리 트랜지스터가 위에 놓이는 p형 트랜지스터 차동쌍.|
|`$COMPLEMENTARY_DIFFERENTIAL_PAIR(pairup: usage, pairdown: usage)`|라우팅|예: pairup, pairdown|레일투레일 입력단: 같은 입력을 받는 위쪽 차동쌍과 아래쪽 차동쌍.|
|`$PSEUDO_DIFFERENTIAL_PAIR(left: instance, right: instance)`|배치; 자신의 점(중심)||의사 차동쌍: 꼬리 없이 대칭으로 그려진 두 정합 입력 트랜지스터.|
|`$DEGENERATED_DIFFERENTIAL_PAIR_TYPE_H(left: instance, right: instance, tailleft: instance/usage, tailright: instance/usage, bridge: instance/usage)`|배치, 라우팅; 자신의 점(중심)|예: left, right, tailleft, tailright, bridge|두 꼬리 사이의 브리지로 디제너레이션된 H형 차동쌍.|
|`$DEGENERATED_DIFFERENTIAL_PAIR_TYPE_T_N(left: instance, right: instance, degleft: instance/usage, degright: instance/usage, tail: instance/usage)`|배치, 라우팅; 자신의 점(중심)|예: left, right, degleft, degright, tail|각 입력 아래의 소자로 디제너레이션된 n형 트랜지스터의 T형 차동쌍으로, 꼬리가 아래에 있다.|
|`$DEGENERATED_DIFFERENTIAL_PAIR_TYPE_T_P(left: instance, right: instance, degleft: instance/usage, degright: instance/usage, tail: instance/usage)`|배치, 라우팅; 자신의 점(중심)|예: left, right, degleft, degright, tail|각 입력 위의 소자로 디제너레이션된 p형 트랜지스터의 뒤집힌 T형 차동쌍으로, 꼬리가 위에 있다.|
|`$CROSS_COUPLED_PAIR(left: instance, right: instance)`|배치, 라우팅; 자신의 점(중심)|예: left, right|교차 결합 쌍: 정귀환으로 교차 결합되어 대칭으로 그려진 두 정합 트랜지스터.|
|`$CURRENT_MIRROR_N({ref}: instance, {out}: instance)`|배치; 자신의 점(y)||소스 또는 이미터가 아래를 향하는 n형 트랜지스터의 저전압 측 전류 거울.|
|`$CURRENT_MIRROR_P({ref}: instance, {out}: instance)`|배치; 자신의 점(y)||소스 또는 이미터가 위를 향하는 p형 트랜지스터의 고전압 측 전류 거울.|
|`$CASCODE_CURRENT_MIRROR_N({ref}: instance, {out}: instance, {refcascode}: instance, {outcascode}: instance)`|배치; 자신의 점(y)||저전압 측 캐스코드 전류 거울: 아래쪽 거울 행과 그 위의 캐스코드 행.|
|`$CASCODE_CURRENT_MIRROR_P({ref}: instance, {out}: instance, {refcascode}: instance, {outcascode}: instance)`|배치; 자신의 점(y)||고전압 측 캐스코드 전류 거울: 위쪽 거울 행과 그 아래의 캐스코드 행.|
|`$TRANS_LINEAR_LOOP({a}: instance/usage, {b}: instance/usage)`|배치, 라우팅; 자신의 점(x)|예: a, b|한 열로 그려지는 두 트랜지스터 그룹의 트랜스선형 루프.|
|`$TRANS_LINEAR_LOOP_HORIZONTAL({a}: instance/usage, {b}: instance/usage)`|배치, 라우팅; 자신의 점(y)|예: a, b|한 행으로 그려지는 두 트랜지스터 그룹의 트랜스선형 루프.|
|`$DIODE_CONNECTED(device: instance)`|라우팅||게이트가 드레인 쪽으로, 또는 베이스가 컬렉터 쪽으로 연결된 다이오드 연결 트랜지스터.|
|`$COMMON_SOURCE(device: instance)`|의미만||FET 하나짜리 공통 소스 단.|
|`$COMMON_GATE(device: instance)`|의미만||FET 하나짜리 공통 게이트 단.|
|`$COMMON_DRAIN(device: instance)`|의미만||FET 하나짜리 소스 팔로워(공통 드레인 단).|
|`$COMMON_EMITTER(device: instance)`|의미만||바이폴라 트랜지스터 하나짜리 공통 이미터 단.|
|`$COMMON_BASE(device: instance)`|의미만||바이폴라 트랜지스터 하나짜리 공통 베이스 단.|
|`$COMMON_COLLECTOR(device: instance)`|의미만||바이폴라 트랜지스터 하나짜리 이미터 팔로워(공통 컬렉터 단).|
|`$FOLLOWER(device: instance)`|의미만||트랜지스터 하나짜리 전압 팔로워 단.|
|`$CASCODE(source: instance/usage, cascode: instance)`|라우팅||캐스코드: 공통 게이트 트랜지스터와 직렬로 연결된 소스 소자 또는 하위 구조.|
|`$FOLDED_CASCODE(source: instance/usage, cascode: instance, bias: instance/usage)`|라우팅||폴디드 캐스코드: 바이어스와 함께 공통 게이트 트랜지스터로 접혀 들어간 입력 소자.|
|`$COMMON_SOURCE_AMPLIFIER(input: instance, load: instance/usage)`|라우팅|`$TRANSCONDUCTOR_Z_AMPLIFIER` 경유: input, load|공통 소스 증폭기: FET 입력 트랜지스터와 그 부하.|
|`$COMMON_EMITTER_AMPLIFIER(input: instance, load: instance/usage)`|라우팅|`$TRANSCONDUCTOR_Z_AMPLIFIER` 경유: input, load|공통 이미터 증폭기: 바이폴라 입력 트랜지스터와 그 부하.|
|`$DEGENERATED_COMMON_SOURCE_AMPLIFIER(input: instance, load: instance/usage, degenerator: instance/usage)`|라우팅|`$DEGENERATED_TRANSCONDUCTOR_Z_AMPLIFIER` 경유: input, load, degenerator|디제너레이션된 공통 소스 증폭기: FET 입력 트랜지스터, 그 부하 및 디제너레이션 요소.|
|`$DEGENERATED_COMMON_EMITTER_AMPLIFIER(input: instance, load: instance/usage, degenerator: instance/usage)`|라우팅|`$DEGENERATED_TRANSCONDUCTOR_Z_AMPLIFIER` 경유: input, load, degenerator|디제너레이션된 공통 이미터 증폭기: 바이폴라 입력 트랜지스터, 그 부하 및 디제너레이션 요소.|
|`$MILLER_AMPLIFIER(stage: instance/usage, compensation: instance/usage)`|라우팅|예: stage, compensation|밀러 보상 증폭기: 이득 단과 그 보상 회로망.|
|`$PUSH_PULL_STAGE(pullup: instance/usage, pulldown: instance/usage)`|라우팅|예: pullup, pulldown|푸시풀 출력단: 직렬로 연결된 풀업과 풀다운 출력 트랜지스터들만 가리키며, 그 바이어스와 구동단은 포함하지 않는다.|
|`$TRANSCONDUCTOR_Z_AMPLIFIER(transconductor: instance/usage, load: instance/usage)`|라우팅|예: transconductor, load|증폭기 단: 트랜스컨덕터와 그 부하.|
|`$DEGENERATED_TRANSCONDUCTOR_Z_AMPLIFIER(transconductor: instance/usage, load: instance/usage, degenerator: instance/usage)`|라우팅|예: transconductor, load, degenerator|디제너레이션된 증폭기 단: 트랜스컨덕터, 그 부하 및 디제너레이션 요소.|
|`$PASS_SWITCH(device: instance/usage)`|의미만||양쪽 사이에서 신호를 통과시키거나 차단하는 스위치.|
|`$RESET_SWITCH(device: instance)`|의미만||프리차지, 균등화 또는 리셋 트랜지스터.|
|`$GROUND_SWITCH(device: instance)`|의미만||접지 넷에 연결된 저전압 측 스위치.|
|`$POWER_SWITCH(device: instance)`|의미만||전원 넷에 연결된 고전압 측 스위치.|
|`$TRANSMISSION_GATE(pmos: instance/usage, nmos: instance/usage)`|배치, 라우팅; 자신의 점(중심)|예: pmos, nmos|PMOS가 NMOS 위에 놓이는 CMOS 전송 게이트.|
|`$TRANSMISSION_GATE_HORIZONTAL(pmos: instance/usage, nmos: instance/usage)`|배치, 라우팅; 자신의 점(중심)|예: pmos, nmos|PMOS가 왼쪽에 놓여 나란히 그려지는 CMOS 전송 게이트.|
|`$MULTI_POLE_SWITCH({throws}: instance/usage)`|라우팅|예: throws|하나의 극을 공유하는 여러 접점의 스위치.|
|`$BOOTSTRAPPED_SWITCH(device: instance/usage)`|의미만||부트스트랩 샘플링 스위치.|
|`$SC_SAMPLER({switch}: instance/usage, {capacitor}: instance/usage)`|라우팅||스위치드 커패시터 샘플러: 샘플링 스위치와 샘플링 커패시터.|
|`$CHARGE_SHARING(a: instance/usage, switch: instance/usage, b: instance/usage)`|라우팅||전하 공유 회로망: 스위치를 통해 전하를 공유하는 두 커패시터 또는 커패시터 뱅크.|
|`$CHOPPER({straight}: instance/usage, {crossed}: instance/usage)`|라우팅||초퍼: 신호 극성을 반전시키는 네 개의 스위치 브리지.|
|`$FLYING_CAPACITOR(capacitor: instance/usage)`|의미만||클록 위상 사이에서 전환되는 플라잉 커패시터.|
|`$CMOS_INVERTER(pmos: instance, nmos: instance)`|배치, 라우팅; 자신의 점(중심)|`$CMOS_GATE` 경유: pmos, nmos|PMOS가 NMOS 위에 놓이는 CMOS 인버터.|
|`$CMOS_INVERTER_HORIZONTAL(pmos: instance, nmos: instance)`|배치, 라우팅; 자신의 점(중심)|`$CMOS_GATE` 경유: pmos, nmos|PMOS가 왼쪽에 놓여 나란히 그려지는 CMOS 인버터.|
|`$CMOS_GATE({pullup}: instance/usage, {pulldown}: instance/usage)`|라우팅|예: pullup, pulldown|정적 CMOS 논리 게이트: 출력에서 직렬로 만나는 풀업 회로망과 풀다운 회로망.|
|`$LATCH({members}: instance/usage)`|라우팅|예: members|래치: 쌍안정 코어와 그 주변 소자들.|
|`$INVERTER_LATCH(a: instance/usage, b: instance/usage)`|라우팅|`$LATCH` 경유: a, b|교차 결합된 두 인버터로 이루어진 래치.|
|`$SHIFT_REGISTER({stages}: instance/usage)`|의미만||시프트 레지스터: 플립플롭 또는 래치의 연쇄로, 흔히 겹쳐서 그리며 클록은 거의 배선하지 않는다.|
|`$DIVIDER(upper: instance/usage, lower: instance/usage)`|라우팅||위와 아래 두 직렬 가지로 이루어진 분배기.|
|`$RESISTOR_LADDER({resistors}: instance)`|배치, 라우팅; 자신의 점(x)|예: resistors|위에서 아래로 한 열에 직렬로 놓인 저항 래더.|
|`$RESISTOR_LADDER_HORIZONTAL({resistors}: instance)`|배치, 라우팅; 자신의 점(y)|예: resistors|행으로 그려진 직렬 저항 래더.|
|`$PI_NETWORK(shunt1: instance/usage, series: instance/usage, shunt2: instance/usage)`|라우팅||파이 회로망: 병렬 가지, 직렬 가지, 또 하나의 병렬 가지.|
|`$T_NETWORK(series1: instance/usage, shunt: instance/usage, series2: instance/usage)`|라우팅||T형 회로망: 직렬 가지, 병렬 가지, 또 하나의 직렬 가지.|
|`$LC_TANK_PARALLEL(inductor: instance/usage, capacitor: instance/usage)`|라우팅|예: inductor, capacitor|병렬 LC 공진 탱크.|
|`$RLC_TANK_PARALLEL(resistor: instance/usage, inductor: instance/usage, capacitor: instance/usage)`|라우팅|예: resistor, inductor, capacitor|병렬 RLC 공진 탱크.|
|`$LC_TANK_SERIES(inductor: instance/usage, capacitor: instance/usage)`|라우팅|예: inductor, capacitor|직렬 LC 공진 탱크.|
|`$RLC_TANK_SERIES(resistor: instance/usage, inductor: instance/usage, capacitor: instance/usage)`|라우팅|예: resistor, inductor, capacitor|직렬 RLC 공진 탱크.|
|`$DC_FEED(device: instance/usage)`|의미만||노드에 DC 전원이나 바이어스를 공급하는 RF 초크 또는 바이어스 저항.|
|`$DECAP(device: instance)`|배치; 자신의 점(x)||전원이나 바이어스 노드의 디커플링 커패시터.|
|`$AC_COUPLE(device: instance/usage)`|의미만||AC 신호는 통과시키고 DC는 차단하는 직렬 결합 커패시터.|
|`$OSCILLATOR({members}: instance/usage)`|라우팅|예: members|능동 소자와 공진 또는 지연 요소로 이루어진 발진기.|
|**rnl-std-signal-flow**||||
|`$ADDER(in1: net, in2: net, out: net)`|—||`$ADDER_SYMBOL`로 그려지는 이상적 신호 흐름 가산기(out = in1 + in2).|
|`$SUBTRACTOR(in1: net, in2: net, out: net)`|—||`$SUBTRACTOR_SYMBOL`로 그려지는 이상적 신호 흐름 감산기(out = in1 - in2).|
|`$MULTIPLIER(in1: net, in2: net, out: net)`|—||`$MULTIPLIER_SYMBOL`로 그려지는 이상적 신호 흐름 곱셈기(out = in1 * in2).|
|`$INTEGRATOR(in: net, out: net)`|—||`$INTEGRATOR_SYMBOL`로 그려지는 이상적 연속시간 적분기.|
|`$DISCRETE_TIME_INTEGRATOR(in: net, out: net)`|—||`$DISCRETE_TIME_INTEGRATOR_SYMBOL`로 그려지는 이상적 이산시간 적분기.|
|`$QUANTIZER(in: net, out: net)`|—||`$QUANTIZER_SYMBOL`로 그려지는 이상적 양자화기.|
|`$UNIT_DELAY(in: net, out: net)`|—||`$UNIT_DELAY_SYMBOL`로 그려지는 이상적 단위 지연기.|
|**rnl-std-45deg**||||
|`$ARROW_45(at: point)`|—||심볼 안의 점이나 장면 속 45도 배선 위의 점에 그려지는 45도 화살표 머리.|
|`$CURRENT_ARROW_45(p: terminal/point)`|—||단자나 45도 가지를 따라 놓인 점 위의 45도 전류 화살표.|

### 심볼

핀은 자신이 바인딩하는 단자나 역할 이름으로 적으며, `pin` (`p`)는 역할 `p`에 바인딩된 `pin`이라는 핀이고, `pin` (nothing)은 아무것도 바인딩하지 않는다.

|심볼|핀|의미|
|---|---|---|
|**rnl-std-basic**|||
|`$RESISTOR`|`p` `n`|저항.|
|`$VARIABLE_RESISTOR`|`p` `n`|가변 저항: 화살표가 가로지르는 저항.|
|`$CAPACITOR`|`p` `n`|커패시터.|
|`$VARIABLE_CAPACITOR`|`p` `n`|가변 커패시터: 화살표가 가로지르는 커패시터.|
|`$INDUCTOR`|`p` `n`|인덕터.|
|`$VARIABLE_INDUCTOR`|`p` `n`|가변 인덕터: 화살표가 가로지르는 인덕터.|
|`$TRANSFORMER`|`p1` `n1` `p2` `n2`|변압기: 극성 점이 위쪽에 있고 나란히 서 있는 두 결합 권선.|
|`$POTENTIOMETER`|`n1` `nw` `n2`|와이퍼가 가운데에 있는 3단자 전위차계.|
|`$PHOTORESISTOR`|`p` `n`|포토레지스터(광의존 저항).|
|`$POLARIZED_CAPACITOR`|`p` `n`|p 쪽에 양극판이 있는 유극(전해) 커패시터.|
|`$CRYSTAL`|`n1` `n2`|수정 진동자 공진기.|
|`$SPEAKER`|`p` `n`|스피커.|
|`$VOLTAGE_SOURCE`|`p` `n`|독립 전압원.|
|`$CURRENT_SOURCE`|`p` `n`|화살표가 p에서 n을 향하는 독립 전류원.|
|`$BATTERY`|`p` `n`|배터리.|
|`$CONTROLLED_VOLTAGE_SOURCE`|`p` `n`|출력 핀만 있는 마름모꼴로 그려진 종속 전압원.|
|`$CONTROLLED_CURRENT_SOURCE`|`p` `n`|출력 핀만 있는 마름모꼴로 그려진 종속 전류원.|
|`$VCVS_4TERM`|`p` `n` `cp` `cn`|제어 핀이 왼쪽에 있는 전압 제어 전압원(E).|
|`$VCCS_4TERM`|`p` `n` `cp` `cn`|제어 핀이 왼쪽에 있는 전압 제어 전류원(G).|
|`$BEHAVIORAL_SOURCE`|`p` `n`|동작 전원(B): B가 표기된 원.|
|`$RF_PORT`|`p` `n`|RF 포트: Spectre의 port용으로 내부 저항과 직렬 연결된 정현파 전원.|
|`$SINE_VOLTAGE_SOURCE`|`p` `n`|p 쪽에 +가 있는 정현파 전압원.|
|`$PULSE_VOLTAGE_SOURCE`|`p` `n`|p 쪽에 +가 있는 펄스 전압원.|
|`$MICROPHONE`|`p` `n`|p 쪽에 +가 있는 마이크.|
|`$AMMETER`|`p` `n`|전류를 p에서 n으로 측정하는 전류계.|
|`$SWITCH`|`p` `n`|전류 제어 스위치(W) 또는 Spectre의 switch용으로 열림 상태로 그려진 스위치.|
|`$CLOSED_SWITCH`|`p` `n`|닫힘 상태로 그려진 스위치.|
|`$EXTERNALLY_CONTROLLED_SWITCH`|`p` `n` `cp`|옆면에 단일 제어 핀이 있는 스위치.|
|`$VOLTAGE_CONTROLLED_SWITCH`|`p` `n` `cp` `cn`|차동 제어 핀이 아래에 있는 전압 제어 스위치(S).|
|`$SPDT_SWITCH_SYMBOL`|`com` (nothing) `a` (nothing) `b` (nothing)|`$SPDT_SWITCH`의 심볼: 핀 com, a, b가 아무것도 바인딩하지 않고 이름으로 배선되는 단극 쌍투 스위치(9.2.4).|
|`$TRANSMISSION_LINE`|`p1` `n1` `p2` `n2`|포트 1이 왼쪽에, 포트 2가 오른쪽에 있는 전송선(T 또는 O).|
|`$RC_LINE`|`a` `b` `ref`|균일 분포 RC 선로(U).|
|`$GROUND_SYMBOL`|`pin` (`p`)|넷 0 위의 `$GROUND` 심볼: 핀이 위쪽에 있는 3선 접지.|
|`$NAMED_GROUND_SYMBOL`|`pin` (`p`)|0 이외의 넷 위의 `$GROUND` 심볼: 아래에 넷 이름이 있는 3선 접지.|
|`$SUPPLY_SYMBOL`|`pin` (`p`)|`$SUPPLY`의 심볼: 핀이 아래쪽에 있고 넷 이름이 표시된 수평 막대.|
|`$BIAS_SYMBOL`|`pin` (`p`)|`$BIAS`의 심볼: 핀이 아래쪽에 있고 넷 이름이 표시된 기울어진 막대.|
|`$IO_PORT_SYMBOL`|`pin` (`p`)|`$IO_PORT`의 심볼: 핀이 오른쪽에 있고 넷 이름이 표시된 작은 원.|
|`$NET_LABEL_SYMBOL`|`pin` (`p`)|`$NET_LABEL`의 심볼: 1단위 길이 리드선의 왼쪽에 넷 이름(또는 버스 핀의 넷 이름들)이 있고, `$IO_PORT_SYMBOL`처럼 핀이 오른쪽을 향한다.|
|`$ARROW_SYMBOL`|—|오른쪽을 향한 채워진 화살표 머리로, `$ARROW`의 심볼이다. `$CURRENT_ARROW`는 그 절반 크기로 그린다.|
|`$NO_CONNECT_SYMBOL`|—|`$NO_CONNECT`의 심볼: X자 표기.|
|`$JUNCTION_SYMBOL`|—|`$JUNCTION`의 심볼: 채워진 점.|
|**rnl-std-semiconductors**|||
|`$DIODE`|`a` `k`|다이오드.|
|`$ZENER_DIODE`|`a` `k`|제너 다이오드.|
|`$LED`|`a` `k`|발광 다이오드(LED).|
|`$SCHOTTKY_DIODE`|`a` `k`|쇼트키 장벽 다이오드.|
|`$SHOCKLEY_DIODE`|`a` `k`|쇼클리 4층 다이오드.|
|`$PUT`|`a` `g` `k`|프로그래머블 단일접합 트랜지스터(PUT).|
|`$NMOS`|`d` `g` `s`|바디를 그리지 않은 3핀 NMOS 트랜지스터.|
|`$NMOS_4TERM`|`d` `g` `s` `b`|드레인·소스와 정렬된 바디 핀과 채널을 향하는 바디 화살표가 있는 NMOS 트랜지스터.|
|`$PMOS`|`d` `g` `s`|바디를 그리지 않은 3핀 PMOS 트랜지스터.|
|`$PMOS_4TERM`|`d` `g` `s` `b`|드레인·소스와 정렬된 바디 핀을 가진 PMOS 트랜지스터.|
|`$NMOS_LOGIC`|`d` `g` `s`|소스 화살표가 없는 3핀 논리 NMOS 트랜지스터.|
|`$PMOS_LOGIC`|`d` `g` `s`|소스 화살표가 없고 게이트에 반전 버블이 있는 3핀 논리 PMOS 트랜지스터.|
|`$DEPLETION_NMOS`|`d` `g` `s`|바디를 그리지 않은 공핍형 3핀 NMOS 트랜지스터.|
|`$DEPLETION_NMOS_4TERM`|`d` `g` `s` `b`|드레인·소스와 정렬된 바디 핀을 가진 공핍형 NMOS 트랜지스터.|
|`$DEPLETION_PMOS`|`d` `g` `s`|바디를 그리지 않은 공핍형 3핀 PMOS 트랜지스터.|
|`$DEPLETION_PMOS_4TERM`|`d` `g` `s` `b`|드레인·소스와 정렬된 바디 핀을 가진 공핍형 PMOS 트랜지스터.|
|`$NDMOS`|`d` `g` `s`|바디를 그리지 않은 N형 3핀 DMOS 트랜지스터.|
|`$NDMOS_4TERM`|`d` `g` `s` `b`|드레인·소스와 정렬된 바디 핀을 가진 N형 DMOS 트랜지스터.|
|`$PDMOS`|`d` `g` `s`|바디를 그리지 않은 P형 3핀 DMOS 트랜지스터.|
|`$PDMOS_4TERM`|`d` `g` `s` `b`|드레인·소스와 정렬된 바디 핀을 가진 P형 DMOS 트랜지스터.|
|`$NPN`|`c` `b` `e`|기판을 그리지 않은 NPN 바이폴라 트랜지스터.|
|`$PNP`|`c` `b` `e`|기판을 그리지 않은 PNP 바이폴라 트랜지스터.|
|`$NJFET`|`d` `g` `s`|게이트 화살표가 채널을 향하는 N채널 JFET.|
|`$PJFET`|`d` `g` `s`|게이트 화살표가 채널 바깥쪽을 향하는 P채널 JFET.|
|`$UJT`|`e` `b1` `b2`|b2가 위, b1이 아래에 있는 단일접합 트랜지스터(UJT).|
|**rnl-std-functional**|||
|`$OPAMP`|`inp` `inn` `out`|inp, inn, out 세 단자의 연산 증폭기.|
|`$OPAMP_SUPPLY`|`inp` `inn` `out` `vp` `vn`|전원 핀이 있는 연산 증폭기: inp, inn, out, vp, vn.|
|`$OPAMP_INVERTED_SUPPLY`|`inp` `inn` `out` `vp` `vn`|전원 핀이 있는 연산 증폭기로, 입력을 맞바꾸고(inn 위, inp 아래) 전원은 그대로 둔다(vp 위, vn 아래). `$OPAMP_SUPPLY`를 MX로 반전하면 이렇게 되지 않는다: inp, inn, out, vp, vn.|
|`$DIFFERENTIAL_OPAMP`|`inp` `inn` `outp` `outn`|완전 차동 연산 증폭기: inp, inn, outp, outn.|
|`$DIFFERENTIAL_OPAMP_SUPPLY`|`inp` `inn` `outp` `outn` `vp` `vn`|전원 핀이 있는 완전 차동 연산 증폭기: inp, inn, outp, outn, vp, vn.|
|`$DIFFERENTIAL_OPAMP_INVERTED_SUPPLY`|`inp` `inn` `outp` `outn` `vp` `vn`|전원 핀이 있는 완전 차동 연산 증폭기로, 입력과 출력을 맞바꾸고(inn과 outn 위, inp와 outp 아래) 전원은 그대로 둔다(vp 위, vn 아래). `$DIFFERENTIAL_OPAMP_SUPPLY`를 MX로 반전하면 이렇게 되지 않는다: inp, inn, outp, outn, vp, vn.|
|`$DIFFERENTIAL_OPAMP_TWISTED`|`inp` `inn` `outp` `outn`|출력 극성이 바뀐(outn 위, outp 아래) 완전 차동 연산 증폭기: inp, inn, outp, outn.|
|`$DIFFERENTIAL_OPAMP_TWISTED_SUPPLY`|`inp` `inn` `outp` `outn` `vp` `vn`|교차 출력과 전원 핀이 있는 완전 차동 연산 증폭기: inp, inn, outp, outn, vp, vn.|
|`$DIFFERENTIAL_OPAMP_TWISTED_INVERTED_SUPPLY`|`inp` `inn` `outp` `outn` `vp` `vn`|전원 핀이 있는 완전 차동 연산 증폭기로, 입력을 맞바꾸고(inn 위, inp 아래) 출력은 `$DIFFERENTIAL_OPAMP_SUPPLY`와 같으며(outp 위, outn 아래) 전원은 그대로 둔다(vp 위, vn 아래). `$DIFFERENTIAL_OPAMP_TWISTED_SUPPLY`를 MX로 반전하면 이렇게 되지 않는다: inp, inn, outp, outn, vp, vn.|
|`$COMPARATOR`|`inp` `inn` `out`|inp, inn, out 세 단자의 비교기.|
|`$COMPARATOR_SUPPLY`|`inp` `inn` `out` `vp` `vn`|전원 핀이 있는 비교기: inp, inn, out, vp, vn.|
|`$COMPARATOR_INVERTED_SUPPLY`|`inp` `inn` `out` `vp` `vn`|전원 핀이 있는 비교기로, 입력을 맞바꾸고(inn 위, inp 아래) 전원은 그대로 둔다(vp 위, vn 아래). `$COMPARATOR_SUPPLY`를 MX로 반전하면 이렇게 되지 않으며, 계단 기호는 `$COMPARATOR_SUPPLY`와 같게 읽힌다: inp, inn, out, vp, vn.|
|`$VOLTAGE_AMPLIFIER`|`in` `out`|key가 label인 선택적 중앙 텍스트가 있는 전압 증폭기: in, out.|
|`$VOLTAGE_AMPLIFIER_SUPPLY`|`in` `out` `vp` `vn`|전원 핀이 있는 전압 증폭기: in, out, vp, vn.|
|`$TRANSCONDUCTOR`|`in` `out`|key가 label인 중앙 텍스트(기본값 g_m)가 있는 트랜스컨덕터(gm 셀): in, out.|
|`$TRANSCONDUCTOR_SUPPLY`|`in` `out` `vp` `vn`|전원 핀과 key가 label인 중앙 텍스트(기본값 g_m)가 있는 트랜스컨덕터: in, out, vp, vn.|
|`$DIFFERENTIAL_TRANSCONDUCTOR`|`inp` `inn` `out`|차동 입력과 key가 label인 텍스트(기본값 g_m)가 있는 트랜스컨덕터: inp, inn, out.|
|`$DIFFERENTIAL_TRANSCONDUCTOR_SUPPLY`|`inp` `inn` `out` `vp` `vn`|전원 핀과 key가 label인 텍스트(기본값 g_m)가 있는 차동 입력 트랜스컨덕터: inp, inn, out, vp, vn.|
|`$DIFFERENTIAL_TRANSCONDUCTOR_INVERTED_SUPPLY`|`inp` `inn` `out` `vp` `vn`|전원 핀과 key가 label인 텍스트(기본값 g_m)가 있는 차동 입력 트랜스컨덕터로, 입력을 맞바꾸고(inn 위, inp 아래) 전원은 그대로 둔다(vp 위, vn 아래). `$DIFFERENTIAL_TRANSCONDUCTOR_SUPPLY`를 MX로 반전하면 이렇게 되지 않는다: inp, inn, out, vp, vn.|
|`$ADC`|`in` `out`|아날로그-디지털 변환기: in, out.|
|`$ADC_SUPPLY`|`in` `out` `vp` `vn`|전원 핀이 있는 아날로그-디지털 변환기: in, out, vp, vn.|
|`$DAC`|`in` `out`|디지털-아날로그 변환기: in, out.|
|`$DAC_SUPPLY`|`in` `out` `vp` `vn`|전원 핀이 있는 디지털-아날로그 변환기: in, out, vp, vn.|
|`$CHOPPER_SYMBOL`|`inp` `inn` `outp` `outn`|초퍼 변조기 블록(스위치 브리지 구조 `$CHOPPER`와 별개): inp, inn, outp, outn.|
|**rnl-std-rf**|||
|`$CENTER_TAP_INDUCTOR`|`n1` `n2` `tap`|센터탭이 있는 인덕터.|
|`$T_COIL`|`n1` `n2` `tap`|T-코일: 센터탭과 브리지 커패시터를 가진 두 결합 코일.|
|**rnl-std-logic**|||
|`$AND_2INPUT`|`a` `b` `y`|2입력 AND 게이트: a, b, y.|
|`$AND_3INPUT`|`a` `b` `c` `y`|3입력 AND 게이트: a, b, c, y.|
|`$NAND_2INPUT`|`a` `b` `y`|2입력 NAND 게이트: a, b, y.|
|`$NAND_3INPUT`|`a` `b` `c` `y`|3입력 NAND 게이트: a, b, c, y.|
|`$OR_2INPUT`|`a` `b` `y`|2입력 OR 게이트: a, b, y.|
|`$OR_3INPUT`|`a` `b` `c` `y`|3입력 OR 게이트: a, b, c, y.|
|`$NOR_2INPUT`|`a` `b` `y`|2입력 NOR 게이트: a, b, y.|
|`$NOR_3INPUT`|`a` `b` `c` `y`|3입력 NOR 게이트: a, b, c, y.|
|`$XOR_2INPUT`|`a` `b` `y`|2입력 XOR 게이트: a, b, y.|
|`$XOR_3INPUT`|`a` `b` `c` `y`|3입력 XOR 게이트: a, b, c, y.|
|`$XNOR_2INPUT`|`a` `b` `y`|2입력 XNOR 게이트: a, b, y.|
|`$XNOR_3INPUT`|`a` `b` `c` `y`|3입력 XNOR 게이트: a, b, c, y.|
|`$AND_4INPUT`|`a` `b` `c` `d` `y`|정수 격자점 위에 핀이 있는 4입력 AND 게이트: a, b, c, d, y.|
|`$NAND_4INPUT`|`a` `b` `c` `d` `y`|정수 격자점 위에 핀이 있는 4입력 NAND 게이트: a, b, c, d, y.|
|`$OR_4INPUT`|`a` `b` `c` `d` `y`|정수 격자점 위에 핀이 있는 4입력 OR 게이트: a, b, c, d, y.|
|`$NOR_4INPUT`|`a` `b` `c` `d` `y`|정수 격자점 위에 핀이 있는 4입력 NOR 게이트: a, b, c, d, y.|
|`$XOR_4INPUT`|`a` `b` `c` `d` `y`|정수 격자점 위에 핀이 있는 4입력 XOR 게이트: a, b, c, d, y.|
|`$XNOR_4INPUT`|`a` `b` `c` `d` `y`|정수 격자점 위에 핀이 있는 4입력 XNOR 게이트: a, b, c, d, y.|
|`$BUFFER`|`a` `y`|버퍼: a, y.|
|`$INVERTER`|`a` `y`|인버터(NOT 게이트): a, y.|
|`$DELAY_CELL`|`a` `y`|지연 셀: a, y.|
|`$D_FLIP_FLOP`|`d` `ck` `q` `qb`|출력 Q와 반전 출력을 가진 D 플립플롭: d, ck, q, qb.|
|`$D_FLIP_FLOP_Q_ONLY`|`d` `ck` `q`|출력 Q만 있는 D 플립플롭: d, ck, q.|
|`$D_FLIP_FLOP_WITH_RESET`|`d` `ck` `rst` `q` `qb`|리셋이 있는 D 플립플롭: d, ck, rst, q, qb.|
|`$AND_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|전원 핀이 있는 2입력 AND 게이트: a, b, y, vp, vn.|
|`$AND_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|전원 핀이 있는 3입력 AND 게이트: a, b, c, y, vp, vn.|
|`$AND_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|전원 핀이 있는 4입력 AND 게이트: a, b, c, d, y, vp, vn.|
|`$NAND_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|전원 핀이 있는 2입력 NAND 게이트: a, b, y, vp, vn.|
|`$NAND_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|전원 핀이 있는 3입력 NAND 게이트: a, b, c, y, vp, vn.|
|`$NAND_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|전원 핀이 있는 4입력 NAND 게이트: a, b, c, d, y, vp, vn.|
|`$OR_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|전원 핀이 있는 2입력 OR 게이트: a, b, y, vp, vn.|
|`$OR_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|전원 핀이 있는 3입력 OR 게이트: a, b, c, y, vp, vn.|
|`$OR_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|전원 핀이 있는 4입력 OR 게이트: a, b, c, d, y, vp, vn.|
|`$NOR_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|전원 핀이 있는 2입력 NOR 게이트: a, b, y, vp, vn.|
|`$NOR_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|전원 핀이 있는 3입력 NOR 게이트: a, b, c, y, vp, vn.|
|`$NOR_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|전원 핀이 있는 4입력 NOR 게이트: a, b, c, d, y, vp, vn.|
|`$XOR_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|전원 핀이 있는 2입력 XOR 게이트: a, b, y, vp, vn.|
|`$XOR_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|전원 핀이 있는 3입력 XOR 게이트: a, b, c, y, vp, vn.|
|`$XOR_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|전원 핀이 있는 4입력 XOR 게이트: a, b, c, d, y, vp, vn.|
|`$XNOR_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|전원 핀이 있는 2입력 XNOR 게이트: a, b, y, vp, vn.|
|`$XNOR_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|전원 핀이 있는 3입력 XNOR 게이트: a, b, c, y, vp, vn.|
|`$XNOR_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|전원 핀이 있는 4입력 XNOR 게이트: a, b, c, d, y, vp, vn.|
|`$BUFFER_SUPPLY`|`a` `y` `vp` `vn`|전원 핀이 있는 버퍼: a, y, vp, vn.|
|`$INVERTER_SUPPLY`|`a` `y` `vp` `vn`|전원 핀이 있는 인버터: a, y, vp, vn.|
|`$DELAY_CELL_SUPPLY`|`a` `y` `vp` `vn`|전원 핀이 있는 지연 셀: a, y, vp, vn.|
|`$D_FLIP_FLOP_SUPPLY`|`d` `ck` `q` `qb` `vp` `vn`|전원 핀이 있는 D 플립플롭: d, ck, q, qb, vp, vn.|
|`$D_FLIP_FLOP_Q_ONLY_SUPPLY`|`d` `ck` `q` `vp` `vn`|출력 Q만 있고 전원 핀이 있는 D 플립플롭: d, ck, q, vp, vn.|
|`$D_FLIP_FLOP_WITH_RESET_SUPPLY`|`d` `ck` `rst` `q` `qb` `vp` `vn`|리셋과 전원 핀이 있는(vp와 vn이 x = 0에 정렬된) D 플립플롭: d, ck, rst, q, qb, vp, vn.|
|**rnl-std-signal-flow**|||
|`$ADDER_SYMBOL`|`a` (`in1`) `b` (`in2`) `y` (`out`)|`$ADDER`의 심볼: 입력 리드선 a (in1)와 b (in2)에 화살표 머리가 있고 출력 y (out)를 갖는 합산 원.|
|`$SUBTRACTOR_SYMBOL`|`a` (`in1`) `b` (`in2`) `y` (`out`)|`$SUBTRACTOR`의 심볼: 가운데에 +가 있고 음의 입력 b (in2) 옆 바깥쪽에 -가 있으며, 입력 a (in1)·b (in2)와 출력 y (out)를 갖는 합산 원.|
|`$MULTIPLIER_SYMBOL`|`a` (`in1`) `b` (`in2`) `y` (`out`)|`$MULTIPLIER`의 심볼: 입력 리드선 a (in1)와 b (in2)에 화살표 머리가 있고 출력 y (out)를 갖는 곱셈 원.|
|`$INTEGRATOR_SYMBOL`|`a` (`in`) `y` (`out`)|`$INTEGRATOR`의 심볼: 1/s가 표기되고 입력 a (in)와 출력 y (out)를 갖는 상자.|
|`$DISCRETE_TIME_INTEGRATOR_SYMBOL`|`a` (`in`) `y` (`out`)|`$DISCRETE_TIME_INTEGRATOR`의 심볼: z 영역 전달 함수가 표기되고 입력 a (in)와 출력 y (out)를 갖는 상자.|
|`$QUANTIZER_SYMBOL`|`a` (`in`) `y` (`out`)|`$QUANTIZER`의 심볼: 계단형이 그려지고 입력 a (in)와 출력 y (out)를 갖는 상자.|
|`$UNIT_DELAY_SYMBOL`|`a` (`in`) `y` (`out`)|`$UNIT_DELAY`의 심볼: z의 -1승이 표기되고 입력 a (in)와 출력 y (out)를 갖는 상자.|
|**rnl-std-45deg**|||
|`$RESISTOR_45`|`p` `n`|45도 저항.|
|`$CAPACITOR_45`|`p` `n`|45도 커패시터.|
|`$INDUCTOR_45`|`p` `n`|45도 인덕터(대각선 변압기나 발룬 권선에도 쓰인다).|
|`$SWITCH_45`|`p` `n`|45도 열림 스위치.|
|`$VOLTAGE_SOURCE_45`|`p` `n`|p 쪽의 +와 n 쪽의 -가 똑바로 유지되는 45도 독립 전압원.|
|`$CURRENT_SOURCE_45`|`p` `n`|화살표가 p에서 n을 향하는 45도 독립 전류원.|
|`$DIODE_45`|`a` `k`|45도 다이오드.|
|`$ZENER_DIODE_45`|`a` `k`|45도 제너 다이오드.|
|`$SCHOTTKY_DIODE_45`|`a` `k`|45도 쇼트키 다이오드.|
|`$LED_45`|`a` `k`|45도 발광 다이오드(LED).|
|`$NMOS_45`|`d` `g` `s`|45도 3핀 NMOS 트랜지스터.|
|`$NMOS_4TERM_45`|`d` `g` `s` `b`|바디가 (0, 0)에 있는 45도 4핀 NMOS 트랜지스터.|
|`$PMOS_45`|`d` `g` `s`|45도 3핀 PMOS 트랜지스터.|
|`$PMOS_4TERM_45`|`d` `g` `s` `b`|바디가 (0, 0)에 있는 45도 4핀 PMOS 트랜지스터.|
|`$NPN_45`|`c` `b` `e`|45도 NPN 바이폴라 트랜지스터.|
|`$PNP_45`|`c` `b` `e`|45도 PNP 바이폴라 트랜지스터.|
|`$ARROW_45_SYMBOL`|—|-45도(R90에서 45도 심볼의 p->n / a->k 방향을 따르는 오른쪽 아래)를 향하며, R0/R90/R180/R270에 따라 네 대각선 방향 모두로 회전하는 채워진 화살표 머리.|

### 집합과 스타일

|이름|패키지|의미|
|---|---|---|
|`$GROUND_NET_SET`|rnl-std-basic|접지 넷 집합으로, 기본값은 0이며 파일이 ADD-TO와 REMOVE-FROM으로 편집한다. rnl-std-fill은 그 넷들의 배선되지 않은 핀에 접지 스텁을 붙이고 라우팅하지 않은 채 둔다. 접지가 어떻게 그려지는지는 이 집합에 좌우되지 않는다.|
|`$SUPPLY_NET_SET`|rnl-std-basic|전원 넷 집합으로, 기본값은 비어 있으며 파일이 ADD-TO와 REMOVE-FROM으로 편집한다.|
|`$BIAS_NET_SET`|rnl-std-basic|바이어스 넷 집합으로, 기본값은 비어 있으며 파일이 ADD-TO와 REMOVE-FROM으로 편집한다.|
|`$PORT_FILL_SET`|rnl-std-basic|rnl-std-fill이 포트 태그를 붙이는 포트 넷 집합: 모든 레벨의 모든 포트 넷(전원 넷과 접지 넷 포함)에서 `$IO_PORT`가 작성된 넷과 `$PORT_LIST`가 있는 레벨의 넷을 제외한 집합.|
|`$GROUND_FILL_SET`|rnl-std-fill|접지 넷 위의 배선되지 않은 핀들로, 자동 보충이 접지 스텁을 붙인다.|
|`$SUPPLY_FILL_SET`|rnl-std-fill|전원 넷 위의 배선되지 않은 핀들로, 자동 보충이 전원 스텁을 붙인다.|
|`$BIAS_FILL_SET`|rnl-std-fill|바이어스 넷 위의 배선되지 않은 핀들로, 자동 보충이 바이어스 스텁을 붙인다.|
|`$LABEL_FILL_SET`|rnl-std-fill|나머지 넷 위의 배선되지 않은 핀들로, 자동 보충이 넷 라벨 스텁을 붙이며 버스 핀에는 라벨 하나만 붙는다.|
|`$base`|rnl-std-style|모든 객체의 기본 스타일: Analog Canvas가 그리는 선, 텍스트, 채우기.|
|`$bus`|rnl-std-style|버스 스타일: 선을 굵게 그린다.|
|`$signal`|rnl-std-style|신호 배선 스타일: 파란색에 굵게 그린다.|
|`$note`|rnl-std-style|메모 스타일: 회색에 더 작은 텍스트.|
|`$grey`|rnl-std-style|선과 텍스트를 위한 회색 스타일.|

<!-- lists:end -->
