/* RNL language at a glance and examples · interactions of those two sections of the language tour */
(function () {
  'use strict';

  var S = window.RNLSchem, SC = window.RNLScenes, C = window.RNLCode, PG = window.RNLPage;
  var $ = PG.$, $$ = PG.$$, icon = PG.icon, onVisible = PG.onVisible, setPressed = PG.setPressed;
  var REDUCE = S.REDUCE;

  /* ================= 07 Language ================= */

  var AN = {
    pref: ['*', '<en>The host’s comment prefix. SPICE uses * in column 1, Spectre // or *, followed by a space; the RNL itself does not depend on the netlist format.</en>'],
    rec: ['@RNL', '<en>Record entry. Only content in a legal comment that starts with @RNL is RNL.</en>'],
    src: ['(author="designer")', '<en>The record header applies to every statement inside. author says who wrote it; the solver’s records have type="generated" and priority high.</en>'],
    brace: ['{ }', '<en>The record’s content goes in braces, with statements separated by semicolons; even a single statement needs the braces.</en>'],
    free: ['<en>Free text</en>', '<en>“Input pair:”, “and” and “treated” are for people; machines do not read them. Following the standard sentences, a statement reads as a sentence.</en>'],
    name: ['%dp1 =', '<en>Naming: the object this statement makes (a usage) is called dp1. Names starting with % are visible only on this level, and each is bound once per level.</en>'],
    inst: ['<en>#M1, #M2</en>', '<en>References instances in the netlist. Everything starting with # is a netlist object; RNL references it and never creates it.</en>'],
    role: ['<en>(in_p), (in_n)</en>', '<en>Roles: parentheses right after an object say which role of the definition it fills. A statement marks every role or none; without marks, objects fill the roles in order.</en>'],
    kw: ['@AS', '<en>Keyword: AS applies the definition after it to the objects before it, meaning they form a differential pair together. A statement has at most one keyword.</en>'],
    def: ['$DIFFERENTIAL_PAIR', '<en>Definition: $DIFFERENTIAL_PAIR comes from the standard package and has roles but no drawing: it says what something is, not how to draw it.</en>'],
    semi: [';', '<en>A semicolon ends the statement. Each statement takes effect, or fails, on its own.</en>']
  };

  var FAMS = [
    ['<en>Placement</en>', ['PLACE', 'ORIENT', 'LAYER', 'GROUP']],
    ['<en>Constraints</en>', ['ALIGN', 'ORDER', 'DISTRIBUTE', 'DISTANCE', 'ORIENT-RELATION', 'PRIORITY']],
    ['<en>Appearance</en>', ['SET-SYMBOL', 'SET-STYLE', 'DEFINE-SYMBOL', 'DEFINE-STYLE']],
    ['<en>Primitives</en>', ['PIN', 'LINE', 'RECTANGLE', 'CIRCLE', 'ELLIPSE', 'ARC', 'BEZIER', 'TEXT']],
    ['<en>Wires</en>', ['WIRE', 'ROUTE', 'BUNDLE']],
    ['<en>Definitions</en>', ['DEFINE', 'AS', 'ATTACH', 'CURRENT']],
    ['<en>Selection</en>', ['SELECT-INSTANCE', 'SELECT-TERMINAL', 'SELECT-NET', 'SELECT-WIRE', 'SELECT-LABEL', 'SELECT-ANNOTATION', 'SELECT-USAGE', 'SELECT-ANCESTOR-USAGE']],
    ['<en>Sets</en>', ['ADD-TO', 'REMOVE-FROM', 'UNION']],
    ['<en>Header</en>', ['VERSION', 'ALIAS', 'USE-ALIASES', 'DEVICE', 'USE-PACKAGE', 'PACKAGE']]
  ];

  /* keyword: [description, English standard sentence] */
  var KW = {
    'PLACE': ['<en>Puts the origin at (x, y); one axis may be left to the solver. x points right and y up.</en>', '#M1 is at @PLACE(10, 20)'],
    'ORIENT': ['<en>Orientation, one of eight codes: R0, R90 (90° counterclockwise), R180, R270, MY (flipped left-right), MX (flipped upside down), MYR90, MXR90. Without it, the solver chooses.</en>', '#M2 @ORIENT("MY"), i.e., flipped left-right'],
    'LAYER': ['<en>The drawing layer: bottom, middle (the default) or top.</en>', '#C1 is on @LAYER("bottom")'],
    'GROUP': ['<en>The solver keeps this set together as one block; its name still refers to the set.</en>', '%s1_set = {#M1 #M2 #M5} as a @GROUP'],
    'ALIGN': ['<en>Equal coordinates on one axis: "y" puts the members in a row, "x" in a column.</en>', '{#M1 #M2} @ALIGN("y"), i.e., same y'],
    'ORDER': ['<en>Strictly ordered in a direction, which must be written; "x" or "y" fixes only the axis, either way round.</en>', '{#M1 #M2 #M3} are in @ORDER("left-to-right")'],
    'DISTRIBUTE': ['<en>Equal steps along an axis.</en>', 'the x of {#R1 #R2 #R3} @DISTRIBUTE("x"), i.e., in equal steps'],
    'DISTANCE': ['<en>The distance between two objects or groups, with min, max or exact. A direction fixes which comes first; "x" or "y" does not.</en>', 'from #M1 to #M2, they have @DISTANCE("bottom-to-top", min=4)'],
    'ORIENT-RELATION': ['<en>An orientation relation: the second’s orientation is the first’s with this transform applied. "MY" makes them mirror images without saying which way either faces.</en>', 'from #M1 to #M2, they have @ORIENT-RELATION("MY"), i.e., flipped left-right'],
    'PRIORITY': ['<en>Changes the priority of a constraint or usage afterwards: disabled, low, normal or high. In a conflict the lower one gives way.</en>', '%c1 has @PRIORITY("disabled")'],
    'SET-SYMBOL': ['<en>Uses a symbol. What is written directly on an object beats what a rule gives; on # inside a subcircuit, it sets the block symbol.</en>', '{#M1 #M2} @SET-SYMBOL to $NMOS'],
    'SET-STYLE': ['<en>Uses a style, merged property by property: a style changes only the properties it defines.</en>', '{#CC #RZ} @SET-STYLE to $orange'],
    'DEFINE-SYMBOL': ['<en>Defines a symbol: pins, primitives and text, with (0, 0) as its origin; it can include other symbols.</en>', '$TRI = @DEFINE-SYMBOL {…}'],
    'DEFINE-STYLE': ['<en>Defines a style: line color, line style, line width, fill color, text color, size and so on; it can be based on other styles.</en>', '$wide = @DEFINE-STYLE(linewidth=2) based on {$orange}'],
    'PIN': ['<en>A pin in a symbol: where a terminal is drawn and wires connect, with the direction they leave in.</en>', 'at (-4, 0) there is a @PIN(terminal="in", direction="left")'],
    'LINE': ['<en>A polyline: two points make a segment, more make a polyline; arrows and fill are optional.</en>', 'a @LINE running through {(0, 0) (4, 0)}'],
    'RECTANGLE': ['<en>A rectangle: a center with width and height, or two corners.</en>', 'centered at (0, 0), a @RECTANGLE(width=4, height=2)'],
    'CIRCLE': ['<en>A circle: a center and a radius.</en>', 'centered at (0, 0), a @CIRCLE(radius=1)'],
    'ELLIPSE': ['<en>An ellipse: a center with width and height, or two corners of its box.</en>', 'centered at (0, 0), an @ELLIPSE(width=4, height=2)'],
    'ARC': ['<en>An arc through three points.</en>', 'an @ARC running through {(0, 2) (2, 0) (0, -2)}'],
    'BEZIER': ['<en>A Bézier curve with 3n+1 control points.</en>', 'a @BEZIER with control points {(0, 0) (1, 1) (2, 1) (3, 0)}'],
    'TEXT': ['<en>Text: on an object it is a label that moves with it; on a point it is an annotation. key names it, and a later text with the same key replaces an earlier one.</en>', 'on #M1 there is a @TEXT("W/L = ${w}/${l}", key="wl", dy=-3)'],
    'WIRE': ['<en>One wire: a net and an ordered list of points; wires connect only by sharing a point or terminal. mode sets the path style.</en>', 'net &n1 has a @WIRE connecting {#R1:2 %j #R2:1}'],
    'ROUTE': ['<en>Draws a whole net as wires; the solver decides branches, paths and junction dots.</en>', 'nets {&a &b} are fully @ROUTE'],
    'BUNDLE': ['<en>A bundle: several instances drawn as one symbol, such as a group XI<3:0>; each of its pins stands for the members’ terminals of that name.</en>', '#XI<3:0> is drawn as a @BUNDLE => named %xi'],
    'DEFINE': ['<en>A definition: roles, parameters and a body, applied with AS or ATTACH. With an empty body it only records what something is.</en>', '$PAIR = a definition, @DEFINE(roles=["a", "b"], parameters=[["gap", 2]]) {…}'],
    'AS': ['<en>Applies a definition to objects that together form it, such as a differential pair or a mirrored pair. Makes a usage, which can be named and selected.</en>', '#M1(in_p) and #M2(in_n) are treated @AS $DIFFERENTIAL_PAIR'],
    'ATTACH': ['<en>Applies a definition written first, adding it onto objects, such as a ground, a port tag, a junction dot or a no-connect mark.</en>', '$GROUND is @ATTACH to &vss => it is at @PLACE(24, 38) => named %g0'],
    'CURRENT': ['<en>The current direction at a terminal: in, out or unspecified; it affects only the drawing, with no circuit analysis.</en>', '#M5:1 has @CURRENT("in")'],
    'SELECT-INSTANCE': ['<en>Selects instances by name, model or type; from a net, it selects the instances with a pin on it.</en>', 'among the instances at each level in # and below, @SELECT-INSTANCE(model="nch") => they @SET-SYMBOL to $NMOS'],
    'SELECT-TERMINAL': ['<en>Selects terminals by name, by net, or by whether they are wired.</en>', 'among the terminals connected to $GROUND_NET_SET, @SELECT-TERMINAL(wired=false) => $GROUND_STUB is @ATTACH to them'],
    'SELECT-NET': ['<en>Selects nets by name, port, or whether they are wired; from instances or terminals, it selects their nets.</en>', 'among the nets connected to #M1:g, @SELECT-NET => they are fully @ROUTE'],
    'SELECT-WIRE': ['<en>Selects wires; bus=true picks only buses.</en>', 'among the wires at each level in # and below, @SELECT-WIRE(bus=true) => they @SET-STYLE to $bus'],
    'SELECT-LABEL': ['<en>Selects labels, optionally by key.</en>', 'among the labels attached to #M1, @SELECT-LABEL => they @SET-STYLE to $grey'],
    'SELECT-ANNOTATION': ['<en>Selects annotations: text on points and graphics in the drawing.</en>', 'among the annotations at each level in # and below, @SELECT-ANNOTATION => they @SET-STYLE to $base'],
    'SELECT-USAGE': ['<en>Selects the usages of a definition, looking inward; without a definition, all usages.</en>', 'among the usages at each level in # and below, @SELECT-USAGE of $DIFFERENTIAL_PAIR => they have @PRIORITY("disabled")'],
    'SELECT-ANCESTOR-USAGE': ['<en>Selects the usages that have these objects as members, looking outward.</en>', 'among the usages containing #M1, @SELECT-ANCESTOR-USAGE of $DIFFERENTIAL_PAIR => named %dp_set'],
    'ADD-TO': ['<en>Adds members to a named set, such as vdd to the supply nets, so the standard package adds supply symbols for it.</en>', '&vdd @ADD-TO $SUPPLY_NET_SET'],
    'REMOVE-FROM': ['<en>Removes members from a named set, such as no net name under the grounds on vss.</en>', '&vss @REMOVE-FROM $NAMED_GROUND_NET_SET'],
    'UNION': ['<en>A union without duplicates.</en>', '%u_set = %a_set and %b_set @UNION'],
    'VERSION': ['<en>The language version of the file. Only in the header record; without it, the tool’s default version applies.</en>', '* @RNL(type="header") { @VERSION("0.3")}'],
    'ALIAS': ['<en>Declares another spelling (an alias) for a standard word or a $ name, alias first. Only in the header record.</en>', /*zh-CN*/'@ALIAS("@位于", "@PLACE")'/*/zh-CN*/],
    'USE-ALIASES': ['<en>Loads an alias package, such as the Chinese package rnl-zh. Only in the header record.</en>', '@USE-ALIASES(name="rnl-zh", version="0.3")'],
    'DEVICE': ['<en>Names the terminals of devices the netlist does not define, such as PDK devices, so #XM1:d works. Only in the header record.</en>', '@DEVICE(name="XM*", model="sky130_fd_pr__nfet_*", terminals=["d", "g", "s", "b"])'],
    'USE-PACKAGE': ['<en>Uses a package: a set of symbols, styles, definitions and rules, such as a company’s drawing style.</en>', '@USE-PACKAGE(name="acme-style", version="1.2")'],
    'PACKAGE': ['<en>A package file declares its name and version in its header record; not needed when describing a circuit.</en>', '@PACKAGE(name="acme-style", version="1.2")']
  };

  /* The Chinese standard sentence and alias of each keyword, from the alias package rnl-zh. The header
     keywords have none: the header record is always read in standard spelling. Shown on every page. */
  /*zh-CN*/
  var KW_ZH = {
    'PLACE': ['#M1 @位于(10, 20)', '@位于'],
    'ORIENT': ['#M2 @朝向("MY")，即左右镜像', '@朝向'],
    'LAYER': ['#C1 处于 @图层("底")', '@图层'],
    'GROUP': ['%s1_set = {#M1 #M2 #M5} 视作一个 @分组', '@分组'],
    'ALIGN': ['{#M1 #M2} @对齐("y")，即 y 坐标相同', '@对齐'],
    'ORDER': ['{#M1 #M2 #M3} 按 @顺序("从左到右") 排列', '@顺序'],
    'DISTRIBUTE': ['{#R1 #R2 #R3} 的 x 坐标 @等距("x")', '@等距'],
    'DISTANCE': ['#M1 到 #M2 有 @距离("从下到上", 最小值=4)', '@距离'],
    'ORIENT-RELATION': ['#M1 到 #M2 的 @朝向关系("MY")，即左右镜像', '@朝向关系'],
    'PRIORITY': ['%c1 的 @优先级("停用")', '@优先级'],
    'SET-SYMBOL': ['{#M1 #M2} @使用符号 $NMOS', '@使用符号'],
    'SET-STYLE': ['{#CC #RZ} @使用样式 $orange', '@使用样式'],
    'DEFINE-SYMBOL': ['$TRI = @定义符号 {…}', '@定义符号'],
    'DEFINE-STYLE': ['$wide = @定义样式(线宽=2)，以 {$orange} 为基础', '@定义样式'],
    'PIN': ['在 (-4, 0) 有一个 @引脚(端子="in", 方向="左")', '@引脚'],
    'LINE': ['一条 @折线 经过 {(0, 0) (4, 0)}', '@折线'],
    'RECTANGLE': ['以 (0, 0) 为中心的 @矩形(宽度=4, 高度=2)', '@矩形'],
    'CIRCLE': ['以 (0, 0) 为中心的 @圆(半径=1)', '@圆'],
    'ELLIPSE': ['以 (0, 0) 为中心的 @椭圆(宽度=4, 高度=2)', '@椭圆'],
    'ARC': ['一条 @圆弧 经过 {(0, 2) (2, 0) (0, -2)}', '@圆弧'],
    'BEZIER': ['一条 @贝塞尔 曲线，控制点为 {(0, 0) (1, 1) (2, 1) (3, 0)}', '@贝塞尔'],
    'TEXT': ['#M1 上有 @文本("W/L = ${w}/${l}", 引用名="wl", dy=-3)', '@文本'],
    'WIRE': ['网络 &n1 的 @导线 连接了 {#R1:2 %j #R2:1}', '@导线'],
    'ROUTE': ['网络 {&a &b} 全部 @布线', '@布线'],
    'BUNDLE': ['#XI<3:0> @叠画 => 记作 %xi', '@叠画'],
    'DEFINE': ['$PAIR = 一个普通定义， @定义(角色表=["a", "b"], 参数表=[["gap", 2]]) {…}', '@定义'],
    'AS': ['#M1(正输入) 和 #M2(负输入) @视作 $"差分对"', '@视作'],
    'ATTACH': ['$"地" @附着 在 &vss 上 => 其 @位于(24, 38) => 记作 %g0', '@附着'],
    'CURRENT': ['#M5:1 有 @电流("流入")', '@电流'],
    'SELECT-INSTANCE': ['从本层 # 及以下每一层的元件中 @选出元件(模型="nch") => 其 @使用符号 $NMOS', '@选出元件'],
    'SELECT-TERMINAL': ['从接在 $"地网络集" 上的端子中 @选出端子(已接线=否) => $"接地桩" @附着 在其上', '@选出端子'],
    'SELECT-NET': ['从 #M1:g 所在的网络中 @选出网络 => 其全部 @布线', '@选出网络'],
    'SELECT-WIRE': ['从本层 # 及以下每一层的导线中 @选出导线(总线=是) => 其 @使用样式 $"总线"', '@选出导线'],
    'SELECT-LABEL': ['从附着在 #M1 上的标签中 @选出标签 => 其 @使用样式 $"灰色"', '@选出标签'],
    'SELECT-ANNOTATION': ['从本层 # 及以下每一层的标注中 @选出标注 => 其 @使用样式 $"基础"', '@选出标注'],
    'SELECT-USAGE': ['从本层 # 及以下每一层的用例中 @选出用例 $"差分对" => 其 @优先级("停用")', '@选出用例'],
    'SELECT-ANCESTOR-USAGE': ['从包含 #M1 的用例中 @选出所处用例 $"差分对" => 记作 %dp_set', '@选出所处用例'],
    'ADD-TO': ['&vdd @加入 $"电源网络集"', '@加入'],
    'REMOVE-FROM': ['&vss @移出 $"带标签地网络集"', '@移出'],
    'UNION': ['%u_set = %a_set 和 %b_set 的 @并集', '@并集'],
    'USE-PACKAGE': ['@使用包(名字="acme-style", 版本="1.2")', '@使用包']
  };
  /*/zh-CN*/

  var PIPE = [
    ['<en>Read</en>', '<en>Tokens, grammar and keyword signatures. A malformed statement becomes a “failed object” here; it stays in place and does not affect other statements.</en>'],
    ['<en>Names</en>', '<en>Resolves #, &, %, $ references, allowing use before definition; evaluates the bodies of symbols and styles.</en>'],
    ['<en>Objects</en>', '<en>Points, wires, text, primitives, constraints, usages and sets; the bodies applied by AS and ATTACH are expanded. Selections are not evaluated yet.</en>'],
    ['<en>Symbols</en>', '<en>SET-SYMBOL takes effect in order: the standard package’s rules first, the file’s after; what is written directly on an object beats what a rule gives.</en>'],
    ['<en>Generate</en>', '<en>AS and ATTACH applied to selections run once per object, at each level, such as the standard package adding a ground for every unwired ground terminal.</en>'],
    ['<en>New symbols</en>', '<en>The same symbol queue runs again for the objects just generated.</en>'],
    ['<en>Geometry</en>', '<en>PLACE, ORIENT and constraints are taken one by one, by priority and then by order, dropping any that conflict; objects without a position go to the solver.</en>'],
    ['<en>Styles</en>', '<en>SET-STYLE merges property by property; colors and widths a symbol leaves open get their values only now.</en>'],
    ['<en>Render</en>', '<en>Draws the final effective description.</en>']
  ];

  function initLanguage() {
    var line = $('#an-line');
    if (!line) return;
    var tip = $('#an-tip');
    var IDLE = tip.innerHTML;
    var toks = $$('.tok', line);
    function show(tok) {
      toks.forEach(function (t) { t.classList.remove('is-on'); });
      if (!tok) { tip.innerHTML = IDLE; return; }
      var k = tok.getAttribute('data-k');
      toks.forEach(function (t) { if (t.getAttribute('data-k') === k) t.classList.add('is-on'); });
      var a = AN[k];
      tip.innerHTML = '<span class="k">' + a[0] + '</span><span>' + a[1] + '</span>';
    }
    toks.forEach(function (t) {
      t.addEventListener('mouseenter', function () { show(t); });
      t.addEventListener('focus', function () { show(t); });
    });
    line.addEventListener('mouseleave', function () { show(null); });
    PG.initSeg($('#an-seg'), 'data-m', function (v) { line.classList.toggle('machine', v === 'machine'); });

    var fams = $('#fams'), card = $('#kw-card');
    fams.innerHTML = FAMS.map(function (f, i) {
      return '<div class="fam"><span>' + f[0] + '</span><div>' + f[1].map(function (k) {
        return '<button type="button" class="kw" data-f="' + i + '" aria-pressed="false">@' + k + '</button>';
      }).join('') + '</div></div>';
    }).join('');
    var kws = $$('.kw', fams);
    $('#kw-count').textContent = kws.length;
    function pick(b) {
      setPressed(kws, b);
      var k = b.textContent.slice(1);
      var info = KW[k], zh = KW_ZH[k];
      var fam = FAMS[+b.getAttribute('data-f')][0];
      card.innerHTML = '<h4>@' + k + '</h4>' +
        '<div class="kw-chips"><span class="chip chip-rnl">' + fam + '</span>' +
        (zh ? '<span class="chip" title="rnl-zh"><span class="muted"><en>Chinese alias</en></span> <code lang="zh-CN">' + zh[1] + '</code></span>' :
          '<span class="chip chip-mute"><en>The header is read in standard spelling, without aliases</en></span>') + '</div>' +
        '<p>' + info[0] + '</p>' +
        '<div class="kw-ex"><span class="kw-ex-k"><en>Standard sentence</en></span><div class="minicode">' + C.hlAuto(info[1]) + '</div></div>' +
        (zh ? '<div class="kw-ex"><span class="kw-ex-k"><en>Chinese (rnl-zh)</en></span><div class="minicode" lang="zh-CN">' + C.hlAuto(zh[0]) + '</div></div>' : '');
    }
    kws.forEach(function (b) { b.addEventListener('click', function () { pick(b); }); });
    pick(kws.filter(function (b) { return b.textContent === '@AS'; })[0] || kws[0]);

    var pipe = $('#pipe'), pcap = $('#pipe-cap');
    pipe.innerHTML = PIPE.map(function (p, i) {
      return '<button type="button" data-i="' + i + '"><span class="n">' + (i + 1) + '</span>' + p[0] + '</button>';
    }).join('');
    var pbtns = $$('button', pipe), pi = 0, hover = false, pvis = false;
    function setP(i) {
      pi = i;
      pbtns.forEach(function (b, j) { b.classList.toggle('is-on', j === i); b.setAttribute('aria-pressed', j === i ? 'true' : 'false'); });
      pcap.innerHTML = '<b>' + (i + 1) + ' · ' + PIPE[i][0] + '</b> ' + PIPE[i][1];
    }
    pbtns.forEach(function (b, j) {
      b.addEventListener('mouseenter', function () { hover = true; setP(j); });
      b.addEventListener('focus', function () { hover = true; setP(j); });
      b.addEventListener('click', function () { hover = true; setP(j); });
    });
    pipe.addEventListener('mouseleave', function () { hover = false; });
    setP(0);
    onVisible(pipe, function (v) { pvis = v; });
    if (!REDUCE) {
      setInterval(function () {
        if (hover || !pvis || document.hidden) return;
        setP((pi + 1) % PIPE.length);
      }, 2800);
    }
  }

  /* ================= 08 Examples ================= */

  /* The custom NMOS symbol of example 3, drawn primitive by primitive in its local coordinates */
  function e3Drawing(host) {
    var el = S.el, pts = S.ptsAttr;
    var box = [-5, -3.6, 3.6, 3.6];
    var svg = el('svg', { class: 'sc e3', viewBox: S.viewBoxOf(box), role: 'img', 'aria-label': '<en>The NMOS symbol definition, broken down by primitive</en>' }, host);
    var defs = el('defs', null, svg);
    var pat = el('pattern', { id: 'e3dots', x: -0.5, y: -0.5, width: 1, height: 1, patternUnits: 'userSpaceOnUse' }, defs);
    el('circle', { cx: 0.5, cy: 0.5, r: 0.05, class: 'sc-griddot' }, pat);
    el('rect', { x: box[0], y: -box[3], width: box[2] - box[0], height: box[3] - box[1], fill: 'url(#e3dots)' }, svg);
    el('line', { class: 'e3-axis', x1: -4.7, y1: 0, x2: 3.1, y2: 0 }, svg);
    el('line', { class: 'e3-axis', x1: 0, y1: 3.4, x2: 0, y2: -3.4 }, svg);
    el('text', { class: 'e3-ax', x: 3.2, y: 0.05 }, svg).textContent = 'x';
    el('text', { class: 'e3-ax', x: 0.18, y: -3.25 }, svg).textContent = 'y';
    el('rect', { class: 'e3-box', 'data-ref': 'box', x: -3, y: -2, width: 3, height: 4 }, svg);
    function P(ref, tag, attrs) { attrs['data-ref'] = ref; return el(tag, attrs, svg); }
    P('l1', 'polyline', { class: 'sp e3-p', points: pts([[-3, 0], [-1.4, 0]]) });
    P('l2', 'polyline', { class: 'sp e3-p', points: pts([[-1.4, -1.1], [-1.4, 1.1]]) });
    P('l3', 'polyline', { class: 'sp em e3-p', points: pts([[-0.8, -1.4], [-0.8, 1.4]]) });
    P('l4', 'polyline', { class: 'sp e3-p', points: pts([[-0.8, 1], [0, 1], [0, 2]]) });
    P('l5', 'polyline', { class: 'sp e3-p', points: pts([[-0.8, -1], [0, -1], [0, -2]]) });
    P('poly', 'polygon', { class: 'sp fill e3-p', points: pts([[-0.2, -1], [-0.7, -0.7], [-0.7, -1.3]]) });
    [['pin-d', [0, 2], [0, 2.9], 'D', [0.35, 2.55], 'start'], ['pin-g', [-3, 0], [-3.9, 0], 'G', [-3.35, 0.5], 'end'], ['pin-s', [0, -2], [0, -2.9], 'S', [0.35, -2.6], 'start']].forEach(function (p) {
      var g = el('g', { class: 'e3-pin', 'data-ref': p[0] }, svg);
      el('line', { class: 'e3-dir', x1: p[1][0], y1: -p[1][1], x2: p[2][0], y2: -p[2][1] }, g);
      el('circle', { cx: p[1][0], cy: -p[1][1], r: 0.22 }, g);
      el('text', { x: p[4][0], y: -p[4][1], 'text-anchor': p[5] }, g).textContent = p[3];
    });
    var tg = el('g', { class: 'e3-text', 'data-ref': 'text' }, svg);
    el('rect', { x: 0.45, y: -1.65, width: 1.9, height: 0.9, rx: 0.12 }, tg);
    el('text', { x: 0.6, y: -1.2 }, tg).textContent = 'M1';
    return svg;
  }

  function e3Highlight(svg, ref) {
    $$('.is-hl', svg).forEach(function (e) { e.classList.remove('is-hl'); });
    var any = false;
    if (ref === 'rule') {
      $$('[data-ref]', svg).forEach(function (e) { e.classList.add('is-hl'); });
      any = true;
    } else if (ref) {
      $$('[data-ref="' + ref + '"]', svg).forEach(function (e) { e.classList.add('is-hl'); any = true; });
    }
    svg.classList.toggle('has-hl', any && ref !== 'rule');
  }

  function initExamples() {
    var tabs = $('#ex-tabs');
    if (!tabs) return;
    var codeEl = $('#ex-code'), fig = $('#ex-fig'), cap = $('#ex-cap'), extra = $('#ex-extra'), top = $('#ex-top'), file = $('#ex-file'), openLink = $('#ex-open');
    var timers = [];
    function clearTimers() { timers.forEach(clearTimeout); timers = []; }
    function setOpen(target) {
      if (!openLink) return;
      if (!target) { openLink.hidden = true; return; }
      openLink.setAttribute('href', openLink.getAttribute('href').replace(/url=samples\/site\/[^&#]*/, 'url=samples/site/' + target));
      openLink.hidden = false;
    }

    function mountView(scene, box) {
      fig.innerHTML = '';
      var svg = S.el('svg', { role: 'img', 'aria-label': '<en>Example schematic</en>' }, fig);
      var linker = null;
      var view = new S.View(svg, { box: box || scene.box, onHover: function (ref) { if (linker) linker.markRef(ref); } });
      view.setScene(scene);
      return { view: view, setLinker: function (l) { linker = l; } };
    }

    var EX = {
      e1: function () {
        file.textContent = 'rlc_lowpass.sp';
        var m = mountView(SC.rlcScene('split'), SC.rlcScene('split').box);
        var linker = null;
        function setStyle(s) {
          var lines = SC.e1Lines(s);
          C.render(codeEl, lines);
          if (!linker) { linker = C.link(codeEl, lines, m.view, { modes: true }); m.setLinker(linker); }
          else linker.setLines(lines);
        }
        setStyle('split');
        top.innerHTML = '<span class="seg" role="group" aria-label="<en>Ground symbol style</en>">' +
          '<button type="button" data-s="split" aria-pressed="true"><en>Standard: one per terminal</en></button>' +
          '<button type="button" data-s="merged" aria-pressed="false"><en>Three lines: merged</en></button></span>';
        PG.initSeg($('.seg', top), 'data-s', function (s) {
          setStyle(s);
          m.view.setScene(SC.rlcScene(s), { animate: true, duration: 700 });
        });
        cap.innerHTML = '<en>One statement: <code>$CHAIN</code> says what it is, a chain V1 → {R1 L1} → C1 from source to load, whose middle role, chain, takes a set; it also carries a default drawing, source, chain and load from left to right, of priority low, so what the author writes comes first. The standard package adds the grounds, one per grounded terminal; switch to “merged” at the top right to see the three lines the file needs for a single ground.</en>';
      },
      e2: function () {
        file.textContent = 'ota5t_tb.sp';
        setOpen('ota5t_tb.sp');
        var lines = SC.E2_LINES;
        C.render(codeEl, lines);
        var m = mountView(SC.TB);
        m.setLinker(C.link(codeEl, lines, m.view));
        cap.innerHTML = '<en>Subcircuit ota5t defines its own appearance: a triangular block symbol whose pins name ports in <code>terminal</code>. With <code># @SET-SYMBOL</code> inside the subcircuit, XU1 one level up uses it automatically. The top level states only XU1’s position, which nets to draw as wires and a note; the supply, bias source and load capacitor have no positions, so the solver places them, and the standard package adds the grounds.</en>';
      },
      e3: function () {
        file.textContent = 'my_nmos.sp';
        var lines = SC.E3_LINES;
        C.render(codeEl, lines);
        fig.innerHTML = '';
        var svg = e3Drawing(fig);
        cap.innerHTML = SC.E3_NOTES.idle;
        C.link(codeEl, lines, null, {
          onLine: function (i, l) {
            e3Highlight(svg, l ? l.r : null);
            cap.innerHTML = l ? SC.E3_NOTES[l.r] : SC.E3_NOTES.idle;
          }
        });
        $$('[data-ref]', svg).forEach(function (e) {
          e.addEventListener('mouseenter', function () {
            var ref = e.getAttribute('data-ref');
            e3Highlight(svg, ref);
            $$('.cl', codeEl).forEach(function (r) { r.classList.toggle('is-hot', r.getAttribute('data-refs') === ref); });
            cap.innerHTML = SC.E3_NOTES[ref];
          });
          e.addEventListener('mouseleave', function () {
            e3Highlight(svg, null);
            $$('.cl', codeEl).forEach(function (r) { r.classList.remove('is-hot'); });
            cap.innerHTML = SC.E3_NOTES.idle;
          });
        });
        top.innerHTML = '<span class="chip"><en>Local coordinates · y up</en></span>';
      },
      e4: function () {
        file.textContent = 'fill.sp';
        var lines = SC.E4_LINES;
        C.render(codeEl, lines);
        var m = mountView(SC.fillScene(2));
        m.setLinker(C.link(codeEl, lines, m.view));
        top.innerHTML = '<button type="button" class="btn btn-sm" id="ex-replay">' + icon('replay') + '<en>Replay the fill</en></button>';
        function replay() {
          clearTimers();
          var rows = $$('.cl', codeEl);
          m.view.setScene(SC.fillScene(0), { animate: true, duration: 500 });
          [1, 2].forEach(function (n, k) {
            timers.push(setTimeout(function () {
              m.view.setScene(SC.fillScene(n), { animate: true, duration: 450 });
              rows.forEach(function (r) { r.classList.remove('is-hot'); });
              var gen = n === 1 ? 'gnd' : 'port';
              lines.forEach(function (l, i) { if (l.gen === gen && rows[i]) rows[i].classList.add('is-hot'); });
            }, 900 + k * 1300));
          });
          timers.push(setTimeout(function () { rows.forEach(function (r) { r.classList.remove('is-hot'); }); }, 900 + 2 * 1300 + 900));
        }
        $('#ex-replay').addEventListener('click', replay);
        cap.innerHTML = '<en>Not one line of the file is about grounds: for every unwired terminal on net 0, the standard package adds a ground on a short wire (gray). V1’s negative terminal already has <code>$GROUND @ATTACH #V1:2</code>, a ground drawn right on the pin, so the fill skips it. The unwired gate gets a port from a rule in the file: <code>$PORT_STUB</code> is a definition written here, with a wire and two constraints.</en>';
      },
      e5: function () {
        file.textContent = 'ota5t.sp';
        setOpen('ota5t-edits.sp');
        var lines = SC.E5_LINES;
        C.render(codeEl, lines);
        var m = mountView(SC.e5Scene(), SC.OTA_BOX);
        var linker = C.link(codeEl, lines, m.view);
        m.setLinker(linker);
        extra.innerHTML = '<ul class="diag">' + SC.E5_DIAG.map(function (d) {
          return '<li class="' + d.k + '" data-d="' + d.id + '" tabindex="0">' + icon(d.icon) + '<span>' + d.html + '</span></li>';
        }).join('') + '</ul>';
        $$('.diag li', extra).forEach(function (li) {
          var id = li.getAttribute('data-d');
          var d = SC.E5_DIAG.filter(function (x) { return x.id === id; })[0];
          function on() {
            $$('.cl', codeEl).forEach(function (r, i) { r.classList.toggle('is-hot', lines[i].d === id); });
            m.view.highlight(d.refs.length ? d.refs : null);
          }
          function off() {
            $$('.cl', codeEl).forEach(function (r) { r.classList.remove('is-hot'); });
            m.view.highlight(null);
          }
          li.addEventListener('mouseenter', on);
          li.addEventListener('focus', on);
          li.addEventListener('mouseleave', off);
          li.addEventListener('blur', off);
        });
        top.innerHTML = '<span class="chip chip-err"><en>2 errors</en></span><span class="chip chip-warn"><en>1 conflict</en></span>';
        cap.innerHTML = '<en>Three records: the designer’s semantics, the solver’s coordinates (high) and the editor’s later changes (high, written after). A bad statement disables only itself; a conflict is not an error, and priority, then order, decides which fact stays. Hover over a diagnostic above to see which line and which device it points to.</en>';
      }
    };

    function show(id) {
      clearTimers();
      extra.innerHTML = '';
      top.innerHTML = '';
      setOpen(null);
      EX[id]();
    }
    PG.initTabs(tabs, function (t) { show(t.getAttribute('data-ex')); });
  }

  function boot() {
    initLanguage();
    initExamples();
  }

  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', boot);
  else boot();
})();
