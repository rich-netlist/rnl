## 부록 A 키워드 시그니처(규범적)

이 부록은 단어 목록 `rnl-words-0.4.json`에서 생성되었으며(5.3), 둘이 다를 때는 이 부록의 정오 사항(erratum)이다. 각 키워드가 무엇을 받고 무엇을 내놓는지를 나열하며, 그 의미는 마지막 열에 적힌 절에 있다.

- **슬롯**은 표준 순서대로 나열하며, 슬롯 이름(역할로 쓸 수 있음, 5.3)과 받는 대상을 함께 적는다. *하나(one)*: 단일 객체; 집합이 주어지면 멤버마다 키워드를 적용하며(5.2.1), 다만 PRIORITY는 GROUP이 표시된 집합을 그룹 자체로 취한다(14.8). *집합(set)*: 집합; 슬롯에 나열된 낱개 객체들은 하나의 집합을 이룬다(5.2.1). *임의(any)*: 하나의 객체 또는 집합이며 키워드가 정한 대로 사용한다(DISTANCE에서는 집합이 각 멤버를 대신함, 14.5). *도형(shape)*: 하나의 점 또는 점들의 집합(도형의 두 형식, 9.2.5). *본문(body)*: 중괄호 안의 구문들로, 감싸기로 소비된다(5.4). *정의(definition)*: 정의, 즉 `$` 이름, 정의에 바인딩된 `%` 이름 또는 익명 정의(5.6, 6.6). 흡수 입력은 엄격하고, 감싸기 입력은 관용적이다(5.4, 7.3).
- **슬롯의 종류**는 종류 이름(`instance`, `terminal`, `pin`, `net`, `point`, `wire`, `usage`, `bundle`, `text`, `annotation`, `label`)이거나 다음과 같다: *임의(any)*, 임의 객체; *레벨(level)*, 단독 `#`: 레벨 자체; *소유 객체(owner)*, 단독 `?`: 심볼 본문에서는 심볼로 그려지는 객체; 일반 정의 본문에서는 확장 중인 용례(9.4); *위치(position)*, 객체의 하위 객체 `::position`(그 원점); *위치 지정 가능(locatable)*, 위치를 가진 객체: 인스턴스, 번들, 핀, 핀 있는 단자, 점, 텍스트, 중심 기준으로 쓴 도형의 중심, 자기 자신의 점 기준의 용례; 여기에 주어진 집합은 각 멤버를 대신한다(14.1); *방향 지정 가능(orientable)*, 방향을 가진 객체: 인스턴스, 번들, 심볼 있는 용례; *제약(constraint)*, 제약 키워드가 만든 객체; *집합(set)*, 이름으로 쓴 집합; ASSERT-TOPOLOGY에서는 직접 쓴 `{…}`인 항목도 받음(15.4); *빈 노드(empty node)*, 단독 `&`: 넷 없음; 종류 `net`의 역할과 WIRE의 net 슬롯이 받으며 그 밖의 곳에서는 넷들의 빈 집합으로 취급됨(6.3).
- **매개변수** 중 *위치 1*, *위치 2*로 표시된 것은 그 순서대로 위치로 줄 수 있고, 나머지는 이름으로만 줄 수 있다(4.2). `priority`는 항상 `"disabled"`, `"low"`, `"normal"`, `"high"`를 받으며, 쓰지 않으면 레코드 헤더의 우선순위가 적용되되 AS와 ATTACH는 거기서 `"disabled"`만 취한다(14.8).
- **결과**: 구문이 내놓는 객체(5.5)와 그 사실이 덮어쓰는 방식(5.8). 덮어씀이 없으면 사실들이 누적된다.
- 달리 적히지 않은 한 키워드는 한 레벨의 모든 레코드에 쓸 수 있다. *심볼 본문에서도 가능*은 DEFINE-SYMBOL 본문 안에도 쓸 수 있음을 뜻한다.

### A.1 레코드

`@RNL(…) {…}`은 자신의 구문들을 레벨에 커밋한다(5.4). 레코드 헤더 필드(2.8)는 모두 이름으로 준다.

| 필드 | 값 | 기본값 | 해석에 영향 |
|---|---|---|---|
| `type` | `"authored"`, `"generated"`, `"header"` | `"authored"` | 예 |
| `scope` | 문자열 | — | 예 |
| `priority` | `"disabled"`, `"low"`, `"normal"`, `"high"` | `"normal"` | 예 |
| `author` | 문자열 | — | 아니오 |
| `hash` | 문자열 | — | 아니오 |
| `numbering` | 문자열 | — | 예 |
| `modified` | 문자열 | — | 아니오 |
| `language` | `"spice"`, `"spectre"` | — | 예 |

### A.2 헤더 레코드와 패키지

| 키워드 | 슬롯 | 매개변수 | 결과 | 절 |
|---|---|---|---|---|
| `@VERSION` (헤더 레코드 전용) | — | `version` (위치 1; 필수) | — | 16.1 |
| `@PACKAGE` (패키지 파일의 헤더 레코드 전용) | — | `name` (필수), `version` (필수) | — | 2.7 |
| `@USE-PACKAGE` (헤더 레코드에서도 가능) | `body`: 본문, 선택 | `name` (필수), `version`, `path` | — | 16.2 |
| `@ALIAS` (헤더 레코드 전용) | — | `alias` (위치 1; 필수), `original` (위치 2; 필수), `definition` | — | 16.5 |
| `@USE-ALIASES` (헤더 레코드 전용) | — | `name` (필수), `version`, `path` | — | 16.5 |
| `@DEVICE` (헤더 레코드 전용) | — | `name`, `model`, `terminals` (배열), `type` (문자열); `name`과 `model` 중 적어도 하나; `terminals`와 `type` 중 적어도 하나 | — | 6.12 |

### A.3 정의

| 키워드 | 슬롯 | 매개변수 | 결과 | 절 |
|---|---|---|---|---|
| `@DEFINE-SYMBOL` (심볼 본문에서도 가능) | `body`: 본문 | `box` (배열) | 정의 (심볼) | 9.2 |
| `@DEFINE-STYLE` | `extends`: 본문 (스타일) | 스타일 속성 (9.3) | 정의 (스타일) | 9.3 |
| `@DEFINE` | `body`: 본문 | `roles` (필수; 배열), `parameters` (배열) | 정의 (일반) | 9.4 |

### A.4 심볼 내용과 도형

| 키워드 | 슬롯 | 매개변수 | 결과 | 절 |
|---|---|---|---|---|
| `@PIN` (심볼 본문 전용) | `position`: 하나 (점) | `terminal` (단자, 또는 비어 있지 않은 단자 배열; 생략 가능), `label`, `direction` (`"right"`, `"up"`, `"left"`, `"down"`) | 새 핀 | 9.2.4 |
| `@LINE` (심볼 본문에서도 가능) | `points`: 집합 (점, 핀, 단자) | `arrow` (`"none"`, `"start"`, `"end"`, `"both"`; 기본값 `"none"`), `fill` (`"none"`, `"linecolor"`, `"fillcolor"`; 기본값 `"none"`), 선 스타일 속성 | 심볼의 도형; 장면에서는 새 어노테이션 (13.4) | 9.2.5 |
| `@RECTANGLE` (심볼 본문에서도 가능) | `points`: 도형 (점, 핀, 단자) | `width` (수), `height` (수), `fill` (`"none"`, `"linecolor"`, `"fillcolor"`; 기본값 `"none"`), 영역 스타일 속성; `width`와 `height`는 함께 쓰거나 모두 생략 | 심볼의 도형; 장면에서는 새 어노테이션 (13.4) | 9.2.5 |
| `@CIRCLE` (심볼 본문에서도 가능) | `points`: 도형 (점, 핀, 단자) | `radius` (수), `fill` (`"none"`, `"linecolor"`, `"fillcolor"`; 기본값 `"none"`), 영역 스타일 속성 | 심볼의 도형; 장면에서는 새 어노테이션 (13.4) | 9.2.5 |
| `@ELLIPSE` (심볼 본문에서도 가능) | `points`: 도형 (점, 핀, 단자) | `width` (수), `height` (수), `fill` (`"none"`, `"linecolor"`, `"fillcolor"`; 기본값 `"none"`), 영역 스타일 속성; `width`와 `height`는 함께 쓰거나 모두 생략 | 심볼의 도형; 장면에서는 새 어노테이션 (13.4) | 9.2.5 |
| `@ARC` (심볼 본문에서도 가능) | `points`: 집합 (점, 핀, 단자) | `arrow` (`"none"`, `"start"`, `"end"`, `"both"`; 기본값 `"none"`), `fill` (`"none"`, `"linecolor"`, `"fillcolor"`; 기본값 `"none"`), 선 스타일 속성 | 심볼의 도형; 장면에서는 새 어노테이션 (13.4) | 9.2.5 |
| `@BEZIER` (심볼 본문에서도 가능) | `points`: 집합 (점, 핀, 단자) | `arrow` (`"none"`, `"start"`, `"end"`, `"both"`; 기본값 `"none"`), `fill` (`"none"`, `"linecolor"`, `"fillcolor"`; 기본값 `"none"`), 선 스타일 속성 | 심볼의 도형; 장면에서는 새 어노테이션 (13.4) | 9.2.5 |
| `@ANNOTATION` | `at`: 하나 (위치 지정 가능); `source`: 하나 (심볼, 일반 정의 또는 용례), 마지막에 작성 | `rotate` (수, 반시계 방향 각도(도); 기본값 `0`), `mirror` (진릿값; 기본값 `false`), 스타일 속성 및 `source`로 주어진 심볼이나 일반 정의의 매개변수 (이름으로 전달, 9.4) | 새 어노테이션 | 13.5 |
| `@TEXT` (심볼 본문에서도 가능) | `attach`: 하나 (점, 인스턴스, 용례, 번들, 핀, 단자, 배선, 어노테이션, 넷, `?`) | `text` (위치 1; 필수), `key`, `dx` (수), `dy` (수), `halign` (`"start"`, `"middle"`, `"end"`; 기본값 `"start"`), `valign` (`"baseline"`, `"top"`, `"middle"`, `"bottom"`; 기본값 `"baseline"`), `rotate` (수, 반시계 방향 각도(도); 기본값 `0`), `fix_rotate` (진릿값; 기본값 `false`), `upright` (진릿값; 기본값 `true`), 텍스트 스타일 속성 | 새 텍스트; `key`가 있으면 부착된 객체의 하위 객체; key로 덮어씀 (key: key가 주어졌을 때 부착된 객체와 key) | 13.3 |

### A.5 배치와 외관

| 키워드 | 슬롯 | 매개변수 | 결과 | 절 |
|---|---|---|---|---|
| `@PLACE` (심볼 본문에서도 가능) | `target`: 하나 (인스턴스, 용례, 번들, 점, 텍스트, `::position`) | `x` (위치 1; 수), `y` (위치 2; 수), `priority`; `x`, `y` 중 적어도 하나 | 입력 객체; 기하 사실 (key: 객체와 축) | 11.1 |
| `@ORIENT` (심볼 본문에서도 가능) | `target`: 하나 (인스턴스, 용례, 번들, 핀) | `code` (위치 1; 필수; `"R0"`, `"R90"`, `"R180"`, `"R270"`, `"MX"`, `"MY"`, `"MXR90"`, `"MYR90"`, 또는 앞의 네 개에 대한 단어 `"left"`, `"down"`, `"right"`, `"up"`, 또는 이들의 비어 있지 않은 배열), `priority` | 입력 객체; 기하 사실 (key: 객체, 또는 핀) | 11.2 |
| `@LAYER` | `target`: 하나 (인스턴스, 용례, 번들, 배선, 어노테이션, 텍스트) | `layer` (위치 1; 필수; `"bottom"`, `"middle"`, `"top"`) | 입력 객체; key로 덮어씀 (key: 객체) | 11.6 |
| `@SET-SYMBOL` (심볼 본문에서도 가능) | `target`: 하나 (인스턴스, 용례, 번들, `#`); `symbol`: 정의 (심볼) | `terminals` (단자 이름의 배열, 또는 쌍의 배열) | 입력 객체; 외관 대기열 | 11.3 |
| `@SET-STYLE` | `target`: 하나 (인스턴스, 용례, 번들, 배선, 어노테이션, 텍스트); `style`: 정의 (스타일) | — | 입력 객체; 외관 대기열 | 11.3 |

### A.6 선택식

| 키워드 | 슬롯 | 매개변수 | 결과 | 절 |
|---|---|---|---|---|
| `@SELECT-INSTANCE` | `scope`: 임의 (`#`, 임의 객체) | `name`, `model`, `type`, `terminals` (수, 정수), `symbol` (문자열: `*`를 포함할 수 있는 `$` 이름), `drawn` (진릿값), `role`, `recursive` (진릿값; 기본값 `false`) | 새 인스턴스 집합 | 10.1 |
| `@SELECT-TERMINAL` | `scope`: 임의 (`#`, 임의 객체) | `name`, `model`, `type`, `terminal` (단자), `net`, `current` (`"in"`, `"out"`, `"unspecified"`), `role`, `recursive` (진릿값; 기본값 `false`) | 새 단자 집합 | 10.1 |
| `@SELECT-PIN` | `scope`: 임의 (`#`, 임의 객체) | `name`, `model`, `type`, `terminal` (단자), `net`, `wired` (진릿값), `role`, `recursive` (진릿값; 기본값 `false`) | 새 핀 집합 | 10.1 |
| `@SELECT-NET` | `scope`: 임의 (`#`, 임의 객체) | `name`, `port` (진릿값), `wired` (진릿값), `global` (진릿값), `role`, `recursive` (진릿값; 기본값 `false`) | 새 넷 집합 | 10.1 |
| `@SELECT-WIRE` | `scope`: 임의 (`#`, 임의 객체) | `net`, `bus` (진릿값), `role`, `recursive` (진릿값; 기본값 `false`) | 새 배선 집합 | 10.1 |
| `@SELECT-LABEL` | `scope`: 임의 (`#`, 임의 객체) | `key`, `role`, `recursive` (진릿값; 기본값 `false`) | 새 라벨 집합 | 10.1 |
| `@SELECT-ANNOTATION` | `scope`: 임의 (`#`, 임의 객체) | `role`, `recursive` (진릿값; 기본값 `false`) | 새 어노테이션 집합 | 10.1 |
| `@SELECT-USAGE` | `scope`: 임의 (`#`, 임의 객체); `definition`: 정의 (일반 정의), 선택 | `symbol` (문자열: `*`를 포함할 수 있는 `$` 이름), `drawn` (진릿값), `recursive` (진릿값; 기본값 `false`) | 새 용례 집합 | 10.1 |
| `@SELECT-ANCESTOR-USAGE` | `scope`: 임의 (임의 객체); `definition`: 정의 (일반 정의) | `recursive` (진릿값; 기본값 `false`) | 새 용례 집합 | 10.1 |

### A.7 정의 적용

| 키워드 | 슬롯 | 매개변수 | 결과 | 절 |
|---|---|---|---|---|
| `@AS` (심볼 본문에서도 가능) | 정의의 역할들, 이어서 정의 (`definition`, 마지막에 작성) | 정의의 매개변수 (매개변수 목록 순서대로 위치로, 또는 이름으로; 9.4), `priority` | 새 용례 | 9.4 |
| `@ATTACH` (심볼 본문에서도 가능) | 정의, 이어서 그 역할들 (`definition`, 먼저 작성) | 정의의 매개변수 (매개변수 목록 순서대로 위치로, 또는 이름으로; 9.4), `priority` | 새 용례 | 9.4 |

### A.8 배선

| 키워드 | 슬롯 | 매개변수 | 결과 | 절 |
|---|---|---|---|---|
| `@WIRE` | `net`: 임의 (넷, 또는 빈 노드); `points`: 집합 (점, 핀; 단자는 자기 핀을 대표함, 5.2.4) | `mode` (`"auto"`, `"straight"`, `"horizontal-vertical"`, `"vertical-horizontal"`, `"manhattan"`, `"manhattan45"`; 기본값 `"auto"`), 배선 스타일 속성 | 새 배선; 배선이 주어지면 그 배선을 다시 쓰며 key로 덮어씀 (key: 그 배선) | 13.1 |
| `@ROUTE-NET` | `net`: 하나 (넷) | `mode` (`"auto"`, `"straight"`, `"horizontal-vertical"`, `"vertical-horizontal"`, `"manhattan"`, `"manhattan45"`; 기본값 `"auto"`), 배선 스타일 속성, `priority` | 새 경로 | 13.7 |
| `@ROUTE-OBJECT` | `objects`: 집합 (임의 객체) | `mode` (ROUTE-NET과 동일), 배선 스타일 속성, `priority` | 입력 객체 | 13.7 |
| `@UNROUTE-NET` | `net`: 하나 (넷) | `priority` | 입력 객체 | 13.7 |
| `@UNROUTE-OBJECT` | `objects`: 집합 (임의 객체) | `priority` | 입력 객체 | 13.7 |

### A.9 집합, 그룹 및 번들

| 키워드 | 슬롯 | 매개변수 | 결과 | 절 |
|---|---|---|---|---|
| `@BUNDLE` | `members`: 집합 (인스턴스) | — | 새 번들 | 11.8 |
| `@GROUP` | `members`: 집합 (임의 객체) | `spacing_x` (수, 정수), `spacing_y` (수, 정수) | 입력 객체 | 11.5 |
| `@ADD-TO` | `items`: 임의 (임의 객체); `target`: 임의 (집합) | — | 대상 집합 | 5.2.2 |
| `@REMOVE-FROM` | `items`: 임의 (임의 객체); `target`: 임의 (집합) | — | 대상 집합 | 5.2.2 |
| `@UNION` | `sets`: 집합 (임의 객체) | — | 새 집합 | 5.2.2 |

### A.10 제약과 우선순위

| 키워드 | 슬롯 | 매개변수 | 결과 | 절 |
|---|---|---|---|---|
| `@ALIGN` | `members`: 집합 (위치 지정 가능) | `axis` (위치 1; 필수; `"x"`, `"y"`), `priority` | 새 제약; 기하 사실 | 14.2 |
| `@ORDER` | `members`: 집합 (위치 지정 가능) | `direction` (위치 1; 필수; `"left-to-right"`, `"right-to-left"`, `"bottom-to-top"`, `"top-to-bottom"`), `priority` | 새 제약; 기하 사실 | 14.3 |
| `@DISTRIBUTE` | `members`: 집합 (위치 지정 가능) | `direction` (위치 1; 필수; `"left-to-right"`, `"right-to-left"`, `"bottom-to-top"`, `"top-to-bottom"`), `priority` | 새 제약; 기하 사실 | 14.4 |
| `@DISTANCE` | `from`: 임의 (위치 지정 가능); `to`: 임의 (위치 지정 가능) | `direction` (위치 1; 필수; `"left-to-right"`, `"right-to-left"`, `"bottom-to-top"`, `"top-to-bottom"`), `min` (수), `max` (수), `exact` (수), `priority`; `min`, `max`, `exact` 중 적어도 하나; `exact`는 `min`과 함께 쓰지 않음; `exact`는 `max`와 함께 쓰지 않음 | 새 제약; 기하 사실 | 14.5 |
| `@ORIENT-RELATION` | `from`: 하나 (방향 지정 가능, 또는 핀); `to`: 하나 (방향 지정 가능, 또는 핀; `from`과 같은 종류) | `relation` (위치 1; 필수; ORIENT의 여덟 코드(단어는 불가), 또는 이들의 비어 있지 않은 배열), `priority` | 새 제약; 기하 사실 | 14.9 |
| `@TABLE` | `rows`: 집합 (위치 지정 가능; 중첩된 집합은 표의 목록임) | `transpose` (진릿값; 기본값 `false`), `priority` | 새 제약; 기하 사실 | 14.11 |
| `@SOLVER-CONFIG` (`language` 없는 헤더 레코드, 패키지 파일의 헤더 레코드 및 레벨) | — | 솔버의 이름 붙은 매개변수: 수, 문자열, 진릿값 및 이들의 배열; `spacing_x`, `spacing_y` (수, 정수)와 `junction` (문자열: `$` 이름; 레벨 레코드에서는 그 레벨의 `%` 이름도 가능; 또는 빈 문자열)은 표준이 이름지음; 다른 key들은 솔버의 것 | 객체 없음; 솔버의 설정 | 14.10 |
| `@PRIORITY` | `target`: 하나 (제약, 용례, 집합) | `priority` (위치 1; 필수) | 입력 객체; key로 덮어씀 (key: 객체) | 14.8 |

### A.11 전기적 의미

| 키워드 | 슬롯 | 매개변수 | 결과 | 절 |
|---|---|---|---|---|
| `@CURRENT` | `terminal`: 하나 (단자) | `direction` (위치 1; 필수; `"in"`, `"out"`, `"unspecified"`) | 입력 객체; key로 덮어씀 (key: 단자) | 15.2 |
| `@ASSERT-INSTANCE` | `members`: 집합 (인스턴스) | `role`과 `recursive`를 뺀 SELECT-INSTANCE의 조건들, `has` (속성 이름의 배열), `same` (속성 이름의 배열), `different` (속성 이름의 배열), `message` | 객체 없음; 검사 | 15.3 |
| `@ASSERT-TERMINAL` | `members`: 집합 (단자) | `role`과 `recursive`를 뺀 SELECT-TERMINAL의 조건들, `same`, `different`, `message` | 객체 없음; 검사 | 15.3 |
| `@ASSERT-NET` | `members`: 집합 (넷) | `role`과 `recursive`를 뺀 SELECT-NET의 조건들, `same`, `different`, `message` | 객체 없음; 검사 | 15.3 |
| `@ASSERT-USAGE` | `members`: 집합 (용례); `definition`: 정의 (일반 정의), 선택 | `symbol`, `drawn` (진릿값), `same`, `different` (배열; `"definition"`만 가능), `message` | 객체 없음; 검사 | 15.3 |
| `@ASSERT-IN` | `members`: 집합 (임의 객체); `set`: 임의 (집합) | `message` | 객체 없음; 검사 | 15.3 |
| `@ASSERT-TOPOLOGY` | `members`: 집합 (인스턴스, 번들, 용례, 집합) | `topology` (위치 1; 필수; `"series"`, `"parallel"`, `"star"`, `"loop"`), `exclude` (타입, 단자 쌍의 배열), `message` | 객체 없음; 검사 | 15.4 |

### A.12 기타 표준 단어

- 종류: `instance`, `terminal`, `pin`, `net`, `point`, `wire`, `usage`, `bundle`, `text`, `annotation`, `label`(5.3, 9.4).
- `::` 뒤의 하위 객체: `position`, `net`(6.5).
- `${…}`가 읽는 속성: `name`, `value`, `model`, `type`, `net`, `x`, `y`(13.6).
- 진릿값: `true`, `false`(3.7).

스타일 속성(9.3):

| 속성 | 타입과 값 | 기본값 |
|---|---|---|
| `linecolor` | 색상 | — |
| `linestyle` | `"solid"`, `"dashed"`, `"dotted"`, `"dash-dot"`, `"double"`, `"bold"`, `"none"` | `"solid"` |
| `linewidth` | 수 | `0` |
| `cap` | `"butt"`, `"round"`, `"square"` | — |
| `join` | `"miter"`, `"round"`, `"bevel"` | — |
| `fillcolor` | 색상 | — |
| `textcolor` | 색상 | — |
| `size` | 수 | — |
| `font` | 문자열 | — |
| `bold` | 진릿값 | — |
| `italic` | 진릿값 | — |
| `arrowsize` | 수 | — |
| `textvisible` | 진릿값 | `true` |

키워드가 매개변수로 받는 스타일 속성(9.2.5): *선*: `linecolor`, `linestyle`, `linewidth`, `cap`, `join`, `fillcolor`, `arrowsize`; *영역*: `linecolor`, `linestyle`, `linewidth`, `cap`, `join`, `fillcolor`; *배선*: `linecolor`, `linestyle`, `linewidth`, `cap`, `join`; *텍스트*: `textcolor`, `size`, `font`, `bold`, `italic`, `textvisible`.
