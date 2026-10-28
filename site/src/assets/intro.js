/* RNL introduction page · interactions of the main page */
(function () {
  'use strict';

  var S = window.RNLSchem, SC = window.RNLScenes, C = window.RNLCode, PG = window.RNLPage;
  var $ = PG.$, $$ = PG.$$, icon = PG.icon, onVisible = PG.onVisible, setPressed = PG.setPressed, fill = PG.fill, debounce = PG.debounce;
  var REDUCE = S.REDUCE;

  /* ================= Hero ================= */

  function initLogoViewer() {
    var button = $('#hero-logo'), panel = $('#hero-logo-panel'), closeButton = $('#hero-logo-close'), brand = $('#hero-brand');
    if (!button || !panel || !closeButton || !brand) return;
    var frame = null, viewerOrigin = null, ready = false, needsFit = false;
    var mq = window.matchMedia('(prefers-color-scheme: dark)');
    function theme() {
      var chosen = document.documentElement.getAttribute('data-theme');
      return chosen === 'dark' || chosen === 'light' ? chosen : mq.matches ? 'dark' : 'light';
    }
    function syncTheme() {
      if (frame && ready) frame.contentWindow.postMessage({ rnl: 1, type: 'background', value: theme() }, viewerOrigin);
    }
    function fitIfNeeded() {
      if (!frame || !ready || panel.hidden || !needsFit) return;
      frame.contentWindow.postMessage({ rnl: 1, type: 'fit' }, viewerOrigin);
      needsFit = false;
    }
    function close() {
      panel.hidden = true;
      brand.classList.remove('is-viewing');
      button.setAttribute('aria-expanded', 'false');
      // Let the hidden iframe release focus before returning it to the restored wordmark.
      requestAnimationFrame(function () { if (panel.hidden) button.focus({ preventScroll: true }); });
    }
    function escape(e) {
      if (e.key !== 'Escape' || panel.hidden) return;
      e.preventDefault();
      e.stopPropagation();
      close();
    }
    button.addEventListener('click', function () {
      if (!panel.hidden) { close(); return; }
      panel.hidden = false;
      brand.classList.add('is-viewing');
      button.setAttribute('aria-expanded', 'true');
      if (!frame) {
        var url = new URL(panel.getAttribute('data-viewer-url'), document.baseURI);
        url.search = new URLSearchParams({ embed: '1', url: 'samples/brand/rnl-logo.sp', background: theme(), grid: 'off' });
        if (location.origin !== 'null') url.searchParams.set('origin', location.origin);
        viewerOrigin = url.origin;
        frame = document.createElement('iframe');
        frame.title = panel.getAttribute('aria-label');
        frame.addEventListener('load', function () {
          // The published homepage and viewer are same-origin. Capture Escape inside the iframe
          // too; keyboard events do not bubble between browsing contexts.
          var child = frame.contentDocument;
          if (child) child.addEventListener('keydown', escape, true);
        });
        frame.src = url.href;
        panel.appendChild(frame);
      } else { syncTheme(); fitIfNeeded(); }
      closeButton.focus({ preventScroll: true });
    });
    closeButton.addEventListener('click', close);
    document.addEventListener('keydown', escape, true);
    window.addEventListener('message', function (e) {
      if (!frame || e.source !== frame.contentWindow || e.origin !== viewerOrigin || e.data?.rnl !== 1) return;
      if (e.data.type === 'ready') { ready = true; syncTheme(); fitIfNeeded(); }
    });
    new MutationObserver(syncTheme).observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] });
    new ResizeObserver(function () {
      needsFit = true;
      fitIfNeeded();
    }).observe(brand);
    mq.addEventListener('change', syncTheme);
  }

  function drawBode(svg) {
    var el = S.el, W = 340, H = 220, m = { l: 40, r: 12, t: 24, b: 32 };
    svg.setAttribute('viewBox', '0 0 ' + W + ' ' + H);
    svg.classList.add('bode');
    var fc = 1 / (2 * Math.PI * 10e3 * 10e-9);
    function X(f) { return m.l + (Math.log10(f) - 1) / 5 * (W - m.l - m.r); }
    function Y(db) { return m.t + (-db / 60) * (H - m.t - m.b); }
    var g = el('g', null, svg);
    var names = ['10', '100', '1k', '10k', '100k', '1M'];
    for (var d = 1; d <= 6; d++) {
      if (d < 6) {
        for (var k = 2; k <= 9; k++) {
          var xm = X(k * Math.pow(10, d));
          el('line', { class: 'bd-minor', x1: xm, x2: xm, y1: m.t, y2: H - m.b }, g);
        }
      }
      var x = X(Math.pow(10, d));
      el('line', { class: 'bd-grid', x1: x, x2: x, y1: m.t, y2: H - m.b }, g);
      el('text', { class: 'bd-t', x: x, y: H - m.b + 15, 'text-anchor': 'middle' }, g).textContent = names[d - 1];
    }
    [0, -20, -40, -60].forEach(function (db) {
      var y = Y(db);
      el('line', { class: 'bd-grid', x1: m.l, x2: W - m.r, y1: y, y2: y }, g);
      el('text', { class: 'bd-t', x: m.l - 6, y: y + 3.5, 'text-anchor': 'end' }, g).textContent = db;
    });
    el('text', { class: 'bd-t', x: m.l - 6, y: m.t - 10, 'text-anchor': 'end' }, g).textContent = 'dB';
    el('text', { class: 'bd-t', x: W - m.r, y: H - 3, 'text-anchor': 'end' }, g).textContent = 'Hz';
    var pts = [];
    for (var i = 0; i <= 240; i++) {
      var f = Math.pow(10, 1 + 5 * i / 240);
      pts.push([X(f), Y(-10 * Math.log10(1 + Math.pow(f / fc, 2)))]);
    }
    el('path', { class: 'bd-curve', d: 'M' + pts.map(function (p) { return p[0].toFixed(2) + ' ' + p[1].toFixed(2); }).join(' L') }, g);
    var xc = X(fc), yc = Y(-3.01);
    el('line', { class: 'bd-mark', x1: xc, x2: xc, y1: yc, y2: H - m.b }, g);
    el('circle', { class: 'bd-pt', cx: xc, cy: yc, r: 3.4 }, g);
    var lab = el('text', { class: 'bd-lab', x: xc + 8, y: yc - 7 }, g);
    lab.appendChild(document.createTextNode('f'));
    var sub = el('tspan', { 'baseline-shift': 'sub', 'font-size': '75%' }, lab);
    sub.textContent = 'c';
    lab.appendChild(document.createTextNode(' = 1.59 kHz, −3 dB'));
    el('text', { class: 'bd-t', x: m.l + 4, y: H - m.b - 6 }, g).textContent = '|V(out)|';
    return svg;
  }

  function initHero() {
    var code = $('#hero-code');
    if (!code) return;
    var lines = SC.HERO_LINES;
    var rows = C.render(code, lines);
    rows.forEach(function (r) { r.setAttribute('tabindex', '0'); r.classList.add('is-link'); });
    var view = new S.View($('#hero-svg'), { box: SC.RC.box });
    view.setScene(SC.RC);
    var plot = drawBode($('#hero-plot'));
    var note = $('#hero-note-text'), simChip = $('#hero-sim-chip'), viewChip = $('#hero-view-chip');
    var IDLE = '<en>One file, two readers, each taking what it needs. Hover over any line of the file; left alone, the demo steps through the lines by itself.</en>';
    var SIM = { use: ['chip chip-ok', '<en>Reads this line</en>'], skip: ['chip chip-mute', '<en>Skips it as a comment</en>'], title: ['chip chip-mute', '<en>Treats it as the title</en>'] };
    var VIEW = { draw: ['chip chip-rnl', '<en>Draws from it</en>'], meta: ['chip chip-rnl', '<en>Record boundary</en>'], skip: ['chip chip-mute', '<en>Does not need it</en>'] };

    function show(i) {
      rows.forEach(function (r) { r.classList.remove('is-hot'); });
      if (i < 0) {
        view.setMode(null);
        view.highlight(null);
        plot.classList.remove('is-hot');
        note.innerHTML = IDLE;
        simChip.className = 'chip'; simChip.textContent = '<en>Netlist only</en>';
        viewChip.className = 'chip chip-rnl'; viewChip.textContent = '<en>Netlist + RNL</en>';
        return;
      }
      var l = lines[i];
      rows[i].classList.add('is-hot');
      view.setMode(l.mode || null);
      view.highlight(l.r ? l.r.split(' ') : null);
      plot.classList.toggle('is-hot', l.sim === 'use' && (!!l.plot || !!l.r));
      note.innerHTML = l.note;
      simChip.className = SIM[l.sim][0]; simChip.textContent = SIM[l.sim][1];
      viewChip.className = VIEW[l.view][0]; viewChip.textContent = VIEW[l.view][1];
    }

    var idx = -1, hover = false, visible = false;
    function step() {
      if (hover || !visible || document.hidden) return;
      idx = (idx + 1) % lines.length;
      show(idx);
    }
    if (!REDUCE) setInterval(step, 2600);
    onVisible($('#hero-demo'), function (v) { visible = v; }, 0.3);
    function pick(e) {
      var r = e.target.closest ? e.target.closest('.cl') : null;
      if (!r) return;
      hover = true;
      idx = +r.getAttribute('data-i');
      show(idx);
    }
    code.addEventListener('mouseover', pick);
    code.addEventListener('focusin', pick);
    code.addEventListener('mouseleave', function () { hover = false; });
    code.addEventListener('focusout', function () { hover = false; });
    show(-1);

    var demo = $('#hero-demo');
    var btns = $$('#hero-mode button');
    btns.forEach(function (b) {
      b.addEventListener('click', function () {
        setPressed(btns, b);
        var m = b.getAttribute('data-mode');
        code.classList.toggle('sim-mode', m === 'sim');
        code.classList.toggle('view-mode', m === 'view');
        demo.classList.toggle('focus-sim', m === 'sim');
        demo.classList.toggle('focus-view', m === 'view');
        note.innerHTML = m === 'sim'
          ? '<en><b>Through the simulator’s eyes</b>: the struck-out lines start with *, so they are comments and get skipped. What remains is an ordinary netlist.</en>'
          : m === 'view'
            ? '<en><b>Through an RNL tool’s eyes</b>: the netlist gives the devices and connections, the RNL in the comments gives structure and drawing, and the standard package adds the ground symbols; simulation commands such as .ac are not needed.</en>'
            : IDLE;
      });
    });
  }

  /* ================= 01 Who owns what ================= */

  function initOwners() {
    var svg = $('#own-svg');
    if (!svg) return;
    var view = new S.View(svg, { box: SC.OTA_BOX });
    view.setScene(SC.ownerScene());
    var cap = $('#own-cap');
    var DEFAULT = '<en>The four items on the left come from the netlist and are the circuit itself; the four on the right are written in RNL and only describe understanding and presentation. <b>Hover or click</b> an item to see its part of the drawing.</en>';
    var DEV = ['M1', 'M2', 'M3', 'M4', 'M5'];
    var PORTS = ['P_inp', 'P_inn', 'P_out', 'P_vb'];
    var MODES = {
      devices: { hl: DEV, cap: '<en><b>Devices and types</b>: M1–M5 are the five MOS transistors in the netlist. Their names and kinds are decided by the netlist alone.</en>' },
      params: { hl: DEV, cls: 'show-values', cap: '<en><b>Parameters</b>: values such as W and L stay in the netlist. RNL can reference them in text (<code>${w}</code>), but never keeps a second copy.</en>' },
      nets: { cls: 'color-nets show-nets', cap: '<en><b>Connectivity</b>: each color is one net. Which terminals are joined comes entirely from the netlist; how the wires are drawn can change, the connections cannot.</en>' },
      ports: { hl: PORTS, cap: '<en><b>Subcircuit interface</b>: the ports inp, inn, out and vbias are defined by the .subckt line. The port tags in the drawing are added for them by the standard package.</en>' },
      semantic: { mode: 'semantic', cap: '<en><b>Structure</b>: M1 and M2 are a differential pair with M5 as its tail, and M3 and M4 a current mirror. These are two semantic statements in RNL; they say what things are, not how to draw them.</en>' },
      intent: { mode: 'intent', cap: '<en><b>Intent</b>: symmetry about the center line, a centered tail (two equal segments, which the differential pair carries), M5, M1 and M3 in order from bottom to top (badges 1, 2, 3), and M5’s current direction.</en>' },
      geometry: { mode: 'geometry', cls: 'color-none', cap: '<en><b>Drawing</b>: each device’s position and orientation, each wire’s path. You can write them yourself or let the solver write them back. Coordinates are grid units, x to the right and y up.</en>' },
      virtual: { hl: ['VDD1', 'VDD2', 'GND'].concat(PORTS), cap: '<en><b>Display elements</b>: supply, ground and port symbols exist only for display and are not electrical elements. They attach to nets or terminals, create no connections, and are mostly added by the standard package.</en>' }
    };
    var buttons = $$('#owners .own button');
    var pinned = null;
    function apply(key) {
      svg.classList.remove('show-values', 'color-nets', 'show-nets');
      buttons.forEach(function (b) { b.classList.toggle('is-on', b.getAttribute('data-own') === key); });
      var m = key ? MODES[key] : null;
      view.setMode(m && m.mode ? m.mode : null);
      view.highlight(m && m.hl ? m.hl : null);
      if (m && m.cls) m.cls.split(' ').forEach(function (c) { if (c !== 'color-none') svg.classList.add(c); });
      cap.innerHTML = m ? m.cap : DEFAULT;
    }
    buttons.forEach(function (b) {
      var key = b.getAttribute('data-own');
      b.addEventListener('mouseenter', function () { apply(key); });
      b.addEventListener('focus', function () { apply(key); });
      b.addEventListener('click', function () {
        pinned = pinned === key ? null : key;
        b.setAttribute('aria-pressed', pinned === key ? 'true' : 'false');
        apply(pinned || key);
      });
    });
    $('#owners').addEventListener('mouseleave', function () { apply(pinned); });
    apply(null);
  }

  /* ================= 02 Thumbnails for the workarounds ================= */

  function initFixes() {
    var svg = $('#fix-air');
    if (!svg) return;
    new S.View(svg, { box: SC.OTA_BOX, grid: false }).setScene(SC.otaScene(0));
  }

  /* ================= 03 Vision: the central file and its links ================= */

  function initHub() {
    var hub = $('#hub'), svg = $('#hub-wires'), core = $('#hub-core');
    if (!hub) return;
    $('#hub-lines').innerHTML = C.hlFile([
      '* 5T OTA',
      'M1 x inp tail vss nch W=2u L=0.5u',
      'M2 out inn tail vss nch W=2u L=0.5u',
      '* @RNL {',
      '*   %dp1 = #M1 #M2 @AS $DIFFERENTIAL_PAIR;',
      '*   #M2 @ORIENT("MY")',
      '* }'
    ]).map(function (h) { return '<div>' + h.html + '</div>'; }).join('');
    var nodes = $$('.hub-node', hub);
    function draw() {
      while (svg.firstChild) svg.removeChild(svg.firstChild);
      if (window.getComputedStyle(svg).display === 'none') return;
      var hb = hub.getBoundingClientRect(), cb = core.getBoundingClientRect();
      nodes.forEach(function (n, i) {
        var nb = n.getBoundingClientRect();
        var left = nb.right <= cb.left;
        var x0 = left ? cb.left - hb.left : cb.right - hb.left;
        var y0 = cb.top - hb.top + cb.height * (0.28 + 0.22 * (i % 3));
        var x1 = left ? nb.right - hb.left : nb.left - hb.left;
        var y1 = nb.top - hb.top + nb.height / 2;
        var xm = Math.round((x0 + x1) / 2);
        var d = 'M' + x0 + ' ' + y0 + ' H' + xm + ' V' + y1 + ' H' + x1;
        S.el('path', { d: d }, svg);
        S.el('circle', { cx: x0, cy: y0, r: 3.2, class: 'end' }, svg);
        S.el('circle', { cx: x1, cy: y1, r: 3.2, class: 'end' }, svg);
        if (!REDUCE) {
          var c = S.el('circle', { r: 3.4, class: 'pulse' }, svg);
          S.el('animateMotion', { dur: '2.8s', repeatCount: 'indefinite', begin: (i * 0.45).toFixed(2) + 's', path: d }, c);
        }
      });
    }
    draw();
    window.addEventListener('resize', debounce(draw, 150));
    if (document.fonts && document.fonts.ready) document.fonts.ready.then(draw);
  }

  /* ================= 03 Step-by-step refinement ================= */

  var JR = [
    { cap: '<en><b>Netlist only</b>: the netlist holds only devices and connections. The standard package gives each instance a default symbol by its letter (four-terminal MOS), adds a port tag for each port of the subcircuit and joins everything with airwires. It cannot tell which two transistors form a pair, or how to arrange them.</en>' },
    { cap: '<en><b>Add structure</b>: two semantic statements: M1 and M2 are a differential pair with the tail transistor M5, and M3 and M4 a current mirror. Semantics say what things are, not how to draw them, so nothing moves; a viewer can outline them, and an editor can select the pair as one.</en>' },
    { cap: '<en><b>Add intent</b>: pick three-terminal MOS symbols; treat vdd and vss as supply and ground, so they get no port tags and the ground on vss shows its net; then add symmetry (two mirrored pairs sharing one axis %ax), a bottom-to-top order and M5’s current direction. The centered tail needs no separate statement: the differential pair definition includes it at low priority. Not a single coordinate is written, and the solver lays out the textbook drawing from these constraints.</en>' },
    { cap: '<en><b>Solver writes back</b>: the solver writes its result as a record with <code>type="generated"</code> and priority high: coordinates, orientations, wires. From then on the file opens and displays without solving again; <code>hash</code> is a hash of what the solver was given. Records written by tools carry no free text.</en>' },
    { cap: '<en><b>Tweak by hand</b>: drag M2 two units right in the editor. The editor leaves the solver’s record alone and appends a record of its own, also high: within one priority the later statement wins, so it overrides the solver. The symmetry and the centered tail conflict with these high coordinates and give way by priority.</en>', notice: ['warn', 'warn', '<en>The input changed after solving (hash mismatch). The old result still displays; solving again is suggested.</en>'] },
    { cap: '<en><b>Solve again</b>: the solver replaces its own record as a whole. The editor’s statement is high, so the solver treats it as a fixed position: M2 stays at x = 30, the other devices adjust, the symmetry returns and the tail is centered again.</en>', notice: ['ok', 'check', '<en>Solved again; the result matches the current input.</en>'] }
  ];

  function initJourney() {
    var svg = $('#jr-svg'), code = $('#jr-code');
    if (!svg) return;
    var cap = $('#jr-cap'), notice = $('#jr-notice'), metrics = $('#jr-metrics');
    var steps = $$('#jr-steps button');
    var prevBtn = $('#jr-prev'), nextBtn = $('#jr-next'), playBtn = $('#jr-play'), openLink = $('#jr-open');
    var linker = null;
    var view = new S.View(svg, {
      box: SC.OTA_BOX,
      onHover: function (ref) { if (linker) linker.markRef(ref); }
    });
    var cur = -1, lastLines = null, style = 'en';

    function updateOpen() {
      if (!openLink) return;
      var target = cur === 4 ? 'ota5t-edits.sp' : cur === 5 ? 'ota5t-resolved.sp' : style === 'zh' ? 'ota5t-zh.sp' : 'ota5t.sp';
      openLink.setAttribute('href', openLink.getAttribute('href').replace(/url=samples\/site\/[^&#]*/, 'url=samples/site/' + target));
    }

    function showMetrics() {
      var m = view.metrics();
      metrics.innerHTML = fill(cur <= 1
        ? '<en>Airwires <b>{air}</b> · crossings <b>{crossings}</b></en>'
        : '<en>Crossings <b>{crossings}</b> · bends <b>{bends}</b> · wires <b>{wires}</b></en>', m);
    }

    function renderCode(i, marks) {
      var lines = SC.otaCode(i, style);
      if (marks && lastLines) {
        var old = {};
        lastLines.forEach(function (l) { old[l.id] = l.t; });
        lines.forEach(function (l) {
          if (!(l.id in old)) l.mark = 'is-add';
          else if (old[l.id] !== l.t) l.mark = 'is-chg';
        });
      }
      lastLines = lines;
      C.render(code, lines);
      if (!linker) linker = C.link(code, lines, view);
      else linker.setLines(lines);
      return lines;
    }

    function go(i, animate) {
      if (i < 0 || i > 5) return;
      var back = i < cur;
      cur = i;
      updateOpen();
      steps.forEach(function (b, j) {
        b.setAttribute('aria-selected', j === i ? 'true' : 'false');
        b.tabIndex = j === i ? 0 : -1;
        b.classList.toggle('is-done', j < i);
      });
      renderCode(i, !back);
      var first = code.querySelector('.is-add, .is-chg');
      if (first) {
        var top = first.offsetTop - 48;
        if (code.scrollTo) code.scrollTo({ top: Math.max(0, top), behavior: REDUCE ? 'auto' : 'smooth' });
        else code.scrollTop = top;
      } else if (i === 0) {
        code.scrollTop = 0;
      }
      view.setScene(SC.otaScene(i), { animate: !!animate, duration: 1100, done: showMetrics });
      if (!animate) showMetrics();
      cap.innerHTML = JR[i].cap;
      var n = JR[i].notice;
      if (n) {
        notice.className = 'jr-notice ' + n[0];
        notice.innerHTML = icon(n[1]) + '<span>' + n[2] + '</span>';
        notice.hidden = false;
      } else {
        notice.hidden = true;
      }
      prevBtn.disabled = i === 0;
      nextBtn.disabled = i === 5;
    }

    /* The same file in another writing style: machines read exactly the same statements */
    PG.initSeg($('#jr-style'), 'data-style', function (v) {
      style = v;
      updateOpen();
      var keep = code.scrollTop;
      renderCode(cur, false);
      code.scrollTop = keep;
    });

    var playing = false, timer = null;
    function setPlay(on) {
      playing = on;
      clearTimeout(timer);
      playBtn.setAttribute('aria-pressed', on ? 'true' : 'false');
      playBtn.innerHTML = icon(on ? 'pause' : 'play') + '<span>' + (on ? '<en>Pause</en>' : '<en>Autoplay</en>') + '</span>';
      if (on) schedule();
    }
    function schedule() {
      timer = setTimeout(function () {
        if (!playing) return;
        if (cur >= 5) { setPlay(false); return; }
        go(cur + 1, true);
        schedule();
      }, cur === 0 ? 2600 : 4800);
    }

    steps.forEach(function (b, j) {
      b.addEventListener('click', function () { setPlay(false); go(j, true); });
      b.addEventListener('keydown', function (e) {
        var k = e.key, t = -1;
        if (k === 'ArrowRight') t = Math.min(5, j + 1);
        if (k === 'ArrowLeft') t = Math.max(0, j - 1);
        if (t >= 0) { e.preventDefault(); setPlay(false); go(t, true); steps[t].focus(); }
      });
    });
    prevBtn.addEventListener('click', function () { setPlay(false); go(cur - 1, true); });
    nextBtn.addEventListener('click', function () { setPlay(false); go(cur + 1, true); });
    playBtn.addEventListener('click', function () {
      if (playing) { setPlay(false); return; }
      if (cur >= 5) go(0, true);
      setPlay(true);
    });

    go(0, false);
    var started = false;
    onVisible($('.journey'), function (v) {
      if (v && !started && !REDUCE) {
        started = true;
        setTimeout(function () { if (cur === 0 && !playing) setPlay(true); }, 900);
      }
      if (!v && playing) setPlay(false);
    }, 0.35);
  }

  /* ================= 04 Scenarios ================= */

  function initScenes() {
    var list = $('#sc-tabs');
    if (!list) return;
    var made = {};
    function thumbs(id) {
      if (made[id]) return;
      made[id] = true;
      if (id === 't-ai') {
        new S.View($('#ai-thumb'), { box: SC.OTA_BOX, grid: false }).setScene(SC.otaScene(2));
      }
      if (id === 't-auto') {
        var a = new S.View($('#auto-air'), { box: SC.OTA_BOX, grid: false });
        a.setScene(SC.otaScene(0));
        var b = new S.View($('#auto-sym'), { box: SC.OTA_BOX, grid: false });
        b.setScene(SC.otaScene(2));
        var CROSS = '<en>{crossings} crossings</en>';
        $('#auto-air-cap').innerHTML = '<span class="chip chip-err">' + fill(CROSS, a.metrics()) + '</span><span class="chip chip-err"><en>No visible structure</en></span>';
        $('#auto-sym-cap').innerHTML = '<span class="chip chip-ok">' + fill(CROSS, b.metrics()) + '</span><span class="chip chip-ok"><en>Pair and mirror obvious at a glance</en></span>';
      }
    }
    PG.initTabs(list, function (t) { thumbs(t.id); });
  }

  /* ================= Startup ================= */

  function boot() {
    initLogoViewer();
    initHero();
    initOwners();
    initFixes();
    initHub();
    initJourney();
    initScenes();
  }

  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', boot);
  else boot();
})();
