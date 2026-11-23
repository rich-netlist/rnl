/* RNL introduction page · the example: a five-transistor OTA, reviewed by what it says
 *
 * The files are site/examples/ota5t-review-bare.sp (the notation), ota5t-review.sp (English sentences) and
 * ota5t-review-zh.sp (Chinese sentences, only on the Chinese pages), and ota5t-review-eda.sp: the same file with the
 * public packages eda-symbol and eda-style. Each file is solved by the solver (rnl solve), which writes its record into
 * it. This script keeps the lines the author wrote, and site/build.py checks them against the files; the records of the
 * solver come from ota-looks.js, which takes them from the files. The drawings are not drawn here either: ota-looks.js
 * holds what the RNL core draws from the standard-packages file and from the file with the two packages
 * (node site/render-examples.mjs), with the parts of every device marked in it.
 * The review: picking a statement of meaning, in the file or in the checklist, marks in the drawing what it is about as an
 * RNL viewer marks it (apps/viewer-ui): the frame of the usage it makes, %dp1 with its title and its roles, or, for one
 * that makes none, a frame around its devices; a device, from its line of the netlist or in the drawing, is selected as
 * the viewer selects it, in the colour of the selection with its box. Nothing else is dimmed. The frames and the boxes
 * are the core's (ota-looks.js). The look switch swaps one core drawing for the other; the circuit is the same.
 */
(function (global) {
  'use strict';

  var C = global.RNLCode;
  var REDUCE = !!(global.matchMedia && global.matchMedia('(prefers-reduced-motion: reduce)').matches);

  /* ---------- The file ---------- */

  /* Each statement of meaning in the writing styles: the formal notation, the standard English sentences, the Chinese
     ones (the alias package alias-zh; only the Chinese pages have them) */
  var T = {
    dp: ['%dp1 = #M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N;', '%dp1 = #M1(left), #M2(right) and #M5(tail) treated @AS $DIFFERENTIAL_PAIR_N;'],
    cm1: ['%cm1 = #M3(ref) #M4(out) @AS $CURRENT_MIRROR_P;', '%cm1 = #M3(ref) and #M4(out) treated @AS $CURRENT_MIRROR_P;'],
    amp: ['%amp = %dp1(transconductor) %cm1(load) @AS $TRANSCONDUCTOR_Z_AMPLIFIER;', '%amp = %dp1(transconductor) and %cm1(load) treated @AS $TRANSCONDUCTOR_Z_AMPLIFIER;'],
    br1: ['%left = {#M3 #M1} @AS $BRANCH;', '%left = {#M3 #M1} treated @AS $BRANCH;'],
    br2: ['%right = {#M4 #M2} @AS $BRANCH;', '%right = {#M4 #M2} treated @AS $BRANCH;'],
    n1: ['&vdd @ADD-TO $SUPPLY_NET_SET;', '&vdd @ADD-TO $SUPPLY_NET_SET;'],
    n2: ['&vss @ADD-TO $GROUND_NET_SET;', '&vss @ADD-TO $GROUND_NET_SET;'],
    cur: ['#M5:d @CURRENT("in");', '#M5:d has @CURRENT("in");'],
    /* how to draw it, as the gallery's examples do: no port tags from the fill, a port of its own for each signal and the
       bias, every net as wires, the stage and its ports one block, the output port to the right of the stage */
    fill: ['# @SELECT-NET(port=true) => @REMOVE-FROM $PORT_FILL_SET;', '# @SELECT-NET(port=true) => @REMOVE-FROM $PORT_FILL_SET;'],
    pin: ['%p_inp = $IO_PORT @ATTACH &inp;', '%p_inp = $IO_PORT @ATTACH &inp;'],
    pinn: ['%p_inn = $IO_PORT @ATTACH &inn;', '%p_inn = $IO_PORT @ATTACH &inn;'],
    pout: ['%p_out = $IO_PORT @ATTACH &out;', '%p_out = $IO_PORT @ATTACH &out;'],
    pbias: ['%p_bias = $IO_PORT @ATTACH &vbias;', '%p_bias = $IO_PORT @ATTACH &vbias;'],
    rt: ['# @SELECT-NET => @ROUTE-NET;', '# @SELECT-NET => @ROUTE-NET;'],
    grp: ['%amp_group = {%amp %p_inp %p_inn %p_out %p_bias} @GROUP;', '%amp_group = {%amp %p_inp %p_inn %p_out %p_bias} @GROUP;'],
    ord: ['{%amp %p_out} @ORDER("left-to-right")', '{%amp %p_out} @ORDER("left-to-right")']
  };
  var TZ = {};
  /*zh-CN*/
  TZ = {
    dp: '%dp1 = #M1(左)、#M2(右) 和 #M5(尾管) @视作 $"N型差分对";',
    cm1: '%cm1 = #M3(参考管) 和 #M4(输出管) @视作 $"P型电流镜";',
    amp: '%amp = %dp1(跨导级) 和 %cm1(负载) @视作 $"跨导阻抗放大器";',
    br1: '%left = {#M3 #M1} @视作 $"支路";',
    br2: '%right = {#M4 #M2} @视作 $"支路";',
    n1: '&vdd @加入 $"电源网络集";',
    n2: '&vss @加入 $"地网络集";',
    cur: '#M5:d 有 @电流("流入");',
    fill: '# @选出网络(端口=是) => @移出 $"端口补位集";',
    pin: '%p_inp = $"端口" @附着 &inp;',
    pinn: '%p_inn = $"端口" @附着 &inn;',
    pout: '%p_out = $"端口" @附着 &out;',
    pbias: '%p_bias = $"端口" @附着 &vbias;',
    rt: '# @选出网络 => @布线网络;',
    grp: '%amp_group = {%amp %p_inp %p_inn %p_out %p_bias} 视作一个 @分组;',
    ord: '{%amp %p_out} @顺序("从左到右")'
  };
  /*/zh-CN*/
  /* The usage each statement of meaning makes, whose frame picking it shows; a statement that makes none frames the
     devices it is about */
  var USAGE = { dp: '%dp1', cm1: '%cm1', amp: '%amp', br1: '%left', br2: '%right', pin: '%p_inp', pinn: '%p_inn',
    pout: '%p_out', pbias: '%p_bias', grp: '%amp_group' };
  var STYLES = ['bare', 'en', 'zh'];
  var FILES = { bare: 'ota5t-review-bare.sp', en: 'ota5t-review.sp', zh: 'ota5t-review-zh.sp', eda: 'ota5t-review-eda.sp' };
  var PACKAGES = '; @USE-PACKAGE(name="eda-symbol", version="0.2"); @USE-PACKAGE(name="eda-style", version="0.2")';

  /* The lines of the file as the panel shows them; r is what a line is about (devices and display objects, the notes of
     a statement as ov:…, the usage a statement makes as u:%name, or frame for one that makes none), dev a line of the
     netlist that is about one device.
     The records of the solver are the file's own (ota-looks.js): each line is about the devices and ports it names. */
  function lines(style, look) {
    var k = Math.max(0, STYLES.indexOf(style || 'en'));
    var L = [];
    function add(id, t, r, extra) {
      var o = { id: id, t: t, r: r || '' };
      if (extra) for (var key in extra) o[key] = extra[key];
      L.push(o);
    }
    function st(id, r) { add(id, '*   ' + (k === 2 ? TZ[id] : T[id][k]), r && r + (USAGE[id] ? ' u:' + USAGE[id] : ' frame')); }
    function dev(id, t) { add(id.toLowerCase(), t, id, { dev: id }); }
    function record(n, block) {
      (block || []).forEach(function (t, i) {
        var about = (t.match(/#M\d\b/g) || []).map(function (d) { return d.slice(1); });
        (t.match(/%p_(inp|inn|out|bias)\b/g) || []).forEach(function (p) { about.push(p.slice(1).toUpperCase()); });
        add('g' + n + '-' + i, t, about.join(' '));
      });
    }
    var LOOKS = global.RNLOtaLooks, rec = LOOKS && LOOKS.records && LOOKS.records[look === 'eda' ? FILES.eda : FILES[STYLES[k]]];
    var blocks = rec ? rec.blocks : [];
    add('h0', '* Five-transistor OTA');
    add('h1', '.model nch nmos level=1 vto=0.5 kp=100u lambda=0.05');
    add('h2', '.model pch pmos level=1 vto=-0.5 kp=100u lambda=0.05');
    add('h3', '* @RNL(type="header") { ' + (k === 2 ? '@VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4"); @USE-ALIASES(name="alias-zh", version="0.4")' : '@VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4")') + (look === 'eda' ? PACKAGES : '') + '}', '', look === 'eda' ? { cls: 'is-diff' } : null);
    add('h4', '.subckt ota5t inp inn out vdd vss vbias', 'P_INP P_INN P_OUT P_BIAS');
    dev('M1', 'M1 x    inp   tail vss nch W=2u L=0.5u');
    dev('M2', 'M2 out  inn   tail vss nch W=2u L=0.5u');
    dev('M3', 'M3 x    x     vdd  vdd pch W=4u L=0.5u');
    dev('M4', 'M4 out  x     vdd  vdd pch W=4u L=0.5u');
    dev('M5', 'M5 tail vbias vss  vss nch W=4u L=1u');
    add('r0', '* @RNL {');
    st('dp', 'M1 M2 M5');
    st('cm1', 'M3 M4');
    st('amp', 'M1 M2 M3 M4 M5');
    st('br1', 'M3 M1');
    st('br2', 'M4 M2');
    st('n1', 'VDD');
    st('n2', 'VSS');
    st('cur', 'M5 ov:cur');
    st('fill');
    st('pin', 'P_INP');
    st('pinn', 'P_INN');
    st('pout', 'P_OUT');
    st('pbias', 'P_BIAS');
    st('rt');
    st('grp', 'M1 M2 M3 M4 M5 P_INP P_INN P_OUT P_BIAS');
    st('ord', 'M1 M2 M3 M4 M5 P_OUT');
    add('r1', '* }');
    record(0, blocks[0]);
    add('e0', '.ends ota5t');
    blocks.slice(1).forEach(function (b, n) { record(n + 1, b); });
    add('e1', '.end');
    return L;
  }

  /* ---------- The review ---------- */

  /* One item of the checklist for each statement of meaning: what the agent declared, in plain words */
  var ITEMS = [
    { key: 'dp', kind: 'structure', devs: 'M1 M2 M5', stmt: ['dp'], text: '<en><b>M1</b> and <b>M2</b> are the input pair, <b>M5</b> its tail source.</en>' },
    { key: 'cm1', kind: 'structure', devs: 'M3 M4', stmt: ['cm1'], text: '<en><b>M3</b> and <b>M4</b> are a current mirror, the load.</en>' },
    { key: 'cur', kind: 'intent', devs: 'M5', stmt: ['cur'], text: '<en>The tail current enters at the drain of <b>M5</b>.</en>' }
  ];

  var KIND = { structure: '<en>structure</en>', intent: '<en>intent</en>' };

  var DEFAULT = '<en>Pick a statement in the file or an item in the checklist: its devices light up in the drawing and the netlist.</en>';
  var CAPS = {
    dp: '<en><b>Structure</b>: <code>$DIFFERENTIAL_PAIR_N</code> says which devices form the pair of N devices and which is its tail. The standard package draws the pair as mirror images about one axis, with the tail under its middle.</en>',
    cm1: '<en><b>Structure</b>: <code>$CURRENT_MIRROR_P</code> says which device is the reference and which copies it. Here the mirror of P devices is the pair’s load.</en>',
    cur: '<en><b>Intent</b>: where the current flows, from the top of the circuit to the bottom.</en>'
  };
  var SIM = '<en>A simulator reads the lines that are not dimmed: the netlist. Every RNL line starts with <code>*</code>, so for the simulator it is a comment.</en>';

  /* What picking a statement shows in the drawing besides its frame: the tail current. The solver places the devices, so
     the note is placed from the drawing itself: the box of the parts of each device the core marks (data-dev), and of
     each net's wires (data-net), in the drawing's own units (x grows to the right, y downwards). */
  function arrow(x0, y0, x1, y1) {
    var dx = x1 - x0, dy = y1 - y0, n = Math.sqrt(dx * dx + dy * dy), ux = dx / n, uy = dy / n, h = 7;
    var bx = x1 - ux * h * 1.4, by = y1 - uy * h * 1.4;
    return '<line x1="' + x0 + '" y1="' + y0 + '" x2="' + bx + '" y2="' + by + '"/>' +
      '<path class="head" d="M' + x1 + ' ' + y1 + ' L' + (bx - uy * h * 0.6) + ' ' + (by + ux * h * 0.6) + ' L' + (bx + uy * h * 0.6) + ' ' + (by - ux * h * 0.6) + ' Z"/>';
  }
  function label(x, y, text, anchor) { return '<text x="' + x + '" y="' + y + '" text-anchor="' + (anchor || 'middle') + '">' + text + '</text>'; }
  function box(svg, sel) {
    var b = null;
    Array.prototype.forEach.call(svg.querySelectorAll(sel), function (el) {
      var r;
      try { r = el.getBBox(); } catch (e) { return; }
      if (!r.width && !r.height) return;
      b = b ? { x0: Math.min(b.x0, r.x), y0: Math.min(b.y0, r.y), x1: Math.max(b.x1, r.x + r.width), y1: Math.max(b.y1, r.y + r.height) }
        : { x0: r.x, y0: r.y, x1: r.x + r.width, y1: r.y + r.height };
    });
    return b;
  }
  function notes(svg) {
    var N = {}, tail = box(svg, '[data-net="tail"]'), m5 = box(svg, '[data-dev="M5"]');
    if (tail && m5) {
      /* the tail current, beside the wire that comes down from the pair into the drain of M5 */
      var x = (tail.x0 + tail.x1) / 2 - 14, y1 = m5.y0 + 4, y0 = Math.max(tail.y0 + 4, y1 - 22);
      N.cur = arrow(x, y0, x, y1) + label(x - 8, (y0 + y1) / 2 + 4, 'I', 'end');
    }
    return N;
  }

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
      '<p class="rv-sim-n"><en>1 V differential input and a 1 pF load, simulated from the netlist lines in the file.</en> <code>' + R.analysis + '</code></p>';
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
      stage.classList.toggle('is-black', look === 'eda');   /* the traditional look is drawn on black, as in the EDA tools */
      svg = stage.querySelector('svg');
      svg.setAttribute('role', 'img');
      svg.setAttribute('aria-label', look === 'eda' ? 'Five-transistor OTA in the traditional EDA look' : 'Five-transistor OTA');
      var g = '', N = notes(svg);
      Object.keys(N).forEach(function (key) { g += '<g class="rv-note" data-note="' + key + '">' + N[key] + '</g>'; });
      svg.insertAdjacentHTML('beforeend', g);
      lit(refs);
    }
    /* Mark what refs are about as an RNL viewer marks it (apps/viewer-ui), nothing else dimmed: the notes named ov:key;
       the frame of each usage named u:%name, as the viewer draws a selected usage (usages.js, paintUsages): a dotted
       frame in the usage's colour over a fill of it, with its title and its roles; for a statement that makes no usage
       (frame), a frame of the same kind around its devices, in the colour of the selection, as the viewer frames what a
       constraint acts on (makeConstraintBox); else each device selected, as the viewer selects one (drawing.js): its
       parts in the colour of the selection, and its box. The viewer draws these in pixels of the screen over a drawing
       it zooms; here the drawing is fitted to its frame, so the pixels are turned into the drawing's units. */
    var NS = 'http://www.w3.org/2000/svg', FRAME = '#4263eb', SELECT = '#e8590c', SELECT_FILL = 'rgba(232, 89, 12, 0.07)';
    function el(name, attrs, parent) {
      var e = document.createElementNS(NS, name);
      for (var a in attrs) e.setAttribute(a, attrs[a]);
      if (parent) parent.appendChild(e);
      return e;
    }
    function badge(parent, text, color, x, y, width, centered, px) {
      var g = el('g', { transform: 'translate(' + x + ' ' + y + ') scale(' + px + ')' }, parent);
      var x0 = centered ? -width / 2 : 0;
      el('rect', { x: x0 - 4, y: -9, width: width + 8, height: 18, rx: 3, fill: look === 'eda' ? '#000' : '#fff', 'fill-opacity': 0.75 }, g);
      var t = el('text', { x: x0, fill: color, 'font-size': 13, 'font-family': 'system-ui, sans-serif', 'dominant-baseline': 'central',
        textLength: width, lengthAdjust: 'spacingAndGlyphs' }, g);
      t.textContent = text;
    }
    function frame(under, over, b, color, px, fill) {
      var a = { x: b[0], y: b[1], width: b[2] - b[0], height: b[3] - b[1], rx: 10 * px };
      el('rect', Object.assign({ fill: color, 'fill-opacity': fill }, a), under);
      el('rect', Object.assign({ fill: 'none', stroke: color, 'stroke-width': 8 / 3 * px, 'stroke-dasharray': px + ' ' + 5 * px,
        'stroke-linecap': 'round' }, a), over);
    }
    function lit(list2) {
      refs = list2 && list2.length ? list2 : null;
      if (!svg) return;
      Array.prototype.forEach.call(svg.querySelectorAll('.is-lit'), function (e) { e.classList.remove('is-lit'); });
      Array.prototype.forEach.call(svg.querySelectorAll('.rv-note.is-on'), function (e) { e.classList.remove('is-on'); });
      Array.prototype.forEach.call(svg.querySelectorAll('.rv-marks'), function (e) { e.remove(); });
      if (!refs) return;
      var shown = LOOKS.looks[look], ctm = svg.getScreenCTM();
      var px = ctm && ctm.a ? 1 / ctm.a : 1;   /* the drawing's units in a pixel of the screen */
      var under = el('g', { class: 'rv-marks', 'pointer-events': 'none' });
      svg.insertBefore(under, svg.firstChild);
      var over = el('g', { class: 'rv-marks', 'pointer-events': 'none' }, svg);
      var usages = [], framed = false, devs = [];
      refs.forEach(function (r) {
        if (r.indexOf('ov:') === 0) {
          var n = svg.querySelector('.rv-note[data-note="' + r.slice(3) + '"]');
          if (n) n.classList.add('is-on');
        } else if (r.indexOf('u:') === 0) usages.push(r.slice(2));
        else if (r === 'frame') framed = true;
        else devs.push(r);
      });
      if (usages.length) {
        usages.forEach(function (id) {
          var f = shown.frames && shown.frames[id];
          if (!f) return;
          frame(under, over, f.bounds, FRAME, px, 0.2);
          badge(over, f.title, FRAME, f.at[0], f.at[1], f.width, false, px);
          f.roles.forEach(function (role) { badge(over, role.name, FRAME, role.at[0], role.at[1], role.width, true, px); });
        });
      } else if (framed) {
        var b = null, pad = 14 * px;
        devs.forEach(function (d) {
          ((shown.boxes && shown.boxes[d]) || []).forEach(function (q) {
            b = b ? [Math.min(b[0], q[0]), Math.min(b[1], q[1]), Math.max(b[2], q[2]), Math.max(b[3], q[3])] : q.slice();
          });
        });
        if (b) frame(under, over, [b[0] - pad, b[1] - pad, b[2] + pad, b[3] + pad], SELECT, px, 0.12);
      } else {
        devs.forEach(function (d) {
          Array.prototype.forEach.call(svg.querySelectorAll('[data-dev="' + d + '"]'), function (e) { e.classList.add('is-lit'); });
          ((shown.boxes && shown.boxes[d]) || []).forEach(function (q) {
            el('rect', { x: q[0], y: q[1], width: q[2] - q[0], height: q[3] - q[1], fill: SELECT_FILL, stroke: SELECT, 'stroke-width': 1.5,
              'stroke-dasharray': '5 3', 'vector-effect': 'non-scaling-stroke' }, over);
          });
        });
      }
    }
    /* the marks are drawn in pixels of the screen: drawn again when the drawing changes its size */
    if (global.ResizeObserver) new global.ResizeObserver(function () { if (refs) lit(refs); }).observe(stage);
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
      var devs = item.devs.split(' '), first = data[indexOf(item.stmt[0])];
      lit(first && first.r ? first.r.split(/\s+/).concat(['ov:' + item.key]) : devs.concat(['ov:' + item.key]));
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
    /* the caption is as tall as the tallest text it shows, so that picking an item does not move the page under the pointer */
    function fitCaption() {
      var keep = cap.innerHTML, tallest = 0;
      cap.style.minHeight = '0';
      [DEFAULT, SIM].concat(Object.keys(CAPS).map(function (k) { return CAPS[k]; })).forEach(function (t) {
        cap.innerHTML = t;
        tallest = Math.max(tallest, cap.offsetHeight);
      });
      cap.innerHTML = keep;
      cap.style.minHeight = tallest + 'px';
      sizeSim();
    }
    /* a wide screen: the simulator view is as tall as the drawing it replaces */
    var stageH = 0;
    function sizeSim() {
      if (!stage.hidden && stage.offsetHeight) stageH = stage.offsetHeight;
      sim.style.height = stageH && global.matchMedia('(min-width: 1001px)').matches ? stageH + 'px' : '';
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
      sizeSim();
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
    fitCaption();
    global.addEventListener('resize', function () { clearTimeout(fitCaption.t); fitCaption.t = setTimeout(fitCaption, 150); });
    if (document.fonts && document.fonts.ready) document.fonts.ready.then(fitCaption);
  }

  global.RNLOta = { init: init, lines: lines, ITEMS: ITEMS, T: T, TZ: TZ, FILES: FILES };
})(window);
