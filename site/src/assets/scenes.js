/* RNL introduction page · scenes and example data
 *
 * The circuits come from examples used in the RNL language discussion (5T OTA, RC/RLC, the fill
 * example). Coordinates are RNL coordinates (x right, y up) and match the solver and editor records
 * shown in the files; they are placed by hand for this page, not computed by a solver. The RNL follows
 * the language draft 0.2 (2026-10-12) for illustration.
 */
(function (global) {
  'use strict';

  function merge(a, b) {
    var o = {}, k;
    for (k in a) o[k] = a[k];
    for (k in b) o[k] = b[k];
    return o;
  }

  /* ================= Five-transistor OTA ================= */

  var OTA_BOX = [-4.5, -6.5, 43, 23.5];

  var OTA_SPEC = {
    M1: { sym: 'nmos', label: 'M1', value: 'W/L 2u/0.5u', valueOpt: true },
    M2: { sym: 'nmos', label: 'M2', value: 'W/L 2u/0.5u', valueOpt: true },
    M3: { sym: 'pmos', label: 'M3', value: 'W/L 4u/0.5u', valueOpt: true },
    M4: { sym: 'pmos', label: 'M4', value: 'W/L 4u/0.5u', valueOpt: true },
    M5: { sym: 'nmos', label: 'M5', value: 'W/L 4u/1u', valueOpt: true },
    P_inp: { sym: 'port', label: 'inp', virtual: true },
    P_inn: { sym: 'port', label: 'inn', virtual: true },
    P_out: { sym: 'port', label: 'out', virtual: true },
    P_vb: { sym: 'port', label: 'vbias', virtual: true },
    P_vdd: { sym: 'port', label: 'vdd', virtual: true },
    P_vss: { sym: 'port', label: 'vss', virtual: true },
    VDD1: { sym: 'supply', label: 'vdd', virtual: true },
    VDD2: { sym: 'supply', label: 'vdd', virtual: true },
    GND: { sym: 'gnd', virtual: true }
  };

  /* four: the standard package's default four-terminal MOS symbols, before the file picks its own */
  function layout(tbl, four) {
    var devices = {};
    for (var id in tbl) {
      var t = tbl[id];
      var spec = OTA_SPEC[id];
      if (four && /^M/.test(id)) spec = merge(spec, { sym: spec.sym + '4' });
      devices[id] = merge(spec, { x: t[0], y: t[1], o: t[2] });
    }
    return devices;
  }

  var L_AIR = {
    M3: [9, 10, 'R0'], M4: [16, 10, 'R0'], M1: [25, 10, 'R0'], M2: [32, 10, 'R0'], M5: [39, 10, 'R0'],
    P_inp: [3, 22, 'R0'], P_inn: [3, 18, 'R0'], P_out: [3, 14, 'R0'],
    P_vdd: [3, 6, 'R0'], P_vss: [3, 2, 'R0'], P_vb: [3, -2, 'R0']
  };
  var L_SYM = {
    M1: [12, 8, 'R0'], M2: [28, 8, 'MY'], M3: [12, 16, 'MY'], M4: [28, 16, 'R0'], M5: [20, 0, 'R0'],
    P_inp: [5, 8, 'R0'], P_inn: [35, 8, 'MY'], P_out: [35, 12, 'MY'], P_vb: [13, 0, 'R0'],
    VDD1: [12, 20, 'R0'], VDD2: [28, 20, 'R0'], GND: [20, -4, 'R0']
  };
  var L_TWEAK = merge(L_SYM, { M2: [30, 8, 'MY'] });
  var L_RESOLVED = {
    M1: [10, 8, 'R0'], M2: [30, 8, 'MY'], M3: [10, 16, 'MY'], M4: [30, 16, 'R0'], M5: [20, 0, 'R0'],
    P_inp: [3, 8, 'R0'], P_inn: [37, 8, 'MY'], P_out: [37, 12, 'MY'], P_vb: [13, 0, 'R0'],
    VDD1: [10, 20, 'R0'], VDD2: [30, 20, 'R0'], GND: [20, -4, 'R0']
  };

  /* Connectivity only: each net draws straight lines (airwires) from its first terminal to the others */
  var AIR_NETS = [
    ['inp', ['P_inp.p', 'M1.g']], ['inn', ['P_inn.p', 'M2.g']], ['out', ['P_out.p', 'M2.d', 'M4.d']],
    ['vdd', ['P_vdd.p', 'M3.s', 'M3.b', 'M4.s', 'M4.b']], ['vss', ['P_vss.p', 'M1.b', 'M2.b', 'M5.s', 'M5.b']],
    ['vbias', ['P_vb.p', 'M5.g']], ['x', ['M1.d', 'M3.d', 'M3.g', 'M4.g']], ['tail', ['M1.s', 'M2.s', 'M5.d']]
  ];

  function routeAir(P) {
    var wires = [];
    AIR_NETS.forEach(function (n) {
      var a = P(n[1][0]);
      for (var i = 1; i < n[1].length; i++) wires.push({ net: n[0], pts: [a, P(n[1][i])], air: true });
    });
    return { wires: wires, dots: [] };
  }

  function routeSym(P) {
    var wires = [], dots = [];
    var M3d = P('M3.d'), M1d = P('M1.d'), M3g = P('M3.g'), M4g = P('M4.g');
    var ja = [M3g[0] + (M4g[0] > M3g[0] ? 1 : -1), M3g[1]], jb = [M3d[0], M3d[1] - 2];
    wires.push({ net: 'x', pts: [M3d, M1d] });
    wires.push({ net: 'x', pts: [M3g, M4g] });
    wires.push({ net: 'x', pts: [ja, [ja[0], jb[1]], jb] });
    dots.push([ja[0], ja[1], 'x'], [jb[0], jb[1], 'x']);
    var M4d = P('M4.d'), M2d = P('M2.d'), yo = M4d[1] - 2, jo = [M4d[0], yo];
    wires.push({ net: 'out', pts: [M4d, jo] });
    wires.push({ net: 'out', pts: M2d[0] === jo[0] ? [jo, M2d] : [jo, [M2d[0], yo], M2d] });
    wires.push({ net: 'out', pts: [jo, P('P_out.p')] });
    dots.push([jo[0], jo[1], 'out']);
    var a = P('M1.s'), b = P('M2.s'), m = P('M5.d'), yt = a[1] - 2;
    wires.push({ net: 'tail', pts: [a, [a[0], yt], [b[0], yt], b] });
    wires.push({ net: 'tail', pts: [[m[0], yt], m] });
    dots.push([m[0], yt, 'tail']);
    wires.push({ net: 'inp', pts: [P('M1.g'), P('P_inp.p')] });
    wires.push({ net: 'inn', pts: [P('M2.g'), P('P_inn.p')] });
    wires.push({ net: 'vbias', pts: [P('M5.g'), P('P_vb.p')] });
    wires.push({ net: 'vdd', pts: [P('M3.s'), P('VDD1.p')] });
    wires.push({ net: 'vdd', pts: [P('M4.s'), P('VDD2.p')] });
    wires.push({ net: 'vss', pts: [P('M5.s'), P('GND.p')] });
    return {
      wires: wires, dots: dots,
      labels: [
        { text: 'x', at: [M1d[0] - 0.6, jb[1] + 0.9], anchor: 'end', net: 'x', opt: true },
        { text: 'tail', at: [(a[0] + m[0]) / 2, yt + 0.85], anchor: 'middle', net: 'tail', opt: true },
        { text: 'out', at: [jo[0] + 3.4, yo + 0.85], anchor: 'middle', net: 'out', opt: true }
      ]
    };
  }

  function r1(v) { return Math.round(v * 10) / 10; }

  /* Where each coordinate tag sits, relative to the device's origin */
  var TAG = { M1: [1, -0.5, 'start'], M2: [-1, -0.5, 'end'], M3: [-1, -0.5, 'end'], M4: [1, -0.5, 'start'], M5: [1, -0.6, 'start'] };

  function boxes() {
    return [
      { id: 'cm1', kind: 'box', layer: 'back', ids: ['M3', 'M4'], pad: 1, label: '<zh>%cm1 · 电流镜</zh><en>%cm1 · current mirror</en>', labelAt: [0.5, 1], anchor: 'middle' },
      { id: 'dp1', kind: 'box', layer: 'back', ids: ['M1', 'M2'], pad: 1, label: '<zh>%dp1 · 差分对</zh><en>%dp1 · diff pair</en>', labelAt: [0.5, 1], anchor: 'middle' },
      { id: 'stage1', kind: 'box', layer: 'back', ids: ['M1', 'M2', 'M5'], pad: 2.4, label: '<zh>%st1 · 输入级</zh><en>%st1 · input stage</en>', labelAt: [0.03, 0], anchor: 'start' }
    ];
  }

  function intent(P, S, axisTone, axisX) {
    var L = [];
    var ax = axisX !== undefined ? axisX : (S('M1').x + S('M2').x) / 2;
    L.push({ id: 'ax', kind: 'vline', layer: 'back', x: ax, y0: -1.2, y1: 21.6, label: '%ax', tone: axisTone });
    var a = P('M1.s'), b = P('M2.s'), m = P('M5.d'), yt = a[1] - 2;
    var l1 = r1(m[0] - a[0]), l2 = r1(b[0] - m[0]);
    var eq = Math.abs(l1 - l2) < 0.05;
    L.push({
      id: 'c1', kind: 'ticks', y: yt, segs: [[a[0], m[0]], [m[0], b[0]]], labels: [String(l1), String(l2)],
      tone: eq ? undefined : 'err', mid: eq ? null : [m[0], yt + 1.25], midText: '≠'
    });
    [['M5', '1'], ['M1', '2'], ['M3', '3']].forEach(function (p) {
      var s = S(p[0]);
      L.push({ id: 'ord' + p[1], ref: 'ord', kind: 'badge', at: [s.x - 4.3, s.y + 1.6], text: p[1] });
    });
    var d5 = P('M5.d');
    L.push({ id: 'cur', kind: 'arrow', pts: [[d5[0] + 1.1, d5[1] + 1.7], [d5[0] + 1.1, d5[1] + 0.3]], label: 'I', labelAt: [d5[0] + 1.6, d5[1] + 1], anchor: 'start', mono: true, head: 0.4 });
    return L;
  }

  function tags(S, userId) {
    return ['M1', 'M2', 'M3', 'M4', 'M5'].map(function (id) {
      var st = S(id), o = TAG[id];
      return {
        id: 'tag-' + id, ref: id, kind: 'tag', at: [st.x + o[0], st.y + o[1]], anchor: o[2],
        text: '(' + Math.round(st.x) + ', ' + Math.round(st.y) + ')', tone: id === userId ? 'user' : undefined
      };
    });
  }

  function otaScene(stage) {
    var tbl = [L_AIR, L_AIR, L_SYM, L_SYM, L_TWEAK, L_RESOLVED][stage];
    return {
      rk: stage <= 1 ? 'air' : 'sym',
      devices: layout(tbl, stage <= 1),
      route: stage <= 1 ? routeAir : routeSym,
      overlays: function (P, S) {
        var L = [];
        if (stage === 1) L = L.concat(boxes());
        if (stage >= 2) L = L.concat(intent(P, S, stage === 4 ? 'mute' : undefined, stage === 4 ? (S('M3').x + S('M4').x) / 2 : undefined));
        if (stage >= 3) L = L.concat(tags(S, stage === 4 ? 'M2' : null));
        return L;
      }
    };
  }

  /* Step-by-step refinement: the full file at each step, in one of three writing styles.
     bare: the formal notation only; en: the standard sentences. */
  var U = {
    u1: ['%dp1 = #M1(in_p) #M2(in_n) @AS $DIFFPAIR;', '%dp1 = #M1(in_p) and #M2(in_n) treated @AS $DIFFPAIR;'],
    u2: ['%cm1 = #M3(ref) #M4(out) @AS $CURRENT_MIRROR;', '%cm1 = #M3(ref) and #M4(out) treated @AS $CURRENT_MIRROR;'],
    u3: ['%st1 = %dp1(pair) #M5(tail) @AS $INPUT_STAGE', '%st1 = %dp1(pair) and #M5(tail) treated @AS $INPUT_STAGE'],
    u4: ['{#M1 #M2 #M5} @SET-SYMBOL $NMOS;', '{#M1 #M2 #M5} @SET-SYMBOL to $NMOS;'],
    u5: ['{#M3 #M4} @SET-SYMBOL $PMOS;', '{#M3 #M4} @SET-SYMBOL to $PMOS;'],
    u6: ['&vdd @ADD-TO $SUPPLY_NET_SET;', '&vdd @ADD-TO $SUPPLY_NET_SET;'],
    u7: ['&vss @ADD-TO $GROUND_NET_SET;', '&vss @ADD-TO $GROUND_NET_SET;'],
    u8: ['{&vdd &vss} @REMOVE-FROM $PORT_FILL_SET;', '{&vdd &vss} @REMOVE-FROM $PORT_FILL_SET;'],
    u9: ['%ax = (_, _);', '%ax = (_, _);'],
    u10: ['%sym12 = #M1 #M2 %ax @AS $MIRROR_PAIR;', '%sym12 = #M1(a), #M2(b) and %ax(axis) treated @AS $MIRROR_PAIR;'],
    u11: ['%sym34 = #M3 #M4 %ax @AS $MIRROR_PAIR;', '%sym34 = #M3(a), #M4(b) and %ax(axis) treated @AS $MIRROR_PAIR;'],
    u12: ['%c1 = #M1 #M2 #M5 @AS $TAIL_CENTER;', '%c1 = #M1(a), #M2(b) and #M5(t) treated @AS $TAIL_CENTER;'],
    u13: ['{#M5 #M1 #M3} @ORDER("bottom-to-top");', '{#M5 #M1 #M3} are in @ORDER("bottom-to-top");'],
    u14: ['#M5:d @CURRENT("in")', '#M5:d has @CURRENT("in")']
  };
  /* zh: the Chinese standard sentences, written with the alias package rnl-zh; shown on every page */
  /*zh-CN*/
  var UZ = {
    u1: '%dp1 = #M1(正输入) 和 #M2(负输入) @视作 $"差分对";',
    u2: '%cm1 = #M3(参考管) 和 #M4(输出管) @视作 $"电流镜";',
    u3: '%st1 = %dp1(输入对) 和 #M5(尾管) @视作 $"输入级"',
    u4: '{#M1 #M2 #M5} @使用符号 $NMOS;',
    u5: '{#M3 #M4} @使用符号 $PMOS;',
    u6: '&vdd @加入 $"电源网络集";',
    u7: '&vss @加入 $"地网络集";',
    u8: '{&vdd &vss} @移出 $"端口补位集";',
    u9: '%ax = (_, _);',
    u10: '%sym12 = #M1(甲)、#M2(乙) 和 %ax(轴) @视作 $"镜像对";',
    u11: '%sym34 = #M3(甲)、#M4(乙) 和 %ax(轴) @视作 $"镜像对";',
    u12: '%c1 = #M1(甲)、#M2(乙) 和 #M5(尾端) @视作 $"尾管居中";',
    u13: '{#M5 #M1 #M3} 按 @顺序("从下到上") 排列;',
    u14: '#M5:d 有 @电流("流入")'
  };
  /*/zh-CN*/
  var STYLES = ['bare', 'en', 'zh'];

  function otaCode(stage, style) {
    var k = Math.max(0, STYLES.indexOf(style || 'en'));
    var L = [];
    function add(id, t, r, extra) {
      var o = { id: id, t: t, r: r || '' };
      if (extra) for (var key in extra) o[key] = extra[key];
      L.push(o);
    }
    function u(id, r) {
      var t = k === 2 ? UZ[id] : U[id][k];
      if (id === 'u3' && stage >= 2) t += ';';
      add(id, '*   ' + t, r);
    }
    add('h0', '* 5T OTA');
    add('h1', '.model nch nmos level=1 vto=0.5');
    add('h2', '.model pch pmos level=1 vto=-0.5');
    if (k === 2 && stage >= 1) add('hz', '* @RNL(type="header") { @VERSION("0.2"); @USE-ALIASES(name="rnl-zh")}');
    add('h3', '.subckt ota5t inp inn out vdd vss vbias', stage <= 1 ? 'P_inp P_inn P_out P_vb P_vdd P_vss' : 'P_inp P_inn P_out P_vb');
    add('m1', 'M1 x    inp   tail vss nch W=2u L=0.5u', 'M1');
    add('m2', 'M2 out  inn   tail vss nch W=2u L=0.5u', 'M2');
    add('m3', 'M3 x    x     vdd  vdd pch W=4u L=0.5u', 'M3');
    add('m4', 'M4 out  x     vdd  vdd pch W=4u L=0.5u', 'M4');
    add('m5', 'M5 tail vbias vss  vss nch W=4u L=1u', 'M5');
    if (stage >= 1) {
      add('u0', '* @RNL {');
      u('u1', 'M1 M2 ov:dp1');
      u('u2', 'M3 M4 ov:cm1');
      u('u3', 'M1 M2 M5 ov:stage1');
      if (stage >= 2) {
        u('u4', 'M1 M2 M5');
        u('u5', 'M3 M4');
        u('u6', 'VDD1 VDD2');
        u('u7', 'GND');
        u('u8', 'VDD1 VDD2 GND');
        u('u9', 'ov:ax');
        u('u10', 'M1 M2 ov:ax');
        u('u11', 'M3 M4 ov:ax');
        u('u12', 'M1 M2 M5 ov:c1');
        u('u13', 'M5 M1 M3 ov:ord');
        u('u14', 'M5 ov:cur');
      }
      add('u15', '* }');
    }
    if (stage >= 3) {
      var s5 = stage >= 5;
      var x1 = s5 ? 10 : 12, x2 = s5 ? 30 : 28;
      add('s0', '* @RNL(type="generated", priority="high", author="rnl-solver", hash="sig1:' + (s5 ? '9e27' : '4c1f') + '") {');
      add('s1', '*   #M1 @PLACE(' + x1 + ', 8) => @ORIENT("R0");  #M2 @PLACE(' + x2 + ', 8) => @ORIENT("MY");', 'M1 M2');
      add('s2', '*   #M3 @PLACE(' + x1 + ', 16) => @ORIENT("MY");  #M4 @PLACE(' + x2 + ', 16) => @ORIENT("R0");', 'M3 M4');
      add('s3', '*   #M5 @PLACE(20, 0);  %ja = (' + (x1 + 4) + ', 16);  %jb = (' + x1 + ', 12);', 'M5');
      add('s4', '*   &x @WIRE {#M3:d %jb #M1:d};  &x @WIRE {#M3:g %ja #M4:g};', 'net:x');
      add('s5', '*   /* <zh>另有导线、交汇点、端口、电源和地，这里从略</zh><en>more wires, junctions, ports, supplies and the ground follow; left out here</en> */');
      add('s6', '* }');
    }
    if (stage >= 4) add('e0', '* @RNL(author="rnl-editor") { #M2 @PLACE(x=30, priority="high")}', 'M2');
    add('end', '.ends ota5t');
    return L;
  }

  /* "Who owns what" in section 01: one drawing, different annotations per mode */
  function ownerScene() {
    return {
      rk: 'sym',
      devices: layout(L_SYM),
      route: routeSym,
      overlays: function (P, S, B, mode) {
        if (mode === 'semantic') return boxes();
        if (mode === 'intent') return intent(P, S);
        if (mode === 'geometry') return tags(S, null);
        return [];
      }
    };
  }

  /* The edits-and-errors example: M2 moved right in the editor; the symmetry axis is a free point */
  function e5Scene() {
    return {
      rk: 'sym',
      devices: layout(L_TWEAK),
      route: routeSym,
      overlays: function (P, S) {
        var L = intent(P, S, 'ok').filter(function (o) { return o.id === 'ax' || o.id === 'c1'; });
        L.push({ id: 'tag-M2', ref: 'M2', kind: 'tag', at: [S('M2').x - 1, 7.5], anchor: 'end', text: '(30, 8)', tone: 'user' });
        L.push({ id: 'tag-M4', ref: 'M4', kind: 'tag', at: [29, 15.5], anchor: 'start', text: '(28, 16)', tone: 'mute' });
        return L;
      }
    };
  }

  var E5_LINES = [
    { t: '.subckt ota5t inp inn out vdd vss vbias' },
    { t: 'M1 x    inp   tail vss nch W=2u L=0.5u', r: 'M1' },
    { t: 'M2 out  inn   tail vss nch W=2u L=0.5u', r: 'M2' },
    { t: 'M3 x    x     vdd  vdd pch W=4u L=0.5u', r: 'M3' },
    { t: 'M4 out  x     vdd  vdd pch W=4u L=0.5u', r: 'M4' },
    { t: 'M5 tail vbias vss  vss nch W=4u L=1u', r: 'M5' },
    { t: '* @RNL {' },
    { t: '*   %dp1 = #M1(in_p) #M2(in_n) @AS $DIFFPAIR;', r: 'M1 M2', d: 'ok' },
    { t: '*   %ax = (_, _);', r: 'ov:ax', d: 'ok' },
    { t: '*   %sym12 = #M1 #M2 %ax @AS $MIRROR_PAIR;', r: 'M1 M2 ov:ax', d: 'ok' },
    { t: '*   %c1 = #M1 #M2 #M5 @AS $TAIL_CENTER;', r: 'M1 M2 M5 ov:c1', d: 'conflict' },
    { t: '*   {#M1 #M2 #M9} @ALIGN("y")', cls: 'is-err', d: 'm9' },
    { t: '* }' },
    { t: '* @RNL(type="generated", priority="high", author="rnl-solver", hash="sig1:4c1f") {', d: 'fp' },
    { t: '*   #M1 @PLACE(12, 8) => @ORIENT("R0");  #M2 @PLACE(28, 8) => @ORIENT("MY");', r: 'M1 M2' },
    { t: '*   #M3 @PLACE(12, 16) => @ORIENT("MY");  #M4 @PLACE(28, 16) => @ORIENT("R0");', r: 'M3 M4', d: 'bad' },
    { t: '*   #M5 @PLACE(20, 0)', r: 'M5' },
    { t: '* }' },
    { t: '* @RNL(author="rnl-editor") {', d: 'fp' },
    { t: '*   #M2 @PLACE(x=30, priority="high");', r: 'M2', d: 'conflict' },
    { t: '*   #M4 @PLACE(2O, 16, priority="high")', r: 'M4', cls: 'is-err', d: 'bad' },
    { t: '* }' },
    { t: '.ends ota5t' }
  ];

  var E5_DIAG = [
    { k: 'err', id: 'm9', icon: 'cross', html: '<zh><code>{#M1 #M2 #M9} @ALIGN("y")</code>：本层没有元件 M9。ALIGN 对成员是严格的，整句失效，不会退化成“只对齐 M1、M2”。</zh><en><code>{#M1 #M2 #M9} @ALIGN("y")</code>: this level has no instance M9. ALIGN is strict about its members, so the whole statement fails; it does not fall back to “align only M1 and M2”.</en>', refs: [] },
    { k: 'err', id: 'bad', icon: 'cross', html: '<zh><code>#M4 @PLACE(2O, 16, …)</code>：数后面不能带字母，“2O” 里是字母 O。只有这一句失效，M4 保留求解器给的 (28, 16)。</zh><en><code>#M4 @PLACE(2O, 16, …)</code>: a number cannot be followed by a letter, and “2O” contains the letter O. Only this statement fails; M4 keeps the solver’s (28, 16).</en>', refs: ['M4'] },
    { k: 'warn', id: 'conflict', icon: 'warn', html: '<zh>编辑器的 <code>#M2 @PLACE(x=30, …)</code> 是 high，后写，盖过求解器给的 x = 28。两个源极的中点因此到了 x = 21，而 M5 留在 20：<code>%c1</code> 展开的等距约束是 normal，与这些 high 坐标冲突，被丢弃，尾管不再居中。</zh><en>The editor’s <code>#M2 @PLACE(x=30, …)</code> is high and written later, so it overrides the solver’s x = 28. The midpoint of the two sources moves to x = 21 while M5 stays at 20: the equal-spacing constraint expanded from <code>%c1</code> is normal, conflicts with these high coordinates and is dropped, so the tail is no longer centered.</en>', refs: ['M2', 'M5', 'ov:c1'] },
    { k: 'info', id: 'fp', icon: 'info', html: '<zh>求解记录的 <code>hash</code> 与当前输入不符：求解之后，编辑器加了一条记录。查看器建议重新求解，旧结果照常显示。</zh><en>The solver record’s <code>hash</code> no longer matches the current input: the editor added a record after solving. The viewer suggests solving again; the old result still displays.</en>', refs: [] },
    { k: 'ok', id: 'ok', icon: 'check', html: '<zh>其余照常生效：差分对的语义；对称（对称轴是自由点，跟着移到 x = 21）；其他坐标。</zh><en>Everything else takes effect: the differential pair; the symmetry (the axis is a free point and moves to x = 21); the other coordinates.</en>', refs: ['M1', 'M2', 'ov:ax'] }
  ];

  /* ================= Hero: RC low-pass ================= */

  var RC = {
    rk: 'rc',
    box: [0.5, -1.6, 23.5, 16.2],
    devices: {
      V1: { sym: 'vac', x: 4, y: 6, label: 'V1', value: 'AC 1' },
      R1: { sym: 'res', x: 11, y: 12, o: 'R90', label: 'R1', value: '10k', lab: [-0.25, 1.35, 'end'], val: [0.25, 1.35, 'start'] },
      C1: { sym: 'cap', x: 18, y: 6, label: 'C1', value: '10n' },
      G1: { sym: 'gnd', x: 4, y: 1, virtual: true },
      G2: { sym: 'gnd', x: 18, y: 2, virtual: true }
    },
    route: function (P) {
      var v1p = P('V1.p'), r1p = P('R1.p'), r1n = P('R1.n'), c1p = P('C1.p');
      return {
        wires: [
          { net: 'in', pts: [v1p, [v1p[0], r1p[1]], r1p] },
          { net: 'out', pts: [r1n, [c1p[0], r1n[1]], c1p] },
          { net: '0', pts: [P('V1.n'), P('G1.p')] },
          { net: '0', pts: [P('C1.n'), P('G2.p')] }
        ],
        dots: [],
        labels: [{ text: 'V', sub: 'out', at: [16, 12.75], net: 'out' }]
      };
    },
    overlays: function (P, S, B, mode) {
      if (mode !== 'flow') return [];
      return [{ id: 'flow', kind: 'arrow', pts: [[2.6, 15], [20, 15]], label: '$FLOW · left-to-right', labelAt: [11.3, 15], anchor: 'middle', mono: true }];
    }
  };

  var HERO_LINES = [
    { t: '* RC low-pass filter', sim: 'title', view: 'skip', note: '<zh>第一行是标题。SPICE 把文件第一行当作标题，不属于电路。</zh><en>The first line is the title. SPICE treats a file’s first line as its title, not as part of the circuit.</en>' },
    { t: 'V1 in 0 AC 1', r: 'V1 G1', sim: 'use', view: 'draw', note: '<zh><b>网表</b>：信号源 V1 接在 in 与地之间。接地的那一端不用写 RNL：标准包会自动补一个地符号。</zh><en><b>Netlist</b>: source V1 between in and ground. The grounded end needs no RNL: the standard package adds a ground symbol by itself.</en>' },
    { t: 'R1 in out 10k', r: 'R1', sim: 'use', view: 'draw', note: '<zh><b>网表</b>：10 kΩ 电阻，从 in 接到 out。</zh><en><b>Netlist</b>: a 10 kΩ resistor from in to out.</en>' },
    { t: 'C1 out 0 10n', r: 'C1 G2', sim: 'use', view: 'draw', note: '<zh><b>网表</b>：10 nF 电容，从 out 接到地；地符号同样由标准包补上。</zh><en><b>Netlist</b>: a 10 nF capacitor from out to ground; the standard package adds its ground symbol too.</en>' },
    { t: '.ac dec 20 10 1meg', cls: 'is-analysis', sim: 'use', view: 'skip', plot: true, note: '<zh><b>仿真指令</b>：从 10 Hz 扫到 1 MHz。查看器画图用不到它。</zh><en><b>Simulation command</b>: sweep from 10 Hz to 1 MHz. The viewer does not need it to draw.</en>' },
    { t: '* @RNL {', sim: 'skip', view: 'meta', note: '<zh><b>RNL 记录开始</b>。仿真器眼里，这只是一行以 * 开头的注释。</zh><en><b>Start of an RNL record</b>. To the simulator, this is just a comment line starting with *.</en>' },
    { t: '*   {#V1 #R1 #C1} are treated @AS $FLOW;', r: 'V1 R1 C1 ov:flow', mode: 'flow', sim: 'skip', view: 'draw', note: '<zh><b>RNL</b>：V1、R1、C1 按信号流向排开，默认从左到右。“are treated” 是自由文本，只为读起来像一句话，机器不读。</zh><en><b>RNL</b>: V1, R1 and C1 follow the signal flow, left to right by default. “are treated” is free text, there only so the line reads as a sentence; machines skip it.</en>' },
    { t: '*   #R1 @ORIENT("R90");', r: 'R1', sim: 'skip', view: 'draw', note: '<zh><b>RNL</b>：R1 逆时针转 90°，横着放。</zh><en><b>RNL</b>: R1 turns 90° counterclockwise and lies flat.</en>' },
    { t: '*   on &out there is a @TEXT("V_{out}")', r: 'net:out', sim: 'skip', view: 'draw', note: '<zh><b>RNL</b>：在网络 out 上标出 V<sub>out</sub>，<code>_{…}</code> 写下标。</zh><en><b>RNL</b>: label net out as V<sub>out</sub>; <code>_{…}</code> makes a subscript.</en>' },
    { t: '* }', sim: 'skip', view: 'meta', note: '<zh><b>RNL 记录结束</b>。</zh><en><b>End of the RNL record</b>.</en>' },
    { t: '.end', sim: 'use', view: 'skip', note: '<zh>网表结束。</zh><en>End of the netlist.</en>' }
  ];

  /* ================= Example 1: RLC, what it is and how it flows ================= */

  function rlcScene(style) {
    var dev = {
      V1: { sym: 'vac', x: 3, y: 6, label: 'V1', value: 'AC 1' },
      R1: { sym: 'res', x: 8, y: 12, o: 'R90', label: 'R1', value: '50', lab: [-0.25, 1.35, 'end'], val: [0.25, 1.35, 'start'] },
      L1: { sym: 'ind', x: 16, y: 12, o: 'R90', label: 'L1', value: '1m', lab: [-0.25, 1.35, 'end'], val: [0.25, 1.35, 'start'] },
      C1: { sym: 'cap', x: 23, y: 6, label: 'C1', value: '100n' }
    };
    if (style === 'split') {
      dev.G1 = { sym: 'gnd', x: 3, y: 1, virtual: true };
      dev.G2 = { sym: 'gnd', x: 23, y: 2, virtual: true };
    } else {
      dev.G = { sym: 'gnd', x: 13, y: 1, virtual: true };
    }
    return {
      rk: 'rlc-' + style,
      box: [-1, -2, 28.5, 16.2],
      devices: dev,
      route: function (P) {
        var v1p = P('V1.p'), r1p = P('R1.p'), c1p = P('C1.p'), v1n = P('V1.n'), c1n = P('C1.n');
        var w = [
          { net: 'in', pts: [v1p, [v1p[0], r1p[1]], r1p] },
          { net: 'mid', pts: [P('R1.n'), P('L1.p')] },
          { net: 'out', pts: [P('L1.n'), [c1p[0], P('L1.n')[1]], c1p] }
        ];
        var dots = [];
        if (style === 'split') {
          w.push({ net: '0', pts: [v1n, P('G1.p')] });
          w.push({ net: '0', pts: [c1n, P('G2.p')] });
        } else {
          var g = P('G.p');
          w.push({ net: '0', pts: [v1n, [v1n[0], g[1]], [c1n[0], g[1]], c1n] });
          dots.push([g[0], g[1], '0']);
        }
        return { wires: w, dots: dots, labels: [{ text: 'mid', at: [12, 12.75], net: 'mid' }, { text: 'out', at: [21, 12.75], net: 'out' }] };
      },
      overlays: function (P, S, B, mode) {
        if (mode === 'chain') {
          return [
            { id: 'r-src', ref: 'roles', kind: 'tag', at: [3, -0.4], anchor: 'middle', text: 'source' },
            { id: 'r-ser', ref: 'roles', kind: 'tag', at: [12, 15.1], anchor: 'middle', text: '{series}' },
            { id: 'r-load', ref: 'roles', kind: 'tag', at: [23, -0.4], anchor: 'middle', text: 'load' }
          ];
        }
        if (mode === 'flow') return [{ id: 'flow', kind: 'arrow', pts: [[1.5, 15.1], [25.5, 15.1]], label: '$FLOW · left-to-right', labelAt: [13.5, 15.1], anchor: 'middle', mono: true }];
        return [];
      }
    };
  }

  function e1Lines(style) {
    var merged = style === 'merged';
    var L = [
      { t: '* RLC low-pass' },
      { t: 'V1 in 0 AC 1', r: 'V1' },
      { t: 'R1 in mid 50', r: 'R1' },
      { t: 'L1 mid out 1m', r: 'L1' },
      { t: 'C1 out 0 100n', r: 'C1' },
      { t: '* @RNL {' },
      { t: '*   #V1, {#R1 #L1} and #C1 are treated @AS $CHAIN;', r: 'V1 R1 L1 C1 ov:roles', mode: 'chain' },
      { t: '*   {#V1 #R1 #L1 #C1} are treated @AS $FLOW;', r: 'V1 R1 L1 C1 ov:flow', mode: 'flow' },
      { t: '*   {#R1 #L1} @ORIENT("R90")' + (merged ? ';' : ''), r: 'R1 L1' }
    ];
    if (merged) {
      L.push({ t: '*   &0 @REMOVE-FROM $GROUND_NET_SET;', r: 'G', mark: 'is-add' });
      L.push({ t: '*   $GND is @ATTACH to &0 => named %g;', r: 'G', mark: 'is-add' });
      L.push({ t: '*   net &0 is fully @ROUTE', r: 'G net:0', mark: 'is-add' });
    }
    L.push({ t: '* }' }, { t: '.end' });
    return L;
  }

  /* ================= Example 2: testbench and block symbol ================= */

  var TB = {
    rk: 'tb',
    box: [-1.5, -3, 40, 25.5],
    devices: {
      XU1: { sym: 'block', x: 20, y: 14, label: 'XU1' },
      VIP: { sym: 'vsrc', x: 4, y: 13, label: 'VIP', value: 'DC 0.9', lab: [-2.1, 0.6, 'end'], val: [-2.1, -0.8, 'end'] },
      VIN: { sym: 'vsrc', x: 9, y: 9, label: 'VIN', value: 'DC 0.9' },
      VB: { sym: 'vsrc', x: 17, y: 4, label: 'VB', value: '0.6', lab: [-2.1, 0.6, 'end'], val: [-2.1, -0.8, 'end'] },
      VDD: { sym: 'vsrc', x: 35, y: 17, label: 'VDD', value: '1.8' },
      CL: { sym: 'cap', x: 29, y: 12, label: 'CL', value: '1p' },
      G1: { sym: 'gnd', x: 4, y: 8, virtual: true },
      G2: { sym: 'gnd', x: 9, y: 4, virtual: true },
      G3: { sym: 'gnd', x: 17, y: -1, virtual: true },
      G4: { sym: 'gnd', x: 21, y: 10, virtual: true },
      G5: { sym: 'gnd', x: 35, y: 12, virtual: true },
      G6: { sym: 'gnd', x: 29, y: 8, virtual: true }
    },
    route: function (P) {
      var vd = P('XU1.vdd'), vs = P('VDD.p');
      return {
        wires: [
          { net: 'inp', pts: [P('VIP.p'), [P('VIP.p')[0], P('XU1.inp')[1]], P('XU1.inp')] },
          { net: 'inn', pts: [P('VIN.p'), P('XU1.inn')] },
          { net: 'vb', pts: [P('XU1.vbias'), P('VB.p')] },
          { net: 'vdd', pts: [vd, [vd[0], vs[1]], vs] },
          { net: 'out', pts: [P('XU1.out'), P('CL.p')] },
          { net: '0', pts: [P('VIP.n'), P('G1.p')] },
          { net: '0', pts: [P('VIN.n'), P('G2.p')] },
          { net: '0', pts: [P('VB.n'), P('G3.p')] },
          { net: '0', pts: [P('XU1.vss'), P('G4.p')] },
          { net: '0', pts: [P('VDD.n'), P('G5.p')] },
          { net: '0', pts: [P('CL.n'), P('G6.p')] }
        ],
        dots: [],
        labels: [
          { text: 'inp', at: [9.5, 16.75], net: 'inp' }, { text: 'inn', at: [12, 12.75], net: 'inn' },
          { text: 'vdd', at: [27, 20.75], net: 'vdd' }, { text: 'out', at: [27, 14.75], net: 'out' }
        ]
      };
    },
    overlays: function () {
      return [{ id: 'note', kind: 'text', at: [-0.5, 24], text: '5T OTA testbench', anchor: 'start', tone: 'mute', mono: true }];
    }
  };

  var E2_LINES = [
    { t: '* 5T OTA testbench' },
    { t: '.subckt ota5t inp inn out vdd vss vbias' },
    { t: '* <zh>（M1 … M5 与前面相同，从略）</zh><en>(M1 … M5 as before, left out)</en>' },
    { t: '* @RNL {', r: 'XU1' },
    { t: '*   $OTA_SYM = @DEFINE-SYMBOL {', r: 'XU1' },
    { t: '*     (-5, 2) @PIN(terminal="inp", direction="left");', r: 'XU1 net:inp' },
    { t: '*     (-5, -2) @PIN(terminal="inn", direction="left");', r: 'XU1 net:inn' },
    { t: '*     (5, 0) @PIN(terminal="out", direction="right");', r: 'XU1 net:out' },
    { t: '*     (-1, 3) @PIN(terminal="vdd", direction="up");', r: 'XU1 net:vdd' },
    { t: '*     (1, -2) @PIN(terminal="vss", direction="down");', r: 'XU1 G4' },
    { t: '*     (-3, -4) @PIN(terminal="vbias", direction="down");', r: 'XU1 net:vb' },
    { t: '*     @LINE {(-5, 5) (5, 0) (-5, -5) (-5, 5)};', r: 'XU1' },
    { t: '*     ? @TEXT("${name}", key="name") => @PLACE(-2, 0)', r: 'XU1' },
    { t: '*   };', r: 'XU1' },
    { t: '*   this subcircuit # @SET-SYMBOL to $OTA_SYM', r: 'XU1' },
    { t: '* }', r: 'XU1' },
    { t: '.ends ota5t' },
    { t: 'XU1 inp inn out vdd 0 vb ota5t', r: 'XU1' },
    { t: 'VDD vdd 0 1.8', r: 'VDD G5' },
    { t: 'VB  vb  0 0.6', r: 'VB G3' },
    { t: 'VIP inp 0 DC 0.9 AC 0.5', r: 'VIP G1' },
    { t: 'VIN inn 0 DC 0.9 AC 0.5 180', r: 'VIN G2' },
    { t: 'CL  out 0 1p', r: 'CL G6' },
    { t: '* @RNL {' },
    { t: '*   #XU1 is at @PLACE(20, 14);', r: 'XU1' },
    { t: '*   nets {&inp &inn &out &vdd &vb} are fully @ROUTE;', r: 'net:inp net:inn net:out net:vdd net:vb' },
    { t: '*   at (-0.5, 24) there is a @TEXT("5T OTA testbench") => it @SET-STYLE to $note', r: 'ov:note' },
    { t: '* }' },
    { t: '.end' }
  ];

  /* ================= Example 3: custom symbol ================= */

  var E3_LINES = [
    { t: '* @RNL {' },
    { t: '*   $MY_NMOS = @DEFINE-SYMBOL {', r: 'box' },
    { t: '*     (0, 2) @PIN(terminal="d", label="D", direction="up");', r: 'pin-d' },
    { t: '*     (-3, 0) @PIN(terminal="g", label="G", direction="left");', r: 'pin-g' },
    { t: '*     (0, -2) @PIN(terminal="s", label="S", direction="down");', r: 'pin-s' },
    { t: '*     @LINE {(-3, 0) (-1.4, 0)};', r: 'l1' },
    { t: '*     @LINE {(-1.4, -1.1) (-1.4, 1.1)};', r: 'l2' },
    { t: '*     @LINE(linestyle="bold") {(-0.8, -1.4) (-0.8, 1.4)};', r: 'l3' },
    { t: '*     @LINE {(-0.8, 1) (0, 1) (0, 2)};', r: 'l4' },
    { t: '*     @LINE {(-0.8, -1) (0, -1) (0, -2)};', r: 'l5' },
    { t: '*     @LINE(fill="linecolor") {(-0.2, -1) (-0.7, -0.7) (-0.7, -1.3) (-0.2, -1)};', r: 'poly' },
    { t: '*     ? @TEXT("${name}", key="name") => @PLACE(0.6, 1.2)', r: 'text' },
    { t: '*   };', r: 'box' },
    { t: '*   # @SELECT-INSTANCE(model="nch") => @SET-SYMBOL $MY_NMOS', r: 'rule' },
    { t: '* }' }
  ];

  var E3_NOTES = {
    box: '<zh><b>@DEFINE-SYMBOL</b> 把花括号里的引脚和图元收成一个符号。坐标是符号自己的，(0, 0) 是原点：摆放、对齐都按原点算。</zh><en><b>@DEFINE-SYMBOL</b> gathers the pins and primitives in braces into one symbol. The coordinates are the symbol’s own, and (0, 0) is its origin: placement and alignment are measured from it.</en>',
    'pin-d': '<zh><b>引脚</b>是画端子的点：<code>terminal="d"</code> 对应网表里 MOS 的漏极（写序号 1 也一样），<code>direction="up"</code> 是出线方向，<code>label</code> 只用于显示。</zh><en>A <b>pin</b> is where a terminal is drawn: <code>terminal="d"</code> is the MOS drain in the netlist (the number 1 means the same), <code>direction="up"</code> is the way wires leave, and <code>label</code> is for display only.</en>',
    'pin-g': '<zh>栅极引脚，出线朝左。</zh><en>Gate pin; wires leave to the left.</en>',
    'pin-s': '<zh>源极引脚，出线朝下。衬底不想画，就不写它的引脚：没有引脚的端子不画。</zh><en>Source pin; wires leave downward. To leave the bulk undrawn, give it no pin: a terminal without a pin is not drawn.</en>',
    l1: '<zh><b>@LINE</b> 收一串点：两个点是线段，更多是折线。这一段是栅极的引线。</zh><en><b>@LINE</b> takes a list of points: two make a segment, more make a polyline. This one is the gate lead.</en>',
    l2: '<zh>栅极板。</zh><en>The gate plate.</en>',
    l3: '<zh>沟道，<code>linestyle="bold"</code> 加粗。具体多粗由样式决定，换主题不用改符号。</zh><en>The channel, drawn with <code>linestyle="bold"</code>. How thick that is comes from the style, so a new theme needs no symbol changes.</en>',
    l4: '<zh>漏极的引线，终点就是漏极引脚 (0, 2)。</zh><en>The drain lead, ending on the drain pin at (0, 2).</en>',
    l5: '<zh>源极的引线。</zh><en>The source lead.</en>',
    poly: '<zh>首尾相接的折线加 <code>fill="linecolor"</code>，用线色填满：这是 NMOS 的箭头。</zh><en>A closed polyline with <code>fill="linecolor"</code> is filled with the line color: this is the NMOS arrow.</en>',
    text: '<zh>单独的 <code>?</code> 指用这个符号画出的那个元件：在它旁边显示名字（<code>${name}</code>）。</zh><en>A lone <code>?</code> means the instance drawn with this symbol: its name (<code>${name}</code>) is shown next to it.</en>',
    rule: '<zh>一条<b>规则</b>：所有模型为 nch 的元件都用这个符号。它在写它的这一层求一次，在下面每一层再各求一次。</zh><en>A <b>rule</b>: every instance whose model is nch uses this symbol. It is evaluated on the level it is written in, and once more on each level below.</en>',
    idle: '<zh>把鼠标放到左边任意一行上，看它对应符号里的哪一笔。</zh><en>Hover over any line on the left to see which stroke of the symbol it draws.</en>'
  };

  /* ================= Example 4: rules and fill ================= */

  function fillScene(n) {
    var dev = {
      V1: { sym: 'vsrc', x: -3, y: 10, label: 'V1', value: '1.8', lab: [-2.1, 0.6, 'end'], val: [-2.1, -0.8, 'end'] },
      R1: { sym: 'res', x: 10, y: 14, label: 'R1', value: '10k' },
      M1: { sym: 'nmos', x: 10, y: 6, label: 'M1' },
      C1: { sym: 'cap', x: 18, y: 6, label: 'C1', value: '1p' },
      Gv: { sym: 'gnd', x: -3, y: 7, virtual: true }
    };
    if (n >= 1) {
      dev.Gm = { sym: 'gnd', x: 10, y: 2, virtual: true, cls: 'is-gen' };
      dev.Gc = { sym: 'gnd', x: 18, y: 2, virtual: true, cls: 'is-gen' };
    }
    if (n >= 2) dev.Pin = { sym: 'port', x: 5, y: 6, label: 'in', virtual: true, cls: 'is-gen' };
    return {
      rk: 'fill' + n,
      box: [-7.6, 0, 23, 19.5],
      devices: dev,
      route: function (P) {
        var w = [
          { net: 'vdd', pts: [P('V1.p'), [-3, 17], P('R1.p')] },
          { net: 'out', pts: [P('R1.n'), P('M1.d')] },
          { net: 'out', pts: [[10, 10], [18, 10], P('C1.p')] }
        ];
        if (n >= 1) {
          w.push({ net: '0', pts: [P('M1.s'), P('Gm.p')], cls: 'is-gen' });
          w.push({ net: '0', pts: [P('C1.n'), P('Gc.p')], cls: 'is-gen' });
        }
        if (n >= 2) w.push({ net: 'in', pts: [P('M1.g'), P('Pin.p')], cls: 'is-gen' });
        return { wires: w, dots: [[10, 10, 'out']], labels: [{ text: 'vdd', at: [5, 17.75], net: 'vdd' }, { text: 'out', at: [14, 10.75], net: 'out' }] };
      },
      overlays: function () {
        return [{ id: 'own', ref: 'Gv', kind: 'text', at: [-3, 4.4], text: '<zh>写在引脚上</zh><en>on the pin</en>', anchor: 'middle', tone: 'mute' }];
      }
    };
  }

  var E4_LINES = [
    { t: '* <zh>补位：地由标准包补，端口由一条规则补</zh><en>Fill: grounds from the standard package, the port from a rule</en>' },
    { t: '.model nch nmos level=1' },
    { t: 'V1 vdd 0 1.8', r: 'V1 Gv' },
    { t: 'R1 vdd out 10k', r: 'R1' },
    { t: 'M1 out in 0 0 nch', r: 'M1 Gm', gen: 'gnd' },
    { t: 'C1 out 0 1p', r: 'C1 Gc', gen: 'gnd' },
    { t: '* @RNL {' },
    { t: '*   $PORT_STUB = @DEFINE(roles=[["t", "terminal"]]) {', r: 'Pin' },
    { t: '*     %p = $IOPORT @ATTACH ?t::net;', r: 'Pin' },
    { t: '*     ?t::net @WIRE {%p:p ?t};', r: 'Pin net:in' },
    { t: '*     {?t %p:p} @ALIGN("y");', r: 'Pin' },
    { t: '*     %p:p ?t @DISTANCE("left-to-right", exact=2)', r: 'Pin' },
    { t: '*   };', r: 'Pin' },
    { t: '*   # @SELECT-TERMINAL(terminal="g", wired=false) => $PORT_STUB @ATTACH;', r: 'Pin net:in', gen: 'port' },
    { t: '*   #M1 @SET-SYMBOL $NMOS;', r: 'M1' },
    { t: '*   $GND @ATTACH #V1:2', r: 'Gv ov:own' },
    { t: '* }' },
    { t: '* @RNL(author="rnl-editor") {' },
    { t: '*   #V1 @PLACE(-3, 10);  #R1 @PLACE(10, 14);  #M1 @PLACE(10, 6);  #C1 @PLACE(18, 6);', r: 'V1 R1 M1 C1' },
    { t: '*   %j = (10, 10);', r: 'net:out' },
    { t: '*   &vdd @WIRE(mode="vertical-horizontal") {#V1:1 #R1:1};', r: 'net:vdd' },
    { t: '*   &out @WIRE {#R1:2 %j #M1:d};', r: 'net:out' },
    { t: '*   &out @WIRE(mode="horizontal-vertical") {%j #C1:1};', r: 'net:out' },
    { t: '*   $JUNCTION @ATTACH %j', r: 'net:out' },
    { t: '* }' },
    { t: '.end' }
  ];

  global.RNLScenes = {
    OTA_BOX: OTA_BOX,
    STYLES: STYLES,
    otaScene: otaScene,
    otaCode: otaCode,
    ownerScene: ownerScene,
    e5Scene: e5Scene,
    E5_LINES: E5_LINES,
    E5_DIAG: E5_DIAG,
    RC: RC,
    HERO_LINES: HERO_LINES,
    rlcScene: rlcScene,
    e1Lines: e1Lines,
    TB: TB,
    E2_LINES: E2_LINES,
    E3_LINES: E3_LINES,
    E3_NOTES: E3_NOTES,
    fillScene: fillScene,
    E4_LINES: E4_LINES
  };
})(window);
