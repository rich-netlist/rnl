/* RNL introduction page · scenes and example data
 *
 * The circuits come from examples used in the RNL language discussion (5T OTA, RC/RLC, the fill
 * example). Coordinates are RNL coordinates (x right, y up) and match the solver and editor records
 * shown in the files; they are placed by hand for this page, not computed by a solver. The RNL follows
 * language standard 0.4 and the standard series 0.4, and reads with the rnl command.
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

  var OTA_BOX = [-4.5, -7.5, 43, 23.5];

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
    /* vss is a ground net not named 0, so the standard series 0.4 writes its name under the ground */
    GND: { sym: 'gnd', label: 'vss', virtual: true }
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
      { id: 'cm1', kind: 'box', layer: 'back', ids: ['M3', 'M4'], pad: 1, label: '<en>%cm1 · current mirror</en>', labelAt: [0.5, 1], anchor: 'middle' },
      { id: 'dp1', kind: 'box', layer: 'back', ids: ['M1', 'M2', 'M5'], pad: 1.4, label: '<en>%dp1 · diff pair, tail M5</en>', labelAt: [0.03, 0], anchor: 'start' }
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
    u1: ['%dp1 = #M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N;', '%dp1 = #M1(left), #M2(right) and #M5(tail) treated @AS $DIFFERENTIAL_PAIR_N;'],
    u2: ['%cm1 = #M3(ref) #M4(out) @AS $CURRENT_MIRROR_P', '%cm1 = #M3(ref) and #M4(out) treated @AS $CURRENT_MIRROR_P'],
    u6: ['&vdd @ADD-TO $SUPPLY_NET_SET;', '&vdd @ADD-TO $SUPPLY_NET_SET;'],
    u7: ['&vss @ADD-TO $GROUND_NET_SET;', '&vss @ADD-TO $GROUND_NET_SET;'],
    u14: ['#M5:d @CURRENT("in")', '#M5:d has @CURRENT("in")']
  };
  /* zh: the Chinese standard sentences, written with the alias package alias-zh; only the Chinese pages have them */
  var UZ = {};
  /*zh-CN*/
  UZ = {
    u1: '%dp1 = #M1(左)、#M2(右) 和 #M5(尾管) @视作 $"N型差分对";',
    u2: '%cm1 = #M3(参考管) 和 #M4(输出管) @视作 $"P型电流镜"',
    u6: '&vdd @加入 $"电源网络集";',
    u7: '&vss @加入 $"地网络集";',
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
      if (id === 'u2' && stage >= 2) t += ';';
      add(id, '*   ' + t, r);
    }
    add('h0', '* 5T OTA');
    add('h1', '.model nch nmos level=1 vto=0.5');
    add('h2', '.model pch pmos level=1 vto=-0.5');
    if (stage >= 1) add('hz', k === 2
      ? '* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4"); @USE-ALIASES(name="alias-zh", version="0.4")}'
      : '* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4")}');
    add('h3', '.subckt ota5t inp inn out vdd vss vbias', stage <= 1 ? 'P_inp P_inn P_out P_vb P_vdd P_vss' : 'P_inp P_inn P_out P_vb');
    add('m1', 'M1 x    inp   tail vss nch W=2u L=0.5u', 'M1');
    add('m2', 'M2 out  inn   tail vss nch W=2u L=0.5u', 'M2');
    add('m3', 'M3 x    x     vdd  vdd pch W=4u L=0.5u', 'M3');
    add('m4', 'M4 out  x     vdd  vdd pch W=4u L=0.5u', 'M4');
    add('m5', 'M5 tail vbias vss  vss nch W=4u L=1u', 'M5');
    if (stage >= 1) {
      add('u0', '* @RNL {');
      u('u1', stage >= 2 ? 'M1 M2 M5 ov:dp1 ov:c1 ov:ax ov:ord' : 'M1 M2 M5 ov:dp1');
      u('u2', 'M3 M4 ov:cm1');
      if (stage >= 2) {
        u('u6', 'VDD1 VDD2');
        u('u7', 'GND');
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
      add('s5', '*   /* <en>more wires, junctions, ports, supplies and the ground follow; left out here</en> */');
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
    { t: '* 5T OTA' },
    { t: '* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4")}' },
    { t: '.model nch nmos level=1 vto=0.5' },
    { t: '.model pch pmos level=1 vto=-0.5' },
    { t: '.subckt ota5t inp inn out vdd vss vbias' },
    { t: 'M1 x    inp   tail vss nch W=2u L=0.5u', r: 'M1' },
    { t: 'M2 out  inn   tail vss nch W=2u L=0.5u', r: 'M2' },
    { t: 'M3 x    x     vdd  vdd pch W=4u L=0.5u', r: 'M3' },
    { t: 'M4 out  x     vdd  vdd pch W=4u L=0.5u', r: 'M4' },
    { t: 'M5 tail vbias vss  vss nch W=4u L=1u', r: 'M5' },
    { t: '* @RNL {' },
    { t: '*   %dp1 = #M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N;', r: 'M1 M2 M5 ov:c1 ov:ax', d: 'conflict' },
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
    { k: 'err', id: 'm9', icon: 'cross', html: '<en><code>{#M1 #M2 #M9} @ALIGN("y")</code>: this level has no instance M9. ALIGN is strict about its members, so the whole statement fails; it does not fall back to “align only M1 and M2”.</en>', refs: [] },
    { k: 'err', id: 'bad', icon: 'cross', html: '<en><code>#M4 @PLACE(2O, 16, …)</code>: a number cannot be followed by a letter, and “2O” contains the letter O. Only this statement fails; M4 keeps the solver’s (28, 16).</en>', refs: ['M4'] },
    { k: 'warn', id: 'conflict', icon: 'warn', html: '<en>The editor’s <code>#M2 @PLACE(x=30, …)</code> is high and written later, so it overrides the solver’s x = 28, which is discarded. The midpoint of the two sources moves to x = 21 while M5 stays at 20: the tail centering that <code>%dp1</code> carries is low, conflicts with these high coordinates and gives way, so the tail is no longer centered. A low constraint giving way is expected and noted no further.</en>', refs: ['M2', 'M5', 'ov:c1'] },
    { k: 'info', id: 'fp', icon: 'info', html: '<en>The solver record’s <code>hash</code> no longer matches the current input: the editor added a record after solving. The viewer suggests solving again; the old result still displays.</en>', refs: [] },
    { k: 'ok', id: 'ok', icon: 'check', html: '<en>Everything else takes effect: the differential pair and its tail; the mirror symmetry the pair carries (low as well, but its axis is free and moves to x = 21, so it still holds); the other coordinates.</en>', refs: ['M1', 'M2', 'ov:ax'] }
  ];

  /* ================= Hero: RC low-pass ================= */

  var RC = {
    rk: 'rc',
    box: [0.5, -1.6, 23.5, 16.2],
    devices: {
      V1: { sym: 'vac', x: 4, y: 6, label: 'V1', value: 'AC 1' },
      R1: { sym: 'res', x: 11, y: 12, o: 'R90', label: 'R1', value: '10k', lab: [-0.25, 1.35, 'end'], val: [0.25, 1.35, 'start'] },
      C1: { sym: 'cap', x: 18, y: 6, label: 'C1', value: '10n' },
      G1: { sym: 'gnd', x: 4, y: 2, virtual: true },
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
      return [{ id: 'flow', kind: 'arrow', pts: [[2.6, 15], [20, 15]], label: '@ORDER · left-to-right', labelAt: [11.3, 15], anchor: 'middle', mono: true }];
    }
  };

  var HERO_LINES = [
    { t: '* RC low-pass filter', sim: 'title', view: 'skip', note: '<en>The first line is the title. SPICE treats a file’s first line as its title, not as part of the circuit.</en>' },
    { t: 'V1 in 0 AC 1', r: 'V1 G1', sim: 'use', view: 'draw', note: '<en><b>Netlist</b>: source V1 between in and ground. The grounded end needs no RNL: the standard package adds a ground symbol by itself.</en>' },
    { t: 'R1 in out 10k', r: 'R1', sim: 'use', view: 'draw', note: '<en><b>Netlist</b>: a 10 kΩ resistor from in to out.</en>' },
    { t: 'C1 out 0 10n', r: 'C1 G2', sim: 'use', view: 'draw', note: '<en><b>Netlist</b>: a 10 nF capacitor from out to ground; the standard package adds its ground symbol too.</en>' },
    { t: '.ac dec 20 10 1meg', cls: 'is-analysis', sim: 'use', view: 'skip', plot: true, note: '<en><b>Simulation command</b>: sweep from 10 Hz to 1 MHz. The viewer does not need it to draw.</en>' },
    { t: '* @RNL {', sim: 'skip', view: 'meta', note: '<en><b>Start of an RNL record</b>. To the simulator, this is just a comment line starting with *.</en>' },
    { t: '*   #V1, #R1 and #C1 are in @ORDER("left-to-right");', r: 'V1 R1 C1 ov:flow', mode: 'flow', sim: 'skip', view: 'draw', note: '<en><b>RNL</b>: V1, R1 and C1 run from left to right, the way the signal flows. “are in” is free text, there only so the line reads as a sentence; machines skip it.</en>' },
    { t: '*   #R1 @ORIENT("R90");', r: 'R1', sim: 'skip', view: 'draw', note: '<en><b>RNL</b>: R1 turns 90° counterclockwise and lies flat.</en>' },
    { t: '*   on &out there is a @TEXT("V_{out}")', r: 'net:out', sim: 'skip', view: 'draw', note: '<en><b>RNL</b>: label net out as V<sub>out</sub>; <code>_{…}</code> makes a subscript.</en>' },
    { t: '* }', sim: 'skip', view: 'meta', note: '<en><b>End of the RNL record</b>.</en>' },
    { t: '.end', sim: 'use', view: 'skip', note: '<en>End of the netlist.</en>' }
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
      dev.G1 = { sym: 'gnd', x: 3, y: 2, virtual: true };
      dev.G2 = { sym: 'gnd', x: 23, y: 2, virtual: true };
    } else {
      dev.G = { sym: 'gnd', x: 13, y: 1, virtual: true };
    }
    return {
      rk: 'rlc-' + style,
      box: [-1, -2, 28.5, 17.6],
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
            { id: 'flow', kind: 'arrow', pts: [[1.5, 16.6], [25.5, 16.6]], label: '@ORDER · left-to-right', labelAt: [13.5, 16.6], anchor: 'middle', mono: true }
          ];
        }
        return [];
      }
    };
  }

  function e1Lines(style) {
    var merged = style === 'merged';
    var L = [
      { t: '* RLC low-pass' },
      { t: '* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4")}' },
      { t: 'V1 in 0 AC 1', r: 'V1' },
      { t: 'R1 in mid 50', r: 'R1' },
      { t: 'L1 mid out 1m', r: 'L1' },
      { t: 'C1 out 0 100n', r: 'C1' },
      { t: '* @RNL {' },
      { t: '*   #V1, #R1, #L1 and #C1 are in @ORDER("left-to-right");', r: 'V1 R1 L1 C1 ov:flow', mode: 'chain' },
      { t: '*   {#R1 #L1} @ORIENT("R90")' + (merged ? ';' : ''), r: 'R1 L1' }
    ];
    if (merged) {
      L.push({ t: '*   &0 @REMOVE-FROM $GROUND_NET_SET;', r: 'G', mark: 'is-add' });
      L.push({ t: '*   $GROUND is @ATTACH to &0 => named %g;', r: 'G', mark: 'is-add' });
      L.push({ t: '*   net &0 is @ROUTE-NET', r: 'G net:0', mark: 'is-add' });
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
      VIN: { sym: 'vsrc', x: 9, y: 10, label: 'VIN', value: 'DC 0.9' },
      VB: { sym: 'vsrc', x: 17, y: 4, label: 'VB', value: '0.6', lab: [-2.1, 0.6, 'end'], val: [-2.1, -0.8, 'end'] },
      VDD: { sym: 'vsrc', x: 35, y: 17, label: 'VDD', value: '1.8' },
      CL: { sym: 'cap', x: 29, y: 12, label: 'CL', value: '1p' },
      G1: { sym: 'gnd', x: 4, y: 9, virtual: true },
      G2: { sym: 'gnd', x: 9, y: 6, virtual: true },
      G3: { sym: 'gnd', x: 17, y: 0, virtual: true },
      G4: { sym: 'gnd', x: 21, y: 10, virtual: true },
      G5: { sym: 'gnd', x: 35, y: 13, virtual: true },
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
          { text: 'vdd', at: [27, 19.75], net: 'vdd' }, { text: 'out', at: [27, 14.75], net: 'out' }
        ]
      };
    },
    overlays: function () {
      return [{ id: 'note', kind: 'text', at: [-0.5, 24], text: '5T OTA testbench', anchor: 'start', tone: 'mute', mono: true }];
    }
  };

  var E2_LINES = [
    { t: '* 5T OTA testbench' },
    { t: '* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4")}' },
    { t: '.subckt ota5t inp inn out vdd vss vbias' },
    { t: '* <en>(M1 … M5 as before, left out)</en>' },
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
    { t: '*   nets {&inp &inn &out &vdd &vb} are @ROUTE-NET;', r: 'net:inp net:inn net:out net:vdd net:vb' },
    { t: '*   at (-0.5, 24) there is a @TEXT("5T OTA testbench") => it @SET-STYLE to $note', r: 'ov:note' },
    { t: '* }' },
    { t: '.end' }
  ];

  /* ================= Example 3: custom symbol ================= */

  var E3_LINES = [
    { t: '* A symbol of one\'s own' },
    { t: '* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4")}' },
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
    { t: '*   # @SELECT-INSTANCE(model="nch", recursive=true) => @SET-SYMBOL $MY_NMOS', r: 'rule' },
    { t: '* }' }
  ];

  var E3_NOTES = {
    box: '<en><b>@DEFINE-SYMBOL</b> gathers the pins and primitives in braces into one symbol. The coordinates are the symbol’s own, and (0, 0) is its origin: placement and alignment are measured from it.</en>',
    'pin-d': '<en>A <b>pin</b> is where a terminal is drawn: <code>terminal="d"</code> is the MOS drain in the netlist (the number 1 means the same), <code>direction="up"</code> is the way wires leave, and <code>label</code> is for display only.</en>',
    'pin-g': '<en>Gate pin; wires leave to the left.</en>',
    'pin-s': '<en>Source pin; wires leave downward. To leave the bulk undrawn, give it no pin: a terminal without a pin is not drawn.</en>',
    l1: '<en><b>@LINE</b> takes a list of points: two make a segment, more make a polyline. This one is the gate lead.</en>',
    l2: '<en>The gate plate.</en>',
    l3: '<en>The channel, drawn with <code>linestyle="bold"</code>. How thick that is comes from the style, so a new theme needs no symbol changes.</en>',
    l4: '<en>The drain lead, ending on the drain pin at (0, 2).</en>',
    l5: '<en>The source lead.</en>',
    poly: '<en>A closed polyline with <code>fill="linecolor"</code> is filled with the line color: this is the NMOS arrow.</en>',
    text: '<en>A lone <code>?</code> means the instance drawn with this symbol: its name (<code>${name}</code>) is shown next to it.</en>',
    rule: '<en>A <b>rule</b>: every instance whose model is nch uses this symbol. It is evaluated on the level it is written in and, with <code>recursive=true</code>, once more on each level below; without it, on its own level only.</en>',
    idle: '<en>Hover over any line on the left to see which stroke of the symbol it draws.</en>'
  };

  /* ================= Example 4: rules and fill ================= */

  function fillScene(n) {
    var gen = 'is-gen';
    var dev = {
      R1: { sym: 'res', x: 10, y: 14, label: 'R1', value: '10k' },
      M1: { sym: 'nmos', x: 10, y: 6, label: 'M1' },
      C1: { sym: 'cap', x: 18, y: 6, label: 'C1', value: '1p' },
      Pin: { sym: 'port', x: 4, y: 6, label: 'in', virtual: true }
    };
    if (n >= 1) {
      dev.Gm = { sym: 'gnd', x: 10, y: 2, virtual: true, cls: gen };
      dev.Gc = { sym: 'gnd', x: 18, y: 2, virtual: true, cls: gen };
      dev.Sv = { sym: 'supply', x: 10, y: 18, label: 'vdd', virtual: true, cls: gen };
    }
    if (n >= 2) dev.Pout = { sym: 'port', x: 4, y: 10, label: 'out', virtual: true, cls: gen };
    return {
      rk: 'fill' + n,
      box: [-1.6, 0, 23, 19.5],
      devices: dev,
      route: function (P) {
        var w = [
          { net: 'in', pts: [P('Pin.p'), P('M1.g')] },
          { net: 'out', pts: [P('R1.n'), [10, 10], P('M1.d')] },
          { net: 'out', pts: [[10, 10], [18, 10], P('C1.p')] }
        ];
        if (n >= 1) {
          w.push({ net: '0', pts: [P('M1.s'), P('Gm.p')], cls: gen });
          w.push({ net: '0', pts: [P('C1.n'), P('Gc.p')], cls: gen });
          w.push({ net: 'vdd', pts: [P('R1.p'), P('Sv.p')], cls: gen });
        }
        if (n >= 2) w.push({ net: 'out', pts: [P('Pout.p'), [10, 10]], cls: gen });
        return { wires: w, dots: [[10, 10, 'out']] };
      }
    };
  }

  var E4_LINES = [
    { t: '* <en>Rules and fill: stubs and port tags from the standard package</en>' },
    { t: '* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4")}' },
    { t: '.model nch nmos level=1' },
    { t: '.subckt cs in out vdd' },
    { t: 'R1 vdd out 10k', r: 'R1 Sv' },
    { t: 'M1 out in 0 0 nch', r: 'M1 Gm' },
    { t: 'C1 out 0 1p', r: 'C1 Gc' },
    { t: '* @RNL {' },
    { t: '*   &vdd @ADD-TO $SUPPLY_NET_SET;', r: 'Sv net:vdd', gen: 'stub' },
    { t: '*   &vdd @REMOVE-FROM $PORT_FILL_SET;', r: 'net:vdd', gen: 'port' },
    { t: '*   %tin = $IO_PORT @ATTACH &in;', r: 'Pin net:in' },
    { t: '*   # @SELECT-NET(port=true) => @ROUTE-NET', r: 'net:in net:out', gen: 'port' },
    { t: '* }' },
    { t: '* @RNL(author="rnl-editor") {' },
    { t: '*   #R1 @PLACE(10, 14);  #M1 @PLACE(10, 6);  #C1 @PLACE(18, 6);  %tin @PLACE(4, 6)', r: 'R1 M1 C1 Pin' },
    { t: '* }' },
    { t: '.ends cs' }
  ];

  /* ================= Tour T3: the common-emitter stage ================= */

  /* The file at each preset: 0 only the header record, 1 the supply and ground sets, 2 vdd and vss
     taken out of $PORT_FILL_SET */
  function ceLines(p) {
    var L = ['* Common-emitter stage', '* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4")}', '.model npn1 npn bf=100',
      '.subckt ce in out vdd vss', 'Q1 out in vss npn1', 'RC vdd out 5k', 'CL out vss 1p'];
    if (p >= 1) {
      L.push('* @RNL {');
      L.push('*   &vdd @ADD-TO $SUPPLY_NET_SET;');
      L.push('*   &vss @ADD-TO $GROUND_NET_SET' + (p >= 2 ? ';' : ''));
      if (p >= 2) L.push('*   {&vdd &vss} @REMOVE-FROM $PORT_FILL_SET');
      L.push('* }');
    }
    L.push('.ends ce');
    return L.map(function (s) { return { t: s }; });
  }

  /* ================= Tour: the other complete files ================= */

  var HEADER_LINE = '* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4")}';
  function rows(arr) { return arr.map(function (l) { return typeof l === 'string' ? { t: l } : l; }); }

  /* T2: a pair matched, and laid out as mirror images when on */
  function mpLines(on) {
    var L = rows([
      '* Differential pair', HEADER_LINE, '.model nch nmos level=1',
      { t: 'M1 a b c c nch', r: 'M1' },
      { t: 'M2 d e c c nch', r: 'M2' },
      '* @RNL {',
      { t: '*   %dp = {#M1 #M2} treated @AS $MATCHED' + (on ? ';' : ''), r: 'M1 M2 ov:dp' }
    ]);
    if (on) {
      L.push({ t: '*   %ax = (_, _);', r: 'ov:di' });
      L.push({ t: '*   %sym = #M1(left), #M2(right) and %ax(axis) treated @AS $MIRRORED_LAYOUT', r: 'M1 M2 ov:al ov:di ov:or' });
    }
    L.push({ t: '* }' });
    return L;
  }

  /* T2: a definition with a parameter, applied with gap g */
  function pairLines(g) {
    return rows([
      '* Two resistors', HEADER_LINE, 'R1 a b 1k', 'R2 b c 1k', '* @RNL {',
      '*   $PAIR = @DEFINE(roles=["a", "b"], parameters=[["gap", 2]]) {',
      '*     {?a ?b} @ALIGN("y");',
      '*     ?a ?b @DISTANCE("left-to-right", min=?gap)',
      '*   };',
      { t: g === 2 ? '*   #R1(a) #R2(b) @AS $PAIR   /* gap = 2 */' : '*   #R1(a) #R2(b) @AS(gap=' + g + ') $PAIR', id: 'as' },
      '* }'
    ]);
  }

  /* T5: three wires that meet at one named point, or at three points that only share coordinates */
  function teeLines(shared) {
    var L = rows(['* Three resistors', HEADER_LINE, 'RA n1 0 1k', 'RB n1 0 1k', 'RC n1 0 1k', '* @RNL {']);
    if (shared) {
      L.push({ t: '*   %j = (6, 0);' }, { t: '*   &n1 @WIRE {#RA:1 %j #RB:1};', r: 'RA RB net:n1' }, { t: '*   &n1 @WIRE {%j #RC:1};', r: 'RC net:n1' }, { t: '*   $JUNCTION @ATTACH %j', r: 'net:n1' });
    } else {
      L.push({ t: '*   &n1 @WIRE {#RA:1 (6, 0) #RB:1};', r: 'RA RB net:n1' }, { t: '*   &n1 @WIRE {(6, 0) #RC:1}', r: 'RC net:n1' });
    }
    L.push({ t: '* }' });
    return L;
  }

  /* T5: a bus of two bits */
  var BUS_LINES = rows([
    '* Two inverters on a bus', HEADER_LINE,
    '.subckt inv a y vdd vss', 'M1 y a vdd vdd pch', 'M2 y a vss vss nch', '.ends inv',
    '.model nch nmos level=1', '.model pch pmos level=1',
    { t: 'XI<1> D<1> Y<1> vdd 0 inv', r: 'XI' },
    { t: 'XI<0> D<0> Y<0> vdd 0 inv', r: 'XI' },
    { t: 'R1 D<1> 0 1k', r: 'R1' },
    { t: 'R0 D<0> 0 1k', r: 'R0' },
    '* @RNL {',
    { t: '*   #XI<1:0> @BUNDLE => %xi;', r: 'XI' },
    { t: '*   %s = (_, _); %t1 = (_, _); %t0 = (_, _);', r: 'net:D' },
    { t: '*   %bus = &D<1:0> @WIRE {%s %t1 %t0 %xi:a};', r: 'net:D XI' },
    { t: '*   &D<1> @WIRE {%t1 #R1:1};', r: 'net:D1 R1' },
    { t: '*   &D<0> @WIRE {%t0 #R0:1};', r: 'net:D0 R0' },
    { t: '*   %bus @TEXT("D<1:0>")', r: 'net:D' },
    '* }', '.end'
  ]);

  /* T6: a label of M1 and an annotation of a point */
  function txLines(moved, keyed) {
    var L = rows([
      '* Label and annotation', HEADER_LINE, '.model nch nmos level=1',
      { t: 'M1 d g s s nch W=2u L=0.5u', r: 'M1' },
      '* @RNL {',
      { t: '*   #M1 @PLACE(' + (moved ? 12 : 4) + ', 4);', r: 'M1', id: 'place' },
      { t: '*   #M1 @TEXT("W/L = ${w}/${l}", key="wl", dy=-3);', r: 'M1 ov:wl' },
      { t: '*   (2, 9) @TEXT("bias branch")' + (keyed ? ';' : ''), r: 'ov:an' }
    ]);
    if (keyed) L.push({ t: '*   #M1 @TEXT("${w}/${l}", key="wl", dy=-3)', r: 'M1 ov:wl' });
    L.push({ t: '* }' });
    return L;
  }

  /* T8: a transistor of a PDK whose model the netlist does not define; DEVICE names its terminals */
  function devLines(on) {
    return rows([
      '* PDK example',
      { t: on ? '* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4"); @DEVICE(name="XM*", model="sky130_fd_pr__nfet_*", terminals=["d", "g", "s", "b"])}' : HEADER_LINE, mark: 'is-chg' },
      'XM1 out in 0 0 sky130_fd_pr__nfet_01v8 w=1 l=0.15',
      'RL vdd out 10k',
      '* @RNL {',
      { t: '*   #XM1:d @CURRENT("in")', cls: on ? '' : 'is-err' },
      '* }', '.end'
    ]);
  }

  /* ================= Every complete file the pages show ================= */

  /* Each file the pages show whole, in every style and step they show it in: site/build.py --check reads
     them through site/inline-files.mjs and checks each with rnl check, so that none has an error, a violation
     or a warning, but the ones shown with errors on purpose (expect: 'errors'), which must have one. */
  function files() {
    var F = [];
    function add(name, lines, expect) { F.push({ name: name, lines: lines, expect: expect || 'clean' }); }
    STYLES.forEach(function (style) { add('tour-ota-' + style + '.sp', otaCode(2, style)); });
    [0, 1, 2].forEach(function (p) { add('tour-ce-' + p + '.sp', ceLines(p)); });
    [false, true].forEach(function (on) { add('tour-pair-' + (on ? 'mirrored' : 'matched') + '.sp', mpLines(on)); });
    [2, 4].forEach(function (g) { add('tour-pair-gap-' + g + '.sp', pairLines(g)); });
    [true, false].forEach(function (s) { add('tour-tee-' + (s ? 'shared' : 'apart') + '.sp', teeLines(s)); });
    add('tour-bus.sp', BUS_LINES);
    [[false, false], [true, true]].forEach(function (v) { add('tour-text-' + (v[0] ? 'moved' : 'first') + '.sp', txLines(v[0], v[1])); });
    add('tour-device.sp', devLines(true));
    add('tour-device-unknown.sp', devLines(false), 'errors');
    add('example1-rlc.sp', e1Lines('plain'));
    add('example1-rlc-merged.sp', e1Lines('merged'));
    add('example2-testbench.sp', E2_LINES);
    add('example3-symbol.sp', E3_LINES);
    add('example4-fill.sp', E4_LINES);
    add('example5-errors.sp', E5_LINES, 'errors');
    return F;
  }

  global.RNLScenes = {
    files: files,
    OTA_BOX: OTA_BOX,
    STYLES: STYLES,
    otaScene: otaScene,
    otaCode: otaCode,
    ceLines: ceLines,
    mpLines: mpLines,
    pairLines: pairLines,
    teeLines: teeLines,
    BUS_LINES: BUS_LINES,
    txLines: txLines,
    devLines: devLines,
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
