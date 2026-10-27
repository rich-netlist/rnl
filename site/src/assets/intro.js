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

  /* ================= 01 Thumbnails for the workarounds ================= */

  function initFixes() {
    var svg = $('#fix-air');
    if (!svg) return;
    new S.View(svg, { box: SC.OTA_BOX, grid: false }).setScene(SC.otaScene(0));
  }

  /* ================= 05 Vision: the central file and its links ================= */

  function initHub() {
    var hub = $('#hub'), svg = $('#hub-wires'), core = $('#hub-core');
    if (!hub) return;
    $('#hub-lines').innerHTML = C.hlFile([
      '* Two-stage Miller-compensated op-amp',
      'M1 x inp tail vss nch W=40u L=1u',
      'M2 n1 inn tail vss nch W=40u L=1u',
      '* @RNL {',
      '*   %dp1 = #M1 #M2 #M5 @AS $DIFFERENTIAL_PAIR;',
      '*   %cm1 = #M3 #M4 @AS $CURRENT_MIRROR',
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

  /* ================= 04 Scenarios ================= */

  function initScenes() {
    var list = $('#sc-tabs');
    if (!list) return;
    var made = {};
    function thumbs(id) {
      if (made[id]) return;
      made[id] = true;
      if (id === 't-ai') {
        new S.View($('#ai-thumb'), { box: window.RNLOpamp.BOX, grid: false }).setScene(window.RNLOpamp.scene());
      }
    }
    PG.initTabs(list, function (t) { thumbs(t.id); });
  }

  /* ================= Startup ================= */

  function boot() {
    initLogoViewer();
    initHero();
    initFixes();
    initHub();
    if (window.RNLOpamp) window.RNLOpamp.init();
    initScenes();
  }

  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', boot);
  else boot();
})();
