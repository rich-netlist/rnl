/* RNL introduction page · small schematic renderer
 *
 * Written only for this page's demos: the symbols are textbook drawings made for this page; only their
 * pin positions follow the standard package of the language draft, so that the coordinates in the
 * examples mean the same thing here as in an RNL tool.
 * Scenes are written in RNL coordinates: grid units, x to the right and y upward. SVG draws y downward,
 * so every point is drawn at (x, -y).
 * Orientation codes follow the draft (11.2): R90 turns counterclockwise; MYR90 and MXR90 mirror first.
 */
(function (global) {
  'use strict';

  var NS = 'http://www.w3.org/2000/svg';
  var REDUCE = !!(global.matchMedia && global.matchMedia('(prefers-reduced-motion: reduce)').matches);
  var uid = 0;

  function el(tag, attrs, parent) {
    var e = document.createElementNS(NS, tag);
    if (attrs) {
      for (var k in attrs) {
        if (attrs[k] !== undefined && attrs[k] !== null) e.setAttribute(k, attrs[k]);
      }
    }
    if (parent) parent.appendChild(e);
    return e;
  }

  function r3(v) { return Math.round(v * 1000) / 1000; }
  /* RNL point (y up) to SVG point (y down) */
  function pathOf(pts) {
    var s = '';
    for (var i = 0; i < pts.length; i++) s += (i ? ' L' : 'M') + r3(pts[i][0]) + ' ' + r3(-pts[i][1]);
    return s;
  }
  function ptsAttr(pts) {
    return pts.map(function (p) { return r3(p[0]) + ',' + r3(-p[1]); }).join(' ');
  }
  /* Local symbol points stay in RNL orientation; the group transform flips them */
  function localPts(pts) {
    return pts.map(function (p) { return r3(p[0]) + ',' + r3(p[1]); }).join(' ');
  }
  /* A world box [x0, y0, x1, y1] (y up) as an SVG viewBox */
  function viewBoxOf(b) { return [b[0], -b[3], b[2] - b[0], b[3] - b[1]].map(r3).join(' '); }

  /* Orientation: mirror left-right first when sx is -1, then turn counterclockwise by rot (11.2) */
  var ORIENT = {
    R0: { rot: 0, sx: 1 }, R90: { rot: 90, sx: 1 }, R180: { rot: 180, sx: 1 }, R270: { rot: 270, sx: 1 },
    MY: { rot: 0, sx: -1 }, MX: { rot: 180, sx: -1 }, MYR90: { rot: 90, sx: -1 }, MXR90: { rot: 270, sx: -1 }
  };

  function stateOf(d) {
    var o = ORIENT[d.o || 'R0'];
    return { x: d.x, y: d.y, rot: o.rot, sx: o.sx };
  }

  /* Local point of a symbol to a world point */
  function apply(st, p) {
    var a = st.rot * Math.PI / 180, c = Math.cos(a), s = Math.sin(a);
    var mx = st.sx * p[0], my = p[1];
    return [r3(st.x + mx * c - my * s), r3(st.y + mx * s + my * c)];
  }

  /* The same transform for a symbol group, followed by the flip to SVG's y-down */
  function xformAttr(st) {
    var a = st.rot * Math.PI / 180, c = Math.cos(a), s = Math.sin(a);
    return 'matrix(' + [c * st.sx, -s * st.sx, -s, -c, st.x, -st.y].map(r3).join(' ') + ')';
  }

  /* ---------- Symbols ---------- */
  /* Local coordinates, y up. Pins follow the draft's standard package: MOS gate (-3, 0), NMOS drain
     (0, 2) and source (0, -2), bulk (1, 0); R, L and sources (0, ±3); C (0, ±2); ground, supply and
     port tag on (0, 0). */

  function mos(p, four) {
    var drainUp = p === 'n';
    var prims = [
      ['line', [[-3, 0], [-1.4, 0]]],
      ['line', [[-1.4, -1.1], [-1.4, 1.1]]],
      ['line', [[-0.8, -1.4], [-0.8, 1.4]], 'em'],
      ['line', [[-0.8, 1], [0, 1], [0, 2]]],
      ['line', [[-0.8, -1], [0, -1], [0, -2]]]
    ];
    /* the arrow sits on the source: out of the channel for NMOS, into it for PMOS */
    prims.push(drainUp ?
      ['poly', [[-0.15, -1], [-0.65, -0.72], [-0.65, -1.28]]] :
      ['poly', [[-0.75, 1], [-0.25, 0.72], [-0.25, 1.28]]]);
    var pins = drainUp ? { d: [0, 2], g: [-3, 0], s: [0, -2] } : { d: [0, -2], g: [-3, 0], s: [0, 2] };
    if (four) {
      prims.push(['line', [[-0.8, 0], [1, 0]]]);
      pins.b = [1, 0];
    }
    return {
      pins: pins,
      box: [-3, -2, four ? 1 : 0.4, 2],
      prims: prims,
      label: [0.65, 1.25], value: [0.65, -1.35]
    };
  }

  var SYM = {
    nmos: mos('n'), pmos: mos('p'), nmos4: mos('n', true), pmos4: mos('p', true),
    res: {
      pins: { p: [0, 3], n: [0, -3] },
      box: [-0.7, -3, 0.7, 3],
      prims: [['line', [[0, 3], [0, 2], [0.65, 1.667], [-0.65, 1], [0.65, 0.333], [-0.65, -0.333], [0.65, -1], [-0.65, -1.667], [0, -2], [0, -3]]]],
      label: [1.2, 0.6], value: [1.2, -0.8]
    },
    cap: {
      pins: { p: [0, 2], n: [0, -2] },
      box: [-1.3, -2, 1.3, 2],
      prims: [
        ['line', [[0, 2], [0, 0.35]]], ['line', [[-1.3, 0.35], [1.3, 0.35]], 'em'],
        ['line', [[-1.3, -0.35], [1.3, -0.35]], 'em'], ['line', [[0, -0.35], [0, -2]]]
      ],
      label: [1.9, 0.65], value: [1.9, -0.75]
    },
    ind: {
      pins: { p: [0, 3], n: [0, -3] },
      box: [-0.3, -3, 0.8, 3],
      prims: [['path', 'M0 3 L0 2 A0.5 0.5 0 0 0 0 1 A0.5 0.5 0 0 0 0 0 A0.5 0.5 0 0 0 0 -1 A0.5 0.5 0 0 0 0 -2 L0 -3']],
      label: [1.2, 0.6], value: [1.2, -0.8]
    },
    vsrc: {
      pins: { p: [0, 3], n: [0, -3] },
      box: [-1.5, -3, 1.5, 3],
      prims: [
        ['line', [[0, 3], [0, 1.5]]], ['circle', [0, 0], 1.5], ['line', [[0, -1.5], [0, -3]]],
        ['line', [[-0.4, 0.7], [0.4, 0.7]]], ['line', [[0, 0.3], [0, 1.1]]], ['line', [[-0.4, -0.7], [0.4, -0.7]]]
      ],
      label: [2.1, 0.6], value: [2.1, -0.8]
    },
    vac: {
      pins: { p: [0, 3], n: [0, -3] },
      box: [-1.5, -3, 1.5, 3],
      prims: [
        ['line', [[0, 3], [0, 1.5]]], ['circle', [0, 0], 1.5], ['line', [[0, -1.5], [0, -3]]],
        ['path', 'M-0.85 0 C-0.55 0.95 -0.3 0.95 0 0 S0.55 -0.95 0.85 0']
      ],
      label: [2.1, 0.6], value: [2.1, -0.8]
    },
    isrc: {
      pins: { p: [0, 3], n: [0, -3] },
      box: [-1.5, -3, 1.5, 3],
      prims: [
        ['line', [[0, 3], [0, 1.5]]], ['circle', [0, 0], 1.5], ['line', [[0, -1.5], [0, -3]]],
        ['line', [[0, 0.9], [0, -0.6]]], ['poly', [[0, -0.95], [-0.35, -0.35], [0.35, -0.35]]]
      ],
      label: [2.1, 0.6], value: [2.1, -0.8]
    },
    gnd: {
      pins: { p: [0, 0] },
      box: [-1, -1.5, 1, 0],
      prims: [
        ['line', [[0, 0], [0, -0.8]]], ['line', [[-1, -0.8], [1, -0.8]]],
        ['line', [[-0.6, -1.15], [0.6, -1.15]]], ['line', [[-0.22, -1.5], [0.22, -1.5]]]
      ]
    },
    supply: {
      pins: { p: [0, 0] },
      box: [-1, 0, 1, 1],
      prims: [['line', [[0, 0], [0, 0.8]]], ['line', [[-1, 0.8], [1, 0.8]], 'em']],
      label: [0, 1.75], labelAnchor: 'middle'
    },
    port: {
      pins: { p: [0, 0] },
      box: [-3, -0.6, 0, 0.6],
      prims: [['poly', [[0, 0], [-0.7, 0.6], [-3, 0.6], [-3, -0.6], [-0.7, -0.6]], 'bg']],
      label: [-3.35, 0]
    },
    opamp: {
      pins: { inp: [-4, -1], inn: [-4, 1], out: [4, 0] },
      box: [-4, -2.5, 4, 2.5],
      prims: [
        ['poly', [[-3, 2.5], [3, 0], [-3, -2.5]], 'bg'],
        ['line', [[-4, -1], [-3, -1]]], ['line', [[-4, 1], [-3, 1]]], ['line', [[3, 0], [4, 0]]],
        ['line', [[-2.6, -1], [-1.9, -1]]], ['line', [[-2.25, -1.35], [-2.25, -0.65]]],
        ['line', [[-2.6, 1], [-1.9, 1]]]
      ],
      label: [0, 2.2], labelAnchor: 'middle'
    },
    /* The block symbol $OTA_SYM of the testbench example: pins sit on the triangle's edges */
    block: {
      pins: { inp: [-5, 2], inn: [-5, -2], out: [5, 0], vdd: [-1, 3], vss: [1, -2], vbias: [-3, -4] },
      box: [-5, -5, 5, 5],
      prims: [
        ['poly', [[-5, 5], [5, 0], [-5, -5]], 'bg'],
        ['line', [[-4.3, 2], [-3.5, 2]]], ['line', [[-3.9, 2.4], [-3.9, 1.6]]],
        ['line', [[-4.3, -2], [-3.5, -2]]]
      ],
      label: [-1.6, 0], labelAnchor: 'middle'
    },
    nc: {
      pins: { p: [0, 0] },
      box: [-0.5, -0.5, 0.5, 0.5],
      prims: [['line', [[-0.5, -0.5], [0.5, 0.5]]], ['line', [[-0.5, 0.5], [0.5, -0.5]]]]
    },
    /* NPN as in the standard package: collector (0, 2), base (-3, 0), emitter (0, -2) */
    npn: {
      pins: { c: [0, 2], b: [-3, 0], e: [0, -2] },
      box: [-3, -2, 0.4, 2],
      prims: [
        ['line', [[-3, 0], [-1, 0]]], ['line', [[-1, -1], [-1, 1]], 'em'],
        ['line', [[-1, 0.5], [0, 1.1], [0, 2]]], ['line', [[-1, -0.5], [-0.47, -0.82]]], ['line', [[0, -1.1], [0, -2]]],
        ['poly', [[0, -1.1], [-0.34, -0.6], [-0.6, -1.03]]]
      ],
      label: [0.6, 0.65], value: [0.6, -0.75]
    },
    /* The block symbol of the inverter subcircuit in the tour: input a, output y */
    inv: {
      pins: { a: [-3, 0], y: [3, 0] },
      box: [-3, -1.5, 3, 1.5],
      prims: [
        ['poly', [[-1.8, 1.5], [1.4, 0], [-1.8, -1.5]], 'bg'], ['circle', [1.8, 0], 0.4],
        ['line', [[-3, 0], [-1.8, 0]]], ['line', [[2.2, 0], [3, 0]]]
      ],
      label: [-0.6, 2.2], labelAnchor: 'middle'
    }
  };

  function drawSymbol(g, sym) {
    sym.prims.forEach(function (pr) {
      var t = pr[0];
      if (t === 'line') el('polyline', { points: localPts(pr[1]), class: 'sp' + (pr[2] === 'em' ? ' em' : '') }, g);
      else if (t === 'poly') el('polygon', { points: localPts(pr[1]), class: pr[2] === 'bg' ? 'sp bg' : 'sp fill' }, g);
      else if (t === 'circle') el('circle', { cx: pr[1][0], cy: pr[1][1], r: pr[2], class: 'sp' }, g);
      else if (t === 'path') el('path', { d: pr[1], class: 'sp' }, g);
    });
  }

  function anchorFor(dx, dy) {
    if (Math.abs(dx) > 0.05 && Math.abs(dx) >= 0.5 * Math.abs(dy)) return dx > 0 ? 'start' : 'end';
    return 'middle';
  }

  /* ---------- Animation ---------- */

  function ease(t) { return t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2; }
  function lerp(a, b, t) { return a + (b - a) * t; }
  function lerpState(a, b, t) {
    var dr = ((b.rot - a.rot) % 360 + 540) % 360 - 180;
    return { x: lerp(a.x, b.x, t), y: lerp(a.y, b.y, t), rot: a.rot + dr * t, sx: lerp(a.sx, b.sx, t) };
  }

  /* Run on the next frame; when the page is not painting (requestAnimationFrame suspended), a timer takes over so the final state is always reached */
  function nextFrame(fn) {
    var ran = false;
    function run() { if (!ran) { ran = true; fn(); } }
    if (global.requestAnimationFrame) global.requestAnimationFrame(run);
    setTimeout(run, 40);
  }
  function afterPaint(fn) { nextFrame(function () { nextFrame(fn); }); }

  function tween(ms, frame, done) {
    var stopped = false;
    if (REDUCE || ms <= 0) {
      frame(1);
      if (done) done();
      return { stop: function () {} };
    }
    var start = performance.now();
    function step() {
      if (stopped) return;
      var t = Math.min(1, (performance.now() - start) / ms);
      frame(ease(t));
      if (t < 1) nextFrame(step);
      else if (done) done();
    }
    nextFrame(step);
    return { stop: function () { stopped = true; } };
  }

  function polyLen(pts) {
    var L = 0;
    for (var i = 1; i < pts.length; i++) L += Math.hypot(pts[i][0] - pts[i - 1][0], pts[i][1] - pts[i - 1][1]);
    return L;
  }

  function drawIn(path, pts, delay) {
    var L = polyLen(pts);
    if (L <= 0) return;
    path.style.strokeDasharray = L + ' ' + L;
    path.style.strokeDashoffset = L;
    path.getBoundingClientRect();
    setTimeout(function () {
      path.style.transition = 'stroke-dashoffset .55s ease-out';
      path.style.strokeDashoffset = '0';
      setTimeout(function () {
        path.style.transition = '';
        path.style.strokeDasharray = '';
        path.style.strokeDashoffset = '';
      }, 650);
    }, delay || 0);
  }

  /* ---------- Segment intersections (for counting crossings) ---------- */

  function orient(a, b, c) { return (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]); }
  function onSeg(a, b, p) {
    return Math.min(a[0], b[0]) - 1e-6 <= p[0] && p[0] <= Math.max(a[0], b[0]) + 1e-6 &&
      Math.min(a[1], b[1]) - 1e-6 <= p[1] && p[1] <= Math.max(a[1], b[1]) + 1e-6;
  }
  function properCross(s, t) {
    var a = s.a, b = s.b, c = t.a, d = t.b;
    var o1 = orient(a, b, c), o2 = orient(a, b, d), o3 = orient(c, d, a), o4 = orient(c, d, b);
    var E = 1e-7;
    if (Math.abs(o1) < E && onSeg(a, b, c)) return false;
    if (Math.abs(o2) < E && onSeg(a, b, d)) return false;
    if (Math.abs(o3) < E && onSeg(c, d, a)) return false;
    if (Math.abs(o4) < E && onSeg(c, d, b)) return false;
    return (o1 > 0) !== (o2 > 0) && (o3 > 0) !== (o4 > 0);
  }

  /* ---------- View ---------- */

  /* opts.box is the visible world box [x0, y0, x1, y1] in RNL coordinates */
  function View(svg, opts) {
    opts = opts || {};
    this.svg = svg;
    this.opts = opts;
    this.id = ++uid;
    var box = opts.box || [0, 0, 40, 30];
    this.box = box;
    var vb = [box[0], -box[3], box[2] - box[0], box[3] - box[1]];
    svg.setAttribute('viewBox', viewBoxOf(box));
    svg.classList.add('sc');
    var defs = el('defs', null, svg);
    var pat = el('pattern', { id: 'scdots' + this.id, x: -0.5, y: -0.5, width: 1, height: 1, patternUnits: 'userSpaceOnUse' }, defs);
    el('circle', { cx: 0.5, cy: 0.5, r: 0.075, class: 'sc-griddot' }, pat);
    if (opts.grid !== false) {
      el('rect', { x: vb[0], y: vb[1], width: vb[2], height: vb[3], fill: 'url(#scdots' + this.id + ')', class: 'sc-grid' }, svg);
    }
    this.L = {};
    var self = this;
    ['back', 'route', 'devs', 'labels', 'front'].forEach(function (k) {
      self.L[k] = el('g', { class: 'sc-l-' + k }, svg);
    });
    this.devs = {};
    this.cur = {};
    this.scene = null;
    this.routeG = null;
    this.wireEls = [];
    this.dotEls = [];
    this.netEls = [];
    this.ov = {};
    this.last = { wires: [], dots: [], labels: [] };
    this.tw = null;
    if (opts.onHover) {
      svg.addEventListener('mouseover', function (e) {
        var t = e.target.closest ? e.target.closest('[data-ref]') : null;
        opts.onHover(t ? t.getAttribute('data-ref') : null);
      });
      svg.addEventListener('mouseleave', function () { opts.onHover(null); });
    }
  }

  /* P('M1.d') is the world point of a pin; S(id) the current state of a device; B(id) its world box */
  View.prototype._P = function () {
    var self = this;
    return function (ref) {
      var i = ref.indexOf('.');
      var id = ref.slice(0, i), pin = ref.slice(i + 1);
      var d = self.scene.devices[id];
      if (!d) throw new Error('unknown device ' + id);
      var st = self.cur[id] || stateOf(d);
      var p = SYM[d.sym].pins[pin];
      if (!p) throw new Error('unknown pin ' + ref);
      return apply(st, p);
    };
  };

  View.prototype._S = function () {
    var self = this;
    return function (id) { return self.cur[id] || stateOf(self.scene.devices[id]); };
  };

  View.prototype._B = function () {
    var self = this;
    return function (id) {
      var d = self.scene.devices[id];
      var st = self.cur[id] || stateOf(d);
      var b = SYM[d.sym].box;
      var c = [apply(st, [b[0], b[1]]), apply(st, [b[2], b[1]]), apply(st, [b[0], b[3]]), apply(st, [b[2], b[3]])];
      return [
        Math.min(c[0][0], c[1][0], c[2][0], c[3][0]), Math.min(c[0][1], c[1][1], c[2][1], c[3][1]),
        Math.max(c[0][0], c[1][0], c[2][0], c[3][0]), Math.max(c[0][1], c[1][1], c[2][1], c[3][1])
      ];
    };
  };

  View.prototype._ensureDev = function (id, spec) {
    var d = this.devs[id];
    if (d && d.sym === spec.sym) {
      d.spec = spec;
      var keep = (d.root.classList.contains('is-gone') ? ' is-gone' : '') + (d.root.classList.contains('is-hl') ? ' is-hl' : '');
      d.root.setAttribute('class', 'sc-dev' + (spec.virtual ? ' is-virt' : '') + (spec.cls ? ' ' + spec.cls : '') + keep);
      this._setTexts(d);
      return false;
    }
    if (d) this._dropDev(id, true);
    var sym = SYM[spec.sym];
    var root = el('g', { class: 'sc-dev is-gone' + (spec.virtual ? ' is-virt' : '') + (spec.cls ? ' ' + spec.cls : ''), 'data-ref': id }, this.L.devs);
    var b = sym.box;
    el('rect', { class: 'sc-hit', x: b[0] - 0.5, y: b[1] - 0.5, width: b[2] - b[0] + 1, height: b[3] - b[1] + 1 }, root);
    drawSymbol(root, sym);
    var lab = el('text', { class: 'sc-label is-gone', 'data-ref': id }, this.L.labels);
    var val = el('text', { class: 'sc-value is-gone', 'data-ref': id }, this.L.labels);
    d = this.devs[id] = { id: id, sym: spec.sym, spec: spec, root: root, lab: lab, val: val };
    this._setTexts(d);
    return true;
  };

  View.prototype._setTexts = function (d) {
    var spec = d.spec, sym = SYM[spec.sym];
    var text = spec.label !== undefined ? spec.label : (sym.text || '');
    d.lab.textContent = text;
    d.val.textContent = spec.value || '';
    var extra = (d.root.classList.contains('is-gone') ? ' is-gone' : '') + (d.lab.classList.contains('is-hl') ? ' is-hl' : '');
    d.lab.setAttribute('class', 'sc-label' + (spec.cls ? ' ' + spec.cls : '') + (spec.labelCls ? ' ' + spec.labelCls : '') + extra);
    d.val.setAttribute('class', 'sc-value' + (spec.valueOpt ? ' opt' : '') + extra);
  };

  View.prototype._dropDev = function (id, now) {
    var d = this.devs[id];
    if (!d) return;
    delete this.devs[id];
    delete this.cur[id];
    [d.root, d.lab, d.val].forEach(function (e) {
      e.classList.add('is-gone');
      if (now) e.remove(); else setTimeout(function () { e.remove(); }, 400);
    });
  };

  /* off is a world offset [dx, dy, anchor]; localDefault a point in the symbol, moved with its orientation */
  View.prototype._placeText = function (t, st, off, forced, localDefault) {
    if (!t.textContent) return;
    var x, y, anchor;
    if (off) {
      x = st.x + off[0]; y = st.y + off[1];
      anchor = off[2] || anchorFor(off[0], off[1]);
    } else if (localDefault) {
      var p = apply(st, localDefault);
      x = p[0]; y = p[1];
      anchor = forced || anchorFor(p[0] - st.x, p[1] - st.y);
    } else return;
    t.setAttribute('x', r3(x));
    t.setAttribute('y', r3(-y));
    t.setAttribute('text-anchor', anchor);
  };

  View.prototype._applyDevs = function () {
    for (var id in this.devs) {
      var d = this.devs[id];
      var st = this.cur[id];
      if (!st) continue;
      d.root.setAttribute('transform', xformAttr(st));
      var sym = SYM[d.spec.sym];
      this._placeText(d.lab, st, d.spec.lab, sym.labelAnchor, sym.label);
      this._placeText(d.val, st, d.spec.val, null, sym.value);
    }
  };

  View.prototype._retireRoute = function () {
    var g = this.routeG;
    if (!g) return;
    this.routeG = null;
    if (REDUCE) { g.remove(); return; }
    g.classList.add('is-retiring');
    setTimeout(function () { g.remove(); }, 260);
  };

  View.prototype._route = function (rebuild, animateNew) {
    var scene = this.scene;
    var out = scene.route ? scene.route(this._P(), this._S()) : { wires: [], dots: [], labels: [] };
    out.wires = out.wires || [];
    out.dots = out.dots || [];
    out.labels = out.labels || [];
    var prevKeys = this.keys || {};
    this.last = out;
    var self = this;
    if (rebuild || !this.routeG) {
      if (this.routeG) this._retireRoute();
      var g = el('g', { class: 'sc-route' }, this.L.route);
      this.routeG = g;
      this.wireEls = [];
      this.dotEls = [];
      this.netEls = [];
      var keys = {};
      var delay = 0;
      out.wires.forEach(function (w) {
        var key = w.key || (w.net + '|' + pathOf(w.pts));
        keys[key] = 1;
        var p = el('path', {
          d: pathOf(w.pts),
          class: (w.air ? 'sc-air' : 'sc-wire') + (w.cls ? ' ' + w.cls : ''),
          'data-ref': w.ref || ('net:' + w.net)
        }, g);
        if (animateNew && !REDUCE && !prevKeys[key]) {
          drawIn(p, w.pts, delay);
          delay += 45;
        }
        self.wireEls.push(p);
      });
      out.dots.forEach(function (d) {
        var c = el('circle', { cx: d[0], cy: -d[1], r: 0.3, class: 'sc-dot' + (d[3] ? ' ' + d[3] : ''), 'data-ref': 'net:' + (d[2] || '') }, g);
        if (animateNew && !REDUCE && !prevKeys['dot:' + d[0] + ',' + d[1]]) {
          c.style.opacity = '0';
          setTimeout(function () { c.style.transition = 'opacity .3s'; c.style.opacity = ''; }, delay + 350);
        }
        keys['dot:' + d[0] + ',' + d[1]] = 1;
        self.dotEls.push(c);
      });
      out.labels.forEach(function (l) {
        var t = el('text', { class: 'sc-netlabel' + (l.opt ? ' opt' : '') + (l.cls ? ' ' + l.cls : ''), 'data-ref': 'net:' + l.net, x: l.at[0], y: -l.at[1], 'text-anchor': l.anchor || 'middle' }, g);
        t.textContent = l.text;
        /* a subscript, as TEXT writes it with _{…} */
        if (l.sub) el('tspan', { class: 'sub', dy: '0.3', 'font-size': '72%' }, t).textContent = l.sub;
        self.netEls.push(t);
      });
      this.keys = keys;
    } else {
      out.wires.forEach(function (w, i) { if (self.wireEls[i]) self.wireEls[i].setAttribute('d', pathOf(w.pts)); });
      out.dots.forEach(function (d, i) {
        var c = self.dotEls[i];
        if (c) { c.setAttribute('cx', r3(d[0])); c.setAttribute('cy', r3(-d[1])); }
      });
      out.labels.forEach(function (l, i) {
        var t = self.netEls[i];
        if (t) { t.setAttribute('x', r3(l.at[0])); t.setAttribute('y', r3(-l.at[1])); }
      });
    }
  };

  /* ---------- Annotation layer ---------- */
  /* Annotation specs are in world coordinates too; they are flipped when drawn */

  function arrowHead(g, a, b, size) {
    var dx = b[0] - a[0], dy = b[1] - a[1], L = Math.hypot(dx, dy) || 1;
    var ux = dx / L, uy = dy / L, s = size || 0.55;
    el('polygon', {
      class: 'o-fill',
      points: ptsAttr([b, [b[0] - ux * s * 1.6 + uy * s * 0.8, b[1] - uy * s * 1.6 - ux * s * 0.8], [b[0] - ux * s * 1.6 - uy * s * 0.8, b[1] - uy * s * 1.6 + ux * s * 0.8]])
    }, g);
  }

  function textWidth(text, mono) {
    var fs = mono ? 0.95 : 1;
    var w = 0;
    for (var i = 0; i < text.length; i++) {
      /* CJK characters and full-width forms are about one em wide */
      var c = text.charCodeAt(i);
      var wide = (c >= 0x3000 && c <= 0x9fff) || (c >= 0xff00 && c <= 0xffef);
      w += wide ? fs : fs * (mono ? 0.6 : 0.56);
    }
    return w + 0.5;
  }

  function textBox(g, x, y, text, anchor, mono) {
    var w = textWidth(text, mono);
    var x0 = anchor === 'end' ? x - w + 0.25 : anchor === 'middle' ? x - w / 2 : x - 0.25;
    el('rect', { class: 'o-bg', x: r3(x0), y: r3(-y - 0.72), width: r3(w), height: 1.44, rx: 0.3 }, g);
    var t = el('text', { x: r3(x), y: r3(-y), 'text-anchor': anchor || 'start', class: mono ? 'o-mono' : '' }, g);
    t.textContent = text;
    return t;
  }

  View.prototype._drawOverlay = function (g, sp, B) {
    while (g.firstChild) g.removeChild(g.firstChild);
    var k = sp.kind;
    if (k === 'box') {
      var bb = [Infinity, Infinity, -Infinity, -Infinity];
      sp.ids.forEach(function (id) {
        var b = B(id);
        bb = [Math.min(bb[0], b[0]), Math.min(bb[1], b[1]), Math.max(bb[2], b[2]), Math.max(bb[3], b[3])];
      });
      var p = sp.pad === undefined ? 1 : sp.pad;
      var x0 = bb[0] - p, y0 = bb[1] - p, x1 = bb[2] + p, y1 = bb[3] + p;
      el('rect', { class: 'o-tint', x: r3(x0), y: r3(-y1), width: r3(x1 - x0), height: r3(y1 - y0), rx: 0.8 }, g);
      el('rect', { class: 'o-dash', x: r3(x0), y: r3(-y1), width: r3(x1 - x0), height: r3(y1 - y0), rx: 0.8 }, g);
      if (sp.label) {
        /* labelAt is a fraction of the box from its lower left corner */
        var lp = sp.labelAt ? [x0 + sp.labelAt[0] * (x1 - x0), y0 + sp.labelAt[1] * (y1 - y0)] : [x0 + 0.6, y1 - 0.9];
        textBox(g, lp[0], lp[1], sp.label, sp.anchor || 'start');
      }
    } else if (k === 'vline') {
      el('line', { class: 'o-dash', x1: r3(sp.x), y1: r3(-sp.y0), x2: r3(sp.x), y2: r3(-sp.y1) }, g);
      if (sp.label) textBox(g, sp.x, sp.y1 + 0.45, sp.label, 'middle', true);
    } else if (k === 'arrow') {
      el('polyline', { class: 'o-line', points: ptsAttr(sp.pts) }, g);
      var n = sp.pts.length;
      arrowHead(g, sp.pts[n - 2], sp.pts[n - 1], sp.head);
      if (sp.label) textBox(g, sp.labelAt[0], sp.labelAt[1], sp.label, sp.anchor || 'start', sp.mono);
    } else if (k === 'ticks') {
      /* Equal-length marks: a short tick on each of the two segments, with the length below it */
      var y = sp.y;
      sp.segs.forEach(function (s, i) {
        var m = (s[0] + s[1]) / 2;
        el('line', { class: 'o-line', x1: r3(m - 0.2), y1: r3(-y + 0.45), x2: r3(m + 0.2), y2: r3(-y - 0.45) }, g);
        if (sp.labels) textBox(g, m, y - 1.2, sp.labels[i], 'middle', true);
      });
      if (sp.mid) textBox(g, sp.mid[0], sp.mid[1], sp.midText, 'middle', true);
    } else if (k === 'tag') {
      textBox(g, sp.at[0], sp.at[1], sp.text, sp.anchor || 'start', true);
    } else if (k === 'ring') {
      el('circle', { class: 'o-line', cx: r3(sp.at[0]), cy: r3(-sp.at[1]), r: sp.r || 1 }, g);
    } else if (k === 'text') {
      textBox(g, sp.at[0], sp.at[1], sp.text, sp.anchor || 'start', sp.mono);
    } else if (k === 'dash') {
      el('polyline', { class: 'o-dash', points: ptsAttr(sp.pts) }, g);
    } else if (k === 'badge') {
      el('circle', { class: 'o-badge', cx: r3(sp.at[0]), cy: r3(-sp.at[1]), r: 0.75 }, g);
      el('text', { class: 'o-badge-t', x: r3(sp.at[0]), y: r3(-sp.at[1]), 'text-anchor': 'middle' }, g).textContent = sp.text;
    }
  };

  View.prototype._overlays = function () {
    var scene = this.scene;
    var specs = scene.overlays ? scene.overlays(this._P(), this._S(), this._B(), this.mode) : [];
    var seen = {};
    var B = this._B();
    var self = this;
    specs.forEach(function (sp) {
      seen[sp.id] = 1;
      var o = self.ov[sp.id];
      var cls = 'ov ov-' + sp.kind + (sp.tone ? ' tone-' + sp.tone : '');
      if (!o || o.layer !== (sp.layer || 'front')) {
        if (o) o.g.remove();
        var g = el('g', { class: cls + ' is-gone', 'data-ref': 'ov:' + (sp.ref || sp.id) }, sp.layer === 'back' ? self.L.back : self.L.front);
        o = self.ov[sp.id] = { g: g, layer: sp.layer || 'front', cls: cls };
        afterPaint(function () { g.classList.remove('is-gone'); });
      } else if (o.cls !== cls) {
        var gone = o.g.classList.contains('is-gone');
        var hl = o.g.classList.contains('is-hl');
        o.g.setAttribute('class', cls + (gone ? ' is-gone' : '') + (hl ? ' is-hl' : ''));
        o.cls = cls;
      }
      self._drawOverlay(o.g, sp, B);
    });
    for (var id in this.ov) {
      if (!seen[id]) {
        var old = this.ov[id];
        delete this.ov[id];
        old.g.classList.add('is-gone');
        (function (g) { setTimeout(function () { g.remove(); }, 380); })(old.g);
      }
    }
  };

  View.prototype.setScene = function (scene, o) {
    o = o || {};
    var prev = this.scene;
    var animate = !!o.animate && !REDUCE && !!prev;
    var dur = o.duration || 950;
    this.scene = scene;
    if (this.tw) { this.tw.stop(); this.tw = null; }
    var target = {}, id;
    var fresh = [];
    for (id in scene.devices) {
      target[id] = stateOf(scene.devices[id]);
      if (this._ensureDev(id, scene.devices[id])) fresh.push(id);
    }
    for (id in this.devs) if (!scene.devices[id]) this._dropDev(id, false);
    var from = {};
    for (id in target) from[id] = this.cur[id] && fresh.indexOf(id) < 0 ? this.cur[id] : target[id];
    var sameRoute = !!prev && prev.rk === scene.rk;
    var self = this;
    var reveal = function () {
      afterPaint(function () {
        fresh.forEach(function (fid) {
          var d = self.devs[fid];
          if (!d) return;
          d.root.classList.remove('is-gone');
          d.lab.classList.remove('is-gone');
          d.val.classList.remove('is-gone');
        });
      });
    };
    if (!animate) {
      for (id in target) this.cur[id] = target[id];
      this._applyDevs();
      this._route(true, !!o.animate);
      this._overlays();
      reveal();
      if (o.done) o.done();
      return;
    }
    if (!sameRoute) this._retireRoute();
    reveal();
    this.tw = tween(dur, function (t) {
      for (var k in target) self.cur[k] = lerpState(from[k], target[k], t);
      self._applyDevs();
      if (sameRoute) self._route(false, false);
      self._overlays();
    }, function () {
      self.tw = null;
      for (var k in target) self.cur[k] = target[k];
      self._applyDevs();
      self._route(!sameRoute, true);
      self._overlays();
      if (o.done) o.done();
    });
  };

  /* Recompute only the annotations (for example after switching the display mode) */
  View.prototype.refresh = function () {
    if (this.scene) this._overlays();
  };

  View.prototype.setMode = function (mode) {
    this.mode = mode;
    this.refresh();
  };

  View.prototype.highlight = function (refs) {
    var svg = this.svg;
    var on = svg.querySelectorAll('.is-hl');
    for (var i = 0; i < on.length; i++) on[i].classList.remove('is-hl');
    var any = false;
    var all = [];
    (refs || []).forEach(function (r) {
      all.push(r);
      /* A device's annotations (such as coordinate tags) highlight together with the device */
      if (r.indexOf(':') < 0) all.push('ov:' + r);
    });
    all.forEach(function (r) {
      var els = svg.querySelectorAll('[data-ref="' + r.replace(/"/g, '\\"') + '"]');
      for (var j = 0; j < els.length; j++) { els[j].classList.add('is-hl'); any = true; }
    });
    svg.classList.toggle('has-hl', any);
  };

  View.prototype.metrics = function () {
    var segs = [], bends = 0, wires = 0, air = 0;
    (this.last.wires || []).forEach(function (w) {
      if (w.air) air++; else wires++;
      for (var i = 1; i < w.pts.length; i++) segs.push({ a: w.pts[i - 1], b: w.pts[i], net: w.net });
      if (!w.air) bends += Math.max(0, w.pts.length - 2);
    });
    var cross = 0;
    for (var i = 0; i < segs.length; i++) {
      for (var j = i + 1; j < segs.length; j++) {
        if (segs[i].net !== segs[j].net && properCross(segs[i], segs[j])) cross++;
      }
    }
    return { crossings: cross, bends: bends, wires: wires, air: air };
  };

  global.RNLSchem = {
    View: View, SYM: SYM, ORIENT: ORIENT, apply: apply, stateOf: stateOf, tween: tween, el: el, REDUCE: REDUCE,
    pathOf: pathOf, ptsAttr: ptsAttr, localPts: localPts, viewBoxOf: viewBoxOf, xformAttr: xformAttr, drawSymbol: drawSymbol
  };
})(window);
