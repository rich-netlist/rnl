/* RNL introduction page · scenes and example data
 *
 * The circuits come from examples used in the RNL language discussion (5T OTA, RC/RLC, the fill
 * example). The five-transistor OTA is not drawn here: its files are site/examples/ota5t*.sp, solved by
 * the solver, and the core draws them (site/render-examples.mjs, ota-looks.js). The other scenes are
 * drawn by this page from the coordinates below, in RNL coordinates (x right, y up), which the files
 * shown with them write as the author's positions. The RNL follows language standard 0.4 and the
 * standard series 0.4, and reads with the rnl command.
 */
(function (global) {
  'use strict';

  /* ================= Five-transistor OTA ================= */

  /* The core's drawing of a look of the OTA (ota-looks.js: standard, eda, plain, edits) put into the <svg> element el,
     or into a new one in el when el is not an <svg>. Its parts keep data-dev (a device) and data-net (a wire), so that
     the drawing can light up what a line of the file is about: the result is a view for RNLCode.link, whose hover over
     a device calls onHover with its name. */
  function coreView(el, look, onHover) {
    var L = global.RNLOtaLooks && global.RNLOtaLooks.looks[look];
    if (!el || !L) return null;
    var box = document.createElement('div');
    box.innerHTML = L.svg;
    var src = box.querySelector('svg'), svg = el;
    if (el.tagName.toLowerCase() !== 'svg') {
      el.innerHTML = '';
      svg = el.appendChild(document.createElementNS('http://www.w3.org/2000/svg', 'svg'));
    }
    ['viewBox', 'font-family', 'style'].forEach(function (a) { if (src.getAttribute(a)) svg.setAttribute(a, src.getAttribute(a)); });
    svg.innerHTML = src.innerHTML;
    svg.classList.add('core-drawing');
    function highlight(refs) {
      var any = false;
      Array.prototype.forEach.call(svg.querySelectorAll('.is-lit'), function (g) { g.classList.remove('is-lit'); });
      (refs || []).forEach(function (r) {
        Array.prototype.forEach.call(svg.querySelectorAll('[data-dev="' + r + '"]'), function (g) { g.classList.add('is-lit'); any = true; });
      });
      svg.classList.toggle('has-lit', any);
    }
    if (onHover) {
      svg.addEventListener('mouseover', function (e) {
        var g = e.target.closest ? e.target.closest('[data-dev]') : null;
        onHover(g ? g.getAttribute('data-dev') : null);
      });
      svg.addEventListener('mouseleave', function () { onHover(null); });
    }
    return { svg: svg, highlight: highlight, setMode: function () {} };
  }

  /* The whole file, as the language page shows it, in one of three writing styles: bare, the formal notation only;
     en, the standard sentences; zh, the Chinese ones. Each statement is a line of site/examples/ota5t.sp (bare and en)
     or ota5t-zh.sp (zh); site/build.py checks them. The file the viewer opens has the solver's record as well. */
  var U = {
    u1: ['%dp1 = #M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N;', '%dp1 = #M1(left), #M2(right) and #M5(tail) treated @AS $DIFFERENTIAL_PAIR_N;'],
    u2: ['%cm1 = #M3(ref) #M4(out) @AS $CURRENT_MIRROR_P;', '%cm1 = #M3(ref) and #M4(out) treated @AS $CURRENT_MIRROR_P;'],
    u3: ['%amp = %dp1(transconductor) %cm1(load) @AS $TRANSCONDUCTOR_Z_AMPLIFIER;', '%amp = %dp1(transconductor) and %cm1(load) treated @AS $TRANSCONDUCTOR_Z_AMPLIFIER;'],
    u4: ['%left = {#M3 #M1} @AS $BRANCH;', '%left = {#M3 #M1} treated @AS $BRANCH;'],
    u5: ['%right = {#M4 #M2} @AS $BRANCH;', '%right = {#M4 #M2} treated @AS $BRANCH;'],
    u6: ['&vdd @ADD-TO $SUPPLY_NET_SET;', '&vdd @ADD-TO $SUPPLY_NET_SET;'],
    u7: ['&vss @ADD-TO $GROUND_NET_SET;', '&vss @ADD-TO $GROUND_NET_SET;'],
    u14: ['#M5:d @CURRENT("in");', '#M5:d has @CURRENT("in");'],
    u16: ['# @SELECT-NET(port=true) => @REMOVE-FROM $PORT_FILL_SET;', '# @SELECT-NET(port=true) => @REMOVE-FROM $PORT_FILL_SET;'],
    u17: ['%p_inp = $IO_PORT @ATTACH &inp;', '%p_inp = $IO_PORT @ATTACH &inp;'],
    u18: ['%p_inn = $IO_PORT @ATTACH &inn;', '%p_inn = $IO_PORT @ATTACH &inn;'],
    u19: ['%p_out = $IO_PORT @ATTACH &out;', '%p_out = $IO_PORT @ATTACH &out;'],
    u21: ['%p_bias = $IO_PORT @ATTACH &vbias;', '%p_bias = $IO_PORT @ATTACH &vbias;'],
    u20: ['# @SELECT-NET => @ROUTE-NET;', '# @SELECT-NET => @ROUTE-NET;'],
    u22: ['%amp_group = {%amp %p_inp %p_inn %p_out %p_bias} @GROUP;', '%amp_group = {%amp %p_inp %p_inn %p_out %p_bias} @GROUP;'],
    u23: ['{%amp %p_out} @ORDER("left-to-right")', '{%amp %p_out} @ORDER("left-to-right")']
  };
  /* zh: the Chinese standard sentences, written with the alias package alias-zh; only the Chinese pages have them */
  var UZ = {};
  /*zh-CN*/
  UZ = {
    u1: '%dp1 = #M1(左)、#M2(右) 和 #M5(尾管) @视作 $"N型差分对";',
    u2: '%cm1 = #M3(参考管) 和 #M4(输出管) @视作 $"P型电流镜";',
    u3: '%amp = %dp1(跨导级) 和 %cm1(负载) @视作 $"跨导阻抗放大器";',
    u4: '%left = {#M3 #M1} @视作 $"支路";',
    u5: '%right = {#M4 #M2} @视作 $"支路";',
    u6: '&vdd @加入 $"电源网络集";',
    u7: '&vss @加入 $"地网络集";',
    u14: '#M5:d 有 @电流("流入");',
    u16: '# @选出网络(端口=是) => @移出 $"端口补位集";',
    u17: '%p_inp = $"端口" @附着 &inp;',
    u18: '%p_inn = $"端口" @附着 &inn;',
    u19: '%p_out = $"端口" @附着 &out;',
    u21: '%p_bias = $"端口" @附着 &vbias;',
    u20: '# @选出网络 => @布线网络;',
    u22: '%amp_group = {%amp %p_inp %p_inn %p_out %p_bias} 视作一个 @分组;',
    u23: '{%amp %p_out} @顺序("从左到右")'
  };
  /*/zh-CN*/
  var STYLES = ['bare', 'en', 'zh'];

  function otaCode(style) {
    var k = Math.max(0, STYLES.indexOf(style || 'en'));
    var L = [];
    function add(id, t) { L.push({ id: id, t: t }); }
    add('h0', '* 5T OTA');
    add('h1', '.model nch nmos level=1 vto=0.5');
    add('h2', '.model pch pmos level=1 vto=-0.5');
    add('hz', k === 2
      ? '* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4"); @USE-ALIASES(name="alias-zh", version="0.4")}'
      : '* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4")}');
    add('h3', '.subckt ota5t inp inn out vdd vss vbias');
    add('m1', 'M1 x    inp   tail vss nch W=2u L=0.5u');
    add('m2', 'M2 out  inn   tail vss nch W=2u L=0.5u');
    add('m3', 'M3 x    x     vdd  vdd pch W=4u L=0.5u');
    add('m4', 'M4 out  x     vdd  vdd pch W=4u L=0.5u');
    add('m5', 'M5 tail vbias vss  vss nch W=4u L=1u');
    add('u0', '* @RNL {');
    ['u1', 'u2', 'u3', 'u4', 'u5', 'u6', 'u7', 'u14', 'u16', 'u17', 'u18', 'u19', 'u21', 'u20', 'u22', 'u23'].forEach(function (id) {
      add(id, '*   ' + (k === 2 ? UZ[id] : U[id][k]));
    });
    add('u15', '* }');
    add('end', '.ends ota5t');
    return L;
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
    { t: '*   %dp1 = #M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N;', r: 'M1 M2 M5' },
    { t: '*   {#M1 #M2 #M9} @ALIGN("y")', cls: 'is-err', d: 'm9' },
    { t: '* }' },
    { t: '* @RNL(author="rnl-editor") {', d: 'move' },
    { t: '*   #M2 @PLACE(x=18, priority="high");', r: 'M2', d: 'move' },
    { t: '*   #M4 @PLACE(2O, 26, priority="high")', r: 'M4', cls: 'is-err', d: 'bad' },
    { t: '* }' },
    { t: '* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:a0c875a9451d0e20.3cb60203afe238d5", numbering="9912cd934557ce94") {', d: 'solve' },
    { t: '*   #M1 @PLACE(6, 16) => @ORIENT("R0");', r: 'M1', d: 'move' },
    { t: '*   #M2 @PLACE(18, 16) => @ORIENT("MY");', r: 'M2', d: 'move' },
    { t: '*   #M3 @PLACE(6, 26) => @ORIENT("MY");', r: 'M3', d: 'move' },
    { t: '*   #M4 @PLACE(18, 26) => @ORIENT("R0");', r: 'M4', d: 'bad' },
    { t: '*   #M5 @PLACE(12, 9) => @ORIENT("R0");', r: 'M5', d: 'move' },
    { t: '*   /* <en>more wires, junctions, ports, supplies and the ground follow; left out here</en> */', fold: true },
    { t: '* }' },
    { t: '.ends ota5t' }
  ];

  /* What the core says of the file (rnl check site/examples/ota5t-edits.sp, with the two statements shown as errors
     taken out of their comments): solved again after the editor's move, so nothing conflicts */
  var E5_DIAG = [
    { k: 'err', id: 'm9', icon: 'cross', html: '<en><code>{#M1 #M2 #M9} @ALIGN("y")</code>: this level has no instance M9. ALIGN is strict about its members, so the whole statement fails; it does not fall back to “align only M1 and M2”.</en>', refs: [] },
    { k: 'err', id: 'bad', icon: 'cross', html: '<en><code>#M4 @PLACE(2O, 26, …)</code>: a number cannot be followed by a letter, and “2O” contains the letter O. Only this statement fails; the solver places M4, at (18, 26) above M2.</en>', refs: ['M4'] },
    { k: 'ok', id: 'move', icon: 'check', html: '<en>The editor’s <code>#M2 @PLACE(x=18, …)</code> is a record like the author’s, and high. Solved again, M2 stays at x = 18 and the solver lays out the rest around it: M1 moves to x = 6, M4 stays above M2 and the tail M5 stays centered under the pair, at x = 12. To move a device, write where it goes in a record of your own and let the solver arrange the rest.</en>', refs: ['M1', 'M2', 'M4', 'M5'] },
    { k: 'info', id: 'solve', icon: 'info', html: '<en>The solver writes its record after the others, with a <code>hash</code> of what it read. Had the editor’s record been added without solving again, the hash would no longer match: the viewer would still show the old drawing and suggest solving again.</en>', refs: [] },
    { k: 'ok', id: 'ok', icon: 'check', html: '<en>Everything else takes effect: the differential pair and its tail, the current mirror, the orientations and the other coordinates.</en>', refs: ['M1', 'M2', 'M5'] }
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
    { t: '*     (-1, 5) @PIN(terminal="vdd", direction="up");', r: 'XU1 net:vdd' },
    { t: '*     (1, -5) @PIN(terminal="vss", direction="down");', r: 'XU1 G4' },
    { t: '*     (-3, -5) @PIN(terminal="vbias", direction="down");', r: 'XU1 net:vb' },
    { t: '*     @LINE {(-5, 5) (5, 0) (-5, -5) (-5, 5)};', r: 'XU1' },
    { t: '*     @LINE {(-1, 3) (-1, 5)}; @LINE {(1, -2) (1, -5)}; @LINE {(-3, -4) (-3, -5)};', r: 'XU1' },
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
    { t: '*   {#VIP #VIN #XU1 #CL} are in @ORDER("left-to-right");', r: 'VIP VIN XU1 CL' },
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
    STYLES.forEach(function (style) { add('tour-ota-' + style + '.sp', otaCode(style)); });
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
    coreView: coreView,
    STYLES: STYLES,
    otaCode: otaCode,
    ceLines: ceLines,
    mpLines: mpLines,
    pairLines: pairLines,
    teeLines: teeLines,
    BUS_LINES: BUS_LINES,
    txLines: txLines,
    devLines: devLines,
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
