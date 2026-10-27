/* RNL introduction page · the example: a five-transistor OTA, reviewed by what it says
 *
 * The files are site/examples/ota5t-review-bare.sp (the notation), ota5t-review.sp (English sentences) and
 * ota5t-review-zh.sp (Chinese sentences, only on the Chinese pages), and ota5t-review-eda.sp: the same file with the
 * public packages eda-symbol and eda-style. This script keeps their lines, and site/build.py checks them against the files.
 * The drawings are not drawn here: ota-looks.js holds what the RNL core draws from the standard-packages file and from
 * the file with the two packages (node site/render-examples.mjs), with the parts of every device marked in it.
 * The review: picking a statement of meaning, in the file or in the checklist, lights up the devices it is about in the
 * drawing and their lines in the netlist. The look switch swaps one core drawing for the other; the circuit is the same.
 */
(function (global) {
  'use strict';

  var C = global.RNLCode;
  var REDUCE = !!(global.matchMedia && global.matchMedia('(prefers-reduced-motion: reduce)').matches);

  /* ---------- The file ---------- */

  /* Each statement of meaning in the writing styles: the formal notation, the standard English sentences, the Chinese
     ones (the alias package rnl-zh; only the Chinese pages have them) */
  var T = {
    dp: ['%dp1 = #M1(in_p) #M2(in_n) #M5(tail) @AS $DIFFERENTIAL_PAIR;', '%dp1 = #M1(in_p), #M2(in_n) and #M5(tail) treated @AS $DIFFERENTIAL_PAIR;'],
    cm1: ['%cm1 = #M3(ref) #M4(out) @AS $CURRENT_MIRROR;', '%cm1 = #M3(ref) and #M4(out) treated @AS $CURRENT_MIRROR;'],
    y1: ['{#M1 #M2 #M5} @SET-SYMBOL $NMOS;', '{#M1 #M2 #M5} @SET-SYMBOL to $NMOS;'],
    y2: ['{#M3 #M4} @SET-SYMBOL $PMOS;', '{#M3 #M4} @SET-SYMBOL to $PMOS;'],
    n1: ['&vdd @ADD-TO $SUPPLY_NET_SET;', '&vdd @ADD-TO $SUPPLY_NET_SET;'],
    n2: ['&vss @ADD-TO $GROUND_NET_SET;', '&vss @ADD-TO $GROUND_NET_SET;'],
    ax: ['%ax = (_, _);', '%ax = (_, _);'],
    s12: ['%sym12 = #M1 #M2 %ax @AS $MIRRORED;', '%sym12 = #M1(a), #M2(b) and %ax(axis) treated @AS $MIRRORED;'],
    s34: ['%sym34 = #M3 #M4 %ax @AS $MIRRORED;', '%sym34 = #M3(a), #M4(b) and %ax(axis) treated @AS $MIRRORED;'],
    ord: ['{#M5 #M1 #M3} @ORDER("bottom-to-top");', '{#M5 #M1 #M3} are in @ORDER("bottom-to-top");'],
    cur: ['#M5:d @CURRENT("in")', '#M5:d has @CURRENT("in")']
  };
  var TZ = {};
  /*zh-CN*/
  TZ = {
    dp: '%dp1 = #M1(正输入)、#M2(负输入) 和 #M5(尾管) @视作 $"差分对";',
    cm1: '%cm1 = #M3(参考管) 和 #M4(输出管) @视作 $"电流镜";',
    y1: '{#M1 #M2 #M5} @使用符号 $NMOS;',
    y2: '{#M3 #M4} @使用符号 $PMOS;',
    n1: '&vdd @加入 $"电源网络集";',
    n2: '&vss @加入 $"地网络集";',
    ax: '%ax = (_, _);',
    s12: '%sym12 = #M1(甲)、#M2(乙) 和 %ax(轴) @视作 $"镜像对称";',
    s34: '%sym34 = #M3(甲)、#M4(乙) 和 %ax(轴) @视作 $"镜像对称";',
    ord: '{#M5 #M1 #M3} 按 @顺序("从下到上") 排列;',
    cur: '#M5:d 有 @电流("流入")'
  };
  /*/zh-CN*/
  var STYLES = ['bare', 'en', 'zh'];
  var FILES = { bare: 'ota5t-review-bare.sp', en: 'ota5t-review.sp', zh: 'ota5t-review-zh.sp', eda: 'ota5t-review-eda.sp' };
  var PACKAGES = '; @USE-PACKAGE(name="eda-symbol", version="0.1"); @USE-PACKAGE(name="eda-style", version="0.1")';

  /* The lines of the file as the panel shows them; r is what a line is about (devices and display objects, and the notes
     of a statement as ov:…), mode the note it makes in the drawing, dev a line of the netlist that is about one device */
  function lines(style, look) {
    var k = Math.max(0, STYLES.indexOf(style || 'en'));
    var L = [];
    function add(id, t, r, extra) {
      var o = { id: id, t: t, r: r || '' };
      if (extra) for (var key in extra) o[key] = extra[key];
      L.push(o);
    }
    function st(id, r) { add(id, '*   ' + (k === 2 ? TZ[id] : T[id][k]), r); }
    function dev(id, t) { add(id.toLowerCase(), t, id, { dev: id }); }
    add('h0', '* Five-transistor OTA');
    add('h1', '.model nch nmos level=1 vto=0.5 kp=100u lambda=0.05');
    add('h2', '.model pch pmos level=1 vto=-0.5 kp=100u lambda=0.05');
    add('h3', '* @RNL(type="header") { @VERSION("0.3")' + (k === 2 ? '; @USE-ALIASES(name="rnl-zh", version="0.3")' : '') + (look === 'eda' ? PACKAGES : '') + '}', '', look === 'eda' ? { cls: 'is-diff' } : null);
    add('h4', '.subckt ota5t inp inn out vdd vss vbias', 'PI PN PO PB');
    dev('M1', 'M1 x    inp   tail vss nch W=2u L=0.5u');
    dev('M2', 'M2 out  inn   tail vss nch W=2u L=0.5u');
    dev('M3', 'M3 x    x     vdd  vdd pch W=4u L=0.5u');
    dev('M4', 'M4 out  x     vdd  vdd pch W=4u L=0.5u');
    dev('M5', 'M5 tail vbias vss  vss nch W=4u L=1u');
    add('r0', '* @RNL {');
    st('dp', 'M1 M2 M5 ov:dp');
    st('cm1', 'M3 M4 ov:cm1');
    st('y1', 'M1 M2 M5');
    st('y2', 'M3 M4');
    st('n1', 'VDD');
    st('n2', 'VSS');
    st('ax', 'ov:sym');
    st('s12', 'M1 M2 ov:sym');
    st('s34', 'M3 M4 ov:sym');
    st('ord', 'M5 M1 M3 ov:ord');
    st('cur', 'M5 ov:cur');
    add('r1', '* }');
    add('d0', '* @RNL(author="agent") {');
    add('d1', '*   #M1 @PLACE(12, 8) => @ORIENT("R0");  #M2 @PLACE(28, 8) => @ORIENT("MY");', 'M1 M2');
    add('d2', '*   #M3 @PLACE(12, 16) => @ORIENT("MY");  #M4 @PLACE(28, 16) => @ORIENT("R0");', 'M3 M4');
    add('d3', '*   #M5 @PLACE(20, 0);  %ja = (16, 16);  %jb = (12, 12);', 'M5');
    add('d4', '*   &x @WIRE {#M3:d %jb #M1:d};  &x @WIRE {#M3:g %ja #M4:g};');
    add('d5', '*   &x @WIRE(mode="vertical-horizontal") {%ja %jb};');
    add('d6', '*   %jo = (28, 12);  %jt = (20, 4);');
    add('d7', '*   &out @WIRE {#M4:d %jo};  &out @WIRE(mode="horizontal-vertical") {%jo #M2:d};');
    add('d8', '*   &tail @WIRE(mode="vertical-horizontal") {#M1:s %jt};');
    add('d9', '*   &tail @WIRE(mode="vertical-horizontal") {#M2:s %jt};');
    add('da', '*   &tail @WIRE {%jt #M5:d};');
    add('db', '*   $JUNCTION @ATTACH {%ja %jb %jo %jt};');
    add('dc', '*   %pi = $IO_PORT @ATTACH &inp;  %pi @PLACE(5, 8);', 'PI');
    add('dd', '*   %pn = $IO_PORT @ATTACH &inn;  %pn @PLACE(35, 8) => @ORIENT("MY");', 'PN');
    add('de', '*   %po = $IO_PORT @ATTACH &out;  %po @PLACE(35, 12) => @ORIENT("MY");', 'PO');
    add('df', '*   %pb = $IO_PORT @ATTACH &vbias;  %pb @PLACE(13, 0);', 'PB');
    add('dg', '*   &inp @WIRE {#M1:g %pi:p};  &inn @WIRE {#M2:g %pn:p};');
    add('dh', '*   &out @WIRE {%jo %po:p};  &vbias @WIRE {#M5:g %pb:p};');
    add('di', '*   %vdd1 = $SUPPLY_STUB @ATTACH #M3:s;  %vdd1::s @PLACE(12, 20);', 'VDD');
    add('dj', '*   %vdd2 = $SUPPLY_STUB @ATTACH #M4:s;  %vdd2::s @PLACE(28, 20);', 'VDD');
    add('dk', '*   %gnd = $GROUND_STUB @ATTACH #M5:s;  %gnd::g @PLACE(20, -4);', 'VSS');
    add('dl', '*   %ax @PLACE(20, 8)');
    add('d9z', '* }');
    add('e0', '.ends ota5t');
    add('e1', '.end');
    return L;
  }

  /* ---------- The review ---------- */

  /* One item of the checklist for each statement of meaning: what the agent declared, in plain words */
  var ITEMS = [
    { key: 'dp', kind: 'structure', devs: 'M1 M2 M5', stmt: ['dp'], text: '<en><b>M1</b> and <b>M2</b> are the input differential pair; <b>M5</b> is its tail current source.</en>' },
    { key: 'cm1', kind: 'structure', devs: 'M3 M4', stmt: ['cm1'], text: '<en><b>M3</b> and <b>M4</b> are a current mirror: the load of the pair.</en>' },
    { key: 'sym', kind: 'intent', devs: 'M1 M2 M3 M4', stmt: ['s12', 's34', 'ax'], text: '<en>The circuit is symmetric: <b>M1</b> and <b>M2</b>, and <b>M3</b> and <b>M4</b>, are mirror images about one axis.</en>' },
    { key: 'ord', kind: 'intent', devs: 'M5 M1 M3', stmt: ['ord'], text: '<en>From bottom to top: the tail <b>M5</b>, the pair <b>M1</b>, the load <b>M3</b>.</en>' },
    { key: 'cur', kind: 'intent', devs: 'M5', stmt: ['cur'], text: '<en>The tail current enters at the drain of <b>M5</b>.</en>' }
  ];

  var KIND = { structure: '<en>structure</en>', intent: '<en>intent</en>' };

  var DEFAULT = '<en>Pick a statement in the file or an item of the checklist: the devices it is about light up in the drawing and in the netlist. Nothing here needs the netlist to be read line by line.</en>';
  var CAPS = {
    dp: '<en><b>Structure</b>: <code>$DIFFERENTIAL_PAIR</code> says which devices form the pair and which is its tail. The standard package’s definition puts the tail under the middle of the pair.</en>',
    cm1: '<en><b>Structure</b>: <code>$CURRENT_MIRROR</code> says which device is the reference and which one copies it: here the load of the pair.</en>',
    sym: '<en><b>Intent</b>: two mirrored pairs about one axis. The standard package’s definition puts each pair at one height, mirror images about the axis; a tool that draws from it keeps the symmetry.</en>',
    ord: '<en><b>Intent</b>: the tail at the bottom, the pair above it, the load on top. A drawing that follows it reads as a schematic usually does.</en>',
    cur: '<en><b>Intent</b>: where the current flows. The tail current runs from the top to the bottom of the circuit.</en>'
  };
  var SIM = '<en>A simulator reads the lines that are not dimmed: the netlist. Every RNL line starts with <code>*</code>, so for the simulator it is a comment; nothing has to be stripped before it runs.</en>';

  /* What picking a statement shows in the drawing besides the lit devices, in the drawing's own units (the core draws
     the grid unit as 14: x grows to the right, y downwards) */
  function arrow(x0, y0, x1, y1) {
    var dx = x1 - x0, dy = y1 - y0, n = Math.sqrt(dx * dx + dy * dy), ux = dx / n, uy = dy / n, h = 7;
    var bx = x1 - ux * h * 1.4, by = y1 - uy * h * 1.4;
    return '<line x1="' + x0 + '" y1="' + y0 + '" x2="' + bx + '" y2="' + by + '"/>' +
      '<path class="head" d="M' + x1 + ' ' + y1 + ' L' + (bx - uy * h * 0.6) + ' ' + (by + ux * h * 0.6) + ' L' + (bx + uy * h * 0.6) + ' ' + (by - ux * h * 0.6) + ' Z"/>';
  }
  function label(x, y, text, anchor) { return '<text x="' + x + '" y="' + y + '" text-anchor="' + (anchor || 'middle') + '">' + text + '</text>'; }
  var NOTES = {
    dp: label(280, -104, '%dp1'),
    cm1: label(280, -236, '%cm1'),
    sym: '<line class="axis" x1="280" y1="-300" x2="280" y2="40"/>' + label(286, -302, '%ax', 'start'),
    ord: arrow(0, 40, 0, -290) + label(10, -296, '@ORDER', 'start'),
    cur: arrow(266, -56, 266, -36) + label(258, -42, 'I', 'end')
  };

  /* The simulator's answer: the AC response of the testbench from the Spectre run (ota-sim.js, site/sim/make-result.py) */
  function fmtHz(f) {
    var u = f >= 1e9 ? [1e9, ' GHz'] : f >= 1e6 ? [1e6, ' MHz'] : f >= 1e3 ? [1e3, ' kHz'] : [1, ' Hz'];
    var v = f / u[0];
    return (v >= 100 ? v.toFixed(0) : v.toPrecision(2)).replace(/\.0$/, '') + u[1];
  }
  function drawSim(el) {
    var R = global.RNLOtaSim;
    if (!el || !R) return;
    var W = 520, H = 372, L = 46, Rr = 10, lx0 = Math.log10(R.freq[0]), lx1 = Math.log10(R.freq[R.freq.length - 1]);
    function X(f) { return L + (Math.log10(f) - lx0) / (lx1 - lx0) * (W - L - Rr); }
    var panels = [
      { y0: 22, y1: 170, lo: -60, hi: 50, ticks: [-60, -40, -20, 0, 20, 40], data: R.gain_db, unit: 'dB' },
      { y0: 214, y1: 330, lo: -135, hi: 0, ticks: [-135, -90, -45, 0], data: R.phase_deg, unit: '°' }
    ];
    var s = '<svg class="bode" viewBox="0 0 ' + W + ' ' + H + '" role="img" aria-label="AC response from Spectre: gain and phase of the OTA">';
    var d, i;
    panels.forEach(function (p) {
      function Y(v) { return p.y1 - (v - p.lo) / (p.hi - p.lo) * (p.y1 - p.y0); }
      for (d = Math.ceil(lx0); d <= lx1; d++) {
        var x = X(Math.pow(10, d));
        s += '<line class="bd-grid" x1="' + x + '" x2="' + x + '" y1="' + p.y0 + '" y2="' + p.y1 + '"/>';
      }
      p.ticks.forEach(function (v) {
        s += '<line class="bd-grid" x1="' + L + '" x2="' + (W - Rr) + '" y1="' + Y(v) + '" y2="' + Y(v) + '"/>' +
          '<text class="bd-t" x="' + (L - 6) + '" y="' + (Y(v) + 3.5) + '" text-anchor="end">' + v + '</text>';
      });
      s += '<text class="bd-t" x="' + (L - 6) + '" y="' + (p.y0 - 8) + '" text-anchor="end">' + p.unit + '</text>';
      var pts = R.freq.map(function (f, k) { return X(f).toFixed(1) + ' ' + Math.max(p.y0, Math.min(p.y1, Y(p.data[k]))).toFixed(1); });
      s += '<path class="bd-curve" d="M' + pts.join(' L') + '"/>';
    });
    [0, 2, 4, 6, 8, 10].forEach(function (e) {
      if (e < lx0 || e > lx1) return;
      var lab = e === 0 ? '1' : e === 2 ? '100' : e === 4 ? '10k' : e === 6 ? '1M' : e === 8 ? '100M' : '10G';
      s += '<text class="bd-t" x="' + X(Math.pow(10, e)) + '" y="' + (panels[1].y1 + 15) + '" text-anchor="middle">' + lab + '</text>';
    });
    s += '<text class="bd-t" x="' + (W - Rr) + '" y="' + (H - 4) + '" text-anchor="end">Hz</text></svg>';
    var dc = R.gain_db[0], f3 = 0, f0 = 0;
    for (i = 0; i < R.freq.length; i++) {
      if (!f3 && R.gain_db[i] <= dc - 3) f3 = R.freq[i];
      if (!f0 && R.gain_db[i] <= 0) f0 = R.freq[i];
    }
    el.innerHTML = '<p class="rv-sim-h"><svg class="ico"><use href="#i-wave"/></svg><span><en>AC response from</en> ' + R.simulator + ' ' + R.version.replace(/^(\d+\.\d+\.\d+).*/, '$1') + '</span></p>' + s +
      '<ul class="rv-sim-m"><li><en>Low-frequency gain</en> <b>' + dc.toFixed(1) + ' dB</b></li><li><en>−3 dB at</en> <b>' + fmtHz(f3) + '</b></li><li><en>Unity gain at</en> <b>' + fmtHz(f0) + '</b></li></ul>' +
      '<p class="rv-sim-n"><en>One volt of differential input and a 1 pF load, simulated from the netlist lines of the file; the RNL lines are comments to the simulator.</en> <code>' + R.analysis + '</code></p>';
  }

  function init() {
    var PG = global.RNLPage, LOOKS = global.RNLOtaLooks;
    var stage = document.getElementById('rv-stage'), code = document.getElementById('rv-code');
    if (!stage || !code || !C || !PG || !LOOKS) return;
    var list = document.getElementById('rv-items'), cap = document.getElementById('rv-cap');
    var openLink = document.getElementById('rv-open'), sim = document.getElementById('rv-sim'), figure = document.getElementById('rv-figure');
    var linker = null, style = 'en', look = 'standard', mode = 'viewer';
    var rows = [], cur = null, pinned = null, data = [], refs = null;

    /* the drawing: the core's SVG of the picked look, the notes laid over it */
    var svg = null;
    function draw() {
      stage.innerHTML = LOOKS.looks[look].svg;
      svg = stage.querySelector('svg');
      svg.setAttribute('role', 'img');
      svg.setAttribute('aria-label', look === 'eda' ? 'Five-transistor OTA in the traditional EDA look' : 'Five-transistor OTA');
      var g = '';
      Object.keys(NOTES).forEach(function (key) { g += '<g class="rv-note" data-note="' + key + '">' + NOTES[key] + '</g>'; });
      svg.insertAdjacentHTML('beforeend', g);
      lit(refs);
    }
    /* light up the parts of the devices named in refs, and the notes named ov:key; dim the rest of the drawing */
    function lit(list2) {
      refs = list2 && list2.length ? list2 : null;
      if (!svg) return;
      var any = false;
      Array.prototype.forEach.call(svg.querySelectorAll('.is-lit'), function (el) { el.classList.remove('is-lit'); });
      Array.prototype.forEach.call(svg.querySelectorAll('.rv-note.is-on'), function (el) { el.classList.remove('is-on'); });
      (refs || []).forEach(function (r) {
        if (r.indexOf('ov:') === 0) {
          var n = svg.querySelector('.rv-note[data-note="' + r.slice(3) + '"]');
          if (n) n.classList.add('is-on');
        } else {
          Array.prototype.forEach.call(svg.querySelectorAll('[data-dev="' + r + '"]'), function (el) { el.classList.add('is-lit'); any = true; });
        }
      });
      svg.classList.toggle('has-lit', any);
    }
    /* the code panel's link to the drawing is this small view */
    var view = { highlight: lit, setMode: function () {} };

    function renderCode() {
      data = lines(style, look);
      rows = C.render(code, data);
      if (!linker) linker = C.link(code, data, view, { modes: true });
      else linker.setLines(data);
      if (cur) mark(cur);
    }
    function indexOf(id) { for (var i = 0; i < data.length; i++) if (data[i].id === id) return i; return -1; }

    /* Show one item: its devices in the drawing, its statements and the devices' lines in the file */
    function mark(item, quiet) {
      var devs = item.devs.split(' ');
      lit(devs.concat(['ov:' + item.key]));
      rows.forEach(function (r) { r.classList.remove('is-hot', 'is-dev'); });
      item.stmt.forEach(function (id) { var i = indexOf(id); if (i >= 0) rows[i].classList.add('is-hot'); });
      data.forEach(function (l, i) { if (l.dev && devs.indexOf(l.dev) >= 0) rows[i].classList.add('is-dev'); });
      var marked = rows.filter(function (r) { return r.classList.contains('is-hot') || r.classList.contains('is-dev'); });
      if (!quiet && marked.length && code.scrollTo) {
        var top = Infinity, bottom = 0;
        marked.forEach(function (r) { top = Math.min(top, r.offsetTop); bottom = Math.max(bottom, r.offsetTop + r.offsetHeight); });
        if (bottom - top > code.clientHeight - 40) {
          var hot = rows.filter(function (r) { return r.classList.contains('is-hot'); });
          top = hot.length ? hot[0].offsetTop : top;
        }
        code.scrollTo({ top: Math.max(0, top - 28), behavior: REDUCE ? 'auto' : 'smooth' });
      }
      cap.innerHTML = CAPS[item.key];
      Array.prototype.forEach.call(list.children, function (li) { li.classList.toggle('is-on', li.getAttribute('data-key') === item.key); });
    }
    function clear() {
      cur = null;
      lit(null);
      rows.forEach(function (r) { r.classList.remove('is-hot', 'is-dev'); });
      cap.innerHTML = mode === 'sim' ? SIM : DEFAULT;
      Array.prototype.forEach.call(list.children, function (li) { li.classList.remove('is-on'); });
    }
    function show(item, quiet) { if (mode !== 'viewer') setView('viewer'); cur = item; mark(item, quiet); }
    function leave() { if (pinned) show(pinned, true); else clear(); }

    /* the checklist */
    list.innerHTML = ITEMS.map(function (it) {
      return '<li class="rv-item is-' + it.kind + '" data-key="' + it.key + '" tabindex="0" role="button" aria-pressed="false">' +
        '<span class="rv-k">' + KIND[it.kind] + '</span><span class="rv-t">' + it.text + '</span></li>';
    }).join('');
    function itemOf(el) {
      var li = el && el.closest ? el.closest('.rv-item') : null;
      if (!li) return null;
      var key = li.getAttribute('data-key');
      for (var i = 0; i < ITEMS.length; i++) if (ITEMS[i].key === key) return ITEMS[i];
      return null;
    }
    list.addEventListener('mouseover', function (e) { var it = itemOf(e.target); if (it) show(it); });
    list.addEventListener('focusin', function (e) { var it = itemOf(e.target); if (it) show(it); });
    list.addEventListener('mouseleave', leave);
    list.addEventListener('focusout', leave);
    list.addEventListener('click', function (e) {
      var it = itemOf(e.target);
      if (!it) return;
      pinned = pinned === it ? null : it;
      Array.prototype.forEach.call(list.children, function (li) { li.setAttribute('aria-pressed', pinned && li.getAttribute('data-key') === pinned.key ? 'true' : 'false'); });
      if (pinned) show(pinned); else clear();
    });
    list.addEventListener('keydown', function (e) {
      if (e.key !== 'Enter' && e.key !== ' ') return;
      var li = e.target.closest ? e.target.closest('.rv-item') : null;
      if (li) { e.preventDefault(); li.click(); }
    });

    /* a statement in the file picks its item too */
    code.addEventListener('mouseover', function (e) {
      if (mode !== 'viewer') return;
      var row = e.target.closest ? e.target.closest('.cl') : null;
      var l = row && data[+row.getAttribute('data-i')];
      if (!l) return;
      for (var i = 0; i < ITEMS.length; i++) {
        if (ITEMS[i].stmt.indexOf(l.id) >= 0) { show(ITEMS[i], true); return; }
      }
    });
    code.addEventListener('mouseleave', leave);

    /* a device in the drawing marks its lines in the file */
    stage.addEventListener('mouseover', function (e) {
      if (pinned || cur || !linker) return;
      var g = e.target.closest ? e.target.closest('[data-dev]') : null;
      if (g) linker.markRef(g.getAttribute('data-dev'));
    });
    stage.addEventListener('mouseleave', function () { if (!pinned && !cur && linker) linker.markRef(null); });

    function link() {
      var name = look === 'eda' ? FILES.eda : FILES[style];
      if (openLink) openLink.setAttribute('href', openLink.getAttribute('href').replace(/url=samples\/site\/[^&#]*/, 'url=samples/site/' + name));
    }
    var styleSeg = document.getElementById('rv-style'), lookSeg = document.getElementById('rv-look'), viewSeg = document.getElementById('rv-view');
    if (styleSeg) PG.initSeg(styleSeg, 'data-style', function (v) {
      style = v;
      link();
      var keep = code.scrollTop;
      renderCode();
      code.scrollTop = keep;
    });
    if (lookSeg) PG.initSeg(lookSeg, 'data-look', function (v) {
      look = v;
      link();
      draw();
      var keep = code.scrollTop;
      renderCode();
      code.scrollTop = keep;
    });
    function setView(v) {
      mode = v;
      figure.classList.toggle('is-sim', v === 'sim');
      code.classList.toggle('is-sim', v === 'sim');
      sim.hidden = v !== 'sim';
      stage.hidden = v === 'sim';
      if (viewSeg) Array.prototype.forEach.call(viewSeg.querySelectorAll('button'), function (b) { b.setAttribute('aria-pressed', b.getAttribute('data-view') === v ? 'true' : 'false'); });
      if (v === 'sim') { pinned = null; Array.prototype.forEach.call(list.children, function (li) { li.setAttribute('aria-pressed', 'false'); }); clear(); }
      else clear();
    }
    if (viewSeg) PG.initSeg(viewSeg, 'data-view', setView);

    drawSim(document.getElementById('rv-simplot'));
    draw();
    renderCode();
    clear();
  }

  global.RNLOta = { init: init, lines: lines, ITEMS: ITEMS, T: T, TZ: TZ, FILES: FILES };
})(window);
