/* RNL language tour · interactions
 *
 * Every RNL statement shown here reads without errors with rnl check, in language 0.3 with the
 * standard series 0.3. The drawings are laid out by hand in RNL coordinates (x right, y up), as on
 * the introduction page, with the pins where the series 0.3 puts them.
 */
(function () {
  'use strict';

  var S = window.RNLSchem, SC = window.RNLScenes, C = window.RNLCode, PG = window.RNLPage;
  var $ = PG.$, $$ = PG.$$, icon = PG.icon, esc = C.esc;
  var REDUCE = S.REDUCE;

  /* Code rows for fragments inside a record (no host prefix), in the same markup as C.render */
  function snippetRows(el, lines) {
    el.innerHTML = lines.map(function (l, i) {
      if (typeof l === 'string') l = { t: l };
      return '<div class="cl' + (l.cls ? ' ' + l.cls : '') + (l.mark ? ' ' + l.mark : '') + (l.r ? ' is-link' : '') + '" data-i="' + i + '"' +
        (l.r ? ' data-refs="' + esc(l.r) + '" tabindex="0"' : '') + '><span class="ln" aria-hidden="true">' + (i + 1) +
        '</span><span class="tx">' + (C.hlSnippet(l.t) || ' ') + '</span></div>';
    }).join('');
  }
  function lines(arr) { return arr.map(function (l) { return typeof l === 'string' ? { t: l } : l; }); }
  /* Mark lines that are new compared with the previous version of the same file */
  function marks(prev, next) {
    if (!prev) return next;
    var old = {};
    prev.forEach(function (l) { old[l.t] = 1; });
    next.forEach(function (l) { if (!old[l.t] && !l.mark) l.mark = 'is-add'; });
    return next;
  }
  function fmt(n) { n = Math.round(n * 100) / 100; return n < 0 ? '−' + (-n) : String(n); }
  function pt(p) { return '(' + fmt(p[0]) + ', ' + fmt(p[1]) + ')'; }

  /* ================= T1 Sentences ================= */

  var LAB = [
    { k: 'PLACE', bare: '#M1 @PLACE(10, 20)', en: '#M1 is at @PLACE(10, 20)',
      fact: '<en>M1’s origin is at (10, 20). These are two facts, one for x and one for y; in a conflict each can be dropped on its own.</en>' },
    { k: 'ALIGN', bare: '{#M1 #M2} @ALIGN("y")', en: '{#M1 #M2} @ALIGN("y"), i.e., same y',
      fact: '<en>M1 and M2 have the same y, so they sit in a row. “, i.e., same y” is only a gloss.</en>' },
    { k: 'ORDER', bare: '{#M1 #M2 #M3} @ORDER("left-to-right")', en: '{#M1 #M2 #M3} are in @ORDER("left-to-right")',
      fact: '<en>M1, M2 and M3 run strictly from left to right. With "x" only the axis is fixed, and the reverse holds too.</en>' },
    { k: 'DISTANCE', bare: '#M1 #M2 @DISTANCE("bottom-to-top", min=4)', en: 'from #M1 to #M2, they have @DISTANCE("bottom-to-top", min=4)',
      fact: '<en>From M1 up to M2 is at least 4 units: M2’s y exceeds M1’s by 4 or more.</en>' },
    { k: 'AS', bare: '%dp = #M1(in_p) #M2(in_n) @AS $DIFFERENTIAL_PAIR', en: '%dp = #M1(in_p) and #M2(in_n) treated @AS $DIFFERENTIAL_PAIR',
      fact: '<en>M1 (in_p) and M2 (in_n) together form a differential pair. This application is a usage, named %dp.</en>' },
    { k: 'ATTACH', bare: '$GROUND @ATTACH &c => @PLACE(24, 38) => %g0', en: '$GROUND is @ATTACH to &c => it is at @PLACE(24, 38) => named %g0',
      fact: '<en>A ground is added on net c, at (24, 38), and named %g0. =&gt; hands each part’s result to the next.</en>' },
    { k: 'SELECT-INSTANCE', bare: '# @SELECT-INSTANCE(model="nch") => @SET-SYMBOL $NMOS', en: 'among the instances at each level in # and below, @SELECT-INSTANCE(model="nch") => they @SET-SYMBOL to $NMOS',
      fact: '<en>On this level and on every level below, instances of model nch are drawn with $NMOS. This is a rule, evaluated once per level.</en>' },
    { k: 'WIRE', bare: '&n1 @WIRE {#R1:2 %j #R2:1}', en: 'net &n1 has a @WIRE connecting {#R1:2 %j #R2:1}',
      fact: '<en>Net n1 has a wire running through R1’s terminal 2, the point %j and R2’s terminal 1, in that order.</en>' }
  ];
  /* The same statements as Chinese standard sentences, with the alias package rnl-zh; shown on every page */
  /*zh-CN*/
  var LAB_ZH = [
    '#M1 @位于(10, 20)',
    '{#M1 #M2} @对齐("y")，即 y 坐标相同',
    '{#M1 #M2 #M3} 按 @顺序("从左到右") 排列',
    '#M1 到 #M2 有 @距离("从下到上", 最小值=4)',
    '%dp = #M1(正输入) 和 #M2(负输入) @视作 $"差分对"',
    '$"地" @附着 在 &c 上 => 其 @位于(24, 38) => 记作 %g0',
    '从本层 # 及以下每一层的元件中 @选出元件(模型="nch") => 其 @使用符号 $NMOS',
    '网络 &n1 的 @导线 连接了 {#R1:2 %j #R2:1}'
  ];
  /*/zh-CN*/

  function initLab() {
    var list = $('#lab-list');
    if (!list) return;
    var line = $('#lab-line'), bare = $('#lab-bare'), fact = $('#lab-fact'), machine = $('#lab-machine');
    var style = 'en', cur = 0;
    list.innerHTML = LAB.map(function (l, i) {
      return '<button type="button" role="tab" data-i="' + i + '" aria-selected="false"><span class="kwn">@' + l.k + '</span></button>';
    }).join('');
    var tabs = $$('button', list);
    function show() {
      var l = LAB[cur];
      var t = style === 'bare' ? l.bare : style === 'zh' ? LAB_ZH[cur] : l.en;
      line.innerHTML = C.hlSnippet(t);
      if (style === 'zh') line.setAttribute('lang', 'zh-CN'); else line.removeAttribute('lang');
      bare.innerHTML = C.hlSnippet(l.bare);
      fact.innerHTML = l.fact;
      tabs.forEach(function (b, i) { b.setAttribute('aria-selected', i === cur ? 'true' : 'false'); b.tabIndex = i === cur ? 0 : -1; });
    }
    tabs.forEach(function (b, i) {
      b.addEventListener('click', function () { cur = i; show(); });
      b.addEventListener('keydown', function (e) {
        var j = -1;
        if (e.key === 'ArrowDown' || e.key === 'ArrowRight') j = (i + 1) % tabs.length;
        if (e.key === 'ArrowUp' || e.key === 'ArrowLeft') j = (i - 1 + tabs.length) % tabs.length;
        if (j >= 0) { e.preventDefault(); cur = j; show(); tabs[j].focus(); }
      });
    });
    PG.initSeg($('#lab-style'), 'data-style', function (v) { style = v; show(); });
    machine.addEventListener('change', function () { line.classList.toggle('machine', machine.checked); });
    show();
  }

  /* The whole OTA file in one of three writing styles */
  function initFile() {
    var el = $('#file-code');
    if (!el) return;
    var openLink = $('#file-open');
    var btns = PG.initSeg($('#file-style'), 'data-style', show);
    function show(style) {
      C.render(el, SC.otaCode(2, style).map(function (l) { return { t: l.t }; }));
      if (openLink) {
        var target = style === 'zh' ? 'ota5t-zh.sp' : 'ota5t.sp';
        openLink.setAttribute('href', openLink.getAttribute('href').replace(/url=samples\/site\/[^&#]*/, 'url=samples/site/' + target));
      }
    }
    var first = '<en>en</en>';
    PG.setPressed(btns, btns.filter(function (b) { return b.getAttribute('data-style') === first; })[0]);
    show(first);
  }

  /* ================= T2 Definitions ================= */

  function mpScene(on) {
    return {
      rk: 'mp',
      devices: {
        M1: { sym: 'nmos', x: on ? 6 : 5, y: on ? 4 : 2, label: 'M1' },
        M2: { sym: 'nmos', x: on ? 20 : 18, y: on ? 4 : 6, o: on ? 'MY' : 'R0', label: 'M2' }
      },
      route: function () { return { wires: [], dots: [] }; },
      overlays: function (P, S) {
        if (!on) {
          return [{ id: 'dp', kind: 'box', layer: 'back', ids: ['M1', 'M2'], pad: 0.8, label: '%dp · $DIFFERENTIAL_PAIR', labelAt: [0.5, 1], anchor: 'middle' }];
        }
        var a = S('M1').x, b = S('M2').x, m = (a + b) / 2;
        return [
          { id: 'ax', ref: 'di', kind: 'vline', layer: 'back', x: m, y0: -0.6, y1: 8.2, label: '%ax' },
          { id: 'al', ref: 'al', kind: 'dash', pts: [[a, 4], [b, 4]], tone: 'user' },
          { id: 'di1', ref: 'di', kind: 'dash', pts: [[a, 0.4], [b, 0.4]] },
          { id: 'di', ref: 'di', kind: 'ticks', y: 0.4, segs: [[a, m], [m, b]], labels: [fmt(m - a), fmt(b - m)] },
          { id: 'or', ref: 'or', kind: 'tag', at: [m, 6.6], anchor: 'middle', text: 'MY' }
        ];
      }
    };
  }

  function initMirror() {
    var svg = $('#mp-svg');
    if (!svg) return;
    var codeEl = $('#mp-code'), cap = $('#mp-cap');
    var view = new S.View(svg, { box: [-3, -2.6, 29, 10.4] });
    var on = false, prev = null;
    function file() {
      var L = [
        { t: '* Differential pair' },
        { t: '.model nch nmos level=1' },
        { t: 'M1 a b c c nch', r: 'M1' },
        { t: 'M2 d e c c nch', r: 'M2' },
        { t: '* @RNL {' },
        { t: '*   %dp = #M1(in_p) and #M2(in_n) treated @AS $DIFFERENTIAL_PAIR' + (on ? ';' : ''), r: 'M1 M2 ov:dp' }
      ];
      if (on) {
        L.push({ t: '*   %ax = (_, _);', r: 'ov:di' });
        L.push({ t: '*   %sym = #M1(a), #M2(b) and %ax(axis) treated @AS $MIRRORED', r: 'M1 M2 ov:al ov:di ov:or' });
      }
      L.push({ t: '* }' });
      return L;
    }
    var exp = document.createElement('div');
    exp.className = 'exp';
    codeEl.parentNode.appendChild(exp);
    var linker = null;
    function show(animate) {
      var L = marks(prev, file());
      prev = L;
      C.render(codeEl, L);
      if (!linker) linker = C.link(codeEl, L, view); else linker.setLines(L);
      view.setScene(mpScene(on), { animate: animate, duration: 900 });
      if (on) {
        var X = [
          ['{#M1 #M2} @ALIGN("y", priority="low");', 'M1 M2 ov:al', '<en>same row</en>'],
          ['{#M1 %ax #M2} @DISTRIBUTE("x", priority="low");', 'M1 M2 ov:di', '<en>equal steps around the axis</en>'],
          ['{#M1 #M2} @ORDER("left-to-right", priority="low");', 'M1 M2', '<en>M1 on the left</en>'],
          ['#M1 #M2 @ORIENT-RELATION("MY", priority="low")', 'M1 M2 ov:or', '<en>mirror images</en>']
        ];
        exp.innerHTML = '<p class="exp-k"><en>Expanded, $MIRRORED adds four statements to this level (not written in the file), all of priority low:</en></p>' +
          X.map(function (x) { return '<div class="exp-l" data-refs="' + x[1] + '" tabindex="0"><code>' + C.hlSnippet(x[0]) + '</code><span>' + x[2] + '</span></div>'; }).join('');
        $$('.exp-l', exp).forEach(function (row) {
          function hl() { view.highlight(row.getAttribute('data-refs').split(' ')); }
          row.addEventListener('mouseenter', hl);
          row.addEventListener('focus', hl);
          row.addEventListener('mouseleave', function () { view.highlight(null); });
          row.addEventListener('blur', function () { view.highlight(null); });
        });
        cap.innerHTML = '<en>With the roles a, b and axis replaced by M1, M2 and %ax, the four constraints in the body expand into this level: same row, equal steps around the axis, M1 on the left, mirror images. They are all of priority low, so what the author writes comes first. The axis %ax is a free point that the solver places.</en>';
      } else {
        exp.innerHTML = '';
        cap.innerHTML = '<en>Only $DIFFERENTIAL_PAIR is applied: it records that M1 and M2 are a differential pair, and a viewer may outline them, but nothing moves.</en>';
      }
    }
    PG.initSeg($('#mp-seg'), 'data-v', function (v) { on = v === '1'; show(true); });
    show(false);
  }

  function initPair() {
    var svg = $('#pair-svg');
    if (!svg) return;
    var codeEl = $('#pair-code'), range = $('#gap'), out = $('#gap-v');
    var view = new S.View(svg, { box: [-4, -3, 26, 10] });
    function scene(g) {
      return {
        rk: 'pair',
        devices: {
          R1: { sym: 'res', x: 6, y: 4, label: 'R1', value: '1k' },
          R2: { sym: 'res', x: 6 + g, y: 4, label: 'R2', value: '1k' }
        },
        route: function () { return { wires: [], dots: [] }; },
        overlays: function (P, S) {
          var a = S('R1').x, b = S('R2').x;
          return [
            { id: 'dim', kind: 'dash', pts: [[a, 0], [b, 0]] },
            { id: 'dimt', kind: 'tag', at: [(a + b) / 2, -1.1], anchor: 'middle', text: '≥ ' + g }
          ];
        }
      };
    }
    var prev = null;
    function show(g, animate) {
      out.textContent = g;
      var L = lines([
        '* Two resistors',
        'R1 a b 1k',
        'R2 b c 1k',
        '* @RNL {',
        '*   $PAIR = @DEFINE(roles=["a", "b"], parameters=[["gap", 2]]) {',
        '*     {?a ?b} @ALIGN("y");',
        '*     ?a ?b @DISTANCE("left-to-right", min=?gap)',
        '*   };',
        g === 2 ? '*   #R1(a) #R2(b) @AS $PAIR   /* gap = 2 */' : '*   #R1(a) #R2(b) @AS(gap=' + g + ') $PAIR',
        '* }'
      ]);
      if (prev) L[8].mark = 'is-chg';
      prev = L;
      C.render(codeEl, L);
      view.setScene(scene(g), { animate: animate, duration: 350 });
    }
    range.addEventListener('input', function () { show(+range.value, true); });
    show(+range.value, false);
  }

  /* The definitions of the standard series 0.3, as its package files write them (libraries/standard/0.3) */
  var STD = [
    ['sem', '$CURRENT_MIRROR', '@DEFINE(roles=[["{ref}", "instance"], ["{out}", "instance"]]) {}'],
    ['sem', '$MATCHED', '@DEFINE(roles=["{members}"]) {}'],
    ['sem', '$CASCADE', '@DEFINE(roles=["{first}", "{second}", "{feedback}", "{load}"]) {}'],
    ['sem', '$SUPPLY_HIGH', '@DEFINE(roles=[["pin", "terminal"]]) {}'],
    ['sem', '$SUPPLY_LOW', '@DEFINE(roles=[["pin", "terminal"]]) {}'],
    ['lay', '$DIFFERENTIAL_PAIR', '@DEFINE(roles=[["in_p", "instance"], ["in_n", "instance"], ["{tail}", "instance"]]) {\n  {?in_p {?tail} ?in_n} @DISTRIBUTE("x", priority="low")\n}'],
    ['lay', '$BRANCH', '@DEFINE(roles=["{members}"]) {\n  ?members @ALIGN("x", priority="low")\n}'],
    ['lay', '$CHAIN', '@DEFINE(roles=["source", "{chain}", "load"]) {\n  {?source ?chain ?load} @ORDER("left-to-right", priority="low")\n}'],
    ['lay', '$MIRRORED', '@DEFINE(roles=["a", "b", "axis"]) {\n  {?a ?b} @ALIGN("y", priority="low");\n  {?a ?axis ?b} @DISTRIBUTE("x", priority="low");\n  {?a ?b} @ORDER("left-to-right", priority="low");\n  ?a ?b @ORIENT-RELATION("MY", priority="low")\n}'],
    ['lay', '$FLOW', '@DEFINE(roles=["{path}"], parameters=[["direction", "left-to-right"]]) {\n  ?path @ORDER(?direction, priority="low")\n}'],
    ['vir', '$GROUND_STUB', '@DEFINE(roles=[["t", "terminal"]], parameters=[["gap", 2]]) {\n  %g = $GROUND @ATTACH ?t::net;\n  ?t::net @WIRE {?t %g:p};\n  {?t %g:p} @ALIGN("x", priority="low");\n  %g:p ?t @DISTANCE("bottom-to-top", min=?gap, priority="low");\n  %g @ORIENT("R0", priority="low")\n}'],
    ['vir', '$SUPPLY_STUB', '@DEFINE(roles=[["t", "terminal"]], parameters=[["gap", 2]]) {\n  %s = $SUPPLY @ATTACH ?t::net;\n  ?t::net @WIRE {?t %s:p};\n  {?t %s:p} @ALIGN("x", priority="low");\n  ?t %s:p @DISTANCE("bottom-to-top", min=?gap, priority="low");\n  %s @ORIENT("R0", priority="low")\n}'],
    ['vir', '$GROUND', '@DEFINE(roles=[["p", ["net", "terminal"]]]) {}'],
    ['vir', '$SUPPLY', '@DEFINE(roles=[["p", ["net", "terminal"]]]) {}'],
    ['vir', '$IO_PORT', '@DEFINE(roles=[["p", ["net", "terminal"]]]) {}'],
    ['vir', '$NO_CONNECT', '@DEFINE(roles=[["t", "terminal"]]) {}'],
    ['vir', '$JUNCTION', '@DEFINE(roles=[["p", ["point", "terminal"]]]) {}'],
    ['vir', '$CURRENT_ARROW', '@DEFINE(roles=[["p", ["terminal", "point"]]]) {}']
  ];
  var STD_GROUPS = [
    ['sem', '<en>Structures with roles only, applied with AS</en>'],
    ['lay', '<en>Structures with a default drawing in the body: all of priority low, so what the author writes comes first</en>'],
    ['vir', '<en>Display elements and stubs: added with ATTACH, given symbols by rules; the stubs’ constraints are low too</en>']
  ];

  function initStd() {
    var box = $('#std');
    if (!box) return;
    var show = $('#std-show');
    box.innerHTML = STD_GROUPS.map(function (g) {
      return '<div class="std-g"><span class="std-k">' + g[1] + '</span><div>' + STD.filter(function (d) { return d[0] === g[0]; }).map(function (d) {
        return '<button type="button" class="kw std-' + d[0] + '" aria-pressed="false">' + d[1] + '</button>';
      }).join('') + '</div></div>';
    }).join('');
    var btns = $$('button', box);
    function pick(b) {
      PG.setPressed(btns, b);
      var d = STD.filter(function (x) { return x[1] === b.textContent; })[0];
      show.innerHTML = C.hlSnippet(d[1] + ' = ' + d[2]);
    }
    btns.forEach(function (b) { b.addEventListener('click', function () { pick(b); }); });
    pick(btns.filter(function (b) { return b.textContent === '$MIRRORED'; })[0]);
  }

  /* ================= T3 Defaults ================= */

  function ceScene(p) {
    var gen = 'is-gen';
    var dev = {
      Q1: { sym: 'npn', x: 10, y: 0, label: 'Q1', value: 'npn1' },
      RC: { sym: 'res', x: 10, y: 9, label: 'RC', value: '5k' },
      CL: { sym: 'cap', x: 16, y: 1, label: 'CL', value: '1p' },
      P_in: { sym: 'port', x: 3, y: 0, label: 'in', virtual: true, cls: gen },
      P_out: { sym: 'port', x: 21, y: 4, o: 'MY', label: 'out', virtual: true, cls: gen }
    };
    if (p === 0) {
      dev.P_vdd = { sym: 'port', x: 3, y: 13, label: 'vdd', virtual: true, cls: gen };
      dev.P_vss = { sym: 'port', x: 3, y: -4, label: 'vss', virtual: true, cls: gen };
    }
    if (p >= 1) {
      /* vss is a ground net not named 0: until the file takes it out of $NAMED_GROUND_NET_SET, its
         grounds show its name */
      dev.SUP = { sym: 'supply', x: 10, y: 13, label: 'vdd', virtual: true, cls: gen };
      dev.G1 = { sym: 'gnd', x: 10, y: -4, label: p === 1 ? 'vss' : undefined, virtual: true, cls: gen };
      dev.G2 = { sym: 'gnd', x: 16, y: -3, label: p === 1 ? 'vss' : undefined, virtual: true, cls: gen };
    }
    return {
      rk: 'ce' + p,
      devices: dev,
      route: function (P) {
        var jo = [10, 4], jc = [16, 4];
        var w = [
          { net: 'in', pts: [P('P_in.p'), P('Q1.b')] },
          { net: 'out', pts: [P('RC.n'), jo, P('Q1.c')] },
          { net: 'out', pts: [jo, jc, P('P_out.p')] },
          { net: 'out', pts: [jc, P('CL.p')] }
        ];
        var dots = [[10, 4, 'out'], [16, 4, 'out']];
        if (p === 0) {
          w.push({ net: 'vdd', pts: [P('P_vdd.p'), [10, 13], P('RC.p')] });
          w.push({ net: 'vss', pts: [P('P_vss.p'), [10, -4], P('Q1.e')] });
          w.push({ net: 'vss', pts: [[10, -4], [16, -4], P('CL.n')] });
          dots.push([10, -4, 'vss']);
        } else {
          w.push({ net: 'vdd', pts: [P('RC.p'), P('SUP.p')], cls: gen });
          w.push({ net: 'vss', pts: [P('Q1.e'), P('G1.p')], cls: gen });
          w.push({ net: 'vss', pts: [P('CL.n'), P('G2.p')], cls: gen });
        }
        return { wires: w, dots: dots };
      }
    };
  }

  var CE_GEN = [
    [
      ['check', '<en>Symbols by first letter: Q1 → $NPN, RC → $RESISTOR, CL → $CAPACITOR.</en>'],
      ['plus', '<en>Port tags in, out, vdd and vss: $PORT_FILL_SET starts with every port of the subcircuit.</en>'],
      ['info', '<en>vdd and vss are just two ports: they are not in the supply or ground sets, so no supply or ground is added.</en>']
    ],
    [
      ['plus', '<en>Supply stub: RC’s vdd terminal has no wire, so $SUPPLY_STUB adds a supply above it, at least 2 units away.</en>'],
      ['plus', '<en>Ground stubs: Q1:e and CL:n have no wires, so $GROUND_STUB adds a ground to each. vss is not named 0, so the grounds show its name.</en>'],
      ['minus', '<en>The port tags of vdd and vss are gone: supply and ground nets have their supplies and grounds, and get no port tags.</en>']
    ],
    [
      ['minus', '<en>The net name under the grounds is gone: vss is no longer in $NAMED_GROUND_NET_SET.</en>'],
      ['check', '<en>The supply and grounds stay: the textbook look, from three lines in the file.</en>']
    ]
  ];

  function initCe() {
    var svg = $('#ce-svg');
    if (!svg) return;
    var codeEl = $('#ce-code'), gen = $('#ce-gen');
    var view = new S.View(svg, { box: [-6.6, -7, 30, 16.6] });
    var prev = null;
    function show(p, animate) {
      var L = lines(['* Common-emitter stage', '.model npn1 npn bf=100', '.subckt ce in out vdd vss', 'Q1 out in vss npn1', 'RC vdd out 5k', 'CL out vss 1p']);
      if (p >= 1) {
        L.push({ t: '* @RNL {' });
        L.push({ t: '*   &vdd @ADD-TO $SUPPLY_NET_SET;' });
        L.push({ t: '*   &vss @ADD-TO $GROUND_NET_SET' + (p >= 2 ? ';' : '') });
        if (p >= 2) L.push({ t: '*   &vss @REMOVE-FROM $NAMED_GROUND_NET_SET' });
        L.push({ t: '* }' });
      }
      L.push({ t: '.ends ce' });
      L = marks(prev, L);
      prev = L;
      C.render(codeEl, L);
      view.setScene(ceScene(p), { animate: animate, duration: 700 });
      gen.innerHTML = CE_GEN[p].map(function (g) {
        return '<li class="g-' + g[0] + '">' + icon(g[0]) + '<span>' + g[1] + '</span></li>';
      }).join('');
    }
    PG.initSeg($('#ce-seg'), 'data-p', function (v) { show(+v, true); });
    show(0, false);

    snippetRows($('#fill-code'), [
      { t: '/* <en>sets for files to edit</en> */', cls: 'is-cmt' },
      '$GROUND_NET_SET = # @SELECT-NET(name="0");',
      '$SUPPLY_NET_SET = {};',
      '$PORT_FILL_SET = # @SELECT-NET(port=true);',
      { t: '/* <en>a stub for each unwired terminal on ground and supply nets</en> */', cls: 'is-cmt' },
      '$GROUND_FILL_SET = $GROUND_NET_SET @SELECT-TERMINAL(wired=false);',
      '$GROUND_STUB @ATTACH $GROUND_FILL_SET;',
      '$SUPPLY_FILL_SET = $SUPPLY_NET_SET @SELECT-TERMINAL(wired=false);',
      '$SUPPLY_STUB @ATTACH $SUPPLY_FILL_SET;',
      { t: '/* <en>one port tag for each port of a subcircuit, but not on supply and ground nets</en> */', cls: 'is-cmt' },
      '$SUPPLY_NET_SET @REMOVE-FROM $PORT_FILL_SET;',
      '$GROUND_NET_SET @REMOVE-FROM $PORT_FILL_SET;',
      '$IO_PORT @ATTACH $PORT_FILL_SET;',
      { t: '/* <en>on a ground net other than 0, the grounds show the net’s name</en> */', cls: 'is-cmt' },
      '$NAMED_GROUND_NET_SET = {$GROUND_NET_SET};',
      '# @SELECT-NET(name="0") => @REMOVE-FROM $NAMED_GROUND_NET_SET;',
      '$NAMED_GROUND_NET_SET @SELECT-ANCESTOR-USAGE $GROUND => @SET-SYMBOL $NAMED_GROUND_SYMBOL'
    ]);
  }

  /* ================= T4 Sets and selections ================= */

  function initSpread() {
    var flat = $('#sp-flat'), nested = $('#sp-nested');
    if (!flat) return;
    var NAMES = ['M1', 'M2', 'M3', 'M4'];
    function build(svg) {
      var el = S.el;
      el('line', { class: 'sp-axis', x1: 14, y1: 82, x2: 306, y2: 82 }, svg);
      el('path', { class: 'sp-head', d: 'M306 82l-7 -4v8z' }, svg);
      el('text', { class: 'sp-ax', x: 309, y: 86 }, svg).textContent = 'x';
      var g = el('g', { class: 'sp-groups' }, svg);
      var boxes = NAMES.map(function (n, i) {
        var b = el('g', { class: 'sp-box ' + (i < 2 ? 's1' : 's2') }, svg);
        el('rect', { x: -22, y: -15, width: 44, height: 30, rx: 5 }, b);
        el('text', { x: 0, y: 1 }, b).textContent = n;
        return b;
      });
      var ok = el('text', { class: 'sp-ok', x: 160, y: 14, 'text-anchor': 'middle' }, svg);
      return { boxes: boxes, groups: g, ok: ok };
    }
    var A = build(flat), B = build(nested);
    function place(o, order, groupsOn) {
      order.forEach(function (idx, slot) {
        var x = 48 + slot * 75;
        o.boxes[idx].style.transform = 'translate(' + x + 'px, 46px)';
        o.boxes[idx].setAttribute('data-x', x);
      });
      while (o.groups.firstChild) o.groups.removeChild(o.groups.firstChild);
      if (groupsOn) {
        [[0, 1, 's1'], [2, 3, 's2']].forEach(function (g) {
          var xa = +o.boxes[g[0]].getAttribute('data-x'), xb = +o.boxes[g[1]].getAttribute('data-x');
          var x0 = Math.min(xa, xb) - 30, x1 = Math.max(xa, xb) + 30;
          S.el('rect', { class: 'sp-grp ' + g[2], x: x0, y: 25, width: x1 - x0, height: 42, rx: 8 }, o.groups);
          S.el('circle', { class: 'sp-ctr ' + g[2], cx: (xa + xb) / 2, cy: 82, r: 3.5 }, o.groups);
        });
      }
    }
    function rnd() { return Math.random() < 0.5; }
    function shuffle() {
      var f = [0, 1, 2, 3];
      if (rnd()) f.reverse();
      place(A, f, false);
      A.ok.textContent = '✓ ' + f.map(function (i) { return NAMES[i]; }).join(' < ');
      var g1 = rnd() ? [0, 1] : [1, 0], g2 = rnd() ? [2, 3] : [3, 2];
      var n = rnd() ? g1.concat(g2) : g2.concat(g1);
      place(B, n, true);
      B.ok.textContent = '✓ ' + (n[0] < 2 ? '{s1} < {s2}' : '{s2} < {s1}');
    }
    $('#sp-shuffle').addEventListener('click', shuffle);
    shuffle();
  }

  function selScene() {
    return {
      rk: 'sel',
      devices: {
        M3: { sym: 'nmos', x: 2, y: 3, label: 'M3' },
        M1: { sym: 'pmos', x: 11, y: 10, label: 'M1' },
        M2: { sym: 'nmos', x: 11, y: 4, label: 'M2' },
        M4: { sym: 'nmos', x: 20, y: 4, label: 'M4' },
        VDD: { sym: 'supply', x: 11, y: 13, label: 'vdd', virtual: true }
      },
      route: function (P) {
        var je = [6, 7], jo = [11, 7];
        return {
          wires: [
            { net: 'en', pts: [P('M3.d'), [2, 7], je] },
            { net: 'en', pts: [je, [6, 10], P('M1.g')] },
            { net: 'en', pts: [je, [6, 4], P('M2.g')] },
            { net: 'out', pts: [P('M1.d'), jo, P('M2.d')] },
            { net: 'out', pts: [jo, [14.5, 7]] },
            { net: 'vdd', pts: [P('M1.s'), P('VDD.p')] },
            { net: 'ctl', pts: [P('M3.g'), [-2.5, 3]] }
          ],
          dots: [[6, 7, 'en'], [11, 7, 'out']],
          labels: [
            { text: 'en', at: [4, 7.75], net: 'en' }, { text: 'out', at: [14.9, 7], anchor: 'start', net: 'out' },
            { text: 'ctl', at: [-2.9, 3], anchor: 'end', net: 'ctl' },
            { text: 'p', at: [20.6, 6.6], anchor: 'start', net: 'p' }, { text: 'q', at: [16.6, 4.75], net: 'q' },
            { text: '0', at: [2.6, 0.6], anchor: 'start', net: '0' }, { text: '0', at: [11.6, 1.6], anchor: 'start', net: '0' },
            { text: '0', at: [20.6, 1.6], anchor: 'start', net: '0' }
          ]
        };
      },
      overlays: function (P, S, B, mode) {
        if (mode === 'g') return ['M1.g', 'M2.g'].map(function (r, i) { return { id: 'pg' + i, ref: 'pin', kind: 'ring', at: P(r), r: 0.55, tone: 'user' }; });
        if (mode === 'z') return ['M3.s', 'M2.s', 'M4.s'].map(function (r, i) { return { id: 'pz' + i, ref: 'pin', kind: 'ring', at: P(r), r: 0.55, tone: 'user' }; });
        return [];
      }
    };
  }

  var SEL = [
    { t: '# @SELECT-INSTANCE(type="pmos")', hl: ['M1'], n: '<en>1 instance</en>',
      cap: '<en>The scope is the whole level (#) and the condition is type pmos: only M1.</en>' },
    { t: '&en @SELECT-INSTANCE', hl: ['M1', 'M2', 'M3', 'net:en'], n: '<en>3 instances</en>',
      cap: '<en>The scope is net en: the instances with a pin on it. M3’s drain drives en, and the gates of M1 and M2 sit on it.</en>' },
    { t: '&en @SELECT-TERMINAL(terminal="g") => @SELECT-INSTANCE', hl: ['M1', 'M2', 'ov:pin'], mode: 'g', n: '<en>2 instances</en>',
      cap: '<en>First the terminals on en named g, then the instances they belong to: M1 and M2. M3 touches en with its drain, so it is left out.</en>' },
    { t: '# @SELECT-TERMINAL(net="0", wired=false)', hl: ['ov:pin', 'M2', 'M3', 'M4'], mode: 'z', n: '<en>6 terminals</en>',
      cap: '<en>Terminals on net 0 with no wire yet, across the level: the sources of M2, M3 and M4, and their bulk terminals (three-terminal symbols do not draw the bulk). This is exactly how the standard package finds where to add grounds.</en>' }
  ];

  function initSel() {
    var svg = $('#sel-svg');
    if (!svg) return;
    var view = new S.View(svg, { box: [-7, -1.6, 25, 16] });
    view.setScene(selScene());
    var list = $('#sel-list'), cap = $('#sel-cap'), count = $('#sel-count');
    list.innerHTML = SEL.map(function (s, i) {
      return '<li><button type="button" data-i="' + i + '" aria-pressed="false"><code>' + C.hlSnippet(s.t) + '</code></button></li>';
    }).join('');
    var btns = $$('button', list);
    function pick(i) {
      var s = SEL[i];
      PG.setPressed(btns, btns[i]);
      view.setMode(s.mode || null);
      view.highlight(s.hl);
      cap.innerHTML = s.cap;
      count.innerHTML = '<span class="chip chip-user">' + s.n + '</span>';
    }
    btns.forEach(function (b, i) { b.addEventListener('click', function () { pick(i); }); });
    pick(0);
  }

  function initLevels() {
    var root = $('#levels');
    if (!root) return;
    var codeEl = $('#lv-code'), cap = $('#lv-cap');
    var rec = true, step = 0, visible = false, timer = null;
    var lvs = $$('.lv', root);
    var TITLE = [
      '<en>Pass 1, the top level</en>',
      '<en>Pass 2, subcircuit amp</en>',
      '<en>Pass 3, subcircuit bias</en>'
    ];
    var FOUND = [
      '<en>: M9 is picked.</en>',
      '<en>: M1 and M2 are picked. amp is one level; XA and XB are both instances of it, and the rule is evaluated once in amp.</en>',
      '<en>: MB is picked.</en>'
    ];
    var EMPTY = '<en>: the result is empty. recursive=false picks objects only on the level where it is written.</en>';
    function render() {
      codeEl.innerHTML = C.hlSnippet('# @SELECT-INSTANCE(model="nch"' + (rec ? '' : ', recursive=false') + ') => @SET-SYMBOL $MY_NMOS');
    }
    function paint() {
      lvs.forEach(function (lv, i) {
        lv.classList.toggle('is-now', i === step);
        lv.classList.toggle('is-done', i < step);
        $$('.inst', lv).forEach(function (n) {
          var hit = n.getAttribute('data-m') === 'nch' && i <= step && (rec || i === 0);
          n.classList.toggle('is-sel', hit);
        });
      });
      cap.innerHTML = TITLE[step] + (rec || step === 0 ? FOUND[step] : EMPTY);
    }
    function tick() {
      step = (step + 1) % 3;
      paint();
    }
    function run() {
      clearInterval(timer);
      if (!REDUCE && visible) timer = setInterval(function () { if (!document.hidden) tick(); }, 2200);
    }
    PG.initSeg($('#lv-seg'), 'data-r', function (v) { rec = v === '1'; step = 0; render(); paint(); run(); });
    lvs.forEach(function (lv, i) { lv.addEventListener('click', function () { step = i; paint(); run(); }); });
    PG.onVisible(root, function (v) { visible = v; run(); }, 0.3);
    render();
    paint();
  }

  /* ================= T5 Wires ================= */

  function teeScene(shared) {
    var gen = 'is-gen';
    return {
      rk: 'tee-' + (shared ? 's' : 'c'),
      devices: {
        RA: { sym: 'res', x: -1, y: 0, o: 'R270', label: 'RA', value: '1k', lab: [0, 1.2, 'middle'], val: [0, -1.25, 'middle'] },
        RB: { sym: 'res', x: 13, y: 0, o: 'R90', label: 'RB', value: '1k', lab: [0, 1.2, 'middle'], val: [0, -1.25, 'middle'] },
        RC: { sym: 'res', x: 6, y: -5, label: 'RC', value: '1k' },
        GA: { sym: 'gnd', x: -3, y: -2, virtual: true, cls: gen },
        GB: { sym: 'gnd', x: 15, y: -2, virtual: true, cls: gen },
        GC: { sym: 'gnd', x: 6, y: -9, virtual: true, cls: gen }
      },
      route: function (P) {
        return {
          wires: [
            { net: 'n1', pts: [P('RA.p'), [6, 0], P('RB.p')] },
            { net: 'n1', pts: [[6, 0], P('RC.p')], cls: shared ? '' : 'is-apart' },
            { net: '0', pts: [P('RA.n'), P('GA.p')], cls: gen },
            { net: '0', pts: [P('RB.n'), P('GB.p')], cls: gen },
            { net: '0', pts: [P('RC.n'), P('GC.p')], cls: gen }
          ],
          dots: shared ? [[6, 0, 'n1']] : []
        };
      },
      overlays: function () {
        if (shared) return [{ id: 'j', kind: 'tag', at: [6.6, 1.2], anchor: 'start', text: '%j' }];
        return [
          { id: 'ring', kind: 'ring', at: [6, 0], r: 0.9, tone: 'err' },
          { id: 'two', kind: 'text', at: [7.2, 1.6], anchor: 'start', tone: 'err', text: '<en>not joined</en>' }
        ];
      }
    };
  }

  function initTee() {
    var svg = $('#tee-svg');
    if (!svg) return;
    var codeEl = $('#tee-code'), cap = $('#tee-cap');
    var view = new S.View(svg, { box: [-9, -12.6, 21, 4] });
    function show(shared, animate) {
      var L = lines(['* Three resistors', 'RA n1 0 1k', 'RB n1 0 1k', 'RC n1 0 1k', '* @RNL {']);
      if (shared) {
        L.push({ t: '*   %j = (6, 0);' }, { t: '*   &n1 @WIRE {#RA:1 %j #RB:1};', r: 'RA RB net:n1' }, { t: '*   &n1 @WIRE {%j #RC:1};', r: 'RC net:n1' }, { t: '*   $JUNCTION @ATTACH %j', r: 'net:n1' });
      } else {
        L.push({ t: '*   &n1 @WIRE {#RA:1 (6, 0) #RB:1};', r: 'RA RB net:n1' }, { t: '*   &n1 @WIRE {(6, 0) #RC:1}', r: 'RC net:n1' });
      }
      L.push({ t: '* }' });
      C.render(codeEl, L);
      view.setScene(teeScene(shared), { animate: animate, duration: 400 });
      cap.innerHTML = shared
        ? '<en>Both wires share the point object %j, so they join; the dot is drawn explicitly by $JUNCTION, and renderers never add one on their own. The grounds come from the standard package.</en>'
        : '<en>The two (6, 0) are separate point objects that merely share coordinates: RC’s wire is not joined. The drawing looks almost the same, but means something different; use one named point to join them.</en>';
    }
    PG.initSeg($('#tee-seg'), 'data-v', function (v) { show(v === 'shared', true); });
    show(true, false);
  }

  var MODES = [
    ['auto', '<en>Without mode it is auto: the solver’s and viewer’s settings decide, and a solver may optimize the path.</en>'],
    ['straight', '<en>One straight segment.</en>'],
    ['horizontal-vertical', '<en>Horizontal first, then vertical: A → (x₂, y₁) → B.</en>'],
    ['vertical-horizontal', '<en>Vertical first, then horizontal: A → (x₁, y₂) → B.</en>'],
    ['manhattan', '<en>When the horizontal run is longer: half of it, then the whole vertical run, then the other half; otherwise the axes swap. Bends may land on half units.</en>'],
    ['manhattan45', '<en>A horizontal run, a 45° diagonal, then the rest horizontally; when the vertical run is longer, the axes swap.</en>']
  ];

  function modePts(mode, a, b) {
    var dx = b[0] - a[0], dy = b[1] - a[1], ax = Math.abs(dx), ay = Math.abs(dy);
    var sx = dx < 0 ? -1 : 1, sy = dy < 0 ? -1 : 1;
    switch (mode) {
      case 'straight': return [a, b];
      case 'horizontal-vertical': return [a, [b[0], a[1]], b];
      case 'vertical-horizontal': return [a, [a[0], b[1]], b];
      case 'manhattan':
        if (ax >= ay) return [a, [a[0] + dx / 2, a[1]], [a[0] + dx / 2, b[1]], b];
        return [a, [a[0], a[1] + dy / 2], [b[0], a[1] + dy / 2], b];
      case 'manhattan45':
        if (ax >= ay) { var h = (ax - ay) / 2; return [a, [a[0] + sx * h, a[1]], [a[0] + sx * (h + ay), b[1]], b]; }
        var v = (ay - ax) / 2;
        return [a, [a[0], a[1] + sy * v], [b[0], a[1] + sy * (v + ax)], b];
    }
    return null;
  }

  function initModes() {
    var svg = $('#mode-svg');
    if (!svg) return;
    var box = [-2, -2, 16, 11], A = [0, 0], B = [8, 5], mode = 'manhattan';
    var el = S.el;
    svg.setAttribute('viewBox', S.viewBoxOf(box));
    svg.classList.add('sc', 'modes-sc');
    var defs = el('defs', null, svg);
    var pat = el('pattern', { id: 'mdots', x: -0.5, y: -0.5, width: 1, height: 1, patternUnits: 'userSpaceOnUse' }, defs);
    el('circle', { cx: 0.5, cy: 0.5, r: 0.075, class: 'sc-griddot' }, pat);
    el('rect', { x: box[0], y: -box[3], width: box[2] - box[0], height: box[3] - box[1], fill: 'url(#mdots)' }, svg);
    var g = el('g', null, svg);
    var seg = $('#mode-seg'), codeEl = $('#mode-code'), cap = $('#mode-cap');
    seg.innerHTML = MODES.map(function (m) { return '<button type="button" data-mode="' + m[0] + '" aria-pressed="' + (m[0] === mode ? 'true' : 'false') + '">' + m[0] + '</button>'; }).join('');
    function draw() {
      while (g.firstChild) g.removeChild(g.firstChild);
      var p = modePts(mode, A, B);
      if (p) {
        el('path', { class: 'sc-wire mode-path', d: S.pathOf(p) }, g);
        p.slice(1, -1).forEach(function (q) { el('rect', { class: 'mode-bend', x: q[0] - 0.22, y: -q[1] - 0.22, width: 0.44, height: 0.44 }, g); });
      } else {
        el('path', { class: 'sc-air mode-auto', d: S.pathOf([A, B]) }, g);
        el('text', { class: 'mode-q', x: (A[0] + B[0]) / 2 + 0.4, y: -(A[1] + B[1]) / 2 - 0.4 }, g).textContent = '?';
      }
      [[A, 'A', '%a'], [B, 'B', '%b']].forEach(function (q) {
        el('circle', { class: 'mode-pt', cx: q[0][0], cy: -q[0][1], r: 0.32 }, g);
        el('text', { class: 'mode-l', x: q[0][0] + 0.5, y: -q[0][1] + (q[1] === 'A' ? 0.95 : -0.55) }, g).textContent = q[2] + ' ' + pt(q[0]);
      });
      codeEl.innerHTML = C.hlSnippet('%a = ' + pt(A).replace(/−/g, '-') + '; %b = ' + pt(B).replace(/−/g, '-') + ';\n&n @WIRE' + (mode === 'auto' ? '' : '(mode="' + mode + '")') + ' {%a %b}');
      var m = MODES.filter(function (x) { return x[0] === mode; })[0];
      cap.innerHTML = m[1] + (p && p.length > 2 ? ' <span class="muted"><en>The hollow squares are bends added by the mode: they are not objects, cannot be referenced and cannot start a branch.</en></span>' : '');
    }
    PG.initSeg(seg, 'data-mode', function (v) { mode = v; draw(); });
    svg.addEventListener('click', function (e) {
      var ctm = svg.getScreenCTM();
      if (!ctm) return;
      var q = svg.createSVGPoint();
      q.x = e.clientX; q.y = e.clientY;
      var w = q.matrixTransform(ctm.inverse());
      var x = Math.round(w.x), y = Math.round(-w.y);
      x = Math.max(-1, Math.min(15, x)); y = Math.max(-1, Math.min(10, y));
      if (x === A[0] && y === A[1]) return;
      B = [x, y];
      draw();
    });
    draw();
  }

  function initBus() {
    var svg = $('#bus-svg');
    if (!svg) return;
    var gen = 'is-gen';
    var L = [
      { t: '* Two inverters on a bus' },
      { t: '.subckt inv a y vdd vss' },
      { t: 'M1 y a vdd vdd pch' },
      { t: 'M2 y a vss vss nch' },
      { t: '.ends inv' },
      { t: '.model nch nmos level=1' },
      { t: '.model pch pmos level=1' },
      { t: 'XI<1> D<1> Y<1> vdd 0 inv', r: 'XI' },
      { t: 'XI<0> D<0> Y<0> vdd 0 inv', r: 'XI' },
      { t: 'R1 D<1> 0 1k', r: 'R1' },
      { t: 'R0 D<0> 0 1k', r: 'R0' },
      { t: '* @RNL {' },
      { t: '*   #XI<1:0> @BUNDLE => %xi;', r: 'XI' },
      { t: '*   %s = (_, _); %t1 = (_, _); %t0 = (_, _);', r: 'net:D' },
      { t: '*   %bus = &D<1:0> @WIRE {%s %t1 %t0 %xi:a};', r: 'net:D XI' },
      { t: '*   &D<1> @WIRE {%t1 #R1:1};', r: 'net:D1 R1' },
      { t: '*   &D<0> @WIRE {%t0 #R0:1};', r: 'net:D0 R0' },
      { t: '*   %bus @TEXT("D<1:0>")', r: 'net:D' },
      { t: '* }' },
      { t: '.end' }
    ];
    C.render($('#bus-code'), L);
    var view = new S.View(svg, { box: [-3, -7, 27, 10] });
    view.setScene({
      rk: 'bus',
      devices: {
        XI: { sym: 'inv', x: 16, y: 6, label: 'XI<1:0>' },
        R1: { sym: 'res', x: 4, y: 1, label: 'R1', value: '1k' },
        R0: { sym: 'res', x: 8, y: 1, label: 'R0', value: '1k' },
        G1: { sym: 'gnd', x: 4, y: -3, virtual: true, cls: gen },
        G0: { sym: 'gnd', x: 8, y: -3, virtual: true, cls: gen }
      },
      route: function (P) {
        return {
          wires: [
            { net: 'D', pts: [[0, 6], [4, 6], [8, 6], P('XI.a')], cls: 'is-bus' },
            { net: 'D1', pts: [[4, 6], P('R1.p')] },
            { net: 'D0', pts: [[8, 6], P('R0.p')] },
            { net: 'Y', pts: [P('XI.y'), [21.5, 6]], cls: 'is-bus' },
            { net: '0', pts: [P('R1.n'), P('G1.p')], cls: gen },
            { net: '0', pts: [P('R0.n'), P('G0.p')], cls: gen }
          ],
          dots: [],
          labels: [{ text: 'D<1:0>', at: [0.4, 6.8], anchor: 'start', net: 'D' }, { text: 'Y<1:0>', at: [19.4, 6.8], anchor: 'start', net: 'Y' }]
        };
      }
    });
    C.link($('#bus-code'), L, view);
  }

  /* ================= T6 Text ================= */

  function initText() {
    var svg = $('#tx-svg');
    if (!svg) return;
    var codeEl = $('#tx-code'), cap = $('#tx-cap');
    var view = new S.View(svg, { box: [-4, -3, 26, 11.6] });
    var moved = false, keyed = false, prev = null, linker = null;
    function scene() {
      var label = keyed ? '2u/0.5u' : 'W/L = 2u/0.5u';
      return {
        rk: 'tx',
        devices: { M1: { sym: 'nmos', x: moved ? 12 : 4, y: 4, label: 'M1' } },
        route: function () { return { wires: [], dots: [] }; },
        overlays: function (P, S) {
          var m = S('M1');
          return [
            { id: 'wl', ref: 'wl', kind: 'tag', at: [m.x, m.y - 3], anchor: 'middle', text: label, tone: 'user' },
            { id: 'an-pt', ref: 'an', kind: 'ring', at: [2, 9], r: 0.18 },
            { id: 'an', ref: 'an', kind: 'text', at: [2.5, 9], anchor: 'start', text: 'bias branch' }
          ];
        }
      };
    }
    function show(animate, what) {
      var L = [
        { t: '* Label and annotation' },
        { t: '.model nch nmos level=1' },
        { t: 'M1 d g s s nch W=2u L=0.5u', r: 'M1' },
        { t: '* @RNL {' },
        { t: '*   #M1 @PLACE(' + (moved ? 12 : 4) + ', 4);', r: 'M1' },
        { t: '*   #M1 @TEXT("W/L = ${w}/${l}", key="wl", dy=-3);', r: 'M1 ov:wl' },
        { t: '*   (2, 9) @TEXT("bias branch")' + (keyed ? ';' : ''), r: 'ov:an' }
      ];
      if (keyed) L.push({ t: '*   #M1 @TEXT("${w}/${l}", key="wl", dy=-3)', r: 'M1 ov:wl' });
      L.push({ t: '* }' });
      L = marks(prev, L);
      if (prev && what === 'move') L[4].mark = 'is-chg';
      prev = L;
      C.render(codeEl, L);
      if (!linker) linker = C.link(codeEl, L, view); else linker.setLines(L);
      view.setScene(scene(), { animate: animate, duration: 700 });
      cap.innerHTML = what === 'move'
        ? (moved ? '<en>M1 moved to x = 12: the label goes with it, the annotation stays.</en>'
          : '<en>M1 is back at x = 4, and so is its label.</en>')
        : what === 'key'
          ? (keyed ? '<en>Same object and same key means the same text: the later statement replaces the content and keeps the position. Other records find it as #M1::wl.</en>'
            : '<en>Without the second statement, the label has its original content.</en>')
          : '<en>The label is attached to M1: ${w} and ${l} read M1’s parameters, and dy=-3 puts it 3 units below the origin. The annotation is attached to the point (2, 9) and can read only that point’s coordinates.</en>';
    }
    $('#tx-move').addEventListener('click', function () { moved = !moved; show(true, 'move'); });
    $('#tx-key').addEventListener('click', function () { keyed = !keyed; show(true, 'key'); });
    show(false, '');
  }

  /* A small renderer for the text markup of standard 0.3 (13.6), for the playground */
  var GREEK = {
    alpha: 'α', beta: 'β', gamma: 'γ', delta: 'δ', epsilon: 'ϵ', varepsilon: 'ε', zeta: 'ζ', eta: 'η', theta: 'θ', vartheta: 'ϑ',
    iota: 'ι', kappa: 'κ', lambda: 'λ', mu: 'μ', nu: 'ν', xi: 'ξ', pi: 'π', varpi: 'ϖ', rho: 'ρ', varrho: 'ϱ', sigma: 'σ', varsigma: 'ς',
    tau: 'τ', phi: 'ϕ', varphi: 'φ', chi: 'χ', psi: 'ψ', omega: 'ω',
    Gamma: 'Γ', Delta: 'Δ', Theta: 'Θ', Lambda: 'Λ', Xi: 'Ξ', Pi: 'Π', Sigma: 'Σ', Phi: 'Φ', Psi: 'Ψ', Omega: 'Ω',
    pm: '±', times: '×', cdot: '·', infty: '∞', approx: '≈', neq: '≠', leq: '≤', geq: '≥', ll: '≪', gg: '≫', propto: '∝',
    parallel: '∥', angle: '∠', circ: '∘', to: '→', downarrow: '↓', textbackslash: '\\'
  };
  /* upsilon, Upsilon and uparrow are added here so that no escape-like sequence appears in the source */
  GREEK['u' + 'psilon'] = 'υ'; GREEK['U' + 'psilon'] = 'Υ'; GREEK['u' + 'parrow'] = '↑';
  var WRAP = { textbf: ['<b>', '</b>'], textit: ['<i>', '</i>'], overline: ['<span class="ovl">', '</span>'] };
  var M1_ATTR = { name: 'M1', value: '', model: 'nch', type: 'nmos', w: '2u', l: '0.5u' };

  function markup(src) {
    var warn = [], i = 0, n = src.length, BS = '\\';
    function group() {
      /* src[i] is '{': parse to the matching '}' */
      i++;
      var h = seq('}');
      if (src[i] === '}') { i++; return h; }
      warn.push('<en>unmatched brace</en>');
      return h;
    }
    function seq(stop) {
      var out = '';
      while (i < n) {
        var c = src[i];
        if (stop && c === stop) return out;
        if (c === '$' && src[i + 1] === '{') {
          var e = src.indexOf('}', i);
          if (e < 0) { out += esc(src.slice(i)); i = n; break; }
          var key = src.slice(i + 2, e).replace(/^#/, '');
          if (Object.prototype.hasOwnProperty.call(M1_ATTR, key)) out += '<span class="ref">' + esc(M1_ATTR[key]) + '</span>';
          else { out += '<span class="bad">' + esc(src.slice(i, e + 1)) + '</span>'; warn.push(PG.fill('<en>M1 has no property {key}: the text fails</en>', { key: esc(key) })); }
          i = e + 1;
          continue;
        }
        if ((c === '_' || c === '^') && src[i + 1] === '{') {
          i++;
          var inner = group();
          out += c === '_' ? '<sub>' + inner + '</sub>' : '<sup>' + inner + '</sup>';
          continue;
        }
        if (c === BS) {
          var d = src[i + 1];
          if (d === BS) { out += '<br>'; i += 2; continue; }
          if (d && '_{}$%&#'.indexOf(d) >= 0) { out += esc(d); i += 2; continue; }
          var m = /^[A-Za-z]+/.exec(src.slice(i + 1));
          if (!m) { out += BS; i++; continue; }
          var name = m[0];
          var j = i + 1 + name.length;
          if (WRAP[name] && src[j] === '{') {
            i = j;
            out += WRAP[name][0] + group() + WRAP[name][1];
            continue;
          }
          if (name === 'frac' && src[j] === '{') {
            i = j;
            var num = group();
            if (src[i] === '{') {
              var den = group();
              out += '<span class="frac"><span>' + num + '</span><span>' + den + '</span></span>';
            } else {
              out += esc(BS + 'frac') + '{' + num + '}';
              warn.push('<en>\\frac needs two braces right after it</en>');
            }
            continue;
          }
          if (Object.prototype.hasOwnProperty.call(GREEK, name)) {
            out += esc(GREEK[name]);
            i = j;
            /* a command name swallows the spaces after it, and an empty {} right after it */
            while (src[i] === ' ') i++;
            if (src[i] === '{' && src[i + 1] === '}') i += 2;
            continue;
          }
          out += '<span class="bad">' + esc(BS + name) + '</span>';
          warn.push(PG.fill('<en>unknown command \\{name}: shown as written</en>', { name: esc(name) }));
          i = j;
          continue;
        }
        if (c === '{') {
          i++;
          var plain = seq('}');
          if (src[i] === '}') { i++; out += '{' + plain + '}'; }
          else { out += '{' + plain; warn.push('<en>unmatched brace</en>'); }
          continue;
        }
        if (c === '}') { warn.push('<en>unmatched brace</en>'); out += '}'; i++; continue; }
        out += esc(c);
        i++;
      }
      return out;
    }
    var html = seq(null);
    return { html: html, warn: warn };
  }

  var MK_PRESETS = ['R_{load}', 'W/L = ${w}/${l}', '\\frac{${w}}{${l}}', 'f_{c} = \\frac{1}{2\\pi RC}', '\\overline{RST}', 'V_{DD} = 1.8 V\\\\I_{D} = 10 \\mu A', 'g_m', '\\sqrt{2}'];

  function initMarkup() {
    var input = $('#mk-src');
    if (!input) return;
    var view = $('#mk-view'), warnEl = $('#mk-warn'), chips = $('#mk-chips');
    chips.innerHTML = MK_PRESETS.map(function (p) { return '<button type="button" class="kw mk-chip">' + esc(p) + '</button>'; }).join('');
    function update() {
      var r = markup(input.value);
      view.innerHTML = '<span class="mk-line">' + (r.html || '&nbsp;') + '</span>';
      warnEl.innerHTML = r.warn.length ? icon('warn') + '<span>' + r.warn.filter(function (w, k, a) { return a.indexOf(w) === k; }).join('<en>; </en>') + '</span>' : icon('check') + '<span><en>No warnings</en></span>';
      warnEl.classList.toggle('is-ok', !r.warn.length);
    }
    $$('.mk-chip', chips).forEach(function (b) {
      b.addEventListener('click', function () { input.value = b.textContent; update(); });
    });
    input.addEventListener('input', update);
    update();
  }

  /* ================= T7 Orientation ================= */

  var CODES = ['R0', 'R90', 'R180', 'R270', 'MY', 'MX', 'MYR90', 'MXR90'];
  var MAP = { R0: '(x, y)', R90: '(−y, x)', R180: '(−x, −y)', R270: '(y, −x)', MY: '(−x, y)', MX: '(x, −y)', MYR90: '(−y, −x)', MXR90: '(y, x)' };
  var CODE_NOTE = {
    R0: '<en>unchanged</en>', R90: '<en>90° counterclockwise</en>', R180: '<en>turned 180°</en>',
    R270: '<en>270° counterclockwise, i.e. 90° clockwise</en>', MY: '<en>flipped left-right</en>',
    MX: '<en>flipped upside down</en>', MYR90: '<en>MY, then R90</en>', MXR90: '<en>MX, then R90</en>'
  };
  function matOf(code) { var p = S.apply(S.stateOf({ x: 0, y: 0, o: code }), [1, 0]), q = S.apply(S.stateOf({ x: 0, y: 0, o: code }), [0, 1]); return [p[0], q[0], p[1], q[1]]; }
  function codeOfMat(m) { return CODES.filter(function (c) { var k = matOf(c); return k[0] === m[0] && k[1] === m[1] && k[2] === m[2] && k[3] === m[3]; })[0]; }
  function mul(a, b) { return [a[0] * b[0] + a[1] * b[2], a[0] * b[1] + a[1] * b[3], a[2] * b[0] + a[3] * b[2], a[2] * b[1] + a[3] * b[3]]; }

  function initOrient() {
    var svg = $('#or-svg');
    if (!svg) return;
    var title = $('#or-title'), pins = $('#or-pins'), grid = $('#or-grid');
    var cur = 'R0';
    var view = new S.View(svg, { box: [-10, -10, 10, 10] });
    var PINS = [['d', 'D', [0, 2]], ['g', 'G', [-3, 0]], ['s', 'S', [0, -2]]];
    function scene() {
      return {
        rk: 'or',
        devices: { M: { sym: 'nmos', x: 0, y: 0, o: cur, label: '' } },
        route: function () { return { wires: [], dots: [] }; },
        overlays: function (P) {
          var L = [
            { id: 'ax', kind: 'arrow', pts: [[-9, 0], [9, 0]], tone: 'mute', head: 0.35, label: 'x', labelAt: [9.05, 0.8], anchor: 'end' },
            { id: 'ay', kind: 'arrow', pts: [[0, -9], [0, 9]], tone: 'mute', head: 0.35, label: 'y', labelAt: [0.6, 8.7], anchor: 'start' },
            { id: 'o', kind: 'ring', at: [0, 0], r: 0.22, tone: 'mute' }
          ];
          var st = S.stateOf({ x: 0, y: 0, o: cur });
          PINS.forEach(function (p) {
            var at = P('M.' + p[0]), fin = S.apply(st, p[2]);
            L.push({ id: 'p' + p[0], ref: 'pin', kind: 'ring', at: at, r: 0.4, tone: 'user' });
            var off = [fin[0] === 0 ? 0.7 : (fin[0] > 0 ? 0.6 : -0.6), fin[1] === 0 ? 0.85 : (fin[1] > 0 ? 0.6 : -0.6)];
            L.push({ id: 't' + p[0], ref: 'pin', kind: 'tag', at: [at[0] + off[0], at[1] + off[1]], anchor: off[0] >= 0 ? 'start' : 'end', text: p[1] + ' ' + pt(fin), tone: 'user' });
          });
          return L;
        }
      };
    }
    grid.innerHTML = CODES.map(function (c) {
      return '<button type="button" data-c="' + c + '" aria-pressed="false"><svg class="or-mini" aria-hidden="true"></svg><span class="or-code">' + c + '</span><span class="or-map">' + MAP[c] + '</span></button>';
    }).join('');
    $$('button', grid).forEach(function (b) {
      var v = new S.View($('svg', b), { box: [-3.6, -3.4, 2.4, 3.4], grid: false });
      var c = b.getAttribute('data-c');
      v.setScene({ rk: 'mini', devices: { M: { sym: 'nmos', x: 0, y: 0, o: c, label: '' } }, route: function () { return { wires: [], dots: [] }; } });
      b.addEventListener('click', function () { set(c, true); });
    });
    function set(c, animate) {
      cur = c;
      $$('button', grid).forEach(function (b) { b.setAttribute('aria-pressed', b.getAttribute('data-c') === c ? 'true' : 'false'); });
      title.innerHTML = '#M1 @ORIENT("' + c + '") · ' + CODE_NOTE[c];
      var st = S.stateOf({ x: 0, y: 0, o: c });
      pins.innerHTML = '<thead><tr><th><en>Terminal</en></th><th><en>In the symbol</en></th><th><en>Drawn at</en></th></tr></thead><tbody>' +
        PINS.map(function (p) { return '<tr><td>' + p[1] + '</td><td>' + pt(p[2]) + '</td><td><b>' + pt(S.apply(st, p[2])) + '</b></td></tr>'; }).join('') +
        '</tbody><caption>' + c + ' : (x, y) → ' + MAP[c] + '</caption>';
      view.setScene(scene(), { animate: animate, duration: 800 });
    }
    set('R0', false);
  }

  function initRelation() {
    var svg = $('#rel-svg');
    if (!svg) return;
    var sa = $('#rel-a'), stt = $('#rel-t'), codeEl = $('#rel-code'), cap = $('#rel-cap');
    var opts = CODES.map(function (c) { return '<option value="' + c + '">' + c + '</option>'; }).join('');
    sa.innerHTML = opts;
    stt.innerHTML = opts;
    sa.value = 'R90';
    stt.value = 'MY';
    var view = new S.View(svg, { box: [-14, -6.2, 14, 5.4] });
    function show(animate) {
      var a = sa.value, t = stt.value, b = codeOfMat(mul(matOf(t), matOf(a)));
      view.setScene({
        rk: 'rel',
        devices: {
          A: { sym: 'nmos', x: -6, y: 0, o: a, label: 'A', lab: [0, -4.2, 'middle'] },
          B: { sym: 'nmos', x: 6, y: 0, o: b, label: 'B', lab: [0, -4.2, 'middle'] }
        },
        route: function () { return { wires: [], dots: [] }; },
        overlays: function () {
          return [
            { id: 'oa', kind: 'tag', at: [-6, 3.6], anchor: 'middle', text: a },
            { id: 'ob', kind: 'tag', at: [6, 3.6], anchor: 'middle', text: b, tone: 'user' }
          ];
        }
      }, { animate: animate, duration: 700 });
      codeEl.innerHTML = C.hlSnippet('#A @ORIENT("' + a + '");\n#A #B @ORIENT-RELATION("' + t + '")');
      cap.innerHTML = PG.fill('<en>A is {a} and the relation is {t}, so B is <b>{b}</b>: A’s orientation followed by {t}.</en>', { a: a, t: t, b: b });
    }
    sa.addEventListener('change', function () { show(true); });
    stt.addEventListener('change', function () { show(true); });
    show(false);
  }

  /* ================= T8 Priority ================= */

  var PRIOS = ['disabled', 'low', 'normal', 'high'];
  var RANK = { disabled: 0, low: 1, normal: 2, high: 3 };
  var PR = [
    { text: function (p) { return '#M1 @PLACE(x=4' + (p === 'normal' ? '' : ', priority="' + p + '"') + ')'; }, def: 'low', fact: function (a, b) { return a === 4; } },
    { text: function (p) { return '#M2 @PLACE(x=12' + (p === 'normal' ? '' : ', priority="' + p + '"') + ')'; }, def: 'normal', fact: function (a, b) { return b === 12; } },
    { text: function (p) { return '{#M1 #M2} @ALIGN("x"' + (p === 'normal' ? '' : ', priority="' + p + '"') + ')'; }, def: 'high', fact: function (a, b) { return a === b; } },
    { text: function (p) { return '#M1 #M2 @DISTANCE("left-to-right", min=2' + (p === 'normal' ? '' : ', priority="' + p + '"') + ')'; }, def: 'normal', fact: function (a, b) { return b - a >= 2; } },
    { text: function (p) { return '%c = #M1 #M2 @DISTANCE("x", min=1)' + (p === 'normal' ? '' : ' => @PRIORITY("' + p + '")'); }, def: 'disabled', fact: function (a, b) { return Math.abs(b - a) >= 1; } }
  ];
  var PR_NOTE = [
    '<en>M1’s x = 4</en>', '<en>M2’s x = 12</en>', '<en>same x</en>',
    '<en>M2 at least 2 right of M1</en>', '<en>at least 1 apart, either way</en>'
  ];
  var GRID = []; for (var gx = -4; gx <= 24; gx++) GRID.push(gx);

  function solve(kept) {
    var sols = [];
    GRID.forEach(function (a) {
      GRID.forEach(function (b) {
        for (var k = 0; k < kept.length; k++) if (!PR[kept[k]].fact(a, b)) return;
        sols.push([a, b]);
      });
    });
    return sols;
  }

  function initPriority() {
    var rows = $('#prio-rows');
    if (!rows) return;
    var svg = $('#prio-svg'), cap = $('#prio-cap');
    var prio = PR.map(function (r) { return r.def; });
    rows.innerHTML = PR.map(function (r, i) {
      return '<div class="prio-row" data-i="' + i + '"><span class="prio-n"></span><code class="prio-code"></code>' +
        '<label class="prio-sel"><span class="vh"><en>Priority</en></span><select data-i="' + i + '">' +
        PRIOS.map(function (p) { return '<option value="' + p + '">' + p + '</option>'; }).join('') + '</select></label>' +
        '<span class="prio-st"></span><span class="prio-note">' + PR_NOTE[i] + '</span></div>';
    }).join('') + '<div class="prio-foot"><button class="btn btn-sm" type="button" id="prio-reset">' + icon('replay') + '<en>Reset</en></button><span class="muted"><en>Taken by priority, highest first; within one priority, later first.</en></span></div>';
    var el = S.el;
    /* number line: x from -2 to 22 */
    function X(v) { return 24 + (v + 2) * 14.5; }
    el('line', { class: 'pr-axis', x1: 14, y1: 110, x2: 392, y2: 110 }, svg);
    for (var v = -2; v <= 22; v += 2) {
      el('line', { class: 'pr-tick', x1: X(v), y1: 106, x2: X(v), y2: 114 }, svg);
      el('text', { class: 'pr-tl', x: X(v), y: 126, 'text-anchor': 'middle' }, svg).textContent = v;
    }
    var marks4 = el('line', { class: 'pr-ref', x1: X(4), y1: 18, x2: X(4), y2: 106 }, svg);
    var marks12 = el('line', { class: 'pr-ref', x1: X(12), y1: 18, x2: X(12), y2: 106 }, svg);
    void marks4; void marks12;
    var boxes = ['M1', 'M2'].map(function (n, k) {
      var g = el('g', { class: 'pr-box' }, svg);
      el('rect', { x: -17, y: -13, width: 34, height: 26, rx: 4 }, g);
      el('text', { x: 0, y: 1 }, g).textContent = n;
      g.style.transform = 'translate(' + X(k ? 12 : 4) + 'px, ' + (k ? 76 : 36) + 'px)';
      return g;
    });
    function update() {
      var order = PR.map(function (r, i) { return i; }).filter(function (i) { return prio[i] !== 'disabled'; })
        .sort(function (a, b) { return RANK[prio[b]] - RANK[prio[a]] || b - a; });
      var kept = [], status = {}, rank = {};
      order.forEach(function (i, k) {
        rank[i] = k + 1;
        var trial = kept.concat([i]);
        if (solve(trial).length) { kept = trial; status[i] = 'kept'; } else status[i] = 'dropped';
      });
      $$('.prio-row', rows).forEach(function (row, i) {
        var p = prio[i];
        $('.prio-code', row).innerHTML = C.hlSnippet(PR[i].text(p));
        $('select', row).value = p;
        $('.prio-n', row).textContent = rank[i] ? rank[i] : '–';
        var st = status[i] || 'off';
        row.className = 'prio-row is-' + st;
        $('.prio-st', row).innerHTML = st === 'kept' ? '<span class="chip chip-ok">' + icon('check') + '<en>kept</en></span>'
          : st === 'dropped' ? '<span class="chip chip-err">' + icon('cross') + '<en>dropped</en></span>'
            : '<span class="chip chip-mute"><en>disabled</en></span>';
      });
      var sols = solve(kept);
      var xs1 = sols.map(function (s) { return s[0]; }), xs2 = sols.map(function (s) { return s[1]; });
      var fixed1 = xs1.every(function (v) { return v === xs1[0]; }), fixed2 = xs2.every(function (v) { return v === xs2[0]; });
      /* draw the arrangement closest to what was written: x = 4 and x = 12 */
      var best = sols.slice().sort(function (p, q) { return (Math.abs(p[0] - 4) + Math.abs(p[1] - 12)) - (Math.abs(q[0] - 4) + Math.abs(q[1] - 12)); })[0];
      [best[0], best[1]].forEach(function (x, k) {
        boxes[k].style.transform = 'translate(' + X(Math.max(-2, Math.min(22, x))) + 'px, ' + (k ? 76 : 36) + 'px)';
        boxes[k].classList.toggle('is-free', !(k ? fixed2 : fixed1));
      });
      function desc(n, fixed, v) {
        return fixed ? n + ' x = ' + v : PG.fill('<en>{n}’s x is left to the solver</en>', { n: n });
      }
      cap.innerHTML = desc('M1', fixed1, xs1[0]) + '<en>; </en>' + desc('M2', fixed2, xs2[0]) + '<en>.</en>' +
        (fixed1 && fixed2 && xs1[0] === xs2[0] ? ' <en>They are aligned, and the DISTANCE is dropped.</en>' : '');
    }
    $$('select', rows).forEach(function (s) {
      s.addEventListener('change', function () { prio[+s.getAttribute('data-i')] = s.value; update(); });
    });
    $('#prio-reset').addEventListener('click', function () { prio = PR.map(function (r) { return r.def; }); update(); });
    update();
  }

  /* ================= T9 Header ================= */

  var HDR = [
    [['* ', ''], ['@RNL(type="header")', 'rec'], [' {', '']],
    [['*   ', ''], ['@VERSION("0.3")', 'ver'], [';', '']],
    [['*   ', ''], ['@USE-ALIASES(name="rnl-zh", version="0.3")', 'ali'], [';', '']],
    [['*   ', ''], ['@DEVICE(name="XM*", model="sky130_fd_pr__nfet_*", terminals=["d", "g", "s", "b"])', 'dev']],
    [['* ', ''], ['}', '']]
  ];
  var HDR_TIP = {
    rec: ['type="header"', '<en>This is the header record: the file’s first record, outside any subcircuit, at most one. It is always read in standard spelling, without aliases.</en>'],
    ver: ['@VERSION', '<en>The language version of the file. Without it the tool’s default version applies, and tools add it when they save the file.</en>'],
    ali: ['@USE-ALIASES', '<en>Loads an alias package. rnl-zh gives Chinese spellings to keywords, parameter names and the names in the standard package, so later records may mix both. version= says which version: without it the newest found is used, and a later version could change how the file reads.</en>'],
    dev: ['@DEVICE', '<en>Names the terminals of devices the netlist does not define: instances whose name matches XM* and model matches sky130_fd_pr__nfet_* get the terminals d, g, s and b, in order.</en>']
  };

  function initHeader() {
    var line = $('#hdr-line');
    if (!line) return;
    var tip = $('#hdr-tip'), IDLE = tip.innerHTML;
    function seg(s) {
      if (/^\*\s*$/.test(s)) return '<span class="t-pref">*</span>' + s.slice(1);
      if (s === ' {' || s === ';' || s === '}') return '<span class="t-punct">' + esc(s) + '</span>';
      if (s === '@RNL(type="header")') {
        return '<span class="t-rec">@RNL</span><span class="t-punct">(</span><span class="t-pn">type</span><span class="t-punct">=</span>' +
          '<span class="t-str">"header"</span><span class="t-punct">)</span>';
      }
      return C.hlSnippet(s);
    }
    line.innerHTML = HDR.map(function (row) {
      return row.map(function (s) {
        return s[1] ? '<span class="tok" data-k="' + s[1] + '" tabindex="0">' + seg(s[0]) + '</span>' : seg(s[0]);
      }).join('');
    }).join('\n');
    var toks = $$('.tok', line);
    function show(t) {
      toks.forEach(function (x) { x.classList.toggle('is-on', x === t); });
      if (!t) { tip.innerHTML = IDLE; return; }
      var a = HDR_TIP[t.getAttribute('data-k')];
      tip.innerHTML = '<span class="k">' + a[0] + '</span><span>' + a[1] + '</span>';
    }
    toks.forEach(function (t) {
      t.addEventListener('mouseenter', function () { show(t); });
      t.addEventListener('focus', function () { show(t); });
    });
    line.addEventListener('mouseleave', function () { show(null); });
  }

  function initDevice() {
    var codeEl = $('#dev-code');
    if (!codeEl) return;
    var diag = $('#dev-diag');
    function show(on) {
      var L = [
        { t: '* PDK example' },
        { t: on ? '* @RNL(type="header") { @VERSION("0.3"); @DEVICE(name="XM*", model="sky130_fd_pr__nfet_*", terminals=["d", "g", "s", "b"])}' : '* @RNL(type="header") { @VERSION("0.3")}', mark: 'is-chg' },
        { t: 'XM1 out in 0 0 sky130_fd_pr__nfet_01v8 w=1 l=0.15' },
        { t: '* @RNL {' },
        { t: '*   #XM1:d @CURRENT("in")', cls: on ? '' : 'is-err' },
        { t: '* }' },
        { t: '.end' }
      ];
      C.render(codeEl, L);
      diag.innerHTML = on
        ? '<li class="ok">' + icon('check') + '<span><en><code>#XM1:d</code> is XM1’s first terminal, named by @DEVICE.</en></span></li>' +
          '<li class="info">' + icon('info') + '<span><en>Patterns let one statement cover a whole family: every nfet variant matches sky130_fd_pr__nfet_*.</en></span></li>'
        : '<li class="err">' + icon('cross') + '<span><en><code>#XM1:d</code>: XM1 has no terminal d. The netlist does not define sky130_fd_pr__nfet_01v8, so its terminal names are unknown; only this statement fails.</en></span></li>' +
          '<li class="info">' + icon('info') + '<span><en>The position <code>#XM1:1</code> still works; or add @DEVICE to the header record.</en></span></li>';
    }
    PG.initSeg($('#dev-seg'), 'data-v', function (v) { show(v === '1'); });
    show(true);
  }

  /* ================= Startup ================= */

  function boot() {
    initLab();
    initFile();
    initMirror();
    initPair();
    initStd();
    initCe();
    initSpread();
    initSel();
    initLevels();
    initTee();
    initModes();
    initBus();
    initText();
    initMarkup();
    initOrient();
    initRelation();
    initPriority();
    initHeader();
    initDevice();
  }

  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', boot);
  else boot();
})();
