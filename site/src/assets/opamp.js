/* RNL introduction page · the example: a two-stage Miller-compensated op-amp, reviewed by what it says
 *
 * The file is site/examples/opamp2.sp (and opamp2-zh.sp for the Chinese sentences); this script keeps its
 * lines and its coordinates, which site/build.py checks against those files. The drawing is the
 * page's own small renderer (schem.js) laid out with the coordinates of the file, so that it shows what
 * an RNL viewer draws from it; the coordinates are written by hand for this page, as in scenes.js, and
 * the RNL follows language standard 0.3 and the standard series 0.3, and reads with the rnl command.
 * The review: picking a statement of meaning, in the file or in the checklist, lights up the devices it is
 * about in the drawing and their lines in the netlist.
 */
(function (global) {
  'use strict';

  var S = global.RNLSchem, C = global.RNLCode;

  var BOX = [-3.5, -12.5, 64.5, 27.5];

  var SPEC = {
    M1: { sym: 'nmos', label: 'M1', value: 'W/L 40u/1u', valueOpt: true },
    M2: { sym: 'nmos', label: 'M2', value: 'W/L 40u/1u', valueOpt: true },
    M3: { sym: 'pmos', label: 'M3', value: 'W/L 20u/1u', valueOpt: true },
    M4: { sym: 'pmos', label: 'M4', value: 'W/L 20u/1u', valueOpt: true },
    M5: { sym: 'nmos', label: 'M5', value: 'W/L 20u/1u', valueOpt: true },
    M6: { sym: 'pmos', label: 'M6', value: 'W/L 200u/1u', valueOpt: true },
    M7: { sym: 'nmos', label: 'M7', value: 'W/L 100u/1u', valueOpt: true },
    MB: { sym: 'nmos', label: 'MB', value: 'W/L 10u/1u', valueOpt: true },
    RZ: { sym: 'res', label: 'RZ', value: '2k' },
    CC: { sym: 'cap', label: 'CC', value: '1p' },
    P_inp: { sym: 'port', label: 'inp', virtual: true },
    P_inn: { sym: 'port', label: 'inn', virtual: true },
    P_out: { sym: 'port', label: 'out', virtual: true },
    P_ib: { sym: 'port', label: 'ibias', virtual: true },
    VDD1: { sym: 'supply', label: 'vdd', virtual: true },
    VDD2: { sym: 'supply', label: 'vdd', virtual: true },
    VDD3: { sym: 'supply', label: 'vdd', virtual: true },
    G1: { sym: 'gnd', label: 'vss', virtual: true },
    G2: { sym: 'gnd', label: 'vss', virtual: true },
    G3: { sym: 'gnd', label: 'vss', virtual: true }
  };

  /* The positions of the file's drawing record (checked against it by site/build.py) */
  var PLACE = {
    M1: [12, 8, 'R0'], M2: [32, 8, 'MY'], M3: [12, 16, 'MY'], M4: [32, 16, 'R0'], M5: [22, 0, 'R0'], MB: [14, 0, 'MY'],
    M6: [48, 20, 'R0'], M7: [48, 8, 'R0'], RZ: [39, 12, 'R90'], CC: [45, 12, 'R90']
  };
  /* the display symbols: ports, supplies and grounds are not devices of the circuit */
  var VIRT = {
    P_inp: [5, 8, 'R0'], P_inn: [37, 8, 'MY'], P_out: [58, 12, 'MY'], P_ib: [5, 2, 'R0'],
    VDD1: [12, 20, 'R0'], VDD2: [32, 20, 'R0'], VDD3: [48, 24, 'R0'], G1: [22, -4, 'R0'], G2: [14, -4, 'R0'], G3: [48, 4, 'R0']
  };

  function devices() {
    var d = {}, id, t;
    for (id in PLACE) { t = PLACE[id]; d[id] = merge(SPEC[id], { x: t[0], y: t[1], o: t[2] }); }
    for (id in VIRT) { t = VIRT[id]; d[id] = merge(SPEC[id], { x: t[0], y: t[1], o: t[2] }); }
    return d;
  }
  function merge(a, b) {
    var o = {}, k;
    for (k in a) o[k] = a[k];
    for (k in b) o[k] = b[k];
    return o;
  }

  /* The wires of the file's drawing record, net by net */
  function route(P) {
    var wires = [], dots = [];
    function w(net, pts) { wires.push({ net: net, pts: pts }); }
    var ja = [16, 16], jb = [12, 12], jo = [32, 12], j6 = [36, 12], jt = [22, 4], jq = [48, 12], jv = [18, 0];
    w('x', [P('M3.d'), jb, P('M1.d')]);
    w('x', [P('M3.g'), ja, P('M4.g')]);
    w('x', [ja, [16, 12], jb]);
    w('n1', [P('M4.d'), jo, P('M2.d')]);
    w('n1', [jo, j6]);
    w('n1', [j6, P('RZ.p')]);
    w('n1', [j6, [36, 20], P('M6.g')]);
    w('nz', [P('RZ.n'), P('CC.p')]);
    w('tail', [P('M1.s'), [12, 4], jt]);
    w('tail', [P('M2.s'), [32, 4], jt]);
    w('tail', [jt, P('M5.d')]);
    w('out', [P('M6.d'), jq, P('M7.d')]);
    w('out', [P('CC.n'), jq]);
    w('out', [jq, P('P_out.p')]);
    w('ibias', [P('MB.d'), [17, 2], P('MB.g')]);
    w('ibias', [P('MB.g'), jv, P('M5.g')]);
    w('ibias', [jv, [18, -9], [45, -9], P('M7.g')]);
    w('ibias', [P('P_ib.p'), P('MB.d')]);
    w('inp', [P('M1.g'), P('P_inp.p')]);
    w('inn', [P('M2.g'), P('P_inn.p')]);
    w('vdd', [P('M3.s'), P('VDD1.p')]);
    w('vdd', [P('M4.s'), P('VDD2.p')]);
    w('vdd', [P('M6.s'), P('VDD3.p')]);
    w('vss', [P('M5.s'), P('G1.p')]);
    w('vss', [P('MB.s'), P('G2.p')]);
    w('vss', [P('M7.s'), P('G3.p')]);
    [ja, jb, jo, j6, jt, jq, jv].forEach(function (p, i) {
      dots.push([p[0], p[1], ['x', 'x', 'n1', 'n1', 'tail', 'out', 'ibias'][i]]);
    });
    return {
      wires: wires, dots: dots,
      labels: [
        { text: 'x', at: [11.4, 12.9], anchor: 'end', net: 'x', opt: true },
        { text: 'n1', at: [32.6, 13.0], anchor: 'start', net: 'n1', opt: true },
        { text: 'tail', at: [24, 4.85], anchor: 'start', net: 'tail', opt: true }
      ]
    };
  }

  /* What picking a statement shows besides the lit devices: a name next to them, the axis, the arrows */
  var NOTE = {
    dp: [{ id: 'n-dp', ref: 'dp', kind: 'text', at: [22, 6.9], text: '%dp1', anchor: 'middle', mono: true }],
    cm1: [{ id: 'n-cm1', ref: 'cm1', kind: 'text', at: [22, 17.7], text: '%cm1', anchor: 'middle', mono: true }],
    cm2: [{ id: 'n-cm2', ref: 'cm2', kind: 'text', at: [33, 0], text: '%cm2', anchor: 'middle', mono: true }],
    mc: [{ id: 'n-mc', ref: 'mc', kind: 'text', at: [42, 15.6], text: '%mc', anchor: 'middle', mono: true }],
    sym: [
      { id: 'ax', ref: 'sym', kind: 'vline', layer: 'back', x: 22, y0: -2.2, y1: 22.6, label: '%ax' }
    ],
    flow: [
      { id: 'n-flow', ref: 'flow', kind: 'arrow', pts: [[5, -11.2], [60, -11.2]], label: '%flow', labelAt: [32, -11.2], anchor: 'middle', mono: true, head: 0.5 }
    ],
    cur: [
      { id: 'c5', ref: 'cur', kind: 'arrow', pts: [[23.1, 4], [23.1, 2.4]], label: 'I', labelAt: [23.7, 3.2], anchor: 'start', mono: true, head: 0.4 },
      { id: 'c6', ref: 'cur', kind: 'arrow', pts: [[49.1, 17.8], [49.1, 16.2]], label: 'I', labelAt: [49.7, 17], anchor: 'start', mono: true, head: 0.4 },
      { id: 'c7', ref: 'cur', kind: 'arrow', pts: [[49.1, 10], [49.1, 8.4]], label: 'I', labelAt: [49.7, 9.2], anchor: 'start', mono: true, head: 0.4 }
    ]
  };

  function scene() {
    return {
      rk: 'op',
      devices: devices(),
      route: route,
      overlays: function (P, St, B, mode) { return mode && NOTE[mode] ? NOTE[mode] : []; }
    };
  }

  /* ---------- The file ---------- */

  /* Each statement of meaning in the three writing styles: the formal notation, the standard English sentences,
     the Chinese ones (the alias package rnl-zh; only the Chinese pages have them) */
  var T = {
    dp: ['%dp1 = #M1(in_p) #M2(in_n) #M5(tail) @AS $DIFFERENTIAL_PAIR;', '%dp1 = #M1(in_p), #M2(in_n) and #M5(tail) treated @AS $DIFFERENTIAL_PAIR;'],
    cm1: ['%cm1 = #M3(ref) #M4(out) @AS $CURRENT_MIRROR;', '%cm1 = #M3(ref) and #M4(out) treated @AS $CURRENT_MIRROR;'],
    cm2: ['%cm2 = #MB(ref) {#M5 #M7}(out) @AS $CURRENT_MIRROR;', '%cm2 = #MB(ref) and {#M5 #M7}(out) treated @AS $CURRENT_MIRROR;'],
    mc: ['%mc = {#M6}(stage) {#RZ #CC}(comp) @AS $MILLER_COMPENSATION;', '%mc = {#M6}(stage) and {#RZ #CC}(comp) treated @AS $MILLER_COMPENSATION;'],
    ax: ['%ax = (_, _);', '%ax = (_, _);'],
    s12: ['%sym12 = #M1(a) #M2(b) %ax(axis) @AS $MIRRORED;', '%sym12 = #M1(a), #M2(b) and %ax(axis) treated @AS $MIRRORED;'],
    s34: ['%sym34 = #M3(a) #M4(b) %ax(axis) @AS $MIRRORED;', '%sym34 = #M3(a), #M4(b) and %ax(axis) treated @AS $MIRRORED;'],
    fl: ['%flow = {#M1 #M2 #M6} @AS $FLOW;', '%flow = {#M1 #M2 #M6} treated @AS $FLOW;'],
    i5: ['#M5:d @CURRENT("in");', '#M5:d has @CURRENT("in");'],
    i7: ['#M7:d @CURRENT("in");', '#M7:d has @CURRENT("in");'],
    i6: ['#M6:d @CURRENT("out")', '#M6:d has @CURRENT("out")'],
    n1: ['&vdd @ADD-TO $SUPPLY_NET_SET;', '&vdd @ADD-TO $SUPPLY_NET_SET;'],
    n2: ['&vss @ADD-TO $GROUND_NET_SET;', '&vss @ADD-TO $GROUND_NET_SET;'],
    y1: ['{#M1 #M2 #M5 #M7 #MB} @SET-SYMBOL $NMOS;', '{#M1 #M2 #M5 #M7 #MB} @SET-SYMBOL to $NMOS;'],
    y2: ['{#M3 #M4 #M6} @SET-SYMBOL $PMOS;', '{#M3 #M4 #M6} @SET-SYMBOL to $PMOS;']
  };
  var TZ = {};
  /*zh-CN*/
  TZ = {
    dp: '%dp1 = #M1(正输入)、#M2(负输入) 和 #M5(尾管) @视作 $"差分对";',
    cm1: '%cm1 = #M3(参考管) 和 #M4(输出管) @视作 $"电流镜";',
    cm2: '%cm2 = #MB(参考管) 和 {#M5 #M7}(输出管) @视作 $"电流镜";',
    mc: '%mc = {#M6}(stage) 和 {#RZ #CC}(comp) @视作 $MILLER_COMPENSATION;',
    ax: '%ax = (_, _);',
    s12: '%sym12 = #M1(甲)、#M2(乙) 和 %ax(轴) @视作 $"镜像对称";',
    s34: '%sym34 = #M3(甲)、#M4(乙) 和 %ax(轴) @视作 $"镜像对称";',
    fl: '%flow = {#M1 #M2 #M6} @视作 $"信号流向";',
    i5: '#M5:d 有 @电流("流入");',
    i7: '#M7:d 有 @电流("流入");',
    i6: '#M6:d 有 @电流("流出")',
    n1: '&vdd @加入 $"电源网络集";',
    n2: '&vss @加入 $"地网络集";',
    y1: '{#M1 #M2 #M5 #M7 #MB} @使用符号 $NMOS;',
    y2: '{#M3 #M4 #M6} @使用符号 $PMOS;'
  };
  /*/zh-CN*/
  var STYLES = ['bare', 'en', 'zh'];

  /* The lines of the file as the panel shows them; r is what a line is about (devices, and the notes of a
     statement as ov:…), mode the note it makes in the drawing, dev a line of the netlist that is about one device */
  function lines(style) {
    var k = Math.max(0, STYLES.indexOf(style || 'en'));
    var L = [];
    function add(id, t, r, extra) {
      var o = { id: id, t: t, r: r || '' };
      if (extra) for (var key in extra) o[key] = extra[key];
      L.push(o);
    }
    function st(id, r, mode) { add(id, '*   ' + (k === 2 ? TZ[id] : T[id][k]), r, mode ? { mode: mode } : null); }
    function dev(id, t) { add(id.toLowerCase(), t, id, { dev: id }); }
    add('h0', '* Two-stage Miller-compensated op-amp with its bias');
    add('h1', '.model nch nmos level=1 vto=0.5');
    add('h2', '.model pch pmos level=1 vto=-0.5');
    add('h3', k === 2 ? '* @RNL(type="header") { @VERSION("0.3"); @USE-ALIASES(name="rnl-zh", version="0.3")}' : '* @RNL(type="header") { @VERSION("0.3")}');
    add('h4', '.subckt opamp2 inp inn out vdd vss ibias', 'P_inp P_inn P_out P_ib');
    add('c1', '* first stage: differential pair, tail current source, current-mirror load');
    dev('M1', 'M1 x    inp    tail vss nch W=40u  L=1u');
    dev('M2', 'M2 n1   inn    tail vss nch W=40u  L=1u');
    dev('M3', 'M3 x    x      vdd  vdd pch W=20u  L=1u');
    dev('M4', 'M4 n1   x      vdd  vdd pch W=20u  L=1u');
    dev('M5', 'M5 tail ibias  vss  vss nch W=20u  L=1u');
    add('c2', '* second stage: common-source amplifier with a current-source load');
    dev('M6', 'M6 out  n1     vdd  vdd pch W=200u L=1u');
    dev('M7', 'M7 out  ibias  vss  vss nch W=100u L=1u');
    add('c3', '* Miller compensation: a nulling resistor and a capacitor from n1 to out');
    dev('RZ', 'RZ n1 nz 2k');
    dev('CC', 'CC nz out 1p');
    add('c4', '* bias: the current into ibias is mirrored to the tail and to the second stage');
    dev('MB', 'MB ibias ibias vss vss nch W=10u L=1u');
    add('p1', '* PROVISIONAL: the standard packages have no definition of the Miller compensation yet. This local one', 'M6 RZ CC ov:mc');
    add('p2', '* says only what it is; it is replaced by the standard one when there is one.', 'M6 RZ CC ov:mc');
    add('p3', '* @RNL { $MILLER_COMPENSATION = @DEFINE(roles=["{stage}", "{comp}"]) {}}', 'M6 RZ CC ov:mc', { mode: 'mc' });
    add('r0', '* @RNL {');
    st('n1', 'VDD1 VDD2 VDD3');
    st('n2', 'G1 G2 G3');
    st('y1', 'M1 M2 M5 M7 MB');
    st('y2', 'M3 M4 M6');
    st('dp', 'M1 M2 M5 ov:dp', 'dp');
    st('cm1', 'M3 M4 ov:cm1', 'cm1');
    st('cm2', 'MB M5 M7 ov:cm2', 'cm2');
    st('mc', 'M6 RZ CC ov:mc', 'mc');
    st('ax', 'ov:sym');
    st('s12', 'M1 M2 ov:sym', 'sym');
    st('s34', 'M3 M4 ov:sym', 'sym');
    st('fl', 'M1 M2 M6 ov:flow', 'flow');
    st('i5', 'M5 ov:cur', 'cur');
    st('i7', 'M7 ov:cur', 'cur');
    st('i6', 'M6 ov:cur', 'cur');
    add('r1', '* }');
    add('d0', '* @RNL(author="agent") {');
    add('d1', '*   #M1 @PLACE(12, 8) => @ORIENT("R0");  #M2 @PLACE(32, 8) => @ORIENT("MY");', 'M1 M2');
    add('d2', '*   #M3 @PLACE(12, 16) => @ORIENT("MY");  #M4 @PLACE(32, 16) => @ORIENT("R0");', 'M3 M4');
    add('d3', '*   #M5 @PLACE(22, 0);  #MB @PLACE(14, 0) => @ORIENT("MY");', 'M5 MB');
    add('d4', '*   #M6 @PLACE(48, 20);  #M7 @PLACE(48, 8);', 'M6 M7');
    add('d5', '*   #RZ @PLACE(39, 12) => @ORIENT("R90");  #CC @PLACE(45, 12) => @ORIENT("R90");', 'RZ CC');
    add('x0', '*   /* <en>then the wires, junctions, ports, supplies and grounds: 38 more lines in the whole file</en> */');
    add('d9', '* }');
    add('e0', '.ends opamp2');
    return L;
  }

  /* ---------- The review ---------- */

  /* One item of the checklist for each statement of meaning: what the agent declared, in plain words */
  var ITEMS = [
    { key: 'dp', kind: 'structure', devs: 'M1 M2 M5', stmt: ['dp'], text: '<en><b>M1</b> and <b>M2</b> are the input differential pair; <b>M5</b> is its tail current source.</en>' },
    { key: 'cm1', kind: 'structure', devs: 'M3 M4', stmt: ['cm1'], text: '<en><b>M3</b> and <b>M4</b> are a current mirror: the load of the first stage.</en>' },
    { key: 'cm2', kind: 'structure', devs: 'MB M5 M7', stmt: ['cm2'], text: '<en><b>MB</b> sets the bias: its current is mirrored to the tail <b>M5</b> and to the second stage’s load <b>M7</b>.</en>' },
    { key: 'mc', kind: 'provisional', devs: 'M6 RZ CC', stmt: ['mc'], text: '<en>The second stage <b>M6</b> is compensated by <b>RZ</b> and <b>CC</b>: Miller compensation.</en>', note: '<en>provisional</en>' },
    { key: 'sym', kind: 'intent', devs: 'M1 M2 M3 M4', stmt: ['s12', 's34', 'ax'], text: '<en>The first stage is symmetric: <b>M1</b> and <b>M2</b>, and <b>M3</b> and <b>M4</b>, are mirror images about one axis.</en>' },
    { key: 'flow', kind: 'intent', devs: 'M1 M2 M6', stmt: ['fl'], text: '<en>The signal flows from left to right: the input pair, then the second stage.</en>' },
    { key: 'cur', kind: 'intent', devs: 'M5 M6 M7', stmt: ['i5', 'i6', 'i7'], text: '<en>Currents run from top to bottom: through <b>M5</b>, <b>M6</b> and <b>M7</b>.</en>' }
  ];

  var KIND = {
    structure: '<en>structure</en>',
    intent: '<en>intent</en>',
    provisional: '<en>structure</en>'
  };

  var DEFAULT = '<en>Pick a statement in the file or an item of the checklist: the devices it is about light up in the drawing and in the netlist. Nothing here needs the netlist to be read line by line.</en>';
  var CAPS = {
    dp: '<en><b>Structure</b>: <code>$DIFFERENTIAL_PAIR</code> says which devices form the pair and which is its tail. The standard package’s definition puts the tail under the middle of the pair.</en>',
    cm1: '<en><b>Structure</b>: <code>$CURRENT_MIRROR</code> says which device is the reference and which one copies it: here the load of the first stage.</en>',
    cm2: '<en><b>Structure</b>: one mirror, one reference and two outputs: <code>{#M5 #M7}</code> is a set of devices that copy the same bias current.</en>',
    mc: '<en><b>Provisional</b>: the standard packages do not yet have a definition for Miller compensation, so the file defines <code>$MILLER_COMPENSATION</code> itself and says so. It is replaced when the standard has one; the rest of the file does not change.</en>',
    sym: '<en><b>Intent</b>: two mirrored pairs about one axis. A tool that draws from it keeps the symmetry: the standard package’s definition puts each pair at one height, mirror images about the axis.</en>',
    flow: '<en><b>Intent</b>: the signal path runs left to right. The drawing follows it: input pair, then the second stage.</en>',
    cur: '<en><b>Intent</b>: where the current flows in these branches. The tail and the second stage run from top to bottom.</en>'
  };

  function init() {
    var PG = global.RNLPage;
    var svg = document.getElementById('rv-svg'), code = document.getElementById('rv-code');
    if (!svg || !code || !S || !C || !PG) return;
    var list = document.getElementById('rv-items'), cap = document.getElementById('rv-cap');
    var openLink = document.getElementById('rv-open');
    var linker = null, style = 'en', rows = [], cur = null, pinned = null, data = [];
    var view = new S.View(svg, { box: BOX, onHover: function (ref) { if (!pinned && !cur && linker) linker.markRef(ref); } });
    view.setScene(scene());

    function renderCode() {
      data = lines(style);
      rows = C.render(code, data);
      if (!linker) linker = C.link(code, data, view, { modes: true });
      else linker.setLines(data);
      if (cur) mark(cur);
    }

    function indexOf(id) { for (var i = 0; i < data.length; i++) if (data[i].id === id) return i; return -1; }

    /* Show one item: its devices in the drawing, its statements and the devices' lines in the file */
    function mark(item, quiet) {
      var devs = item.devs.split(' ');
      view.setMode(item.key);
      view.highlight(devs.concat(['ov:' + item.key]));
      rows.forEach(function (r) { r.classList.remove('is-hot', 'is-dev'); });
      item.stmt.forEach(function (id) { var i = indexOf(id); if (i >= 0) rows[i].classList.add('is-hot'); });
      data.forEach(function (l, i) { if (l.dev && devs.indexOf(l.dev) >= 0) rows[i].classList.add('is-dev'); });
      /* scroll so that the statements and the devices' lines are in view, the statements first when they are too far apart */
      var marked = rows.filter(function (r) { return r.classList.contains('is-hot') || r.classList.contains('is-dev'); });
      if (!quiet && marked.length && code.scrollTo) {
        var top = Infinity, bottom = 0;
        marked.forEach(function (r) { top = Math.min(top, r.offsetTop); bottom = Math.max(bottom, r.offsetTop + r.offsetHeight); });
        if (bottom - top > code.clientHeight - 40) {
          var hot = rows.filter(function (r) { return r.classList.contains('is-hot'); });
          top = hot.length ? hot[0].offsetTop : top;
        }
        code.scrollTo({ top: Math.max(0, top - 28), behavior: S.REDUCE ? 'auto' : 'smooth' });
      }
      cap.innerHTML = CAPS[item.key];
      Array.prototype.forEach.call(list.children, function (li) { li.classList.toggle('is-on', li.getAttribute('data-key') === item.key); });
    }
    function clear() {
      cur = null;
      view.setMode(null);
      view.highlight(null);
      rows.forEach(function (r) { r.classList.remove('is-hot', 'is-dev'); });
      cap.innerHTML = DEFAULT;
      Array.prototype.forEach.call(list.children, function (li) { li.classList.remove('is-on'); });
    }
    function show(item, quiet) { cur = item; mark(item, quiet); }
    function leave() { if (pinned) show(pinned, true); else clear(); }

    /* the checklist */
    list.innerHTML = ITEMS.map(function (it) {
      return '<li class="rv-item is-' + it.kind + '" data-key="' + it.key + '" tabindex="0" role="button" aria-pressed="false">' +
        '<span class="rv-k">' + (it.kind === 'provisional' ? KIND.provisional : KIND[it.kind]) + '</span>' +
        '<span class="rv-t">' + it.text + (it.note ? ' <span class="chip chip-warn">' + it.note + '</span>' : '') + '</span></li>';
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
      var row = e.target.closest ? e.target.closest('.cl') : null;
      var l = row && data[+row.getAttribute('data-i')];
      if (!l) return;
      for (var i = 0; i < ITEMS.length; i++) {
        if (ITEMS[i].stmt.indexOf(l.id) >= 0 || (ITEMS[i].key === 'mc' && /^p[123]$/.test(l.id))) { show(ITEMS[i], true); return; }
      }
    });
    code.addEventListener('mouseleave', leave);

    if (openLink) {
      PG.initSeg(document.getElementById('rv-style'), 'data-style', function (v) {
        style = v;
        openLink.setAttribute('href', openLink.getAttribute('href').replace(/url=samples\/site\/[^&#]*/, 'url=samples/site/' + (v === 'zh' ? 'opamp2-zh.sp' : 'opamp2.sp')));
        var keep = code.scrollTop;
        renderCode();
        code.scrollTop = keep;
      });
    }
    renderCode();
    clear();
  }

  global.RNLOpamp = { init: init, lines: lines, scene: scene, BOX: BOX, PLACE: PLACE, ITEMS: ITEMS, T: T, TZ: TZ };
})(window);
